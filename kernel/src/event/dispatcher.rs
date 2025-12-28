//! Event Dispatcher Module
//!
//! Provides event dispatching functionality

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use nos_api::Result;

use crate::event::{Event, EventHandler};

/// Event dispatcher
pub struct EventDispatcher {
    handlers: BTreeMap<String, Vec<Arc<dyn EventHandler>>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self { handlers: BTreeMap::new() }
    }
    
    pub fn register_handler(&mut self, event_type: &str, handler: Arc<dyn EventHandler>) -> Result<()> {
        self.handlers.entry(event_type.to_string()).or_insert_with(Vec::new).push(handler);
        Ok(())
    }
    
    pub fn dispatch(&self, event: &dyn Event) -> Result<()> {
        if let Some(handlers) = self.handlers.get(event.event_type()) {
            for handler in handlers {
                handler.handle(event)?;
            }
        }
        Ok(())
    }
}
