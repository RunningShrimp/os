//! Virtual File System (VFS) Layer
//! 
//! Provides a unified interface for different file systems:
//! - Mount/unmount operations
//! - Path resolution with dentry cache
//! - File system type registration
//! - Inode and dentry abstraction

extern crate alloc;

use alloc::{string::{String, ToString}, vec::Vec, collections::BTreeMap, sync::Arc};
use hashbrown::HashMap;
use core::hash::{Hash, Hasher};
use crate::compat::DefaultHasherBuilder;

use crate::sync::{Mutex, RwLock};

// Re-export modules
pub mod error;
pub mod types;
pub mod dir;
pub mod fs;
pub mod file;
pub mod dentry;
pub mod mount;
pub mod journal;
pub mod log_buffer;
pub mod ramfs;
pub mod ext4;
pub mod procfs;
pub mod sysfs;

// Re-export common types
pub use error::*;
pub use types::*;
pub use dir::DirEntry;
pub use fs::{FileSystemType, SuperBlock, InodeOps};
pub use file::VfsFile;

use self::{
    dentry::Dentry,
    mount::Mount,
};
use self::journal::{Journal, JournalOptions};
use self::log_buffer::LogBuffer;

// ============================================================================
// VFS Core
// ============================================================================

/// LRU cache entry for dentry cache
struct DentryCacheEntry {
    dentry: Arc<Mutex<Dentry>>,
    access_time: u64,
}

/// Global VFS state
pub struct Vfs {
    /// Registered file system types
    fs_types: Mutex<BTreeMap<String, Arc<dyn FileSystemType>>>,
    /// Mount points
    mounts: Mutex<Vec<Arc<Mount>>>,
    /// Root dentry
    root: Mutex<Option<Arc<Mutex<Dentry>>>>,
    /// Dentry cache with LRU eviction
    dentry_cache: Mutex<HashMap<String, DentryCacheEntry, DefaultHasherBuilder>>,
    /// Maximum cache size
    max_cache_size: usize,
    /// Path hash index for faster lookups
    path_hash_index: Mutex<HashMap<u64, String, DefaultHasherBuilder>>,
    /// 读多写少的路径保护
    path_guard: RwLock<()>,
    /// 简单日志接口（占位）
    journal: Journal,
    /// 内存日志缓冲
    logbuf: LogBuffer,
}

impl Vfs {
    /// Create a new VFS
    pub const fn new() -> Self {
        Self {
            fs_types: Mutex::new(BTreeMap::new()),
            mounts: Mutex::new(Vec::new()),
            root: Mutex::new(None),
            dentry_cache: Mutex::new(HashMap::with_hasher(DefaultHasherBuilder)),
            max_cache_size: 1024, // Maximum 1024 cached entries
            path_hash_index: Mutex::new(HashMap::with_hasher(DefaultHasherBuilder)),
            path_guard: RwLock::new(()),
            journal: Journal {
                opts: JournalOptions {
                    enabled: false,
                    sync_on_commit: false,
                },
            },
            logbuf: LogBuffer::with_capacity(128),
        }
    }
    
    /// Hash a path for indexing (simple implementation)
    fn hash_path(path: &str) -> u64 {
        // Simple hash function for path indexing
        let mut hash: u64 = 0;
        for (i, byte) in path.bytes().enumerate() {
            hash = hash.wrapping_add((byte as u64).wrapping_mul(i as u64 + 1));
        }
        hash
    }
    
