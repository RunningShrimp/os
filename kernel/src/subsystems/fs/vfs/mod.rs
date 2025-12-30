//! # 虚拟文件系统（VFS）核心
//!
//! 提供统一的文件系统抽象层，支持多种文件系统实现。
//!
//! ## 概述
//!
//! VFS 是文件系统的抽象层，提供：
//! - **统一接口**: 对所有文件系统提供相同的操作接口
//! - **挂载管理**: 管理挂载点和文件系统类型
//! - **缓存机制**: dentry 和 inode 缓存提高性能
//! - **路径解析**: 统一的路径查找和解析
//! - **文件抽象**: 文件、目录、设备等的统一抽象
//!
//! ## 主要组件
//!
//! - [`fs`]: 文件系统抽象和超级块
//! - [`mount`]: 挂载点管理
//! - [`dentry`]: 目录项缓存
//! - [`file`]: 文件抽象和操作
//! - [`dir`]: 目录操作
//! - [`types`]: VFS 公共类型定义
//!
//! ## 支持的文件系统
//!
//! - [`ramfs`]: 内存文件系统
//! - [`tmpfs`]: 临时文件系统
//! - [`procfs`]: proc 文件系统（/proc）
//! - [`sysfs`]: sys 文件系统（/sys）
//! - [`ext4`]: ext4 文件系统
//!
//! ## 架构
//!
//! ```
//! 应用程序
//!     ├── 系统调用 (open, read, write, etc.)
//! VFS 层
//!     ├── 路径解析 (lookup)
//!     ├── dentry 缓存
//!     ├── inode 缓存
//!     ├── 文件操作接口
//!     └── 挂载管理
//! 具体文件系统
//!     ├── ext4
//!     ├── ramfs
//!     └── procfs
//! ```
//!
//! ## 使用示例
//!
//! ### 文件操作
//!
//! ```no_run
//! use kernel::vfs::{File, OpenFlags};
//!
//! // 打开文件
//! let file = File::open("/etc/hostname", OpenFlags::O_RDONLY)?;
//!
//! // 读取文件
//! let mut buffer = [0u8; 1024];
//! let n = file.read(&mut buffer)?;
//!
//! // 关闭文件
//! file.close()?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 路径查找
//!
//! ```no_run
//! use kernel::vfs::lookup;
//!
//! // 查找路径
//! let inode = lookup("/usr/bin/bash")?;
//! ```
//!
//! ## 设计决策
//!
//! ### 分离的文件系统和 VFS
//!
//! VFS 和具体文件系统实现分离：
//! - 文件系统独立开发和测试
//! - 易于添加新的文件系统
//! - 清晰的接口定义
//!
//! ### 缓存优先
//!
//! 使用多级缓存提高性能：
//! - dentry 缓存: 目录查找结果
//! - inode 缓存: inode 数据
//! - 页缓存: 文件内容
//!
//! ## 性能特征
//!
//! - **路径解析**: O(n) n=路径组件数（有缓存）
//! - **文件打开**: O(1) 缓存命中时
//! - **缓存命中率**: > 90% 典型工作负载
//!
//! ## 线程安全
//!
//! - 使用锁保护 VFS 结构
//! - RCU 用于缓存查找
//! - Per-inode 锁减少竞争
//!
//! ## 相关模块
//!
//! - [`crate::subsystems::fs`]: 文件系统实现
//! - [`crate::subsystems::syscalls::fs`]: 文件系统系统调用

//! Virtual File System (VFS) module
//!
//! Provides unified interface for filesystem operations and mount management.

extern crate alloc;

pub mod core;
pub mod dentry;
pub mod devices;
pub mod dir;
pub mod directory;
pub mod error;
pub mod ext4;
pub mod file;
pub mod fs;
pub mod inode;
pub mod kernel;
pub mod mount;
pub mod path;
pub mod procfs;
pub mod ramfs;
pub mod sysfs;
pub mod symlink;
pub mod tmpfs;
pub mod types;

// Re-export from vfs_interface to break circular dependency
// These are the core VFS types used throughout the kernel
pub use crate::subsystems::fs::vfs_interface::{FileMode, VfsError, FileSystemType, SuperBlock, Inode, FileAttr, FileType, DirEntry};

