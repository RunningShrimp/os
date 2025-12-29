//! System Call Error Conversion Module
//!
//! This module provides unified error conversion mechanisms for system calls.
//! It implements logic for converting various error types (KernelError, UnifiedError,
//! FileSystem errors, etc.) into standard SyscallError type.
//!
//! # Design Principles
//! - Type safety: Preserve error information through conversion chain
//! - Consistency: All system calls return same error type
//! - Traceability: Maintain error context for debugging
//! - Performance: Zero-cost abstractions where possible

use crate::error::UnifiedError;
use crate::error::unified_framework::{FrameworkError, FrameworkResult};
use crate::subsystems::syscalls::interface::SyscallError;
use alloc::string::String;
use alloc::string::ToString;

/// Error conversion trait for system call errors
///
/// This trait allows different error types to be converted to SyscallError
/// while preserving error context.
pub trait IntoSyscallError {
    /// Convert error to a SyscallError
    fn into_syscall_error(self) -> SyscallError;

    /// Convert error to a SyscallError with additional context
    fn into_syscall_error_with_context(self, context: &str) -> SyscallError;
}

impl IntoSyscallError for SyscallError {
    fn into_syscall_error(self) -> SyscallError {
        self
    }

    fn into_syscall_error_with_context(self, _context: &str) -> SyscallError {
        self
    }
}

/// Conversion from UnifiedError to SyscallError
impl IntoSyscallError for UnifiedError {
    fn into_syscall_error(self) -> SyscallError {
        match self {
            UnifiedError::InvalidArgument => SyscallError::InvalidArguments,
            UnifiedError::PermissionDenied => SyscallError::PermissionDenied,
            UnifiedError::NotFound => SyscallError::NotFound,
            UnifiedError::AlreadyExists => SyscallError::AlreadyExists,
            UnifiedError::IoError => SyscallError::IoError,
            UnifiedError::MemoryError(_) => SyscallError::OutOfMemory,
            UnifiedError::Timeout => SyscallError::TimedOut,
            UnifiedError::NotSupported => SyscallError::NotSupported,
            UnifiedError::Busy => SyscallError::ResourceBusy,
            UnifiedError::Interrupted => SyscallError::Interrupted,
            UnifiedError::AccessDenied => SyscallError::AccessDenied,
            UnifiedError::InvalidAddress => SyscallError::InvalidAddress,
            UnifiedError::WouldBlock => SyscallError::WouldBlock,
            UnifiedError::QuotaExceeded => SyscallError::QuotaExceeded,
            _ => SyscallError::Unknown,
        }
    }

    fn into_syscall_error_with_context(self, context: &str) -> SyscallError {
        let syscall_error = self.into_syscall_error();
        #[cfg(feature = "debug")]
        crate::println!("[syscall] UnifiedError in {}: {:?}", context, self);
        syscall_error
    }
}

/// Conversion from FrameworkError to SyscallError
///
/// Note: KernelError is an alias for FrameworkError (defined in api/error.rs),
/// so we only need to implement conversion for FrameworkError.
impl IntoSyscallError for FrameworkError {
    fn into_syscall_error(self) -> SyscallError {
        match self {
            FrameworkError::NotFound(_) => SyscallError::NotFound,
            FrameworkError::PermissionDenied => SyscallError::PermissionDenied,
            FrameworkError::InvalidArgument => SyscallError::InvalidArguments,
            FrameworkError::IoError => SyscallError::IoError,
            FrameworkError::Timeout => SyscallError::TimedOut,
            FrameworkError::NotSupported => SyscallError::NotSupported,
            FrameworkError::AccessDenied => SyscallError::AccessDenied,
            FrameworkError::ResourceBusy => SyscallError::ResourceBusy,
            FrameworkError::OutOfMemory => SyscallError::OutOfMemory,
            _ => SyscallError::Unknown,
        }
    }

    fn into_syscall_error_with_context(self, context: &str) -> SyscallError {
        let syscall_error = self.into_syscall_error();
        #[cfg(feature = "debug")]
        crate::println!("[syscall] FrameworkError in {}: {:?}", context, self);
        syscall_error
    }
}

/// Result type conversion helper
///
/// Converts any Result<T, E> where E: IntoSyscallError to SyscallResult
pub fn convert_result<T, E>(result: Result<T, E>) -> crate::subsystems::syscalls::interface::SyscallResult
where
    T: Into<u64>,
    E: IntoSyscallError,
{
    result.map(|v| v.into()).map_err(|e| e.into_syscall_error())
}

/// Result type conversion helper with context
///
/// Converts any Result<T, E> where E: IntoSyscallError to SyscallResult<i64>
/// adding context information for debugging.
pub fn convert_result_with_context<T, E>(
    result: Result<T, E>,
    context: &str,
) -> crate::subsystems::syscalls::interface::SyscallResult
where
    T: Into<u64>,
    E: IntoSyscallError,
{
    result
        .map(|v| v.into())
        .map_err(|e| e.into_syscall_error_with_context(context))
}

/// Unified syscall result wrapper
///
/// This wrapper provides a unified interface for handling different
/// result types from subsystems.
pub struct UnifiedSyscallResult<T = u64> {
    inner: Result<T, SyscallError>,
}

impl<T> UnifiedSyscallResult<T> {
    /// Create a new unified result
    pub fn new(inner: Result<T, SyscallError>) -> Self {
        Self { inner }
    }

    /// Map success value
    pub fn map<U, F>(self, f: F) -> UnifiedSyscallResult<U>
    where
        F: FnOnce(T) -> U,
    {
        UnifiedSyscallResult<i64>{
            inner: self.inner.map(f),
        }
    }

