//! 优化的进程表锁定策略
//!
//! 提供高效的进程表锁定机制，包括：
//! - 减少进程表锁的粒度
//! - 实现读写锁替代互斥锁
//! - 优化锁竞争和并发性能
//! - 支持细粒度锁定

extern crate alloc;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::sync::RwLock;

/// 锁定统计信息
#[derive(Debug)]
pub struct LockStats {
    /// 读锁获取次数
    pub read_lock_acquires: AtomicUsize,
    /// 写锁获取次数
    pub write_lock_acquires: AtomicUsize,
    /// 读锁释放次数
    pub read_lock_releases: AtomicUsize,
    /// 写锁释放次数
    pub write_lock_releases: AtomicUsize,
    /// 锁竞争次数
    pub contentions: AtomicUsize,
    /// 平均等待时间（纳秒）
    pub avg_wait_time_ns: AtomicUsize,
    /// 最大等待时间（纳秒）
    pub max_wait_time_ns: AtomicUsize,
}

impl LockStats {
    #[inline]
    pub const fn new() -> Self {
        Self {
            read_lock_acquires: AtomicUsize::new(0),
            write_lock_acquires: AtomicUsize::new(0),
            read_lock_releases: AtomicUsize::new(0),
            write_lock_releases: AtomicUsize::new(0),
            contentions: AtomicUsize::new(0),
            avg_wait_time_ns: AtomicUsize::new(0),
            max_wait_time_ns: AtomicUsize::new(0),
        }
    }

    /// 记录读锁获取
    #[inline]
    pub fn record_read_lock_acquire(&self) {
        self.read_lock_acquires.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录写锁获取
    #[inline]
    pub fn record_write_lock_acquire(&self) {
        self.write_lock_acquires.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录读锁释放
    #[inline]
    pub fn record_read_lock_release(&self) {
        self.read_lock_releases.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录写锁释放
    #[inline]
    pub fn record_write_lock_release(&self) {
        self.write_lock_releases.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录锁竞争
    #[inline]
    pub fn record_contention(&self, wait_time_ns: u64) {
        self.contentions.fetch_add(1, Ordering::Relaxed);
        
        // 更新平均等待时间
        let current_avg = self.avg_wait_time_ns.load(Ordering::Relaxed);
        let new_avg = (current_avg as u64 + wait_time_ns) / 2;
        self.avg_wait_time_ns.store(new_avg as usize, Ordering::Relaxed);
        
        // 更新最大等待时间
        let current_max = self.max_wait_time_ns.load(Ordering::Relaxed);
        if wait_time_ns as usize > current_max {
            self.max_wait_time_ns.store(wait_time_ns as usize, Ordering::Relaxed);
        }
    }

    /// 获取统计快照
    pub fn get_snapshot(&self) -> LockStatsSnapshot {
        LockStatsSnapshot {
            read_lock_acquires: self.read_lock_acquires.load(Ordering::Relaxed),
            write_lock_acquires: self.write_lock_acquires.load(Ordering::Relaxed),
            read_lock_releases: self.read_lock_releases.load(Ordering::Relaxed),
            write_lock_releases: self.write_lock_releases.load(Ordering::Relaxed),
            contentions: self.contentions.load(Ordering::Relaxed),
            avg_wait_time_ns: self.avg_wait_time_ns.load(Ordering::Relaxed),
            max_wait_time_ns: self.max_wait_time_ns.load(Ordering::Relaxed),
        }
    }

    /// 重置统计信息
    pub fn reset(&self) {
        self.read_lock_acquires.store(0, Ordering::Relaxed);
        self.write_lock_acquires.store(0, Ordering::Relaxed);
        self.read_lock_releases.store(0, Ordering::Relaxed);
        self.write_lock_releases.store(0, Ordering::Relaxed);
        self.contentions.store(0, Ordering::Relaxed);
        self.avg_wait_time_ns.store(0, Ordering::Relaxed);
        self.max_wait_time_ns.store(0, Ordering::Relaxed);
    }
}

impl Default for LockStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 锁定统计快照
#[derive(Debug, Clone)]
pub struct LockStatsSnapshot {
    pub read_lock_acquires: usize,
    pub write_lock_acquires: usize,
    pub read_lock_releases: usize,
    pub write_lock_releases: usize,
    pub contentions: usize,
    pub avg_wait_time_ns: usize,
    pub max_wait_time_ns: usize,
}

impl LockStatsSnapshot {
    /// 计算锁竞争率
    #[inline]
    pub fn contention_rate(&self) -> f64 {
        let total_acquires = self.read_lock_acquires + self.write_lock_acquires;
        if total_acquires == 0 {
            0.0
        } else {
            self.contentions as f64 / total_acquires as f64
        }
    }

    /// 计算锁利用率
    #[inline]
    pub fn utilization_rate(&self) -> f64 {
        let total_acquires = self.read_lock_acquires + self.write_lock_acquires;
        let total_releases = self.read_lock_releases + self.write_lock_releases;
        if total_acquires == 0 {
            0.0
        } else {
            total_releases as f64 / total_acquires as f64
        }
    }
}

/// 锁定策略配置
#[derive(Debug, Clone)]
pub struct LockConfig {
    /// 是否启用读写锁
    pub enable_rw_locks: bool,
    /// 是否启用细粒度锁定
    pub enable_fine_grained: bool,
    /// 是否启用自适应锁策略
    pub enable_adaptive: bool,
    /// 锁升级超时时间（纳秒）
    pub lock_upgrade_timeout_ns: u64,
    /// 最大读锁数量
    pub max_readers: usize,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            enable_rw_locks: true,
            enable_fine_grained: true,
            enable_adaptive: true,
            lock_upgrade_timeout_ns: 1_000_000, // 1ms
            max_readers: 16,
        }
    }
}

/// 细粒度锁类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FineGrainedLockType {
    /// 进程基本信息锁
    ProcessInfo,
    /// 进程状态锁
    ProcessState,
    /// 文件描述符锁
    FileDescriptors,
    /// 内存管理锁
    MemoryManagement,
    /// 信号状态锁
    SignalState,
    /// 调度信息锁
    SchedulingInfo,
}

/// 优化的进程表锁管理器
pub struct OptimizedProcessLockManager {
    config: LockConfig,
    stats: LockStats,
    
