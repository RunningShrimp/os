//! Hot system call fast-path implementations
//!
//! This module provides optimized fast-path implementations for frequently
//! called system calls that can be handled without full dispatch overhead.
//! These implementations bypass normal validation and dispatch mechanisms
//! for maximum performance.

use crate::subsystems::syscalls::common::{SyscallError, SyscallResult};
use crate::process;
use crate::process::manager::PROC_TABLE;
use core::sync::atomic::{AtomicU64, Ordering};

/// System call numbers for fast-path syscalls
pub mod syscall_numbers {
    pub const SYS_GETPID: u32 = 0x1004;
    pub const SYS_GETPPID: u32 = 0x1005;
    pub const SYS_GETUID: u32 = 0x1007;
    pub const SYS_GETGID: u32 = 0x1009;
    pub const SYS_GETTID: u32 = 0x1008;
    pub const SYS_GETEUID: u32 = 0x1009;
    pub const SYS_GETEGID: u32 = 0x100A;
}

/// Fast-path implementation for getpid
/// Returns the process ID of the calling process
pub fn fast_getpid(_args: &[u64]) -> SyscallResult {
    // Direct access to current process PID without locking overhead
    if let Some(pid) = process::myproc() {
        Ok(pid as u64)
    } else {
        Err(SyscallError::NotFound)
    }
}

/// Fast-path implementation for getuid
/// Returns the real user ID of the calling process
pub fn fast_getuid(_args: &[u64]) -> SyscallResult {
    Ok(process::getuid() as u64)
}

/// Fast-path implementation for getgid
/// Returns the real group ID of the calling process
pub fn fast_getgid(_args: &[u64]) -> SyscallResult {
    Ok(process::getgid() as u64)
}

/// Fast-path implementation for getppid
/// Returns the process ID of the parent of the calling process
pub fn fast_getppid(_args: &[u64]) -> SyscallResult {
    if let Some(pid) = process::myproc() {
        let proc_table = PROC_TABLE.lock();
        if let Some(proc) = proc_table.find_ref(pid) {
            if let Some(ppid) = proc.parent {
                return Ok(ppid as u64);
            }
        }
    }
    Ok(0) // No parent (init or orphaned)
}

/// Fast-path implementation for gettid
/// Returns the thread ID of the calling thread
pub fn fast_gettid(_args: &[u64]) -> SyscallResult {
    // In a single-threaded process, TID equals PID
    // For multi-threaded processes, this would return thread ID
    if let Some(pid) = process::myproc() {
        Ok(pid as u64)
    } else {
        Err(SyscallError::NotFound)
    }
}

/// Fast-path implementation for geteuid
/// Returns the effective user ID of the calling process
pub fn fast_geteuid(_args: &[u64]) -> SyscallResult {
    // For now, effective UID equals real UID
    // In a full implementation, this would check for setuid bits
    Ok(process::getuid() as u64)
}

/// Fast-path implementation for getegid
/// Returns the effective group ID of the calling process
pub fn fast_getegid(_args: &[u64]) -> SyscallResult {
    // For now, effective GID equals real GID
    // In a full implementation, this would check for setgid bits
    Ok(process::getgid() as u64)
}

/// Fast-path handler type
pub type FastPathHandler = fn(&[u64]) -> SyscallResult;

/// Fast-path syscall registry
pub struct FastPathRegistry {
    handlers: [Option<FastPathHandler>; 256],
    call_counters: [AtomicU64; 256],
}

impl FastPathRegistry {
    /// Create a new fast-path registry
    pub const fn new() -> Self {
        const NONE_HANDLER: Option<FastPathHandler> = None;
        const ZERO_COUNTER: AtomicU64 = AtomicU64::new(0);
        
        Self {
            handlers: [NONE_HANDLER; 256],
            call_counters: [ZERO_COUNTER; 256],
        }
    }
    
