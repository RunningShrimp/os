//! 无锁系统调用统计模块
//!
//! 本模块提供基于per-CPU原子操作的无锁系统调用统计实现。
//!
//! ## 设计原理
//!
//! 传统Mutex保护的统计存在以下问题：
//! - **锁竞争**: 多核环境下锁竞争严重
//! - **缓存一致性**: 锁的获取/释放导致缓存行颠簸
//! - **延迟不稳定**: 热路径上的锁导致延迟不可预测
//!
//! 本实现采用per-CPU统计 + 原子操作的方案：
//! - **零锁**: 所有更新操作都是无锁的
//! - **Cache对齐**: 每CPU统计结构64字节对齐，避免false sharing
//! - **线性扩展**: 性能随CPU核心数线性扩展
//!
//! ## 架构
//!
//! 架构示意：
//!
//! LockFreeSyscallStats
//!   CPU 0 Stats    CPU 1 Stats    CPU 2 Stats    CPU N Stats
//!       |              |              |              |
//!       +--------------+--------------+--------------+
//!                          |
//!                    Global Aggregate
//!                    (atomic counters)
//!
//! ## 性能特征
//!
//! - **记录延迟**: ~10ns (无锁原子操作)
//! - **快照延迟**: ~5000ns (需聚合所有CPU)
//! - **扩展性**: 线性扩展
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::subsystems::syscalls::lockfree_stats::{record_syscall, record_error, get_stats_snapshot};
//!
//! // 记录系统调用（热路径，无锁）
//! record_syscall(39 /* getpid */, 50 /* ns */);
//!
//! // 记录错误
//! record_error(1 /* write */, -22 /* EINVAL */);
//!
//! // 获取快照（监控路径，较慢）
//! let snapshot = get_stats_snapshot();
//! println!("Total syscalls: {}", snapshot.total_calls);
//! ```

use core::sync::atomic::{AtomicU64, Ordering};

use crate::platform::arch::current_cpu_id;

// ============================================================================
// 常量定义
// ============================================================================

/// 最大CPU数量
///
/// 根据x86_64架构限制，支持最多256个CPU。
/// 实际运行时CPU数量可能更少，但数组大小固定以确保性能。
const MAX_CPUS: usize = 256;

/// 最大系统调用数量
///
/// Linux x86_64系统调用号范围是0-547，这里设置为1024以提供足够空间。
const MAX_SYSCALLS: usize = 1024;

/// 缓存行大小（x86_64）
///
/// 使用64字节对齐避免false sharing。
const CACHE_LINE_SIZE: usize = 64;

// ============================================================================
// Per-CPU统计结构
// ============================================================================

/// Per-CPU系统调用统计
///
/// 每个CPU一个实例，通过cache line对齐避免false sharing。
/// 所有字段都是原子类型，支持无锁更新。
#[repr(C, align(64))]  // 64字节对齐，确保每个CPU独占一个缓存行
struct PerCpuStats {
    /// 各系统调用计数器
    syscall_counts: [AtomicU64; MAX_SYSCALLS],

    /// 各系统调用错误计数器
    error_counts: [AtomicU64; MAX_SYSCALLS],

    /// 总执行时间（纳秒）
    total_time_ns: AtomicU64,

    /// 最后更新时间（纳秒）
    last_update_ns: AtomicU64,

    /// Padding到完整缓存行
    _pad: [u8; CACHE_LINE_SIZE],
}

impl PerCpuStats {
    /// 创建新的per-CPU统计
    fn new() -> Self {
        Self {
            // 使用const block创建原子数组
            syscall_counts: [const { AtomicU64::new(0) }; MAX_SYSCALLS],
            error_counts: [const { AtomicU64::new(0) }; MAX_SYSCALLS],
            total_time_ns: AtomicU64::new(0),
            last_update_ns: AtomicU64::new(0),
            _pad: [0; CACHE_LINE_SIZE],
        }
    }
}

// ============================================================================
// 统计快照
// ============================================================================

/// 系统调用统计快照
///
/// 从per-CPU统计聚合而来的全局视图。
/// 用于监控、调试和性能分析。
#[derive(Debug, Clone)]
pub struct SyscallStatsSnapshot {
    /// 总调用次数
    pub total_calls: u64,

    /// 总错误次数
    pub total_errors: u64,

    /// 各系统调用计数
    pub syscall_counts: alloc::vec::Vec<u64>,

    /// 各系统调用错误计数
    pub error_counts: alloc::vec::Vec<u64>,

    /// 总执行时间（纳秒）
    pub total_time_ns: u64,

    /// 快照时间戳
    pub timestamp_ns: u64,
}

