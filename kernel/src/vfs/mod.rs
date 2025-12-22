//! Virtual File System (VFS) module
//!
//! Provides unified interface for filesystem operations and mount management.

extern crate alloc;

pub mod fs;
pub mod devices;
pub mod kernel;
pub mod error;
pub mod mount;
pub mod dentry;
pub mod dir;
pub mod file;
pub mod types;
pub mod ramfs;
pub mod tmpfs;
pub mod ext4;
pub mod procfs;
pub mod sysfs;

pub use fs::*;
pub use types::*;
pub use error::*;
pub use mount::*;
pub use dentry::*;
pub use dir::*;
pub use file::*;

/// Get the global VFS manager instance
/// 
/// This function provides access to the VFS manager from within the vfs module.
/// It delegates to subsystems::fs::vfs().
pub fn vfs() -> &'static alloc::sync::Arc<crate::subsystems::fs::VfsManager> {
    crate::subsystems::fs::vfs()
}

