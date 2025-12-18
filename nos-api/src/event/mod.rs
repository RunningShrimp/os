//! Event system for NOS operating system

use crate::error::Result;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};
#[cfg(not(feature = "alloc"))]
use crate::fmt_utils::ToStringExt;

// Display and String are not used in no-alloc mode

#[cfg(feature = "alloc")]
use alloc::boxed::Box;
// Box is already imported through interfaces module

// Import types from interfaces module for no-alloc mode
#[cfg(not(feature = "alloc"))]
use crate::interfaces::String;

#[cfg(not(feature = "alloc"))]
use crate::Vec;

use core::sync::atomic::{AtomicU64, Ordering};

/// Simple time counter for event timestamps
static EVENT_TIME_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Get current time in nanoseconds for events
/// This is a simple counter that increments each time it's called
pub fn get_time_ns() -> u64 {
    EVENT_TIME_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Event categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    System,
    User,
    Security,
    Network,
    Storage,
    Process,
    Service,
    Hardware,
}

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Event trait for type-erased event handling
pub trait Event: core::any::Any {
    /// Get event ID
    fn id(&self) -> Option<u64> {
        None
    }
    
    /// Get event source
    fn source(&self) -> &str {
        "unknown"
    }
    
    /// Get event category
    fn category(&self) -> EventCategory {
        EventCategory::System
    }
    
    /// Get event priority
    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }
    
    /// Get event timestamp
    fn timestamp(&self) -> u64 {
        get_time_ns()
    }
    
    /// Get event tags
    fn tags(&self) -> &[&str] {
        &[]
    }
    
    /// Get event data
    fn data(&self) -> Option<&[u8]> {
        None
    }
    
    /// Serialize event to bytes
    fn serialize(&self) -> Result<Vec<u8>> {
        Err(crate::error::Error::NotImplemented("Event serialization not implemented".to_string()))
    }
    
    /// Deserialize event from bytes
    fn deserialize(_data: &[u8]) -> Result<Self> where Self: Sized {
        Err(crate::error::Error::NotImplemented("Deserialization not implemented".to_string()))
    }
}

/// Basic event implementation
#[derive(Debug)]
pub struct BasicEvent {
    id: Option<u64>,
    timestamp: u64,
    source: String,
    category: EventCategory,
    priority: EventPriority,
    #[cfg(feature = "alloc")]
    tags: Vec<&'static str>,
    #[cfg(not(feature = "alloc"))]
    tags: &'static [&'static str],
}

impl BasicEvent {
    /// Create a new basic event
    #[cfg(feature = "alloc")]
    pub fn new(category: EventCategory, priority: EventPriority, source: &str) -> Self {
        Self {
            id: None, // ID will be assigned when event is dispatched
            timestamp: get_time_ns(),
            source: source.to_string(),
            category,
            priority,
            tags: Vec::new(),
        }
    }
    
    /// Create a new basic event for no-alloc mode
    #[cfg(not(feature = "alloc"))]
    pub fn new(category: EventCategory, priority: EventPriority, source: &'static str) -> Self {
        Self {
            id: None, // ID will be assigned when event is dispatched
            timestamp: get_time_ns(),
            source: source.to_string(),
            category,
            priority,
            tags: &[],
        }
    }
    
    /// Adds a tag to the event
    #[cfg(feature = "alloc")]
    pub fn with_tag(mut self, tag: &'static str) -> Self {
        self.tags.push(tag);
        self
    }
    
    /// Sets the event source
    #[cfg(feature = "alloc")]
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }
}

impl Event for BasicEvent {
    fn id(&self) -> Option<u64> {
        self.id
    }
    
    fn source(&self) -> &str {
        &self.source
    }
    
    fn category(&self) -> EventCategory {
        self.category
    }
    
    fn priority(&self) -> EventPriority {
        self.priority
    }
    
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
    
    fn tags(&self) -> &[&str] {
        &self.tags
    }
    
    fn data(&self) -> Option<&[u8]> {
        None
    }
    
    fn serialize(&self) -> Result<Vec<u8>> {
        Err(crate::error::Error::NotImplemented("Event serialization not implemented".to_string()))
    }
    
    fn deserialize(_data: &[u8]) -> Result<Self> where Self: Sized {
        Err(crate::error::Error::NotImplemented("Deserialization not implemented".to_string()))
    }
}

/// Event dispatcher for managing event distribution
pub trait EventDispatcher {
    /// Dispatch an event to all registered listeners
    fn dispatch(&mut self, event: &BasicEvent) -> Result<()>;
    
    /// Register an event listener
    #[cfg(feature = "alloc")]
    fn register_listener(&mut self, listener: Box<dyn EventListener>) -> Result<()>;
    
    /// Register an event listener for no-alloc mode
    #[cfg(not(feature = "alloc"))]
    fn register_listener(&mut self, listener: &'static dyn EventListener) -> Result<()>;
    
    /// Unregister an event listener
    fn unregister_listener(&mut self, listener_id: u64) -> Result<()>;
    
    /// Get number of registered listeners
    fn listener_count(&self) -> usize;
}

/// Event listener trait
pub trait EventListener {
    /// Handle an event
    fn handle_event(&mut self, event: &BasicEvent) -> Result<()>;
    
