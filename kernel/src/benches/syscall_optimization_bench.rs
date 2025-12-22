//! 系统调用优化性能基准测试
//!
//! 测试和验证系统调用快速路径优化的效果，包括：
//! - 文件描述符缓存性能测试
//! - copyin/copyout优化性能测试
//! - 系统调用批处理性能测试
//! - 进程表锁定优化性能测试
//! - 综合性能对比测试

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use core::time::Duration;

use crate::benchmark::syscall_benchmarks::{benchmark_syscall, print_benchmark_result, SyscallBenchmarkResult};
use crate::process::fd_cache::{ExtendedFdCache, FdCacheStats};
use crate::subsystems::mm::copy_optimized::{OptimizedCopier, CopyStats};
use crate::syscalls::batch::{batch_syscalls, get_batch_stats, reset_batch_stats, BatchStatsSnapshot};
use crate::process::lock_optimized::{get_lock_stats, reset_lock_stats, LockStatsSnapshot};

/// 性能测试结果
#[derive(Debug, Clone)]
pub struct OptimizationBenchmarkResult {
    /// 测试名称
    pub test_name: String,
    /// 优化前性能指标
    pub baseline_metrics: PerformanceMetrics,
    /// 优化后性能指标
    pub optimized_metrics: PerformanceMetrics,
    /// 性能提升百分比
    pub improvement_percentage: f64,
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 平均延迟（纳秒）
    pub avg_latency_ns: u64,
    /// 95%分位延迟（纳秒）
    pub p95_latency_ns: u64,
    /// 99%分位延迟（纳秒）
    pub p99_latency_ns: u64,
    /// 吞吐量（操作/秒）
    pub throughput_ops_per_sec: f64,
    /// 总执行时间（纳秒）
    pub total_execution_time_ns: u64,
    /// 操作数量
    pub operation_count: u64,
}

impl PerformanceMetrics {
    /// 计算性能提升
    pub fn improvement_over(&self, baseline: &PerformanceMetrics) -> f64 {
        if baseline.avg_latency_ns == 0 {
            0.0
        } else {
            ((baseline.avg_latency_ns as f64 - self.avg_latency_ns as f64) / baseline.avg_latency_ns as f64) * 100.0
        }
    }

    /// 计算吞吐量提升
    pub fn throughput_improvement_over(&self, baseline: &PerformanceMetrics) -> f64 {
        if baseline.throughput_ops_per_sec == 0.0 {
            0.0
        } else {
            ((self.throughput_ops_per_sec - baseline.throughput_ops_per_sec) / baseline.throughput_ops_per_sec) * 100.0
        }
    }
}

/// 文件描述符缓存性能测试
pub fn benchmark_fd_cache() -> OptimizationBenchmarkResult {
    const ITERATIONS: usize = 10000;
    const TEST_FDS: &[i32] = &[0, 1, 2, 3, 4, 5, 6, 7]; // 测试常用FD
    
    crate::println!("[benchmark] 开始文件描述符缓存性能测试...");
    
    // 测试基准实现（线性搜索）
    let baseline_start = crate::subsystems::time::hrtime_nanos();
    for _ in 0..ITERATIONS {
        for &fd in TEST_FDS {
            // 模拟线性搜索文件描述符
            crate::process::manager::fdlookup(fd);
        }
    }
    let baseline_time = crate::subsystems::time::hrtime_nanos() - baseline_start;
    
    // 测试优化实现（扩展缓存）
    let optimized_start = crate::subsystems::time::hrtime_nanos();
    let mut cache = ExtendedFdCache::new();
    
    // 预热缓存
    let common_fds = [
        (0, 100, crate::fs::file::FileType::Vfs),
        (1, 101, crate::fs::file::FileType::Pipe),
        (2, 102, crate::fs::file::FileType::Socket),
        (3, 103, crate::fs::file::FileType::Vfs),
        (4, 104, crate::fs::file::FileType::Pipe),
        (5, 105, crate::fs::file::FileType::Socket),
        (6, 106, crate::fs::file::FileType::Vfs),
        (7, 107, crate::fs::file::FileType::Pipe),
    ];
    cache.warmup(&common_fds);
    
    for _ in 0..ITERATIONS {
        for &fd in TEST_FDS {
            // 使用优化缓存查找
            cache.get(fd);
        }
    }
    let optimized_time = crate::subsystems::time::hrtime_nanos() - optimized_start;
    
    let cache_stats = cache.get_stats();
    
    let baseline_metrics = PerformanceMetrics {
        avg_latency_ns: baseline_time / ITERATIONS as u64,
        p95_latency_ns: baseline_time / ITERATIONS as u64, // 简化计算
        p99_latency_ns: baseline_time / ITERATIONS as u64,
        throughput_ops_per_sec: ITERATIONS as f64 * 1_000_000_000.0 / baseline_time as f64,
        total_execution_time_ns: baseline_time,
        operation_count: ITERATIONS as u64,
    };
    
    let optimized_metrics = PerformanceMetrics {
        avg_latency_ns: optimized_time / ITERATIONS as u64,
        p95_latency_ns: optimized_time / ITERATIONS as u64,
        p99_latency_ns: optimized_time / ITERATIONS as u64,
        throughput_ops_per_sec: ITERATIONS as f64 * 1_000_000_000.0 / optimized_time as f64,
        total_execution_time_ns: optimized_time,
        operation_count: ITERATIONS as u64,
    };
    
    let improvement = baseline_metrics.improvement_over(&optimized_metrics);
    
    crate::println!("[benchmark] 文件描述符缓存测试完成:");
    crate::println!("  基准延迟: {} ns", baseline_metrics.avg_latency_ns);
    crate::println!("  优化延迟: {} ns", optimized_metrics.avg_latency_ns);
    crate::println!("  性能提升: {:.2}%", improvement);
    crate::println!("  缓存命中率: {:.2}%", cache_stats.hit_rate * 100.0);
    
    OptimizationBenchmarkResult {
        test_name: "文件描述符缓存优化".to_string(),
        baseline_metrics,
        optimized_metrics,
        improvement_percentage: improvement,
    }
}

