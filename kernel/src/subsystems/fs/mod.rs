//! # 文件系统子系统
//!
//! 提供虚拟文件系统（VFS）抽象和多种文件系统实现。
//!
//! ## 概述
//!
//! 文件系统子系统提供统一的文件系统接口，支持：
//! - **多种文件系统**: ext2、ext4、ramfs、tmpfs 等
//! - **VFS 抽象**: 统一的文件操作接口
//! - **日志机制**: 保证文件系统一致性
//! - **文件锁**: 支持文件锁和并发访问
//! - **持久化**: 数据恢复和快照
//!
//! ## 主要组件
//!
//! - [`VfsManager`]: VFS 管理器，管理文件系统类型和挂载点
//! - [`api`]: 文件系统 API 接口
//! - [`ext2`]: ext2 文件系统实现
//! - [`ext4`]: ext4 文件系统实现（带日志）
//! - [`ext4_persistence`]: ext4 持久化和恢复
//! - [`file`]: 文件抽象和操作
//! - [`file_locking`]: 文件锁机制
//! - [`file_permissions`]: 文件权限管理
//! - [`fs_cache`]: 文件系统缓存
//! - [`journaling_fs`]: 日志文件系统框架
//! - [`recovery`]: 文件系统恢复
//!
//! ## 架构
//!
//! ```
//! VFS 层
//!     ├── 挂载管理 (VfsManager)
//!     ├── 超级块管理
//!     ├── inode 缓存
//!     ├── dentry 缓存
//!     └── 文件操作接口
//! 具体文件系统
//!     ├── ext2
//!     ├── ext4 (带日志)
//!     ├── ramfs (内存文件系统)
//!     └── tmpfs (临时文件系统)
//! ```
//!
//! ## 使用示例
//!
//! ### 挂载文件系统
//!
//! ```no_run
//! use kernel::subsystems::fs::{vfs, FileSystemType, MountFlags};
//!
//! // 获取 VFS 管理器
//! let vfs_mgr = vfs();
//!
//! // 挂载 ext4 文件系统
//! let fs_type = vfs_mgr.get_filesystem_type("ext4")?;
//! vfs_mgr.mount("/dev/sda1", "/mnt", fs_type, MountFlags::empty())?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 文件操作
//!
//! ```no_run
//! use kernel::subsystems::fs::{open, read, write, close, OpenFlags};
//!
//! // 打开文件
//! let fd = open("/etc/hostname", OpenFlags::O_RDONLY)?;
//!
//! // 读取文件
//! let mut buffer = [0u8; 1024];
//! let n = read(fd, &mut buffer)?;
//!
//! // 关闭文件
//! close(fd)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ## 设计决策
//!
//! ### VFS 抽象
//!
//! 使用 VFS 提供统一接口：
//! - 支持多种文件系统
//! - 透明的文件操作
//! - 易于扩展新的文件系统
//!
//! ### 日志文件系统
//!
//! ext4 使用日志保证一致性：
//! - 先写日志，后写数据
//! - 崩溃后可恢复
//! - 保证元数据一致性
//!
//! ## 性能特征
//!
//! - **文件查找**: O(log n) 使用 B+ 树
//! - **目录缓存**: 命中率 > 90%
//! - **延迟写入**: 批量写优化
//!
//! ## 线程安全
//!
//! - 使用锁保护文件系统元数据
//! - 文件锁支持并发访问
//! - Per-inode 锁减少竞争
//!
//! ## 相关模块
//!
//! - [`crate::vfs`]: 虚拟文件系统核心
//! - [`crate::subsystems::syscalls::fs`]: 文件系统系统调用
//! - [`crate::subsystems::mm`]: 页缓存和内存映射

extern crate alloc;
use alloc::{collections::BTreeMap, string::String, string::ToString, sync::Arc};

use spin::Once;

use crate::subsystems::sync::Mutex;
use crate::subsystems::fs::vfs::Mount;

// VFS modules (moved from kernel/src/)
pub mod vfs;
pub mod vfs_interface;

