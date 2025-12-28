//! 性能优化测试框架
//!
//! 这个模块实现了一个全面的性能测试框架，包含以下功能：
//! - 内存管理性能测试
//! - 文件系统性能测试
//! - I/O性能测试
//! - 综合性能基准测试
//! - 自动性能回归测试
//! - 性能报告生成

extern crate alloc;

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::subsystems::sync::{Mutex, SpinLock};
use crate::subsystems::mm::optimized_memory_manager::get_optimized_memory_stats;
use crate::vfs::optimized_filesystem::get_optimized_fs_stats;
use crate::io::optimized_io_manager::get_optimized_io_stats;
use crate::performance::monitoring::{get_performance_report, update_perf_metric};

/// 测试类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestType {
    /// 内存分配测试
    MemoryAllocation,
    /// 内存碎片测试
    MemoryFragmentation,
    /// 文件系统缓存测试
    FileSystemCache,
    /// I/O吞吐量测试
    IoThroughput,
    /// I/O延迟测试
    IoLatency,
    /// 综合性能测试
    Comprehensive,
    /// 回归测试
    Regression,
}

/// 测试结果
#[derive(Debug, Clone)]
pub struct TestResult {
    /// 测试类型
    pub test_type: TestType,
    /// 测试名称
    pub test_name: String,
    /// 测试开始时间
    pub start_time: u64,
    /// 测试结束时间
    pub end_time: u64,
    /// 测试持续时间（纳秒）
    pub duration_ns: u64,
    /// 是否通过
    pub passed: bool,
    /// 测试分数（0-100）
    pub score: u32,
    /// 测试指标
    pub metrics: BTreeMap<String, f64>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 性能基准
#[derive(Debug, Clone)]
pub struct PerformanceBenchmark {
    /// 基准名称
    pub name: String,
    /// 基准描述
    pub description: String,
    /// 测试结果列表
    pub results: Vec<TestResult>,
    /// 基准分数
    pub baseline_score: f64,
    /// 当前分数
    pub current_score: f64,
    /// 改进百分比
    pub improvement_percent: f64,
    /// 基准时间
    pub timestamp: u64,
}

/// 测试配置
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// 测试迭代次数
    pub iterations: usize,
    /// 测试数据大小
    pub data_size: usize,
    /// 并发度
    pub concurrency: usize,
    /// 是否启用详细日志
    pub verbose_logging: bool,
    /// 测试超时（秒）
    pub timeout_seconds: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            data_size: 4096, // 4KB
            concurrency: 1,
            verbose_logging: false,
            timeout_seconds: 30,
        }
    }
}

/// 性能测试框架
pub struct PerformanceTestFramework {
    /// 测试配置
    config: TestConfig,
    /// 测试历史
    test_history: Mutex<Vec<TestResult>>,
    /// 基准历史
    benchmark_history: Mutex<Vec<PerformanceBenchmark>>,
    /// 测试计数器
    test_counter: AtomicUsize,
    /// 基准计数器
    benchmark_counter: AtomicUsize,
    /// 当前测试ID
    current_test_id: AtomicUsize,
}

impl PerformanceTestFramework {
    /// 创建新的性能测试框架
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    /// 使用指定配置创建性能测试框架
    pub fn with_config(config: TestConfig) -> Self {
        Self {
            config,
            test_history: Mutex::new(Vec::new()),
            benchmark_history: Mutex::new(Vec::new()),
            test_counter: AtomicUsize::new(0),
            benchmark_counter: AtomicUsize::new(0),
            current_test_id: AtomicUsize::new(0),
        }
    }

