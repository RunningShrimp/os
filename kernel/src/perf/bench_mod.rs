//! Benchmarking Manager Module
//!
//! This module provides comprehensive benchmark management capabilities including:
//! - Benchmark suite orchestration and test execution
//! - CI/CD integration for automated performance testing
//! - Performance regression detection and alerting
//! - Benchmark suite management and organization
//! - Result aggregation across multiple runs
//! - Public API exports for external integration
//!
//! # Architecture
//!
//! The benchmarking manager provides a high-level API for organizing and running
//! comprehensive benchmark suites:
//! 1. **Benchmark Suites**: Logical groupings of related benchmarks
//! 2. **Test Orchestration**: Automated execution of multiple benchmarks
//! 3. **Result Aggregation**: Statistical aggregation of results
//! 4. **Regression Detection**: Automated performance regression detection
//! 5. **CI/CD Integration**: Hooks for continuous integration systems
//!
//! # Features
//!
//! - Automated benchmark execution
//! - Historical result tracking
//! - Performance trend analysis
//! - Regression alerts
//! - Export to multiple formats (JSON, CSV, HTML)
//! - Parallel benchmark execution
//! - Resource usage monitoring

#![allow(missing_docs)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;

/// Benchmark suite
pub struct BenchmarkSuite {
    /// Suite name
    pub name: String,
    /// Suite description
    pub description: String,
    /// Benchmarks in this suite
    benchmarks: Vec<Arc<dyn Benchmarkable>>,
    /// Suite configuration
    config: SuiteConfig,
}

/// Benchmark suite configuration
#[derive(Debug, Clone)]
pub struct SuiteConfig {
    /// Number of iterations per benchmark
    pub iterations: usize,
    /// Warmup iterations
    pub warmup_iterations: usize,
    /// Timeout per benchmark (seconds)
    pub timeout_secs: u64,
    /// Run benchmarks in parallel
    pub parallel: bool,
    /// Enable regression detection
    pub enable_regression_detection: bool,
    /// Baseline comparison suite
    pub baseline_suite: Option<String>,
}

impl Default for SuiteConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            warmup_iterations: 10,
            timeout_secs: 300,
            parallel: false,
            enable_regression_detection: true,
            baseline_suite: None,
        }
    }
}

/// Benchmark suite result
#[derive(Debug, Clone)]
pub struct SuiteResult {
    /// Suite name
    pub suite_name: String,
    /// Individual benchmark results
    pub benchmark_results: BTreeMap<String, BenchmarkResult>,
    /// Suite summary
    pub summary: SuiteSummary,
    /// Regression detected
    pub regressions: Vec<BenchmarkRegression>,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

/// Suite summary statistics
#[derive(Debug, Clone)]
pub struct SuiteSummary {
    /// Total benchmarks executed
    pub total_benchmarks: usize,
    /// Successful benchmarks
    pub successful_benchmarks: usize,
    /// Failed benchmarks
    pub failed_benchmarks: usize,
    /// Total execution time (nanoseconds)
    pub total_time_ns: u64,
    /// Average execution time per benchmark
    pub avg_time_ns: u64,
}

/// Benchmark regression
#[derive(Debug, Clone)]
pub struct BenchmarkRegression {
    /// Benchmark name
    pub benchmark_name: String,
    /// Baseline result
    pub baseline: BenchmarkResult,
    /// Current result
    pub current: BenchmarkResult,
    /// Performance change percentage
    pub change_pct: f64,
    /// Is regression (performance degradation)
    pub is_regression: bool,
    /// Significance level
    pub significance: f64,
}

/// Execution metadata
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    /// Start timestamp
    pub start_timestamp: u64,
    /// End timestamp
    pub end_timestamp: u64,
    /// Host information
    pub host_info: HostInfo,
    /// Environment variables
    pub environment: BTreeMap<String, String>,
}

