//! System call type definitions
//!
//! This module contains system call numbers and type definitions.

/// System call numbers
pub const SYS_READ: u32 = 0;
pub const SYS_WRITE: u32 = 1;
pub const SYS_OPEN: u32 = 2;
pub const SYS_CLOSE: u32 = 3;
pub const SYS_FORK: u32 = 4;
pub const SYS_EXEC: u32 = 5;
pub const SYS_WAIT: u32 = 6;
pub const SYS_EXIT: u32 = 7;
pub const SYS_SOCKET: u32 = 8;
pub const SYS_CONNECT: u32 = 9;
pub const SYS_ACCEPT: u32 = 10;
pub const SYS_MMAP: u32 = 11;
pub const SYS_MUNMAP: u32 = 12;
pub const SYS_MPROTECT: u32 = 13;
pub const SYS_CLOCK_GETTIME: u32 = 14;
pub const SYS_GETTIMEOFDAY: u32 = 15;
pub const SYS_NANOSLEEP: u32 = 16;

// Advanced system calls
pub const SYS_ADVANCED_MMAP: u32 = 1000;
pub const SYS_ASYNC_OP: u32 = 1001;
pub const SYS_ASYNC_WAIT: u32 = 1002;
pub const SYS_EPOLL_CREATE: u32 = 1003;
pub const SYS_EPOLL_CTL: u32 = 1004;
pub const SYS_EPOLL_WAIT: u32 = 1005;
pub const SYS_SCHED_YIELD: u32 = 1006;

// Zero-copy network I/O system calls
pub const SYS_ZERO_COPY_SEND: u32 = 2000;
pub const SYS_ZERO_COPY_RECV: u32 = 2001;

// Performance monitoring system calls
pub const SYS_PERF_MONITOR: u32 = 3000;