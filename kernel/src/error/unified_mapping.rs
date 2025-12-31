//! Unified Error Code Mapping
//!
//! This module provides unified error code mapping between:
//! - Kernel internal error types (UnifiedError)
//! - System call errors (SyscallError)
//! - POSIX errno values
//! - nos-error-handling crate errors
//!
//! This ensures consistent error reporting across all layers of the kernel.

use crate::error::ErrorType;
use crate::error::error_types::from_nos_error_type;

#[allow(deprecated)]
use nos_error_handling::kernel_integration::ErrorType as NosErrorType;

use crate::{
    error::unified::{UnifiedError, MemoryError, FileSystemError, NetworkError},
    subsystems::syscalls::{
        api::syscall_result::SyscallError as ApiSyscallError,
        interface::InterfaceSyscallError,
    },
};

/// POSIX errno values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Errno {
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    ENOTBLK = 15,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    ETXTBSY = 26,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EMLINK = 31,
    EPIPE = 32,
    EDOM = 33,
    ERANGE = 34,
    EDEADLK = 35,
    ENAMETOOLONG = 36,
    ENOLCK = 37,
    ENOSYS = 38,
    ENOTEMPTY = 39,
    ELOOP = 40,
    EWOULDBLOCK = 41,
    ENOMSG = 42,
    EIDRM = 43,
    ECHRNG = 44,
    EL2NSYNC = 45,
    EL3HLT = 46,
    EL3RST = 47,
    ELNRNG = 48,
    EUNATCH = 49,
    ENOCSI = 50,
    EL2HLT = 51,
    EBADE = 52,
    EBADR = 53,
    EXFULL = 54,
    ENOANO = 55,
    EBADRQC = 56,
    EBADSLT = 57,
    EBFONT = 59,
    ENOSTR = 60,
    ENODATA = 61,
    ETIME = 62,
    ENOSR = 63,
    ENONET = 64,
    ENOPKG = 65,
    EREMOTE = 66,
    ENOLINK = 67,
    EADV = 68,
    ESRMNT = 69,
    ECOMM = 70,
    EPROTO = 71,
    EMULTIHOP = 72,
    EDOTDOT = 73,
    EBADMSG = 74,
    EOVERFLOW = 75,
    ENOTUNIQ = 76,
    EBADFD = 77,
    EREMCHG = 78,
    ELIBACC = 79,
    ELIBBAD = 80,
    ELIBSCN = 81,
    ELIBMAX = 82,
    ELIBEXEC = 83,
    EILSEQ = 84,
    ERESTART = 85,
    ESTRPIPE = 86,
    EUSERS = 87,
    ENOTSOCK = 88,
    EDESTADDRREQ = 89,
    EMSGSIZE = 90,
    EPROTOTYPE = 91,
    ENOPROTOOPT = 92,
    EPROTONOSUPPORT = 93,
    ESOCKTNOSUPPORT = 94,
    EOPNOTSUPP = 95,
    EPFNOSUPPORT = 96,
    EAFNOSUPPORT = 97,
    EADDRINUSE = 98,
    EADDRNOTAVAIL = 99,
    ENETDOWN = 100,
    ENETUNREACH = 101,
    ENETRESET = 102,
    ECONNABORTED = 103,
    ECONNRESET = 104,
    ENOBUFS = 105,
    EISCONN = 106,
    ENOTCONN = 107,
    ESHUTDOWN = 108,
    ETOOMANYREFS = 109,
    ETIMEDOUT = 110,
    ECONNREFUSED = 111,
    EHOSTDOWN = 112,
    EHOSTUNREACH = 113,
    EALREADY = 114,
    EINPROGRESS = 115,
    ESTALE = 116,
    EUCLEAN = 117,
    ENOTNAM = 118,
    ENAVAIL = 119,
    EISNAM = 120,
    EREMOTEIO = 121,
    EDQUOT = 122,
    ENOMEDIUM = 123,
    EMEDIUMTYPE = 124,
    ECANCELED = 125,
    ENOKEY = 126,
    EKEYEXPIRED = 127,
    EKEYREVOKED = 128,
    EKEYREJECTED = 129,
    EOWNERDEAD = 130,
    ENOTRECOVERABLE = 131,
    ERFKILL = 132,
    EHWPOISON = 133,
}

