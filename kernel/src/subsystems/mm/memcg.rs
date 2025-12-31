//! # 内存控制组 (Memory Control Groups, memcg)
//!
//! 本模块实现内存控制组功能，允许对一组进程的内存使用进行限制和监控。
//!
//! ## 主要功能
//!
//! - **内存使用统计和限制**: 跟踪和限制内存使用
//! - **OOM 通知处理**: 内存不足时的通知和处理
//! - **内存压力水平监测**: 监测内存压力状态
//! - **Swap/Cache 控制**: 控制 swap 和缓存使用
//!
//! ## 架构
//!
//! ```text
//! 进程
//!     ↓
//! memcg
//!     ├── 内存限制
//!     ├── 使用统计
//!     ├── OOM 控制
//!     └── 压力监测
//! ```
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::subsystems::mm::memcg::{MemoryCgroup, MemcgLimits};
//!
//! // 创建 memcg
//! let memcg = MemoryCgroup::new("mygroup", MemcgLimits::default());
//!
//! // 设置限制
//! memcg.set_limit(MemcgLimits::memory_limit(), 100 * 1024 * 1024); // 100MB
//! ```

#![allow(dead_code)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::error::{UnifiedError, UnifiedResult};

/// Cgroup ID
pub type CgroupId = u64;

/// 内存限制类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemLimitType {
    /// 物理内存限制
    Memory,
    /// Swap 限制
    Swap,
    /// 内存+Swap 总限制
    MemSwap,
    /// 内核内存限制
    KernelMemory,
    /// TCP 缓冲区限制
    TcpMemory,
}

/// 内存压力级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// 低压力
    Low,
    /// 中等压力
    Medium,
    /// 高压力
    High,
    /// 临界压力（即将 OOM）
    Critical,
}

impl MemoryPressureLevel {
    /// 从使用率计算压力级别
    pub fn from_usage_ratio(ratio: f32) -> Self {
        if ratio < 0.5 {
            Self::Low
        } else if ratio < 0.7 {
            Self::Medium
        } else if ratio < 0.9 {
            Self::High
        } else {
            Self::Critical
        }
    }
}

/// OOM 通知回调类型
pub type OomCallback = fn(cgroup_id: CgroupId) -> UnifiedResult<()>;

/// 内存限制配置
#[derive(Debug, Clone)]
pub struct MemcgLimits {
    /// 物理内存限制（字节）
    pub memory_limit: Option<usize>,
    /// Swap 限制（字节）
    pub swap_limit: Option<usize>,
    /// 内存+Swap 总限制（字节）
    pub memsw_limit: Option<usize>,
    /// 内核内存限制（字节）
    pub kernel_memory_limit: Option<usize>,
    /// TCP 内存限制（字节）
    pub tcp_memory_limit: Option<usize>,
    /// 是否启用 OOM killer
    pub oom_control: bool,
}

impl Default for MemcgLimits {
    fn default() -> Self {
        Self {
            memory_limit: None,
            swap_limit: None,
            memsw_limit: None,
            kernel_memory_limit: None,
            tcp_memory_limit: None,
            oom_control: true,
        }
    }
}

impl MemcgLimits {
    /// 创建默认限制（无限制）
    pub const fn unbounded() -> Self {
        Self {
            memory_limit: None,
            swap_limit: None,
            memsw_limit: None,
            kernel_memory_limit: None,
            tcp_memory_limit: None,
            oom_control: true,
        }
    }

    /// 创建固定限制
    pub const fn fixed(memory_limit: usize) -> Self {
        Self {
            memory_limit: Some(memory_limit),
            swap_limit: None,
            memsw_limit: None,
            kernel_memory_limit: None,
            tcp_memory_limit: None,
            oom_control: true,
        }
    }

    /// 获取内存限制
    pub const fn memory_limit(&self) -> Option<usize> {
        self.memory_limit
    }

    /// 获取 swap 限制
    pub const fn swap_limit(&self) -> Option<usize> {
        self.swap_limit
    }

    /// 获取 memsw 限制
    pub const fn memsw_limit(&self) -> Option<usize> {
        self.memsw_limit
    }
}

