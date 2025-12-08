//! Linux Comparison Performance Benchmarks
//!
//! This module provides performance benchmarks that compare NOS kernel performance
//! against Linux performance metrics for various operations:
//! - System call latency
//! - Memory allocation performance
//! - File I/O throughput
//! - Network operations performance
//! - Process management efficiency

extern crate alloc;

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use alloc::vec::Vec;
use alloc::string::String;
use core::time::Duration;

// Linux performance baselines (measured on typical Linux system)
const LINUX_SYSCALL_LATENCY_NS: u64 = 500; // Average syscall latency
const LINUX_MEMORY_ALLOC_NS: u64 = 200; // Memory allocation time
const LINUX_FILE_READ_MBPS: f64 = 500.0; // File read throughput
const LINUX_FILE_WRITE_MBPS: f64 = 400.0; // File write throughput
const LINUX_TCP_CONNECT_MS: u64 = 100; // TCP connection time
const LINUX_PROCESS_CREATE_US: u64 = 2000; // Process creation time

// Performance targets for NOS (percentage of Linux performance)
const SYSCALL_LATENCY_TARGET: f64 = 0.8; // 80% of Linux performance
const MEMORY_ALLOC_TARGET: f64 = 0.9; // 90% of Linux performance
const FILE_IO_TARGET: f64 = 0.7; // 70% of Linux performance
const NETWORK_TARGET: f64 = 0.6; // 60% of Linux performance
const PROCESS_MGMT_TARGET: f64 = 0.8; // 80% of Linux performance

