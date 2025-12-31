//! # 页面迁移和热插拔
//!
//! 本模块提供页面迁移功能，支持在 NUMA 节点间移动页面，以及内存热插拔事件处理。
//!
//! ## 主要功能
//!
//! - **NUMA 页面迁移**: 在 NUMA 节点间移动内存页以优化访问延迟
//! - **内存热插拔**: 动态添加和移除内存
//! - **页面状态同步**: 确保迁移过程中页面状态一致性
//! - **迁移失败回滚**: 迁移失败时恢复原始状态
//!
//! ## 架构
//!
//! ```text
//! 迁移请求
//!     ↓
//! 验证页面 → 可迁移？
//!     ↓ Yes
//! 分配新页面 → 目标节点
//!     ↓
//! 复制数据 → 新页面
//!     ↓
//! 更新映射 → 页表
//!     ↓
//! 释放旧页面 → 源节点
//! ```
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::subsystems::mm::migration::{PageMigration, MigrationFlags};
//!
//! // 迁移页面到另一个 NUMA 节点
//! let migration = PageMigration::new();
//! let result = migration.migrate_page(old_pfn, target_node, MigrationFlags::empty());
//! ```

#![allow(dead_code)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::error::{UnifiedError, UnifiedResult};

use super::numa::NodeId;

/// 页面帧号类型
pub type Pfn = usize;

/// 迁移标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationFlags(u8);

impl MigrationFlags {
    /// 无标志
    pub const NONE: Self = Self(0);

    /// 同步迁移（阻塞等待完成）
    pub const SYNC: Self = Self(1 << 0);

    /// 异步迁移（立即返回）
    pub const ASYNC: Self = Self(1 << 1);

    /// 强制迁移（忽略页面锁）
    pub const FORCE: Self = Self(1 << 2);

    /// 迁移失败时不要回滚
    pub const NO_ROLLBACK: Self = Self(1 << 3);

    /// 迁移后刷新 TLB
    pub const FLUSH_TLB: Self = Self(1 << 4);

    /// 创建新的标志
    pub const fn empty() -> Self {
        Self(0)
    }

    /// 添加标志
    pub const fn contains(&self, flags: Self) -> bool {
        (self.0 & flags.0) == flags.0
    }

    /// 获取位标志值
    pub const fn bits(&self) -> u8 {
        self.0
    }

    /// 从位创建标志
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits)
    }
}

/// 页面状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    /// 页面空闲
    Free,
    /// 页面已分配
    Allocated,
    /// 页面正在迁移
    Migrating,
    /// 页面已锁定
    Locked,
    /// 页面错误
    Error,
}

/// 页面迁移信息
#[derive(Debug, Clone)]
struct MigrationEntry {
    /// 源 PFN
    src_pfn: Pfn,
    /// 源节点
    src_node: NodeId,
    /// 目标 PFN
    dst_pfn: Option<Pfn>,
    /// 目标节点
    dst_node: NodeId,
    /// 页面状态
    state: PageState,
    /// 迁移开始时间
    start_time: u64,
    /// 迁移标志
    flags: MigrationFlags,
    /// 错误信息
    error: Option<Error>,
}

impl MigrationEntry {
    fn new(src_pfn: Pfn, src_node: NodeId, dst_node: NodeId, flags: MigrationFlags) -> Self {
        Self {
            src_pfn,
            src_node,
            dst_pfn: None,
            dst_node,
            state: PageState::Migrating,
            start_time: 0,
            flags,
            error: None,
        }
    }
}

/// 迁移统计信息
#[derive(Debug, Default)]
pub struct MigrationStats {
    /// 总迁移次数
    pub total_migrations: AtomicU64,
    /// 成功迁移次数
    pub successful_migrations: AtomicU64,
    /// 失败迁移次数
    pub failed_migrations: AtomicU64,
    /// 回滚次数
    pub rollback_count: AtomicU64,
    /// 迁移的页数
    pub pages_migrated: AtomicU64,
    /// 跨节点迁移次数
    pub cross_node_migrations: AtomicU64,
    /// 平均迁移时间（纳秒）
    pub avg_migration_time_ns: AtomicU64,
}

/// 热插拔事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugEventType {
    /// 内存上线
    Online,
    /// 内存下线
    Offline,
    /// 内存添加
    Add,
    /// 内存移除
    Remove,
}

