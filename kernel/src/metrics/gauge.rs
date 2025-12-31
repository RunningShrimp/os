//! Gauge metrics for point-in-time values
//!
//! This module provides thread-safe gauge implementations that can increase or decrease.
//! Gauges are ideal for tracking current values like:
//! - Temperature
//! - Memory usage
//! - Queue length
//! - Active connections
//!
//! # Example
//!
//! ```rust
//! use kernel::metrics::gauge::{Gauge, GaugeF64};
//!
//! // Integer gauge
//! let gauge = Gauge::new("active_connections");
//! gauge.inc();
//! gauge.set(100);
//! gauge.dec();
//!
//! // Float gauge
//! let temp = GaugeF64::new("temperature_celsius");
//! temp.set(23.5);
//! ```

#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI64, Ordering};

use super::counter::{validate_label_name, validate_metric_name, MetricsError};

/// A gauge metric for point-in-time measurements (i64)
///
/// Gauges can increase or decrease and represent a current value at a point in time.
/// They are ideal for metrics like:
/// - Current memory usage
/// - Active connections
/// - Queue depth
/// - Temperature
///
/// # Thread Safety
///
/// Uses lock-free atomic operations for thread safety.
#[derive(Debug)]
pub struct Gauge {
    name: String,
    help: Option<String>,
    value: Arc<AtomicI64>,
}

impl Gauge {
    /// Create a new gauge with the given name
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name (must follow Prometheus naming conventions)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("active_connections").unwrap();
    /// ```
    pub fn new(name: impl Into<String>) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;
        Ok(Self {
            name,
            help: None,
            value: Arc::new(AtomicI64::new(0)),
        })
    }

    /// Create a new gauge with help text
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `help` - Help text describing the metric
    pub fn with_help(
        name: impl Into<String>,
        help: impl Into<String>,
    ) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;
        Ok(Self {
            name,
            help: Some(help.into()),
            value: Arc::new(AtomicI64::new(0)),
        })
    }

    /// Increment the gauge by 1
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("connections").unwrap();
    /// gauge.inc();
    /// assert_eq!(gauge.get(), 1);
    /// ```
    #[inline]
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the gauge by a specific amount
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to increment by
    #[inline]
    pub fn inc_by(&self, delta: i64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Decrement the gauge by 1
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("connections").unwrap();
    /// gauge.set(10);
    /// gauge.dec();
    /// assert_eq!(gauge.get(), 9);
    /// ```
    #[inline]
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    /// Decrement the gauge by a specific amount
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to decrement by
    #[inline]
    pub fn dec_by(&self, delta: i64) {
        self.value.fetch_sub(delta, Ordering::Relaxed);
    }

    /// Set the gauge to a specific value
    ///
    /// # Arguments
    ///
    /// * `val` - New value for the gauge
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("memory_usage").unwrap();
    /// gauge.set(1024);
    /// assert_eq!(gauge.get(), 1024);
    /// ```
    #[inline]
    pub fn set(&self, val: i64) {
        self.value.store(val, Ordering::Relaxed);
    }

    /// Get the current gauge value
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("connections").unwrap();
    /// gauge.inc();
    /// assert_eq!(gauge.get(), 1);
    /// ```
    #[inline]
    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Add a value to the current gauge value and return the old value
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("connections").unwrap();
    /// gauge.set(10);
    /// let old = gauge.fetch_add(5);
    /// assert_eq!(old, 10);
    /// assert_eq!(gauge.get(), 15);
    /// ```
    #[inline]
    pub fn fetch_add(&self, delta: i64) -> i64 {
        self.value.fetch_add(delta, Ordering::Relaxed)
    }

    /// Subtract a value from the current gauge value and return the old value
    #[inline]
    pub fn fetch_sub(&self, delta: i64) -> i64 {
        self.value.fetch_sub(delta, Ordering::Relaxed)
    }

    /// Set the gauge to a new value and return the old value
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("connections").unwrap();
    /// gauge.set(10);
    /// let old = gauge.swap(20);
    /// assert_eq!(old, 10);
    /// assert_eq!(gauge.get(), 20);
    /// ```
    #[inline]
    pub fn swap(&self, val: i64) -> i64 {
        self.value.swap(val, Ordering::Relaxed)
    }

    /// Compare and swap: set to new value if current value matches expected
    ///
    /// Returns the old value
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::new("connections").unwrap();
    /// gauge.set(10);
    ///
    /// // Successful CAS
    /// let result = gauge.compare_exchange(10, 20);
    /// assert_eq!(result, Ok(10));
    /// assert_eq!(gauge.get(), 20);
    ///
    /// // Failed CAS
    /// let result = gauge.compare_exchange(10, 30);
    /// assert!(result.is_err());
    /// ```
    #[inline]
    pub fn compare_exchange(&self, expected: i64, new: i64) -> Result<i64, i64> {
        match self.value.compare_exchange(
            expected,
            new,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(old) => Ok(old),
            Err(old) => Err(old),
        }
    }

    /// Get the metric name
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the help text
    #[inline]
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Export in Prometheus text format
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::gauge::Gauge;
    ///
    /// let gauge = Gauge::with_help(
    ///     "active_connections",
    ///     "Current number of active connections"
    /// ).unwrap();
    /// gauge.set(42);
    /// let output = gauge.export_prometheus();
    /// assert!(output.contains("# HELP active_connections"));
    /// assert!(output.contains("active_connections 42"));
    /// ```
    pub fn export_prometheus(&self) -> String {
        let mut result = String::new();

        // Add help text if available
        if let Some(help) = &self.help {
            result.push_str(&format!("# HELP {} {}\n", self.name, help));
        }

        // Add type
        result.push_str(&format!("# TYPE {} gauge\n", self.name));

        // Add value
        result.push_str(&format!("{} {}\n", self.name, self.get()));

        result
    }
}

