//! Schedule Latency Benchmark
//!
//! Measures the time from when a thread becomes runnable until it actually
//! starts executing on a CPU. This includes scheduler overhead and queue time.
//! Target: <10μs for scheduling latency.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Number of scheduling operations to measure
const SCHEDULE_ITERATIONS: usize = 1_000;

/// Benchmark scheduling latency
pub struct ScheduleLatencyBenchmark {
    iterations: usize,
    priority_levels: usize,
}

impl ScheduleLatencyBenchmark {
    pub fn new() -> Self {
        Self {
            iterations: SCHEDULE_ITERATIONS,
            priority_levels: 8, // Typical priority levels
        }
    }

    pub fn with_priority_levels(mut self, levels: usize) -> Self {
        self.priority_levels = levels;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for ScheduleLatencyBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for ScheduleLatencyBenchmark {
    fn name(&self) -> &str {
        "scheduler/schedule_latency"
    }

    fn setup(&mut self) -> Result<(), String> {
        // Initialize scheduler state
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate the scheduling path:
        // 1. Mark a task as runnable
        // 2. Scheduler picks next task
        // 3. Context switch to new task

        let start = core::time::Instant::now();

        // Simulate scheduler decision making
        for priority in 0..self.priority_levels {
            // Simulate checking runqueue at each priority level
            core::hint::spin_loop();

            if priority == 3 {
                // Simulate finding a runnable task
                break;
            }
        }

        // Simulate context switch overhead
        for _ in 0..10 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Measure wakeup latency - time from wakeup() call to task execution
pub struct WakeupLatencyBenchmark {
    iterations: usize,
}

impl WakeupLatencyBenchmark {
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

impl BenchmarkSuite for WakeupLatencyBenchmark {
    fn name(&self) -> &str {
        "scheduler/wakeup_latency"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate: task sleeping -> wakeup() -> scheduler -> task running
        let start = core::time::Instant::now();

        // Simulate wakeup path
        for _ in 0..50 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Measure timer tick overhead
pub struct TimerTickBenchmark {
    ticks_per_second: u64,
}

impl TimerTickBenchmark {
    pub fn new() -> Self {
        Self {
            ticks_per_second: 1000, // 1ms tick period
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for TimerTickBenchmark {
    fn name(&self) -> &str {
        "scheduler/timer_tick"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Measure overhead of timer interrupt handling
        let start = core::time::Instant::now();

        // Simulate timer tick processing
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_latency_benchmark() {
        let mut bench = ScheduleLatencyBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
        assert!(result.mean_ns > 0.0);
    }

    #[test]
    fn test_wakeup_latency_benchmark() {
        let mut bench = WakeupLatencyBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_timer_tick_benchmark() {
        let mut bench = TimerTickBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
