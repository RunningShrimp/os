//! Unified Error Handling Framework
//! 
//! This module provides a comprehensive error handling framework that
//! unifies all error types across the kernel. It includes error conversion
//! mechanisms, error context management, and error recovery integration.

extern crate alloc;

use alloc::string::String;
use alloc::format;
use core::fmt;
use core::any::Any;

// Re-export existing unified error types
use super::unified::*;

/// Unified error result type
pub type FrameworkResult<T> = core::result::Result<T, FrameworkError>;

/// Enhanced framework error type
#[derive(Debug, Clone)]
pub enum FrameworkError {
    /// Wrapped unified error
    Unified(UnifiedError),
    /// Error with additional context
    Contextual {
        /// The underlying error
        error: UnifiedError,
        /// Additional context information
        context: String,
        /// Error location
        location: String,
    },
    /// Error chain for nested errors
    Chain {
        /// The main error
        error: UnifiedError,
        /// The cause of this error
        cause: Box<FrameworkError>,
    },
}

/// Error conversion trait
pub trait IntoFrameworkError {
    /// Convert self into a FrameworkError
    fn into_framework_error(self) -> FrameworkError;
    
    /// Convert self into a FrameworkError with context
    fn with_context(self, context: &str, location: &str) -> FrameworkError;
}

/// Error conversion implementation for UnifiedError
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

/// Error conversion implementation for &str
impl IntoFrameworkError for &str {
    fn into_framework_error(self) -> FrameworkError {
        UnifiedError::Other(self.to_string()).into_framework_error()
    }
    
    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        UnifiedError::Other(self.to_string()).with_context(context, location)
    }
}

/// Error conversion implementation for String
impl IntoFrameworkError for String {
    fn into_framework_error(self) -> FrameworkError {
        UnifiedError::Other(self).into_framework_error()
    }
    
    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        UnifiedError::Other(self).with_context(context, location)
    }
}

/// Error conversion macro
#[macro_export]
macro_rules! err {
    ($err:expr) => {
        $err.into_framework_error()
    };
    
    ($err:expr, $context:expr) => {
        $err.with_context($context, module_path!())
    };
    
    ($err:expr, $context:expr, $location:expr) => {
        $err.with_context($context, $location)
    };
}

/// Result conversion macro
#[macro_export]
macro_rules! res {
    ($res:expr) => {
        $res.map_err(|e| e.into_framework_error())
    };
    
    ($res:expr, $context:expr) => {
        $res.map_err(|e| e.with_context($context, module_path!()))
    };
    
    ($res:expr, $context:expr, $location:expr) => {
        $res.map_err(|e| e.with_context($context, $location))
    };
}

/// Error handler trait with enhanced capabilities
pub trait FrameworkErrorHandler: Send + Sync + 'static {
    /// Handle an error and return appropriate action
    fn handle_error(&self, error: &FrameworkError) -> ErrorAction;
    
    /// Check if this handler can handle the given error
    fn can_handle(&self, error: &FrameworkError) -> bool;
    
    /// Get handler name for identification
    fn name(&self) -> &str;
}

/// Error manager with framework integration
pub struct FrameworkErrorManager {
    inner: super::ErrorManager,
    handlers: alloc::vec::Vec<Box<dyn FrameworkErrorHandler>>,
}

impl FrameworkErrorManager {
    /// Create a new framework error manager
    pub const fn new() -> Self {
        Self {
            inner: super::ErrorManager::new(),
            handlers: alloc::vec::Vec::new(),
        }
    }
    
    /// Add a framework error handler
    pub fn add_framework_handler(&mut self, handler: Box<dyn FrameworkErrorHandler>) {
        self.handlers.push(handler);
    }
    
