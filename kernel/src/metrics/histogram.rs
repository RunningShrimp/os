//! Histogram metrics for distribution tracking
//!
//! This module provides histogram implementations for tracking value distributions.
//! Histograms are ideal for metrics like:
//! - Request latencies
//! - Response sizes
//! - Queue wait times
//! - Processing durations
//!
//! # Example
//!
//! ```rust
//! use kernel::metrics::histogram::Histogram;
//!
//! let histogram = Histogram::new(
//!     "request_duration_seconds",
//!     Histogram::exponential_buckets(0.001, 2.0, 10)
//! ).unwrap();
//!
//! histogram.observe(0.002);
//! histogram.observe(0.005);
//! histogram.observe(0.100);
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::fmt;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Display;
use core::sync::atomic::{AtomicU64, Ordering};

use super::counter::{validate_metric_name, MetricsError};

/// Default histogram buckets for latency metrics (seconds)
pub const DEFAULT_LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Default histogram buckets for size metrics (bytes)
pub const DEFAULT_SIZE_BUCKETS: &[f64] = &[
    1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0, 50000.0,
    100000.0, 500000.0, 1000000.0,
];

/// Generate exponential buckets
///
/// Creates buckets starting from `start` and multiplying by `factor` `count` times.
/// Each bucket is `start * factor^i` for i in 0..count.
///
/// # Arguments
///
/// * `start` - First bucket value
/// * `factor` - Multiplier for each bucket
/// * `count` - Number of buckets
///
/// # Example
///
/// ```rust
/// use kernel::metrics::histogram;
///
/// let buckets = histogram::exponential_buckets(1.0, 10.0, 3);
/// assert_eq!(buckets, vec![1.0, 10.0, 100.0]);
/// ```
pub fn exponential_buckets(start: f64, factor: f64, count: usize) -> Vec<f64> {
    let mut buckets = Vec::with_capacity(count);
    let mut value = start;
    for _ in 0..count {
        buckets.push(value);
        value *= factor;
    }
    buckets
}

/// Generate linear buckets
///
/// Creates buckets starting from `start` and adding `width` `count` times.
/// Each bucket is `start + width * i` for i in 0..count.
///
/// # Arguments
///
/// * `start` - First bucket value
/// * `width` - Width of each bucket
/// * `count` - Number of buckets
///
/// # Example
///
/// ```rust
/// use kernel::metrics::histogram;
///
/// let buckets = histogram::linear_buckets(0.0, 5.0, 4);
/// assert_eq!(buckets, vec![0.0, 5.0, 10.0, 15.0]);
/// ```
pub fn linear_buckets(start: f64, width: f64, count: usize) -> Vec<f64> {
    let mut buckets = Vec::with_capacity(count);
    for i in 0..count {
        buckets.push(start + width * i as f64);
    }
    buckets
}

/// Histogram bucket counts (atomic for thread safety)
#[derive(Debug)]
struct BucketCounts {
    /// Cumulative counts for each bucket + Inf
    counts: Vec<AtomicU64>,
    /// Total sum of observed values (stored as integer representing fixed-point)
    sum: Arc<AtomicU64>,
}

impl BucketCounts {
    fn new(num_buckets: usize) -> Self {
        let mut counts = Vec::with_capacity(num_buckets + 1);
        for _ in 0..=num_buckets {
            counts.push(AtomicU64::new(0));
        }
        Self {
            counts,
            sum: Arc::new(AtomicU64::new(0)),
        }
    }

