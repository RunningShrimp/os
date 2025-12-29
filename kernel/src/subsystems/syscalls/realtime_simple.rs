//! Real-time scheduling syscalls (simplified version)

use crate::subsystems::syscalls::common::SyscallError;

// Re-export SyscallResult from common to avoid duplication
pub use crate::subsystems::syscalls::common::SyscallResult;

/// Dispatch real-time scheduling syscalls (returns NotSupported)
pub fn dispatch(syscall_id: u32, _args: &[u64]) -> SyscallResult {
    match syscall_id {
        0xE000..=0xEFFF => Err(SyscallError::NotSupported),
        _ => Err(SyscallError::InvalidSyscall),
    }
}
