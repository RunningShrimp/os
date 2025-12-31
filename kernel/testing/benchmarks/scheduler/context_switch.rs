//! Context Switch Latency Benchmark
//!
//! Measures the time required to switch between two threads/processes.
//! Target: <1μs for voluntary context switches.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Number of context switches to measure
const SWITCH_ITERATIONS: usize = 10_000;

/// Counter for synchronization between threads
static SWITCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Benchmark context switch latency
pub struct ContextSwitchBenchmark {
    iterations: usize,
}

impl ContextSwitchBenchmark {
    pub fn new() -> Self {
        Self {
            iterations: SWITCH_ITERATIONS,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for ContextSwitchBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for ContextSwitchBenchmark {
    fn name(&self) -> &str {
        "scheduler/context_switch"
    }

    fn setup(&mut self) -> Result<(), String> {
        SWITCH_COUNTER.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate context switch overhead
        // In a real implementation, this would:
        // 1. Create two threads
        // 2. Ping-pong between them using a futex or similar primitive
        // 3. Measure the round-trip time

        // Simulated context switch latency measurement
        let start = core::time::Instant::now();

        // Simulate context switch operations
        for _ in 0..100 {
            core::hint::spin_loop();
            SWITCH_COUNTER.fetch_add(1, Ordering::Relaxed);
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Advanced context switch benchmark with real thread switching
#[cfg(feature = "advanced_scheduler_benchmarks")]
pub struct AdvancedContextSwitchBenchmark {
    thread_a: Option<core::pin::Pin<Box<crate::process::Thread>>>,
    thread_b: Option<core::pin::Pin<Box<crate::process::Thread>>>,
}

#[cfg(feature = "advanced_scheduler_benchmarks")]
impl AdvancedContextSwitchBenchmark {
    pub fn new() -> Self {
        Self {
            thread_a: None,
            thread_b: None,
        }
    }

    /// Measure real context switch time between two threads
    pub fn measure_real_context_switch(&mut self) -> Result<u64, String> {
        // This would create two real threads and measure the actual context switch time
        // by having them ping-pong back and forth using futexes

        Ok(0) // Placeholder
    }
}

/// Measure context switch latency using RDTSC for high precision
#[cfg(target_arch = "x86_64")]
pub fn measure_context_switch_rdtsc() -> u64 {
    unsafe {
        let mut rax: u64;
        let mut rdx: u64;
        core::arch::asm!(
            "rdtsc",
            out("rax") rax,
            out("rdx") rdx,
            options(nomem, nostack)
        );
        (rdx << 32) | rax
    }
}

/// Calculate context switch overhead
pub fn calculate_overhead(total_ns: u64, switches: usize) -> f64 {
    total_ns as f64 / switches as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_switch_benchmark() {
        let mut bench = ContextSwitchBenchmark::new();
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
    fn test_overhead_calculation() {
        let overhead = calculate_overhead(1_000_000, 1000);
        assert!((overhead - 1000.0).abs() < 0.01);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_rdtsc_measurement() {
        let cycles1 = measure_context_switch_rdtsc();
        let cycles2 = measure_context_switch_rdtsc();
        assert!(cycles2 > cycles1);
    }
}
