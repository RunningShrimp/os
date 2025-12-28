//! Event types and metadata

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Event type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    System,
    User,
    Security,
    Network,
    Storage,
    Process,
    Service,
    Hardware,
    Memory,
}

// Convert from EventCategory to EventType
impl From<crate::event::EventCategory> for EventType {
    fn from(category: crate::event::EventCategory) -> Self {
        match category {
            crate::event::EventCategory::System => EventType::System,
            crate::event::EventCategory::User => EventType::User,
            crate::event::EventCategory::Security => EventType::Security,
            crate::event::EventCategory::Network => EventType::Network,
            crate::event::EventCategory::Storage => EventType::Storage,
            crate::event::EventCategory::Process => EventType::Process,
            crate::event::EventCategory::Service => EventType::Service,
            crate::event::EventCategory::Hardware => EventType::Hardware,
        }
    }
}

/// Event metadata
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub id: Option<u64>,
    pub timestamp: u64,
    pub source: String,
    pub category: EventType,
    pub priority: crate::event::EventPriority,
    pub tags: Vec<String>,
}

impl EventMetadata {
    pub fn new(source: &str, category: EventType, priority: crate::event::EventPriority) -> Self {
        Self {
            id: None,
            timestamp: crate::event::get_time_ns(),
            source: String::from(source),
            category,
            priority,
            tags: Vec::new(),
        }
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// Event filter trait
pub trait EventFilter: Send + Sync {
    fn matches(&self, event: &crate::event::BasicEvent) -> bool;
}

/// System event data
#[derive(Debug, Clone)]
pub enum SystemEventData {
    Boot { stage: String },
    Shutdown { reason: String },
    StageChange { stage: String },
}

/// Memory event data
#[derive(Debug, Clone)]
pub enum MemoryEventData {
    Allocation { size: usize, success: bool },
    Deallocation { size: usize },
}

/// Process event data
#[derive(Debug, Clone)]
pub enum ProcessEventData {
    Created { pid: u32, ppid: u32, name: String },
    Terminated { pid: u32, exit_code: i32 },
    Forked { parent: u32, child: u32 },
}
