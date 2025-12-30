//! # VFS Core Traits
//!
//! 定义虚拟文件系统的核心 trait，包括文件系统类型和超级块。

extern crate alloc;

use crate::prelude::*;
use alloc::sync::Arc;

use crate::subsystems::fs::vfs::{
    inode::InodeOps,
    VfsResult,
};

/// 文件系统统计信息
#[derive(Debug, Clone)]
pub struct FsStats {
    /// 块大小
    pub bsize: u64,
    /// 总块数
    pub blocks: u64,
    /// 空闲块数
    pub bfree: u64,
    /// 可用块数（非特权用户）
    pub bavail: u64,
    /// 总文件节点数
    pub files: u64,
    /// 空闲文件节点数
    pub ffree: u64,
    /// 最大文件名长度
    pub namelen: u64,
}

/// 文件系统类型 trait
///
/// 所有文件系统类型必须实现此 trait。
pub trait FileSystemType: Send + Sync {
    /// 返回文件系统类型名称
    fn name(&self) -> &str;

    /// 挂载文件系统
    ///
    /// 创建并返回此文件系统类型的超级块实例。
    /// device 参数对于块设备文件系统是设备路径，对于内存文件系统通常为 None。
    fn mount(
        &self,
        device: Option<&str>,
        flags: u32,
    ) -> VfsResult<Arc<dyn SuperBlock>>;
}

/// 超级块 trait
///
/// 超级块代表一个已挂载的文件系统实例。
pub trait SuperBlock: Send + Sync {
    /// 返回根 inode
    fn root(&self) -> Arc<dyn InodeOps>;

    /// 返回文件系统类型名称
    fn fs_type(&self) -> &str;

    /// 同步文件系统
    ///
    /// 将所有缓存的数据写入磁盘。
    fn sync(&self) -> VfsResult<()>;

    /// 获取文件系统统计信息
    fn statfs(&self) -> VfsResult<FsStats>;

    /// 卸载文件系统
    fn unmount(&self) -> VfsResult<()>;
}
