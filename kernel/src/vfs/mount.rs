//! Mount point information for VFS
extern crate alloc;

use crate::prelude::*;

use super::SuperBlock;
use crate::vfs_interface::{Mount as MountTrait, FileSystemType};

/// Mount point information
pub struct Mount {
    /// Mount point path
    pub path: String,
    /// Mounted superblock
    pub superblock: Arc<dyn SuperBlock>,
    /// Mount flags
    pub flags: u32,
}

impl Mount {
    pub fn new(path: String, superblock: Arc<dyn SuperBlock>, flags: u32) -> Self {
        Self { path, superblock, flags }
    }
}

impl MountTrait for Mount {
    /// Get the mount point path
    fn mount_point(&self) -> &str {
        &self.path
    }

    /// Get the filesystem type
    fn filesystem_type(&self) -> &dyn FileSystemType {
        // Since SuperBlock doesn't have a direct reference to FileSystemType,
        // we need to get it from somewhere else. For now, create a dummy implementation.
        // This might need to be adjusted based on how FileSystemType is stored in SuperBlock implementations.
        todo!("filesystem_type needs proper implementation")
    }

    /// Get the superblock
    fn superblock(&self) -> Arc<dyn SuperBlock> {
        self.superblock.clone()
    }

    /// Check if this is the root mount
    fn is_root(&self) -> bool {
        self.path == "/"
    }
}
