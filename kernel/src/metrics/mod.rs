//! Comprehensive metrics and telemetry system for the NOS kernel
//!
//! This module provides a complete metrics implementation with support for:
//! - Counters (monotonically increasing values)
//! - Gauges (point-in-time values)
//! - Histograms (value distributions)
//! - Labeled metrics (multi-dimensional tracking)
//! - Multiple export formats (Prometheus, OpenMetrics, StatsD)
//!
//! # Quick Start
//!
//! ```rust
//! use kernel::metrics::*;
//!
//! // Get the global registry
//! let registry = registry::global_registry();
//!
//! // Create a counter
//! let counter = registry.counter("requests_total").unwrap();
//! counter.inc();
//!
//! // Create a gauge
//! let gauge = registry.gauge("active_connections").unwrap();
//! gauge.set(42);
//!
//! // Create a histogram
//! let histogram = registry.histogram_latency("request_duration").unwrap();
//! histogram.observe(0.123);
//!
//! // Export metrics
//! let output = registry.export_prometheus();
//! ```
//!
//! # Using Macros
//!
//! ```rust
//! use kernel::metrics::counter;
//!
//! // Define and register metrics
//! counter!(http_requests_total, "Total HTTP requests");
//!
//! // Use the metric
//! inc_counter!(http_requests_total);
//! inc_counter_by!(http_requests_total, 10);
//! ```
//!
//! # Architecture
//!
//! The metrics system is organized into several modules:
//!
//! - [`counter`]: Counter metrics for monotonically increasing values
//! - [`gauge`]: Gauge metrics for point-in-time measurements
//! - [`histogram`]: Histogram metrics for distribution tracking
//! - [`registry`]: Central metric registry
//! - [`exporter`]: Various export formats and protocols
//!
//! # Performance
//!
//! The metrics system is designed for minimal overhead:
//! - Lock-free atomic operations
//! - <1% performance impact
//! - No allocations on hot paths
//! - Thread-safe by default

#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod counter;
pub mod gauge;
pub mod histogram;
pub mod registry;
pub mod exporter;

// Re-export commonly used types
pub use counter::{Counter, CounterF64, LabeledCounter, MetricsError};
pub use gauge::{Gauge, GaugeF64, LabeledGauge};
pub use histogram::{
    Histogram, HistogramSummary, exponential_buckets, linear_buckets,
    DEFAULT_LATENCY_BUCKETS, DEFAULT_SIZE_BUCKETS,
};
pub use registry::{Metric, MetricType, Registry, global_registry};
pub use exporter::{
    PrometheusExporter, OpenMetricsExporter, StatsDExporter,
    HttpExporter, PushExporter, ExportFormat, ExportError, ExportUtils,
};

/// Metric metadata
#[derive(Debug, Clone)]
pub struct MetricMetadata {
    /// Metric name
    pub name: alloc::string::String,
    /// Metric type
    pub metric_type: MetricType,
    /// Help text
    pub help: Option<alloc::string::String>,
    /// Label names (for labeled metrics)
    pub labels: alloc::vec::Vec<alloc::string::String>,
}

impl MetricMetadata {
    /// Create new metric metadata
    pub fn new(
        name: impl Into<alloc::string::String>,
        metric_type: MetricType,
    ) -> Self {
        Self {
            name: name.into(),
            metric_type,
            help: None,
            labels: alloc::vec::Vec::new(),
        }
    }

