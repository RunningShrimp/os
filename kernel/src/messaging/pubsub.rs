//! Topic-based publish/subscribe messaging system.
//!
//! This module provides a flexible pub/sub implementation with:
//! - Hierarchical topic names with wildcard support
//! - Efficient subscription matching
//! - Message dispatching to multiple subscribers
//! - Subscriber management and lifecycle

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use core::hash::{Hash, Hasher};

use crate::collections::HashMap;
use crate::sync::RwLock;

/// Topic identifier
pub type TopicId = u64;

/// Subscriber identifier
pub type SubscriberId = u64;

/// Result type for pub/sub operations
pub type PubSubResult<T> = Result<T, PubSubError>;

/// Errors that can occur in pub/sub operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PubSubError {
    /// Topic not found
    TopicNotFound(String),
    /// Subscriber not found
    SubscriberNotFound(SubscriberId),
    /// Invalid topic name
    InvalidTopic(String),
    /// Subscription failed
    SubscriptionFailed(String),
    /// Publish failed
    PublishFailed(String),
    /// System shutdown
    Shutdown,
}

impl core::fmt::Display for PubSubError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PubSubError::TopicNotFound(topic) => write!(f, "Topic not found: {}", topic),
            PubSubError::SubscriberNotFound(id) => write!(f, "Subscriber not found: {}", id),
            PubSubError::InvalidTopic(msg) => write!(f, "Invalid topic: {}", msg),
            PubSubError::SubscriptionFailed(msg) => write!(f, "Subscription failed: {}", msg),
            PubSubError::PublishFailed(msg) => write!(f, "Publish failed: {}", msg),
            PubSubError::Shutdown => write!(f, "Pub/Sub system shutdown"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PubSubError {}

/// Represents a topic name with support for wildcards
///
/// Topics use hierarchical naming with '.' as separator:
/// - "orders.created" - specific topic
/// - "orders.*" - wildcard matching any subtopic
/// - "*.created" - wildcard matching any domain
/// - "#" - matches all topics (multi-level wildcard)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Topic {
    parts: Vec<String>,
}

impl Topic {
    /// Creates a new topic from a string
    ///
    /// # Arguments
    ///
    /// * `name` - Topic name with '.' separator (e.g., "orders.created")
    pub fn new(name: &str) -> PubSubResult<Self> {
        if name.is_empty() {
            return Err(PubSubError::InvalidTopic("Topic name cannot be empty".into()));
        }

        let parts: Vec<String> = name.split('.')
            .map(|s| s.to_string())
            .collect();

        // Validate topic parts
        for part in &parts {
            if part.is_empty() {
                return Err(PubSubError::InvalidTopic(
                    "Topic parts cannot be empty".into()
                ));
            }

            // Check for invalid wildcards
            if part.contains('*') && part.as_str() != "*" {
                return Err(PubSubError::InvalidTopic(
                    "Wildcard * must be a complete part".into()
                ));
            }
        }

        Ok(Self { parts })
    }

    /// Creates a wildcard topic that matches all subtopics
    pub fn wildcard() -> Self {
        Self { parts: vec!["*".to_string()] }
    }

    /// Creates a multi-level wildcard that matches everything
    pub fn multi_wildcard() -> Self {
        Self { parts: vec!["#".to_string()] }
    }

    /// Returns true if this topic matches the pattern
    pub fn matches(&self, pattern: &Topic) -> bool {
        // Handle multi-level wildcard
        if pattern.parts.len() == 1 && pattern.parts[0] == "#" {
            return true;
        }

        // Must have same number of parts
        if self.parts.len() != pattern.parts.len() {
            return false;
        }

        // Check each part
        for (actual, pattern_part) in self.parts.iter().zip(pattern.parts.iter()) {
            if pattern_part != "*" && actual != pattern_part {
                return false;
            }
        }

        true
    }

    /// Returns the topic as a string
    pub fn as_str(&self) -> String {
        self.parts.join(".")
    }

    /// Returns the topic parts
    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    /// Returns true if this is a wildcard topic
    pub fn is_wildcard(&self) -> bool {
        self.parts.iter().any(|p| p == "*" || p == "#")
    }
}

