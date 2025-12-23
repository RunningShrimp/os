//! Error handling module for NOS operating system

use core::fmt;
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::format;


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
    ServiceError(String),
    /// System error
    SystemError(String),
    /// Circular dependency error
    CircularDependency(String),
    /// Custom error with code and message
    Custom(i32, String),
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
            Error::ServiceError(msg) => write!(f, "Service error: {}", msg),
            Error::SystemError(msg) => write!(f, "System error: {}", msg),
            Error::CircularDependency(msg) => write!(f, "Circular dependency: {}", msg),
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
            Err(error) => Err(Error::SystemError(format!("{}: {}", _context, error))),
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
        self.error = Error::SystemError(format!("{}: {}", _context, self.error));
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

/// Creates a new invalid argument error
pub fn invalid_argument(msg: &str) -> Error {
    Error::InvalidArgument(msg.to_string())
}

/// Creates a new not implemented error
pub fn not_implemented(msg: &str) -> Error {
    Error::NotImplemented(msg.to_string())
}

/// Creates a new not found error
pub fn not_found(msg: &str) -> Error {
    Error::NotFound(msg.to_string())
}

/// Creates a new permission denied error
pub fn permission_denied(msg: &str) -> Error {
    Error::PermissionDenied(msg.to_string())
}

/// Creates a new busy error
pub fn busy(msg: &str) -> Error {
    Error::Busy(msg.to_string())
}

/// Creates a new out of memory error
pub fn out_of_memory() -> Error {
    Error::OutOfMemory
}

/// Creates a new IO error
pub fn io_error(msg: &str) -> Error {
    Error::IoError(msg.to_string())
}

/// Creates a new network error
pub fn network_error(msg: &str) -> Error {
    Error::NetworkError(msg.to_string())
}

/// Creates a new protocol error
pub fn protocol_error(msg: &str) -> Error {
    Error::ProtocolError(msg.to_string())
}

/// Creates a new timeout error
pub fn timeout() -> Error {
    Error::Timeout
}

/// Creates a new connection error
pub fn connection_error(msg: &str) -> Error {
    Error::ConnectionError(msg.to_string())
}

/// Creates a new parse error
pub fn parse_error(msg: &str) -> Error {
    Error::ParseError(msg.to_string())
}

/// Creates a new config error
pub fn config_error(msg: &str) -> Error {
    Error::ConfigError(msg.to_string())
}

/// Creates a new service error
pub fn service_error(msg: &str) -> Error {
    Error::ServiceError(msg.to_string())
}

/// Creates a new system error
pub fn system_error(msg: &str) -> Error {
    Error::SystemError(msg.to_string())
}

/// Creates a new circular dependency error
pub fn circular_dependency(msg: &str) -> Error {
    Error::CircularDependency(msg.to_string())
}

/// Creates a new custom error
pub fn custom(code: i32, msg: &str) -> Error {
    Error::Custom(code, msg.to_string())
}