//! Memory system calls module
//!
//! This module provides memory-related system call implementations.

pub mod heap;
pub mod map;
pub mod service;
pub mod service_impl;

// Re-export service types for convenience
// Re-export map functions
pub use map::{mmap as sys_mmap, munmap as sys_munmap};
pub use service::{MemoryManager, MemoryService, MemoryServiceStats};

use crate::api::SyscallError;

/// brk 系统调用 - 设置堆的绝对边界
///
/// 参考: kernel/src/subsystems/mm/brk.rs
pub fn sys_brk(addr: usize) -> Result<isize, SyscallError> {
    crate::subsystems::mm::brk::sys_brk(addr)
}

/// sbrk 系统调用 - 增量调整堆大小
///
/// 参考: kernel/src/subsystems/mm/brk.rs
pub fn sys_sbrk(increment: isize) -> Result<isize, SyscallError> {
    crate::subsystems::mm::brk::sys_sbrk(increment)
}

/// Dispatch memory syscalls
///
/// # Arguments
/// * `syscall_num` - The system call number
/// * `args` - System call arguments
///
/// # Returns
/// * `Result<usize, SyscallError>` - System call result
pub fn dispatch(syscall_num: u32, args: &[u64]) -> Result<usize, SyscallError> {
    match syscall_num {
        // brk syscall (214) - 设置堆的绝对边界
        214 => {
            if args.len() < 1 {
                return Err(SyscallError::InvalidArgument);
            }
            let addr = args[0] as usize;
            sys_brk(addr).map(|v| v as usize)
        },
        // sbrk syscall (Obsolete) - 增量调整堆大小
        // 注意: sbrk 在现代系统中已过时，但为兼容性保留
        213 => {
            // sbrk 通常不是独立的系统调用号，这里作为示例
            // 实际上 Linux 中 brk 是系统调用，sbrk 是 libc 函数
            if args.len() < 1 {
                return Err(SyscallError::InvalidArgument);
            }
            let increment = args[0] as isize;
            sys_sbrk(increment).map(|v| v as usize)
        },
        // mmap syscall
        0x3001 => {
            if args.len() < 6 {
                return Err(SyscallError::InvalidArgument);
            }
            let addr = args[0] as usize;
            let length = args[1] as usize;
            let prot = args[2] as u32;
            let flags = args[3] as u32;
            let fd = args[4] as i32;
            let offset = args[5] as usize;

            // Call mmap implementation
            sys_mmap(addr, length, prot, flags, fd, offset).map(|ptr| ptr as usize)
        },
        // munmap syscall
        0x3002 => {
            if args.len() < 2 {
                return Err(SyscallError::InvalidArgument);
            }
            let addr = args[0] as usize;
            let length = args[1] as usize;

            // Call munmap implementation
            sys_munmap(addr, length).map(|_| 0)
        },
        // Unknown memory syscall
        _ => Err(SyscallError::NotImplemented),
    }
}