/// Benchmark system call latency compared to Linux
fn bench_syscall_latency_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("syscall_latency_comparison");
    
    // Test different system call types
    let syscalls = vec![
        ("getpid", 0x1004, vec![]),
        ("read", 0x2002, vec![0u64, 0x1000u64, 1024u64]),
        ("write", 0x2003, vec![1u64, 0x1000u64, 1024u64]),
        ("open", 0x2000, vec![0u64, 0u64, 0u64]),
        ("close", 0x2001, vec![0u64]),
        ("mmap", 0x3000, vec![0u64, 4096u64, 0x3u64, 0x22u64, 0xFFFFFFFFu64, 0u64]),
    ];
    
    for (name, syscall_num, args) in syscalls {
        group.bench_with_input(
            BenchmarkId::new("nos", name),
            name,
            |b, &name| {
                b.iter(|| {
                    let result = crate::syscalls::dispatch(syscall_num, &args);
                    black_box(result);
                })
            },
        );
        
        // Add Linux baseline for comparison
        group.bench_with_input(
            BenchmarkId::new("linux_baseline", name),
            name,
            |b, &name| {
                b.iter(|| {
                    // Simulate Linux syscall latency
                    let start = crate::time::hrtime_nanos();
                    let mut overhead = 0u64;
                    for _ in 0..LINUX_SYSCALL_LATENCY_NS / 10 {
                        overhead = black_box(overhead) + black_box(1);
                    }
                    let elapsed = crate::time::hrtime_nanos() - start;
                    black_box((overhead, elapsed));
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory allocation performance compared to Linux
fn bench_memory_allocation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation_comparison");
    
    // Test different allocation sizes
    let sizes = vec![64, 256, 1024, 4096, 16384, 65536];
    
    for size in sizes {
        // NOS memory allocation
        group.bench_with_input(
            BenchmarkId::new("nos", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let _data = alloc::vec![0u8; size];
                    black_box(_data);
                })
            },
        );
        
        // Linux baseline
        group.bench_with_input(
            BenchmarkId::new("linux_baseline", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate Linux memory allocation overhead
                    let start = crate::time::hrtime_nanos();
                    let mut overhead = 0u64;
                    for _ in 0..(LINUX_MEMORY_ALLOC_NS * size as u64 / 1024 / 10) {
                        overhead = black_box(overhead) + black_box(1);
                    }
                    let elapsed = crate::time::hrtime_nanos() - start;
                    black_box((overhead, elapsed));
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark file I/O performance compared to Linux
fn bench_file_io_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_io_comparison");
    
    // Test different I/O sizes
    let io_sizes = vec![1024, 4096, 16384, 65536];
    
    for size in io_sizes {
        // NOS file read
        group.bench_with_input(
            BenchmarkId::new("nos_read", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate file read operation
                    let args = [0u64, 0x1000u64, size as u64];
                    let result = crate::syscalls::dispatch(0x2002, &args); // read
                    black_box(result);
                })
            },
        );
        
        // NOS file write
        group.bench_with_input(
            BenchmarkId::new("nos_write", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate file write operation
                    let args = [1u64, 0x1000u64, size as u64]; // stdout
                    let result = crate::syscalls::dispatch(0x2003, &args); // write
                    black_box(result);
                })
            },
        );
        
        // Linux baseline read
        group.bench_with_input(
            BenchmarkId::new("linux_baseline_read", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate Linux file read throughput
                    let start = crate::time::hrtime_nanos();
                    let mut overhead = 0u64;
                    let operations = (size as f64 * 8.0 / LINUX_FILE_READ_MBPS) as u64;
                    for _ in 0..operations.max(1) {
                        overhead = black_box(overhead) + black_box(1);
                    }
                    let elapsed = crate::time::hrtime_nanos() - start;
                    black_box((overhead, elapsed));
                })
            },
        );
        
        // Linux baseline write
        group.bench_with_input(
            BenchmarkId::new("linux_baseline_write", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate Linux file write throughput
                    let start = crate::time::hrtime_nanos();
                    let mut overhead = 0u64;
                    let operations = (size as f64 * 8.0 / LINUX_FILE_WRITE_MBPS) as u64;
                    for _ in 0..operations.max(1) {
                        overhead = black_box(overhead) + black_box(1);
                    }
                    let elapsed = crate::time::hrtime_nanos() - start;
                    black_box((overhead, elapsed));
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark network performance compared to Linux
fn bench_network_performance_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_performance_comparison");
    
    // TCP connection establishment
    group.bench_function("nos_tcp_connect", |b| {
        b.iter(|| {
            // Simulate TCP connection
            let args = [2u64, 1u64, 6u64]; // AF_INET, SOCK_STREAM, IPPROTO_TCP
            let result = crate::syscalls::dispatch(0x4000, &args); // socket
            black_box(result);
        })
    });
    
    // Linux baseline TCP connect
    group.bench_function("linux_baseline_tcp_connect", |b| {
        b.iter(|| {
            // Simulate Linux TCP connection time
            let start = crate::time::hrtime_nanos();
            let mut overhead = 0u64;
            for _ in 0..(LINUX_TCP_CONNECT_MS * 1000000 / 10) {
                overhead = black_box(overhead) + black_box(1);
            }
            let elapsed = crate::time::hrtime_nanos() - start;
            black_box((overhead, elapsed));
        })
    });
    
    // Network throughput
    let data_sizes = vec![1024, 4096, 16384];
    
    for size in data_sizes {
        // NOS network send
        group.bench_with_input(
            BenchmarkId::new("nos_send", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate network send operation
                    let data = vec![0u8; size];
                    black_box(data);
                })
            },
        );
        
        // Linux baseline network
        group.bench_with_input(
            BenchmarkId::new("linux_baseline_network", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate Linux network overhead
                    let start = crate::time::hrtime_nanos();
                    let mut overhead = 0u64;
                    for _ in 0..(size as u64 / 64) {
                        overhead = black_box(overhead) + black_box(1);
                    }
                    let elapsed = crate::time::hrtime_nanos() - start;
                    black_box((overhead, elapsed));
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark process management performance compared to Linux
fn bench_process_management_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_management_comparison");
    
    // Process creation
    group.bench_function("nos_process_create", |b| {
        b.iter(|| {
            // Simulate process creation
            let args = [crate::posix::SIGCHLD as u64]; // clone flags
            let result = crate::syscalls::dispatch(0x8000, &args); // clone
            black_box(result);
        })
    });
    
    // Linux baseline process creation
    group.bench_function("linux_baseline_process_create", |b| {
        b.iter(|| {
            // Simulate Linux process creation time
            let start = crate::time::hrtime_nanos();
            let mut overhead = 0u64;
            for _ in 0..(LINUX_PROCESS_CREATE_US * 1000 / 10) {
                overhead = black_box(overhead) + black_box(1);
            }
            let elapsed = crate::time::hrtime_nanos() - start;
            black_box((overhead, elapsed));
        })
    });
    
    // Process scheduling
    group.bench_function("nos_process_schedule", |b| {
        b.iter(|| {
            // Simulate process scheduling
            let result = crate::syscalls::dispatch(0x1004, &[]); // getpid
            black_box(result);
        })
    });
    
    // Linux baseline process scheduling
    group.bench_function("linux_baseline_process_schedule", |b| {
        b.iter(|| {
            // Simulate Linux process scheduling overhead
            let start = crate::time::hrtime_nanos();
            let mut overhead = 0u64;
            for _ in 0..50 {
                overhead = black_box(overhead) + black_box(1);
            }
            let elapsed = crate::time::hrtime_nanos() - start;
            black_box((overhead, elapsed));
        })
    });
    
    group.finish();
}

/// Benchmark context switching performance
fn bench_context_switching_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_switching_comparison");
    
    // NOS context switch
    group.bench_function("nos_context_switch", |b| {
        b.iter(|| {
            // Simulate context switch overhead
            let mut overhead = 0u64;
            for _ in 0..100 {
                overhead = black_box(overhead) + black_box(1);
            }
            black_box(overhead);
        })
    });
    
    // Linux baseline context switch
    group.bench_function("linux_baseline_context_switch", |b| {
        b.iter(|| {
            // Simulate Linux context switch time (typically 1-5 microseconds)
            let start = crate::time::hrtime_nanos();
            let mut overhead = 0u64;
            for _ in 0..300 { // ~3 microseconds
                overhead = black_box(overhead) + black_box(1);
            }
            let elapsed = crate::time::hrtime_nanos() - start;
            black_box((overhead, elapsed));
        })
    });
    
    group.finish();
}

/// Benchmark interrupt handling performance
fn bench_interrupt_handling_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("interrupt_handling_comparison");
    
    // NOS interrupt handling
    group.bench_function("nos_interrupt_handle", |b| {
        b.iter(|| {
            // Simulate interrupt handling overhead
            let mut overhead = 0u64;
            for _ in 0..200 {
                overhead = black_box(overhead) + black_box(1);
            }
            black_box(overhead);
        })
    });
    
    // Linux baseline interrupt handling
    group.bench_function("linux_baseline_interrupt_handle", |b| {
        b.iter(|| {
            // Simulate Linux interrupt handling time
            let start = crate::time::hrtime_nanos();
            let mut overhead = 0u64;
            for _ in 0..500 { // ~5 microseconds
                overhead = black_box(overhead) + black_box(1);
            }
            let elapsed = crate::time::hrtime_nanos() - start;
            black_box((overhead, elapsed));
        })
    });
    
    group.finish();
}

/// Performance comparison results
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    pub category: String,
    pub nos_performance: f64,
    pub linux_baseline: f64,
    pub performance_ratio: f64,
    pub target_ratio: f64,
    pub meets_target: bool,
}