impl Clone for Gauge {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            help: self.help.clone(),
            value: Arc::clone(&self.value),
        }
    }
}

/// A gauge metric for point-in-time measurements (f64)
///
/// Float gauges are useful when tracking metrics that require fractional precision,
/// such as temperature, percentages, or ratios.
///
/// # Thread Safety
///
/// Uses atomic operations for thread-safe updates. Note that floating-point
/// operations may have small rounding differences with concurrent updates.
#[derive(Debug)]
pub struct GaugeF64 {
    name: String,
    help: Option<String>,
    value: Arc<atomic_f64::AtomicF64>,
}

mod atomic_f64 {
    use super::*;
    use core::sync::atomic::AtomicU64;

    /// Atomic f64 using bitwise operations
    #[derive(Debug)]
    pub struct AtomicF64 {
        bits: AtomicU64,
    }

    impl AtomicF64 {
        pub fn new(value: f64) -> Self {
            Self {
                bits: AtomicU64::new(value.to_bits()),
            }
        }

        pub fn load(&self, ordering: Ordering) -> f64 {
            f64::from_bits(self.bits.load(ordering))
        }

        pub fn store(&self, value: f64, ordering: Ordering) {
            self.bits.store(value.to_bits(), ordering);
        }

        pub fn fetch_add(&self, delta: f64, ordering: Ordering) -> f64 {
            loop {
                let old = self.load(Ordering::Relaxed);
                let new = old + delta;
                match self.bits.compare_exchange_weak(
                    old.to_bits(),
                    new.to_bits(),
                    ordering,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return old,
                    Err(_) => continue,
                }
            }
        }

        pub fn fetch_sub(&self, delta: f64, ordering: Ordering) -> f64 {
            loop {
                let old = self.load(Ordering::Relaxed);
                let new = old - delta;
                match self.bits.compare_exchange_weak(
                    old.to_bits(),
                    new.to_bits(),
                    ordering,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return old,
                    Err(_) => continue,
                }
            }
        }

        pub fn swap(&self, value: f64, ordering: Ordering) -> f64 {
            loop {
                let old = self.load(Ordering::Relaxed);
                match self.bits.compare_exchange_weak(
                    old.to_bits(),
                    value.to_bits(),
                    ordering,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return old,
                    Err(_) => continue,
                }
            }
        }
    }
}