impl Errno {
    /// Convert errno to i32
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// Get errno from i32
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            1 => Some(Errno::EPERM),
            2 => Some(Errno::ENOENT),
            3 => Some(Errno::ESRCH),
            4 => Some(Errno::EINTR),
            5 => Some(Errno::EIO),
            6 => Some(Errno::ENXIO),
            7 => Some(Errno::E2BIG),
            8 => Some(Errno::ENOEXEC),
            9 => Some(Errno::EBADF),
            10 => Some(Errno::ECHILD),
            11 => Some(Errno::EAGAIN),
            12 => Some(Errno::ENOMEM),
            13 => Some(Errno::EACCES),
            14 => Some(Errno::EFAULT),
            15 => Some(Errno::ENOTBLK),
            16 => Some(Errno::EBUSY),
            17 => Some(Errno::EEXIST),
            18 => Some(Errno::EXDEV),
            19 => Some(Errno::ENODEV),
            20 => Some(Errno::ENOTDIR),
            21 => Some(Errno::EISDIR),
            22 => Some(Errno::EINVAL),
            23 => Some(Errno::ENFILE),
            24 => Some(Errno::EMFILE),
            25 => Some(Errno::ENOTTY),
            26 => Some(Errno::ETXTBSY),
            27 => Some(Errno::EFBIG),
            28 => Some(Errno::ENOSPC),
            29 => Some(Errno::ESPIPE),
            30 => Some(Errno::EROFS),
            31 => Some(Errno::EMLINK),
            32 => Some(Errno::EPIPE),
            _ => None,
        }
    }
}

/// Unified error code mapper
///
/// Note: This mapper uses match statements for all conversions since the error types
/// contain variants with data (e.g., Other(String)) which cannot be used as BTreeMap keys.
#[allow(deprecated)]
pub struct UnifiedErrorMapper {
    // Private marker to prevent direct construction
    _private: (),
}

// The mapping is done entirely through match statements in the implementation methods
// This avoids the need for BTreeMap keys with Ord trait

