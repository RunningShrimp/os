//! Unified Error Handling Module
//!
//! 统一错误处理模块
//! 提供一致的错误处理机制，包括错误类型定义、转换和传播

extern crate alloc;

use alloc::string::String;

/// 统一内核错误类型
///
/// 这个枚举表示所有可能的内核错误
/// 它为所有内核操作提供统一的错误处理机制
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnifiedError {
    /// 基本错误
    InvalidArgument,
    InvalidAddress,
    PermissionDenied,
    NotFound,
    AlreadyExists,
    ResourceBusy,
    ResourceUnavailable,
    OutOfMemory,
    
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
    InvalidState,
    StackOverflow,
    HeapCorruption,
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
        Self {
            severity: error.default_severity(),
            error,
            location: location.to_string(),
            timestamp: get_timestamp(),
            description: error.default_description(),
            cause: None,
            recovery_hint: error.default_recovery_hint(),
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
            UnifiedError::InvalidArgument | UnifiedError::InvalidAddress => ErrorSeverity::Warning,
            UnifiedError::PermissionDenied | UnifiedError::SecurityError(_) => ErrorSeverity::Error,
            UnifiedError::OutOfMemory | UnifiedError::MemoryError(MemoryError::OutOfMemory) => ErrorSeverity::Critical,
            UnifiedError::ResourceBusy | UnifiedError::ResourceUnavailable => ErrorSeverity::Warning,
            UnifiedError::NotFound | UnifiedError::AlreadyExists => ErrorSeverity::Info,
            UnifiedError::MemoryError(_) | UnifiedError::FileSystemError(_) | 
            UnifiedError::NetworkError(_) | UnifiedError::ProcessError(_) |
            UnifiedError::SyscallError(_) | UnifiedError::DriverError(_) => ErrorSeverity::Error,
            UnifiedError::Other(_) => ErrorSeverity::Warning,
        }
    }
    
    /// 获取错误的默认描述
    pub fn default_description(&self) -> String {
        match self {
            UnifiedError::InvalidArgument => "Invalid argument provided".to_string(),
            UnifiedError::InvalidAddress => "Invalid memory address".to_string(),
            UnifiedError::PermissionDenied => "Permission denied".to_string(),
            UnifiedError::NotFound => "Resource not found".to_string(),
            UnifiedError::AlreadyExists => "Resource already exists".to_string(),
            UnifiedError::ResourceBusy => "Resource is busy".to_string(),
            UnifiedError::ResourceUnavailable => "Resource is unavailable".to_string(),
            UnifiedError::OutOfMemory => "Out of memory".to_string(),
            UnifiedError::MemoryError(err) => format!("Memory error: {:?}", err),
            UnifiedError::FileSystemError(err) => format!("File system error: {:?}", err),
            UnifiedError::NetworkError(err) => format!("Network error: {:?}", err),
            UnifiedError::ProcessError(err) => format!("Process error: {:?}", err),
            UnifiedError::SyscallError(err) => format!("System call error: {:?}", err),
            UnifiedError::DriverError(err) => format!("Driver error: {:?}", err),
            UnifiedError::SecurityError(err) => format!("Security error: {:?}", err),
            UnifiedError::Other(msg) => format!("Other error: {}", msg),
        }
    }
    
    /// 获取错误的默认恢复建议
    pub fn default_recovery_hint(&self) -> Option<String> {
        match self {
            UnifiedError::InvalidArgument => Some("Check the arguments and try again".to_string()),
            UnifiedError::InvalidAddress => Some("Check the memory address and try again".to_string()),
            UnifiedError::PermissionDenied => Some("Check permissions and try again".to_string()),
            UnifiedError::NotFound => Some("Check if the resource exists and try again".to_string()),
            UnifiedError::AlreadyExists => Some("Use a different name or delete the existing resource".to_string()),
            UnifiedError::ResourceBusy => Some("Wait for the resource to become available and try again".to_string()),
            UnifiedError::ResourceUnavailable => Some("Check if the resource is available and try again".to_string()),
            UnifiedError::OutOfMemory => Some("Free up memory and try again".to_string()),
            UnifiedError::MemoryError(MemoryError::OutOfMemory) => Some("Free up memory and try again".to_string()),
            UnifiedError::MemoryError(MemoryError::InvalidAlignment) => Some("Check memory alignment and try again".to_string()),
            UnifiedError::MemoryError(MemoryError::InvalidSize) => Some("Check memory size and try again".to_string()),
            UnifiedError::FileSystemError(FileSystemError::PermissionDenied) => Some("Check file permissions and try again".to_string()),
            UnifiedError::FileSystemError(FileSystemError::PathNotFound) => Some("Check the path and try again".to_string()),
            UnifiedError::FileSystemError(FileSystemError::FileSystemFull) => Some("Free up disk space and try again".to_string()),
            UnifiedError::NetworkError(NetworkError::ConnectionRefused) => Some("Check if the service is running and try again".to_string()),
            UnifiedError::NetworkError(NetworkError::HostUnreachable) => Some("Check network connectivity and try again".to_string()),
            UnifiedError::ProcessError(ProcessError::PermissionDenied) => Some("Check process permissions and try again".to_string()),
            UnifiedError::ProcessError(ProcessError::ProcessNotFound) => Some("Check if the process exists and try again".to_string()),
            UnifiedError::SyscallError(SyscallError::PermissionDenied) => Some("Check permissions and try again".to_string()),
            UnifiedError::SyscallError(SyscallError::InvalidArgument) => Some("Check arguments and try again".to_string()),
            UnifiedError::DriverError(DriverError::DeviceNotFound) => Some("Check if the device is connected and try again".to_string()),
            UnifiedError::DriverError(DriverError::DeviceBusy) => Some("Wait for the device to become available and try again".to_string()),
            UnifiedError::SecurityError(SecurityError::AccessDenied) => Some("Check access permissions and try again".to_string()),
            UnifiedError::SecurityError(SecurityError::AuthenticationFailed) => Some("Check credentials and try again".to_string()),
            _ => None,
        }
    }
    
    /// 转换为POSIX错误代码
    pub fn to_errno(&self) -> i32 {
        match self {
            UnifiedError::InvalidArgument => crate::reliability::errno::EINVAL,
            UnifiedError::InvalidAddress => crate::reliability::errno::EFAULT,
            UnifiedError::PermissionDenied => crate::reliability::errno::EPERM,
            UnifiedError::NotFound => crate::reliability::errno::ENOENT,
            UnifiedError::AlreadyExists => crate::reliability::errno::EEXIST,
            UnifiedError::ResourceBusy => crate::reliability::errno::EBUSY,
            UnifiedError::ResourceUnavailable => crate::reliability::errno::EAGAIN,
            UnifiedError::OutOfMemory => crate::reliability::errno::ENOMEM,
            UnifiedError::MemoryError(err) => err.to_errno(),
            UnifiedError::FileSystemError(err) => err.to_errno(),
            UnifiedError::NetworkError(err) => err.to_errno(),
            UnifiedError::ProcessError(err) => err.to_errno(),
            UnifiedError::SyscallError(err) => err.to_errno(),
            UnifiedError::DriverError(err) => err.to_errno(),
            UnifiedError::SecurityError(err) => err.to_errno(),
            UnifiedError::Other(_) => crate::reliability::errno::EIO,
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
impl From<crate::subsystems::syscalls::common::SyscallError> for UnifiedError {
    fn from(err: crate::subsystems::syscalls::common::SyscallError) -> Self {
        UnifiedError::SyscallError(SyscallError::from(err))
    }
}

impl From<crate::subsystems::fs::api::error::FsError> for UnifiedError {
    fn from(err: crate::subsystems::fs::api::error::FsError) -> Self {
        UnifiedError::FileSystemError(FileSystemError::from(err))
    }
}

impl From<crate::mm::api::AllocError> for UnifiedError {
    fn from(err: crate::mm::api::AllocError) -> Self {
        UnifiedError::MemoryError(MemoryError::from(err))
    }
}

impl From<crate::mm::api::VmError> for UnifiedError {
    fn from(err: crate::mm::api::VmError) -> Self {
        UnifiedError::MemoryError(MemoryError::from(err))
    }
}

/// 从系统调用错误转换为统一错误中的系统调用错误
impl From<crate::subsystems::syscalls::common::SyscallError> for SyscallError {
    fn from(err: crate::subsystems::syscalls::common::SyscallError) -> Self {
        match err {
            crate::subsystems::syscalls::common::SyscallError::InvalidSyscall => SyscallError::InvalidSyscall,
            crate::subsystems::syscalls::common::SyscallError::PermissionDenied => SyscallError::PermissionDenied,
            crate::subsystems::syscalls::common::SyscallError::InvalidArgument => SyscallError::InvalidArgument,
            crate::subsystems::syscalls::common::SyscallError::NotFound => SyscallError::NotFound,
            crate::subsystems::syscalls::common::SyscallError::OutOfMemory => SyscallError::OutOfMemory,
            crate::subsystems::syscalls::common::SyscallError::Interrupted => SyscallError::Interrupted,
            crate::subsystems::syscalls::common::SyscallError::IoError => SyscallError::IoError,
            crate::subsystems::syscalls::common::SyscallError::WouldBlock => SyscallError::WouldBlock,
            crate::subsystems::syscalls::common::SyscallError::NotSupported => SyscallError::NotSupported,
            crate::subsystems::syscalls::common::SyscallError::BadFileDescriptor => SyscallError::BadFileDescriptor,
            crate::subsystems::syscalls::common::SyscallError::TooManyOpenFiles => SyscallError::TooManyOpenFiles,
            crate::subsystems::syscalls::common::SyscallError::NoBufferSpace => SyscallError::NoBufferSpace,
            crate::subsystems::syscalls::common::SyscallError::NotADirectory => SyscallError::NotADirectory,
            crate::subsystems::syscalls::common::SyscallError::IsADirectory => SyscallError::IsADirectory,
            crate::subsystems::syscalls::common::SyscallError::DirectoryNotEmpty => SyscallError::DirectoryNotEmpty,
            crate::subsystems::syscalls::common::SyscallError::FileExists => SyscallError::FileExists,
            crate::subsystems::syscalls::common::SyscallError::CrossDeviceLink => SyscallError::CrossDeviceLink,
            crate::subsystems::syscalls::common::SyscallError::FileTooBig => SyscallError::FileTooBig,
            crate::subsystems::syscalls::common::SyscallError::NoSpaceLeft => SyscallError::NoSpaceLeft,
            crate::subsystems::syscalls::common::SyscallError::BadAddress => SyscallError::BadAddress,
            crate::subsystems::syscalls::common::SyscallError::DeadlockWouldOccur => SyscallError::DeadlockWouldOccur,
            crate::subsystems::syscalls::common::SyscallError::NameTooLong => SyscallError::NameTooLong,
            crate::subsystems::syscalls::common::SyscallError::TooManySymlinks => SyscallError::TooManySymlinks,
            crate::subsystems::syscalls::common::SyscallError::ConnectionRefused => SyscallError::ConnectionRefused,
            crate::subsystems::syscalls::common::SyscallError::ConnectionReset => SyscallError::ConnectionReset,
            crate::subsystems::syscalls::common::SyscallError::BrokenPipe => SyscallError::BrokenPipe,
            crate::subsystems::syscalls::common::SyscallError::TimedOut => SyscallError::TimedOut,
        }
    }
}

/// 从文件系统错误转换为统一错误中的文件系统错误
impl From<crate::subsystems::fs::api::error::FsError> for FileSystemError {
    fn from(err: crate::subsystems::fs::api::error::FsError) -> Self {
        match err {
            crate::subsystems::fs::api::error::FsError::PathNotFound => FileSystemError::PathNotFound,
            crate::subsystems::fs::api::error::FsError::FileNotFound => FileSystemError::FileNotFound,
            crate::subsystems::fs::api::error::FsError::PermissionDenied => FileSystemError::PermissionDenied,
            crate::subsystems::fs::api::error::FsError::FileExists => FileSystemError::FileExists,
            crate::subsystems::fs::api::error::FsError::NotADirectory => FileSystemError::NotADirectory,
            crate::subsystems::fs::api::error::FsError::IsADirectory => FileSystemError::IsADirectory,
            crate::subsystems::fs::api::error::FsError::DirectoryNotEmpty => FileSystemError::DirectoryNotEmpty,
            crate::subsystems::fs::api::error::FsError::InvalidPath => FileSystemError::InvalidPath,
            crate::subsystems::fs::api::error::FsError::PathTooLong => FileSystemError::PathTooLong,
            crate::subsystems::fs::api::error::FsError::FileSystemFull => FileSystemError::FileSystemFull,
            crate::subsystems::fs::api::error::FsError::IoError => FileSystemError::IoError,
            crate::subsystems::fs::api::error::FsError::ResourceBusy => FileSystemError::ResourceBusy,
            crate::subsystems::fs::api::error::FsError::OperationNotSupported => FileSystemError::OperationNotSupported,
            _ => FileSystemError::IoError,
        }
    }
}

/// 从内存分配错误转换为统一错误中的内存错误
impl From<crate::mm::api::AllocError> for MemoryError {
    fn from(err: crate::mm::api::AllocError) -> Self {
        match err {
            crate::mm::api::AllocError::OutOfMemory => MemoryError::OutOfMemory,
            crate::mm::api::AllocError::InvalidAlignment => MemoryError::InvalidAlignment,
            crate::mm::api::AllocError::InvalidSize => MemoryError::InvalidSize,
            crate::mm::api::AllocError::CorruptedAllocator => MemoryError::CorruptedAllocator,
            crate::mm::api::AllocError::TooFragmented => MemoryError::TooFragmented,
        }
    }
}

/// 从虚拟内存错误转换为统一错误中的内存错误
impl From<crate::mm::api::VmError> for MemoryError {
    fn from(err: crate::mm::api::VmError) -> Self {
        match err {
            crate::mm::api::VmError::InvalidAddress => MemoryError::InvalidAddress,
            crate::mm::api::VmError::InvalidSize => MemoryError::InvalidSize,
            crate::mm::api::VmError::InvalidProtection => MemoryError::InvalidProtection,
            crate::mm::api::VmError::MappingFailed => MemoryError::MappingFailed,
            crate::mm::api::VmError::UnmappingFailed => MemoryError::UnmappingFailed,
            _ => MemoryError::InvalidAddress,
        }
    }
}