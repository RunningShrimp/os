//! VFS 接口层
//!
//! 此模块提供 VFS 核心接口，用于打破 VFS 和 FS 之间的循环依赖。
//! VFS 和 FS 模块都依赖此接口层，而不是相互依赖。

extern crate alloc;

use alloc::sync::Arc;
use alloc::string::String;

// ============================================================================
// Re-export VFS Core Types to break circular dependency
// ============================================================================

// Re-export FileMode from vfs::types
pub use crate::vfs::types::FileMode;

// Re-export VfsError from vfs::error
pub use crate::vfs::error::VfsError;

// Re-export FileAttr from vfs::types
pub use crate::vfs::types::FileAttr;

// Re-export DirEntry from vfs::dir
pub use crate::vfs::dir::DirEntry;

// Re-export FileType from vfs::types to avoid conflicts
pub use crate::vfs::types::FileType;

// ============================================================================
// VFS Interface uses vfs::types::FileType directly
// ============================================================================

// ============================================================================
// VFS Interface Traits
// ============================================================================

/// 文件系统类型 trait - 所有文件系统实现都需要实现
///
/// 这是一个类型别名，指向 vfs::core::FileSystemType
pub use crate::vfs::core::FileSystemType;

/// 超级块 trait - 表示已挂载的文件系统实例
///
/// 这是一个类型别名，指向 vfs::core::SuperBlock
pub use crate::vfs::core::SuperBlock;

/// Inode trait - 表示文件系统中的文件/目录
///
/// 扩展 InodeOps trait，添加额外的辅助方法
pub trait Inode: crate::vfs::inode::InodeOps {
    /// 获取文件类型
    fn file_type(&self) -> FileType;

    /// 获取文件名
    fn name(&self) -> String;

    /// 获取父目录
    fn parent(&self) -> Option<Arc<dyn Inode>>;

    /// 软链接目标
    fn symlink_target(&self) -> Option<String>;

    /// 同步文件
    fn sync(&self) -> Result<(), VfsError> {
        Err(VfsError::NotSupported)
    }

    /// 重命名
    fn rename(&self, old_name: &str, new_name: &str) -> Result<(), VfsError> {
        let _ = (old_name, new_name);
        Err(VfsError::NotSupported)
    }

    /// 获取 inode 号
    fn ino(&self) -> u64 {
        0
    }

    /// 获取文件模式
    fn mode(&self) -> FileMode {
        FileMode(0)
    }
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

// ============================================================================
// Helper macro for implementing Inode trait
// ============================================================================

/// Macro to implement vfs_interface::Inode for types that implement InodeOps
#[macro_export]
macro_rules! impl_inode {
    ($type:ty) => {
        impl $crate::vfs_interface::Inode for $type {
            fn file_type(&self) -> $crate::vfs_interface::FileType {
                self.getattr()
                    .map(|attr| {
                        // Convert vfs::types::FileType to vfs_interface::FileType using From trait
                        $crate::vfs_interface::FileType::from(attr.mode.file_type())
                    })
                    .unwrap_or($crate::vfs_interface::FileType::Regular)
            }

            fn name(&self) -> String {
                // Try to get name from getattr, return empty string if not available
                self.getattr()
                    .ok()
                    .and_then(|_| {
                        // FileAttr doesn't have a name field, so return a default
                        None
                    })
                    .unwrap_or_else(|| String::new())
            }

            fn parent(&self) -> Option<alloc::sync::Arc<dyn $crate::vfs_interface::Inode>> {
                // Default implementation - most filesystems don't track parent
                None
            }

            fn symlink_target(&self) -> Option<String> {
                // Try to read symlink target
                self.readlink().ok()
            }

            fn ino(&self) -> u64 {
                self.getattr()
                    .map(|attr| attr.ino)
                    .unwrap_or(0)
            }

            fn mode(&self) -> $crate::vfs_interface::FileMode {
                self.getattr()
                    .map(|attr| attr.mode)
                    .unwrap_or($crate::vfs_interface::FileMode(0))
            }
        }
    };
}

