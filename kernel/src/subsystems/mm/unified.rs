//! # 内存与存储统一编址
//!
//! 提供内存和存储的统一地址空间管理，实现数据自动分层。
//!
//! ## 功能
//!
//! - 混合内存/存储管理
//! - 数据放置策略
//! - 自动分层（hot/cold data）
//! - 迁移引擎
//! - 统一寻址
//!
//! ## 架构
//!
//! ```
//! 统一地址空间
//!     ├── 内存层（DRAM）
//!     ├── 持久化内存层（SCM）
//!     ├── 存储层（SSD/HDD）
//!     ├── 数据分类器
//!     └── 迁移引擎
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use nos_api::Error;

use crate::subsystems::sync::Mutex as AdvancedMutex;

/// 内存层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryTier {
    /// DRAM 层（最快）
    Dram = 0,
    /// 持久化内存层（中等）
    Scm = 1,
    /// SSD 层（较慢）
    Ssd = 2,
    /// HDD 层（最慢）
    Hdd = 3,
}

/// 数据温度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTemperature {
    /// 热（频繁访问）
    Hot,
    /// 温（偶尔访问）
    Warm,
    /// 冷（很少访问）
    Cold,
}

/// 数据块状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    /// 活动
    Active,
    /// 迁移中
    Migrating,
    /// 脏（需写回）
    Dirty,
    /// 清洁
    Clean,
}

/// 数据块元数据
#[derive(Debug)]
pub struct BlockMetadata {
    /// 虚拟地址
    pub virt_addr: u64,
    /// 当前所在层级
    pub tier: MemoryTier,
    /// 物理地址
    pub phys_addr: u64,
    /// 块大小
    pub size: u64,
    /// 访问计数
    pub access_count: AtomicU64,
    /// 上次访问时间
    pub last_access: AtomicU64,
    /// 数据温度
    pub temperature: DataTemperature,
    /// 状态
    pub state: BlockState,
}

impl Clone for BlockMetadata {
    fn clone(&self) -> Self {
        Self {
            virt_addr: self.virt_addr,
            tier: self.tier,
            phys_addr: self.phys_addr,
            size: self.size,
            access_count: AtomicU64::new(self.access_count.load(core::sync::atomic::Ordering::SeqCst)),
            last_access: AtomicU64::new(self.last_access.load(core::sync::atomic::Ordering::SeqCst)),
            temperature: self.temperature,
            state: self.state,
        }
    }
}

/// 迁移操作
#[derive(Debug, Clone)]
pub struct MigrationOperation {
    /// 源层级
    pub src_tier: MemoryTier,
    /// 目标层级
    pub dst_tier: MemoryTier,
    /// 源地址
    pub src_addr: u64,
    /// 目标地址
    pub dst_addr: u64,
    /// 大小
    pub size: u64,
    /// 操作 ID
    pub op_id: u64,
    /// 是否完成
    pub completed: bool,
}

/// 分层策略
#[derive(Debug, Clone, Copy)]
pub enum TieringPolicy {
    /// 基于访问频率
    AccessFrequency,
    /// 基于时间局部性
    TemporalLocality,
    /// 基于空间局部性
    SpatialLocality,
    /// 混合策略
    Hybrid,
}

/// 统一地址空间配置
#[derive(Debug, Clone)]
pub struct UnifiedConfig {
    /// DRAM 大小（字节）
    pub dram_size: u64,
    /// SCM 大小（字节）
    pub scm_size: u64,
    /// SSD 大小（字节）
    pub ssd_size: u64,
    /// 块大小（字节）
    pub block_size: u64,
    /// 分层策略
    pub tiering_policy: TieringPolicy,
    /// 热数据阈值（访问次数/秒）
    pub hot_threshold: u64,
    /// 温数据阈值（访问次数/秒）
    pub warm_threshold: u64,
    /// 迁移间隔（秒）
    pub migration_interval: u64,
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        Self {
            dram_size: 16 * 1024 * 1024 * 1024, // 16 GB
            scm_size: 64 * 1024 * 1024 * 1024, // 64 GB
            ssd_size: 512 * 1024 * 1024 * 1024, // 512 GB
            block_size: 4096, // 4 KB
            tiering_policy: TieringPolicy::Hybrid,
            hot_threshold: 1000,
            warm_threshold: 100,
            migration_interval: 60,
        }
    }
}

