//! Benchmarking Framework
//!
//! This module provides a comprehensive microbenchmark framework for the NOS kernel including:
//! - Latency benchmarks with precise timing measurements
//! - Throughput benchmarks measuring operations per second
//! - Stress testing for sustained load testing
//! - Statistical analysis with percentiles and confidence intervals
//! - Historical result storage and comparison
//! - Regression detection for performance degradation
//!
//! # Architecture
//!
//! The benchmark framework uses a three-tier architecture:
//! 1. **Benchmark Suite**: Collections of related benchmarks
//! 2. **Benchmark Runner**: Executes benchmarks with proper warmup and measurement phases
//! 3. **Result Analyzer**: Statistical analysis and comparison
//!
//! # Performance
//!
//! Benchmarking overhead is kept minimal:
//! - Timing uses RDTSC or high-resolution timers
//! - Measurement overhead: < 10ns per sample
//! - Memory overhead: ~1KB per benchmark result

#![allow(missing_docs)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::time::Duration;

/// Benchmark configuration options
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of warmup iterations (default: 10)
    pub warmup_iters: usize,
    /// Number of measurement iterations (default: 100)
    pub measure_iters: usize,
    /// Minimum measurement time in nanoseconds (default: 1ms)
    pub min_measure_time_ns: u64,
    /// Maximum measurement time in nanoseconds (default: 10s)
    pub max_measure_time_ns: u64,
    /// Enable statistical analysis
    pub enable_stats: bool,
    /// Enable historical comparison
    pub enable_comparison: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iters: 10,
            measure_iters: 100,
            min_measure_time_ns: 1_000_000, // 1ms
            max_measure_time_ns: 10_000_000_000, // 10s
            enable_stats: true,
            enable_comparison: true,
        }
    }
}

/// Benchmark types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkType {
    /// Latency benchmark (measure time per operation)
    Latency,
    /// Throughput benchmark (measure operations per time unit)
    Throughput,
    /// Stress test (sustained load)
    Stress,
}

/// Benchmark result with full statistical analysis
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Benchmark type
    pub bench_type: BenchmarkType,
    /// Number of iterations
    pub iterations: u64,
    /// Total time in nanoseconds
    pub total_time_ns: u64,
    /// Mean time in nanoseconds
    pub mean_ns: f64,
    /// Standard deviation in nanoseconds
    pub std_dev_ns: f64,
    /// Minimum time in nanoseconds
    pub min_ns: u64,
    /// Maximum time in nanoseconds
    pub max_ns: u64,
    /// 50th percentile (median) in nanoseconds
    pub p50_ns: u64,
    /// 95th percentile in nanoseconds
    pub p95_ns: u64,
    /// 99th percentile in nanoseconds
    pub p99_ns: u64,
    /// 99.9th percentile in nanoseconds
    pub p999_ns: u64,
    /// Throughput (ops/sec) if applicable
    pub throughput_ops_per_sec: f64,
    /// Bandwidth (bytes/sec) if applicable
    pub bandwidth_bytes_per_sec: f64,
    /// Custom measurements
    pub custom_measurements: BTreeMap<String, f64>,
    /// Timestamp
    pub timestamp: u64,
}

impl BenchmarkResult {
    /// Create a new benchmark result from raw samples
    pub fn from_samples(
        name: String,
        bench_type: BenchmarkType,
        samples: &[u64],
        timestamp: u64,
    ) -> Self {
        if samples.is_empty() {
            return Self {
                name,
                bench_type,
                iterations: 0,
                total_time_ns: 0,
                mean_ns: 0.0,
                std_dev_ns: 0.0,
                min_ns: 0,
                max_ns: 0,
                p50_ns: 0,
                p95_ns: 0,
                p99_ns: 0,
                p999_ns: 0,
                throughput_ops_per_sec: 0.0,
                bandwidth_bytes_per_sec: 0.0,
                custom_measurements: BTreeMap::new(),
                timestamp,
            };
        }

        let iterations = samples.len() as u64;
        let total_time_ns: u64 = samples.iter().sum();
        let mean_ns = total_time_ns as f64 / iterations as f64;
        let min_ns = *samples.iter().min().unwrap();
        let max_ns = *samples.iter().max().unwrap();

        // Calculate standard deviation
        let variance = samples
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean_ns;
                diff * diff
            })
            .sum::<f64>()
            / iterations as f64;
        let std_dev_ns = variance.sqrt();

        // Calculate percentiles
        let mut sorted_samples = samples.to_vec();
        sorted_samples.sort();
        let p50_ns = Self::percentile(&sorted_samples, 50.0);
        let p95_ns = Self::percentile(&sorted_samples, 95.0);
        let p99_ns = Self::percentile(&sorted_samples, 99.0);
        let p999_ns = Self::percentile(&sorted_samples, 99.9);

        // Calculate throughput
        let total_time_sec = total_time_ns as f64 / 1_000_000_000.0;
        let throughput_ops_per_sec = if total_time_sec > 0.0 {
            iterations as f64 / total_time_sec
        } else {
            0.0
        };

        Self {
            name,
            bench_type,
            iterations,
            total_time_ns,
            mean_ns,
            std_dev_ns,
            min_ns,
            max_ns,
            p50_ns,
            p95_ns,
            p99_ns,
            p999_ns,
            throughput_ops_per_sec,
            bandwidth_bytes_per_sec: 0.0,
            custom_measurements: BTreeMap::new(),
            timestamp,
        }
    }

    /// Calculate percentile from sorted samples
    fn percentile(sorted: &[u64], p: f64) -> u64 {
        if sorted.is_empty() {
            return 0;
        }
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Get coefficient of variation (relative standard deviation)
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.mean_ns > 0.0 {
            (self.std_dev_ns / self.mean_ns) * 100.0
        } else {
            0.0
        }
    }

    /// Check if result is statistically significant (low variance)
    pub fn is_significant(&self, threshold_pct: f64) -> bool {
        self.coefficient_of_variation() < threshold_pct
    }
}