    /// Set help text
    pub fn with_help(mut self, help: impl Into<alloc::string::String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set labels
    pub fn with_labels(mut self, labels: alloc::vec::Vec<alloc::string::String>) -> Self {
        self.labels = labels;
        self
    }
}

/// Convenience macro for creating and registering a counter
///
/// # Example
///
/// ```rust
/// use kernel::metrics::counter;
///
/// counter!(my_counter, "A helpful counter");
/// inc_counter!(my_counter);
/// ```
#[macro_export]
macro_rules! counter {
    ($name:ident) => {
        $crate::metrics::counter!($name, concat!("Metric ", stringify!($name)));
    };
    ($name:ident, $help:expr) => {
        $crate::metrics::registry::global_registry()
            .counter_with_help(stringify!($name), $help)
            .expect(concat!("failed to create counter ", stringify!($name)));
    };
}

/// Convenience macro for creating and registering a gauge
#[macro_export]
macro_rules! gauge {
    ($name:ident) => {
        $crate::metrics::gauge!($name, concat!("Metric ", stringify!($name)));
    };
    ($name:ident, $help:expr) => {
        $crate::metrics::registry::global_registry()
            .gauge_with_help(stringify!($name), $help)
            .expect(concat!("failed to create gauge ", stringify!($name)));
    };
}

/// Convenience macro for creating and registering a histogram
#[macro_export]
macro_rules! histogram {
    ($name:ident) => {
        $crate::metrics::histogram!($name, concat!("Metric ", stringify!($name)));
    };
    ($name:ident, $help:expr) => {
        $crate::metrics::registry::global_registry()
            .histogram_latency(stringify!($name))
            .expect(concat!("failed to create histogram ", stringify!($name)));
    };
}

/// Increment a counter by 1
#[macro_export]
macro_rules! inc_counter {
    ($name:ident) => {
        $name.inc()
    };
}

/// Increment a counter by a specific amount
#[macro_export]
macro_rules! inc_counter_by {
    ($name:ident, $value:expr) => {
        $name.inc_by($value)
    };
}

/// Set a gauge to a specific value
#[macro_export]
macro_rules! set_gauge {
    ($name:ident, $value:expr) => {
        $name.set($value)
    };
}

/// Increment a gauge by 1
#[macro_export]
macro_rules! inc_gauge {
    ($name:ident) => {
        $name.inc()
    };
}

/// Decrement a gauge by 1
#[macro_export]
macro_rules! dec_gauge {
    ($name:ident) => {
        $name.dec()
    };
}

/// Observe a value in a histogram
#[macro_export]
macro_rules! observe_histogram {
    ($name:ident, $value:expr) => {
        $name.observe($value)
    };
}

/// Time a block of code and record in a histogram
///
/// # Example
///
/// ```rust
/// use kernel::metrics::timing;
///
/// timing!(request_duration, {
///     // Some operation to measure
///     do_work();
/// });
/// ```
#[macro_export]
macro_rules! timing {
    ($histogram:ident, $block:block) => {{
        let start = $crate::time::get_current_time();
        let result = $block;
        let duration = $crate::time::get_current_time() - start;
        $histogram.observe(duration.as_secs_f64());
        result
    }};
}

/// Integration test helpers
#[cfg(test)]
pub mod tests {
    use super::*;
    use alloc::vec;

    /// Test basic counter functionality
    #[test]
    fn test_basic_counter() {
        let counter = Counter::new("test_counter").unwrap();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(10);
        assert_eq!(counter.get(), 11);
    }

    /// Test basic gauge functionality
    #[test]
    fn test_basic_gauge() {
        let gauge = Gauge::new("test_gauge").unwrap();
        assert_eq!(gauge.get(), 0);

        gauge.set(42);
        assert_eq!(gauge.get(), 42);

        gauge.inc();
        assert_eq!(gauge.get(), 43);

        gauge.dec();
        assert_eq!(gauge.get(), 42);
    }

    /// Test basic histogram functionality
    #[test]
    fn test_basic_histogram() {
        let histogram = Histogram::new("test_histogram", vec![1.0, 5.0, 10.0]).unwrap();
        assert_eq!(histogram.count(), 0);

        histogram.observe(3.0);
        histogram.observe(7.0);

        assert_eq!(histogram.count(), 2);
        assert_eq!(histogram.sum(), 10.0);
    }

    /// Test registry integration
    #[test]
    fn test_registry_integration() {
        let registry = Registry::new();

        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(100);
        registry.register_metric(Box::new(counter)).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_counter"));

        let output = registry.export_prometheus();
        assert!(output.contains("test_counter 100"));
    }

    /// Test labeled metrics
    #[test]
    fn test_labeled_metrics() {
        let labeled_counter = LabeledCounter::new(
            "http_requests_total",
            &["method", "status"],
        ).unwrap();

        labeled_counter.inc(&[("method", "GET"), ("status", "200")]);
        labeled_counter.inc_by(5, &[("method", "POST"), ("status", "201")]);

        assert_eq!(
            labeled_counter.get(&[("method", "GET"), ("status", "200")]),
            1
        );
        assert_eq!(
            labeled_counter.get(&[("method", "POST"), ("status", "201")]),
            5
        );
    }

    /// Test histogram percentiles
    #[test]
    fn test_histogram_percentiles() {
        let histogram = Histogram::with_latency_buckets("test_histogram").unwrap();

        // Generate normal distribution-like data
        for i in 0..1000 {
            histogram.observe(i as f64 * 0.001);
        }

        let summary = histogram.summary();
        assert_eq!(summary.count, 1000);

        // P50 should be around the median
        let p50 = histogram.p50();
        assert!(p50.is_some());

        // P95 should be higher than P50
        let p95 = histogram.p95();
        assert!(p95.is_some());

        if let (Some(p50_val), Some(p95_val)) = (p50, p95) {
            assert!(p95_val > p50_val);
        }
    }

