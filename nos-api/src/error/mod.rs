//! Error handling module for NOS operating system

use core::fmt;
#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};
#[cfg(feature = "alloc")]
use alloc::format;

// ToStringExt is not needed as string literals already have 'static lifetime
#[cfg(not(feature = "alloc"))]
use crate::interfaces::String;


/// Common error type used throughout NOS operating system
#[derive(Debug, Clone)]
pub enum Error {
    /// Kernel error
    Kernel(crate::core::types::KernelError),
    /// Invalid argument
    InvalidArgument(String),
    /// Invalid state
    InvalidState(String),
    /// Not implemented
    NotImplemented(String),
    /// Resource not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
    /// Resource busy
    Busy(String),
    /// Out of memory
    OutOfMemory,
    /// I/O error
    IoError(String),
    /// Network error
    NetworkError(String),
    /// Protocol error
    ProtocolError(String),
    /// Timeout
    Timeout,
    /// Connection error
    ConnectionError(String),
    /// Parse error
    ParseError(String),
    /// Configuration error
    ConfigError(String),
    /// Service error
    #[cfg(feature = "alloc")]
    ServiceError(String),
    #[cfg(not(feature = "alloc"))]
    ServiceError(&'static str),
    /// System error
    #[cfg(feature = "alloc")]
    SystemError(String),
    #[cfg(not(feature = "alloc"))]
    SystemError(&'static str),
    /// Circular dependency error
    #[cfg(feature = "alloc")]
    CircularDependency(String),
    #[cfg(not(feature = "alloc"))]
    CircularDependency(&'static str),
    /// Custom error with code and message
    #[cfg(feature = "alloc")]
    Custom(i32, String),
    #[cfg(not(feature = "alloc"))]
    Custom(i32, &'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Kernel(err) => write!(f, "Kernel error: {}", err),
            Error::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            Error::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Error::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            Error::NotFound(msg) => write!(f, "Not found: {}", msg),
            Error::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Error::Busy(msg) => write!(f, "Resource busy: {}", msg),
            Error::OutOfMemory => write!(f, "Out of memory"),
            Error::IoError(msg) => write!(f, "I/O error: {}", msg),
            Error::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Error::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Error::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Error::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            #[cfg(feature = "alloc")]
            Error::ServiceError(msg) => write!(f, "Service error: {}", msg),
            #[cfg(not(feature = "alloc"))]
            Error::ServiceError(msg) => write!(f, "Service error: {}", msg),
            #[cfg(feature = "alloc")]
            Error::SystemError(msg) => write!(f, "System error: {}", msg),
            #[cfg(not(feature = "alloc"))]
            Error::SystemError(msg) => write!(f, "System error: {}", msg),
            #[cfg(feature = "alloc")]
            Error::CircularDependency(msg) => write!(f, "Circular dependency: {}", msg),
            #[cfg(not(feature = "alloc"))]
            Error::CircularDependency(msg) => write!(f, "Circular dependency: {}", msg),
            #[cfg(feature = "alloc")]
            Error::Custom(code, msg) => write!(f, "Error {}: {}", code, msg),
            #[cfg(not(feature = "alloc"))]
            Error::Custom(code, msg) => write!(f, "Error {}: {}", code, msg),
        }
    }
}

impl From<crate::core::types::KernelError> for Error {
    fn from(err: crate::core::types::KernelError) -> Self {
        Error::Kernel(err)
    }
}

/// Result type for operations that can fail
pub type Result<T> = core::result::Result<T, Error>;

/// Error context trait for adding context to errors
pub trait ErrorContext<T> {
    /// Adds context to the error
    fn context(self, context: &str) -> Result<T>;
}

impl<T> ErrorContext<T> for Result<T> {
    fn context(self, _context: &str) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            #[cfg(feature = "alloc")]
            Err(error) => Err(Error::SystemError(format!("{}: {}", _context, error))),
            #[cfg(not(feature = "alloc"))]
            Err(_error) => Err(Error::SystemError("System error".into())),
        }
    }
}

/// Error conversion trait for converting between error types
pub trait ErrorFrom<T> {
    /// Converts from another error type
    fn from_error(error: T) -> Self;
}

/// Error builder for creating errors with context
pub struct ErrorBuilder {
    error: Error,
}

impl ErrorBuilder {
    /// Creates a new error builder
    pub fn new(error: Error) -> Self {
        Self { error }
    }
    
    /// Adds context to the error
    pub fn context(mut self, _context: &str) -> Self {
        #[cfg(feature = "alloc")]
        {
            self.error = Error::SystemError(format!("{}: {}", _context, self.error));
        }
        #[cfg(not(feature = "alloc"))]
        {
            self.error = Error::SystemError("System error".into());
        }
        self
    }
    
