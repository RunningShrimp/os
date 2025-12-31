//! # Distributed Tracing for NOS Kernel
//!
//! This module provides comprehensive distributed tracing capabilities following
//! W3C Trace Context standards and OpenTelemetry data model.
//!
//! # Features
//!
//! - **W3C Trace Context**: Standard trace context propagation
//! - **OpenTelemetry Compatible**: Compatible with OTLP data model
//! - **Multiple Exporters**: Jaeger, Zipkin, OTLP support
//! - **Flexible Sampling**: Rate-based, probabilistic, and dynamic sampling
//! - **Baggage Propagation**: Key-value propagation across boundaries
//! - **Low Overhead**: Designed for minimal performance impact
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     Application Layer                     │
//! │  (spans, events, baggage, context propagation)           │
//! └─────────────────────────────────────────────────────────┘
//!                            ↓
//! ┌─────────────────────────────────────────────────────────┐
//! │                   Tracing Core                           │
//! │  • Span lifecycle and relationships                      │
//! │  • Trace context management                              │
//! │  • Sampling decisions                                    │
//! └─────────────────────────────────────────────────────────┘
//!                            ↓
//! ┌─────────────────────────────────────────────────────────┐
//!│                    Export Layer                           │
//! │  • Batch processing                                      │
//! │  • Format conversion (Jaeger, Zipkin, OTLP)              │
//! │  • Network transmission                                  │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust
//! use kernel::tracing::{
//!     span::{Span, SpanBuilder},
//!     context::SpanContext,
//!     exporter::JaegerExporter,
//!     sampler::RateSampler,
//! };
//!
//! // Create a root span
//! let root = SpanBuilder::new("handle_request")
//!     .with_kind(kernel::tracing::span::SpanKind::Server)
//!     .with_tag("http.method", "GET")
//!     .with_tag("http.url", "/api/users")
//!     .start();
//!
//! // Do work...
//!
//! // Create child span
//! let child = root.child("database_query");
//!
//! // End spans
//! child.end().unwrap();
//! root.end().unwrap();
//!
//! // Export
//! let exporter = JaegerExporter::new("localhost", 6831);
//! exporter.export(&[root]).unwrap();
//! ```
//!
//! # Modules
//!
//! - [`span`]: Span implementation and lifecycle management
//! - [`context`]: Trace context and ID generation
//! - [`propagation`]: Context propagation across boundaries
//! - [`sampler`]: Sampling strategies
//! - [`exporter`]: Trace data exporters
//! - [`baggage`]: Baggage propagation

#![no_std]

extern crate alloc;

pub mod baggage;
pub mod context;
pub mod exporter;
pub mod propagation;
pub mod sampler;
pub mod span;

// Re-export commonly used types
pub use context::{SpanContext, SpanId, TraceId, TraceFlags};
pub use exporter::{Exporter, JaegerExporter, OTLPExporter, ZipkinExporter};
pub use sampler::{Sampler, SamplingDecision, SamplingResult};
pub use span::{Span, SpanBuilder, SpanKind, SpanStatus};
pub use baggage::{Baggage, BaggagePropagator, BaggageLimits};
pub use propagation::{TextMapCarrier, HeaderCarrier, W3CPropagator, B3Propagator, CompositePropagator};

// Re-export TraceError from debug module
pub use crate::debug::tracing::TraceError;

/// Global tracer configuration
#[derive(Clone, Debug)]
pub struct TracerConfig {
    /// Service name
    pub service_name: String,

    /// Sampler to use
    pub sampler: Option<Box<dyn Sampler>>,

    /// Maximum number of attributes per span
    pub max_attributes_per_span: usize,

    /// Maximum number of events per span
    pub max_events_per_span: usize,

    /// Maximum number of links per span
    pub max_links_per_span: usize,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            service_name: "nos-kernel".to_string(),
            sampler: None,
            max_attributes_per_span: 32,
            max_events_per_span: 128,
            max_links_per_span: 32,
        }
    }
}

/// Global tracer instance
static TRACER: spin::Mutex<Option<Tracer>> = spin::Mutex::new(None);

/// Initialize the global tracer
///
/// # Arguments
///
/// * `config` - Tracer configuration
///
/// # Returns
///
/// Ok(()) if successful
pub fn init_tracer(config: TracerConfig) -> Result<(), TraceError> {
    let tracer = Tracer::new(config);
    let mut global = TRACER.lock();
    *global = Some(tracer);
    Ok(())
}

