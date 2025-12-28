//! System Call API Interface
//!
//! This module defines the trait interfaces for system call dispatching
//! and handling. It re-exports interfaces from nos-api to ensure consistency.

pub use nos_api::syscall::interface::{SyscallDispatcher, SyscallHandler};
pub use nos_api::syscall::types::{SyscallNumber, SyscallArgs, SyscallResult};
use crate::error::unified_framework::{FrameworkError, IntoFrameworkError, FrameworkResult};
use crate::error::unified::UnifiedError;

// Migrated to unified error framework
pub type KernelError = FrameworkError;
pub type SyscallError = FrameworkError;

pub trait KernelErrorExt {
    fn to_errno(&self) -> i32;
}

impl KernelErrorExt for FrameworkError {
    fn to_errno(&self) -> i32 {
        match self {
            FrameworkError::Unified(e) => match e {
                UnifiedError::PermissionDenied => 13,
                UnifiedError::NotFound => 2,
                UnifiedError::IoError => 5,
                UnifiedError::NoDevice => 19,
                UnifiedError::InvalidArgument => 22,
                UnifiedError::OutOfMemory => 12,
                UnifiedError::Busy => 16,
                UnifiedError::WouldBlock => 11,
                UnifiedError::AlreadyInProgress => 114,
                UnifiedError::ConnectionReset => 104,
                UnifiedError::ConnectionAborted => 103,
                UnifiedError::NoProcess => 3,
                UnifiedError::Interrupted => 4,
                UnifiedError::BadFileDescriptor => 9,
                UnifiedError::NotSupported => 95,
                UnifiedError::TimedOut => 110,
                UnifiedError::OutOfSpace => 28,
                UnifiedError::QuotaExceeded => 122,
                UnifiedError::Unknown(code) => *code,
                _ => 255, // Unknown error
            },
            FrameworkError::Contextual { error, .. } => match error {
                UnifiedError::PermissionDenied => 13,
                UnifiedError::NotFound => 2,
                UnifiedError::IoError => 5,
                UnifiedError::NoDevice => 19,
                UnifiedError::InvalidArgument => 22,
                UnifiedError::OutOfMemory => 12,
                UnifiedError::Busy => 16,
                UnifiedError::WouldBlock => 11,
                UnifiedError::AlreadyInProgress => 114,
                UnifiedError::ConnectionReset => 104,
                UnifiedError::ConnectionAborted => 103,
                UnifiedError::NoProcess => 3,
                UnifiedError::Interrupted => 4,
                UnifiedError::BadFileDescriptor => 9,
                UnifiedError::NotSupported => 95,
                UnifiedError::TimedOut => 110,
                UnifiedError::OutOfSpace => 28,
                UnifiedError::QuotaExceeded => 122,
                UnifiedError::Unknown(code) => *code,
                _ => 255, // Unknown error
            },
            FrameworkError::Chain { error, .. } => match error {
                UnifiedError::PermissionDenied => 13,
                UnifiedError::NotFound => 2,
                UnifiedError::IoError => 5,
                UnifiedError::NoDevice => 19,
                UnifiedError::InvalidArgument => 22,
                UnifiedError::OutOfMemory => 12,
                UnifiedError::Busy => 16,
                UnifiedError::WouldBlock => 11,
                UnifiedError::AlreadyInProgress => 114,
                UnifiedError::ConnectionReset => 104,
                UnifiedError::ConnectionAborted => 103,
                UnifiedError::NoProcess => 3,
                UnifiedError::Interrupted => 4,
                UnifiedError::BadFileDescriptor => 9,
                UnifiedError::NotSupported => 95,
                UnifiedError::TimedOut => 110,
                UnifiedError::OutOfSpace => 28,
                UnifiedError::QuotaExceeded => 122,
                UnifiedError::Unknown(code) => *code,
                _ => 255, // Unknown error
            },
        }
    }
}


/// System call context trait
///
/// This trait provides access to system call context information
/// such as the current process, user credentials, and other
/// execution context.
pub trait SyscallContext {
    /// Get the current process ID
    fn get_pid(&self) -> u32;

    /// Get the current user ID
    fn get_uid(&self) -> u32;

    /// Get the current group ID
    fn get_gid(&self) -> u32;

