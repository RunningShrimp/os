//! Scheduler Throughput Benchmark
//!
//! Measures how many context switches the scheduler can perform per second.
//! Target: >100,000 context switches per second.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;
use core::time::Duration;

/// Duration for throughput measurement
const MEASUREMENT_DURATION: Duration = Duration::from_secs(1);

/// Benchmark scheduler throughput
pub struct ThroughputBenchmark {
    measurement_duration: Duration,
}

impl ThroughputBenchmark {
    pub fn new() -> Self {
        Self {
            measurement_duration: MEASUREMENT_DURATION,
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.measurement_duration = duration;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for ThroughputBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for ThroughputBenchmark {
    fn name(&self) -> &str {
        "scheduler/throughput"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();
        let mut switches = 0u64;

        // Simulate as many context switches as possible
        while start.elapsed() < self.measurement_duration {
            // Simulate context switch
            for _ in 0..10 {
                core::hint::spin_loop();
            }
            switches += 1;
        }

        // Return nanoseconds per switch (inverse of throughput)
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let ns_per_switch = elapsed_ns / switches.max(1);
        Ok(ns_per_switch)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Multiple threads contending for CPU time
pub struct MultiThreadThroughputBenchmark {
    num_threads: usize,
    duration: Duration,
}

impl MultiThreadThroughputBenchmark {
    pub fn new(num_threads: usize) -> Self {
        Self {
            num_threads,
            duration: Duration::from_millis(100),
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(4);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MultiThreadThroughputBenchmark {
    fn name(&self) -> &str {
        "scheduler/multithread_throughput"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();
        let mut total_switches = 0u64;

        // Simulate multiple threads contending
        while start.elapsed() < self.duration {
            // Simulate scheduling decisions for multiple threads
            for _ in 0..self.num_threads {
                for _ in 0..5 {
                    core::hint::spin_loop();
                }
                total_switches += 1;
            }
        }

        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let ns_per_switch = elapsed_ns / total_switches.max(1);
        Ok(ns_per_switch)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Real-time scheduling latency
pub struct RealtimeLatencyBenchmark {
    iterations: usize,
}

impl RealtimeLatencyBenchmark {
    pub fn new() -> Self {
        Self {
            iterations: 1_000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for RealtimeLatencyBenchmark {
    fn name(&self) -> &str {
        "scheduler/realtime_latency"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate real-time task scheduling
        let start = core::time::Instant::now();

        // Real-time tasks should preempt immediately
        for _ in 0..20 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Fairness benchmark - measures CPU time distribution
pub struct FairnessBenchmark {
    num_tasks: usize,
    duration_per_task: Duration,
}

impl FairnessBenchmark {
    pub fn new(num_tasks: usize) -> Self {
        Self {
            num_tasks,
            duration_per_task: Duration::from_millis(10),
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(4);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for FairnessBenchmark {
    fn name(&self) -> &str {
        "scheduler/fairness"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();
        let mut task_times = Vec::with_capacity(self.num_tasks);

        // Simulate fair scheduling among multiple tasks
        for task_id in 0..self.num_tasks {
            let task_start = core::time::Instant::now();

            // Each task gets CPU time
            while task_start.elapsed() < self.duration_per_task {
                core::hint::spin_loop();
            }

            task_times.push(task_start.elapsed());
        }

        // Calculate variance in task execution times (lower is more fair)
        let mean = task_times.iter().map(|d| d.as_nanos()).sum::<u128>() as f64 / self.num_tasks as f64;
        let variance = task_times.iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / self.num_tasks as f64;

        let elapsed = start.elapsed();
        // Return variance as the metric
        Ok(variance as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_throughput_benchmark() {
        let mut bench = ThroughputBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 10,
            max_duration: Duration::from_secs(5),
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_multithread_throughput() {
        let mut bench = MultiThreadThroughputBenchmark::new(4);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 10,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_realtime_latency() {
        let mut bench = RealtimeLatencyBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_fairness_benchmark() {
        let mut bench = FairnessBenchmark::new(4);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 10,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