    /// 细粒度锁数组
    fine_grained_locks: [RwLock<()>; 7],
    
    /// 主进程表读写锁
    main_table_lock: RwLock<()>,
    
    /// 锁升级标志
    lock_upgrade_in_progress: AtomicBool,
}

impl OptimizedProcessLockManager {
    /// 创建新的优化锁管理器
    #[inline]
    pub fn new(config: LockConfig) -> Self {
        Self {
            config,
            stats: LockStats::new(),
            fine_grained_locks: [
                RwLock::new(()),
                RwLock::new(()),
                RwLock::new(()),
                RwLock::new(()),
                RwLock::new(()),
                RwLock::new(()),
                RwLock::new(()),
            ],
            main_table_lock: RwLock::new(()),
            lock_upgrade_in_progress: AtomicBool::new(false),
        }
    }

    /// 使用默认配置创建锁管理器
    #[inline]
    pub fn with_defaults() -> Self {
        Self::new(LockConfig::default())
    }

    /// 获取细粒度读锁
    #[inline]
    pub fn acquire_fine_grained_read_lock(&self, lock_type: FineGrainedLockType) -> FineGrainedReadLockGuard {
        let start_time = crate::time::hrtime_nanos();
        
        let lock = match lock_type {
            FineGrainedLockType::ProcessInfo => &self.fine_grained_locks[0],
            FineGrainedLockType::ProcessState => &self.fine_grained_locks[1],
            FineGrainedLockType::FileDescriptors => &self.fine_grained_locks[2],
            FineGrainedLockType::MemoryManagement => &self.fine_grained_locks[3],
            FineGrainedLockType::SignalState => &self.fine_grained_locks[4],
            FineGrainedLockType::SchedulingInfo => &self.fine_grained_locks[5],
        };
        
        // 尝试获取读锁
        let guard = lock.read();
        
        let end_time = crate::time::hrtime_nanos();
        let wait_time = end_time.saturating_sub(start_time);
        
        if wait_time > 0 {
            self.stats.record_contention(wait_time);
        }
        
        self.stats.record_read_lock_acquire();
        
        FineGrainedReadLockGuard {
            _guard: guard,
            lock_type,
            stats: &self.stats,
        }
    }

