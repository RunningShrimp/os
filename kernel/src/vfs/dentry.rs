//! Directory entry cache for VFS
extern crate alloc;
use alloc::{string::String, sync::Arc, collections::BTreeMap};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::Mutex;

use super::{
    fs::InodeOps,
    mount::Mount,
};

/// Directory entry cache
pub struct Dentry {
    name: String,
    parent: Option<Arc<Mutex<Dentry>>>,
    pub inode: Arc<dyn InodeOps>,
    mount: Option<Arc<Mount>>,
    children: BTreeMap<String, Arc<Mutex<Dentry>>>,
    ref_count: AtomicUsize,
}

impl Dentry {
    pub fn new(name: String, inode: Arc<dyn InodeOps>, parent: Option<Arc<Mutex<Dentry>>>) -> Self {
        Self {
            name,
            parent,
            inode,
            mount: None,
            children: BTreeMap::new(),
            ref_count: AtomicUsize::new(1),
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
