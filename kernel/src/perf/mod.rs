//! Performance Monitoring and Optimization Module
//!
//! This module provides comprehensive performance monitoring and optimization capabilities including:
//! - Hardware performance counters (CPU, cache, branch prediction, TLB)
//! - Software performance counters (syscalls, context switches, interrupts)
//! - CPU profiling with flame graph generation
//! - Memory profiling and leak detection
//! - I/O profiling and analysis
//! - Lock contention profiling
//! - JIT compilation and optimization
//! - Profile-guided optimization (PGO)
//! - Advanced memory allocators (arena, pool, slab)
//! - Cache optimization and prefetching
//! - NUMA-aware scheduling
//! - CPU topology-aware task placement
//! - Power-aware scheduling
//! - Real-time scheduling support
//! - Performance metrics collection and aggregation
//!
//! # Submodules
//!
//! - **core**: Core performance monitoring infrastructure
//! - **monitoring**: Performance statistics collection
//! - **hardware**: Hardware performance counter management
//! - **software**: Software performance counter management
//! - **counter_manager**: Unified counter management
//! - **profiler**: CPU, memory, I/O, and lock profiling
//! - **optimizer**: JIT compilation, PGO, and code optimization
//! - **allocator**: Advanced memory allocation strategies
//! - **cache**: Cache optimization and data structures
//! - **scheduler**: Advanced scheduling algorithms
//! - **metrics**: Performance metrics collection
//! - **sync**: Synchronization primitives optimization
//! - **atomic**: Atomic operation optimization
//! - **rcu**: Read-Copy-Update implementation
//! - **parallel**: Parallel execution and workqueue optimization
//! - **concurrency_mod**: Concurrency optimization manager
//! - **cpuidle**: CPU idle state management (C-states)
//! - **cpuhotplug**: CPU hotplug support
//! - **freq**: CPU frequency scaling (P-states)
//! - **energy**: Energy model and power management
//! - **power_mod**: Power management manager

use alloc::string::ToString;

pub mod core;
pub mod monitoring;
pub mod hardware;
pub mod software;
pub mod counter_manager;

// New performance optimization modules
pub mod profiler;
pub mod optimizer;
pub mod allocator;
pub mod cache;
pub mod scheduler;
pub mod metrics;

// New benchmarking and profiling modules
pub mod bench;
pub mod trace;
pub mod report;
pub mod bench_mod;

// Concurrency optimization modules
pub mod sync;
pub mod atomic;
pub mod rcu;
pub mod parallel;
pub mod concurrency_mod;

// Power management modules
pub mod cpuidle;
pub mod cpuhotplug;
pub mod freq;
pub mod energy;
pub mod power_mod;

// Track EJ: CPU & Scheduler Optimization modules
pub mod cpu_optimizer;
pub mod scheduler_optimizer;
pub mod lock_optimizer;
pub mod instruction;
pub mod cpu_mod;

// Track EK: Memory Optimization modules
pub mod mem_allocator;
pub mod paging;
pub mod zero;
pub mod kmem;
pub mod mmap;
pub mod memory_mod;

// Track EL: I/O Optimization modules
pub mod block;
pub mod network;
pub mod filesystem;
pub mod io;
pub mod io_mod;

#[cfg(test)]
pub mod examples;

use crate::prelude::*;

// Re-export commonly used types
pub use core::{SyscallStatsSnapshot, UnifiedSyscallStats};

pub use monitoring::get_perf_stats;

// Re-export performance counter types and functions
pub use hardware::{
    HardwareCounterManager, HardwareCounterType, HardwareCounterValue, PerCpuHardwareCounters,
    init_hardware_counters, get_hw_counter_manager, increment_hw_counter, read_tsc,
};

pub use software::{
    SoftwareCounterManager, SoftwareCounterType, PerCpuSoftwareCounters,
    init_software_counters, get_sw_counter_manager, increment_sw_counter,
};

pub use counter_manager::{
    PerformanceCounterManager, Counter, CounterCategory, SimpleCounter,
    CounterSnapshot, PerCpuCounterSnapshot, CounterId,
    init_counter_manager, get_counter_manager,
    create_snapshot, export_counters, get_counters_summary,
};

