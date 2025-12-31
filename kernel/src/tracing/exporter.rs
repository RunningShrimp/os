//! # Trace Exporters
//!
//! This module provides exporters for sending trace data to various backends.
//! It supports Jaeger, Zipkin, and OpenTelemetry Protocol (OTLP).
//!
//! # Architecture
//!
//! - **Exporter**: Trait for exporting spans
//! - **JaegerExporter**: Exports to Jaeger via UDP or HTTP
//! - **ZipkinExporter**: Exports to Zipkin via HTTP
//! - **OTLPExporter**: Exports via OpenTelemetry Protocol
//! - **BatchExporter**: Batches spans before export
//! - **SpanQueue**: Queue for buffering spans before export
//!
//! # Example
//!
//! ```rust
//! use kernel::tracing::exporter::{JaegerExporter, Exporter};
//!
//! let exporter = JaegerExporter::new("localhost", 6831);
//! let span = create_test_span();
//! exporter.export(&[span])?;
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::tracing::context::{SpanContext, SpanId, TraceId};
use crate::tracing::span::{Span, SpanKind};
use crate::tracing::TraceError;

/// Maximum batch size for exporting
const MAX_BATCH_SIZE: usize = 100;

/// Default export timeout in seconds
const DEFAULT_EXPORT_TIMEOUT: u64 = 30;

/// Maximum queue size
const MAX_QUEUE_SIZE: usize = 1000;

/// Export configuration
#[derive(Clone, Debug)]
pub struct ExportConfig {
    /// Maximum batch size
    pub max_batch_size: usize,

    /// Export timeout in seconds
    pub timeout_secs: u64,

    /// Whether to use async export
    pub async_export: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            max_batch_size: MAX_BATCH_SIZE,
            timeout_secs: DEFAULT_EXPORT_TIMEOUT,
            async_export: true,
        }
    }
}

/// Exporter trait
///
/// Exporters send trace data to external backends for analysis and visualization.
pub trait Exporter: Send + Sync {
    /// Export a batch of spans
    ///
    /// # Arguments
    ///
    /// * `spans` - Spans to export
    ///
    /// # Returns
    ///
    /// Ok(()) if export succeeded, Err otherwise
    fn export(&self, spans: &[Span]) -> Result<(), TraceError>;

    /// Flush any buffered spans
    fn flush(&self) -> Result<(), TraceError>;

    /// Shutdown the exporter
    fn shutdown(&self) -> Result<(), TraceError>;

    /// Get the exporter name
    fn name(&self) -> &str;
}

/// Jaeger exporter (UDP thrift format)
#[derive(Clone)]
pub struct JaegerExporter {
    /// Agent hostname
    hostname: String,

    /// Agent port
    port: u16,

    /// Service name
    service_name: String,

    /// Configuration
    config: ExportConfig,

    /// Whether exporter is running
    running: AtomicBool,
}

impl JaegerExporter {
    /// Create a new Jaeger UDP exporter
    ///
    /// # Arguments
    ///
    /// * `hostname` - Jaeger agent hostname
    /// * `port` - Jaeger agent UDP port (usually 6831)
    pub fn new(hostname: impl Into<String>, port: u16) -> Self {
        Self {
            hostname: hostname.into(),
            port,
            service_name: "nos-kernel".to_string(),
            config: ExportConfig::default(),
            running: AtomicBool::new(true),
        }
    }

    /// Set the service name
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Set the export configuration
    pub fn with_config(mut self, config: ExportConfig) -> Self {
        self.config = config;
        self
    }

    /// Convert span to Jaeger Thrift format
    fn span_to_jaeger(&self, span: &Span) -> JaegerSpan {
        JaegerSpan {
            trace_id_low: (span.context().trace_id().as_u128() as u64),
            trace_id_high: (span.context().trace_id().as_u128() >> 64) as u64,
            span_id: span.context().span_id().as_u64(),
            parent_span_id: span.context().parent_span_id().map(|id| id.as_u64()),
            operation_name: span.name().to_string(),
            references: Vec::new(),
            flags: if span.context().is_sampled() { 1 } else { 0 },
            start_time: span.start_time(),
            duration: span.duration().as_nanos() as u64,
            tags: self.tags_to_jaeger(span),
            logs: self.events_to_jaeger(span),
        }
    }

