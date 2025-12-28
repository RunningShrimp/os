//! Fast-path system call optimization module
//!
//! This module provides fast-path system call handling with:
//! - Per-CPU syscall caches
//! - Fast-path dispatch for common syscalls
//! - Syscall batching
//! - Adaptive optimization

pub mod hot_syscalls;

// Re-export hot syscalls for convenience
pub use hot_syscalls::{
    init_fast_path_registry,
    can_use_fast_path,
    dispatch_fast_path,
    get_fast_path_call_count,
    syscall_numbers,
};

/// Initialize fast-path system call optimization
pub fn init() {
    hot_syscalls::init_fast_path_registry();
}

/// Check if a syscall can use fast-path
pub fn can_use(syscall_num: u32) -> bool {
    hot_syscalls::can_use_fast_path(syscall_num)
}

/// Dispatch a syscall through fast-path
pub fn dispatch(syscall_num: u32, args: &[u64]) -> Option<crate::subsystems::syscalls::common::SyscallResult> {
    hot_syscalls::dispatch_fast_path(syscall_num, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_path_initialization() {
        init();
        assert!(can_use(syscall_numbers::SYS_GETPID));
    }
    
    #[test]
    fn test_fast_path_dispatch() {
        init();
        
        let result = dispatch(syscall_numbers::SYS_GETPID, &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }
}