/// Host information
#[derive(Debug, Clone)]
pub struct HostInfo {
    /// CPU count
    pub cpu_count: u32,
    /// Total memory
    pub total_memory: u64,
    /// Kernel version
    pub kernel_version: String,
    /// Architecture
    pub architecture: String,
}

/// CI/CD integration configuration
#[derive(Debug, Clone)]
pub struct CICDConfig {
    /// Enable CI/CD mode
    pub enabled: bool,
    /// Regression threshold (percentage)
    pub regression_threshold_pct: f64,
    /// Fail on regression
    pub fail_on_regression: bool,
    /// Export results to file
    pub export_results: bool,
    /// Export format
    pub export_format: ExportFormat,
    /// Output directory
    pub output_dir: String,
}

impl Default for CICDConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            regression_threshold_pct: 5.0,
            fail_on_regression: true,
            export_results: true,
            export_format: ExportFormat::Json,
            output_dir: String::from("/tmp/bench_results"),
        }
    }
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// HTML format
    Html,
    /// Plain text
    Text,
}

/// Re-export from bench module
use super::bench::{Benchmark, BenchmarkResult, BenchmarkType};

/// Benchmarkable trait (extends Benchmark trait)
pub trait Benchmarkable: Send + Sync {
    /// Get benchmark name
    fn name(&self) -> &str;

    /// Run the benchmark
    fn run(&self) -> Result<u64>;

    /// Setup before benchmark
    fn setup(&self) -> Result<()> {
        Ok(())
    }

    /// Teardown after benchmark
    fn teardown(&self) -> Result<()> {
        Ok(())
    }

    /// Get benchmark type
    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Latency
    }

    /// Get custom measurements
    fn custom_measurements(&self) -> BTreeMap<String, f64> {
        BTreeMap::new()
    }
}

// Implement Benchmarkable for specific benchmark types
impl Benchmarkable for super::bench::LatencyBenchmark<fn() -> Result<()>> {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self) -> Result<u64> {
        let start = crate::subsystems::time::hrtime_nanos();
        (self.func)();
        let end = crate::subsystems::time::hrtime_nanos();
        Ok(end - start)
    }

    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Latency
    }
}

impl<F> Benchmarkable for super::bench::ThroughputBenchmark<F>
where
    F: Fn(&mut u64) -> Result<()> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self) -> Result<u64> {
        let mut ops = 0;
        let start = crate::subsystems::time::hrtime_nanos();
        (self.func)(&mut ops)?;
        let end = crate::subsystems::time::hrtime_nanos();
        Ok(end - start)
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

impl<F> Benchmarkable for super::bench::StressBenchmark<F>
where
    F: Fn(u64) -> Result<()> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

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

    fn bench_type(&self) -> BenchmarkType {
        BenchmarkType::Stress
    }
}

/// Benchmarking manager
pub struct BenchmarkManager {
    /// Registered suites
    suites: Mutex<BTreeMap<String, Arc<BenchmarkSuite>>>,
    /// Historical results
    history: Mutex<BTreeMap<String, Vec<SuiteResult>>>,
    /// CI/CD configuration
    cicd_config: Mutex<CICDConfig>,
    /// Active flag
    active: AtomicBool,
}

impl BenchmarkManager {
    /// Create a new benchmark manager
    pub fn new() -> Self {
        Self {
            suites: Mutex::new(BTreeMap::new()),
            history: Mutex::new(BTreeMap::new()),
            cicd_config: Mutex::new(CICDConfig::default()),
            active: AtomicBool::new(false),
        }
    }

    /// Register a benchmark suite
    pub fn register_suite(&self, suite: BenchmarkSuite) -> Result<(), Error> {
        let name = suite.name.clone();
        let mut suites = self.suites.lock();
        suites.insert(name, Arc::new(suite));
        Ok(())
    }

