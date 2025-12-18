//! 性能监控和统计系统
//!
//! 本模块提供系统性能监控和统计功能，包括：
//! - 系统调用性能统计
//! - 资源使用统计
//! - 性能指标收集
//! - 性能报告生成

// TODO: Re-enable these imports when optimization modules are refactored
// use crate::syscalls::file_io_optimized::{get_io_stats, IoStats};
// use crate::syscalls::process_optimized::{get_proc_stats, ProcStats};
// use crate::syscalls::memory_optimized::{get_mem_stats, MemStats};
// use crate::syscalls::signal_optimized::{get_signal_stats, SignalStats};
use crate::sync::Mutex;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

/// 全局性能统计
static PERF_STATS: Mutex<PerfStats> = Mutex::new(PerfStats::new());

/// 性能统计信息
#[derive(Debug, Default)]
pub struct PerfStats {
    pub syscall_count: AtomicU64,
    pub total_syscall_time: AtomicU64,
    pub max_syscall_time: AtomicU64,
    pub min_syscall_time: AtomicU64,
    pub context_switches: AtomicU64,
    pub interrupts: AtomicU64,
    pub page_faults: AtomicU64,
}
impl Clone for PerfStats {
    fn clone(&self) -> Self {
        Self {
            syscall_count: AtomicU64::new(self.syscall_count.load(core::sync::atomic::Ordering::Relaxed)),
            total_syscall_time: AtomicU64::new(self.total_syscall_time.load(core::sync::atomic::Ordering::Relaxed)),
            max_syscall_time: AtomicU64::new(self.max_syscall_time.load(core::sync::atomic::Ordering::Relaxed)),
            min_syscall_time: AtomicU64::new(self.min_syscall_time.load(core::sync::atomic::Ordering::Relaxed)),
            context_switches: AtomicU64::new(self.context_switches.load(core::sync::atomic::Ordering::Relaxed)),
            interrupts: AtomicU64::new(self.interrupts.load(core::sync::atomic::Ordering::Relaxed)),
            page_faults: AtomicU64::new(self.page_faults.load(core::sync::atomic::Ordering::Relaxed)),
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
            context_switches: AtomicU64::new(0),
            interrupts: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
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
/// TODO: Re-enable when optimization modules are refactored
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
    
    pub fn generate_text_report(&self) -> String {
        let mut report = alloc::string::String::new();
        report.push_str("# NOS系统性能报告\n\n");
        report.push_str(&alloc::format!("生成时间: {}\n\n", self.timestamp));
        report.push_str("## 性能统计\n");
        report.push_str(&alloc::format!("- 系统调用次数: {}\n", self.perf_stats.syscall_count.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 平均系统调用时间: {:.2} μs\n", self.perf_stats.get_average_syscall_time()));
        report.push_str(&alloc::format!("- 最大系统调用时间: {} μs\n", self.perf_stats.max_syscall_time.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 最小系统调用时间: {} μs\n", self.perf_stats.min_syscall_time.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 上下文切换次数: {}\n", self.perf_stats.context_switches.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 中断次数: {}\n", self.perf_stats.interrupts.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("- 页面错误次数: {}\n", self.perf_stats.page_faults.load(Ordering::Relaxed)));
        report
    }
    
    pub fn generate_json_report(&self) -> alloc::string::String {
        let mut report = alloc::string::String::new();
        report.push_str("{\n");
        report.push_str(&alloc::format!("  \"timestamp\": {},\n", self.timestamp));
        report.push_str("  \"perf_stats\": {\n");
        report.push_str(&alloc::format!("    \"syscall_count\": {},\n", self.perf_stats.syscall_count.load(Ordering::Relaxed)));
        report.push_str(&alloc::format!("    \"avg_time\": {:.2}\n", self.perf_stats.get_average_syscall_time()));
        report.push_str("  }\n");
        report.push_str("}\n");
        report
    }
}
        

/// 获取当前时间戳
fn get_current_timestamp() -> u64 {
    0  // TODO: Get from system clock
}

/// 全局性能统计
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
