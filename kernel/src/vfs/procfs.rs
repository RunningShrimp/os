//! ProcFS file system for testing

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};

use crate::{
    subsystems::fs::{
        FileMode, FilesystemStats, VfsError,
        api::{DirEntry, DirEntryType, types::FileAttr},
    },
    vfs::types::FsStats,
    vfs_interface::{FileAttr as VfsInterfaceFileAttr, FileSystemType, Inode, SuperBlock},
};

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

/// ProcFS inode implementation
impl Inode for ProcFsInode {
    fn getattr(&self) -> Result<VfsInterfaceFileAttr, VfsError> {
        Ok(VfsInterfaceFileAttr {
            inode: self.ino,
            file_type: self.file_type,
            mode: self.mode.0, // Extract u32 from FileMode
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
        })
    }

    fn setattr(&self, _attr: &VfsInterfaceFileAttr) -> Result<(), VfsError> {
        Ok(())
    }

    fn lookup(&self, _name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        Err(VfsError::NoEntry)
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, VfsError> {
        Ok(Vec::new())
    }

    fn read(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, VfsError> {
        Ok(0)
    }

    fn write(&self, _offset: u64, _buf: &[u8]) -> Result<usize, VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn create(
        &self,
        _name: &str,
        _mode: FileMode,
        _file_type: VfsFileType,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        Err(VfsError::PermissionDenied)
    }

    fn mkdir(&self, _name: &str, _mode: FileMode) -> Result<Arc<dyn Inode>, VfsError> {
        Err(VfsError::PermissionDenied)
    }

    fn unlink(&self, _name: &str) -> Result<(), VfsError> {
        Err(VfsError::PermissionDenied)
    }

    fn rmdir(&self, _name: &str) -> Result<(), VfsError> {
        Err(VfsError::PermissionDenied)
    }

    fn is_empty(&self) -> Result<bool, VfsError> {
        Ok(true)
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

    fn file_type(&self) -> VfsFileType {
        match self.file_type {
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
        self.name.clone()
    }

    fn parent(&self) -> Option<Arc<dyn Inode>> {
        None
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

impl SuperBlock for ProcFsSuperBlock {
    fn root(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }

    fn fs_type(&self) -> &str {
        "procfs"
    }

    fn sync(&self) -> Result<(), VfsError> {
        Ok(())
    }

    fn statfs(&self) -> Result<FilesystemStats, VfsError> {
        let stats = FsStats {
            bsize: 4096,
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: 1,
            ffree: u64::MAX,
            namelen: 255,
            f_type: 0,
            f_bsize: 4096,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 1,
            f_ffree: u64::MAX,
            f_fsid: 0,
            f_namelen: 255,
            f_frsize: 4096,
        };
        Ok(stats.into())
    }

    fn unmount(&self) -> Result<(), VfsError> {
        Ok(())
    }
}

/// ProcFS file system type
pub struct ProcFsType;

impl FileSystemType for ProcFsType {
    fn name(&self) -> &str {
        "procfs"
    }

    fn mount(&self, _device: Option<&str>, _flags: u32) -> Result<Arc<dyn SuperBlock>, VfsError> {
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
