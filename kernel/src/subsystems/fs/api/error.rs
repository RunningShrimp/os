//! File system error definitions for the public API.
//!
//! This module contains all public file system error types.

extern crate alloc;

/// File system error type.
///
/// 统一的文件系统错误处理
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    /// 路径不存在
    PathNotFound,
    /// 文件不存在
    FileNotFound,
    NotFound,
    /// 权限被拒绝
    PermissionDenied,
    /// 文件已存在
    FileExists,
    Exists,
    /// 不是目录
    NotADirectory,
    /// 是目录
    IsADirectory,
    /// 目录非空
    DirectoryNotEmpty,
    NotEmpty,
    /// 无效路径
    InvalidPath,
    InvalidInput,
    /// 路径过长
    PathTooLong,
    /// 文件系统已满
    FileSystemFull,
    NoSpace,
    /// 输入/输出错误
    IoError,
    /// 文件系统不支持操作
    OperationNotSupported,
    NotSupported,
    /// 资源忙
    ResourceBusy,
    Busy,
    /// 配额超限
    QuotaExceeded,
    /// 文件系统未挂载
    NotMounted,
    /// 只读文件系统
    ReadOnly,
    /// 无效操作
    InvalidOperation,
    /// 循环链接
    Loop,
    /// 符号链接循环
    TooManyLinks,
}

impl FsError {
    /// 转换为POSIX错误码
    pub fn to_errno(&self) -> i32 {
        match self {
            FsError::PathNotFound => crate::reliability::errno::ENOENT,
            FsError::FileNotFound => crate::reliability::errno::ENOENT,
            FsError::NotFound => crate::reliability::errno::ENOENT,
            FsError::PermissionDenied => crate::reliability::errno::EACCES,
            FsError::FileExists => crate::reliability::errno::EEXIST,
            FsError::Exists => crate::reliability::errno::EEXIST,
            FsError::NotADirectory => crate::reliability::errno::ENOTDIR,
            FsError::IsADirectory => crate::reliability::errno::EISDIR,
            FsError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            FsError::NotEmpty => crate::reliability::errno::ENOTEMPTY,
            FsError::InvalidPath => crate::reliability::errno::EINVAL,
            FsError::InvalidInput => crate::reliability::errno::EINVAL,
            FsError::PathTooLong => crate::reliability::errno::ENAMETOOLONG,
            FsError::FileSystemFull => crate::reliability::errno::ENOSPC,
            FsError::NoSpace => crate::reliability::errno::ENOSPC,
            FsError::IoError => crate::reliability::errno::EIO,
            FsError::OperationNotSupported => crate::reliability::errno::EOPNOTSUPP,
            FsError::NotSupported => crate::reliability::errno::EOPNOTSUPP,
            FsError::ResourceBusy => crate::reliability::errno::EBUSY,
            FsError::Busy => crate::reliability::errno::EBUSY,
            FsError::QuotaExceeded => crate::reliability::errno::EDQUOT,
            FsError::NotMounted => crate::reliability::errno::ENODEV,
            FsError::ReadOnly => crate::reliability::errno::EROFS,
            FsError::InvalidOperation => crate::reliability::errno::EINVAL,
            FsError::Loop => crate::reliability::errno::ELOOP,
            FsError::TooManyLinks => crate::reliability::errno::EMLINK,
        }
    }

    /// 获取错误描述
    pub fn description(&self) -> &'static str {
        match self {
            FsError::PathNotFound => "Path not found",
            FsError::FileNotFound => "File not found",
            FsError::NotFound => "Not found",
            FsError::PermissionDenied => "Permission denied",
            FsError::FileExists => "File already exists",
            FsError::Exists => "Already exists",
            FsError::NotADirectory => "Not a directory",
            FsError::IsADirectory => "Is a directory",
            FsError::DirectoryNotEmpty => "Directory not empty",
            FsError::NotEmpty => "Not empty",
            FsError::InvalidPath => "Invalid path",
            FsError::InvalidInput => "Invalid input",
            FsError::PathTooLong => "Path too long",
            FsError::FileSystemFull => "File system full",
            FsError::NoSpace => "No space left on device",
            FsError::IoError => "Input/output error",
            FsError::OperationNotSupported => "Operation not supported",
            FsError::NotSupported => "Not supported",
            FsError::ResourceBusy => "Resource busy",
            FsError::Busy => "Resource busy",
            FsError::QuotaExceeded => "Quota exceeded",
            FsError::NotMounted => "File system not mounted",
            FsError::ReadOnly => "Read-only file system",
            FsError::InvalidOperation => "Invalid operation",
            FsError::Loop => "Too many levels of symbolic links",
            FsError::TooManyLinks => "Too many links",
        }
    }
}

impl From<crate::syscalls::common::SyscallError> for FsError {
    fn from(err: crate::syscalls::common::SyscallError) -> Self {
        match err {
            crate::syscalls::common::SyscallError::InvalidArgument => FsError::InvalidPath,
            crate::syscalls::common::SyscallError::PermissionDenied => FsError::PermissionDenied,
            crate::syscalls::common::SyscallError::BadFileDescriptor => FsError::PathNotFound,
            crate::syscalls::common::SyscallError::IoError => FsError::IoError,
            _ => FsError::IoError,
        }
    }
}

impl From<crate::vfs::error::VfsError> for FsError {
    fn from(err: crate::vfs::error::VfsError) -> Self {
        match err {
            crate::vfs::error::VfsError::NotFound => FsError::NotFound,
            crate::vfs::error::VfsError::NoEntry => FsError::PathNotFound,
            crate::vfs::error::VfsError::PermissionDenied => FsError::PermissionDenied,
            crate::vfs::error::VfsError::NotADirectory => FsError::NotADirectory,
            crate::vfs::error::VfsError::IsDirectory => FsError::IsADirectory,
            crate::vfs::error::VfsError::NotEmpty => FsError::NotEmpty,
            crate::vfs::error::VfsError::Exists => FsError::Exists,
            crate::vfs::error::VfsError::NoSpace => FsError::NoSpace,
            crate::vfs::error::VfsError::InvalidPath => FsError::InvalidPath,
            crate::vfs::error::VfsError::IoError => FsError::IoError,
            crate::vfs::error::VfsError::NotSupported => FsError::NotSupported,
            crate::vfs::error::VfsError::Busy => FsError::Busy,
            crate::vfs::error::VfsError::ReadOnly => FsError::ReadOnly,
            crate::vfs::error::VfsError::InvalidOperation => FsError::InvalidOperation,
            crate::vfs::error::VfsError::Loop => FsError::Loop,
            crate::vfs::error::VfsError::NotDirectory => FsError::NotADirectory,
            crate::vfs::error::VfsError::NotMounted => FsError::NotMounted,
            crate::vfs::error::VfsError::TooManyLinks => FsError::TooManyLinks,
            crate::vfs::error::VfsError::NotASymlink => FsError::InvalidOperation,
            crate::vfs::error::VfsError::InvalidInput => FsError::InvalidInput,
            _ => FsError::IoError,
        }
    }
}
