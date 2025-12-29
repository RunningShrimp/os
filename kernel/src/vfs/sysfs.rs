//! SysFS file system for testing

extern crate alloc;
use alloc::{collections::BTreeMap, string::ToString, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use super::{error::*, types::*};
use crate::{
    subsystems::fs::{FileMode, FilesystemStats, api::DirEntry},
    vfs::types::FsStats,
    vfs_interface::{
        DirEntryType, FileAttr as VfsInterfaceFileAttr, FileType as VfsFileType, Inode, SuperBlock,
    },
};

/// SysFS file system type
pub struct SysFsType;

impl crate::vfs_interface::FileSystemType for SysFsType {
    fn name(&self) -> &str {
        "sysfs"
    }

    fn mount(&self, _device: Option<&str>, _flags: u32) -> Result<Arc<dyn SuperBlock>, VfsError> {
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

    fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl SuperBlock for SysFsSuperBlock {
    fn root(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }

    fn fs_type(&self) -> &str {
        "sysfs"
    }

    fn sync(&self) -> Result<(), VfsError> {
        Ok(())
    }

    fn statfs(&self) -> Result<FilesystemStats, VfsError> {
        let total_files = self.next_ino.load(Ordering::Relaxed) as u64;
        let stats = FsStats {
            f_type: 0,
            f_bsize: 4096,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: total_files,
            f_ffree: u64::MAX,
            f_fsid: 0,
            f_namelen: 255,
            f_frsize: 4096,
            bsize: 4096,
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: total_files,
            ffree: u64::MAX,
            namelen: 255,
        };
        Ok(stats.into())
    }

    fn unmount(&self) -> Result<(), VfsError> {
        Ok(())
    }
}

/// SysFS inode
struct SysFsInode {
    name: Mutex<String>,
    parent_ino: Mutex<Option<u64>>,
    attr: Mutex<VfsInterfaceFileAttr>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn Inode>>>,
}

impl SysFsInode {
    fn new_dir(ino: u64, name: &str) -> Self {
        Self {
            name: Mutex::new(name.to_string()),
            parent_ino: Mutex::new(None),
            attr: Mutex::new(VfsInterfaceFileAttr {
                inode: ino,
                file_type: DirEntryType::Directory,
                mode: 0o555,
                nlink: 2,
                uid: 0,
                gid: 0,
                rdev: 0,
                size: 0,
                blksize: 4096,
                blocks: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
            }),
            children: Mutex::new(BTreeMap::new()),
        }
    }

    fn new_file(ino: u64, name: &str) -> Self {
        Self {
            name: Mutex::new(name.to_string()),
            parent_ino: Mutex::new(Some(1)),
            attr: Mutex::new(VfsInterfaceFileAttr {
                inode: ino,
                file_type: DirEntryType::File,
                mode: 0o644,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                size: 0,
                blksize: 4096,
                blocks: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
            }),
            children: Mutex::new(BTreeMap::new()),
        }
    }

    fn new_symlink(ino: u64, name: &str, target: &str) -> Self {
        Self {
            name: Mutex::new(name.to_string()),
            parent_ino: Mutex::new(Some(1)),
            attr: Mutex::new(VfsInterfaceFileAttr {
                inode: ino,
                file_type: DirEntryType::SymbolicLink,
                mode: 0o777,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                size: target.len() as u64,
                blksize: 4096,
                blocks: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
            }),
            children: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Inode for SysFsInode {
    fn getattr(&self) -> Result<VfsInterfaceFileAttr, VfsError> {
        let attr = self.attr.lock().clone();
        Ok(VfsInterfaceFileAttr {
            inode: attr.inode,
            file_type: attr.file_type,
            mode: attr.mode,
            nlink: attr.nlink,
            uid: attr.uid,
            gid: attr.gid,
            rdev: attr.rdev,
            size: attr.size,
            blksize: attr.blksize,
            blocks: attr.blocks,
            atime: attr.atime,
            mtime: attr.mtime,
            ctime: attr.ctime,
        })
    }

    fn setattr(&self, _attr: &VfsInterfaceFileAttr) -> Result<(), VfsError> {
        // Default implementation - no-op
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let children = self.children.lock();
        children.get(name).cloned().ok_or(VfsError::NoEntry)
    }

    fn create(
        &self,
        name: &str,
        mode: FileMode,
        file_type: VfsFileType,
    ) -> Result<Arc<dyn Inode>, VfsError> {
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
        let parent_ino = Some(ino);

        let inode: Arc<dyn Inode> = match file_type {
            VfsFileType::Directory => Arc::new(SysFsInode::new_dir(ino, name)),
            _ => Arc::new(SysFsInode::new_file(ino, name)),
        };

        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }

    fn mkdir(&self, name: &str, mode: FileMode) -> Result<Arc<dyn Inode>, VfsError> {
        self.create(name, mode, VfsFileType::Directory)
    }

    fn unlink(&self, name: &str) -> Result<(), VfsError> {
        let mut children = self.children.lock();

        let inode = children.get(name).ok_or(VfsError::NoEntry)?;
        let attr = inode.getattr()?;
        if attr.file_type == DirEntryType::Directory {
            return Err(VfsError::IsDirectory);
        }

        children.remove(name);
        Ok(())
    }

    fn rmdir(&self, name: &str) -> Result<(), VfsError> {
        let mut children = self.children.lock();

        let inode = children.get(name).ok_or(VfsError::NoEntry)?;
        let attr = inode.getattr()?;
        if attr.file_type != DirEntryType::Directory {
            return Err(VfsError::NotDirectory);
        }

        if !inode.is_empty()? {
            return Err(VfsError::NotEmpty);
        }

        children.remove(name);
        Ok(())
    }

    fn is_empty(&self) -> Result<bool, VfsError> {
        let children = self.children.lock();
        Ok(children.is_empty())
    }

    fn link(&self, _name: &str, _inode: Arc<dyn Inode>) -> Result<(), VfsError> {
        Err(VfsError::IoError)
    }

    fn symlink(&self, _name: &str, _target: &str) -> Result<Arc<dyn Inode>, VfsError> {
        Err(VfsError::IoError)
    }

    fn readlink(&self) -> Result<String, VfsError> {
        Err(VfsError::IoError)
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, VfsError> {
        let attr = self.attr.lock();
        if attr.file_type != DirEntryType::Directory {
            return Err(VfsError::NotDirectory);
        }
        drop(attr);

        let children = self.children.lock();
        let mut entries = Vec::new();

        for (name, inode) in children.iter() {
            let iattr = inode.getattr()?;
            entries.push(DirEntry {
                name: name.clone(),
                entry_type: iattr.file_type,
                attributes: iattr.clone(),
                inode: iattr.inode,
            });
        }

        Ok(entries)
    }

    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn write(&self, _offset: u64, _buf: &[u8]) -> Result<usize, VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn ino(&self) -> u64 {
        self.attr.lock().inode
    }

    fn mode(&self) -> FileMode {
        FileMode::new(self.attr.lock().mode)
    }

    fn file_type(&self) -> VfsFileType {
        match self.attr.lock().file_type {
            DirEntryType::File => VfsFileType::RegularFile,
            DirEntryType::Directory => VfsFileType::Directory,
            DirEntryType::SymbolicLink => VfsFileType::SymbolicLink,
            DirEntryType::BlockDevice => VfsFileType::BlockDevice,
            DirEntryType::CharacterDevice => VfsFileType::CharacterDevice,
            DirEntryType::FIFO => VfsFileType::Fifo,
            DirEntryType::Socket => VfsFileType::Socket,
        }
    }

    fn name(&self) -> String {
        self.name.lock().clone()
    }

    fn parent(&self) -> Option<Arc<dyn Inode>> {
        self.parent_ino
            .lock()
            .and_then(|parent_ino| self.children.lock().get(&".".to_string()).cloned())
    }

    fn symlink_target(&self) -> Option<String> {
        None
    }

    fn sync(&self) -> Result<(), VfsError> {
        Ok(())
    }

    fn truncate(&self, _size: u64) -> Result<(), VfsError> {
        Err(VfsError::IoError)
    }

    fn rename(
        &self,
        _old_name: &str,
        _new_dir: &dyn Inode,
        _new_name: &str,
    ) -> Result<(), VfsError> {
        Err(VfsError::IoError)
    }
}

/// Initialize and register SysFS
pub fn init() {
    let sysfs = Arc::new(SysFsType);
    if let Err(e) = crate::subsystems::fs::vfs().register_fs(sysfs) {
        crate::println!("[sysfs] Failed to register sysfs: {:?}", e);
    }
}