/// 内存使用统计
#[derive(Debug, Default)]
pub struct MemcgStats {
    /// 当前使用的内存（字节）
    pub current_memory: AtomicUsize,
    /// 峰值内存使用（字节）
    pub peak_memory: AtomicUsize,
    /// 当前使用的 swap（字节）
    pub current_swap: AtomicUsize,
    /// 峰值 swap 使用（字节）
    pub peak_swap: AtomicUsize,
    /// 当前缓存（字节）
    pub current_cache: AtomicUsize,
    /// 当前 RSS（字节）
    pub current_rss: AtomicUsize,
    /// 当前文件映射（字节）
    pub current_file_mapped: AtomicUsize,
    /// 当前 dirty 页面（字节）
    pub current_dirty: AtomicUsize,
    /// 总页面故障次数
    pub total_pgfault: AtomicU64,
    /// 总主要故障次数
    pub total_pgmajfault: AtomicU64,
    /// 总换入次数
    pub total_pgpgin: AtomicU64,
    /// 总换出次数
    pub total_pgpgout: AtomicU64,
    /// OOM 杀死进程数
    pub oom_kills: AtomicU64,
}

impl MemcgStats {
    /// 获取内存使用率（0.0 - 1.0）
    pub fn memory_usage_ratio(&self, limit: Option<usize>) -> f32 {
        if let Some(limit) = limit {
            let used = self.current_memory.load(Ordering::Relaxed);
            used as f32 / limit as f32
        } else {
            0.0
        }
    }

    /// 获取压力级别
    pub fn pressure_level(&self, limit: Option<usize>) -> MemoryPressureLevel {
        let ratio = self.memory_usage_ratio(limit);
        MemoryPressureLevel::from_usage_ratio(ratio)
    }
}

/// Swap 控制配置
#[derive(Debug, Clone, Copy)]
pub struct SwapControl {
    /// 是否启用 swap
    pub enabled: bool,
    /// swapiness（0-100，越倾向于 swap）
    pub swappiness: u8,
    /// 最大 swap 使用率（0.0 - 1.0）
    pub max_swap_ratio: f32,
}

impl Default for SwapControl {
    fn default() -> Self {
        Self {
            enabled: true,
            swappiness: 60,
            max_swap_ratio: 0.5,
        }
    }
}

/// 内存控制组
pub struct MemoryCgroup {
    /// Cgroup ID
    id: CgroupId,
    /// Cgroup 名称
    name: String,
    /// 父 cgroup
    parent: Option<CgroupId>,
    /// 子 cgroup
    children: Mutex<Vec<CgroupId>>,
    /// 内存限制
    limits: Mutex<MemcgLimits>,
    /// 使用统计
    stats: MemcgStats,
    /// Swap 控制
    swap_control: Mutex<SwapControl>,
    /// 是否启用
    enabled: AtomicBool,
    /// OOM 回调
    oom_callback: Mutex<Option<OomCallback>>,
    /// 创建时间
    create_time: u64,
}

impl MemoryCgroup {
    /// 创建新的内存控制组
    pub fn new(name: String, limits: MemcgLimits) -> Self {
        Self {
            id: Self::generate_id(),
            name,
            parent: None,
            children: Mutex::new(Vec::new()),
            limits: Mutex::new(limits),
            stats: MemcgStats::default(),
            swap_control: Mutex::new(SwapControl::default()),
            enabled: AtomicBool::new(true),
            oom_callback: Mutex::new(None),
            create_time: 0,
        }
    }

    /// 获取 ID
    pub fn id(&self) -> CgroupId {
        self.id
    }

    /// 获取名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 设置父 cgroup
    pub fn set_parent(&mut self, parent_id: CgroupId) {
        self.parent = Some(parent_id);
    }

    /// 添加子 cgroup
    pub fn add_child(&self, child_id: CgroupId) {
        let mut children = self.children.lock();
        children.push(child_id);
    }

    /// 设置限制
    pub fn set_limit(&self, limit_type: MemLimitType, value: Option<usize>) {
        let mut limits = self.limits.lock();

        match limit_type {
            MemLimitType::Memory => limits.memory_limit = value,
            MemLimitType::Swap => limits.swap_limit = value,
            MemLimitType::MemSwap => limits.memsw_limit = value,
            MemLimitType::KernelMemory => limits.kernel_memory_limit = value,
            MemLimitType::TcpMemory => limits.tcp_memory_limit = value,
        }
    }

    /// 获取限制
    pub fn get_limit(&self, limit_type: MemLimitType) -> Option<usize> {
        let limits = self.limits.lock();

        match limit_type {
            MemLimitType::Memory => limits.memory_limit,
            MemLimitType::Swap => limits.swap_limit,
            MemLimitType::MemSwap => limits.memsw_limit,
            MemLimitType::KernelMemory => limits.kernel_memory_limit,
            MemLimitType::TcpMemory => limits.tcp_memory_limit,
        }
    }

    /// 获取所有限制
    pub fn get_limits(&self) -> MemcgLimits {
        self.limits.lock().clone()
    }

