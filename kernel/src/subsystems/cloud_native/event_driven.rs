//! Event-Driven Architecture
//!
//! This module implements event-driven architecture for cloud-native:
//! - Event bus
//! - Event sourcing
//! - Command/Query Separation (CQRS)
//!
//! Features:
//! - Publish-subscribe pattern
//! - Event replay
//! - Event persistence
//! - Async event processing

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;

// ============================================================================
// Event-Driven Constants
// ============================================================================

/// Maximum event types
pub const MAX_EVENT_TYPES: usize = 1 << 10;

/// Maximum events
pub const MAX_EVENTS: usize = 1 << 16;

/// Maximum subscribers
pub const MAX_SUBSCRIBERS: usize = 1 << 8;

// ============================================================================
// Event Types
// ============================================================================

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// Service lifecycle events
    ServiceCreated,
    ServiceUpdated,
    ServiceDeleted,
    
    /// Configuration events
    ConfigChanged,
    ConfigRolledBack,
    
    /// Scaling events
    ScaleUp,
    ScaleDown,
    
    /// Health events
    HealthCheckFailed,
    HealthCheckPassed,
    
    /// Custom event
    Custom(String),
}

/// Event priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPriority {
    High,
    Normal,
    Low,
}

// ============================================================================
// Event
// ============================================================================

