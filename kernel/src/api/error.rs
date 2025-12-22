//! Kernel Error API Interface
//!
//! This module defines unified error types for the kernel.
//! It provides a consistent error handling mechanism across all modules.

use crate::error::unified::UnifiedError;
use crate::error::unified_framework::{FrameworkError, FrameworkResult, IntoFrameworkError};

/// Kernel error type - migrated to unified framework
///
/// This type is now an alias for the unified FrameworkError
/// to ensure consistent error handling across the kernel.
pub type KernelError = FrameworkError;

/// Kernel result type - migrated to unified framework
pub type KernelResult<T> = FrameworkResult<T>;

/// Convert UnifiedError to KernelError (FrameworkError)
impl IntoFrameworkError for UnifiedError {
    fn into_framework_error(self) -> FrameworkError {
        FrameworkError::Unified(self)
    }
    
    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        FrameworkError::Contextual {
            error: self,
            context: context.to_string(),
            location: location.to_string(),
        }
    }
}
    /// Condition variable limit exceeded
    ConditionVariableLimitExceeded,
    /// Invalid semaphore
    InvalidSemaphore,
    /// Semaphore not found
    SemaphoreNotFound,
    /// Semaphore already exists
    SemaphoreAlreadyExists,
    /// Semaphore limit exceeded
    SemaphoreLimitExceeded,
    /// Semaphore is locked
    SemaphoreIsLocked,
    /// Semaphore is not locked
    SemaphoreIsNotLocked,
    /// Invalid shared memory
    InvalidSharedMemory,
    /// Shared memory not found
    SharedMemoryNotFound,
    /// Shared memory already exists
    SharedMemoryAlreadyExists,
    /// Shared memory limit exceeded
    SharedMemoryLimitExceeded,
    /// Invalid message queue
    InvalidMessageQueue,
    /// Message queue not found
    MessageQueueNotFound,
    /// Message queue already exists
    MessageQueueAlreadyExists,
    /// Message queue limit exceeded
    MessageQueueLimitExceeded,
    /// Message queue is full
    MessageQueueIsFull,
    /// Message queue is empty
    MessageQueueIsEmpty,
    /// Invalid message
    InvalidMessage,
    /// Message too large
    MessageTooLarge,
    /// Message not found
    MessageNotFound,
    /// Invalid socket
    InvalidSocket,
    /// Socket not found
    SocketNotFound,
    /// Socket already exists
    SocketAlreadyExists,
    /// Socket limit exceeded
    SocketLimitExceeded,
    /// Socket is not connected
    SocketIsNotConnected,
    /// Socket is connected
    SocketIsConnected,
    /// Socket is not bound
    SocketIsNotBound,
    /// Socket is bound
    SocketIsBound,
    /// Socket is not listening
    SocketIsNotListening,
    /// Socket is listening
    SocketIsListening,
    /// Socket is not closed
    SocketIsNotClosed,
    /// Socket is closed
    SocketIsClosed,
    /// Invalid address family
    InvalidAddressFamily,
    /// Invalid socket type
    InvalidSocketType,
    /// Invalid protocol
    InvalidProtocol,
    /// Address in use
    AddressInUse,
    /// Address not available
    AddressNotAvailable,
    /// Network is unreachable
    NetworkIsUnreachable,
    /// Network is down
    NetworkIsDown,
    /// Connection timed out
    ConnectionTimedOut,
    /// Connection reset by peer
    ConnectionResetByPeer,
    /// Host is unreachable
    HostIsUnreachable,
    /// Host is down
    HostIsDown,
    /// No route to host
    NoRouteToHost,
    /// Unknown error
    Unknown,
}

