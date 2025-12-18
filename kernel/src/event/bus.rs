//! Event Bus Implementation
//! 
//! This module provides a concrete implementation of the EventBus trait
//! for managing event publication, subscription, and routing.

use alloc::{
    collections::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
    string::String,
    boxed::Box,
};
use core::sync::atomic::{AtomicUsize, Ordering};

use nos_api::{
    event::{
        Event, EventHandler, EventDispatcher, EventBus, EventFilter, EventId,
        EventType, EventMetadata,
    },
    Result,
};

use super::dispatcher::{DefaultEventDispatcher, PriorityEventDispatcher};

/// Default implementation of an EventBus
pub struct DefaultEventBus {
    /// Event dispatcher for handling events
    dispatcher: Box<dyn EventDispatcher>,
    /// Map of topic names to subscribers
    topics: BTreeMap<String, Vec<Weak<dyn EventHandler>>>,
    /// Global event handlers (receive all events)
    global_handlers: Vec<Weak<dyn EventHandler>>,
    /// Event statistics
    stats: EventBusStats,
    /// Configuration
    config: EventBusConfig,
}

/// Statistics for the event bus
#[derive(Debug, Default)]
pub struct EventBusStats {
    /// Total number of events published
    pub events_published: AtomicUsize,
    /// Total number of events processed
    pub events_processed: AtomicUsize,
    /// Total number of events failed
    pub events_failed: AtomicUsize,
    /// Number of active subscribers
    pub active_subscribers: AtomicUsize,
    /// Number of active topics
    pub active_topics: AtomicUsize,
}

/// Configuration for the event bus
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum number of subscribers per topic
    pub max_subscribers_per_topic: usize,
    /// Maximum number of topics
    pub max_topics: usize,
    /// Whether to automatically clean up dead references
    pub auto_cleanup: bool,
    /// Cleanup interval (number of events before cleanup)
    pub cleanup_interval: usize,
    /// Whether to enable event batching
    pub enable_batching: bool,
    /// Maximum batch size
    pub max_batch_size: usize,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_subscribers_per_topic: 1000,
            max_topics: 100,
            auto_cleanup: true,
            cleanup_interval: 100,
            enable_batching: false,
            max_batch_size: 50,
        }
    }
}

impl DefaultEventBus {
    /// Create a new DefaultEventBus with default dispatcher
    pub fn new() -> Self {
        Self::with_dispatcher(Box::new(DefaultEventDispatcher::new()))
    }

    /// Create a new DefaultEventBus with priority dispatcher
    pub fn with_priority_dispatcher() -> Self {
        Self::with_dispatcher(Box::new(PriorityEventDispatcher::new()))
    }

    /// Create a new DefaultEventBus with custom dispatcher
    pub fn with_dispatcher(dispatcher: Box<dyn EventDispatcher>) -> Self {
        Self {
            dispatcher,
            topics: BTreeMap::new(),
            global_handlers: Vec::new(),
            stats: EventBusStats::default(),
            config: EventBusConfig::default(),
        }
    }