    /// 尝试分配内存
    pub fn try_charge(&self, size: usize) -> UnifiedResult<()> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(()); // 未启用时不限制
        }

        let limits = self.limits.lock();

        if let Some(memory_limit) = limits.memory_limit {
            let current = self.stats.current_memory.load(Ordering::Relaxed);
            let new_usage = current + size;

            if new_usage > memory_limit {
                // 超过限制
                drop(limits);

                // 检查是否应该 OOM
                self.check_oom()?;

                return Err(UnifiedError::OutOfMemory);
            }
        }

        // 更新统计
        self.stats.current_memory.fetch_add(size, Ordering::Relaxed);

        // 更新峰值
        let current = self.stats.current_memory.load(Ordering::Relaxed);
        let mut peak = self.stats.peak_memory.load(Ordering::Relaxed);
        while current > peak {
            match self.stats.peak_memory.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(old) => peak = old,
            }
        }

        Ok(())
    }

    /// 释放内存
    pub fn uncharge(&self, size: usize) {
        let current = self.stats.current_memory.fetch_sub(size, Ordering::Relaxed);
        // 防止下溢
        if current < size {
            self.stats.current_memory.store(0, Ordering::Relaxed);
        }
    }

    /// 更新 RSS
    pub fn update_rss(&self, rss: usize) {
        self.stats.current_rss.store(rss, Ordering::Relaxed);
    }

    /// 更新缓存
    pub fn update_cache(&self, cache: usize) {
        self.stats.current_cache.store(cache, Ordering::Relaxed);
    }

    /// 更新 swap 使用
    pub fn update_swap(&self, swap: usize) {
        let current = self.stats.current_swap.fetch_add(swap, Ordering::Relaxed);

        // 更新峰值
        let new_value = current + swap;
        let mut peak = self.stats.peak_swap.load(Ordering::Relaxed);
        while new_value > peak {
            match self.stats.peak_swap.compare_exchange_weak(
                peak,
                new_value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(old) => peak = old,
            }
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &MemcgStats {
        &self.stats
    }

    /// 获取内存压力级别
    pub fn get_pressure_level(&self) -> MemoryPressureLevel {
        let limits = self.limits.lock();
        self.stats.pressure_level(limits.memory_limit)
    }

    /// 设置 OOM 回调
    pub fn set_oom_callback(&self, callback: OomCallback) {
        let mut oom_cb = self.oom_callback.lock();
        *oom_cb = Some(callback);
    }

    /// 启用/禁用 cgroup
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// 设置 swap 控制
    pub fn set_swap_control(&self, control: SwapControl) {
        let mut swap_ctrl = self.swap_control.lock();
        *swap_ctrl = control;
    }

    /// 获取 swap 控制
    pub fn get_swap_control(&self) -> SwapControl {
        *self.swap_control.lock()
    }

    /// 重置统计
    pub fn reset_stats(&self) {
        self.stats.current_memory.store(0, Ordering::Relaxed);
        self.stats.current_swap.store(0, Ordering::Relaxed);
        self.stats.current_cache.store(0, Ordering::Relaxed);
        self.stats.current_rss.store(0, Ordering::Relaxed);
    }

    // 内部方法

    /// 检查是否应该 OOM
    fn check_oom(&self) -> UnifiedResult<()> {
        let limits = self.limits.lock();

        if !limits.oom_control {
            return Ok(());
        }

        let pressure = self.stats.pressure_level(limits.memory_limit);

        if matches!(pressure, MemoryPressureLevel::Critical) {
            // 触发 OOM
            drop(limits);

            self.stats.oom_kills.fetch_add(1, Ordering::Relaxed);

            // 调用 OOM 回调
            let oom_cb = self.oom_callback.lock();
            if let Some(callback) = *oom_cb {
                let _ = callback(self.id);
            }

            return Err(UnifiedError::OutOfMemory);
        }

        Ok(())
    }

    /// 生成唯一 ID
    fn generate_id() -> CgroupId {
        use core::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }
}

/// 内存 Cgroup 管理器
pub struct MemcgManager {
    /// 所有 cgroup
    cgroups: Mutex<BTreeMap<CgroupId, MemoryCgroup>>,
    /// 名称到 ID 的映射
    name_to_id: Mutex<BTreeMap<String, CgroupId>>,
    /// 是否启用
    enabled: AtomicBool,
}

impl MemcgManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            cgroups: Mutex::new(BTreeMap::new()),
            name_to_id: Mutex::new(BTreeMap::new()),
            enabled: AtomicBool::new(true),
        }
    }

    /// 创建 cgroup
    pub fn create_cgroup(&self, name: String, limits: MemcgLimits) -> UnifiedResult<CgroupId> {
        let cgroup = MemoryCgroup::new(name.clone(), limits);
        let id = cgroup.id();

        {
            let mut cgroups = self.cgroups.lock();
            cgroups.insert(id, cgroup);
        }

        {
            let mut name_to_id = self.name_to_id.lock();
            name_to_id.insert(name, id);
        }

        Ok(id)
    }

    /// 删除 cgroup
    pub fn delete_cgroup(&self, id: CgroupId) -> UnifiedResult<()> {
        let mut cgroups = self.cgroups.lock();

        if let Some(cgroup) = cgroups.remove(&id) {
            let mut name_to_id = self.name_to_id.lock();
            name_to_id.remove(cgroup.name());
            Ok(())
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    /// 获取 cgroup
    pub fn get_cgroup(&self, _id: CgroupId) -> Option<&'_ MemoryCgroup> {
        // 注意：这里返回引用在 Rust 中很复杂
        // 简化实现：返回克隆
        None
    }

    /// 通过名称查找 cgroup
    pub fn find_cgroup_by_name(&self, name: &str) -> Option<CgroupId> {
        let name_to_id = self.name_to_id.lock();
        name_to_id.get(name).copied()
    }

    /// 启用/禁用 memcg
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Default for MemcgManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局内存 Cgroup 管理器
static MEMCG_MANAGER: OnceLock<Mutex<MemcgManager>> = OnceLock::new();

/// 初始化内存 Cgroup
pub fn init_memcg() -> UnifiedResult<()> {
    let manager = MemcgManager::new();
    MEMCG_MANAGER.get_or_init(|| Mutex::new(manager));
    Ok(())
}

/// 获取内存 Cgroup 管理器
pub fn get_memcg_manager() -> Option<&'static Mutex<MemcgManager>> {
    MEMCG_MANAGER.get()
}