impl KernelError {
    /// Convert kernel error to errno value
    ///
    /// # Returns
    /// * `i32` - errno value
    pub fn to_errno(&self) -> i32 {
        match self {
            KernelError::InvalidArgument => 22,        // EINVAL
            KernelError::InvalidAddress => 14,        // EFAULT
            KernelError::PermissionDenied => 13,      // EACCES
            KernelError::NotFound => 2,              // ENOENT
            KernelError::AlreadyExists => 17,        // EEXIST
            KernelError::ResourceBusy => 16,        // EBUSY
            KernelError::ResourceUnavailable => 11,   // EAGAIN
            KernelError::OutOfMemory => 12,          // ENOMEM
            KernelError::NotSupported => 95,        // EOPNOTSUPP
            KernelError::WouldBlock => 11,           // EAGAIN
            KernelError::Interrupted => 4,           // EINTR
            KernelError::InvalidState => 22,         // EINVAL
            KernelError::InvalidFd => 9,             // EBADF
            KernelError::IoError => 5,               // EIO
            KernelError::FileSystemError => 5,        // EIO
            KernelError::NetworkError => 5,           // EIO
            KernelError::ProtocolError => 71,         // EPROTO
            KernelError::Timeout => 110,              // ETIMEDOUT
            KernelError::QuotaExceeded => 122,       // EDQUOT
            KernelError::AccessDenied => 13,          // EACCES
            KernelError::ConnectionRefused => 111,     // ECONNREFUSED
            KernelError::ConnectionReset => 104,       // ECONNRESET
            KernelError::ConnectionAborted => 103,     // ECONNABORTED
            KernelError::BrokenPipe => 32,            // EPIPE
            KernelError::BufferOverflow => 75,         // EOVERFLOW
            KernelError::BufferUnderflow => 22,       // EINVAL
            KernelError::InvalidOperation => 22,       // EINVAL
            KernelError::OperationInProgress => 115,    // EINPROGRESS
            KernelError::OperationAlreadyInProgress => 114, // EALREADY
            KernelError::OperationNotPermitted => 1,   // EPERM
            KernelError::OperationNotSupportedByDevice => 95, // EOPNOTSUPP
            KernelError::DeviceNotConfigured => 22,    // EINVAL
            KernelError::DeviceBusy => 16,            // EBUSY
            KernelError::DeviceNotFound => 19,         // ENODEV
            KernelError::InvalidDevice => 22,         // EINVAL
            KernelError::NoSuchDevice => 19,           // ENODEV
            KernelError::NoSuchFileOrDirectory => 2,    // ENOENT
            KernelError::NotADirectory => 20,         // ENOTDIR
            KernelError::IsADirectory => 21,          // EISDIR
            KernelError::NotARegularFile => 28,       // ENOTREG
            KernelError::FileTooLarge => 27,          // EFBIG
            KernelError::NoSpaceLeftOnDevice => 28,    // ENOSPC
            KernelError::ReadOnlyFileSystem => 30,      // EROFS
            KernelError::TooManyLinks => 31,          // EMLINK
            KernelError::TooManyOpenFiles => 24,       // EMFILE
            KernelError::TooManyOpenFilesInSystem => 23, // ENFILE
            KernelError::FilenameTooLong => 36,        // ENAMETOOLONG
            KernelError::NoSuchProcess => 3,            // ESRCH
            KernelError::ProcessAlreadyExists => 17,     // EEXIST
            KernelError::ProcessIsDead => 22,          // EINVAL
            KernelError::ProcessIsNotAChild => 10,     // ECHILD
            KernelError::ProcessIsNotStopped => 22,     // EINVAL
            KernelError::ProcessIsNotRunning => 22,     // EINVAL
            KernelError::ProcessIsNotAZombie => 22,     // EINVAL
            KernelError::ProcessIsAZombie => 22,        // EINVAL
            KernelError::ProcessIsNotSuspended => 22,   // EINVAL
            KernelError::ProcessIsSuspended => 22,      // EINVAL
            KernelError::ProcessLimitExceeded => 35,     // EAGAIN
            KernelError::ThreadLimitExceeded => 11,     // EAGAIN
            KernelError::NoChildProcesses => 10,        // ECHILD
            KernelError::ChildProcessHasExited => 10,    // ECHILD
            KernelError::ChildProcessIsNotStopped => 10, // ECHILD
            KernelError::ChildProcessIsNotAZombie => 10, // ECHILD
            KernelError::ChildProcessIsAZombie => 10,    // ECHILD
            KernelError::ChildProcessIsNotSuspended => 10, // ECHILD
            KernelError::ChildProcessIsSuspended => 10,  // ECHILD
            KernelError::InvalidSignal => 22,           // EINVAL
            KernelError::SignalNotPermitted => 1,       // EPERM
            KernelError::SignalAlreadyPending => 22,     // EINVAL
            FrameworkError::Unified(UnifiedError::InvalidArgument) => 22, // EINVAL
            FrameworkError::Unified(UnifiedError::InvalidAddress) => 14, // EFAULT
            FrameworkError::Unified(UnifiedError::PermissionDenied) => 13, // EACCES
            FrameworkError::Unified(UnifiedError::NotFound) => 2, // ENOENT
            FrameworkError::Unified(UnifiedError::AlreadyExists) => 17, // EEXIST
            FrameworkError::Unified(UnifiedError::ResourceBusy) => 16, // EBUSY
            FrameworkError::Unified(UnifiedError::ResourceUnavailable) => 11, // EAGAIN
            FrameworkError::Unified(UnifiedError::OutOfMemory) => 12, // ENOMEM
            FrameworkError::Unified(UnifiedError::MemoryError(MemoryError::OutOfMemory)) => 12, // ENOMEM
            FrameworkError::Unified(UnifiedError::MemoryError(MemoryError::InvalidAlignment)) => 22, // EINVAL
            FrameworkError::Unified(UnifiedError::MemoryError(MemoryError::InvalidSize)) => 22, // EINVAL
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::PathNotFound)) => 2, // ENOENT
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::FileNotFound)) => 2, // ENOENT
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::PermissionDenied)) => 13, // EACCES
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::FileExists)) => 17, // EEXIST
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::NotADirectory)) => 20, // ENOTDIR
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::IsADirectory)) => 21, // EISDIR
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::ConnectionRefused)) => 111, // ECONNREFUSED
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::ConnectionReset)) => 104, // ECONNRESET
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::BrokenPipe)) => 32, // EPIPE
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::TimedOut)) => 110, // ETIMEDOUT
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ProcessNotFound)) => 3, // ESRCH
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::PermissionDenied)) => 13, // EACCES
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::InvalidArgument)) => 22, // EINVAL
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ResourceLimitExceeded)) => 12, // ENOMEM
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ProcessAlreadyExists)) => 17, // EEXIST
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ProcessTerminated)) => 3, // ESRCH
            _ => 38, // ENOSYS
        }
    }

    /// Get a description of the error
    ///
    /// # Returns
    /// * `&str` - Error description
    pub fn description(&self) -> &'static str {
        match self {
            FrameworkError::Unified(UnifiedError::InvalidArgument) => "Invalid argument",
            FrameworkError::Unified(UnifiedError::InvalidAddress) => "Invalid address",
            FrameworkError::Unified(UnifiedError::PermissionDenied) => "Permission denied",
            FrameworkError::Unified(UnifiedError::NotFound) => "Not found",
            FrameworkError::Unified(UnifiedError::AlreadyExists) => "Already exists",
            FrameworkError::Unified(UnifiedError::ResourceBusy) => "Resource busy",
            FrameworkError::Unified(UnifiedError::ResourceUnavailable) => "Resource unavailable",
            FrameworkError::Unified(UnifiedError::OutOfMemory) => "Out of memory",
            FrameworkError::Unified(UnifiedError::MemoryError(MemoryError::OutOfMemory)) => "Out of memory",
            FrameworkError::Unified(UnifiedError::MemoryError(MemoryError::InvalidAlignment)) => "Invalid alignment",
            FrameworkError::Unified(UnifiedError::MemoryError(MemoryError::InvalidSize)) => "Invalid size",
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::PathNotFound)) => "Path not found",
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::FileNotFound)) => "File not found",
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::PermissionDenied)) => "Permission denied",
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::FileExists)) => "File exists",
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::NotADirectory)) => "Not a directory",
            FrameworkError::Unified(UnifiedError::FileSystemError(FileSystemError::IsADirectory)) => "Is a directory",
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::ConnectionRefused)) => "Connection refused",
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::ConnectionReset)) => "Connection reset",
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::BrokenPipe)) => "Broken pipe",
            FrameworkError::Unified(UnifiedError::NetworkError(NetworkError::TimedOut)) => "Timeout",
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ProcessNotFound)) => "Process not found",
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::PermissionDenied)) => "Permission denied",
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::InvalidArgument)) => "Invalid argument",
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ResourceLimitExceeded)) => "Resource limit exceeded",
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ProcessAlreadyExists)) => "Process already exists",
            FrameworkError::Unified(UnifiedError::ProcessError(ProcessError::ProcessTerminated)) => "Process terminated",
            FrameworkError::Contextual { error, context: _, location: _ } => error.description(),
            FrameworkError::Chain { error, cause: _ } => error.description(),
            _ => "Unknown error",
        }
    }
}

