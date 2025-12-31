//! Unified Error Handling Module
//!
//! 统一错误处理模块
//! 提供一致的错误处理机制，包括错误类型定义、转换和传播

use alloc::string::{String, ToString};
use alloc::fmt;

/// 统一内核错误类型
///
/// 这个枚举表示所有可能的内核错误
/// 它为所有内核操作提供统一的错误处理机制
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedError {
    /// 基本错误
    InvalidArgument,
    InvalidAddress,
    InvalidInput,
    InvalidData,
    InvalidState,
    InvalidOperation,
    PermissionDenied,
    NotFound,
    NoProcess,
    NoDevice,
    AlreadyExists,
    AlreadyInProgress,
    FileExists,
    ResourceBusy,
    Busy,
    ResourceUnavailable,
    OutOfMemory,
    OutOfSpace,
    QuotaExceeded,
    NotSupported,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    Interrupted,
    TimedOut,
    WouldBlock,
    IoError,
    BadAddress,
    BadFileDescriptor,
    ConnectionAborted,
    ConnectionReset,
    Unknown,

    /// Resource limit errors
    ResourceLimitExceeded { resource: String, usage: u64, limit: u64 },
    InsufficientResources { resource: String, requested: u64, available: u64 },
    IoQuotaExceeded { operation: String, quota: u64, usage: u64 },
    MemoryLimitExceeded { requested: u64, limit: u64 },

    /// 内存相关错误
    MemoryError(MemoryError),

    /// 文件系统相关错误
    FileSystemError(FileSystemError),

    /// 网络相关错误
    NetworkError(NetworkError),

    /// 进程相关错误
    ProcessError(ProcessError),

    /// 系统调用相关错误
    SyscallError(SyscallError),

    /// 驱动程序相关错误
    DriverError(DriverError),

    /// 安全相关错误
    SecurityError(SecurityError),

    /// 其他错误
    Other(String),
}

impl fmt::Display for UnifiedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnifiedError::InvalidArgument => write!(f, "Invalid argument provided"),
            UnifiedError::InvalidAddress => write!(f, "Invalid memory address"),
            UnifiedError::InvalidInput => write!(f, "Invalid input"),
            UnifiedError::InvalidData => write!(f, "Invalid data"),
            UnifiedError::InvalidState => write!(f, "Invalid state"),
            UnifiedError::InvalidOperation => write!(f, "Invalid operation"),
            UnifiedError::PermissionDenied => write!(f, "Permission denied"),
            UnifiedError::NotFound => write!(f, "Resource not found"),
            UnifiedError::NoProcess => write!(f, "No such process"),
            UnifiedError::NoDevice => write!(f, "No such device"),
            UnifiedError::AlreadyExists => write!(f, "Resource already exists"),
            UnifiedError::AlreadyInProgress => write!(f, "Operation already in progress"),
            UnifiedError::FileExists => write!(f, "File exists"),
            UnifiedError::ResourceBusy => write!(f, "Resource busy"),
            UnifiedError::Busy => write!(f, "Device busy"),
            UnifiedError::ResourceUnavailable => write!(f, "Resource unavailable"),
            UnifiedError::OutOfMemory => write!(f, "Out of memory"),
            UnifiedError::OutOfSpace => write!(f, "No space left on device"),
            UnifiedError::QuotaExceeded => write!(f, "Quota exceeded"),
            UnifiedError::NotSupported => write!(f, "Operation not supported"),
            UnifiedError::NotADirectory => write!(f, "Not a directory"),
            UnifiedError::IsADirectory => write!(f, "Is a directory"),
            UnifiedError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            UnifiedError::Interrupted => write!(f, "Operation interrupted"),
            UnifiedError::TimedOut => write!(f, "Operation timed out"),
            UnifiedError::WouldBlock => write!(f, "Operation would block"),
            UnifiedError::IoError => write!(f, "I/O error"),
            UnifiedError::BadAddress => write!(f, "Bad address"),
            UnifiedError::BadFileDescriptor => write!(f, "Bad file descriptor"),
            UnifiedError::ConnectionAborted => write!(f, "Connection aborted"),
            UnifiedError::ConnectionReset => write!(f, "Connection reset"),
            UnifiedError::Unknown => write!(f, "Unknown error"),
            UnifiedError::ResourceLimitExceeded { resource, usage, limit } => {
                write!(f, "Resource limit exceeded: {} (usage: {}, limit: {})", resource, usage, limit)
            }
            UnifiedError::InsufficientResources { resource, requested, available } => {
                write!(f, "Insufficient resources: {} (requested: {}, available: {})",
                       resource, requested, available)
            }
            UnifiedError::IoQuotaExceeded { operation, quota, usage } => {
                write!(f, "I/O quota exceeded: {} (usage: {}, quota: {})", operation, usage, quota)
            }
            UnifiedError::MemoryLimitExceeded { requested, limit } => {
                write!(f, "Memory limit exceeded (requested: {}, limit: {})", requested, limit)
            }
            UnifiedError::MemoryError(e) => write!(f, "Memory error: {}", e.to_string()),
            UnifiedError::FileSystemError(e) => write!(f, "Filesystem error: {}", e.to_string()),
            UnifiedError::NetworkError(e) => write!(f, "Network error: {}", e.to_string()),
            UnifiedError::ProcessError(e) => write!(f, "Process error: {}", e.to_string()),
            UnifiedError::SyscallError(e) => write!(f, "System call error: {}", e.to_string()),
            UnifiedError::DriverError(e) => write!(f, "Driver error: {}", e.to_string()),
            UnifiedError::SecurityError(e) => write!(f, "Security error: {}", e.to_string()),
            UnifiedError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

/// 内存相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    OutOfMemory,
    InvalidAlignment,
    InvalidSize,
    CorruptedAllocator,
    TooFragmented,
    InvalidAddress,
    InvalidProtection,
    MappingFailed,
    UnmappingFailed,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::OutOfMemory => write!(f, "Out of memory"),
            MemoryError::InvalidAlignment => write!(f, "Invalid memory alignment"),
            MemoryError::InvalidSize => write!(f, "Invalid memory size"),
            MemoryError::CorruptedAllocator => write!(f, "Corrupted allocator"),
            MemoryError::TooFragmented => write!(f, "Memory too fragmented"),
            MemoryError::InvalidAddress => write!(f, "Invalid memory address"),
            MemoryError::InvalidProtection => write!(f, "Invalid memory protection"),
            MemoryError::MappingFailed => write!(f, "Memory mapping failed"),
            MemoryError::UnmappingFailed => write!(f, "Memory unmapping failed"),
        }
    }
}

