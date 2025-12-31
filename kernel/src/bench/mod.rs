//! NOS Kernel Performance Benchmark Suite
//!
//! This module provides comprehensive performance testing infrastructure for the NOS kernel,
//! measuring scheduler, memory, network, filesystem, and syscall performance.

mod scheduler;
mod memory;
mod network;
mod filesystem;
mod syscall;
mod report;

pub use report::{BenchmarkReport, PerformanceMetrics};

use core::time::Duration;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::f64::consts::E;

/// Maximum number of samples for statistical analysis
const MAX_SAMPLES: usize = 10000;

/// Benchmark result with statistical analysis
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub name: String,
    /// Number of iterations
    pub iterations: usize,
    /// Total duration
    pub total_duration: Duration,
    /// Average duration per iteration
    pub avg_duration: Duration,
    /// Minimum duration
    pub min_duration: Duration,
    /// Maximum duration
    pub max_duration: Duration,
    /// Median duration
    pub median_duration: Duration,
    /// Standard deviation (nanoseconds)
    pub std_dev: u64,
    /// Percentiles (nanoseconds)
    pub percentiles: BTreeMap<u8, u64>, // p50, p90, p95, p99, p99.9
    /// Throughput (operations per second)
    pub throughput: f64,
    /// Additional metrics
    pub metrics: BTreeMap<String, f64>,
}

impl BenchmarkResult {
    /// Create empty benchmark result
    pub fn new(name: String) -> Self {
        Self {
            name,
            iterations: 0,
            total_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            median_duration: Duration::ZERO,
            std_dev: 0,
            percentiles: BTreeMap::new(),
            throughput: 0.0,
            metrics: BTreeMap::new(),
        }
    }

    /// Add a metric
    pub fn add_metric(&mut self, key: String, value: f64) {
        self.metrics.insert(key, value);
    }
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of warmup iterations
    pub warmup_iterations: usize,
    /// Number of measured iterations
    pub measured_iterations: usize,
    /// Timeout for each iteration
    pub timeout: Duration,
    /// Whether to collect detailed statistics
    pub detailed_stats: bool,
    /// Number of threads for parallel benchmarks
    pub num_threads: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 10,
            measured_iterations: 1000,
            timeout: Duration::from_secs(10),
            detailed_stats: true,
            num_threads: 1,
        }
    }
}

/// Benchmark trait
pub trait Benchmark: Send + Sync {
    /// Get benchmark name
    fn name(&self) -> &str;

    /// Setup before benchmark
    fn setup(&mut self) -> Result<(), BenchmarkError>;

    /// Run single iteration
    fn run(&mut self) -> Result<Duration, BenchmarkError>;

    /// Teardown after benchmark
    fn teardown(&mut self) -> Result<(), BenchmarkError>;

    /// Get benchmark configuration
    fn config(&self) -> &BenchmarkConfig {
        &BenchmarkConfig::default()
    }
}

/// Benchmark errors
#[derive(Debug, Clone)]
pub enum BenchmarkError {
    Timeout(String),
    InvalidInput(String),
    ResourceUnavailable(String),
    Internal(String),
}

impl core::fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Timeout(msg) => write!(f, "Benchmark timeout: {}", msg),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::ResourceUnavailable(msg) => write!(f, "Resource unavailable: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
}

impl BenchmarkRunner {
    /// Create new benchmark runner
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Run a benchmark
    pub fn run<B: Benchmark>(&self, bench: &mut B) -> Result<BenchmarkResult, BenchmarkError> {
        bench.setup()?;

        // Warmup
        for _ in 0..self.config.warmup_iterations {
            let _ = bench.run();
        }

        let mut durations = Vec::with_capacity(self.config.measured_iterations);

        // Measured iterations
        for _ in 0..self.config.measured_iterations {
            let duration = bench.run()?;
            durations.push(duration.as_nanos() as u64);
        }

        bench.teardown()?;

        // Calculate statistics
        let total_duration: u64 = durations.iter().sum();
        let avg_duration = total_duration / durations.len() as u64;
        let min_duration = *durations.iter().min().unwrap_or(&0);
        let max_duration = *durations.iter().max().unwrap_or(&0);

        // Calculate median
        let mut sorted = durations.clone();
        sorted.sort();
        let median_duration = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2
        } else {
            sorted[sorted.len() / 2]
        };