    /// Builds the final error
    pub fn build(self) -> Error {
        self.error
    }
}

/// Creates a new error builder
pub fn error(error: Error) -> ErrorBuilder {
    ErrorBuilder::new(error)
}

/// Creates a new kernel error
pub fn kernel_error(err: crate::core::types::KernelError) -> Error {
    Error::Kernel(err)
}

#[cfg(feature = "alloc")]
/// Creates a new invalid argument error
pub fn invalid_argument(msg: &str) -> Error {
    Error::InvalidArgument(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
/// Creates a new invalid argument error (no-alloc version)
pub fn invalid_argument(msg: &'static str) -> Error {
    Error::InvalidArgument(msg.into())
}

#[cfg(feature = "alloc")]
/// Creates a new not implemented error
pub fn not_implemented(msg: &str) -> Error {
    Error::NotImplemented(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
/// Creates a new not implemented error (no-alloc version)
pub fn not_implemented(msg: &'static str) -> Error {
    Error::NotImplemented(msg.into())
}

#[cfg(feature = "alloc")]
/// Creates a new not found error
pub fn not_found(msg: &str) -> Error {
    Error::NotFound(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
/// Creates a new not found error (no-alloc version)
pub fn not_found(msg: &'static str) -> Error {
    Error::NotFound(msg.into())
}

#[cfg(feature = "alloc")]
/// Creates a new permission denied error
pub fn permission_denied(msg: &str) -> Error {
    Error::PermissionDenied(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
/// Creates a new permission denied error (no-alloc version)
pub fn permission_denied(msg: &'static str) -> Error {
    Error::PermissionDenied(msg.into())
}

#[cfg(feature = "alloc")]
/// Creates a new busy error
pub fn busy(msg: &str) -> Error {
    Error::Busy(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
/// Creates a new busy error (no-alloc version)
pub fn busy(msg: &'static str) -> Error {
    Error::Busy(msg.into())
}

/// Creates a new out of memory error
pub fn out_of_memory() -> Error {
    Error::OutOfMemory
}

/// Creates a new IO error
#[cfg(feature = "alloc")]
pub fn io_error(msg: &str) -> Error {
    Error::IoError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn io_error(msg: &'static str) -> Error {
    Error::IoError(msg.into())
}

/// Creates a new network error
#[cfg(feature = "alloc")]
pub fn network_error(msg: &str) -> Error {
    Error::NetworkError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn network_error(msg: &'static str) -> Error {
    Error::NetworkError(msg.into())
}

/// Creates a new protocol error
#[cfg(feature = "alloc")]
pub fn protocol_error(msg: &str) -> Error {
    Error::ProtocolError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn protocol_error(msg: &'static str) -> Error {
    Error::ProtocolError(msg.into())
}

/// Creates a new timeout error
pub fn timeout() -> Error {
    Error::Timeout
}

/// Creates a new connection error
#[cfg(feature = "alloc")]
pub fn connection_error(msg: &str) -> Error {
    Error::ConnectionError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn connection_error(msg: &'static str) -> Error {
    Error::ConnectionError(msg.into())
}

/// Creates a new parse error
#[cfg(feature = "alloc")]
pub fn parse_error(msg: &str) -> Error {
    Error::ParseError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn parse_error(msg: &'static str) -> Error {
    Error::ParseError(msg.into())
}

/// Creates a new config error
#[cfg(feature = "alloc")]
pub fn config_error(msg: &str) -> Error {
    Error::ConfigError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn config_error(msg: &'static str) -> Error {
    Error::ConfigError(msg.into())
}

/// Creates a new service error
#[cfg(feature = "alloc")]
pub fn service_error(msg: &str) -> Error {
    Error::ServiceError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn service_error(msg: &'static str) -> Error {
    Error::ServiceError(msg.into())
}

/// Creates a new system error
#[cfg(feature = "alloc")]
pub fn system_error(msg: &str) -> Error {
    Error::SystemError(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn system_error(msg: &'static str) -> Error {
    Error::SystemError(msg.into())
}

/// Creates a new circular dependency error
#[cfg(feature = "alloc")]
pub fn circular_dependency(msg: &str) -> Error {
    Error::CircularDependency(msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn circular_dependency(msg: &'static str) -> Error {
    Error::CircularDependency(msg.into())
}

/// Creates a new custom error
#[cfg(feature = "alloc")]
pub fn custom(code: i32, msg: &str) -> Error {
    Error::Custom(code, msg.to_string())
}

#[cfg(not(feature = "alloc"))]
pub fn custom(code: i32, msg: &'static str) -> Error {
    Error::Custom(code, msg.into())
}