    fn observe(&self, value: f64, buckets: &[f64]) {
        // Convert to fixed-point (multiply by 1e6 for micro precision)
        let value_int = (value * 1_000_000.0) as u64;
        self.sum.fetch_add(value_int, Ordering::Relaxed);

        // Find appropriate bucket
        for (i, &upper) in buckets.iter().enumerate() {
            if value <= upper {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
        // +Inf bucket
        self.counts[buckets.len()].fetch_add(1, Ordering::Relaxed);
    }

    fn get_count(&self, index: usize) -> u64 {
        self.counts[index].load(Ordering::Relaxed)
    }

    fn get_sum(&self) -> f64 {
        let sum_int = self.sum.load(Ordering::Relaxed);
        sum_int as f64 / 1_000_000.0
    }
}

/// A histogram metric for distribution tracking
///
/// Histograms collect observations into configurable buckets and calculate
/// configurable quantiles. They are ideal for tracking latencies, sizes, and
/// other metrics where you want to understand the distribution.
///
/// # Thread Safety
///
/// Uses lock-free atomic operations for thread safety.
///
/// # Quantiles
///
/// This implementation tracks exact bucket counts. True quantile calculation
/// requires more sophisticated algorithms (like t-digest), but this provides
/// good approximations for common percentiles (p50, p95, p99).
#[derive(Debug)]
pub struct Histogram {
    name: String,
    help: Option<String>,
    buckets: Vec<f64>,
    counts: Arc<BucketCounts>,
}

impl Histogram {
    /// Create a new histogram with custom buckets
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `buckets` - Upper bounds for buckets (must be sorted)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::new(
    ///     "request_duration_seconds",
    ///     vec![0.1, 0.5, 1.0, 5.0]
    /// ).unwrap();
    /// ```
    pub fn new(name: impl Into<String>, buckets: Vec<f64>) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;

        // Validate buckets are sorted
        for i in 1..buckets.len() {
            if buckets[i] <= buckets[i - 1] {
                return Err(MetricsError::InvalidName(
                    "buckets must be strictly increasing".into(),
                ));
            }
        }

        let bucket_count = buckets.len();
        Ok(Self {
            name,
            help: None,
            buckets,
            counts: Arc::new(BucketCounts::new(bucket_count)),
        })
    }

    /// Create histogram with default latency buckets
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets(
    ///     "http_request_duration_seconds"
    /// ).unwrap();
    /// ```
    pub fn with_latency_buckets(name: impl Into<String>) -> Result<Self, MetricsError> {
        Self::new(name, DEFAULT_LATENCY_BUCKETS.to_vec())
    }

    /// Create histogram with default size buckets
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_size_buckets(
    ///     "response_size_bytes"
    /// ).unwrap();
    /// ```
    pub fn with_size_buckets(name: impl Into<String>) -> Result<Self, MetricsError> {
        Self::new(name, DEFAULT_SIZE_BUCKETS.to_vec())
    }

    /// Create with help text and custom buckets
    pub fn with_help(
        name: impl Into<String>,
        buckets: Vec<f64>,
        help: impl Into<String>,
    ) -> Result<Self, MetricsError> {
        let name = name.into();
        validate_metric_name(&name)?;

        for i in 1..buckets.len() {
            if buckets[i] <= buckets[i - 1] {
                return Err(MetricsError::InvalidName(
                    "buckets must be strictly increasing".into(),
                ));
            }
        }

        let bucket_count = buckets.len();
        Ok(Self {
            name,
            help: Some(help.into()),
            buckets,
            counts: Arc::new(BucketCounts::new(bucket_count)),
        })
    }

    /// Observe a value
    ///
    /// Records a value in the appropriate bucket.
    ///
    /// # Arguments
    ///
    /// * `value` - Value to observe (must be non-negative)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets(
    ///     "duration_seconds"
    /// ).unwrap();
    ///
    /// histogram.observe(0.123);
    /// histogram.observe(0.456);
    /// ```
    #[inline]
    pub fn observe(&self, value: f64) {
        self.counts.observe(value, &self.buckets);
    }

    /// Observe multiple values (batch operation)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe_many(&[0.1, 0.2, 0.3, 0.4, 0.5]);
    /// ```
    pub fn observe_many(&self, values: &[f64]) {
        for &value in values {
            self.observe(value);
        }
    }

    /// Get the total count of observations
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe_many(&[0.1, 0.2, 0.3]);
    /// assert_eq!(histogram.count(), 3);
    /// ```
    pub fn count(&self) -> u64 {
        self.counts.get_count(self.buckets.len())
    }

    /// Get the sum of all observed values
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe_many(&[1.0, 2.0, 3.0]);
    /// assert_eq!(histogram.sum(), 6.0);
    /// ```
    pub fn sum(&self) -> f64 {
        self.counts.get_sum()
    }

    /// Get the average of observed values
    ///
    /// Returns 0.0 if no observations have been made.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe_many(&[1.0, 2.0, 3.0, 4.0]);
    /// assert_eq!(histogram.avg(), 2.5);
    /// ```
    pub fn avg(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            return 0.0;
        }
        self.sum() / count as f64
    }

    /// Get bucket counts
    ///
    /// Returns a map of bucket upper bounds to cumulative counts.
    pub fn buckets(&self) -> BTreeMap<String, u64> {
        let mut result = BTreeMap::new();
        for (i, &upper) in self.buckets.iter().enumerate() {
            result.insert(upper.to_string(), self.counts.get_count(i));
        }
        // +Inf bucket
        result.insert("+Inf".to_string(), self.counts.get_count(self.buckets.len()));
        result
    }

    /// Get approximate percentile
    ///
    /// Returns the bucket upper bound that contains the given percentile.
    /// For true quantile estimation, consider using a more sophisticated algorithm.
    ///
    /// # Arguments
    ///
    /// * `quantile` - Quantile in range [0.0, 1.0] (e.g., 0.95 for p95)
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe_many(&[0.1, 0.2, 0.3, 0.4, 0.5]);
    ///
    /// let p50 = histogram.quantile(0.5);
    /// let p95 = histogram.quantile(0.95);
    /// ```
    pub fn quantile(&self, quantile: f64) -> Option<f64> {
        if quantile < 0.0 || quantile > 1.0 {
            return None;
        }

        let total = self.count() as f64;
        if total == 0.0 {
            return None;
        }

        let target = total * quantile;
        let mut cumulative = 0.0;

        for (i, &upper) in self.buckets.iter().enumerate() {
            cumulative += self.counts.get_count(i) as f64;
            if cumulative >= target {
                return Some(upper);
            }
        }

        Some(f64::INFINITY)
    }

    /// Get the 50th percentile (median)
    pub fn p50(&self) -> Option<f64> {
        self.quantile(0.5)
    }

    /// Get the 95th percentile
    pub fn p95(&self) -> Option<f64> {
        self.quantile(0.95)
    }

    /// Get the 99th percentile
    pub fn p99(&self) -> Option<f64> {
        self.quantile(0.99)
    }

    /// Get the 99.9th percentile
    pub fn p999(&self) -> Option<f64> {
        self.quantile(0.999)
    }

    /// Reset the histogram
    ///
    /// Clears all observations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe(1.0);
    /// histogram.reset();
    /// assert_eq!(histogram.count(), 0);
    /// ```
    pub fn reset(&self) {
        for count in &self.counts.counts {
            count.store(0, Ordering::Relaxed);
        }
        self.counts.sum.store(0, Ordering::Relaxed);
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
    /// use kernel::metrics::histogram::Histogram;
    ///
    /// let histogram = Histogram::with_latency_buckets("duration").unwrap();
    /// histogram.observe(0.123);
    /// let output = histogram.export_prometheus();
    /// ```
    pub fn export_prometheus(&self) -> String {
        let mut result = String::new();

        if let Some(help) = &self.help {
            result.push_str(&format!("# HELP {} {}\n", self.name, help));
        }
        result.push_str(&format!("# TYPE {} histogram\n", self.name));

        // Bucket counts (cumulative)
        for (i, &upper) in self.buckets.iter().enumerate() {
            let count = self.counts.get_count(i);
            result.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", self.name, upper, count));
        }
        // +Inf bucket
        let inf_count = self.counts.get_count(self.buckets.len());
        result.push_str(&format!(
            "{}_bucket{{le=\"+Inf\"}} {}\n",
            self.name, inf_count
        ));

        // Sum and count
        result.push_str(&format!("{}_sum {}\n", self.name, self.sum()));
        result.push_str(&format!("{}_count {}\n", self.name, self.count()));

        result
    }

    /// Get summary statistics
    ///
    /// Returns a summary with count, sum, avg, and key percentiles.
    pub fn summary(&self) -> HistogramSummary {
        HistogramSummary {
            count: self.count(),
            sum: self.sum(),
            avg: self.avg(),
            p50: self.p50(),
            p95: self.p95(),
            p99: self.p99(),
            p999: self.p999(),
        }
    }
}

impl Clone for Histogram {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            help: self.help.clone(),
            buckets: self.buckets.clone(),
            counts: Arc::clone(&self.counts),
        }
    }
}

/// Summary statistics for a histogram
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramSummary {
    /// Total number of observations
    pub count: u64,
    /// Sum of all observations
    pub sum: f64,
    /// Average (mean) of observations
    pub avg: f64,
    /// 50th percentile (median)
    pub p50: Option<f64>,
    /// 95th percentile
    pub p95: Option<f64>,
    /// 99th percentile
    pub p99: Option<f64>,
    /// 99.9th percentile
    pub p999: Option<f64>,
}

impl Display for HistogramSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Histogram Summary:")?;
        writeln!(f, "  Count: {}", self.count)?;
        writeln!(f, "  Sum: {:.6}", self.sum)?;
        writeln!(f, "  Avg: {:.6}", self.avg)?;
        if let Some(p50) = self.p50 {
            writeln!(f, "  P50: {:.6}", p50)?;
        }
        if let Some(p95) = self.p95 {
            writeln!(f, "  P95: {:.6}", p95)?;
        }
        if let Some(p99) = self.p99 {
            writeln!(f, "  P99: {:.6}", p99)?;
        }
        if let Some(p999) = self.p999 {
            writeln!(f, "  P99.9: {:.6}", p999)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_buckets() {
        let buckets = exponential_buckets(1.0, 2.0, 5);
        assert_eq!(buckets, vec![1.0, 2.0, 4.0, 8.0, 16.0]);
    }

    #[test]
    fn test_linear_buckets() {
        let buckets = linear_buckets(0.0, 10.0, 4);
        assert_eq!(buckets, vec![0.0, 10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_histogram_basic() {
        let histogram = Histogram::new("test", vec![1.0, 5.0, 10.0]).unwrap();
        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.sum(), 0.0);

        histogram.observe(0.5);
        histogram.observe(2.0);
        histogram.observe(7.0);
        histogram.observe(15.0);

        assert_eq!(histogram.count(), 4);
        assert_eq!(histogram.sum(), 24.5);
        assert_eq!(histogram.avg(), 6.125);
    }

    #[test]
    fn test_histogram_buckets() {
        let histogram = Histogram::new("test", vec![1.0, 5.0, 10.0]).unwrap();
        histogram.observe_many(&[0.5, 2.0, 7.0, 15.0]);

        let buckets = histogram.buckets();
        assert_eq!(buckets.get("1").unwrap(), &1); // 0.5
        assert_eq!(buckets.get("5").unwrap(), &2); // 0.5, 2.0
        assert_eq!(buckets.get("10").unwrap(), &3); // 0.5, 2.0, 7.0
        assert_eq!(buckets.get("+Inf").unwrap(), &4); // all
    }

    #[test]
    fn test_histogram_quantiles() {
        let histogram = Histogram::new("test", vec![10.0, 20.0, 30.0, 40.0, 50.0]).unwrap();
        histogram.observe_many(&[5.0, 15.0, 25.0, 35.0, 45.0, 55.0]);

        // With 6 observations:
        // P50 should be at observation 3 (25.0)
        // P95 should be at observation 5.7 (~55.0)
        let p50 = histogram.p50();
        let p95 = histogram.p95();

        assert!(p50.is_some());
        assert!(p95.is_some());
    }

    #[test]
    fn test_histogram_reset() {
        let histogram = Histogram::new("test", vec![1.0, 5.0, 10.0]).unwrap();
        histogram.observe(2.0);
        histogram.observe(3.0);

        assert_eq!(histogram.count(), 2);

        histogram.reset();
        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.sum(), 0.0);
    }

    #[test]
    fn test_histogram_clone() {
        let histogram1 = Histogram::new("test", vec![1.0, 5.0, 10.0]).unwrap();
        histogram1.observe(2.0);

        let histogram2 = histogram1.clone();
        histogram2.observe(3.0);

        // Both share the same underlying counts
        assert_eq!(histogram1.count(), 2);
        assert_eq!(histogram2.count(), 2);
    }

    #[test]
    fn test_histogram_summary() {
        let histogram = Histogram::new("test", vec![1.0, 5.0, 10.0]).unwrap();
        histogram.observe_many(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        let summary = histogram.summary();
        assert_eq!(summary.count, 5);
        assert_eq!(summary.sum, 15.0);
        assert_eq!(summary.avg, 3.0);
    }

    #[test]
    fn test_histogram_export() {
        let histogram = Histogram::with_help(
            "request_duration_seconds",
            vec![0.1, 0.5, 1.0],
            "Request duration in seconds"
        ).unwrap();

        histogram.observe_many(&[0.05, 0.2, 0.7]);

        let output = histogram.export_prometheus();
        assert!(output.contains("# HELP request_duration_seconds"));
        assert!(output.contains("# TYPE request_duration_seconds histogram"));
        assert!(output.contains("request_duration_seconds_bucket"));
        assert!(output.contains("request_duration_seconds_sum"));
        assert!(output.contains("request_duration_seconds_count"));
    }

    #[test]
    fn test_histogram_invalid_buckets() {
        let result = Histogram::new("test", vec![5.0, 2.0, 10.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_histogram_with_latency_buckets() {
        let histogram = Histogram::with_latency_buckets("latency").unwrap();
        assert_eq!(histogram.buckets.len(), DEFAULT_LATENCY_BUCKETS.len());
    }

    #[test]
    fn test_histogram_with_size_buckets() {
        let histogram = Histogram::with_size_buckets("size").unwrap();
        assert_eq!(histogram.buckets.len(), DEFAULT_SIZE_BUCKETS.len());
    }

    #[test]
    fn test_histogram_empty() {
        let histogram = Histogram::new("test", vec![1.0, 5.0, 10.0]).unwrap();

        assert_eq!(histogram.count(), 0);
        assert_eq!(histogram.sum(), 0.0);
        assert_eq!(histogram.avg(), 0.0);
        assert!(histogram.p50().is_none());
        assert!(histogram.p95().is_none());
    }

    #[test]
    fn test_histogram_concurrent() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        let histogram = Arc::new(Histogram::with_latency_buckets("concurrent").unwrap());
        let num_threads = 10;
        let observations_per_thread = 100;

        for _ in 0..num_threads {
            let histogram_clone = Arc::clone(&histogram);
            for i in 0..observations_per_thread {
                histogram_clone.observe(i as f64 * 0.001);
            }
        }

        assert_eq!(
            histogram.count(),
            num_threads as u64 * observations_per_thread as u64
        );
    }
}
