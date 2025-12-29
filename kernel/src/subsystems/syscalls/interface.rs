//! System Call Interface Module
//!
//! This module defines the core interfaces and traits for system call handling.

use alloc::sync::Arc;


/// Interface system call error
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterfaceSyscallError {
    /// Invalid interface ID
    InvalidInterface,
    /// Interface not found
    InterfaceNotFound,
    /// Operation not supported
    NotSupported,
    /// Permission denied
    PermissionDenied,
    /// Resource busy
    ResourceBusy,
    /// Invalid argument
    InvalidArgument,
    /// System call failed
    SyscallFailed(crate::api::SyscallError),
}

/// System call arguments
pub type SyscallArgs = [u64; 6];

/// System call result type
pub type SyscallResult<T> = core::result::Result<T, InterfaceSyscallError>;

/// SyscallError type alias for compatibility
pub type SyscallError = InterfaceSyscallError;

/// System call number
pub type SyscallNumber = u32;

/// System call dispatcher trait
///
/// This trait must be implemented by all system call dispatchers.
pub trait SyscallDispatcher {
    /// Dispatch a system call
    ///
    /// # Arguments
    /// * `num` - System call number
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `SyscallResult` - Result of the system call
    fn dispatch(&self, num: SyscallNumber, args: &[u64]) -> SyscallResult<()>;

    /// Check if a system call is supported
    ///
    /// # Arguments
    /// * `num` - System call number
    ///
    /// # Returns
    /// * `bool` - True if supported, false otherwise
    fn is_supported(&self, num: SyscallNumber) -> bool;

    /// Get the name of a system call
    ///
    /// # Arguments
    /// * `num` - System call number
    ///
    /// # Returns
    /// * `Option<&'static str>` - Name of the system call if known
    fn get_name(&self, num: SyscallNumber) -> Option<&'static str>;

    /// Get the system call context
    ///
    /// # Returns
    /// * `&dyn SyscallContext` - System call context
    fn get_context(&self) -> &dyn SyscallContext;

    /// Get system call statistics
    ///
    /// # Returns
    /// * `SyscallStats` - System call statistics
    fn get_stats(&self) -> SyscallStats;

    /// List all registered handlers
    ///
    /// # Returns
    /// * `Vec<(usize, &str)>` - List of handler IDs and names
    fn list_handlers(&self) -> Vec<(usize, &str)>;
}

/// System call handler trait
///
/// This trait must be implemented by all system call handlers.
pub trait SyscallHandler {
    /// Get the system call number this handler handles
    ///
    /// # Returns
    /// * `SyscallNumber` - System call number
    fn get_syscall_number(&self) -> SyscallNumber;

    /// Get the name of this handler
    ///
    /// # Returns
    /// * `&'static str` - Handler name
    fn get_name(&self) -> &'static str;

    /// Handle a system call
    ///
    /// # Arguments
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `SyscallResult` - Result of the system call
    fn handle(&self, args: &[u64]) -> SyscallResult<()>;
}

/// System call context trait
///
/// This trait provides access to system call context information.
pub trait SyscallContext {
    /// Get the current process ID
    ///
    /// # Returns
    /// * `u32` - Process ID
    fn get_pid(&self) -> u32;

    /// Get the current user ID
    ///
    /// # Returns
    /// * `u32` - User ID
    fn get_uid(&self) -> u32;

    /// Get the current group ID
    ///
    /// # Returns
    /// * `u32` - Group ID
    fn get_gid(&self) -> u32;

    /// Check if the current process has permission for an operation
    ///
    /// # Arguments
    /// * `operation` - Operation to check
    ///
    /// # Returns
    /// * `bool` - True if permission granted
    fn has_permission(&self, operation: &str) -> bool;

    /// Get the current working directory
    ///
    /// # Returns
    /// * `&str` - Current working directory
    fn get_cwd(&self) -> &str;
}

/// System call category
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

/// System call statistics
#[derive(Debug, Default)]
pub struct SyscallStats {
    /// Total number of calls
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of failed calls
    pub failed_calls: u64,
    /// Average execution time in nanoseconds
    pub avg_execution_time_ns: f64,
}

impl SyscallStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total calls
    pub fn total_calls(&self) -> u64 {
        self.total_calls
    }

    /// Get successful calls
    pub fn successful_calls(&self) -> u64 {
        self.successful_calls
    }

    /// Get failed calls
    pub fn failed_calls(&self) -> u64 {
        self.failed_calls
    }

    /// Get average execution time
    pub fn avg_execution_time_ns(&self) -> f64 {
        self.avg_execution_time_ns
    }
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