impl UnifiedErrorMapper {
    /// Create a new error mapper
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Map UnifiedError to Errno
    ///
    /// This function uses static match for all error types.
    pub fn map_unified_error(&self, error: &UnifiedError) -> Errno {
        // Use static match for all errors
        match error {
            UnifiedError::InvalidArgument => Errno::EINVAL,
            UnifiedError::InvalidAddress => Errno::EFAULT,
            UnifiedError::PermissionDenied => Errno::EACCES,
            UnifiedError::NotFound => Errno::ENOENT,
            UnifiedError::AlreadyExists => Errno::EEXIST,
            UnifiedError::ResourceBusy => Errno::EBUSY,
            UnifiedError::ResourceUnavailable => Errno::EAGAIN,
            UnifiedError::OutOfMemory => Errno::ENOMEM,
            UnifiedError::InvalidInput => Errno::EINVAL,
            UnifiedError::InvalidData => Errno::EINVAL,
            UnifiedError::InvalidState => Errno::EINVAL,
            UnifiedError::InvalidOperation => Errno::EOPNOTSUPP,
            UnifiedError::NoProcess => Errno::ESRCH,
            UnifiedError::NoDevice => Errno::ENODEV,
            UnifiedError::AlreadyInProgress => Errno::EINPROGRESS,
            UnifiedError::FileExists => Errno::EEXIST,
            UnifiedError::Busy => Errno::EBUSY,
            UnifiedError::OutOfSpace => Errno::ENOSPC,
            UnifiedError::QuotaExceeded => Errno::EDQUOT,
            UnifiedError::NotSupported => Errno::EOPNOTSUPP,
            UnifiedError::NotADirectory => Errno::ENOTDIR,
            UnifiedError::IsADirectory => Errno::EISDIR,
            UnifiedError::DirectoryNotEmpty => Errno::ENOTEMPTY,
            UnifiedError::Interrupted => Errno::EINTR,
            UnifiedError::TimedOut => Errno::ETIMEDOUT,
            UnifiedError::WouldBlock => Errno::EWOULDBLOCK,
            UnifiedError::IoError => Errno::EIO,
            UnifiedError::BadAddress => Errno::EFAULT,
            UnifiedError::BadFileDescriptor => Errno::EBADF,
            UnifiedError::ConnectionAborted => Errno::ECONNABORTED,
            UnifiedError::ConnectionReset => Errno::ECONNRESET,
            UnifiedError::Unknown => Errno::EIO,
            UnifiedError::MemoryError(MemoryError::OutOfMemory) => Errno::ENOMEM,
            UnifiedError::MemoryError(MemoryError::InvalidAlignment) => Errno::EINVAL,
            UnifiedError::MemoryError(MemoryError::InvalidSize) => Errno::EINVAL,
            UnifiedError::MemoryError(MemoryError::InvalidAddress) => Errno::EFAULT,
            UnifiedError::FileSystemError(FileSystemError::PathNotFound) => Errno::ENOENT,
            UnifiedError::FileSystemError(FileSystemError::FileNotFound) => Errno::ENOENT,
            UnifiedError::FileSystemError(FileSystemError::PermissionDenied) => Errno::EACCES,
            UnifiedError::FileSystemError(FileSystemError::FileExists) => Errno::EEXIST,
            UnifiedError::FileSystemError(FileSystemError::FileSystemFull) => Errno::ENOSPC,
            UnifiedError::NetworkError(NetworkError::ConnectionRefused) => Errno::ECONNREFUSED,
            UnifiedError::NetworkError(NetworkError::TimedOut) => Errno::ETIMEDOUT,
            UnifiedError::ProcessError(_) => Errno::ESRCH,
            UnifiedError::SyscallError(_) => Errno::ENOSYS,
            UnifiedError::DriverError(_) => Errno::ENODEV,
            UnifiedError::SecurityError(_) => Errno::EACCES,
            // Fallback based on error category
            UnifiedError::MemoryError(_) => Errno::ENOMEM,
            UnifiedError::FileSystemError(_) => Errno::EIO,
            UnifiedError::NetworkError(_) => Errno::ECONNREFUSED,
            UnifiedError::Other(_) => Errno::EIO,
        }
    }

    /// Map ApiSyscallError to Errno
    pub fn map_api_syscall_error(&self, error: &ApiSyscallError) -> Errno {
        match error {
            ApiSyscallError::EPERM => Errno::EPERM,
            ApiSyscallError::ENOENT => Errno::ENOENT,
            ApiSyscallError::ESRCH => Errno::ESRCH,
            ApiSyscallError::EINTR => Errno::EINTR,
            ApiSyscallError::EIO => Errno::EIO,
            ApiSyscallError::ENXIO => Errno::ENXIO,
            ApiSyscallError::E2BIG => Errno::E2BIG,
            ApiSyscallError::ENOEXEC => Errno::ENOEXEC,
            ApiSyscallError::EBADF => Errno::EBADF,
            ApiSyscallError::ECHILD => Errno::ECHILD,
            ApiSyscallError::EAGAIN => Errno::EAGAIN,
            ApiSyscallError::ENOMEM => Errno::ENOMEM,
            ApiSyscallError::EACCES => Errno::EACCES,
            ApiSyscallError::EFAULT => Errno::EFAULT,
            ApiSyscallError::ENOTBLK => Errno::ENOTBLK,
            ApiSyscallError::EBUSY => Errno::EBUSY,
            ApiSyscallError::EEXIST => Errno::EEXIST,
            ApiSyscallError::EXDEV => Errno::EXDEV,
            ApiSyscallError::ENODEV => Errno::ENODEV,
            ApiSyscallError::ENOTDIR => Errno::ENOTDIR,
            ApiSyscallError::EISDIR => Errno::EISDIR,
            ApiSyscallError::EINVAL => Errno::EINVAL,
            ApiSyscallError::ENFILE => Errno::ENFILE,
            ApiSyscallError::EMFILE => Errno::EMFILE,
            ApiSyscallError::ENOTTY => Errno::ENOTTY,
            ApiSyscallError::ETXTBSY => Errno::ETXTBSY,
            ApiSyscallError::EFBIG => Errno::EFBIG,
            ApiSyscallError::ENOSPC => Errno::ENOSPC,
            ApiSyscallError::ESPIPE => Errno::ESPIPE,
            ApiSyscallError::EROFS => Errno::EROFS,
            ApiSyscallError::EMLINK => Errno::EMLINK,
            ApiSyscallError::EPIPE => Errno::EPIPE,
        }
    }

