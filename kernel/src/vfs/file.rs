//! VFS file handle
extern crate alloc;
use alloc::sync::Arc;

use super::{fs::InodeOps, error::VfsResult, types::FileAttr};

/// Open file handle
pub struct VfsFile {
    pub inode: Arc<dyn InodeOps>,
    pub offset: u64,
    flags: u32,
}

impl VfsFile {
    pub fn new(inode: Arc<dyn InodeOps>, flags: u32) -> Self {
        Self {
            inode,
            offset: 0,
            flags,
        }
    }
    
    /// Read from file
    pub fn read(&mut self, addr: usize, len: usize) -> Result<usize, ()> {
        // Create a buffer from the address and length
        let buf = unsafe {
            core::slice::from_raw_parts_mut(addr as *mut u8, len)
        };
        
        match self.inode.read(self.offset, buf) {
            Ok(n) => {
                self.offset += n as u64;
                Ok(n)
            }
            Err(_) => Err(()),
        }
    }
    
    /// Write to file
    pub fn write(&mut self, addr: usize, len: usize) -> Result<usize, ()> {
        // Create a buffer from the address and length
        let buf = unsafe {
            core::slice::from_raw_parts(addr as *const u8, len)
        };
        
        match self.inode.write(self.offset, &buf) {
            Ok(n) => {
                self.offset += n as u64;
                Ok(n)
            }
            Err(_) => Err(()),
        }
    }
    
    /// Seek to position
    pub fn seek(&mut self, offset: usize) -> isize {
        // Simple implementation that just sets the offset directly
        self.offset = offset as u64;
        offset as isize
    }

    /// Truncate file
    pub fn truncate(&self, size: u64) -> VfsResult<()> {
        let attr = self.inode.getattr()?;
        let mut new_attr = attr;
        new_attr.size = size;
        self.inode.setattr(&new_attr)
    }
    
    /// Get file attributes
    pub fn stat(&self) -> VfsResult<FileAttr> {
        self.inode.getattr()
    }

    /// Set file attributes
    pub fn set_attr(&self, attr: &FileAttr) -> VfsResult<()> {
        self.inode.setattr(attr)
    }
}
