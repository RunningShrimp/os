//! EXT4 file system implementation
//!
//! Implements EXT4 file system support for VFS layer
//! This is a simplified implementation focusing on basic operations

extern crate alloc;

use alloc::{string::{String, ToString}, sync::Arc, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::sync::Mutex;
use crate::drivers::BlockDevice;

use super::{
    error::*,
    types::*,
    fs::{FileSystemType, SuperBlock, InodeOps, FsStats},
    dir::DirEntry,
};

// ============================================================================
// EXT4 Constants
// ============================================================================

/// EXT4 magic number
const EXT4_SUPER_MAGIC: u16 = 0xEF53;

/// EXT4 block size (4KB)
const EXT4_BLOCK_SIZE: usize = 4096;

/// EXT4 inode size
const EXT4_INODE_SIZE: usize = 256;

/// EXT4 directory entry file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Ext4FileType {
    Unknown = 0,
    Regular = 1,
    Directory = 2,
    CharDevice = 3,
    BlockDevice = 4,
    Fifo = 5,
    Socket = 6,
    Symlink = 7,
}

// ============================================================================
// EXT4 On-Disk Structures
// ============================================================================

/// EXT4 superblock structure (simplified)
#[repr(C, packed)]
struct Ext4SuperBlock {
    inodes_count: u32,           // Total inodes count
    blocks_count_lo: u32,        // Total blocks count
    r_blocks_count_lo: u32,      // Reserved blocks count
    free_blocks_count_lo: u32,   // Free blocks count
    free_inodes_count: u32,      // Free inodes count
    first_data_block: u32,       // First data block
    log_block_size: u32,          // Block size = 1024 << log_block_size
    log_cluster_size: u32,        // Cluster size
    blocks_per_group: u32,        // Blocks per group
    clusters_per_group: u32,      // Clusters per group
    inodes_per_group: u32,        // Inodes per group
    mtime: u32,                   // Mount time
    wtime: u32,                   // Write time
    mnt_count: u16,               // Mount count
    max_mnt_count: u16,           // Max mount count
    magic: u16,                   // Magic signature (0xEF53)
    state: u16,                   // File system state
    errors: u16,                  // Behavior when detecting errors
    minor_rev_level: u16,         // Minor revision level
    lastcheck: u32,               // Last check time
    checkinterval: u32,           // Check interval
    creator_os: u32,              // Creator OS
    rev_level: u32,               // Revision level
    def_resuid: u16,              // Default uid for reserved blocks
    def_resgid: u16,              // Default gid for reserved blocks
    first_ino: u32,               // First non-reserved inode
    inode_size: u16,              // Size of inode structure
    block_group_nr: u16,          // Block group number of this superblock
    feature_compat: u32,          // Compatible feature set
    feature_incompat: u32,        // Incompatible feature set
    feature_ro_compat: u32,       // Readonly-compatible feature set
    uuid: [u8; 16],               // 128-bit UUID for volume
    volume_name: [u8; 16],        // Volume name
    last_mounted: [u8; 64],       // Directory where last mounted
    algorithm_usage_bitmap: u32,  // For compression
    // ... more fields omitted for simplicity
}

/// EXT4 inode structure (simplified)
#[repr(C, packed)]
struct Ext4Inode {
    mode: u16,                    // File mode
    uid: u16,                      // Lower 16 bits of owner UID
    size_lo: u32,                 // Lower 32 bits of size in bytes
    atime: u32,                   // Access time
    ctime: u32,                   // Change time
    mtime: u32,                   // Modification time
    dtime: u32,                   // Deletion time
    gid: u16,                      // Lower 16 bits of group ID
    links_count: u16,             // Links count
    blocks_lo: u32,               // Lower 32 bits of block count
    flags: u32,                   // File flags
    // ... more fields omitted for simplicity
    block: [u32; 15],             // Pointers to blocks
    generation: u32,              // File version (for NFS)
    file_acl_lo: u32,             // Lower 32 bits of extended attributes
    size_hi: u32,                 // Upper 32 bits of size in bytes
    // ... more fields omitted for simplicity
}

/// EXT4 directory entry (simplified)
#[repr(C, packed)]
struct Ext4DirEntry {
    inode: u32,                    // Inode number
    rec_len: u16,                  // Directory entry length
    name_len: u8,                  // Name length
    file_type: u8,                 // File type
    name: [u8; 255],               // File name (variable length)
}

// ============================================================================
// EXT4 File System Type
// ============================================================================

/// EXT4 file system type
pub struct Ext4FsType;

impl FileSystemType for Ext4FsType {
    fn name(&self) -> &str {
        "ext4"
    }
    