    /// Map InterfaceSyscallError to Errno
    pub fn map_interface_syscall_error(&self, error: &InterfaceSyscallError) -> Errno {
        // Handle variant with data and other variants
        match error {
            InterfaceSyscallError::InvalidInterface => Errno::EINVAL,
            InterfaceSyscallError::InterfaceNotFound => Errno::ENOENT,
            InterfaceSyscallError::NotSupported => Errno::ENOSYS,
            InterfaceSyscallError::PermissionDenied => Errno::EACCES,
            InterfaceSyscallError::ResourceBusy => Errno::EBUSY,
            InterfaceSyscallError::InvalidArgument => Errno::EINVAL,
            InterfaceSyscallError::SyscallFailed(_) => Errno::EIO,
            InterfaceSyscallError::OperationNotPermitted => Errno::EPERM,
            InterfaceSyscallError::NotFound => Errno::ENOENT,
            InterfaceSyscallError::WouldBlock => Errno::EWOULDBLOCK,
            InterfaceSyscallError::OutOfMemory => Errno::ENOMEM,
            InterfaceSyscallError::FileExists => Errno::EEXIST,
            InterfaceSyscallError::NoSpaceLeft => Errno::ENOSPC,
            InterfaceSyscallError::BrokenPipe => Errno::EPIPE,
            InterfaceSyscallError::ConnectionRefused => Errno::ECONNREFUSED,
            InterfaceSyscallError::ConnectionReset => Errno::ECONNRESET,
            InterfaceSyscallError::TimedOut => Errno::ETIMEDOUT,
            InterfaceSyscallError::NameTooLong => Errno::ENAMETOOLONG,
            InterfaceSyscallError::DeadlockWouldOccur => Errno::EDEADLK,
            InterfaceSyscallError::BadFileDescriptor => Errno::EBADF,
            InterfaceSyscallError::NoBufferSpace => Errno::ENOBUFS,
            InterfaceSyscallError::IoError => Errno::EIO,
        }
    }

    /// Map NosErrorType to Errno (deprecated - kept for backward compatibility)
    #[allow(deprecated)]
    pub fn map_nos_error_type(&self, error: &NosErrorType) -> Errno {
        match error {
            NosErrorType::RuntimeError => Errno::EIO,
            NosErrorType::LogicError => Errno::EINVAL,
            NosErrorType::ResourceError => Errno::ENOMEM,
            NosErrorType::PermissionError => Errno::EACCES,
            NosErrorType::NetworkError => Errno::ECONNREFUSED,
            NosErrorType::IOError => Errno::EIO,
            NosErrorType::MemoryError => Errno::ENOMEM,
            NosErrorType::SystemCallError => Errno::ENOSYS,
            NosErrorType::ValidationError => Errno::EINVAL,
            NosErrorType::TimeoutError => Errno::ETIMEDOUT,
            NosErrorType::CancellationError => Errno::ECANCELED,
            NosErrorType::SystemError => Errno::EIO,
            _ => Errno::EIO,
        }
    }

