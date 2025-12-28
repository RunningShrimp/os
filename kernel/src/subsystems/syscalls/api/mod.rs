#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! System Call API Module
//!
//! Provides type definitions for system call results

use nos_api::Result;

/// System call error type
#[derive(Debug, Clone)]
pub enum SyscallError {
    InvalidArgument,
    PermissionDenied,
    NotFound,
    IoError,
}

/// System call result type
pub type SyscallResult<T = isize> = nos_api::Result<T>;

/// Error module
pub mod error {
    pub use super::SyscallError as Error;
    pub type Result<T = isize> = super::SyscallResult<T>;
}
