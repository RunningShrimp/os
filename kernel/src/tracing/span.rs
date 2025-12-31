//! # Span Implementation
//!
//! This module provides the core span implementation for distributed tracing.
//! Spans represent operations within the system and track their duration,
//! metadata, and relationships.
//!
//! # Architecture
//!
//! - **Span**: Represents a single operation with timing and metadata
//! - **SpanContext**: Contains trace and span IDs for propagation
//! - **SpanBuilder**: Fluent API for constructing spans
//! - **Event**: Timestamped annotations within a span
//! - **Links**: Relationships to spans in other traces
//!
//! # Example
//!
//! ```rust
//! use kernel::tracing::span::{Span, SpanBuilder};
//!
//! let span = SpanBuilder::new("process_request")
//!     .with_tag("http.method", "GET")
//!     .with_tag("http.url", "/api/users")
//!     .start();
//!
//! // Do work...
//!
//! span.end();
//! ```

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use crate::tracing::context::{SpanContext, TraceId};
use crate::tracing::TraceError;

/// Maximum number of events per span
const MAX_EVENTS_PER_SPAN: usize = 128;

/// Maximum number of links per span
const MAX_LINKS_PER_SPAN: usize = 32;

/// Maximum number of tags per span
const MAX_TAGS_PER_SPAN: usize = 64;

/// Maximum number of attributes per event
const MAX_EVENT_ATTRIBUTES: usize = 16;

/// Represents a single span in a distributed trace
///
/// A span represents a single operation within the system. It tracks:
/// - Operation name
/// - Start and end timestamps
/// - Parent-child relationships
/// - Tags/attributes
/// - Events/logs
/// - Links to other spans
///
/// # Thread Safety
///
/// Spans are designed to be used from a single thread for best performance.
/// For cross-thread spans, use `SpanContext` to propagate trace information.
pub struct Span {
    /// Span context (trace ID, span ID, parent ID)
    context: SpanContext,

    /// Operation name
    name: String,

    /// Span kind (client, server, internal, etc.)
    kind: SpanKind,

    /// Start timestamp (nanoseconds since epoch)
    start_time: u64,

    /// End timestamp (nanoseconds since epoch, 0 if not ended)
    end_time: UnsafeCell<u64>,

    /// Duration in nanoseconds
    duration: UnsafeCell<u64>,

    /// Span status
    status: UnsafeCell<SpanStatus>,

    /// Tags/attributes
    tags: UnsafeCell<Vec<SpanTag>>,

    /// Events within the span
    events: UnsafeCell<Vec<SpanEvent>>,

    /// Links to other spans
    links: Vec<SpanLink>,

    /// Whether the span has been ended
    ended: AtomicBool,

    /// Span baggage
    baggage: UnsafeCell<Vec<(String, String)>>,
}

unsafe impl Send for Span {}
unsafe impl Sync for Span {}

impl Span {
    /// Create a new span with the given name and context
    ///
    /// # Arguments
    ///
    /// * `name` - Operation name
    /// * `context` - Span context with trace and span IDs
    ///
    /// # Returns
    ///
    /// A new span that has been started
    #[inline]
    pub fn new(name: impl Into<String>, context: SpanContext) -> Self {
        Self {
            context,
            name: name.into(),
            kind: SpanKind::Internal,
            start_time: Self::now(),
            end_time: UnsafeCell::new(0),
            duration: UnsafeCell::new(0),
            status: UnsafeCell::new(SpanStatus::Ok),
            tags: UnsafeCell::new(Vec::with_capacity(8)),
            events: UnsafeCell::new(Vec::with_capacity(4)),
            links: Vec::new(),
            ended: AtomicBool::new(false),
            baggage: UnsafeCell::new(Vec::new()),
        }
    }

    /// Get current time in nanoseconds
    ///
    /// In a real kernel, this would read from the system clock.
    /// For now, we use a simplified implementation.
    #[inline]
    fn now() -> u64 {
        // This would be replaced with actual time source in production
        extern "C" {
            fn rust_currrent_time() -> u64;
        }
        unsafe { rust_currrent_time() }
    }

