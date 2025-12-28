//! System call ID definitions for the NOS kernel.
//!
//! This file defines all public system call IDs that applications can use.
//! Internal implementation details are hidden from the public API.

/// System call ID type
pub type SyscallId = u64;

/// System call IDs
pub mod syscall_ids {
    /// Process management system calls
    pub const SYSCALL_EXEC: u64 = 1;
    pub const SYSCALL_FORK: u64 = 2;
    pub const SYSCALL_EXIT: u64 = 3;
    pub const SYSCALL_WAIT4: u64 = 4;
    
    /// File I/O system calls
    pub const SYSCALL_OPEN: u64 = 5;
    pub const SYSCALL_CLOSE: u64 = 6;
    pub const SYSCALL_READ: u64 = 7;
    pub const SYSCALL_WRITE: u64 = 8;
    pub const SYSCALL_LSEEK: u64 = 9;
    
    /// Memory management system calls
    pub const SYSCALL_MMAP: u64 = 10;
    pub const SYSCALL_MUNMAP: u64 = 11;
    pub const SYSCALL_MPROTECT: u64 = 12;
    
    /// Network system calls
    pub const SYSCALL_SOCKET: u64 = 13;
    pub const SYSCALL_BIND: u64 = 14;
    pub const SYSCALL_LISTEN: u64 = 15;
    pub const SYSCALL_ACCEPT: u64 = 16;
    pub const SYSCALL_CONNECT: u64 = 17;
    
    /// Other system calls
    pub const SYSCALL_GETPID: u64 = 18;
    pub const SYSCALL_GETUID: u64 = 19;
    pub const SYSCALL_TIME: u64 = 20;
}