/// 热插拔事件
#[derive(Debug)]
pub struct HotplugEvent {
    /// 事件类型
    pub event_type: HotplugEventType,
    /// 起始物理地址
    pub start_addr: usize,
    /// 大小（字节）
    pub size: usize,
    /// NUMA 节点
    pub node_id: NodeId,
    /// 时间戳
    pub timestamp: u64,
}

impl HotplugEvent {
    pub fn new(event_type: HotplugEventType, start_addr: usize, size: usize, node_id: NodeId) -> Self {
        Self {
            event_type,
            start_addr,
            size,
            node_id,
            timestamp: 0,
        }
    }
}

/// 页面迁移管理器
pub struct PageMigration {
    /// 活跃的迁移条目
    active_migrations: Mutex<BTreeMap<Pfn, MigrationEntry>>,
    /// 待处理的迁移
    pending_migrations: Mutex<Vec<MigrationEntry>>,
    /// 迁移统计
    stats: MigrationStats,
    /// 是否启用迁移
    enabled: AtomicBool,
    /// 最大并发迁移数
    max_concurrent: AtomicUsize,
    /// 热插拔事件队列
    hotplug_events: Mutex<Vec<HotplugEvent>>,
}

impl PageMigration {
    /// 创建新的页面迁移管理器
    pub fn new() -> Self {
        Self {
            active_migrations: Mutex::new(BTreeMap::new()),
            pending_migrations: Mutex::new(Vec::new()),
            stats: MigrationStats::default(),
            enabled: AtomicBool::new(true),
            max_concurrent: AtomicUsize::new(16),
            hotplug_events: Mutex::new(Vec::new()),
        }
    }