/// 内存层级描述符
#[derive(Debug)]
pub struct MemoryTierDescriptor {
    /// 层级类型
    pub tier_type: MemoryTier,
    /// 基址
    pub base: u64,
    /// 大小
    pub size: u64,
    /// 已使用大小
    pub used_size: AtomicU64,
    /// 是否持久化
    pub persistent: bool,
}

impl Clone for MemoryTierDescriptor {
    fn clone(&self) -> Self {
        Self {
            tier_type: self.tier_type,
            base: self.base,
            size: self.size,
            used_size: AtomicU64::new(self.used_size.load(core::sync::atomic::Ordering::SeqCst)),
            persistent: self.persistent,
        }
    }
}

/// 统一地址空间管理器
#[derive(Debug)]
pub struct UnifiedAddressSpace {
    /// 配置
    pub config: UnifiedConfig,
    /// 内存层级
    pub tiers: Vec<MemoryTierDescriptor>,
    /// 块元数据映射
    pub blocks: Mutex<BTreeMap<u64, Arc<BlockMetadata>>>,
    /// 迁移操作队列
    pub migrations: Mutex<Vec<MigrationOperation>>,
    /// 下一个迁移操作 ID
    pub next_migration_id: AtomicU64,
    /// 统计信息
    pub stats: UnifiedStats,
    /// 是否启用
    pub enabled: bool,
}

#[derive(Debug)]
pub struct UnifiedStats {
    pub total_blocks: AtomicU64,
    pub dram_blocks: AtomicU64,
    pub scm_blocks: AtomicU64,
    pub ssd_blocks: AtomicU64,
    pub hot_blocks: AtomicU64,
    pub warm_blocks: AtomicU64,
    pub cold_blocks: AtomicU64,
    pub migration_count: AtomicU64,
    pub promotion_count: AtomicU64,
    pub demotion_count: AtomicU64,
}