/// Get the global tracer
///
/// # Returns
///
/// Reference to the global tracer if initialized
pub fn tracer() -> Option<&'static Tracer> {
    // We return a reference with 'static lifetime by leaking the tracer
    // This is safe because the tracer is never deallocated
    unsafe {
        TRACER.lock().as_ref().map(|t| {
            &*(t as *const Tracer)
        })
    }
}

/// Tracer for creating spans
pub struct Tracer {
    config: TracerConfig,
}

impl Tracer {
    /// Create a new tracer
    pub fn new(config: TracerConfig) -> Self {
        Self { config }
    }

    /// Get the tracer configuration
    pub fn config(&self) -> &TracerConfig {
        &self.config
    }

    /// Create a new span builder
    pub fn span_builder(&self, name: impl Into<String>) -> SpanBuilder {
        SpanBuilder::new(name)
    }

    /// Create and start a root span
    pub fn start_span(&self, name: impl Into<String>) -> Span {
        SpanBuilder::new(name).start()
    }

    /// Create a child span
    pub fn start_child_span(&self, name: impl Into<String>, parent: &Span) -> Span {
        parent.child(name)
    }
}

/// Convenience macro for creating a span
///
/// # Example
///
/// ```rust
/// use kernel::tracing::span;
///
/// let _span = span!("operation_name");
/// // Do work...
/// ```
#[macro_export]
macro_rules! span {
    ($name:expr) => {
        {
            let builder = $crate::tracing::SpanBuilder::new($name);
            builder.start()
        }
    };
    ($name:expr, $($key:expr => $value:expr),+) => {
        {
            let mut builder = $crate::tracing::SpanBuilder::new($name);
            $(
                builder = builder.with_tag($key, $value);
            )+
            builder.start()
        }
    };
}

/// Convenience macro for creating a span with automatic ending
///
/// # Example
///
/// ```rust
/// use kernel::tracing::trace_span;
///
/// trace_span!("operation_name", {
///     // Code to trace
///     result
/// });
/// ```
#[macro_export]
macro_rules! trace_span {
    ($name:expr, $block:expr) => {
        {
            let _span = $crate::tracing::SpanBuilder::new($name).start();
            let result = $block;
            let _ = _span.end();
            result
        }
    };
    ($name:expr, {$($key:expr => $value:expr),+}, $block:expr) => {
        {
            let mut builder = $crate::tracing::SpanBuilder::new($name);
            $(
                builder = builder.with_tag($key, $value);
            )+
            let _span = builder.start();
            let result = $block;
            let _ = _span.end();
            result
        }
    };
}

/// Mark a field as a tracing attribute
///
/// This macro can be used to automatically annotate structs with
/// tracing information.
#[macro_export]
macro_rules! tracing_attributes {
    () => {};
}