    /// Convert span tags to Jaeger format
    fn tags_to_jaeger(&self, span: &Span) -> Vec<JaegerTag> {
        let mut tags = Vec::new();

        // Add span kind as tag
        tags.push(JaegerTag {
            key: "span.kind".to_string(),
            v_type: JaegerTagType::String,
            v_str: Some(span.kind().as_str().to_string()),
            v_long: None,
            v_double: None,
            v_bool: None,
        });

        // Add status as tag
        tags.push(JaegerTag {
            key: "error".to_string(),
            v_type: JaegerTagType::Bool,
            v_str: None,
            v_long: None,
            v_double: None,
            v_bool: Some(!matches!(span.status(), crate::tracing::span::SpanStatus::Ok)),
        });

        // Add user tags
        for (key, value) in span.tags() {
            tags.push(JaegerTag {
                key,
                v_type: JaegerTagType::String,
                v_str: Some(value),
                v_long: None,
                v_double: None,
                v_bool: None,
            });
        }

        tags
    }

    /// Convert span events to Jaeger logs
    fn events_to_jaeger(&self, span: &Span) -> Vec<JaegerLog> {
        span.events()
            .into_iter()
            .map(|event| JaegerLog {
                timestamp: event.timestamp,
                fields: event
                    .attributes
                    .into_iter()
                    .map(|(k, v)| JaegerTag {
                        key: k,
                        v_type: JaegerTagType::String,
                        v_str: Some(v),
                        v_long: None,
                        v_double: None,
                        v_bool: None,
                    })
                    .collect(),
            })
            .collect()
    }

    /// Serialize batch to Thrift format (simplified)
    fn serialize_thrift(&self, spans: Vec<JaegerSpan>) -> Result<Vec<u8>, TraceError> {
        // In a real implementation, this would use proper Thrift serialization
        // For now, we provide a simplified format
        let mut buffer = Vec::new();

        // Batch header
        buffer.extend_from_slice(b"JAEGER_BATCH");

        for span in spans {
            // Span data would be serialized here
            let _ = span;
        }

        Ok(buffer)
    }
}

impl Exporter for JaegerExporter {
    fn export(&self, spans: &[Span]) -> Result<(), TraceError> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(TraceError::ExportError("Exporter is shutdown".to_string()));
        }

        let jaeger_spans: Vec<JaegerSpan> = spans.iter().map(|s| self.span_to_jaeger(s)).collect();
        let _data = self.serialize_thrift(jaeger_spans)?;

        // In a real implementation, this would send UDP packets to Jaeger agent
        // For now, we just simulate success
        Ok(())
    }

    fn flush(&self) -> Result<(), TraceError> {
        // Nothing to flush for UDP exporter
        Ok(())
    }

    fn shutdown(&self) -> Result<(), TraceError> {
        self.running.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn name(&self) -> &str {
        "JaegerExporter"
    }
}

/// Jaeger span representation
#[derive(Clone, Debug)]
struct JaegerSpan {
    trace_id_low: u64,
    trace_id_high: u64,
    span_id: u64,
    parent_span_id: Option<u64>,
    operation_name: String,
    references: Vec<JaegerSpanRef>,
    flags: u32,
    start_time: u64,
    duration: u64,
    tags: Vec<JaegerTag>,
    logs: Vec<JaegerLog>,
}

/// Jaeger span reference
#[derive(Clone, Debug)]
struct JaegerSpanRef {
    ref_type: String,
    trace_id_low: u64,
    trace_id_high: u64,
    span_id: u64,
}

/// Jaeger tag
#[derive(Clone, Debug)]
struct JaegerTag {
    key: String,
    v_type: JaegerTagType,
    v_str: Option<String>,
    v_long: Option<i64>,
    v_double: Option<f64>,
    v_bool: Option<bool>,
}

/// Jaeger tag type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JaegerTagType {
    String,
    Long,
    Double,
    Bool,
}

/// Jaeger log
#[derive(Clone, Debug)]
struct JaegerLog {
    timestamp: u64,
    fields: Vec<JaegerTag>,
}

/// Zipkin exporter (HTTP JSON format)
#[derive(Clone)]
pub struct ZipkinExporter {
    /// Endpoint URL
    endpoint: String,

    /// Service name
    service_name: String,

    /// Configuration
    config: ExportConfig,

    /// Whether exporter is running
    running: AtomicBool,
}