    /// Map error value
    pub fn map_err<F>(self, f: F) -> UnifiedSyscallResult<T>
    where
        F: FnOnce(SyscallError) -> SyscallError,
    {
        UnifiedSyscallResult<i64>{
            inner: self.inner.map_err(f),
        }
    }

    /// Convert to standard SyscallResult
    pub fn into_syscall_result(self) -> crate::subsystems::syscalls::interface::SyscallResult
    where
        T: Into<u64>,
    {
        self.inner.map(|v| v.into())
    }

    /// Check if result is ok
    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }

    /// Check if result is an error
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
    }

    /// Get the errno value for error results
    pub fn to_errno(&self) -> i32 {
        match &self.inner {
            Ok(_) => 0,
            Err(e) => e.to_errno(),
        }
    }
}

impl<T> From<Result<T, SyscallError>> for UnifiedSyscallResult<T> {
    fn from(inner: Result<T, SyscallError>) -> Self {
        Self { inner }
    }
}

impl<T> From<UnifiedSyscallResult<T>> for Result<T, SyscallError> {
    fn from(result: UnifiedSyscallResult<T>) -> Self {
        result.inner
    }
}

/// System call error context
///
/// Provides additional context for system call errors, including
/// syscall number, operation name, and additional information.
#[derive(Debug, Clone)]
pub struct SyscallErrorContext {
    /// System call number
    pub syscall_number: u32,
    /// System call name
    pub syscall_name: &'static str,
    /// Operation being performed
    pub operation: String,
    /// Additional context information
    pub additional_info: Option<String>,
}

impl SyscallErrorContext {
    /// Create a new error context
    pub fn new(syscall_number: u32, syscall_name: &'static str, operation: &str) -> Self {
        Self {
            syscall_number,
            syscall_name,
            operation: operation.to_string(),
            additional_info: None,
        }
    }

    /// Add additional information to context
    pub fn with_info(mut self, info: &str) -> Self {
        self.additional_info = Some(info.to_string());
        self
    }

    /// Format error context as a string
    pub fn format(&self) -> String {
        let mut result = {
            let mut s = alloc::string::String::from("syscall #");
            s.push_str(&self.syscall_number.to_string());
            s.push_str(" (");
            s.push_str(&self.syscall_name);
            s.push_str(") - ");
            s.push_str(&self.operation.to_string());
            s
        };

        if let Some(ref info) = self.additional_info {
            result.push_str(&{ let mut s = alloc::string::String::from(": "); s.push_str(&info.to_string()); s });
        }

        result
    }
}

/// Enhanced error conversion with context
///
/// Converts errors to SyscallError with full context preservation.
pub fn convert_error_with_context<E>(
    error: E,
    context: &SyscallErrorContext,
) -> SyscallError
where
    E: IntoSyscallError,
{
    let syscall_error = error.into_syscall_error();
    #[cfg(feature = "syscall-debug")]
    crate::println!(
        "[syscall] Error in {}: {:?}",
        context.format(),
        syscall_error
    );
    syscall_error
}

/// Helper function to handle common system call error scenarios
pub fn handle_syscall_error<E>(error: E, context: &SyscallErrorContext) -> SyscallError
where
    E: IntoSyscallError,
{
    convert_error_with_context(error, context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_error_conversion() {
        // Since KernelError is an alias for FrameworkError, test it through FrameworkError
        let framework_err = FrameworkError::NotFound;
        let syscall_err = framework_err.into_syscall_error();
        assert_eq!(syscall_err, SyscallError::NotFound);
    }

    #[test]
    fn test_unified_error_conversion() {
        let unified_err = UnifiedError::PermissionDenied;
        let syscall_err = unified_err.into_syscall_error();
        assert_eq!(syscall_err, SyscallError::PermissionDenied);
    }

    #[test]
    fn test_framework_error_conversion() {
        let framework_err = FrameworkError::IoError;
        let syscall_err = framework_err.into_syscall_error();
        assert_eq!(syscall_err, SyscallError::IoError);
    }

    #[test]
    fn test_result_conversion() {
        let result: Result<u32, FrameworkError> = Ok(42);
        let syscall_result = convert_result(result);
        assert_eq!(syscall_result, Ok(42u64));
    }

    #[test]
    fn test_error_result_conversion() {
        let result: Result<u32, FrameworkError> = Err(FrameworkError::NotFound);
        let syscall_result = convert_result(result);
        assert_eq!(syscall_result, Err(SyscallError::NotFound));
    }

    #[test]
    fn test_error_context_formatting() {
        let context = SyscallErrorContext::new(0x1000, "read", "reading from file")
            .with_info("fd: 3, offset: 1024");
        let formatted = context.format();
        assert!(formatted.contains("read"));
        assert!(formatted.contains("reading from file"));
        assert!(formatted.contains("fd: 3"));
    }

    #[test]
    fn test_unified_result() {
        let result: Result<u32, SyscallError> = Ok(123);
        let unified = UnifiedSyscallResult<i64>:new(result);
        assert!(unified.is_ok());
        assert!(!unified.is_err());
        assert_eq!(unified.to_errno(), 0);
    }

    #[test]
    fn test_unified_result_error() {
        let result: Result<u32, SyscallError> = Err(SyscallError::InvalidFd);
        let unified = UnifiedSyscallResult<i64>:new(result);
        assert!(!unified.is_ok());
        assert!(unified.is_err());
        assert_eq!(unified.to_errno(), 9); // EBADF
    }
}