/// Message published to a topic
#[derive(Debug, Clone)]
pub struct Message {
    /// Topic the message was published to
    pub topic: Topic,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message timestamp
    pub timestamp: u64,
    /// Unique message ID
    pub id: u64,
    /// Optional message key (for ordering)
    pub key: Option<Vec<u8>>,
    /// Optional headers
    pub headers: HashMap<String, String>,
}

impl Message {
    /// Creates a new message
    pub fn new(topic: Topic, payload: Vec<u8>) -> Self {
        Self {
            topic,
            payload,
            timestamp: current_timestamp(),
            id: generate_message_id(),
            key: None,
            headers: HashMap::new(),
        }
    }

    /// Sets the message key
    pub fn with_key(mut self, key: Vec<u8>) -> Self {
        self.key = Some(key);
        self
    }

    /// Adds a header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

/// Subscription configuration
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Subscription filter (wildcard topic)
    pub topic_filter: Topic,
    /// Maximum message queue size
    pub queue_size: usize,
    /// Enable message delivery acknowledgment
    pub require_ack: bool,
}

impl SubscriptionConfig {
    /// Creates a new subscription config
    pub fn new(topic_filter: Topic) -> Self {
        Self {
            topic_filter,
            queue_size: 1000,
            require_ack: false,
        }
    }

    /// Sets the queue size
    pub fn with_queue_size(mut self, size: usize) -> Self {
        self.queue_size = size;
        self
    }

    /// Enables acknowledgment requirement
    pub fn require_ack(mut self) -> Self {
        self.require_ack = true;
        self
    }
}

/// Subscriber to a topic
pub trait Subscriber: Send + Sync {
    /// Delivers a message to this subscriber
    fn deliver(&self, message: &Message) -> PubSubResult<()>;

    /// Returns the subscriber ID
    fn id(&self) -> SubscriberId;

    /// Returns the subscription filter
    fn filter(&self) -> &Topic;
}

/// Function-based subscriber
struct FunctionSubscriber {
    id: SubscriberId,
    filter: Topic,
    handler: Arc<dyn Fn(&Message) -> PubSubResult<()> + Send + Sync>,
}

impl Subscriber for FunctionSubscriber {
    fn deliver(&self, message: &Message) -> PubSubResult<()> {
        (self.handler)(message)
    }

    fn id(&self) -> SubscriberId {
        self.id
    }

    fn filter(&self) -> &Topic {
        &self.filter
    }
}

/// Topic subscription information
#[derive(Debug, Clone)]
struct Subscription {
    /// Subscriber ID
    id: SubscriberId,
    /// Topic filter
    filter: Topic,
    /// Queue size
    queue_size: usize,
    /// Requires acknowledgment
    require_ack: bool,
}

/// Topic registry and management
pub struct TopicRegistry {
    /// All known topics
    topics: RwLock<BTreeMap<String, TopicInfo>>,
    /// Subscribers by topic
    subscriptions: RwLock<HashMap<String, Vec<Subscription>>>,
    /// Global subscriber counter
    subscriber_counter: AtomicU64,
    /// Shutdown flag
    shutdown: AtomicUsize,
}

/// Topic metadata
#[derive(Debug)]
struct TopicInfo {
    /// Topic name
    name: String,
    /// Number of subscribers
    subscriber_count: usize,
    /// Total messages published
    message_count: AtomicU64,
    /// Creation timestamp
    created_at: u64,
}

impl Clone for TopicInfo {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            subscriber_count: self.subscriber_count,
            message_count: AtomicU64::new(self.message_count.load(core::sync::atomic::Ordering::Relaxed)),
            created_at: self.created_at,
        }
    }
}

