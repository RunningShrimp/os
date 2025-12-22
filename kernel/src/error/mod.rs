//! Error Handling Module
//!
//! This module provides unified error handling for the NOS kernel,
//! including error types, error contexts, and error recovery mechanisms.

extern crate alloc;

use alloc::vec::Vec;

/// Initialize error handling subsystem
pub fn init() -> crate::error::UnifiedResult<()> {
    crate::log_info!("Error handling subsystem initialized");
    Ok(())
}

/// Shutdown error handling subsystem
pub fn shutdown() -> crate::error::UnifiedResult<()> {
    crate::log_info!("Error handling subsystem shutdown");
    Ok(())
}

// Re-export submodules
pub mod unified;
pub mod unified_mapping;
pub mod panic_handler;
pub mod recovery;

// Re-export unified mapping
pub use unified_mapping::{
    Errno, UnifiedErrorMapper, init_error_mapper, get_error_mapper,
    unified_error_to_errno, api_syscall_error_to_errno,
};

// Re-export main types for convenience
pub use unified::{
    UnifiedError, UnifiedResult, ErrorContext, ErrorSeverity,
    MemoryError, FileSystemError, NetworkError, ProcessError,
    SyscallError, DriverError, SecurityError,
    create_error, return_error,
};

// TODO: Implement and re-export errno types
// pub use errno::{Errno, set_errno, get_errno};

/// Error handler trait
pub trait ErrorHandler {
    /// Handle an error
    fn handle_error(&self, error: &ErrorContext) -> ErrorAction;
    
    /// Check if this handler can handle the given error
    fn can_handle(&self, error: &UnifiedError) -> bool;
}

/// Error action
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

/// Error manager
pub struct ErrorManager {
    handlers: Vec<Box<dyn ErrorHandler>>,
    error_count: core::sync::atomic::AtomicUsize,
    critical_error_count: core::sync::atomic::AtomicUsize,
}

