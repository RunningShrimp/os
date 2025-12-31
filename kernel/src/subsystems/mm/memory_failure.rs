//! # 内存故障隔离和恢复
//!
//! 本模块实现内存错误处理功能，包括硬件错误响应、poison 页面标记、
//! 页面杀掉和迁移，以及软错误恢复策略。
//!
//! ## 主要功能
//!
//! - **硬件错误 (MCE) 响应**: 响应机器检查异常
//! - **Poison 页面标记**: 标记损坏的页面
//! - **页面杀掉和迁移**: 隔离和迁移损坏页面
//! - **软错误恢复策略**: 处理可纠正的错误
//!
//! ## 架构
//!
//! ```text
//! 硬件错误 (MCE)
//!     ↓
//! 错误检测 → 可纠正？
//!     ↓ No          ↓ Yes
//! 硬件故障处理      软错误处理
//!     ↓               ↓
//! Poison 页面      计数错误
//!     ↓               ↓
//! 页面迁移         阈值检查
//!     ↓               ↓
//! 数据恢复        触发硬件故障处理
//! ```
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::subsystems::mm::memory_failure::{MemoryFailure, MemoryError};
//!
//! // 处理内存错误
//! let handler = MemoryFailure::new();
//! handler.handle_error(MemoryError {
//!     pfn: 0x1000,
//!     error_type: ErrorType::HardwareError,
//!     ..
//! })?;
//! # Ok::<(), UnifiedError>(())
//! ```

#![allow(dead_code)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use crate::error::{UnifiedError, UnifiedResult};

/// 页面帧号
pub type Pfn = usize;

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// 可纠正错误（CE）
    Correctable,
    /// 可恢复错误
    Recoverable,
    /// 致命错误（不可恢复）
    Fatal,
    /// 未知严重程度
    Unknown,
}

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// 硬件错误（MCE）
    HardwareError,
    /// 软错误（可纠正）
    SoftError,
    /// ECC 错误
    EccError,
    /// 地址错误
    AddressError,
    /// 数据错误
    DataError,
    /// 超时错误
    TimeoutError,
}

/// 内存错误信息
#[derive(Debug, Clone)]
pub struct MemoryError {
    /// 页面帧号
    pub pfn: Pfn,
    /// 错误类型
    pub error_type: ErrorType,
    /// 错误严重程度
    pub severity: ErrorSeverity,
    /// 错误地址（可选）
    pub error_addr: Option<usize>,
    /// 错误时间戳
    pub timestamp: u64,
    /// CPU ID
    pub cpu_id: usize,
    /// 错误计数（对于软错误）
    pub error_count: u32,
    /// 额外信息
    pub extra_info: u64,
}

impl MemoryError {
    /// 创建新的内存错误
    pub fn new(pfn: Pfn, error_type: ErrorType, severity: ErrorSeverity) -> Self {
        Self {
            pfn,
            error_type,
            severity,
            error_addr: None,
            timestamp: 0,
            cpu_id: 0,
            error_count: 1,
            extra_info: 0,
        }
    }

    /// 设置错误地址
    pub fn with_error_addr(mut self, addr: usize) -> Self {
        self.error_addr = Some(addr);
        self
    }

    /// 设置错误计数
    pub fn with_error_count(mut self, count: u32) -> Self {
        self.error_count = count;
        self
    }
}

/// Poison 页面状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoisonPageState {
    /// 未 poison
    NotPoisoned,
    /// 已 poison（不可访问）
    Poisoned,
    /// 正在迁移
    Migrating,
    /// 已迁移（新页面可用）
    Migrated,
    /// 恢复中
    Recovering,
}

/// Poison 页面条目
#[derive(Debug)]
struct PoisonPageEntry {
    /// 页面帧号
    pfn: Pfn,
    /// 状态
    state: PoisonPageState,
    /// 错误类型
    error_type: ErrorType,
    /// 错误计数
    error_count: AtomicU32,
    /// 迁移后的 PFN
    migrated_pfn: Option<Pfn>,
    /// Poison 时间
    poison_time: u64,
    /// 迁移时间
    migrate_time: Option<u64>,
    /// 引用计数
    ref_count: AtomicUsize,
}

