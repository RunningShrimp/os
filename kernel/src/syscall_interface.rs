//! System Call Interface Module
//!
//! This module provides abstract interfaces for system call handling,
//! breaking the circular dependency between syscalls and services modules.

use nos_api::Result;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;

/// System call dispatcher interface
/// This trait defines the interface for dispatching system calls
/// without depending on concrete implementations
pub trait SyscallDispatcher {
    /// Dispatch a system call
    ///
    /// # Arguments
    /// * `syscall_num` - System call number
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `isize` - System call return value
    fn dispatch(&self, syscall_num: usize, args: &[usize]) -> isize;
    
    /// Get system call statistics
    ///
    /// # Returns
    /// * `SyscallStats` - System call statistics
    fn get_stats(&self) -> SyscallStats;
}

/// System call statistics
#[derive(Debug, Clone)]
pub struct SyscallStats {
    /// Total number of system calls
    pub total_calls: u64,
    /// Number of successful system calls
    pub successful_calls: u64,
    /// Number of failed system calls
    pub failed_calls: u64,
    /// Average execution time (in nanoseconds)
    pub avg_execution_time_ns: u64,
}

impl Default for SyscallStats {
    fn default() -> Self {
        Self {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            avg_execution_time_ns: 0,
        }
    }
}

/// Service manager interface
/// This trait defines the interface for managing services
/// without depending on concrete implementations
pub trait ServiceManager {
    /// Register a new service
    ///
    /// # Arguments
    /// * `service` - Service to register
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    fn register_service(&mut self, service: Arc<dyn Service>) -> Result<()>;
    
    /// Get a service by name
    ///
    /// # Arguments
    /// * `name` - Service name
    ///
    /// # Returns
    /// * `Option<Arc<dyn Service>>` - Service if found
    fn get_service(&self, name: &str) -> Option<Arc<dyn Service>>;
    
    /// List all registered services
    ///
    /// # Returns
    /// * `Vec<&str>` - List of service names
    fn list_services(&self) -> Vec<&str>;
    
    /// Get service statistics
    ///
    /// # Returns
    /// * `ServiceStats` - Service statistics
    fn get_stats(&self) -> ServiceStats;
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// Total number of services
    pub total_services: u64,
    /// Number of active services
    pub active_services: u64,
    /// Number of failed services
    pub failed_services: u64,
}

impl Default for ServiceStats {
    fn default() -> Self {
        Self {
            total_services: 0,
            active_services: 0,
            failed_services: 0,
        }
    }
}

/// Service trait for system call services
/// This trait defines the interface for services that handle system calls
pub trait Service {
    /// Get service name
    ///
    /// # Returns
    /// * `&str` - Service name
    fn name(&self) -> &str;
    
    /// Get service version
    ///
    /// # Returns
    /// * `&str` - Service version
    fn version(&self) -> &str;
    
    /// Initialize the service
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    fn initialize(&mut self) -> Result<()>;
    
    /// Shutdown the service
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    fn shutdown(&mut self) -> Result<()>;
    
    /// Handle a system call
    ///
    /// # Arguments
    /// * `syscall_num` - System call number
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `isize` - System call return value
    fn handle_syscall(&mut self, syscall_num: usize, args: &[usize]) -> isize;
    
    /// Get service status
    ///
    /// # Returns
    /// * `ServiceStatus` - Service status
    fn get_status(&self) -> ServiceStatus;
    
    /// Get service health
    ///
    /// # Returns
    /// * `ServiceHealth` - Service health
    fn get_health(&self) -> ServiceHealth;
}

/// Service lifecycle status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    /// Service is uninitialized
    Uninitialized,
    /// Service is initializing
    Initializing,
    /// Service is running
    Running,
    /// Service is stopping
    Stopping,
    /// Service is stopped
    Stopped,
    /// Service has failed
    Failed,
}

/// Service health status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    /// Service is healthy
    Healthy,
    /// Service is degraded
    Degraded,
    /// Service is unhealthy
    Unhealthy,
}

/// Service metadata
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service description
    pub description: String,
    /// Service dependencies
    pub dependencies: Vec<String>,
    /// Service capabilities
    pub capabilities: Vec<String>,
}

impl ServiceMetadata {
    /// Create new service metadata
    ///
    /// # Arguments
    /// * `name` - Service name
    /// * `version` - Service version
    /// * `description` - Service description
    ///
    /// # Returns
    /// * `Self` - Service metadata
    pub fn new(name: &str, version: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            dependencies: Vec::new(),
            capabilities: Vec::new(),
        }
    }
}