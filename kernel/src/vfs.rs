//! Virtual File System (VFS) Layer
//! 
//! Provides a unified interface for different file systems:
//! - Mount/unmount operations
//! - Path resolution with dentry cache
//! - File system type registration
//! - Inode and dentry abstraction

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::Mutex;

// ============================================================================
// Error Types
// ============================================================================

/// VFS error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    PermissionDenied,
    NotDirectory,
    IsDirectory,
    NotEmpty,
    Exists,
    NoSpace,
    InvalidPath,
    NotMounted,
    Busy,
    ReadOnly,
    IoError,
    NotSupported,
    InvalidOperation,
}

pub type VfsResult<T> = Result<T, VfsError>;

// ============================================================================
// File Types and Modes
// ============================================================================

/// File type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Symlink,
}

/// File mode/permissions
#[derive(Debug, Clone, Copy, Default)]
pub struct FileMode(pub u32);

impl FileMode {
    pub const S_IFMT: u32   = 0o170000;  // Type mask
    pub const S_IFREG: u32  = 0o100000;  // Regular file
    pub const S_IFDIR: u32  = 0o040000;  // Directory
    pub const S_IFCHR: u32  = 0o020000;  // Character device
    pub const S_IFBLK: u32  = 0o060000;  // Block device
    pub const S_IFIFO: u32  = 0o010000;  // FIFO
    pub const S_IFSOCK: u32 = 0o140000;  // Socket
    pub const S_IFLNK: u32  = 0o120000;  // Symbolic link
    
    pub const S_ISUID: u32  = 0o4000;   // Set UID
    pub const S_ISGID: u32  = 0o2000;   // Set GID
    pub const S_ISVTX: u32  = 0o1000;   // Sticky bit
    
    pub const S_IRWXU: u32  = 0o700;    // User RWX
    pub const S_IRUSR: u32  = 0o400;    // User read
    pub const S_IWUSR: u32  = 0o200;    // User write
    pub const S_IXUSR: u32  = 0o100;    // User execute
    
    pub const S_IRWXG: u32  = 0o070;    // Group RWX
    pub const S_IRGRP: u32  = 0o040;    // Group read
    pub const S_IWGRP: u32  = 0o020;    // Group write
    pub const S_IXGRP: u32  = 0o010;    // Group execute
    
    pub const S_IRWXO: u32  = 0o007;    // Other RWX
    pub const S_IROTH: u32  = 0o004;    // Other read
    pub const S_IWOTH: u32  = 0o002;    // Other write
    pub const S_IXOTH: u32  = 0o001;    // Other execute
    
    pub fn new(mode: u32) -> Self {
        Self(mode)
    }
    
    pub fn file_type(&self) -> FileType {
        match self.0 & Self::S_IFMT {
            Self::S_IFREG => FileType::Regular,
            Self::S_IFDIR => FileType::Directory,
            Self::S_IFCHR => FileType::CharDevice,
            Self::S_IFBLK => FileType::BlockDevice,
            Self::S_IFIFO => FileType::Fifo,
            Self::S_IFSOCK => FileType::Socket,
            Self::S_IFLNK => FileType::Symlink,
            _ => FileType::Regular,
        }
    }
    
    pub fn is_dir(&self) -> bool {
        self.0 & Self::S_IFMT == Self::S_IFDIR
    }
    
    pub fn is_regular(&self) -> bool {
        self.0 & Self::S_IFMT == Self::S_IFREG
    }
    
    pub fn permissions(&self) -> u32 {
        self.0 & 0o777
    }
}

// ============================================================================
// File Attributes
// ============================================================================

/// File attributes (stat structure equivalent)
#[derive(Debug, Clone, Default)]
pub struct FileAttr {
    pub ino: u64,           // Inode number
    pub mode: FileMode,     // Mode and permissions
    pub nlink: u32,         // Number of hard links
    pub uid: u32,           // Owner user ID
    pub gid: u32,           // Owner group ID
    pub size: u64,          // Size in bytes
    pub blksize: u32,       // Block size
    pub blocks: u64,        // Number of 512B blocks
    pub atime: u64,         // Access time
    pub mtime: u64,         // Modification time
    pub ctime: u64,         // Change time
    pub rdev: u64,          // Device ID (for device files)
}

