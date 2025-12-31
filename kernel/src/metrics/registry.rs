//! Metrics registry for centralized metric management
//!
//! This module provides a thread-safe registry for storing and accessing metrics.
//! The registry serves as the central point for metric registration, lookup,
//! and export.
//!
//! # Example
//!
//! ```rust
//! use kernel::metrics::registry::{Registry, MetricType};
//! use kernel::metrics::counter::Counter;
//!
//! let registry = Registry::new();
//!
//! // Register a counter
//! let counter = Counter::new("requests_total").unwrap();
//! registry.register("requests_total", MetricType::Counter, Box::new(counter));
//!
//! // Export all metrics
//! let output = registry.export_prometheus();
//! ```

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::fmt;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;
use spin::RwLock;

use super::counter::{Counter, CounterF64, MetricsError};
use super::gauge::{Gauge, GaugeF64};
use super::histogram::Histogram;

/// Metric type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricType {
    /// Counter metric (monotonically increasing)
    Counter,
    /// Gauge metric (point-in-time value)
    Gauge,
    /// Histogram metric (distribution)
    Histogram,
    /// Summary metric (not yet implemented)
    Summary,
}

impl Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricType::Counter => write!(f, "counter"),
            MetricType::Gauge => write!(f, "gauge"),
            MetricType::Histogram => write!(f, "histogram"),
            MetricType::Summary => write!(f, "summary"),
        }
    }
}

/// Trait for metrics that can be registered
pub trait Metric: Display {
    /// Get the metric name
    fn name(&self) -> &str;

    /// Get the metric type
    fn metric_type(&self) -> MetricType;

    /// Get the help text
    fn help(&self) -> Option<&str>;

    /// Export in Prometheus format
    fn export_prometheus(&self) -> String;

    /// Clone the metric (for internal use)
    fn clone_box(&self) -> Box<dyn Metric>;
}

impl Clone for Box<dyn Metric> {
    fn clone(&self) -> Box<dyn Metric> {
        self.clone_box()
    }
}

// Implement Metric for Counter
impl Metric for Counter {
    fn name(&self) -> &str {
        self.name()
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Counter
    }

    fn help(&self) -> Option<&str> {
        self.help()
    }

    fn export_prometheus(&self) -> String {
        self.export_prometheus()
    }

    fn clone_box(&self) -> Box<dyn Metric> {
        Box::new(self.clone())
    }
}

impl Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Counter({})", self.name())
    }
}

// Implement Metric for CounterF64
impl Metric for CounterF64 {
    fn name(&self) -> &str {
        self.name()
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Counter
    }

    fn help(&self) -> Option<&str> {
        self.help()
    }

    fn export_prometheus(&self) -> String {
        self.export_prometheus()
    }

    fn clone_box(&self) -> Box<dyn Metric> {
        Box::new(self.clone())
    }
}

impl Display for CounterF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CounterF64({})", self.name())
    }
}

// Implement Metric for Gauge
impl Metric for Gauge {
    fn name(&self) -> &str {
        self.name()
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Gauge
    }

    fn help(&self) -> Option<&str> {
        self.help()
    }

    fn export_prometheus(&self) -> String {
        self.export_prometheus()
    }

    fn clone_box(&self) -> Box<dyn Metric> {
        Box::new(self.clone())
    }
}

impl Display for Gauge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gauge({})", self.name())
    }
}

// Implement Metric for GaugeF64
impl Metric for GaugeF64 {
    fn name(&self) -> &str {
        self.name()
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Gauge
    }

    fn help(&self) -> Option<&str> {
        self.help()
    }

    fn export_prometheus(&self) -> String {
        self.export_prometheus()
    }

    fn clone_box(&self) -> Box<dyn Metric> {
        Box::new(self.clone())
    }
}

impl Display for GaugeF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GaugeF64({})", self.name())
    }
}

// Implement Metric for Histogram
impl Metric for Histogram {
    fn name(&self) -> &str {
        self.name()
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Histogram
    }

    fn help(&self) -> Option<&str> {
        self.help()
    }

    fn export_prometheus(&self) -> String {
        self.export_prometheus()
    }

