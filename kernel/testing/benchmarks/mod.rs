//! Comprehensive Performance Benchmarking Suite
//!
//! This module provides a complete benchmarking infrastructure for the kernel
//! with support for statistical analysis, regression detection, and detailed reporting.

pub mod scheduler;
pub mod memory;
pub mod ipc;
pub mod filesystem;
pub mod network;

use core::time::Duration;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// Result of a single benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub name: String,
    /// Number of iterations executed
    pub iterations: usize,
    /// Total time elapsed
    pub total_time: Duration,
    /// Individual sample times in nanoseconds
    pub samples: Vec<u64>,
    /// Mean execution time in nanoseconds
    pub mean_ns: f64,
    /// Median execution time in nanoseconds
    pub median_ns: f64,
    /// 95th percentile in nanoseconds
    pub p95_ns: f64,
    /// 99th percentile in nanoseconds
    pub p99_ns: f64,
    /// Minimum execution time in nanoseconds
    pub min_ns: u64,
    /// Maximum execution time in nanoseconds
    pub max_ns: u64,
    /// Standard deviation in nanoseconds
    pub std_dev_ns: f64,
    /// Throughput (operations per second)
    pub throughput_ops_per_sec: f64,
}

impl BenchmarkResult {
    /// Create a new benchmark result from samples
    pub fn from_samples(name: String, mut samples: Vec<u64>) -> Self {
        let iterations = samples.len();
        samples.sort_unstable();

        let min_ns = samples.first().copied().unwrap_or(0);
        let max_ns = samples.last().copied().unwrap_or(0);
        let sum_ns: u64 = samples.iter().sum();
        let mean_ns = sum_ns as f64 / iterations as f64;

        let median_ns = if iterations % 2 == 0 {
            let mid = iterations / 2;
            (samples[mid - 1] + samples[mid]) as f64 / 2.0
        } else {
            samples[iterations / 2] as f64
        };

        let p95_idx = (iterations as f64 * 0.95) as usize;
        let p99_idx = (iterations as f64 * 0.99) as usize;
        let p95_ns = samples.get(p95_idx).copied().unwrap_or(max_ns) as f64;
        let p99_ns = samples.get(p99_idx).copied().unwrap_or(max_ns) as f64;

        // Calculate standard deviation
        let variance = samples.iter()
            .map(|&x| {
                let diff = x as f64 - mean_ns;
                diff * diff
            })
            .sum::<f64>() / iterations as f64;
        let std_dev_ns = variance.sqrt();

        let total_time = Duration::from_nanos(sum_ns);
        let total_sec = sum_ns as f64 / 1_000_000_000.0;
        let throughput_ops_per_sec = if total_sec > 0.0 {
            iterations as f64 / total_sec
        } else {
            0.0
        };

        Self {
            name,
            iterations,
            total_time,
            samples,
            mean_ns,
            median_ns,
            p95_ns,
            p99_ns,
            min_ns,
            max_ns,
            std_dev_ns,
            throughput_ops_per_sec,
        }
    }

    /// Format time in appropriate units
    pub fn format_time(ns: f64) -> String {
        if ns < 1_000.0 {
            format!("{:.2} ns", ns)
        } else if ns < 1_000_000.0 {
            format!("{:.2} μs", ns / 1_000.0)
        } else if ns < 1_000_000_000.0 {
            format!("{:.2} ms", ns / 1_000_000.0)
        } else {
            format!("{:.2} s", ns / 1_000_000_000.0)
        }
    }

    /// Format throughput in appropriate units
    pub fn format_throughput(ops_per_sec: f64) -> String {
        if ops_per_sec < 1_000.0 {
            format!("{:.2} ops/sec", ops_per_sec)
        } else if ops_per_sec < 1_000_000.0 {
            format!("{:.2} Kops/sec", ops_per_sec / 1_000.0)
        } else if ops_per_sec < 1_000_000_000.0 {
            format!("{:.2} Mops/sec", ops_per_sec / 1_000_000.0)
        } else {
            format!("{:.2} Gops/sec", ops_per_sec / 1_000_000_000.0)
        }
    }

    /// Generate a human-readable report
    pub fn report(&self) -> String {
        let mut output = format!("## Benchmark: {}\n", self.name);
        output.push_str(&format!("  Iterations: {}\n", self.iterations));
        output.push_str(&format!("  Total Time: {}\n", Self::format_time(self.total_time.as_nanos() as f64)));
        output.push_str(&format!("  Throughput: {}\n", Self::format_throughput(self.throughput_ops_per_sec)));
        output.push_str("\n  Latency Statistics:\n");
        output.push_str(&format!("    Mean:   {}\n", Self::format_time(self.mean_ns)));
        output.push_str(&format!("    Median: {}\n", Self::format_time(self.median_ns)));
        output.push_str(&format!("    Min:    {}\n", Self::format_time(self.min_ns as f64)));
        output.push_str(&format!("    Max:    {}\n", Self::format_time(self.max_ns as f64)));
        output.push_str(&format!("    P95:    {}\n", Self::format_time(self.p95_ns)));
        output.push_str(&format!("    P99:    {}\n", Self::format_time(self.p99_ns)));
        output.push_str(&format!("    StdDev: {}\n", Self::format_time(self.std_dev_ns)));
        output
    }