    /// Register a fast-path handler for a syscall
    pub fn register(&mut self, syscall_num: u32, handler: FastPathHandler) {
        let index = (syscall_num % 256) as usize;
        self.handlers[index] = Some(handler);
    }
    
    /// Check if a syscall has a fast-path handler
    pub fn has_handler(&self, syscall_num: u32) -> bool {
        let index = (syscall_num % 256) as usize;
        self.handlers[index].is_some()
    }
    
    /// Dispatch a syscall through fast-path
    pub fn dispatch(&self, syscall_num: u32, args: &[u64]) -> Option<SyscallResult> {
        let index = (syscall_num % 256) as usize;
        
        if let Some(handler) = self.handlers[index] {
            // Increment call counter
            self.call_counters[index].fetch_add(1, Ordering::Relaxed);
            
            // Call fast-path handler
            Some(handler(args))
        } else {
            None
        }
    }
    
    /// Get call count for a syscall
    pub fn get_call_count(&self, syscall_num: u32) -> u64 {
        let index = (syscall_num % 256) as usize;
        self.call_counters[index].load(Ordering::Relaxed)
    }
    
    /// Initialize default fast-path handlers
    pub fn init_default_handlers(&mut self) {
        use syscall_numbers::*;
        
        // Register hot syscalls for fast-path
        self.register(SYS_GETPID, fast_getpid);
        self.register(SYS_GETPPID, fast_getppid);
        self.register(SYS_GETUID, fast_getuid);
        self.register(SYS_GETGID, fast_getgid);
        self.register(SYS_GETTID, fast_gettid);
        self.register(SYS_GETEUID, fast_geteuid);
        self.register(SYS_GETEGID, fast_getegid);
    }
}

/// Global fast-path registry
static mut FAST_PATH_REGISTRY: Option<FastPathRegistry> = None;
static FAST_PATH_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Initialize fast-path registry
pub fn init_fast_path_registry() {
    if FAST_PATH_INIT.compare_exchange(
        false,
        true,
        core::sync::atomic::Ordering::Acquire,
        core::sync::atomic::Ordering::Relaxed,
    ).is_ok() {
        unsafe {
            let mut registry = FastPathRegistry::new();
            registry.init_default_handlers();
            FAST_PATH_REGISTRY = Some(registry);
        }
    }
}

/// Check if a syscall can use fast-path
pub fn can_use_fast_path(syscall_num: u32) -> bool {
    unsafe {
        if let Some(ref registry) = FAST_PATH_REGISTRY {
            registry.has_handler(syscall_num)
        } else {
            false
        }
    }
}

/// Dispatch a syscall through fast-path
pub fn dispatch_fast_path(syscall_num: u32, args: &[u64]) -> Option<SyscallResult> {
    unsafe {
        if let Some(ref registry) = FAST_PATH_REGISTRY {
            registry.dispatch(syscall_num, args)
        } else {
            None
        }
    }
}

/// Get call count for a fast-path syscall
pub fn get_fast_path_call_count(syscall_num: u32) -> u64 {
    unsafe {
        if let Some(ref registry) = FAST_PATH_REGISTRY {
            registry.get_call_count(syscall_num)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_path_registry() {
        init_fast_path_registry();
        
        assert!(can_use_fast_path(syscall_numbers::SYS_GETPID));
        assert!(can_use_fast_path(syscall_numbers::SYS_GETUID));
        assert!(!can_use_fast_path(0x9999)); // Non-existent syscall
    }
    
    #[test]
    fn test_fast_getpid() {
        init_fast_path_registry();
        
        let result = dispatch_fast_path(syscall_numbers::SYS_GETPID, &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }
    
    #[test]
    fn test_fast_getuid() {
        init_fast_path_registry();
        
        let result = dispatch_fast_path(syscall_numbers::SYS_GETUID, &[]);
        assert!(result.is_some());
        let uid_result = result.unwrap();
        assert!(uid_result.is_ok());
    }
}