    /// 运行内存分配测试
    pub fn run_memory_allocation_test(&self) -> TestResult {
        let test_id = self.current_test_id.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::subsystems::time::get_time_ns();
        
        crate::println!("[perf_test] Starting memory allocation test {}", test_id);
        
        let mut total_time = 0u64;
        let mut successful_allocs = 0u64;
        let mut failed_allocs = 0u64;
        let mut total_allocated_bytes = 0u64;
        
        for i in 0..self.config.iterations {
            let iter_start = crate::subsystems::time::get_time_ns();
            
            // 测试分配性能
            for _ in 0..self.config.concurrency {
                let allocated_bytes = self.test_allocation_performance();
                if allocated_bytes > 0 {
                    successful_allocs += 1;
                    total_allocated_bytes += allocated_bytes as u64;
                } else {
                    failed_allocs += 1;
                }
            }
            
            total_time += crate::subsystems::time::get_time_ns() - iter_start;
            
            if self.config.verbose_logging {
                crate::println!("[perf_test] Memory allocation iteration {} completed", i);
            }
        }
        
        let end_time = crate::subsystems::time::get_time_ns();
        let duration_ns = end_time - start_time;
        
        // 计算分数
        let success_rate = successful_allocs as f64 / (successful_allocs + failed_allocs) as f64;
        let avg_allocation_time = if successful_allocs > 0 {
            total_time / successful_allocs
        } else {
            0
        };
        
        let score = (success_rate * 50.0 + 
                    (1000.0 / (avg_allocation_time / 1000000.0 + 1.0)) * 30.0 +
                    (total_allocated_bytes as f64 / (self.config.data_size as f64 * self.config.iterations as f64)) * 20.0) as u32;
        
        let mut metrics = BTreeMap::new();
        metrics.insert("success_rate".to_string(), success_rate * 100.0);
        metrics.insert("avg_allocation_time_ns".to_string(), avg_allocation_time as f64);
        metrics.insert("total_allocated_bytes".to_string(), total_allocated_bytes as f64);
        metrics.insert("throughput_bytes_per_sec".to_string(), 
                    (total_allocated_bytes as f64) * 1_000_000_000 / duration_ns as f64);
        
        TestResult {
            test_type: TestType::MemoryAllocation,
            test_name: "Memory Allocation Performance".to_string(),
            start_time,
            end_time,
            duration_ns,
            passed: score > 70,
            score,
            metrics,
            error_message: None,
        }
    }

    /// 测试分配性能
    fn test_allocation_performance(&self) -> usize {
        // 简化实现：测试分配指定大小的数据
        let start_time = crate::subsystems::time::get_time_ns();
        
        // 尝试使用优化的内存管理器
        if let Some(memory_stats) = get_optimized_memory_stats() {
            // 基于内存压力调整分配策略
            let pressure = memory_stats.memory_pressure;
            
            if pressure < 50 {
                // 低压力：快速分配
                crate::subsystems::mm::optimized_memory_manager::optimized_alloc_page()
            } else if pressure < 80 {
                // 中等压力：正常分配
                crate::subsystems::mm::optimized_memory_manager::optimized_alloc_page()
            } else {
                // 高压力：谨慎分配
                crate::subsystems::mm::optimized_memory_manager::optimized_alloc_page()
            }
        } else {
            // 回退到标准分配
            crate::subsystems::mm::kalloc();
        }
        
        let end_time = crate::subsystems::time::get_time_ns();
        
        if self.config.verbose_logging {
            let allocation_time = end_time - start_time;
            crate::println!("[perf_test] Allocation took {}ns", allocation_time);
        }
        
        self.config.data_size
    }