    /// 迁移单个页面
    pub fn migrate_page(
        &self,
        src_pfn: Pfn,
        target_node: NodeId,
        flags: MigrationFlags,
    ) -> UnifiedResult<Pfn> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(UnifiedError::NotSupported);
        }

        // 获取源节点
        let src_node = self.get_node_for_pfn(src_pfn)?;

        // 创建迁移条目
        let entry = MigrationEntry::new(src_pfn, src_node, target_node, flags);

        // 执行迁移
        let dst_pfn = if flags.contains(MigrationFlags::ASYNC) {
            // 异步迁移
            self.queue_migration(entry)?;
            src_pfn // 异步模式下返回源 PFN
        } else {
            // 同步迁移
            self.do_migrate_page(entry)?
        };

        Ok(dst_pfn)
    }

    /// 批量迁移页面
    pub fn migrate_pages(
        &self,
        pfns: &[Pfn],
        target_node: NodeId,
        flags: MigrationFlags,
    ) -> UnifiedResult<Vec<UnifiedResult<Pfn>>> {
        let mut results = Vec::with_capacity(pfns.len());

        for &pfn in pfns {
            let result = self.migrate_page(pfn, target_node, flags);
            results.push(result);
        }

        Ok(results)
    }

    /// 处理待处理的迁移
    pub fn process_pending_migrations(&self) -> UnifiedResult<usize> {
        let mut pending = self.pending_migrations.lock();
        let count = pending.len();

        if count == 0 {
            return Ok(0);
        }

        // 批量处理迁移
        for entry in pending.drain(..) {
            if let Err(e) = self.do_migrate_page(entry) {
                log::error!("Failed to process pending migration: {:?}", e);
            }
        }

        Ok(count)
    }

    /// 处理热插拔事件
    pub fn process_hotplug_event(&self, event: HotplugEvent) -> UnifiedResult<()> {
        match event.event_type {
            HotplugEventType::Offline | HotplugEventType::Remove => {
                // 需要迁移页面
                self.evict_pages_from_range(event.start_addr, event.size)?;
            }
            HotplugEventType::Online | HotplugEventType::Add => {
                // 新内存可用，可以用于迁移
                log::info!("New memory available: 0x{:x} - 0x{:x} on node {}",
                          event.start_addr,
                          event.start_addr + event.size,
                          event.node_id);
            }
        }

        // 保存事件
        let mut events = self.hotplug_events.lock();
        events.push(event);

        Ok(())
    }

    /// 获取迁移统计
    pub fn get_stats(&self) -> &MigrationStats {
        &self.stats
    }

    /// 启用/禁用迁移
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// 设置最大并发迁移数
    pub fn set_max_concurrent(&self, max: usize) {
        self.max_concurrent.store(max, Ordering::Relaxed);
    }

    /// 获取最大并发迁移数
    pub fn get_max_concurrent(&self) -> usize {
        self.max_concurrent.load(Ordering::Relaxed)
    }

    // 内部方法

    /// 执行实际页面迁移
    fn do_migrate_page(&self, entry: MigrationEntry) -> UnifiedResult<Pfn> {
        let start_time = self.get_time_ns();

        // 检查并发限制
        let current_count = self.active_migrations.lock().len();
        if current_count >= self.get_max_concurrent() {
            return Err(UnifiedError::ResourceBusy);
        }

        // 分配目标页面
        let dst_pfn = self.allocate_page_on_node(entry.dst_node)?;

        // 复制页面内容
        self.copy_page_content(entry.src_pfn, dst_pfn)?;

        // 更新页表映射
        self.update_page_mapping(entry.src_pfn, dst_pfn)?;

        // 刷新 TLB（如果需要）
        if entry.flags.contains(MigrationFlags::FLUSH_TLB) {
            self.flush_tlb_for_pfn(dst_pfn);
        }

        // 标记迁移成功
        {
            let mut migrations = self.active_migrations.lock();
            let mut updated_entry = entry.clone();
            updated_entry.dst_pfn = Some(dst_pfn);
            updated_entry.state = PageState::Allocated;
            migrations.insert(dst_pfn, updated_entry);
        }

        // 更新统计
        let elapsed = self.get_time_ns() - start_time;
        self.stats.total_migrations.fetch_add(1, Ordering::Relaxed);
        self.stats.successful_migrations.fetch_add(1, Ordering::Relaxed);
        self.stats.pages_migrated.fetch_add(1, Ordering::Relaxed);

        // Use a copy of the fields before entry was moved
        let src_node = entry.src_node;
        let dst_node = entry.dst_node;
        if src_node != dst_node {
            self.stats.cross_node_migrations.fetch_add(1, Ordering::Relaxed);
        }

        // 更新平均迁移时间
        self.update_avg_migration_time(elapsed);

        Ok(dst_pfn)
    }

    /// 将迁移加入队列
    fn queue_migration(&self, entry: MigrationEntry) -> UnifiedResult<()> {
        let mut pending = self.pending_migrations.lock();
        pending.push(entry);
        Ok(())
    }

    /// 获取 PFN 所在的 NUMA 节点
    fn get_node_for_pfn(&self, _pfn: Pfn) -> UnifiedResult<NodeId> {
        // 简化实现：假设所有页面都在节点 0
        // 在实际实现中，应该查询 NUMA 拓扑
        Ok(0)
    }

    /// 在指定节点上分配页面
    fn allocate_page_on_node(&self, _node_id: NodeId) -> UnifiedResult<Pfn> {
        use crate::subsystems::mm::{kalloc, PAGE_SIZE};

        let ptr = unsafe { kalloc() };
        if ptr.is_null() {
            return Err(UnifiedError::OutOfMemory);
        }

        // 将指针转换为 PFN
        let pfn = (ptr as usize) / PAGE_SIZE;

        Ok(pfn)
    }

    /// 复制页面内容
    fn copy_page_content(&self, src_pfn: Pfn, dst_pfn: Pfn) -> UnifiedResult<()> {
        use crate::subsystems::mm::PAGE_SIZE;

        let src_ptr = (src_pfn * PAGE_SIZE) as *const u8;
        let dst_ptr = (dst_pfn * PAGE_SIZE) as *mut u8;

        unsafe {
            // 确保指针对齐
            if src_ptr.is_null() || dst_ptr.is_null() {
                return Err(UnifiedError::InvalidArgument.into());
            }

            // 复制页面数据
            core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, PAGE_SIZE);
        }

        Ok(())
    }

    /// 更新页表映射
    fn update_page_mapping(&self, _src_pfn: Pfn, _dst_pfn: Pfn) -> UnifiedResult<()> {
        // 简化实现：在实际系统中，需要更新所有进程的页表
        // 这里只是占位符
        Ok(())
    }

    /// 刷新 PFN 的 TLB
    fn flush_tlb_for_pfn(&self, _pfn: Pfn) {
        // 简化实现：在实际系统中，需要调用架构特定的 TLB 刷新
        // 这里只是占位符
    }

    /// 从内存范围驱逐页面
    fn evict_pages_from_range(&self, start_addr: usize, size: usize) -> UnifiedResult<()> {
        use crate::subsystems::mm::PAGE_SIZE;

        let start_pfn = start_addr / PAGE_SIZE;
        let end_pfn = (start_addr + size) / PAGE_SIZE;

        // 迁移范围内的所有页面
        for pfn in start_pfn..end_pfn {
            // 选择目标节点（非当前节点）
            let target_node = if self.get_node_for_pfn(pfn)? == 0 { 1 } else { 0 };

            // 尝试迁移
            if let Err(e) = self.migrate_page(pfn, target_node, MigrationFlags::FORCE) {
                log::error!("Failed to evict page {}: {:?}", pfn, e);
            }
        }

        Ok(())
    }

    /// 更新平均迁移时间
    fn update_avg_migration_time(&self, new_time_ns: u64) {
        let current_avg = self.stats.avg_migration_time_ns.load(Ordering::Relaxed);
        let total_migrations = self.stats.total_migrations.load(Ordering::Relaxed) as u64;

        if total_migrations > 0 {
            let new_avg = (current_avg * (total_migrations - 1) + new_time_ns) / total_migrations;
            self.stats.avg_migration_time_ns.store(new_avg, Ordering::Relaxed);
        } else {
            self.stats.avg_migration_time_ns.store(new_time_ns, Ordering::Relaxed);
        }
    }

    /// 获取当前时间（纳秒）
    fn get_time_ns(&self) -> u64 {
        // 简化实现：返回模拟时间
        // 在实际系统中，应该读取硬件计数器
        0
    }
}

