//! Distributed Tracing (OpenTelemetry compatible)
//!
//! This module provides distributed tracing capabilities compatible with OpenTelemetry standards.
//! It enables tracking request flows across the kernel subsystems, collecting performance data,
//! and maintaining causality between asynchronous operations.
//!
//! ## Key Features
//!
//! - **OpenTelemetry Compatibility**: Export traces in OpenTelemetry format
//! - **Span Management**: Create, track, and manage spans with parent-child relationships
//! - **Context Propagation**: Inject and extract span context across boundaries
//! - **Sampling**: Configurable sampling rate to control overhead
//! - **Attributes**: Attach key-value metadata to spans for detailed analysis
//!
//! ## Architecture
//!
//! The tracing system consists of:
//! - [`Span`]: Represents a unit of work with timing and metadata
//! - [`SpanContext`]: Carries trace information across boundaries
//! - [`Tracer`]: Manages span lifecycle and sampling
//! - [`SpanHandle`]: Reference to an active span
//!
//! ## Example
//!
//! ```rust
//! use kernel::monitoring::tracing::{Tracer, SpanKind};
//!
//! let mut tracer = Tracer::new(0.1); // 10% sampling
//! let handle = tracer.start_span("syscall_handler");
//! // ... do work ...
//! tracer.end_span(handle);
//! let traces = tracer.export_traces();
//! ```

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::subsystems::sync::Mutex;

/// Wrapper for u128 atomic operations using spinlock
struct AtomicU128 {
    value: Mutex<u128>,
}

impl AtomicU128 {
    const fn new(value: u128) -> Self {
        Self {
            value: Mutex::new(value),
        }
    }

    fn fetch_add(&self, delta: u128, _ordering: Ordering) -> u128 {
        let mut val = self.value.lock();
        let old = *val;
        *val = val.wrapping_add(delta);
        old
    }
}

/// Maximum number of spans to keep in memory
const MAX_SPANS: usize = 10000;

/// Maximum number of attributes per span
const MAX_ATTRIBUTES: usize = 128;

/// Default sampling rate (10%)
const DEFAULT_SAMPLING_RATE: f32 = 0.1;

/// Unique identifier for a trace
pub type TraceId = u128;

/// Unique identifier for a span within a trace
pub type SpanId = u64;

/// Handle to an active span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanHandle {
    /// Index in the tracer's span vector
    index: u32,
    /// Generation count for validation
    generation: u32,
}

impl SpanHandle {
    /// Create a new span handle
    #[inline]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index
    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation
    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

/// Type of span
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// Span represents an internal operation
    Internal,
    /// Span represents a server-side operation
    Server,
    /// Span represents a client-side operation
    Client,
    /// Span represents a producer operation
    Producer,
    /// Span represents a consumer operation
    Consumer,
}

impl fmt::Display for SpanKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal => write!(f, "INTERNAL"),
            Self::Server => write!(f, "SERVER"),
            Self::Client => write!(f, "CLIENT"),
            Self::Producer => write!(f, "PRODUCER"),
            Self::Consumer => write!(f, "CONSUMER"),
        }
    }
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Operation completed successfully
    Ok,
    /// Operation failed with an error
    Error,
    /// Operation was cancelled
    Unset,
}

impl fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Error => write!(f, "ERROR"),
            Self::Unset => write!(f, "UNSET"),
        }
    }
}

/// A span represents a unit of work with timing and metadata
#[derive(Debug, Clone)]
pub struct Span {
    /// Unique trace identifier
    pub trace_id: TraceId,
    /// Unique span identifier
    pub span_id: SpanId,
    /// Parent span identifier (if any)
    pub parent_id: Option<SpanId>,
    /// Span name
    pub name: String,
    /// Span kind
    pub kind: SpanKind,
    /// Start time in nanoseconds
    pub start_time: u64,
    /// Duration in nanoseconds
    pub duration: u64,
    /// Span attributes
    pub attributes: BTreeMap<String, String>,
    /// Span status
    pub status: SpanStatus,
    /// Status description
    pub status_message: String,
    /// Events that occurred during the span
    pub events: Vec<SpanEvent>,
    /// Links to related spans
    pub links: Vec<SpanLink>,
    /// Whether the span is still active
    pub active: bool,
}