        // Calculate standard deviation
        let variance: u64 = durations
            .iter()
            .map(|&d| {
                let diff = d as i64 - avg_duration as i64;
                (diff * diff) as u64
            })
            .sum();
        let std_dev = libm::sqrt((variance / durations.len() as u64) as f64) as u64;

        // Calculate percentiles
        let mut percentiles = BoundedBTreeMap::new();
        if self.config.detailed_stats && !sorted.is_empty() {
            let percentile_indices = [
                (50u8, sorted.len() * 50 / 100),
                (90u8, sorted.len() * 90 / 100),
                (95u8, sorted.len() * 95 / 100),
                (99u8, sorted.len() * 99 / 100),
                (100u8, sorted.len() - 1),
            ];

            for (p, idx) in percentile_indices {
                percentiles.insert(p, sorted[idx]);
            }
        }

        // Calculate throughput
        let throughput = if total_duration > 0 {
            (durations.len() as f64 * 1_000_000_000.0) / total_duration as f64
        } else {
            0.0
        };

        Ok(BenchmarkResult {
            name: String::from(bench.name()),
            iterations: durations.len(),
            total_duration: Duration::from_nanos(total_duration),
            avg_duration: Duration::from_nanos(avg_duration),
            min_duration: Duration::from_nanos(min_duration),
            max_duration: Duration::from_nanos(max_duration),
            median_duration: Duration::from_nanos(median_duration),
            std_dev,
            percentiles,
            throughput,
            metrics: BoundedBTreeMap::new(),
        })
    }

    /// Run multiple benchmarks
    pub fn run_all<B: Benchmark>(
        &self,
        benchmarks: &mut [B],
    ) -> Result<Vec<BenchmarkResult>, BenchmarkError> {
        let mut results = Vec::with_capacity(benchmarks.len());

        for bench in benchmarks {
            let result = self.run(bench)?;
            results.push(result);
        }

        Ok(results)
    }
}

/// Statistical analysis utilities
pub struct Stats;

impl Stats {
    /// Calculate mean
    pub fn mean(values: &[u64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let sum: u64 = values.iter().sum();
        sum as f64 / values.len() as f64
    }

    /// Calculate standard deviation
    pub fn std_dev(values: &[u64], mean: f64) -> f64 {
        if values.len() <= 1 {
            return 0.0;
        }
        let variance: f64 = values
            .iter()
            .map(|&v| {
                let diff = v as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / values.len() as f64;
        libm::sqrt(variance)
    }

    /// Calculate percentile
    pub fn percentile(values: &mut [u64], p: u8) -> u64 {
        if values.is_empty() {
            return 0;
        }
        values.sort();
        let idx = (values.len() * p as usize) / 100;
        values[idx.min(values.len() - 1)]
    }

    /// Calculate coefficient of variation
    pub fn cv(std_dev: f64, mean: f64) -> f64 {
        if mean == 0.0 {
            0.0
        } else {
            (std_dev / mean) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_benchmark_result_creation() {
        let result = BenchmarkResult::new(String::from("test"));
        assert_eq!(result.name, "test");
        assert_eq!(result.iterations, 0);
    }

    #[test_case]
    fn test_stats_calculations() {
        let values = vec![100u64, 200, 300, 400, 500];
        let mean = Stats::mean(&values);
        assert!((mean - 300.0).abs() < 0.01);

        let std_dev = Stats::std_dev(&values, mean);
        assert!(std_dev > 0.0);

        let p90 = Stats::percentile(&mut values.clone(), 90);
        assert!(p90 >= 400);
    }
}