    /// Create a new DefaultEventBus with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self {
        Self {
            dispatcher: Box::new(DefaultEventDispatcher::new()),
            topics: BTreeMap::new(),
            global_handlers: Vec::new(),
            stats: EventBusStats::default(),
            config,
        }
    }

    /// Clean up dead references
    fn cleanup_dead_references(&mut self) {
        // Clean up topic subscribers
        for (_, subscribers) in self.topics.iter_mut() {
            subscribers.retain(|sub| sub.strong_count() > 0);
        }

        // Clean up global handlers
        self.global_handlers.retain(|handler| handler.strong_count() > 0);

        // Update active subscribers count
        let total_subscribers: usize = self.topics.values()
            .map(|subs| subs.len())
            .sum::<usize>() + self.global_handlers.len();
        self.stats.active_subscribers.store(total_subscribers, Ordering::SeqCst);
    }

    /// Check if cleanup is needed
    fn should_cleanup(&self) -> bool {
        self.config.auto_cleanup && 
        self.stats.events_published.load(Ordering::SeqCst) % self.config.cleanup_interval == 0
    }

    /// Get or create a topic
    fn get_or_create_topic(&mut self, topic: &str) -> Result<&mut Vec<Weak<dyn EventHandler>>> {
        if self.topics.len() >= self.config.max_topics && !self.topics.contains_key(topic) {
            return Err(nos_api::error::Error::EventError(
                alloc::format!("Maximum number of topics ({}) reached", self.config.max_topics)
            ));
        }

        Ok(self.topics.entry(topic.to_string()).or_insert_with(Vec::new))
    }

    /// Publish event to a specific topic
    fn publish_to_topic(&mut self, topic: &str, event: &dyn Event) -> Result<usize> {
        if let Some(subscribers) = self.topics.get(topic) {
            let mut delivered = 0;
            
            for weak_subscriber in subscribers {
                if let Some(subscriber) = weak_subscriber.upgrade() {
                    if let Err(e) = subscriber.handle(event) {
                        self.stats.events_failed.fetch_add(1, Ordering::SeqCst);
                        #[cfg(feature = "log")]
                        log::error!("Event delivery error to topic '{}': {:?}", topic, e);
                    } else {
                        delivered += 1;
                    }
                }
            }
            
            Ok(delivered)
        } else {
            Ok(0) // No subscribers for this topic
        }
    }

    /// Publish event to global handlers
    fn publish_to_global(&mut self, event: &dyn Event) -> Result<usize> {
        let mut delivered = 0;
        
        for weak_handler in &self.global_handlers {
            if let Some(handler) = weak_handler.upgrade() {
                if let Err(e) = handler.handle(event) {
                    self.stats.events_failed.fetch_add(1, Ordering::SeqCst);
                    #[cfg(feature = "log")]
                    log::error!("Global event handler error: {:?}", e);
                } else {
                    delivered += 1;
                }
            }
        }
        
        Ok(delivered)
    }
}

impl Default for DefaultEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for DefaultEventBus {
    fn publish(&mut self, event: Box<dyn Event>) -> Result<()> {
        // Update statistics
        self.stats.events_published.fetch_add(1, Ordering::SeqCst);

        // Clean up if needed
        if self.should_cleanup() {
            self.cleanup_dead_references();
        }

        // Get event type and topic
        let event_type = event.event_type().name();
        let topic = event_type; // Use event type as topic name

        // Publish to topic subscribers
        let topic_delivered = self.publish_to_topic(topic, &*event)?;
        
        // Publish to global handlers
        let global_delivered = self.publish_to_global(&*event)?;

        // Update processed count
        let total_delivered = topic_delivered + global_delivered;
        self.stats.events_processed.fetch_add(total_delivered, Ordering::SeqCst);

        // Also dispatch through the internal dispatcher
        self.dispatcher.dispatch(event)?;

        Ok(())
    }


    fn subscribe(&mut self, topic: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        let subscribers = self.get_or_create_topic(topic)?;
        
        if subscribers.len() >= self.config.max_subscribers_per_topic {
            return Err(nos_api::error::Error::EventError(
                alloc::format!("Maximum subscribers ({}) reached for topic '{}'", 
                    self.config.max_subscribers_per_topic, topic)
            ));
        }

        subscribers.push(Arc::downgrade(&handler));
        
        // Update active topics count if this is a new topic
        if subscribers.len() == 1 {
            self.stats.active_topics.fetch_add(1, Ordering::SeqCst);
        }

        Ok(())
    }

    fn unsubscribe(&mut self, topic: &str, handler: &Arc<dyn EventHandler>) -> Result<()> {
        if let Some(subscribers) = self.topics.get_mut(topic) {
            let initial_len = subscribers.len();
            subscribers.retain(|weak_sub| {
                if let Some(strong_sub) = weak_sub.upgrade() {
                    !Arc::ptr_eq(&strong_sub, handler)
                } else {
                    false // Remove dead references
                }
            });
            
            // Update active topics count if topic is now empty
            if initial_len > 0 && subscribers.is_empty() {
                self.stats.active_topics.fetch_sub(1, Ordering::SeqCst);
            }
            
            Ok(())
        } else {
            Err(nos_api::error::Error::EventError(
                alloc::format!("Topic '{}' not found", topic)
            ))
        }
    }

    fn subscribe_all(&mut self, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.global_handlers.push(Arc::downgrade(&handler));
        Ok(())
    }