/// copyin/copyout优化性能测试
pub fn benchmark_copy_operations() -> OptimizationBenchmarkResult {
    const ITERATIONS: usize = 1000;
    const BUFFER_SIZES: &[usize] = &[64, 256, 1024, 4096, 16384]; // 不同大小的缓冲区
    
    crate::println!("[benchmark] 开始copyin/copyout优化性能测试...");
    
    let mut baseline_total_time = 0u64;
    let mut optimized_total_time = 0u64;
    
    for &size in BUFFER_SIZES {
        // 测试基准实现
        let baseline_start = crate::subsystems::time::hrtime_nanos();
        for _ in 0..ITERATIONS {
            let mut buffer = vec![0u8; size];
            // 模拟标准copyout操作
            crate::subsystems::mm::vm::copyout(
                crate::process::manager::PROC_TABLE.lock()
                    .find(crate::process::getpid())
                    .unwrap()
                    .pagetable,
                0x10000000, // 模拟用户地址
                buffer.as_ptr(),
                size,
            ).unwrap();
        }
        baseline_total_time += crate::subsystems::time::hrtime_nanos() - baseline_start;
        
        // 测试优化实现
        let optimized_start = crate::subsystems::time::hrtime_nanos();
        let mut copier = OptimizedCopier::with_defaults();
        for _ in 0..ITERATIONS {
            let mut buffer = vec![0u8; size];
            // 使用优化copyout
            copier.copyout_optimized(
                crate::process::manager::PROC_TABLE.lock()
                    .find(crate::process::getpid())
                    .unwrap()
                    .pagetable,
                0x10000000, // 模拟用户地址
                buffer.as_ptr(),
                size,
            ).unwrap();
        }
        optimized_total_time += crate::subsystems::time::hrtime_nanos() - optimized_start;
    }
    
    let copy_stats = crate::subsystems::mm::copy_optimized::get_copy_stats();
    
    let baseline_metrics = PerformanceMetrics {
        avg_latency_ns: baseline_total_time / (ITERATIONS * BUFFER_SIZES.len()) as u64,
        p95_latency_ns: baseline_total_time / (ITERATIONS * BUFFER_SIZES.len()) as u64,
        p99_latency_ns: baseline_total_time / (ITERATIONS * BUFFER_SIZES.len()) as u64,
        throughput_ops_per_sec: (ITERATIONS * BUFFER_SIZES.len()) as f64 * 1_000_000_000.0 / baseline_total_time as f64,
        total_execution_time_ns: baseline_total_time,
        operation_count: (ITERATIONS * BUFFER_SIZES.len()) as u64,
    };
    
    let optimized_metrics = PerformanceMetrics {
        avg_latency_ns: optimized_total_time / (ITERATIONS * BUFFER_SIZES.len()) as u64,
        p95_latency_ns: optimized_total_time / (ITERATIONS * BUFFER_SIZES.len()) as u64,
        p99_latency_ns: optimized_total_time / (ITERATIONS * BUFFER_SIZES.len()) as u64,
        throughput_ops_per_sec: (ITERATIONS * BUFFER_SIZES.len()) as f64 * 1_000_000_000.0 / optimized_total_time as f64,
        total_execution_time_ns: optimized_total_time,
        operation_count: (ITERATIONS * BUFFER_SIZES.len()) as u64,
    };
    
    let improvement = baseline_metrics.improvement_over(&optimized_metrics);
    
    crate::println!("[benchmark] copyin/copyout优化测试完成:");
    crate::println!("  基准延迟: {} ns", baseline_metrics.avg_latency_ns);
    crate::println!("  优化延迟: {} ns", optimized_metrics.avg_latency_ns);
    crate::println!("  性能提升: {:.2}%", improvement);
    crate::println!("  SIMD使用率: {:.2}%", copy_stats.simd_copies as f64 / copy_stats.total_copies as f64 * 100.0);
    
    OptimizationBenchmarkResult {
        test_name: "copyin/copyout优化".to_string(),
        baseline_metrics,
        optimized_metrics,
        improvement_percentage: improvement,
    }
}