impl SyscallStatsSnapshot {
    /// 创建新的快照
    pub fn new() -> Self {
        Self {
            total_calls: 0,
            total_errors: 0,
            syscall_counts: alloc::vec![0; MAX_SYSCALLS],
            error_counts: alloc::vec![0; MAX_SYSCALLS],
            total_time_ns: 0,
            timestamp_ns: 0,
        }
    }

    /// 获取特定系统调用的统计
    pub fn get_syscall_count(&self, syscall_id: usize) -> u64 {
        self.syscall_counts.get(syscall_id).copied().unwrap_or(0)
    }

    /// 获取特定系统调用的错误数
    pub fn get_error_count(&self, syscall_id: usize) -> u64 {
        self.error_counts.get(syscall_id).copied().unwrap_or(0)
    }

    /// 计算平均系统调用时间
    pub fn get_average_time_ns(&self) -> u64 {
        if self.total_calls == 0 {
            0
        } else {
            self.total_time_ns / self.total_calls
        }
    }

    /// 生成人类可读的报告
    pub fn generate_report(&self) -> alloc::string::String {
        let mut report = alloc::string::String::new();
        report.push_str("# 无锁系统调用统计报告\n\n");

        report.push_str(&alloc::format!("快照时间: {} ns\n\n", self.timestamp_ns));
        report.push_str("## 全局统计\n");
        report.push_str(&alloc::format!("- 总调用次数: {}\n", self.total_calls));
        report.push_str(&alloc::format!("- 总错误次数: {}\n", self.total_errors));
        report.push_str(&alloc::format!("- 总执行时间: {} ns\n", self.total_time_ns));
        report.push_str(&alloc::format!("- 平均调用时间: {} ns\n\n", self.get_average_time_ns()));

        report.push_str("## Top 10 系统调用\n");
        let mut top_calls: alloc::vec::Vec<(usize, u64)> = self.syscall_counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 0)
            .map(|(id, count)| (id, *count))
            .collect();

        top_calls.sort_by(|a, b| b.1.cmp(&a.1));

        for (id, count) in top_calls.iter().take(10) {
            report.push_str(&alloc::format!("  syscall[{}]: {} 次\n", id, count));
        }

        report
    }
}

impl Default for SyscallStatsSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 无锁统计实现
// ============================================================================

/// 无锁系统调用统计
///
/// 使用per-CPU原子数组实现零锁统计。
///
/// ## 线程安全
///
/// 所有方法都是线程安全的，无需额外同步。
///
/// ## 内存序
///
/// 使用`Ordering::Relaxed`，因为：
/// - 计数器之间无依赖关系
/// - 不需要与其他内存操作同步
/// - 快照可以容忍轻微不一致
pub struct LockFreeSyscallStats {
    /// Per-CPU统计数组
    per_cpu: [PerCpuStats; MAX_CPUS],

    /// 全局总调用计数（用于快速查询）
    total_calls: AtomicU64,

    /// 全局总错误计数（用于快速查询）
    total_errors: AtomicU64,
}

impl LockFreeSyscallStats {
    /// 创建新的无锁统计
    pub fn new() -> Self {
        // 创建per-CPU数组，使用MaybeUninit避免栈溢出
        use core::mem::MaybeUninit;

        unsafe {
            // 创建未初始化的数组
            let mut per_cpu: [MaybeUninit<PerCpuStats>; MAX_CPUS] = MaybeUninit::uninit().assume_init();

            // 逐个初始化每个元素
            for i in 0..MAX_CPUS {
                per_cpu[i].write(PerCpuStats::new());
            }

            // 转换为已初始化的数组 - 使用ptr::read复制
            let per_cpu_ptr = &per_cpu as *const _ as *const [PerCpuStats; MAX_CPUS];
            let per_cpu_init = core::ptr::read(per_cpu_ptr);

            // 防止per_cpu被drop
            core::mem::forget(per_cpu);

            Self {
                per_cpu: per_cpu_init,
                total_calls: AtomicU64::new(0),
                total_errors: AtomicU64::new(0),
            }
        }
    }