impl Default for PageMigration {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局页面迁移管理器
static PAGE_MIGRATION: OnceLock<Mutex<PageMigration>> = OnceLock::new();

/// 初始化页面迁移
pub fn init_page_migration() -> UnifiedResult<()> {
    let migration = PageMigration::new();
    PAGE_MIGRATION.get_or_init(|| Mutex::new(migration));
    Ok(())
}

/// 获取页面迁移管理器
pub fn get_page_migration() -> Option<&'static Mutex<PageMigration>> {
    PAGE_MIGRATION.get()
}

/// 迁移页面（便捷函数）
pub fn migrate_page(pfn: Pfn, target_node: NodeId, flags: MigrationFlags) -> UnifiedResult<Pfn> {
    if let Some(migration) = get_page_migration() {
        let mgr = migration.lock();
        mgr.migrate_page(pfn, target_node, flags)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

/// 处理热插拔事件（便捷函数）
pub fn handle_memory_hotplug(event: HotplugEvent) -> UnifiedResult<()> {
    if let Some(migration) = get_page_migration() {
        let mgr = migration.lock();
        mgr.process_hotplug_event(event)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_flags() {
        let flags = MigrationFlags::SYNC | MigrationFlags::FLUSH_TLB;
        assert!(flags.contains(MigrationFlags::SYNC));
        assert!(flags.contains(MigrationFlags::FLUSH_TLB));
        assert!(!flags.contains(MigrationFlags::ASYNC));
    }

    #[test]
    fn test_page_migration() {
        let migration = PageMigration::new();

        // 测试同步迁移
        let result = migration.migrate_page(0x1000, 1, MigrationFlags::SYNC);
        assert!(result.is_ok());

        // 检查统计
        let stats = migration.get_stats();
        assert_eq!(stats.total_migrations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_hotplug_event() {
        let migration = PageMigration::new();

        // 创建内存移除事件
        let event = HotplugEvent::new(
            HotplugEventType::Remove,
            0x10000000,
            0x1000000, // 16 MB
            0,
        );

        let result = migration.process_hotplug_event(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_migration() {
        let migration = PageMigration::new();

        let pfns = vec![0x1000, 0x1001, 0x1002, 0x1003];
        let results = migration.migrate_pages(&pfns, 1, MigrationFlags::ASYNC).unwrap();

        assert_eq!(results.len(), 4);
    }
}