/// 文件系统相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemError {
    PathNotFound,
    FileNotFound,
    PermissionDenied,
    FileExists,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    InvalidPath,
    PathTooLong,
    FileSystemFull,
    IoError,
    ResourceBusy,
    OperationNotSupported,
    FileSystemCorrupted,
    QuotaExceeded,
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemError::PathNotFound => write!(f, "Path not found"),
            FileSystemError::FileNotFound => write!(f, "File not found"),
            FileSystemError::PermissionDenied => write!(f, "Permission denied"),
            FileSystemError::FileExists => write!(f, "File exists"),
            FileSystemError::NotADirectory => write!(f, "Not a directory"),
            FileSystemError::IsADirectory => write!(f, "Is a directory"),
            FileSystemError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            FileSystemError::InvalidPath => write!(f, "Invalid path"),
            FileSystemError::PathTooLong => write!(f, "Path too long"),
            FileSystemError::FileSystemFull => write!(f, "Filesystem full"),
            FileSystemError::IoError => write!(f, "I/O error"),
            FileSystemError::ResourceBusy => write!(f, "Resource busy"),
            FileSystemError::OperationNotSupported => write!(f, "Operation not supported"),
            FileSystemError::FileSystemCorrupted => write!(f, "Filesystem corrupted"),
            FileSystemError::QuotaExceeded => write!(f, "Quota exceeded"),
        }
    }
}

/// 网络相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    ConnectionRefused,
    ConnectionReset,
    BrokenPipe,
    TimedOut,
    HostUnreachable,
    NetworkUnreachable,
    AddressInUse,
    NoBufferSpace,
    MessageTooLarge,
    ProtocolError,
    NetworkDown,
    ConnectionAborted,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionRefused => write!(f, "Connection refused"),
            NetworkError::ConnectionReset => write!(f, "Connection reset"),
            NetworkError::BrokenPipe => write!(f, "Broken pipe"),
            NetworkError::TimedOut => write!(f, "Connection timed out"),
            NetworkError::HostUnreachable => write!(f, "Host unreachable"),
            NetworkError::NetworkUnreachable => write!(f, "Network unreachable"),
            NetworkError::AddressInUse => write!(f, "Address already in use"),
            NetworkError::NoBufferSpace => write!(f, "No buffer space available"),
            NetworkError::MessageTooLarge => write!(f, "Message too large"),
            NetworkError::ProtocolError => write!(f, "Protocol error"),
            NetworkError::NetworkDown => write!(f, "Network down"),
            NetworkError::ConnectionAborted => write!(f, "Connection aborted"),
        }
    }
}

/// 进程相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    ProcessNotFound,
    PermissionDenied,
    InvalidArgument,
    ResourceLimitExceeded,
    ProcessAlreadyExists,
    ProcessTerminated,
    ProcessNotRunning,
    ProcessKilled,
    InvalidState,
    StackOverflow,
    HeapCorruption,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::ProcessNotFound => write!(f, "Process not found"),
            ProcessError::PermissionDenied => write!(f, "Permission denied"),
            ProcessError::InvalidArgument => write!(f, "Invalid argument"),
            ProcessError::ResourceLimitExceeded => write!(f, "Resource limit exceeded"),
            ProcessError::ProcessAlreadyExists => write!(f, "Process already exists"),
            ProcessError::ProcessTerminated => write!(f, "Process terminated"),
            ProcessError::ProcessNotRunning => write!(f, "Process not running"),
            ProcessError::ProcessKilled => write!(f, "Process killed"),
            ProcessError::InvalidState => write!(f, "Invalid process state"),
            ProcessError::StackOverflow => write!(f, "Stack overflow"),
            ProcessError::HeapCorruption => write!(f, "Heap corruption"),
        }
    }
}

/// 系统调用相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    PermissionDenied,
    InvalidArgument,
    NotFound,
    OutOfMemory,
    Interrupted,
    IoError,
    WouldBlock,
    NotSupported,
    NotImplemented,
    BadFileDescriptor,
    TooManyOpenFiles,
    NoBufferSpace,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    FileExists,
    CrossDeviceLink,
    FileTooBig,
    NoSpaceLeft,
    BadAddress,
    DeadlockWouldOccur,
    NameTooLong,
    TooManySymlinks,
    ConnectionRefused,
    ConnectionReset,
    BrokenPipe,
    TimedOut,
    NoProcess,
    OperationNotPermitted,
}

impl fmt::Display for SyscallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyscallError::InvalidSyscall => write!(f, "Invalid system call"),
            SyscallError::PermissionDenied => write!(f, "Permission denied"),
            SyscallError::InvalidArgument => write!(f, "Invalid argument"),
            SyscallError::NotFound => write!(f, "Not found"),
            SyscallError::OutOfMemory => write!(f, "Out of memory"),
            SyscallError::Interrupted => write!(f, "Interrupted system call"),
            SyscallError::IoError => write!(f, "I/O error"),
            SyscallError::WouldBlock => write!(f, "Operation would block"),
            SyscallError::NotSupported => write!(f, "Operation not supported"),
            SyscallError::NotImplemented => write!(f, "Not implemented"),
            SyscallError::BadFileDescriptor => write!(f, "Bad file descriptor"),
            SyscallError::TooManyOpenFiles => write!(f, "Too many open files"),
            SyscallError::NoBufferSpace => write!(f, "No buffer space available"),
            SyscallError::NotADirectory => write!(f, "Not a directory"),
            SyscallError::IsADirectory => write!(f, "Is a directory"),
            SyscallError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            SyscallError::FileExists => write!(f, "File exists"),
            SyscallError::CrossDeviceLink => write!(f, "Cross-device link"),
            SyscallError::FileTooBig => write!(f, "File too large"),
            SyscallError::NoSpaceLeft => write!(f, "No space left on device"),
            SyscallError::BadAddress => write!(f, "Bad address"),
            SyscallError::DeadlockWouldOccur => write!(f, "Deadlock would occur"),
            SyscallError::NameTooLong => write!(f, "Name too long"),
            SyscallError::TooManySymlinks => write!(f, "Too many symbolic links"),
            SyscallError::ConnectionRefused => write!(f, "Connection refused"),
            SyscallError::ConnectionReset => write!(f, "Connection reset"),
            SyscallError::BrokenPipe => write!(f, "Broken pipe"),
            SyscallError::TimedOut => write!(f, "Operation timed out"),
            SyscallError::NoProcess => write!(f, "No such process"),
            SyscallError::OperationNotPermitted => write!(f, "Operation not permitted"),
        }
    }
}