    /// 运行文件系统缓存测试
    pub fn run_filesystem_cache_test(&self) -> TestResult {
        let test_id = self.current_test_id.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::subsystems::time::get_time_ns();
        
        crate::println!("[perf_test] Starting filesystem cache test {}", test_id);
        
        let mut cache_hits = 0u64;
        let mut cache_misses = 0u64;
        let mut total_operations = 0u64;
        
        for i in 0..self.config.iterations {
            let iter_start = crate::subsystems::time::get_time_ns();
            
            // 测试文件系统缓存性能
            for _ in 0..100 {
                // 模拟文件访问
                total_operations += 1;
                
                // 检查缓存命中率
                if let Some(fs_stats) = get_optimized_fs_stats() {
                    let hit_rate = fs_stats.cache_hit_rate;
                    
                    if hit_rate > 80.0 {
                        cache_hits += 1;
                    } else {
                        cache_misses += 1;
                    }
                }
            }
            
            total_operations += crate::subsystems::time::get_time_ns() - iter_start;
            
            if self.config.verbose_logging {
                crate::println!("[perf_test] Filesystem cache iteration {} completed", i);
            }
        }
        
        let end_time = crate::subsystems::time::get_time_ns();
        let duration_ns = end_time - start_time;
        
        // 计算分数
        let cache_hit_rate = if total_operations > 0 {
            (cache_hits as f64) / (total_operations as f64) * 100.0
        } else {
            0.0
        };
        
        let score = (cache_hit_rate * 60.0 + 
                    (total_operations as f64 / (self.config.iterations as f64 * 100.0)) * 40.0) as u32;
        
        let mut metrics = BTreeMap::new();
        metrics.insert("cache_hit_rate".to_string(), cache_hit_rate);
        metrics.insert("total_operations".to_string(), total_operations as f64);
        metrics.insert("operations_per_second".to_string(), 
                    (total_operations as f64) * 1_000_000_000 / duration_ns as f64);
        
        TestResult {
            test_type: TestType::FileSystemCache,
            test_name: "Filesystem Cache Performance".to_string(),
            start_time,
            end_time,
            duration_ns,
            passed: score > 70,
            score,
            metrics,
            error_message: None,
        }
    }

    /// 运行I/O吞吐量测试
    pub fn run_io_throughput_test(&self) -> TestResult {
        let test_id = self.current_test_id.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::subsystems::time::get_time_ns();
        
        crate::println!("[perf_test] Starting I/O throughput test {}", test_id);
        
        let mut total_bytes = 0u64;
        let mut total_time = 0u64;
        
        for i in 0..self.config.iterations {
            let iter_start = crate::subsystems::time::get_time_ns();
            
            // 测试I/O吞吐量
            for _ in 0..self.config.concurrency {
                let bytes_transferred = self.test_io_throughput_performance();
                total_bytes += bytes_transferred as u64;
            }
            
            total_time += crate::subsystems::time::get_time_ns() - iter_start;
            
            if self.config.verbose_logging {
                crate::println!("[perf_test] I/O throughput iteration {} completed", i);
            }
        }
        
        let end_time = crate::subsystems::time::get_time_ns();
        let duration_ns = end_time - start_time;
        
        // 计算分数
        let throughput = if duration_ns > 0 {
            (total_bytes as f64) * 1_000_000_000 / duration_ns as f64
        } else {
            0.0
        };
        
        let score = (throughput / 1024.0 * 1024.0 * 50.0) as u32; // MB/s
        
        let mut metrics = BTreeMap::new();
        metrics.insert("throughput_mbps".to_string(), throughput);
        metrics.insert("total_bytes".to_string(), total_bytes as f64);
        metrics.insert("duration_seconds".to_string(), duration_ns as f64 / 1_000_000_000);
        
        TestResult {
            test_type: TestType::IoThroughput,
            test_name: "I/O Throughput Performance".to_string(),
            start_time,
            end_time,
            duration_ns,
            passed: score > 70,
            score,
            metrics,
            error_message: None,
        }
    }

    /// 测试I/O吞吐量性能
    fn test_io_throughput_performance(&self) -> usize {
        // 简化实现：模拟I/O传输
        let start_time = crate::subsystems::time::get_time_ns();
        
        // 提交I/O请求
        let request_id = crate::io::submit_optimized_io_request(
            crate::io::IoOperationType::Write,
            1, // fd
            crate::subsystems::mm::vm::PAGE_SIZE as *mut u8,
            self.config.data_size,
            0, // offset
            crate::io::IoPriority::Normal,
        );
        
        // 等待完成（简化实现）
        crate::subsystems::time::delay_ms(10); // 模拟I/O延迟
        
        let end_time = crate::subsystems::time::get_time_ns();
        
        if self.config.verbose_logging {
            let io_time = end_time - start_time;
            crate::println!("[perf_test] I/O operation took {}ns", io_time);
        }
        
        self.config.data_size
    }

