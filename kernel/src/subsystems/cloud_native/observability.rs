//! Observability and Monitoring
//!
//! This module implements observability for cloud-native:
//! - Metrics collection
//! - Distributed tracing
//! - Log aggregation
//!
//! Features:
//! - OpenTelemetry-compatible metrics
//! - Distributed tracing (W3C context propagation)
//! - Log aggregation and search
//! - Real-time monitoring dashboards

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// Observability Constants
// ============================================================================

/// Maximum metrics
pub const MAX_METRICS: usize = 1 << 14;

/// Maximum traces
pub const MAX_TRACES: usize = 1 << 16;

// ============================================================================
// Metric Types
// ============================================================================

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<u64>),
    Summary(MetricSummary),
}

#[derive(Debug, Clone)]
pub struct MetricSummary {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

// ============================================================================
// Trace Context
// ============================================================================

/// Trace context (W3C)
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub operation: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub status: TraceStatus,
    pub tags: Vec<String>,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceStatus {
    Started,
    InProgress,
    Finished,
    Error,
}

// ============================================================================
// Observability Manager
// ============================================================================

/// Observability manager
pub struct ObservabilityManager {
    pub metrics: Mutex<BTreeMap<String, MetricValue>>,
    pub traces: Mutex<Vec<TraceContext>>,
    pub next_trace_id: AtomicU64,
    pub stats: Mutex<ObservabilityStats>,
}

#[derive(Debug, Clone, Copy)]
pub struct ObservabilityStats {
    pub total_metrics: usize,
    pub total_traces: u64,
    pub active_traces: usize,
}

impl Default for ObservabilityStats {
    fn default() -> Self {
        Self {
            total_metrics: 0,
            total_traces: 0,
            active_traces: 0,
        }
    }
}

impl ObservabilityManager {
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            traces: Mutex::new(Vec::new()),
            next_trace_id: AtomicU64::new(1),
            stats: Mutex::new(ObservabilityStats::default()),
        }
    }

    pub fn record_metric(&self, name: String, value: MetricValue) {
        let mut metrics = self.metrics.lock();
        metrics.insert(name, value);
    }

    pub fn start_trace(&self, service: String, operation: String) -> String {
        let trace_id = self.next_trace_id.fetch_add(1, Ordering::Relaxed).to_string();
        let trace = TraceContext {
            trace_id: trace_id.clone(),
            span_id: &trace_id.to_string() + alloc::string::String::from("-1"),
            parent_span_id: None,
            service_name: service,
            operation,
            start_time: crate::subsystems::time::timestamp_nanos(),
            end_time: None,
            status: TraceStatus::Started,
            tags: Vec::new(),
            attributes: BTreeMap::new(),
        };
        self.traces.lock().push(trace);
        crate::println!("[observability] Started trace {}", trace_id);
        trace_id
    }

    pub fn end_trace(&self, trace_id: String, status: TraceStatus) {
        let mut traces = self.traces.lock();
        if let Some(trace) = traces.iter_mut().find(|t| t.trace_id == trace_id) {
            trace.end_time = Some(crate::subsystems::time::timestamp_nanos());
            trace.status = status;
            crate::println!("[observability] Ended trace {} with status {:?}", trace_id, status);
        }
    }
}
