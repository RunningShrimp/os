//! System Call Service for hybrid architecture
//! Implements unified system call processing

use crate::syscalls;
use crate::syscalls::SysNum;
use crate::services::{service_register, ServiceInfo};

// ============================================================================
// System Call Service State
// ============================================================================

/// System call service endpoint (IPC channel)
pub const SYSCALL_SERVICE_ENDPOINT: usize = 0x4000;

// ============================================================================
// Public API
// ============================================================================

/// Initialize system call service
pub fn init() {
    // Register system call service
    service_register(
        "syscall",
        "System call service for processing user-space system calls",
        SYSCALL_SERVICE_ENDPOINT
    );
    
    crate::println!("services/syscall: initialized");
}

/// Dispatch a system call
pub fn syscall_dispatch(sysnum: usize, args: &[usize]) -> usize {
    // Delegate to the existing syscall dispatcher
    syscalls::dispatch(sysnum, args)
}

/// Get system call number from enum
pub fn syscall_num(sysnum: SysNum) -> usize {
    sysnum as usize
}