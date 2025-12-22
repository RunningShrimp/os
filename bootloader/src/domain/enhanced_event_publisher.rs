//! Enhanced Event Publisher - Complete event publishing system
//!
//! Provides a comprehensive event publishing system with:
//! - Advanced subscription management
//! - Event filtering and routing
//! - Event persistence and replay
//! - Performance monitoring
//! - Error handling and recovery

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use alloc::sync::Arc;
use spin::Mutex;

/// Subscription statistics
#[derive(Debug, Clone)]
pub struct SubscriptionStats {
    /// Total subscribers
    pub total_subscribers: usize,
    /// Subscribers by event type
    pub subscribers_by_type: BTreeMap<String, usize>,
    /// Global subscribers
    pub global_subscribers: usize,
    /// Active subscriptions
    pub active_subscriptions: usize,
}

impl Default for SubscriptionStats {
    fn default() -> Self {
        Self {
            total_subscribers: 0,
            subscribers_by_type: BTreeMap::new(),
            global_subscribers: 0,
            active_subscriptions: 0,
        }
    }
}

/// Event subscription manager
pub struct EventSubscriptionManager {
    /// Subscribers by event type
    event_subscribers: BTreeMap<String, Vec<Box<dyn DomainEventSubscriber>>>,
    /// Global subscribers (receive all events)
    global_subscribers: Vec<Box<dyn DomainEventSubscriber>>,
    /// Subscription statistics
    stats: SubscriptionStats,
}

impl EventSubscriptionManager {
    /// Create new subscription manager
    pub fn new() -> Self {
        Self {
            event_subscribers: BTreeMap::new(),
            global_subscribers: Vec::new(),
            stats: SubscriptionStats::default(),
        }
    }
    
    /// Get subscribers for specific event type
    pub fn get_subscribers_for_event(&self, event_type: &'static str) -> Vec<Box<dyn DomainEventSubscriber>> {
        let mut subscribers = Vec::new();
        
        // Add subscribers for this specific event type
        if let Some(event_subs) = self.event_subscribers.get(event_type) {
            subscribers.extend(event_subs.clone());
        }
        
        // Add all global subscribers
        subscribers.extend(self.global_subscribers.clone());
        
        subscribers
    }
    
    /// Add subscription for specific event type
    pub fn add_subscription(&mut self, event_type: String, subscriber: Box<dyn DomainEventSubscriber>) {
        self.event_subscribers
            .entry(event_type.clone())
            .or_insert_with(Vec::new)
            .push(subscriber);
        
        // Update stats
        self.stats.total_subscribers += 1;
        *self.stats.subscribers_by_type.entry(event_type).or_insert(0) += 1;
        self.stats.active_subscriptions += 1;
    }
    
    /// Add global subscription (receives all events)
    pub fn add_global_subscription(&mut self, subscriber: Box<dyn DomainEventSubscriber>) {
        self.global_subscribers.push(subscriber);
        
        // Update stats
        self.stats.total_subscribers += 1;
        self.stats.global_subscribers += 1;
        self.stats.active_subscriptions += 1;
    }
    
    /// Remove subscription by name
    pub fn remove_subscription(&mut self, subscriber_name: &'static str) {
        // Remove from event-specific subscribers
        for (event_type, subscribers) in self.event_subscribers.iter_mut() {
            let initial_len = subscribers.len();
            subscribers.retain(|sub| sub.subscriber_name() != subscriber_name);
            let removed = initial_len - subscribers.len();
            
            if removed > 0 {
                self.stats.total_subscribers -= removed;
                if let Some(count) = self.stats.subscribers_by_type.get_mut(event_type) {
                    *count -= removed;
                    if *count == 0 {
                        self.stats.subscribers_by_type.remove(event_type);
                    }
                }
                self.stats.active_subscriptions -= removed;
            }
        }
        
        // Remove from global subscribers
        let initial_len = self.global_subscribers.len();
        self.global_subscribers.retain(|sub| sub.subscriber_name() != subscriber_name);
        let removed = initial_len - self.global_subscribers.len();
        
        if removed > 0 {
            self.stats.total_subscribers -= removed;
            self.stats.global_subscribers -= removed;
            self.stats.active_subscriptions -= removed;
        }
    }
    