// ============================================================================
// Inode Operations Trait
// ============================================================================

/// Operations that can be performed on an inode
pub trait InodeOps: Send + Sync {
    /// Get file attributes
    fn getattr(&self) -> VfsResult<FileAttr>;
    
    /// Set file attributes
    fn setattr(&self, attr: &FileAttr) -> VfsResult<()> {
        let _ = attr;
        Err(VfsError::NotSupported)
    }
    
    /// Lookup a name in a directory
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = name;
        Err(VfsError::NotDirectory)
    }
    
    /// Create a file in a directory
    fn create(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, mode);
        Err(VfsError::NotDirectory)
    }
    
    /// Create a directory
    fn mkdir(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, mode);
        Err(VfsError::NotDirectory)
    }
    
    /// Remove a file
    fn unlink(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotDirectory)
    }

    /// Create a hard link
    fn link(&self, name: &str, inode: Arc<dyn InodeOps>) -> VfsResult<()> {
        let _ = (name, inode);
        Err(VfsError::NotSupported)
    }
    
    /// Check if directory is empty
    fn is_empty(&self) -> VfsResult<bool> {
        Err(VfsError::NotSupported)
    }
    
    /// Remove a directory
    fn rmdir(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotDirectory)
    }
    
    /// Rename
    fn rename(&self, old_name: &str, new_dir: &dyn InodeOps, new_name: &str) -> VfsResult<()> {
        let _ = (old_name, new_dir, new_name);
        Err(VfsError::NotSupported)
    }
    
    
    
    /// Create a symbolic link
    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, target);
        Err(VfsError::NotSupported)
    }
    
    /// Read symbolic link target
    fn readlink(&self) -> VfsResult<String> {
        Err(VfsError::InvalidOperation)
    }
    
    /// Read directory entries
    fn readdir(&self, offset: usize) -> VfsResult<Vec<DirEntry>> {
        let _ = offset;
        Err(VfsError::NotDirectory)
    }
    
    /// Read data
    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let _ = (offset, buf);
        Err(VfsError::IsDirectory)
    }
    
    /// Write data
    fn write(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let _ = (offset, buf);
        Err(VfsError::IsDirectory)
    }
    
    /// Truncate file
    fn truncate(&self, size: u64) -> VfsResult<()> {
        let _ = size;
        Err(VfsError::NotSupported)
    }
    
    /// Sync file to storage
    fn sync(&self) -> VfsResult<()> {
        Ok(())
    }
}

// ============================================================================
// Directory Entry
// ============================================================================

/// Directory entry for readdir
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub ino: u64,
    pub file_type: FileType,
}

// ============================================================================
// File System Type
// ============================================================================

/// File system type trait
pub trait FileSystemType: Send + Sync {
    /// Get file system name
    fn name(&self) -> &str;
    
    /// Mount the file system
    fn mount(&self, device: Option<&str>, flags: u32) -> VfsResult<Arc<dyn SuperBlock>>;
}

/// Superblock trait representing a mounted file system
pub trait SuperBlock: Send + Sync {
    /// Get root inode
    fn root(&self) -> Arc<dyn InodeOps>;
    
    /// Get file system type name
    fn fs_type(&self) -> &str;
    
    /// Sync all data to storage
    fn sync(&self) -> VfsResult<()>;
    
    /// Get file system statistics
    fn statfs(&self) -> VfsResult<FsStats>;
    
    /// Unmount (cleanup)
    fn unmount(&self) -> VfsResult<()>;
}

/// File system statistics
#[derive(Debug, Clone, Default)]
pub struct FsStats {
    pub bsize: u64,     // Block size
    pub blocks: u64,    // Total blocks
    pub bfree: u64,     // Free blocks
    pub bavail: u64,    // Available blocks
    pub files: u64,     // Total inodes
    pub ffree: u64,     // Free inodes
    pub namelen: u64,   // Max name length
}

// ============================================================================
// Dentry Cache
// ============================================================================

/// Directory entry cache
struct Dentry {
    name: String,
    parent: Option<Arc<Mutex<Dentry>>>,
    inode: Arc<dyn InodeOps>,
    mount: Option<Arc<Mount>>,
    children: BTreeMap<String, Arc<Mutex<Dentry>>>,
    ref_count: AtomicUsize,
}