    fn unsubscribe_all(&mut self, handler: &Arc<dyn EventHandler>) -> Result<()> {
        let initial_len = self.global_handlers.len();
        self.global_handlers.retain(|weak_handler| {
            if let Some(strong_handler) = weak_handler.upgrade() {
                !Arc::ptr_eq(&strong_handler, handler)
            } else {
                false // Remove dead references
            }
        });
        
        if initial_len == self.global_handlers.len() {
            Err(nos_api::error::Error::EventError("Handler not found in global subscribers".to_string()))
        } else {
            Ok(())
        }
    }

    fn get_stats(&self) -> &EventBusStats {
        &self.stats
    }
}

/// Batch event bus for collecting and processing events in batches
pub struct BatchEventBus {
    /// Base event bus
    base: DefaultEventBus,
    /// Event batch
    event_batch: Vec<Box<dyn Event>>,
    /// Maximum batch size
    max_batch_size: usize,
    /// Whether to auto-flush when batch is full
    auto_flush: bool,
}

impl BatchEventBus {
    /// Create a new BatchEventBus
    pub fn new(max_batch_size: usize, auto_flush: bool) -> Self {
        Self {
            base: DefaultEventBus::new(),
            event_batch: Vec::with_capacity(max_batch_size),
            max_batch_size,
            auto_flush,
        }
    }

    /// Flush all events in the batch
    pub fn flush(&mut self) -> Result<()> {
        for event in self.event_batch.drain(..) {
            self.base.publish(&*event)?;
        }
        Ok(())
    }

    /// Add an event to the batch
    pub fn add_to_batch(&mut self, event: Box<dyn Event>) -> Result<()> {
        self.event_batch.push(event);
        
        if self.auto_flush && self.event_batch.len() >= self.max_batch_size {
            self.flush()?;
        }
        
        Ok(())
    }

    /// Get current batch size
    pub fn batch_size(&self) -> usize {
        self.event_batch.len()
    }

    /// Check if batch is full
    pub fn is_batch_full(&self) -> bool {
        self.event_batch.len() >= self.max_batch_size
    }
}

impl EventBus for BatchEventBus {
    type Event = dyn Event;

    fn publish(&mut self, event: Box<dyn Event>) -> Result<()> {
        // For batch event bus, we add to batch instead of publishing immediately
        self.add_to_batch(event)
    }



    fn subscribe(&mut self, topic: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.base.subscribe(topic, handler)
    }

    fn unsubscribe(&mut self, topic: &str, handler: &Arc<dyn EventHandler>) -> Result<()> {
        self.base.unsubscribe(topic, handler)
    }

    fn subscribe_all(&mut self, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.base.subscribe_all(handler)
    }

    fn unsubscribe_all(&mut self, handler: &Arc<dyn EventHandler>) -> Result<()> {
        self.base.unsubscribe_all(handler)
    }

    fn get_stats(&self) -> &EventBusStats {
        self.base.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use nos_api::event::{SystemEvent, SystemEventData};

    struct TestHandler {
        call_count: core::sync::atomic::AtomicUsize,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                call_count: core::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(core::sync::atomic::Ordering::SeqCst)
        }
    }

    impl EventHandler for TestHandler {
        fn handle(&self, event: &dyn Event) -> Result<()> {
            self.call_count.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_event_bus() {
        let mut event_bus = DefaultEventBus::new();
        let handler = Arc::new(TestHandler::new());
        
        event_bus.subscribe("system_boot", handler.clone()).unwrap();
        
        let event = SystemEvent {
            metadata: EventMetadata::new("test", EventType::System, EventPriority::Normal),
            data: SystemEventData { stage: "test".to_string() },
        };
        
        event_bus.publish(&event).unwrap();
        assert_eq!(handler.call_count(), 1);
    }

    #[test]
    fn test_batch_event_bus() {
        let mut batch_bus = BatchEventBus::new(3, true);
        let handler = Arc::new(TestHandler::new());
        
        batch_bus.subscribe("system_boot", handler.clone()).unwrap();
        
        let event = SystemEvent {
            metadata: EventMetadata::new("test", EventType::System, EventPriority::Normal),
            data: SystemEventData { stage: "test".to_string() },
        };
        
        // Add events to batch
        batch_bus.publish(&event).unwrap();
        batch_bus.publish(&event).unwrap();
        assert_eq!(handler.call_count(), 0); // Not yet flushed
        
        batch_bus.publish(&event).unwrap(); // Should trigger auto-flush
        assert_eq!(handler.call_count(), 3); // All events should be delivered
    }
}