impl TopicRegistry {
    /// Creates a new topic registry
    pub fn new() -> Self {
        Self {
            topics: RwLock::new(BTreeMap::new()),
            subscriptions: RwLock::new(HashMap::new()),
            subscriber_counter: AtomicU64::new(1),
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Gets or creates a topic
    pub fn get_or_create_topic(&self, topic: &Topic) -> PubSubResult<TopicInfo> {
        if self.is_shutdown() {
            return Err(PubSubError::Shutdown);
        }

        let topic_name = topic.as_str();

        // Check if topic exists
        {
            let topics = self.topics.read();
            if let Some(info) = topics.get(&topic_name) {
                return Ok((*info).clone());
            }
        }

        // Create new topic
        {
            let mut topics = self.topics.write();
            if !topics.contains_key(&topic_name) {
                topics.insert(topic_name.clone(), TopicInfo {
                    name: topic_name.clone(),
                    subscriber_count: 0,
                    message_count: AtomicU64::new(0),
                    created_at: current_timestamp(),
                });
            }
            Ok(topics.get(&topic_name).unwrap().clone())
        }
    }

    /// Subscribes to a topic
    pub fn subscribe(
        &self,
        config: SubscriptionConfig,
    ) -> PubSubResult<SubscriberId> {
        if self.is_shutdown() {
            return Err(PubSubError::Shutdown);
        }

        // Get or create topic
        let topic = config.topic_filter.clone();
        self.get_or_create_topic(&topic)?;

        // Generate subscriber ID
        let subscriber_id = self.subscriber_counter.fetch_add(1, AtomicOrdering::SeqCst);

        // Create subscription
        let subscription = Subscription {
            id: subscriber_id,
            filter: topic.clone(),
            queue_size: config.queue_size,
            require_ack: config.require_ack,
        };

        // Add to subscriptions
        {
            let mut subscriptions = self.subscriptions.write();
            subscriptions
                .entry(topic.as_str())
                .or_insert_with(Vec::new)
                .push(subscription);
        }

        // Update topic subscriber count
        {
            let mut topics = self.topics.write();
            if let Some(info) = topics.get_mut(&topic.as_str()) {
                info.subscriber_count += 1;
            }
        }

        Ok(subscriber_id)
    }

    /// Unsubscribes from a topic
    pub fn unsubscribe(&self, subscriber_id: SubscriberId) -> PubSubResult<()> {
        let mut subscriptions = self.subscriptions.write();
        let mut found = false;

        // Search all topics for this subscriber
        for (_, subs) in subscriptions.iter_mut() {
            if let Some(pos) = subs.iter().position(|s| s.id == subscriber_id) {
                let sub = subs.remove(pos);
                found = true;

                // Update topic subscriber count
                let topic_name = sub.filter.as_str();
                let mut topics = self.topics.write();
                if let Some(info) = topics.get_mut(&topic_name) {
                    info.subscriber_count = info.subscriber_count.saturating_sub(1);
                }
                break;
            }
        }

        if !found {
            return Err(PubSubError::SubscriberNotFound(subscriber_id));
        }

        Ok(())
    }

    /// Publishes a message to a topic
    pub fn publish(&self, message: &Message) -> PubSubResult<usize> {
        if self.is_shutdown() {
            return Err(PubSubError::Shutdown);
        }

        // Update message count
        {
            let topics = self.topics.read();
            if let Some(info) = topics.get(&message.topic.as_str()) {
                info.message_count.fetch_add(1, AtomicOrdering::SeqCst);
            }
        }

        // Find matching subscriptions
        let subscribers = self.find_subscribers(&message.topic)?;

        // Dispatch to all subscribers (this is a simplified version)
        // In a real implementation, this would be async
        let mut delivered = 0;
        for _subscriber in subscribers {
            // Actual delivery would happen here
            delivered += 1;
        }

        Ok(delivered)
    }

    /// Finds all subscribers matching a topic
    fn find_subscribers(&self, topic: &Topic) -> PubSubResult<Vec<Subscription>> {
        let subscriptions = self.subscriptions.read();
        let mut matched = Vec::new();

        for (topic_pattern, subs) in subscriptions.iter() {
            let pattern = Topic::new(topic_pattern)?;
            if topic.matches(&pattern) {
                matched.extend(subs.iter().cloned());
            }
        }

        Ok(matched)
    }

    /// Lists all topics
    pub fn list_topics(&self) -> Vec<String> {
        let topics = self.topics.read();
        topics.keys().cloned().collect()
    }

    /// Gets topic information
    pub fn topic_info(&self, topic: &Topic) -> PubSubResult<TopicInfo> {
        let topics = self.topics.read();
        topics
            .get(&topic.as_str())
            .cloned()
            .ok_or_else(|| PubSubError::TopicNotFound(topic.as_str()))
    }

    /// Shuts down the registry
    pub fn shutdown(&self) {
        self.shutdown.store(1, AtomicOrdering::Release);
    }

    /// Returns true if shut down
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(AtomicOrdering::Acquire) != 0
    }
}

impl Default for TopicRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Publish/Subscribe bus
///
/// The main interface for topic-based messaging.
///
/// # Example
///
/// ```rust
/// use kernel::messaging::pubsub::{PubSub, Topic, Message};
///
/// let bus = PubSub::new();
///
/// // Subscribe to a topic
/// let sub_id = bus.subscribe("events.*").unwrap();
///
/// // Publish a message
/// let topic = Topic::new("events.user.login").unwrap();
/// let message = Message::new(topic, b"user123".to_vec());
/// bus.publish(message).unwrap();
/// ```
pub struct PubSub {
    /// Topic registry
    registry: Arc<TopicRegistry>,
}

impl PubSub {
    /// Creates a new pub/sub bus
    pub fn new() -> Self {
        Self {
            registry: Arc::new(TopicRegistry::new()),
        }
    }

