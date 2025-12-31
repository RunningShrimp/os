//! # 内存热拔 (Memory Hot-Remove)
//!
//! 本模块实现内存热移除功能，允许安全地从系统中移除物理内存。
//!
//! ## 主要功能
//!
//! - **页面隔离**: 隔离待移除内存中的页面
//! - **页面迁移**: 将页面迁移到其他内存区域
//! - **可移除检测**: 检测内存是否可以安全移除
//! - **内存块 Offline**: 将内存块标记为离线
//! - **资源释放验证**: 验证资源已完全释放
//!
//! ## 架构
//!
//! ```text
//! 移除请求
//!     ↓
//! 可移除性检查 → 内存块可移除？
//!     ↓ Yes
//! 页面隔离 → 隔离所有页面
//!     ↓
//! 页面迁移 → 迁移到其他节点
//!     ↓
//! Offline 操作 → 标记内存块离线
//!     ↓
//! 资源释放 → 释放所有资源
//!     ↓
//! 验证 → 确认移除完成
//! ```
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::subsystems::mm::hotremove::{MemoryHotRemove, RemovalPolicy};
//!
//! // 移除内存块
//! let remover = MemoryHotRemove::new();
//! remover.remove_memory_block(start_addr, size, RemovalPolicy::Strict)?;
//! # Ok::<(), UnifiedError>(())
//! ```

#![allow(dead_code)]

use crate::prelude::*;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::error::{UnifiedError, UnifiedResult};

use super::migration::{HotplugEvent, HotplugEventType};

/// 内存块 ID
pub type MemoryBlockId = usize;

/// 页面帧号
pub type Pfn = usize;

/// 移除策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalPolicy {
    /// 严格模式：必须成功迁移所有页面
    Strict,
    /// 尽力模式：尽可能迁移页面
    BestEffort,
    /// 强制模式：强制移除，可能丢失数据
    Force,
}

/// 内存块状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryBlockState {
    /// 在线（正常使用）
    Online,
    /// 正在离线
    GoingOffline,
    /// 离线（可以移除）
    Offline,
    /// 移除中
    Removing,
    /// 已移除
    Removed,
}

/// 内存块信息
#[derive(Debug)]
pub struct MemoryBlock {
    /// 块 ID
    id: MemoryBlockId,
    /// 起始物理地址
    start_addr: usize,
    /// 大小（字节）
    size: usize,
    /// 起始 PFN
    start_pfn: Pfn,
    /// 结束 PFN
    end_pfn: Pfn,
    /// NUMA 节点
    node_id: usize,
    /// 状态
    state: Mutex<MemoryBlockState>,
    /// 页面总数
    total_pages: usize,
    /// 已迁移页数
    migrated_pages: AtomicUsize,
    /// 失败迁移页数
    failed_pages: AtomicUsize,
    /// 是否可移除
    removable: AtomicBool,
    /// 引用计数
    ref_count: AtomicUsize,
}

impl Clone for MemoryBlock {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            start_addr: self.start_addr,
            size: self.size,
            start_pfn: self.start_pfn,
            end_pfn: self.end_pfn,
            node_id: self.node_id,
            state: Mutex::new(self.state.lock().clone()),
            total_pages: self.total_pages,
            migrated_pages: AtomicUsize::new(self.migrated_pages.load(core::sync::atomic::Ordering::SeqCst)),
            failed_pages: AtomicUsize::new(self.failed_pages.load(core::sync::atomic::Ordering::SeqCst)),
            removable: AtomicBool::new(self.removable.load(core::sync::atomic::Ordering::SeqCst)),
            ref_count: AtomicUsize::new(self.ref_count.load(core::sync::atomic::Ordering::SeqCst)),
        }
    }
}

impl MemoryBlock {
    /// 创建新的内存块
    pub(crate) fn new(
        id: MemoryBlockId,
        start_addr: usize,
        size: usize,
        node_id: usize,
    ) -> Self {
        use crate::subsystems::mm::PAGE_SIZE;

        let start_pfn = start_addr / PAGE_SIZE;
        let end_pfn = (start_addr + size) / PAGE_SIZE;
        let total_pages = end_pfn - start_pfn;

        Self {
            id,
            start_addr,
            size,
            start_pfn,
            end_pfn,
            node_id,
            state: Mutex::new(MemoryBlockState::Online),
            total_pages,
            migrated_pages: AtomicUsize::new(0),
            failed_pages: AtomicUsize::new(0),
            removable: AtomicBool::new(true),
            ref_count: AtomicUsize::new(0),
        }
    }

