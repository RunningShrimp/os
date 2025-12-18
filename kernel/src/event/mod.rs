//! Event System Implementation
//! 
//! This module provides the kernel's event system implementation,
//! including event dispatchers, event bus, and integration with other modules.

pub mod dispatcher;
pub mod bus;

use alloc::{
    sync::Arc,
    collections::BTreeMap,
    string::String,
};
use core::sync::atomic::{AtomicUsize, Ordering};

use nos_api::{
    event::{
        Event, EventHandler, EventDispatcher, EventBus, EventFilter, EventId,
        EventType, EventMetadata, EventPriority,
        SystemEvent, MemoryEvent, ProcessEvent, FileSystemEvent,
        NetworkEvent, SecurityEvent, HardwareEvent, UserEvent,
    },
    Result,
};

use dispatcher::PriorityEventDispatcher;
use bus::{DefaultEventBus, EventBusConfig, EventBusStats};

/// Global event system instance
static mut GLOBAL_EVENT_SYSTEM: Option<EventSystem> = None;
static GLOBAL_EVENT_SYSTEM_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Main event system that combines dispatcher and bus functionality
pub struct EventSystem {
    /// Event bus for publishing and subscribing
    bus: DefaultEventBus,
    /// Priority dispatcher for critical events
    priority_dispatcher: PriorityEventDispatcher,
    /// Event filters
    filters: BTreeMap<String, Vec<Arc<dyn EventFilter>>>,
    /// Event statistics
    stats: EventSystemStats,
}


/// Statistics for the entire event system
#[derive(Debug, Default)]
pub struct EventSystemStats {
    /// Total events created
    pub events_created: AtomicUsize,
    /// Total events dispatched
    pub events_dispatched: AtomicUsize,
    /// Total filters applied
    pub filters_applied: AtomicUsize,
    /// System uptime in milliseconds
    pub uptime_ms: AtomicUsize,
}

impl EventSystem {
    /// Create a new event system
    pub fn new() -> Self {
        Self {
            bus: DefaultEventBus::with_priority_dispatcher(),
            priority_dispatcher: PriorityEventDispatcher::new(),
            filters: BTreeMap::new(),
            stats: EventSystemStats::default(),
        }
    }

    /// Create a new event system with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self {
        Self {
            bus: DefaultEventBus::with_config(config),
            priority_dispatcher: PriorityEventDispatcher::new(),
            filters: BTreeMap::new(),
            stats: EventSystemStats::default(),
        }
    }

    /// Initialize the global event system
    pub fn init_global() -> Result<()> {
        if GLOBAL_EVENT_SYSTEM_INIT.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
            unsafe {
                GLOBAL_EVENT_SYSTEM = Some(EventSystem::new());
            }
            Ok(())
        } else {
            Err(nos_api::error::Error::EventError("Event system already initialized".to_string()))
        }
    }

    /// Get the global event system
    pub fn global() -> &'static mut EventSystem {
        unsafe {
            GLOBAL_EVENT_SYSTEM.as_mut().expect("Event system not initialized")
        }
    }

    /// Check if the global event system is initialized
    pub fn is_global_initialized() -> bool {
        GLOBAL_EVENT_SYSTEM_INIT.load(Ordering::SeqCst)
    }

    /// Publish an event
    pub fn publish(&mut self, event: Box<dyn Event>) -> Result<()> {
        self.stats.events_created.fetch_add(1, Ordering::SeqCst);
        
        // Route to appropriate dispatcher based on priority
        let priority = event.metadata().priority;
        
        match priority {
            EventPriority::Critical => {
                self.priority_dispatcher.dispatch(&*event)?;
            }
            _ => {
                self.bus.publish(&*event)?;
            }
        }
        
        self.stats.events_dispatched.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Subscribe to events of a specific type
    pub fn subscribe(&mut self, event_type: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.bus.subscribe(event_type, handler)
    }

    /// Subscribe to all events
    pub fn subscribe_all(&mut self, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.bus.subscribe_all(handler)
    }

    /// Unsubscribe from events of a specific type
    pub fn unsubscribe(&mut self, event_type: &str, handler: &Arc<dyn EventHandler>) -> Result<()> {
        self.bus.unsubscribe(event_type, handler)
    }

    /// Unsubscribe from all events
    pub fn unsubscribe_all(&mut self, handler: &Arc<dyn EventHandler>) -> Result<()> {
        self.bus.unsubscribe_all(handler)
    }

    /// Add an event filter
    pub fn add_filter(&mut self, event_type: String, filter: Arc<dyn EventFilter>) -> Result<()> {
        let filters = self.filters.entry(event_type).or_insert_with(Vec::new);
        filters.push(filter);
        Ok(())
    }

    /// Remove an event filter
    pub fn remove_filter(&mut self, event_type: &str, filter_id: usize) -> Result<()> {
        if let Some(filters) = self.filters.get_mut(event_type) {
            if filter_id < filters.len() {
                filters.remove(filter_id);
                Ok(())
            } else {
                Err(nos_api::error::Error::EventError(
                    alloc::format!("Invalid filter ID: {} for event type: {}", filter_id, event_type)
                ))
            }
        } else {
            Err(nos_api::error::Error::EventError(
                alloc::format!("No filters registered for event type: {}", event_type)
            ))
        }
    }

    /// Get event system statistics
    pub fn get_stats(&self) -> &EventSystemStats {
        &self.stats
    }

    /// Get event bus statistics
    pub fn get_bus_stats(&self) -> &EventBusStats {
        self.bus.get_stats()
    }
}