    /// Get the span's context
    #[inline]
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// Get the span name
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the span kind
    #[inline]
    pub fn kind(&self) -> SpanKind {
        self.kind
    }

    /// Set the span kind
    #[inline]
    pub fn set_kind(&mut self, kind: SpanKind) {
        self.kind = kind;
    }

    /// Get the start time
    #[inline]
    pub fn start_time(&self) -> u64 {
        self.start_time
    }

    /// Get the end time
    ///
    /// Returns 0 if the span has not been ended
    #[inline]
    pub fn end_time(&self) -> u64 {
        unsafe { *self.end_time.get() }
    }

    /// Get the duration
    ///
    /// Returns 0 if the span has not been ended
    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_nanos(unsafe { *self.duration.get() })
    }

    /// Get the span status
    #[inline]
    pub fn status(&self) -> SpanStatus {
        unsafe { *self.status.get() }
    }

    /// Set the span status
    #[inline]
    pub fn set_status(&self, status: SpanStatus) {
        unsafe {
            *self.status.get() = status;
        }
    }

    /// Add a tag to the span
    ///
    /// # Arguments
    ///
    /// * `key` - Tag key
    /// * `value` - Tag value
    ///
    /// # Returns
    ///
    /// Ok(()) if tag was added, Err if limit exceeded
    pub fn set_tag(&self, key: impl Into<String>, value: impl Into<String>) -> Result<(), TraceError> {
        let tags = unsafe { &mut *self.tags.get() };
        if tags.len() >= MAX_TAGS_PER_SPAN {
            return Err(TraceError::TagLimitExceeded);
        }
        tags.push(SpanTag {
            key: key.into(),
            value: value.into(),
        });
        Ok(())
    }

    /// Get all tags
    pub fn tags(&self) -> Vec<(String, String)> {
        let tags = unsafe { &*self.tags.get() };
        tags.iter().map(|t| (t.key.clone(), t.value.clone())).collect()
    }

    /// Record an event within the span
    ///
    /// # Arguments
    ///
    /// * `name` - Event name
    /// * `attributes` - Optional event attributes
    ///
    /// # Returns
    ///
    /// Ok(()) if event was recorded, Err if limit exceeded
    pub fn add_event(
        &self,
        name: impl Into<String>,
        attributes: Option<Vec<(String, String)>>,
    ) -> Result<(), TraceError> {
        let events = unsafe { &mut *self.events.get() };
        if events.len() >= MAX_EVENTS_PER_SPAN {
            return Err(TraceError::EventLimitExceeded);
        }

        let attrs = attributes.unwrap_or_default();
        if attrs.len() > MAX_EVENT_ATTRIBUTES {
            return Err(TraceError::AttributeLimitExceeded);
        }

        events.push(SpanEvent {
            timestamp: Self::now(),
            name: name.into(),
            attributes: attrs,
        });
        Ok(())
    }

    /// Get all events
    pub fn events(&self) -> Vec<SpanEvent> {
        let events = unsafe { &*self.events.get() };
        events.clone()
    }

    /// Add a link to another span
    ///
    /// # Arguments
    ///
    /// * `context` - Context of the linked span
    /// * `attributes` - Optional link attributes
    ///
    /// # Returns
    ///
    /// Ok(()) if link was added, Err if limit exceeded
    pub fn add_link(
        &mut self,
        context: SpanContext,
        attributes: Option<Vec<(String, String)>>,
    ) -> Result<(), TraceError> {
        if self.links.len() >= MAX_LINKS_PER_SPAN {
            return Err(TraceError::LinkLimitExceeded);
        }
        self.links.push(SpanLink {
            context,
            attributes: attributes.unwrap_or_default(),
        });
        Ok(())
    }

    /// Get all links
    pub fn links(&self) -> &[SpanLink] {
        &self.links
    }

    /// Set baggage item
    ///
    /// # Arguments
    ///
    /// * `key` - Baggage key
    /// * `value` - Baggage value
    pub fn set_baggage(&self, key: impl Into<String>, value: impl Into<String>) {
        let baggage = unsafe { &mut *self.baggage.get() };
        let key_str = key.into();
        // Remove existing key if present
        baggage.retain(|(k, _)| k != &key_str);
        baggage.push((key_str, value.into()));
    }

    /// Get baggage value
    ///
    /// # Arguments
    ///
    /// * `key` - Baggage key
    ///
    /// # Returns
    ///
    /// The baggage value if present
    pub fn get_baggage(&self, key: &str) -> Option<String> {
        let baggage = unsafe { &*self.baggage.get() };
        baggage.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// Get all baggage
    pub fn baggage(&self) -> Vec<(String, String)> {
        let baggage = unsafe { &*self.baggage.get() };
        baggage.clone()
    }

    /// End the span
    ///
    /// This marks the span as complete and records the end time.
    /// Can only be called once per span.
    ///
    /// # Returns
    ///
    /// Ok(()) if span was ended successfully, Err if already ended
    pub fn end(&self) -> Result<(), TraceError> {
        if self.ended.swap(true, Ordering::SeqCst) {
            return Err(TraceError::SpanAlreadyEnded);
        }

        let end = Self::now();
        unsafe {
            *self.end_time.get() = end;
            *self.duration.get() = end.saturating_sub(self.start_time);
        }
        Ok(())
    }

    /// Check if the span has ended
    #[inline]
    pub fn is_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    /// Annotate the span with a message
    ///
    /// Convenience method for adding a simple event
    #[inline]
    pub fn annotate(&self, message: impl Into<String>) -> Result<(), TraceError> {
        self.add_event(message, None)
    }

    /// Record an error in the span
    ///
    /// Sets the span status to error and adds an error event
    pub fn record_error(&self, error: impl Into<String>) {
        let error_msg = error.into();
        self.set_status(SpanStatus::Error {
            message: error_msg.clone(),
        });
        let _ = self.add_event("error", Some(vec![("error.message".to_string(), error_msg)]));
    }

    /// Create a child span
    ///
    /// # Arguments
    ///
    /// * `name` - Child span name
    ///
    /// # Returns
    ///
    /// A new child span with this span as parent
    pub fn child(&self, name: impl Into<String>) -> Self {
        let child_context = SpanContext::child(&self.context);
        let mut child = Span::new(name, child_context);
        child.kind = self.kind;
        child
    }
}