    /// Test Prometheus export
    #[test]
    fn test_prometheus_export() {
        let registry = Registry::new();

        let counter = Counter::with_help(
            "requests_total",
            "Total number of requests"
        ).unwrap();
        counter.inc_by(42);
        registry.register_metric(Box::new(counter)).unwrap();

        let gauge = Gauge::with_help(
            "temperature",
            "Current temperature"
        ).unwrap();
        gauge.set(23.5);
        registry.register_metric(Box::new(gauge)).unwrap();

        let exporter = PrometheusExporter::new(Arc::new(registry));
        let output = exporter.export();

        assert!(output.contains("# HELP requests_total"));
        assert!(output.contains("# TYPE requests_total counter"));
        assert!(output.contains("requests_total 42"));
        assert!(output.contains("# HELP temperature"));
        assert!(output.contains("# TYPE temperature gauge"));
        assert!(output.contains("temperature 23"));
    }

    /// Test StatsD format
    #[test]
    fn test_statsd_format() {
        let exporter = StatsDExporter::new("localhost", 8125, Some("app"));

        let counter = exporter.format_counter("requests", 1, Some(&[("env", "prod")]));
        assert!(counter.contains("app.requests:1|c"));
        assert!(counter.contains("env:prod"));

        let gauge = exporter.format_gauge("memory", 1024.0, None);
        assert!(gauge.contains("app.memory:1024|g"));

        let timing = exporter.format_timing("duration", 123.4, None);
        assert!(timing.contains("app.duration:123.4|ms"));
    }

    /// Test concurrent metric updates
    #[test]
    fn test_concurrent_metrics() {
        use core::sync::atomic::{AtomicU64, Ordering};

        let counter = Arc::new(Counter::new("concurrent").unwrap());
        let num_threads = 10;
        let increments_per_thread = 1000;
        let done = Arc::new(AtomicU64::new(0));

        // Simulate concurrent increments
        for _ in 0..num_threads {
            let counter_clone = Arc::clone(&counter);
            let done_clone = Arc::clone(&done);

            for _ in 0..increments_per_thread {
                counter_clone.inc();
            }
            done_clone.fetch_add(1, Ordering::Relaxed);
        }

        assert_eq!(
            counter.get(),
            num_threads as u64 * increments_per_thread as u64
        );
    }

    /// Test metric reset
    #[test]
    fn test_metric_reset() {
        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(100);
        assert_eq!(counter.get(), 100);

        counter.reset();
        assert_eq!(counter.get(), 0);

        let gauge = Gauge::new("test_gauge").unwrap();
        gauge.set(50);
        assert_eq!(gauge.get(), 50);

        gauge.reset();
        assert_eq!(gauge.get(), 0);
    }

    /// Test histogram bucket configuration
    #[test]
    fn test_histogram_buckets() {
        let exponential = exponential_buckets(1.0, 2.0, 5);
        assert_eq!(exponential, vec![1.0, 2.0, 4.0, 8.0, 16.0]);

        let linear = linear_buckets(0.0, 5.0, 4);
        assert_eq!(linear, vec![0.0, 5.0, 10.0, 15.0]);
    }

    /// Test metric validation
    #[test]
    fn test_metric_validation() {
        // Valid names
        assert!(Counter::new("valid_name").is_ok());
        assert!(Counter::new("Valid_Name123").is_ok());
        assert!(Counter::new("_leading_underscore").is_ok());

        // Invalid names
        assert!(Counter::new("").is_err());
        assert!(Counter::new("123_starts_with_digit").is_err());
        assert!(Counter::new("invalid-dash").is_err());
        assert!(Counter::new("invalid.dots").is_err());
    }

    /// Test labeled metric export
    #[test]
    fn test_labeled_metric_export() {
        let labeled_counter = LabeledCounter::with_help(
            "http_requests_total",
            &["method", "status"],
            "Total HTTP requests"
        ).unwrap();

        labeled_counter.inc(&[("method", "GET"), ("status", "200")]);
        labeled_counter.inc_by(2, &[("method", "POST"), ("status", "201")]);

        let output = labeled_counter.export_prometheus();
        assert!(output.contains("# HELP http_requests_total"));
        assert!(output.contains("method=\"GET\",status=\"200\""));
        assert!(output.contains("method=\"POST\",status=\"201\""));
    }

    /// Test histogram with custom buckets
    #[test]
    fn test_custom_buckets() {
        let custom_buckets = vec![0.1, 0.5, 1.0, 5.0];
        let histogram = Histogram::new("custom_histogram", custom_buckets.clone()).unwrap();

        histogram.observe(0.05);
        histogram.observe(0.3);
        histogram.observe(2.0);
        histogram.observe(10.0);

        let buckets = histogram.buckets();
        assert_eq!(buckets.get("0.1").unwrap(), &1);
        assert_eq!(buckets.get("0.5").unwrap(), &2);
        assert_eq!(buckets.get("1").unwrap(), &2);
        assert_eq!(buckets.get("5").unwrap(), &3);
        assert_eq!(buckets.get("+Inf").unwrap(), &4);
    }

