//! Error Types Module
//!
//! This module defines error types for the NOS kernel,
//! providing compatibility with the deprecated nos-error-handling crate.

/// Error types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorType {
    /// Runtime error
    RuntimeError,
    /// Logic error
    LogicError,
    /// Compile error
    CompileError,
    /// Configuration error
    ConfigurationError,
    /// Resource error
    ResourceError,
    /// Permission error
    PermissionError,
    /// Network error
    NetworkError,
    /// I/O error
    IOError,
    /// Memory error
    MemoryError,
    /// System call error
    SystemCallError,
    /// Validation error
    ValidationError,
    /// Timeout error
    TimeoutError,
    /// Cancellation error
    CancellationError,
    /// System error (compatibility with old code)
    SystemError,
}

impl core::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorType::RuntimeError => write!(f, "RuntimeError"),
            ErrorType::LogicError => write!(f, "LogicError"),
            ErrorType::CompileError => write!(f, "CompileError"),
            ErrorType::ConfigurationError => write!(f, "ConfigurationError"),
            ErrorType::ResourceError => write!(f, "ResourceError"),
            ErrorType::PermissionError => write!(f, "PermissionError"),
            ErrorType::NetworkError => write!(f, "NetworkError"),
            ErrorType::IOError => write!(f, "IOError"),
            ErrorType::MemoryError => write!(f, "MemoryError"),
            ErrorType::SystemCallError => write!(f, "SystemCallError"),
            ErrorType::ValidationError => write!(f, "ValidationError"),
            ErrorType::TimeoutError => write!(f, "TimeoutError"),
            ErrorType::CancellationError => write!(f, "CancellationError"),
            ErrorType::SystemError => write!(f, "SystemError"),
        }
    }
}

/// Conversion from external nos-error-handling ErrorType to local ErrorType
/// This allows gradual migration away from the deprecated enum
#[allow(deprecated)]
pub fn from_nos_error_type(nos_error: &nos_error_handling::kernel_integration::ErrorType) -> ErrorType {
    match nos_error {
        nos_error_handling::kernel_integration::ErrorType::RuntimeError => ErrorType::RuntimeError,
        nos_error_handling::kernel_integration::ErrorType::LogicError => ErrorType::LogicError,
        nos_error_handling::kernel_integration::ErrorType::CompileError => ErrorType::CompileError,
        nos_error_handling::kernel_integration::ErrorType::ConfigurationError => ErrorType::ConfigurationError,
        nos_error_handling::kernel_integration::ErrorType::ResourceError => ErrorType::ResourceError,
        nos_error_handling::kernel_integration::ErrorType::PermissionError => ErrorType::PermissionError,
        nos_error_handling::kernel_integration::ErrorType::NetworkError => ErrorType::NetworkError,
        nos_error_handling::kernel_integration::ErrorType::IOError => ErrorType::IOError,
        nos_error_handling::kernel_integration::ErrorType::MemoryError => ErrorType::MemoryError,
        nos_error_handling::kernel_integration::ErrorType::SystemCallError => ErrorType::SystemCallError,
        nos_error_handling::kernel_integration::ErrorType::ValidationError => ErrorType::ValidationError,
        nos_error_handling::kernel_integration::ErrorType::TimeoutError => ErrorType::TimeoutError,
        nos_error_handling::kernel_integration::ErrorType::CancellationError => ErrorType::CancellationError,
        nos_error_handling::kernel_integration::ErrorType::SystemError => ErrorType::SystemError,
    }
}

/// Conversion from local ErrorType to external nos-error-handling ErrorType
/// This is provided for backward compatibility
#[allow(deprecated)]
pub fn to_nos_error_type(local_error: ErrorType) -> nos_error_handling::kernel_integration::ErrorType {
    match local_error {
        ErrorType::RuntimeError => nos_error_handling::kernel_integration::ErrorType::RuntimeError,
        ErrorType::LogicError => nos_error_handling::kernel_integration::ErrorType::LogicError,
        ErrorType::CompileError => nos_error_handling::kernel_integration::ErrorType::CompileError,
        ErrorType::ConfigurationError => nos_error_handling::kernel_integration::ErrorType::ConfigurationError,
        ErrorType::ResourceError => nos_error_handling::kernel_integration::ErrorType::ResourceError,
        ErrorType::PermissionError => nos_error_handling::kernel_integration::ErrorType::PermissionError,
        ErrorType::NetworkError => nos_error_handling::kernel_integration::ErrorType::NetworkError,
        ErrorType::IOError => nos_error_handling::kernel_integration::ErrorType::IOError,
        ErrorType::MemoryError => nos_error_handling::kernel_integration::ErrorType::MemoryError,
        ErrorType::SystemCallError => nos_error_handling::kernel_integration::ErrorType::SystemCallError,
        ErrorType::ValidationError => nos_error_handling::kernel_integration::ErrorType::ValidationError,
        ErrorType::TimeoutError => nos_error_handling::kernel_integration::ErrorType::TimeoutError,
        ErrorType::CancellationError => nos_error_handling::kernel_integration::ErrorType::CancellationError,
        ErrorType::SystemError => nos_error_handling::kernel_integration::ErrorType::SystemError,
    }
}