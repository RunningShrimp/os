//! Metrics Collection (Prometheus compatible)
//!
//! This module provides a comprehensive metrics collection system compatible with
//! Prometheus exposition format. It supports multiple metric types including counters,
//! gauges, histograms, and summaries.
//!
//! ## Key Features
//!
//! - **Prometheus Compatibility**: Export metrics in Prometheus text format
//! - **Multiple Metric Types**: Counters, gauges, histograms, and summaries
//! - **Thread-Safe**: All operations are atomic and thread-safe
//! - **Dynamic Registration**: Register and manage metrics at runtime
//! - **Labels Support**: Attach labels to metrics for dimensional data
//!
//! ## Metric Types
//!
//! - **Counter**: Monotonically increasing counter
//! - **Gauge**: Value that can go up or down
//! - **Histogram**: Distribution of values with configurable buckets
//! - **Summary**: Distribution with configurable quantiles
//!
//! ## Example
//!
//! ```rust
//! use kernel::monitoring::enhanced_metrics::{MetricRegistry, MetricType};
//!
//! let mut registry = MetricRegistry::new();
//! registry.register_counter("requests_total", "Total requests").unwrap();
//! registry.counter_inc("requests_total");
//! let prometheus = registry.export_prometheus();
//! ```

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic {AtomicU64,, Ordering};

use crate::error::UnifiedError;
use crate::subsystems::sync::Mutex;

/// Default histogram buckets (in milliseconds)
const DEFAULT_HISTOGRAM_BUCKETS: &[u64; 11] = &[
    1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000,
];

/// Maximum number of metrics
const MAX_METRICS: usize = 1000;

/// Maximum labels per metric
const MAX_LABELS: usize = 16;

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

impl MetricType {
    /// Get the Prometheus type name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
            Self::Summary => "summary",
        }
    }
}

/// Counter metric - monotonically increasing value
#[derive(Debug)]
pub struct Counter {
    /// Current value
    pub value: AtomicU64,
    /// Help text
    pub help: String,
}

impl Counter {
    /// Create a new counter
    pub fn new(help: String) -> Self {
        Self {
            value: AtomicU64::new(0),
            help,
        }
    }

    /// Increment the counter
    #[inline]
    pub fn inc(&self) {
        self.inc_by(1);
    }

