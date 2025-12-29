//! VFS 接口层
//!
//! 此模块提供 VFS 核心接口，用于打破 VFS 和 FS 之间的循环依赖。
//! VFS 和 FS 模块都依赖此接口层，而不是相互依赖。

extern crate alloc;

use alloc::{string::String, sync::Arc, vec::Vec};

// ============================================================================
// VFS Core Types
// ============================================================================

/// File mode/permission bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMode(pub u32);

impl FileMode {
    pub const fn empty() -> Self {
        FileMode(0)
    }
    pub const fn from_bits(bits: u32) -> Self {
        FileMode(bits)
    }
    pub const fn bits(&self) -> u32 {
        self.0
    }
}

/// File type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    RegularFile,
    Directory,
    CharacterDevice,
    BlockDevice,
    NamedPipe,
    SymbolicLink,
    Socket,
    Unknown,
}

/// VFS Error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsError {
    NoEntry,
    NotADirectory,
    IsADirectory,
    PermissionDenied,
    IoError,
    NotSupported,
    InvalidInput,
    NotFound,
}

/// File attributes
#[derive(Debug, Clone)]
pub struct FileAttr {
    pub file_type: FileType,
    pub mode: FileMode,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub uid: u32,
    pub gid: u32,
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub ino: u64,
    pub name: String,
    pub file_type: FileType,
}

use crate::subsystems::syscalls::fs::service::FilesystemStats;

/// 文件系统类型 trait - 所有文件系统实现都需要实现
pub trait FileSystemType: Send + Sync {
    /// 获取文件系统名称
    fn name(&self) -> &str;

    /// 挂载文件系统
    fn mount(&self, device: Option<&str>, flags: u32) -> Result<Arc<dyn SuperBlock>, VfsError>;
}

/// 超级块 trait - 表示已挂载的文件系统实例
pub trait SuperBlock: Send + Sync {
    /// 获取根 inode
    fn root(&self) -> Arc<dyn Inode>;

    /// 卸载文件系统
    fn unmount(&self) -> Result<(), VfsError>;

    /// 获取文件系统统计信息
    fn statfs(&self) -> Result<FilesystemStats, VfsError>;
}

/// Inode trait - 表示文件系统中的文件/目录
pub trait Inode: Send + Sync {
    /// 获取文件属性
    fn getattr(&self) -> Result<FileAttr, VfsError>;

    /// 读取数据
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError>;

    /// 写入数据
    fn write(&self, offset: u64, buf: &[u8]) -> Result<usize, VfsError>;

    /// 获取目录项
    fn readdir(&self) -> Result<Vec<DirEntry>, VfsError>;

    /// 查找子节点
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError>;

    /// 创建子节点
    fn create(
        &self,
        name: &str,
        mode: FileMode,
        file_type: FileType,
    ) -> Result<Arc<dyn Inode>, VfsError>;

    /// 删除节点
    fn unlink(&self, name: &str) -> Result<(), VfsError>;

    /// 创建目录
    fn mkdir(&self, name: &str, mode: FileMode) -> Result<Arc<dyn Inode>, VfsError>;

    /// 删除目录
    fn rmdir(&self, name: &str) -> Result<(), VfsError>;

    /// 获取文件类型
    fn file_type(&self) -> FileType;

    /// 获取文件名
    fn name(&self) -> String;

    /// 获取父目录
    fn parent(&self) -> Option<Arc<dyn Inode>>;

    /// 软链接目标
    fn symlink_target(&self) -> Option<String>;
}

/// 挂载点 trait
pub trait Mount: Send + Sync {
    /// 获取挂载点路径
    fn mount_point(&self) -> &str;

    /// 获取文件系统类型
    fn filesystem_type(&self) -> &dyn FileSystemType;

    /// 获取超级块
    fn superblock(&self) -> Arc<dyn SuperBlock>;

    /// 检查是否为根挂载
    fn is_root(&self) -> bool;
}