    /// Map local ErrorType to Errno (recommended)
    pub fn map_error_type(&self, error: &ErrorType) -> Errno {
        match error {
            ErrorType::RuntimeError => Errno::EIO,
            ErrorType::LogicError => Errno::EINVAL,
            ErrorType::ResourceError => Errno::ENOMEM,
            ErrorType::PermissionError => Errno::EACCES,
            ErrorType::NetworkError => Errno::ECONNREFUSED,
            ErrorType::IOError => Errno::EIO,
            ErrorType::MemoryError => Errno::ENOMEM,
            ErrorType::SystemCallError => Errno::ENOSYS,
            ErrorType::ValidationError => Errno::EINVAL,
            ErrorType::TimeoutError => Errno::ETIMEDOUT,
            ErrorType::CancellationError => Errno::ECANCELED,
            ErrorType::SystemError => Errno::EIO,
            _ => Errno::EIO,
        }
    }

    /// Convert external nos-error-handling ErrorType to local ErrorType and map to Errno
    #[allow(deprecated)]
    pub fn map_converted_nos_error(&self, nos_error: &NosErrorType) -> Errno {
        let local_error = from_nos_error_type(nos_error);
        self.map_error_type(&local_error)
    }

    /// Convert any error to Errno (generic mapping)
    ///
    /// Note: This method provides a simple fallback for generic error types.
    /// For specific error types, use the dedicated mapping methods instead.
    pub fn to_errno<E>(&self, _error: &E) -> Errno
    where
        E: core::fmt::Debug,
    {
        // Default fallback for unknown error types
        Errno::EIO
    }
}

/// Global error mapper instance
static GLOBAL_ERROR_MAPPER: crate::subsystems::sync::Mutex<Option<UnifiedErrorMapper>> =
    crate::subsystems::sync::Mutex::new(None);

/// Initialize the global error mapper
pub fn init_error_mapper() {
    let mut mapper_guard = GLOBAL_ERROR_MAPPER.lock();
    *mapper_guard = Some(UnifiedErrorMapper::new());
}

/// Get the global error mapper
pub fn get_error_mapper()
-> Option<&'static crate::subsystems::sync::Mutex<Option<UnifiedErrorMapper>>> {
    Some(&GLOBAL_ERROR_MAPPER)
}

/// Fast path: Convert common UnifiedError to Errno without mapper lookup
///
/// This function is optimized for hot path (syscall error returns).
/// It uses static match for the most common errors, avoiding any
/// synchronization or lookup overhead.
#[inline]
pub fn fast_unified_error_to_errno(error: &UnifiedError) -> Errno {
    match error {
        UnifiedError::InvalidArgument => Errno::EINVAL,
        UnifiedError::InvalidAddress => Errno::EFAULT,
        UnifiedError::PermissionDenied => Errno::EACCES,
        UnifiedError::NotFound => Errno::ENOENT,
        UnifiedError::AlreadyExists => Errno::EEXIST,
        UnifiedError::ResourceBusy => Errno::EBUSY,
        UnifiedError::ResourceUnavailable => Errno::EAGAIN,
        UnifiedError::OutOfMemory => Errno::ENOMEM,
        UnifiedError::MemoryError(MemoryError::OutOfMemory) => Errno::ENOMEM,
        UnifiedError::MemoryError(MemoryError::InvalidAddress) => Errno::EFAULT,
        UnifiedError::FileSystemError(FileSystemError::PathNotFound) => Errno::ENOENT,
        UnifiedError::FileSystemError(FileSystemError::FileNotFound) => Errno::ENOENT,
        UnifiedError::FileSystemError(FileSystemError::PermissionDenied) => Errno::EACCES,
        UnifiedError::NetworkError(NetworkError::ConnectionRefused) => Errno::ECONNREFUSED,
        UnifiedError::NetworkError(NetworkError::TimedOut) => Errno::ETIMEDOUT,
        // For less common errors, fall back to full mapping
        _ => unified_error_to_errno(error),
    }
}

