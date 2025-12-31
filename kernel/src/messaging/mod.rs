//! Messaging and Events subsystem for the NOS kernel.
//!
//! This module provides comprehensive messaging capabilities including:
//! - Multiple queue types (FIFO, priority, delay queues)
//! - Topic-based publish/subscribe with wildcards
//! - In-process event bus
//! - Durable message persistence
//! - Dead letter queue for failed messages
//! - Per-key ordering guarantees
//!
//! # Architecture
//!
//! The messaging system is organized into several components:
//!
//! - **Queue**: Basic queue implementations for message storage
//! - **PubSub**: Topic-based publish/subscribe with wildcard support
//! - **EventBus**: In-process event dispatch and handling
//! - **Persistence**: Durable message storage with recovery
//! - **DLQ**: Dead letter queue for failed message handling
//! - **Ordering**: Message ordering and sequencing guarantees
//!
//! # Example Usage
//!
//! ```rust
//! use kernel::messaging::{
//!     queue::{FifoQueue, Priority},
//!     pubsub::{PubSub, Topic, Message},
//!     eventbus::{EventBus, Event},
//! };
//!
//! // FIFO Queue
//! let queue = FifoQueue::new(100);
//! queue.enqueue(42).unwrap();
//! let msg = queue.dequeue().unwrap();
//!
//! // Pub/Sub
//! let bus = PubSub::new();
//! bus.subscribe("events.*").unwrap();
//! let topic = Topic::new("events.user.login").unwrap();
//! let message = Message::new(topic, b"user_data".to_vec());
//! bus.publish(message).unwrap();
//!
//! // Event Bus
//! let event_bus = EventBus::new();
//! event_bus.register_handler(|event: &MyEvent, ctx| {
//!     println!("Event received: {:?}", event);
//!     HandlerResult::Handled
//! }).unwrap();
//! ```

pub mod queue;
pub mod pubsub;
pub mod eventbus;
pub mod persistence;
pub mod dlq;
pub mod ordering;

// Re-export commonly used types
pub use queue::{
    FifoQueue,
    PriorityQueue,
    DelayQueue,
    Priority,
    QueueError,
    QueueResult,
    BackpressureStrategy,
    QueueConfig,
};

pub use pubsub::{
    PubSub,
    Topic,
    TopicId,
    SubscriberId,
    Message as PubSubMessage,
    SubscriptionConfig,
    Subscriber,
    PubSubError,
    PubSubResult,
};

pub use eventbus::{
    EventBus,
    Event,
    EventBox,
    EventContext,
    HandlerId,
    HandlerResult,
    EventPriority,
    EventBusConfig,
    EventStatistics,
    EventBusError,
    EventBusResult,
};

pub use persistence::{
    DurableQueue,
    PersistenceManager,
    PersistentMessage,
    MessageId,
    QueueId as PersistenceQueueId,
    DeliverySemantics,
    AckStatus,
    StorageBackend,
    Transaction,
    RecoveryReport,
    PersistenceError,
    PersistenceResult,
};

pub use dlq::{
    DeadLetterQueue,
    DlqManager,
    DeadLetter,
    DeadLetterId,
    RetryPolicy,
    FailureReason,
    DlqStats,
    DlqError,
    DlqResult,
};

pub use ordering::{
    OrderedQueue,
    SequenceManager,
    OutOfOrderDetector,
    SequencedMessage,
    SequenceNumber,
    PartitionKey,
    OrderingGuarantee,
    OrderingError,
    OrderingResult,
};

use alloc::sync::Arc;

// Import HashMap and String for InMemoryStorageBackend
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::collections::HashMap;
use crate::sync::Mutex;
use core::sync::atomic::AtomicU64;