impl ErrorManager {
    /// Create a new error manager
    pub const fn new() -> Self {
        Self {
            handlers: Vec::new(),
            error_count: core::sync::atomic::AtomicUsize::new(0),
            critical_error_count: core::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    /// Add an error handler
    pub fn add_handler(&mut self, handler: Box<dyn ErrorHandler>) {
        self.handlers.push(handler);
    }
    
    /// Handle an error (with recovery integration)
    pub fn handle_error(&self, error: UnifiedError, context: &str) -> ErrorAction {
        // Record error for recovery statistics
        if let Some(recovery_mgr) = recovery::get_recovery_manager() {
            recovery_mgr.record_error(&error);
        }
        
        // Create error context
        let error_context = ErrorContext::new(error.clone(), context);
        
        // Increment error counters
        self.error_count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        
        if error_context.severity >= ErrorSeverity::Critical {
            self.critical_error_count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }
        
        // Try to find a handler that can handle this error
        for handler in &self.handlers {
            if handler.can_handle(&error) {
                let action = handler.handle_error(&error_context);
                
                // If handler suggests recovery, integrate with recovery manager
                if action == ErrorAction::Recover {
                    if let Some(recovery_action) = recovery::determine_recovery_action(&error, &error_context) {
                        let recovery_result = recovery::execute_recovery(recovery_action, &error, &error_context);
                        match recovery_result {
                            recovery::RecoveryResult::Success => {
                                recovery::record_recovery_success(&error);
                                return ErrorAction::Ignore;
                            }
                            recovery::RecoveryResult::Failed | recovery::RecoveryResult::Timeout => {
                                recovery::record_recovery_failure(&error);
                                return ErrorAction::Propagate;
                            }
                            _ => {}
                        }
                    }
                }
                
                return action;
            }
        }
        
        // Default action based on severity (with recovery support)
        match error_context.severity {
            ErrorSeverity::Info | ErrorSeverity::Warning => ErrorAction::Log,
            ErrorSeverity::Error => {
                // For errors, try recovery if available
                if let Some(recovery_action) = recovery::determine_recovery_action(&error, &error_context) {
                    let recovery_result = recovery::execute_recovery(recovery_action, &error, &error_context);
                    match recovery_result {
                        recovery::RecoveryResult::Success => {
                            recovery::record_recovery_success(&error);
                            ErrorAction::Ignore
                        }
                        _ => ErrorAction::Propagate
                    }
                } else {
                    ErrorAction::Propagate
                }
            }
            ErrorSeverity::Critical | ErrorSeverity::Fatal => ErrorAction::Panic,
        }
    }
    
    /// Get error statistics
    pub fn get_stats(&self) -> ErrorStats {
        ErrorStats {
            total_errors: self.error_count.load(core::sync::atomic::Ordering::Relaxed),
            critical_errors: self.critical_error_count.load(core::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Error statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorStats {
    /// Total number of errors
    pub total_errors: usize,
    /// Number of critical errors
    pub critical_errors: usize,
}

/// Global error manager
static mut ERROR_MANAGER: ErrorManager = ErrorManager::new();
static ERROR_MANAGER_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Get the global error manager
pub fn get_error_manager() -> &'static mut ErrorManager {
    unsafe {
        if !ERROR_MANAGER_INIT.load(core::sync::atomic::Ordering::Acquire) {
            ERROR_MANAGER = ErrorManager::new();
            ERROR_MANAGER_INIT.store(true, core::sync::atomic::Ordering::Release);
        }
        &mut ERROR_MANAGER
    }
}

/// Handle an error with context (with recovery support)
pub fn handle_error(error: UnifiedError, context: &str) -> ErrorAction {
    let action = get_error_manager().handle_error(error.clone(), context);
    
    // If action is Recover, try to execute recovery
    if action == ErrorAction::Recover {
        let error_context = ErrorContext::new(error.clone(), context);
        if let Some(recovery_action) = recovery::determine_recovery_action(&error, &error_context) {
            let recovery_result = recovery::execute_recovery(recovery_action, &error, &error_context);
            match recovery_result {
                recovery::RecoveryResult::Success => {
                    recovery::record_recovery_success(&error);
                    return ErrorAction::Ignore; // Recovery succeeded, ignore error
                }
                recovery::RecoveryResult::Failed | recovery::RecoveryResult::Timeout => {
                    recovery::record_recovery_failure(&error);
                    return ErrorAction::Propagate; // Recovery failed, propagate error
                }
                recovery::RecoveryResult::NotApplicable => {
                    // No recovery available, use original action
                }
            }
        }
    }
    
    action
}

/// Handle an error with file and line context
pub fn handle_error_with_location(
    error: UnifiedError,
    file: &str,
    line: u32,
    function: &str,
) -> ErrorAction {
    let context = format!("{}:{} in {}", file, line, function);
    handle_error(error, &context)
}

/// Get error statistics
pub fn get_error_stats() -> ErrorStats {
    get_error_manager().get_stats()
}

/// Default error handler
pub struct DefaultErrorHandler;

impl ErrorHandler for DefaultErrorHandler {
    fn handle_error(&self, error: &ErrorContext) -> ErrorAction {
        match error.error {
            UnifiedError::MemoryError(MemoryError::OutOfMemory) => {
                crate::log_error!("Out of memory: {}", error.description);
                ErrorAction::Recover
            }
            UnifiedError::FileSystemError(FileSystemError::PermissionDenied) => {
                crate::log_warn!("Permission denied: {}", error.description);
                ErrorAction::Propagate
            }
            UnifiedError::SecurityError(SecurityError::AccessDenied) => {
                crate::log_error!("Security violation: {}", error.description);
                ErrorAction::Panic
            }
            _ => {
                crate::log_error!("Error: {}", error.description);
                ErrorAction::Propagate
            }
        }
    }
    
    fn can_handle(&self, _error: &UnifiedError) -> bool {
        true // Default handler can handle all errors
    }
}

/// Initialize default error handlers
pub fn init_default_handlers() {
    get_error_manager().add_handler(Box::new(DefaultErrorHandler));
}