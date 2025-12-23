// Copyright (c) 2024 NOS Community
// SPDX-License-Identifier: Apache-2.0

//! Performance monitoring and statistics system.
//!
//! This module provides system performance monitoring and statistics functionality,
//! including system call performance statistics, resource usage statistics,
//! performance metrics collection, and performance report generation.

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
    boxed::Box,
    format,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::core::UnifiedSyscallStats;

/// 全局性能统计
static PERF_STATS: Mutex<PerfStats> = Mutex::new(PerfStats::new());

/// 性能统计信息
#[derive(Debug, Default)]
pub struct PerfStats {
    // System call metrics
    pub syscall_count: AtomicU64,
    pub total_syscall_time: AtomicU64,
    pub max_syscall_time: AtomicU64,
    pub min_syscall_time: AtomicU64,
    pub syscall_per_component: BTreeMap<u32, UnifiedSyscallStats>,

    // Cache metrics
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,

    // CPU metrics
    pub cpu_utilization: AtomicU64, // Percentage * 100 to avoid floating point
    pub context_switches: AtomicU64,
    pub interrupts: AtomicU64,

    // Memory metrics
    pub memory_pressure: AtomicU64, // Percentage * 100
    pub page_faults: AtomicU64,
    pub swap_usage: AtomicU64,      // Percentage * 100

    // I/O metrics
    pub io_throughput_read: AtomicU64,  // Bytes per second
    pub io_throughput_write: AtomicU64, // Bytes per second
    pub io_operations: AtomicU64,        // Total I/O operations
}

impl Clone for PerfStats {
    fn clone(&self) -> Self {
        Self {
            syscall_count: AtomicU64::new(self.syscall_count.load(core::sync::atomic::Ordering::Relaxed)),
            total_syscall_time: AtomicU64::new(self.total_syscall_time.load(core::sync::atomic::Ordering::Relaxed)),
            max_syscall_time: AtomicU64::new(self.max_syscall_time.load(core::sync::atomic::Ordering::Relaxed)),
            min_syscall_time: AtomicU64::new(self.min_syscall_time.load(core::sync::atomic::Ordering::Relaxed)),
            syscall_per_component: self.syscall_per_component.clone(),
            cache_hits: AtomicU64::new(self.cache_hits.load(core::sync::atomic::Ordering::Relaxed)),
            cache_misses: AtomicU64::new(self.cache_misses.load(core::sync::atomic::Ordering::Relaxed)),
            cpu_utilization: AtomicU64::new(self.cpu_utilization.load(core::sync::atomic::Ordering::Relaxed)),
            context_switches: AtomicU64::new(self.context_switches.load(core::sync::atomic::Ordering::Relaxed)),
            interrupts: AtomicU64::new(self.interrupts.load(core::sync::atomic::Ordering::Relaxed)),
            memory_pressure: AtomicU64::new(self.memory_pressure.load(core::sync::atomic::Ordering::Relaxed)),
            page_faults: AtomicU64::new(self.page_faults.load(core::sync::atomic::Ordering::Relaxed)),
            swap_usage: AtomicU64::new(self.swap_usage.load(core::sync::atomic::Ordering::Relaxed)),
            io_throughput_read: AtomicU64::new(self.io_throughput_read.load(core::sync::atomic::Ordering::Relaxed)),
            io_throughput_write: AtomicU64::new(self.io_throughput_write.load(core::sync::atomic::Ordering::Relaxed)),
            io_operations: AtomicU64::new(self.io_operations.load(core::sync::atomic::Ordering::Relaxed)),
        }
    }
}

impl PerfStats {
    pub const fn new() -> Self {
        Self {
            syscall_count: AtomicU64::new(0),
            total_syscall_time: AtomicU64::new(0),
            max_syscall_time: AtomicU64::new(0),
            min_syscall_time: AtomicU64::new(u64::MAX),
            syscall_per_component: BTreeMap::new(),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cpu_utilization: AtomicU64::new(0),
            context_switches: AtomicU64::new(0),
            interrupts: AtomicU64::new(0),
            memory_pressure: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
            swap_usage: AtomicU64::new(0),
            io_throughput_read: AtomicU64::new(0),
            io_throughput_write: AtomicU64::new(0),
            io_operations: AtomicU64::new(0),
        }
    }
    