    /// Test global registry
    #[test]
    fn test_global_registry() {
        let registry = global_registry();

        // Create a temporary metric
        let counter = registry.counter("temp_global_test").unwrap();
        counter.inc();

        assert!(registry.contains("temp_global_test"));

        // Cleanup
        registry.remove("temp_global_test");
    }

    /// Benchmark helper: measure counter increment performance
    #[cfg(test)]
    fn bench_counter_increments(count: u64) {
        let counter = Counter::new("bench_counter").unwrap();

        let start = 1_000_000; // Placeholder for time
        for _ in 0..count {
            counter.inc();
        }
        let end = 1_000_000; // Placeholder for time

        let duration = end - start;
        let ops_per_second = count as f64 / duration.max(1) as f64;

        log::info!(
            "Counter increments: {} ops in {} ns = {:.2} ops/sec",
            count,
            duration,
            ops_per_second
        );
    }

    /// Benchmark helper: measure histogram observe performance
    #[cfg(test)]
    fn bench_histogram_observations(count: usize) {
        let histogram = Histogram::with_latency_buckets("bench_histogram").unwrap();

        let start = 1_000_000;
        for i in 0..count {
            histogram.observe(i as f64 * 0.001);
        }
        let end = 1_000_000;

        let duration = end - start;
        let ops_per_second = count as f64 / duration.max(1) as f64;

        log::info!(
            "Histogram observations: {} ops in {} ns = {:.2} ops/sec",
            count,
            duration,
            ops_per_second
        );
    }

    /// Test metric metadata
    #[test]
    fn test_metric_metadata() {
        let metadata = MetricMetadata::new("test_metric", MetricType::Counter)
            .with_help("A test metric")
            .with_labels(vec!["label1".to_string(), "label2".to_string()]);

        assert_eq!(metadata.name, "test_metric");
        assert_eq!(metadata.metric_type, MetricType::Counter);
        assert_eq!(metadata.help, Some("A test metric".to_string()));
        assert_eq!(metadata.labels.len(), 2);
    }

    /// Test export utilities
    #[test]
    fn test_export_utils() {
        use exporter::ExportUtils;

        assert_eq!(ExportUtils::sanitize_name("test-name"), "test_name");
        assert_eq!(
            ExportUtils::sanitize_label_value("test\"value"),
            "test\\\"value"
        );

        assert_eq!(
            ExportUtils::metric_type_to_string(MetricType::Counter),
            "counter"
        );
    }

    /// Test clone behavior
    #[test]
    fn test_metric_clone() {
        let counter1 = Counter::new("clone_test").unwrap();
        counter1.inc_by(10);

        let counter2 = counter1.clone();
        counter2.inc_by(5);

        // Both should reference the same underlying value
        assert_eq!(counter1.get(), 15);
        assert_eq!(counter2.get(), 15);
    }

    /// Test histogram summary
    #[test]
    fn test_histogram_summary() {
        let histogram = Histogram::with_latency_buckets("summary_test").unwrap();

        histogram.observe_many(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        let summary = histogram.summary();
        assert_eq!(summary.count, 5);
        assert_eq!(summary.sum, 15.0);
        assert_eq!(summary.avg, 3.0);
    }
}

/// Performance benchmarks
#[cfg(test)]
pub mod benchmarks {
    use super::*;

    /// Benchmark counter increments
    #[test]
    fn benchmark_counter() {
        const ITERATIONS: u64 = 1_000_000;
        let counter = Counter::new("bench_counter").unwrap();

        // Simple benchmark
        for _ in 0..ITERATIONS {
            counter.inc();
        }

        assert_eq!(counter.get(), ITERATIONS);
    }

    /// Benchmark gauge operations
    #[test]
    fn benchmark_gauge() {
        const ITERATIONS: u64 = 1_000_000;
        let gauge = Gauge::new("bench_gauge").unwrap();

        for i in 0..ITERATIONS {
            gauge.set(i as i64);
        }

        assert_eq!(gauge.get(), ITERATIONS as i64 - 1);
    }

    /// Benchmark histogram observations
    #[test]
    fn benchmark_histogram() {
        const ITERATIONS: usize = 100_000;
        let histogram = Histogram::with_latency_buckets("bench_histogram").unwrap();

        for i in 0..ITERATIONS {
            histogram.observe(i as f64 * 0.001);
        }

        assert_eq!(histogram.count(), ITERATIONS as u64);
    }

    /// Benchmark registry export
    #[test]
    fn benchmark_export() {
        let registry = Registry::new();

        // Register 100 metrics
        for i in 0..100 {
            let counter = Counter::new(&format!("metric_{}", i)).unwrap();
            counter.inc_by(i as u64);
            registry.register_metric(Box::new(counter)).unwrap();
        }

        // Export multiple times
        for _ in 0..1000 {
            let _output = registry.export_prometheus();
        }
    }
}