pub mod api;
pub mod epoll;
pub mod ext2;
pub mod ext4;
pub mod ext4_persistence;
pub mod file;
pub mod file_locking;
pub mod file_permissions;
pub mod fs_cache;
pub mod fs_impl;
pub mod fs_types;
pub mod journaling_fs;
pub mod journaling_wrapper;
pub mod recovery;
pub mod xattr;

#[cfg(feature = "kernel_tests")]
pub mod tests;

// 为避免命名冲突，只导出必要的项而不是全部导出
pub use api::*;
// 注意：我们不导出ext4::*和recovery::*的所有内容，因为它们可能包含同名的init函数
// 而是明确指定需要导出的项

// 从ext4模块导出特定项（避免与recovery模块的init函数冲突）
pub use ext4::{EXT4_MAGIC, Ext4Errors, Ext4State, Ext4SuperBlock};
// 从recovery模块导出特定项
pub use recovery::{DEFAULT_CHECKPOINT_INTERVAL, MAX_SNAPSHOTS, SNAPSHOT_MAGIC};

// Export VfsManager for use in vfs module
// pub use VfsManager;

/// VFS manager structure
///
/// Manages filesystem types, mount points, and provides unified VFS operations
pub struct VfsManager {
    /// Registered filesystem types
    fs_types: Mutex<BTreeMap<String, Arc<dyn crate::vfs::FileSystemType>>>,
    /// Mount points
    mounts: Mutex<BTreeMap<String, Arc<crate::vfs::Mount>>>,
    /// Root filesystem mount point (if mounted)
    root_mounted: Mutex<Option<Arc<crate::vfs::Mount>>>,
}

impl VfsManager {
    /// Create a new VFS manager
    pub fn new() -> Self {
        Self {
            fs_types: Mutex::new(BTreeMap::new()),
            mounts: Mutex::new(BTreeMap::new()),
            root_mounted: Mutex::new(None),
        }
    }

    /// Register a new filesystem type
    pub fn register_fs(
        &self,
        fs_type: Arc<dyn crate::vfs::FileSystemType>,
    ) -> Result<(), FsError> {
        let mut fs_types = self.fs_types.lock();
        let name = fs_type.name().to_string();

        if fs_types.contains_key(&name) {
            return Err(FsError::Exists);
        }

        fs_types.insert(name, fs_type);
        Ok(())
    }

    /// Mount a filesystem
    ///
    /// # Arguments
    /// * `fs_type_name` - Name of the filesystem type (e.g., "ramfs", "ext4")
    /// * `mount_point` - Path where to mount the filesystem (e.g., "/")
    /// * `device` - Optional device name (for block devices)
    /// * `flags` - Mount flags
    pub fn mount(
        &self,
        fs_type_name: &str,
        mount_point: &str,
        device: Option<&str>,
        flags: u32,
    ) -> Result<(), FsError> {
        // Get filesystem type
        let fs_types = self.fs_types.lock();
        let fs_type = fs_types
            .get(fs_type_name)
            .ok_or(FsError::NotFound)?;

        // Mount the filesystem
        let superblock = fs_type.mount(device, flags)?;

        // Create mount point
        let mount =
            Arc::new(Mount::new(mount_point.to_string(), superblock, flags));

        // Register mount point
        let mut mounts = self.mounts.lock();

        // Check if mount point already exists
        if mounts.contains_key(mount_point) {
            return Err(FsError::Busy);
        }

        // Special handling for root mount
        if mount_point == "/" {
            let mut root_mounted = self.root_mounted.lock();
            *root_mounted = Some(mount.clone());
        }

        mounts.insert(mount_point.to_string(), mount);
        Ok(())
    }

    /// Unmount a filesystem
    pub fn unmount(&self, mount_point: &str) -> Result<(), FsError> {
        let mut mounts = self.mounts.lock();

        if let Some(mount) = mounts.remove(mount_point) {
            // Unmount the superblock
            mount.superblock.unmount()?;

            // Clear root mount if this is root
            if mount_point == "/" {
                let mut root_mounted = self.root_mounted.lock();
                *root_mounted = None;
            }

            Ok(())
        } else {
            Err(FsError::NotFound)
        }
    }