// Re-export profiling types
pub use profiler::{
    ProfilerConfig, CpuProfiler, CpuSample, FunctionStats, FlameGraph,
    MemoryProfiler, AllocationEvent, AllocationType, MemoryStats,
    IoProfiler, IoEvent, IoOperation, IoStats, IoThroughput,
    LockProfiler, LockEvent, LockType, LockStats,
    ProfilerManager,
};

// Re-export optimizer types
pub use optimizer::{
    OptLevel, OptimizerConfig, JitCompiler, CompiledCode, JitStats,
    PgoManager, FunctionProfile, ValueProfile, OptimizationHints,
    InliningOptimizer, InliningStats, LoopOptimizer, LoopInfo,
    VectorizationOptimizer, VectorizationStrategy,
    OptimizationManager, OptimizationReport, OptimizationSuggestion,
};

// Re-export allocator types
pub use allocator::{
    AllocatorConfig, ArenaAllocator, ArenaChunk,
    PoolAllocator, PoolStats, SizeClass, SlabAllocator, SlabStats,
    AllocatorHooks, AllocationProfiler, SizeStats, AllocationStats,
    AllocatorManager, AllocatorReport,
};

// Re-export cache types
pub use cache::{
    CacheLevel, CacheInfo, CacheSimulator, CacheAccessResult, CacheStats,
    AlignedBuffer, CachePadded, PrefetchStrategy, Prefetcher,
    CacheHashTable, StructureOfArrays,
    AccessPatternAnalyzer, AccessPattern,
    CacheOptimizationAdvisor, OptimizationRecommendation,
};

// Re-export scheduler types
pub use scheduler::{
    SchedulingPolicy, CpuTopology, CpuUtilization, TaskInfo,
    NumaScheduler, LoadBalancer, PowerState, PowerPolicy, PowerScheduler,
    RealtimeScheduler, UnifiedScheduler, SchedulerStats,
};

// Re-export metrics types
pub use metrics::{
    MetricId,
    MetricValue as MetricsValue,
    MetricType as MetricsType,
    MetricMetadata,
    TimeSeries, TimeSeriesPoint, HardwareCounter, HardwareCounterData,
    PerformanceCounterManager as PerfCounterManager,
    EventTracker, EventMetadata,
    MetricsAggregator, AggregationFunction,
    MetricsExporter, ExportFormat,
    MetricsManager, MetricsSummary,
};

// Re-export memory optimization types (Track EK)
pub use mem_allocator::{
    AllocError, OptimizedAllocator, AllocatorStats, SlabCacheStats,
    init_optimized_allocator, alloc_aligned, alloc_huge, numa_alloc,
};

pub use paging::{
    PagingError, PagingOptimizer, PagingOptimizerStats,
    init_paging_optimizer, optimize_page_table, flush_tlb_range, promote_huge_page,
};

pub use zero::{
    ZeroPageError, ZeroPageOptimizer, ZeroPageOptimizerStats,
    init_zero_page_optimizer, get_zero_page, optimize_cow, handle_page_fault, get_zero_stats,
};

pub use kmem::{
    KmemError, KmemAllocator, KmemCache, KmemStats,
    init_kmem_allocator, kmalloc, kfree, kmem_cache_create,
};

pub use mmap::{
    MmapError, MmapOptimizer, Vma, VmaType, VmaStats, MmapStatistics,
    init_mmap_optimizer, mmap_optimized, munmap_optimized, get_vma_stats, get_mmap_stats,
};

pub use memory_mod::{
    MemOptError, MemoryOptimizationManager, OptimizationPolicy,
    MemoryStatistics, MemoryProfile, NumaStats, MemoryOptimizationReport,
    init_memory_optimization, enable_optimizations, disable_optimizations,
    get_memory_statistics, get_memory_profile, get_numa_statistics,
    get_optimization_policy, set_optimization_policy,
    numa_allocate, update_statistics, get_optimization_report, adapt_optimization_policy,
};

/// Initialize all performance monitoring subsystems
pub fn init_all() {
    log::info!("Initializing performance monitoring subsystems...");

    // Initialize hardware counters
    init_hardware_counters();

    // Initialize software counters
    init_software_counters();

    // Initialize counter manager
    init_counter_manager();

    // Initialize syscall stats
    core::init_syscall_stats();

    log::info!("Performance monitoring subsystems initialized successfully");
}