    /// 获取细粒度写锁
    #[inline]
    pub fn acquire_fine_grained_write_lock(&self, lock_type: FineGrainedLockType) -> FineGrainedWriteLockGuard {
        let start_time = crate::time::hrtime_nanos();
        
        let lock = match lock_type {
            FineGrainedLockType::ProcessInfo => &self.fine_grained_locks[0],
            FineGrainedLockType::ProcessState => &self.fine_grained_locks[1],
            FineGrainedLockType::FileDescriptors => &self.fine_grained_locks[2],
            FineGrainedLockType::MemoryManagement => &self.fine_grained_locks[3],
            FineGrainedLockType::SignalState => &self.fine_grained_locks[4],
            FineGrainedLockType::SchedulingInfo => &self.fine_grained_locks[5],
        };
        
        // 尝试获取写锁
        let guard = lock.write();
        
        let end_time = crate::time::hrtime_nanos();
        let wait_time = end_time.saturating_sub(start_time);
        
        if wait_time > 0 {
            self.stats.record_contention(wait_time);
        }
        
        self.stats.record_write_lock_acquire();
        
        FineGrainedWriteLockGuard {
            _guard: guard,
            lock_type,
            stats: &self.stats,
        }
    }

    /// 获取主表读锁
    #[inline]
    pub fn acquire_main_table_read_lock(&self) -> MainTableReadLockGuard {
        let start_time = crate::time::hrtime_nanos();
        
        let guard = self.main_table_lock.read();
        
        let end_time = crate::time::hrtime_nanos();
        let wait_time = end_time.saturating_sub(start_time);
        
        if wait_time > 0 {
            self.stats.record_contention(wait_time);
        }
        
        self.stats.record_read_lock_acquire();
        
        MainTableReadLockGuard {
            _guard: guard,
            stats: &self.stats,
        }
    }

    /// 获取主表写锁
    #[inline]
    pub fn acquire_main_table_write_lock(&self) -> MainTableWriteLockGuard {
        let start_time = crate::time::hrtime_nanos();
        
        let guard = self.main_table_lock.write();
        
        let end_time = crate::time::hrtime_nanos();
        let wait_time = end_time.saturating_sub(start_time);
        
        if wait_time > 0 {
            self.stats.record_contention(wait_time);
        }
        
        self.stats.record_write_lock_acquire();
        
        MainTableWriteLockGuard {
            _guard: guard,
            stats: &self.stats,
        }
    }

    /// 尝试锁升级（读锁升级为写锁）
    #[inline]
    pub fn try_lock_upgrade(&self, read_guard: &FineGrainedReadLockGuard) -> Option<FineGrainedWriteLockGuard> {
        if self.lock_upgrade_in_progress.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) != Ok(false) {
            return None; // 已有锁升级在进行
        }

        let start_time = crate::time::hrtime_nanos();
        
        // 释放读锁并尝试获取写锁
        let lock = match read_guard.lock_type {
            FineGrainedLockType::ProcessInfo => &self.fine_grained_locks[0],
            FineGrainedLockType::ProcessState => &self.fine_grained_locks[1],
            FineGrainedLockType::FileDescriptors => &self.fine_grained_locks[2],
            FineGrainedLockType::MemoryManagement => &self.fine_grained_locks[3],
            FineGrainedLockType::SignalState => &self.fine_grained_locks[4],
            FineGrainedLockType::SchedulingInfo => &self.fine_grained_locks[5],
        };
        
        // 释放读锁
        core::mem::drop(read_guard);
        
        // 获取写锁
        let write_guard = lock.write();
        
        let end_time = crate::time::hrtime_nanos();
        let wait_time = end_time.saturating_sub(start_time);
        