impl Dentry {
    fn new(name: String, inode: Arc<dyn InodeOps>, parent: Option<Arc<Mutex<Dentry>>>) -> Self {
        Self {
            name,
            parent,
            inode,
            mount: None,
            children: BTreeMap::new(),
            ref_count: AtomicUsize::new(1),
        }
    }
}

// ============================================================================
// Mount Point
// ============================================================================

/// Mount point information
struct Mount {
    /// Mount point path
    path: String,
    /// Mounted superblock
    superblock: Arc<dyn SuperBlock>,
    /// Parent mount (if any)
    parent: Option<Arc<Mount>>,
    /// Mount flags
    flags: u32,
}

// ============================================================================
// VFS Core
// ============================================================================

/// Global VFS state
pub struct Vfs {
    /// Registered file system types
    fs_types: Mutex<BTreeMap<String, Arc<dyn FileSystemType>>>,
    /// Mount points
    mounts: Mutex<Vec<Arc<Mount>>>,
    /// Root dentry
    root: Mutex<Option<Arc<Mutex<Dentry>>>>,
    /// Dentry cache
    dentry_cache: Mutex<BTreeMap<String, Arc<Mutex<Dentry>>>>,
}

impl Vfs {
    /// Create a new VFS
    pub const fn new() -> Self {
        Self {
            fs_types: Mutex::new(BTreeMap::new()),
            mounts: Mutex::new(Vec::new()),
            root: Mutex::new(None),
            dentry_cache: Mutex::new(BTreeMap::new()),
        }
    }
    
    /// Register a file system type
    pub fn register_fs(&self, fs: Arc<dyn FileSystemType>) -> VfsResult<()> {
        let name = fs.name().to_string();
        let mut types = self.fs_types.lock();
        
        if types.contains_key(&name) {
            return Err(VfsError::Exists);
        }
        
        types.insert(name, fs);
        Ok(())
    }
    
    /// Unregister a file system type
    pub fn unregister_fs(&self, name: &str) -> VfsResult<()> {
        let mut types = self.fs_types.lock();
        types.remove(name).ok_or(VfsError::NotFound)?;
        Ok(())
    }
    
    /// Mount a file system
    pub fn mount(
        &self,
        fs_type: &str,
        mount_point: &str,
        device: Option<&str>,
        flags: u32,
    ) -> VfsResult<()> {
        // Find file system type
        let fs = {
            let types = self.fs_types.lock();
            types.get(fs_type).cloned().ok_or(VfsError::NotFound)?
        };
        
        // Mount the file system
        let superblock = fs.mount(device, flags)?;
        
        // Create mount structure
        let mount = Arc::new(Mount {
            path: mount_point.to_string(),
            superblock: superblock.clone(),
            parent: None,
            flags,
        });
        
        // Special case: mounting root
        if mount_point == "/" {
            let root_inode = superblock.root();
            let root_dentry = Arc::new(Mutex::new(Dentry::new(
                String::from("/"),
                root_inode,
                None,
            )));
            
            *self.root.lock() = Some(root_dentry.clone());
            self.dentry_cache.lock().insert(String::from("/"), root_dentry);
        } else {
            // Non-root mount: resolve mount point and attach
            let dentry = self.lookup_path(mount_point)?;
            let mut d = dentry.lock();
            d.mount = Some(mount.clone());
        }
        
        self.mounts.lock().push(mount);
        Ok(())
    }
    
    /// Unmount a file system
    pub fn unmount(&self, path: &str) -> VfsResult<()> {
        let mut mounts = self.mounts.lock();
        
        // Find and remove the mount
        let pos = mounts.iter()
            .position(|m| m.path == path)
            .ok_or(VfsError::NotMounted)?;
        
        let mount = mounts.remove(pos);
        
        // Unmount the superblock
        mount.superblock.unmount()?;
        
        // Remove from dentry cache if not root
        if path != "/" {
            if let Ok(dentry) = self.lookup_path(path) {
                dentry.lock().mount = None;
            }
        }
        
        Ok(())
    }
    
