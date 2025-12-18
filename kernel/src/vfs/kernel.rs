//! Kernel information in /sys/kernel

extern crate alloc;
use alloc::{boxed::Box, string::{String, ToString}, sync::Arc};
use crate::vfs::{
    error::*,
    types::*,
    fs::InodeOps,
};
use super::fs::SysFsInode;

/// Create /sys/kernel root
pub fn create_root() -> VfsResult<Arc<dyn InodeOps>> {
    let root = Arc::new(SysFsInode::new_dir(2000));
    
    // Add common kernel files
    let mut children = root.children().lock();
    
    // /sys/kernel/version - kernel version
    children.insert("version".to_string(), Arc::new(SysFsInode::new_file(
        2001,
        Box::new(|| {
            format!(
                "NOS {}\n",
                env!("CARGO_PKG_VERSION")
            )
        })
    )));
    
    // /sys/kernel/hostname - system hostname
    children.insert("hostname".to_string(), Arc::new(SysFsInode::new_file(
        2002,
        Box::new(|| {
            "nos\n".to_string()
        })
    )));
    
    // /sys/kernel/domainname - system domain name
    children.insert("domainname".to_string(), Arc::new(SysFsInode::new_file(
        2003,
        Box::new(|| {
            "(none)\n".to_string()
        })
    )));
    
    // /sys/kernel/osrelease - kernel release version
    children.insert("osrelease".to_string(), Arc::new(SysFsInode::new_file(
        2004,
        Box::new(|| {
            format!("{}\n", env!("CARGO_PKG_VERSION"))
        })
    )));
    
    // /sys/kernel/ostype - operating system type
    children.insert("ostype".to_string(), Arc::new(SysFsInode::new_file(
        2005,
        Box::new(|| {
            "NOS\n".to_string()
        })
    )));
    
    drop(children);
    
    Ok(root)
}

/// Create /sys/module root
pub fn create_module_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(3000)))
}