    /// Unregister a benchmark suite
    pub fn unregister_suite(&self, name: &str) -> Result<(), Error> {
        let mut suites = self.suites.lock();
        suites
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| Error::Other(format!("Suite {} not found", name)))
    }

    /// List all registered suites
    pub fn list_suites(&self) -> Vec<String> {
        let suites = self.suites.lock();
        suites.keys().cloned().collect()
    }

    /// Run a specific benchmark suite
    pub fn run_suite(&self, suite_name: &str) -> Result<SuiteResult, Error> {
        let suite = {
            let suites = self.suites.lock();
            suites
                .get(suite_name)
                .cloned()
                .ok_or_else(|| Error::Other(format!("Suite {} not found", suite_name)))?
        };

        self.execute_suite(&suite)
    }

    /// Run all registered suites
    pub fn run_all_suites(&self) -> Result<Vec<SuiteResult>, Error> {
        let suite_names = self.list_suites();
        let mut results = Vec::new();

        for name in &suite_names {
            match self.run_suite(name) {
                Ok(result) => results.push(result),
                Err(e) => {
                    log::error!("Failed to run suite {}: {:?}", name, e);
                }
            }
        }

        Ok(results)
    }

    /// Execute a benchmark suite
    fn execute_suite(&self, suite: &BenchmarkSuite) -> Result<SuiteResult, Error> {
        let start_timestamp = crate::subsystems::time::hrtime_nanos();

        log::info!("Running benchmark suite: {}", suite.name);

        let mut benchmark_results = BTreeMap::new();
        let mut successful = 0;
        let mut failed = 0;

        // Run each benchmark in the suite
        for benchmark in &suite.benchmarks {
            let bench_name = benchmark.name();

            // Setup
            if let Err(e) = benchmark.setup() {
                log::error!("Benchmark {} setup failed: {:?}", bench_name, e);
                failed += 1;
                continue;
            }

            // Warmup
            for _ in 0..suite.config.warmup_iterations {
                let _ = benchmark.run();
            }

            // Measurement
            let mut samples = Vec::with_capacity(suite.config.iterations);
            for _ in 0..suite.config.iterations {
                match benchmark.run() {
                    Ok(time_ns) => samples.push(time_ns),
                    Err(e) => {
                        log::error!("Benchmark {} iteration failed: {:?}", bench_name, e);
                        failed += 1;
                        break;
                    }
                }
            }

            // Teardown
            let _ = benchmark.teardown();

            // Create result
            let result = BenchmarkResult::from_samples(
                bench_name.to_string(),
                benchmark.bench_type(),
                &samples,
                crate::subsystems::time::hrtime_nanos(),
            );

            benchmark_results.insert(bench_name.to_string(), result);
            successful += 1;
        }

        let end_timestamp = crate::subsystems::time::hrtime_nanos();
        let total_time_ns = end_timestamp - start_timestamp;

        // Check for regressions
        let regressions = if suite.config.enable_regression_detection {
            self.detect_regressions(&suite.name, &benchmark_results)?
        } else {
            Vec::new()
        };

        // Create summary
        let summary = SuiteSummary {
            total_benchmarks: suite.benchmarks.len(),
            successful_benchmarks: successful,
            failed_benchmarks: failed,
            total_time_ns,
            avg_time_ns: if suite.benchmarks.len() > 0 {
                total_time_ns / suite.benchmarks.len() as u64
            } else {
                0
            },
        };

        // Create metadata
        let metadata = ExecutionMetadata {
            start_timestamp,
            end_timestamp,
            host_info: HostInfo {
                cpu_count: 4, // Placeholder
                total_memory: 8 * 1024 * 1024 * 1024, // Placeholder
                kernel_version: String::from("1.0.0"),
                architecture: String::from("x86_64"),
            },
            environment: BTreeMap::new(),
        };

        let result = SuiteResult {
            suite_name: suite.name.clone(),
            benchmark_results,
            summary,
            regressions,
            metadata,
        };

        // Store in history
        let mut history = self.history.lock();
        history
            .entry(suite.name.clone())
            .or_insert_with(Vec::new)
            .push(result.clone());

        // Limit history size
        if let Some(results) = history.get_mut(&suite.name) {
            if results.len() > 100 {
                results.remove(0);
            }
        }

        log::info!(
            "Suite {} completed: {}/{} benchmarks successful",
            suite.name,
            successful,
            suite.benchmarks.len()
        );

        Ok(result)
    }

    /// Detect performance regressions
    fn detect_regressions(
        &self,
        suite_name: &str,
        current_results: &BTreeMap<String, BenchmarkResult>,
    ) -> Result<Vec<BenchmarkRegression>, Error> {
        let mut regressions = Vec::new();

        // Get baseline results
        let baseline_results = {
            let history = self.history.lock();
            history
                .get(suite_name)
                .and_then(|results| results.get(results.len().saturating_sub(2)))
                .cloned()
        };

        if let Some(baseline_suite) = baseline_results {
            for (bench_name, current_result) in current_results {
                if let Some(baseline_result) = baseline_suite.benchmark_results.get(bench_name) {
                    let change_pct = if baseline_result.mean_ns > 0.0 {
                        ((current_result.mean_ns - baseline_result.mean_ns) / baseline_result.mean_ns)
                            * 100.0
                    } else {
                        0.0
                    };

                    let is_regression = change_pct > 5.0;

                    if is_regression {
                        regressions.push(BenchmarkRegression {
                            benchmark_name: bench_name.clone(),
                            baseline: baseline_result.clone(),
                            current: current_result.clone(),
                            change_pct,
                            is_regression,
                            significance: 0.95, // Placeholder
                        });
                    }
                }
            }
        }

        Ok(regressions)
    }

    /// Get historical results for a suite
    pub fn get_history(&self, suite_name: &str) -> Vec<SuiteResult> {
        let history = self.history.lock();
        history.get(suite_name).cloned().unwrap_or_default()
    }

    /// Aggregate results across multiple runs
    pub fn aggregate_results(
        &self,
        suite_name: &str,
        count: usize,
    ) -> Result<BTreeMap<String, BenchmarkResult>, Error> {
        let history = self.get_history(suite_name);

        if history.is_empty() {
            return Err(Error::Other(String::from("No historical results found")));
        }

        let mut aggregated = BTreeMap::new();
        let mut benchmark_samples: BTreeMap<String, Vec<u64>> = BTreeMap::new();

        // Collect samples from all runs
        let runs_to_include = count.min(history.len());
        for result in history.iter().rev().take(runs_to_include) {
            for (bench_name, bench_result) in &result.benchmark_results {
                let samples = benchmark_samples
                    .entry(bench_name.clone())
                    .or_insert_with(Vec::new);
                // Add mean as a sample (simplified)
                samples.push(bench_result.mean_ns as u64);
            }
        }

        // Create aggregated results
        for (bench_name, samples) in benchmark_samples {
            let result = BenchmarkResult::from_samples(
                bench_name.clone(),
                BenchmarkType::Latency,
                &samples,
                crate::subsystems::time::hrtime_nanos(),
            );
            aggregated.insert(bench_name, result);
        }

        Ok(aggregated)
    }

    /// Set CI/CD configuration
    pub fn set_cicd_config(&self, config: CICDConfig) {
        let mut cicd = self.cicd_config.lock();
        *cicd = config;
    }

    /// Export results in specified format
    pub fn export_results(
        &self,
        result: &SuiteResult,
        format: ExportFormat,
    ) -> Result<String, Error> {
        match format {
            ExportFormat::Json => self.export_json(result),
            ExportFormat::Csv => self.export_csv(result),
            ExportFormat::Html => self.export_html(result),
            ExportFormat::Text => self.export_text(result),
        }
    }

    /// Export as JSON
    fn export_json(&self, result: &SuiteResult) -> Result<String, Error> {
        let mut json = String::new();

        json.push_str("{\n");
        json.push_str(&format!("  \"suite_name\": \"{}\",\n", result.suite_name));
        json.push_str(&format!("  \"total_benchmarks\": {},\n", result.summary.total_benchmarks));
        json.push_str(&format!(
            "  \"successful\": {},\n",
            result.summary.successful_benchmarks
        ));
        json.push_str(&format!("  \"failed\": {},\n", result.summary.failed_benchmarks));
        json.push_str(&format!("  \"total_time_ns\": {},\n", result.summary.total_time_ns));
        json.push_str("  \"benchmarks\": {\n");

        let mut first = true;
        for (name, bench_result) in &result.benchmark_results {
            if !first {
                json.push_str(",\n");
            }
            first = false;

            json.push_str(&format!("    \"{}\": {{\n", name));
            json.push_str(&format!("      \"iterations\": {},\n", bench_result.iterations));
            json.push_str(&format!("      \"mean_ns\": {:.2},\n", bench_result.mean_ns));
            json.push_str(&format!("      \"min_ns\": {},\n", bench_result.min_ns));
            json.push_str(&format!("      \"max_ns\": {},\n", bench_result.max_ns));
            json.push_str(&format!("      \"p95_ns\": {}\n", bench_result.p95_ns));
            json.push_str("    }");
        }

        json.push_str("\n  }\n}\n");

        Ok(json)
    }

    /// Export as CSV
    fn export_csv(&self, result: &SuiteResult) -> Result<String, Error> {
        let mut csv = String::new();

        csv.push_str("benchmark,iterations,mean_ns,min_ns,max_ns,p95_ns,p99_ns\n");

        for (name, bench_result) in &result.benchmark_results {
            csv.push_str(&format!(
                "{},{},{:.2},{},{},{},{}\n",
                name,
                bench_result.iterations,
                bench_result.mean_ns,
                bench_result.min_ns,
                bench_result.max_ns,
                bench_result.p95_ns,
                bench_result.p99_ns
            ));
        }

        Ok(csv)
    }

    /// Export as HTML
    fn export_html(&self, result: &SuiteResult) -> Result<String, Error> {
        let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>Benchmark Results</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4CAF50; color: white; }
        tr:nth-child(even) { background-color: #f2f2f2; }
        .regression { color: red; font-weight: bold; }
    </style>
</head>
<body>
"#);

        html.push_str(&format!("<h1>Benchmark Suite: {}</h1>\n", result.suite_name));
        html.push_str(&format!(
            "<p>Total: {} | Successful: {} | Failed: {}</p>\n",
            result.summary.total_benchmarks,
            result.summary.successful_benchmarks,
            result.summary.failed_benchmarks
        ));

        html.push_str("<table>\n");
        html.push_str("<tr><th>Benchmark</th><th>Iterations</th><th>Mean (ns)</th><th>Min (ns)</th><th>Max (ns)</th><th>P95 (ns)</th></tr>\n");

        for (name, bench_result) in &result.benchmark_results {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{:.2}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                name, bench_result.iterations, bench_result.mean_ns, bench_result.min_ns,
                bench_result.max_ns, bench_result.p95_ns
            ));
        }

        html.push_str("</table>\n");
        html.push_str("</body>\n</html>");

        Ok(html)
    }

    /// Export as plain text
    fn export_text(&self, result: &SuiteResult) -> Result<String, Error> {
        let mut text = String::new();

        text.push_str(&format!("Benchmark Suite: {}\n", result.suite_name));
        text.push_str(&format!(
            "Total: {} | Successful: {} | Failed: {}\n\n",
            result.summary.total_benchmarks,
            result.summary.successful_benchmarks,
            result.summary.failed_benchmarks
        ));

        text.push_str("Benchmarks:\n");
        for (name, bench_result) in &result.benchmark_results {
            text.push_str(&format!(
                "  {}:\n    Iterations: {}\n    Mean: {:.2} ns\n    Min: {} ns\n    Max: {} ns\n    P95: {} ns\n",
                name, bench_result.iterations, bench_result.mean_ns, bench_result.min_ns,
                bench_result.max_ns, bench_result.p95_ns
            ));
        }

        Ok(text)
    }
}

