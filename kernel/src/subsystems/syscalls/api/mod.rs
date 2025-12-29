//! System Call API Module
//!
//! Provides type definitions for system call results

use nos_api::Result;

pub mod syscall_result;
pub mod syscall_id;

/// System call error type
#[derive(Debug, Clone)]
pub enum SyscallError {
    InvalidArgument,
    PermissionDenied,
    NotFound,
    IoError,
}

/// System call result type - re-exported from nos_api
/// Note: We use nos_api's SyscallResult (an enum) rather than a type alias
pub use nos_api::syscall::types::SyscallResult;

/// Error module
pub mod error {
    pub use super::SyscallError as Error;
    pub use nos_api::syscall::types::SyscallResult as Result;
}

pub use syscall_result::*;