    /// Evict least recently used entries from cache
    fn evict_lru(&self) {
        let mut cache = self.dentry_cache.lock();
        if cache.len() <= self.max_cache_size {
            return;
        }
        
        // Find entries to evict (remove oldest 10% or at least 10 entries)
        let to_remove = core::cmp::max(
            (cache.len() - self.max_cache_size).max(10),
            cache.len() / 10
        );
        
        // Collect entries sorted by access time
        let mut entries: Vec<(String, u64)> = cache.iter()
            .map(|(path, entry)| (path.clone(), entry.access_time))
            .collect();
        
        // Sort by access time (oldest first)
        entries.sort_by_key(|(_, time)| *time);
        
        // Remove oldest entries
        for (path, _) in entries.iter().take(to_remove) {
            cache.remove(path);
            // Also remove from hash index
            let hash = Self::hash_path(path);
            self.path_hash_index.lock().remove(&hash);
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
        let mount = Arc::new(Mount::new(
            mount_point.to_string(),
            superblock.clone(),
            flags,
        ));
        
        // Special case: mounting root
        if mount_point == "/" {
            let root_inode = superblock.root();
            let root_dentry = Arc::new(Mutex::new(Dentry::new(
                String::from("/"),
                root_inode,
                None,
            )));
            
            *self.root.lock() = Some(root_dentry.clone());
            self.dentry_cache.lock().insert(String::from("/"), DentryCacheEntry {
                dentry: root_dentry.clone(),
                access_time: crate::time::get_ticks(),
            });
        } else {
            // Non-root mount: resolve mount point and attach
            let dentry = self.lookup_path(mount_point)?;
            let mut d = dentry.lock();
            d.mount(mount.clone());
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
                dentry.lock().unmount();
            }
        }
        
        Ok(())
    }
    
    /// Check if root file system is mounted
    pub fn is_root_mounted(&self) -> bool {
        self.root.lock().is_some()
    }
    
    /// Verify root file system is accessible
    pub fn verify_root(&self) -> VfsResult<()> {
        if !self.is_root_mounted() {
            return Err(VfsError::NotMounted);
        }
        
        // Try to stat root directory to verify it's accessible
        self.stat("/")?;
        Ok(())
    }
    
    /// Lookup a path and return the dentry
    pub fn lookup_path(&self, path: &str) -> VfsResult<Arc<Mutex<Dentry>>> {
        if path.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        // 读锁保护的快路径
        {
            let _r = self.path_guard.read();
            let path_hash = Self::hash_path(path);
            let cache = self.dentry_cache.lock();
            if let Some(entry) = cache.get(path) {
                let dentry = entry.dentry.clone();
                drop(cache);
                self.path_hash_index.lock().insert(path_hash, path.to_string());
                return Ok(dentry);
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
        
        let _w = self.path_guard.write();
        let path_hash = Self::hash_path(path);
        for (i, component) in components.iter().enumerate() {
            current = self.lookup_child(&current, component)?;
            
            // Check for mount points
            {
                let d = current.lock();
                if let Some(ref mount) = d.get_mount() {
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
                // Evict LRU entries if cache is full
                self.evict_lru();
                
                // Add to cache with current access time
                let mut cache = self.dentry_cache.lock();
                cache.insert(path.to_string(), DentryCacheEntry {
                    dentry: current.clone(),
                    access_time: crate::time::get_ticks(),
                });
                
                // Update hash index
                self.path_hash_index.lock().insert(path_hash, path.to_string());
            }
        }
        
        Ok(current)
    }

    /// 仅在读锁下使用的快速缓存读取
    fn fast_lookup(&self, path: &str) -> Option<Arc<Mutex<Dentry>>> {
        let cache = self.dentry_cache.lock();
        cache.get(path).map(|e| e.dentry.clone())
    }
    
    /// Lookup a child in a directory dentry
    fn lookup_child(&self, parent: &Arc<Mutex<Dentry>>, name: &str) -> VfsResult<Arc<Mutex<Dentry>>> {
        let mut p = parent.lock();
        
        // Check children cache
        if let Some(child) = p.lookup_child(name) {
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
        
        p.add_child(name.to_string(), child_dentry.clone());
        
        Ok(child_dentry)
    }
    
    /// Lookup a path (alias for lookup_path)
    pub fn lookup(&self, path: &str) -> VfsResult<Arc<Mutex<Dentry>>> {
        self.lookup_path(path)
    }

    /// Read from a file
    pub fn read(&self, path: &str, buffer: &mut [u8], offset: u64) -> VfsResult<usize> {
        let dentry = self.lookup_path(path)?;
        let inode = dentry.lock().inode.clone();
        inode.read(offset, buffer)
    }

    /// Write to a file
    pub fn write(&self, path: &str, buffer: &[u8], offset: u64) -> VfsResult<usize> {
        let dentry = self.lookup_path(path)?;
        let inode = dentry.lock().inode.clone();
        inode.write(offset, buffer)
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

        // Generate inotify events for parent directory
        self.generate_inotify_events(&parent_path, crate::syscalls::glib::inotify_mask::IN_CREATE, 0, &name);

        Ok(VfsFile::new(inode, 0))
    }
    
    /// Create a directory
    pub fn mkdir(&self, path: &str, mode: FileMode) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        let parent_inode = parent_dentry.lock().inode.clone();

        parent_inode.mkdir(&name, mode)?;

        // Generate inotify events for parent directory
        self.generate_inotify_events(&parent_path, crate::syscalls::glib::inotify_mask::IN_CREATE | crate::syscalls::glib::inotify_mask::IN_ISDIR, 0, &name);
        let _ = self.journal.record("mkdir");
        self.logbuf.push(format!("mkdir {} {}", parent_path, name));

        Ok(())
    }
    
    /// Remove a file
    pub fn unlink(&self, path: &str) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;

        // Remove from dentry cache
        {
            let mut parent = parent_dentry.lock();
            parent.remove_child(&name);
            parent.inode.unlink(&name)?;
        }

        let _ = self.journal.record("unlink");
        self.logbuf.push(format!("unlink {} {}", parent_path, name));
        self.dentry_cache.lock().remove(path);

        // Generate inotify events for parent directory
        self.generate_inotify_events(&parent_path, crate::syscalls::glib::inotify_mask::IN_DELETE, 0, &name);

        Ok(())
    }
    
    /// Remove a directory
    pub fn rmdir(&self, path: &str) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;

        {
            let mut parent = parent_dentry.lock();
            parent.remove_child(&name);
            parent.inode.rmdir(&name)?;
        }

        self.dentry_cache.lock().remove(path);
        let _ = self.journal.record("rmdir");
        self.logbuf.push(format!("rmdir {} {}", parent_path, name));

        // Generate inotify events for parent directory
        self.generate_inotify_events(&parent_path, crate::syscalls::glib::inotify_mask::IN_DELETE | crate::syscalls::glib::inotify_mask::IN_ISDIR, 0, &name);

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

        // 维护缓存与事件
        let full_new = if parent_path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", parent_path, name)
        };
        self.dentry_cache.lock().remove(&full_new);
        let _ = self.journal.record("link"); // 占位
        self.generate_inotify_events(
            &parent_path,
            crate::syscalls::glib::inotify_mask::IN_CREATE,
            0,
            &name,
        );
        Ok(())
    }
    