    /// Check if result meets threshold (returns true if pass)
    pub fn meets_threshold(&self, threshold_ns: u64) -> bool {
        self.p95_ns <= threshold_ns as f64
    }

    /// Calculate percentage difference from baseline
    pub fn regression_from_baseline(&self, baseline_ns: u64) -> f64 {
        if baseline_ns == 0 {
            0.0
        } else {
            ((self.p95_ns - baseline_ns as f64) / baseline_ns as f64) * 100.0
        }
    }
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of warmup iterations (not counted in results)
    pub warmup_iterations: usize,
    /// Number of measurement iterations
    pub measurement_iterations: usize,
    /// Maximum time to spend benchmarking
    pub max_duration: Duration,
    /// Whether to detect regressions
    pub check_regressions: bool,
    /// Regression threshold percentage (warn if > X% degradation)
    pub regression_threshold: f64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 100,
            measurement_iterations: 1000,
            max_duration: Duration::from_secs(10),
            check_regressions: true,
            regression_threshold: 5.0,
        }
    }
}

/// Trait that all benchmarks must implement
pub trait BenchmarkSuite {
    /// Get the name of this benchmark suite
    fn name(&self) -> &str;

    /// Set up the benchmark (called once before all iterations)
    fn setup(&mut self) -> Result<(), String>;

    /// Run a single iteration of the benchmark
    fn run_iteration(&mut self) -> Result<u64, String>;

    /// Tear down the benchmark (called once after all iterations)
    fn teardown(&mut self) -> Result<(), String>;

    /// Execute the benchmark with given configuration
    fn execute(&mut self, config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        self.setup()?;

        // Warmup iterations
        for _ in 0..config.warmup_iterations {
            self.run_iteration().ok();
        }

        // Measurement iterations
        let mut samples = Vec::with_capacity(config.measurement_iterations);
        let start_time = core::time::Instant::now();

        for _ in 0..config.measurement_iterations {
            if start_time.elapsed() > config.max_duration {
                break;
            }

            match self.run_iteration() {
                Ok(ns) => samples.push(ns),
                Err(e) => {
                    self.teardown()?;
                    return Err(format!("Benchmark iteration failed: {}", e));
                }
            }
        }

        self.teardown()?;

        Ok(BenchmarkResult::from_samples(
            self.name().to_string(),
            samples,
        ))
    }
}

/// Benchmark threshold configuration
#[derive(Debug, Clone)]
pub struct BenchmarkThresholds {
    pub scheduler: SchedulerThresholds,
    pub memory: MemoryThresholds,
    pub ipc: IpcThresholds,
    pub filesystem: FilesystemThresholds,
    pub network: NetworkThresholds,
}

#[derive(Debug, Clone)]
pub struct SchedulerThresholds {
    pub context_switch_latency_ns: u64,
    pub schedule_latency_ns: u64,
    pub throughput_switches_per_sec: f64,
}

#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    pub page_alloc_throughput_pages_per_sec: f64,
    pub page_alloc_latency_ns: u64,
    pub fragmentation_ratio_percent: f64,
    pub numa_locality_percent: f64,
}

#[derive(Debug, Clone)]
pub struct IpcThresholds {
    pub pipe_throughput_mb_per_sec: f64,
    pub message_latency_ns: u64,
}

#[derive(Debug, Clone)]
pub struct FilesystemThresholds {
    pub ext4_read_throughput_mb_per_sec: f64,
    pub metadata_ops_per_sec: f64,
}

#[derive(Debug, Clone)]
pub struct NetworkThresholds {
    pub tcp_throughput_gb_per_sec: f64,
    pub connection_setup_latency_ns: u64,
}

/// Summary of all benchmark results
#[derive(Debug)]
pub struct BenchmarkSummary {
    pub results: Vec<BenchmarkResult>,
    pub passed: usize,
    pub failed: usize,
    pub regressions: Vec<String>,
}

