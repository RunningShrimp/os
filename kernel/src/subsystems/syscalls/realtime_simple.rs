//! Real-time scheduling syscalls (simplified version)

use crate::syscalls::common::SyscallError;

pub type SyscallResult = Result<u64, SyscallError>;

/// Dispatch real-time scheduling syscalls (returns NotSupported)
pub fn dispatch(syscall_id: u32, _args: &[u64]) -> SyscallResult {
    match syscall_id {
        0xE000..=0xEFFF => Err(SyscallError::NotSupported),
        _ => Err(SyscallError::InvalidSyscall),
    }
}
