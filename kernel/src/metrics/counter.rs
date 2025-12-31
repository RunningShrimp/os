//! Counter metrics for monotonically increasing values
//!
//! This module provides thread-safe counter implementations that can only increase.
//! Counters are ideal for tracking cumulative values like:
//! - Request counts
//! - Bytes transferred
//! - Error counts
//! - Operations processed
//!
//! # Example
//!
//! ```rust
//! use kernel::metrics::counter::{Counter, CounterF64};
//!
//! // Integer counter
//! let counter = Counter::new("requests_total");
//! counter.inc();
//! counter.inc_by(10);
//!
//! // Float counter
//! let float_counter = CounterF64::new("bytes_total");
//! float_counter.inc_by(1024.5);
//! ```

#![no_std]

extern crate alloc;

use alloc::fmt;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Display;
use core::sync::atomic::{AtomicU64, Ordering};

/// Metrics error type
#[derive(Debug, Clone, PartialEq)]
pub enum MetricsError {
    /// Invalid metric name
    InvalidName(String),
    /// Invalid label name
    InvalidLabel(String),
    /// Metric already registered
    AlreadyRegistered(String),
    /// Metric not found
    NotFound(String),
    /// Registry error
    RegistryError(String),
}

impl Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricsError::InvalidName(msg) => write!(f, "Invalid metric name: {}", msg),
            MetricsError::InvalidLabel(msg) => write!(f, "Invalid label: {}", msg),
            MetricsError::AlreadyRegistered(name) => write!(f, "Metric already registered: {}", name),
            MetricsError::NotFound(name) => write!(f, "Metric not found: {}", name),
            MetricsError::RegistryError(msg) => write!(f, "Registry error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MetricsError {}

/// Validate metric name according to Prometheus conventions
///
/// Metric names must match the regex `[a-zA-Z_:][a-zA-Z0-9_:]*`
pub fn validate_metric_name(name: &str) -> Result<(), MetricsError> {
    if name.is_empty() {
        return Err(MetricsError::InvalidName("name cannot be empty".into()));
    }

    let chars = name.chars().collect::<Vec<_>>();
    let first = chars[0];

    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' && first != ':' {
        return Err(MetricsError::InvalidName(format!(
            "must start with letter or underscore: {}",
            name
        )));
    }

    // Remaining characters must be alphanumeric, underscore, or colon
    for ch in &chars[1..] {
        if !ch.is_alphanumeric() && *ch != '_' && *ch != ':' {
            return Err(MetricsError::InvalidName(format!(
                "invalid character '{}' in name: {}",
                ch, name
            )));
        }
    }

    Ok(())
}

/// Validate label name according to Prometheus conventions
///
/// Label names must match the regex `[a-zA-Z_][a-zA-Z0-9_]*`
/// and cannot start with `__`
pub fn validate_label_name(name: &str) -> Result<(), MetricsError> {
    if name.is_empty() {
        return Err(MetricsError::InvalidLabel("label name cannot be empty".into()));
    }

    // Cannot start with __ (reserved)
    if name.starts_with("__") {
        return Err(MetricsError::InvalidLabel(format!(
            "label cannot start with __ (reserved): {}",
            name
        )));
    }

    let chars = name.chars().collect::<Vec<_>>();
    let first = chars[0];

    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return Err(MetricsError::InvalidLabel(format!(
            "must start with letter or underscore: {}",
            name
        )));
    }

    // Remaining characters must be alphanumeric or underscore
    for ch in &chars[1..] {
        if !ch.is_alphanumeric() && *ch != '_' {
            return Err(MetricsError::InvalidLabel(format!(
                "invalid character '{}' in label: {}",
                ch, name
            )));
        }
    }

    Ok(())
}

/// A monotonically increasing counter metric (u64)
///
/// Counters can only increase and are typically used for cumulative values
/// like total requests, total bytes, etc. They can be reset to zero.
///
/// # Thread Safety
///
/// This implementation uses lock-free atomic operations for thread safety.
#[derive(Debug)]
pub struct Counter {
    name: String,
    help: Option<String>,
    value: Arc<AtomicU64>,
}

