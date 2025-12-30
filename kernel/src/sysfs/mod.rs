//! /sys filesystem implementation

extern crate alloc;

use alloc::sync::Arc;
use crate::fs::{Fs, VfsResult, FileAttr, FileType};

/// Sysfs filesystem implementation
pub struct SysFs {
    _private: (),
}

impl SysFs {
    /// Create a new sysfs filesystem instance
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Initialize sysfs
    pub fn init(&mut self) -> VfsResult<()> {
        // Initialize sysfs filesystem
        Ok(())
    }
}

impl Fs for SysFs {
    /// Get filesystem name
    fn name(&self) -> &str {
        "sysfs"
    }

    /// Get filesystem root
    fn root(&self) -> Arc<dyn crate::vfs::InodeOps> {
        // Return sysfs root inode
        use crate::subsystems::fs::vfs::dir::DirInode;
        Arc::new(DirInode::new(1, FileType::Directory, 0o555))
    }

    /// Get filesystem statistics
    fn statfs(&self) -> VfsResult<crate::vfs::FsStats> {
        Ok(crate::vfs::FsStats {
            bsize: 4096,
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: 0,
            ffree: 0,
            namelen: 255,
        })
    }

    /// Sync filesystem
    fn sync(&self) -> VfsResult<()> {
        Ok(())
    }
}

/// Create sysfs filesystem
pub fn create_sysfs() -> Arc<dyn Fs> {
    Arc::new(SysFs::new())
}

// Additional Fs trait methods needed for compatibility
impl crate::vfs::Fs for SysFs {
    fn mount(&self) -> Result<(), crate::vfs::VfsError> {
        Ok(())
    }
    
    fn unmount(&self) -> Result<(), crate::vfs::VfsError> {
        Ok(())
    }
}