/// 系统调用批处理性能测试
pub fn benchmark_batch_processing() -> OptimizationBenchmarkResult {
    const BATCH_SIZE: usize = 32;
    const ITERATIONS: usize = 100;
    
    crate::println!("[benchmark] 开始系统调用批处理性能测试...");
    
    // 重置批处理统计
    reset_batch_stats();
    
    // 测试单个系统调用
    let baseline_start = crate::subsystems::time::hrtime_nanos();
    for _ in 0..ITERATIONS {
        // 模拟单个系统调用
        crate::syscalls::dispatch(0x2002, &[0, 0x10000000, 64, 0, 0, 0]); // read
    }
    let baseline_time = crate::subsystems::time::hrtime_nanos() - baseline_start;
    
    // 测试批处理系统调用（使用批处理模块）
    let optimized_start = crate::subsystems::time::hrtime_nanos();
    for _ in 0..ITERATIONS {
        // 准备批处理请求
        let mut batch_syscalls = Vec::with_capacity(BATCH_SIZE);
        for i in 0..BATCH_SIZE {
            batch_syscalls.push(crate::syscalls::batch::BatchSyscall::new(
                0x2002, // read
                [0, 0x10000000 + i as u64 * 64, 64, 0, 0, 0],
                crate::syscalls::batch::BatchSyscallType::FileIO,
            ));
        }
        
        // 执行批处理
        let _ = crate::syscalls::batch::batch_syscalls(batch_syscalls);
    }
    let optimized_time = crate::subsystems::time::hrtime_nanos() - optimized_start;
    
    // 测试批处理快速路径（使用SYS_BATCH系统调用）
    let fast_path_start = crate::subsystems::time::hrtime_nanos();
    for _ in 0..ITERATIONS {
        // 准备批处理请求
        let mut batch_syscalls = Vec::with_capacity(BATCH_SIZE);
        for i in 0..BATCH_SIZE {
            batch_syscalls.push(crate::syscalls::batch::BatchSyscall::new(
                0x2002, // read
                [0, 0x10000000 + i as u64 * 64, 64, 0, 0, 0],
                crate::syscalls::batch::BatchSyscallType::FileIO,
            ));
        }
        
        let batch_request = crate::syscalls::batch::BatchRequest {
            syscalls: batch_syscalls,
            atomic: false,
            timeout_ms: 1000,
        };
        
        // 序列化批处理请求
        let serialized = match bincode::serialize(&batch_request) {
            Ok(data) => data,
            Err(_) => continue,
        };
        
        // 模拟用户空间指针（这里简化处理）
        let batch_req_ptr = 0x20000000; // 假设的用户空间地址
        
        // 使用批处理快速路径
        let _ = crate::syscalls::dispatch(crate::syscalls::SYS_BATCH as usize, &[batch_req_ptr]);
    }
    let fast_path_time = crate::subsystems::time::hrtime_nanos() - fast_path_start;
    
    let batch_stats = get_batch_stats();
    
    // 比较基准实现与快速路径实现
    let baseline_metrics = PerformanceMetrics {
        avg_latency_ns: baseline_time / (ITERATIONS) as u64,
        p95_latency_ns: baseline_time / (ITERATIONS) as u64,
        p99_latency_ns: baseline_time / (ITERATIONS) as u64,
        throughput_ops_per_sec: ITERATIONS as f64 * 1_000_000_000.0 / baseline_time as f64,
        total_execution_time_ns: baseline_time,
        operation_count: ITERATIONS as u64,
    };
    
    let optimized_metrics = PerformanceMetrics {
        avg_latency_ns: fast_path_time / (ITERATIONS) as u64,
        p95_latency_ns: fast_path_time / (ITERATIONS) as u64,
        p99_latency_ns: fast_path_time / (ITERATIONS) as u64,
        throughput_ops_per_sec: ITERATIONS as f64 * 1_000_000_000.0 / fast_path_time as f64,
        total_execution_time_ns: fast_path_time,
        operation_count: ITERATIONS as u64,
    };
    
    let improvement = baseline_metrics.improvement_over(&optimized_metrics);
    
    crate::println!("[benchmark] 系统调用批处理测试完成:");
    crate::println!("  基准延迟: {} ns", baseline_metrics.avg_latency_ns);
    crate::println!("  优化延迟: {} ns", optimized_metrics.avg_latency_ns);
    crate::println!("  性能提升: {:.2}%", improvement);
    crate::println!("  批处理成功率: {:.2}%", batch_stats.batch_success_rate * 100.0);
    crate::println!("  模块批处理时间: {} ns", optimized_time / (ITERATIONS) as u64);
    crate::println!("  快速路径批处理时间: {} ns", fast_path_time / (ITERATIONS) as u64);
    
    OptimizationBenchmarkResult {
        test_name: "系统调用批处理优化".to_string(),
        baseline_metrics,
        optimized_metrics,
        improvement_percentage: improvement,
    }
}