/// Messaging system configuration
#[derive(Debug, Clone)]
pub struct MessagingConfig {
    /// Default queue capacity
    pub default_queue_capacity: usize,
    /// Default backpressure strategy
    pub backpressure_strategy: BackpressureStrategy,
    /// Enable message persistence
    pub enable_persistence: bool,
    /// Default delivery semantics
    pub delivery_semantics: DeliverySemantics,
    /// Maximum dead letter queue size
    pub max_dlq_size: usize,
    /// Default retry policy for dead letters
    pub retry_policy: RetryPolicy,
    /// Ordering guarantee level
    pub ordering_guarantee: OrderingGuarantee,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            default_queue_capacity: 1000,
            backpressure_strategy: BackpressureStrategy::Block,
            enable_persistence: false,
            delivery_semantics: DeliverySemantics::AtLeastOnce,
            max_dlq_size: 10000,
            retry_policy: RetryPolicy::default(),
            ordering_guarantee: OrderingGuarantee::PerPartition,
        }
    }
}

// Unified messaging system
//
// Provides a single interface to all messaging components.
pub struct MessagingSystem {
    /// Configuration
    config: MessagingConfig,
    /// Pub/Sub bus
    pubsub: Arc<PubSub>,
    /// Event bus
    event_bus: Arc<EventBus>,
    /// Persistence manager (optional)
    persistence: Option<Arc<PersistenceManager>>,
    /// Dead letter queue manager
    dlq_manager: Arc<DlqManager>,
    /// Sequence manager
    sequence_manager: Arc<SequenceManager>,
}

impl MessagingSystem {
    /// Creates a new messaging system
    pub fn new() -> Self {
        Self::with_config(MessagingConfig::default())
    }

    /// Creates a messaging system with custom configuration
    pub fn with_config(config: MessagingConfig) -> Self {
        let pubsub = Arc::new(PubSub::new());
        let event_bus = Arc::new(EventBus::new());
        let dlq_manager = Arc::new(DlqManager::new());
        let sequence_manager = Arc::new(SequenceManager::new());

        // Create persistence manager if enabled
        let persistence = if config.enable_persistence {
            // In a real implementation, this would use actual storage
            Some(Arc::new(PersistenceManager::new(Arc::new(
                InMemoryStorageBackend::new(),
            ))))
        } else {
            None
        };

        Self {
            config,
            pubsub,
            event_bus,
            persistence,
            dlq_manager,
            sequence_manager,
        }
    }

    /// Returns the pub/sub bus
    pub fn pubsub(&self) -> &PubSub {
        &self.pubsub
    }

    /// Returns the event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Returns the persistence manager (if enabled)
    pub fn persistence(&self) -> Option<&PersistenceManager> {
        self.persistence.as_ref().map(|p| p.as_ref())
    }

    /// Returns the DLQ manager
    pub fn dlq_manager(&self) -> &DlqManager {
        &self.dlq_manager
    }

    /// Returns the sequence manager
    pub fn sequence_manager(&self) -> &SequenceManager {
        &self.sequence_manager
    }

    /// Shuts down all messaging components
    pub fn shutdown(&self) {
        self.pubsub.shutdown();
        self.event_bus.shutdown();
    }

    /// Returns system statistics
    pub fn stats(&self) -> MessagingStats {
        MessagingStats {
            pubsub_topics: self.pubsub.topics().len(),
            event_handlers: self.event_bus.handler_count(),
            event_stats: self.event_bus.stats(),
            dlq_stats: self.dlq_manager.stats(),
        }
    }
}

impl Default for MessagingSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Messaging system statistics
#[derive(Debug, Clone)]
pub struct MessagingStats {
    /// Number of pub/sub topics
    pub pubsub_topics: usize,
    /// Number of event handlers registered
    pub event_handlers: usize,
    /// Event bus statistics
    pub event_stats: EventStatistics,
    /// Dead letter queue statistics
    pub dlq_stats: DlqStats,
}

// In-memory storage backend stub (for persistence module)
// Arc, Vec, HashMap, Mutex, and persistence types already imported above

pub struct InMemoryStorageBackend {
    messages: Mutex<HashMap<u64, PersistentMessage>>,
    queue_messages: Mutex<HashMap<String, Vec<u64>>>,
}

impl InMemoryStorageBackend {
    fn new() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
            queue_messages: Mutex::new(HashMap::new()),
        }
    }
}