impl Span {
    /// Create a new span
    pub fn new(trace_id: TraceId, span_id: SpanId, name: String, kind: SpanKind) -> Self {
        Self {
            trace_id,
            span_id,
            parent_id: None,
            name,
            kind,
            start_time: crate::subsystems::time::hrtime_nanos(),
            duration: 0,
            attributes: BTreeMap::new(),
            status: SpanStatus::Unset,
            status_message: String::new(),
            events: Vec::new(),
            links: Vec::new(),
            active: true,
        }
    }

    /// Set the parent span
    #[inline]
    pub fn with_parent(mut self, parent_id: SpanId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Add an attribute to the span
    pub fn add_attribute(&mut self, key: String, value: String) -> Result<(), &'static str> {
        if self.attributes.len() >= MAX_ATTRIBUTES {
            return Err("Maximum number of attributes exceeded");
        }
        self.attributes.insert(key, value);
        Ok(())
    }

    /// Add multiple attributes
    pub fn add_attributes(&mut self, attrs: BTreeMap<String, String>) -> Result<(), &'static str> {
        if self.attributes.len() + attrs.len() > MAX_ATTRIBUTES {
            return Err("Maximum number of attributes exceeded");
        }
        for (key, value) in attrs {
            self.attributes.insert(key, value);
        }
        Ok(())
    }

    /// Set span status
    #[inline]
    pub fn set_status(&mut self, status: SpanStatus, message: String) {
        self.status = status;
        self.status_message = message;
    }

    /// Add an event to the span
    pub fn add_event(&mut self, event: SpanEvent) {
        self.events.push(event);
    }

    /// Add a link to another span
    pub fn add_link(&mut self, link: SpanLink) {
        self.links.push(link);
    }

    /// Check if span is sampled
    #[inline]
    pub fn is_sampled(&self) -> bool {
        !self.attributes.contains_key("otel.sampled") || self.attributes["otel.sampled"] == "true"
    }
}

/// Event that occurred during a span
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Event attributes
    pub attributes: BTreeMap<String, String>,
}

impl SpanEvent {
    /// Create a new span event
    pub fn new(name: String) -> Self {
        Self {
            name,
            timestamp: crate::subsystems::time::hrtime_nanos(),
            attributes: BTreeMap::new(),
        }
    }

    /// Add an attribute to the event
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }
}

/// Link to another span
#[derive(Debug, Clone)]
pub struct SpanLink {
    /// Linked trace ID
    pub trace_id: TraceId,
    /// Linked span ID
    pub span_id: SpanId,
    /// Link attributes
    pub attributes: BTreeMap<String, String>,
}

impl SpanLink {
    /// Create a new span link
    pub fn new(trace_id: TraceId, span_id: SpanId) -> Self {
        Self {
            trace_id,
            span_id,
            attributes: BTreeMap::new(),
        }
    }
}

/// Span context for propagation across boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanContext {
    /// Trace ID
    pub trace_id: TraceId,
    /// Span ID
    pub span_id: SpanId,
    /// Whether this span is sampled
    pub sampled: bool,
}

impl SpanContext {
    /// Create a new span context
    #[inline]
    pub const fn new(trace_id: TraceId, span_id: SpanId, sampled: bool) -> Self {
        Self {
            trace_id,
            span_id,
            sampled,
        }
    }

    /// Create an invalid span context
    #[inline]
    pub const fn invalid() -> Self {
        Self {
            trace_id: 0,
            span_id: 0,
            sampled: false,
        }
    }

    /// Check if the context is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.trace_id != 0 && self.span_id != 0
    }

    /// Serialize to string (traceparent format)
    pub fn to_string(&self) -> String {
        format!(
            "{:032x}-{:016x}-{:02x}",
            self.trace_id,
            self.span_id,
            if self.sampled { 1 } else { 0 }
        )
    }

    /// Parse from string (traceparent format)
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err("Invalid traceparent format");
        }

        let trace_id = u128::from_str_radix(parts[0], 16)
            .map_err(|_| "Invalid trace ID")?;
        let span_id = u64::from_str_radix(parts[1], 16)
            .map_err(|_| "Invalid span ID")?;
        let sampled = parts[2] == "01";

        Ok(Self {
            trace_id,
            span_id,
            sampled,
        })
    }
}