impl PoisonPageEntry {
    fn new(pfn: Pfn, error_type: ErrorType) -> Self {
        Self {
            pfn,
            state: PoisonPageState::Poisoned,
            error_type,
            error_count: AtomicU32::new(1),
            migrated_pfn: None,
            poison_time: 0,
            migrate_time: None,
            ref_count: AtomicUsize::new(0),
        }
    }

    fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Release)
    }
}

/// 恢复策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// 不恢复
    None,
    /// 迁移页面
    Migrate,
    /// 重试访问
    Retry,
    /// 杀掉进程
    KillProcess,
    /// 恢复到备份
    RestoreFromBackup,
}

/// 内存故障统计
#[derive(Debug, Default)]
pub struct MemoryFailureStats {
    /// 总错误数
    pub total_errors: AtomicU64,
    /// 硬件错误数
    pub hardware_errors: AtomicU64,
    /// 软错误数
    pub soft_errors: AtomicU64,
    /// 可纠正错误数
    pub correctable_errors: AtomicU64,
    /// 致命错误数
    pub fatal_errors: AtomicU64,
    /// Poison 页面数
    pub poisoned_pages: AtomicU64,
    /// 迁移的页面数
    pub migrated_pages: AtomicU64,
    /// 恢复的页面数
    pub recovered_pages: AtomicU64,
    /// 杀掉的进程数
    pub killed_processes: AtomicU64,
    /// 总恢复时间（纳秒）
    pub total_recovery_time_ns: AtomicU64,
}

/// 内存故障处理器
pub struct MemoryFailure {
    /// Poison 页面映射
    poisoned_pages: Mutex<BTreeMap<Pfn, PoisonPageEntry>>,
    /// 软错误计数（按 PFN）
    soft_error_counts: Mutex<BTreeMap<Pfn, AtomicU32>>,
    /// 恢复策略
    recovery_strategy: Mutex<RecoveryStrategy>,
    /// 统计信息
    stats: MemoryFailureStats,
    /// 是否启用处理
    enabled: AtomicBool,
    /// 软错误阈值
    soft_error_threshold: AtomicU32,
    /// 是否启用自动迁移
    auto_migrate: AtomicBool,
    /// 是否启用自动恢复
    auto_recover: AtomicBool,
}

impl MemoryFailure {
    /// 创建新的内存故障处理器
    pub fn new() -> Self {
        Self {
            poisoned_pages: Mutex::new(BTreeMap::new()),
            soft_error_counts: Mutex::new(BTreeMap::new()),
            recovery_strategy: Mutex::new(RecoveryStrategy::Migrate),
            stats: MemoryFailureStats::default(),
            enabled: AtomicBool::new(true),
            soft_error_threshold: AtomicU32::new(10),
            auto_migrate: AtomicBool::new(true),
            auto_recover: AtomicBool::new(false),
        }
    }

    /// 处理内存错误
    pub fn handle_error(&self, error: MemoryError) -> UnifiedResult<()> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let start_time = self.get_time_ns();

        // 更新统计
        self.stats.total_errors.fetch_add(1, Ordering::Relaxed);

        match error.severity {
            ErrorSeverity::Correctable => {
                // 可纠正错误
                self.handle_correctable_error(error)?;
            }
            ErrorSeverity::Recoverable => {
                // 可恢复错误
                self.handle_recoverable_error(error)?;
            }
            ErrorSeverity::Fatal => {
                // 致命错误
                self.handle_fatal_error(error)?;
            }
            ErrorSeverity::Unknown => {
                // 未知严重程度，按致命处理
                self.handle_fatal_error(error)?;
            }
        }

        // 更新统计时间
        let elapsed = self.get_time_ns() - start_time;
        self.stats.total_recovery_time_ns.fetch_add(elapsed, Ordering::Relaxed);