    /// Increment by a specific amount
    #[inline]
    pub fn inc_by(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Get the current value
    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset to zero
    #[inline]
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// Gauge metric - value that can go up or down
#[derive(Debug)]
pub struct Gauge {
    /// Current value
    pub value: AtomicU64,
    /// Help text
    pub help: String,
}

impl Gauge {
    /// Create a new gauge
    pub fn new(help: String) -> Self {
        Self {
            value: AtomicU64::new(0),
            help,
        }
    }

    /// Set the gauge value
    #[inline]
    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    /// Increment the gauge
    #[inline]
    pub fn inc(&self) {
        self.inc_by(1);
    }

    /// Increment by a specific amount
    #[inline]
    pub fn inc_by(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Decrement the gauge
    #[inline]
    pub fn dec(&self) {
        self.dec_by(1);
    }

    /// Decrement by a specific amount
    #[inline]
    pub fn dec_by(&self, delta: u64) {
        self.value.fetch_sub(delta, Ordering::Relaxed);
    }

    /// Get the current value
    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Histogram bucket
#[derive(Debug)]
pub struct HistogramBucket {
    /// Upper bound
    pub upper_bound: u64,
    /// Count in this bucket
    pub count: AtomicU64,
}

impl Clone for HistogramBucket {
    fn clone(&self) -> Self {
        Self {
            upper_bound: self.upper_bound,
            count: AtomicU64::new(self.count.load(Ordering::Relaxed)),
        }
    }
}

impl HistogramBucket {
    /// Create a new bucket
    #[inline]
    pub fn new(upper_bound: u64) -> Self {
        Self {
            upper_bound,
            count: AtomicU64::new(0),
        }
    }

    /// Increment the bucket count
    #[inline]
    pub fn inc(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the count
    #[inline]
    pub fn get(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

/// Histogram metric - distribution of values
#[derive(Debug)]
pub struct Histogram {
    /// Buckets
    pub buckets: Vec<HistogramBucket>,
    /// Sum of all observed values
    pub sum: AtomicU64,
    /// Total count of observations
    pub count: AtomicU64,
    /// Help text
    pub help: String,
}

impl Histogram {
    /// Create a new histogram with default buckets
    pub fn new(help: String) -> Self {
        let buckets = DEFAULT_HISTOGRAM_BUCKETS
            .iter()
            .map(|&b| HistogramBucket::new(b))
            .collect();

        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            help,
        }
    }

    /// Create a new histogram with custom buckets
    pub fn with_buckets(help: String, bucket_bounds: &[u64]) -> Self {
        let buckets = bucket_bounds
            .iter()
            .map(|&b| HistogramBucket::new(b))
            .collect();

        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            help,
        }
    }

    /// Observe a value
    pub fn observe(&self, value: u64) {
        self.sum.fetch_add(value, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Find the appropriate bucket
        for bucket in &self.buckets {
            if value <= bucket.upper_bound {
                bucket.inc();
            }
        }
    }

    /// Get the sum of observed values
    #[inline]
    pub fn get_sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }

    /// Get the count of observations
    #[inline]
    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get bucket counts
    pub fn get_buckets(&self) -> Vec<(u64, u64)> {
        self.buckets
            .iter()
            .map(|b| (b.upper_bound, b.get()))
            .collect()
    }
}

/// Quantile definition for summary
#[derive(Debug)]
pub struct Quantile {
    /// Quantile value (e.g., 0.5 for median, 0.95 for p95)
    pub value: f64,
    /// Estimated value at this quantile
    pub estimate: AtomicU64,
}

impl Clone for Quantile {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            estimate: AtomicU64::new(self.estimate.load(Ordering::Relaxed)),
        }
    }
}

impl Quantile {
    /// Create a new quantile
    #[inline]
    pub fn new(value: f64) -> Self {
        Self {
            value,
            estimate: AtomicU64::new(0),
        }
    }

    /// Set the estimated value
    #[inline]
    pub fn set(&self, value: u64) {
        self.estimate.store(value, Ordering::Relaxed);
    }

    /// Get the estimated value
    #[inline]
    pub fn get(&self) -> u64 {
        self.estimate.load(Ordering::Relaxed)
    }
}

/// Summary metric - distribution with configurable quantiles
#[derive(Debug)]
pub struct Summary {
    /// Quantiles to track
    pub quantiles: Vec<Quantile>,
    /// Sum of all observed values
    pub sum: AtomicU64,
    /// Total count of observations
    pub count: AtomicU64,
    /// Help text
    pub help: String,
}

impl Summary {
    /// Create a new summary with default quantiles
    pub fn new(help: String) -> Self {
        let quantiles = vec![0.5, 0.9, 0.95, 0.99]
            .into_iter()
            .map(Quantile::new)
            .collect();

        Self {
            quantiles,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            help,
        }
    }

    /// Create a new summary with custom quantiles
    pub fn with_quantiles(help: String, quantile_values: &[f64]) -> Self {
        let quantiles = quantile_values
            .iter()
            .map(|&q| Quantile::new(q))
            .collect();

        Self {
            quantiles,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            help,
        }
    }

    /// Observe a value
    pub fn observe(&self, value: u64) {
        self.sum.fetch_add(value, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // For simplicity, we don't implement accurate quantile estimation here.
        // A real implementation would use a t-digest or similar algorithm.
        // For now, we just update the max value for each quantile.
        for quantile in &self.quantiles {
            let current = quantile.get();
            if value > current {
                quantile.set(value);
            }
        }
    }

    /// Get the sum of observed values
    #[inline]
    pub fn get_sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }

    /// Get the count of observations
    #[inline]
    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get quantile values
    pub fn get_quantiles(&self) -> Vec<(f64, u64)> {
        self.quantiles
            .iter()
            .map(|q| (q.value, q.get()))
            .collect()
    }
}

/// Labeled metric instance
pub enum LabeledMetric {
    Counter(Counter),
    Gauge(Gauge),
    Histogram(Histogram),
    Summary(Summary),
}

impl Clone for LabeledMetric {
    fn clone(&self) -> Self {
        match self {
            Self::Counter(c) => Self::Counter(Counter {
                value: core::sync::atomic::AtomicU64::new(c.value.load(core::sync::atomic::Ordering::Relaxed)),
                help: c.help.clone(),
            }),
            Self::Gauge(g) => Self::Gauge(Gauge {
                value: core::sync::atomic::AtomicU64::new(g.value.load(core::sync::atomic::Ordering::Relaxed)),
                help: g.help.clone(),
            }),
            Self::Histogram(h) => Self::Histogram(Histogram {
                buckets: h.buckets.clone(),
                sum: core::sync::atomic::AtomicU64::new(h.sum.load(core::sync::atomic::Ordering::Relaxed)),
                count: core::sync::atomic::AtomicU64::new(h.count.load(core::sync::atomic::Ordering::Relaxed)),
                help: h.help.clone(),
            }),
            Self::Summary(s) => Self::Summary(Summary {
                quantiles: s.quantiles.clone(),
                sum: core::sync::atomic::AtomicU64::new(s.sum.load(core::sync::atomic::Ordering::Relaxed)),
                count: core::sync::atomic::AtomicU64::new(s.count.load(core::sync::atomic::Ordering::Relaxed)),
                help: s.help.clone(),
            }),
        }
    }
}

impl LabeledMetric {
    /// Get the metric type
    pub fn metric_type(&self) -> MetricType {
        match self {
            Self::Counter(_) => MetricType::Counter,
            Self::Gauge(_) => MetricType::Gauge,
            Self::Histogram(_) => MetricType::Histogram,
            Self::Summary(_) => MetricType::Summary,
        }
    }
}

/// Metric definition
pub struct Metric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Help text
    pub help: String,
    /// Labeled instances
    pub labeled: Mutex<BTreeMap<String, LabeledMetric>>,
}

impl Metric {
    /// Create a new metric
    pub fn new(name: String, metric_type: MetricType, help: String) -> Self {
        Self {
            name,
            metric_type,
            help,
            labeled: Mutex::new(BTreeMap::new()),
        }
    }

    /// Get or create a labeled instance
    pub fn get_or_create_label(&self, labels: &str) -> LabeledMetric {
        let mut labeled = self.labeled.lock();

        if let Some(metric) = labeled.get(labels) {
            // Return cloned reference (simplified - in real implementation would use Arc)
            return match metric {
                LabeledMetric::Counter(c) => LabeledMetric::Counter(Counter::new(c.help.clone())),
                LabeledMetric::Gauge(g) => LabeledMetric::Gauge(Gauge::new(g.help.clone())),
                LabeledMetric::Histogram(h) => LabeledMetric::Histogram(Histogram::new(h.help.clone())),
                LabeledMetric::Summary(s) => LabeledMetric::Summary(Summary::new(s.help.clone())),
            };
        }

        // Create new instance
        let metric = match self.metric_type {
            MetricType::Counter => LabeledMetric::Counter(Counter::new(self.help.clone())),
            MetricType::Gauge => LabeledMetric::Gauge(Gauge::new(self.help.clone())),
            MetricType::Histogram => LabeledMetric::Histogram(Histogram::new(self.help.clone())),
            MetricType::Summary => LabeledMetric::Summary(Summary::new(self.help.clone())),
        };

        labeled.insert(labels.to_string(), metric.clone());
        metric
    }
}

/// Metric registry - manages all metrics
pub struct MetricRegistry {
    /// Registered metrics by name
    metrics: Mutex<BTreeMap<String, Metric>>,
    /// Total metrics registered
    total_metrics: AtomicU64,
}

impl MetricRegistry {
    /// Create a new metric registry
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            total_metrics: AtomicU64::new(0),
        }
    }

