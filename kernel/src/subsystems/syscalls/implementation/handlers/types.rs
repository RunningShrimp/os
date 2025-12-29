//! Type definitions for syscall handlers

use crate::error::KernelError;

/// Result type for syscall handlers
pub type Result<T> = core::result::Result<T, KernelError>;