impl core::fmt::Debug for Span {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Span")
            .field("context", &self.context)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("start_time", &self.start_time)
            .field("end_time", &self.end_time())
            .field("duration", &self.duration())
            .field("status", &self.status())
            .field("tags", &unsafe { &*self.tags.get() })
            .field("events", &unsafe { &*self.events.get() })
            .field("links", &self.links)
            .field("ended", &self.ended)
            .finish()
    }
}

/// Span builder for fluent API
pub struct SpanBuilder {
    name: String,
    context: Option<SpanContext>,
    kind: SpanKind,
    parent_context: Option<SpanContext>,
    tags: Vec<(String, String)>,
    links: Vec<(SpanContext, Vec<(String, String)>)>,
    start Immediately,
}

/// Whether to start the span immediately or defer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Immediately {
    Yes,
    No,
}

impl SpanBuilder {
    /// Create a new span builder
    ///
    /// # Arguments
    ///
    /// * `name` - Operation name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            context: None,
            kind: SpanKind::Internal,
            parent_context: None,
            tags: Vec::with_capacity(8),
            links: Vec::new(),
            start: Immediately::Yes,
        }
    }

    /// Set the span context
    ///
    /// If not set, a new context will be generated
    pub fn with_context(mut self, context: SpanContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Set the parent context
    ///
    /// The new span will be a child of this context
    pub fn with_parent(mut self, parent: SpanContext) -> Self {
        self.parent_context = Some(parent);
        self
    }

    /// Set the span kind
    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.push((key.into(), value.into()));
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: Vec<(String, String)>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Add a link to another span
    pub fn with_link(mut self, context: SpanContext, attributes: Vec<(String, String)>) -> Self {
        self.links.push((context, attributes));
        self
    }

    /// Defer starting the span
    pub fn defer_start(mut self) -> Self {
        self.start = Immediately::No;
        self
    }

    /// Build and start the span
    pub fn start(self) -> Span {
        let context = self.context.unwrap_or_else(|| {
            if let Some(parent) = self.parent_context {
                SpanContext::child(&parent)
            } else {
                SpanContext::new()
            }
        });

        let mut span = Span::new(self.name, context);
        span.kind = self.kind;

        // Add tags
        for (key, value) in self.tags {
            let _ = span.set_tag(key, value);
        }

        // Add links
        for (link_context, attrs) in self.links {
            let _ = span.add_link(link_context, Some(attrs));
        }

        span
    }
}