    /// Register a counter metric
    pub fn register_counter(&mut self, name: &str, help: &str) -> Result<(), UnifiedError> {
        self.register_metric(name, MetricType::Counter, help)
    }

    /// Register a gauge metric
    pub fn register_gauge(&mut self, name: &str, help: &str) -> Result<(), UnifiedError> {
        self.register_metric(name, MetricType::Gauge, help)
    }

    /// Register a histogram metric
    pub fn register_histogram(&mut self, name: &str, help: &str) -> Result<(), UnifiedError> {
        self.register_metric(name, MetricType::Histogram, help)
    }

    /// Register a summary metric
    pub fn register_summary(&mut self, name: &str, help: &str) -> Result<(), UnifiedError> {
        self.register_metric(name, MetricType::Summary, help)
    }

    /// Register a metric
    fn register_metric(&mut self, name: &str, metric_type: MetricType, help: &str) -> Result<(), UnifiedError> {
        let mut metrics = self.metrics.lock();

        if metrics.contains_key(name) {
            return Err(UnifiedError::Other(format!("Metric '{}' already registered", name)));
        }

        if metrics.len() >= MAX_METRICS {
            return Err(UnifiedError::Other("Maximum metrics limit reached".to_string()));
        }

        metrics.insert(
            name.to_string(),
            Metric::new(name.to_string(), metric_type, help.to_string()),
        );

        self.total_metrics.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Increment a counter
    pub fn counter_inc(&self, name: &str) {
        self.counter_inc_by(name, 1);
    }

    /// Increment a counter by a specific amount
    pub fn counter_inc_by(&self, name: &str, delta: u64) {
        let metrics = self.metrics.lock();
        if let Some(metric) = metrics.get(name) {
            let instance = metric.get_or_create_label("");
            if let LabeledMetric::Counter(counter) = instance {
                counter.inc_by(delta);
            }
        }
    }

    /// Set a gauge value
    pub fn gauge_set(&self, name: &str, value: u64) {
        let metrics = self.metrics.lock();
        if let Some(metric) = metrics.get(name) {
            let instance = metric.get_or_create_label("");
            if let LabeledMetric::Gauge(gauge) = instance {
                gauge.set(value);
            }
        }
    }

    /// Observe a histogram value
    pub fn histogram_observe(&self, name: &str, value: u64) {
        let metrics = self.metrics.lock();
        if let Some(metric) = metrics.get(name) {
            let instance = metric.get_or_create_label("");
            if let LabeledMetric::Histogram(histogram) = instance {
                histogram.observe(value);
            }
        }
    }

    /// Observe a summary value
    pub fn summary_observe(&self, name: &str, value: u64) {
        let metrics = self.metrics.lock();
        if let Some(metric) = metrics.get(name) {
            let instance = metric.get_or_create_label("");
            if let LabeledMetric::Summary(summary) = instance {
                summary.observe(value);
            }
        }
    }

    /// Export metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let metrics = self.metrics.lock();
        let mut output = String::new();

        for (name, metric) in metrics.iter() {
            // Add HELP line
            output.push_str(&format!("# HELP {} {}\n", name, metric.help));
            // Add TYPE line
            output.push_str(&format!("# TYPE {} {}\n", name, metric.metric_type.as_str()));

            // Export labeled instances
            let labeled = metric.labeled.lock();
            for (labels, instance) in labeled.iter() {
                match instance {
                    LabeledMetric::Counter(counter) => {
                        let label_str = if labels.is_empty() { String::new() } else { format!("{{{}}}", labels) };
                        output.push_str(&format!("{}{} {}\n", name, label_str, counter.get()));
                    },
                    LabeledMetric::Gauge(gauge) => {
                        let label_str = if labels.is_empty() { String::new() } else { format!("{{{}}}", labels) };
                        output.push_str(&format!("{}{} {}\n", name, label_str, gauge.get()));
                    },
                    LabeledMetric::Histogram(histogram) => {
                        let label_prefix = if labels.is_empty() { String::new() } else { format!("{{{}}}", labels) };

                        // _sum and _count
                        output.push_str(&format!("{}_bucket{} {{}} {}\n", name, label_prefix, histogram.get_count()));
                        output.push_str(&format!("{}_sum{} {}\n", name, label_prefix, histogram.get_sum()));
                        output.push_str(&format!("{}_count{} {}\n", name, label_prefix, histogram.get_count()));

                        // Buckets
                        let mut cumulative_count = 0;
                        for bucket in &histogram.buckets {
                            cumulative_count += bucket.get();
                            output.push_str(&format!(
                                "{}_bucket{} {{le=\"{}\"}} {}\n",
                                name, label_prefix, bucket.upper_bound, cumulative_count
                            ));
                        }
                        // +Inf bucket
                        output.push_str(&format!(
                            "{}_bucket{} {{le=\"+Inf\"}} {}\n",
                            name, label_prefix, histogram.get_count()
                        ));
                    },
                    LabeledMetric::Summary(summary) => {
                        let label_str = if labels.is_empty() { String::new() } else { format!("{{{}}}", labels) };

                        // Quantiles
                        for (quantile, value) in summary.get_quantiles() {
                            output.push_str(&format!(
                                "{}{} {{quantile=\"{}\"}} {}\n",
                                name, label_str, quantile, value
                            ));
                        }

                        // _sum and _count
                        output.push_str(&format!("{}_sum{} {}\n", name, label_str, summary.get_sum()));
                        output.push_str(&format!("{}_count{} {}\n", name, label_str, summary.get_count()));
                    },
                }
            }

            output.push('\n');
        }

        output
    }

