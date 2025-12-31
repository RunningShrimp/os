//! Graphics error types

#![allow(dead_code)]

use alloc::string::String;
use core::fmt;

/// Graphics error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsError {
    /// Operation not supported
    NotSupported(String),
    /// Invalid argument
    InvalidArgument(String),
    /// Operation failed
    OperationFailed(String),
    /// No device found
    NoDevice(String),
    /// Out of memory
    OutOfMemory(String),
    /// Invalid context
    InvalidContext(String),
    /// Timeout
    Timeout(String),
    /// Device busy
    DeviceBusy(String),
    /// Permission denied
    PermissionDenied(String),
    /// Invalid mode
    InvalidMode(String),
    /// Resource allocation failed
    ResourceAllocationFailed(String),
    /// Hardware error
    HardwareError(String),
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            Self::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
            Self::NoDevice(msg) => write!(f, "No device: {}", msg),
            Self::OutOfMemory(msg) => write!(f, "Out of memory: {}", msg),
            Self::InvalidContext(msg) => write!(f, "Invalid context: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::DeviceBusy(msg) => write!(f, "Device busy: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::InvalidMode(msg) => write!(f, "Invalid mode: {}", msg),
            Self::ResourceAllocationFailed(msg) => {
                write!(f, "Resource allocation failed: {}", msg)
            }
            Self::HardwareError(msg) => write!(f, "Hardware error: {}", msg),
        }
    }
}

/// Graphics result type
pub type GraphicsResult<T> = core::result::Result<T, GraphicsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GraphicsError::NotSupported("feature".to_string());
        assert_eq!(format!("{}", err), "Not supported: feature");

        let err = GraphicsError::InvalidArgument("param".to_string());
        assert_eq!(format!("{}", err), "Invalid argument: param");
    }

    #[test]
    fn test_error_equality() {
        let err1 = GraphicsError::NotSupported("test".to_string());
        let err2 = GraphicsError::NotSupported("test".to_string());
        assert_eq!(err1, err2);

        let err3 = GraphicsError::OperationFailed("test".to_string());
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_ok_result() {
        let result: GraphicsResult<u32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_err_result() {
        let result: GraphicsResult<u32> = Err(GraphicsError::InvalidArgument("test".to_string()));
        assert!(result.is_err());
    }
}
