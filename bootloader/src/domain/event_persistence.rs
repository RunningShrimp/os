//! Event Persistence - Event storage and retrieval for bootloader diagnostics
//! 
//! Provides in-memory event storage capabilities for system crash diagnostics.
//! This module implements event persistence, query, and replay functionality
//! specifically designed for bootloader environments with memory constraints.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::boxed::Box;
use core::fmt;
use crate::domain::events::{DomainEvent, EventSeverity, DomainEventSubscriber};

/// Event store trait for event persistence
pub trait EventStore {
    /// Store an event
    fn store_event(&mut self, event: Box<dyn DomainEvent>) -> Result<(), &'static str>;
    
    /// Get all events
    fn get_all_events(&self, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>>;
    
    /// Get events by their type identifier
    fn get_events_by_type(&self, event_type: &'static str, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>>;
    
    /// Clear all events
    fn clear_events(&mut self);
    
    /// Get event count
    fn event_count(&self) -> usize;
    
    /// Get events by severity level
    fn get_events_by_severity(&self, severity: EventSeverity, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>>;
    
    /// Get events by time range
    fn get_events_by_time_range(&self, start_time: u64, end_time: u64, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>>;
    
    /// Get store statistics
    fn get_store_stats(&self) -> EventStoreStats;
}

/// Event store statistics
#[derive(Debug, Clone)]
pub struct EventStoreStats {
    /// Total number of events stored
    pub total_events: usize,
    /// Maximum capacity
    pub max_capacity: usize,
    /// Events by severity
    pub events_by_severity: BTreeMap<EventSeverity, usize>,
    /// Storage size in bytes
    pub storage_size: usize,
}

impl EventStoreStats {
    /// Create new event store statistics
    pub fn new() -> Self {
        Self {
            total_events: 0,
            max_capacity: 0,
            events_by_severity: BTreeMap::new(),
            storage_size: 0,
        }
    }
}

/// Event replayer trait for event retrieval and replay
pub trait EventReplayer {
    /// Replay events with filter
    fn replay_events(&self, filter: Option<&dyn Fn(&dyn DomainEvent) -> bool>) -> Vec<&Box<dyn DomainEvent>>;
    
    /// Replay events of specific type
    fn replay_events_by_type(&self, event_type: &'static str) -> Vec<&Box<dyn DomainEvent>>;
    
    /// Replay events in time range
    fn replay_events_by_time_range(&self, start_time: u64, end_time: u64) -> Vec<&Box<dyn DomainEvent>>;
    
    /// Get replay statistics
    fn get_replay_stats(&self) -> ReplayStats;
}

/// Persistent event store with memory backing
///
/// Provides durable event storage for bootloader diagnostics.
/// Events are stored in a reserved memory region for crash recovery.
pub struct PersistentEventStore {
    /// In-memory storage for events
    memory_store: VecDeque<Box<dyn DomainEvent>>,
    /// Maximum number of events to store
    max_events: usize,
    /// Event sequence counter
    sequence_counter: u64,
    /// Event type index for fast lookup
    event_type_index: BTreeMap<String, Vec<usize>>,
    /// Severity index for fast lookup
    severity_index: BTreeMap<EventSeverity, Vec<usize>>,
    /// Time-based index for range queries
    time_index: BTreeMap<u64, Vec<usize>>,
    /// Store statistics
    stats: EventStoreStats,
    /// Persistence enabled flag
    persistence_enabled: bool,
}

impl PersistentEventStore {
    /// Create new persistent event store
    ///
    /// # Arguments
    /// * `max_events` - Maximum number of events to store
    /// * `persistence_enabled` - Whether to enable persistence to memory
    ///
    /// # Returns
    /// New persistent event store instance
    pub fn new(max_events: usize, persistence_enabled: bool) -> Self {
        Self {
            memory_store: VecDeque::new(),
            max_events,
            sequence_counter: 0,
            event_type_index: BTreeMap::new(),
            severity_index: BTreeMap::new(),
            time_index: BTreeMap::new(),
            stats: EventStoreStats::new(),
            persistence_enabled,
        }
    }

    /// Create persistent event store with default settings
    pub fn with_default_settings() -> Self {
        Self::new(1000, true) // Default to 1000 events with persistence enabled
    }

    /// Store an event with full indexing
    ///
    /// This method stores an event and updates all indexes for fast retrieval
    ///
    /// # Arguments
    /// * `event` - The event to store
    ///
    /// # Returns
    /// Ok(()) if stored successfully, Err with error message if failed
    pub fn store_event(&mut self, event: Box<dyn DomainEvent>) -> Result<(), &'static str> {
        // Update sequence counter
        self.sequence_counter += 1;
        
        // Get event index
        let event_index = self.memory_store.len();
        
        // Store event
        self.memory_store.push_back(event);
        
        // Get event information before calling update_indexes
        let event_ref = self.memory_store.back().unwrap().as_ref();
        let event_type = event_ref.event_type();
        let severity = event_ref.severity();
        let timestamp = event_ref.timestamp();
        
        // Update indexes
        self.update_indexes(event_index, event_type, severity, timestamp);
        
        // Maintain size limit
        while self.memory_store.len() > self.max_events {
            if let Some(removed_event) = self.memory_store.pop_front() {
                let event_ref = removed_event.as_ref();
                self.remove_from_indexes(event_ref.event_type(), event_ref.severity(), event_ref.timestamp(), 0);
            }
        }
        
        // Update statistics
        self.update_stats();
        
        Ok(())
    }

    /// Update all indexes for an event
    ///
    /// # Arguments
    /// * `event_index` - The index of the event in the store
    /// * `event_type` - The type of the event
    /// * `severity` - The severity of the event
    /// * `timestamp` - The timestamp of the event
    /// Update indexes for a newly stored event
    fn update_indexes(&mut self, event_index: usize, event_type: &str, severity: EventSeverity, timestamp: u64) {
        // Update event type index
        self.event_type_index.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(event_index);
        
        // Update severity index
        self.severity_index.entry(severity)
            .or_insert_with(Vec::new)
            .push(event_index);
        
        // Update time index
        self.time_index.entry(timestamp)
            .or_insert_with(Vec::new)
            .push(event_index);
    }

    /// Remove an event from all indexes
    ///
    /// # Arguments
    /// * `event_type` - The type of the event to remove
    /// * `severity` - The severity of the event to remove
    /// * `timestamp` - The timestamp of the event to remove
    /// * `index` - The index of the event to remove (optional)
    fn remove_from_indexes(&mut self, event_type: &str, severity: EventSeverity, timestamp: u64, index: usize) {
        // Remove from type index
        if let Some(indices) = self.event_type_index.get_mut(&event_type.to_string()) {
            indices.retain(|&i| i != index);
        }
        
        // Remove from severity index
        if let Some(indices) = self.severity_index.get_mut(&severity) {
            indices.retain(|&i| i != index);
        }
        
        // Remove from time index
        if let Some(indices) = self.time_index.get_mut(&timestamp) {
            indices.retain(|&i| i != index);
        }
    }

    /// Get events by type with optimized index lookup
    ///
    /// # Arguments
    /// * `event_type` - The event type to filter by
    /// * `limit` - Maximum number of events to return
    ///
    /// # Returns
    /// Vector of events matching the type
    pub fn get_events_by_type(&self, event_type: &str, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        
        if let Some(indices) = self.event_type_index.get(event_type) {
            let mut count = 0;
            
            for &event_index in indices {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                    count += 1;
                    
                    if let Some(limit) = limit {
                        if count >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        result
    }

    /// Get events by time range with optimized index lookup
    ///
    /// # Arguments
    /// * `start_time` - Start timestamp (inclusive)
    /// * `end_time` - End timestamp (inclusive)
    /// * `limit` - Maximum number of events to return
    ///
    /// # Returns
    /// Vector of events in the time range
    pub fn get_events_by_time_range(&self, start_time: u64, end_time: u64, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        let mut count = 0;
        
        // Iterate through time range
        for (timestamp, indices) in self.time_index.range(start_time..=end_time) {
            log::trace!("Retrieving events from time range starting at: {}", timestamp);
            for &event_index in indices {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                    count += 1;
                    
                    if let Some(limit) = limit {
                        if count >= limit {
                            return result;
                        }
                    }
                }
            }
        }
        
        result
    }

    /// Get events by severity with optimized index lookup
    ///
    /// # Arguments
    /// * `severity` - The severity level to filter by
    /// * `limit` - Maximum number of events to return
    ///
    /// # Returns
    /// Vector of events with the specified severity
    pub fn get_events_by_severity(&self, severity: EventSeverity, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        let mut count = 0;
        
        if let Some(indices) = self.severity_index.get(&severity) {
            for &event_index in indices {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                    count += 1;
                    
                    if let Some(limit) = limit {
                        if count >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        result
    }

    /// Get all events with optional limit
    ///
    /// # Arguments
    /// * `limit` - Maximum number of events to return
    ///
    /// # Returns
    /// Vector of all events
    pub fn get_all_events(&self, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        let mut count = 0;
        
        for event in &self.memory_store {
            result.push(event.clone());
            count += 1;
            
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }
        }
        
        result
    }

    /// Clear all events and reset indexes
    pub fn clear_events(&mut self) {
        self.memory_store.clear();
        self.event_type_index.clear();
        self.severity_index.clear();
        self.time_index.clear();
        self.sequence_counter = 0;
        self.update_stats();
    }

    /// Get current event count
    pub fn event_count(&self) -> usize {
        self.memory_store.len()
    }

    /// Get store statistics
    pub fn get_store_stats(&self) -> EventStoreStats {
        self.stats.clone()
    }

    /// Update store statistics
    fn update_stats(&mut self) {
        let mut events_by_type = BTreeMap::new();
        let mut events_by_severity = BTreeMap::new();
        let mut total_events = 0;
        
        for event in &self.memory_store {
            let event_type = event.event_type().to_string();
            let severity = event.severity();
            
            // Count by type
            *events_by_type.entry(event_type).or_insert(0) += 1;
            
            // Count by severity
            *events_by_severity.entry(severity).or_insert(0) += 1;
            
            total_events += 1;
        }
        
        let storage_size = total_events * core::mem::size_of::<Box<dyn DomainEvent>>();
        
        self.stats = EventStoreStats {
            total_events,
            max_capacity: self.max_events,
            events_by_severity,
            storage_size,
        };
    }

    /// Rebuild all indexes from scratch
    ///
    /// This is used when an event is removed from the front of the store,
    /// which makes all existing indexes invalid. By rebuilding the indexes,
    /// we ensure they are consistent with the current state of the memory store.
    fn rebuild_indexes(&mut self) {
        // Clear existing indexes
        self.event_type_index.clear();
        self.severity_index.clear();
        self.time_index.clear();
        
        // Collect event information first to avoid borrow conflicts
        let event_info: Vec<(usize, &str, EventSeverity, u64)> = self.memory_store
            .iter()
            .enumerate()
            .map(|(index, event)| {
                (index, event.event_type(), event.severity(), event.timestamp())
            })
            .collect();
        
        // Rebuild indexes using the collected information
        for (event_index, event_type, severity, timestamp) in event_info {
            self.update_indexes(event_index, event_type, severity, timestamp);
        }
    }

    /// Get event by sequence number
    ///
    /// # Arguments
    /// * `sequence_number` - The sequence number to look up
    ///
    /// # Returns
    /// Option of event with the specified sequence number
    pub fn get_event_by_sequence(&self, sequence_number: u64) -> Option<Box<dyn DomainEvent>> {
        // Simple linear search for now - in production, this could be optimized
        for (index, event) in self.memory_store.iter().enumerate() {
            if event.event_id() == sequence_number {
                log::trace!("Found event {} at index {}", sequence_number, index);
                return Some(event.clone());
            }
        }
        
        log::debug!("Event with sequence number {} not found", sequence_number);
        None
    }

    /// Get latest events by type
    ///
    /// # Arguments
    /// * `event_type` - The event type
    /// * `count` - Number of latest events to return
    ///
    /// # Returns
    /// Vector of latest events of the specified type
    pub fn get_latest_events_by_type(&self, event_type: &str, count: usize) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        
        if let Some(indices) = self.event_type_index.get(event_type) {
            // Get the last 'count' indices
            let start_index = if indices.len() > count { indices.len() - count } else { 0 };
            
            for &event_index in &indices[start_index..] {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                }
            }
        }
        
        result
    }

    /// Check if persistence is enabled
    pub fn is_persistence_enabled(&self) -> bool {
        self.persistence_enabled
    }

    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> MemoryUsageStats {
        let store_size = self.memory_store.len() * core::mem::size_of::<Box<dyn DomainEvent>>();
        let index_size = (
            self.event_type_index.len() + self.severity_index.len() + self.time_index.len()
        ) * core::mem::size_of::<usize>();
        
        MemoryUsageStats {
            store_size_bytes: store_size,
            index_size_bytes: index_size,
            total_size_bytes: store_size + index_size,
            max_events: self.max_events,
            current_events: self.memory_store.len(),
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryUsageStats {
    /// Size of event store in bytes
    pub store_size_bytes: usize,
    /// Size of indexes in bytes
    pub index_size_bytes: usize,
    /// Total size in bytes
    pub total_size_bytes: usize,
    /// Maximum number of events
    pub max_events: usize,
    /// Current number of events
    pub current_events: usize,
}

impl EventStore for PersistentEventStore {
    fn store_event(&mut self, event: Box<dyn DomainEvent>) -> Result<(), &'static str> {
        // Update sequence counter
        self.sequence_counter += 1;
        
        // Get event index
        let event_index = self.memory_store.len();
        
        // Store event
        self.memory_store.push_back(event);
        
        // Get event information before calling update_indexes
        let event_ref = self.memory_store.back().unwrap();
        let event_type = event_ref.event_type();
        let severity = event_ref.severity();
        let timestamp = event_ref.timestamp();
        
        // Update indexes
        self.update_indexes(event_index, event_type, severity, timestamp);
        
        // Maintain size limit
        while self.memory_store.len() > self.max_events {
            // When we pop from the front, the first event has index 0
            // We need to update all indexes by decrementing them by 1
            // First, get the event information before popping
            let removed_event = self.memory_store.pop_front().ok_or("Failed to remove event")?;
            
            // Log the removed event information
            log::debug!("Removed event: type={}, id={}", removed_event.event_type(), removed_event.event_id());
            
            // For simplicity, we'll rebuild the indexes when an event is removed
            // This is less efficient but avoids complex index management
            self.rebuild_indexes();
        }
        
        // Update statistics
        self.update_stats();
        
        Ok(())
    }

    fn get_events_by_type(&self, event_type: &'static str, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        
        if let Some(indices) = self.event_type_index.get(event_type) {
            let mut count = 0;
            
            for &event_index in indices {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                    count += 1;
                    
                    if let Some(limit) = limit {
                        if count >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        result
    }

    fn get_events_by_time_range(&self, start_time: u64, end_time: u64, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        let mut count = 0;
        
        // Iterate through time range
        for (_timestamp, indices) in self.time_index.range(start_time..=end_time) {
            for &event_index in indices {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                    count += 1;
                    
                    if let Some(limit) = limit {
                        if count >= limit {
                            return result;
                        }
                    }
                }
            }
        }
        
        result
    }

    fn get_events_by_severity(&self, severity: EventSeverity, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        let mut count = 0;
        
        if let Some(indices) = self.severity_index.get(&severity) {
            for &event_index in indices {
                if let Some(event) = self.memory_store.get(event_index) {
                    result.push(event.clone());
                    count += 1;
                    
                    if let Some(limit) = limit {
                        if count >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        result
    }

    fn get_all_events(&self, limit: Option<usize>) -> Vec<Box<dyn DomainEvent>> {
        let mut result = Vec::new();
        let mut count = 0;
        
        for event in &self.memory_store {
            result.push(event.clone());
            count += 1;
            
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }
        }
        
        result
    }

    fn clear_events(&mut self) {
        self.memory_store.clear();
        self.event_type_index.clear();
        self.severity_index.clear();
        self.time_index.clear();
        self.sequence_counter = 0;
        self.update_stats();
    }

    fn event_count(&self) -> usize {
        self.memory_store.len()
    }

    fn get_store_stats(&self) -> EventStoreStats {
        self.stats.clone()
    }
}

/// Event replay with diagnostics support
///
/// Provides event replay capabilities for system diagnostics and recovery
pub struct DiagnosticEventReplayer {
    event_store: Box<dyn EventStore>,
    replay_position: usize,
    replay_filters: Vec<ReplayFilter>,
}

/// Replay filter for selective event replay
pub enum ReplayFilter {
    /// Filter by event type
    EventType(String),
    /// Filter by severity level
    Severity(EventSeverity),
    /// Filter by time range
    TimeRange(u64, u64),
    /// Filter by custom predicate
    Custom(Box<dyn Fn(&dyn DomainEvent) -> bool>),
}

impl fmt::Debug for ReplayFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayFilter::EventType(s) => write!(f, "ReplayFilter::EventType({:?})", s),
            ReplayFilter::Severity(s) => write!(f, "ReplayFilter::Severity({:?})", s),
            ReplayFilter::TimeRange(start, end) => write!(f, "ReplayFilter::TimeRange({:?}, {:?})", start, end),
            ReplayFilter::Custom(_) => write!(f, "ReplayFilter::Custom(closure)")
        }
    }
}



impl DiagnosticEventReplayer {
    /// Create new diagnostic event replayer
    ///
    /// # Arguments
    /// * `event_store` - The event store to replay from
    ///
    /// # Returns
    /// New diagnostic event replayer instance
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            event_store,
            replay_position: 0,
            replay_filters: Vec::new(),
        }
    }

    /// Add replay filter
    ///
    /// # Arguments
    /// * `filter` - The filter to add
    pub fn add_filter(&mut self, filter: ReplayFilter) {
        self.replay_filters.push(filter);
    }

    /// Clear all replay filters
    pub fn clear_filters(&mut self) {
        self.replay_filters.clear();
    }

    /// Check if event passes all filters
    ///
    /// # Arguments
    /// * `event` - The event to check
    ///
    /// # Returns
    /// True if event passes all filters
    fn event_passes_filters(&self, event: &dyn DomainEvent) -> bool {
        for filter in &self.replay_filters {
            match filter {
                ReplayFilter::EventType(event_type) => {
                    if event.event_type() != event_type {
                        return false;
                    }
                }
                ReplayFilter::Severity(severity) => {
                    if event.severity() != *severity {
                        return false;
                    }
                }
                ReplayFilter::TimeRange(start, end) => {
                    let timestamp = event.timestamp();
                    if timestamp < *start || timestamp > *end {
                        return false;
                    }
                }
                ReplayFilter::Custom(predicate) => {
                    if !predicate(event) {
                        return false;
                    }
                }
            }
        }
        
        true
    }

    /// Replay events to subscriber
    ///
    /// # Arguments
    /// * `subscriber` - The subscriber to notify with replayed events
    /// * `limit` - Maximum number of events to replay
    ///
    /// # Returns
    /// Ok(()) if replay completed successfully, Err with error message if failed
    pub fn replay_to_subscriber(
        &mut self,
        subscriber: &dyn DomainEventSubscriber,
        limit: Option<usize>,
    ) -> Result<(), &'static str> {
        let events = self.event_store.get_all_events(limit);
        let mut replayed_count = 0;
        
        for event in events {
            if self.event_passes_filters(event.as_ref()) {
                subscriber.handle(event.as_ref())?;
                replayed_count += 1;
            }
        }
        
        self.replay_position += replayed_count;
        Ok(())
    }

    /// Get replay statistics
    pub fn get_replay_stats(&self) -> ReplayStats {
        ReplayStats {
            total_events_replayed: self.replay_position,
            current_position: self.replay_position,
            filters_active: !self.replay_filters.is_empty(),
        }
    }

    /// Reset replay position
    pub fn reset_replay(&mut self) {
        self.replay_position = 0;
    }
}

/// Replay statistics
#[derive(Debug, Clone)]
pub struct ReplayStats {
    /// Total number of events replayed
    pub total_events_replayed: usize,
    /// Current replay position
    pub current_position: usize,
    /// Whether filters are active
    pub filters_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::*;

    #[test]
    fn test_persistent_event_store_creation() {
        let store = PersistentEventStore::new(100, true);
        assert_eq!(store.event_count(), 0);
        assert!(store.is_persistence_enabled());
    }

    #[test]
    fn test_event_storage_and_retrieval() {
        let mut store = PersistentEventStore::new(10, true);
        
        // Store some events
        let event1 = Box::new(BootPhaseCompletedEvent::new("test", 1000, true));
        let event2 = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 2000));
        
        assert!(store.store_event(event1).is_ok());
        assert!(store.store_event(event2).is_ok());
        assert_eq!(store.event_count(), 2);
        
        // Test retrieval by type
        let boot_events = store.get_events_by_type("BootPhaseCompleted", Some(10));
        assert_eq!(boot_events.len(), 1);
        
        // Test retrieval by time range
        let time_events = store.get_events_by_time_range(500, 3000, Some(10));
        assert_eq!(time_events.len(), 2);
        
        // Test retrieval by severity
        let info_events = store.get_events_by_severity(EventSeverity::Info, Some(10));
        assert_eq!(info_events.len(), 2);
    }

    #[test]
    fn test_event_store_capacity_limit() {
        let mut store = PersistentEventStore::new(2, true);
        
        // Store more events than capacity
        let event1 = Box::new(BootPhaseCompletedEvent::new("test1", 1000, true));
        let event2 = Box::new(BootPhaseCompletedEvent::new("test2", 2000, true));
        let event3 = Box::new(BootPhaseCompletedEvent::new("test3", 3000, true));
        
        assert!(store.store_event(event1).is_ok());
        assert!(store.store_event(event2).is_ok());
        assert!(store.store_event(event3).is_ok());
        
        // Should only keep the last 2 events
        assert_eq!(store.event_count(), 2);
        
        let events = store.get_all_events(None);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type(), "BootPhaseCompleted");
        assert_eq!(events[1].event_type(), "BootPhaseCompleted");
    }
    #[test]
    fn test_diagnostic_replayer() {
        let mut store = PersistentEventStore::new(10, true);
        
        // Add some test events
        let event1 = Box::new(BootPhaseCompletedEvent::new("test", 1000, true));
        let event2 = Box::new(SystemErrorEvent::new(1, "Test error".to_string(), "test", 2000));
        
        assert!(store.store_event(event1).is_ok());
        assert!(store.store_event(event2).is_ok());
        
        // Now create replayer with the populated store
        let mut replayer = DiagnosticEventReplayer::new(Box::new(store));
        
        // Add filter for only boot events
        replayer.add_filter(ReplayFilter::EventType("BootPhaseCompleted".to_string()));
        
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
        
        let test_subscriber = TestSubscriber { events_received: Vec::new() };
        
        // Replay - should only get boot events
        assert!(replayer.replay_to_subscriber(&test_subscriber, Some(10)).is_ok());
        assert_eq!(test_subscriber.events_received.len(), 1);
        assert_eq!(test_subscriber.events_received[0], "BootPhaseCompleted");
    }

    #[test]
    fn test_memory_usage_stats() {
        let store = PersistentEventStore::new(100, true);
        let stats = store.get_memory_usage();
        
        assert_eq!(stats.max_events, 100);
        assert_eq!(stats.current_events, 0);
        assert!(stats.total_size_bytes > 0);
    }
}