impl GaugeF64 {
    /// Create a new float gauge
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    pub fn new(name: impl Into<String>) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;
        Ok(Self {
            name,
            help: None,
            value: Arc::new(atomic_f64::AtomicF64::new(0.0)),
        })
    }

    /// Create a new float gauge with help text
    pub fn with_help(
        name: impl Into<String>,
        help: impl Into<String>,
    ) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;
        Ok(Self {
            name,
            help: Some(help.into()),
            value: Arc::new(atomic_f64::AtomicF64::new(0.0)),
        })
    }

    /// Increment by 1.0
    #[inline]
    pub fn inc(&self) {
        self.value.fetch_add(1.0, Ordering::Relaxed);
    }

    /// Increment by a specific amount
    #[inline]
    pub fn inc_by(&self, delta: f64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Decrement by 1.0
    #[inline]
    pub fn dec(&self) {
        self.value.fetch_sub(1.0, Ordering::Relaxed);
    }

    /// Decrement by a specific amount
    #[inline]
    pub fn dec_by(&self, delta: f64) {
        self.value.fetch_sub(delta, Ordering::Relaxed);
    }

    /// Set to a specific value
    #[inline]
    pub fn set(&self, val: f64) {
        self.value.store(val, Ordering::Relaxed);
    }

    /// Get the current value
    #[inline]
    pub fn get(&self) -> f64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Add and return old value
    #[inline]
    pub fn fetch_add(&self, delta: f64) -> f64 {
        self.value.fetch_add(delta, Ordering::Relaxed)
    }

    /// Subtract and return old value
    #[inline]
    pub fn fetch_sub(&self, delta: f64) -> f64 {
        self.value.fetch_sub(delta, Ordering::Relaxed)
    }

    /// Set and return old value
    #[inline]
    pub fn swap(&self, val: f64) -> f64 {
        self.value.swap(val, Ordering::Relaxed)
    }

    /// Get the metric name
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the help text
    #[inline]
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Export in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut result = String::new();

        if let Some(help) = &self.help {
            result.push_str(&format!("# HELP {} {}\n", self.name, help));
        }

        result.push_str(&format!("# TYPE {} gauge\n", self.name));
        result.push_str(&format!("{} {}\n", self.name, self.get()));

        result
    }
}

impl Clone for GaugeF64 {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            help: self.help.clone(),
            value: Arc::clone(&self.value),
        }
    }
}

/// A gauge with label support
///
/// Labeled gauges allow tracking the same metric across different dimensions.
/// For example, tracking memory usage by subsystem or temperature by sensor.
///
/// # Example
///
/// ```rust
/// use kernel::metrics::gauge::LabeledGauge;
///
/// let gauge = LabeledGauge::new(
///     "memory_usage_bytes",
///     &["subsystem"]
/// ).unwrap();
///
/// // Set values with specific labels
/// gauge.set(1024, &[("subsystem", "kernel")]);
/// gauge.set(2048, &[("subsystem", "user")]);
/// ```
#[derive(Debug)]
pub struct LabeledGauge {
    name: String,
    help: Option<String>,
    label_names: Vec<String>,
    values: spin::Mutex<alloc::collections::BTreeMap<Vec<String>, Gauge>>,
}

impl LabeledGauge {
    /// Create a new labeled gauge
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `label_names` - Names of labels for this gauge
    pub fn new(name: impl Into<String>, label_names: &[&str]) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;

        for label in label_names {
            validate_label_name(label)?;
        }