        if wait_time > self.config.lock_upgrade_timeout_ns {
            // 超时，释放写锁并返回失败
            core::mem::drop(write_guard);
            self.lock_upgrade_in_progress.store(false, Ordering::Release);
            return None;
        }
        
        if wait_time > 0 {
            self.stats.record_contention(wait_time);
        }
        
        self.stats.record_write_lock_acquire();
        self.stats.record_read_lock_release(); // 记录读锁释放
        
        Some(FineGrainedWriteLockGuard {
            _guard: write_guard,
            lock_type: read_guard.lock_type,
            stats: &self.stats,
        })
    }

    /// 获取锁定统计信息
    #[inline]
    pub fn get_stats(&self) -> &LockStats {
        &self.stats
    }

    /// 重置统计信息
    #[inline]
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }
}

/// 细粒度读锁守卫
pub struct FineGrainedReadLockGuard<'a> {
    _guard: crate::sync::RwLockReadGuard<'a, ()>,
    lock_type: FineGrainedLockType,
    stats: &'a LockStats,
}

impl<'a> Drop for FineGrainedReadLockGuard<'a> {
    fn drop(&mut self) {
        self.stats.record_read_lock_release();
    }
}

/// 细粒度写锁守卫
pub struct FineGrainedWriteLockGuard<'a> {
    _guard: crate::sync::RwLockWriteGuard<'a, ()>,
    lock_type: FineGrainedLockType,
    stats: &'a LockStats,
}

impl<'a> Drop for FineGrainedWriteLockGuard<'a> {
    fn drop(&mut self) {
        self.stats.record_write_lock_release();
    }
}

/// 主表读锁守卫
pub struct MainTableReadLockGuard<'a> {
    _guard: crate::sync::RwLockReadGuard<'a, ()>,
    stats: &'a LockStats,
}

impl<'a> Drop for MainTableReadLockGuard<'a> {
    fn drop(&mut self) {
        self.stats.record_read_lock_release();
    }
}

/// 主表写锁守卫
pub struct MainTableWriteLockGuard<'a> {
    _guard: crate::sync::RwLockWriteGuard<'a, ()>,
    stats: &'a LockStats,
}

impl<'a> Drop for MainTableWriteLockGuard<'a> {
    fn drop(&mut self) {
        self.stats.record_write_lock_release();
    }
}

/// 全局优化锁管理器实例
static mut GLOBAL_LOCK_MANAGER: Option<OptimizedProcessLockManager> = None;
static LOCK_MANAGER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// 获取全局锁管理器
pub fn get_global_lock_manager() -> &'static mut OptimizedProcessLockManager {
    unsafe {
        if !LOCK_MANAGER_INITIALIZED.load(Ordering::Relaxed) {
            GLOBAL_LOCK_MANAGER = Some(OptimizedProcessLockManager::with_defaults());
            LOCK_MANAGER_INITIALIZED.store(true, Ordering::Relaxed);
        }
        GLOBAL_LOCK_MANAGER.as_mut().unwrap()
    }
}

/// 便捷的锁获取接口
pub mod convenience {
    use super::*;

    /// 获取进程信息读锁
    #[inline]
    pub fn lock_process_info_read() -> FineGrainedReadLockGuard<'static> {
        unsafe {
            get_global_lock_manager().acquire_fine_grained_read_lock(FineGrainedLockType::ProcessInfo)
        }
    }

    /// 获取进程信息写锁
    #[inline]
    pub fn lock_process_info_write() -> FineGrainedWriteLockGuard<'static> {
        unsafe {
            get_global_lock_manager().acquire_fine_grained_write_lock(FineGrainedLockType::ProcessInfo)
        }
    }

    /// 获取文件描述符读锁
    #[inline]
    pub fn lock_file_descriptors_read() -> FineGrainedReadLockGuard<'static> {
        unsafe {
            get_global_lock_manager().acquire_fine_grained_read_lock(FineGrainedLockType::FileDescriptors)
        }
    }

    /// 获取文件描述符写锁
    #[inline]
    pub fn lock_file_descriptors_write() -> FineGrainedWriteLockGuard<'static> {
        unsafe {
            get_global_lock_manager().acquire_fine_grained_write_lock(FineGrainedLockType::FileDescriptors)
        }
    }

    /// 获取主表读锁
    #[inline]
    pub fn lock_main_table_read() -> MainTableReadLockGuard<'static> {
        unsafe {
            get_global_lock_manager().acquire_main_table_read_lock()
        }
    }

    /// 获取主表写锁
    #[inline]
    pub fn lock_main_table_write() -> MainTableWriteLockGuard<'static> {
        unsafe {
            get_global_lock_manager().acquire_main_table_write_lock()
        }
    }

    /// 尝试锁升级
    #[inline]
    pub fn try_upgrade_lock(read_guard: &FineGrainedReadLockGuard<'static>) -> Option<FineGrainedWriteLockGuard<'static>> {
        unsafe {
            get_global_lock_manager().try_lock_upgrade(read_guard)
        }
    }
}

