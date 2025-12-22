// Shared utilities for syscall modules

/// Extended syscall error types with POSIX compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallError {
    // Basic errors
    InvalidSyscall,          // ENOSYS
    PermissionDenied,        // EPERM/EACCES
    InvalidArgument,         // EINVAL
    NotFound,                // ENOENT
    OutOfMemory,             // ENOMEM
    Interrupted,             // EINTR
    IoError,                 // EIO
    WouldBlock,              // EAGAIN/EWOULDBLOCK
    NotSupported,            // EOPNOTSUPP
    
    // Extended errors for better diagnostics
    BadFileDescriptor,       // EBADF
    TooManyOpenFiles,        // EMFILE
    NoBufferSpace,           // ENOBUFS
    NotADirectory,           // ENOTDIR
    IsADirectory,            // EISDIR
    DirectoryNotEmpty,       // ENOTEMPTY
    FileExists,              // EEXIST
    CrossDeviceLink,         // EXDEV
    FileTooBig,              // EFBIG
    NoSpaceLeft,             // ENOSPC
    BadAddress,              // EFAULT
    DeadlockWouldOccur,      // EDEADLK
    NameTooLong,             // ENAMETOOLONG
    TooManySymlinks,         // ELOOP
    ConnectionRefused,       // ECONNREFUSED
    ConnectionReset,         // ECONNRESET
    BrokenPipe,              // EPIPE
    TimedOut,                // ETIMEDOUT
}

pub type SyscallResult = Result<u64, SyscallError>;

/// Convert syscall result to raw value for return
pub fn result_to_raw(result: SyscallResult) -> u64 {
    match result {
        Ok(value) => value,
        Err(error) => {
            // Error codes are negative, using high bits
            // This is a placeholder implementation
            match error {
                SyscallError::InvalidSyscall => u64::MAX - 1,
                SyscallError::PermissionDenied => u64::MAX - 2,
                SyscallError::InvalidArgument => u64::MAX - 3,
                SyscallError::NotFound => u64::MAX - 4,
                SyscallError::OutOfMemory => u64::MAX - 5,
                SyscallError::Interrupted => u64::MAX - 6,
                SyscallError::IoError => u64::MAX - 7,
                SyscallError::WouldBlock => u64::MAX - 8,
                SyscallError::NotSupported => u64::MAX - 9,
                SyscallError::BadFileDescriptor => u64::MAX - 10,
                SyscallError::TooManyOpenFiles => u64::MAX - 11,
                SyscallError::NoBufferSpace => u64::MAX - 12,
                SyscallError::NotADirectory => u64::MAX - 13,
                SyscallError::IsADirectory => u64::MAX - 14,
                SyscallError::DirectoryNotEmpty => u64::MAX - 15,
                SyscallError::FileExists => u64::MAX - 16,
                SyscallError::CrossDeviceLink => u64::MAX - 17,
                SyscallError::FileTooBig => u64::MAX - 18,
                SyscallError::NoSpaceLeft => u64::MAX - 19,
                SyscallError::BadAddress => u64::MAX - 20,
                SyscallError::DeadlockWouldOccur => u64::MAX - 21,
                SyscallError::NameTooLong => u64::MAX - 22,
                SyscallError::TooManySymlinks => u64::MAX - 23,
                SyscallError::ConnectionRefused => u64::MAX - 24,
                SyscallError::ConnectionReset => u64::MAX - 25,
                SyscallError::BrokenPipe => u64::MAX - 26,
                SyscallError::TimedOut => u64::MAX - 27,
            }
        }
    }
}

/// Extract arguments from syscall context
pub fn extract_args(args: &[u64], count: usize) -> Result<&[u64], SyscallError> {
    if args.len() < count {
        Err(SyscallError::InvalidArgument)
    } else {
        Ok(&args[..count])
    }
}