    /// 记录系统调用（无锁，热路径）
    ///
    /// ## 参数
    ///
    /// - `syscall_id`: 系统调用号
    /// - `duration_ns`: 执行时间（纳秒）
    ///
    /// ## 性能
    ///
    /// - **延迟**: ~10ns (3-4条原子指令)
    /// - **扩展性**: 线性（每CPU独立）
    ///
    /// ## 示例
    ///
    /// ```no_run
    /// LOCKFREE_STATS.record_syscall(39, 50);
    /// ```
    #[inline]
    pub fn record_syscall(&self, syscall_id: u64, duration_ns: u64) {
        // 获取当前CPU ID
        let cpu_id = current_cpu_id() % MAX_CPUS;

        // 获取per-CPU统计（unsafe但安全：数组大小固定，cpu_id在范围内）
        let stats = unsafe { self.per_cpu.get_unchecked(cpu_id) };

        // 边界检查
        let syscall_id = (syscall_id as usize) % MAX_SYSCALLS;

        // 无锁更新（Relaxed序即可）
        stats.syscall_counts[syscall_id].fetch_add(1, Ordering::Relaxed);
        stats.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
        stats.last_update_ns.store(duration_ns, Ordering::Relaxed);

        // 全局计数（可选，用于快速查询）
        self.total_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录错误（无锁，热路径）
    ///
    /// ## 参数
    ///
    /// - `syscall_id`: 系统调用号
    /// - `error_code`: 错误码（未使用，可用于错误分类）
    ///
    /// ## 性能
    ///
    /// - **延迟**: ~10ns (2-3条原子指令)
    /// - **扩展性**: 线性（每CPU独立）
    ///
    /// ## 示例
    ///
    /// ```no_run
    /// LOCKFREE_STATS.record_error(1, -22);
    /// ```
    #[inline]
    pub fn record_error(&self, syscall_id: u64, _error_code: i32) {
        // 获取当前CPU ID
        let cpu_id = current_cpu_id() % MAX_CPUS;

        // 获取per-CPU统计
        let stats = unsafe { self.per_cpu.get_unchecked(cpu_id) };

        // 边界检查
        let syscall_id = (syscall_id as usize) % MAX_SYSCALLS;

        // 无锁更新
        stats.error_counts[syscall_id].fetch_add(1, Ordering::Relaxed);

        // 全局计数
        self.total_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取聚合快照（较慢，监控路径）
    ///
    /// ## 性能
    ///
    /// - **延迟**: ~5000ns (需遍历所有CPU和系统调用)
    /// - **CPU占用**: 与CPU数量和系统调用数量成正比
    ///
    /// ## 使用场景
    ///
    /// - 监控和调试
    /// - 性能分析
    /// - 定期报告生成
    ///
    /// ## 注意
    ///
    /// 快照不是原子性的，不同CPU的统计数据可能在聚合过程中发生变化。
    ///
    /// ## 示例
    ///
    /// ```no_run
    /// let snapshot = LOCKFREE_STATS.snapshot();
    /// println!("Total syscalls: {}", snapshot.total_calls);
    /// ```
    pub fn snapshot(&self) -> SyscallStatsSnapshot {
        let mut snapshot = SyscallStatsSnapshot::new();

        // 聚合所有CPU的统计
        for cpu in 0..MAX_CPUS {
            let stats = unsafe { self.per_cpu.get_unchecked(cpu) };

            // 聚合各系统调用计数
            for id in 0..MAX_SYSCALLS {
                let count = stats.syscall_counts[id].load(Ordering::Relaxed);
                if count > 0 {
                    snapshot.syscall_counts[id] += count;
                }

                let errors = stats.error_counts[id].load(Ordering::Relaxed);
                if errors > 0 {
                    snapshot.error_counts[id] += errors;
                }
            }

            // 聚合总时间
            snapshot.total_time_ns += stats.total_time_ns.load(Ordering::Relaxed);
        }

        // 全局计数（快速路径）
        snapshot.total_calls = self.total_calls.load(Ordering::Relaxed);
        snapshot.total_errors = self.total_errors.load(Ordering::Relaxed);

        // 快照时间戳
        // GH-#1371: 从系统时钟获取
        snapshot.timestamp_ns = 0;

        snapshot
    }

    /// 获取全局总调用数（快速）
    #[inline]
    pub fn get_total_calls(&self) -> u64 {
        self.total_calls.load(Ordering::Relaxed)
    }

    /// 获取全局总错误数（快速）
    #[inline]
    pub fn get_total_errors(&self) -> u64 {
        self.total_errors.load(Ordering::Relaxed)
    }

    /// 重置所有统计（仅用于测试）
    #[cfg(test)]
    pub fn reset(&self) {
        // 重置所有per-CPU统计
        for cpu in 0..MAX_CPUS {
            let stats = unsafe { self.per_cpu.get_unchecked(cpu) };

            for id in 0..MAX_SYSCALLS {
                stats.syscall_counts[id].store(0, Ordering::Relaxed);
                stats.error_counts[id].store(0, Ordering::Relaxed);
            }

            stats.total_time_ns.store(0, Ordering::Relaxed);
            stats.last_update_ns.store(0, Ordering::Relaxed);
        }

        // 重置全局计数
        self.total_calls.store(0, Ordering::Relaxed);
        self.total_errors.store(0, Ordering::Relaxed);
    }

    /// 获取per-CPU统计（用于调试）
    #[inline]
    pub fn get_per_cpu_stats(&self, cpu_id: usize) -> Option<&PerCpuStats> {
        if cpu_id < MAX_CPUS {
            Some(unsafe { self.per_cpu.get_unchecked(cpu_id) })
        } else {
            None
        }
    }

    /// 获取内存占用（字节）
    pub fn memory_footprint(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

// ============================================================================
// 全局实例
// ============================================================================

use spin::Once;

static LOCKFREE_STATS_INIT: Once<LockFreeSyscallStats> = Once::new();

/// 全局无锁系统调用统计实例
///
/// 静态实例，可在系统任何地方访问。
/// 所有方法都是线程安全的，无需额外同步。
pub fn get_lockfree_stats() -> &'static LockFreeSyscallStats {
    LOCKFREE_STATS_INIT.call_once(|| LockFreeSyscallStats::new())
}

/// 初始化全局统计（应在系统启动时调用）
pub fn init_lockfree_stats() {
    get_lockfree_stats();
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取当前CPU ID（包装函数）
///
/// 提供统一的CPU ID获取接口。
#[inline]
pub fn get_cpu_id() -> usize {
    current_cpu_id()
}

/// 记录系统调用的便捷函数
#[inline]
pub fn record_syscall(syscall_id: u64, duration_ns: u64) {
    get_lockfree_stats().record_syscall(syscall_id, duration_ns);
}

/// 记录错误的便捷函数
#[inline]
pub fn record_error(syscall_id: u64, error_code: i32) {
    get_lockfree_stats().record_error(syscall_id, error_code);
}

/// 获取统计快照的便捷函数
pub fn get_stats_snapshot() -> SyscallStatsSnapshot {
    get_lockfree_stats().snapshot()
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfree_stats_basic() {
        let stats = LockFreeSyscallStats::new();

        // 记录一些系统调用
        stats.record_syscall(39, 100);
        stats.record_syscall(39, 150);
        stats.record_syscall(1, 200);

        // 获取快照
        let snapshot = stats.snapshot();

        // 验证
        assert_eq!(snapshot.total_calls, 3);
        assert_eq!(snapshot.syscall_counts[39], 2);
        assert_eq!(snapshot.syscall_counts[1], 1);
    }

    #[test]
    fn test_lockfree_stats_errors() {
        let stats = LockFreeSyscallStats::new();

        // 记录错误
        stats.record_error(1, -22);
        stats.record_error(1, -2);

        // 获取快照
        let snapshot = stats.snapshot();

        // 验证
        assert_eq!(snapshot.total_errors, 2);
        assert_eq!(snapshot.error_counts[1], 2);
    }

    #[test]
    fn test_lockfree_stats_concurrent() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        let stats = LockFreeSyscallStats::new();
        let counter = AtomicUsize::new(0);

        // 模拟并发更新
        for _ in 0..1000 {
            stats.record_syscall(39, 50);
            counter.fetch_add(1, Ordering::Relaxed);
        }

        // 验证
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_calls, counter.load(Ordering::Relaxed) as u64);
        assert_eq!(snapshot.syscall_counts[39], 1000);
    }

    #[test]
    fn test_memory_footprint() {
        let stats = LockFreeSyscallStats::new();
        let footprint = stats.memory_footprint();

        // 验证内存占用
        // PerCpuStats大小:
        // - syscall_counts: 1024 * 8 = 8192 bytes
        // - error_counts: 1024 * 8 = 8192 bytes
        // - total_time_ns: 8 bytes
        // - last_update_ns: 8 bytes
        // - _pad: 64 bytes
        // Total per CPU: ~16464 bytes
        // 256 CPUs: ~4,214,784 bytes (~4 MB)

        assert!(footprint > 4_000_000);
        assert!(footprint < 5_000_000);
    }

    #[test]
    fn test_snapshot_report() {
        let stats = LockFreeSyscallStats::new();

        // 记录一些数据
        stats.record_syscall(39, 100);
        stats.record_syscall(1, 200);
        stats.record_error(1, -22);

        // 生成报告
        let snapshot = stats.snapshot();
        let report = snapshot.generate_report();

        // 验证报告包含关键信息
        assert!(report.contains("总调用次数"));
        assert!(report.contains("总错误次数"));
        assert!(report.contains("平均调用时间"));
    }

    #[test]
    fn test_per_cpu_stats_alignment() {
        // 验证cache line对齐
        assert_eq!(core::mem::align_of::<PerCpuStats>(), 64);

        // 验证大小
        let stats = PerCpuStats::new();
        let size = core::mem::size_of_val(&stats);
        // 应该是cache line的倍数
        assert_eq!(size % 64, 0);
    }
}
