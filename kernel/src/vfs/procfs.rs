//! ProcFS file system for testing

extern crate alloc;

use crate::prelude::*;

use alloc::{sync::Arc, vec::Vec};

use crate::vfs::{
    core::{FileSystemType, FsStats, SuperBlock as VfsSuperBlock},
    inode::InodeOps,
    types::FileMode,
    VfsResult,
};
use crate::vfs_interface::{
    DirEntry, FileAttr, FileType as VfsFileType, Inode, VfsError,
};

// Directory entry types (local to procfs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryType {
    File,
    Directory,
    SymbolicLink,
    BlockDevice,
    CharacterDevice,
    FIFO,
    Socket,
}

/// ProcFS inode structure
pub struct ProcFsInode {
    ino: u64,
    name: String,
    file_type: DirEntryType,
    mode: FileMode,
}

impl ProcFsInode {
    /// Create a new procfs inode
    pub fn new(ino: u64, name: &str, file_type: DirEntryType, mode: FileMode) -> Self {
        Self { ino, name: name.to_string(), file_type, mode }
    }
}

/// ProcFS inode implementation - implements InodeOps
impl InodeOps for ProcFsInode {
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(FileAttr {
            ino: self.ino,
            mode: self.mode,
            nlink: 1,
            uid: 0,
            gid: 0,
            size: 0,
            blksize: 4096,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            rdev: 0,
        })
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        Err(VfsError::NoEntry)
    }

    fn readdir(&self, _offset: usize) -> VfsResult<Vec<DirEntry>> {
        Ok(Vec::new())
    }

    fn read(&self, _offset: u64, _buf: &mut [u8]) -> VfsResult<usize> {
        Ok(0)
    }

    fn write(&self, _offset: u64, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::IoError)
    }

    fn create(&self, _name: &str, _mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        Err(VfsError::PermissionDenied)
    }

    fn mkdir(&self, _name: &str, _mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        Err(VfsError::PermissionDenied)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn is_empty(&self) -> VfsResult<bool> {
        Ok(true)
    }

    fn link(&self, _name: &str, _inode: Arc<dyn InodeOps>) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn symlink(&self, _name: &str, _target: &str) -> VfsResult<Arc<dyn InodeOps>> {
        Err(VfsError::NotSupported)
    }

    fn readlink(&self) -> VfsResult<String> {
        Err(VfsError::NotASymlink)
    }

    fn truncate(&self, _size: u64) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }
}

/// Implement vfs_interface::Inode for ProcFsInode
impl Inode for ProcFsInode {
    fn file_type(&self) -> VfsFileType {
        match self.file_type {
            DirEntryType::File => VfsFileType::Regular,
            DirEntryType::Directory => VfsFileType::Directory,
            DirEntryType::SymbolicLink => VfsFileType::Symlink,
            DirEntryType::BlockDevice => VfsFileType::BlockDevice,
            DirEntryType::CharacterDevice => VfsFileType::CharDevice,
            DirEntryType::FIFO => VfsFileType::Fifo,
            DirEntryType::Socket => VfsFileType::Socket,
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn parent(&self) -> Option<Arc<dyn Inode>> {
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
        self.ino
    }

    fn mode(&self) -> FileMode {
        self.mode
    }
}

/// ProcFS superblock
struct ProcFsSuperBlock {
    root: Arc<ProcFsInode>,
}

impl ProcFsSuperBlock {
    fn new() -> Self {
        Self {
            root: Arc::new(ProcFsInode::new(1, "/", DirEntryType::Directory, FileMode::new(0o555))),
        }
    }
}

impl VfsSuperBlock for ProcFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }

    fn fs_type(&self) -> &str {
        "procfs"
    }

    fn sync(&self) -> VfsResult<()> {
        Ok(())
    }

    fn statfs(&self) -> VfsResult<FsStats> {
        Ok(FsStats {
            bsize: 4096,
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: 0,
            ffree: 0,
            namelen: 255,
        })
    }

    fn unmount(&self) -> VfsResult<()> {
        Ok(())
    }
}

/// ProcFS file system type
pub struct ProcFsType;

impl FileSystemType for ProcFsType {
    fn name(&self) -> &str {
        "procfs"
    }

    fn mount(&self, _device: Option<&str>, _flags: u32) -> VfsResult<Arc<dyn VfsSuperBlock>> {
        Ok(Arc::new(ProcFsSuperBlock::new()))
    }
}

/// Initialize and register ProcFS
pub fn init() {
    let procfs = Arc::new(ProcFsType);
    if let Err(e) = crate::subsystems::fs::vfs().register_fs(procfs) {
        crate::println!("[procfs] Failed to register procfs: {:?}", e);
    } else {
        crate::println!("[procfs] ProcFS file system registered");
    }
}