impl BenchmarkSummary {
    /// Generate a comprehensive report
    pub fn report(&self) -> String {
        let mut output = String::from("═══════════════════════════════════════════════════════════\n");
        output.push_str("                    KERNEL BENCHMARK REPORT                      \n");
        output.push_str("═══════════════════════════════════════════════════════════\n\n");

        output.push_str(&format!("Total Benchmarks: {}\n", self.results.len()));
        output.push_str(&format!("Passed: {}\n", self.passed));
        output.push_str(&format!("Failed: {}\n", self.failed));

        if !self.regressions.is_empty() {
            output.push_str(&format!("\n⚠️  Regressions Detected: {}\n", self.regressions.len()));
            for regression in &self.regressions {
                output.push_str(&format!("  - {}\n", regression));
            }
        }

        output.push_str("\n───────────────────────────────────────────────────────────────\n\n");

        for result in &self.results {
            output.push_str(&result.report());
            output.push_str("\n───────────────────────────────────────────────────────────────\n\n");
        }

        output.push_str("═══════════════════════════════════════════════════════════\n");
        output
    }
}

/// Run all benchmarks and generate a comprehensive report
pub fn run_all_benchmarks() -> Result<BenchmarkSummary, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut regressions = Vec::new();

    let config = BenchmarkConfig::default();

    // Run scheduler benchmarks
    #[cfg(feature = "scheduler_benchmarks")]
    {
        log::info!("Running scheduler benchmarks...");
        if let Ok(result) = scheduler::run_context_switch_benchmark(&config) {
            if result.p95_ns <= 5_000.0 {
                passed += 1;
            } else {
                failed += 1;
            }
            results.push(result);
        }
    }

    // Run memory benchmarks
    #[cfg(feature = "memory_benchmarks")]
    {
        log::info!("Running memory benchmarks...");
        if let Ok(result) = memory::run_page_allocation_benchmark(&config) {
            if result.throughput_ops_per_sec >= 1_000_000.0 {
                passed += 1;
            } else {
                failed += 1;
            }
            results.push(result);
        }
    }

    // Run IPC benchmarks
    #[cfg(feature = "ipc_benchmarks")]
    {
        log::info!("Running IPC benchmarks...");
        if let Ok(result) = ipc::run_pipe_benchmark(&config) {
            passed += 1;
            results.push(result);
        }
    }

    // Run filesystem benchmarks
    #[cfg(feature = "filesystem_benchmarks")]
    {
        log::info!("Running filesystem benchmarks...");
        if let Ok(result) = filesystem::run_ext4_read_benchmark(&config) {
            passed += 1;
            results.push(result);
        }
    }

    // Run network benchmarks
    #[cfg(feature = "network_benchmarks")]
    {
        log::info!("Running network benchmarks...");
        if let Ok(result) = network::run_tcp_benchmark(&config) {
            passed += 1;
            results.push(result);
        }
    }

    Ok(BenchmarkSummary {
        results,
        passed,
        failed,
        regressions,
    })
}

/// Utility function to measure execution time of a closure
pub fn measure_time<F, R>(mut f: F) -> (R, u64)
where
    F: FnMut() -> R,
{
    let start = core::time::Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    (result, elapsed.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_statistics() {
        let samples = vec
![100, 200, 300, 400, 500];
        let result = BenchmarkResult::from_samples("test".to_string(), samples);

        assert_eq!(result.min_ns, 100);
        assert_eq!(result.max_ns, 500);
        assert_eq!(result.median_ns, 300.0);
        assert!((result.mean_ns - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_time_formatting() {
        assert_eq!(BenchmarkResult::format_time(500.0), "500.00 ns");
        assert_eq!(BenchmarkResult::format_time(5_000.0), "5.00 μs");
        assert_eq!(BenchmarkResult::format_time(5_000_000.0), "5.00 ms");
        assert_eq!(BenchmarkResult::format_time(5_000_000_000.0), "5.00 s");
    }

    #[test]
    fn test_throughput_formatting() {
        assert_eq!(BenchmarkResult::format_throughput(500.0), "500.00 ops/sec");
        assert_eq!(BenchmarkResult::format_throughput(5_000.0), "5.00 Kops/sec");
        assert_eq!(BenchmarkResult::format_throughput(5_000_000.0), "5.00 Mops/sec");
        assert_eq!(BenchmarkResult::format_throughput(5_000_000_000.0), "5.00 Gops/sec");
    }

    #[test]
    fn test_threshold_checking() {
        let samples = vec
![1000; 100];
        let result = BenchmarkResult::from_samples("test".to_string(), samples);

        assert!(result.meets_threshold(2000));
        assert!(!result.meets_threshold(500)
);
    }

    #[test]
    fn test_regression_detection() {
        let samples = vec
![1050; 100];
        let result = BenchmarkResult::from_samples("test".to_string(), samples);

        let regression = result.regression_from_baseline(1000);
        assert!((regression - 5.0).abs() < 0.1);
    }
}
