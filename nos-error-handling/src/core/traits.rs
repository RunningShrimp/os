//! Core error handling traits
//! 
//! This module defines the core traits for error handling in NOS.
//! Implementations of these traits are in kernel/src/error.

extern crate alloc;

use crate::types::{ErrorRecord, ErrorSeverity, ErrorCategory};
use alloc::string::String;

/// Error handler trait
/// 
/// Implementations of this trait handle errors and decide on actions.
pub trait ErrorHandler: Send + Sync {
    /// Handle an error and return the action to take
    fn handle_error(&self, error: &ErrorContext) -> ErrorAction;
    
    /// Check if this handler can handle the given error
    fn can_handle(&self, error: &ErrorRecord) -> bool;
}

/// Error action to take after handling an error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// Ignore the error
    Ignore,
    /// Log the error
    Log,
    /// Try to recover from the error
    Recover,
    /// Propagate the error to the caller
    Propagate,
    /// Panic the kernel
    Panic,
    /// Shut down the kernel
    Shutdown,
}

/// Error context for error handlers
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The error record
    pub error: ErrorRecord,
    /// Context description
    pub context: String,
    /// File name where error occurred
    pub file: Option<String>,
    /// Line number where error occurred
    pub line: Option<u32>,
    /// Function name where error occurred
    pub function: Option<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(error: ErrorRecord, context: &str) -> Self {
        Self {
            error,
            context: context.into(),
            file: None,
            line: None,
            function: None,
        }
    }
    
    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        self.error.severity
    }
    
    /// Get error description
    pub fn description(&self) -> &str {
        &self.error.message
    }
}

/// Error recovery strategy trait
pub trait RecoveryStrategy: Send + Sync {
    /// Attempt to recover from an error
    fn recover(&self, error: &ErrorContext) -> RecoveryResult;
    
    /// Get the recovery strategy name
    fn name(&self) -> &str;
}

/// Recovery result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    /// Recovery successful
    Success,
    /// Recovery partially successful
    Partial,
    /// Recovery failed
    Failed,
    /// Recovery not applicable
    NotApplicable,
}

/// Error classifier trait
pub trait ErrorClassifier: Send + Sync {
    /// Classify an error
    fn classify(&self, error: &ErrorRecord) -> ErrorCategory;
    
    /// Get classifier name
    fn name(&self) -> &str;
}

/// Health monitor trait
pub trait HealthMonitor: Send + Sync {
    /// Check system health
    fn check_health(&self) -> HealthStatus;
    
    /// Get health monitor name
    fn name(&self) -> &str;
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded
    Degraded,
    /// System is unhealthy
    Unhealthy,
    /// System is critical
    Critical,
}

