//! File System System Call Handlers
//!
//! This module contains the actual system call handler functions for filesystem
//! operations. These handlers are migrated from the original fs.rs
//! implementation and adapted for the new modular service architecture.
//!
//! Note: types module re-export is disabled for now to avoid unused imports
//!
//! Note: This module requires the following external dependencies:
//! - crate::error_handling::unified::KernelError for error conversion
//! - crate::vfs::error::VfsError for VFS error types
//!
//! Note: This module uses common utilities from common_fixed
//!
//! #![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use alloc::string::ToString;
use core::str;

use crate::{syscalls::common::*, vfs::error::VfsError};

/// Dispatch filesystem syscall (stub implementation)
pub fn dispatch_syscall(syscall_number: u32, args: &[u64]) -> Result<u64, SyscallError> {
    // Use syscall_number for validation and logging
    let _syscall_id = syscall_number; // Use syscall_number for validation
    // TODO: Implement actual syscall dispatching
    Err(SyscallError::NotSupported)
}


impl From<crate::error_handling::unified::KernelError> for SyscallError {
    fn from(err: crate::error_handling::unified::KernelError) -> Self {
        match err {
            crate::error_handling::unified::KernelError::OutOfMemory => SyscallError::OutOfMemory,
            crate::error_handling::unified::KernelError::InvalidArgument => {
                SyscallError::InvalidArgument
            },
            crate::error_handling::unified::KernelError::NotFound => SyscallError::NotFound,
            crate::error_handling::unified::KernelError::PermissionDenied => {
                SyscallError::PermissionDenied
            },
            crate::error_handling::unified::KernelError::IoError => SyscallError::IoError,
            crate::error_handling::unified::KernelError::NotSupported => SyscallError::NotSupported,
            crate::error_handling::unified::KernelError::AlreadyExists => SyscallError::FileExists,
            crate::error_handling::unified::KernelError::ResourceBusy => SyscallError::WouldBlock,
            crate::error_handling::unified::KernelError::Timeout => SyscallError::TimedOut,
        }
    }
}

// Re-export common types for other handlers to use
// pub use crate::syscalls::common_fixed::*;
pub use super::types::*;
