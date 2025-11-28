//! Temporary file system (tmpfs) implementation
//! 
//! Similar to ramfs but with size limits and better performance

use alloc::{string::String, sync::Arc, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::Mutex;

use super::{
    error::*,
    types::*,
    fs::{FileSystemType, SuperBlock, InodeOps},
    dir::DirEntry,
};

/// TmpFS file system type
pub struct TmpFsType;

impl FileSystemType for TmpFsType {
    fn name(&self) -> &str {
        "tmpfs"
    }
    
    fn mount(&self, _device: Option<&str>, _flags: u32) -> VfsResult<Arc<dyn SuperBlock>> {
        Ok(Arc::new(TmpFsSuperBlock::new()))
    }
}

/// TmpFS superblock
struct TmpFsSuperBlock {
    root: Arc<TmpFsInode>,
    next_ino: AtomicUsize,
    total_bytes: AtomicUsize,
    max_bytes: AtomicUsize,
}

impl TmpFsSuperBlock {
    fn new() -> Self {
        Self {
            root: Arc::new(TmpFsInode::new_dir(1, None)),
            next_ino: AtomicUsize::new(2),
            total_bytes: AtomicUsize::new(0),
            max_bytes: AtomicUsize::new(100 * 1024 * 1024), // 100MB default
        }
    }
    
    fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl SuperBlock for TmpFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }
    
    fn fs_type(&self) -> &str {
        "tmpfs"
    }
    
    fn sync(&self) -> VfsResult<()> {
        Ok(()) // TmpFS doesn't need sync
    }
    
    fn statfs(&self) -> VfsResult<FsStats> {
        Ok(FsStats {
            bsize: 4096,
            blocks: (self.total_bytes.load(Ordering::Relaxed) + 4095) / 4096,
            bfree: 0, // No limit enforcement yet
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

/// TmpFS inode
struct TmpFsInode {
    attr: Mutex<FileAttr>,
    // For regular files
    data: Mutex<Vec<u8>>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn InodeOps>>>,
    // For symlinks
    target: Mutex<Option<String>>,
    // Parent superblock reference
    sb: Option<Arc<TmpFsSuperBlock>>,
}

impl TmpFsInode {
    fn new_file(ino: u64, sb: Option<Arc<TmpFsSuperBlock>>) -> Self {
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
            sb,
        }
    }
    
    fn new_dir(ino: u64, sb: Option<Arc<TmpFsSuperBlock>>) -> Self {
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
            sb,
        }
    }
    
    fn new_symlink(ino: u64, target: &str, sb: Option<Arc<TmpFsSuperBlock>>) -> Self {
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
            sb,
        }
    }
}

impl InodeOps for TmpFsInode {
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
        
        // Create new file inode
        let sb = self.sb.clone();
        let ino = sb.as_ref().map(|s| s.alloc_ino()).unwrap_or(1000 + name.len() as u64);
        
        let inode = Arc::new(TmpFsInode::new_file(ino, sb));
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
        
        let sb = self.sb.clone();
        let ino = sb.as_ref().map(|s| s.alloc_ino()).unwrap_or(1000 + name.len() as u64);
        
        let inode = Arc::new(TmpFsInode::new_dir(ino, sb));
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
        let attr = inode.getattr()?;
        if attr.mode.is_dir() {
            return Err(VfsError::IsDirectory);
        }
        
        // Decrement nlink
        let mut iattr = attr.clone();
        if iattr.nlink > 0 {
            iattr.nlink -= 1;
            inode.setattr(&iattr)?;
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
    
    fn symlink(&self, name: &str, target: &str) -> VfsResult<()> {
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::Exists);
        }
        
        let sb = self.sb.clone();
        let ino = sb.as_ref().map(|s| s.alloc_ino()).unwrap_or(1000 + name.len() as u64);
        
        let inode = Arc::new(TmpFsInode::new_symlink(ino, target, sb));
        children.insert(name.to_string(), inode);
        Ok(())
    }
    
    fn readlink(&self) -> VfsResult<String> {
        let target = self.target.lock();
        target.clone().ok_or(VfsError::InvalidArgument)
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
        let new_len = start + buf.len();
        if new_len > data.len() {
            let current_len = data.len();
            data.resize(new_len, 0);
            
            // Update superblock total bytes
            if let Some(sb) = &self.sb {
                sb.total_bytes.fetch_add(new_len - current_len, Ordering::Relaxed);
            }
        }
        
        data[start..start + buf.len()].copy_from_slice(buf);
        
        // Update size
        let mut attr = self.attr.lock();
        attr.size = data.len() as u64;
        
        Ok(buf.len())
    }
    
    fn truncate(&self, size: u64) -> VfsResult<()> {
        let mut data = self.data.lock();
        let current_len = data.len();
        let new_len = size as usize;
        
        if new_len < current_len {
            // Shrink
            data.truncate(new_len);
            
            // Update superblock total bytes
            if let Some(sb) = &self.sb {
                sb.total_bytes.fetch_sub(current_len - new_len, Ordering::Relaxed);
            }
        } else if new_len > current_len {
            // Extend
            data.resize(new_len, 0);
            
            // Update superblock total bytes
            if let Some(sb) = &self.sb {
                sb.total_bytes.fetch_add(new_len - current_len, Ordering::Relaxed);
            }
        }
        
        // Update inode size
        let mut attr = self.attr.lock();
        attr.size = size;
        
        Ok(())
    }
}

/// Initialize and register TmpFS
pub fn init() {
    let tmpfs = Arc::new(TmpFsType);
    super::vfs().register_fs(tmpfs).expect("Failed to register tmpfs");
}