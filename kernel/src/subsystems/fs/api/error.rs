//! File system error definitions for the public API.
//!
//! This module contains all public file system error types.

/// File system error type.
///
///统一的文件系统错误处理
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    ///路径不存在
    PathNotFound,
    ///文件不存在
    FileNotFound,
    ///权限被拒绝
    PermissionDenied,
    ///文件已存在
    FileExists,
    ///不是目录
    NotADirectory,
    ///是目录
    IsADirectory,
    ///目录非空
    DirectoryNotEmpty,
    ///无效路径
    InvalidPath,
    ///路径过长
    PathTooLong,
    ///文件系统已满
    FileSystemFull,
    ///输入/输出错误
    IoError,
    ///文件系统只读
    ReadOnlyFileSystem,
    ///文件系统不支持操作
    OperationNotSupported,
    ///资源忙
    ResourceBusy,
    ///文件名过长
    FileNameTooLong,
    ///符号链接循环
    SymbolicLinkLoop,
    ///配额超限
    QuotaExceeded,
    ///存储空间不足
    NoSpaceLeft,
    ///坏文件系统
    CorruptedFileSystem,
}

impl FsError {
    ///转换为POSIX错误码
    pub fn to_errno(&self) -> i32 {
        match self {
            FsError::PathNotFound => crate::reliability::errno::ENOENT,
            FsError::PermissionDenied => crate::reliability::errno::EACCES,
            FsError::FileExists => crate::reliability::errno::EEXIST,
            FsError::IoError => crate::reliability::errno::EIO,
            FsError::NoSpaceLeft => crate::reliability::errno::ENOSPC,
            FsError::NotADirectory => crate::reliability::errno::ENOTDIR,
            FsError::IsADirectory => crate::reliability::errno::EISDIR,
            FsError::InvalidPath => crate::reliability::errno::EINVAL,
            FsError::DirectoryNotEmpty => crate::reliability::errno::ENOTEMPTY,
            FsError::OperationNotSupported => crate::reliability::errno::EOPNOTSUPP,
            // ... 其他错误映射
            _ => crate::reliability::errno::EINVAL,
        }
    }
    
    ///获取错误描述
    pub fn description(&self) -> &'static str {
        match self {
            FsError::PathNotFound => "Path not found",
            FsError::PermissionDenied => "Permission denied",
            FsError::FileExists => "File already exists",
            FsError::IoError => "Input/output error",
            FsError::NoSpaceLeft => "No space left on device",
            // ... 其他错误描述
            _ => "Unknown file system error",
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
            crate::syscalls::common::SyscallError::ResourceBusy => FsError::ResourceBusy,
            crate::syscalls::common::SyscallError::NotImplemented => FsError::OperationNotSupported,
            _ => FsError::IoError,
        }
    }
}