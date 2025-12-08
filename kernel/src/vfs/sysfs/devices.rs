//! Device and bus information in /sys

extern crate alloc;
use alloc::{string::String, sync::Arc};
use crate::vfs::{
    error::*,
    types::*,
    fs::InodeOps,
};
use super::fs::SysFsInode;

/// Create /sys/devices root
pub fn create_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1000)))
}

/// Create /sys/bus root
pub fn create_bus_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1001)))
}

/// Create /sys/class root
pub fn create_class_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1002)))
}

/// Create /sys/dev root
pub fn create_dev_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1003)))
}

/// Create /sys/firmware root
pub fn create_firmware_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1004)))
}

/// Create /sys/fs root
pub fn create_fs_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1005)))
}

/// Create /sys/power root
pub fn create_power_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(1006)))
}

