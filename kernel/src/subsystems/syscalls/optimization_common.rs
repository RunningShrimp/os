//! 通用优化模块
//!
//! 提供系统调用优化的通用组件，包括：
//! - 统一的统计信息收集
//! - 通用的缓存机制
//! - 共享的性能监控工具
//! - 标准化的优化策略

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
// removed unused imports
use spin::Mutex;

use crate::syscalls::optimization_core::{
    UnifiedSyscallStats, SyscallStatsSnapshot, OptimizationConfig
};

/// 通用文件描述符验证器
pub struct FileDescriptorValidator;

impl FileDescriptorValidator {
    /// 验证文件描述符是否有效
    pub fn is_valid_fd(fd: i32) -> bool {
        fd >= 0 && (fd as usize) < crate::process::NOFILE
    }

    /// 验证文件描述符范围
    pub fn validate_fd_range(fd: i32) -> Result<(), crate::syscalls::common::SyscallError> {
        if fd < 0 {
            Err(crate::syscalls::common::SyscallError::BadFileDescriptor)
        } else if (fd as usize) >= crate::process::NOFILE {
            Err(crate::syscalls::common::SyscallError::BadFileDescriptor)
        } else {
            Ok(())
        }
    }

    /// 获取进程的文件描述符索引
    pub fn get_file_index(fd: i32) -> Result<usize, crate::syscalls::common::SyscallError> {
        Self::validate_fd_range(fd)?;
        
        let pid = crate::process::myproc().ok_or(crate::syscalls::common::SyscallError::InvalidArgument)?;
        let proc_table = crate::process::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(crate::syscalls::common::SyscallError::InvalidArgument)?;
        
        proc.ofile[fd as usize].ok_or(crate::syscalls::common::SyscallError::BadFileDescriptor)
    }
}

/// 通用性能监控器
pub struct PerformanceMonitor {
    /// 全局统计信息
    global_stats: UnifiedSyscallStats,
    /// 各系统调用的统计信息
    syscall_stats: Arc<Mutex<BTreeMap<u32, UnifiedSyscallStats>>>,
    /// 配置
    config: OptimizationConfig,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            global_stats: UnifiedSyscallStats::new(),
            syscall_stats: Arc::new(Mutex::new(BTreeMap::new())),
            config,
        }
    }

    /// 记录系统调用性能
    pub fn record_syscall_performance(
        &self,
        syscall_num: u32,
        duration_ns: u64,
        success: bool,
        cache_hit: bool,
    ) {
        // 记录全局统计
        self.global_stats.record_call(duration_ns, success, cache_hit);
        
        // 记录特定系统调用统计
        let mut stats_map = self.syscall_stats.lock();
        let stats = stats_map.entry(syscall_num).or_insert_with(UnifiedSyscallStats::new);
        stats.record_call(duration_ns, success, cache_hit);
    }

    /// 获取全局统计信息
    pub fn get_global_stats(&self) -> SyscallStatsSnapshot {
        self.global_stats.get_snapshot()
    }

    /// 获取特定系统调用的统计信息
    pub fn get_syscall_stats(&self, syscall_num: u32) -> Option<SyscallStatsSnapshot> {
        let stats_map = self.syscall_stats.lock();
        stats_map.get(&syscall_num).map(|stats| stats.get_snapshot())
    }

    /// 获取所有系统调用统计信息
    pub fn get_all_syscall_stats(&self) -> BTreeMap<u32, SyscallStatsSnapshot> {
        let stats_map = self.syscall_stats.lock();
        stats_map.iter().map(|(num, stats)| (*num, stats.get_snapshot())).collect()
    }

    /// 重置所有统计信息
    pub fn reset_all_stats(&self) {
        self.global_stats.reset();
        let mut stats_map = self.syscall_stats.lock();
        for stats in stats_map.values_mut() {
            stats.reset();
        }
    }

    /// 判断是否应该使用快速路径
    pub fn should_use_fast_path(&self, syscall_num: u32) -> bool {
        if let Some(stats) = self.get_syscall_stats(syscall_num) {
            stats.average_time_ns > self.config.fast_path_threshold_ns as f64
        } else {
            false
        }
    }

    /// 判断是否应该使用批处理
    pub fn should_use_batch(&self, consecutive_calls: usize) -> bool {
        self.config.enable_batch && consecutive_calls >= self.config.batch_threshold
    }

    /// 判断是否应该缓存结果
    pub fn should_cache_result(&self, syscall_num: u32) -> bool {
        self.config.enable_cache && self.is_pure_syscall(syscall_num)
    }

    /// 判断是否为纯函数系统调用
    fn is_pure_syscall(&self, syscall_num: u32) -> bool {
        match syscall_num {
            0x1004 | 0x1005 => true, // getpid, getppid
            _ => false,
        }
    }
}

/// 全局性能监控器实例
static GLOBAL_PERFORMANCE_MONITOR: Mutex<Option<PerformanceMonitor>> = Mutex::new(None);

/// 初始化全局性能监控器
pub fn init_global_performance_monitor(config: OptimizationConfig) {
    let mut monitor_guard = GLOBAL_PERFORMANCE_MONITOR.lock();
    *monitor_guard = Some(PerformanceMonitor::new(config));
}

/// 获取全局性能监控器
pub fn get_global_performance_monitor() -> Option<&'static Mutex<Option<PerformanceMonitor>>> {
    Some(&GLOBAL_PERFORMANCE_MONITOR)
}

/// 便捷函数：记录系统调用性能
pub fn record_syscall_performance(syscall_num: u32, duration_ns: u64, success: bool, cache_hit: bool) {
    if let Some(monitor_guard) = get_global_performance_monitor() {
        let monitor = monitor_guard.lock();
        if let Some(ref monitor) = *monitor {
            monitor.record_syscall_performance(syscall_num, duration_ns, success, cache_hit);
        }
    }
}






