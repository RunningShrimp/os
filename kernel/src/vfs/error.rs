//! VFS error types

/// VFS error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    NoEntry,
    PermissionDenied,
    NotDirectory,
    NotADirectory,
    IsDirectory,
    IsADirectory,
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

pub type VfsResult<T> = Result<T, VfsError>;