    fn mount(&self, device: Option<&str>, flags: u32) -> VfsResult<Arc<dyn SuperBlock>> {
        // TODO: Open device and read superblock
        // For now, create a minimal implementation
        let _ = (device, flags);
        
        // Create a basic EXT4 superblock
        Ok(Arc::new(Ext4SuperBlockImpl::new()))
    }
}

// ============================================================================
// EXT4 Superblock Implementation
// ============================================================================

/// EXT4 superblock implementation
struct Ext4SuperBlockImpl {
    root: Arc<Ext4InodeImpl>,
    next_ino: AtomicU64,
    total_blocks: AtomicU64,
    free_blocks: AtomicU64,
    total_inodes: AtomicU64,
    free_inodes: AtomicU64,
}

impl Ext4SuperBlockImpl {
    fn new() -> Self {
        // Create root inode
        let root_ino = Ext4InodeImpl::new_dir(2); // Inode 2 is root in EXT4
        
        Self {
            root: Arc::new(root_ino),
            next_ino: AtomicU64::new(3),
            total_blocks: AtomicU64::new(0),
            free_blocks: AtomicU64::new(0),
            total_inodes: AtomicU64::new(0),
            free_inodes: AtomicU64::new(0),
        }
    }
    
    fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed)
    }
}

impl SuperBlock for Ext4SuperBlockImpl {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }
    
    fn fs_type(&self) -> &str {
        "ext4"
    }
    
    fn sync(&self) -> VfsResult<()> {
        // TODO: Sync all dirty blocks to disk
        Ok(())
    }
    
    fn statfs(&self) -> VfsResult<FsStats> {
        Ok(FsStats {
            bsize: EXT4_BLOCK_SIZE as u64,
            blocks: self.total_blocks.load(Ordering::Relaxed),
            bfree: self.free_blocks.load(Ordering::Relaxed),
            bavail: self.free_blocks.load(Ordering::Relaxed),
            files: self.total_inodes.load(Ordering::Relaxed),
            ffree: self.free_inodes.load(Ordering::Relaxed),
            namelen: 255, // EXT4 supports up to 255 character filenames
        })
    }
    
    fn unmount(&self) -> VfsResult<()> {
        // TODO: Sync and cleanup
        self.sync()
    }
}

// ============================================================================
// EXT4 Inode Implementation
// ============================================================================

/// EXT4 inode implementation
struct Ext4InodeImpl {
    attr: Mutex<FileAttr>,
    // For regular files
    data: Mutex<Vec<u8>>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn InodeOps>>>,
    // For symlinks
    target: Mutex<Option<String>>,
}

impl Ext4InodeImpl {
    fn new_file(ino: u64) -> Self {
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFREG | 0o644),
                nlink: 1,
                size: 0,
                ..Default::default()
            }),
            data: Mutex::new(Vec::new()),
            children: Mutex::new(BTreeMap::new()),
            target: Mutex::new(None),
        }
    }
    
    fn new_dir(ino: u64) -> Self {
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFDIR | 0o755),
                nlink: 2,
                size: 0,
                ..Default::default()
            }),
            data: Mutex::new(Vec::new()),
            children: Mutex::new(BTreeMap::new()),
            target: Mutex::new(None),
        }
    }
    
    fn new_symlink(ino: u64, target: &str) -> Self {
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFLNK | 0o777),
                nlink: 1,
                size: target.len() as u64,
                ..Default::default()
            }),
            data: Mutex::new(Vec::new()),
            children: Mutex::new(BTreeMap::new()),
            target: Mutex::new(Some(target.to_string())),
        }
    }
}

