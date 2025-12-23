//! Signal system calls
//!
//! This module provides signal-related system calls.

use nos_api::Result;
use crate::SyscallDispatcher;

/// Register signal system call handlers
pub fn register_handlers(_dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // TODO: Implement signal system calls
    Ok(())
}