    fn clone_box(&self) -> Box<dyn Metric> {
        Box::new(self.clone())
    }
}

impl Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Histogram({})", self.name())
    }
}

/// Metric registry for centralized storage
///
/// The registry provides thread-safe storage for all metrics and handles
/// registration, lookup, and bulk export operations.
///
/// # Thread Safety
///
/// Uses RwLock for concurrent read access and exclusive write access.
pub struct Registry {
    metrics: RwLock<BTreeMap<String, Box<dyn Metric>>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Create a new empty registry
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::registry::Registry;
    ///
    /// let registry = Registry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a metric
    ///
    /// Returns an error if a metric with the same name already exists.
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `metric` - The metric to register
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::registry::Registry;
    /// use kernel::metrics::counter::Counter;
    ///
    /// let registry = Registry::new();
    /// let counter = Counter::new("requests_total").unwrap();
    ///
    /// registry.register_metric(Box::new(counter)).unwrap();
    /// ```
    pub fn register_metric(&self, metric: Box<dyn Metric>) -> Result<(), MetricsError> {
        let name = metric.name().to_string();

        if let Some(mut metrics) = self.metrics.try_write() {
            if metrics.contains_key(&name) {
                return Err(MetricsError::AlreadyRegistered(name));
            }

            metrics.insert(name, metric);
            Ok(())
        } else {
            Err(MetricsError::RegistryError("Failed to acquire write lock".into()))
        }
    }

    /// Register or get a counter
    ///
    /// If a counter with this name exists, returns it. Otherwise creates and
    /// registers a new one.
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    pub fn counter(&self, name: impl Into<String>) -> Result<Counter, MetricsError> {
        let name = name.into();
        super::counter::validate_metric_name(&name)?;

        // Try to get existing metric
        {
            if let Some(metrics) = self.metrics.try_read() {
                if metrics.get(&name).is_some() {
                    return Ok(Counter::new(&name)?);
                }
            }
        }

        // Create and register new counter
        let counter = Counter::new(&name)?;
        self.register_metric(Box::new(counter.clone()))?;
        Ok(counter)
    }

    /// Register or get a counter with help text
    pub fn counter_with_help(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
    ) -> Result<Counter, MetricsError> {
        let name = name.into();
        super::counter::validate_metric_name(&name)?;

        let counter = Counter::with_help(&name, help)?;
        self.register_metric(Box::new(counter.clone()))?;
        Ok(counter)
    }

    /// Register or get a gauge
    pub fn gauge(&self, name: impl Into<String>) -> Result<Gauge, MetricsError> {
        let name = name.into();
        super::counter::validate_metric_name(&name)?;

        {
            if let Some(metrics) = self.metrics.try_read() {
                if metrics.get(&name).is_some() {
                    return Ok(Gauge::new(&name)?);
                }
            }
        }

        let gauge = Gauge::new(&name)?;
        self.register_metric(Box::new(gauge.clone()))?;
        Ok(gauge)
    }

    /// Register or get a gauge with help text
    pub fn gauge_with_help(
        &self,
        name: impl Into<String>,
        help: impl Into<String>,
    ) -> Result<Gauge, MetricsError> {
        let name = name.into();
        super::counter::validate_metric_name(&name)?;

        let gauge = Gauge::with_help(&name, help)?;
        self.register_metric(Box::new(gauge.clone()))?;
        Ok(gauge)
    }

    /// Register or get a histogram
    pub fn histogram(
        &self,
        name: impl Into<String>,
        buckets: Vec<f64>,
    ) -> Result<Histogram, MetricsError> {
        let name = name.into();
        super::counter::validate_metric_name(&name)?;

        {
            if let Some(metrics) = self.metrics.try_read() {
                if metrics.get(&name).is_some() {
                    return Ok(Histogram::new(&name, buckets)?);
                }
            }
        }

        let histogram = Histogram::new(&name, buckets)?;
        self.register_metric(Box::new(histogram.clone()))?;
        Ok(histogram)
    }