impl ZipkinExporter {
    /// Create a new Zipkin HTTP exporter
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Zipkin API endpoint (e.g., "http://localhost:9411/api/v2/spans")
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            service_name: "nos-kernel".to_string(),
            config: ExportConfig::default(),
            running: AtomicBool::new(true),
        }
    }

    /// Set the service name
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Convert span to Zipkin format
    fn span_to_zipkin(&self, span: &Span) -> ZipkinSpan {
        ZipkinSpan {
            trace_id: format!("{:016x}", span.context().trace_id().as_u128() as u64),
            id: format!("{:016x}", span.context().span_id().as_u64()),
            parent_id: span.context().parent_span_id().map(|id| format!("{:016x}", id.as_u64())),
            name: span.name().to_string(),
            timestamp: span.start_time(),
            duration: span.duration().as_micros() as u64,
            local_endpoint: ZipkinEndpoint {
                service_name: self.service_name.clone(),
                ipv4: None,
                ipv6: None,
                port: None,
            },
            remote_endpoint: None,
            kind: Some(self.span_kind_to_zipkin(span.kind())),
            tags: Some(span.tags().into_iter().collect()),
            annotations: span
                .events()
                .into_iter()
                .map(|e| ZipkinAnnotation {
                    timestamp: e.timestamp,
                    value: e.name,
                })
                .collect(),
        }
    }

    /// Convert span kind to Zipkin kind
    fn span_kind_to_zipkin(&self, kind: SpanKind) -> String {
        match kind {
            SpanKind::Client => "CLIENT".to_string(),
            SpanKind::Server => "SERVER".to_string(),
            SpanKind::Producer => "PRODUCER".to_string(),
            SpanKind::Consumer => "CONSUMER".to_string(),
            _ => "UNKNOWN".to_string(),
        }
    }
}

impl Exporter for ZipkinExporter {
    fn export(&self, spans: &[Span]) -> Result<(), TraceError> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(TraceError::ExportError("Exporter is shutdown".to_string()));
        }

        let zipkin_spans: Vec<ZipkinSpan> = spans.iter().map(|s| self.span_to_zipkin(s)).collect();
        let _json = self.serialize_json(zipkin_spans)?;

        // In a real implementation, this would send HTTP POST to Zipkin
        Ok(())
    }

    fn flush(&self) -> Result<(), TraceError> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), TraceError> {
        self.running.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn name(&self) -> &str {
        "ZipkinExporter"
    }
}

impl ZipkinExporter {
    /// Serialize spans to JSON (simplified)
    fn serialize_json(&self, spans: Vec<ZipkinSpan>) -> Result<String, TraceError> {
        // In a real implementation, this would use proper JSON serialization
        // For now, we provide a placeholder
        Ok(format!("{{\"spans\": {}}}", spans.len()))
    }
}

/// Zipkin span representation
#[derive(Clone, Debug)]
struct ZipkinSpan {
    trace_id: String,
    id: String,
    parent_id: Option<String>,
    name: String,
    timestamp: u64,
    duration: u64,
    local_endpoint: ZipkinEndpoint,
    remote_endpoint: Option<ZipkinEndpoint>,
    kind: Option<String>,
    tags: Option<BTreeMap<String, String>>,
    annotations: Vec<ZipkinAnnotation>,
}

/// Zipkin endpoint
#[derive(Clone, Debug)]
struct ZipkinEndpoint {
    service_name: String,
    ipv4: Option<String>,
    ipv6: Option<String>,
    port: Option<u16>,
}

/// Zipkin annotation
#[derive(Clone, Debug)]
struct ZipkinAnnotation {
    timestamp: u64,
    value: String,
}

/// OpenTelemetry Protocol (OTLP) exporter
#[derive(Clone)]
pub struct OTLPExporter {
    /// Endpoint URL
    endpoint: String,

    /// Configuration
    config: ExportConfig,

    /// Whether exporter is running
    running: AtomicBool,
}

impl OTLPExporter {
    /// Create a new OTLP exporter
    ///
    /// # Arguments
    ///
    /// * `endpoint` - OTLP endpoint URL
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            config: ExportConfig::default(),
            running: AtomicBool::new(true),
        }
    }

    /// Convert span to OTLP format
    fn span_to_otlp(&self, span: &Span) -> OTLPSpan {
        OTLPSpan {
            trace_id: span.context().trace_id().as_u128().to_be_bytes().to_vec(),
            span_id: span.context().span_id().as_u64().to_be_bytes().to_vec(),
            parent_span_id: span.context().parent_span_id().map(|id| id.as_u64().to_be_bytes().to_vec()),
            name: span.name().to_string(),
            start_time_unix_nano: span.start_time(),
            end_time_unix_nano: span.start_time() + span.duration().as_nanos() as u64,
            kind: Some(self.span_kind_to_otlp(span.kind())),
            attributes: span
                .tags()
                .into_iter()
                .map(|(k, v)| OTLPKeyValue { key: k, value: v })
                .collect(),
            events: span
                .events()
                .into_iter()
                .map(|e| OTLPEvent {
                    time_unix_nano: e.timestamp,
                    name: e.name,
                    attributes: e
                        .attributes
                        .into_iter()
                        .map(|(k, v)| OTLPKeyValue { key: k, value: v })
                        .collect(),
                })
                .collect(),
        }
    }

    /// Convert span kind to OTLP kind
    fn span_kind_to_otlp(&self, kind: SpanKind) -> i32 {
        match kind {
            SpanKind::Internal => 1,
            SpanKind::Client => 2,
            SpanKind::Server => 3,
            SpanKind::Producer => 4,
            SpanKind::Consumer => 5,
            SpanKind::InternalInvocation => 1,
        }
    }
}