impl BenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            benchmarks: Vec::new(),
            config: SuiteConfig::default(),
        }
    }

    /// Add a benchmark to the suite
    pub fn add_benchmark(&mut self, benchmark: Arc<dyn Benchmarkable>) {
        self.benchmarks.push(benchmark);
    }

    /// Set suite configuration
    pub fn set_config(&mut self, config: SuiteConfig) {
        self.config = config;
    }
}

/// Global benchmark manager
static GLOBAL_BENCHMARK_MANAGER: OnceLock<Mutex<BenchmarkManager>> = OnceLock::new();

/// Initialize global benchmark manager
pub fn init_benchmark_manager() -> Result<(), Error> {
    GLOBAL_BENCHMARK_MANAGER
        .set(Mutex::new(BenchmarkManager::new()))
        .map_err(|_| Error::Other(String::from("Benchmark manager already initialized")))?;
    Ok(())
}

/// Get global benchmark manager
pub fn get_benchmark_manager() -> Option<&'static Mutex<BenchmarkManager>> {
    GLOBAL_BENCHMARK_MANAGER.get()
}

/// Run benchmark suite by name
pub fn run_suite(suite_name: &str) -> Result<SuiteResult, Error> {
    let manager = get_benchmark_manager()
        .ok_or_else(|| Error::Other(String::from("Benchmark manager not initialized")))?;

    let manager = manager.lock();
    manager.run_suite(suite_name)
}