// Import Mount from mount.rs (the concrete Mount struct)
pub use mount::Mount;

// Define VfsResult as a type alias using vfs_interface::VfsError
pub type VfsResult<T> = ::core::result::Result<T, crate::vfs_interface::VfsError>;

// Re-export key types from submodules
// Note: Only export what actually exists and is needed, avoid conflicts with vfs_interface
pub use dentry::Dentry;
pub use file::VfsFile;
pub use fs::SysFsType;
pub use inode::{InodeOps, FileLock};
pub use path::Path;
pub use symlink::{SymlinkCache, ResolveOptions, SymlinkInfo, SymlinkStats};
pub use types::SeekWhence;

/// Mount a filesystem at the specified path
///
/// This is a convenience function for mounting filesystems.
/// It delegates to the VFS manager's mount function.
pub fn mount(fs_type: &str, device: Option<&str>, mount_point: &str, flags: u32) -> VfsResult<()> {
    // VfsManager::mount expects: fs_type_name, mount_point, device, flags
    crate::subsystems::fs::vfs().mount(fs_type, mount_point, device, flags)
        .map_err(|fs_err| match fs_err {
            crate::subsystems::fs::api::error::FsError::NotFound |
            crate::subsystems::fs::api::error::FsError::PathNotFound |
            crate::subsystems::fs::api::error::FsError::FileNotFound => VfsError::NotFound,
            crate::subsystems::fs::api::error::FsError::PermissionDenied => VfsError::PermissionDenied,
            crate::subsystems::fs::api::error::FsError::NotADirectory => VfsError::NotADirectory,
            crate::subsystems::fs::api::error::FsError::IsADirectory => VfsError::IsADirectory,
            crate::subsystems::fs::api::error::FsError::DirectoryNotEmpty |
            crate::subsystems::fs::api::error::FsError::NotEmpty => VfsError::NotEmpty,
            crate::subsystems::fs::api::error::FsError::FileExists |
            crate::subsystems::fs::api::error::FsError::Exists => VfsError::Exists,
            crate::subsystems::fs::api::error::FsError::FileSystemFull |
            crate::subsystems::fs::api::error::FsError::NoSpace => VfsError::NoSpace,
            crate::subsystems::fs::api::error::FsError::InvalidPath |
            crate::subsystems::fs::api::error::FsError::InvalidInput => VfsError::InvalidPath,
            crate::subsystems::fs::api::error::FsError::NotMounted => VfsError::NotMounted,
            crate::subsystems::fs::api::error::FsError::ResourceBusy |
            crate::subsystems::fs::api::error::FsError::Busy => VfsError::Busy,
            crate::subsystems::fs::api::error::FsError::ReadOnly => VfsError::ReadOnly,
            crate::subsystems::fs::api::error::FsError::IoError => VfsError::IoError,
            crate::subsystems::fs::api::error::FsError::OperationNotSupported |
            crate::subsystems::fs::api::error::FsError::NotSupported => VfsError::NotSupported,
            crate::subsystems::fs::api::error::FsError::InvalidOperation => VfsError::InvalidOperation,
            crate::subsystems::fs::api::error::FsError::Loop => VfsError::Loop,
            crate::subsystems::fs::api::error::FsError::TooManyLinks => VfsError::TooManyLinks,
            crate::subsystems::fs::api::error::FsError::QuotaExceeded => VfsError::NoSpace,
            crate::subsystems::fs::api::error::FsError::PathTooLong => VfsError::InvalidPath,
        })
}

// Re-export additional functions from submodules
pub use symlink::{resolve_symlink, resolve_symlink_with_options, readlink, symlink, is_symlink, get_symlink_info, get_stats, reset_stats};

/// Get the global VFS manager instance
///
/// This function provides access to the VFS manager from within the vfs module.
/// It delegates to subsystems::fs::vfs().
pub fn vfs() -> &'static alloc::sync::Arc<crate::subsystems::fs::VfsManager> {
    crate::subsystems::fs::vfs()
}
