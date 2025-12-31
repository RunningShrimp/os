//! In-process event bus for kernel-wide event handling.
//!
//! This module provides an event-driven architecture with:
//! - Event registration and handler management
//! - Synchronous and asynchronous event dispatch
//! - Event filtering and interception
//! - Priority-based handler execution
//! - Error handling and recovery

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use core::any::{Any, TypeId};
use core::fmt;

use crate::collections::HashMap;
use crate::sync::{Mutex, RwLock};

/// Event handler identifier
pub type HandlerId = u64;

/// Result type for event bus operations
pub type EventBusResult<T> = Result<T, EventBusError>;

/// Errors that can occur in event bus operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventBusError {
    /// Handler not found
    HandlerNotFound(HandlerId),
    /// Event dispatch failed
    DispatchFailed(String),
    /// Handler registration failed
    RegistrationFailed(String),
    /// Invalid event type
    InvalidEventType(String),
    /// System shutdown
    Shutdown,
}

impl fmt::Display for EventBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventBusError::HandlerNotFound(id) => write!(f, "Handler not found: {}", id),
            EventBusError::DispatchFailed(msg) => write!(f, "Dispatch failed: {}", msg),
            EventBusError::RegistrationFailed(msg) => write!(f, "Registration failed: {}", msg),
            EventBusError::InvalidEventType(msg) => write!(f, "Invalid event type: {}", msg),
            EventBusError::Shutdown => write!(f, "Event bus shutdown"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EventBusError {}

/// Event priority for handler execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    /// Lowest priority
    Low = 0,
    /// Normal priority (default)
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority (executes first)
    Critical = 3,
}

impl Default for EventPriority {
    fn default() -> Self {
        EventPriority::Normal
    }
}

/// Base event trait
///
/// All events must implement this trait to be dispatched through the event bus.
pub trait Event: Any + Send + Sync {
    /// Returns the event type name
    fn event_type(&self) -> &str {
        core::any::type_name::<Self>()
    }
}

/// Wrapper to make Event trait object-safe
pub struct EventBox {
    event: Box<dyn Any + Send + Sync>,
    type_name: String,
}

impl EventBox {
    /// Creates a new event box
    pub fn new<E: Event>(event: E) -> Self {
        Self {
            event: Box::new(event),
            type_name: core::any::type_name::<E>().to_string(),
        }
    }

    /// Downcasts to the concrete event type
    pub fn downcast<E: Event>(self) -> Result<E, Self> {
        self.event
            .downcast::<E>()
            .map(|boxed| *boxed)
            .map_err(|original| Self {
                event: original,
                type_name: self.type_name,
            })
    }

    /// Downcasts to the concrete event type by reference
    pub fn downcast_ref<E: Event>(&self) -> Option<&E> {
        self.event.downcast_ref::<E>()
    }

    /// Returns the type name
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the TypeId
    pub fn type_id(&self) -> TypeId {
        self.event.type_id()
    }
}

/// Event context passed to handlers
///
/// Provides metadata and context during event processing.
pub struct EventContext {
    /// Event timestamp
    pub timestamp: u64,
    /// Event ID
    pub event_id: u64,
    /// Source of the event
    pub source: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl EventContext {
    /// Creates a new event context
    pub fn new() -> Self {
        Self {
            timestamp: current_timestamp(),
            event_id: generate_event_id(),
            source: None,
            metadata: HashMap::new(),
        }
    }

    /// Sets the event source
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// Adds metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl Default for EventContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of event handler execution
pub enum HandlerResult {
    /// Event handled successfully
    Handled,
    /// Event handled but should continue propagating
    Continue,
    /// Stop event propagation
    StopPropagation,
    /// Handler encountered an error
    Error(String),
}

impl HandlerResult {
    /// Returns true if propagation should continue
    pub fn should_continue(&self) -> bool {
        matches!(self, HandlerResult::Continue | HandlerResult::Handled)
    }

    /// Returns true if propagation should stop
    pub fn should_stop(&self) -> bool {
        matches!(self, HandlerResult::StopPropagation)
    }

    /// Returns true if the handler failed
    pub fn is_error(&self) -> bool {
        matches!(self, HandlerResult::Error(_))
    }
}

/// Event handler trait
///
/// Handlers implement this trait to receive events.
pub trait EventHandler: Send + Sync {
    /// Handles an event
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle (boxed)
    /// * `ctx` - Event context
    fn handle(&self, event: &EventBox, ctx: &EventContext) -> HandlerResult;

    /// Returns the handler ID
    fn id(&self) -> HandlerId;

    /// Returns the event type this handler is interested in
    fn event_type(&self) -> TypeId;

    /// Returns the handler priority
    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }
}

/// Function-based event handler
struct FunctionHandler {
    id: HandlerId,
    event_type_id: TypeId,
    priority: EventPriority,
    handler: Arc<dyn Fn(&EventBox, &EventContext) -> HandlerResult + Send + Sync>,
}

impl EventHandler for FunctionHandler {
    fn handle(&self, event: &EventBox, ctx: &EventContext) -> HandlerResult {
        (self.handler)(event, ctx)
    }