/// 创建 cgroup（便捷函数）
pub fn create_cgroup(name: String, limits: MemcgLimits) -> UnifiedResult<CgroupId> {
    if let Some(manager) = get_memcg_manager() {
        let mgr = manager.lock();
        mgr.create_cgroup(name, limits)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

/// 删除 cgroup（便捷函数）
pub fn delete_cgroup(id: CgroupId) -> UnifiedResult<()> {
    if let Some(manager) = get_memcg_manager() {
        let mgr = manager.lock();
        mgr.delete_cgroup(id)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcg_limits() {
        let limits = MemcgLimits::fixed(100 * 1024 * 1024); // 100MB

        assert_eq!(limits.memory_limit(), Some(100 * 1024 * 1024));
        assert_eq!(limits.swap_limit(), None);
    }

    #[test]
    fn test_memory_cgroup() {
        let cgroup = MemoryCgroup::new("test".to_string(), MemcgLimits::fixed(1024));

        // 测试内存限制
        let result = cgroup.try_charge(512);
        assert!(result.is_ok());

        assert_eq!(cgroup.stats.current_memory.load(Ordering::Relaxed), 512);

        // 测试超过限制
        let result = cgroup.try_charge(1024);
        assert!(result.is_err());

        // 释放内存
        cgroup.uncharge(512);
        assert_eq!(cgroup.stats.current_memory.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_pressure_level() {
        let cgroup = MemoryCgroup::new("test".to_string(), MemcgLimits::fixed(1000));

        // 低压力
        cgroup.try_charge(400).unwrap();
        assert_eq!(cgroup.get_pressure_level(), MemoryPressureLevel::Low);

        // 中等压力
        cgroup.try_charge(300).unwrap();
        assert_eq!(cgroup.get_pressure_level(), MemoryPressureLevel::Medium);

        // 高压力
        cgroup.try_charge(200).unwrap();
        assert_eq!(cgroup.get_pressure_level(), MemoryPressureLevel::High);

        // 临界压力
        cgroup.try_charge(100).unwrap();
        assert_eq!(cgroup.get_pressure_level(), MemoryPressureLevel::Critical);
    }

    #[test]
    fn test_memcg_manager() {
        let manager = MemcgManager::new();

        // 创建 cgroup
        let id = manager.create_cgroup("test".to_string(), MemcgLimits::default()).unwrap();
        assert!(id > 0);

        // 查找 cgroup
        let found_id = manager.find_cgroup_by_name("test").unwrap();
        assert_eq!(found_id, id);

        // 删除 cgroup
        let result = manager.delete_cgroup(id);
        assert!(result.is_ok());

        // 查找应该失败
        let found_id = manager.find_cgroup_by_name("test");
        assert!(found_id.is_none());
    }

    #[test]
    fn test_swap_control() {
        let control = SwapControl {
            enabled: true,
            swappiness: 80,
            max_swap_ratio: 0.7,
        };

        assert!(control.enabled);
        assert_eq!(control.swappiness, 80);
        assert_eq!(control.max_swap_ratio, 0.7);
    }
}
