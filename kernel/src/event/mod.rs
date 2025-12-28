//! Event System Module
//!
//! Provides event bus and dispatcher functionality

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use nos_api::Result;

/// Event trait
pub trait Event: Send + Sync {
    fn event_type(&self) -> &str;
}

/// Event handler trait
pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &dyn Event) -> Result<()>;
}

/// Simple event type
#[derive(Debug)]
pub struct SimpleEvent {
    pub event_type: String,
    pub data: Vec<u8>,
}

impl Event for SimpleEvent {
    fn event_type(&self) -> &str {
        &self.event_type
    }
}

/// Event bus
pub struct EventBus {
    handlers: BTreeMap<String, Vec<Arc<dyn EventHandler>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { handlers: BTreeMap::new() }
    }
    
    pub fn subscribe(&mut self, event_type: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.handlers.entry(event_type.to_string()).or_insert_with(Vec::new).push(handler);
        Ok(())
    }
    
    pub fn publish(&self, event: &dyn Event) -> Result<()> {
        if let Some(handlers) = self.handlers.get(event.event_type()) {
            for handler in handlers {
                handler.handle(event)?;
            }
        }
        Ok(())
    }
}

/// Global event bus
static mut EVENT_BUS: Option<EventBus> = None;

pub fn init_event_bus() -> Result<()> {
    unsafe {
        EVENT_BUS = Some(EventBus::new());
    }
    Ok(())
}

pub fn get_event_bus() -> &'static mut EventBus {
    unsafe {
        EVENT_BUS.as_mut().expect("Event bus not initialized")
    }
}

pub fn subscribe(event_type: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
    get_event_bus().subscribe(event_type, handler)
}

pub fn publish(event: &dyn Event) -> Result<()> {
    get_event_bus().publish(event)
}