/// Convert UnifiedError to Errno
///
/// This function uses the error mapper if available, otherwise uses
/// fast path fallback. For hot path syscall errors, use fast_unified_error_to_errno instead.
pub fn unified_error_to_errno(error: &UnifiedError) -> Errno {
    if let Some(mapper_guard) = get_error_mapper() {
        let mapper = mapper_guard.lock();
        if let Some(ref mapper) = *mapper {
            mapper.map_unified_error(error)
        } else {
            // Fallback mapping using fast path
            fast_unified_error_to_errno(error)
        }
    } else {
        // No mapper available, use fast path
        fast_unified_error_to_errno(error)
    }
}

/// Convert ApiSyscallError to Errno
pub fn api_syscall_error_to_errno(error: &ApiSyscallError) -> Errno {
    if let Some(mapper_guard) = get_error_mapper() {
        let mapper = mapper_guard.lock();
        if let Some(ref mapper) = *mapper {
            mapper.map_api_syscall_error(error)
        } else {
            // Fallback: convert enum to errno directly
            match error {
                ApiSyscallError::EPERM => Errno::EPERM,
                ApiSyscallError::ENOENT => Errno::ENOENT,
                ApiSyscallError::ESRCH => Errno::ESRCH,
                ApiSyscallError::EINTR => Errno::EINTR,
                ApiSyscallError::EIO => Errno::EIO,
                ApiSyscallError::ENXIO => Errno::ENXIO,
                ApiSyscallError::E2BIG => Errno::E2BIG,
                ApiSyscallError::ENOEXEC => Errno::ENOEXEC,
                ApiSyscallError::EBADF => Errno::EBADF,
                ApiSyscallError::ECHILD => Errno::ECHILD,
                ApiSyscallError::EAGAIN => Errno::EAGAIN,
                ApiSyscallError::ENOMEM => Errno::ENOMEM,
                ApiSyscallError::EACCES => Errno::EACCES,
                ApiSyscallError::EFAULT => Errno::EFAULT,
                ApiSyscallError::ENOTBLK => Errno::ENOTBLK,
                ApiSyscallError::EBUSY => Errno::EBUSY,
                ApiSyscallError::EEXIST => Errno::EEXIST,
                ApiSyscallError::EXDEV => Errno::EXDEV,
                ApiSyscallError::ENODEV => Errno::ENODEV,
                ApiSyscallError::ENOTDIR => Errno::ENOTDIR,
                ApiSyscallError::EISDIR => Errno::EISDIR,
                ApiSyscallError::EINVAL => Errno::EINVAL,
                ApiSyscallError::ENFILE => Errno::ENFILE,
                ApiSyscallError::EMFILE => Errno::EMFILE,
                ApiSyscallError::ENOTTY => Errno::ENOTTY,
                ApiSyscallError::ETXTBSY => Errno::ETXTBSY,
                ApiSyscallError::EFBIG => Errno::EFBIG,
                ApiSyscallError::ENOSPC => Errno::ENOSPC,
                ApiSyscallError::ESPIPE => Errno::ESPIPE,
                ApiSyscallError::EROFS => Errno::EROFS,
                ApiSyscallError::EMLINK => Errno::EMLINK,
                ApiSyscallError::EPIPE => Errno::EPIPE,
            }
        }
    } else {
        Errno::EIO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_mapping() {
        init_error_mapper();

        let unified_err = UnifiedError::InvalidArgument;
        let errno = unified_error_to_errno(&unified_err);
        assert_eq!(errno, Errno::EINVAL);

        let unified_err = UnifiedError::OutOfMemory;
        let errno = unified_error_to_errno(&unified_err);
        assert_eq!(errno, Errno::ENOMEM);
    }
}
