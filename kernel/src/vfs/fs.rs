//! SysFS file system type and superblock

extern crate alloc;
use alloc::{string::String, string::ToString, sync::Arc, vec::Vec, collections::BTreeMap, boxed::Box};
use crate::vfs::types::FileType;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;

use super::{
    devices,
    kernel,
};
use crate::vfs::{
    error::*,
    types::*,
    dir::DirEntry,
};

/// SysFS file system type
pub struct SysFsType;

impl super::fs::FileSystemType for SysFsType {
    fn name(&self) -> &str {
        "sysfs"
    }
    
    fn mount(&self, _device: Option<&str>, _flags: u32) -> VfsResult<Arc<dyn super::fs::SuperBlock>> {
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
        let root = Arc::new(SysFsInode::new_dir(1));
        
        Self {
            root,
            next_ino: AtomicUsize::new(2),
        }
    }
    
    fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl super::fs::SuperBlock for SysFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }
    
    fn fs_type(&self) -> &str {
        "sysfs"
    }
    
    fn sync(&self) -> VfsResult<()> {
        Ok(()) // SysFS doesn't need sync
    }
    
    fn statfs(&self) -> VfsResult<super::fs::FsStats> {
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

/// SysFS inode
pub struct SysFsInode {
    attr: Mutex<FileAttr>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn InodeOps>>>,
    // For regular files - content generator
    content_gen: Mutex<Option<Box<dyn Fn() -> String + Send + Sync>>>,
    // Inode type
    inode_type: SysFsInodeType,
}

#[derive(Clone, Copy)]
enum SysFsInodeType {
    Directory,
    RegularFile,
    Symlink,
}

impl SysFsInode {
    /// Create a new directory inode
    pub fn new_dir(ino: u64) -> Self {
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFDIR | 0o555),
                nlink: 2,
                ..Default::default()
            }),
            children: Mutex::new(BTreeMap::new()),
            content_gen: Mutex::new(None),
            inode_type: SysFsInodeType::Directory,
        }
    }
    
    /// Create a new regular file inode with content generator
    pub fn new_file(ino: u64, content_gen: Box<dyn Fn() -> String + Send + Sync>) -> Self {
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFREG | 0o444),
                nlink: 1,
                size: 0, // Will be calculated on read
                ..Default::default()
            }),
            children: Mutex::new(BTreeMap::new()),
            content_gen: Mutex::new(Some(content_gen)),
            inode_type: SysFsInodeType::RegularFile,
        }
    }
    
    /// Add a child inode
    pub fn add_child(&self, name: String, inode: Arc<dyn InodeOps>) {
        self.children.lock().insert(name, inode);
    }

    /// Get mutable reference to children map
    pub fn children(&self) -> &Mutex<BTreeMap<String, Arc<dyn InodeOps>>> {
        &self.children
    }
    
    /// Create a symlink inode
    pub fn new_symlink(ino: u64, target: &str) -> Self {
        let target_clone = target.to_string();
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFLNK | 0o777),
                nlink: 1,
                size: target_clone.len() as u64,
                ..Default::default()
            }),
            children: Mutex::new(BTreeMap::new()),
            content_gen: Mutex::new(Some(Box::new(move || target_clone.clone()))),
            inode_type: SysFsInodeType::Symlink,
        }
    }
}

impl super::fs::InodeOps for SysFsInode {
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(self.attr.lock().clone())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        // Check standard /sys directories
        match name {
            "devices" => {
                return devices::create_root();
            }
            "bus" => {
                return devices::create_bus_root();
            }
            "class" => {
                return devices::create_class_root();
            }
            "dev" => {
                return devices::create_dev_root();
            }
            "kernel" => {
                return kernel::create_root();
            }
            "module" => {
                return kernel::create_module_root();
            }
            "firmware" => {
                return devices::create_firmware_root();
            }
            "fs" => {
                return devices::create_fs_root();
            }
            "power" => {
                return devices::create_power_root();
            }
            _ => {}
        }
        
        // Check children
        let children = self.children.lock();
        if let Some(inode) = children.get(name) {
            return Ok(inode.clone());
        }
        
        Err(VfsError::NotFound)
    }

    fn readdir(&self, offset: usize) -> VfsResult<Vec<DirEntry>> {
        if !self.attr.lock().mode.is_dir() {
            return Err(VfsError::NotDirectory);
        }
        
        let mut entries = Vec::new();
        
        // Add "." and ".."
        if offset == 0 {
            entries.push(DirEntry {
                ino: self.attr.lock().ino,
                name: ".".to_string(),
                file_type: FileType::Directory,
            });
        }
        if offset <= 1 {
            entries.push(DirEntry {
                ino: 1, // Root inode
                name: "..".to_string(),
                file_type: FileType::Directory,
            });
        }
        
        // Add standard /sys directories
        let standard_dirs = ["devices", "bus", "class", "dev", "kernel", "module", "firmware", "fs", "power"];
        let start_idx = if offset > 2 { offset - 2 } else { 0 };
        for (idx, dir) in standard_dirs.iter().enumerate().skip(start_idx) {
            entries.push(DirEntry {
                ino: 1000 + idx as u64,
                name: dir.to_string(),
                file_type: FileType::Directory,
            });
        }
        
        // Add children
        let children = self.children.lock();
        let child_start = if offset > 2 + standard_dirs.len() { offset - 2 - standard_dirs.len() } else { 0 };
        for (idx, (name, _)) in children.iter().enumerate().skip(child_start) {
            entries.push(DirEntry {
                ino: 2000 + idx as u64,
                name: name.clone(),
                file_type: FileType::Directory,
            });
        }
        
        Ok(entries)
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        if self.attr.lock().mode.is_dir() {
            return Err(VfsError::IsDirectory);
        }
        
        let content_gen = self.content_gen.lock();
        if let Some(ref r#gen) = *content_gen {
            let content = r#gen();
            let content_bytes = content.as_bytes();
            let start = offset as usize;
            if start >= content_bytes.len() {
                return Ok(0);
            }
            let end = core::cmp::min(start + buf.len(), content_bytes.len());
            let len = end - start;
            buf[..len].copy_from_slice(&content_bytes[start..end]);
            
            // Update size in attributes
            let mut attr = self.attr.lock();
            attr.size = content_bytes.len() as u64;
            
            Ok(len)
        } else {
            Err(VfsError::InvalidOperation)
        }
    }
    
    fn readlink(&self) -> VfsResult<String> {
        if self.attr.lock().mode.file_type() != FileType::Symlink {
            return Err(VfsError::InvalidOperation);
        }
        
        let content_gen = self.content_gen.lock();
        if let Some(ref r#gen) = *content_gen {
            Ok(r#gen())
        } else {
            Err(VfsError::InvalidOperation)
        }
    }
}

/// Initialize and register SysFS
pub fn init() {
    let sysfs = Arc::new(SysFsType);
    if let Err(e) = super::super::vfs().register_fs(sysfs) {
        crate::println!("[sysfs] Failed to register sysfs: {:?}", e);
    } else {
        crate::println!("[sysfs] Registered sysfs filesystem");
    }
}

