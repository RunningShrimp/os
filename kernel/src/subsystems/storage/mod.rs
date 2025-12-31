//! # 分布式存储子系统
//!
//! 提供企业级分布式存储解决方案，支持高可用、高性能的数据存储。
//!
//! ## 概述
//!
//! 分布式存储子系统提供：
//! - **RAID 支持**: RAID 0/1/5/6/10 级别
//! - **逻辑卷管理**: LVM 功能，动态扩容
//! - **分布式锁**: 分布式环境下的锁管理
//! - **一致性哈希**: 数据分布和负载均衡
//! - **副本同步**: 多副本数据同步机制
//! - **仲裁协议**: Raft 一致性协议实现
//! - **故障恢复**: 自愈和数据重建
//!
//! ## 主要组件
//!
//! - [`raid`]: RAID 实现
//! - [`lvm`]: 逻辑卷管理
//! - [`distributed`]: 分布式存储核心
//! - [`replication`]: 副本管理
//! - [`quorum`]: 仲裁和一致性
//! - [`healing`]: 故障恢复
//!
//! ## 架构
//!
//! ```
//! 应用层
//!     ├── 文件系统
//!     └── 数据库
//! 存储层
//!     ├── LVM (逻辑卷)
//!     ├── RAID (条带/镜像)
//!     └── 分布式层
//!         ├── 一致性哈希
//!         ├── 副本管理
//!         └── 仲裁协议
//! 设备层
//!     ├── 本地块设备
//!     └── 网络设备
//! ```
//!
//! ## 使用示例
//!
//! ### 创建 RAID 阵列
//!
//! ```no_run
//! use kernel::storage::raid::{RaidArray, RaidLevel};
//!
//! // 创建 RAID 5 阵列
//! let devices = vec![device1, device2, device3];
//! let raid = RaidArray::new(RaidLevel::Raid5, devices)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 创建逻辑卷
//!
//! ```no_run
//! use kernel::storage::lvm::{LogicalVolume, VolumeGroup};
//!
//! // 创建卷组
//! let vg = VolumeGroup::new("vg0", physical_volumes)?;
//!
//! // 创建逻辑卷
//! let lv = vg.create_logical_volume("data", 1024 * 1024 * 1024)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ## 设计决策
//!
//! ### 数据一致性
//!
//! - 使用 Raft 协议保证强一致性
//! - 多副本机制确保数据可靠性
//! - 读写验证保证数据完整性
//!
//! ### 高可用性
//!
//! - 自动故障检测
//! - 快速故障转移
//! - 数据自动重建
//!
//! ## 性能特征
//!
//! - **RAID 0**: 线性性能提升
//! - **RAID 1**: 读取性能翻倍
//! - **RAID 5/6**: 均衡读写性能
//! - **分布式**: 水平扩展能力
//!
//! ## 线程安全
//!
//! - 所有组件都是线程安全的
//! - 使用适当的锁保护共享状态
//! - 支持并发访问

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic {AtomicBool, AtomicU64, Ordering, Ordering};

pub mod raid;
pub mod lvm;
pub mod distributed;
pub mod replication;
pub mod quorum;
pub mod healing;

pub use raid::*;
pub use lvm::*;
pub use distributed::*;
pub use replication::*;
pub use quorum::*;
pub use healing::*;

/// 分布式存储系统状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageState {
    /// 系统正在初始化
    Initializing,
    /// 系统正常运行
    Running,
    /// 系统正在降级运行
    Degraded,
    /// 系统正在重建
    Rebuilding,
    /// 系统发生故障
    Failed,
    /// 系统已关闭
    Shutdown,
}

/// 分布式存储系统配置
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// 是否启用 RAID
    pub enable_raid: bool,
    /// 是否启用 LVM
    pub enable_lvm: bool,
    /// 是否启用分布式功能
    pub enable_distributed: bool,
    /// 副本数量
    pub replication_factor: usize,
    /// 重建线程数
    pub rebuild_threads: usize,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 故障超时（毫秒）
    pub failure_timeout_ms: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            enable_raid: true,
            enable_lvm: true,
            enable_distributed: true,
            replication_factor: 3,
            rebuild_threads: 2,
            heartbeat_interval_ms: 1000,
            failure_timeout_ms: 5000,
        }
    }
}