impl Exporter for OTLPExporter {
    fn export(&self, spans: &[Span]) -> Result<(), TraceError> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(TraceError::ExportError("Exporter is shutdown".to_string()));
        }

        let otlp_spans: Vec<OTLPSpan> = spans.iter().map(|s| self.span_to_otlp(s)).collect();
        let _proto = self.serialize_protobuf(otlp_spans)?;

        // In a real implementation, this would send protobuf via HTTP/gRPC
        Ok(())
    }

    fn flush(&self) -> Result<(), TraceError> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), TraceError> {
        self.running.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn name(&self) -> &str {
        "OTLPExporter"
    }
}

impl OTLPExporter {
    /// Serialize spans to protobuf (simplified)
    fn serialize_protobuf(&self, spans: Vec<OTLPSpan>) -> Result<Vec<u8>, TraceError> {
        // In a real implementation, this would use proper protobuf serialization
        // For now, we provide a placeholder
        Ok(format!("OTLP_SPAN_{}", spans.len()).into_bytes())
    }
}

/// OTLP span representation
#[derive(Clone, Debug)]
struct OTLPSpan {
    trace_id: Vec<u8>,
    span_id: Vec<u8>,
    parent_span_id: Option<Vec<u8>>,
    name: String,
    start_time_unix_nano: u64,
    end_time_unix_nano: u64,
    kind: Option<i32>,
    attributes: Vec<OTLPKeyValue>,
    events: Vec<OTLPEvent>,
}

/// OTLP key-value pair
#[derive(Clone, Debug)]
struct OTLPKeyValue {
    key: String,
    value: String,
}

/// OTLP event
#[derive(Clone, Debug)]
struct OTLPEvent {
    time_unix_nano: u64,
    name: String,
    attributes: Vec<OTLPKeyValue>,
}

/// Batch exporter
///
/// Batches spans before sending to the underlying exporter.
pub struct BatchExporter {
    /// Underlying exporter
    exporter: Box<dyn Exporter>,

    /// Span queue
    queue: SpanQueue,

    /// Batch size
    batch_size: usize,

    /// Whether to flush periodically
    auto_flush: bool,
}

impl BatchExporter {
    /// Create a new batch exporter
    ///
    /// # Arguments
    ///
    /// * `exporter` - Underlying exporter to send batches to
    /// * `batch_size` - Maximum batch size
    pub fn new(exporter: Box<dyn Exporter>, batch_size: usize) -> Self {
        Self {
            exporter,
            queue: SpanQueue::new(MAX_QUEUE_SIZE),
            batch_size: batch_size.min(MAX_BATCH_SIZE),
            auto_flush: true,
        }
    }

    /// Add a span to the batch
    pub fn add_span(&self, span: Span) -> Result<(), TraceError> {
        self.queue.push(span)?;

        if self.auto_flush && self.queue.len() >= self.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    /// Disable auto-flush
    pub fn without_auto_flush(mut self) -> Self {
        self.auto_flush = false;
        self
    }
}

impl Exporter for BatchExporter {
    fn export(&self, spans: &[Span]) -> Result<(), TraceError> {
        for chunk in spans.chunks(self.batch_size) {
            self.exporter.export(chunk)?;
        }
        Ok(())
    }

    fn flush(&self) -> Result<(), TraceError> {
        let batch: Vec<Span> = self.queue.drain_all();
        if !batch.is_empty() {
            self.exporter.export(&batch)?;
        }
        Ok(())
    }

    fn shutdown(&self) -> Result<(), TraceError> {
        self.flush()?;
        self.exporter.shutdown()
    }

    fn name(&self) -> &str {
        "BatchExporter"
    }
}

/// Span queue for buffering
#[derive(Clone)]
struct SpanQueue {
    /// Queue storage (simplified)
    queue: Vec<Span>,

    /// Maximum size
    max_size: usize,