    /// Verify root filesystem is mounted and accessible
    pub fn verify_root(&self) -> Result<(), FsError> {
        let root_mounted = self.root_mounted.lock();

        if root_mounted.is_none() {
            return Err(FsError::NotMounted);
        }

        // Try to access root inode
        let mount = root_mounted.as_ref().unwrap();
        let root_inode = mount.superblock.root();
        let _attr = root_inode.getattr()?;

        Ok(())
    }

    /// Get file attributes (stat)
    pub fn stat(
        &self,
        path: &str,
    ) -> Result<crate::vfs::types::FileAttr, FsError> {
        // For now, only support root path
        if path == "/" {
            let root_mounted = self.root_mounted.lock();
            if let Some(mount) = root_mounted.as_ref() {
                let root_inode = mount.superblock.root();
                return root_inode.getattr().map_err(|e| FsError::from(e));
            }
        }

        Err(FsError::NotFound)
    }

    /// Create a new directory
    pub fn mkdir(
        &self,
        _path: &str,
        _mode: crate::vfs::types::FileMode,
    ) -> Result<(), FsError> {
        // TODO: Implement directory creation
        // For now, return error as this requires path resolution
        Err(FsError::NotSupported)
    }

    /// Create a new file
    pub fn create(
        &self,
        _path: &str,
        _mode: crate::vfs::types::FileMode,
    ) -> Result<(), FsError> {
        // TODO: Implement file creation
        // For now, return error as this requires path resolution
        Err(FsError::NotSupported)
    }

    /// Write to a file
    pub fn write(
        &self,
        _path: &str,
        _data: &[u8],
        _offset: u64,
    ) -> Result<usize, FsError> {
        // TODO: Implement file writing
        // For now, return error as this requires path resolution
        Err(FsError::NotSupported)
    }

    /// Delete a file or directory
    pub fn unlink(&self, _path: &str) -> Result<(), FsError> {
        // TODO: Implement file/directory deletion
        // For now, return error as this requires path resolution
        Err(FsError::NotSupported)
    }

    /// Check if root filesystem is mounted
    pub fn is_root_mounted(&self) -> bool {
        let root_mounted = self.root_mounted.lock();
        root_mounted.is_some()
    }
}

/// Global VFS manager instance
static VFS_MANAGER: Once<Arc<VfsManager>> = Once::new();

/// Get the global VFS manager instance
pub fn vfs() -> &'static Arc<VfsManager> {
    VFS_MANAGER.call_once(|| Arc::new(VfsManager::new()))
}

/// Mount a filesystem (global convenience function)
pub fn mount(
    fs_type: &str,
    mount_point: &str,
    device: Option<&str>,
    flags: u32,
) -> Result<(), FsError> {
    vfs().mount(fs_type, mount_point, device, flags)
}

/// Unmount a filesystem (global convenience function)
pub fn unmount(mount_point: &str) -> Result<(), FsError> {
    vfs().unmount(mount_point)
}

/// Verify root filesystem is mounted and accessible (global convenience function)
pub fn verify_root() -> Result<(), FsError> {
    vfs().verify_root()
}

/// Initialize file system subsystem
///
/// This function initializes all file system components including:
/// - VFS manager
/// - File system types (ext2, ext4, etc.)
/// - File system cache
/// - File permissions
/// - File locking
/// - Extended attributes
pub fn init() -> nos_api::Result<()> {
    // Initialize VFS manager (already initialized on first access)
    let _ = vfs();

    // Initialize file system cache
    fs_cache::init();

    // Initialize file permissions
    file_permissions::init();

    // Initialize file locking
    file_locking::init();

    // Initialize extended attributes
    xattr::init();

    // Initialize file system implementations
    fs_impl::init();

    crate::println!("[fs] File system subsystem initialized");
    Ok(())
}

/// Shutdown file system subsystem
///
/// This function cleans up file system resources:
/// - Sync all file systems
/// - Unmount all mount points (except root)
/// - Clean up file system cache
pub fn shutdown() -> nos_api::Result<()> {
    // TODO: Implement graceful shutdown
    // - Sync all file systems
    // - Unmount non-root file systems
    // - Clean up VFS manager

    crate::println!("[fs] File system subsystem shutdown");
    Ok(())
}
// fs_types is implemented as a file, not a module