    /// Get file attributes
    pub fn stat(&self, path: &str) -> VfsResult<FileAttr> {
        let dentry = self.lookup_path(path)?;
        let guard = dentry.lock();
        // Call getattr while the lock is held and return an owned value
        let attr = guard.inode.getattr()?;
        Ok(attr)
    }
    
    /// Read directory entries
    pub fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let dentry = self.lookup_path(path)?;
        let guard = dentry.lock();
        let entries = guard.inode.readdir(0)?;
        Ok(entries)
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

    /// Create a symbolic link
    pub fn symlink(&self, path: &str, target: &str) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        let parent_inode = parent_dentry.lock().inode.clone();

        parent_inode.symlink(&name, target)?;

        // Generate inotify events for parent directory
        self.generate_inotify_events(&parent_path, crate::syscalls::glib::inotify_mask::IN_CREATE, 0, &name);
        let _ = self.journal.record("symlink");
        self.logbuf.push(format!("symlink {} -> {}", path, target));

        Ok(())
    }

    /// Generate inotify events for all watching instances
    /// Note: This is a simplified implementation. A full implementation would need
    /// to properly track inotify instances and generate events for matching watches.
    fn generate_inotify_events(&self, _path: &str, _mask: u32, _cookie: u32, _name: &str) {
        // TODO: Implement proper inotify event generation
        // This would require:
        // 1. Maintaining a global registry of inotify instances
        // 2. Checking which watches match the path
        // 3. Generating events for matching watches
        // 4. Handling event queue overflow
    }

    /// Read a symbolic link
    pub fn readlink(&self, path: &str) -> VfsResult<String> {
        let dentry = self.lookup_path(path)?;
        let guard = dentry.lock();
        let target = guard.inode.readlink()?;
        Ok(target)
    }
}

// ============================================================================
// Global VFS Instance
// ============================================================================

/// Global VFS instance
static VFS: Vfs = Vfs::new();