impl crate::messaging::persistence::StorageBackend for InMemoryStorageBackend {
    fn store(&self, message: &PersistentMessage) -> PersistenceResult<()> {
        let mut messages = self.messages.lock();
        messages.insert(message.id, message.clone());

        let mut queue_messages = self.queue_messages.lock();
        queue_messages
            .entry(message.queue_id.clone())
            .or_insert_with(Vec::new)
            .push(message.id);

        Ok(())
    }

    fn retrieve(&self, id: u64) -> PersistenceResult<PersistentMessage> {
        let messages = self.messages.lock();
        messages
            .get(&id)
            .cloned()
            .ok_or(PersistenceError::MessageNotFound(id))
    }

    fn delete(&self, id: u64) -> PersistenceResult<()> {
        let queue_id = {
            let messages = self.messages.lock();
            messages
                .get(&id)
                .map(|m| m.queue_id.clone())
                .ok_or(PersistenceError::MessageNotFound(id))?
        };

        {
            let mut messages = self.messages.lock();
            messages.remove(&id);
        }

        {
            let mut queue_messages = self.queue_messages.lock();
            if let Some(ids) = queue_messages.get_mut(&queue_id) {
                ids.retain(|&msg_id| msg_id != id);
            }
        }

        Ok(())
    }

    fn list_messages(&self, queue_id: &nos_api::String) -> PersistenceResult<Vec<PersistentMessage>> {
        let queue_messages = self.queue_messages.lock();
        let messages = self.messages.lock();

        let ids = queue_messages
            .get(&queue_id.to_string())
            .ok_or(persistence::PersistenceError::QueueNotFound(queue_id.to_string()))?;

        let result = ids
            .iter()
            .filter_map(|id| messages.get(id).cloned())
            .collect();

        Ok(result)
    }

    fn begin_transaction(&self) -> PersistenceResult<Box<dyn crate::messaging::persistence::Transaction>> {
        Ok(Box::new(InMemoryTransaction::new(self)))
    }

    fn recover(&self) -> PersistenceResult<crate::messaging::persistence::RecoveryReport> {
        Ok(crate::messaging::persistence::RecoveryReport {
            messages_recovered: 0,
            messages_corrupted: 0,
            messages_expired: 0,
            queues_recovered: 0,
            recovery_duration_ms: 0,
        })
    }
}

struct InMemoryTransaction {
    storage: Arc<InMemoryStorageBackend>,
    to_store: Vec<PersistentMessage>,
    to_delete: Vec<u64>,
    committed: bool,
}

impl InMemoryTransaction {
    fn new(_storage: &InMemoryStorageBackend) -> Self {
        // This is a simplified stub - real implementation would need proper Arc sharing
        Self {
            storage: Arc::new(InMemoryStorageBackend::new()),
            to_store: Vec::new(),
            to_delete: Vec::new(),
            committed: false,
        }
    }
}

impl crate::messaging::persistence::Transaction for InMemoryTransaction {
    fn store(&mut self, message: &PersistentMessage) -> PersistenceResult<()> {
        self.to_store.push(message.clone());
        Ok(())
    }

    fn delete(&mut self, id: u64) -> PersistenceResult<()> {
        self.to_delete.push(id);
        Ok(())
    }

    fn commit(mut self: Box<Self>) -> PersistenceResult<()> {
        self.committed = true;
        for msg in &self.to_store {
            self.storage.store(msg)?;
        }
        for id in &self.to_delete {
            self.storage.delete(*id)?;
        }
        Ok(())
    }

    fn rollback(mut self: Box<Self>) -> PersistenceResult<()> {
        self.committed = false;
        self.to_store.clear();
        self.to_delete.clear();
        Ok(())
    }
}

