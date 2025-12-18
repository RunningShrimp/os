//! System Call Interface Layer
//!
//! This module defines the interface layer for system calls.
//! It provides trait definitions and abstractions that separate
//! the system call interface from its implementation.
//!
//! # Architecture
//!
//! The interface layer consists of:
//! - System call dispatcher trait
//! - System call handler trait
//! - System call context trait
//! - System call result types
//!
//! # Design Principles
//!
//! - **Dependency Inversion**: High-level modules depend on abstractions
//! - **Interface Segregation**: Small, focused interfaces
//! - **Single Responsibility**: Each interface has a single purpose

use alloc::vec::Vec;
use crate::types::stubs::{pid_t, uid_t, gid_t};

/// System call dispatcher trait
///
/// This trait defines the interface for dispatching system calls
/// to their appropriate handlers. Implementations should handle
/// argument validation, permission checking, and routing to the
/// correct subsystem.
pub trait SyscallDispatcher {
    /// Dispatch a system call by number with arguments
    ///
    /// # Arguments
    /// * `num` - System call number
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `Ok(u64)` - System call return value
    /// * `Err(SyscallError)` - System call error
    fn dispatch(&self, num: u32, args: &[u64]) -> Result<u64, SyscallError>;

    /// Check if a system call is supported
    ///
    /// # Arguments
    /// * `num` - System call number
    ///
    /// # Returns
    /// * `true` if the system call is supported
    /// * `false` if the system call is not supported
    fn is_supported(&self, num: u32) -> bool;

    /// Get the name of a system call by number
    ///
    /// # Arguments
    /// * `num` - System call number
    ///
    /// # Returns
    /// * `Some(&str)` - System call name if supported
    /// * `None` - System call name if not supported
    fn get_name(&self, num: u32) -> Option<&'static str>;
    
    /// Get the context
    ///
    /// # Returns
    /// * `&dyn SyscallContext` - Context reference
    fn get_context(&self) -> &dyn SyscallContext;
}

/// System call handler trait
///
/// This trait defines the interface for handling specific system calls.
/// Each system call category should implement this trait.
pub trait SyscallHandler: core::fmt::Debug {
    /// Handle a system call with arguments
    ///
    /// # Arguments
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `Ok(u64)` - System call return value
    /// * `Err(SyscallError)` - System call error
    fn handle(&self, args: &[u64]) -> Result<u64, SyscallError>;

    /// Get the system call number this handler handles
    ///
    /// # Returns
    /// * `u32` - System call number
    fn get_syscall_number(&self) -> u32;

    /// Get the system call name
    ///
    /// # Returns
    /// * `&str` - System call name
    fn get_name(&self) -> &'static str;
}

/// System call context trait
///
/// This trait provides access to system call context information
/// such as the current process, user credentials, and other
/// execution context.
pub trait SyscallContext {
    /// Get the current process ID
    ///
    /// # Returns
    /// * `pid_t` - Current process ID
    fn get_pid(&self) -> pid_t;

    /// Get the current user ID
    ///
    /// # Returns
    /// * `uid_t` - Current user ID
    fn get_uid(&self) -> uid_t;

    /// Get the current group ID
    ///
    /// # Returns
    /// * `gid_t` - Current group ID
    fn get_gid(&self) -> gid_t;

    /// Check if the current process has permission to perform an operation
    ///
    /// # Arguments
    /// * `operation` - Operation to check permission for
    ///
    /// # Returns
    /// * `true` if the operation is permitted
    /// * `false` if the operation is not permitted
    fn has_permission(&self, operation: &str) -> bool;

    /// Get the current working directory
    ///
    /// # Returns
    /// * `&str` - Current working directory path
    fn get_cwd(&self) -> &str;
}

/// System call error type
///
/// This enum represents all possible system call errors.
/// It provides a unified error handling mechanism for all system calls.
#[derive(Debug, Clone, PartialEq)]
pub enum SyscallError {
    /// Invalid system call number
    InvalidSyscall(u32),
    /// Invalid arguments
    InvalidArguments,
    /// Permission denied
    PermissionDenied,
    /// Resource not found
    NotFound,
    /// Resource already exists
    AlreadyExists,
    /// Invalid file descriptor
    InvalidFd,
    /// I/O error
    IoError,
    /// Memory allocation failed
    OutOfMemory,
    /// Operation not supported
    NotSupported,
    /// Operation would block
    WouldBlock,
    /// Operation interrupted
    Interrupted,
    /// Invalid address
    InvalidAddress,
    /// Access denied
    AccessDenied,
    /// Resource busy
    ResourceBusy,
    /// Resource temporarily unavailable
    ResourceUnavailable,
    /// Operation timed out
    TimedOut,
    /// Quota exceeded
    QuotaExceeded,
    /// File system error
    FileSystemError,
    /// Network error
    NetworkError,
    /// Protocol error
    ProtocolError,
    /// Unknown error
    Unknown,
}

impl SyscallError {
    /// Convert system call error to errno value
    ///
    /// # Returns
    /// * `i32` - errno value
    pub fn to_errno(&self) -> i32 {
        match self {
            SyscallError::InvalidSyscall(_) => 38, // ENOSYS
            SyscallError::InvalidArguments => 22,  // EINVAL
            SyscallError::PermissionDenied => 13,  // EACCES
            SyscallError::NotFound => 2,          // ENOENT
            SyscallError::AlreadyExists => 17,     // EEXIST
            SyscallError::InvalidFd => 9,         // EBADF
            SyscallError::IoError => 5,           // EIO
            SyscallError::OutOfMemory => 12,       // ENOMEM
            SyscallError::NotSupported => 95,     // EOPNOTSUPP
            SyscallError::WouldBlock => 11,        // EAGAIN
            SyscallError::Interrupted => 4,        // EINTR
            SyscallError::InvalidAddress => 14,     // EFAULT
            SyscallError::AccessDenied => 13,      // EACCES
            SyscallError::ResourceBusy => 16,      // EBUSY
            SyscallError::ResourceUnavailable => 11, // EAGAIN
            SyscallError::TimedOut => 110,        // ETIMEDOUT
            SyscallError::QuotaExceeded => 122,    // EDQUOT
            SyscallError::FileSystemError => 5,     // EIO
            SyscallError::NetworkError => 5,       // EIO
            SyscallError::ProtocolError => 71,     // EPROTO
            SyscallError::Unknown => 38,           // ENOSYS
        }
    }
}

/// System call result type
///
/// This is a convenience type alias for system call results.
pub type SyscallResult = Result<u64, SyscallError>;

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
///
/// # Arguments
/// * `num` - System call number
///
/// # Returns
/// * `Some(SyscallCategory)` - System call category if valid
/// * `None` - System call category if invalid
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