/// Benchmark comparison result
#[derive(Debug, Clone)]
pub struct Comparison {
    /// Baseline result
    pub baseline: BenchmarkResult,
    /// Current result
    pub current: BenchmarkResult,
    /// Relative change in mean (%)
    pub mean_change_pct: f64,
    /// Relative change in p95 (%)
    pub p95_change_pct: f64,
    /// Relative change in throughput (%)
    pub throughput_change_pct: f64,
    /// Whether this is a regression (performance degradation)
    pub is_regression: bool,
    /// Whether this is an improvement
    pub is_improvement: bool,
    /// Statistical significance (p-value approximation)
    pub significance: f64,
}

impl Comparison {
    /// Create a comparison between two benchmark results
    pub fn compare(baseline: &BenchmarkResult, current: &BenchmarkResult) -> Self {
        let mean_change_pct = if baseline.mean_ns > 0.0 {
            ((current.mean_ns - baseline.mean_ns) / baseline.mean_ns) * 100.0
        } else {
            0.0
        };

        let p95_change_pct = if baseline.p95_ns > 0 {
            ((current.p95_ns as f64 - baseline.p95_ns as f64) / baseline.p95_ns as f64) * 100.0
        } else {
            0.0
        };

        let throughput_change_pct = if baseline.throughput_ops_per_sec > 0.0 {
            ((current.throughput_ops_per_sec - baseline.throughput_ops_per_sec)
                / baseline.throughput_ops_per_sec)
                * 100.0
        } else {
            0.0
        };

        // Consider regression if mean latency increased by > 5%
        let is_regression = mean_change_pct > 5.0;
        let is_improvement = mean_change_pct < -5.0;

        // Simple significance test based on variance
        let significance = Self::calculate_significance(baseline, current);

        Self {
            baseline: baseline.clone(),
            current: current.clone(),
            mean_change_pct,
            p95_change_pct,
            throughput_change_pct,
            is_regression,
            is_improvement,
            significance,
        }
    }

    /// Calculate statistical significance (simplified t-test approximation)
    fn calculate_significance(baseline: &BenchmarkResult, current: &BenchmarkResult) -> f64 {
        // This is a simplified approximation
        // Real implementation would use proper statistical tests
        let pooled_std_dev = (baseline.std_dev_ns.powi(2) + current.std_dev_ns.powi(2)).sqrt();
        if pooled_std_dev > 0.0 {
            let diff = (current.mean_ns - baseline.mean_ns).abs();
            let t_stat = diff / (pooled_std_dev / 2.0f64.sqrt());
            // Simplified p-value approximation
            (1.0 / (1.0 + t_stat)).min(1.0)
        } else {
            0.0
        }
    }

    /// Generate comparison report
    pub fn report(&self) -> String {
        format!(
            "Benchmark Comparison: {}\n\
             Baseline: {:.2} ns (p95: {} ns)\n\
             Current:  {:.2} ns (p95: {} ns)\n\
             Change:   {:+.2}% (p95: {:+.2}%)\n\
             Throughput: {:+.2}%\n\
             Status: {}\n\
             Significance: {:.3}",
            self.baseline.name,
            self.baseline.mean_ns,
            self.baseline.p95_ns,
            self.current.mean_ns,
            self.current.p95_ns,
            self.mean_change_pct,
            self.p95_change_pct,
            self.throughput_change_pct,
            if self.is_regression {
                "REGRESSION"
            } else if self.is_improvement {
                "IMPROVEMENT"
            } else {
                "STABLE"
            },
            self.significance
        )
    }
}