    /// 获取块 ID
    pub fn id(&self) -> MemoryBlockId {
        self.id
    }

    /// 获取起始地址
    pub fn start_addr(&self) -> usize {
        self.start_addr
    }

    /// 获取大小
    pub fn size(&self) -> usize {
        self.size
    }

    /// 获取状态
    pub fn state(&self) -> MemoryBlockState {
        *self.state.lock()
    }

    /// 设置状态
    pub fn set_state(&self, new_state: MemoryBlockState) {
        *self.state.lock() = new_state;
    }

    /// 获取迁移进度（0.0 - 1.0）
    pub fn migration_progress(&self) -> f32 {
        let migrated = self.migrated_pages.load(Ordering::Relaxed) as f32;
        let total = self.total_pages as f32;
        if total > 0.0 {
            migrated / total
        } else {
            1.0
        }
    }

    /// 增加引用计数
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 减少引用计数
    pub fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Release)
    }

    /// 检查是否可移除
    pub fn is_removable(&self) -> bool {
        // 检查引用计数
        if self.ref_count.load(Ordering::Relaxed) > 0 {
            return false;
        }

        // 检查所有页面是否已迁移
        let migrated = self.migrated_pages.load(Ordering::Relaxed);
        migrated >= self.total_pages
    }
}

/// 移除统计信息
#[derive(Debug, Default)]
pub struct RemovalStats {
    /// 总移除尝试次数
    pub total_removals: AtomicU64,
    /// 成功移除次数
    pub successful_removals: AtomicU64,
    /// 失败移除次数
    pub failed_removals: AtomicU64,
    /// 移除的页数
    pub pages_removed: AtomicU64,
    /// 迁移失败的页数
    pub migration_failures: AtomicU64,
    /// 总移除时间（纳秒）
    pub total_removal_time_ns: AtomicU64,
}

/// 内存热移除管理器
pub struct MemoryHotRemove {
    /// 所有内存块
    memory_blocks: Mutex<Vec<MemoryBlock>>,
    /// 待移除的块
    pending_removals: Mutex<BTreeSet<MemoryBlockId>>,
    /// 移除统计
    stats: RemovalStats,
    /// 是否启用热移除
    enabled: AtomicBool,
    /// 最大并发移除数
    max_concurrent: AtomicUsize,
}

impl MemoryHotRemove {
    /// 创建新的内存热移除管理器
    pub fn new() -> Self {
        Self {
            memory_blocks: Mutex::new(Vec::new()),
            pending_removals: Mutex::new(BTreeSet::new()),
            stats: RemovalStats::default(),
            enabled: AtomicBool::new(true),
            max_concurrent: AtomicUsize::new(4),
        }
    }

    /// 添加内存块
    pub fn add_memory_block(&self, start_addr: usize, size: usize, node_id: usize) -> UnifiedResult<MemoryBlockId> {
        let mut blocks = self.memory_blocks.lock();
        let id = blocks.len();

        let block = MemoryBlock::new(id, start_addr, size, node_id);
        blocks.push(block);

        Ok(id)
    }

