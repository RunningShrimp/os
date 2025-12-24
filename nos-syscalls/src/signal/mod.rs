//! Signal system calls
//!
//! This module provides signal-related system calls.

#[cfg(feature = "alloc")]
use nos_api::Result;

#[cfg(feature = "alloc")]
use crate::core::dispatcher::SyscallDispatcher;

/// Register signal system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers(_dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // TODO: Implement signal system calls
    Ok(())
}