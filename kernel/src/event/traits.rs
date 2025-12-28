#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Event System Traits
//!
//! This module defines the core traits for the event system.

use alloc::{
    collections::BTreeMap,
    string::String,
    boxed::Box,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::AtomicUsize;
use nos_api::{
    event::{
        Event, EventMetadata, EventType, EventPriority, EventHandler,
    },
    Result,
};

/// Type alias for event ID
pub type EventId = u64;

/// Trait for event bus implementations
pub trait EventBus {
    /// Publish an event to the bus
    fn publish(&mut self, event: Box<dyn Event>) -> Result<()>;

    /// Subscribe to events of a specific topic
    fn subscribe(&mut self, topic: &str, handler: Arc<dyn EventHandler>) -> Result<()>;

    /// Unsubscribe from events of a specific topic
    fn unsubscribe(&mut self, topic: &str, handler: &Arc<dyn EventHandler>) -> Result<()>;

    /// Subscribe to all events
    fn subscribe_all(&mut self, handler: Arc<dyn EventHandler>) -> Result<()>;

    /// Unsubscribe from all events
    fn unsubscribe_all(&mut self, handler: &Arc<dyn EventHandler>) -> Result<()>;

    /// Get event bus statistics
    fn get_stats(&self) -> &EventBusStats;
}

/// Trait for event filters
/// This trait is used to filter events before they are dispatched
pub trait EventFilter: Send + Sync {
    /// Check if an event should be dispatched
    fn should_dispatch(&self, event: &dyn Event) -> bool;

    /// Check if an event matches the filter criteria
    /// Default implementation uses should_dispatch
    fn matches(&self, event: &dyn Event) -> bool {
        self.should_dispatch(event)
    }
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

/// Basic event filter that matches all events
pub struct BasicEventFilter;

impl BasicEventFilter {
    /// Create a new basic event filter
    pub fn new() -> Self {
        Self
    }
}

impl Default for BasicEventFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventFilter for BasicEventFilter {
    fn should_dispatch(&self, _event: &dyn Event) -> bool {
        true // Pass all events
    }
}

/// Event filter based on event category
pub struct CategoryFilter {
    /// Event categories to match
    categories: Vec<nos_api::event::EventCategory>,
}

impl CategoryFilter {
    /// Create a new category filter
    pub fn new(categories: Vec<nos_api::event::EventCategory>) -> Self {
        Self { categories }
    }

    /// Create a filter for a single category
    pub fn single(category: nos_api::event::EventCategory) -> Self {
        Self { categories: alloc::vec::Vec::from([category]) }
    }
}

impl EventFilter for CategoryFilter {
    fn should_dispatch(&self, event: &dyn Event) -> bool {
        self.categories.contains(&event.category())
    }
}

/// Event filter based on priority
pub struct PriorityFilter {
    /// Minimum priority to pass
    min_priority: nos_api::event::EventPriority,
}

impl PriorityFilter {
    /// Create a new priority filter
    pub fn new(min_priority: nos_api::event::EventPriority) -> Self {
        Self { min_priority }
    }
}

impl EventFilter for PriorityFilter {
    fn should_dispatch(&self, event: &dyn Event) -> bool {
        event.priority() >= self.min_priority
    }
}

/// Event filter based on source
pub struct SourceFilter {
    /// Sources to match
    sources: Vec<String>,
}

impl SourceFilter {
    /// Create a new source filter
    pub fn new(sources: Vec<String>) -> Self {
        Self { sources }
    }

    /// Create a filter for a single source
    pub fn single(source: String) -> Self {
        Self { sources: {
    let mut v = alloc::vec::Vec::new();
    v.push(source);
    v
} }
    }
}

impl EventFilter for SourceFilter {
    fn should_dispatch(&self, event: &dyn Event) -> bool {
        let source = event.source();
        self.sources.iter().any(|s| s == source)
    }
}

/// Composite event filter that combines multiple filters
pub struct CompositeEventFilter {
    /// Filters to combine
    filters: Vec<Arc<dyn EventFilter>>,
    /// Whether to use AND (true) or OR (false) logic
    and_logic: bool,
}

impl CompositeEventFilter {
    /// Create a new composite filter with AND logic
    pub fn and(filters: Vec<Arc<dyn EventFilter>>) -> Self {
        Self { filters, and_logic: true }
    }

    /// Create a new composite filter with OR logic
    pub fn or(filters: Vec<Arc<dyn EventFilter>>) -> Self {
        Self { filters, and_logic: false }
    }
}

impl EventFilter for CompositeEventFilter {
    fn should_dispatch(&self, event: &dyn Event) -> bool {
        if self.and_logic {
            // All filters must match
            self.filters.iter().all(|f| f.should_dispatch(event))
        } else {
            // At least one filter must match
            self.filters.iter().any(|f| f.should_dispatch(event))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nos_api::event::{BasicEvent, EventCategory, EventPriority};

    #[test]
    fn test_basic_event_filter() {
        let filter = BasicEventFilter::new();
        let event = BasicEvent::new(EventCategory::System, EventPriority::Normal, "test");

        assert!(filter.should_dispatch(&event));
    }

    #[test]
    fn test_category_filter() {
        let filter = CategoryFilter::single(EventCategory::System);
        let event = BasicEvent::new(EventCategory::System, EventPriority::Normal, "test");

        assert!(filter.should_dispatch(&event));
    }

    #[test]
    fn test_priority_filter() {
        let filter = PriorityFilter::new(EventPriority::High);
        let high_event = BasicEvent::new(EventCategory::System, EventPriority::High, "test");
        let low_event = BasicEvent::new(EventCategory::System, EventPriority::Low, "test");

        assert!(filter.should_dispatch(&high_event));
        assert!(!filter.should_dispatch(&low_event));
    }
}
