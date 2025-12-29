//! VFS error types

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

pub type VfsResult<T> = Result<T, VfsError>;