/// Run all registered suites
pub fn run_all_suites() -> Result<Vec<SuiteResult>, Error> {
    let manager = get_benchmark_manager()
        .ok_or_else(|| Error::Other(String::from("Benchmark manager not initialized")))?;

    let manager = manager.lock();
    manager.run_all_suites()
}

/// Register a benchmark suite
pub fn register_suite(suite: BenchmarkSuite) -> Result<(), Error> {
    let manager = get_benchmark_manager()
        .ok_or_else(|| Error::Other(String::from("Benchmark manager not initialized")))?;

    let manager = manager.lock();
    manager.register_suite(suite)
}

/// Get suite history
pub fn get_suite_history(suite_name: &str) -> Vec<SuiteResult> {
    if let Some(manager) = get_benchmark_manager() {
        let manager = manager.lock();
        manager.get_history(suite_name)
    } else {
        Vec::new()
    }
}

/// Aggregate suite results
pub fn aggregate_suite_results(suite_name: &str, count: usize) -> Result<BTreeMap<String, BenchmarkResult>, Error> {
    let manager = get_benchmark_manager()
        .ok_or_else(|| Error::Other(String::from("Benchmark manager not initialized")))?;

    let manager = manager.lock();
    manager.aggregate_results(suite_name, count)
}

/// Export suite results
pub fn export_suite_results(result: &SuiteResult, format: ExportFormat) -> Result<String, Error> {
    let manager = get_benchmark_manager()
        .ok_or_else(|| Error::Other(String::from("Benchmark manager not initialized")))?;

    let manager = manager.lock();
    manager.export_results(result, format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::bench::LatencyBenchmark;

    #[test]
    fn test_benchmark_suite() {
        let mut suite = BenchmarkSuite::new(
            String::from("test_suite"),
            String::from("Test suite"),
        );

        let bench = Arc::new(LatencyBenchmark::new(String::from("test"), || Ok(())));
        suite.add_benchmark(bench);

        assert_eq!(suite.benchmarks.len(), 1);
    }

    #[test]
    fn test_benchmark_manager() {
        let manager = BenchmarkManager::new();

        let suite = BenchmarkSuite::new(
            String::from("test_suite"),
            String::from("Test"),
        );

        manager.register_suite(suite).unwrap();
        let suites = manager.list_suites();

        assert_eq!(suites.len(), 1);
        assert_eq!(suites[0], "test_suite");
    }
}