/// Convert SyscallError to POSIX errno with full mapping
pub fn syscall_error_to_errno(error: SyscallError) -> i32 {
    use crate::reliability::errno::*;
    match error {
        SyscallError::InvalidSyscall => ENOSYS,
        SyscallError::PermissionDenied => EPERM,
        SyscallError::InvalidArgument => EINVAL,
        SyscallError::NotFound => ENOENT,
        SyscallError::OutOfMemory => ENOMEM,
        SyscallError::Interrupted => EINTR,
        SyscallError::IoError => EIO,
        SyscallError::WouldBlock => EAGAIN,
        SyscallError::NotSupported => EOPNOTSUPP,
        SyscallError::BadFileDescriptor => EBADF,
        SyscallError::TooManyOpenFiles => EMFILE,
        SyscallError::NoBufferSpace => ENOBUFS,
        SyscallError::NotADirectory => ENOTDIR,
        SyscallError::IsADirectory => EISDIR,
        SyscallError::DirectoryNotEmpty => ENOTEMPTY,
        SyscallError::FileExists => EEXIST,
        SyscallError::CrossDeviceLink => EXDEV,
        SyscallError::FileTooBig => EFBIG,
        SyscallError::NoSpaceLeft => ENOSPC,
        SyscallError::BadAddress => EFAULT,
        SyscallError::DeadlockWouldOccur => EDEADLK,
        SyscallError::NameTooLong => ENAMETOOLONG,
        SyscallError::TooManySymlinks => ELOOP,
        SyscallError::ConnectionRefused => ECONNREFUSED,
        SyscallError::ConnectionReset => ECONNRESET,
        SyscallError::BrokenPipe => EPIPE,
        SyscallError::TimedOut => ETIMEDOUT,
    }
}

/// Unified system call error handler
/// Maps internal errors to POSIX errno and returns negative value
#[inline]
pub fn syscall_error_to_neg_errno(error: SyscallError) -> isize {
    let errno = syscall_error_to_errno(error);
    -(errno as isize)
}

/// Convert ExecError to SyscallError
impl From<crate::process::exec::ExecError> for SyscallError {
    fn from(err: crate::process::exec::ExecError) -> Self {
        use crate::process::exec::ExecError;
        match err {
            ExecError::FileNotFound => SyscallError::NotFound,
            ExecError::FileTooLarge => SyscallError::FileTooBig,
            ExecError::InvalidElf => SyscallError::InvalidArgument,
            ExecError::OutOfMemory => SyscallError::OutOfMemory,
            ExecError::TooManyArgs => SyscallError::InvalidArgument,
            ExecError::ArgTooLong => SyscallError::InvalidArgument,
            ExecError::NoProcess => SyscallError::NotFound,
            ExecError::PermissionDenied => SyscallError::PermissionDenied,
        }
    }
}


/// Convert VfsError to SyscallError
impl From<crate::vfs::error::VfsError> for SyscallError {
    fn from(err: crate::vfs::error::VfsError) -> Self {
        use crate::vfs::error::VfsError;
        match err {
            VfsError::NotFound => SyscallError::NotFound,
            VfsError::PermissionDenied => SyscallError::PermissionDenied,
            VfsError::NotDirectory => SyscallError::NotADirectory,
            VfsError::IsDirectory => SyscallError::IsADirectory,
            VfsError::NotEmpty => SyscallError::DirectoryNotEmpty,
            VfsError::Exists => SyscallError::FileExists,
            VfsError::NoSpace => SyscallError::NoSpaceLeft,
            VfsError::InvalidPath => SyscallError::InvalidArgument,
            VfsError::NotMounted => SyscallError::IoError,
            VfsError::Busy => SyscallError::WouldBlock,
            VfsError::ReadOnly => SyscallError::PermissionDenied,
            VfsError::IoError => SyscallError::IoError,
            VfsError::NotSupported => SyscallError::NotSupported,
            VfsError::InvalidOperation => SyscallError::InvalidArgument,
        }
    }
}