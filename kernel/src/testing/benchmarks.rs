//! Benchmark Testing Module
//! 
//! This module provides comprehensive benchmarking capabilities for the NOS kernel,
//! including performance measurement, comparison with baselines, and regression detection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of iterations to run
    pub iterations: usize,
    /// Warmup iterations
    pub warmup_iterations: usize,
    /// Enable statistical analysis
    pub enable_statistics: bool,
    /// Enable comparison with baseline
    pub enable_baseline_comparison: bool,
    /// Baseline data file path
    pub baseline_file: Option<String>,
    /// Output directory for results
    pub output_directory: String,
    /// Enable detailed profiling
    pub enable_profiling: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            warmup_iterations: 10,
            enable_statistics: true,
            enable_baseline_comparison: true,
            baseline_file: None,
            output_directory: String::from("/tmp/benchmarks"),
            enable_profiling: false,
        }
    }
}

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Category
    pub category: String,
    /// Unit of measurement
    pub unit: String,
    /// Average value
    pub average: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Median value
    pub median: f64,
    /// Standard deviation
    pub stddev: f64,
    /// Percentile 95
    pub p95: f64,
    /// Percentile 99
    pub p99: f64,
    /// Baseline value for comparison
    pub baseline: Option<f64>,
    /// Performance ratio (current/baseline)
    pub performance_ratio: Option<f64>,
    /// Regression detected
    pub regression_detected: bool,
    /// Improvement detected
    pub improvement_detected: bool,
    /// Custom metrics
    pub custom_metrics: BTreeMap<String, f64>,
}

/// Benchmark suite
#[derive(Debug)]
pub struct BenchmarkSuite {
    /// Suite name
    pub name: String,
    /// Benchmark cases
    pub benchmark_cases: Vec<BenchmarkCase>,
    /// Setup function
    pub setup_fn: Option<fn() -> Result<(), String>>,
    /// Teardown function
    pub teardown_fn: Option<fn() -> Result<(), String>>,
}

/// Benchmark case
#[derive(Debug)]
pub struct BenchmarkCase {
    /// Benchmark name
    pub name: String,
    /// Benchmark function
    pub benchmark_fn: fn() -> BenchmarkMeasurement,
    /// Category
    pub category: String,
    /// Unit of measurement
    pub unit: String,
    /// Expected performance range
    pub expected_range: Option<(f64, f64)>,
    /// Tags
    pub tags: Vec<String>,
}

/// Benchmark measurement
#[derive(Debug, Clone)]
pub struct BenchmarkMeasurement {
    /// Primary measurement value
    pub value: f64,
    /// Unit of measurement
    pub unit: String,
    /// Additional metrics
    pub metrics: BTreeMap<String, f64>,
}

/// Benchmark system
pub struct BenchmarkSystem {
    /// Configuration
    config: BenchmarkConfig,
    /// Benchmark suites
    benchmark_suites: Mutex<Vec<BenchmarkSuite>>,
    /// Baseline data
    baseline_data: Mutex<BTreeMap<String, f64>>,
    /// Global statistics
    global_stats: BenchmarkStats,
}

/// Benchmark statistics
#[derive(Debug, Default)]
pub struct BenchmarkStats {
    /// Total benchmarks run
    pub total_benchmarks_run: AtomicU64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: AtomicU64,
    /// Regressions detected
    pub regressions_detected: AtomicU64,
    /// Improvements detected
    pub improvements_detected: AtomicU64,
}

