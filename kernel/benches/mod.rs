//! Kernel Benchmark Framework
//!
//! A comprehensive benchmark framework for measuring kernel component performance.
//! Supports:
//! - Micro-benchmarks (single operation timing)
//! - Macro-benchmarks (end-to-end scenarios)
//! - Regression detection
//! - Statistical analysis

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub ops_per_sec: f64,
}

impl BenchmarkResult {
    pub fn new(name: String, iterations: u64, timings: Vec<Duration>) -> Self {
        let total_time: Duration = timings.iter().sum();
        let avg_time = total_time / iterations as u32;

        let min_time = *timings.iter().min().unwrap_or(&Duration::from_nanos(0));
        let max_time = *timings.iter().max().unwrap_or(&Duration::from_nanos(0));

        let total_secs = total_time.as_secs_f64();
        let ops_per_sec = if total_secs > 0.0 {
            iterations as f64 / total_secs
        } else {
            0.0
        };

        Self {
            name,
            iterations,
            total_time,
            avg_time,
            min_time,
            max_time,
            ops_per_sec,
        }
    }
}

/// Benchmark configuration
#[derive(Debug, Clone, Copy)]
pub struct BenchmarkConfig {
    pub warmup_iterations: u64,
    pub measurement_iterations: u64,
    pub sample_size: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 10,
            measurement_iterations: 100,
            sample_size: 1000,
        }
    }
}

/// Benchmark trait
pub trait Benchmark {
    fn name(&self) -> &str;
    fn setup(&mut self);
    fn teardown(&mut self);
    fn run(&mut self) -> Duration;
}

/// Benchmark runner
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            config: BenchmarkConfig::default(),
        }
    }

    pub fn with_config(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Run a benchmark and return results
    pub fn run(&self, benchmark: &mut dyn Benchmark) -> BenchmarkResult {
        let name = benchmark.name().to_string();

        for _ in 0..self.config.warmup_iterations {
            benchmark.setup();
            benchmark.run();
            benchmark.teardown();
        }

        let mut timings = Vec::with_capacity(self.config.sample_size as usize);

        for _ in 0..self.config.sample_size {
            benchmark.setup();
            let elapsed = benchmark.run();
            timings.push(elapsed);
            benchmark.teardown();
        }

        BenchmarkResult::new(name, self.config.sample_size, timings)
    }

    /// Run multiple benchmarks
    pub fn run_all(&self, benchmarks: &mut [&mut dyn Benchmark]) -> Vec<BenchmarkResult> {
        benchmarks
            .iter()
            .map(|b| self.run(*b))
            .collect()
    }
}

/// Macro-benchmark trait
pub trait MacroBenchmark {
    fn name(&self) -> &str;
    fn setup(&mut self);
    fn run(&mut self);
    fn teardown(&mut self);
    fn measure(&self) -> BenchmarkMetrics;
}

/// Benchmark metrics
#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    pub cpu_cycles: u64,
    pub cache_misses: u64,
    pub instructions: u64,
    pub memory_used: usize,
}

/// Timer for benchmarking
pub struct Timer {
    start: u64,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Self::read_cycles(),
        }
    }

    fn read_cycles() -> u64 {
        crate::arch::x86_64::time::rdtsc()
    }

    pub fn elapsed_cycles(&self) -> u64 {
        Self::read_cycles() - self.start
    }

    pub fn elapsed_nanos(&self) -> u64 {
        let cycles = self.elapsed_cycles();
        let freq = crate::subsystems::time::cpu_frequency() as f64;
        (cycles as f64 / freq * 1e9) as u64
    }

    pub fn elapsed(&self) -> Duration {
        Duration::from_nanos(self.elapsed_nanos())
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

/// Regression detection
pub struct RegressionDetector {
    baseline: Vec<BenchmarkResult>,
    tolerance: f64,
}

impl RegressionDetector {
    pub fn new(tolerance: f64) -> Self {
        Self {
            baseline: Vec::new(),
            tolerance,
        }
    }

    pub fn set_baseline(&mut self, results: Vec<BenchmarkResult>) {
        self.baseline = results;
    }

    pub fn check_regression(&self, results: &[BenchmarkResult]) -> Vec<RegressionReport> {
        let mut reports = Vec::new();

        for result in results {
            if let Some(baseline) = self.baseline.iter().find(|b| b.name == result.name) {
                let ratio = result.avg_time.as_nanos() as f64 / baseline.avg_time.as_nanos() as f64;

                if ratio > (1.0 + self.tolerance) {
                    reports.push(RegressionReport {
                        name: result.name.clone(),
                        baseline_avg: baseline.avg_time,
                        current_avg: result.avg_time,
                        regression_ratio: ratio,
                        severity: if ratio > 2.0 {
                            RegressionSeverity::Critical
                        } else if ratio > 1.5 {
                            RegressionSeverity::High
                        } else {
                            RegressionSeverity::Medium
                        },
                    });
                }
            }
        }

        reports
    }
}

/// Regression report
#[derive(Debug, Clone)]
pub struct RegressionReport {
    pub name: String,
    pub baseline_avg: Duration,
    pub current_avg: Duration,
    pub regression_ratio: f64,
    pub severity: RegressionSeverity,
}

/// Regression severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result() {
        let timings = vec![
            Duration::from_nanos(100),
            Duration::from_nanos(200),
            Duration::from_nanos(150),
        ];

        let result = BenchmarkResult::new("test".to_string(), 3, timings);
        assert_eq!(result.name, "test");
        assert_eq!(result.iterations, 3);
        assert_eq!(result.min_time, Duration::from_nanos(100));
        assert_eq!(result.max_time, Duration::from_nanos(200));
    }

    #[test]
    fn test_regression_detector() {
        let mut detector = RegressionDetector::new(0.1);

        let baseline = vec![BenchmarkResult {
            name: "test".to_string(),
            iterations: 100,
            total_time: Duration::from_nanos(10000),
            avg_time: Duration::from_nanos(100),
            min_time: Duration::from_nanos(90),
            max_time: Duration::from_nanos(110),
            ops_per_sec: 10000000.0,
        }];
        detector.set_baseline(baseline);

        let current = vec![BenchmarkResult {
            name: "test".to_string(),
            iterations: 100,
            total_time: Duration::from_nanos(12000),
            avg_time: Duration::from_nanos(120),
            min_time: Duration::from_nanos(110),
            max_time: Duration::from_nanos(130),
            ops_per_sec: 8333333.33,
        }];

        let regressions = detector.check_regression(&current);
        assert!(!regressions.is_empty());
        assert_eq!(regressions[0].name, "test");
    }
}
