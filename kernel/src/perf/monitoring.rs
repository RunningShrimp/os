//! 性能监控模块
//!
//! 提供性能统计和监控功能

use alloc::collections::BTreeMap;
use core::sync::atomic;

use spin::Mutex;

/// 性能统计信息
#[derive(Debug, Clone)]
pub struct PerfStats {
    /// 总操作次数
    pub total_operations: u64,
    /// 成功操作次数
    pub successful_operations: u64,
    /// 失败操作次数
    pub failed_operations: u64,
    /// 总执行时间（纳秒）
    pub total_time_ns: u64,
    /// 最小执行时间（纳秒）
    pub min_time_ns: u64,
    /// 最大执行时间（纳秒）
    pub max_time_ns: u64,
}

impl Default for PerfStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            total_time_ns: 0,
            min_time_ns: u64::MAX,
            max_time_ns: 0,
        }
    }
}

impl PerfStats {
    /// 创建新的性能统计
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次操作
    pub fn record_operation(&mut self, success: bool, time_ns: u64) {
        self.total_operations += 1;
        if success {
            self.successful_operations += 1;
        } else {
            self.failed_operations += 1;
        }
        self.total_time_ns += time_ns;
        self.min_time_ns = self.min_time_ns.min(time_ns);
        self.max_time_ns = self.max_time_ns.max(time_ns);
    }

    /// 获取平均执行时间
    pub fn average_time_ns(&self) -> u64 {
        if self.total_operations == 0 {
            0
        } else {
            self.total_time_ns / self.total_operations
        }
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            1.0
        } else {
            self.successful_operations as f64 / self.total_operations as f64
        }
    }
}

/// 性能统计映射
static PERF_STATS: spin::Mutex<BTreeMap<&'static str, AtomicU64>> =
    spin::Mutex::new(BTreeMap::new());

/// 获取性能统计信息
pub fn get_perf_stats(name: &'static str) -> u64 {
    let stats = PERF_STATS.lock();
    stats
        .get(name)
        .map(|v| v.load(Ordering::Relaxed))
        .unwrap_or(0)
}

/// 设置性能统计信息
pub fn set_perf_stats(name: &'static str, value: u64) {
    let mut stats = PERF_STATS.lock();
    stats
        .entry(name)
        .or_insert_with(|| AtomicU64::new(value))
        .store(value, Ordering::Relaxed);
}

/// 增加性能统计信息
pub fn increment_perf_stats(name: &'static str, delta: u64) {
    let mut stats = PERF_STATS.lock();
    stats
        .entry(name)
        .or_insert_with(|| AtomicU64::new(0))
        .fetch_add(delta, Ordering::Relaxed);
}

/// 重置所有性能统计
pub fn reset_perf_stats() {
    let mut stats = PERF_STATS.lock();
    stats.clear();
}
