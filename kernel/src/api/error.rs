//! Kernel Error API Interface
//!
//! This module defines unified error types for the kernel.
//! It provides a consistent error handling mechanism across all modules.

use alloc::string::String;
use alloc::vec::Vec;

/// Kernel error type
///
/// This enum represents all possible kernel errors.
/// It provides a unified error handling mechanism for all kernel operations.
#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    /// Invalid argument
    InvalidArgument,
    /// Invalid address
    InvalidAddress,
    /// Permission denied
    PermissionDenied,
    /// Resource not found
    NotFound,
    /// Resource already exists
    AlreadyExists,
    /// Resource busy
    ResourceBusy,
    /// Resource unavailable
    ResourceUnavailable,
    /// Out of memory
    OutOfMemory,
    /// Operation not supported
    NotSupported,
    /// Operation would block
    WouldBlock,
    /// Operation interrupted
    Interrupted,
    /// Invalid state
    InvalidState,
    /// Invalid file descriptor
    InvalidFd,
    /// I/O error
    IoError,
    /// File system error
    FileSystemError,
    /// Network error
    NetworkError,
    /// Protocol error
    ProtocolError,
    /// Timeout
    Timeout,
    /// Quota exceeded
    QuotaExceeded,
    /// Access denied
    AccessDenied,
    /// Connection refused
    ConnectionRefused,
    /// Connection reset
    ConnectionReset,
    /// Connection aborted
    ConnectionAborted,
    /// Broken pipe
    BrokenPipe,
    /// Buffer overflow
    BufferOverflow,
    /// Buffer underflow
    BufferUnderflow,
    /// Invalid operation
    InvalidOperation,
    /// Operation in progress
    OperationInProgress,
    /// Operation already in progress
    OperationAlreadyInProgress,
    /// Operation not permitted
    OperationNotPermitted,
    /// Operation not supported by device
    OperationNotSupportedByDevice,
    /// Device not configured
    DeviceNotConfigured,
    /// Device busy
    DeviceBusy,
    /// Device not found
    DeviceNotFound,
    /// Invalid device
    InvalidDevice,
    /// No such device
    NoSuchDevice,
    /// No such file or directory
    NoSuchFileOrDirectory,
    /// Not a directory
    NotADirectory,
    /// Is a directory
    IsADirectory,
    /// Not a regular file
    NotARegularFile,
    /// File too large
    FileTooLarge,
    /// No space left on device
    NoSpaceLeftOnDevice,
    /// Read-only file system
    ReadOnlyFileSystem,
    /// Too many links
    TooManyLinks,
    /// Too many open files
    TooManyOpenFiles,
    /// Too many open files in system
    TooManyOpenFilesInSystem,
    /// Filename too long
    FilenameTooLong,
    /// No such process
    NoSuchProcess,
    /// Process already exists
    ProcessAlreadyExists,
    /// Process is dead
    ProcessIsDead,
    /// Process is not a child
    ProcessIsNotAChild,
    /// Process is not stopped
    ProcessIsNotStopped,
    /// Process is not running
    ProcessIsNotRunning,
    /// Process is not a zombie
    ProcessIsNotAZombie,
    /// Process is a zombie
    ProcessIsAZombie,
    /// Process is not suspended
    ProcessIsNotSuspended,
    /// Process is suspended
    ProcessIsSuspended,
    /// Process limit exceeded
    ProcessLimitExceeded,
    /// Thread limit exceeded
    ThreadLimitExceeded,
    /// No child processes
    NoChildProcesses,
    /// Child process has exited
    ChildProcessHasExited,
    /// Child process is not stopped
    ChildProcessIsNotStopped,
    /// Child process is not a zombie
    ChildProcessIsNotAZombie,
    /// Child process is a zombie
    ChildProcessIsAZombie,
    /// Child process is not suspended
    ChildProcessIsNotSuspended,
    /// Child process is suspended
    ChildProcessIsSuspended,
    /// Invalid signal
    InvalidSignal,
    /// Signal not permitted
    SignalNotPermitted,
    /// Signal already pending
    SignalAlreadyPending,
    /// Signal not pending
    SignalNotPending,
    /// Signal queue overflow
    SignalQueueOverflow,
    /// Signal queue full
    SignalQueueFull,
    /// Invalid timer
    InvalidTimer,
    /// Timer not found
    TimerNotFound,
    /// Timer already exists
    TimerAlreadyExists,
    /// Timer expired
    TimerExpired,
    /// Timer not active
    TimerNotActive,
    /// Timer already active
    TimerAlreadyActive,
    /// Timer limit exceeded
    TimerLimitExceeded,
    /// Invalid event
    InvalidEvent,
    /// Event not found
    EventNotFound,
    /// Event already exists
    EventAlreadyExists,
    /// Event limit exceeded
    EventLimitExceeded,
    /// Invalid mutex
    InvalidMutex,
    /// Mutex not found
    MutexNotFound,
    /// Mutex already exists
    MutexAlreadyExists,
    /// Mutex limit exceeded
    MutexLimitExceeded,
    /// Mutex is locked
    MutexIsLocked,
    /// Mutex is not locked
    MutexIsNotLocked,
    /// Mutex is owned by another thread
    MutexIsOwnedByAnotherThread,
    /// Invalid condition variable
    InvalidConditionVariable,
    /// Condition variable not found
    ConditionVariableNotFound,
    /// Condition variable already exists
    ConditionVariableAlreadyExists,
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
            KernelError::SignalNotPending => 22,        // EINVAL
            KernelError::SignalQueueOverflow => 6,       // ENXIO
            KernelError::SignalQueueFull => 6,          // ENXIO
            KernelError::InvalidTimer => 22,            // EINVAL
            KernelError::TimerNotFound => 22,           // EINVAL
            KernelError::TimerAlreadyExists => 17,       // EEXIST
            KernelError::TimerExpired => 110,           // ETIMEDOUT
            KernelError::TimerNotActive => 22,         // EINVAL
            KernelError::TimerAlreadyActive => 22,       // EINVAL
            KernelError::TimerLimitExceeded => 35,      // EAGAIN
            KernelError::InvalidEvent => 22,            // EINVAL
            KernelError::EventNotFound => 22,           // EINVAL
            KernelError::EventAlreadyExists => 17,       // EEXIST
            KernelError::EventLimitExceeded => 35,      // EAGAIN
            KernelError::InvalidMutex => 22,            // EINVAL
            KernelError::MutexNotFound => 22,           // EINVAL
            KernelError::MutexAlreadyExists => 17,       // EEXIST
            KernelError::MutexLimitExceeded => 35,      // EAGAIN
            KernelError::MutexIsLocked => 16,           // EBUSY
            KernelError::MutexIsNotLocked => 22,        // EINVAL
            KernelError::MutexIsOwnedByAnotherThread => 16, // EBUSY
            KernelError::InvalidConditionVariable => 22,   // EINVAL
            KernelError::ConditionVariableNotFound => 22,  // EINVAL
            KernelError::ConditionVariableAlreadyExists => 17, // EEXIST
            KernelError::ConditionVariableLimitExceeded => 35, // EAGAIN
            KernelError::InvalidSemaphore => 22,         // EINVAL
            KernelError::SemaphoreNotFound => 22,        // EINVAL
            KernelError::SemaphoreAlreadyExists => 17,    // EEXIST
            KernelError::SemaphoreLimitExceeded => 35,   // EAGAIN
            KernelError::SemaphoreIsLocked => 16,        // EBUSY
            KernelError::SemaphoreIsNotLocked => 22,      // EINVAL
            KernelError::InvalidSharedMemory => 22,       // EINVAL
            KernelError::SharedMemoryNotFound => 22,      // EINVAL
            KernelError::SharedMemoryAlreadyExists => 17,  // EEXIST
            KernelError::SharedMemoryLimitExceeded => 35, // EAGAIN
            KernelError::InvalidMessageQueue => 22,       // EINVAL
            KernelError::MessageQueueNotFound => 22,      // EINVAL
            KernelError::MessageQueueAlreadyExists => 17,  // EEXIST
            KernelError::MessageQueueLimitExceeded => 35, // EAGAIN
            KernelError::MessageQueueIsFull => 6,        // ENXIO
            KernelError::MessageQueueIsEmpty => 6,        // ENXIO
            KernelError::InvalidMessage => 22,           // EINVAL
            KernelError::MessageTooLarge => 90,          // EMSGSIZE
            KernelError::MessageNotFound => 22,          // EINVAL
            KernelError::InvalidSocket => 22,            // EINVAL
            KernelError::SocketNotFound => 22,           // EINVAL
            KernelError::SocketAlreadyExists => 17,       // EEXIST
            KernelError::SocketLimitExceeded => 24,       // EMFILE
            KernelError::SocketIsNotConnected => 107,     // ENOTCONN
            KernelError::SocketIsConnected => 106,        // EISCONN
            KernelError::SocketIsNotBound => 22,         // EINVAL
            KernelError::SocketIsBound => 22,            // EINVAL
            KernelError::SocketIsNotListening => 22,      // EINVAL
            KernelError::SocketIsListening => 22,         // EINVAL
            KernelError::SocketIsNotClosed => 22,        // EINVAL
            KernelError::SocketIsClosed => 22,            // EINVAL
            KernelError::InvalidAddressFamily => 97,       // EAFNOSUPPORT
            KernelError::InvalidSocketType => 22,         // EINVAL
            KernelError::InvalidProtocol => 22,           // EINVAL
            KernelError::AddressInUse => 98,             // EADDRINUSE
            KernelError::AddressNotAvailable => 99,       // EADDRNOTAVAIL
            KernelError::NetworkIsUnreachable => 101,     // ENETUNREACH
            KernelError::NetworkIsDown => 100,           // ENETDOWN
            KernelError::ConnectionTimedOut => 110,       // ETIMEDOUT
            KernelError::ConnectionRefused => 111,         // ECONNREFUSED
            KernelError::ConnectionResetByPeer => 104,     // ECONNRESET
            KernelError::ConnectionAborted => 103,         // ECONNABORTED
            KernelError::HostIsUnreachable => 113,       // EHOSTUNREACH
            KernelError::HostIsDown => 112,             // EHOSTDOWN
            KernelError::NoRouteToHost => 114,           // EHOSTUNREACH
            KernelError::Unknown => 38,                  // ENOSYS
        }
    }

    /// Get a description of the error
    ///
    /// # Returns
    /// * `&str` - Error description
    pub fn description(&self) -> &'static str {
        match self {
            KernelError::InvalidArgument => "Invalid argument",
            KernelError::InvalidAddress => "Invalid address",
            KernelError::PermissionDenied => "Permission denied",
            KernelError::NotFound => "Not found",
            KernelError::AlreadyExists => "Already exists",
            KernelError::ResourceBusy => "Resource busy",
            KernelError::ResourceUnavailable => "Resource unavailable",
            KernelError::OutOfMemory => "Out of memory",
            KernelError::NotSupported => "Not supported",
            KernelError::WouldBlock => "Operation would block",
            KernelError::Interrupted => "Operation interrupted",
            KernelError::InvalidState => "Invalid state",
            KernelError::InvalidFd => "Invalid file descriptor",
            KernelError::IoError => "I/O error",
            KernelError::FileSystemError => "File system error",
            KernelError::NetworkError => "Network error",
            KernelError::ProtocolError => "Protocol error",
            KernelError::Timeout => "Timeout",
            KernelError::QuotaExceeded => "Quota exceeded",
            KernelError::AccessDenied => "Access denied",
            KernelError::ConnectionRefused => "Connection refused",
            KernelError::ConnectionReset => "Connection reset",
            KernelError::ConnectionAborted => "Connection aborted",
            KernelError::BrokenPipe => "Broken pipe",
            KernelError::BufferOverflow => "Buffer overflow",
            KernelError::BufferUnderflow => "Buffer underflow",
            KernelError::InvalidOperation => "Invalid operation",
            KernelError::OperationInProgress => "Operation in progress",
            KernelError::OperationAlreadyInProgress => "Operation already in progress",
            KernelError::OperationNotPermitted => "Operation not permitted",
            KernelError::OperationNotSupportedByDevice => "Operation not supported by device",
            KernelError::DeviceNotConfigured => "Device not configured",
            KernelError::DeviceBusy => "Device busy",
            KernelError::DeviceNotFound => "Device not found",
            KernelError::InvalidDevice => "Invalid device",
            KernelError::NoSuchDevice => "No such device",
            KernelError::NoSuchFileOrDirectory => "No such file or directory",
            KernelError::NotADirectory => "Not a directory",
            KernelError::IsADirectory => "Is a directory",
            KernelError::NotARegularFile => "Not a regular file",
            KernelError::FileTooLarge => "File too large",
            KernelError::NoSpaceLeftOnDevice => "No space left on device",
            KernelError::ReadOnlyFileSystem => "Read-only file system",
            KernelError::TooManyLinks => "Too many links",
            KernelError::TooManyOpenFiles => "Too many open files",
            KernelError::TooManyOpenFilesInSystem => "Too many open files in system",
            KernelError::FilenameTooLong => "Filename too long",
            KernelError::NoSuchProcess => "No such process",
            KernelError::ProcessAlreadyExists => "Process already exists",
            KernelError::ProcessIsDead => "Process is dead",
            KernelError::ProcessIsNotAChild => "Process is not a child",
            KernelError::ProcessIsNotStopped => "Process is not stopped",
            KernelError::ProcessIsNotRunning => "Process is not running",
            KernelError::ProcessIsNotAZombie => "Process is not a zombie",
            KernelError::ProcessIsAZombie => "Process is a zombie",
            KernelError::ProcessIsNotSuspended => "Process is not suspended",
            KernelError::ProcessIsSuspended => "Process is suspended",
            KernelError::ProcessLimitExceeded => "Process limit exceeded",
            KernelError::ThreadLimitExceeded => "Thread limit exceeded",
            KernelError::NoChildProcesses => "No child processes",
            KernelError::ChildProcessHasExited => "Child process has exited",
            KernelError::ChildProcessIsNotStopped => "Child process is not stopped",
            KernelError::ChildProcessIsNotAZombie => "Child process is not a zombie",
            KernelError::ChildProcessIsAZombie => "Child process is a zombie",
            KernelError::ChildProcessIsNotSuspended => "Child process is not suspended",
            KernelError::ChildProcessIsSuspended => "Child process is suspended",
            KernelError::InvalidSignal => "Invalid signal",
            KernelError::SignalNotPermitted => "Signal not permitted",
            KernelError::SignalAlreadyPending => "Signal already pending",
            KernelError::SignalNotPending => "Signal not pending",
            KernelError::SignalQueueOverflow => "Signal queue overflow",
            KernelError::SignalQueueFull => "Signal queue full",
            KernelError::InvalidTimer => "Invalid timer",
            KernelError::TimerNotFound => "Timer not found",
            KernelError::TimerAlreadyExists => "Timer already exists",
            KernelError::TimerExpired => "Timer expired",
            KernelError::TimerNotActive => "Timer not active",
            KernelError::TimerAlreadyActive => "Timer already active",
            KernelError::TimerLimitExceeded => "Timer limit exceeded",
            KernelError::InvalidEvent => "Invalid event",
            KernelError::EventNotFound => "Event not found",
            KernelError::EventAlreadyExists => "Event already exists",
            KernelError::EventLimitExceeded => "Event limit exceeded",
            KernelError::InvalidMutex => "Invalid mutex",
            KernelError::MutexNotFound => "Mutex not found",
            KernelError::MutexAlreadyExists => "Mutex already exists",
            KernelError::MutexLimitExceeded => "Mutex limit exceeded",
            KernelError::MutexIsLocked => "Mutex is locked",
            KernelError::MutexIsNotLocked => "Mutex is not locked",
            KernelError::MutexIsOwnedByAnotherThread => "Mutex is owned by another thread",
            KernelError::InvalidConditionVariable => "Invalid condition variable",
            KernelError::ConditionVariableNotFound => "Condition variable not found",
            KernelError::ConditionVariableAlreadyExists => "Condition variable already exists",
            KernelError::ConditionVariableLimitExceeded => "Condition variable limit exceeded",
            KernelError::InvalidSemaphore => "Invalid semaphore",
            KernelError::SemaphoreNotFound => "Semaphore not found",
            KernelError::SemaphoreAlreadyExists => "Semaphore already exists",
            KernelError::SemaphoreLimitExceeded => "Semaphore limit exceeded",
            KernelError::SemaphoreIsLocked => "Semaphore is locked",
            KernelError::SemaphoreIsNotLocked => "Semaphore is not locked",
            KernelError::InvalidSharedMemory => "Invalid shared memory",
            KernelError::SharedMemoryNotFound => "Shared memory not found",
            KernelError::SharedMemoryAlreadyExists => "Shared memory already exists",
            KernelError::SharedMemoryLimitExceeded => "Shared memory limit exceeded",
            KernelError::InvalidMessageQueue => "Invalid message queue",
            KernelError::MessageQueueNotFound => "Message queue not found",
            KernelError::MessageQueueAlreadyExists => "Message queue already exists",
            KernelError::MessageQueueLimitExceeded => "Message queue limit exceeded",
            KernelError::MessageQueueIsFull => "Message queue is full",
            KernelError::MessageQueueIsEmpty => "Message queue is empty",
            KernelError::InvalidMessage => "Invalid message",
            KernelError::MessageTooLarge => "Message too large",
            KernelError::MessageNotFound => "Message not found",
            KernelError::InvalidSocket => "Invalid socket",
            KernelError::SocketNotFound => "Socket not found",
            KernelError::SocketAlreadyExists => "Socket already exists",
            KernelError::SocketLimitExceeded => "Socket limit exceeded",
            KernelError::SocketIsNotConnected => "Socket is not connected",
            KernelError::SocketIsConnected => "Socket is connected",
            KernelError::SocketIsNotBound => "Socket is not bound",
            KernelError::SocketIsBound => "Socket is bound",
            KernelError::SocketIsNotListening => "Socket is not listening",
            KernelError::SocketIsListening => "Socket is listening",
            KernelError::SocketIsNotClosed => "Socket is not closed",
            KernelError::SocketIsClosed => "Socket is closed",
            KernelError::InvalidAddressFamily => "Invalid address family",
            KernelError::InvalidSocketType => "Invalid socket type",
            KernelError::InvalidProtocol => "Invalid protocol",
            KernelError::AddressInUse => "Address in use",
            KernelError::AddressNotAvailable => "Address not available",
            KernelError::NetworkIsUnreachable => "Network is unreachable",
            KernelError::NetworkIsDown => "Network is down",
            KernelError::ConnectionTimedOut => "Connection timed out",
            KernelError::ConnectionRefused => "Connection refused",
            KernelError::ConnectionResetByPeer => "Connection reset by peer",
            KernelError::ConnectionAborted => "Connection aborted",
            KernelError::HostIsUnreachable => "Host is unreachable",
            KernelError::HostIsDown => "Host is down",
            KernelError::NoRouteToHost => "No route to host",
            KernelError::Unknown => "Unknown error",
        }
    }
}

/// Kernel result type
///
/// This is a convenience type alias for kernel results.
pub type KernelResult<T> = Result<T, KernelError>;

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