/// Span kind defines the relationship between the span and its parent
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpanKind {
    /// Internal operation (no remote parent)
    Internal,

    /// Client span (outbound request)
    Client,

    /// Server span (inbound request)
    Server,

    /// Producer span (message publisher)
    Producer,

    /// Consumer span (message subscriber)
    Consumer,

    /// Span representing an invocation of a function/method
    InternalInvocation,
}

impl SpanKind {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanKind::Internal => "internal",
            SpanKind::Client => "client",
            SpanKind::Server => "server",
            SpanKind::Producer => "producer",
            SpanKind::Consumer => "consumer",
            SpanKind::InternalInvocation => "internal_invocation",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "internal" => Some(SpanKind::Internal),
            "client" => Some(SpanKind::Client),
            "server" => Some(SpanKind::Server),
            "producer" => Some(SpanKind::Producer),
            "consumer" => Some(SpanKind::Consumer),
            "internal_invocation" => Some(SpanKind::InternalInvocation),
            _ => None,
        }
    }
}

/// Span status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpanStatus {
    /// Operation completed successfully
    Ok,

    /// Operation completed with an error
    Error {
        /// Error message
        message: String,
    },

    /// Operation was cancelled
    Cancelled,

    /// Operation is still ongoing
    Unset,
}

/// Span tag (key-value attribute)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanTag {
    pub key: String,
    pub value: String,
}

/// Event within a span
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanEvent {
    /// Event timestamp
    pub timestamp: u64,

    /// Event name
    pub name: String,

    /// Event attributes
    pub attributes: Vec<(String, String)>,
}

