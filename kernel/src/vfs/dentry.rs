//! Directory entry cache for VFS
extern crate alloc;

use crate::prelude::*;

use super::{InodeOps, Mount};
use crate::subsystems::sync::Mutex;

/// Directory entry cache
pub struct Dentry {
    pub inode: Arc<dyn InodeOps>,
    mount: Option<Arc<Mount>>,
    children: BTreeMap<String, Arc<Mutex<Dentry>>>,
}

impl Dentry {
    pub fn new(_name: String, inode: Arc<dyn InodeOps>, _parent: Option<Arc<Mutex<Dentry>>>) -> Self {
        Self {
            inode,
            mount: None,
            children: BTreeMap::new(),
        }
    }

    pub fn lookup_child(&self, name: &str) -> Option<Arc<Mutex<Dentry>>> {
        self.children.get(name).cloned()
    }

    pub fn add_child(&mut self, name: String, dentry: Arc<Mutex<Dentry>>) {
        self.children.insert(name, dentry);
    }

    pub fn remove_child(&mut self, name: &str) -> Option<Arc<Mutex<Dentry>>> {
        self.children.remove(name)
    }

    pub fn mount(&mut self, mount: Arc<Mount>) {
        self.mount = Some(mount);
    }

    pub fn unmount(&mut self) {
        self.mount = None;
    }

    pub fn has_mount(&self) -> bool {
        self.mount.is_some()
    }

    pub fn get_mount(&self) -> Option<Arc<Mount>> {
        self.mount.clone()
    }
}
