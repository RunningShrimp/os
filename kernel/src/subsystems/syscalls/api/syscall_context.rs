//! System call context definition.
//!
//! This module defines the context structure that is passed to all system
//! call handlers.

use super::{SyscallId, SyscallResult};

/// System call context.
///
/// Contains information about the current system call being executed.
pub struct SyscallContext {
    /// The system call ID
    pub syscall_id: SyscallId,
    /// System call arguments
    pub args: [u64; 6],
    /// Process ID of the calling process
    pub pid: u64,
    /// Thread ID of the calling thread
    pub tid: u64,
}

impl SyscallContext {
    /// Create a new SyscallContext.
    pub fn new(syscall_id: SyscallId, args: [u64; 6], pid: u64, tid: u64) -> Self {
        Self {
            syscall_id,
            args,
            pid,
            tid,
        }
    }
    
    /// Get the nth argument.
    pub fn arg(&self, index: usize) -> u64 {
        if index < self.args.len() {
            self.args[index]
        } else {
            0
        }
    }
    
    /// Get the arguments as a slice.
    pub fn args(&self) -> &[u64; 6] {
        &self.args
    }
}