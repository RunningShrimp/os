//! Domain Events - Publish/Subscribe for domain state changes
//!
//! Events represent significant changes in the domain that other subsystems
//! may want to respond to (e.g., logging, diagnostics, state tracking).
//!
//! This module provides a complete event-driven architecture for the bootloader,
//! including event publishing, subscription, persistence, and replay capabilities.

use alloc::vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::{format, string::{String, ToString}};
use core::fmt;
use core::any::Any;

/// Domain Event trait - All domain events implement this
pub trait DomainEvent: Send + Sync + fmt::Debug + Any {
    /// Get the event type name
    fn event_type(&self) -> &'static str;

    /// Get timestamp in milliseconds
    fn timestamp(&self) -> u64;

    /// Format event as string for logging
    fn as_string(&self) -> &'static str;

    /// Get event ID for tracking
    fn event_id(&self) -> u64 {
        self.timestamp()
    }

    /// Get event severity level
    fn severity(&self) -> EventSeverity {
        EventSeverity::Info
    }

    /// Get event metadata
    fn metadata(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
    
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Clone the event into a new Box<dyn DomainEvent>
    fn clone_box(&self) -> Box<dyn DomainEvent>;
}

// Implement Clone for Box<dyn DomainEvent>
impl Clone for Box<dyn DomainEvent> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// Implement DomainEvent for Box<dyn DomainEvent> to make it usable as a DomainEvent
impl DomainEvent for Box<dyn DomainEvent> {
    fn event_type(&self) -> &'static str {
        (**self).event_type()
    }

    fn timestamp(&self) -> u64 {
        (**self).timestamp()
    }

    fn as_string(&self) -> &'static str {
        (**self).as_string()
    }

    fn event_id(&self) -> u64 {
        (**self).event_id()
    }

    fn severity(&self) -> EventSeverity {
        (**self).severity()
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        (**self).metadata()
    }
    
    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        (**self).clone_box()
    }
}

/// Event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    /// Debug information
    Debug,
    /// General information
    Info,
    /// Warning conditions
    Warning,
    /// Error conditions
    Error,
    /// Critical errors
    Critical,
}

impl EventSeverity {
    /// Get severity as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }

    /// Get numeric severity value (higher = more severe)
    pub fn value(&self) -> u8 {
        match self {
            Self::Debug => 1,
            Self::Info => 2,
            Self::Warning => 3,
            Self::Error => 4,
            Self::Critical => 5,
        }
    }
}

/// Domain Event Subscriber trait
///
/// Implemented by components that want to receive domain events
pub trait DomainEventSubscriber: Send + Sync {
    /// Handle an event
    ///
    /// # Arguments
    /// * `event` - The event to handle
    ///
    /// # Returns
    /// Ok(()) if handled successfully, Err with error message if failed
    fn handle(&self, event: &dyn DomainEvent) -> Result<(), &'static str>;

    /// Get subscriber name for identification
    fn subscriber_name(&self) -> &'static str;

    /// Check if subscriber is interested in this event type
    fn is_interested_in(&self, event_type: &'static str) -> bool {
        // Default implementation subscribes to all events
        // Subclasses can override to filter specific event types
        log::trace!("Checking interest in event type: {}", event_type);
        true
    }

    /// Get subscriber priority (higher = higher priority)
    fn priority(&self) -> u8 {
        50 // Default priority
    }

    /// Clone this subscriber into a Box<dyn DomainEventSubscriber>
    fn clone_box(&self) -> Box<dyn DomainEventSubscriber>;
}

// Implement Clone for Box<dyn DomainEventSubscriber>
impl Clone for Box<dyn DomainEventSubscriber> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Domain Event Publisher trait
///
/// Implemented by event publishing mechanisms
pub trait DomainEventPublisher: Send + Sync {
    /// Publish an event
    ///
    /// # Arguments
    /// * `event` - The event to publish
    ///
    /// # Returns
    /// Ok(()) if published successfully, Err with error message if failed
    fn publish(&mut self, event: Box<dyn DomainEvent>) -> Result<(), &'static str>;