/// 性能监控器
pub struct PerformanceMonitor {
    metrics: Mutex<BTreeMap<String, PerformanceMetric>>,
    collectors: Mutex<Vec<Arc<dyn PerformanceCollector>>>,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            collectors: Mutex::new(Vec::new()),
        }
    }

    /// 添加性能收集器
    pub fn add_collector(&self, collector: Arc<dyn PerformanceCollector>) {
        let mut collectors = self.collectors.lock();
        collectors.push(collector);
    }

    /// 移除性能收集器
    pub fn remove_collector(&self, collector_name: &str) {
        let mut collectors = self.collectors.lock();
        collectors.retain(|c| c.name() != collector_name);
    }

    /// 收集性能指标
    pub fn collect_metrics(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let collectors = self.collectors.lock();
        let mut all_metrics = BTreeMap::new();

        for collector in collectors.iter() {
            let metrics = collector.collect()?;
            for (name, metric) in metrics {
                all_metrics.insert(name, metric);
            }
        }

        // 更新内部指标
        {
            let mut internal_metrics = self.metrics.lock();
            for (name, metric) in &all_metrics {
                internal_metrics.insert(name.clone(), metric.clone());
            }
        }

        Ok(all_metrics)
    }

    /// 获取特定指标
    pub fn get_metric(&self, name: &str) -> Option<PerformanceMetric> {
        let metrics = self.metrics.lock();
        metrics.get(name).cloned()
    }

    /// 获取所有指标
    pub fn get_all_metrics(&self) -> BTreeMap<String, PerformanceMetric> {
        let metrics = self.metrics.lock();
        metrics.clone()
    }

    /// 清除所有指标
    pub fn clear_metrics(&self) {
        let mut metrics = self.metrics.lock();
        metrics.clear();
    }
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标值
    pub value: MetricValue,
    /// 指标单位
    pub unit: String,
    /// 时间戳
    pub timestamp: u64,
    /// 标签
    pub tags: BTreeMap<String, String>,
}

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// 计数器
    Counter,
    /// 计量器
    Gauge,
    /// 直方图
    Histogram,
    /// 摘要
    Summary,
}

/// 指标值
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// 整数值
    Integer(i64),
    /// 浮点数值
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 字符串值
    String(String),
    /// 数组值
    Array(Vec<MetricValue>),
}

/// 性能收集器
pub trait PerformanceCollector: Send + Sync {
    /// 收集性能指标
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>>;

    /// 获取收集器名称
    fn name(&self) -> &str;

    /// 获取收集器描述
    fn description(&self) -> &str {
        "Performance collector"
    }
}

/// CPU性能收集器
pub struct CpuPerformanceCollector {
    name: String,
}

impl CpuPerformanceCollector {
    /// 创建新的CPU性能收集器
    pub fn new() -> Self {
        Self { name: "cpu".to_string() }
    }
}

impl PerformanceCollector for CpuPerformanceCollector {
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let mut metrics = BTreeMap::new();

        // 占位符实现：收集CPU使用率
        metrics.insert(
            "cpu_usage".to_string(),
            PerformanceMetric {
                name: "cpu_usage".to_string(),
                metric_type: MetricType::Gauge,
                value: MetricValue::Float(0.0),
                unit: "percent".to_string(),
                timestamp: nos_api::event::get_time_ns(),
                tags: BTreeMap::new(),
            },
        );

        // 占位符实现：收集CPU温度
        metrics.insert(
            "cpu_temperature".to_string(),
            PerformanceMetric {
                name: "cpu_temperature".to_string(),
                metric_type: MetricType::Gauge,
                value: MetricValue::Float(0.0),
                unit: "celsius".to_string(),
                timestamp: nos_api::event::get_time_ns(),
                tags: BTreeMap::new(),
            },
        );

        Ok(metrics)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "CPU performance collector"
    }
}

/// 内存性能收集器
pub struct MemoryPerformanceCollector {
    name: String,
}