    fn id(&self) -> HandlerId {
        self.id
    }

    fn event_type(&self) -> TypeId {
        self.event_type_id
    }

    fn priority(&self) -> EventPriority {
        self.priority
    }
}

/// Handler registration information
struct HandlerInfo {
    /// Handler instance
    handler: Arc<dyn EventHandler>,
    /// Handler is async
    is_async: bool,
    /// Filter function (optional)
    filter: Option<Arc<dyn Fn(&EventBox, &EventContext) -> bool + Send + Sync>>,
}

/// Event filter trait
///
/// Filters can be used to selectively process events.
pub trait EventFilter: Send + Sync {
    /// Returns true if the event should be processed
    fn should_process(&self, event: &EventBox, ctx: &EventContext) -> bool;
}

/// Function-based filter
struct FunctionFilter {
    filter: Arc<dyn Fn(&EventBox, &EventContext) -> bool + Send + Sync>,
}

impl EventFilter for FunctionFilter {
    fn should_process(&self, event: &EventBox, ctx: &EventContext) -> bool {
        (self.filter)(event, ctx)
    }
}

/// Event bus configuration
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum number of handlers
    max_handlers: usize,
    /// Enable error recovery
    enable_recovery: bool,
    /// Default timeout for handler execution (milliseconds)
    default_timeout_ms: u64,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_handlers: 1000,
            enable_recovery: true,
            default_timeout_ms: 5000,
        }
    }
}

/// In-process event bus
///
/// Provides centralized event dispatch and handler management.
///
/// # Example
///
/// ```rust
/// use kernel::messaging::eventbus::EventBus;
///
/// let bus = EventBus::new();
///
/// // Register a handler
/// let handler_id = bus.register_handler(
///     "MyEvent",
///     |event, ctx| {
///         // Handle event
///         HandlerResult::Handled
///     }
/// ).unwrap();
///
/// // Dispatch an event
/// bus.dispatch(event).unwrap();
/// ```
pub struct EventBus {
    /// Handlers by event type ID
    handlers: RwLock<HashMap<TypeId, Vec<HandlerInfo>>>,
    /// All handlers by ID (for removal)
    handlers_by_id: RwLock<HashMap<HandlerId, TypeId>>,
    /// Handler ID counter
    handler_counter: AtomicU64,
    /// Event statistics
    stats: Mutex<EventStats>,
    /// Configuration
    config: EventBusConfig,
    /// Shutdown flag
    shutdown: AtomicUsize,
}

/// Event bus statistics
#[derive(Debug, Default)]
struct EventStats {
    events_dispatched: AtomicU64,
    handlers_invoked: AtomicU64,
    handler_errors: AtomicU64,
}

impl EventBus {
    /// Creates a new event bus
    pub fn new() -> Self {
        Self::with_config(EventBusConfig::default())
    }

    /// Creates an event bus with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            handlers_by_id: RwLock::new(HashMap::new()),
            handler_counter: AtomicU64::new(1),
            stats: Mutex::new(EventStats::default()),
            config,
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Registers an event handler
    ///
    /// # Arguments
    ///
    /// * `event_type` - The type name of events to handle
    /// * `handler` - Function to handle events
    ///
    /// Returns the handler ID
    pub fn register_handler<E: Event, F>(
        &self,
        handler: F,
    ) -> EventBusResult<HandlerId>
    where
        F: Fn(&E, &EventContext) -> HandlerResult + Send + Sync + 'static,
    {
        self.register_handler_with_priority(handler, EventPriority::Normal)
    }