/// Link to another span
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpanLink {
    /// Linked span context
    pub context: SpanContext,

    /// Link attributes
    pub attributes: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        assert_eq!(span.name(), "test_operation");
        assert_eq!(span.kind(), SpanKind::Internal);
        assert!(!span.is_ended());
        assert_eq!(span.status(), SpanStatus::Ok);
    }

    #[test]
    fn test_span_lifecycle() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        assert!(!span.is_ended());
        assert_eq!(span.duration().as_nanos(), 0);

        // End the span
        assert!(span.end().is_ok());
        assert!(span.is_ended());
        assert!(span.duration().as_nanos() > 0);

        // Cannot end twice
        assert!(matches!(span.end(), Err(TraceError::SpanAlreadyEnded)));
    }

    #[test]
    fn test_span_tags() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        assert!(span.set_tag("key1", "value1").is_ok());
        assert!(span.set_tag("key2", "value2").is_ok());

        let tags = span.tags();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], (String::from("key1"), String::from("value1")));
    }

    #[test]
    fn test_tag_limit() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        for i in 0..=MAX_TAGS_PER_SPAN {
            let result = span.set_tag(format!("key{}", i), format!("value{}", i));
            if i < MAX_TAGS_PER_SPAN {
                assert!(result.is_ok());
            } else {
                assert!(matches!(result, Err(TraceError::TagLimitExceeded)));
            }
        }
    }

    #[test]
    fn test_span_events() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        assert!(span
            .add_event("event1", Some(vec![("attr1".to_string(), "val1".to_string())]))
            .is_ok());

        let events = span.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "event1");
        assert_eq!(events[0].attributes.len(), 1);
    }

    #[test]
    fn test_event_limit() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        for i in 0..=MAX_EVENTS_PER_SPAN {
            let result = span.add_event(format!("event{}", i), None);
            if i < MAX_EVENTS_PER_SPAN {
                assert!(result.is_ok());
            } else {
                assert!(matches!(result, Err(TraceError::EventLimitExceeded)));
            }
        }
    }

    #[test]
    fn test_span_status() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        assert_eq!(span.status(), SpanStatus::Ok);

        span.set_status(SpanStatus::Cancelled);
        assert_eq!(span.status(), SpanStatus::Cancelled);
    }

    #[test]
    fn test_record_error() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        span.record_error("Something went wrong");

        assert!(matches!(span.status(), SpanStatus::Error { .. }));
        let events = span.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "error");
    }

    #[test]
    fn test_span_annotate() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        assert!(span.annotate("checkpoint 1").is_ok());
        assert!(span.annotate("checkpoint 2").is_ok());

        let events = span.events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_span_baggage() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        span.set_baggage("user.id", "12345");
        span.set_baggage("tenant.id", "abcde");

        assert_eq!(span.get_baggage("user.id"), Some(String::from("12345")));
        assert_eq!(span.get_baggage("tenant.id"), Some(String::from("abcde")));
        assert_eq!(span.get_baggage("nonexistent"), None);

        // Update existing baggage
        span.set_baggage("user.id", "67890");
        assert_eq!(span.get_baggage("user.id"), Some(String::from("67890")));
    }

    #[test]
    fn test_span_links() {
        let context1 = SpanContext::new();
        let context2 = SpanContext::new();
        let mut span = Span::new("test_operation", SpanContext::new());

        assert!(span
            .add_link(context1, Some(vec![("link_type".to_string(), "related".to_string())]))
            .is_ok());
        assert!(span.add_link(context2, None).is_ok());

        assert_eq!(span.links().len(), 2);
    }

    #[test]
    fn test_link_limit() {
        let context = SpanContext::new();
        let mut span = Span::new("test_operation", SpanContext::new());

        for i in 0..=MAX_LINKS_PER_SPAN {
            let result = span.add_link(SpanContext::new(), None);
            if i < MAX_LINKS_PER_SPAN {
                assert!(result.is_ok());
            } else {
                assert!(matches!(result, Err(TraceError::LinkLimitExceeded)));
            }
        }
    }

    #[test]
    fn test_child_span() {
        let parent_context = SpanContext::new();
        let parent = Span::new("parent", parent_context);

        let child = parent.child("child");

        assert_eq!(child.name(), "child");
        assert_eq!(child.context().trace_id(), parent.context().trace_id());
        assert_ne!(child.context().span_id(), parent.context().span_id());
        assert_eq!(
            child.context().parent_span_id(),
            Some(parent.context().span_id())
        );
    }

    #[test]
    fn test_span_builder() {
        let parent_context = SpanContext::new();
        let span = SpanBuilder::new("test_operation")
            .with_kind(SpanKind::Server)
            .with_tag("http.method", "GET")
            .with_tag("http.status", "200")
            .with_parent(parent_context)
            .start();

        assert_eq!(span.name(), "test_operation");
        assert_eq!(span.kind(), SpanKind::Server);
        assert_eq!(span.tags().len(), 2);
        assert_eq!(
            span.context().parent_span_id(),
            Some(parent_context.span_id())
        );
    }

    #[test]
    fn test_span_kind_conversion() {
        assert_eq!(SpanKind::Internal.as_str(), "internal");
        assert_eq!(SpanKind::Client.as_str(), "client");
        assert_eq!(SpanKind::Server.as_str(), "server");

        assert_eq!(SpanKind::from_str("internal"), Some(SpanKind::Internal));
        assert_eq!(SpanKind::from_str("server"), Some(SpanKind::Server));
        assert_eq!(SpanKind::from_str("invalid"), None);
    }

    #[test]
    fn test_span_debug() {
        let context = SpanContext::new();
        let span = Span::new("test_operation", context);

        let debug_str = format!("{:?}", span);
        assert!(debug_str.contains("test_operation"));
    }
}