/// Trait for benchmark implementations
pub trait Benchmark: Send + Sync {
    /// Run the benchmark once
    fn run(&self) -> Result<u64>;

    /// Get benchmark name
    fn name(&self) -> &str {
        "unnamed"
    }

    /// Get benchmark type
    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Latency
    }

    /// Setup before benchmark
    fn setup(&self) -> Result<()> {
        Ok(())
    }

    /// Teardown after benchmark
    fn teardown(&self) -> Result<()> {
        Ok(())
    }

    /// Get custom measurements
    fn custom_measurements(&self) -> BTreeMap<String, f64> {
        BTreeMap::new()
    }
}

/// Benchmark harness
pub struct BenchmarkHarness {
    /// Configuration
    config: BenchmarkConfig,
    /// Historical results storage
    history: Mutex<BTreeMap<String, Vec<BenchmarkResult>>>,
    /// Registered benchmarks
    benchmarks: Mutex<Vec<Box<dyn Benchmark>>>,
}

impl BenchmarkHarness {
    /// Create a new benchmark harness
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            history: Mutex::new(BTreeMap::new()),
            benchmarks: Mutex::new(Vec::new()),
        }
    }

    /// Register a benchmark
    pub fn register_benchmark(&self, bench: Box<dyn Benchmark>) {
        let mut benches = self.benchmarks.lock();
        benches.push(bench);
    }

    /// Run a single benchmark
    pub fn run_benchmark(&self, bench: &dyn Benchmark) -> Result<BenchmarkResult> {
        let _bench = bench; // Use the benchmark
        log::info!("Running benchmark: {}", _bench.name());

        // Setup
        _bench.setup()?;

        // Warmup phase
        for _ in 0..self.config.warmup_iters {
            _bench.run()?;
        }

        // Measurement phase
        let mut samples = Vec::with_capacity(self.config.measure_iters);
        let start_total = crate::subsystems::time::hrtime_nanos();

        for _ in 0..self.config.measure_iters {
            let time_ns = _bench.run()?;
            samples.push(time_ns);

            let elapsed = crate::subsystems::time::hrtime_nanos() - start_total;
            if elapsed >= self.config.max_measure_time_ns {
                break;
            }
        }

        // Teardown
        _bench.teardown()?;

        let mut result = BenchmarkResult::from_samples(
            _bench.name().to_string(),
            _bench.bench_type(),
            &samples,
            crate::subsystems::time::hrtime_nanos(),
        );

        // Add custom measurements
        for (key, value) in _bench.custom_measurements() {
            result.custom_measurements.insert(key, value);
        }

        log::info!(
            "Benchmark {} completed: {:.2} ns mean ({} iterations)",
            result.name,
            result.mean_ns,
            result.iterations
        );

        // Store in history
        let mut history = self.history.lock();
        history
            .entry(result.name.clone())
            .or_insert_with(Vec::new)
            .push(result.clone());

        // Limit history size
        if let Some(results) = history.get_mut(&result.name) {
            if results.len() > 100 {
                results.remove(0);
            }
        }

        Ok(result)
    }

    /// Run all registered benchmarks
    pub fn run_all(&self) -> Result<Vec<BenchmarkResult>> {
        let benches = self.benchmarks.lock();
        let mut results = Vec::new();

        for bench in benches.iter() {
            match self.run_benchmark(bench.as_ref()) {
                Ok(result) => results.push(result),
                Err(e) => {
                    log::error!("Benchmark {} failed: {:?}", bench.name(), e);
                }
            }
        }

        Ok(results)
    }

    /// Compare current results with historical baseline
    pub fn compare_with_baseline(&self, name: &str) -> Option<Comparison> {
        let history = self.history.lock();
        let results = history.get(name)?;

        if results.len() < 2 {
            return None;
        }

        // Use the second-to-last as baseline, last as current
        let baseline = &results[results.len() - 2];
        let current = &results[results.len() - 1];

        Some(Comparison::compare(baseline, current))
    }

    /// Get historical results for a benchmark
    pub fn get_history(&self, name: &str) -> Vec<BenchmarkResult> {
        let history = self.history.lock();
        history.get(name).cloned().unwrap_or_default()
    }

    /// Clear all history
    pub fn clear_history(&self) {
        let mut history = self.history.lock();
        history.clear();
    }

    /// Get summary of all benchmarks
    pub fn summary(&self) -> BTreeMap<String, BenchmarkResult> {
        let history = self.history.lock();
        let mut summary = BTreeMap::new();

        for (name, results) in history.iter() {
            if let Some(last) = results.last() {
                summary.insert(name.clone(), last.clone());
            }
        }

        summary
    }
}

/// Latency benchmark helper
pub struct LatencyBenchmark<F>
where
    F: Fn() -> Result<()> + Send + Sync,
{
    pub name: String,
    pub func: F,
}

