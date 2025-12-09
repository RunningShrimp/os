//! Filesystem System Call Module
//!
//! This module provides filesystem-related system call services, including:
//! - Directory operations (chdir, getcwd, mkdir, rmdir)
//! - File operations (stat, lstat, access, unlink, rename)
//! - Link operations (link, symlink, readlink)
//! - Permission operations (chmod, chown, umask)
//! - Mount operations (mount, umount, pivot_root)
//!
//! The module adopts a layered architecture design, integrating with the system call dispatcher
//! through service interfaces.

pub mod handlers;
pub mod service;
pub mod types;

// Re-export main interfaces
pub use service::FilesystemService;
pub use types::*;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::syscalls::services::SyscallService;

/// Create filesystem syscall service instance
///
/// Creates and returns an instance of the filesystem syscall service.
///
/// # Returns
///
/// * `Box<dyn SyscallService>` - Filesystem syscall service instance
pub fn create_fs_service() -> Box<dyn SyscallService> {
    Box::new(FilesystemService::new())
}

/// Module initialization function
///
/// Initializes the filesystem module and registers necessary syscall handlers.
///
/// # Returns
///
/// * `Result<(), crate::error_handling::unified::KernelError>` - Initialization result
pub fn initialize_fs_module() -> Result<(), crate::error_handling::unified::KernelError> {
    crate::println!("[fs] Initializing filesystem module");

    // Ensure VFS is initialized
    if !crate::vfs::is_root_mounted() {
        crate::println!("[fs] Warning: Root filesystem not yet mounted");
        // This is not necessarily an error - root might be mounted later
    }

    Ok(())
}

/// Legacy dispatch function for backward compatibility
///
/// This function maintains backward compatibility with the old dispatch mechanism
/// during the transition to the new service architecture.
///
/// # Parameters
///
/// * `syscall_id` - System call number
/// * `args` - System call arguments
///
/// # Returns
///
/// * `Result<u64, crate::syscalls::common::SyscallError>` - Syscall result
pub fn dispatch(syscall_id: u32, args: &[u64]) -> Result<u64, crate::syscalls::common::SyscallError> {
    // For now, delegate to handlers directly
    // TODO: This should eventually be removed in favor of service-based dispatch
    use handlers::*;
    match syscall_id {
        0x7000 => handle_chdir(args),
        0x7001 => handle_fchdir(args),
        0x7002 => handle_getcwd(args),
        0x7003 => handle_mkdir(args),
        0x7004 => handle_rmdir(args),
        0x7005 => handle_unlink(args),
        0x7006 => handle_rename(args),
        0x7007 => handle_link(args),
        0x7008 => handle_symlink(args),
        _ => Err(crate::error_handling::unified::KernelError::SyscallNotSupported),
    }
}

// Helper functions for common filesystem operations

/// Check if a path represents a directory
pub fn is_directory_path(path: &str) -> bool {
    // Basic check - in full implementation would query VFS
    !path.contains('.') || path.ends_with('/')
}

/// Mount a filesystem
/// TODO: Implement proper filesystem mounting
pub fn mount(_source: &str, _target: &str, _fstype: Option<&str>, _flags: u32) -> Result<(), crate::syscalls::common::SyscallError> {
    // Placeholder implementation - always succeed for now
    Ok(())
}

/// Check if a path is absolute
pub fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/')
}

/// Join two path components
pub fn join_paths(base: &str, component: &str) -> String {
    if base.ends_with('/') {
        format!("{}{}", base, component)
    } else {
        format!("{}/{}", base, component)
    }
}

/// Clean path by removing redundant separators and . components
pub fn clean_path(path: &str) -> String {
    let service = FilesystemService::new();
    service.normalize_path(path, None).unwrap_or_else(|_| path.to_string())
}

// Filesystem module metadata
pub const MODULE_NAME: &str = "filesystem";
pub const MODULE_VERSION: &str = "1.0.0";
pub const SUPPORTED_SYSCALL_COUNT: usize = 19; // Update as syscalls are added/removed