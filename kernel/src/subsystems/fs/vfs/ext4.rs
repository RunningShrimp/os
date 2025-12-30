//! EXT4 file system implementation
//!
//! Implements EXT4 file system support for VFS layer
//! This is a simplified implementation focusing on basic operations

extern crate alloc;

use crate::prelude::*;

use core::sync::atomic::{AtomicU64, Ordering};
use alloc::{collections::BTreeMap, string::String, string::ToString, vec::Vec};

use super::{
    core::{FileSystemType, FsStats, SuperBlock},
    inode::{FileLock, InodeOps},
};
use crate::subsystems::sync::Mutex;

// Re-export types from vfs_interface to match InodeOps trait signature
use crate::subsystems::fs::vfs_interface::{
    DirEntry as DirEntry,
    FileAttr as FileAttr,
    FileMode as FileMode,
    VfsError as VfsError,
};

// VfsResult type alias
pub type VfsResult<T> = core::result::Result<T, VfsError>;

// ============================================================================
// EXT4 Constants
// ============================================================================

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
            total_blocks: AtomicU64::new(0),
            free_blocks: AtomicU64::new(0),
            total_inodes: AtomicU64::new(0),
            free_inodes: AtomicU64::new(0),
        }
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
            bsize: 4096,
            blocks: self.total_blocks.load(Ordering::Relaxed),
            bfree: self.free_blocks.load(Ordering::Relaxed),
            bavail: self.free_blocks.load(Ordering::Relaxed),
            files: self.total_inodes.load(Ordering::Relaxed),
            ffree: self.free_inodes.load(Ordering::Relaxed),
            namelen: 255,
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
    // Extended attributes (name -> value)
    xattrs: Mutex<BTreeMap<String, Vec<u8>>>,
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
            xattrs: Mutex::new(BTreeMap::new()),
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
            xattrs: Mutex::new(BTreeMap::new()),
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
            xattrs: Mutex::new(BTreeMap::new()),
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
        my_attr.nlink = attr.nlink;
        Ok(())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let attr = self.attr.lock();
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);

        let children = self.children.lock();
        children.get(name).cloned().ok_or(VfsError::NotFound)
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

    fn get_file_lock(&self, _cmd: u32, _lock: &FileLock) -> VfsResult<u64> {
        // TODO: Implement file locking
        Err(VfsError::NotSupported)
    }

    fn release_file_lock(&self, _lock: &FileLock) -> VfsResult<()> {
        // TODO: Implement file locking
        Err(VfsError::NotSupported)
    }

    fn set_xattr(&self, name: &str, value: &[u8], _flags: u32) -> VfsResult<()> {
        let mut xattrs = self.xattrs.lock();
        xattrs.insert(name.to_string(), value.to_vec());
        Ok(())
    }

    fn get_xattr(&self, name: &str, value: &mut [u8]) -> VfsResult<usize> {
        let xattrs = self.xattrs.lock();
        if let Some(val) = xattrs.get(name) {
            let len = val.len().min(value.len());
            value[..len].copy_from_slice(&val[..len]);
            Ok(val.len())
        } else {
            Err(VfsError::NotFound)
        }
    }

    fn remove_xattr(&self, name: &str) -> VfsResult<()> {
        let mut xattrs = self.xattrs.lock();
        xattrs.remove(name)
            .map(|_| ())
            .ok_or(VfsError::NotFound)
    }

    fn list_xattr(&self, list: &mut [u8]) -> VfsResult<usize> {
        let xattrs = self.xattrs.lock();
        let mut offset = 0;
        for name in xattrs.keys() {
            let name_bytes = name.as_bytes();
            if offset + name_bytes.len() > list.len() {
                break;
            }
            list[offset..offset + name_bytes.len()].copy_from_slice(name_bytes);
            offset += name_bytes.len();
            if offset < list.len() {
                list[offset] = b'\0';
                offset += 1;
            }
        }
        Ok(offset)
    }
}

// Implement vfs_interface::Inode for Ext4InodeImpl
crate::impl_inode!(Ext4InodeImpl);

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
