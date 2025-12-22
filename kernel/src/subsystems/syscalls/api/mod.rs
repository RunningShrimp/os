//! Public API for the syscalls module.
//!
//! This module contains all the public interfaces and types for the syscalls
//! module. Internal implementation details are hidden.

pub mod syscall_id;
pub mod syscall_result;
pub mod syscall_context;

// Re-export the main API types for convenience
pub use syscall_id::{SyscallId, syscall_ids};
pub use syscall_result::{SyscallResult, SyscallError};
pub use syscall_context::SyscallContext;

// Re-export service-related types from services module
pub use super::services::traits::{Service, SyscallService, ServiceLifecycle, ServiceStatus, ServiceHealth};
pub use super::services::registry::{
    ServiceRegistry, ServiceEntry, ServiceMetadata, ServiceType, Version,
    DependencyGraph, ServiceRegistryError
};

// Re-export dispatcher-related types
pub use super::services::dispatcher::{
    SyscallDispatcher, DispatchResult, DispatchStats, DispatcherConfig
};

// Re-export service system types
pub use super::services::{ServiceSystem, SystemStats, init_service_system, create_default_service_system};

/// System call batch operation result.
pub struct SyscallBatchResult {
    /// Results for each system call in the batch.
    pub results: alloc::vec::Vec<SyscallResult>,
    /// Total time taken to execute all system calls.
    pub total_time: u64,
}

impl SyscallBatchResult {
    /// Create a new SyscallBatchResult.
    pub fn new(results: alloc::vec::Vec<SyscallResult>, total_time: u64) -> Self {
        Self {
            results,
            total_time,
        }
    }
    
    /// Check if all system calls in the batch succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.results.iter().all(|r| r.is_ok())
    }
    
    /// Get the number of successful system calls.
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_ok()).count()
    }
    
    /// Get the number of failed system calls.
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_err()).count()
    }
}