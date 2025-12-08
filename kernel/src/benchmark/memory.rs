//! Memory performance benchmarks
//!
//! Measures memory allocation and access performance.

extern crate alloc;

use alloc::vec::Vec;
use crate::time::hrtime_nanos;

/// Memory benchmark results
#[derive(Debug, Clone)]
pub struct MemoryBenchmarkResult {
    /// Benchmark name
    pub name: &'static str,
    /// Allocation size (bytes)
    pub allocation_size: usize,
    /// Number of allocations
    pub num_allocations: usize,
    /// Total time (nanoseconds)
    pub total_time_ns: u64,
    /// Average allocation time (nanoseconds)
    pub avg_allocation_time_ns: u64,
    /// Throughput (MB/s)
    pub throughput_mb_per_sec: f64,
}

/// Benchmark memory allocation
pub fn benchmark_allocation(size: usize, count: usize) -> MemoryBenchmarkResult {
    let start_time = hrtime_nanos();
    
    let mut allocations = Vec::with_capacity(count);
    for _ in 0..count {
        let alloc_start = hrtime_nanos();
        let ptr = crate::mm::kalloc(size);
        let alloc_end = hrtime_nanos();
        
        if let Some(addr) = ptr {
            allocations.push((addr, alloc_end - alloc_start));
        }
    }
    
    // Free allocations
    for (addr, _) in &allocations {
        unsafe {
            crate::mm::kfree(*addr, size);
        }
    }
    
    let end_time = hrtime_nanos();
    let total_time = end_time - start_time;
    let avg_time = allocations.iter().map(|(_, t)| *t).sum::<u64>() / allocations.len() as u64;
    let total_bytes = (allocations.len() * size) as u64;
    let throughput = (total_bytes as f64 * 1_000_000_000.0) / total_time as f64 / (1024.0 * 1024.0);
    
    MemoryBenchmarkResult {
        name: "allocation",
        allocation_size: size,
        num_allocations: allocations.len(),
        total_time_ns: total_time,
        avg_allocation_time_ns: avg_time,
        throughput_mb_per_sec: throughput,
    }
}

/// Benchmark memory access patterns
pub fn benchmark_memory_access(size: usize, iterations: usize) -> MemoryBenchmarkResult {
    // Allocate buffer
    let buf = crate::mm::kalloc(size).expect("Failed to allocate");
    
    let start_time = hrtime_nanos();
    
    // Sequential write
    for i in 0..iterations {
        unsafe {
            let ptr = (buf + (i % size)) as *mut u8;
            *ptr = (i & 0xFF) as u8;
        }
    }
    
    // Sequential read
    let mut sum = 0u64;
    for i in 0..iterations {
        unsafe {
            let ptr = (buf + (i % size)) as *const u8;
            sum += *ptr as u64;
        }
    }
    
    let end_time = hrtime_nanos();
    let total_time = end_time - start_time;
    
    // Free buffer
    unsafe {
        crate::mm::kfree(buf, size);
    }
    
    let total_bytes = (iterations * core::mem::size_of::<u8>()) as u64;
    let throughput = (total_bytes as f64 * 1_000_000_000.0) / total_time as f64 / (1024.0 * 1024.0);
    
    MemoryBenchmarkResult {
        name: "memory_access",
        allocation_size: size,
        num_allocations: iterations,
        total_time_ns: total_time,
        avg_allocation_time_ns: total_time / iterations as u64,
        throughput_mb_per_sec: throughput,
    }
}

/// Run all memory benchmarks
pub fn run_all_memory_benchmarks() {
    crate::println!("[benchmark] Running memory benchmarks...");
    
    // Small allocation benchmark
    let small_result = benchmark_allocation(64, 1000);
    crate::println!("[benchmark] Small allocations (64B, 1000x):");
    crate::println!("  Average time: {} ns", small_result.avg_allocation_time_ns);
    crate::println!("  Throughput: {:.2} MB/s", small_result.throughput_mb_per_sec);
    
    // Large allocation benchmark
    let large_result = benchmark_allocation(4096, 100);
    crate::println!("[benchmark] Large allocations (4KB, 100x):");
    crate::println!("  Average time: {} ns", large_result.avg_allocation_time_ns);
    crate::println!("  Throughput: {:.2} MB/s", large_result.throughput_mb_per_sec);
    
    // Memory access benchmark
    let access_result = benchmark_memory_access(4096, 10000);
    crate::println!("[benchmark] Memory access (4KB buffer, 10000 iterations):");
    crate::println!("  Average time: {} ns", access_result.avg_allocation_time_ns);
    crate::println!("  Throughput: {:.2} MB/s", access_result.throughput_mb_per_sec);
    
    crate::println!("[benchmark] Memory benchmarks completed");
}

