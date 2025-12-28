//! Mount point information for VFS
extern crate alloc;
use alloc::{string::String, sync::Arc};

use super::{fs::SuperBlock};

/// Mount point information
pub struct Mount {
    /// Mount point path
    pub path: String,
    /// Mounted superblock
    pub superblock: Arc<dyn SuperBlock>,
    /// Parent mount (if any)
    parent: Option<Arc<Mount>>,
    /// Mount flags
    pub flags: u32,
}

impl Mount {
    pub fn new(path: String, superblock: Arc<dyn SuperBlock>, flags: u32) -> Self {
        Self {
            path,
            superblock,
            parent: None,
            flags,
        }
    }
}