impl InodeOps for Ext4InodeImpl {
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(self.attr.lock().clone())
    }
    
    fn setattr(&self, attr: &FileAttr) -> VfsResult<()> {
        let mut my_attr = self.attr.lock();
        my_attr.mode = attr.mode;
        my_attr.uid = attr.uid;
        my_attr.gid = attr.gid;
        my_attr.size = attr.size;
        my_attr.atime = attr.atime;
        my_attr.mtime = attr.mtime;
        my_attr.ctime = attr.ctime;
        Ok(())
    }
    
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);
        
        let children = self.children.lock();
        children.get(name)
            .cloned()
            .ok_or(VfsError::NotFound)
    }
    
    fn create(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);
        
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        // Allocate new inode (simplified - would use superblock allocator)
        static NEXT_INO: AtomicU64 = AtomicU64::new(100);
        let ino = NEXT_INO.fetch_add(1, Ordering::Relaxed);
        
        let inode = Arc::new(Ext4InodeImpl::new_file(ino));
        {
            let mut attr = inode.attr.lock();
            attr.mode = mode;
        }
        
        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }
    
    fn mkdir(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);
        
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        static NEXT_INO: AtomicU64 = AtomicU64::new(100);
        let ino = NEXT_INO.fetch_add(1, Ordering::Relaxed);
        
        let inode = Arc::new(Ext4InodeImpl::new_dir(ino));
        {
            let mut attr = inode.attr.lock();
            attr.mode = FileMode(FileMode::S_IFDIR | mode.permissions());
        }
        
        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }
    
    fn unlink(&self, name: &str) -> VfsResult<()> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);
        
        let mut children = self.children.lock();
        let inode = children.get(name).ok_or(VfsError::NotFound)?;
        
        let iattr = inode.getattr()?;
        if iattr.mode.is_dir() {
            return Err(VfsError::IsDirectory);
        }
        
        // Decrement nlink
        let mut new_attr = iattr.clone();
        if new_attr.nlink > 0 {
            new_attr.nlink -= 1;
            inode.setattr(&new_attr)?;
        }
        
        children.remove(name);
        Ok(())
    }
    
    fn rmdir(&self, name: &str) -> VfsResult<()> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);
        
        let mut children = self.children.lock();
        let inode = children.get(name).ok_or(VfsError::NotFound)?;
        
        if !inode.getattr()?.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        
        if !inode.is_empty()? {
            return Err(VfsError::NotEmpty);
        }
        
        children.remove(name);
        Ok(())
    }
    
    fn is_empty(&self) -> VfsResult<bool> {
        let children = self.children.lock();
        Ok(children.is_empty())
    }
    
    fn link(&self, name: &str, inode: Arc<dyn InodeOps>) -> VfsResult<()> {
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        // Increment nlink
        let mut attr = inode.getattr()?;
        attr.nlink += 1;
        inode.setattr(&attr)?;
        
        children.insert(name.to_string(), inode);
        Ok(())
    }
    
    fn rename(&self, old_name: &str, new_dir: &dyn InodeOps, new_name: &str) -> VfsResult<()> {
        let mut children = self.children.lock();
        let inode = children.remove(old_name).ok_or(VfsError::NotFound)?;
        
        // Add to new directory
        // Note: This is simplified - real implementation would handle cross-directory rename
        drop(children);
        
        // For same directory rename, just update the name
        if core::ptr::eq(self as *const _ as *const (), new_dir as *const _ as *const ()) {
            let mut children = self.children.lock();
            children.insert(new_name.to_string(), inode);
            Ok(())
        } else {
            // Cross-directory rename - would need proper implementation
            Err(VfsError::NotSupported)
        }
    }
    
    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        static NEXT_INO: AtomicU64 = AtomicU64::new(100);
        let ino = NEXT_INO.fetch_add(1, Ordering::Relaxed);
        
        let inode = Arc::new(Ext4InodeImpl::new_symlink(ino, target));
        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }
    
    fn readlink(&self) -> VfsResult<String> {
        let target = self.target.lock();
        target.clone().ok_or(VfsError::InvalidOperation)
    }
    
    fn readdir(&self, _offset: usize) -> VfsResult<Vec<DirEntry>> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);
        
        let children = self.children.lock();
        let mut entries = Vec::new();
        
        for (name, inode) in children.iter() {
            let iattr = inode.getattr()?;
            entries.push(DirEntry {
                name: name.clone(),
                ino: iattr.ino,
                file_type: iattr.mode.file_type(),
            });
        }
        
        Ok(entries)
    }
    
    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let data = self.data.lock();
        let start = offset as usize;
        
        if start >= data.len() {
            return Ok(0);
        }
        
        let end = (start + buf.len()).min(data.len());
        let len = end - start;
        buf[..len].copy_from_slice(&data[start..end]);
        
        Ok(len)
    }
    
    fn write(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let mut data = self.data.lock();
        let start = offset as usize;
        
        // Extend if necessary
        if start + buf.len() > data.len() {
            data.resize(start + buf.len(), 0);
        }
        
        data[start..start + buf.len()].copy_from_slice(buf);
        
        // Update size
        let mut attr = self.attr.lock();
        attr.size = data.len() as u64;
        
        Ok(buf.len())
    }
    
    fn truncate(&self, size: u64) -> VfsResult<()> {
        let mut data = self.data.lock();
        data.resize(size as usize, 0);
        
        let mut attr = self.attr.lock();
        attr.size = size;
        
        Ok(())
    }
    
    fn sync(&self) -> VfsResult<()> {
        // TODO: Sync inode to disk
        Ok(())
    }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize and register EXT4 file system
pub fn init() {
    let ext4 = Arc::new(Ext4FsType);
    if let Err(e) = super::vfs().register_fs(ext4) {
        crate::println!("[ext4] Failed to register ext4: {:?}", e);
    } else {
        crate::println!("[ext4] EXT4 file system registered");
    }
}