/// Tracer manages span lifecycle and sampling
pub struct Tracer {
    /// Collected spans
    spans: Mutex<Vec<Span>>,
    /// Sampling rate (0.0 to 1.0)
    sampling_rate: f32,
    /// Next trace ID
    next_trace_id: AtomicU128,
    /// Next span ID
    next_span_id: AtomicU64,
    /// Current active span context
    current_context: Mutex<SpanContext>,
    /// Span generation counts for validation
    generations: Mutex<Vec<u32>>,
    /// Total spans created
    total_spans: AtomicU64,
    /// Dropped spans due to capacity
    dropped_spans: AtomicU64,
}

impl Tracer {
    /// Create a new tracer with the given sampling rate
    pub fn new(sampling_rate: f32) -> Self {
        Self {
            spans: Mutex::new(Vec::with_capacity(MAX_SPANS)),
            sampling_rate: sampling_rate.clamp(0.0, 1.0),
            next_trace_id: AtomicU128::new(1),
            next_span_id: AtomicU64::new(1),
            current_context: Mutex::new(SpanContext::invalid()),
            generations: Mutex::new(Vec::with_capacity(MAX_SPANS)),
            total_spans: AtomicU64::new(0),
            dropped_spans: AtomicU64::new(0),
        }
    }

    /// Create a tracer with default sampling rate
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_SAMPLING_RATE)
    }

    /// Start a new span
    pub fn start_span(&self, name: &str) -> SpanHandle {
        self.start_span_with_kind(name, SpanKind::Internal)
    }

    /// Start a new span with a specific kind
    pub fn start_span_with_kind(&self, name: &str, kind: SpanKind) -> SpanHandle {
        // Generate IDs
        let trace_id = self.next_trace_id.fetch_add(1, Ordering::Relaxed);
        let span_id = self.next_span_id.fetch_add(1, Ordering::Relaxed);

        // Get parent context
        let parent_ctx = *self.current_context.lock();
        let parent_id = if parent_ctx.is_valid() {
            Some(parent_ctx.span_id)
        } else {
            None
        };

        // Determine sampling
        let sampled = self.should_sample();
        let trace_id = if parent_ctx.is_valid() {
            parent_ctx.trace_id
        } else {
            trace_id
        };

        // Create span
        let mut span = Span::new(trace_id, span_id, name.to_string(), kind);
        if let Some(pid) = parent_id {
            span.parent_id = Some(pid);
        }
        let _ = span.add_attribute("otel.sampled".to_string(), if sampled { "true" } else { "false" }.to_string());

        // Store span
        let mut spans = self.spans.lock();
        let mut generations = self.generations.lock();

        let index = if spans.len() < MAX_SPANS {
            spans.push(span);
            generations.push(0);
            spans.len() - 1
        } else {
            // Drop oldest span
            self.dropped_spans.fetch_add(1, Ordering::Relaxed);
            generations[0] = generations[0].wrapping_add(1);
            spans[0] = span;
            0
        };

        let generation = generations[index];
        let handle = SpanHandle::new(index as u32, generation);

        // Update current context if sampled
        if sampled {
            let ctx = SpanContext::new(trace_id, span_id, true);
            *self.current_context.lock() = ctx;
        }

        self.total_spans.fetch_add(1, Ordering::Relaxed);
        handle
    }

    /// End a span
    pub fn end_span(&self, handle: SpanHandle) {
        let mut spans = self.spans.lock();
        let generations = self.generations.lock();

        let index = handle.index() as usize;
        if index >= spans.len() {
            return;
        }

        // Validate generation
        if generations[index] != handle.generation() {
            return;
        }

        let span = &mut spans[index];
        if !span.active {
            return;
        }

        let now = crate::subsystems::time::hrtime_nanos();
        span.duration = now.saturating_sub(span.start_time);
        span.active = false;

        // Clear current context if this was the active span
        if span.span_id == self.current_context.lock().span_id {
            *self.current_context.lock() = SpanContext::invalid();
        }
    }

    /// End a span with a specific status
    pub fn end_span_with_status(&self, handle: SpanHandle, status: SpanStatus, message: String) {
        let mut spans = self.spans.lock();
        let generations = self.generations.lock();

        let index = handle.index() as usize;
        if index >= spans.len() {
            return;
        }

        if generations[index] != handle.generation() {
            return;
        }

        let span = &mut spans[index];
        span.set_status(status, message);
        span.active = false;

        let now = crate::subsystems::time::hrtime_nanos();
        span.duration = now.saturating_sub(span.start_time);

        if span.span_id == self.current_context.lock().span_id {
            *self.current_context.lock() = SpanContext::invalid();
        }
    }

    /// Extract the current span context
    pub fn extract_context(&self) -> SpanContext {
        *self.current_context.lock()
    }

    /// Inject a span context (e.g., from incoming request)
    pub fn inject_context(&self, ctx: SpanContext) {
        if ctx.is_valid() {
            *self.current_context.lock() = ctx;
        }
    }

    /// Add attributes to an active span
    pub fn add_attributes(&self, handle: SpanHandle, attrs: BTreeMap<String, String>) -> Result<(), &'static str> {
        let mut spans = self.spans.lock();
        let generations = self.generations.lock();

        let index = handle.index() as usize;
        if index >= spans.len() {
            return Err("Invalid span handle");
        }

        if generations[index] != handle.generation() {
            return Err("Span handle expired");
        }

        spans[index].add_attributes(attrs)
    }

    /// Add an event to an active span
    pub fn add_event(&self, handle: SpanHandle, name: String) -> Result<(), &'static str> {
        let mut spans = self.spans.lock();
        let generations = self.generations.lock();

        let index = handle.index() as usize;
        if index >= spans.len() {
            return Err("Invalid span handle");
        }

        if generations[index] != handle.generation() {
            return Err("Span handle expired");
        }

        spans[index].add_event(SpanEvent::new(name));
        Ok(())
    }

    /// Export traces in OpenTelemetry JSON format
    pub fn export_traces(&self) -> String {
        let spans = self.spans.lock();
        let mut output = String::from("{\"resourceSpans\": [{\"resource\": {\"attributes\": {\"service.name\": \"nos-kernel\"}},\"scopeSpans\": [{\"scope\": {\"name\": \"nos.tracing\"},\"spans\": [");

        let mut first = true;
        for span in spans.iter() {
            if !span.is_sampled() {
                continue;
            }

            if !first {
                output.push_str(",");
            }
            first = false;

            output.push_str(&self.format_span(span));
        }

        output.push_str("]}]}]}");
        output
    }

    /// Format a single span in OpenTelemetry format
    fn format_span(&self, span: &Span) -> String {
        let mut s = format!(
            "{{\"traceId\": \"{:032x}\",\"spanId\": \"{:016x}\",\"name\": \"{}\",\"kind\": \"{}\",\"startTimeUnixNano\": {},\"durationNano\": {}",
            span.trace_id,
            span.span_id,
            span.name,
            span.kind,
            span.start_time,
            span.duration
        );

        if let Some(parent_id) = span.parent_id {
            s.push_str(&format!(",\"parentSpanId\": \"{:016x}\"", parent_id));
        }

        if !span.attributes.is_empty() {
            s.push_str(",\"attributes\": [");
            let mut first = true;
            for (key, value) in &span.attributes {
                if !first {
                    s.push_str(",");
                }
                first = false;
                s.push_str(&format!("{{\"key\": \"{}\",\"value\": {{\"stringValue\": \"{}\"}}}}", key, value));
            }
            s.push_str("]");
        }

        if span.status != SpanStatus::Unset {
            s.push_str(&format!(
                ",\"status\": {{\"status\": \"{}\"}}",
                span.status
            ));
        }

        if !span.events.is_empty() {
            s.push_str(",\"events\": [");
            let mut first = true;
            for event in &span.events {
                if !first {
                    s.push_str(",");
                }
                first = false;
                s.push_str(&format!(
                    "{{\"name\": \"{}\",\"timeUnixNano\": {}}}",
                    event.name, event.timestamp
                ));
            }
            s.push_str("]");
        }

        if !span.links.is_empty() {
            s.push_str(",\"links\": [");
            let mut first = true;
            for link in &span.links {
                if !first {
                    s.push_str(",");
                }
                first = false;
                s.push_str(&format!(
                    "{{\"traceId\": \"{:032x}\",\"spanId\": \"{:016x}\"}}",
                    link.trace_id, link.span_id
                ));
            }
            s.push_str("]");
        }

        s.push_str("}");
        s
    }

    /// Get all spans
    pub fn get_spans(&self) -> Vec<Span> {
        self.spans.lock().clone()
    }

    /// Get spans by trace ID
    pub fn get_trace(&self, trace_id: TraceId) -> Vec<Span> {
        self.spans
            .lock()
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .cloned()
            .collect()
    }

    /// Clear all spans
    pub fn clear(&self) {
        self.spans.lock().clear();
        self.generations.lock().clear();
    }

    /// Get statistics
    pub fn get_stats(&self) -> TracerStats {
        TracerStats {
            total_spans: self.total_spans.load(Ordering::Relaxed),
            active_spans: self.spans.lock().iter().filter(|s| s.active).count(),
            dropped_spans: self.dropped_spans.load(Ordering::Relaxed),
            sampling_rate: self.sampling_rate,
        }
    }

    /// Determine if a span should be sampled
    fn should_sample(&self) -> bool {
        if self.sampling_rate >= 1.0 {
            return true;
        }
        if self.sampling_rate <= 0.0 {
            return false;
        }

        // Simple random sampling
        let random = (self.next_span_id.load(Ordering::Relaxed) % 100) as f32;
        random < (self.sampling_rate * 100.0)
    }
}

