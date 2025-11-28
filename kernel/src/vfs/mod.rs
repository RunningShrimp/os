//! Virtual File System (VFS) Layer
//! 
//! Provides a unified interface for different file systems:
//! - Mount/unmount operations
//! - Path resolution with dentry cache
//! - File system type registration
//! - Inode and dentry abstraction

extern crate alloc;

use alloc::{string::{String, ToString}, vec::Vec, collections::BTreeMap, sync::Arc};

use crate::sync::Mutex;

// Re-export modules
pub mod error;
pub mod types;
pub mod dir;
pub mod fs;
pub mod file;
pub mod dentry;
pub mod mount;
pub mod ramfs;

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

// ============================================================================
// VFS Core
// ============================================================================

/// Global VFS state
struct Vfs {
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
            self.dentry_cache.lock().insert(String::from("/"), root_dentry);
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
            parent.remove_child(&name);
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
            parent.remove_child(&name);
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

    /// Create a symbolic link
    pub fn symlink(&self, path: &str, target: &str) -> VfsResult<()> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_dentry = self.lookup_path(&parent_path)?;
        let parent_inode = parent_dentry.lock().inode.clone();
        
        parent_inode.symlink(&name, target)?;
        Ok(())
    }
    
    /// Read a symbolic link
    pub fn readlink(&self, path: &str) -> VfsResult<String> {
        let dentry = self.lookup_path(path)?;
        dentry.lock().inode.readlink()
    }