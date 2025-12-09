//! File system type and superblock definitions
extern crate alloc;
use alloc::sync::Arc;

use super::{error::*, types::*};

/// File system type trait
pub trait FileSystemType: Send + Sync {
    /// Get file system name
    fn name(&self) -> &str;
    
    /// Mount the file system
    fn mount(&self, device: Option<&str>, flags: u32) -> VfsResult<Arc<dyn SuperBlock>>;
}

/// Superblock trait representing a mounted file system
pub trait SuperBlock: Send + Sync {
    /// Get root inode
    fn root(&self) -> Arc<dyn InodeOps>;
    
    /// Get file system type name
    fn fs_type(&self) -> &str;
    
    /// Sync all data to storage
    fn sync(&self) -> VfsResult<()>;
    
    /// Get file system statistics
    fn statfs(&self) -> VfsResult<FsStats>;
    
    /// Unmount (cleanup)
    fn unmount(&self) -> VfsResult<()>;
}

/// File system statistics
#[derive(Debug, Clone, Default)]
pub struct FsStats {
    pub bsize: u64,     // Block size
    pub blocks: u64,    // Total blocks
    pub bfree: u64,     // Free blocks
    pub bavail: u64,    // Available blocks
    pub files: u64,     // Total inodes
    pub ffree: u64,     // Free inodes
    pub namelen: u64,   // Max name length
}

/// Operations that can be performed on an inode
pub trait InodeOps: Send + Sync {
    /// Get file attributes
    fn getattr(&self) -> VfsResult<FileAttr>;
    
    /// Set file attributes
    fn setattr(&self, attr: &FileAttr) -> VfsResult<()> {
        let _ = attr;
        Err(VfsError::NotSupported)
    }
    
    /// Lookup a name in a directory
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = name;
        Err(VfsError::NotDirectory)
    }
    
    /// Create a file in a directory
    fn create(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, mode);
        Err(VfsError::NotDirectory)
    }
    
    /// Create a directory
    fn mkdir(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, mode);
        Err(VfsError::NotDirectory)
    }
    
    /// Remove a file
    fn unlink(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotDirectory)
    }

    /// Create a hard link
    fn link(&self, name: &str, inode: Arc<dyn InodeOps>) -> VfsResult<()> {
        let _ = (name, inode);
        Err(VfsError::NotSupported)
    }
    
    /// Check if directory is empty
    fn is_empty(&self) -> VfsResult<bool> {
        Err(VfsError::NotSupported)
    }
    
    /// Remove a directory
    fn rmdir(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotDirectory)
    }
    
    /// Rename
    fn rename(&self, old_name: &str, new_dir: &dyn InodeOps, new_name: &str) -> VfsResult<()> {
        let _ = (old_name, new_dir, new_name);
        Err(VfsError::NotSupported)
    }
    
    
    /// Create a symbolic link
    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, target);
        Err(VfsError::NotSupported)
    }
    
    /// Read symbolic link target
    fn readlink(&self) -> VfsResult<alloc::string::String> {
        Err(VfsError::InvalidOperation)
    }
    
    /// Read directory entries
    fn readdir(&self, offset: usize) -> VfsResult<alloc::vec::Vec<super::dir::DirEntry>> {
        let _ = offset;
        Err(VfsError::NotDirectory)
    }
    
    /// Read data
    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let _ = (offset, buf);
        Err(VfsError::IsDirectory)
    }
    
    /// Write data
    fn write(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let _ = (offset, buf);
        Err(VfsError::IsDirectory)
    }
    
    /// Truncate file
    fn truncate(&self, size: u64) -> VfsResult<()> {
        let _ = size;
        Err(VfsError::NotSupported)
    }
    
    /// Sync file to storage
    fn sync(&self) -> VfsResult<()> {
        Ok(())
    }
}