    /// Get metric count
    pub fn metric_count(&self) -> usize {
        self.metrics.lock().len()
    }

    /// Clear all metrics
    pub fn clear(&self) {
        self.metrics.lock().clear();
    }
}

/// Global metric registry instance
static GLOBAL_REGISTRY: Mutex<Option<MetricRegistry>> = Mutex::new(None);

/// Initialize the global metric registry
pub fn init_metric_registry() -> Result<(), UnifiedError> {
    let mut registry = GLOBAL_REGISTRY.lock();
    if registry.is_some() {
        return Err(UnifiedError::Other("Metric registry already initialized".to_string()));
    }

    let mut reg = MetricRegistry::new();

    // Register default kernel metrics
    reg.register_counter("syscalls_total", "Total system calls")?;
    reg.register_counter("syscalls_success_total", "Successful system calls")?;
    reg.register_counter("syscalls_failed_total", "Failed system calls")?;
    reg.register_counter("context_switches_total", "Total context switches")?;
    reg.register_counter("interrupts_total", "Total interrupts")?;
    reg.register_gauge("processes_running", "Running processes")?;
    reg.register_gauge("processes_total", "Total processes")?;
    reg.register_gauge("memory_used_bytes", "Used memory in bytes")?;
    reg.register_gauge("memory_free_bytes", "Free memory in bytes")?;
    reg.register_gauge("memory_total_bytes", "Total memory in bytes")?;
    reg.register_histogram("syscall_duration_ms", "System call duration in milliseconds")?;
    reg.register_histogram("schedule_latency_ms", "Scheduler latency in milliseconds")?;

    *registry = Some(reg);
    crate::log_info!("[metrics] Metric registry initialized with default metrics");
    Ok(())
}