        Ok(())
    }

    /// 标记页面为 poison
    pub fn poison_page(&self, pfn: Pfn, error_type: ErrorType) -> UnifiedResult<()> {
        let mut poisoned = self.poisoned_pages.lock();

        if poisoned.contains_key(&pfn) {
            // 已经 poison，增加错误计数
            if let Some(entry) = poisoned.get_mut(&pfn) {
                entry.error_count.fetch_add(1, Ordering::Relaxed);
            }
            return Ok(());
        }

        // 创建 poison 条目
        let entry = PoisonPageEntry::new(pfn, error_type);
        poisoned.insert(pfn, entry);

        // 更新统计
        self.stats.poisoned_pages.fetch_add(1, Ordering::Relaxed);

        log::warn!("Page PFN {} poisoned due to {:?}", pfn, error_type);

        Ok(())
    }

    /// 检查页面是否被 poison
    pub fn is_page_poisoned(&self, pfn: Pfn) -> bool {
        let poisoned = self.poisoned_pages.lock();
        poisoned.contains_key(&pfn)
    }

    /// 获取 poison 页面状态
    pub fn get_poison_page_state(&self, pfn: Pfn) -> Option<PoisonPageState> {
        let poisoned = self.poisoned_pages.lock();
        poisoned.get(&pfn).map(|entry| entry.state)
    }

    /// 迁移 poison 页面
    pub fn migrate_poison_page(&self, pfn: Pfn) -> UnifiedResult<Pfn> {
        use super::migration::{migrate_page, MigrationFlags};

        let mut poisoned = self.poisoned_pages.lock();

        if let Some(entry) = poisoned.get_mut(&pfn) {
            if entry.state != PoisonPageState::Poisoned {
                return Err(UnifiedError::InvalidArgument.into());
            }

            // 设置状态为迁移中
            entry.state = PoisonPageState::Migrating;

            // 选择目标节点（不同于当前节点）
            let target_node = if self.get_node_for_pfn(pfn)? == 0 { 1 } else { 0 };

            drop(poisoned);

            // 执行迁移
            let new_pfn = migrate_page(pfn, target_node, MigrationFlags::SYNC)?;

            // 更新条目
            let mut poisoned = self.poisoned_pages.lock();
            if let Some(entry) = poisoned.get_mut(&pfn) {
                entry.state = PoisonPageState::Migrated;
                entry.migrated_pfn = Some(new_pfn);
                entry.migrate_time = Some(self.get_time_ns());
            }

            // 更新统计
            self.stats.migrated_pages.fetch_add(1, Ordering::Relaxed);

            log::info!("Migrated poisoned page {} to {}", pfn, new_pfn);

            Ok(new_pfn)
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    /// 恢复 poison 页面
    pub fn recover_page(&self, pfn: Pfn) -> UnifiedResult<()> {
        let mut poisoned = self.poisoned_pages.lock();

        if let Some(entry) = poisoned.get_mut(&pfn) {
            entry.state = PoisonPageState::Recovering;

            // 根据策略恢复
            let strategy = *self.recovery_strategy.lock();

            drop(poisoned);

            match strategy {
                RecoveryStrategy::Migrate => {
                    // 迁移到新页面
                    let _ = self.migrate_poison_page(pfn)?;
                }
                RecoveryStrategy::KillProcess => {
                    // 杀掉使用此页面的进程
                    self.stats.killed_processes.fetch_add(1, Ordering::Relaxed);
                }
                RecoveryStrategy::RestoreFromBackup => {
                    // 从备份恢复（需要实现备份机制）
                    log::warn!("Restore from backup not implemented for PFN {}", pfn);
                }
                _ => {
                    // 其他策略
                }
            }

            // 更新状态
            let mut poisoned = self.poisoned_pages.lock();
            if let Some(entry) = poisoned.get_mut(&pfn) {
                entry.state = PoisonPageState::Migrated;
            }

            // 更新统计
            self.stats.recovered_pages.fetch_add(1, Ordering::Relaxed);

            Ok(())
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    /// 清除 poison 标记
    pub fn unpoison_page(&self, pfn: Pfn) -> UnifiedResult<()> {
        let mut poisoned = self.poisoned_pages.lock();

        if poisoned.remove(&pfn).is_some() {
            self.stats.poisoned_pages.fetch_sub(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    /// 设置恢复策略
    pub fn set_recovery_strategy(&self, strategy: RecoveryStrategy) {
        let mut current_strategy = self.recovery_strategy.lock();
        *current_strategy = strategy;
    }

    /// 获取恢复策略
    pub fn get_recovery_strategy(&self) -> RecoveryStrategy {
        *self.recovery_strategy.lock()
    }

    /// 设置软错误阈值
    pub fn set_soft_error_threshold(&self, threshold: u32) {
        self.soft_error_threshold.store(threshold, Ordering::Relaxed);
    }

    /// 获取软错误阈值
    pub fn get_soft_error_threshold(&self) -> u32 {
        self.soft_error_threshold.load(Ordering::Relaxed)
    }

    /// 启用/禁用自动迁移
    pub fn set_auto_migrate(&self, enabled: bool) {
        self.auto_migrate.store(enabled, Ordering::Relaxed);
    }

    /// 启用/禁用自动恢复
    pub fn set_auto_recover(&self, enabled: bool) {
        self.auto_recover.store(enabled, Ordering::Relaxed);
    }

    /// 启用/禁用故障处理
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &MemoryFailureStats {
        &self.stats
    }

    // 内部方法

    /// 处理可纠正错误
    fn handle_correctable_error(&self, error: MemoryError) -> UnifiedResult<()> {
        self.stats.correctable_errors.fetch_add(1, Ordering::Relaxed);

        // 增加软错误计数
        let mut soft_counts = self.soft_error_counts.lock();
        let counter = soft_counts.entry(error.pfn).or_insert_with(|| AtomicU32::new(0));
        let count = counter.fetch_add(1, Ordering::Relaxed) + 1;

        drop(soft_counts);

        // 检查是否超过阈值
        let threshold = self.soft_error_threshold.load(Ordering::Relaxed);
        if count >= threshold {
            log::warn!("Soft error threshold exceeded for PFN {}, treating as hardware error", error.pfn);

            // 标记为硬件错误
            let hw_error = MemoryError {
                severity: ErrorSeverity::Fatal,
                error_type: ErrorType::HardwareError,
                ..error
            };

            return self.handle_fatal_error(hw_error);
        }

        log::debug!("Correctable error at PFN {}, count: {}", error.pfn, count);

        Ok(())
    }

    /// 处理可恢复错误
    fn handle_recoverable_error(&self, error: MemoryError) -> UnifiedResult<()> {
        self.stats.hardware_errors.fetch_add(1, Ordering::Relaxed);

        // Poison 页面
        self.poison_page(error.pfn, error.error_type)?;

        // 如果启用自动恢复，尝试恢复
        if self.auto_recover.load(Ordering::Relaxed) {
            self.recover_page(error.pfn)?;
        }

        Ok(())
    }

    /// 处理致命错误
    fn handle_fatal_error(&self, error: MemoryError) -> UnifiedResult<()> {
        self.stats.fatal_errors.fetch_add(1, Ordering::Relaxed);

        // Poison 页面
        self.poison_page(error.pfn, error.error_type)?;

        // 如果启用自动迁移，迁移页面
        if self.auto_migrate.load(Ordering::Relaxed) {
            let _ = self.migrate_poison_page(error.pfn);
        }

        // 根据策略处理
        let strategy = self.get_recovery_strategy();

        match strategy {
            RecoveryStrategy::KillProcess => {
                self.stats.killed_processes.fetch_add(1, Ordering::Relaxed);
                log::error!("Fatal error at PFN {}, killing processes", error.pfn);
            }
            RecoveryStrategy::Migrate => {
                log::error!("Fatal error at PFN {}, migrating page", error.pfn);
            }
            _ => {
                log::error!("Fatal error at PFN {}, no recovery action", error.pfn);
            }
        }

        Err(UnifiedError::Other("Hardware failure".to_string()))
    }

    /// 获取 PFN 所在节点
    fn get_node_for_pfn(&self, _pfn: Pfn) -> UnifiedResult<usize> {
        // 简化实现
        Ok(0)
    }

    /// 获取当前时间（纳秒）
    fn get_time_ns(&self) -> u64 {
        // 简化实现
        0
    }
}

impl Default for MemoryFailure {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局内存故障处理器
static MEMORY_FAILURE: OnceLock<Mutex<MemoryFailure>> = OnceLock::new();

/// 初始化内存故障处理
pub fn init_memory_failure() -> UnifiedResult<()> {
    let handler = MemoryFailure::new();
    MEMORY_FAILURE.get_or_init(|| Mutex::new(handler));
    Ok(())
}

/// 获取内存故障处理器
pub fn get_memory_failure() -> Option<&'static Mutex<MemoryFailure>> {
    MEMORY_FAILURE.get()
}

/// 处理内存错误（便捷函数）
pub fn handle_memory_error(error: MemoryError) -> UnifiedResult<()> {
    if let Some(handler) = get_memory_failure() {
        let mgr = handler.lock();
        mgr.handle_error(error)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

/// 标记页面为 poison（便捷函数）
pub fn poison_page(pfn: Pfn, error_type: ErrorType) -> UnifiedResult<()> {
    if let Some(handler) = get_memory_failure() {
        let mgr = handler.lock();
        mgr.poison_page(pfn, error_type)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

/// 检查页面是否被 poison（便捷函数）
pub fn is_page_poisoned(pfn: Pfn) -> bool {
    if let Some(handler) = get_memory_failure() {
        let mgr = handler.lock();
        mgr.is_page_poisoned(pfn)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_error() {
        let error = MemoryUnifiedError::new(0x1000, ErrorType::HardwareError, ErrorSeverity::Fatal);

        assert_eq!(error.pfn, 0x1000);
        assert_eq!(error.error_type, ErrorType::HardwareError);
        assert_eq!(error.severity, ErrorSeverity::Fatal);
        assert_eq!(error.error_count, 1);
    }

    #[test]
    fn test_poison_page() {
        let handler = MemoryFailure::new();

        // Poison 页面
        let result = handler.poison_page(0x1000, ErrorType::HardwareError);
        assert!(result.is_ok());

        // 检查是否 poison
        assert!(handler.is_page_poisoned(0x1000));

        // 获取状态
        let state = handler.get_poison_page_state(0x1000);
        assert_eq!(state, Some(PoisonPageState::Poisoned));

        // 清除 poison
        let result = handler.unpoison_page(0x1000);
        assert!(result.is_ok());

        // 应该不再 poison
        assert!(!handler.is_page_poisoned(0x1000));
    }

    #[test]
    fn test_correctable_error() {
        let handler = MemoryFailure::new();

        // 设置软错误阈值
        handler.set_soft_error_threshold(5);

        // 生成多个可纠正错误
        for _ in 0..4 {
            let error = MemoryUnifiedError::new(0x1000, ErrorType::SoftError, ErrorSeverity::Correctable);
            let result = handler.handle_error(error);
            assert!(result.is_ok());
        }

        // 应该仍然是正确处理
        let stats = handler.get_stats();
        assert_eq!(stats.correctable_errors.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn test_recovery_strategy() {
        let handler = MemoryFailure::new();

        // 设置策略
        handler.set_recovery_strategy(RecoveryStrategy::KillProcess);
        assert_eq!(handler.get_recovery_strategy(), RecoveryStrategy::KillProcess);

        handler.set_recovery_strategy(RecoveryStrategy::Migrate);
        assert_eq!(handler.get_recovery_strategy(), RecoveryStrategy::Migrate);
    }

    #[test]
    fn test_failure_stats() {
        let handler = MemoryFailure::new();

        // 处理一些错误
        let error1 = MemoryUnifiedError::new(0x1000, ErrorType::SoftError, ErrorSeverity::Correctable);
        let error2 = MemoryUnifiedError::new(0x2000, ErrorType::HardwareError, ErrorSeverity::Fatal);

        let _ = handler.handle_error(error1);
        let _ = handler.handle_error(error2);

        let stats = handler.get_stats();
        assert_eq!(stats.total_errors.load(Ordering::Relaxed), 2);
        assert_eq!(stats.correctable_errors.load(Ordering::Relaxed), 1);
        assert_eq!(stats.fatal_errors.load(Ordering::Relaxed), 1);
    }
}
