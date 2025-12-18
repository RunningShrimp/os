//! Process service module
//!
//! This module provides process-related system call services, including:
//! - Process creation and termination
//! - Process state management
//! - Process scheduling and priority
//! - Inter-process synchronization
//!
//! The module uses a layered architecture design and integrates with the system call dispatcher through service interfaces.

pub mod handlers;
pub mod service;
pub mod types;

// Re-export main interfaces
pub use service::ProcessService;

use crate::syscalls::services::SyscallService;
use alloc::boxed::Box;

/// Get process system call service instance
///
/// Creates and returns an instance of the process system call service.
///
/// # Returns
///
/// * `Box<dyn SyscallService>` - Process system call service instance
pub fn create_process_service() -> Box<dyn SyscallService> {
    Box::new(ProcessService::new())
}

/// Module initialization function
///
/// Initializes the process module and registers necessary system call handlers.
///
/// # Returns
///
/// * `Result<(), crate::error::KernelError>` - Initialization result
pub fn initialize_process_module() -> Result<(), nos_error_handling::KernelError> {
    // TODO: Implement module initialization logic
    crate::log_info!("Initializing process module");
    Ok(())
}