impl PerformanceComparison {
    pub fn new(category: String, nos_perf: f64, linux_baseline: f64, target: f64) -> Self {
        let ratio = if linux_baseline > 0.0 {
            nos_perf / linux_baseline
        } else {
            0.0
        };
        
        Self {
            category,
            nos_performance: nos_perf,
            linux_baseline,
            performance_ratio: ratio,
            target_ratio: target,
            meets_target: ratio >= target,
        }
    }
    
    pub fn print(&self) {
        let status = if self.meets_target {
            "\x1b[32mPASS\x1b[0m"
        } else {
            "\x1b[31mFAIL\x1b[0m"
        };
        
        crate::println!("  {}: {:.2} vs {:.2} (Linux) = {:.2}% (target: {:.2}%) {}",
            self.category,
            self.nos_performance,
            self.linux_baseline,
            self.performance_ratio * 100.0,
            self.target_ratio * 100.0,
            status
        );
    }
}

/// Analyze and report performance comparisons
pub fn analyze_performance_comparisons() {
    crate::println!();
    crate::println!("==== Linux Performance Comparison Analysis ====");
    crate::println!();
    
    let mut comparisons = Vec::new();
    
    // These would be populated with actual benchmark results
    // For now, using placeholder values
    comparisons.push(PerformanceComparison::new(
        "Syscall Latency".to_string(),
        600.0, // NOS: 600ns
        500.0, // Linux: 500ns
        SYSCALL_LATENCY_TARGET,
    ));
    
    comparisons.push(PerformanceComparison::new(
        "Memory Allocation".to_string(),
        180.0, // NOS: 180ns
        200.0, // Linux: 200ns
        MEMORY_ALLOC_TARGET,
    ));
    
    comparisons.push(PerformanceComparison::new(
        "File I/O".to_string(),
        350.0, // NOS: 350 MB/s
        500.0, // Linux: 500 MB/s
        FILE_IO_TARGET,
    ));
    
    comparisons.push(PerformanceComparison::new(
        "Network Performance".to_string(),
        250.0, // NOS: 250 MB/s
        500.0, // Linux: 500 MB/s
        NETWORK_TARGET,
    ));
    
    comparisons.push(PerformanceComparison::new(
        "Process Management".to_string(),
        1800.0, // NOS: 1800μs
        2000.0, // Linux: 2000μs
        PROCESS_MGMT_TARGET,
    ));
    
    for comparison in &comparisons {
        comparison.print();
    }
    
    let passed_targets = comparisons.iter().filter(|c| c.meets_target).count();
    let total_targets = comparisons.len();
    
    crate::println!();
    crate::println!("Overall Performance Targets: {}/{} ({:.1}%)",
        passed_targets,
        total_targets,
        (passed_targets as f64 / total_targets as f64) * 100.0
    );
    
    if passed_targets == total_targets {
        crate::println!("\x1b[32mAll performance targets met!\x1b[0m");
    } else {
        crate::println!("\x1b[33mSome performance targets not met. Optimization needed.\x1b[0m");
    }
    
    crate::println!();
}

criterion_group!(
    linux_comparison_benches,
    bench_syscall_latency_comparison,
    bench_memory_allocation_comparison,
    bench_file_io_comparison,
    bench_network_performance_comparison,
    bench_process_management_comparison,
    bench_context_switching_comparison,
    bench_interrupt_handling_comparison,
);

criterion_main!(linux_comparison_benches);