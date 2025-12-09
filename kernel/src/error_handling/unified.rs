//! Unified Error Handling
//!
//! 统一错误处理模块
//! 提供统一的错误类型转换和POSIX错误码映射

use crate::syscalls::common::SyscallError;
use crate::reliability::errno;
use alloc::string::String;

/// 统一错误类型
/// 所有内核模块应使用此类型进行错误处理
#[derive(Debug, Clone, PartialEq)]
pub enum KernelError {
    /// 系统调用错误
    Syscall(SyscallError),
    /// 网络错误
    Network(crate::syscalls::net::types::NetworkError),
    /// 内存错误
    OutOfMemory,
    /// 无效参数
    InvalidArgument,
    /// 未找到
    NotFound,
    /// 权限 denied
    PermissionDenied,
    /// I/O错误
    IoError,
    /// 不支持的操作
    NotSupported,
    /// 资源已存在
    AlreadyExists,
    /// 资源忙
    ResourceBusy,
    /// 超时
    Timeout,
    /// 无效地址
    BadAddress,
    /// 无效文件描述符
    BadFileDescriptor,
    /// 不是目录
    NotADirectory,
    /// 文件已存在
    FileExists,
    /// 目录不为空
    DirectoryNotEmpty,
    /// 文件太大
    FileTooBig,
    /// 只读文件系统
    ReadOnlyFilesystem,
    /// 名称太长
    NameTooLong,
    /// 没有缓冲区空间
    NoBufferSpace,
    /// 地址已在使用
    AddressInUse,
    /// 地址不可达
    AddressNotAvailable,
    /// 是目录
    IsADirectory,
    /// 不支持的系统调用
    UnsupportedSyscall,
    /// 服务已存在
    ServiceAlreadyExists,
    /// 服务有依赖项
    ServiceHasDependents,
    /// 服务未找到
    ServiceNotFound,
    /// 依赖项未找到
    DependencyNotFound,
    /// 循环依赖
    CircularDependency,
    /// 系统调用不支持
    SyscallNotSupported,
    /// 服务不可用
    ServiceUnavailable(String),
    /// 超过最大重试次数
    MaxRetriesExceeded(u32),
    /// 未知错误
    Unknown(String),
}

impl From<SyscallError> for KernelError {
    fn from(err: SyscallError) -> Self {
        KernelError::Syscall(err)
    }
}

// 从KernelError转换到SyscallError
impl From<KernelError> for SyscallError {
    fn from(err: KernelError) -> Self {
        match err {
            KernelError::Syscall(e) => e,
            KernelError::OutOfMemory => SyscallError::OutOfMemory,
            KernelError::InvalidArgument => SyscallError::InvalidArgument,
            KernelError::NotFound => SyscallError::NotFound,
            KernelError::PermissionDenied => SyscallError::PermissionDenied,
            KernelError::IoError => SyscallError::IoError,
            KernelError::NotSupported => SyscallError::NotSupported,
            KernelError::AlreadyExists => SyscallError::FileExists,
            KernelError::ResourceBusy => SyscallError::WouldBlock,
            KernelError::Timeout => SyscallError::TimedOut,
            KernelError::BadAddress => SyscallError::BadAddress,
            KernelError::BadFileDescriptor => SyscallError::BadFileDescriptor,
            KernelError::NotADirectory => SyscallError::NotADirectory,
            KernelError::FileExists => SyscallError::FileExists,
            KernelError::DirectoryNotEmpty => SyscallError::DirectoryNotEmpty,
            KernelError::FileTooBig => SyscallError::FileTooBig,
            KernelError::ReadOnlyFilesystem => SyscallError::NoSpaceLeft,
            KernelError::NameTooLong => SyscallError::NameTooLong,
            KernelError::NoBufferSpace => SyscallError::NoBufferSpace,
            KernelError::AddressInUse => SyscallError::InvalidArgument,
            KernelError::AddressNotAvailable => SyscallError::InvalidArgument,
            KernelError::IsADirectory => SyscallError::IsADirectory,
            KernelError::Network(_) => SyscallError::IoError,
            KernelError::UnsupportedSyscall => SyscallError::InvalidSyscall,
            KernelError::ServiceAlreadyExists => SyscallError::FileExists,
            KernelError::ServiceHasDependents => SyscallError::WouldBlock,
            KernelError::ServiceNotFound => SyscallError::NotFound,
            KernelError::DependencyNotFound => SyscallError::NotFound,
            KernelError::CircularDependency => SyscallError::InvalidArgument,
            KernelError::SyscallNotSupported => SyscallError::InvalidSyscall,
            KernelError::ServiceUnavailable(_) => SyscallError::NotSupported,
            KernelError::MaxRetriesExceeded(_) => SyscallError::TimedOut,
            KernelError::Unknown(_) => SyscallError::IoError,
        }
    }
}