/// Get the global metric registry
pub fn get_registry() -> Option<&'static MetricRegistry> {
    unsafe {
        GLOBAL_REGISTRY.lock().as_ref().map(|r| {
            &*(r as *const MetricRegistry)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new("test counter".to_string());
        assert_eq!(counter.get(), 0);
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new("test gauge".to_string());
        assert_eq!(gauge.get(), 0);
        gauge.set(10);
        assert_eq!(gauge.get(), 10);
        gauge.inc();
        assert_eq!(gauge.get(), 11);
        gauge.dec();
        assert_eq!(gauge.get(), 10);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram::new("test histogram".to_string());
        histogram.observe(5);
        histogram.observe(15);
        histogram.observe(25);

        assert_eq!(histogram.get_count(), 3);
        assert_eq!(histogram.get_sum(), 45);
    }

    #[test]
    fn test_metric_registry() {
        let mut registry = MetricRegistry::new();
        assert!(registry.register_counter("test_counter", "A test counter").is_ok());
        registry.counter_inc("test_counter");
        registry.counter_inc_by("test_counter", 5);

        let output = registry.export_prometheus();
        assert!(output.contains("test_counter"));
        assert!(output.contains("# TYPE test_counter counter"));
    }

    #[test]
    fn test_gauge_operations() {
        let mut registry = MetricRegistry::new();
        assert!(registry.register_gauge("test_gauge", "A test gauge").is_ok());
        registry.gauge_set("test_gauge", 42);
        // Verify in output
        let output = registry.export_prometheus();
        assert!(output.contains("test_gauge 42"));
    }
}