impl BenchmarkSystem {
    /// Create a new benchmark system
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            benchmark_suites: Mutex::new(Vec::new()),
            baseline_data: Mutex::new(BTreeMap::new()),
            global_stats: BenchmarkStats::default(),
        }
    }

    /// Register a benchmark suite
    pub fn register_suite(&self, suite: BenchmarkSuite) {
        let mut suites = self.benchmark_suites.lock();
        suites.push(suite);
    }

    /// Run all benchmarks
    pub fn run_all_benchmarks(&self) -> Result<Vec<BenchmarkResult>, String> {
        let suites = self.benchmark_suites.lock();
        let mut all_results = Vec::new();

        for suite in suites.iter() {
            let suite_results = self.run_benchmark_suite(suite)?;
            all_results.extend(suite_results);
        }

        Ok(all_results)
    }

    /// Run a specific benchmark suite
    pub fn run_benchmark_suite(&self, suite: &BenchmarkSuite) -> Result<Vec<BenchmarkResult>, String> {
        let mut results = Vec::new();

        // Run setup function
        if let Some(setup_fn) = suite.setup_fn {
            setup_fn().map_err(|e| format!("Setup failed: {}", e))?;
        }

        // Run all benchmark cases
        for benchmark_case in &suite.benchmark_cases {
            let result = self.run_benchmark_case(benchmark_case)?;
            results.push(result);
        }

        // Run teardown function
        if let Some(teardown_fn) = suite.teardown_fn {
            teardown_fn().map_err(|e| format!("Teardown failed: {}", e))?;
        }

        Ok(results)
    }

    /// Run a single benchmark case
    pub fn run_benchmark_case(&self, benchmark_case: &BenchmarkCase) -> Result<BenchmarkResult, String> {
        let mut measurements = Vec::with_capacity(self.config.iterations + self.config.warmup_iterations);

        // Warmup iterations
        for _ in 0..self.config.warmup_iterations {
            let _ = (benchmark_case.benchmark_fn)();
        }

        // Actual benchmark iterations
        for _ in 0..self.config.iterations {
            let measurement = (benchmark_case.benchmark_fn)();
            measurements.push(measurement.value);
        }

        // Calculate statistics
        let stats = self.calculate_statistics(&measurements);

        // Get baseline for comparison
        let baseline = self.get_baseline(&benchmark_case.name);
        let performance_ratio = baseline.map(|b| stats.average / b);
        let regression_detected = performance_ratio.map_or(false, |r| r > 1.1); // 10% regression threshold
        let improvement_detected = performance_ratio.map_or(false, |r| r < 0.9); // 10% improvement threshold

        // Update global statistics
        self.global_stats.total_benchmarks_run.fetch_add(1, Ordering::SeqCst);

        if regression_detected {
            self.global_stats.regressions_detected.fetch_add(1, Ordering::SeqCst);
        }

        if improvement_detected {
            self.global_stats.improvements_detected.fetch_add(1, Ordering::SeqCst);
        }

        Ok(BenchmarkResult {
            name: benchmark_case.name.clone(),
            category: benchmark_case.category.clone(),
            unit: benchmark_case.unit.clone(),
            average: stats.average,
            min: stats.min,
            max: stats.max,
            median: stats.median,
            stddev: stats.stddev,
            p95: stats.p95,
            p99: stats.p99,
            baseline,
            performance_ratio,
            regression_detected,
            improvement_detected,
            custom_metrics: BTreeMap::new(),
        })
    }

    /// Calculate statistics from measurements
    fn calculate_statistics(&self, measurements: &[f64]) -> BenchmarkStatistics {
        let mut sorted_measurements = measurements.to_vec();
        sorted_measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = measurements.len();
        let sum: f64 = measurements.iter().sum();
        let average = sum / count as f64;

        // Calculate standard deviation
        let variance = measurements.iter()
            .map(|x| (x - average).powi(2))
            .sum::<f64>() / count as f64;
        let stddev = variance.sqrt();

        // Calculate percentiles
        let median = if count % 2 == 0 {
            (sorted_measurements[count / 2 - 1] + sorted_measurements[count / 2]) / 2.0
        } else {
            sorted_measurements[count / 2]
        };

        let p95_index = (count as f64 * 0.95) as usize;
        let p95 = sorted_measurements[p95_index.min(count - 1)];

        let p99_index = (count as f64 * 0.99) as usize;
        let p99 = sorted_measurements[p99_index.min(count - 1)];

        BenchmarkStatistics {
            average,
            min: sorted_measurements[0],
            max: sorted_measurements[count - 1],
            median,
            stddev,
            p95,
            p99,
        }
    }

    /// Get baseline value for a benchmark
    fn get_baseline(&self, benchmark_name: &str) -> Option<f64> {
        let baseline_data = self.baseline_data.lock();
        baseline_data.get(benchmark_name).copied()
    }

    /// Set baseline value for a benchmark
    pub fn set_baseline(&self, benchmark_name: &str, value: f64) {
        let mut baseline_data = self.baseline_data.lock();
        baseline_data.insert(benchmark_name.to_string(), value);
    }

    /// Load baseline data from file
    pub fn load_baseline_data(&self, file_path: &str) -> Result<(), String> {
        // In a real implementation, this would read from file
        // For now, we'll just return success
        Ok(())
    }

    /// Save baseline data to file
    pub fn save_baseline_data(&self, file_path: &str) -> Result<(), String> {
        // In a real implementation, this would write to file
        // For now, we'll just return success
        Ok(())
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> &BenchmarkStats {
        &self.global_stats
    }
}

/// Benchmark statistics
#[derive(Debug)]
struct BenchmarkStatistics {
    /// Average value
    pub average: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Median value
    pub median: f64,
    /// Standard deviation
    pub stddev: f64,
    /// Percentile 95
    pub p95: f64,
    /// Percentile 99
    pub p99: f64,
}

/// Global benchmark system instance
static mut BENCHMARK_SYSTEM: Option<BenchmarkSystem> = None;
static BENCHMARK_SYSTEM_INIT: spin::Once = spin::Once::new();

/// Initialize the global benchmark system
pub fn init_benchmark_system(config: BenchmarkConfig) -> Result<(), String> {
    BENCHMARK_SYSTEM_INIT.call_once(|| {
        let system = BenchmarkSystem::new(config);
        unsafe {
            BENCHMARK_SYSTEM = Some(system);
        }
    });
    Ok(())
}

