extern crate alloc;
use alloc::{sync::Arc, collections::BTreeMap, string::String};
use spin::Once;
use crate::subsystems::sync::Mutex;

pub mod api;
pub mod ext2;
pub mod ext4;
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

// Export VfsManager for use in vfs module
// pub use VfsManager;

/// VFS manager structure
/// 
/// Manages filesystem types, mount points, and provides unified VFS operations
pub struct VfsManager {
    /// Registered filesystem types
    fs_types: Mutex<BTreeMap<String, Arc<dyn crate::vfs::fs::FileSystemType>>>,
    /// Mount points
    mounts: Mutex<BTreeMap<String, Arc<crate::vfs::mount::Mount>>>,
    /// Root filesystem mount point (if mounted)
    root_mounted: Mutex<Option<Arc<crate::vfs::mount::Mount>>>,
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
    pub fn register_fs(&self, fs_type: Arc<dyn crate::vfs::fs::FileSystemType>) -> Result<(), crate::vfs::error::VfsError> {
        let mut fs_types = self.fs_types.lock();
        let name = fs_type.name().to_string();
        
        if fs_types.contains_key(&name) {
            return Err(crate::vfs::error::VfsError::Exists);
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
    pub fn mount(&self, fs_type_name: &str, mount_point: &str, device: Option<&str>, flags: u32) -> Result<(), crate::vfs::error::VfsError> {
        // Get filesystem type
        let fs_types = self.fs_types.lock();
        let fs_type = fs_types.get(fs_type_name)
            .ok_or(crate::vfs::error::VfsError::NotFound)?;
        
        // Mount the filesystem
        let superblock = fs_type.mount(device, flags)?;
        
        // Create mount point
        let mount = Arc::new(crate::vfs::mount::Mount::new(
            mount_point.to_string(),
            superblock,
            flags,
        ));
        
        // Register mount point
        let mut mounts = self.mounts.lock();
        
        // Check if mount point already exists
        if mounts.contains_key(mount_point) {
            return Err(crate::vfs::error::VfsError::Busy);
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
    pub fn unmount(&self, mount_point: &str) -> Result<(), crate::vfs::error::VfsError> {
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
            Err(crate::vfs::error::VfsError::NotFound)
        }
    }
    
    /// Verify root filesystem is mounted and accessible
    pub fn verify_root(&self) -> Result<(), crate::vfs::error::VfsError> {
        let root_mounted = self.root_mounted.lock();
        
        if root_mounted.is_none() {
            return Err(crate::vfs::error::VfsError::NotMounted);
        }
        
        // Try to access root inode
        let mount = root_mounted.as_ref().unwrap();
        let root_inode = mount.superblock.root();
        let _attr = root_inode.getattr()?;
        
        Ok(())
    }
    
    /// Get file attributes (stat)
    pub fn stat(&self, path: &str) -> Result<crate::vfs::types::FileAttr, crate::vfs::error::VfsError> {
        // For now, only support root path
        if path == "/" {
            let root_mounted = self.root_mounted.lock();
            if let Some(mount) = root_mounted.as_ref() {
                let root_inode = mount.superblock.root();
                return root_inode.getattr();
            }
        }
        
        Err(crate::vfs::error::VfsError::NotFound)
    }
    
    /// Create a new directory
    pub fn mkdir(&self, path: &str, mode: crate::vfs::types::FileMode) -> Result<(), crate::vfs::error::VfsError> {
        // TODO: Implement directory creation
        // For now, return error as this requires path resolution
        Err(crate::vfs::error::VfsError::NotSupported)
    }
    
    /// Create a new file
    pub fn create(&self, path: &str, mode: crate::vfs::types::FileMode) -> Result<(), crate::vfs::error::VfsError> {
        // TODO: Implement file creation
        // For now, return error as this requires path resolution
        Err(crate::vfs::error::VfsError::NotSupported)
    }
    
    /// Write to a file
    pub fn write(&self, path: &str, data: &[u8], offset: u64) -> Result<usize, crate::vfs::error::VfsError> {
        // TODO: Implement file writing
        // For now, return error as this requires path resolution
        Err(crate::vfs::error::VfsError::NotSupported)
    }
    
    /// Delete a file or directory
    pub fn unlink(&self, path: &str) -> Result<(), crate::vfs::error::VfsError> {
        // TODO: Implement file/directory deletion
        // For now, return error as this requires path resolution
        Err(crate::vfs::error::VfsError::NotSupported)
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
    VFS_MANAGER.call_once(|| {
        Arc::new(VfsManager::new())
    })
}

/// Mount a filesystem (global convenience function)
pub fn mount(fs_type: &str, mount_point: &str, device: Option<&str>, flags: u32) -> Result<(), crate::vfs::error::VfsError> {
    vfs().mount(fs_type, mount_point, device, flags)
}

/// Unmount a filesystem (global convenience function)
pub fn unmount(mount_point: &str) -> Result<(), crate::vfs::error::VfsError> {
    vfs().unmount(mount_point)
}

/// Verify root filesystem is mounted and accessible (global convenience function)
pub fn verify_root() -> Result<(), crate::vfs::error::VfsError> {
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
pub fn init() -> nos_api::Result<()> {
    // Initialize VFS manager (already initialized on first access)
    let _ = vfs();
    
    // Initialize file system cache
    fs_cache::init();
    
    // Initialize file permissions
    file_permissions::init();
    
    // Initialize file locking
    file_locking::init();
    
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