// 从process模块错误类型转换
impl From<crate::process::exec::ExecError> for KernelError {
    fn from(err: crate::process::exec::ExecError) -> Self {
        use crate::process::exec::ExecError;
        match err {
            ExecError::FileNotFound => KernelError::NotFound,
            ExecError::FileTooLarge => KernelError::InvalidArgument,
            ExecError::InvalidElf => KernelError::InvalidArgument,
            ExecError::OutOfMemory => KernelError::OutOfMemory,
            ExecError::TooManyArgs => KernelError::InvalidArgument,
            ExecError::ArgTooLong => KernelError::InvalidArgument,
            ExecError::NoProcess => KernelError::NotFound,
            ExecError::PermissionDenied => KernelError::PermissionDenied,
        }
    }
}

impl From<crate::process::thread::ThreadError> for KernelError {
    fn from(err: crate::process::thread::ThreadError) -> Self {
        use crate::process::thread::ThreadError;
        match err {
            ThreadError::InvalidThreadId => KernelError::InvalidArgument,
            ThreadError::NoSlotsAvailable => KernelError::ResourceBusy,
            ThreadError::OutOfMemory => KernelError::OutOfMemory,
            ThreadError::OperationNotPermitted => KernelError::PermissionDenied,
            ThreadError::PermissionDenied => KernelError::PermissionDenied,
            ThreadError::InvalidOperation => KernelError::InvalidArgument,
            ThreadError::ThreadKilled => KernelError::NotFound,
            ThreadError::AlreadyDetached => KernelError::AlreadyExists,
            ThreadError::NotJoinable => KernelError::InvalidArgument,
            ThreadError::ResourceLimitExceeded => KernelError::ResourceBusy,
        }
    }
}

/// Convert VfsError to KernelError
impl From<crate::vfs::error::VfsError> for KernelError {
    fn from(err: crate::vfs::error::VfsError) -> Self {
        use crate::vfs::error::VfsError;
        match err {
            VfsError::NotFound => KernelError::NotFound,
            VfsError::PermissionDenied => KernelError::PermissionDenied,
            VfsError::NotDirectory => KernelError::InvalidArgument,
            VfsError::IsDirectory => KernelError::InvalidArgument,
            VfsError::NotEmpty => KernelError::ResourceBusy,
            VfsError::Exists => KernelError::AlreadyExists,
            VfsError::NoSpace => KernelError::OutOfMemory,
            VfsError::InvalidPath => KernelError::InvalidArgument,
            VfsError::NotMounted => KernelError::IoError,
            VfsError::Busy => KernelError::ResourceBusy,
            VfsError::ReadOnly => KernelError::PermissionDenied,
            VfsError::IoError => KernelError::IoError,
            VfsError::NotSupported => KernelError::NotSupported,
            VfsError::InvalidOperation => KernelError::InvalidArgument,
        }
    }
}

impl KernelError {
    /// 转换为POSIX errno
    pub fn to_errno(self) -> i32 {
        match self {
            KernelError::Syscall(e) => crate::syscalls::common::syscall_error_to_errno(e),
            KernelError::Network(e) => e.error_code(),
            KernelError::OutOfMemory => errno::ENOMEM,
            KernelError::InvalidArgument => errno::EINVAL,
            KernelError::NotFound => errno::ENOENT,
            KernelError::PermissionDenied => errno::EPERM,
            KernelError::IoError => errno::EIO,
            KernelError::NotSupported => errno::EOPNOTSUPP,
            KernelError::AlreadyExists => errno::EEXIST,
            KernelError::ResourceBusy => errno::EBUSY,
            KernelError::Timeout => errno::ETIMEDOUT,
        }
    }

    /// 转换为负数errno（用于系统调用返回值）
    pub fn to_neg_errno(self) -> isize {
        -(self.to_errno() as isize)
    }
}

/// 统一结果类型
pub type KernelResult<T> = Result<T, KernelError>;

/// 从Option转换为Result的辅助函数
pub fn option_to_result<T>(opt: Option<T>, error: KernelError) -> KernelResult<T> {
    opt.ok_or(error)
}

/// 从Option转换为Result，使用NotFound错误
pub fn option_to_result_not_found<T>(opt: Option<T>) -> KernelResult<T> {
    opt.ok_or(KernelError::NotFound)
}

/// 从Option转换为Result，使用OutOfMemory错误
pub fn option_to_result_oom<T>(opt: Option<T>) -> KernelResult<T> {
    opt.ok_or(KernelError::OutOfMemory)
}

