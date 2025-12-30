//! SysFS file system for testing

extern crate alloc;

use crate::prelude::*;
use alloc::{collections::BTreeMap, string::ToString, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use crate::vfs_interface::{
    DirEntry, FileAttr, FileType as VfsFileType, FileMode as VfsFileMode, Inode, SuperBlock, VfsError,
};
use crate::vfs::InodeOps;
use crate::vfs::core::FsStats;
use crate::vfs::VfsResult;

/// SysFS file system type
pub struct SysFsType;

impl crate::vfs_interface::FileSystemType for SysFsType {
    fn name(&self) -> &str {
        "sysfs"
    }

    fn mount(&self, _device: Option<&str>, _flags: u32) -> core::result::Result<Arc<dyn SuperBlock>, VfsError> {
        Ok(Arc::new(SysFsSuperBlock::new()))
    }
}

/// SysFS superblock
struct SysFsSuperBlock {
    root: Arc<SysFsInode>,
    next_ino: AtomicUsize,
}

impl SysFsSuperBlock {
    fn new() -> Self {
        Self {
            root: Arc::new(SysFsInode::new_dir(1, "")),
            next_ino: AtomicUsize::new(2),
        }
    }
}

impl SuperBlock for SysFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }

    fn fs_type(&self) -> &str {
        "sysfs"
    }

    fn sync(&self) -> VfsResult<()> {
        Ok(())
    }

    fn statfs(&self) -> VfsResult<FsStats> {
        let total_files = self.next_ino.load(Ordering::Relaxed) as u64;
        let stats = FsStats {
            bsize: 4096,
            blocks: total_files,
            bfree: 0,
            bavail: 0,
            files: total_files,
            ffree: 0,
            namelen: 255,
        };
        Ok(stats)
    }

    fn unmount(&self) -> VfsResult<()> {
        Ok(())
    }
}

/// SysFS inode internal attributes
#[derive(Clone)]
struct SysFsFileAttr {
    file_type: VfsFileType,
    mode: VfsFileMode,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    uid: u32,
    gid: u32,
}

/// SysFS inode
struct SysFsInode {
    name: Mutex<String>,
    parent_ino: Mutex<Option<u64>>,
    attr: Mutex<SysFsFileAttr>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn InodeOps>>>,
}

impl SysFsInode {
    fn new_dir(ino: u64, name: &str) -> Self {
        let _ = ino; // Not used yet
        Self {
            name: Mutex::new(name.to_string()),
            parent_ino: Mutex::new(None),
            attr: Mutex::new(SysFsFileAttr {
                file_type: VfsFileType::Directory,
                mode: VfsFileMode(0o555),
                size: 0,
                blocks: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                uid: 0,
                gid: 0,
            }),
            children: Mutex::new(BTreeMap::new()),
        }
    }

    fn new_file(ino: u64, name: &str) -> Self {
        let _ = ino; // Not used yet
        Self {
            name: Mutex::new(name.to_string()),
            parent_ino: Mutex::new(Some(1)),
            attr: Mutex::new(SysFsFileAttr {
                file_type: VfsFileType::Regular,
                mode: VfsFileMode(0o644),
                size: 0,
                blocks: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                uid: 0,
                gid: 0,
            }),
            children: Mutex::new(BTreeMap::new()),
        }
    }

    fn new_symlink(ino: u64, name: &str, target: &str) -> Self {
        let _ = ino; // Not used yet
        Self {
            name: Mutex::new(name.to_string()),
            parent_ino: Mutex::new(Some(1)),
            attr: Mutex::new(SysFsFileAttr {
                file_type: VfsFileType::Symlink,
                mode: VfsFileMode(0o777),
                size: target.len() as u64,
                blocks: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                uid: 0,
                gid: 0,
            }),
            children: Mutex::new(BTreeMap::new()),
        }
    }
}

impl InodeOps for SysFsInode {
    fn getattr(&self) -> VfsResult<FileAttr> {
        let attr = self.attr.lock();
        // Convert VfsFileMode back to FileMode
        let vfs_mode = crate::vfs::types::FileMode(attr.mode.0);
        let file_attr = FileAttr {
            ino: 0,
            mode: vfs_mode,
            nlink: 1,
            uid: attr.uid,
            gid: attr.gid,
            size: attr.size,
            blksize: 4096,
            blocks: attr.blocks,
            atime: attr.atime,
            mtime: attr.mtime,
            ctime: attr.ctime,
            rdev: 0,
        };
        drop(attr);
        Ok(file_attr)
    }

