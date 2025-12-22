//! System call handler interface

use crate::error::Result;
use crate::syscall::types::{SyscallNumber, SyscallArgs, SyscallResult};
#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// Trait for handling system calls
pub trait SyscallHandler {
    /// Handles a system call
    fn handle(&mut self, number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult>;
    
    /// Returns the name of the handler
    fn name(&self) -> &str;
    
    /// Checks if the handler supports a specific system call
    fn supports(&self, number: SyscallNumber) -> bool;
}

/// Trait for system call dispatchers
pub trait SyscallDispatcher {
    /// Registers a system call handler
    #[cfg(feature = "alloc")]
    fn register_handler(&mut self, number: SyscallNumber, handler: Box<dyn SyscallHandler>);
    
    #[cfg(not(feature = "alloc"))]
    fn register_handler(&mut self, number: SyscallNumber, handler: crate::interfaces::Box<dyn SyscallHandler>);
    
    /// Unregisters a system call handler
    fn unregister_handler(&mut self, number: SyscallNumber);
    
    /// Dispatches a system call to the appropriate handler
    fn dispatch(&mut self, number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult>;
    
    /// Returns the number of registered handlers
    fn handler_count(&self) -> usize;
}

/// Trait for system call validation
pub trait SyscallValidator {
    /// Validates system call arguments
    fn validate(&self, number: SyscallNumber, args: &SyscallArgs) -> Result<()>;
    
    /// Returns the name of the validator
    fn name(&self) -> &str;
}

/// Trait for system call logging
pub trait SyscallLogger {
    /// Logs a system call
    fn log(&mut self, number: SyscallNumber, args: &SyscallArgs, result: &Result<SyscallResult>);
    
    /// Returns the name of the logger
    fn name(&self) -> &str;
}

/// Trait for system call monitoring
pub trait SyscallMonitor {
    /// Called before a system call is executed
    fn before_syscall(&mut self, number: SyscallNumber, args: &SyscallArgs);
    
    /// Called after a system call is executed
    fn after_syscall(&mut self, number: SyscallNumber, args: &SyscallArgs, result: &Result<SyscallResult>);
    
    /// Returns the name of the monitor
    fn name(&self) -> &str;
}

/// Trait for system call filtering
pub trait SyscallFilter {
    /// Checks if a system call should be allowed
    fn allow(&mut self, number: SyscallNumber, args: &SyscallArgs) -> bool;
    
    /// Returns the name of the filter
    fn name(&self) -> &str;
}