    pub fn record_syscall(&self, duration: u64) {
        self.syscall_count.fetch_add(1, Ordering::Relaxed);
        self.total_syscall_time.fetch_add(duration, Ordering::Relaxed);
        
        // 更新最大值
        let mut current_max = self.max_syscall_time.load(Ordering::Relaxed);
        while duration > current_max {
            match self.max_syscall_time.compare_exchange_weak(
                current_max, 
                duration, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
        
        // 更新最小值
        let mut current_min = self.min_syscall_time.load(Ordering::Relaxed);
        while duration < current_min {
            match self.min_syscall_time.compare_exchange_weak(
                current_min, 
                duration, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }
    }
    
    pub fn record_context_switch(&self) {
        self.context_switches.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_interrupt(&self) {
        self.interrupts.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_page_fault(&self) {
        self.page_faults.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_average_syscall_time(&self) -> f64 {
        let count = self.syscall_count.load(Ordering::Relaxed);
        let total = self.total_syscall_time.load(Ordering::Relaxed);
        
        if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        }
    }
}

/// 系统性能报告
#[derive(Debug)]
pub struct SystemPerformanceReport {
    pub timestamp: u64,
    pub perf_stats: PerfStats,
}

impl SystemPerformanceReport {
    pub fn new() -> Self {
        Self {
            timestamp: get_current_timestamp(),
            perf_stats: get_perf_stats(),
        }
    }
    
    /// Generate summary report
    pub fn generate_summary_report(&self) -> String {
        let mut report = alloc::string::String::new();
        report.push_str("# NOS系统性能报告\n\n");
        report.push_str(&alloc::format!("生成时间: {}\n\n", self.timestamp));
        
        // Summary
        report.push_str("## 性能摘要\n");
        report.push_str(&alloc::format!("- 系统调用次数: {}\n", self.perf_stats.syscall_count.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 平均系统调用时间: {:.2} μs\n", self.perf_stats.get_average_syscall_time()));
        
        let cache_hits = self.perf_stats.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.perf_stats.cache_misses.load(Ordering::Relaxed);
        let total_cache = cache_hits + cache_misses;
        let cache_hit_rate = if total_cache > 0 { (cache_hits * 100) / total_cache } else { 0 };
        report.push_str(&alloc::format!("- 缓存命中率: {}%\n", cache_hit_rate));
        
        let cpu_utilization = self.perf_stats.cpu_utilization.load(Ordering::Relaxed);
        report.push_str(&alloc::format!("- CPU利用率: {}.{:02}%\n", cpu_utilization / 100, cpu_utilization % 100));
        
        let memory_pressure = self.perf_stats.memory_pressure.load(Ordering::Relaxed);
        report.push_str(&alloc::format!("- 内存压力: {}.{:02}%\n", memory_pressure / 100, memory_pressure % 100));
        
        report.push_str(&alloc::format!("- I/O吞吐量: {} B/s\n", self.perf_stats.io_throughput_read.load(Ordering::Relaxed) + self.perf_stats.io_throughput_write.load(Ordering::Relaxed)));
        report
    }
    
    /// Generate detailed per-component report
    pub fn generate_detailed_report(&self) -> String {
        let mut report = alloc::string::String::new();
        report.push_str("# NOS系统详细性能报告\n\n");
        report.push_str(&alloc::format!("生成时间: {}\n\n", self.timestamp));
        
        // System calls by component
        report.push_str("## 系统调用性能 (按组件)\n");
        for (syscall_num, stats) in &self.perf_stats.syscall_per_component {
            let snapshot = stats.get_snapshot();
            report.push_str(&alloc::format!("\n### 系统调用 {}:\n", syscall_num));
            report.push_str(&alloc::format!("- 调用次数: {}\n", snapshot.call_count));
            report.push_str(&alloc::format!("- 平均时间: {:.2} ns\n", snapshot.average_time_ns));
            report.push_str(&alloc::format!("- 缓存命中率: {:.2}%\n", snapshot.cache_hit_rate * 100.0));
            report.push_str(&alloc::format!("- 错误率: {:.2}%\n", snapshot.error_rate * 100.0));
        }
        
        // Cache detailed
        report.push_str("\n## 缓存详细信息\n");
        let cache_hits = self.perf_stats.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.perf_stats.cache_misses.load(Ordering::Relaxed);
        let total_cache = cache_hits + cache_misses;
        report.push_str(&alloc::format!("- 缓存命中: {}\n", cache_hits));
        report.push_str(&alloc::format!("- 缓存未命中: {}\n", cache_misses));
        report.push_str(&alloc::format!("- 总缓存访问: {}\n", total_cache));
        
        // CPU detailed
        report.push_str("\n## CPU 详细信息\n");
        report.push_str(&alloc::format!("- 上下文切换次数: {}\n", self.perf_stats.context_switches.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 中断次数: {}\n", self.perf_stats.interrupts.load(Ordering::Relaxed)));
        
        // Memory detailed
        report.push_str("\n## 内存详细信息\n");
        report.push_str(&alloc::format!("- 页面错误次数: {}\n", self.perf_stats.page_faults.load(Ordering::Relaxed)));
        
        // I/O detailed
        report.push_str("\n## I/O 详细信息\n");
        report.push_str(&alloc::format!("- 读吞吐量: {} B/s\n", self.perf_stats.io_throughput_read.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 写吞吐量: {} B/s\n", self.perf_stats.io_throughput_write.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- I/O 操作数: {}\n", self.perf_stats.io_operations.load(Ordering::Relaxed)));
        
        report
    }
    
    pub fn generate_json_report(&self) -> alloc::string::String {
        let mut report = alloc::string::String::new();
        report.push_str("{\n");
        report.push_str(&alloc::format!("  \"timestamp\": {},\n", self.timestamp));
        
        // Summary
        report.push_str("  \"summary\": {\n");
        report.push_str(&alloc::format!("    \"syscall_count\": {},\n", self.perf_stats.syscall_count.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("    \"avg_syscall_time_us\": {:.2},\n", self.perf_stats.get_average_syscall_time()));
        
        let cache_hits = self.perf_stats.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.perf_stats.cache_misses.load(Ordering::Relaxed);
        let total_cache = cache_hits + cache_misses;
        let cache_hit_rate = if total_cache > 0 { cache_hits as f64 / total_cache as f64 } else { 0.0 };
        report.push_str(&alloc::format!("    \"cache_hit_rate\": {:.2},\n", cache_hit_rate));
        
        let cpu_utilization = self.perf_stats.cpu_utilization.load(Ordering::Relaxed);
        report.push_str(&alloc::format!("    \"cpu_utilization\": {}.{:02},\n", cpu_utilization / 100, cpu_utilization % 100));
        
        let memory_pressure = self.perf_stats.memory_pressure.load(Ordering::Relaxed);
        report.push_str(&alloc::format!("    \"memory_pressure\": {}.{:02},\n", memory_pressure / 100, memory_pressure % 100));
        
        let total_io = self.perf_stats.io_throughput_read.load(Ordering::Relaxed) + self.perf_stats.io_throughput_write.load(Ordering::Relaxed);
        report.push_str(&alloc::format!("    \"total_io_throughput_bps\": {}\n", total_io));
        report.push_str("  },\n");
        
        // Detailed component stats
        report.push_str("  \"detailed\": {\n");
        
        // System call per component
        report.push_str("    \"syscall_per_component\": {\n");
        let mut first_entry = true;
        for (syscall_num, stats) in &self.perf_stats.syscall_per_component {
            if !first_entry {
                report.push_str(",\n");
            }
            first_entry = false;
            let snapshot = stats.get_snapshot();
            report.push_str(&alloc::format!("      \"{}\": {{ \"call_count\": {}, \"avg_time_ns\": {:.2} }}",
                syscall_num, snapshot.call_count, snapshot.average_time_ns));
        }
        report.push_str("\n    },\n");
        
        // Cache detailed
        report.push_str("    \"cache\": {\n");
        report.push_str(&alloc::format!("      \"hits\": {},\n", cache_hits));
        report.push_str(&alloc::format!("      \"misses\": {}\n", cache_misses));
        report.push_str("    },\n");
        
        // CPU detailed
        report.push_str("    \"cpu\": {\n");
        report.push_str(&alloc::format!("      \"context_switches\": {},\n", self.perf_stats.context_switches.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("      \"interrupts\": {}\n", self.perf_stats.interrupts.load(Ordering::Relaxed)));
        report.push_str("    },\n");
        
        // Memory detailed
        report.push_str("    \"memory\": {\n");
        report.push_str(&alloc::format!("      \"page_faults\": {},\n", self.perf_stats.page_faults.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("      \"memory_pressure\": {}\n", self.perf_stats.memory_pressure.load(Ordering::Relaxed)));
        report.push_str("    },\n");
        
        // I/O detailed
        report.push_str("    \"io\": {\n");
        report.push_str(&alloc::format!("      \"read_throughput_bps\": {},\n", self.perf_stats.io_throughput_read.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("      \"write_throughput_bps\": {},\n", self.perf_stats.io_throughput_write.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("      \"operations\": {}\n", self.perf_stats.io_operations.load(Ordering::Relaxed)));
        report.push_str("    }\n");
        
        report.push_str("  }\n");
        report.push_str("}\n");
        report
    }
    
    /// Generate trend analysis data
    pub fn generate_trend_data(&self) -> alloc::vec::Vec<u64> {
        alloc::vec![
            self.timestamp,
            self.perf_stats.syscall_count.load(Ordering::Relaxed),
            (self.perf_stats.get_average_syscall_time() * 1000.0) as u64, // Convert to ns
            self.perf_stats.cache_hits.load(Ordering::Relaxed),
            self.perf_stats.cache_misses.load(Ordering::Relaxed),
            self.perf_stats.cpu_utilization.load(Ordering::Relaxed),
            self.perf_stats.memory_pressure.load(Ordering::Relaxed),
            self.perf_stats.io_throughput_read.load(Ordering::Relaxed) + self.perf_stats.io_throughput_write.load(Ordering::Relaxed)
        ]
    }
}
        

/// 获取当前时间戳
fn get_current_timestamp() -> u64 {
    // TODO: Implement with real time source
    0
}

/// 获取全局性能统计
pub fn get_perf_stats() -> PerfStats {
    PERF_STATS.lock().clone()
}

/// 记录系统调用性能
pub fn record_syscall_performance(duration: u64) {
    PERF_STATS.lock().record_syscall(duration);
}

/// 记录上下文切换
pub fn record_context_switch() {
    PERF_STATS.lock().record_context_switch();
}

/// 记录中断
pub fn record_interrupt() {
    PERF_STATS.lock().record_interrupt();
}

/// 记录页面错误

/// 性能监控配置
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub enabled_metrics: MetricFlags,
    pub report_interval_ms: u64,
    pub alert_thresholds: AlertThresholds,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled_metrics: MetricFlags::all(),
            report_interval_ms: 1000, // 1 second
            alert_thresholds: AlertThresholds::default(),
        }
    }
}


bitflags::bitflags! {
    #[derive(Debug, Clone)]
    pub struct MetricFlags: u32 {
        const SYSCALL = 0b00000001;
        const CACHE = 0b00000010;
        const CPU = 0b00000100;
        const MEMORY = 0b00001000;
        const IO = 0b00010000;
    }
}

/// 告警阈值
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub high_cpu_utilization: u64, // Percentage * 100
    pub high_memory_pressure: u64, // Percentage * 100
    pub high_io_throughput: u64,   // Bytes per second
    pub low_cache_hit_rate: u64,   // Percentage * 100
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            high_cpu_utilization: 9000, // 90%
            high_memory_pressure: 9000, // 90%
            high_io_throughput: 100_000_000, // 100 MB/s
            low_cache_hit_rate: 2000,    // 20%
        }
    }
}
pub fn record_page_fault() {
    PERF_STATS.lock().record_page_fault();
}

pub fn perf_stats_json() -> alloc::string::String {
    let stats = get_perf_stats();
    let mut s = alloc::string::String::new();
    s.push_str("{\n");
    s.push_str(&alloc::format!("  \"syscall_count\": {},\n", stats.syscall_count.load(Ordering::Relaxed)));
    s.push_str(&alloc::format!("  \"avg_time_us\": {:.2},\n", stats.get_average_syscall_time()));
    s.push_str(&alloc::format!("  \"max_time_us\": {},\n", stats.max_syscall_time.load(Ordering::Relaxed)));
    s.push_str(&alloc::format!("  \"min_time_us\": {},\n", stats.min_syscall_time.load(Ordering::Relaxed)));
    s.push_str(&alloc::format!("  \"context_switches\": {},\n", stats.context_switches.load(Ordering::Relaxed)));
    s.push_str(&alloc::format!("  \"interrupts\": {},\n", stats.interrupts.load(Ordering::Relaxed)));
    s.push_str(&alloc::format!("  \"page_faults\": {}\n", stats.page_faults.load(Ordering::Relaxed)));
    s.push_str("}\n");
    s
}

/// 周期性刷新机制
pub trait MetricFlush: Send + Sync {
    fn flush(&self, report: &SystemPerformanceReport) -> Result<(), &'static str>;
}

/// 全局监控配置
static MONITORING_CONFIG: Mutex<Option<MonitoringConfig>> = Mutex::new(None);

/// 获取监控配置
fn get_monitoring_config() -> MonitoringConfig {
    let mut config = MONITORING_CONFIG.lock();
    if config.is_none() {
        *config = Some(MonitoringConfig::default());
    }
    config.as_ref().unwrap().clone()
}

/// 刷新器列表
static FLUSHERS: Mutex<Vec<Box<dyn MetricFlush>>> = Mutex::new(Vec::new());

/// 注册刷新器
pub fn register_flusher(flusher: Box<dyn MetricFlush>) {
    FLUSHERS.lock().push(flusher);
}

/// 设置监控配置
pub fn set_monitoring_config(config: MonitoringConfig) {
    *MONITORING_CONFIG.lock() = Some(config);
}

/// 触发所有刷新器
pub fn flush_metrics() {
    let report = SystemPerformanceReport::new();
    for flusher in FLUSHERS.lock().iter() {
        let _ = flusher.flush(&report) as Result<(), &'static str>;
    }
}