//! Signal handling syscalls (simplified version)

use super::common::{SyscallError, SyscallResult};

/// Dispatch signal handling syscalls
pub fn dispatch(syscall_id: u32, _args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Signal operations - temporarily disabled
        0x5000..=0x5FFF => Err(SyscallError::NotSupported),
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Kill a process (stub implementation)
pub fn kill_process(_pid: usize, _signal: i32) {
    // Stub implementation - signal handling disabled
}