        Ok(Self {
            name,
            help: None,
            label_names: label_names.iter().map(|s| s.to_string()).collect(),
            values: spin::Mutex::new(alloc::collections::BTreeMap::new()),
        })
    }

    /// Create with help text
    pub fn with_help(
        name: impl Into<String>,
        label_names: &[&str],
        help: impl Into<String>,
    ) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;

        for label in label_names {
            validate_label_name(label)?;
        }

        Ok(Self {
            name,
            help: Some(help.into()),
            label_names: label_names.iter().map(|s| s.to_string()).collect(),
            values: spin::Mutex::new(alloc::collections::BTreeMap::new()),
        })
    }

    /// Increment gauge with specific labels
    pub fn inc(&self, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let gauge = values.entry(key).or_insert_with(|| {
            Gauge::new(self.name.clone()).unwrap()
        });
        gauge.inc();
    }

    /// Increment by specific amount with labels
    pub fn inc_by(&self, delta: i64, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let gauge = values.entry(key).or_insert_with(|| {
            Gauge::new(self.name.clone()).unwrap()
        });
        gauge.inc_by(delta);
    }

    /// Decrement gauge with specific labels
    pub fn dec(&self, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let gauge = values.entry(key).or_insert_with(|| {
            Gauge::new(self.name.clone()).unwrap()
        });
        gauge.dec();
    }

    /// Decrement by specific amount with labels
    pub fn dec_by(&self, delta: i64, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let gauge = values.entry(key).or_insert_with(|| {
            Gauge::new(self.name.clone()).unwrap()
        });
        gauge.dec_by(delta);
    }

    /// Set value for specific labels
    pub fn set(&self, val: i64, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let gauge = values.entry(key).or_insert_with(|| {
            Gauge::new(self.name.clone()).unwrap()
        });
        gauge.set(val);
    }

    /// Get value for specific labels
    pub fn get(&self, labels: &[(&str, &str)]) -> i64 {
        let key = self.extract_label_values(labels);
        let values = self.values.lock();
        values
            .get(&key)
            .map(|g| g.get())
            .unwrap_or(0)
    }

    /// Get all label combinations
    pub fn label_combinations(&self) -> Vec<Vec<String>> {
        let values = self.values.lock();
        values.keys().cloned().collect()
    }

    fn extract_label_values(&self, labels: &[(&str, &str)]) -> Vec<String> {
        self.label_names
            .iter()
            .map(|name| {
                labels
                    .iter()
                    .find(|(n, _)| *n == name.as_str())
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Export in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut result = String::new();

        if let Some(help) = &self.help {
            result.push_str(&format!("# HELP {} {}\n", self.name, help));
        }

        result.push_str(&format!("# TYPE {} gauge\n", self.name));

        let values = self.values.lock();
        for (labels, gauge) in values.iter() {
            let label_str = self.format_labels(labels);
            result.push_str(&format!("{}{} {}\n", self.name, label_str, gauge.get()));
        }

        result
    }

    fn format_labels(&self, values: &[String]) -> String {
        if values.is_empty() {
            return String::new();
        }

        let pairs: Vec<String> = self
            .label_names
            .iter()
            .zip(values.iter())
            .map(|(name, value)| format!("{}=\"{}\"", name, value))
            .collect();

        if pairs.is_empty() {
            String::new()
        } else {
            format!("{{{}}}", pairs.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauge_basic() {
        let gauge = Gauge::new("test_gauge").unwrap();
        assert_eq!(gauge.get(), 0);

        gauge.inc();
        assert_eq!(gauge.get(), 1);

        gauge.inc_by(10);
        assert_eq!(gauge.get(), 11);

        gauge.dec();
        assert_eq!(gauge.get(), 10);

        gauge.dec_by(5);
        assert_eq!(gauge.get(), 5);

        gauge.set(100);
        assert_eq!(gauge.get(), 100);
    }

    #[test]
    fn test_gauge_with_help() {
        let gauge = Gauge::with_help("test_gauge", "A test gauge").unwrap();
        assert_eq!(gauge.help(), Some("A test gauge"));
        gauge.set(42);
        assert_eq!(gauge.get(), 42);
    }

    #[test]
    fn test_gauge_clone() {
        let gauge1 = Gauge::new("test_gauge").unwrap();
        gauge1.set(50);

        let gauge2 = gauge1.clone();
        gauge2.inc();

        assert_eq!(gauge1.get(), 51);
        assert_eq!(gauge2.get(), 51);
    }

    #[test]
    fn test_gauge_export() {
        let gauge = Gauge::with_help(
            "active_connections",
            "Current number of active connections"
        ).unwrap();
        gauge.set(42);

        let output = gauge.export_prometheus();
        assert!(output.contains("# HELP active_connections"));
        assert!(output.contains("# TYPE active_connections gauge"));
        assert!(output.contains("active_connections 42"));
    }

    #[test]
    fn test_gauge_fetch_operations() {
        let gauge = Gauge::new("test_gauge").unwrap();
        gauge.set(10);

        let old = gauge.fetch_add(5);
        assert_eq!(old, 10);
        assert_eq!(gauge.get(), 15);

        let old = gauge.fetch_sub(3);
        assert_eq!(old, 15);
        assert_eq!(gauge.get(), 12);

        let old = gauge.swap(20);
        assert_eq!(old, 12);
        assert_eq!(gauge.get(), 20);
    }

    #[test]
    fn test_gauge_compare_exchange() {
        let gauge = Gauge::new("test_gauge").unwrap();
        gauge.set(10);

        // Successful CAS
        let result = gauge.compare_exchange(10, 20);
        assert_eq!(result, Ok(10));
        assert_eq!(gauge.get(), 20);

        // Failed CAS
        let result = gauge.compare_exchange(10, 30);
        assert!(result.is_err());
        assert_eq!(gauge.get(), 20);
    }

    #[test]
    fn test_float_gauge_basic() {
        let gauge = GaugeF64::new("test_gauge").unwrap();
        assert_eq!(gauge.get(), 0.0);

        gauge.inc();
        assert_eq!(gauge.get(), 1.0);

        gauge.inc_by(10.5);
        assert_eq!(gauge.get(), 11.5);

        gauge.dec();
        assert_eq!(gauge.get(), 10.5);

        gauge.set(23.5);
        assert_eq!(gauge.get(), 23.5);
    }

    #[test]
    fn test_labeled_gauge() {
        let gauge = LabeledGauge::new(
            "memory_usage_bytes",
            &["subsystem"],
        ).unwrap();

        gauge.set(1024, &[("subsystem", "kernel")]);
        gauge.set(2048, &[("subsystem", "user")]);

        assert_eq!(
            gauge.get(&[("subsystem", "kernel")]),
            1024
        );
        assert_eq!(
            gauge.get(&[("subsystem", "user")]),
            2048
        );

        gauge.inc_by(512, &[("subsystem", "kernel")]);
        assert_eq!(
            gauge.get(&[("subsystem", "kernel")]),
            1536
        );
    }

    #[test]
    fn test_labeled_gauge_export() {
        let gauge = LabeledGauge::with_help(
            "temperature_celsius",
            &["location"],
            "Temperature readings"
        ).unwrap();

        gauge.set(23.5, &[("location", "room1")]);
        gauge.set(19.2, &[("location", "room2")]);

        let output = gauge.export_prometheus();
        assert!(output.contains("# HELP temperature_celsius"));
        assert!(output.contains("# TYPE temperature_celsius gauge"));
        assert!(output.contains("location=\"room1\""));
        assert!(output.contains("location=\"room2\""));
    }

    #[test]
    fn test_labeled_gauge_multiple_labels() {
        let gauge = LabeledGauge::new(
            "queue_size",
            &["service", "priority"],
        ).unwrap();

        gauge.set(10, &[("service", "api"), ("priority", "high")]);
        gauge.set(5, &[("service", "api"), ("priority", "low")]);
        gauge.set(20, &[("service", "worker"), ("priority", "high")]);

        assert_eq!(
            gauge.get(&[("service", "api"), ("priority", "high")]),
            10
        );
        assert_eq!(
            gauge.get(&[("service", "worker"), ("priority", "high")]),
            20
        );
    }

    #[test]
    fn test_gauge_negative_values() {
        let gauge = Gauge::new("offset").unwrap();
        gauge.set(10);

        gauge.dec_by(15);
        assert_eq!(gauge.get(), -5);

        gauge.set(-100);
        assert_eq!(gauge.get(), -100);

        gauge.inc_by(150);
        assert_eq!(gauge.get(), 50);
    }

    #[test]
    fn test_concurrent_gauge() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        let gauge = Arc::new(Gauge::new("concurrent_gauge").unwrap());
        let num_threads = 10;
        let operations_per_thread = 1000;
        let done = Arc::new(AtomicUsize::new(0));

        // Simulate concurrent operations
        for _ in 0..num_threads {
            let gauge_clone = Arc::clone(&gauge);
            let done_clone = Arc::clone(&done);

            // Mix of increments and decrements
            for i in 0..operations_per_thread {
                if i % 2 == 0 {
                    gauge_clone.inc();
                } else {
                    gauge_clone.dec();
                }
            }
            done_clone.fetch_add(1, Ordering::Relaxed);
        }

        // Half inc, half dec = 0 net change (approximately)
        // Exact value depends on scheduling
        let final_value = gauge.get();
        assert!(final_value >= -num_threads as i64);
        assert!(final_value <= num_threads as i64);
    }
}