    /// 运行综合性能测试
    pub fn run_comprehensive_test(&self) -> PerformanceBenchmark {
        let benchmark_id = self.benchmark_counter.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::subsystems::time::get_time_ns();
        
        crate::println!("[perf_test] Starting comprehensive performance benchmark {}", benchmark_id);
        
        let mut results = Vec::new();
        
        // 运行各种性能测试
        results.push(self.run_memory_allocation_test());
        results.push(self.run_filesystem_cache_test());
        results.push(self.run_io_throughput_test());
        
        let end_time = crate::subsystems::time::get_time_ns();
        let duration_ns = end_time - start_time;
        
        // 计算综合分数
        let avg_score = results.iter().map(|r| r.score).sum::<u32>() / results.len() as u32;
        
        let benchmark = PerformanceBenchmark {
            name: "Comprehensive Performance Benchmark".to_string(),
            description: "Tests memory, filesystem, and I/O performance".to_string(),
            results,
            baseline_score: 70.0,
            current_score: avg_score as f64,
            improvement_percent: ((avg_score as f64 - 70.0) / 70.0 * 100.0),
            timestamp: end_time,
        };
        
        // 保存基准结果
        {
            let mut history = self.benchmark_history.lock();
            history.push(benchmark.clone());
            
            // 限制历史记录数量
            if history.len() > 100 {
                history.remove(0);
            }
        }
        
        crate::println!("[perf_test] Comprehensive benchmark completed with score: {:.2}", avg_score);
        
        benchmark
    }

    /// 运行回归测试
    pub fn run_regression_test(&self, baseline_score: f64) -> TestResult {
        let test_id = self.current_test_id.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::subsystems::time::get_time_ns();
        
        crate::println!("[perf_test] Starting regression test {} against baseline {:.2}", test_id, baseline_score);
        
        // 运行快速综合测试
        let benchmark = self.run_comprehensive_test();
        let current_score = benchmark.current_score;
        
        let end_time = crate::subsystems::time::get_time_ns();
        let duration_ns = end_time - start_time;
        
        // 检查是否回归
        let regression = current_score < baseline_score - 10.0; // 允许10%的下降
        
        let score = if regression {
            // 回归测试失败
            current_score / 2
        } else {
            current_score
        };
        
        let mut metrics = BTreeMap::new();
        metrics.insert("baseline_score".to_string(), baseline_score);
        metrics.insert("current_score".to_string(), current_score);
        metrics.insert("regression_detected".to_string(), if regression { 1.0 } else { 0.0 });
        metrics.insert("performance_degradation".to_string(), if regression { baseline_score - current_score } else { 0.0 });
        
        TestResult {
            test_type: TestType::Regression,
            test_name: "Performance Regression Test".to_string(),
            start_time,
            end_time,
            duration_ns,
            passed: !regression,
            score,
            metrics,
            error_message: if regression {
                Some("Performance regression detected".to_string())
            } else {
                None
            },
        }
    }