    /// Get all subscribers
    pub fn get_all_subscribers(&self) -> Vec<Box<dyn DomainEventSubscriber>> {
        let mut subscribers = Vec::new();
        
        // Add all event-specific subscribers
        for (_, event_subs) in &self.event_subscribers {
            subscribers.extend(event_subs.clone());
        }
        
        // Add all global subscribers
        subscribers.extend(self.global_subscribers.clone());
        
        subscribers
    }
    
    /// Get subscription statistics
    pub fn get_subscription_stats(&self) -> SubscriptionStats {
        self.stats.clone()
    }
}

use crate::domain::events::{
    DomainEvent, DomainEventSubscriber, EventFilter,
    EventSeverity
};
use crate::domain::event_persistence::{
    EventStoreStats, PersistentEventStore,
    DiagnosticEventReplayer
};

/// Enhanced event publisher with full feature support
///
/// This publisher provides:
/// - Event persistence to memory
/// - Advanced subscription management
/// - Event filtering and routing
/// - Performance monitoring
/// - Error handling and recovery
pub struct EnhancedEventPublisher {
    /// Event store for persistence
    event_store: Arc<Mutex<PersistentEventStore>>,
    /// Subscription manager
    subscription_manager: Arc<Mutex<EventSubscriptionManager>>,
    /// Event replayer for diagnostics
    event_replayer: Arc<Mutex<DiagnosticEventReplayer>>,
    /// Event filters
    filters: Vec<Box<dyn EventFilter>>,
    /// Publisher statistics
    stats: PublisherStats,
    /// Configuration
    config: PublisherConfig,
}

/// Publisher configuration
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// Maximum number of events to store
    max_events: usize,
    /// Enable event persistence
    enable_persistence: bool,
    /// Enable performance monitoring
    enable_monitoring: bool,
    /// Enable error recovery
    enable_recovery: bool,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            max_events: 1000,
            enable_persistence: true,
            enable_monitoring: true,
            enable_recovery: true,
        }
    }
}

/// Publisher statistics
#[derive(Debug, Clone)]
pub struct PublisherStats {
    /// Total events published
    pub total_published: u64,
    /// Total events failed
    pub total_failed: u64,
    /// Total subscribers
    pub total_subscribers: usize,
    /// Events by type
    pub events_by_type: BTreeMap<String, u64>,
    /// Events by severity
    pub events_by_severity: BTreeMap<EventSeverity, u64>,
    /// Average publish time (microseconds)
    pub avg_publish_time_us: u64,
    /// Last error
    pub last_error: Option<String>,
}

impl Default for PublisherStats {
    fn default() -> Self {
        Self {
            total_published: 0,
            total_failed: 0,
            total_subscribers: 0,
            events_by_type: BTreeMap::new(),
            events_by_severity: BTreeMap::new(),
            avg_publish_time_us: 0,
            last_error: None,
        }
    }
}

impl EnhancedEventPublisher {
    /// Create new enhanced event publisher
    ///
    /// # Arguments
    /// * `config` - Publisher configuration
    ///
    /// # Returns
    /// New enhanced event publisher instance
    pub fn new(config: PublisherConfig) -> Self {
        let event_store = Arc::new(Mutex::new(
            PersistentEventStore::new(config.max_events, config.enable_persistence)
        ));
        let subscription_manager = Arc::new(Mutex::new(EventSubscriptionManager::new()));
        let event_replayer = Arc::new(Mutex::new(
            DiagnosticEventReplayer::new(Box::new(PersistentEventStore::new(config.max_events, config.enable_persistence)))
        ));
        
        Self {
            event_store,
            subscription_manager,
            event_replayer,
            filters: Vec::new(),
            stats: PublisherStats::default(),
            config,
        }
    }

    /// Create enhanced event publisher with default configuration
    pub fn with_default_config() -> Self {
        Self::new(PublisherConfig::default())
    }

    /// Add event filter
    ///
    /// # Arguments
    /// * `filter` - The filter to add
    pub fn add_filter(&mut self, filter: Box<dyn EventFilter>) {
        self.filters.push(filter);
    }

    /// Remove all filters
    pub fn clear_filters(&mut self) {
        self.filters.clear();
    }

