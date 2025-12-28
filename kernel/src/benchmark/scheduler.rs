//! Scheduler performance benchmarks
//!
//! Measures scheduler latency and context switch performance.

extern crate alloc;

use alloc::vec::Vec;
use crate::subsystems::time::hrtime_nanos;

/// Scheduler benchmark results
#[derive(Debug, Clone)]
pub struct SchedulerBenchmarkResult {
    /// Benchmark name
    pub name: &'static str,
    /// Number of context switches
    pub num_switches: usize,
    /// Total time (nanoseconds)
    pub total_time_ns: u64,
    /// Average context switch time (nanoseconds)
    pub avg_switch_time_ns: u64,
    /// Minimum switch time (nanoseconds)
    pub min_switch_time_ns: u64,
    /// Maximum switch time (nanoseconds)
    pub max_switch_time_ns: u64,
}

/// Benchmark context switch performance
pub fn benchmark_context_switch(count: usize) -> SchedulerBenchmarkResult {
    let mut switch_times = Vec::with_capacity(count);
    
    let start_time = hrtime_nanos();
    
    for _ in 0..count {
        let switch_start = hrtime_nanos();
        // Simulate context switch
        // In real implementation, this would trigger actual context switch
        crate::process::yield_cpu();
        let switch_end = hrtime_nanos();
        switch_times.push(switch_end - switch_start);
    }
    
    let end_time = hrtime_nanos();
    let total_time = end_time - start_time;
    
    switch_times.sort();
    let avg_time = switch_times.iter().sum::<u64>() / switch_times.len() as u64;
    let min_time = switch_times[0];
    let max_time = switch_times[switch_times.len() - 1];
    
    SchedulerBenchmarkResult {
        name: "context_switch",
        num_switches: count,
        total_time_ns: total_time,
        avg_switch_time_ns: avg_time,
        min_switch_time_ns: min_time,
        max_switch_time_ns: max_time,
    }
}

/// Run all scheduler benchmarks
pub fn run_all_scheduler_benchmarks() {
    crate::println!("[benchmark] Running scheduler benchmarks...");
    
    let result = benchmark_context_switch(1000);
    crate::println!("[benchmark] Context switch (1000x):");
    crate::println!("  Average time: {} ns ({:.2} us)", result.avg_switch_time_ns, result.avg_switch_time_ns as f64 / 1000.0);
    crate::println!("  Min time: {} ns", result.min_switch_time_ns);
    crate::println!("  Max time: {} ns", result.max_switch_time_ns);
    
    crate::println!("[benchmark] Scheduler benchmarks completed");
}