impl Default for EventSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common event types
pub mod events {
    use super::*;
    use nos_api::event::{SystemEventData, MemoryEventData, ProcessEventData};

    /// Create a system boot event
    pub fn system_boot_event(stage: String) -> Box<dyn Event> {
        Box::new(SystemEvent {
            metadata: EventMetadata::new("kernel", EventType::System, EventPriority::High)
                .with_tag("boot"),
            data: SystemEventData { stage },
        })
    }

    /// Create a system shutdown event
    pub fn system_shutdown_event(reason: String) -> Box<dyn Event> {
        Box::new(SystemEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: 0,
                source: "kernel".to_string(),
                category: EventType::System,
                priority: EventPriority::Critical,
                tags: alloc::vec!["shutdown".to_string()],
            },
            data: SystemEventData::Shutdown { reason },
        })
    }

    /// Create a memory allocation event
    pub fn memory_allocation_event(size: usize, success: bool) -> Box<dyn Event> {
        Box::new(MemoryEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: 0,
                source: "memory_manager".to_string(),
                category: EventType::Memory,
                priority: if success { EventPriority::Normal } else { EventPriority::High },
                tags: alloc::vec!["allocation".to_string()],
            },
            data: MemoryEventData::Allocation { size, success },
        })
    }

    /// Create a process creation event
    pub fn process_created_event(pid: u32, ppid: u32, name: String) -> Box<dyn Event> {
        Box::new(ProcessEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: 0,
                source: "process_manager".to_string(),
                category: EventType::Process,
                priority: EventPriority::Normal,
                tags: alloc::vec!["created".to_string()],
            },
            data: ProcessEventData::Created { pid, ppid, name },
        })
    }
}

/// Initialize the event system
pub fn init() -> Result<()> {
    EventSystem::init_global()
}

/// Publish an event using the global event system
pub fn publish(event: Box<dyn Event>) -> Result<()> {
    EventSystem::global().publish(event)
}

/// Subscribe to events using the global event system
pub fn subscribe(event_type: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
    EventSystem::global().subscribe(event_type, handler)
}

/// Subscribe to all events using the global event system
pub fn subscribe_all(handler: Arc<dyn EventHandler>) -> Result<()> {
    EventSystem::global().subscribe_all(handler)
}

/// Unsubscribe from events of a specific type
pub fn unsubscribe(event_type: &str, handler: &Arc<dyn EventHandler>) -> Result<()> {
    EventSystem::global().unsubscribe(event_type, handler)
}

/// Unsubscribe from all events
pub fn unsubscribe_all(handler: &Arc<dyn EventHandler>) -> Result<()> {
    EventSystem::global().unsubscribe_all(handler)
}

/// Get event system statistics
pub fn get_stats() -> &'static EventSystemStats {
    EventSystem::global().get_stats()
}

/// Get event bus statistics
pub fn get_bus_stats() -> &'static EventBusStats {
    EventSystem::global().get_bus_stats()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;

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
        type Event = dyn Event;

        fn handle(&self, _event: &Self::Event) -> Result<()> {
            self.call_count.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_event_system() {
        // Initialize event system
        init().unwrap();
        
        let handler = Arc::new(TestHandler::new());
        subscribe("system_boot", handler.clone()).unwrap();
        
        let event = events::system_boot_event("test".to_string());
        publish(event).unwrap();
        
        assert_eq!(handler.call_count(), 1);
    }
}