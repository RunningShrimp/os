//! VFS error types
extern crate alloc;

use crate::prelude::*;

use alloc::fmt;

/// VFS error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    PermissionDenied,
    NotDirectory,
    IsDirectory,
    NotEmpty,
    Exists,
    NoSpace,
    InvalidPath,
    NotMounted,
    Busy,
    ReadOnly,
    IoError,
    NotSupported,
    InvalidOperation,
    Loop,
    TooManyLinks,
    NotASymlink,
    InvalidInput,
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::NotFound => write!(f, "File not found"),
            VfsError::PermissionDenied => write!(f, "Permission denied"),
            VfsError::NotDirectory => write!(f, "Not a directory"),
            VfsError::IsDirectory => write!(f, "Is a directory"),
            VfsError::NotEmpty => write!(f, "Directory not empty"),
            VfsError::Exists => write!(f, "File exists"),
            VfsError::NoSpace => write!(f, "No space left"),
            VfsError::InvalidPath => write!(f, "Invalid path"),
            VfsError::NotMounted => write!(f, "Not mounted"),
            VfsError::Busy => write!(f, "Device busy"),
            VfsError::ReadOnly => write!(f, "Read-only filesystem"),
            VfsError::IoError => write!(f, "I/O error"),
            VfsError::NotSupported => write!(f, "Operation not supported"),
            VfsError::InvalidOperation => write!(f, "Invalid operation"),
            VfsError::Loop => write!(f, "Too many levels of symbolic links"),
            VfsError::TooManyLinks => write!(f, "Too many links"),
            VfsError::NotASymlink => write!(f, "Not a symbolic link"),
            VfsError::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

impl core::error::Error for VfsError {}

/// VFS result type
pub type VfsResult<T> = Result<T, VfsError>;
