//! Error types for the logging system

#![no_std]

extern crate alloc;

use alloc::fmt;
use alloc::string::String;

/// Result type for logging operations
pub type Result<T> = core::result::Result<T, LogError>;

/// Logging error types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogError {
    /// Queue is full
    QueueFull,
    /// Logger is not running
    LoggerNotRunning,
    /// Logger already initialized
    AlreadyInitialized,
    /// I/O error
    IoError,
    /// Format error
    FormatError(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Filter error
    FilterError(String),
    /// Rotation error
    RotationError(String),
    /// Compression error
    CompressionError(String),
    /// Other error
    Other(String),
    /// Fmt error from write operations
    FmtError,
}

impl From<core::fmt::Error> for LogError {
    fn from(_: core::fmt::Error) -> Self {
        LogError::FmtError
    }
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogError::QueueFull => write!(f, "Log queue is full"),
            LogError::LoggerNotRunning => write!(f, "Logger is not running"),
            LogError::AlreadyInitialized => {
                write!(f, "Logger is already initialized")
            }
            LogError::IoError => write!(f, "I/O error"),
            LogError::FormatError(msg) => write!(f, "Format error: {}", msg),
            LogError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            LogError::FilterError(msg) => write!(f, "Filter error: {}", msg),
            LogError::RotationError(msg) => write!(f, "Rotation error: {}", msg),
            LogError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            LogError::Other(msg) => write!(f, "Error: {}", msg),
            LogError::FmtError => write!(f, "Format error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", LogError::QueueFull),
            "Log queue is full"
        );
        assert_eq!(
            format!("{}", LogError::LoggerNotRunning),
            "Logger is not running"
        );
        assert_eq!(
            format!("{}", LogError::FormatError("test".to_string())),
            "Format error: test"
        );
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(LogError::QueueFull, LogError::QueueFull);
        assert_eq!(LogError::IoError, LogError::IoError);
        assert_ne!(
            LogError::FormatError("a".to_string()),
            LogError::FormatError("b".to_string())
        );
    }

    #[test]
    fn test_result_type() {
        let ok_result: Result<()> = Ok(());
        assert!(ok_result.is_ok());

        let err_result: Result<()> = Err(LogError::QueueFull);
        assert!(err_result.is_err());
    }
}
