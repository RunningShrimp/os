//! Filesystem System Call Module
//!
//! This module provides filesystem-related system call services, including:
//! - Directory operations (chdir, getcwd, mkdir, rmdir)
//! - File operations (stat, lstat, access, unlink, rename)
//! - Link operations (link, symlink, readlink)
//! - Permission operations (chmod, chown, umask)
//! - Mount operations (mount, umount, pivot_root)
//!
//! The module adopts a layered architecture design, integrating with the system
//! call dispatcher through service interfaces.

pub mod handlers;
pub mod service;
pub mod types;

// Re-export main interfaces
use alloc::{string::ToString, boxed::Box, string::String};

pub use service::FilesystemService;
pub use types::*;

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

/// Mount a filesystem
///
/// Mount a filesystem with the specified type, target, device and flags.
///
/// # Parameters
///
/// * `fs_type` - Type of filesystem to mount
/// * `target` - Target path where to mount
/// * `device` - Device or source to mount (optional)
/// * `flags` - Mount flags
///
/// # Returns
///
/// * `Result<(), KernelError>` - Mount result
pub fn mount(
    fs_type: &str,
    target: &str,
    device: Option<&str>,
    flags: u32,
) -> Result<(), crate::error_handling::unified::KernelError> {
    // For now, delegate to VFS mount operation
    crate::vfs::mount(fs_type, target, device, flags)
}

/// Module initialization function
///
/// Initializes the filesystem module and registers necessary syscall handlers.
///
/// # Returns
///
/// * `Result<(), crate::error_handling::unified::KernelError>` - Initialization
///   result
pub fn initialize_fs_module() -> Result<(), crate::error_handling::unified::KernelError> {
    // println removed for no_std compatibility

    // Ensure VFS is initialized
    if !crate::vfs::is_root_mounted() {
        // println removed for no_std compatibility
        // This is not necessarily an error - root might be mounted later
    }

    Ok(())
}

/// Legacy dispatch function for backward compatibility
///
/// This function maintains backward compatibility with the old dispatch
/// mechanism during the transition to the new service architecture.
///
/// # Parameters
///
/// * `syscall_id` - System call number
/// * `args` - System call arguments
///
/// # Returns
///
/// * `Result<u64, crate::syscalls::common::SyscallError>` - Syscall result
pub fn dispatch(
    syscall_id: u32,
    args: &[u64],
) -> Result<u64, crate::syscalls::common::SyscallError> {
    // For now, delegate to handlers directly
    // TODO: This should eventually be removed in favor of service-based dispatch
    handlers::dispatch_syscall(syscall_id, args)
}

// Helper functions for common filesystem operations

/// Check if a path represents a directory
pub fn is_directory_path(path: &str) -> bool {
    // Basic check - in full implementation would query VFS
    !path.contains('.') || path.ends_with('/')
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
    // TODO: Implement proper path cleaning
    // println removed for no_std compatibility
    path.to_string()
}

// Filesystem module metadata
pub const MODULE_NAME: &str = "filesystem";
pub const MODULE_VERSION: &str = "1.0.0";
pub const SUPPORTED_SYSCALL_COUNT: usize = 19; // Update as syscalls are added/removed