/// Cloud-native event
#[derive(Debug, Clone)]
pub struct CloudEvent {
    pub event_id: String,
    pub event_type: EventType,
    pub source: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub priority: EventPriority,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl CloudEvent {
    pub fn new(event_type: EventType, source: String, data: Vec<u8>) -> Self {
        Self {
            event_id: { let mut s = alloc::string::String::from("evt-"); s.push_str(&generate_event_id(.to_string()); s }),
            event_type,
            source,
            data,
            timestamp: crate::subsystems::time::timestamp_nanos(),
            priority: EventPriority::Normal,
            correlation_id: None,
            causation_id: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_correlation(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// Simple event ID generator
fn generate_event_id() -> u64 {
    static EVENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
    EVENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ============================================================================
// Event Subscriber
// ============================================================================

/// Event handler function
pub type EventHandler = fn(&CloudEvent) -> Result<(), String>;

/// Event subscriber
#[derive(Debug, Clone)]
pub struct EventSubscriber {
    pub id: String,
    pub subscriber_name: String,
    pub event_types: Vec<EventType>,
    pub handler: EventHandler,
    pub enabled: bool,
    pub processed_count: AtomicU64,
}

impl EventSubscriber {
    pub fn new(name: String, event_types: Vec<EventType>, handler: EventHandler) -> Self {
        Self {
            id: { let mut s = alloc::string::String::from("sub-"); s.push_str(&generate_event_id(.to_string()); s }),
            subscriber_name: name,
            event_types,
            handler,
            enabled: true,
            processed_count: AtomicU64::new(0),
        }
    }
}

// ============================================================================
// Event Bus
// ============================================================================

/// Event bus (publish-subscribe)
pub struct EventBus {
    pub subscribers: Mutex<BTreeMap<String, Vec<Arc<EventSubscriber>>>>,
    pub event_log: Mutex<Vec<Arc<CloudEvent>>>,
    pub next_event_id: AtomicU64,
    pub stats: Mutex<EventBusStats>,
}

#[derive(Debug, Clone, Copy)]
pub struct EventBusStats {
    pub total_events: u64,
    pub total_subscribers: usize,
    pub active_subscribers: usize,
    pub processing_errors: u64,
}

impl Default for EventBusStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            total_subscribers: 0,
            active_subscribers: 0,
            processing_errors: 0,
        }
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(BTreeMap::new()),
            event_log: Mutex::new(Vec::new()),
            next_event_id: AtomicU64::new(1),
            stats: Mutex::new(EventBusStats::default()),
        }
    }

    pub fn subscribe(&self, subscriber: Arc<EventSubscriber>) -> Result<(), String> {
        let mut subscribers = self.subscribers.lock();
        let subscriber_id = subscriber.id.clone();

        for event_type in &subscriber.event_types {
            subscribers.entry(event_type_to_string(event_type))
                .or_insert_with(Vec::new)
                .push(subscriber.clone());
        }

        crate::println!("[event_bus] Subscribed {} to {:?}", subscriber.subscriber_name, subscriber.event_types);
        Ok(())
    }

    pub fn publish(&self, event: Arc<CloudEvent>) -> Result<(), String> {
        self.stats.lock().total_events.fetch_add(1, Ordering::Relaxed);

        let subscribers = self.subscribers.lock();
        let key = event_type_to_string(&event.event_type);
        
        if let Some(sub_list) = subscribers.get(&key) {
            for subscriber in sub_list {
                if subscriber.enabled {
                    match (subscriber.handler)(&event) {
                        Ok(_) => {
                            subscriber.processed_count.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            crate::println!("[event_bus] Handler error in {}: {}", subscriber.subscriber_name, e);
                            self.stats.lock().processing_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        }

        crate::println!("[event_bus] Published event {:?} from {}", event.event_type, event.source);
        Ok(())
    }

    pub fn unsubscribe(&self, subscriber_id: String) -> Result<(), String> {
        let mut subscribers = self.subscribers.lock();
        for (_, sub_list) in subscribers.iter_mut() {
            sub_list.retain(|s| s.id != subscriber_id);
        }
        crate::println!("[event_bus] Unsubscribed {}", subscriber_id);
        Ok(())
    }

    pub fn replay_events(&self, from_timestamp: u64) -> Vec<Arc<CloudEvent>> {
        let log = self.event_log.lock();
        log.iter().filter(|e| e.timestamp >= from_timestamp).cloned().collect()
    }

    pub fn get_stats(&self) -> EventBusStats {
        let mut stats = self.stats.lock();
        stats.total_subscribers = self.subscribers.lock().values().map(|v| v.len()).sum();
        stats.active_subscribers = self.subscribers.lock().values()
            .flat_map(|v| v.iter())
            .filter(|s| s.enabled)
            .count();
        *stats
    }
}

fn event_type_to_string(event_type: &EventType) -> String {
    match event_type {
        EventType::ServiceCreated => "ServiceCreated".to_string(),
        EventType::ServiceUpdated => "ServiceUpdated".to_string(),
        EventType::ServiceDeleted => "ServiceDeleted".to_string(),
        EventType::ConfigChanged => "ConfigChanged".to_string(),
        EventType::ConfigRolledBack => "ConfigRolledBack".to_string(),
        EventType::ScaleUp => "ScaleUp".to_string(),
        EventType::ScaleDown => "ScaleDown".to_string(),
        EventType::HealthCheckFailed => "HealthCheckFailed".to_string(),
        EventType::HealthCheckPassed => "HealthCheckPassed".to_string(),
        EventType::Custom(s) => s.clone(),
    }
}

// ============================================================================
// Event Sourcing
// ============================================================================

/// Event store (for event sourcing)
pub struct EventStore {
    pub events: Mutex<BTreeMap<String, Vec<Arc<CloudEvent>>>>,
    pub snapshots: Mutex<BTreeMap<String, CloudEvent>>,
    pub next_event_id: AtomicU64,
}

impl EventStore {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(BTreeMap::new()),
            snapshots: Mutex::new(BTreeMap::new()),
            next_event_id: AtomicU64::new(1),
        }
    }

    pub fn append_event(&self, aggregate_id: String, event: Arc<CloudEvent>) -> Result<(), String> {
        let mut events = self.events.lock();
        events.entry(aggregate_id)
            .or_insert_with(Vec::new)
            .push(event);
        crate::println!("[event_store] Appended event to aggregate {}", aggregate_id);
        Ok(())
    }

    pub fn get_events(&self, aggregate_id: String) -> Vec<Arc<CloudEvent>> {
        let events = self.events.lock();
        events.get(&aggregate_id).cloned().unwrap_or_default()
    }

    pub fn create_snapshot(&self, aggregate_id: String, event: CloudEvent) {
        let mut snapshots = self.snapshots.lock();
        snapshots.insert(aggregate_id, event);
        crate::println!("[event_store] Created snapshot for aggregate {}", aggregate_id);
    }

    pub fn get_snapshot(&self, aggregate_id: String) -> Option<CloudEvent> {
        let snapshots = self.snapshots.lock();
        snapshots.get(&aggregate_id).cloned()
    }
}