    /// Lookup a path and return the dentry
    pub fn lookup_path(&self, path: &str) -> VfsResult<Arc<Mutex<Dentry>>> {
        if path.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        
        // Check cache first
        {
            let cache = self.dentry_cache.lock();
            if let Some(dentry) = cache.get(path) {
                return Ok(dentry.clone());
            }
        }
        
        // Start from root
        let root = self.root.lock().clone().ok_or(VfsError::NotMounted)?;
        
        if path == "/" {
            return Ok(root);
        }
        
        // Parse and resolve each component
        let mut current = root;
        let components: Vec<&str> = path.split('/')
            .filter(|s| !s.is_empty())
            .collect();
        
        for (i, component) in components.iter().enumerate() {
            current = self.lookup_child(&current, component)?;
            
            // Check for mount points
            {
                let d = current.lock();
                if let Some(ref mount) = d.mount {
                    // Cross into mounted file system
                    let root_inode = mount.superblock.root();
                    drop(d);
                    current = Arc::new(Mutex::new(Dentry::new(
                        component.to_string(),
                        root_inode,
                        Some(current.clone()),
                    )));
                }
            }
            
            // Cache intermediate dentries
            if i == components.len() - 1 {
                self.dentry_cache.lock().insert(path.to_string(), current.clone());
            }
        }
        
        Ok(current)
    }
    
    /// Lookup a child in a directory dentry
    fn lookup_child(&self, parent: &Arc<Mutex<Dentry>>, name: &str) -> VfsResult<Arc<Mutex<Dentry>>> {
        let mut p = parent.lock();
        
        // Check children cache
        if let Some(child) = p.children.get(name) {
            return Ok(child.clone());
        }
        
        // Lookup in inode
        let child_inode = p.inode.lookup(name)?;
        
        // Create new dentry
        let child_dentry = Arc::new(Mutex::new(Dentry::new(
            name.to_string(),
            child_inode,
            Some(parent.clone()),
        )));
        
        p.children.insert(name.to_string(), child_dentry.clone());
        
        Ok(child_dentry)
    }
    
    /// Open a file by path
    pub fn open(&self, path: &str, flags: u32) -> VfsResult<VfsFile> {
        // 解析路径
        let dentry = self.lookup_path(path)?;
        let inode = dentry.lock().inode.clone();
        
        // 创建VfsFile实例
        let vfs_file = VfsFile::new(inode, flags);
        
        Ok(vfs_file)
    }
    
    /// Create a file
    pub fn create(&self, path: &str, mode: FileMode) -> VfsResult<VfsFile> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        let parent_inode = parent_dentry.lock().inode.clone();
        
        let inode = parent_inode.create(&name, mode)?;
        
        Ok(VfsFile {
            inode,
            offset: 0,
            flags: 0,
        })
    }
    
    /// Create a directory
    pub fn mkdir(&self, path: &str, mode: FileMode) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        let parent_inode = parent_dentry.lock().inode.clone();
        
        parent_inode.mkdir(&name, mode)?;
        Ok(())
    }
    
    /// Remove a file
    pub fn unlink(&self, path: &str) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        
        // Remove from dentry cache
        {
            let mut parent = parent_dentry.lock();
            parent.children.remove(&name);
            parent.inode.unlink(&name)?;
        }
        
        self.dentry_cache.lock().remove(path);
        Ok(())
    }
    
    /// Remove a directory
    pub fn rmdir(&self, path: &str) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        
        {
            let mut parent = parent_dentry.lock();
            parent.children.remove(&name);
            parent.inode.rmdir(&name)?;
        }
        
        self.dentry_cache.lock().remove(path);
        Ok(())
    }

    /// Create a hard link
    pub fn link(&self, old_path: &str, new_path: &str) -> VfsResult<()> {
        let old_dentry = self.lookup_path(old_path)?;
        let old_inode = old_dentry.lock().inode.clone();
        
        let (parent_path, name) = self.split_path(new_path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        let parent_inode = parent_dentry.lock().inode.clone();
        
        parent_inode.link(&name, old_inode)?;
        Ok(())
    }
    
    /// Get file attributes
    pub fn stat(&self, path: &str) -> VfsResult<FileAttr> {
        let dentry = self.lookup_path(path)?;
        dentry.lock().inode.getattr()
    }
    
    /// Read directory entries
    pub fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let dentry = self.lookup_path(path)?;
        dentry.lock().inode.readdir(0)
    }
    
    /// Split path into parent and name
    fn split_path(&self, path: &str) -> VfsResult<(String, String)> {
        if path.is_empty() || path == "/" {
            return Err(VfsError::InvalidPath);
        }
        
        let path = path.trim_end_matches('/');
        
        if let Some(pos) = path.rfind('/') {
            let parent = if pos == 0 { "/" } else { &path[..pos] };
            let name = &path[pos + 1..];
            Ok((parent.to_string(), name.to_string()))
        } else {
            Ok(("/".to_string(), path.to_string()))
        }
    }
    
    /// Sync all file systems
    pub fn sync_all(&self) -> VfsResult<()> {
        let mounts = self.mounts.lock();
        for mount in mounts.iter() {
            mount.superblock.sync()?;
        }
        Ok(())
    }
}

