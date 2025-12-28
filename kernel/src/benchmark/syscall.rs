//! System call performance benchmarks
//!
//! Measures system call latency and throughput for production validation.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::subsystems::time::hrtime_nanos;

/// System call benchmark results
#[derive(Debug, Clone)]
pub struct SyscallBenchmarkResult {
    /// System call name
    pub syscall_name: &'static str,
    /// Number of iterations
    pub iterations: usize,
    /// Total time (nanoseconds)
    pub total_time_ns: u64,
    /// Average latency (nanoseconds)
    pub avg_latency_ns: u64,
    /// Minimum latency (nanoseconds)
    pub min_latency_ns: u64,
    /// Maximum latency (nanoseconds)
    pub max_latency_ns: u64,
    /// 50th percentile latency (nanoseconds)
    pub p50_latency_ns: u64,
    /// 95th percentile latency (nanoseconds)
    pub p95_latency_ns: u64,
    /// 99th percentile latency (nanoseconds)
    pub p99_latency_ns: u64,
    /// Throughput (operations per second)
    pub throughput_ops_per_sec: f64,
}

/// Benchmark a system call
pub fn benchmark_syscall<F>(name: &'static str, iterations: usize, syscall_fn: F) -> SyscallBenchmarkResult
where
    F: Fn() -> Result<u64, i32>,
{
    let mut latencies = Vec::with_capacity(iterations);
    let mut total_time = 0u64;
    
    // Warmup
    for _ in 0..10 {
        let _ = syscall_fn();
    }
    
    // Benchmark
    let start_time = hrtime_nanos();
    for _ in 0..iterations {
        let call_start = hrtime_nanos();
        let _ = syscall_fn();
        let call_end = hrtime_nanos();
        let latency = call_end - call_start;
        latencies.push(latency);
        total_time += latency;
    }
    let end_time = hrtime_nanos();
    
    // Calculate statistics
    latencies.sort();
    let avg_latency = total_time / iterations as u64;
    let min_latency = latencies[0];
    let max_latency = latencies[iterations - 1];
    let p50_idx = iterations / 2;
    let p95_idx = (iterations as f64 * 0.95) as usize;
    let p99_idx = (iterations as f64 * 0.99) as usize;
    
    let p50_latency = latencies[p50_idx.min(iterations - 1)];
    let p95_latency = latencies[p95_idx.min(iterations - 1)];
    let p99_latency = latencies[p99_idx.min(iterations - 1)];
    
    let elapsed_ns = end_time - start_time;
    let throughput = (iterations as f64 * 1_000_000_000.0) / elapsed_ns as f64;
    
    SyscallBenchmarkResult {
        syscall_name: name,
        iterations,
        total_time_ns: elapsed_ns,
        avg_latency_ns: avg_latency,
        min_latency_ns: min_latency,
        max_latency_ns: max_latency,
        p50_latency_ns: p50_latency,
        p95_latency_ns: p95_latency,
        p99_latency_ns: p99_latency,
        throughput_ops_per_sec: throughput,
    }
}

/// Print benchmark results
pub fn print_benchmark_result(result: &SyscallBenchmarkResult) {
    crate::println!("[benchmark] {}:", result.syscall_name);
    crate::println!("  Iterations: {}", result.iterations);
    crate::println!("  Average latency: {} ns ({:.2} us)", result.avg_latency_ns, result.avg_latency_ns as f64 / 1000.0);
    crate::println!("  Min latency: {} ns", result.min_latency_ns);
    crate::println!("  Max latency: {} ns", result.max_latency_ns);
    crate::println!("  P50 latency: {} ns", result.p50_latency_ns);
    crate::println!("  P95 latency: {} ns", result.p95_latency_ns);
    crate::println!("  P99 latency: {} ns", result.p99_latency_ns);
    crate::println!("  Throughput: {:.2} ops/sec", result.throughput_ops_per_sec);
}

/// Run all system call benchmarks
pub fn run_all_syscall_benchmarks() {
    crate::println!("[benchmark] Running system call benchmarks...");
    
    // Benchmark getpid
    let getpid_result = benchmark_syscall("getpid", 10000, || {
        let pid = crate::process::myproc();
        Ok(pid.unwrap_or(0) as u64)
    });
    print_benchmark_result(&getpid_result);
    
    // Benchmark gettid
    let gettid_result = benchmark_syscall("gettid", 10000, || {
        let tid = crate::process::thread::thread_self();
        Ok(tid as u64)
    });
    print_benchmark_result(&gettid_result);
    
    crate::println!("[benchmark] System call benchmarks completed");
}