/// 驱动程序相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    DeviceNotFound,
    DeviceBusy,
    DeviceNotConnected,
    UnsupportedOperation,
    HardwareFailure,
    InvalidConfiguration,
    ResourceConflict,
    Timeout,
    FirmwareMissing,
    DriverNotLoaded,
    InvalidParameter,
    DeviceManager(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::DeviceNotFound => write!(f, "Device not found"),
            DriverError::DeviceBusy => write!(f, "Device busy"),
            DriverError::DeviceNotConnected => write!(f, "Device not connected"),
            DriverError::UnsupportedOperation => write!(f, "Unsupported operation"),
            DriverError::HardwareFailure => write!(f, "Hardware failure"),
            DriverError::InvalidConfiguration => write!(f, "Invalid configuration"),
            DriverError::ResourceConflict => write!(f, "Resource conflict"),
            DriverError::Timeout => write!(f, "Timeout"),
            DriverError::FirmwareMissing => write!(f, "Firmware missing"),
            DriverError::DriverNotLoaded => write!(f, "Driver not loaded"),
            DriverError::InvalidParameter => write!(f, "Invalid parameter"),
            DriverError::DeviceManager(msg) => write!(f, "Device manager error: {}", msg),
        }
    }
}

/// 安全相关错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    AccessDenied,
    PermissionDenied,
    AuthenticationFailed,
    AuthorizationFailed,
    SecurityPolicyViolation,
    InvalidCredentials,
    AccountLocked,
    PasswordExpired,
    AccountDisabled,
    InsufficientPrivileges,
    SecurityBreach,
    AttestationFailed(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::AccessDenied => write!(f, "Access denied"),
            SecurityError::PermissionDenied => write!(f, "Permission denied"),
            SecurityError::AuthenticationFailed => write!(f, "Authentication failed"),
            SecurityError::AuthorizationFailed => write!(f, "Authorization failed"),
            SecurityError::SecurityPolicyViolation => write!(f, "Security policy violation"),
            SecurityError::InvalidCredentials => write!(f, "Invalid credentials"),
            SecurityError::AccountLocked => write!(f, "Account locked"),
            SecurityError::PasswordExpired => write!(f, "Password expired"),
            SecurityError::AccountDisabled => write!(f, "Account disabled"),
            SecurityError::InsufficientPrivileges => write!(f, "Insufficient privileges"),
            SecurityError::SecurityBreach => write!(f, "Security breach"),
            SecurityError::AttestationFailed(msg) => write!(f, "Attestation failed: {}", msg),
        }
    }
}

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Fatal,
}

/// 错误上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 错误类型
    pub error: UnifiedError,
    /// 错误严重级别
    pub severity: ErrorSeverity,
    /// 错误发生位置
    pub location: String,
    /// 错误发生时间
    pub timestamp: u64,
    /// 错误描述
    pub description: String,
    /// 错误原因
    pub cause: Option<String>,
    /// 错误恢复建议
    pub recovery_hint: Option<String>,
}

impl ErrorContext {
    /// 创建新的错误上下文
    pub fn new(error: UnifiedError, location: &str) -> Self {
        let description = error.default_description();
        let recovery_hint = error.default_recovery_hint();
        Self {
            severity: error.default_severity(),
            error,
            location: location.to_string(),
            timestamp: get_timestamp(),
            description,
            cause: None,
            recovery_hint,
        }
    }

    /// 设置错误严重级别
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// 设置错误描述
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// 设置错误原因
    pub fn with_cause(mut self, cause: String) -> Self {
        self.cause = Some(cause);
        self
    }

    /// 设置错误恢复建议
    pub fn with_recovery_hint(mut self, hint: String) -> Self {
        self.recovery_hint = Some(hint);
        self
    }
}

impl UnifiedError {
    /// 获取错误的默认严重级别
    pub fn default_severity(&self) -> ErrorSeverity {
        match self {
            UnifiedError::InvalidArgument | UnifiedError::InvalidAddress | UnifiedError::InvalidInput | UnifiedError::InvalidData | UnifiedError::InvalidOperation => ErrorSeverity::Warning,
            UnifiedError::PermissionDenied | UnifiedError::SecurityError(_) => ErrorSeverity::Error,
            UnifiedError::OutOfMemory | UnifiedError::MemoryError(MemoryError::OutOfMemory) => {
                ErrorSeverity::Critical
            },
            UnifiedError::ResourceBusy | UnifiedError::ResourceUnavailable | UnifiedError::Busy => {
                ErrorSeverity::Warning
            },
            UnifiedError::NotFound | UnifiedError::AlreadyExists | UnifiedError::NoProcess | UnifiedError::NoDevice | UnifiedError::FileExists => ErrorSeverity::Info,
            UnifiedError::MemoryError(_)
            | UnifiedError::FileSystemError(_)
            | UnifiedError::NetworkError(_)
            | UnifiedError::ProcessError(_)
            | UnifiedError::SyscallError(_)
            | UnifiedError::DriverError(_) => ErrorSeverity::Error,
            UnifiedError::Other(_) => ErrorSeverity::Warning,
            UnifiedError::InvalidState => ErrorSeverity::Error,
            UnifiedError::IoError => ErrorSeverity::Error,
            UnifiedError::BadAddress => ErrorSeverity::Error,
            UnifiedError::BadFileDescriptor => ErrorSeverity::Error,
            UnifiedError::ConnectionAborted => ErrorSeverity::Error,
            UnifiedError::ConnectionReset => ErrorSeverity::Error,
            UnifiedError::Interrupted => ErrorSeverity::Warning,
            UnifiedError::TimedOut => ErrorSeverity::Error,
            UnifiedError::WouldBlock => ErrorSeverity::Warning,
            UnifiedError::QuotaExceeded => ErrorSeverity::Error,
            UnifiedError::NotSupported => ErrorSeverity::Warning,
            UnifiedError::NotADirectory => ErrorSeverity::Warning,
            UnifiedError::IsADirectory => ErrorSeverity::Warning,
            UnifiedError::DirectoryNotEmpty => ErrorSeverity::Warning,
            UnifiedError::AlreadyInProgress => ErrorSeverity::Warning,
            UnifiedError::OutOfSpace => ErrorSeverity::Error,
            UnifiedError::Unknown => ErrorSeverity::Error,
            UnifiedError::MemoryLimitExceeded { .. } => ErrorSeverity::Critical,
            UnifiedError::ResourceLimitExceeded { .. } => ErrorSeverity::Error,
            UnifiedError::InsufficientResources { .. } => ErrorSeverity::Critical,
            UnifiedError::IoQuotaExceeded { .. } => ErrorSeverity::Error,
        }
    }