// ============================================================================
// VFS File Handle
// ============================================================================

/// Open file handle
pub struct VfsFile {
    inode: Arc<dyn InodeOps>,
    offset: u64,
    flags: u32,
}

impl VfsFile {
    pub fn new(inode: Arc<dyn InodeOps>, flags: u32) -> Self {
        Self {
            inode,
            offset: 0,
            flags,
        }
    }
    
    /// Read from file
    pub fn read(&mut self, addr: usize, len: usize) -> Result<usize, ()> {
        // Create a buffer from the address and length
        let buf = unsafe {
            core::slice::from_raw_parts_mut(addr as *mut u8, len)
        };
        
        match self.inode.read(self.offset, buf) {
            Ok(n) => {
                self.offset += n as u64;
                Ok(n)
            }
            Err(_) => Err(()),
        }
    }
    
    /// Write to file
    pub fn write(&mut self, addr: usize, len: usize) -> Result<usize, ()> {
        // Create a buffer from the address and length
        let buf = unsafe {
            core::slice::from_raw_parts(addr as *const u8, len)
        };
        
        match self.inode.write(self.offset, &buf) {
            Ok(n) => {
                self.offset += n as u64;
                Ok(n)
            }
            Err(_) => Err(()),
        }
    }
    
    /// Seek to position
    pub fn seek(&mut self, offset: usize) -> isize {
        // Simple implementation that just sets the offset directly
        self.offset = offset as u64;
        offset as isize
    }

    /// Truncate file
    pub fn truncate(&self, size: u64) -> VfsResult<()> {
        let attr = self.inode.getattr()?;
        let mut new_attr = attr;
        new_attr.size = size;
        self.inode.setattr(&new_attr)
    }
    
    /// Get file attributes
    pub fn stat(&self) -> VfsResult<FileAttr> {
        self.inode.getattr()
    }

    /// Set file attributes
    pub fn set_attr(&self, attr: &FileAttr) -> VfsResult<()> {
        self.inode.setattr(attr)
    }
}

/// Seek whence
#[derive(Debug, Clone, Copy)]
pub enum SeekWhence {
    Set,  // Absolute position
    Cur,  // Relative to current
    End,  // Relative to end
}

// ============================================================================
// Global VFS Instance
// ============================================================================

/// Global VFS instance
static VFS: Vfs = Vfs::new();

/// Get the global VFS
pub fn vfs() -> &'static Vfs {
    &VFS
}

/// Mount a file system (convenience function)
pub fn mount(fs_type: &str, path: &str, device: Option<&str>, flags: u32) -> VfsResult<()> {
    VFS.mount(fs_type, path, device, flags)
}

/// Unmount a file system
pub fn unmount(path: &str) -> VfsResult<()> {
    VFS.unmount(path)
}

/// Open a file
pub fn open(path: &str, flags: u32) -> VfsResult<VfsFile> {
    VFS.open(path, flags)
}

/// Get file stats
pub fn stat(path: &str) -> VfsResult<FileAttr> {
    VFS.stat(path)
}

// ============================================================================
// RamFS Implementation (Simple in-memory file system)
// ============================================================================

/// Simple RAM file system for testing
pub mod ramfs {
    use super::*;
    
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
        vfs().register_fs(ramfs).expect("Failed to register ramfs");
    }
}