impl Clone for UnifiedStats {
    fn clone(&self) -> Self {
        Self {
            total_blocks: AtomicU64::new(self.total_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            dram_blocks: AtomicU64::new(self.dram_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            scm_blocks: AtomicU64::new(self.scm_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            ssd_blocks: AtomicU64::new(self.ssd_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            hot_blocks: AtomicU64::new(self.hot_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            warm_blocks: AtomicU64::new(self.warm_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            cold_blocks: AtomicU64::new(self.cold_blocks.load(core::sync::atomic::Ordering::SeqCst)),
            migration_count: AtomicU64::new(self.migration_count.load(core::sync::atomic::Ordering::SeqCst)),
            promotion_count: AtomicU64::new(self.promotion_count.load(core::sync::atomic::Ordering::SeqCst)),
            demotion_count: AtomicU64::new(self.demotion_count.load(core::sync::atomic::Ordering::SeqCst)),
        }
    }
}

impl Default for UnifiedStats {
    fn default() -> Self {
        Self {
            total_blocks: AtomicU64::new(0),
            dram_blocks: AtomicU64::new(0),
            scm_blocks: AtomicU64::new(0),
            ssd_blocks: AtomicU64::new(0),
            hot_blocks: AtomicU64::new(0),
            warm_blocks: AtomicU64::new(0),
            cold_blocks: AtomicU64::new(0),
            migration_count: AtomicU64::new(0),
            promotion_count: AtomicU64::new(0),
            demotion_count: AtomicU64::new(0),
        }
    }
}

impl UnifiedAddressSpace {
    /// 创建新的统一地址空间
    pub fn new(config: UnifiedConfig) -> Self {
        let tiers = vec![
            MemoryTierDescriptor {
                tier_type: MemoryTier::Dram,
                base: 0x8000_0000,
                size: config.dram_size,
                used_size: AtomicU64::new(0),
                persistent: false,
            },
            MemoryTierDescriptor {
                tier_type: MemoryTier::Scm,
                base: 0x1000_0000_0000,
                size: config.scm_size,
                used_size: AtomicU64::new(0),
                persistent: true,
            },
            MemoryTierDescriptor {
                tier_type: MemoryTier::Ssd,
                base: 0x2000_0000_0000,
                size: config.ssd_size,
                used_size: AtomicU64::new(0),
                persistent: true,
            },
        ];

        Self {
            config,
            tiers,
            blocks: Mutex::new(BTreeMap::new()),
            migrations: Mutex::new(Vec::new()),
            next_migration_id: AtomicU64::new(1),
            stats: UnifiedStats::default(),
            enabled: false,
        }
    }

    /// 初始化统一地址空间
    pub fn init(&mut self) {
        self.enabled = true;
        crate::println!("[unified] Unified address space initialized");
        crate::println!("[unified]   DRAM: 0x{:x} - 0x{:x}",
            self.tiers[0].base, self.tiers[0].base + self.tiers[0].size);
        crate::println!("[unified]   SCM:  0x{:x} - 0x{:x}",
            self.tiers[1].base, self.tiers[1].base + self.tiers[1].size);
        crate::println!("[unified]   SSD:  0x{:x} - 0x{:x}",
            self.tiers[2].base, self.tiers[2].base + self.tiers[2].size);
    }

    /// 分配块
    pub fn allocate_block(&self, virt_addr: u64, size: u64) -> Result<Arc<BlockMetadata>, Error> {
        if !self.enabled {
            return Err(Error::InvalidState("unified memory manager not enabled".to_string()));
        }

        // 默认分配到 DRAM
        let tier = &self.tiers[0]; // DRAM
        let offset = tier.used_size.fetch_add(size, Ordering::Relaxed);

        if offset + size > tier.size {
            return Err(Error::OutOfMemory);
        }

        let metadata = Arc::new(BlockMetadata {
            virt_addr,
            tier: MemoryTier::Dram,
            phys_addr: tier.base + offset,
            size,
            access_count: AtomicU64::new(1),
            last_access: AtomicU64::new(0),
            temperature: DataTemperature::Hot,
            state: BlockState::Active,
        });

        self.blocks.lock().insert(virt_addr, metadata.clone());
        self.stats.total_blocks.fetch_add(1, Ordering::Relaxed);
        self.stats.dram_blocks.fetch_add(1, Ordering::Relaxed);
        self.stats.hot_blocks.fetch_add(1, Ordering::Relaxed);

        Ok(metadata)
    }

    /// 访问块
    pub fn access_block(&self, virt_addr: u64) -> Result<Arc<BlockMetadata>, Error> {
        let blocks = self.blocks.lock();
        let block = blocks.get(&virt_addr).ok_or_else(|| Error::NotFound("not found".to_string()))?.clone();
        drop(blocks);

        // 更新访问计数和时间
        block.access_count.fetch_add(1, Ordering::Relaxed);
        block.last_access.store(
            self.current_time(),
            Ordering::Relaxed
        );

        // 重新评估数据温度
        self.update_temperature(&block);

        Ok(block)
    }

    /// 更新数据温度
    fn update_temperature(&self, block: &BlockMetadata) {
        let count = block.access_count.load(Ordering::Relaxed);

        let new_temp = if count > self.config.hot_threshold {
            DataTemperature::Hot
        } else if count > self.config.warm_threshold {
            DataTemperature::Warm
        } else {
            DataTemperature::Cold
        };

        // 如果温度改变，可能需要迁移
        if new_temp != block.temperature {
            // 标记需要迁移
            let _ = self.schedule_migration(Arc::new(BlockMetadata {
                virt_addr: block.virt_addr,
                tier: block.tier,
                phys_addr: block.phys_addr,
                size: block.size,
                access_count: AtomicU64::new(block.access_count.load(Ordering::SeqCst)),
                last_access: AtomicU64::new(block.last_access.load(Ordering::SeqCst)),
                temperature: block.temperature,
                state: block.state,
            }));
        }
    }

    /// 调度迁移
    fn schedule_migration(&self, block: Arc<BlockMetadata>) -> Result<(), Error> {
        let dst_tier = match block.temperature {
            DataTemperature::Hot => MemoryTier::Dram,
            DataTemperature::Warm => MemoryTier::Scm,
            DataTemperature::Cold => MemoryTier::Ssd,
        };

        if dst_tier as u8 == block.tier as u8 {
            return Ok(()); // 已经在正确的层级
        }

        let op = MigrationOperation {
            src_tier: block.tier,
            dst_tier,
            src_addr: block.phys_addr,
            dst_addr: 0, // 稍后分配
            size: block.size,
            op_id: self.next_migration_id.fetch_add(1, Ordering::SeqCst),
            completed: false,
        };

        self.migrations.lock().push(op);
        self.stats.migration_count.fetch_add(1, Ordering::Relaxed);

        if (dst_tier as u8) < (block.tier as u8) {
            self.stats.promotion_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats.demotion_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// 执行迁移
    pub fn execute_migration(&self) -> Result<(), Error> {
        let mut migrations = self.migrations.lock();

        while let Some(mut migration) = migrations.pop() {
            // 分配目标地址
            let dst_tier_idx = migration.dst_tier as usize;
            let dst_tier = &self.tiers[dst_tier_idx];
            let offset = dst_tier.used_size.fetch_add(migration.size, Ordering::Relaxed);

            if offset + migration.size > dst_tier.size {
                return Err(Error::OutOfMemory);
            }

            migration.dst_addr = dst_tier.base + offset;

            // 执行数据复制
            unsafe {
                self.copy_data(
                    migration.src_addr,
                    migration.dst_addr,
                    migration.size,
                    migration.dst_tier,
                );
            }

            // 更新块元数据
            let mut blocks = self.blocks.lock();
            for block in blocks.values_mut() {
                if Arc::make_mut(block).phys_addr == migration.src_addr {
                    Arc::make_mut(block).tier = migration.dst_tier;
                    Arc::make_mut(block).phys_addr = migration.dst_addr;
                    Arc::make_mut(block).state = BlockState::Active;

                    // 更新统计
                    match migration.dst_tier {
                        MemoryTier::Dram => {
                            self.stats.dram_blocks.fetch_add(1, Ordering::Relaxed);
                        }
                        MemoryTier::Scm => {
                            self.stats.scm_blocks.fetch_add(1, Ordering::Relaxed);
                        }
                        MemoryTier::Ssd => {
                            self.stats.ssd_blocks.fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                    break;
                }
            }

            let _migration = &mut migration; // Mark as used
        }

        Ok(())
    }

    /// 拷贝数据
    unsafe fn copy_data(&self, src: u64, dst: u64, size: u64, dst_tier: MemoryTier) {
        let src_ptr = src as *const u8;
        let dst_ptr = dst as *mut u8;

        for i in 0..size {
            unsafe {
                dst_ptr.add(i as usize).write_volatile(src_ptr.add(i as usize).read_volatile());
            }
        }

        // 如果目标是持久化层，需要刷新缓存
        if dst_tier != MemoryTier::Dram {
            // GH-#1310: 使用 clflush 指令
            // See: https://github.com/npos/kernel/issues/1310
            core::sync::atomic::fence(Ordering::Release);
        }
    }

    /// 释放块
    pub fn free_block(&self, virt_addr: u64) -> Result<(), Error> {
        let mut blocks = self.blocks.lock();
        let block = blocks.remove(&virt_addr).ok_or_else(|| Error::NotFound("not found".to_string()))?;

        // 更新统计
        match block.tier {
            MemoryTier::Dram => {
                self.stats.dram_blocks.fetch_sub(1, Ordering::Relaxed);
            }
            MemoryTier::Scm => {
                self.stats.scm_blocks.fetch_sub(1, Ordering::Relaxed);
            }
            MemoryTier::Ssd => {
                self.stats.ssd_blocks.fetch_sub(1, Ordering::Relaxed);
            }
            _ => {}
        }

        self.stats.total_blocks.fetch_sub(1, Ordering::Relaxed);

        Ok(())
    }

    /// 获取当前时间（简化版本）
    fn current_time(&self) -> u64 {
        // GH-#1311: 使用实际的时间戳
        // See: https://github.com/npos/kernel/issues/1311
        0
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> UnifiedTierStats {
        UnifiedTierStats {
            total_blocks: self.stats.total_blocks.load(Ordering::Relaxed),
            dram_blocks: self.stats.dram_blocks.load(Ordering::Relaxed),
            scm_blocks: self.stats.scm_blocks.load(Ordering::Relaxed),
            ssd_blocks: self.stats.ssd_blocks.load(Ordering::Relaxed),
            hot_blocks: self.stats.hot_blocks.load(Ordering::Relaxed),
            warm_blocks: self.stats.warm_blocks.load(Ordering::Relaxed),
            cold_blocks: self.stats.cold_blocks.load(Ordering::Relaxed),
            migration_count: self.stats.migration_count.load(Ordering::Relaxed),
            promotion_count: self.stats.promotion_count.load(Ordering::Relaxed),
            demotion_count: self.stats.demotion_count.load(Ordering::Relaxed),
        }
    }
}

/// 统一层级统计信息
#[derive(Debug, Clone)]
pub struct UnifiedTierStats {
    pub total_blocks: u64,
    pub dram_blocks: u64,
    pub scm_blocks: u64,
    pub ssd_blocks: u64,
    pub hot_blocks: u64,
    pub warm_blocks: u64,
    pub cold_blocks: u64,
    pub migration_count: u64,
    pub promotion_count: u64,
    pub demotion_count: u64,
}

/// 全局统一地址空间
static UNIFIED_ADDRSPACE: AdvancedMutex<Option<UnifiedAddressSpace>> = AdvancedMutex::new(None);

/// 初始化统一地址空间
pub fn init(config: Option<UnifiedConfig>) -> Result<(), Error> {
    let cfg = config.unwrap_or_default();
    let mut uas = UnifiedAddressSpace::new(cfg);
    uas.init();
    *UNIFIED_ADDRSPACE.lock() = Some(uas);
    Ok(())
}

/// 关闭统一地址空间
pub fn shutdown() -> Result<(), Error> {
    *UNIFIED_ADDRSPACE.lock() = None;
    Ok(())
}

/// 分配块（便捷函数）
pub fn allocate_block(virt_addr: u64, size: u64) -> Result<Arc<BlockMetadata>, Error> {
    let uas_guard = UNIFIED_ADDRSPACE.lock();
    let uas = uas_guard.as_ref().ok_or_else(|| Error::InvalidState("unified address space not initialized".into()))?;
    uas.allocate_block(virt_addr, size)
}

/// 访问块（便捷函数）
pub fn access_block(virt_addr: u64) -> Result<Arc<BlockMetadata>, Error> {
    let uas_guard = UNIFIED_ADDRSPACE.lock();
    let uas = uas_guard.as_ref().ok_or_else(|| Error::InvalidState("unified address space not initialized".into()))?;
    uas.access_block(virt_addr)
}

/// 释放块（便捷函数）
pub fn free_block(virt_addr: u64) -> Result<(), Error> {
    let uas_guard = UNIFIED_ADDRSPACE.lock();
    let uas = uas_guard.as_ref().ok_or_else(|| Error::InvalidState("unified address space not initialized".into()))?;
    uas.free_block(virt_addr)
}

/// 执行迁移（便捷函数）
pub fn execute_migration() -> Result<(), Error> {
    let uas_guard = UNIFIED_ADDRSPACE.lock();
    let uas = uas_guard.as_ref().ok_or_else(|| Error::InvalidState("unified address space not initialized".into()))?;
    uas.execute_migration()
}

/// 获取统计信息（便捷函数）
pub fn get_stats() -> Result<UnifiedTierStats, Error> {
    let uas_guard = UNIFIED_ADDRSPACE.lock();
    let uas = uas_guard.as_ref().ok_or_else(|| Error::InvalidState("unified address space not initialized".into()))?;
    Ok(uas.get_stats())
}
