#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Memory system calls module
//!
//! This module provides memory-related system call implementations.

pub mod heap;
pub mod map;
pub mod service_impl;
pub mod service;

// Re-export service types for convenience
pub use service::{MemoryService, MemoryServiceStats, MemoryManager};

// Re-export map functions
pub use map::{mmap as sys_mmap, munmap as sys_munmap};

use crate::api::SyscallError;

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
        }
        // munmap syscall
        0x3002 => {
            if args.len() < 2 {
                return Err(SyscallError::InvalidArgument);
            }
            let addr = args[0] as usize;
            let length = args[1] as usize;

            // Call munmap implementation
            sys_munmap(addr, length).map(|_| 0)
        }
        // Unknown memory syscall
        _ => Err(SyscallError::NotImplemented),
    }
}