/// Error context
///
/// This struct provides additional context for errors.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The error that occurred
    pub error: KernelError,
    /// The operation that failed
    pub operation: String,
    /// The file where the error occurred
    pub file: String,
    /// The line where the error occurred
    pub line: u32,
    /// Additional context information
    pub context: Vec<String>,
}

impl ErrorContext {
    /// Create a new error context
    ///
    /// # Arguments
    /// * `error` - The error that occurred
    /// * `operation` - The operation that failed
    /// * `file` - The file where the error occurred
    /// * `line` - The line where the error occurred
    ///
    /// # Returns
    /// * `ErrorContext` - New error context
    pub fn new(error: KernelError, operation: &str, file: &str, line: u32) -> Self {
        Self {
            error,
            operation: operation.to_string(),
            file: file.to_string(),
            line,
            context: Vec::new(),
        }
    }

    /// Add context information
    ///
    /// # Arguments
    /// * `context` - Context information
    pub fn add_context(&mut self, context: &str) {
        self.context.push(context.to_string());
    }

    /// Get the error description with context
    ///
    /// # Returns
    /// * `String` - Error description with context
    pub fn to_string(&self) -> String {
        let mut result = format!(
            "{}: {} ({}:{}): {}",
            self.operation,
            self.error.description(),
            self.file,
            self.line,
            self.error.to_errno()
        );

        for context in &self.context {
            result.push_str(&format!(" - {}", context));
        }

        result
    }
}

/// Macro for creating error context
#[macro_export]
macro_rules! error_context {
    ($error:expr, $operation:expr) => {
        ErrorContext::new($error, $operation, file!(), line!())
    };
    ($error:expr, $operation:expr, $($context:expr),*) => {
        {
            let mut ctx = ErrorContext::new($error, $operation, file!(), line!());
            $(ctx.add_context($context);)*
            ctx
        }
    };
}