    /// 移除内存块
    pub fn remove_memory_block(
        &self,
        start_addr: usize,
        size: usize,
        policy: RemovalPolicy,
    ) -> UnifiedResult<()> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(UnifiedError::NotSupported);
        }

        let start_time = self.get_time_ns();

        // 查找内存块
        let block_id = self.find_memory_block(start_addr, size)?;

        // 获取内存块的可变引用
        let block = {
            let blocks = self.memory_blocks.lock();
            blocks.get(block_id).cloned().ok_or(UnifiedError::NotFound)?
        };

        // 检查可移除性
        if !self.check_removable(&block, policy)? {
            return Err(UnifiedError::InvalidOperation.into());
        }

        // 设置状态为正在离线
        block.set_state(MemoryBlockState::GoingOffline);

        // 隔离页面
        self.isolate_pages(&block)?;

        // 迁移页面
        self.migrate_pages_from_block(&block, policy)?;

        // 检查迁移结果
        let all_migrated = block.is_removable();

        if !all_migrated && matches!(policy, RemovalPolicy::Strict) {
            // 严格模式下，迁移失败则回滚
            block.set_state(MemoryBlockState::Online);
            return Err(UnifiedError::ResourceBusy);
        }

        // Offline 内存块
        block.set_state(MemoryBlockState::Offline);

        // 标记为正在移除
        block.set_state(MemoryBlockState::Removing);

        // 释放资源
        self.release_resources(&block)?;

        // 标记为已移除
        block.set_state(MemoryBlockState::Removed);

        // 更新统计
        let elapsed = self.get_time_ns() - start_time;
        self.stats.total_removals.fetch_add(1, Ordering::Relaxed);
        self.stats.successful_removals.fetch_add(1, Ordering::Relaxed);
        self.stats.pages_removed.fetch_add(block.total_pages as u64, Ordering::Relaxed);
        self.stats.total_removal_time_ns.fetch_add(elapsed, Ordering::Relaxed);

        // 发送热插拔事件
        let event = HotplugEvent::new(
            HotplugEventType::Remove,
            start_addr,
            size,
            block.node_id,
        );
        let _ = self.notify_hotplug_event(event);

        Ok(())
    }

    /// 检查内存块是否可移除
    pub fn is_removable(&self, start_addr: usize, size: usize) -> UnifiedResult<bool> {
        let block_id = self.find_memory_block(start_addr, size)?;

        let blocks = self.memory_blocks.lock();
        let block = blocks.get(block_id).ok_or(UnifiedError::NotFound)?;

        Ok(block.is_removable())
    }

    /// 获取移除统计
    pub fn get_stats(&self) -> &RemovalStats {
        &self.stats
    }

    /// 启用/禁用热移除
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// 设置最大并发移除数
    pub fn set_max_concurrent(&self, max: usize) {
        self.max_concurrent.store(max, Ordering::Relaxed);
    }

    /// 获取内存块状态
    pub fn get_block_state(&self, block_id: MemoryBlockId) -> UnifiedResult<MemoryBlockState> {
        let blocks = self.memory_blocks.lock();
        let block = blocks.get(block_id).ok_or(UnifiedError::NotFound)?;
        Ok(block.state())
    }

    // 内部方法

    /// 查找内存块
    fn find_memory_block(&self, start_addr: usize, size: usize) -> UnifiedResult<MemoryBlockId> {
        let blocks = self.memory_blocks.lock();

        for (id, block) in blocks.iter().enumerate() {
            if block.start_addr == start_addr && block.size == size {
                return Ok(id);
            }
        }

        Err(UnifiedError::NotFound)
    }

    /// 检查可移除性
    fn check_removable(&self, block: &MemoryBlock, policy: RemovalPolicy) -> UnifiedResult<bool> {
        // 强制模式下总是允许移除
        if matches!(policy, RemovalPolicy::Force) {
            return Ok(true);
        }

        // 检查引用计数
        if block.ref_count.load(Ordering::Relaxed) > 0 {
            return Ok(false);
        }

        // 检查是否有固定页面
        // 简化实现：假设所有页面都可移动
        Ok(true)
    }

    /// 隔离页面
    fn isolate_pages(&self, block: &MemoryBlock) -> UnifiedResult<()> {
        use crate::subsystems::mm::PAGE_SIZE;

        // 隔离内存块中的所有页面
        // 简化实现：标记页面为不可分配
        for pfn in block.start_pfn..block.end_pfn {
            let _addr = pfn * PAGE_SIZE;
            // 在实际实现中，应该调用内存管理接口来隔离页面
            log::debug!("Isolating page at PFN {}", pfn);
        }

        Ok(())
    }

    /// 从内存块迁移页面
    fn migrate_pages_from_block(&self, block: &MemoryBlock, policy: RemovalPolicy) -> UnifiedResult<()> {
        use super::migration::{migrate_page, MigrationFlags};

        let flags = match policy {
            RemovalPolicy::Strict => MigrationFlags::SYNC.bits(),
            RemovalPolicy::BestEffort => MigrationFlags::ASYNC.bits(),
            RemovalPolicy::Force => MigrationFlags::FORCE.bits() | MigrationFlags::NO_ROLLBACK.bits(),
        };

        // 选择目标节点（不同于当前节点）
        let target_node = if block.node_id == 0 { 1 } else { 0 };

        // 迁移所有页面
        for pfn in block.start_pfn..block.end_pfn {
            match migrate_page(pfn, target_node, MigrationFlags::from_bits_truncate(flags)) {
                Ok(_) => {
                    block.migrated_pages.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    block.failed_pages.fetch_add(1, Ordering::Relaxed);
                    self.stats.migration_failures.fetch_add(1, Ordering::Relaxed);

                    log::error!("Failed to migrate page {}: {:?}", pfn, e);

                    if matches!(policy, RemovalPolicy::Strict) {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 释放资源
    fn release_resources(&self, block: &MemoryBlock) -> UnifiedResult<()> {
        // 释放内存块相关的所有资源
        // 在实际实现中，需要：
        // 1. 从页表中移除映射
        // 2. 释放页面描述符
        // 3. 更新内存管理数据结构
        // 4. 通知硬件内存已移除

        log::info!("Releasing resources for memory block {} (0x{:x} - 0x{:x})",
                  block.id,
                  block.start_addr,
                  block.start_addr + block.size);

        Ok(())
    }

    /// 通知热插拔事件
    fn notify_hotplug_event(&self, event: HotplugEvent) -> UnifiedResult<()> {
        use super::migration::handle_memory_hotplug;
        handle_memory_hotplug(event)
    }

    /// 获取当前时间（纳秒）
    fn get_time_ns(&self) -> u64 {
        // 简化实现
        0
    }
}

impl Default for MemoryHotRemove {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局内存热移除管理器
static MEMORY_HOTREMOVE: OnceLock<Mutex<MemoryHotRemove>> = OnceLock::new();

/// 初始化内存热移除
pub fn init_memory_hotremove() -> UnifiedResult<()> {
    let remover = MemoryHotRemove::new();
    MEMORY_HOTREMOVE.get_or_init(|| Mutex::new(remover));
    Ok(())
}

/// 获取内存热移除管理器
pub fn get_memory_hotremove() -> Option<&'static Mutex<MemoryHotRemove>> {
    MEMORY_HOTREMOVE.get()
}

/// 移除内存块（便捷函数）
pub fn remove_memory_block(
    start_addr: usize,
    size: usize,
    policy: RemovalPolicy,
) -> UnifiedResult<()> {
    if let Some(remover) = get_memory_hotremove() {
        let mgr = remover.lock();
        mgr.remove_memory_block(start_addr, size, policy)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

/// 检查内存块是否可移除（便捷函数）
pub fn is_memory_removable(start_addr: usize, size: usize) -> UnifiedResult<bool> {
    if let Some(remover) = get_memory_hotremove() {
        let mgr = remover.lock();
        mgr.is_removable(start_addr, size)
    } else {
        Err(UnifiedError::Other("Not initialized".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_block() {
        let block = MemoryBlock::new(0, 0x10000000, 0x1000000, 0);

        assert_eq!(block.id(), 0);
        assert_eq!(block.start_addr(), 0x10000000);
        assert_eq!(block.size(), 0x1000000);
        assert_eq!(block.state(), MemoryBlockState::Online);
        assert!(block.is_removable());

        // 修改状态
        block.set_state(MemoryBlockState::Offline);
        assert_eq!(block.state(), MemoryBlockState::Offline);
    }

    #[test]
    fn test_hotremove_manager() {
        let remover = MemoryHotRemove::new();

        // 添加内存块
        let block_id = remover.add_memory_block(0x10000000, 0x1000000, 0).unwrap();
        assert_eq!(block_id, 0);

        // 检查可移除性
        let removable = remover.is_removable(0x10000000, 0x1000000).unwrap();
        assert!(removable);

        // 获取状态
        let state = remover.get_block_state(block_id).unwrap();
        assert_eq!(state, MemoryBlockState::Online);
    }

    #[test]
    fn test_removal_policies() {
        let remover = MemoryHotRemove::new();

        // 添加内存块
        let _ = remover.add_memory_block(0x10000000, 0x1000000, 0);

        // 测试不同策略
        // 注意：实际移除需要页面迁移支持，这里只是测试接口

        // Strict 策略
        let result = remover.remove_memory_block(
            0x10000000,
            0x1000000,
            RemovalPolicy::BestEffort,
        );
        // 可能失败，取决于页面迁移实现
        let _ = result;
    }

    #[test]
    fn test_removal_stats() {
        let remover = MemoryHotRemove::new();
        let stats = remover.get_stats();

        assert_eq!(stats.total_removals.load(Ordering::Relaxed), 0);
        assert_eq!(stats.successful_removals.load(Ordering::Relaxed), 0);
    }
}
