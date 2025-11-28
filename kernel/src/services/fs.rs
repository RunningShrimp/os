//! File System Service for hybrid architecture
//! Separates file system functionality from kernel core

use crate::fs::{
    BSIZE, IPB, DPB, NDIRECT, NINDIRECT, MAXFILE, 
    MAXPATH, DIRSIZ, ROOTINO, FS_MAGIC, SuperBlock,
    InodeType, DiskInode, Dirent, BufCache, Fs,
};
use crate::drivers::{BlockDevice, RamDisk};
use crate::services::{service_register, ServiceInfo};

// ============================================================================
// File System Service State
// ============================================================================

/// File system service endpoint (IPC channel)
pub const FS_SERVICE_ENDPOINT: usize = 0x3000;

// ============================================================================
// Public API
// ============================================================================

/// Initialize file system service
pub fn init() {
    // Register file system service
    service_register(
        "filesystem",
        "File system service for file and directory management",
        FS_SERVICE_ENDPOINT
    );
    
    // The actual filesystem initialization will happen during boot
    // as before, but now it's under the service layer
    
    crate::println!("services/fs: initialized");
}

/// Read superblock from disk
pub fn fs_read_super(fs: &Fs) -> SuperBlock {
    fs.read_super()
}

/// Write superblock to disk
pub fn fs_write_super(fs: &Fs, sb: &SuperBlock) {
    fs.write_super(sb);
}

/// Initialize file system on device
pub fn fs_init(fs: &mut Fs) -> bool {
    fs.init()
}

/// Allocate an inode
pub fn fs_ialloc(fs: &Fs, itype: InodeType) -> Option<u32> {
    fs.ialloc(itype)
}

/// Get inode by number
pub fn fs_iget(fs: &Fs, inum: u32) -> Option<usize> {
    fs.iget(inum)
}

/// Release inode
pub fn fs_iput(fs: &Fs, idx: usize) {
    fs.iput(idx)
}

/// Look up directory entry
pub fn fs_dirlookup(fs: &Fs, dir_inum: u32, name: &str) -> Option<u32> {
    fs.dirlookup(dir_inum, name)
}

/// Create a new directory entry
pub fn fs_dirlink(fs: &Fs, dir_inum: u32, name: &str, inum: u32) -> bool {
    fs.dirlink(dir_inum, name, inum)
}

/// List directory contents
pub fn fs_list_dir(fs: &Fs, dir_inum: u32) -> alloc::vec::Vec<(alloc::string::String, u32)> {
    fs.list_dir(dir_inum)
}

/// Create file system on device
pub fn fs_mkfs(fs: &Fs) {
    fs.mkfs()
}