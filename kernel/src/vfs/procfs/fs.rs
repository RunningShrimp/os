//! ProcFS file system type and superblock

extern crate alloc;
use alloc::{string::String, string::ToString, sync::Arc, vec::Vec, collections::BTreeMap, boxed::Box};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::Mutex;

use super::{
    proc_info::ProcInfoInode,
    sys_info,
};
use crate::vfs::{
    error::*,
    types::*,
    fs::{FileSystemType, SuperBlock, InodeOps, FsStats},
    dir::DirEntry,
};

/// ProcFS file system type
pub struct ProcFsType;

impl FileSystemType for ProcFsType {
    fn name(&self) -> &str {
        "proc"
    }
    
    fn mount(&self, _device: Option<&str>, _flags: u32) -> VfsResult<Arc<dyn SuperBlock>> {
        Ok(Arc::new(ProcFsSuperBlock::new()))
    }
}

/// ProcFS superblock
struct ProcFsSuperBlock {
    root: Arc<ProcFsInode>,
    next_ino: AtomicUsize,
}

impl ProcFsSuperBlock {
    fn new() -> Self {
        let root = Arc::new(ProcFsInode::new_dir(1));
        
        // Create standard /proc entries
        let children = root.children.lock();
        
        // /proc/[pid] directories (created on-demand)
        // /proc/self -> symlink to current process
        // /proc/sys -> system information
        // /proc/meminfo -> memory information
        // /proc/stat -> CPU statistics
        // /proc/version -> kernel version
        // /proc/uptime -> system uptime
        // /proc/loadavg -> system load average
        
        drop(children);
        
        Self {
            root,
            next_ino: AtomicUsize::new(2),
        }
    }
    
    fn alloc_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed) as u64
    }
}

impl SuperBlock for ProcFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }
    
    fn fs_type(&self) -> &str {
        "proc"
    }
    
    fn sync(&self) -> VfsResult<()> {
        Ok(()) // ProcFS doesn't need sync
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

/// ProcFS inode
pub struct ProcFsInode {
    attr: Mutex<FileAttr>,
    // For directories
    children: Mutex<BTreeMap<String, Arc<dyn InodeOps>>>,
    // For regular files - content generator
    content_gen: Mutex<Option<Box<dyn Fn() -> String + Send + Sync>>>,
    // Inode type
    inode_type: ProcFsInodeType,
}

#[derive(Clone, Copy)]
enum ProcFsInodeType {
    Directory,
    RegularFile,
    Symlink,
}

impl ProcFsInode {
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
            inode_type: ProcFsInodeType::Directory,
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
            inode_type: ProcFsInodeType::RegularFile,
        }
    }
    
    /// Add a child inode
    pub fn add_child(&self, name: String, inode: Arc<dyn InodeOps>) {
        self.children.lock().insert(name, inode);
    }
}

impl InodeOps for ProcFsInode {
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(self.attr.lock().clone())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        // Check if it's a numeric PID
        if let Ok(pid) = name.parse::<usize>() {
            return ProcInfoInode::create_for_pid(pid);
        }
        
        // Check standard entries
        match name {
            "self" => {
                // Return symlink to current process
                let current_pid = crate::process::myproc().ok_or(VfsError::NotFound)?;
                let target = format!("{}", current_pid);
                return Ok(Arc::new(ProcFsInode::new_symlink(0, &target)));
            }
            "sys" => {
                return sys_info::create_root();
            }
            "meminfo" => {
                return sys_info::create_meminfo();
            }
            "stat" => {
                return sys_info::create_stat();
            }
            "version" => {
                return sys_info::create_version();
            }
            "uptime" => {
                return sys_info::create_uptime();
            }
            "loadavg" => {
                return sys_info::create_loadavg();
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
        
        // Add process directories
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let mut pids: Vec<usize> = proc_table.iter().map(|proc| proc.pid as usize).collect();
        drop(proc_table);
        pids.sort();
        
        let start_idx = if offset > 2 { offset - 2 } else { 0 };
        for pid in pids.iter().skip(start_idx) {
            entries.push(DirEntry {
                ino: (*pid as u64) + 10000, // Use PID + offset for inode number
                name: format!("{}", pid),
                file_type: FileType::Directory,
            });
        }
        
        // Add standard entries
        let standard_entries = ["self", "sys", "meminfo", "stat", "version", "uptime", "loadavg"];
        let std_start = if offset > 2 + pids.len() { offset - 2 - pids.len() } else { 0 };
        for (idx, entry) in standard_entries.iter().enumerate().skip(std_start) {
            entries.push(DirEntry {
                ino: 1000 + idx as u64,
                name: entry.to_string(),
                file_type: if *entry == "sys" { FileType::Directory } else { FileType::Regular },
            });
        }
        
        Ok(entries)
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        if self.attr.lock().mode.is_dir() {
            return Err(VfsError::IsDirectory);
        }
        
        let content_gen = self.content_gen.lock();
        if let Some(ref gen) = *content_gen {
            let content = gen();
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
}

/// Initialize and register ProcFS
pub fn init() {
    let procfs = Arc::new(ProcFsType);
    if let Err(e) = super::super::vfs().register_fs(procfs) {
        crate::println!("[procfs] Failed to register procfs: {:?}", e);
    } else {
        crate::println!("[procfs] Registered procfs filesystem");
    }
}

impl ProcFsInode {
    /// Create a symlink inode
    fn new_symlink(ino: u64, target: &str) -> Self {
        let target_string = target.to_string();
        Self {
            attr: Mutex::new(FileAttr {
                ino,
                mode: FileMode(FileMode::S_IFLNK | 0o777),
                nlink: 1,
                size: target.len() as u64,
                ..Default::default()
            }),
            children: Mutex::new(BTreeMap::new()),
            content_gen: Mutex::new(Some(Box::new(move || target_string.clone()))),
            inode_type: ProcFsInodeType::Symlink,
        }
    }
    
    /// Read symlink target
    pub fn readlink_impl(&self) -> VfsResult<String> {
        if self.attr.lock().mode.file_type() != FileType::Symlink {
            return Err(VfsError::InvalidOperation);
        }
        
        let content_gen = self.content_gen.lock();
        if let Some(ref gen) = *content_gen {
            Ok(gen())
        } else {
            Err(VfsError::InvalidOperation)
        }
    }
}