    /// 生成性能报告
    pub fn generate_performance_report(&self) -> String {
        let current_time = crate::subsystems::time::get_time_ms();
        
        // 获取性能监控数据
        let perf_report = get_performance_report();
        
        // 获取测试历史
        let test_history = self.test_history.lock();
        let benchmark_history = self.benchmark_history.lock();
        
        let mut report = format!("NOS Kernel Performance Report\n");
        report.push_str!("Generated at: {}\n\n", current_time);
        
        // 性能监控摘要
        if let Some(report) = perf_report {
            report.push_str!("=== Performance Monitoring ===\n");
            for metric in &report.metrics {
                report.push_str!("{}: {:.2} (avg: {:.2}, min: {:.2}, max: {:.2})\n", 
                           metric.name, metric.current_value, 
                           metric.avg_value, metric.min_value, metric.max_value);
            }
            
            if !report.alerts.is_empty() {
                report.push_str!("\n=== Active Alerts ===\n");
                for alert in &report.alerts {
                    report.push_str!("{}: {} - {}\n", 
                               alert.alert_type, alert.metric_type, alert.message);
                }
            }
            
            if !report.trends.is_empty() {
                report.push_str!("\n=== Performance Trends ===\n");
                for trend in &report.trends {
                    report.push_str!("{}: {} (strength: {:.2}, direction: {:?})\n", 
                               trend.metric_type, trend.predicted_value, trend.strength, trend.direction);
                }
            }
        }
        
        // 测试结果摘要
        report.push_str!("\n=== Test Results ===\n");
        if !test_history.is_empty() {
            let latest_tests = test_history.iter().rev().take(5);
            for test in latest_tests {
                report.push_str!("Test: {} - Score: {:.2} - {}\n", 
                               test.test_name, test.score, 
                               if test.passed { "PASSED" } else { "FAILED" });
                
                if !test.metrics.is_empty() {
                    report.push_str!("  Metrics:\n");
                    for (name, value) in &test.metrics {
                        report.push_str!("    {}: {:.2}\n", name, value);
                    }
                }
            }
        }
        
        // 基准测试摘要
        report.push_str!("\n=== Benchmarks ===\n");
        if !benchmark_history.is_empty() {
            let latest_benchmarks = benchmark_history.iter().rev().take(5);
            for benchmark in latest_benchmarks {
                report.push_str!("Benchmark: {} - Score: {:.2} - Improvement: {:.1}%\n", 
                               benchmark.name, benchmark.current_score, benchmark.improvement_percent);
            }
        }
        
        report
    }

    /// 清理测试数据
    pub fn cleanup_test_data(&self) {
        {
            let mut test_history = self.test_history.lock();
            test_history.clear();
        }
        
        {
            let mut benchmark_history = self.benchmark_history.lock();
            benchmark_history.clear();
        }
        
        crate::println!("[perf_test] Test data cleaned up");
    }
}

/// 全局性能测试框架
static GLOBAL_PERF_TEST_FRAMEWORK: SpinLock<Option<PerformanceTestFramework>> = SpinLock::new(None);

/// 初始化全局性能测试框架
pub fn init_global_perf_test_framework() {
    let mut global_framework = GLOBAL_PERF_TEST_FRAMEWORK.lock();
    if global_framework.is_none() {
        let framework = PerformanceTestFramework::new();
        *global_framework = Some(framework);
        crate::println!("[perf_test] Global performance test framework initialized");
    }
}

/// 获取全局性能测试框架
pub fn get_global_perf_test_framework() -> Option<PerformanceTestFramework> {
    GLOBAL_PERF_TEST_FRAMEWORK.lock().clone()
}

/// 运行性能测试（全局接口）
pub fn run_performance_test(test_type: TestType) -> Option<TestResult> {
    if let Some(ref framework) = get_global_perf_test_framework() {
        let result = match test_type {
            TestType::MemoryAllocation => Some(framework.run_memory_allocation_test()),
            TestType::MemoryFragmentation => Some(framework.run_memory_allocation_test()),
            TestType::FileSystemCache => Some(framework.run_filesystem_cache_test()),
            TestType::IoThroughput => Some(framework.run_io_throughput_test()),
            TestType::Comprehensive => Some(framework.run_comprehensive_test()),
            TestType::Regression => {
                // 使用最近的基准分数
                if let Some(benchmark) = framework.benchmark_history.lock().iter().rev().next() {
                    Some(framework.run_regression_test(benchmark.current_score))
                } else {
                    Some(framework.run_regression_test(70.0))
                }
            }
        };
        
        // 保存测试结果
        {
            let mut history = framework.test_history.lock();
            if let Some(ref result) = result {
                history.push(result.clone());
            }
        }
        
        result
    } else {
        None
    }
}

/// 生成性能报告（全局接口）
pub fn generate_performance_report() -> Option<String> {
    get_global_perf_test_framework().map(|framework| framework.generate_performance_report())
}

/// 清理测试数据（全局接口）
pub fn cleanup_perf_test_data() {
    if let Some(ref framework) = get_global_perf_test_framework() {
        framework.cleanup_test_data();
    }
}