    fn setattr(&self, _attr: &FileAttr) -> VfsResult<()> {
        // Default implementation - no-op
        Ok(())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let children = self.children.lock();
        children.get(name).cloned().ok_or(VfsError::NotFound)
    }

    fn create(&self, name: &str, _mode: VfsFileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let mut children = self.children.lock();

        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }

        // Get and increment inode number
        let ino = {
            let mut guard = self.parent_ino.lock();
            let current = guard.unwrap_or(0);
            *guard = Some(current + 1);
            current
        };

        let inode: Arc<dyn InodeOps> = Arc::new(SysFsInode::new_file(ino, name));

        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }

    fn mkdir(&self, name: &str, _mode: VfsFileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let mut children = self.children.lock();

        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }

        // Get and increment inode number
        let ino = {
            let mut guard = self.parent_ino.lock();
            let current = guard.unwrap_or(0);
            *guard = Some(current + 1);
            current
        };

        let inode: Arc<dyn InodeOps> = Arc::new(SysFsInode::new_dir(ino, name));

        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        let mut children = self.children.lock();

        let inode = children.get(name).ok_or(VfsError::NotFound)?;
        let attr = inode.getattr()?;
        if attr.mode.is_dir() {
            return Err(VfsError::IsDirectory);
        }

        children.remove(name);
        Ok(())
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        let mut children = self.children.lock();

        let inode = children.get(name).ok_or(VfsError::NotFound)?;
        let attr = inode.getattr()?;
        if !attr.mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }

        if !inode.is_empty()? {
            return Err(VfsError::IoError);
        }

        children.remove(name);
        Ok(())
    }

    fn is_empty(&self) -> VfsResult<bool> {
        let children = self.children.lock();
        Ok(children.is_empty())
    }

    fn link(&self, _name: &str, _inode: Arc<dyn InodeOps>) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let mut children = self.children.lock();

        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }

        // Get and increment inode number
        let ino = {
            let mut guard = self.parent_ino.lock();
            let current = guard.unwrap_or(0);
            *guard = Some(current + 1);
            current
        };

        let inode: Arc<dyn InodeOps> = Arc::new(SysFsInode::new_symlink(ino, name, target));

        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }

    fn readlink(&self) -> VfsResult<String> {
        Err(VfsError::InvalidOperation)
    }

    fn readdir(&self, _offset: usize) -> VfsResult<Vec<DirEntry>> {
        let attr = self.attr.lock();
        if attr.file_type != VfsFileType::Directory {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);

        let children = self.children.lock();
        let mut entries = Vec::new();

        for (name, inode) in children.iter() {
            let iattr = inode.getattr()?;
            entries.push(DirEntry {
                ino: 0, // We don't track this separately
                name: name.clone(),
                file_type: iattr.mode.file_type().into(),
            });
        }

        Ok(entries)
    }

    fn read(&self, _offset: u64, _buf: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }

    fn write(&self, _offset: u64, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }

    fn truncate(&self, _size: u64) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn get_file_lock(&self, _cmd: u32, _lock: &crate::vfs::inode::FileLock) -> VfsResult<u64> {
        Err(VfsError::NotSupported)
    }

    fn release_file_lock(&self, _lock: &crate::vfs::inode::FileLock) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn set_xattr(&self, _name: &str, _value: &[u8], _flags: u32) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn get_xattr(&self, _name: &str, _value: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }

    fn remove_xattr(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn list_xattr(&self, _list: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }
}

impl Inode for SysFsInode {
    fn file_type(&self) -> VfsFileType {
        self.attr.lock().file_type
    }

    fn name(&self) -> String {
        self.name.lock().clone()
    }

    fn parent(&self) -> Option<Arc<dyn Inode>> {
        // For simplicity, return None
        None
    }

    fn symlink_target(&self) -> Option<String> {
        None
    }

    fn sync(&self) -> VfsResult<()> {
        Ok(())
    }

    fn rename(&self, _old_name: &str, _new_name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn ino(&self) -> u64 {
        0
    }

    fn mode(&self) -> VfsFileMode {
        self.attr.lock().mode
    }
}

/// Initialize and register SysFS
pub fn init() {
    let sysfs = Arc::new(SysFsType);
    if let Err(e) = crate::subsystems::fs::vfs().register_fs(sysfs) {
        crate::println!("[sysfs] Failed to register sysfs: {:?}", e);
    }
}