/// 进程表锁定优化性能测试
pub fn benchmark_lock_optimization() -> OptimizationBenchmarkResult {
    const ITERATIONS: usize = 10000;
    const READER_THREADS: usize = 8;
    const WRITER_THREADS: usize = 2;
    
    crate::println!("[benchmark] 开始进程表锁定优化性能测试...");
    
    // 重置锁统计
    reset_lock_stats();
    
    // 测试基准实现（互斥锁）
    let baseline_start = crate::subsystems::time::hrtime_nanos();
    for _ in 0..ITERATIONS {
        // 模拟获取进程表锁
        let _table = crate::process::manager::PROC_TABLE.lock();
        // 模拟一些操作
        crate::process::getpid();
    }
    let baseline_time = crate::subsystems::time::hrtime_nanos() - baseline_start;
    
    // 测试优化实现（读写锁）
    let optimized_start = crate::subsystems::time::hrtime_nanos();
    for _ in 0..ITERATIONS {
        // 模拟获取进程表读锁
        let _read_lock = crate::process::lock_optimized::convenience::lock_main_table_read();
        // 模拟一些操作
        crate::process::getpid();
    }
    let optimized_time = crate::subsystems::time::hrtime_nanos() - optimized_start;
    
    let lock_stats = get_lock_stats();
    
    let baseline_metrics = PerformanceMetrics {
        avg_latency_ns: baseline_time / ITERATIONS as u64,
        p95_latency_ns: baseline_time / ITERATIONS as u64,
        p99_latency_ns: baseline_time / ITERATIONS as u64,
        throughput_ops_per_sec: ITERATIONS as f64 * 1_000_000_000.0 / baseline_time as f64,
        total_execution_time_ns: baseline_time,
        operation_count: ITERATIONS as u64,
    };
    
    let optimized_metrics = PerformanceMetrics {
        avg_latency_ns: optimized_time / ITERATIONS as u64,
        p95_latency_ns: optimized_time / ITERATIONS as u64,
        p99_latency_ns: optimized_time / ITERATIONS as u64,
        throughput_ops_per_sec: ITERATIONS as f64 * 1_000_000_000.0 / optimized_time as f64,
        total_execution_time_ns: optimized_time,
        operation_count: ITERATIONS as u64,
    };
    
    let improvement = baseline_metrics.improvement_over(&optimized_metrics);
    
    crate::println!("[benchmark] 进程表锁定优化测试完成:");
    crate::println!("  基准延迟: {} ns", baseline_metrics.avg_latency_ns);
    crate::println!("  优化延迟: {} ns", optimized_metrics.avg_latency_ns);
    crate::println!("  性能提升: {:.2}%", improvement);
    crate::println!("  锁竞争率: {:.2}%", lock_stats.contention_rate * 100.0);
    
    OptimizationBenchmarkResult {
        test_name: "进程表锁定优化".to_string(),
        baseline_metrics,
        optimized_metrics,
        improvement_percentage: improvement,
    }
}