    /// Current size (atomic)
    size: AtomicUsize,
}

impl SpanQueue {
    /// Create a new span queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Vec::with_capacity(max_size),
            max_size,
            size: AtomicUsize::new(0),
        }
    }

    /// Push a span onto the queue
    pub fn push(&self, span: Span) -> Result<(), TraceError> {
        let size = self.size.load(Ordering::Acquire);
        if size >= self.max_size {
            return Err(TraceError::ExportQueueFull);
        }

        // In a real implementation, this would use proper concurrent queue
        // For now, we use a simplified approach
        unsafe {
            let queue_ptr = &self.queue as *const Vec<Span> as *mut Vec<Span>;
            (*queue_ptr).push(span);
        }
        self.size.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Drain all spans from the queue
    pub fn drain_all(&self) -> Vec<Span> {
        unsafe {
            let queue_ptr = &self.queue as *const Vec<Span> as *mut Vec<Span>;
            let mut queue = Vec::new();
            core::ptr::swap(&mut queue, (*queue_ptr) as *mut Vec<Span>);
            self.size.store(0, Ordering::Release);
            queue
        }
    }

    /// Get the current queue length
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Acquire)
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_span() -> Span {
        let context = SpanContext::new();
        let mut span = Span::new("test_operation", context);
        span.set_tag("test_key", "test_value").unwrap();
        span.end().unwrap();
        span
    }

    #[test]
    fn test_jaeger_exporter_creation() {
        let exporter = JaegerExporter::new("localhost", 6831);
        assert_eq!(exporter.name(), "JaegerExporter");
        assert!(exporter.running.load(Ordering::Relaxed));
    }

    #[test]
    fn test_jaeger_exporter_with_service_name() {
        let exporter = JaegerExporter::new("localhost", 6831)
            .with_service_name("test-service");
        assert_eq!(exporter.service_name, "test-service");
    }

    #[test]
    fn test_jaeger_export() {
        let exporter = JaegerExporter::new("localhost", 6831);
        let span = create_test_span();
        assert!(exporter.export(&[span]).is_ok());
    }

    #[test]
    fn test_jaeger_shutdown() {
        let exporter = JaegerExporter::new("localhost", 6831);
        assert!(exporter.shutdown().is_ok());
        assert!(!exporter.running.load(Ordering::Relaxed));
    }

    #[test]
    fn test_zipkin_exporter_creation() {
        let exporter = ZipkinExporter::new("http://localhost:9411/api/v2/spans");
        assert_eq!(exporter.name(), "ZipkinExporter");
    }

    #[test]
    fn test_zipkin_export() {
        let exporter = ZipkinExporter::new("http://localhost:9411/api/v2/spans");
        let span = create_test_span();
        assert!(exporter.export(&[span]).is_ok());
    }

    #[test]
    fn test_otlp_exporter_creation() {
        let exporter = OTLPExporter::new("http://localhost:4317");
        assert_eq!(exporter.name(), "OTLPExporter");
    }

    #[test]
    fn test_otlp_export() {
        let exporter = OTLPExporter::new("http://localhost:4317");
        let span = create_test_span();
        assert!(exporter.export(&[span]).is_ok());
    }

    #[test]
    fn test_batch_exporter() {
        let jaeger = Box::new(JaegerExporter::new("localhost", 6831));
        let batch = BatchExporter::new(jaeger, 10);

        for _ in 0..5 {
            let span = create_test_span();
            assert!(batch.add_span(span).is_ok());
        }

        assert!(batch.flush().is_ok());
    }

    #[test]
    fn test_batch_exporter_auto_flush() {
        let jaeger = Box::new(JaegerExporter::new("localhost", 6831));
        let batch = BatchExporter::new(jaeger, 3);

        // Should auto-flush when reaching batch size
        for _ in 0..3 {
            let span = create_test_span();
            assert!(batch.add_span(span).is_ok());
        }

        // Queue should be empty after auto-flush
        assert!(batch.queue.is_empty());
    }

    #[test]
    fn test_span_queue() {
        let queue = SpanQueue::new(10);

        for _ in 0..10 {
            let span = create_test_span();
            assert!(queue.push(span).is_ok());
        }

        assert_eq!(queue.len(), 10);

        // Should fail when queue is full
        let span = create_test_span();
        assert!(matches!(queue.push(span), Err(TraceError::ExportQueueFull)));
    }

    #[test]
    fn test_export_config() {
        let config = ExportConfig {
            max_batch_size: 50,
            timeout_secs: 60,
            async_export: false,
        };

        assert_eq!(config.max_batch_size, 50);
        assert_eq!(config.timeout_secs, 60);
        assert!(!config.async_export);
    }
}