/// Get the global VFS instance
///
/// Returns a reference to the global Virtual File System instance.
/// This is the main entry point for all VFS operations.
///
/// # Example
///
/// ```
/// use kernel::vfs;
///
/// let vfs = vfs::vfs();
/// let file = vfs.open("/etc/passwd", 0)?;
/// ```
pub fn vfs() -> &'static Vfs {
    &VFS
}

/// Mount a file system (convenience function)
///
/// Mounts a file system of the specified type at the given mount point.
///
/// # Arguments
///
/// * `fs_type` - File system type name (e.g., "ramfs", "tmpfs", "ext4")
/// * `path` - Mount point path (e.g., "/", "/mnt")
/// * `device` - Optional device identifier (for block devices)
/// * `flags` - Mount flags (currently unused, reserved for future use)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(VfsError)` on failure (e.g., file system type not found, mount point invalid)
///
/// # Example
///
/// ```
/// use kernel::vfs;
///
/// // Mount ramfs at root
/// vfs::mount("ramfs", "/", None, 0)?;
///
/// // Mount ext4 from device
/// vfs::mount("ext4", "/mnt", Some("/dev/sda1"), 0)?;
/// ```
pub fn mount(fs_type: &str, path: &str, device: Option<&str>, flags: u32) -> VfsResult<()> {
    VFS.mount(fs_type, path, device, flags)
}

/// Unmount a file system
///
/// Unmounts the file system mounted at the specified path.
///
/// # Arguments
///
/// * `path` - Mount point path to unmount
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(VfsError::NotMounted)` if no file system is mounted at the path
///
/// # Example
///
/// ```
/// use kernel::vfs;
///
/// // Unmount /mnt
/// vfs::unmount("/mnt")?;
/// ```
pub fn unmount(path: &str) -> VfsResult<()> {
    VFS.unmount(path)
}

/// Open a file
///
/// Opens a file at the specified path with the given flags.
///
/// # Arguments
///
/// * `path` - File path (absolute or relative to current working directory)
/// * `flags` - Open flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, etc.)
///
/// # Returns
///
/// * `Ok(VfsFile)` on success
/// * `Err(VfsError)` on failure (e.g., file not found, permission denied)
///
/// # Example
///
/// ```
/// use kernel::vfs;
/// use kernel::posix::O_RDONLY;
///
/// let file = vfs::open("/etc/passwd", O_RDONLY as u32)?;
/// ```
pub fn open(path: &str, flags: u32) -> VfsResult<VfsFile> {
    VFS.open(path, flags)
}

/// Get file statistics
///
/// Retrieves file attributes (metadata) for the file at the specified path.
///
/// # Arguments
///
/// * `path` - File path
///
/// # Returns
///
/// * `Ok(FileAttr)` containing file attributes (size, mode, inode, etc.)
/// * `Err(VfsError)` on failure (e.g., file not found)
///
/// # Example
///
/// ```
/// use kernel::vfs;
///
/// let attr = vfs::stat("/etc/passwd")?;
/// println!("File size: {} bytes", attr.size);
/// ```
pub fn stat(path: &str) -> VfsResult<FileAttr> {
    VFS.stat(path)
}

/// Check if root file system is mounted
///
/// Returns `true` if a root file system has been successfully mounted,
/// `false` otherwise. This is useful for checking system initialization status.
///
/// # Returns
///
/// * `true` if root file system is mounted
/// * `false` otherwise
///
/// # Example
///
/// ```
/// use kernel::vfs;
///
/// if !vfs::is_root_mounted() {
///     panic!("Root file system not mounted!");
/// }
/// ```
pub fn is_root_mounted() -> bool {
    VFS.is_root_mounted()
}

/// Verify root file system is accessible
///
/// Performs a verification check to ensure the root file system is mounted
/// and accessible. This is typically called during system initialization.
///
/// # Returns
///
/// * `Ok(())` if root file system is accessible
/// * `Err(VfsError::NotMounted)` if root file system is not mounted
/// * `Err(VfsError)` if root file system is not accessible
///
/// # Example
///
/// ```
/// use kernel::vfs;
///
/// // Verify root file system during boot
/// vfs::verify_root()?;
/// ```
pub fn verify_root() -> VfsResult<()> {
    VFS.verify_root()
}

  