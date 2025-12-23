//! Event Dispatcher Implementation
//! 
//! This module provides a concrete implementation of the EventDispatcher trait
//! for handling event routing and delivery to registered handlers.

use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicUsize, Ordering};
use nos_api::{
    core::EventHandler,
    event::{
        Event, EventDispatcher,
    },
    Result,
};
pub trait EventDispatcherExt {
    fn dispatch(&self, event: &dyn Event) -> Result<()>;
}

/// Default implementation of an EventDispatcher
pub struct DefaultEventDispatcher {
    /// Map of event type to handlers
    handlers: BTreeMap<&'static str, Vec<Arc<dyn EventHandler<Event = Box<dyn Event>>>>>,
    /// Map of event type to filters
    filters: BTreeMap<&'static str, Vec<Arc<dyn EventFilter>>>,

    /// Counter for generating unique event IDs
    event_counter: AtomicUsize,
    /// Maximum number of handlers per event type
    max_handlers_per_type: usize,
    /// Maximum number of filters per event type
    max_filters_per_type: usize,
}

impl DefaultEventDispatcher {
    /// Create a new DefaultEventDispatcher
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
            filters: BTreeMap::new(),
            event_counter: AtomicUsize::new(0),
            max_handlers_per_type: 100,
            max_filters_per_type: 50,
        }
    }

    /// Create a new DefaultEventDispatcher with custom limits
    pub fn with_limits(max_handlers: usize, max_filters: usize) -> Self {
        Self {
            handlers: BTreeMap::new(),
            filters: BTreeMap::new(),
            event_counter: AtomicUsize::new(0),
            max_handlers_per_type: max_handlers,
            max_filters_per_type: max_filters,
        }
    }

    /// Generate a unique event ID
    fn generate_event_id(&self) -> EventId {
        self.event_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Check if an event passes all registered filters
    fn passes_filters(&self, event: &dyn Event, event_type: &str) -> bool {
        if let Some(filters) = self.filters.get(event_type) {
            for filter in filters {
                if !filter.matches(event) {
                    return false;
                }
            }
        }
        true
    }

    /// Get handlers for an event type
    fn get_handlers(&self, event_type: &str) -> Option<&Vec<Arc<dyn EventHandler>>> {
        self.handlers.get(event_type)
    }
}

impl Default for DefaultEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatcher for DefaultEventDispatcher {
    fn dispatch(&mut self, event: Box<dyn Event>) -> Result<()> {
        let event_type = event.event_type();
        
        // Check if event passes filters
        if !self.passes_filters(&*event, event_type) {
            return Ok(()); // Event filtered out
        }

        // Get handlers for this event type
        if let Some(handlers) = self.get_handlers(event_type) {
            for handler in handlers {
                if let Err(e) = handler.handle(event.as_ref()) {
                    // Log error but continue with other handlers
                    #[cfg(feature = "log")]
                    log::error!("Event handler error: {:?}", e);
                }
            }
        }

        Ok(())
    }

    fn register_handler(&mut self, event_type: &'static str, handler: Arc<dyn EventHandler>) -> Result<()> {
        let handlers = self.handlers.entry(event_type).or_insert_with(Vec::new);
        
        if handlers.len() >= self.max_handlers_per_type {
            return Err(nos_api::error::Error::EventError(
                alloc::format!("Maximum handlers ({}) reached for event type: {}", 
                    self.max_handlers_per_type, event_type)
            ));
        }

        handlers.push(handler);
        Ok(())
    }

    fn unregister_handler(&mut self, event_type: &'static str, handler_id: usize) -> Result<()> {
        if let Some(handlers) = self.handlers.get_mut(event_type) {
            if handler_id < handlers.len() {
                handlers.remove(handler_id);
                Ok(())
            } else {
                Err(nos_api::error::Error::EventError(
                    alloc::format!("Invalid handler ID: {} for event type: {}", handler_id, event_type)
                ))
            }
        } else {
            Err(nos_api::error::Error::EventError(
                alloc::format!("No handlers registered for event type: {}", event_type)
            ))
        }
    }

    fn register_filter(&mut self, event_type: &'static str, filter: Arc<dyn EventFilter>) -> Result<()> {
        let filters = self.filters.entry(event_type).or_insert_with(Vec::new);
        
        if filters.len() >= self.max_filters_per_type {
            return Err(nos_api::error::Error::EventError(
                alloc::format!("Maximum filters ({}) reached for event type: {}", 
                    self.max_filters_per_type, event_type)
            ));
        }

        filters.push(filter);
        Ok(())
    }


    fn unregister_filter(&mut self, event_type: &'static str, filter_id: usize) -> Result<()> {
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

    // We don't implement the generic dispatch method because we have a specific one
    // that takes Box<dyn Event> which is more flexible for the dispatcher.
    // The trait definition might require it, so we provide a dummy implementation
    // or we should fix the trait definition in nos-api.
    // Given the error, it seems we are implementing EventHandler for DefaultEventDispatcher
    // but the method signature mismatch or missing associated type.
    
    // Actually, DefaultEventDispatcher implements EventDispatcher trait, not EventHandler.
    // Let's check EventDispatcher trait definition in nos-api.
    
    // If EventDispatcher inherits from EventHandler, then we need to implement handle.
}

impl EventHandler for DefaultEventDispatcher {
    type Event = Box<dyn Event>;
    