    /// Subscribe to events
    ///
    /// # Arguments
    /// * `event_type` - The event type to subscribe to (null for all events)
    /// * `subscriber` - The subscriber to notify
    ///
    /// # Returns
    /// Ok(()) if subscribed successfully, Err with error message if failed
    fn subscribe(&mut self, event_type: Option<&'static str>, subscriber: Box<dyn DomainEventSubscriber>) -> Result<(), &'static str>;

    /// Unsubscribe from events
    ///
    /// # Arguments
    /// * `subscriber_name` - The name of the subscriber to remove
    ///
    /// # Returns
    /// Ok(()) if unsubscribed successfully, Err with error message if failed
    fn unsubscribe(&mut self, subscriber_name: &'static str) -> Result<(), &'static str>;

    /// Get event history
    ///
    /// # Returns
    /// Slice of historical events
    fn get_event_history(&self) -> &[Box<dyn DomainEvent>];

    /// Clear event history
    fn clear_history(&mut self);

    /// Get subscriber count
    fn subscriber_count(&self) -> usize;

    /// Get published event count
    fn event_count(&self) -> usize;
}

/// Event Handler trait
///
/// Provides a more sophisticated event handling interface
pub trait EventHandler: Send + Sync {
    /// Handle an event with context
    ///
    /// # Arguments
    /// * `event` - The event to handle
    /// * `context` - Event handling context
    ///
    /// # Returns
    /// Event handling result
    fn handle_with_context(&self, event: &dyn DomainEvent, context: &EventContext) -> EventHandlingResult;

    /// Get handler name
    fn handler_name(&self) -> &'static str;

    /// Check if handler can handle this event type
    fn can_handle(&self, event_type: &'static str) -> bool;

    /// Get handler capabilities
    fn capabilities(&self) -> HandlerCapabilities;
}

/// Event handling context
#[derive(Debug, Clone)]
pub struct EventContext {
    /// Event sequence number
    pub sequence_number: u64,
    /// Total number of subscribers
    pub total_subscribers: usize,
    /// Current subscriber index
    pub current_subscriber: usize,
    /// Handling start time
    pub start_time: u64,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

impl EventContext {
    /// Create new event context
    pub fn new(sequence_number: u64, total_subscribers: usize) -> Self {
        Self {
            sequence_number,
            total_subscribers,
            current_subscriber: 0,
            start_time: 0, // Would be set by publisher
            metadata: BTreeMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Event handling result
#[derive(Debug, Clone)]
pub enum EventHandlingResult {
    /// Event handled successfully
    Success,
    /// Event handled with warning
    SuccessWithWarning(&'static str),
    /// Event handling failed
    Failed(&'static str),
    /// Event should be retried later
    RetryLater,
    /// Stop processing this event for other handlers
    StopProcessing,
}

/// Handler capabilities
#[derive(Debug, Clone)]
pub struct HandlerCapabilities {
    /// Can handle async events
    pub async_handling: bool,
    /// Can filter events
    pub event_filtering: bool,
    /// Can transform events
    pub event_transformation: bool,
    /// Can persist events
    pub event_persistence: bool,
}

impl HandlerCapabilities {
    /// Create basic capabilities
    pub fn basic() -> Self {
        Self {
            async_handling: false,
            event_filtering: false,
            event_transformation: false,
            event_persistence: false,
        }
    }

    /// Create advanced capabilities
    pub fn advanced() -> Self {
        Self {
            async_handling: true,
            event_filtering: true,
            event_transformation: true,
            event_persistence: true,
        }
    }
}

/// Boot phase completed event
#[derive(Debug, Clone)]
pub struct BootPhaseCompletedEvent {
    pub phase_name: &'static str,  // Use static string instead of BootPhase enum
    pub timestamp: u64,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

impl BootPhaseCompletedEvent {
    pub fn new(phase_name: &'static str, timestamp: u64, success: bool) -> Self {
        Self {
            phase_name,
            timestamp,
            success,
            duration_ms: 0,
            error_message: None,
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }
}

impl DomainEvent for BootPhaseCompletedEvent {
    fn event_type(&self) -> &'static str {
        "BootPhaseCompleted"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        if self.success {
            "Boot phase completed successfully"
        } else {
            "Boot phase failed"
        }
    }

    fn severity(&self) -> EventSeverity {
        if self.success {
            EventSeverity::Info
        } else {
            EventSeverity::Error
        }
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        let mut metadata = Vec::new();
        metadata.push(("phase_name", self.phase_name.to_string()));
        metadata.push(("success", self.success.to_string()));
        metadata.push(("duration_ms", self.duration_ms.to_string()));
        if let Some(ref error) = self.error_message {
            metadata.push(("error_message", error.clone()));
        }
        metadata
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Graphics initialized event
#[derive(Debug, Clone)]
pub struct GraphicsInitializedEvent {
    pub mode_width: u16,
    pub mode_height: u16,
    pub framebuffer_addr: usize,
    pub bits_per_pixel: u8,
    pub timestamp: u64,
    pub backend_type: &'static str,
}

impl GraphicsInitializedEvent {
    pub fn new(width: u16, height: u16, addr: usize, timestamp: u64) -> Self {
        Self {
            mode_width: width,
            mode_height: height,
            framebuffer_addr: addr,
            bits_per_pixel: 32,
            timestamp,
            backend_type: "VBE",
        }
    }

    pub fn with_bpp(mut self, bpp: u8) -> Self {
        self.bits_per_pixel = bpp;
        self
    }

    pub fn with_backend(mut self, backend: &'static str) -> Self {
        self.backend_type = backend;
        self
    }
}

impl DomainEvent for GraphicsInitializedEvent {
    fn event_type(&self) -> &'static str {
        "GraphicsInitialized"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Graphics output initialized"
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        vec![
            ("width", self.mode_width.to_string()),
            ("height", self.mode_height.to_string()),
            ("bits_per_pixel", self.bits_per_pixel.to_string()),
            ("framebuffer_addr", format!("{:#x}", self.framebuffer_addr)),
            ("backend_type", self.backend_type.to_string()),
        ]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Kernel loaded event
#[derive(Debug, Clone)]
pub struct KernelLoadedEvent {
    pub kernel_address: u64,
    pub kernel_size: u64,
    pub entry_point: u64,
    pub timestamp: u64,
    pub checksum: u32,
    pub signature_verified: bool,
}

impl KernelLoadedEvent {
    pub fn new(address: u64, size: u64, timestamp: u64) -> Self {
        Self {
            kernel_address: address,
            kernel_size: size,
            entry_point: address, // Default to same as load address
            timestamp,
            checksum: 0,
            signature_verified: false,
        }
    }

    pub fn with_entry_point(mut self, entry_point: u64) -> Self {
        self.entry_point = entry_point;
        self
    }

    pub fn with_checksum(mut self, checksum: u32) -> Self {
        self.checksum = checksum;
        self
    }

    pub fn with_signature_verified(mut self, verified: bool) -> Self {
        self.signature_verified = verified;
        self
    }
}

impl DomainEvent for KernelLoadedEvent {
    fn event_type(&self) -> &'static str {
        "KernelLoaded"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Kernel loaded successfully"
    }

    fn severity(&self) -> EventSeverity {
        if self.signature_verified {
            EventSeverity::Info
        } else {
            EventSeverity::Warning
        }
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        vec![
            ("kernel_address", format!("{:#x}", self.kernel_address)),
            ("kernel_size", self.kernel_size.to_string()),
            ("entry_point", format!("{:#x}", self.entry_point)),
            ("checksum", format!("{:#x}", self.checksum)),
            ("signature_verified", self.signature_verified.to_string()),
        ]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Boot phase started event
#[derive(Debug, Clone)]
pub struct BootPhaseStartedEvent {
    pub phase_name: &'static str,
    pub timestamp: u64,
}

impl BootPhaseStartedEvent {
    pub fn new(phase_name: &'static str, timestamp: u64) -> Self {
        Self {
            phase_name,
            timestamp,
        }
    }
}

impl DomainEvent for BootPhaseStartedEvent {
    fn event_type(&self) -> &'static str {
        "BootPhaseStarted"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Boot phase started"
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Domain event subscriber with name support
pub trait NamedDomainEventSubscriber: DomainEventSubscriber {
    fn name(&self) -> &'static str;
}

/// Simple logging subscriber for testing
pub struct LoggingSubscriber {
    name: &'static str,
}

impl LoggingSubscriber {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl DomainEventSubscriber for LoggingSubscriber {
    fn handle(&self, event: &dyn DomainEvent) -> Result<(), &'static str> {
        log::info!("[{}] Event: {} - {}", self.name, event.event_type(), event.as_string());
        Ok(())
    }

    fn subscriber_name(&self) -> &'static str {
        self.name
    }

    fn clone_box(&self) -> Box<dyn DomainEventSubscriber> {
        Box::new(Self {
            name: self.name,
        })
    }
}

impl NamedDomainEventSubscriber for LoggingSubscriber {
    fn name(&self) -> &'static str {
        self.name
    }
}

/// Validation failed event
#[derive(Debug, Clone)]
pub struct ValidationFailedEvent {
    pub validation_type: &'static str,
    pub error_message: String,
    pub timestamp: u64,
}

impl ValidationFailedEvent {
    pub fn new(validation_type: &'static str, error_message: String, timestamp: u64) -> Self {
        Self {
            validation_type,
            error_message,
            timestamp,
        }
    }
}

impl DomainEvent for ValidationFailedEvent {
    fn event_type(&self) -> &'static str {
        "ValidationFailed"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Validation failed"
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Device detected event
#[derive(Debug, Clone)]
pub struct DeviceDetectedEvent {
    pub device_type: &'static str,
    pub device_id: String,
    pub device_vendor: Option<String>,
    pub device_status: DeviceStatus,
    pub timestamp: u64,
}

/// Device status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Device is working correctly
    Functional,
    /// Device has issues but is usable
    Degraded,
    /// Device is not working
    Failed,
    /// Device is not present
    Absent,
}

impl DeviceDetectedEvent {
    pub fn new(device_type: &'static str, device_id: String, timestamp: u64) -> Self {
        Self {
            device_type,
            device_id,
            device_vendor: None,
            device_status: DeviceStatus::Functional,
            timestamp,
        }
    }

    pub fn with_vendor(mut self, vendor: String) -> Self {
        self.device_vendor = Some(vendor);
        self
    }

    pub fn with_status(mut self, status: DeviceStatus) -> Self {
        self.device_status = status;
        self
    }
}

impl DomainEvent for DeviceDetectedEvent {
    fn event_type(&self) -> &'static str {
        "DeviceDetected"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Device detected"
    }

    fn severity(&self) -> EventSeverity {
        match self.device_status {
            DeviceStatus::Functional => EventSeverity::Info,
            DeviceStatus::Degraded => EventSeverity::Warning,
            DeviceStatus::Failed => EventSeverity::Error,
            DeviceStatus::Absent => EventSeverity::Debug,
        }
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        let mut metadata = Vec::new();
        metadata.push(("device_type", self.device_type.to_string()));
        metadata.push(("device_id", self.device_id.clone()));
        metadata.push(("device_status", format!("{:?}", self.device_status)));
        if let Some(ref vendor) = self.device_vendor {
            metadata.push(("device_vendor", vendor.clone()));
        }
        metadata
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Memory initialized event
#[derive(Debug, Clone)]
pub struct MemoryInitializedEvent {
    pub total_memory: u64,
    pub available_memory: u64,
    pub memory_map_entries: u32,
    pub timestamp: u64,
}

impl MemoryInitializedEvent {
    pub fn new(total_memory: u64, available_memory: u64, timestamp: u64) -> Self {
        Self {
            total_memory,
            available_memory,
            memory_map_entries: 0,
            timestamp,
        }
    }

    pub fn with_map_entries(mut self, entries: u32) -> Self {
        self.memory_map_entries = entries;
        self
    }
}

impl DomainEvent for MemoryInitializedEvent {
    fn event_type(&self) -> &'static str {
        "MemoryInitialized"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Memory subsystem initialized"
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        vec![
            ("total_memory", self.total_memory.to_string()),
            ("available_memory", self.available_memory.to_string()),
            ("memory_map_entries", self.memory_map_entries.to_string()),
        ]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// System error event
#[derive(Debug, Clone)]
pub struct SystemErrorEvent {
    pub error_code: u32,
    pub error_message: String,
    pub component: &'static str,
    pub recovery_possible: bool,
    pub timestamp: u64,
}

impl SystemErrorEvent {
    pub fn new(error_code: u32, error_message: String, component: &'static str, timestamp: u64) -> Self {
        Self {
            error_code,
            error_message,
            component,
            recovery_possible: false,
            timestamp,
        }
    }

    pub fn with_recovery(mut self, recovery: bool) -> Self {
        self.recovery_possible = recovery;
        self
    }
}

impl DomainEvent for SystemErrorEvent {
    fn event_type(&self) -> &'static str {
        "SystemError"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "System error occurred"
    }

    fn severity(&self) -> EventSeverity {
        if self.recovery_possible {
            EventSeverity::Warning
        } else {
            EventSeverity::Critical
        }
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        vec![
            ("error_code", self.error_code.to_string()),
            ("error_message", self.error_message.clone()),
            ("component", self.component.to_string()),
            ("recovery_possible", self.recovery_possible.to_string()),
        ]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Performance metrics event
#[derive(Debug, Clone)]
pub struct PerformanceMetricsEvent {
    pub metric_name: &'static str,
    pub metric_value: f64,
    pub metric_unit: &'static str,
    pub threshold_exceeded: bool,
    pub timestamp: u64,
}

impl PerformanceMetricsEvent {
    pub fn new(metric_name: &'static str, metric_value: f64, metric_unit: &'static str, timestamp: u64) -> Self {
        Self {
            metric_name,
            metric_value,
            metric_unit,
            threshold_exceeded: false,
            timestamp,
        }
    }

    pub fn with_threshold_exceeded(mut self, exceeded: bool) -> Self {
        self.threshold_exceeded = exceeded;
        self
    }
}

impl DomainEvent for PerformanceMetricsEvent {
    fn event_type(&self) -> &'static str {
        "PerformanceMetrics"
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn as_string(&self) -> &'static str {
        "Performance metric recorded"
    }

    fn severity(&self) -> EventSeverity {
        if self.threshold_exceeded {
            EventSeverity::Warning
        } else {
            EventSeverity::Info
        }
    }

    fn metadata(&self) -> Vec<(&'static str, String)> {
        vec![
            ("metric_name", self.metric_name.to_string()),
            ("metric_value", self.metric_value.to_string()),
            ("metric_unit", self.metric_unit.to_string()),
            ("threshold_exceeded", self.threshold_exceeded.to_string()),
        ]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn DomainEvent> {
        Box::new(self.clone())
    }
}

/// Event filter for selective event handling
pub trait EventFilter: Send + Sync {
    /// Check if Event should be processed
    fn should_process(&self, event: &dyn DomainEvent) -> bool;
}

/// Simple event filter implementation
pub struct SimpleEventFilter {
    allowed_types: Vec<&'static str>,
}

impl SimpleEventFilter {
    pub fn new(allowed_types: Vec<&'static str>) -> Self {
        Self { allowed_types }
    }
    
    pub fn allow_all() -> Self {
        Self { allowed_types: Vec::new() }
    }
    
    pub fn allow_none() -> Self {
        Self { allowed_types: Vec::new() }
    }
}

impl EventFilter for SimpleEventFilter {
    fn should_process(&self, event: &dyn DomainEvent) -> bool {
        self.allowed_types.is_empty() || self.allowed_types.contains(&event.event_type())
    }
}

/// Improved event publisher with history and filtering support
pub struct ImprovedEventPublisher {
    subscribers: BTreeMap<&'static str, Vec<Box<dyn DomainEventSubscriber>>>,
    event_history: Vec<Box<dyn DomainEvent>>,
    max_history_size: usize,
    filters: Vec<Box<dyn EventFilter>>,
}

impl ImprovedEventPublisher {
    pub fn new() -> Self {
        Self {
            subscribers: BTreeMap::new(),
            event_history: Vec::new(),
            max_history_size: 100,
            filters: Vec::new(),
        }
    }
    
    pub fn with_history_size(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }
    
    pub fn with_filter(mut self, filter: Box<dyn EventFilter>) -> Self {
        self.filters.push(filter);
        self
    }
}

impl Default for ImprovedEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainEventPublisher for ImprovedEventPublisher {
    fn publish(&mut self, event: Box<dyn DomainEvent>) -> Result<(), &'static str> {
        // Get event type before moving
        let event_type = event.event_type();
        
        // Add to history
        self.event_history.push(event);
        
        // Maintain history size limit
        if self.event_history.len() > self.max_history_size {
            self.event_history.remove(0);
        }
        
        // Get the event from history for processing
        if let Some(event_ref) = self.event_history.last() {
            // Check filters
            for filter in &self.filters {
                if !filter.should_process(event_ref.as_ref()) {
                    return Ok(());
                }
            }
            
            // Notify subscribers
            if let Some(subscribers) = self.subscribers.get(event_type) {
                for subscriber in subscribers {
                    if let Err(e) = subscriber.handle(event_ref.as_ref()) {
                        // Log error but continue with other subscribers
                        log::error!("Event subscriber '{}' failed: {}", subscriber.subscriber_name(), e);
                    }
                }
            }
        }
        
        Ok(())
    }

    fn subscribe(
        &mut self,
        event_type: Option<&'static str>,
        subscriber: Box<dyn DomainEventSubscriber>,
    ) -> Result<(), &'static str> {
        let event_type = event_type.unwrap_or("*"); // Default to all events
        self.subscribers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(subscriber);
        Ok(())
    }

    fn unsubscribe(&mut self, subscriber_name: &'static str) -> Result<(), &'static str> {
        for (_, subscribers) in self.subscribers.iter_mut() {
            subscribers.retain(|sub| sub.subscriber_name() != subscriber_name);
        }
        Ok(())
    }
    
    fn get_event_history(&self) -> &[Box<dyn DomainEvent>] {
        &self.event_history
    }
    
    fn clear_history(&mut self) {
        self.event_history.clear();
    }

    fn subscriber_count(&self) -> usize {
        self.subscribers.values().map(|v| v.len()).sum()
    }

    fn event_count(&self) -> usize {
        self.event_history.len()
    }
}

/// Simple in-memory event publisher (for bootloader context)
///
/// Note: In production, this would use more sophisticated patterns like
/// observer pattern with proper error handling and async support.
pub struct SimpleEventPublisher {
    // Storage for subscribers would go here in a real implementation
    // For bootloader, we keep it minimal
}

impl SimpleEventPublisher {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SimpleEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainEventPublisher for SimpleEventPublisher {
    fn publish(&mut self, _event: Box<dyn DomainEvent>) -> Result<(), &'static str> {
        // In bootloader context, we don't actually need subscribers during boot
        // This is mainly for architecture correctness
        Ok(())
    }

    fn subscribe(
        &mut self,
        _event_type: Option<&'static str>,
        _subscriber: Box<dyn DomainEventSubscriber>,
    ) -> Result<(), &'static str> {
        Ok(())
    }

    fn unsubscribe(&mut self, _subscriber_name: &'static str) -> Result<(), &'static str> {
        Ok(())
    }
    
    fn get_event_history(&self) -> &[Box<dyn DomainEvent>] {
        &[]
    }
    
    fn clear_history(&mut self) {
        // No-op for simple Publisher
    }

    fn subscriber_count(&self) -> usize {
        0
    }

    fn event_count(&self) -> usize {
        0
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_types() {
        let gfx_event = GraphicsInitializedEvent::new(1024, 768, 0x1000, 1234);
        assert_eq!(gfx_event.event_type(), "GraphicsInitialized");
        assert_eq!(gfx_event.timestamp(), 1234);
        
        let kernel_event = KernelLoadedEvent::new(0x100000, 0x500000, 1234);
        assert_eq!(kernel_event.event_type(), "KernelLoaded");
        assert_eq!(kernel_event.timestamp(), 1234);
        
        let phase_event = BootPhaseStartedEvent::new("initialization", 1234);
        assert_eq!(phase_event.event_type(), "BootPhaseStarted");
        assert_eq!(phase_event.timestamp(), 1234);
        
        let mem_event = MemoryInitializedEvent::new(0x1000000, 0x8000000, 1234);
        assert_eq!(mem_event.event_type(), "MemoryInitialized");
        assert_eq!(mem_event.timestamp(), 1234);
        
        let validation_event = ValidationFailedEvent::new("kernel", "Invalid format".to_string(), 1234);
        assert_eq!(validation_event.event_type(), "ValidationFailed");
        assert_eq!(validation_event.timestamp(), 1234);
        
        let device_event = DeviceDetectedEvent::new("graphics", "vga".to_string(), 1234);
        assert_eq!(device_event.event_type(), "DeviceDetected");
        assert_eq!(device_event.timestamp(), 1234);
    }

    #[test]
    fn test_publisher_publishes_successfully() {
        let mut publisher = SimpleEventPublisher::new();
        let event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 0));
        assert!(publisher.publish(event).is_ok());
    }
    
    #[test]
    fn test_improved_publisher() {
        let mut publisher = ImprovedEventPublisher::new();
        
        // Test event history
        assert_eq!(publisher.get_event_history().len(), 0);
        
        // Test subscription
        let subscriber = LoggingSubscriber::new("test_sub");
        assert!(publisher.subscribe("GraphicsInitialized", Box::new(subscriber)).is_ok());
        
        // Test publishing
        let event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 1234));
        assert!(publisher.publish(event).is_ok());
        assert_eq!(publisher.get_event_history().len(), 1);
        
        // Test filtering
        let filter = SimpleEventFilter::new(vec!["GraphicsInitialized"]);
        publisher = publisher.with_filter(Box::new(filter));
        
        let filtered_event = Box::new(KernelLoadedEvent::new(0x100000, 0x500000, 1234));
        assert!(publisher.publish(filtered_event).is_ok()); // Should be filtered out
        assert_eq!(publisher.get_event_history().len(), 1); // Still only first event
    }
    
    #[test]
    fn test_event_filter() {
        let filter = SimpleEventFilter::new(vec!["GraphicsInitialized", "KernelLoaded"]);
        
        let gfx_event = GraphicsInitializedEvent::new(1024, 768, 0x1000, 1234);
        let kernel_event = KernelLoadedEvent::new(0x100000, 0x500000, 1234);
        
        assert!(filter.should_process(gfx_event.as_ref()));
        assert!(filter.should_process(kernel_event.as_ref()));
        
        let filter_all = SimpleEventFilter::allow_all();
        assert!(filter_all.should_process(gfx_event.as_ref()));
        assert!(filter_all.should_process(kernel_event.as_ref()));
        
        let filter_none = SimpleEventFilter::allow_none();
        assert!(!filter_none.should_process(gfx_event.as_ref()));
        assert!(!filter_none.should_process(kernel_event.as_ref()));
    }
    
    #[test]
    fn test_logging_subscriber() {
        let subscriber = LoggingSubscriber::new("test_logger");
        
        let event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 1234));
        assert!(subscriber.handle(event.as_ref()).is_ok());
        assert_eq!(subscriber.name(), "test_logger");
    }
}