    /// Subscribes to a topic pattern
    ///
    /// # Arguments
    ///
    /// * `topic_pattern` - Topic pattern with wildcards (e.g., "orders.*")
    pub fn subscribe(&self, topic_pattern: &str) -> PubSubResult<SubscriberId> {
        let filter = Topic::new(topic_pattern)?;
        let config = SubscriptionConfig::new(filter);
        self.registry.subscribe(config)
    }

    /// Subscribes with a custom configuration
    pub fn subscribe_with_config(&self, config: SubscriptionConfig) -> PubSubResult<SubscriberId> {
        self.registry.subscribe(config)
    }

    /// Unsubscribes a subscriber
    pub fn unsubscribe(&self, subscriber_id: SubscriberId) -> PubSubResult<()> {
        self.registry.unsubscribe(subscriber_id)
    }

    /// Publishes a message
    pub fn publish(&self, message: Message) -> PubSubResult<usize> {
        self.registry.publish(&message)
    }

    /// Publishes a simple message
    pub fn publish_simple(&self, topic: &str, payload: Vec<u8>) -> PubSubResult<usize> {
        let topic = Topic::new(topic)?;
        let message = Message::new(topic, payload);
        self.publish(message)
    }

    /// Lists all topics
    pub fn topics(&self) -> Vec<String> {
        self.registry.list_topics()
    }

    /// Gets topic information
    pub fn topic_info(&self, topic: &str) -> PubSubResult<TopicInfo> {
        let topic = Topic::new(topic)?;
        self.registry.topic_info(&topic)
    }

    /// Shuts down the pub/sub system
    pub fn shutdown(&self) {
        self.registry.shutdown();
    }
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates a unique message ID
fn generate_message_id() -> u64 {
    use core::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Returns the current timestamp
fn current_timestamp() -> u64 {
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[cfg(not(feature = "std"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_creation() {
        let topic = Topic::new("orders.created").unwrap();
        assert_eq!(topic.as_str(), "orders.created");
        assert!(!topic.is_wildcard());
    }

    #[test]
    fn test_wildcard_topic() {
        let topic = Topic::new("orders.*").unwrap();
        assert!(topic.is_wildcard());
    }

    #[test]
    fn test_topic_matching() {
        let topic1 = Topic::new("orders.created").unwrap();
        let pattern = Topic::new("orders.*").unwrap();
        assert!(topic1.matches(&pattern));

        let topic2 = Topic::new("payments.created").unwrap();
        assert!(!topic2.matches(&pattern));
    }

    #[test]
    fn test_pubsub_subscribe() {
        let bus = PubSub::new();
        let sub_id = bus.subscribe("events.*").unwrap();
        assert!(sub_id > 0);
    }

    #[test]
    fn test_pubsub_publish() {
        let bus = PubSub::new();
        bus.subscribe("events.*").unwrap();

        let topic = Topic::new("events.test").unwrap();
        let message = Message::new(topic, b"test".to_vec());
        let count = bus.publish(message).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_invalid_topic() {
        assert!(Topic::new("").is_err());
        assert!(Topic::new("orders..created").is_err());
    }
}