    /// Register or get a histogram with default latency buckets
    pub fn histogram_latency(&self, name: impl Into<String>) -> Result<Histogram, MetricsError> {
        let name = name.into();
        super::counter::validate_metric_name(&name)?;

        {
            if let Some(metrics) = self.metrics.try_read() {
                if metrics.get(&name).is_some() {
                    return Ok(Histogram::with_latency_buckets(&name)?);
                }
            }
        }

        let histogram = Histogram::with_latency_buckets(&name)?;
        self.register_metric(Box::new(histogram.clone()))?;
        Ok(histogram)
    }

    /// Get a metric by name
    ///
    /// Returns None if the metric doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    pub fn get(&self, name: &str) -> Option<Box<dyn Metric>> {
        if let Some(metrics) = self.metrics.try_read() {
            metrics.get(name).map(|m| m.clone_box())
        } else {
            None
        }
    }

    /// Check if a metric exists
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    pub fn contains(&self, name: &str) -> bool {
        if let Some(metrics) = self.metrics.try_read() {
            metrics.contains_key(name)
        } else {
            false
        }
    }

    /// Remove a metric
    ///
    /// Returns the removed metric, or None if it didn't exist.
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    pub fn remove(&self, name: &str) -> Option<Box<dyn Metric>> {
        if let Some(mut metrics) = self.metrics.try_write() {
            metrics.remove(name)
        } else {
            None
        }
    }

    /// Clear all metrics
    pub fn clear(&self) {
        if let Some(mut metrics) = self.metrics.try_write() {
            metrics.clear();
        }
    }

    /// Get the number of registered metrics
    pub fn len(&self) -> usize {
        if let Some(metrics) = self.metrics.try_read() {
            metrics.len()
        } else {
            0
        }
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        if let Some(metrics) = self.metrics.try_read() {
            metrics.is_empty()
        } else {
            true
        }
    }

    /// Get all metric names
    pub fn names(&self) -> Vec<String> {
        if let Some(metrics) = self.metrics.try_read() {
            metrics.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Export all metrics in Prometheus text format
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::registry::Registry;
    /// use kernel::metrics::counter::Counter;
    ///
    /// let registry = Registry::new();
    /// let counter = Counter::new("test_counter").unwrap();
    /// counter.inc_by(42);
    /// registry.register_metric(Box::new(counter)).unwrap();
    ///
    /// let output = registry.export_prometheus();
    /// assert!(output.contains("test_counter 42"));
    /// ```
    pub fn export_prometheus(&self) -> String {
        let mut result = String::new();

        if let Some(metrics) = self.metrics.try_read() {
            for metric in metrics.values() {
                result.push_str(&metric.export_prometheus());
                result.push('\n');
            }
        }

        result
    }

    /// Get metrics grouped by type
    pub fn metrics_by_type(&self) -> BTreeMap<MetricType, Vec<String>> {
        let mut result: BTreeMap<MetricType, Vec<String>> = BTreeMap::new();

        if let Some(metrics) = self.metrics.try_read() {
            for (name, metric) in metrics.iter() {
                result
                    .entry(metric.metric_type())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
            }
        }

        result
    }

    /// Get metrics count by type
    pub fn count_by_type(&self) -> BTreeMap<MetricType, usize> {
        let by_type = self.metrics_by_type();
        by_type
            .into_iter()
            .map(|(k, v)| (k, v.len()))
            .collect()
    }
}

/// Global registry singleton
///
/// This provides a convenient way to access a global default registry.
static mut GLOBAL_REGISTRY: Option<Registry> = None;
static INIT: spin::Once = spin::Once::new();

/// Get the global registry
///
/// # Safety
///
/// This function uses static mutable state and is not thread-safe during
/// initialization. Call this only after the first thread has started.
pub fn global_registry() -> &'static Registry {
    INIT.call_once(|| {
        unsafe {
            GLOBAL_REGISTRY = Some(Registry::new());
        }
    });

    unsafe { GLOBAL_REGISTRY.as_ref().expect("Global registry not initialized") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_empty() {
        let registry = Registry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.names().is_empty());
    }

    #[test]
    fn test_registry_register_counter() {
        let registry = Registry::new();
        let counter = Counter::new("test_counter").unwrap();

        assert!(registry.register_metric(Box::new(counter)).is_ok());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_counter"));
    }

    #[test]
    fn test_registry_duplicate() {
        let registry = Registry::new();
        let counter1 = Counter::new("test_counter").unwrap();
        let counter2 = Counter::new("test_counter").unwrap();

        assert!(registry.register_metric(Box::new(counter1)).is_ok());
        assert!(registry.register_metric(Box::new(counter2)).is_err());
    }

    #[test]
    fn test_registry_get() {
        let registry = Registry::new();
        let counter = Counter::new("test_counter").unwrap();
        counter.inc_by(42);

        registry.register_metric(Box::new(counter)).unwrap();

        let retrieved = registry.get("test_counter");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test_counter");

        let missing = registry.get("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_remove() {
        let registry = Registry::new();
        let counter = Counter::new("test_counter").unwrap();
        registry.register_metric(Box::new(counter)).unwrap();

        assert!(registry.contains("test_counter"));
        registry.remove("test_counter");
        assert!(!registry.contains("test_counter"));
    }

    #[test]
    fn test_registry_clear() {
        let registry = Registry::new();
        registry.register_metric(Box::new(Counter::new("metric1").unwrap())).unwrap();
        registry.register_metric(Box::new(Counter::new("metric2").unwrap())).unwrap();

        assert_eq!(registry.len(), 2);
        registry.clear();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_export() {
        let registry = Registry::new();

        let counter = Counter::with_help("requests_total", "Total requests").unwrap();
        counter.inc_by(100);

        let gauge = Gauge::with_help("active_connections", "Active connections").unwrap();
        gauge.set(42);

        registry.register_metric(Box::new(counter)).unwrap();
        registry.register_metric(Box::new(gauge)).unwrap();

        let output = registry.export_prometheus();
        assert!(output.contains("# HELP requests_total Total requests"));
        assert!(output.contains("# HELP active_connections Active connections"));
        assert!(output.contains("requests_total 100"));
        assert!(output.contains("active_connections 42"));
    }

    #[test]
    fn test_registry_counter() {
        let registry = Registry::new();
        let counter = registry.counter("test_counter").unwrap();

        counter.inc_by(10);
        assert_eq!(counter.get(), 10);
        assert!(registry.contains("test_counter"));
    }

    #[test]
    fn test_registry_gauge() {
        let registry = Registry::new();
        let gauge = registry.gauge("test_gauge").unwrap();

        gauge.set(25);
        assert_eq!(gauge.get(), 25);
        assert!(registry.contains("test_gauge"));
    }

    #[test]
    fn test_registry_histogram() {
        let registry = Registry::new();
        let histogram = registry
            .histogram("test_histogram", vec![1.0, 5.0, 10.0])
            .unwrap();

        histogram.observe(3.0);
        assert_eq!(histogram.count(), 1);
        assert!(registry.contains("test_histogram"));
    }

    #[test]
    fn test_registry_metrics_by_type() {
        let registry = Registry::new();

        registry
            .register_metric(Box::new(Counter::new("counter1").unwrap()))
            .unwrap();
        registry
            .register_metric(Box::new(Gauge::new("gauge1").unwrap()))
            .unwrap();
        registry
            .register_metric(Box::new(Gauge::new("gauge2").unwrap()))
            .unwrap();

        let by_type = registry.metrics_by_type();
        assert_eq!(by_type.get(&MetricType::Counter).unwrap().len(), 1);
        assert_eq!(by_type.get(&MetricType::Gauge).unwrap().len(), 2);
    }

    #[test]
    fn test_registry_count_by_type() {
        let registry = Registry::new();

        registry
            .register_metric(Box::new(Counter::new("counter1").unwrap()))
            .unwrap();
        registry
            .register_metric(Box::new(Gauge::new("gauge1").unwrap()))
            .unwrap();

        let counts = registry.count_by_type();
        assert_eq!(counts.get(&MetricType::Counter), Some(&1));
        assert_eq!(counts.get(&MetricType::Gauge), Some(&1));
    }

    #[test]
    fn test_global_registry() {
        let registry = global_registry();

        let counter = registry.counter("global_test").unwrap();
        counter.inc();

        assert!(registry.contains("global_test"));

        // Cleanup
        registry.remove("global_test");
    }
}
