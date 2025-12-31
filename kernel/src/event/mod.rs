//! Event System Module
//!
//! Provides event bus and dispatcher functionality

use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc, vec::Vec};

use nos_api::Result;

/// Event trait
pub trait Event {
    /// Get the event type identifier
    fn event_type(&self) -> &str;

    /// Handle the event
    fn handle(&self) -> Result<()> {
        Ok(())
    }
}

/// Event handler trait
pub trait EventHandler {
    /// Handle an event
    fn handle(&self, event: &dyn Event) -> Result<()>;
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
        self.handlers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
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