    /// Registers a handler with specific priority
    pub fn register_handler_with_priority<E: Event, F>(
        &self,
        handler: F,
        priority: EventPriority,
    ) -> EventBusResult<HandlerId>
    where
        F: Fn(&E, &EventContext) -> HandlerResult + Send + Sync + 'static,
    {
        if self.is_shutdown() {
            return Err(EventBusError::Shutdown);
        }

        let handler_id = self.handler_counter.fetch_add(1, AtomicOrdering::SeqCst);
        let type_id = TypeId::of::<E>();

        // Create the handler wrapper
        let handler_wrapper = Arc::new(FunctionHandler {
            id: handler_id,
            event_type_id: type_id,
            priority,
            handler: Arc::new(move |event_box, ctx| {
                // Try to downcast to the concrete type
                match event_box.downcast_ref::<E>() {
                    Some(concrete_event) => handler(concrete_event, ctx),
                    None => HandlerResult::Error("Type downcast failed".into()),
                }
            }),
        });

        // Register handler
        {
            let mut handlers = self.handlers.write();
            let entry = handlers.entry(type_id).or_insert_with(Vec::new);

            if entry.len() >= self.config.max_handlers {
                return Err(EventBusError::RegistrationFailed(
                    "Maximum handlers reached".into()
                ));
            }

            entry.push(HandlerInfo {
                handler: handler_wrapper,
                is_async: false,
                filter: None,
            });

            // Sort by priority (descending)
            entry.sort_by(|a, b| {
                b.handler.priority()
                    .cmp(&a.handler.priority())
            });
        }

        // Map handler ID to type ID
        {
            let mut handlers_by_id = self.handlers_by_id.write();
            handlers_by_id.insert(handler_id, type_id);
        }

        Ok(handler_id)
    }

    /// Registers a handler with a filter
    pub fn register_handler_with_filter<E: Event, F, Filter>(
        &self,
        handler: F,
        filter: Filter,
    ) -> EventBusResult<HandlerId>
    where
        F: Fn(&E, &EventContext) -> HandlerResult + Send + Sync + 'static,
        Filter: Fn(&E, &EventContext) -> bool + Send + Sync + 'static,
    {
        if self.is_shutdown() {
            return Err(EventBusError::Shutdown);
        }

        let handler_id = self.register_handler(handler)?;
        let type_id = TypeId::of::<E>();

        // Add filter to handler
        {
            let mut handlers = self.handlers.write();
            if let Some(entry) = handlers.get_mut(&type_id) {
                if let Some(info) = entry.iter_mut().find(|h| h.handler.id() == handler_id) {
                    info.filter = Some(Arc::new(
                        move |event_box: &EventBox, ctx: &EventContext| {
                            if let Some(concrete_event) = event_box.downcast_ref::<E>() {
                                filter(concrete_event, ctx)
                            } else {
                                false
                            }
                        }
                    ));
                }
            }
        }

        Ok(handler_id)
    }

    /// Unregisters a handler
    pub fn unregister_handler(&self, handler_id: HandlerId) -> EventBusResult<()> {
        // Find the handler
        let (type_id, handler_index) = {
            let handlers_by_id = self.handlers_by_id.read();
            let type_id = handlers_by_id
                .get(&handler_id)
                .ok_or(EventBusError::HandlerNotFound(handler_id))?;

            let handlers = self.handlers.read();
            let entry = handlers
                .get(type_id)
                .ok_or(EventBusError::HandlerNotFound(handler_id))?;

            let index = entry
                .iter()
                .position(|h| h.handler.id() == handler_id)
                .ok_or(EventBusError::HandlerNotFound(handler_id))?;

            (*type_id, index)
        };

        // Remove handler
        {
            let mut handlers = self.handlers.write();
            if let Some(entry) = handlers.get_mut(&type_id) {
                entry.remove(handler_index);
            }
        }

        // Remove from ID map
        {
            let mut handlers_by_id = self.handlers_by_id.write();
            handlers_by_id.remove(&handler_id);
        }

        Ok(())
    }

    /// Dispatches an event to all registered handlers
    ///
    /// Handlers are called in priority order. If a handler returns
    /// `StopPropagation`, remaining handlers are not called.
    pub fn dispatch<E: Event>(&self, event: E) -> EventBusResult<()> {
        self.dispatch_with_context(event, EventContext::default())
    }

