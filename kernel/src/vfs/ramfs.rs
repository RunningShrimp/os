//! Simple RAM file system for testing

extern crate alloc;
use alloc::{string::{String, ToString}, sync::Arc, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;

use super::{
    error::*,
    types::*,
    fs::{FileSystemType, SuperBlock, InodeOps, FsStats},
    dir::DirEntry,
};

/// RamFS file system type
pub struct RamFsType;

impl FileSystemType for RamFsType {
    fn name(&self) -> &str {
        "ramfs"
    }
    
    fn mount(&self, _device: Option<&str>, _flags: u32) -> VfsResult<Arc<dyn SuperBlock>> {
        Ok(Arc::new(RamFsSuperBlock::new()))
    }
}

/// RamFS superblock
struct RamFsSuperBlock {
    root: Arc<RamFsInode>,
    next_ino: AtomicUsize,
}

impl RamFsSuperBlock {
    fn new() -> Self {
        Self {
            root: Arc::new(RamFsInode::new_dir(1)),
            next_ino: AtomicUsize::new(2),
        }
    }
    
    fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl SuperBlock for RamFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }
    
    fn fs_type(&self) -> &str {
        "ramfs"
    }
    
    fn sync(&self) -> VfsResult<()> {
        Ok(()) // RAM fs doesn't need sync
    }
    
    fn statfs(&self) -> VfsResult<FsStats> {
        Ok(FsStats {
            bsize: 4096,
            blocks: 0,
            bfree: 0,
            bavail: 0,
            files: self.next_ino.load(Ordering::Relaxed) as u64,
            ffree: u64::MAX,
            namelen: 255,
        })
    }
    
    fn unmount(&self) -> VfsResult<()> {
        Ok(())
    }
}

/// RamFS inode
struct RamFsInode {
    attr: Mutex<FileAttr>,
    // For regular files
    data: Mutex<Vec<u8>>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn InodeOps>>>,
    // For symlinks
    target: Mutex<Option<String>>,
}

impl RamFsInode {
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

impl InodeOps for RamFsInode {
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
        let children = self.children.lock();
        children.get(name)
            .cloned()
            .ok_or(VfsError::NotFound)
    }
    
    fn create(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let mut children = self.children.lock();
        
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        // Use a simple counter for ino
        static NEXT_INO: AtomicUsize = AtomicUsize::new(100);
        let ino = NEXT_INO.fetch_add(1, Ordering::Relaxed) as u64;
        
        let inode = Arc::new(RamFsInode::new_file(ino));
        {
            let mut attr = inode.attr.lock();
            attr.mode = mode;
        }
        
        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }
    
    fn mkdir(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let mut children = self.children.lock();
        
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        static NEXT_INO: AtomicUsize = AtomicUsize::new(100);
        let ino = NEXT_INO.fetch_add(1, Ordering::Relaxed) as u64;
        
        let inode = Arc::new(RamFsInode::new_dir(ino));
        {
            let mut attr = inode.attr.lock();
            attr.mode = FileMode(FileMode::S_IFDIR | mode.permissions());
        }
        
        children.insert(name.to_string(), inode.clone());
        Ok(inode)
    }
    
    fn unlink(&self, name: &str) -> VfsResult<()> {
        let mut children = self.children.lock();
        
        let inode = children.get(name).ok_or(VfsError::NotFound)?;
        if inode.getattr()?.mode.is_dir() {
            return Err(VfsError::IsDirectory);
        }
        
        // Decrement nlink
        let mut attr = inode.getattr()?;
        if attr.nlink > 0 {
            attr.nlink -= 1;
            inode.setattr(&attr)?;
        }

        children.remove(name);
        Ok(())
    }
    
    fn rmdir(&self, name: &str) -> VfsResult<()> {
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
        
        static NEXT_INO: AtomicUsize = AtomicUsize::new(100);
        let ino = NEXT_INO.fetch_add(1, Ordering::Relaxed) as u64;
        
        let inode = Arc::new(RamFsInode::new_symlink(ino, target));
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
}


/// Initialize and register RamFS
pub fn init() {
    let ramfs = Arc::new(RamFsType);
    if let Err(e) = super::vfs().register_fs(ramfs) {
        crate::println!("[ramfs] Failed to register ramfs: {:?}", e);
        // In a production system, this might be fatal, but for now we log and continue
    }
}