    /// Check if the current process has permission to perform an operation
    fn has_permission(&self, operation: &str) -> bool;

    /// Get the current working directory
    fn get_cwd(&self) -> &str;
}

/// System call category
///
/// This enum represents the different categories of system calls.
/// It is used for organizing and routing system calls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyscallCategory {
    /// Process management syscalls (0x1000-0x1FFF)
    Process,
    /// File I/O syscalls (0x2000-0x2FFF)
    FileIo,
    /// Memory management syscalls (0x3000-0x3FFF)
    Memory,
    /// Network syscalls (0x4000-0x4FFF)
    Network,
    /// Signal handling syscalls (0x5000-0x5FFF)
    Signal,
    /// Time-related syscalls (0x6000-0x6FFF)
    Time,
    /// Filesystem syscalls (0x7000-0x7FFF)
    Filesystem,
    /// Thread management syscalls (0x8000-0x8FFF)
    Thread,
    /// Zero-copy I/O syscalls (0x9000-0x9FFF)
    ZeroCopyIo,
    /// epoll syscalls (0xA000-0xAFFF)
    Epoll,
    /// GLib compatibility syscalls (0xB000-0xBFFF)
    Glib,
    /// AIO syscalls (0xC000-0xCFFF)
    Aio,
    /// Message queue syscalls (0xD000-0xDFFF)
    MessageQueue,
    /// Real-time scheduling syscalls (0xE000-0xEFFF)
    Realtime,
    /// Security system calls (0xF000-0xFFFF)
    Security,
}

/// Get the category of a system call by number
pub fn get_syscall_category(num: u32) -> Option<SyscallCategory> {
    match num {
        0x1000..=0x1FFF => Some(SyscallCategory::Process),
        0x2000..=0x2FFF => Some(SyscallCategory::FileIo),
        0x3000..=0x3FFF => Some(SyscallCategory::Memory),
        0x4000..=0x4FFF => Some(SyscallCategory::Network),
        0x5000..=0x5FFF => Some(SyscallCategory::Signal),
        0x6000..=0x6FFF => Some(SyscallCategory::Time),
        0x7000..=0x7FFF => Some(SyscallCategory::Filesystem),
        0x8000..=0x8FFF => Some(SyscallCategory::Thread),
        0x9000..=0x9FFF => Some(SyscallCategory::ZeroCopyIo),
        0xA000..=0xAFFF => Some(SyscallCategory::Epoll),
        0xB000..=0xBFFF => Some(SyscallCategory::Glib),
        0xC000..=0xCFFF => Some(SyscallCategory::Aio),
        0xD000..=0xDFFF => Some(SyscallCategory::MessageQueue),
        0xE000..=0xEFFF => Some(SyscallCategory::Realtime),
        0xF000..=0xFFFF => Some(SyscallCategory::Security),
        _ => None,
    }
}

/// Common system call numbers
pub mod syscall_numbers {
    /// Process management syscalls
    pub const SYS_GETPID: u32 = 0x1000;
    pub const SYS_FORK: u32 = 0x1001;
    pub const SYS_EXECVE: u32 = 0x1002;
    pub const SYS_EXIT: u32 = 0x1003;
    pub const SYS_WAIT4: u32 = 0x1004;
    pub const SYS_KILL: u32 = 0x1005;
    
    /// File I/O syscalls
    pub const SYS_READ: u32 = 0x2000;
    pub const SYS_WRITE: u32 = 0x2001;
    pub const SYS_OPEN: u32 = 0x2002;
    pub const SYS_CLOSE: u32 = 0x2003;
    pub const SYS_STAT: u32 = 0x2004;
    
    /// Memory management syscalls
    pub const SYS_MMAP: u32 = 0x3000;
    pub const SYS_MUNMAP: u32 = 0x3001;
    pub const SYS_BRK: u32 = 0x3002;
    pub const SYS_MPROTECT: u32 = 0x3003;
    
    /// Network syscalls
    pub const SYS_SOCKET: u32 = 0x4000;
    pub const SYS_BIND: u32 = 0x4001;
    pub const SYS_CONNECT: u32 = 0x4002;
    pub const SYS_LISTEN: u32 = 0x4003;
    pub const SYS_ACCEPT: u32 = 0x4004;
}