    /// 获取错误的默认描述
    pub fn default_description(&self) -> String {
        match self {
            UnifiedError::InvalidArgument => "Invalid argument provided".to_string(),
            UnifiedError::InvalidAddress => "Invalid memory address".to_string(),
            UnifiedError::InvalidInput => "Invalid input".to_string(),
            UnifiedError::InvalidData => "Invalid data".to_string(),
            UnifiedError::InvalidState => "Invalid state".to_string(),
            UnifiedError::InvalidOperation => "Invalid operation".to_string(),
            UnifiedError::PermissionDenied => "Permission denied".to_string(),
            UnifiedError::NotFound => "Resource not found".to_string(),
            UnifiedError::NoProcess => "No such process".to_string(),
            UnifiedError::NoDevice => "No such device".to_string(),
            UnifiedError::AlreadyExists => "Resource already exists".to_string(),
            UnifiedError::FileExists => "File exists".to_string(),
            UnifiedError::ResourceBusy => "Resource is busy".to_string(),
            UnifiedError::ResourceUnavailable => "Resource is unavailable".to_string(),
            UnifiedError::Busy => "Device busy".to_string(),
            UnifiedError::OutOfMemory => "Out of memory".to_string(),
            UnifiedError::OutOfSpace => "No space left on device".to_string(),
            UnifiedError::QuotaExceeded => "Quota exceeded".to_string(),
            UnifiedError::NotSupported => "Operation not supported".to_string(),
            UnifiedError::NotADirectory => "Not a directory".to_string(),
            UnifiedError::IsADirectory => "Is a directory".to_string(),
            UnifiedError::DirectoryNotEmpty => "Directory not empty".to_string(),
            UnifiedError::Interrupted => "Operation interrupted".to_string(),
            UnifiedError::TimedOut => "Operation timed out".to_string(),
            UnifiedError::WouldBlock => "Operation would block".to_string(),
            UnifiedError::IoError => "I/O error".to_string(),
            UnifiedError::BadAddress => "Bad address".to_string(),
            UnifiedError::BadFileDescriptor => "Bad file descriptor".to_string(),
            UnifiedError::ConnectionAborted => "Connection aborted".to_string(),
            UnifiedError::ConnectionReset => "Connection reset".to_string(),
            UnifiedError::AlreadyInProgress => "Operation already in progress".to_string(),
            UnifiedError::Unknown => "Unknown error".to_string(),
            UnifiedError::MemoryError(err) => format!("Memory error: {:?}", err),
            UnifiedError::FileSystemError(err) => format!("File system error: {:?}", err),
            UnifiedError::NetworkError(err) => format!("Network error: {:?}", err),
            UnifiedError::ProcessError(err) => format!("Process error: {:?}", err),
            UnifiedError::SyscallError(err) => format!("System call error: {:?}", err),
            UnifiedError::DriverError(err) => format!("Driver error: {:?}", err),
            UnifiedError::SecurityError(err) => format!("Security error: {:?}", err),
            UnifiedError::Other(msg) => format!("Other error: {}", msg),
            UnifiedError::ResourceLimitExceeded { resource, usage, limit } => {
                format!("Resource limit exceeded: {} (usage: {}, limit: {})", resource, usage, limit)
            }
            UnifiedError::InsufficientResources { resource, requested, available } => {
                format!("Insufficient resources: {} (requested: {}, available: {})", resource, requested, available)
            }
            UnifiedError::IoQuotaExceeded { operation, quota, usage } => {
                format!("I/O quota exceeded: {} (quota: {}, usage: {})", operation, quota, usage)
            }
            UnifiedError::MemoryLimitExceeded { requested, limit } => {
                format!("Memory limit exceeded: requested {}, limit {}", requested, limit)
            }
        }
    }