/// Get the global benchmark system
pub fn get_benchmark_system() -> Option<&'static BenchmarkSystem> {
    unsafe {
        BENCHMARK_SYSTEM.as_ref()
    }
}

/// Register a benchmark suite
pub fn register_benchmark_suite(suite: BenchmarkSuite) {
    if let Some(system) = get_benchmark_system() {
        system.register_suite(suite);
    }
}

/// Run all benchmarks
pub fn run_all_benchmarks() -> Result<Vec<BenchmarkResult>, String> {
    let system = get_benchmark_system().ok_or("Benchmark system not initialized")?;
    system.run_all_benchmarks()
}

/// Macro to create a benchmark case
#[macro_export]
macro_rules! benchmark_case {
    ($name:expr, $fn:expr, $unit:expr) => {
        $crate::testing::benchmarks::BenchmarkCase {
            name: $name.to_string(),
            benchmark_fn: $fn,
            category: "General".to_string(),
            unit: $unit.to_string(),
            expected_range: None,
            tags: Vec::new(),
        }
    };
    ($name:expr, $fn:expr, $unit:expr, category => $category:expr) => {
        $crate::testing::benchmarks::BenchmarkCase {
            name: $name.to_string(),
            benchmark_fn: $fn,
            category: $category.to_string(),
            unit: $unit.to_string(),
            expected_range: None,
            tags: Vec::new(),
        }
    };
    ($name:expr, $fn:expr, $unit:expr, category => $category:expr, expected_range => $range:expr) => {
        $crate::testing::benchmarks::BenchmarkCase {
            name: $name.to_string(),
            benchmark_fn: $fn,
            category: $category.to_string(),
            unit: $unit.to_string(),
            expected_range: Some($range),
            tags: Vec::new(),
        }
    };
    ($name:expr, $fn:expr, $unit:expr, category => $category:expr, expected_range => $range:expr, tags => [$($tag:expr),*]) => {
        $crate::testing::benchmarks::BenchmarkCase {
            name: $name.to_string(),
            benchmark_fn: $fn,
            category: $category.to_string(),
            unit: $unit.to_string(),
            expected_range: Some($range),
            tags: vec![$($tag.to_string()),*],
        }
    };
}

/// Macro to create a benchmark suite
#[macro_export]
macro_rules! benchmark_suite {
    ($name:expr, [$($benchmark_case:expr),*]) => {
        $crate::testing::benchmarks::BenchmarkSuite {
            name: $name.to_string(),
            benchmark_cases: vec![$($benchmark_case),*],
            setup_fn: None,
            teardown_fn: None,
        }
    };
    ($name:expr, [$($benchmark_case:expr),*], setup => $setup:expr) => {
        $crate::testing::benchmarks::BenchmarkSuite {
            name: $name.to_string(),
            benchmark_cases: vec![$($benchmark_case),*],
            setup_fn: Some($setup),
            teardown_fn: None,
        }
    };
    ($name:expr, [$($benchmark_case:expr),*], setup => $setup:expr, teardown => $teardown:expr) => {
        $crate::testing::benchmarks::BenchmarkSuite {
            name: $name.to_string(),
            benchmark_cases: vec![$($benchmark_case),*],
            setup_fn: Some($setup),
            teardown_fn: Some($teardown),
        }
    };
}

/// Macro to measure execution time
#[macro_export]
macro_rules! benchmark_time {
    ($block:block) => {{
        let start = $crate::time::get_ticks();
        let _result = $block;
        let end = $crate::time::get_ticks();
        $crate::testing::benchmarks::BenchmarkMeasurement {
            value: (end - start) as f64,
            unit: "ms".to_string(),
            metrics: $crate::alloc::collections::BTreeMap::new(),
        }
    }};
}

/// Macro to measure operations per second
#[macro_export]
macro_rules! benchmark_ops_per_sec {
    ($iterations:expr, $block:block) => {{
        let start = $crate::time::get_ticks();
        for _ in 0..$iterations {
            $block;
        }
        let end = $crate::time::get_ticks();
        let elapsed_ms = end - start;
        let ops_per_sec = ($iterations as f64) / (elapsed_ms as f64 / 1000.0);
        $crate::testing::benchmarks::BenchmarkMeasurement {
            value: ops_per_sec,
            unit: "ops/sec".to_string(),
            metrics: $crate::alloc::collections::BTreeMap::new(),
        }
    }};
}

/// Macro to measure throughput
#[macro_export]
macro_rules! benchmark_throughput {
    ($bytes:expr, $block:block) => {{
        let start = $crate::time::get_ticks();
        $block;
        let end = $crate::time::get_ticks();
        let elapsed_ms = end - start;
        let throughput_mb_per_sec = ($bytes as f64) / (elapsed_ms as f64 / 1000.0) / (1024.0 * 1024.0);
        $crate::testing::benchmarks::BenchmarkMeasurement {
            value: throughput_mb_per_sec,
            unit: "MB/sec".to_string(),
            metrics: $crate::alloc::collections::BTreeMap::new(),
        }
    }};
}