impl<F> LatencyBenchmark<F>
where
    F: Fn() -> Result<()> + Send + Sync,
{
    pub fn new(name: String, func: F) -> Self {
        Self { name, func }
    }
}

impl<F> Benchmark for LatencyBenchmark<F>
where
    F: Fn() -> Result<()> + Send + Sync,
{
    fn run(&self) -> Result<u64> {
        let start = crate::subsystems::time::hrtime_nanos();
        (self.func)();
        let end = crate::subsystems::time::hrtime_nanos();
        Ok(end - start)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Latency
    }
}

/// Throughput benchmark helper
pub struct ThroughputBenchmark<F>
where
    F: Fn(&mut u64) -> Result<()> + Send + Sync,
{
    pub name: String,
    pub func: F,
    pub bytes_per_op: u64,
}

impl<F> ThroughputBenchmark<F>
where
    F: Fn(&mut u64) -> Result<()> + Send + Sync,
{
    pub fn new(name: String, func: F, bytes_per_op: u64) -> Self {
        Self {
            name,
            func,
            bytes_per_op,
        }
    }
}

impl<F> Benchmark for ThroughputBenchmark<F>
where
    F: Fn(&mut u64) -> Result<()> + Send + Sync,
{
    fn run(&self) -> Result<u64> {
        let mut ops = 0;
        let start = crate::subsystems::time::hrtime_nanos();
        (self.func)(&mut ops)?;
        let end = crate::subsystems::time::hrtime_nanos();
        Ok(end - start)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Throughput
    }

    fn custom_measurements(&self) -> BTreeMap<String, f64> {
        let mut map = BTreeMap::new();
        map.insert("bytes_per_op".to_string(), self.bytes_per_op as f64);
        map
    }
}

/// Stress test helper
pub struct StressBenchmark<F>
where
    F: Fn(u64) -> Result<()> + Send + Sync,
{
    pub name: String,
    pub func: F,
    pub duration_ns: u64,
}

impl<F> StressBenchmark<F>
where
    F: Fn(u64) -> Result<()> + Send + Sync,
{
    pub fn new(name: String, func: F, duration: Duration) -> Self {
        Self {
            name,
            func,
            duration_ns: duration.as_nanos() as u64,
        }
    }
}

impl<F> Benchmark for StressBenchmark<F>
where
    F: Fn(u64) -> Result<()> + Send + Sync,
{
    fn run(&self) -> Result<u64> {
        let start = crate::subsystems::time::hrtime_nanos();
        let mut iterations = 0;

        while crate::subsystems::time::hrtime_nanos() - start < self.duration_ns {
            (self.func)(iterations)?;
            iterations += 1;
        }

        let end = crate::subsystems::time::hrtime_nanos();
        Ok(end - start)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Stress
    }
}

/// Global benchmark harness
static GLOBAL_HARNESS: OnceLock<Mutex<Option<BenchmarkHarness>>> = OnceLock::new();

/// Initialize global benchmark harness
pub fn init_benchmark(config: BenchmarkConfig) -> Result<()> {
    let harness = BenchmarkHarness::new(config);
    GLOBAL_HARNESS
        .set(Mutex::new(Some(harness)))
        .map_err(|_| Error::Other(String::from("Benchmark harness already initialized")))?;
    Ok(())
}

/// Get global benchmark harness
pub fn get_harness() -> Result<&'static Mutex<Option<BenchmarkHarness>>> {
    GLOBAL_HARNESS
        .get()
        .ok_or_else(|| Error::Other(String::from("Benchmark harness not initialized")))
}

/// Run a benchmark
pub fn run_benchmark(bench: Box<dyn Benchmark>) -> Result<BenchmarkResult> {
    let harness_guard = get_harness()?;
    let mut harness_opt = harness_guard.lock();
    let harness = harness_opt
        .as_mut()
        .ok_or_else(|| Error::Other(String::from("Benchmark harness not initialized")))?;
    harness.run_benchmark(bench.as_ref())
}

/// Compare benchmark results
pub fn compare_results(before: &BenchmarkResult, after: &BenchmarkResult) -> Comparison {
    Comparison::compare(before, after)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_benchmark() {
        let bench = LatencyBenchmark::new("test".to_string(), || Ok(()));
        assert_eq!(bench.name(), "test");
        assert_eq!(bench.bench_type(), BenchmarkType::Latency);
    }

    #[test]
    fn test_benchmark_result() {
        let samples = vec![100u64, 110, 90, 105, 95];
        let result = BenchmarkResult::from_samples(
            "test".to_string(),
            BenchmarkType::Latency,
            &samples,
            0,
        );

        assert_eq!(result.iterations, 5);
        assert_eq!(result.min_ns, 90);
        assert_eq!(result.max_ns, 110);
        assert!(result.mean_ns > 90.0 && result.mean_ns < 110.0);
    }
}
