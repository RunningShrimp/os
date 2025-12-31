//! Kernel Error API Interface
//!
//! This module defines unified error types for the kernel.
//! It provides a consistent error handling mechanism across all modules.

use alloc::{string::String, string::ToString, vec::Vec};
use crate::error::{
    unified::UnifiedError,
    unified_framework::{FrameworkError, FrameworkResult, IntoFrameworkError},
};

/// Kernel error type - unified error type
///
/// This type is an alias for the unified UnifiedError
/// to ensure consistent error handling across the kernel.
pub type KernelError = UnifiedError;

/// Kernel result type - migrated to unified framework
pub type KernelResult<T> = FrameworkResult<T>;

/// Generic Result type using KernelError
pub type Result<T> = core::result::Result<T, KernelError>;

/// Convert UnifiedError to KernelError (FrameworkError)
impl IntoFrameworkError for UnifiedError {
    fn into_framework_error(self) -> FrameworkError {
        FrameworkError::Unified(self)
    }

    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        FrameworkError::Contextual {
            error: self,
            context: context.to_string(),
            location: location.to_string(),
        }
    }
}

// KernelError already has to_errno implementation through UnifiedError
// No need to reimplement it here

/// Error context
///
/// This struct provides additional context for errors.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The error that occurred
    pub error: KernelError,
    /// The operation that failed
    pub operation: String,
    /// The file where the error occurred
    pub file: String,
    /// The line where the error occurred
    pub line: u32,
    /// Additional context information
    pub context: Vec<String>,
}

impl ErrorContext {
    /// Create a new error context
    ///
    /// # Arguments
    /// * `error` - The error that occurred
    /// * `operation` - The operation that failed
    /// * `file` - The file where the error occurred
    /// * `line` - The line where the error occurred
    ///
    /// # Returns
    /// * `ErrorContext` - New error context
    pub fn new(error: KernelError, operation: &str, file: &str, line: u32) -> Self {
        Self {
            error,
            operation: operation.to_string(),
            file: file.to_string(),
            line,
            context: Vec::new(),
        }
    }

    /// Add context information
    ///
    /// # Arguments
    /// * `context` - Context information
    pub fn add_context(&mut self, context: &str) {
        self.context.push(context.to_string());
    }

    /// Get the error description with context
    ///
    /// # Returns
    /// * `String` - Error description with context
    pub fn to_string(&self) -> String {
        let mut result = format!(
            "{}: {} ({}:{}): {}",
            self.operation,
            self.error.default_description(),
            self.file,
            self.line,
            self.error.to_errno()
        );

        for context in &self.context {
            result.push_str(&format!(" - {}", context));
        }

        result
    }
}

/// Macro for creating error context
#[macro_export]
macro_rules! error_context {
    ($error:expr, $operation:expr) => {
        ErrorContext::new($error, $operation, file!(), line!())
    };
    ($error:expr, $operation:expr, $($context:expr),*) => {
        {
            let mut ctx = ErrorContext::new($error, $operation, file!(), line!());
            $(ctx.add_context($context);)*
            ctx
        }
    };
}
