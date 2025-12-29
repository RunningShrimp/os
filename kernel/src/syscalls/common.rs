//! Auto-generated stub module
use crate::error::KernelError;

#[derive(Debug, Clone, PartialEq)]
pub enum SyscallError {
    InvalidArgument,
    PermissionDenied,
    BadFileDescriptor,
    IoError,
    ResourceBusy,
    NotImplemented,
    WouldBlock,
}

impl std::fmt::Display for SyscallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyscallError::InvalidArgument => write!(f, "Invalid argument"),
            SyscallError::PermissionDenied => write!(f, "Permission denied"),
            SyscallError::BadFileDescriptor => write!(f, "Bad file descriptor"),
            SyscallError::IoError => write!(f, "I/O error"),
            SyscallError::ResourceBusy => write!(f, "Resource busy"),
            SyscallError::NotImplemented => write!(f, "Not implemented"),
            SyscallError::WouldBlock => write!(f, "Would block"),
        }
    }
}

impl std::error::Error for SyscallError {}

pub struct Stub;
impl Stub {
    pub fn new() -> Self { Stub }
}

pub fn stub_function() -> Result<(), KernelError> { Ok(()) }