/// 分布式存储系统
///
/// 管理整个分布式存储系统。
pub struct DistributedStorage {
    /// 系统配置
    config: StorageConfig,
    /// 系统状态
    state: Arc<core::sync::atomic::AtomicU8>,
    /// 初始化完成标志
    initialized: AtomicBool,
    /// 总读操作数
    total_reads: AtomicU64,
    /// 总写操作数
    total_writes: AtomicU64,
    /// 总字节数读取
    total_bytes_read: AtomicU64,
    /// 总字节数写入
    total_bytes_written: AtomicU64,
}

impl DistributedStorage {
    /// 创建新的分布式存储系统
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            state: Arc::new(core::sync::atomic::AtomicU8::new(
                StorageState::Initializing as u8
            )),
            initialized: AtomicBool::new(false),
            total_reads: AtomicU64::new(0),
            total_writes: AtomicU64::new(0),
            total_bytes_read: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(0),
        }
    }

    /// 初始化分布式存储系统
    pub fn init(&self) -> crate::error::Result<()> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        // Initialize RAID subsystem
        if self.config.enable_raid {
            raid::init()?;
        }

        // Initialize LVM subsystem
        if self.config.enable_lvm {
            lvm::init()?;
        }

        // Initialize distributed subsystem
        if self.config.enable_distributed {
            distributed::init(self.config.replication_factor)?;
        }

        // Update state to running
        self.state.store(StorageState::Running as u8, Ordering::Release);
        self.initialized.store(true, Ordering::Release);

        crate::println!("[storage] Distributed storage system initialized");
        Ok(())
    }

    /// 关闭分布式存储系统
    pub fn shutdown(&self) -> crate::error::Result<()> {
        // Update state to shutdown
        self.state.store(StorageState::Shutdown as u8, Ordering::Release);

        crate::println!("[storage] Distributed storage system shutdown");
        Ok(())
    }

    /// 获取系统状态
    pub fn state(&self) -> StorageState {
        match self.state.load(Ordering::Acquire) {
            0 => StorageState::Initializing,
            1 => StorageState::Running,
            2 => StorageState::Degraded,
            3 => StorageState::Rebuilding,
            4 => StorageState::Failed,
            5 => StorageState::Shutdown,
            _ => StorageState::Failed,
        }
    }

    /// 记录读操作
    pub fn record_read(&self, bytes: u64) {
        self.total_reads.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 记录写操作
    pub fn record_write(&self, bytes: u64) {
        self.total_writes.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> StorageStats {
        StorageStats {
            total_reads: self.total_reads.load(Ordering::Relaxed),
            total_writes: self.total_writes.load(Ordering::Relaxed),
            total_bytes_read: self.total_bytes_read.load(Ordering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(Ordering::Relaxed),
            state: self.state(),
        }
    }
}

/// 存储统计信息
#[derive(Debug, Clone, Copy)]
pub struct StorageStats {
    /// 总读操作数
    pub total_reads: u64,
    /// 总写操作数
    pub total_writes: u64,
    /// 总字节数读取
    pub total_bytes_read: u64,
    /// 总字节数写入
    pub total_bytes_written: u64,
    /// 系统状态
    pub state: StorageState,
}

/// 全局分布式存储系统实例
static DISTRIBUTED_STORAGE: spin::Once<Arc<DistributedStorage>> = spin::Once::new();

/// 获取全局分布式存储系统实例
pub fn distributed_storage() -> &'static Arc<DistributedStorage> {
    DISTRIBUTED_STORAGE.call_once(|| {
        Arc::new(DistributedStorage::new(StorageConfig::default()))
    })
}

/// 初始化分布式存储子系统
///
/// # Arguments
///
/// * `config` - 存储系统配置
///
/// # Returns
///
/// 成功返回 Ok(())，失败返回错误
pub fn init(config: Option<StorageConfig>) -> crate::error::Result<()> {
    let storage = if let Some(cfg) = config {
        Arc::new(DistributedStorage::new(cfg))
    } else {
        distributed_storage().clone()
    };

    storage.init()
}

/// 关闭分布式存储子系统
pub fn shutdown() -> crate::error::Result<()> {
    if let Some(storage) = DISTRIBUTED_STORAGE.get() {
        storage.shutdown()?;
    }
    Ok(())
}

/// 获取存储统计信息
pub fn get_stats() -> Option<StorageStats> {
    DISTRIBUTED_STORAGE.get().map(|s| s.get_stats())
}