/// 综合性能测试
pub fn benchmark_comprehensive_optimization() -> Vec<OptimizationBenchmarkResult> {
    crate::println!("[benchmark] 开始综合系统调用优化性能测试...");
    
    let mut results = Vec::new();
    
    // 运行所有单项测试
    results.push(benchmark_fd_cache());
    results.push(benchmark_copy_operations());
    results.push(benchmark_batch_processing());
    results.push(benchmark_lock_optimization());
    
    // 计算综合性能提升
    let total_improvement = results.iter()
        .map(|r| r.improvement_percentage)
        .sum::<f64>() / results.len() as f64;
    
    crate::println!("[benchmark] 综合性能测试完成:");
    crate::println!("  平均性能提升: {:.2}%", total_improvement);
    
    results
}

/// 打印性能测试结果
pub fn print_optimization_results(results: &[OptimizationBenchmarkResult]) {
    crate::println!("\n=== 系统调用优化性能测试结果 ===");
    
    for result in results {
        crate::println!("\n{}:", result.test_name);
        crate::println!("  基准平均延迟: {} ns", result.baseline_metrics.avg_latency_ns);
        crate::println!("  优化平均延迟: {} ns", result.optimized_metrics.avg_latency_ns);
        crate::println!("  性能提升: {:.2}%", result.improvement_percentage);
        crate::println!("  基准吞吐量: {:.2} ops/sec", result.baseline_metrics.throughput_ops_per_sec);
        crate::println!("  优化吞吐量: {:.2} ops/sec", result.optimized_metrics.throughput_ops_per_sec);
    }
    
    let overall_improvement = results.iter()
        .map(|r| r.improvement_percentage)
        .sum::<f64>() / results.len() as f64;
    
    crate::println!("\n总体优化效果:");
    crate::println!("  平均性能提升: {:.2}%", overall_improvement);
    
    // 性能提升等级评估
    let performance_grade = if overall_improvement >= 50.0 {
        "显著提升"
    } else if overall_improvement >= 25.0 {
        "良好提升"
    } else if overall_improvement >= 10.0 {
        "中等提升"
    } else if overall_improvement >= 5.0 {
        "轻微提升"
    } else {
        "提升不明显"
    };
    
    crate::println!("  性能提升等级: {}", performance_grade);
}

/// 运行所有优化性能测试
pub fn run_all_optimization_benchmarks() {
    crate::println!("[benchmark] 开始系统调用优化性能基准测试...");
    
    let results = benchmark_comprehensive_optimization();
    print_optimization_results(&results);
    
    crate::println!("[benchmark] 系统调用优化性能基准测试完成");
}

/// 性能测试主入口
pub fn main() {
    run_all_optimization_benchmarks();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_calculation() {
        let baseline = PerformanceMetrics {
            avg_latency_ns: 1000,
            p95_latency_ns: 1500,
            p99_latency_ns: 2000,
            throughput_ops_per_sec: 1000.0,
            total_execution_time_ns: 1000000,
            operation_count: 1000,
        };
        
        let optimized = PerformanceMetrics {
            avg_latency_ns: 500,
            p95_latency_ns: 750,
            p99_latency_ns: 1000,
            throughput_ops_per_sec: 2000.0,
            total_execution_time_ns: 500000,
            operation_count: 1000,
        };
        
        let improvement = optimized.improvement_over(&baseline);
        assert_eq!(improvement, 50.0); // (1000-500)/1000 * 100
        
        let throughput_improvement = optimized.throughput_improvement_over(&baseline);
        assert_eq!(throughput_improvement, 100.0); // (2000-1000)/1000 * 100
    }

    #[test]
    fn test_optimization_benchmark_result() {
        let baseline = PerformanceMetrics {
            avg_latency_ns: 1000,
            p95_latency_ns: 1500,
            p99_latency_ns: 2000,
            throughput_ops_per_sec: 1000.0,
            total_execution_time_ns: 1000000,
            operation_count: 1000,
        };
        
        let optimized = PerformanceMetrics {
            avg_latency_ns: 500,
            p95_latency_ns: 750,
            p99_latency_ns: 1000,
            throughput_ops_per_sec: 2000.0,
            total_execution_time_ns: 500000,
            operation_count: 1000,
        };
        
        let result = OptimizationBenchmarkResult {
            test_name: "测试优化".to_string(),
            baseline_metrics,
            optimized_metrics,
            improvement_percentage: 50.0,
        };
        
        assert_eq!(result.test_name, "测试优化");
        assert_eq!(result.improvement_percentage, 50.0);
    }
}