extern crate alloc;
use alloc::sync::Arc;
use spin::Once;

pub mod api;
pub mod ext2;
pub mod ext4;
pub mod ext4_enhanced;
pub mod ext4_enhanced_impl;
pub mod ext4_enhanced_impl2;
pub mod ext4_persistence;
pub mod fs_cache;
pub mod fs_impl;
pub mod file;
pub mod file_permissions;
pub mod file_locking;
pub mod journaling_fs;
pub mod journaling_wrapper;
pub mod recovery;

#[cfg(feature = "kernel_tests")]
pub mod tests;

// 为避免命名冲突，只导出必要的项而不是全部导出
pub use api::*;
// 注意：我们不导出ext4::*和recovery::*的所有内容，因为它们可能包含同名的init函数
// 而是明确指定需要导出的项

// 从ext4模块导出特定项（避免与recovery模块的init函数冲突）
pub use ext4::{EXT4_MAGIC, Ext4State, Ext4Errors, Ext4SuperBlock};

// 从recovery模块导出特定项
pub use recovery::{DEFAULT_CHECKPOINT_INTERVAL, MAX_SNAPSHOTS, SNAPSHOT_MAGIC};

/// VFS manager structure
pub struct VfsManager {
    // Implementation will be added later
}

impl VfsManager {
    /// Register a new filesystem type
    pub fn register_fs(&self, fs_type: Arc<dyn crate::vfs::fs::FileSystemType>) -> Result<(), crate::vfs::error::VfsError> {
        // Simple implementation for now
        Ok(())
    }
    
    /// Create a new directory
    pub fn mkdir(&self, path: &str, mode: crate::vfs::types::FileMode) -> Result<(), crate::vfs::error::VfsError> {
        // Simple implementation for now
        Ok(())
    }
    
    /// Create a new file
    pub fn create(&self, path: &str, mode: crate::vfs::types::FileMode) -> Result<(), crate::vfs::error::VfsError> {
        // Simple implementation for now
        Ok(())
    }
    
    /// Write to a file
    pub fn write(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, crate::vfs::error::VfsError> {
        // Simple implementation for now
        Ok(data.len())
    }
    
    /// Delete a file or directory
    pub fn unlink(&self, path: &str) -> Result<(), crate::vfs::error::VfsError> {
        // Simple implementation for now
        Ok(())
    }
}

/// Global VFS manager instance
static VFS_MANAGER: Once<Arc<VfsManager>> = Once::new();

/// Get the global VFS manager instance
pub fn vfs() -> &'static Arc<VfsManager> {
    VFS_MANAGER.call_once(|| {
        Arc::new(VfsManager {})
    })
}