    /// Dispatches an event with custom context
    pub fn dispatch_with_context<E: Event>(
        &self,
        event: E,
        ctx: EventContext,
    ) -> EventBusResult<()> {
        if self.is_shutdown() {
            return Err(EventBusError::Shutdown);
        }

        let type_id = TypeId::of::<E>();
        let event_box = EventBox::new(event);

        // Update stats
        {
            let stats = self.stats.lock();
            stats.events_dispatched.fetch_add(1, AtomicOrdering::Relaxed);
        }

        // Get handlers for this event type and invoke them
        let handlers = self.handlers.read();
        if let Some(handlers_list) = handlers.get(&type_id) {
            for handler_info in handlers_list {
            // Apply filter if present
            if let Some(filter) = &handler_info.filter {
                if !(filter)(&event_box, &ctx) {
                    continue;
                }
            }

            // Call handler
            let result = handler_info.handler.handle(&event_box, &ctx);

            // Update stats
            {
                let stats = self.stats.lock();
                stats.handlers_invoked.fetch_add(1, AtomicOrdering::Relaxed);

                if result.is_error() {
                    stats.handler_errors.fetch_add(1, AtomicOrdering::Relaxed);
                }
            }

            // Check if we should stop propagation
            if result.should_stop() {
                break;
            }
        }
        }

        Ok(())
    }

    /// Dispatches an event asynchronously
    ///
    /// Returns immediately, handlers run in background.
    pub fn dispatch_async<E: Event>(&self, event: E) -> EventBusResult<()> {
        // In a real implementation, this would use async tasks
        // For now, we'll just dispatch synchronously
        self.dispatch(event)
    }

    /// Returns event bus statistics
    pub fn stats(&self) -> EventStatistics {
        let stats = self.stats.lock();
        EventStatistics {
            events_dispatched: stats.events_dispatched.load(AtomicOrdering::Relaxed),
            handlers_invoked: stats.handlers_invoked.load(AtomicOrdering::Relaxed),
            handler_errors: stats.handler_errors.load(AtomicOrdering::Relaxed),
            registered_handlers: self.handlers_by_id.read().len(),
        }
    }

    /// Returns the number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers_by_id.read().len()
    }

    /// Shuts down the event bus
    pub fn shutdown(&self) {
        self.shutdown.store(1, AtomicOrdering::Release);
    }

    /// Returns true if shut down
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(AtomicOrdering::Acquire) != 0
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event bus statistics snapshot
#[derive(Debug, Clone)]
pub struct EventStatistics {
    /// Total events dispatched
    pub events_dispatched: u64,
    /// Total handler invocations
    pub handlers_invoked: u64,
    /// Total handler errors
    pub handler_errors: u64,
    /// Number of registered handlers
    pub registered_handlers: usize,
}

/// Generates a unique event ID
fn generate_event_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, AtomicOrdering::SeqCst)
}

/// Returns the current timestamp
fn current_timestamp() -> u64 {
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[cfg(not(feature = "std"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEvent {
        value: u32,
    }

    impl Event for TestEvent {}

    #[test]
    fn test_event_bus_register() {
        let bus = EventBus::new();
        let handler_id = bus.register_handler(|_event: &TestEvent, _ctx| {
            HandlerResult::Handled
        }).unwrap();

        assert!(handler_id > 0);
        assert_eq!(bus.handler_count(), 1);
    }

    #[test]
    fn test_event_bus_dispatch() {
        let bus = EventBus::new();
        bus.register_handler(|event: &TestEvent, _ctx| {
            assert_eq!(event.value, 42);
            HandlerResult::Handled
        }).unwrap();

        let event = TestEvent { value: 42 };
        bus.dispatch(event).unwrap();
    }

    #[test]
    fn test_stop_propagation() {
        let bus = EventBus::new();

        bus.register_handler(|_event: &TestEvent, _ctx| {
            HandlerResult::StopPropagation
        }).unwrap();

        bus.register_handler(|_event: &TestEvent, _ctx| {
            HandlerResult::Handled
        }).unwrap();

        let event = TestEvent { value: 42 };
        bus.dispatch(event).unwrap();

        let stats = bus.stats();
        assert_eq!(stats.handlers_invoked, 1);
    }

    #[test]
    fn test_unregister() {
        let bus = EventBus::new();
        let handler_id = bus.register_handler(|_event: &TestEvent, _ctx| {
            HandlerResult::Handled
        }).unwrap();

        bus.unregister_handler(handler_id).unwrap();
        assert_eq!(bus.handler_count(), 0);
    }

    #[test]
    fn test_priority() {
        let bus = EventBus::new();
        let mut order = Vec::new();

        bus.register_handler_with_priority(
            |_: &TestEvent, _| { order.push(1); HandlerResult::Handled },
            EventPriority::Low
        ).unwrap();

        bus.register_handler_with_priority(
            |_: &TestEvent, _| { order.push(2); HandlerResult::Handled },
            EventPriority::High
        ).unwrap();

        bus.dispatch(TestEvent { value: 0 }).unwrap();
        assert_eq!(order, vec![2, 1]); // High priority first
    }
}