/// Helper function to get current timestamp
#[inline(always)]
pub fn current_timestamp() -> u64 {
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

/// Generates a unique ID
#[inline(always)]
pub fn generate_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::messaging::queue::{FifoQueue, Priority};

    #[test]
    fn test_messaging_system_creation() {
        let system = MessagingSystem::new();
        assert!(system.pubsub().topics().is_empty());
        assert_eq!(system.event_bus().handler_count(), 0);
    }

    #[test]
    fn test_fifo_queue_integration() {
        let queue: FifoQueue<u32> = FifoQueue::new(10);

        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();
        queue.enqueue(3).unwrap();

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.dequeue().unwrap(), 1);
        assert_eq!(queue.dequeue().unwrap(), 2);
        assert_eq!(queue.dequeue().unwrap(), 3);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_pubsub_integration() {
        let system = MessagingSystem::new();

        // Subscribe to a topic
        let sub_id = system.pubsub().subscribe("test.*").unwrap();
        assert!(sub_id > 0);

        // Publish a message
        let topic = Topic::new("test.event").unwrap();
        let message = PubSubMessage::new(topic, b"payload".to_vec());
        let count = system.pubsub().publish(message).unwrap();

        // Should deliver to one subscriber
        assert_eq!(count, 1);
    }

    #[test]
    fn test_eventbus_integration() {
        let system = MessagingSystem::new();

        #[derive(Debug)]
        struct TestEvent {
            value: u32,
        }

        impl Event for TestEvent {}

        let handler_id = system
            .event_bus()
            .register_handler(|event: &TestEvent, _ctx| HandlerResult::Handled)
            .unwrap();

        assert!(handler_id > 0);
        assert_eq!(system.event_bus().handler_count(), 1);

        let event = TestEvent { value: 42 };
        system.event_bus().dispatch(event).unwrap();
    }

    #[test]
    fn test_system_shutdown() {
        let system = MessagingSystem::new();

        system.pubsub().subscribe("test.*").unwrap();
        system.shutdown();

        // Should fail to subscribe after shutdown
        assert!(system.pubsub().subscribe("test2.*").is_err());
    }

    #[test]
    fn test_system_stats() {
        let system = MessagingSystem::new();

        system.pubsub().subscribe("test.*").unwrap();
        system.event_bus().register_handler(|_: &TestEvent, _| HandlerResult::Handled).unwrap();

        let stats = system.stats();
        assert_eq!(stats.pubsub_topics, 0); // No topics created yet
        assert_eq!(stats.event_handlers, 1);
    }

    #[derive(Debug)]
    struct TestEvent;

    impl Event for TestEvent {}
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use crate::messaging::queue::FifoQueue;
    use std::time::Instant;

    #[test]
    #[ignore] // Expensive benchmark
    fn benchmark_fifo_queue() {
        let queue: FifoQueue<u64> = FifoQueue::new(1_000_000);
        let iterations = 100_000;

        // Benchmark enqueue
        let start = Instant::now();
        for i in 0..iterations {
            queue.enqueue(i).unwrap();
        }
        let enqueue_duration = start.elapsed();

        // Benchmark dequeue
        let start = Instant::now();
        for _ in 0..iterations {
            queue.dequeue().unwrap();
        }
        let dequeue_duration = start.elapsed();

        println!("FIFO Queue Benchmark:");
        println!("  Enqueue: {} ops/sec", iterations as f64 / enqueue_duration.as_secs_f64());
        println!("  Dequeue: {} ops/sec", iterations as f64 / dequeue_duration.as_secs_f64());
    }

    #[test]
    #[ignore] // Expensive benchmark
    fn benchmark_pubsub() {
        let bus = PubSub::new();
        let iterations = 10_000;

        // Create multiple subscribers
        for _ in 0..10 {
            bus.subscribe("benchmark.*").unwrap();
        }

        // Benchmark publish
        let topic = Topic::new("benchmark.test").unwrap();
        let start = Instant::now();
        for i in 0..iterations {
            let message = PubSubMessage::new(topic.clone(), vec![i as u8; 100]);
            bus.publish(message).unwrap();
        }
        let duration = start.elapsed();

        println!("Pub/Sub Benchmark:");
        println!("  Publish: {} msgs/sec", iterations as f64 / duration.as_secs_f64());
    }
}