    /// 获取错误的默认恢复建议
    pub fn default_recovery_hint(&self) -> Option<String> {
        match self {
            UnifiedError::InvalidArgument => Some("Check the arguments and try again".to_string()),
            UnifiedError::InvalidAddress => {
                Some("Check the memory address and try again".to_string())
            },
            UnifiedError::InvalidInput => Some("Check the input and try again".to_string()),
            UnifiedError::InvalidData => Some("Check the data and try again".to_string()),
            UnifiedError::InvalidState => Some("Check the system state and try again".to_string()),
            UnifiedError::InvalidOperation => Some("Check the operation and try again".to_string()),
            UnifiedError::PermissionDenied => Some("Check permissions and try again".to_string()),
            UnifiedError::NotFound => {
                Some("Check if the resource exists and try again".to_string())
            },
            UnifiedError::NoProcess => {
                Some("Check if the process exists and try again".to_string())
            },
            UnifiedError::NoDevice => {
                Some("Check if the device is connected and try again".to_string())
            },
            UnifiedError::AlreadyExists => {
                Some("Use a different name or delete the existing resource".to_string())
            },
            UnifiedError::FileExists => {
                Some("Use a different name or delete the existing file".to_string())
            },
            UnifiedError::ResourceBusy => {
                Some("Wait for the resource to become available and try again".to_string())
            },
            UnifiedError::ResourceUnavailable => {
                Some("Check if the resource is available and try again".to_string())
            },
            UnifiedError::Busy => {
                Some("Wait for the device to become available and try again".to_string())
            },
            UnifiedError::OutOfMemory => Some("Free up memory and try again".to_string()),
            UnifiedError::OutOfSpace => Some("Free up disk space and try again".to_string()),
            UnifiedError::QuotaExceeded => Some("Increase quota or free up resources and try again".to_string()),
            UnifiedError::NotSupported => Some("Check if the operation is supported and try again".to_string()),
            UnifiedError::NotADirectory => Some("Check the path and try again".to_string()),
            UnifiedError::IsADirectory => Some("Check the path and try again".to_string()),
            UnifiedError::DirectoryNotEmpty => Some("Remove files from the directory and try again".to_string()),
            UnifiedError::Interrupted => Some("Try the operation again".to_string()),
            UnifiedError::TimedOut => Some("Increase timeout or check system resources and try again".to_string()),
            UnifiedError::WouldBlock => Some("Try the operation again when the resource is available".to_string()),
            UnifiedError::IoError => Some("Check hardware connections and try again".to_string()),
            UnifiedError::BadAddress => Some("Check the memory address and try again".to_string()),
            UnifiedError::BadFileDescriptor => Some("Check the file descriptor and try again".to_string()),
            UnifiedError::ConnectionAborted => Some("Check the connection and try again".to_string()),
            UnifiedError::ConnectionReset => Some("Check the connection and try again".to_string()),
            UnifiedError::Unknown => Some("Check system logs and try again".to_string()),
            UnifiedError::MemoryError(MemoryError::OutOfMemory) => {
                Some("Free up memory and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidAlignment) => {
                Some("Check memory alignment and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidSize) => {
                Some("Check memory size and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::CorruptedAllocator) => {
                Some("Restart the system and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::TooFragmented) => {
                Some("Reboot the system and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidAddress) => {
                Some("Check memory address and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::InvalidProtection) => {
                Some("Check memory protection and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::MappingFailed) => {
                Some("Check memory mapping and try again".to_string())
            },
            UnifiedError::MemoryError(MemoryError::UnmappingFailed) => {
                Some("Check memory unmapping and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::PathNotFound) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileNotFound) => {
                Some("Check the file path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::PermissionDenied) => {
                Some("Check file permissions and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileExists) => {
                Some("Use a different name or delete the existing file".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::NotADirectory) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::IsADirectory) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::DirectoryNotEmpty) => {
                Some("Remove files from the directory and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::InvalidPath) => {
                Some("Check the path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::PathTooLong) => {
                Some("Use a shorter path and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileSystemFull) => {
                Some("Free up disk space and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::IoError) => {
                Some("Check storage device and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::ResourceBusy) => {
                Some("Wait for the resource to become available and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::OperationNotSupported) => {
                Some("Check if the operation is supported and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::FileSystemCorrupted) => {
                Some("Check filesystem integrity and try again".to_string())
            },
            UnifiedError::FileSystemError(FileSystemError::QuotaExceeded) => {
                Some("Increase quota or free up resources and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ConnectionRefused) => {
                Some("Check if the service is running and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ConnectionReset) => {
                Some("Check the connection and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::BrokenPipe) => {
                Some("Check the pipe and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::TimedOut) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::HostUnreachable) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::NetworkUnreachable) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::AddressInUse) => {
                Some("Use a different address and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::NoBufferSpace) => {
                Some("Free up buffers and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::MessageTooLarge) => {
                Some("Reduce message size and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ProtocolError) => {
                Some("Check protocol configuration and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::NetworkDown) => {
                Some("Check network connectivity and try again".to_string())
            },
            UnifiedError::NetworkError(NetworkError::ConnectionAborted) => {
                Some("Check the connection and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessNotFound) => {
                Some("Check if the process exists and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::PermissionDenied) => {
                Some("Check process permissions and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::InvalidArgument) => {
                Some("Check arguments and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ResourceLimitExceeded) => {
                Some("Increase resource limits and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessAlreadyExists) => {
                Some("Use a different process name and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessTerminated) => {
                Some("Restart the process and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::ProcessNotRunning) => {
                Some("Start the process and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::InvalidState) => {
                Some("Check process state and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::StackOverflow) => {
                Some("Increase stack size and try again".to_string())
            },
            UnifiedError::ProcessError(ProcessError::HeapCorruption) => {
                Some("Restart the process and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::PermissionDenied) => {
                Some("Check permissions and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::InvalidArgument) => {
                Some("Check arguments and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::NotFound) => {
                Some("Check if the resource exists and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::NoProcess) => {
                Some("Check if the process exists and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::OperationNotPermitted) => {
                Some("Check permissions and try again".to_string())
            },
            UnifiedError::SyscallError(SyscallError::NotImplemented) => {
                Some("Check if the operation is implemented and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DeviceNotFound) => {
                Some("Check if the device is connected and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DeviceBusy) => {
                Some("Wait for the device to become available and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DeviceNotConnected) => {
                Some("Check if the device is connected and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::UnsupportedOperation) => {
                Some("Check if the operation is supported and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::HardwareFailure) => {
                Some("Check hardware and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::InvalidConfiguration) => {
                Some("Check driver configuration and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::ResourceConflict) => {
                Some("Resolve resource conflicts and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::Timeout) => {
                Some("Increase timeout and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::FirmwareMissing) => {
                Some("Install firmware and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::DriverNotLoaded) => {
                Some("Load the driver and try again".to_string())
            },
            UnifiedError::DriverError(DriverError::InvalidParameter) => {
                Some("Check parameters and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AccessDenied) => {
                Some("Check access permissions and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::PermissionDenied) => {
                Some("Check permissions and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AuthenticationFailed) => {
                Some("Check credentials and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AuthorizationFailed) => {
                Some("Check authorization and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::SecurityPolicyViolation) => {
                Some("Check security policy and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::InvalidCredentials) => {
                Some("Check credentials and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AccountLocked) => {
                Some("Contact administrator to unlock account".to_string())
            },
            UnifiedError::SecurityError(SecurityError::PasswordExpired) => {
                Some("Change password and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::AccountDisabled) => {
                Some("Contact administrator to enable account".to_string())
            },
            UnifiedError::SecurityError(SecurityError::InsufficientPrivileges) => {
                Some("Use higher privileges and try again".to_string())
            },
            UnifiedError::SecurityError(SecurityError::SecurityBreach) => {
                Some("Check security logs and try again".to_string())
            },
            UnifiedError::MemoryLimitExceeded { .. } => {
                Some("Free up memory or increase limits and try again".to_string())
            },
            UnifiedError::ResourceLimitExceeded { .. } => {
                Some("Free up resources or increase limits and try again".to_string())
            },
            UnifiedError::InsufficientResources { .. } => {
                Some("Free up resources and try again".to_string())
            },
            UnifiedError::IoQuotaExceeded { .. } => {
                Some("Reduce I/O or increase quota and try again".to_string())
            },
            _ => None,
        }
    }

    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            UnifiedError::InvalidArgument => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidAddress => crate::reliability::errno::EFAULT,
            UnifiedError::InvalidInput => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidData => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidState => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidOperation => crate::reliability::errno::EINVAL,
            UnifiedError::PermissionDenied => crate::reliability::errno::EPERM,
            UnifiedError::NotFound => crate::reliability::errno::ENOENT,
            UnifiedError::NoProcess => crate::reliability::errno::ESRCH,
            UnifiedError::NoDevice => crate::reliability::errno::ENODEV,
            UnifiedError::AlreadyExists => crate::reliability::errno::EEXIST,
            UnifiedError::FileExists => crate::reliability::errno::EEXIST,
            UnifiedError::ResourceBusy => crate::reliability::errno::EBUSY,
            UnifiedError::ResourceUnavailable => crate::reliability::errno::EAGAIN,
            UnifiedError::Busy => crate::reliability::errno::EBUSY,
            UnifiedError::OutOfMemory => crate::reliability::errno::ENOMEM,
            UnifiedError::OutOfSpace => crate::reliability::errno::ENOSPC,
            UnifiedError::QuotaExceeded => crate::reliability::errno::EDQUOT,
            UnifiedError::NotSupported => crate::reliability::errno::EOPNOTSUPP,
            UnifiedError::NotADirectory => crate::reliability::errno::ENOTDIR,
            UnifiedError::IsADirectory => crate::reliability::errno::EISDIR,
            UnifiedError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            UnifiedError::Interrupted => crate::reliability::errno::EINTR,
            UnifiedError::TimedOut => crate::reliability::errno::ETIMEDOUT,
            UnifiedError::WouldBlock => crate::reliability::errno::EAGAIN,
            UnifiedError::IoError => crate::reliability::errno::EIO,
            UnifiedError::BadAddress => crate::reliability::errno::EFAULT,
            UnifiedError::BadFileDescriptor => crate::reliability::errno::EBADF,
            UnifiedError::ConnectionAborted => crate::reliability::errno::ECONNABORTED,
            UnifiedError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            UnifiedError::AlreadyInProgress => crate::reliability::errno::EINPROGRESS,
            UnifiedError::Unknown => crate::reliability::errno::EIO,
            UnifiedError::MemoryError(err) => err.to_errno(),
            UnifiedError::FileSystemError(err) => err.to_errno(),
            UnifiedError::NetworkError(err) => err.to_errno(),
            UnifiedError::ProcessError(err) => err.to_errno(),
            UnifiedError::SyscallError(err) => err.to_errno(),
            UnifiedError::DriverError(err) => err.to_errno(),
            UnifiedError::SecurityError(err) => err.to_errno(),
            UnifiedError::Other(_) => crate::reliability::errno::EIO,
            UnifiedError::MemoryLimitExceeded { .. } => crate::reliability::errno::ENOMEM,
            UnifiedError::ResourceLimitExceeded { .. } => crate::reliability::errno::EAGAIN,
            UnifiedError::InsufficientResources { .. } => crate::reliability::errno::EAGAIN,
            UnifiedError::IoQuotaExceeded { .. } => crate::reliability::errno::EDQUOT,
        }
    }
}

impl MemoryError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            MemoryError::OutOfMemory => crate::reliability::errno::ENOMEM,
            MemoryError::InvalidAlignment => crate::reliability::errno::EINVAL,
            MemoryError::InvalidSize => crate::reliability::errno::EINVAL,
            MemoryError::CorruptedAllocator => crate::reliability::errno::EIO,
            MemoryError::TooFragmented => crate::reliability::errno::ENOMEM,
            MemoryError::InvalidAddress => crate::reliability::errno::EFAULT,
            MemoryError::InvalidProtection => crate::reliability::errno::EINVAL,
            MemoryError::MappingFailed => crate::reliability::errno::ENOMEM,
            MemoryError::UnmappingFailed => crate::reliability::errno::EINVAL,
        }
    }
}

impl FileSystemError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            FileSystemError::PathNotFound => crate::reliability::errno::ENOENT,
            FileSystemError::FileNotFound => crate::reliability::errno::ENOENT,
            FileSystemError::PermissionDenied => crate::reliability::errno::EPERM,
            FileSystemError::FileExists => crate::reliability::errno::EEXIST,
            FileSystemError::NotADirectory => crate::reliability::errno::ENOTDIR,
            FileSystemError::IsADirectory => crate::reliability::errno::EISDIR,
            FileSystemError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            FileSystemError::InvalidPath => crate::reliability::errno::EINVAL,
            FileSystemError::PathTooLong => crate::reliability::errno::ENAMETOOLONG,
            FileSystemError::FileSystemFull => crate::reliability::errno::ENOSPC,
            FileSystemError::IoError => crate::reliability::errno::EIO,
            FileSystemError::ResourceBusy => crate::reliability::errno::EBUSY,
            FileSystemError::OperationNotSupported => crate::reliability::errno::EOPNOTSUPP,
            FileSystemError::FileSystemCorrupted => crate::reliability::errno::EIO,
            FileSystemError::QuotaExceeded => crate::reliability::errno::EDQUOT,
        }
    }
}

impl NetworkError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            NetworkError::ConnectionRefused => crate::reliability::errno::ECONNREFUSED,
            NetworkError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            NetworkError::BrokenPipe => crate::reliability::errno::EPIPE,
            NetworkError::TimedOut => crate::reliability::errno::ETIMEDOUT,
            NetworkError::HostUnreachable => crate::reliability::errno::EHOSTUNREACH,
            NetworkError::NetworkUnreachable => crate::reliability::errno::ENETUNREACH,
            NetworkError::AddressInUse => crate::reliability::errno::EADDRINUSE,
            NetworkError::NoBufferSpace => crate::reliability::errno::ENOBUFS,
            NetworkError::MessageTooLarge => crate::reliability::errno::EMSGSIZE,
            NetworkError::ProtocolError => crate::reliability::errno::EPROTO,
            NetworkError::NetworkDown => crate::reliability::errno::ENETDOWN,
            NetworkError::ConnectionAborted => crate::reliability::errno::ECONNABORTED,
        }
    }
}

impl ProcessError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            ProcessError::ProcessNotFound => crate::reliability::errno::ESRCH,
            ProcessError::PermissionDenied => crate::reliability::errno::EPERM,
            ProcessError::InvalidArgument => crate::reliability::errno::EINVAL,
            ProcessError::ResourceLimitExceeded => crate::reliability::errno::EAGAIN,
            ProcessError::ProcessAlreadyExists => crate::reliability::errno::EEXIST,
            ProcessError::ProcessTerminated => crate::reliability::errno::ESRCH,
            ProcessError::ProcessNotRunning => crate::reliability::errno::ESRCH,
            ProcessError::InvalidState => crate::reliability::errno::EINVAL,
            ProcessError::StackOverflow => crate::reliability::errno::ENOMEM,
            ProcessError::HeapCorruption => crate::reliability::errno::EIO,
            ProcessError::ProcessKilled => crate::reliability::errno::ESRCH,
        }
    }
}

impl SyscallError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            SyscallError::InvalidSyscall => crate::reliability::errno::ENOSYS,
            SyscallError::PermissionDenied => crate::reliability::errno::EPERM,
            SyscallError::InvalidArgument => crate::reliability::errno::EINVAL,
            SyscallError::NotFound => crate::reliability::errno::ENOENT,
            SyscallError::OutOfMemory => crate::reliability::errno::ENOMEM,
            SyscallError::Interrupted => crate::reliability::errno::EINTR,
            SyscallError::IoError => crate::reliability::errno::EIO,
            SyscallError::WouldBlock => crate::reliability::errno::EAGAIN,
            SyscallError::NotSupported => crate::reliability::errno::EOPNOTSUPP,
            SyscallError::NotImplemented => crate::reliability::errno::ENOSYS,
            SyscallError::BadFileDescriptor => crate::reliability::errno::EBADF,
            SyscallError::TooManyOpenFiles => crate::reliability::errno::EMFILE,
            SyscallError::NoBufferSpace => crate::reliability::errno::ENOBUFS,
            SyscallError::NotADirectory => crate::reliability::errno::ENOTDIR,
            SyscallError::IsADirectory => crate::reliability::errno::EISDIR,
            SyscallError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            SyscallError::FileExists => crate::reliability::errno::EEXIST,
            SyscallError::CrossDeviceLink => crate::reliability::errno::EXDEV,
            SyscallError::FileTooBig => crate::reliability::errno::EFBIG,
            SyscallError::NoSpaceLeft => crate::reliability::errno::ENOSPC,
            SyscallError::BadAddress => crate::reliability::errno::EFAULT,
            SyscallError::DeadlockWouldOccur => crate::reliability::errno::EDEADLK,
            SyscallError::NameTooLong => crate::reliability::errno::ENAMETOOLONG,
            SyscallError::TooManySymlinks => crate::reliability::errno::ELOOP,
            SyscallError::ConnectionRefused => crate::reliability::errno::ECONNREFUSED,
            SyscallError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            SyscallError::BrokenPipe => crate::reliability::errno::EPIPE,
            SyscallError::TimedOut => crate::reliability::errno::ETIMEDOUT,
            SyscallError::NoProcess => crate::reliability::errno::ESRCH,
            SyscallError::OperationNotPermitted => crate::reliability::errno::EPERM,
        }
    }
}

impl DriverError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            DriverError::DeviceNotFound => crate::reliability::errno::ENODEV,
            DriverError::DeviceBusy => crate::reliability::errno::EBUSY,
            DriverError::DeviceNotConnected => crate::reliability::errno::ENXIO,
            DriverError::UnsupportedOperation => crate::reliability::errno::EOPNOTSUPP,
            DriverError::HardwareFailure => crate::reliability::errno::EIO,
            DriverError::InvalidConfiguration => crate::reliability::errno::EINVAL,
            DriverError::ResourceConflict => crate::reliability::errno::EBUSY,
            DriverError::Timeout => crate::reliability::errno::ETIMEDOUT,
            DriverError::FirmwareMissing => crate::reliability::errno::ENOENT,
            DriverError::DriverNotLoaded => crate::reliability::errno::ENODEV,
            DriverError::InvalidParameter => crate::reliability::errno::EINVAL,
            DriverError::DeviceManager(_) => crate::reliability::errno::EIO,
        }
    }
}

impl SecurityError {
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            SecurityError::AccessDenied => crate::reliability::errno::EACCES,
            SecurityError::PermissionDenied => crate::reliability::errno::EPERM,
            SecurityError::AuthenticationFailed => crate::reliability::errno::EACCES,
            SecurityError::AuthorizationFailed => crate::reliability::errno::EACCES,
            SecurityError::SecurityPolicyViolation => crate::reliability::errno::EPERM,
            SecurityError::InvalidCredentials => crate::reliability::errno::EACCES,
            SecurityError::AccountLocked => crate::reliability::errno::EACCES,
            SecurityError::PasswordExpired => crate::reliability::errno::EACCES,
            SecurityError::AccountDisabled => crate::reliability::errno::EACCES,
            SecurityError::InsufficientPrivileges => crate::reliability::errno::EPERM,
            SecurityError::SecurityBreach => crate::reliability::errno::EACCES,
            SecurityError::AttestationFailed(_) => crate::reliability::errno::EACCES,
        }
    }
}

/// 获取当前时间戳（纳秒）
fn get_timestamp() -> u64 {
    // 在实际实现中，这应该从系统时钟获取
    // 这里使用一个简单的实现
    0
}

/// 统一结果类型
pub type UnifiedResult<T> = core::result::Result<T, UnifiedError>;

/// 错误处理宏
/// 用于创建带有位置信息的错误上下文
#[macro_export]
macro_rules! create_error {
    ($error:expr) => {
        ErrorContext::new($error, module_path!())
    };
    ($error:expr, $severity:expr) => {
        ErrorContext::new($error, module_path!()).with_severity($severity)
    };
    ($error:expr, $description:expr, $cause:expr) => {
        ErrorContext::new($error, module_path!())
            .with_description($description.to_string())
            .with_cause($cause.to_string())
    };
}

/// 错误处理宏
/// 用于返回带有位置信息的错误
#[macro_export]
macro_rules! return_error {
    ($error:expr) => {
        return Err(UnifiedError::from($error));
    };
    ($error:expr, $severity:expr) => {
        return Err(UnifiedError::from($error).with_severity($severity));
    };
    ($error:expr, $description:expr, $cause:expr) => {
        return Err(UnifiedError::from($error)
            .with_description($description.to_string())
            .with_cause($cause.to_string()));
    };
}

/// 从其他错误类型转换为统一错误
impl From<crate::subsystems::syscalls::api::SyscallError> for UnifiedError {
    fn from(err: crate::subsystems::syscalls::api::SyscallError) -> Self {
        UnifiedError::SyscallError(SyscallError::from(err))
    }
}

/// 从 syscalls::api::SyscallError 转换为 error::unified::SyscallError
impl From<crate::subsystems::syscalls::api::SyscallError> for SyscallError {
    fn from(err: crate::subsystems::syscalls::api::SyscallError) -> Self {
        match err {
            crate::subsystems::syscalls::api::SyscallError::InvalidArgument => SyscallError::InvalidArgument,
            crate::subsystems::syscalls::api::SyscallError::PermissionDenied => SyscallError::PermissionDenied,
            crate::subsystems::syscalls::api::SyscallError::NotFound => SyscallError::NotFound,
            crate::subsystems::syscalls::api::SyscallError::IoError => SyscallError::IoError,
            crate::subsystems::syscalls::api::SyscallError::NoProcess => SyscallError::NoProcess,
            crate::subsystems::syscalls::api::SyscallError::OperationNotPermitted => SyscallError::OperationNotPermitted,
            crate::subsystems::syscalls::api::SyscallError::NotImplemented => SyscallError::NotImplemented,
        }
    }
}

/// 从 error::unified::SyscallError 转换为 UnifiedError
impl From<SyscallError> for UnifiedError {
    fn from(err: SyscallError) -> Self {
        UnifiedError::SyscallError(err)
    }
}

/// 从系统调用分发器错误转换为统一错误
impl From<crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError> for UnifiedError {
    fn from(err: crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError) -> Self {
        match err {
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::SyscallNotSupported(_) => {
                UnifiedError::NotSupported
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::ServiceUnavailable(_) => {
                UnifiedError::ResourceUnavailable
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::MaxRetriesExceeded(_) => {
                UnifiedError::InvalidOperation
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::CacheError(_) => {
                UnifiedError::IoError
            }
            crate::subsystems::syscalls::dispatch::dispatcher::DispatcherError::InvalidParameters(_) => {
                UnifiedError::InvalidArgument
            }
        }
    }
}

impl From<crate::subsystems::fs::api::error::FsError> for UnifiedError {
    fn from(err: crate::subsystems::fs::api::error::FsError) -> Self {
        UnifiedError::FileSystemError(FileSystemError::from(err))
    }
}

impl From<crate::subsystems::mm::api::AllocError> for UnifiedError {
    fn from(err: crate::subsystems::mm::api::AllocError) -> Self {
        UnifiedError::MemoryError(MemoryError::from(err))
    }
}

impl From<crate::subsystems::mm::api::VmError> for UnifiedError {
    fn from(err: crate::subsystems::mm::api::VmError) -> Self {
        UnifiedError::MemoryError(MemoryError::from(err))
    }
}


/// 从文件系统错误转换为统一错误中的文件系统错误
impl From<crate::subsystems::fs::api::error::FsError> for FileSystemError {
    fn from(err: crate::subsystems::fs::api::error::FsError) -> Self {
        match err {
            crate::subsystems::fs::api::error::FsError::PathNotFound => {
                FileSystemError::PathNotFound
            },
            crate::subsystems::fs::api::error::FsError::FileNotFound => {
                FileSystemError::FileNotFound
            },
            crate::subsystems::fs::api::error::FsError::NotFound => {
                FileSystemError::PathNotFound
            },
            crate::subsystems::fs::api::error::FsError::PermissionDenied => {
                FileSystemError::PermissionDenied
            },
            crate::subsystems::fs::api::error::FsError::FileExists => FileSystemError::FileExists,
            crate::subsystems::fs::api::error::FsError::Exists => FileSystemError::FileExists,
            crate::subsystems::fs::api::error::FsError::NotADirectory => {
                FileSystemError::NotADirectory
            },
            crate::subsystems::fs::api::error::FsError::IsADirectory => {
                FileSystemError::IsADirectory
            },
            crate::subsystems::fs::api::error::FsError::DirectoryNotEmpty => {
                FileSystemError::DirectoryNotEmpty
            },
            crate::subsystems::fs::api::error::FsError::NotEmpty => {
                FileSystemError::DirectoryNotEmpty
            },
            crate::subsystems::fs::api::error::FsError::InvalidPath => FileSystemError::InvalidPath,
            crate::subsystems::fs::api::error::FsError::InvalidInput => FileSystemError::InvalidPath,
            crate::subsystems::fs::api::error::FsError::PathTooLong => FileSystemError::PathTooLong,
            crate::subsystems::fs::api::error::FsError::FileSystemFull => {
                FileSystemError::FileSystemFull
            },
            crate::subsystems::fs::api::error::FsError::NoSpace => {
                FileSystemError::FileSystemFull
            },
            crate::subsystems::fs::api::error::FsError::IoError => FileSystemError::IoError,
            crate::subsystems::fs::api::error::FsError::ResourceBusy => {
                FileSystemError::ResourceBusy
            },
            crate::subsystems::fs::api::error::FsError::Busy => {
                FileSystemError::ResourceBusy
            },
            crate::subsystems::fs::api::error::FsError::OperationNotSupported => {
                FileSystemError::OperationNotSupported
            },
            crate::subsystems::fs::api::error::FsError::NotSupported => {
                FileSystemError::OperationNotSupported
            },
            crate::subsystems::fs::api::error::FsError::QuotaExceeded => {
                FileSystemError::QuotaExceeded
            },
            crate::subsystems::fs::api::error::FsError::NotMounted => {
                FileSystemError::PathNotFound
            },
            crate::subsystems::fs::api::error::FsError::ReadOnly => {
                FileSystemError::IoError
            },
            crate::subsystems::fs::api::error::FsError::InvalidOperation => {
                FileSystemError::InvalidPath
            },
            crate::subsystems::fs::api::error::FsError::Loop => {
                FileSystemError::InvalidPath
            },
            crate::subsystems::fs::api::error::FsError::TooManyLinks => {
                FileSystemError::PathTooLong
            },
        }
    }
}

/// 从内存分配错误转换为统一错误中的内存错误
impl From<crate::subsystems::mm::api::AllocError> for MemoryError {
    fn from(err: crate::subsystems::mm::api::AllocError) -> Self {
        match err {
            crate::subsystems::mm::api::AllocError::OutOfMemory => MemoryError::OutOfMemory,
            crate::subsystems::mm::api::AllocError::InvalidAlignment => {
                MemoryError::InvalidAlignment
            },
            crate::subsystems::mm::api::AllocError::InvalidSize => MemoryError::InvalidSize,
            crate::subsystems::mm::api::AllocError::CorruptedAllocator => {
                MemoryError::CorruptedAllocator
            },
            crate::subsystems::mm::api::AllocError::TooFragmented => MemoryError::TooFragmented,
        }
    }
}

/// 从虚拟内存错误转换为统一错误中的内存错误
impl From<crate::subsystems::mm::api::VmError> for MemoryError {
    fn from(err: crate::subsystems::mm::api::VmError) -> Self {
        match err {
            crate::subsystems::mm::api::VmError::InvalidAddress => MemoryError::InvalidAddress,
            crate::subsystems::mm::api::VmError::InvalidSize => MemoryError::InvalidSize,
            crate::subsystems::mm::api::VmError::InvalidProtection => {
                MemoryError::InvalidProtection
            },
            crate::subsystems::mm::api::VmError::MappingNotFound => MemoryError::InvalidAddress,
            crate::subsystems::mm::api::VmError::PermissionDenied => MemoryError::InvalidProtection,
            crate::subsystems::mm::api::VmError::AddressAlreadyMapped => MemoryError::InvalidAddress,
            crate::subsystems::mm::api::VmError::PageTableError => MemoryError::CorruptedAllocator,
            crate::subsystems::mm::api::VmError::TLBError => MemoryError::InvalidAddress,
        }
    }
}

/// 从 Attestation 错误转换为统一错误
impl From<crate::security::attestation::AttestationError> for UnifiedError {
    fn from(err: crate::security::attestation::AttestationError) -> Self {
        UnifiedError::SecurityError(SecurityError::AttestationFailed(format!("{:?}", err)))
    }
}