    /// Handle an error using the framework
    pub fn handle_framework_error(&self, error: FrameworkError) -> ErrorAction {
        // Try framework handlers first
        for handler in &self.handlers {
            if handler.can_handle(&error) {
                let action = handler.handle_error(&error);
                if action != ErrorAction::Propagate {
                    return action;
                }
            }
        }
        
        // Fall back to inner error manager
        match &error {
            FrameworkError::Unified(e) => {
                self.inner.handle_error(e.clone(), "")
            }
            FrameworkError::Contextual { error, context, location } => {
                self.inner.handle_error(error.clone(), &format!("{} at {}", context, location))
            }
            FrameworkError::Chain { error, cause: _ } => {
                self.inner.handle_error(error.clone(), "")
            }
        }
    }
}

/// Error recovery trait
pub trait ErrorRecovery: Send + Sync + 'static {
    /// Attempt to recover from an error
    fn recover(&self, error: &FrameworkError) -> bool;
    
    /// Get recovery strategy name
    fn strategy_name(&self) -> &str;
}

/// Default error recovery implementation
pub struct DefaultErrorRecovery;

impl ErrorRecovery for DefaultErrorRecovery {
    fn recover(&self, error: &FrameworkError) -> bool {
        // Simple recovery strategy based on error type
        match error {
            FrameworkError::Unified(e) => match e {
                UnifiedError::OutOfMemory => {
                    // Try to free some memory
                    crate::mm::free_unused_memory();
                    true
                }
                UnifiedError::ResourceBusy => {
                    // Wait and retry
                    crate::time::sleep(10);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
    
    fn strategy_name(&self) -> &str {
        "default"
    }
}

/// Error context builder
pub struct ErrorContextBuilder {
    error: UnifiedError,
    context: String,
    location: String,
}

impl ErrorContextBuilder {
    /// Create a new error context builder
    pub fn new(error: UnifiedError) -> Self {
        Self {
            error,
            context: String::new(),
            location: module_path!().to_string(),
        }
    }
    
    /// Add context information
    pub fn with_context(mut self, context: &str) -> Self {
        self.context = context.to_string();
        self
    }
    
    /// Set error location
    pub fn with_location(mut self, location: &str) -> Self {
        self.location = location.to_string();
        self
    }
    
    /// Build the framework error
    pub fn build(self) -> FrameworkError {
        FrameworkError::Contextual {
            error: self.error,
            context: self.context,
            location: self.location,
        }
    }
}

/// Implement Display for FrameworkError
impl fmt::Display for FrameworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameworkError::Unified(e) => write!(f, "{:?}", e),
            FrameworkError::Contextual { error, context, location } => {
                write!(f, "{}: {:?} at {}", context, error, location)
            }
            FrameworkError::Chain { error, cause } => {
                write!(f, "{:?}: caused by {}", error, cause)
            }
        }
    }
}

/// Implement Error for FrameworkError
impl core::error::Error for FrameworkError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            FrameworkError::Chain { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

/// Initialize the error framework
pub fn init_framework() -> FrameworkResult<()> {
    crate::log_info!("Initializing unified error framework");
    
    // Initialize existing error handling
    super::init()?;
    
    // Add default recovery strategy
    let recovery = DefaultErrorRecovery;
    // TODO: Register recovery strategy
    
    Ok(())
}

/// Shutdown the error framework
pub fn shutdown_framework() -> FrameworkResult<()> {
    crate::log_info!("Shutting down unified error framework");
    
    // Shutdown existing error handling
    super::shutdown()?;
    
    Ok(())
}

// Helper macros
#[macro_export]
macro_rules! try_context {
    ($expr:expr, $context:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(e.with_context($context, module_path!())),
        }
    };
    
    ($expr:expr, $context:expr, $location:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(e.with_context($context, $location)),
        }
    };
}

#[macro_export]
macro_rules! bail {
    ($err:expr) => {
        return Err($err.into_framework_error());
    };
    
    ($err:expr, $context:expr) => {
        return Err($err.with_context($context, module_path!()));
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            bail!($err);
        }
    };
    
    ($cond:expr, $err:expr, $context:expr) => {
        if !$cond {
            bail!($err, $context);
        }
    };
}