impl MemoryPerformanceCollector {
    /// 创建新的内存性能收集器
    pub fn new() -> Self {
        Self { name: "memory".to_string() }
    }
}

impl PerformanceCollector for MemoryPerformanceCollector {
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let mut metrics = BTreeMap::new();

        // 占位符实现：收集内存使用量
        metrics.insert(
            "memory_usage".to_string(),
            PerformanceMetric {
                name: "memory_usage".to_string(),
                metric_type: MetricType::Gauge,
                value: MetricValue::Integer(0),
                unit: "bytes".to_string(),
                timestamp: nos_api::event::get_time_ns(),
                tags: BTreeMap::new(),
            },
        );

        // 占位符实现：收集内存使用率
        metrics.insert(
            "memory_usage_percent".to_string(),
            PerformanceMetric {
                name: "memory_usage_percent".to_string(),
                metric_type: MetricType::Gauge,
                value: MetricValue::Float(0.0),
                unit: "percent".to_string(),
                timestamp: nos_api::event::get_time_ns(),
                tags: BTreeMap::new(),
            },
        );

        Ok(metrics)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Memory performance collector"
    }
}

/// 系统调用性能收集器
pub struct SyscallPerformanceCollector {
    name: String,
}

impl SyscallPerformanceCollector {
    /// 创建新的系统调用性能收集器
    pub fn new() -> Self {
        Self { name: "syscall".to_string() }
    }
}

impl PerformanceCollector for SyscallPerformanceCollector {
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let mut metrics = BTreeMap::new();

        // 占位符实现：收集系统调用总数
        metrics.insert(
            "syscall_total".to_string(),
            PerformanceMetric {
                name: "syscall_total".to_string(),
                metric_type: MetricType::Counter,
                value: MetricValue::Integer(0),
                unit: "count".to_string(),
                timestamp: nos_api::event::get_time_ns(),
                tags: BTreeMap::new(),
            },
        );

        // 占位符实现：收集系统调用错误数
        metrics.insert(
            "syscall_errors".to_string(),
            PerformanceMetric {
                name: "syscall_errors".to_string(),
                metric_type: MetricType::Counter,
                value: MetricValue::Integer(0),
                unit: "count".to_string(),
                timestamp: nos_api::event::get_time_ns(),
                tags: BTreeMap::new(),
            },
        );

        Ok(metrics)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "System call performance collector"
    }
}

/// 全局性能监控器
static mut GLOBAL_PERFORMANCE_MONITOR: Option<Arc<PerformanceMonitor>> = None;
static PERFORMANCE_MONITOR_INIT: Mutex<bool> = Mutex::new(false);

/// 初始化全局性能监控器
pub fn init_performance_monitor() -> Result<()> {
    let mut is_init = PERFORMANCE_MONITOR_INIT.lock();
    if *is_init {
        return Ok(());
    }

    let monitor = Arc::new(PerformanceMonitor::new());

    // 添加默认收集器
    monitor.add_collector(Arc::new(CpuPerformanceCollector::new()));
    monitor.add_collector(Arc::new(MemoryPerformanceCollector::new()));
    monitor.add_collector(Arc::new(SyscallPerformanceCollector::new()));

    unsafe {
        GLOBAL_PERFORMANCE_MONITOR = Some(monitor);
    }
    *is_init = true;
    Ok(())
}

/// 获取全局性能监控器
pub fn get_performance_monitor() -> Arc<PerformanceMonitor> {
    unsafe {
        GLOBAL_PERFORMANCE_MONITOR
            .as_ref()
            .expect("Performance monitor not initialized")
            .clone()
    }
}

/// 收集性能指标
pub fn collect_performance_metrics() -> Result<BTreeMap<String, PerformanceMetric>> {
    get_performance_monitor().collect_metrics()
}

/// 获取特定性能指标
pub fn get_performance_metric(name: &str) -> Option<PerformanceMetric> {
    get_performance_monitor().get_metric(name)
}

/// 获取所有性能指标
pub fn get_all_performance_metrics() -> BTreeMap<String, PerformanceMetric> {
    get_performance_monitor().get_all_metrics()
}

/// 清除所有性能指标
pub fn clear_performance_metrics() {
    get_performance_monitor().clear_metrics()
}