/// 获取锁统计信息
pub fn get_lock_stats() -> LockStatsSnapshot {
    unsafe {
        get_global_lock_manager().get_stats().get_snapshot()
    }
}

/// 重置锁统计信息
pub fn reset_lock_stats() {
    unsafe {
        get_global_lock_manager().reset_stats();
    }
}

/// 自适应锁策略
pub mod adaptive {
    use super::*;

    /// 根据系统负载选择锁策略
    #[inline]
    pub fn should_use_fine_grained() -> bool {
        // 简化实现：基于当前进程数量决定
        let proc_count = crate::process::manager::PROC_TABLE.lock().len();
        
        // 如果进程数量较多，使用细粒度锁
        proc_count > 8
    }

    /// 根据访问模式选择锁类型
    #[inline]
    pub fn should_use_read_lock_for_operation(operation_type: OperationType) -> bool {
        match operation_type {
            OperationType::Read => true,
            OperationType::Write => false,
            OperationType::ReadWrite => false,
            OperationType::Query => true,
            OperationType::Modify => false,
        }
    }

    /// 操作类型
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OperationType {
        Read,
        Write,
        ReadWrite,
        Query,
        Modify,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_stats_creation() {
        let stats = LockStats::new();
        let snapshot = stats.get_snapshot();
        
        assert_eq!(snapshot.read_lock_acquires, 0);
        assert_eq!(snapshot.write_lock_acquires, 0);
        assert_eq!(snapshot.contentions, 0);
    }

    #[test]
    fn test_lock_config() {
        let config = LockConfig::default();
        assert!(config.enable_rw_locks);
        assert!(config.enable_fine_grained);
        assert!(config.enable_adaptive);
        assert_eq!(config.max_readers, 16);
    }

    #[test]
    fn test_fine_grained_lock_types() {
        assert_eq!(FineGrainedLockType::ProcessInfo as usize, 0);
        assert_eq!(FineGrainedLockType::FileDescriptors as usize, 2);
        assert_eq!(FineGrainedLockType::MemoryManagement as usize, 3);
    }

    #[test]
    fn test_lock_manager_creation() {
        let manager = OptimizedProcessLockManager::with_defaults();
        let stats = manager.get_stats();
        let snapshot = stats.get_snapshot();
        
        assert_eq!(snapshot.read_lock_acquires, 0);
        assert_eq!(snapshot.write_lock_acquires, 0);
    }

    #[test]
    fn test_lock_contention_recording() {
        let stats = LockStats::new();
        
        stats.record_contention(1000);
        stats.record_contention(2000);
        
        let snapshot = stats.get_snapshot();
        assert_eq!(snapshot.contentions, 2);
        assert_eq!(snapshot.avg_wait_time_ns, 1500); // (1000 + 2000) / 2
        assert_eq!(snapshot.max_wait_time_ns, 2000);
    }

    #[test]
    fn test_adaptive_strategy() {
        assert!(!adaptive::should_use_fine_grained()); // 默认进程数较少
        
        // 测试操作类型判断
        assert!(adaptive::should_use_read_lock_for_operation(adaptive::OperationType::Read));
        assert!(!adaptive::should_use_read_lock_for_operation(adaptive::OperationType::Write));
    }
}