/// Tracer statistics
#[derive(Debug, Clone, Copy)]
pub struct TracerStats {
    /// Total spans created
    pub total_spans: u64,
    /// Currently active spans
    pub active_spans: usize,
    /// Spans dropped due to capacity
    pub dropped_spans: u64,
    /// Sampling rate
    pub sampling_rate: f32,
}

/// Global tracer instance
static GLOBAL_TRACER: Mutex<Option<Tracer>> = Mutex::new(None);

/// Initialize the global tracer
pub fn init_tracer(sampling_rate: f32) -> Result<(), &'static str> {
    let mut tracer = GLOBAL_TRACER.lock();
    if tracer.is_some() {
        return Err("Tracer already initialized");
    }
    *tracer = Some(Tracer::new(sampling_rate));
    crate::log_info!("[tracing] Distributed tracing initialized with {}% sampling", sampling_rate * 100.0);
    Ok(())
}

/// Get the global tracer
pub fn get_tracer() -> Option<&'static Tracer> {
    unsafe {
        // Extend lifetime to static - this is safe because GLOBAL_TRACER is never moved
        GLOBAL_TRACER.lock().as_ref().map(|t| {
            &*(t as *const Tracer)
        })
    }
}

/// Initialize tracer with defaults
pub fn init_tracer_defaults() -> Result<(), &'static str> {
    init_tracer(DEFAULT_SAMPLING_RATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_handle() {
        let handle = SpanHandle::new(5, 10);
        assert_eq!(handle.index(), 5);
        assert_eq!(handle.generation(), 10);
    }

    #[test]
    fn test_span_creation() {
        let span = Span::new(123, 456, "test_span".to_string(), SpanKind::Internal);
        assert_eq!(span.trace_id, 123);
        assert_eq!(span.span_id, 456);
        assert_eq!(span.name, "test_span");
        assert!(span.active);
        assert_eq!(span.parent_id, None);
    }

    #[test]
    fn test_span_with_parent() {
        let span = Span::new(123, 456, "child".to_string(), SpanKind::Internal)
            .with_parent(789);
        assert_eq!(span.parent_id, Some(789));
    }

    #[test]
    fn test_span_attributes() {
        let mut span = Span::new(123, 456, "test".to_string(), SpanKind::Internal);
        assert!(span.add_attribute("key1".to_string(), "value1".to_string()).is_ok());
        assert_eq!(span.attributes.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_span_context() {
        let ctx = SpanContext::new(123, 456, true);
        assert_eq!(ctx.trace_id, 123);
        assert_eq!(ctx.span_id, 456);
        assert!(ctx.sampled);
        assert!(ctx.is_valid());

        let invalid = SpanContext::invalid();
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_tracer_creation() {
        let tracer = Tracer::with_defaults();
        assert_eq!(tracer.sampling_rate, DEFAULT_SAMPLING_RATE);
    }

    #[test]
    fn test_tracer_start_end_span() {
        let tracer = Tracer::with_defaults();
        let handle = tracer.start_span("test_operation");
        tracer.end_span(handle);

        let spans = tracer.get_spans();
        assert_eq!(spans.len(), 1);
        assert!(!spans[0].active);
    }
}