impl Counter {
    /// Create a new counter with the given name
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name (must follow Prometheus naming conventions)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::counter::Counter;
    ///
    /// let counter = Counter::new("requests_total");
    /// ```
    pub fn new(name: impl Into<String>) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;
        Ok(Self {
            name,
            help: None,
            value: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Create a new counter with help text
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `help` - Help text describing the metric
    pub fn with_help(name: impl Into<String>, help: impl Into<String>) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;
        Ok(Self {
            name,
            help: Some(help.into()),
            value: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Increment the counter by 1
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::counter::Counter;
    ///
    /// let counter = Counter::new("requests_total").unwrap();
    /// counter.inc();
    /// assert_eq!(counter.get(), 1);
    /// ```
    #[inline]
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter by a specific amount
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to increment by (must be non-negative)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::counter::Counter;
    ///
    /// let counter = Counter::new("bytes_total").unwrap();
    /// counter.inc_by(1024);
    /// assert_eq!(counter.get(), 1024);
    /// ```
    #[inline]
    pub fn inc_by(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Get the current counter value
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::counter::Counter;
    ///
    /// let counter = Counter::new("requests_total").unwrap();
    /// counter.inc();
    /// assert_eq!(counter.get(), 1);
    /// ```
    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset the counter to zero
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::counter::Counter;
    ///
    /// let counter = Counter::new("requests_total").unwrap();
    /// counter.inc_by(100);
    /// counter.reset();
    /// assert_eq!(counter.get(), 0);
    /// ```
    #[inline]
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
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
    /// use kernel::metrics::counter::Counter;
    ///
    /// let counter = Counter::with_help(
    ///     "requests_total",
    ///     "Total number of requests"
    /// ).unwrap();
    /// counter.inc_by(42);
    /// let output = counter.export_prometheus();
    /// assert!(output.contains("# HELP requests_total"));
    /// assert!(output.contains("requests_total 42"));
    /// ```
    pub fn export_prometheus(&self) -> String {
        let mut result = String::new();

        // Add help text if available
        if let Some(help) = &self.help {
            result.push_str(&format!("# HELP {} {}\n", self.name, help));
        }

        // Add type
        result.push_str(&format!("# TYPE {} counter\n", self.name));

        // Add value
        result.push_str(&format!("{} {}\n", self.name, self.get()));

        result
    }

    /// Get a reference to the atomic value for cloning
    pub(crate) fn clone_atomic(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.value)
    }
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            help: self.help.clone(),
            value: Arc::clone(&self.value),
        }
    }
}

/// A monotonically increasing counter metric (f64)
///
/// Float counters are useful when tracking metrics that require fractional precision.
///
/// # Thread Safety
///
/// Uses atomic operations for thread-safe increments. Note that floating-point
/// operations are not perfectly associative, so concurrent updates may have
/// small rounding differences.
#[derive(Debug)]
pub struct CounterF64 {
    name: String,
    help: Option<String>,
    value: Arc<atomic_f64::AtomicF64>,
}

mod atomic_f64 {
    use super::*;

    /// Atomic f64 using bitwise operations
    #[derive(Debug)]
    pub struct AtomicF64 {
        bits: core::sync::atomic::AtomicU64,
    }

    impl AtomicF64 {
        pub fn new(value: f64) -> Self {
            Self {
                bits: core::sync::atomic::AtomicU64::new(value.to_bits()),
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
    }
}

impl CounterF64 {
    /// Create a new float counter
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

    /// Create a new float counter with help text
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
    ///
    /// # Arguments
    ///
    /// * `delta` - Amount to increment by (must be non-negative)
    #[inline]
    pub fn inc_by(&self, delta: f64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    /// Get the current value
    #[inline]
    pub fn get(&self) -> f64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset to zero
    #[inline]
    pub fn reset(&self) {
        self.value.store(0.0, Ordering::Relaxed);
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

        result.push_str(&format!("# TYPE {} counter\n", self.name));
        result.push_str(&format!("{} {}\n", self.name, self.get()));

        result
    }
}

impl Clone for CounterF64 {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            help: self.help.clone(),
            value: Arc::clone(&self.value),
        }
    }
}

/// A counter with label support
///
/// Labeled counters allow tracking the same metric across different dimensions.
/// For example, tracking requests by status code or method.
///
/// # Example
///
/// ```rust
/// use kernel::metrics::counter::LabeledCounter;
///
/// let counter = LabeledCounter::new(
///     "http_requests_total",
///     &["method", "status"]
/// ).unwrap();
///
/// // Increment with specific labels
/// counter.inc(&[("method", "GET"), ("status", "200")]);
/// counter.inc(&[("method", "POST"), ("status", "201")]);
/// ```
#[derive(Debug)]
pub struct LabeledCounter {
    name: String,
    help: Option<String>,
    label_names: Vec<String>,
    values: spin::Mutex<alloc::collections::BTreeMap<Vec<String>, Counter>>,
}

impl LabeledCounter {
    /// Create a new labeled counter
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `label_names` - Names of labels for this counter
    pub fn new(
        name: impl Into<String>,
        label_names: &[&str],
    ) -> Result<Self, MetricsError> {
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

    /// Increment counter with specific labels
    ///
    /// # Arguments
    ///
    /// * `labels` - Label values (must match label_names in order)
    pub fn inc(&self, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let counter = values.entry(key).or_insert_with(|| {
            Counter::new(self.name.clone()).unwrap()
        });
        counter.inc();
    }

    /// Increment by specific amount with labels
    pub fn inc_by(&self, delta: u64, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        let counter = values.entry(key).or_insert_with(|| {
            Counter::new(self.name.clone()).unwrap()
        });
        counter.inc_by(delta);
    }

    /// Get value for specific labels
    pub fn get(&self, labels: &[(&str, &str)]) -> u64 {
        let key = self.extract_label_values(labels);
        let values = self.values.lock();
        values
            .get(&key)
            .map(|c| c.get())
            .unwrap_or(0)
    }

    /// Reset counter with specific labels
    pub fn reset(&self, labels: &[(&str, &str)]) {
        let key = self.extract_label_values(labels);
        let mut values = self.values.lock();
        if let Some(counter) = values.get_mut(&key) {
            counter.reset();
        }
    }

    /// Reset all label combinations
    pub fn reset_all(&self) {
        let mut values = self.values.lock();
        values.clear();
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

        result.push_str(&format!("# TYPE {} counter\n", self.name));

        let values = self.values.lock();
        for (labels, counter) in values.iter() {
            let label_str = self.format_labels(labels);
            result.push_str(&format!("{}{} {}\n", self.name, label_str, counter.get()));
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
    fn test_counter_basic() {
        let counter = Counter::new("test_counter").unwrap();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.inc_by(10);
        assert_eq!(counter.get(), 11);

        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_with_help() {
        let counter = Counter::with_help("test_counter", "A test counter").unwrap();
        assert_eq!(counter.help(), Some("A test counter"));
        counter.inc();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_counter_clone() {
        let counter1 = Counter::new("test_counter").unwrap();
        counter1.inc_by(5);

        let counter2 = counter1.clone();
        counter2.inc();

        assert_eq!(counter1.get(), 6);
        assert_eq!(counter2.get(), 6);
    }

    #[test]
    fn test_counter_export() {
        let counter = Counter::with_help("requests_total", "Total requests").unwrap();
        counter.inc_by(42);

        let output = counter.export_prometheus();
        assert!(output.contains("# HELP requests_total Total requests"));
        assert!(output.contains("# TYPE requests_total counter"));
        assert!(output.contains("requests_total 42"));
    }

    #[test]
    fn test_float_counter_basic() {
        let counter = CounterF64::new("test_counter").unwrap();
        assert_eq!(counter.get(), 0.0);

        counter.inc();
        assert_eq!(counter.get(), 1.0);

        counter.inc_by(10.5);
        assert_eq!(counter.get(), 11.5);

        counter.reset();
        assert_eq!(counter.get(), 0.0);
    }

    #[test]
    fn test_validate_metric_name() {
        assert!(validate_metric_name("valid_name").is_ok());
        assert!(validate_metric_name("Valid_Name123").is_ok());
        assert!(validate_metric_name("valid:name").is_ok());
        assert!(validate_metric_name("_valid").is_ok());

        assert!(validate_metric_name("").is_err());
        assert!(validate_metric_name("123invalid").is_err());
        assert!(validate_metric_name("invalid-name").is_err());
        assert!(validate_metric_name("invalid.name").is_err());
    }

    #[test]
    fn test_validate_label_name() {
        assert!(validate_label_name("valid_label").is_ok());
        assert!(validate_label_name("_valid_label").is_ok());
        assert!(validate_label_name("valid_label123").is_ok());

        assert!(validate_label_name("").is_err());
        assert!(validate_label_name("__reserved").is_err());
        assert!(validate_label_name("invalid-label").is_err());
        assert!(validate_label_name("invalid.label").is_err());
    }

    #[test]
    fn test_labeled_counter() {
        let counter = LabeledCounter::new(
            "http_requests",
            &["method", "status"],
        ).unwrap();

        counter.inc(&[("method", "GET"), ("status", "200")]);
        counter.inc_by(5, &[("method", "POST"), ("status", "201")]);

        assert_eq!(
            counter.get(&[("method", "GET"), ("status", "200")]),
            1
        );
        assert_eq!(
            counter.get(&[("method", "POST"), ("status", "201")]),
            5
        );
    }

    #[test]
    fn test_labeled_counter_export() {
        let counter = LabeledCounter::with_help(
            "http_requests_total",
            &["method", "status"],
            "Total HTTP requests"
        ).unwrap();

        counter.inc(&[("method", "GET"), ("status", "200")]);
        counter.inc_by(2, &[("method", "POST"), ("status", "201")]);

        let output = counter.export_prometheus();
        assert!(output.contains("# HELP http_requests_total Total HTTP requests"));
        assert!(output.contains("# TYPE http_requests_total counter"));
        assert!(output.contains("method=\"GET\",status=\"200\""));
        assert!(output.contains("method=\"POST\",status=\"201\""));
    }

    #[test]
    fn test_concurrent_counter() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(Counter::new("concurrent").unwrap());
        let num_threads = 10;
        let increments_per_thread = 1000;
        let done = Arc::new(AtomicUsize::new(0));

        // Simulate concurrent increments
        for _ in 0..num_threads {
            let counter_clone = Arc::clone(&counter);
            let done_clone = Arc::clone(&done);

            // In real kernel, we'd spawn threads here
            // For this test, just increment directly
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
}