    /// Get listener ID
    fn id(&self) -> u64;
    
    /// Get listener name
    fn name(&self) -> &str;
}

/// Basic event dispatcher implementation
#[cfg(feature = "alloc")]
pub struct BasicEventDispatcher {
    listeners: Vec<Box<dyn EventListener>>,
    next_listener_id: AtomicU64,
}

#[cfg(not(feature = "alloc"))]
pub struct BasicEventDispatcher {
    listeners: &'static [&'static dyn EventListener],
    next_listener_id: AtomicU64,
}

impl core::fmt::Debug for BasicEventDispatcher {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BasicEventDispatcher")
            .field("listener_count", &self.listeners.len())
            .finish()
    }
}

impl BasicEventDispatcher {
    /// Create a new basic event dispatcher
    #[cfg(feature = "alloc")]
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_listener_id: AtomicU64::new(1),
        }
    }
    
    /// Create a new basic event dispatcher for no-alloc mode
    #[cfg(not(feature = "alloc"))]
    pub fn new() -> Self {
        Self {
            listeners: &[],
            next_listener_id: AtomicU64::new(1),
        }
    }
}

impl Default for BasicEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl EventDispatcher for BasicEventDispatcher {
    fn dispatch(&mut self, event: &BasicEvent) -> Result<()> {
        for listener in &mut self.listeners {
            listener.handle_event(event)?;
        }
        Ok(())
    }
    
    fn register_listener(&mut self, listener: Box<dyn EventListener>) -> Result<()> {
        let _listener_id = self.next_listener_id.fetch_add(1, Ordering::Relaxed);
        self.listeners.push(listener);
        Ok(())
    }
    
    fn unregister_listener(&mut self, listener_id: u64) -> Result<()> {
        self.listeners.retain(|listener| listener.id() != listener_id);
        Ok(())
    }
    
    fn listener_count(&self) -> usize {
        self.listeners.len()
    }
}

#[cfg(not(feature = "alloc"))]
impl EventDispatcher for BasicEventDispatcher {
    fn dispatch(&mut self, _event: &BasicEvent) -> Result<()> {
        for _listener in self.listeners {
            // In no-alloc mode, we can't convert static references to mutable
            // This is a limitation of no-alloc mode
            // For now, we'll skip handling events in no-alloc mode
        }
        Ok(())
    }
    
    fn register_listener(&mut self, _listener: &'static dyn EventListener) -> Result<()> {
        // In no-alloc mode, we can't add listeners dynamically
        // This method is a no-op
        Ok(())
    }
    
    fn unregister_listener(&mut self, _listener_id: u64) -> Result<()> {
        // In no-alloc mode, we can't remove listeners dynamically
        // This method is a no-op
        Ok(())
    }
    
    fn listener_count(&self) -> usize {
        self.listeners.len()
    }
}

/// Basic event listener implementation
#[cfg(feature = "alloc")]
#[derive(Debug)]
pub struct BasicEventListener {
    id: u64,
    name: String,
}

#[cfg(not(feature = "alloc"))]
#[derive(Debug)]
pub struct BasicEventListener {
    id: u64,
    name: &'static str,
}

#[cfg(feature = "alloc")]
impl BasicEventListener {
    /// Create a new basic event listener
    pub fn new(name: &str) -> Self {
        Self {
            id: 0,
            name: name.to_string(),
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl BasicEventListener {
    /// Create a new basic event listener
    pub fn new(name: &'static str) -> Self {
        Self {
            id: 0,
            name,
        }
    }
}

#[cfg(feature = "alloc")]
impl EventListener for BasicEventListener {
    fn handle_event(&mut self, _event: &BasicEvent) -> Result<()> {
        // In a real implementation, this would log the event
        // or perform some action based on event type
        Ok(())
    }
    
    fn id(&self) -> u64 {
        self.id
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(not(feature = "alloc"))]
impl EventListener for BasicEventListener {
    fn handle_event(&mut self, _event: &BasicEvent) -> Result<()> {
        // In a real implementation, this would log the event
        // or perform some action based on event type
        Ok(())
    }
    
    fn id(&self) -> u64 {
        self.id
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_event_creation() {
        let event = BasicEvent::new(
            EventCategory::System,
            EventPriority::Normal,
            "test_source"
        );
        
        assert_eq!(event.category(), EventCategory::System);
        assert_eq!(event.priority(), EventPriority::Normal);
        assert_eq!(event.source(), "test_source");
        assert!(event.tags().is_empty());
    }
    
    #[test]
    fn test_event_dispatcher() {
        let mut dispatcher = BasicEventDispatcher::new();
        let listener = BasicEventListener::new("test_listener");
        
        assert!(dispatcher.register_listener(Box::new(listener)).is_ok());
        assert_eq!(dispatcher.listener_count(), 1);
        
        let event = BasicEvent::new(
            EventCategory::User,
            EventPriority::High,
            "test_event"
        );
        
        assert!(dispatcher.dispatch(&event).is_ok());
    }
}