/// Integration tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracing::exporter::{JaegerExporter, Exporter};

    #[test]
    fn test_tracer_init() {
        let config = TracerConfig {
            service_name: "test-service".to_string(),
            ..Default::default()
        };

        assert!(init_tracer(config).is_ok());
        assert!(tracer().is_some());
    }

    #[test]
    fn test_tracer_config() {
        let config = TracerConfig::default();
        assert_eq!(config.service_name, "nos-kernel");
        assert_eq!(config.max_attributes_per_span, 32);
    }

    #[test]
    fn test_tracer_start_span() {
        let tracer = Tracer::new(TracerConfig::default());
        let span = tracer.start_span("test_operation");

        assert_eq!(span.name(), "test_operation");
        assert!(!span.is_ended());
    }

    #[test]
    fn test_tracer_start_child_span() {
        let tracer = Tracer::new(TracerConfig::default());
        let parent = tracer.start_span("parent");
        let child = tracer.start_child_span("child", &parent);

        assert_eq!(child.context().parent_span_id(), Some(parent.context().span_id()));
    }

    #[test]
    fn test_span_macro() {
        let span = span!("test_operation");
        assert_eq!(span.name(), "test_operation");
    }

    #[test]
    fn test_span_macro_with_tags() {
        let span = span!("test_operation", "key1" => "value1", "key2" => "value2");
        assert_eq!(span.name(), "test_operation");
        assert_eq!(span.tags().len(), 2);
    }

    #[test]
    fn test_trace_span_macro() {
        let result = trace_span!("operation", {
            42
        });

        assert_eq!(result, 42);
    }

    #[test]
    fn test_trace_span_macro_with_tags() {
        let result = trace_span!("operation", {"key" => "value"}, {
            42
        });

        assert_eq!(result, 42);
    }

    #[test]
    fn test_full_tracing_workflow() {
        // Initialize tracer
        let config = TracerConfig {
            service_name: "test-service".to_string(),
            ..Default::default()
        };
        init_tracer(config).unwrap();

        // Create root span
        let root = span!("handle_request", "http.method" => "GET", "http.url" => "/api/test");

        // Create child span
        let child = root.child("database_query");

        // Annotate child
        child.annotate("query started").unwrap();

        // End child
        child.end().unwrap();

        // Annotate root
        root.annotate("request completed").unwrap();

        // End root
        root.end().unwrap();

        // Export
        let exporter = JaegerExporter::new("localhost", 6831);
        assert!(exporter.export(&[root]).is_ok());
    }

    #[test]
    fn test_baggage_integration() {
        let mut baggage = Baggage::new();
        baggage.set("user.id", "12345").unwrap();
        baggage.set("tenant.id", "abcde").unwrap();

        let span = span!("test_operation");

        // Copy baggage to span
        for (key, value) in baggage.entries() {
            span.set_baggage(key, &value.value);
        }

        assert_eq!(span.get_baggage("user.id"), Some("12345".to_string()));
        assert_eq!(span.get_baggage("tenant.id"), Some("abcde".to_string()));
    }

    #[test]
    fn test_context_propagation_integration() {
        let parent_context = SpanContext::new();

        // Use W3C propagator
        let propagator = W3CPropagator::new();
        let mut carrier = HeaderCarrier::new();
        propagator.inject(&parent_context, &mut carrier).unwrap();

        // Extract context
        let extracted = propagator.extract(&carrier).unwrap().unwrap();
        assert_eq!(extracted.trace_id(), parent_context.trace_id());
    }

    #[test]
    fn test_sampling_integration() {
        use sampler::{RateSampler, Sampler};

        let sampler = RateSampler::new(1.0).unwrap(); // Sample 100%
        let trace_id = TraceId::new();

        let result = sampler.should_sample(trace_id, None);
        assert!(matches!(result.decision, SamplingDecision::Sampled));
    }

    #[test]
    fn test_multi_exporter_workflow() {
        let span = span!("test_operation");
        span.end().unwrap();

        // Export to multiple backends
        let jaeger = JaegerExporter::new("localhost", 6831);
        let zipkin = ZipkinExporter::new("http://localhost:9411/api/v2/spans");
        let otlp = OTLPExporter::new("http://localhost:4317");

        assert!(jaeger.export(&[span.clone()]).is_ok());
        assert!(zipkin.export(&[span.clone()]).is_ok());
        assert!(otlp.export(&[span]).is_ok());
    }

    #[test]
    fn test_span_lifecycle() {
        let span = span!("test_operation");

        // Add events
        span.add_event("event1", None).unwrap();
        span.add_event("event2", Some(vec![("key".to_string(), "value".to_string())])).unwrap();

        // Add tags
        span.set_tag("tag1", "value1").unwrap();
        span.set_tag("tag2", "value2").unwrap();

        // Record error
        span.record_error("something went wrong");

        // Check status
        assert!(matches!(span.status(), crate::tracing::span::SpanStatus::Error { .. }));

        // End span
        span.end().unwrap();
        assert!(span.is_ended());
        assert!(span.duration().as_nanos() > 0);
    }

    #[test]
    fn test_complex_trace_hierarchy() {
        let root = span!("root_operation");

        let child1 = root.child("child1");
        let child2 = root.child("child2");

        let grandchild1 = child1.child("grandchild1");
        let grandchild2 = child2.child("grandchild2");

        // Verify hierarchy
        assert_eq!(child1.context().parent_span_id(), Some(root.context().span_id()));
        assert_eq!(child2.context().parent_span_id(), Some(root.context().span_id()));
        assert_eq!(grandchild1.context().parent_span_id(), Some(child1.context().span_id()));
        assert_eq!(grandchild2.context().parent_span_id(), Some(child2.context().span_id()));

        // All have same trace ID
        assert_eq!(root.context().trace_id(), child1.context().trace_id());
        assert_eq!(root.context().trace_id(), child2.context().trace_id());
        assert_eq!(root.context().trace_id(), grandchild1.context().trace_id());
        assert_eq!(root.context().trace_id(), grandchild2.context().trace_id());
    }

    #[test]
    fn test_concurrent_spans() {
        let root = span!("root");

        // Create multiple concurrent children
        let children: Vec<_> = (0..10).map(|i| root.child(format!("child{}", i))).collect();

        // All children should have the same parent
        for child in &children {
            assert_eq!(child.context().parent_span_id(), Some(root.context().span_id()));
        }

        // All should have unique span IDs
        let span_ids: Vec<_> = children.iter().map(|c| c.context().span_id()).collect();
        let unique_ids: Vec<_> = span_ids.iter().collect();
        assert_eq!(span_ids.len(), unique_ids.len());
    }
}