    /// Publish an event with full processing
    ///
    /// This method:
    /// 1. Applies all filters
    /// 2. Stores the event if persistence is enabled
    /// 3. Notifies all relevant subscribers
    /// 4. Updates statistics
    /// 5. Handles errors according to configuration
    ///
    /// # Arguments
    /// * `event` - The event to publish
    ///
    /// # Returns
    /// Ok(()) if published successfully, Err with error message if failed
    pub fn publish(&mut self, event: Box<dyn DomainEvent>) -> Result<(), &'static str> {
        let start_time = self.get_timestamp();
        
        // Check filters
        for filter in &self.filters {
            if !filter.should_process(event.as_ref()) {
                return Ok(()); // Filtered out, not an error
            }
        }
        
        // Store event if persistence is enabled
        if self.config.enable_persistence {
            let store_result = self.event_store.lock().store_event(event.clone_box());
            if let Err(e) = store_result {
                self.handle_publish_error(e, "Event storage failed");
                return Err(e);
            }
        }
        
        // Get subscribers for this event type
        let subscribers = {
            let manager = self.subscription_manager.lock();
            manager.get_subscribers_for_event(event.event_type())
        };
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        // Notify subscribers
        for subscriber in subscribers {
            match subscriber.handle(event.as_ref()) {
                Ok(()) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    log::error!("Event subscriber '{}' failed: {}", subscriber.subscriber_name(), e);
                    
                    if self.config.enable_recovery {
                        self.handle_subscriber_error(subscriber.subscriber_name(), e);
                    }
                }
            }
        }
        
        // Update statistics
        self.update_publish_stats(event.as_ref(), success_count, error_count, start_time);
        
        if error_count > 0 && !self.config.enable_recovery {
            return Err("Some subscribers failed to handle event");
        }
        
        Ok(())
    }

    /// Subscribe to events
    ///
    /// # Arguments
    /// * `event_type` - The event type to subscribe to (None for all events)
    /// * `subscriber` - The subscriber to add
    ///
    /// # Returns
    /// Ok(()) if subscribed successfully, Err with error message if failed
    pub fn subscribe(&mut self, event_type: Option<&'static str>, subscriber: Box<dyn DomainEventSubscriber>) -> Result<(), &'static str> {
        let mut manager = self.subscription_manager.lock();
        
        if let Some(event_type) = event_type {
            manager.add_subscription(event_type.to_string(), subscriber);
        } else {
            manager.add_global_subscription(subscriber);
        }
        
        // Update subscriber count
        self.stats.total_subscribers = manager.get_all_subscribers().len();
        
        Ok(())
    }

    /// Unsubscribe from events
    ///
    /// # Arguments
    /// * `subscriber_name` - The name of the subscriber to remove
    ///
    /// # Returns
    /// Ok(()) if unsubscribed successfully, Err with error message if failed
    pub fn unsubscribe(&mut self, subscriber_name: &'static str) -> Result<(), &'static str> {
        let mut manager = self.subscription_manager.lock();
        manager.remove_subscription(subscriber_name);
        
        // Update subscriber count
        self.stats.total_subscribers = manager.get_all_subscribers().len();
        
        Ok(())
    }

    /// Get event history
    ///
    /// # Returns
    /// Slice of historical events
    pub fn get_event_history(&self) -> Vec<Box<dyn DomainEvent>> {
        if self.config.enable_persistence {
            self.event_store.lock().get_all_events(None)
        } else {
            Vec::new()
        }
    }

    /// Clear event history
    pub fn clear_history(&mut self) {
        if self.config.enable_persistence {
            self.event_store.lock().clear_events();
        }
        
        // Reset relevant statistics
        self.stats.total_published = 0;
        self.stats.total_failed = 0;
        self.stats.events_by_type.clear();
        self.stats.events_by_severity.clear();
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.stats.total_subscribers
    }

    /// Get published event count
    pub fn event_count(&self) -> usize {
        if self.config.enable_persistence {
            self.event_store.lock().event_count()
        } else {
            0
        }
    }

    /// Get publisher statistics
    pub fn get_publisher_stats(&self) -> PublisherStats {
        self.stats.clone()
    }

    /// Get event store statistics
    pub fn get_store_stats(&self) -> Option<EventStoreStats> {
        if self.config.enable_persistence {
            Some(self.event_store.lock().get_store_stats())
        } else {
            None
        }
    }

    /// Replay events for diagnostics
    ///
    /// # Arguments
    /// * `subscriber` - The subscriber to notify with replayed events
    /// * `filter` - Optional replay filter
    ///
    /// # Returns
    /// Ok(()) if replay completed successfully, Err with error message if failed
    pub fn replay_events(&mut self, subscriber: &dyn DomainEventSubscriber, filter: Option<crate::domain::event_persistence::ReplayFilter>) -> Result<(), &'static str> {
        let mut replayer = self.event_replayer.lock();
        
        // Add filter if provided
        if let Some(filter) = filter {
            replayer.add_filter(filter);
        }
        
        // Replay events
        match replayer.replay_to_subscriber(subscriber, Some(100)) {
            Ok(()) => {
                log::info!("Event replay completed successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Event replay failed: {}", e);
                Err(e)
            }
        }
    }

    /// Get subscription statistics
    pub fn get_subscription_stats(&self) -> SubscriptionStats {
        let manager = self.subscription_manager.lock();
        manager.get_subscription_stats()
    }

    /// Handle publish error
    fn handle_publish_error(&mut self, error: &'static str, context: &'static str) {
        self.stats.total_failed += 1;
        self.stats.last_error = Some(format!("{}: {}", context, error));
        
        if self.config.enable_recovery {
            log::warn!("Publish error recovery: {}", error);
            // In a real implementation, this might trigger recovery procedures
        }
    }

    /// Handle subscriber error
    fn handle_subscriber_error(&mut self, subscriber_name: &'static str, error: &'static str) {
        log::error!("Subscriber error recovery for '{}': {}", subscriber_name, error);
        
        if self.config.enable_recovery {
            // In a real implementation, this might:
            // 1. Remove problematic subscriber
            // 2. Retry with a different subscriber
            // 3. Fall back to default behavior
            self.unsubscribe(subscriber_name).ok();
        }
    }

    /// Update publish statistics
    fn update_publish_stats(&mut self, event: &dyn DomainEvent, success_count: usize, error_count: usize, start_time: u64) {
        let end_time = self.get_timestamp();
        let duration_us = end_time.saturating_sub(start_time);
        
        // Update total counts
        self.stats.total_published += 1;
        
        // Update type statistics
        let event_type = event.event_type().to_string();
        *self.stats.events_by_type.entry(event_type).or_insert(0) += 1;
        
        // Update severity statistics
        let severity = event.severity();
        *self.stats.events_by_severity.entry(severity).or_insert(0) += 1;
        
        // Update average publish time
        if self.stats.total_published > 1 {
            self.stats.avg_publish_time_us = 
                (self.stats.avg_publish_time_us * (self.stats.total_published - 1) + duration_us) / self.stats.total_published;
        } else {
            self.stats.avg_publish_time_us = duration_us;
        }
        
        // Log performance if enabled
        if self.config.enable_monitoring {
            if error_count > 0 {
                log::warn!("Event publish had {} errors out of {} subscribers", 
                         error_count, success_count + error_count);
            }
            
            if duration_us > 1000 { // > 1ms is slow
                log::warn!("Slow event publish: {}μs for type {}", 
                         duration_us, event.event_type());
            }
        }
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would use a proper timer
        // For now, use a simple counter
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Create diagnostic report
    pub fn create_diagnostic_report(&self) -> DiagnosticReport {
        let store_stats = self.get_store_stats();
        let subscription_stats = self.get_subscription_stats();
        let replay_stats = {
            let replayer = self.event_replayer.lock();
            replayer.get_replay_stats()
        };
        
        DiagnosticReport {
            publisher_stats: self.stats.clone(),
            store_stats,
            subscription_stats,
            replay_stats,
            config: self.config.clone(),
        }
    }
}

/// Diagnostic report
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// Publisher statistics
    pub publisher_stats: PublisherStats,
    /// Event store statistics
    pub store_stats: Option<EventStoreStats>,
    /// Subscription statistics
    pub subscription_stats: SubscriptionStats,
    /// Replay statistics
    pub replay_stats: crate::domain::event_persistence::ReplayStats,
    /// Configuration
    pub config: PublisherConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::*;

    #[test]
    fn test_enhanced_publisher_creation() {
        let publisher = EnhancedEventPublisher::with_default_config();
        assert_eq!(publisher.subscriber_count(), 0);
        assert_eq!(publisher.event_count(), 0);
    }

    #[test]
    fn test_event_publishing() {
        let mut publisher = EnhancedEventPublisher::with_default_config();
        
        // Create test subscriber
        struct TestSubscriber {
            events_received: Vec<String>,
        }
        
        impl DomainEventSubscriber for TestSubscriber {
            fn handle(&mut self, event: &dyn DomainEvent) -> Result<(), &'static str> {
                self.events_received.push(event.event_type().to_string());
                Ok(())
            }
            
            fn subscriber_name(&self) -> &'static str {
                "test_subscriber"
            }
            
            fn clone_box(&self) -> Box<dyn DomainEventSubscriber> {
                Box::new(Self {
                    events_received: self.events_received.clone(),
                })
            }
        }
        
        
        // Subscribe and publish
        let subscriber_id = publisher.subscribe(Some("BootPhaseCompleted"), Box::new(test_subscriber)).unwrap();
        
        let event = Box::new(BootPhaseCompletedEvent::new("test", 1000, true));
        assert!(publisher.publish(event).is_ok());
        
        // Get subscriber back to check received events
        let subscriber = publisher.get_subscriber(subscriber_id).unwrap();
        let test_subscriber = subscriber.downcast_ref::<TestSubscriber>().unwrap();
        
        // Check subscriber received event
        assert_eq!(test_subscriber.events_received.len(), 1);
        assert_eq!(test_subscriber.events_received[0], "BootPhaseCompleted");
    }

    #[test]
    fn test_event_filtering() {
        let mut publisher = EnhancedEventPublisher::with_default_config();
        
        // Add filter that only allows boot events
        let filter = crate::domain::events::SimpleEventFilter::new(vec!["BootPhaseCompleted"]);
        publisher.add_filter(Box::new(filter));
        
        struct TestSubscriber {
            events_received: Vec<String>,
        }
        
        impl DomainEventSubscriber for TestSubscriber {
            fn handle(&mut self, event: &dyn DomainEvent) -> Result<(), &'static str> {
                self.events_received.push(event.event_type().to_string());
                Ok(())
            }
            
            fn subscriber_name(&self) -> &'static str {
                "test_subscriber"
            }
            
            fn clone_box(&self) -> Box<dyn DomainEventSubscriber> {
                Box::new(Self {
                    events_received: self.events_received.clone(),
                })
            }
        }
        
        assert!(publisher.subscribe(None, Box::new(test_subscriber)).is_ok());
        
        // Publish boot event - should pass through
        let boot_event = Box::new(BootPhaseCompletedEvent::new("test", 1000, true));
        assert!(publisher.publish(boot_event).is_ok());
        
        // Publish graphics event - should be filtered out
        let gfx_event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 2000));
        assert!(publisher.publish(gfx_event).is_ok());
        
        // Should only have received the boot event
        assert_eq!(test_subscriber.events_received.len(), 1);
        assert_eq!(test_subscriber.events_received[0], "BootPhaseCompleted");
    }

    #[test]
    fn test_persistence_integration() {
        let mut publisher = EnhancedEventPublisher::with_default_config();
        
        let event = Box::new(BootPhaseCompletedEvent::new("test", 1000, true));
        assert!(publisher.publish(event).is_ok());
        
        // Check event was stored
        assert_eq!(publisher.event_count(), 1);
        
        let events = publisher.get_event_history();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "BootPhaseCompleted");
    }

    #[test]
    fn test_statistics() {
        let mut publisher = EnhancedEventPublisher::with_default_config();
        
        let stats = publisher.get_publisher_stats();
        assert_eq!(stats.total_published, 0);
        assert_eq!(stats.total_failed, 0);
        
        // Publish some events
        let event = Box::new(BootPhaseCompletedEvent::new("test", 1000, true));
        assert!(publisher.publish(event).is_ok());
        
        let updated_stats = publisher.get_publisher_stats();
        assert_eq!(updated_stats.total_published, 1);
        assert_eq!(updated_stats.total_failed, 0);
    }
}