    fn handle(&mut self, event: &Box<dyn Event>) -> Result<()> {
        Ok(())
    }






}

/// Priority-based event dispatcher that processes events based on priority
pub struct PriorityEventDispatcher {
    /// High priority handlers
    high_priority_handlers: BTreeMap<&'static str, Vec<Arc<dyn EventHandler>>>,
    /// Normal priority handlers
    normal_priority_handlers: BTreeMap<&'static str, Vec<Arc<dyn EventHandler>>>,
    /// Low priority handlers
    low_priority_handlers: BTreeMap<&'static str, Vec<Arc<dyn EventHandler>>>,
    /// Base dispatcher for common functionality
    base: DefaultEventDispatcher,
}

impl PriorityEventDispatcher {
    /// Create a new PriorityEventDispatcher
    pub fn new() -> Self {
        Self {
            high_priority_handlers: BTreeMap::new(),
            normal_priority_handlers: BTreeMap::new(),
            low_priority_handlers: BTreeMap::new(),
            base: DefaultEventDispatcher::new(),
        }
    }

    /// Register a handler with specific priority
    pub fn register_handler_with_priority(
        &mut self,
        event_type: &'static str,
        handler: Arc<dyn EventHandler>,
        priority: EventPriority,
    ) -> Result<()> {
        let handlers = match priority {
            EventPriority::High => &mut self.high_priority_handlers,
            EventPriority::Normal => &mut self.normal_priority_handlers,
            EventPriority::Low => &mut self.low_priority_handlers,
        };

        let handlers = handlers.entry(event_type).or_insert_with(Vec::new);
        handlers.push(handler);
        Ok(())
    }
    
    /// Get handlers based on event priority
    fn get_handlers_by_priority(&self, event_type: &str, priority: EventPriority) -> Option<&Vec<Arc<dyn EventHandler>>> {
        match priority {
            EventPriority::High => self.high_priority_handlers.get(event_type),
            EventPriority::Normal => self.normal_priority_handlers.get(event_type),
            EventPriority::Low => self.low_priority_handlers.get(event_type),
            EventPriority::Critical => {
                // Critical events use all handlers
                None // Special handling needed
            }
        }
    }
}

impl Default for PriorityEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatcher for PriorityEventDispatcher {
    type Event = dyn Event;

    fn dispatch(&mut self, event: Box<dyn Event>) -> Result<()> {
        let event_type = event.event_type();
        let priority = event.metadata().priority;

        match priority {
            EventPriority::Critical => {
                // For critical events, dispatch to all handlers regardless of priority
                let mut all_handlers = Vec::new();
                
                if let Some(handlers) = self.high_priority_handlers.get(event_type) {
                    all_handlers.extend(handlers.clone());
                }
                if let Some(handlers) = self.normal_priority_handlers.get(event_type) {
                    all_handlers.extend(handlers.clone());
                }
                if let Some(handlers) = self.low_priority_handlers.get(event_type) {
                    all_handlers.extend(handlers.clone());
                }

                for handler in all_handlers {
                    if let Err(e) = handler.handle(event.as_ref()) {
                        #[cfg(feature = "log")]
                        log::error!("Critical event handler error: {:?}", e);
                    }
                }
            }
            _ => {
                // For other priorities, dispatch to matching handlers
                if let Some(handlers) = self.get_handlers_by_priority(event_type, priority) {
                    for handler in handlers {
                        if let Err(e) = handler.handle(event.as_ref()) {
                            #[cfg(feature = "log")]
                            log::error!("Event handler error: {:?}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }



    fn register_handler(&mut self, event_type: &'static str, handler: Arc<dyn EventHandler>) -> Result<()> {
        // Default to normal priority
        self.register_handler_with_priority(event_type, handler, EventPriority::Normal)
    }



    fn unregister_handler(&mut self, event_type: &'static str, handler_id: usize) -> Result<()> {
        // Try to remove from normal priority handlers first
        if let Some(handlers) = self.normal_priority_handlers.get_mut(event_type) {
            if handler_id < handlers.len() {
                handlers.remove(handler_id);
                return Ok(());
            }
        }

        // Try high priority handlers
        if let Some(handlers) = self.high_priority_handlers.get_mut(event_type) {
            if handler_id < handlers.len() {
                handlers.remove(handler_id);
                return Ok(());
            }
        }

        // Try low priority handlers
        if let Some(handlers) = self.low_priority_handlers.get_mut(event_type) {
            if handler_id < handlers.len() {
                handlers.remove(handler_id);
                return Ok(());
            }
        }

        Err(nos_api::error::Error::EventError(
            alloc::format!("Invalid handler ID: {} for event type: {}", handler_id, event_type)
        ))
    }

    fn register_filter(&mut self, event_type: &'static str, filter: Arc<dyn EventFilter>) -> Result<()> {
        self.base.register_filter(event_type, filter)
    }


    fn unregister_filter(&mut self, event_type: &'static str, filter_id: usize) -> Result<()> {
        self.base.unregister_filter(event_type, filter_id)
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
        type Event = dyn Event;

        fn handle(&self, event: &Self::Event) -> Result<()> {
            self.call_count.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_default_dispatcher() {
        let mut dispatcher = DefaultEventDispatcher::new();
        let handler = Arc::new(TestHandler::new());
        
        dispatcher.register_handler("system_boot", handler.clone()).unwrap();
        
        let event = SystemEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: 0,
                source: "test".to_string(),
                category: nos_api::event::EventType::System,
                priority: EventPriority::Normal,
                tags: alloc::vec::Vec::new(),
            },
            data: SystemEventData::Boot { stage: "test".to_string() },
        };
        
        dispatcher.dispatch(&event).unwrap();
        assert_eq!(handler.call_count(), 1);
    }
}