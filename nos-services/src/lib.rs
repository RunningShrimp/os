//! NOS Services
//!
//! This crate provides service management and discovery framework for NOS operating system.
//! It includes service registry, discovery, and lifecycle management.
//!
//! # Architecture
//!
//! The services module is organized into several functional domains:
//!
//! - **Registry**: Service registration and lookup
//! - **Discovery**: Service discovery and enumeration
//! - **Core**: Core service interfaces and types
//! - **FS**: File system services
//! - **Process**: Process services
//! - **Network**: Network services
//! - **IPC**: IPC services
//!
//! # Usage
//!
//! ```rust
//! use nos_services::{ServiceRegistry, Service};
//!
//! // Create a service registry
//! let mut registry = ServiceRegistry::new();
//!
//! // Register a service
//! let service = MyService::new();
//! let id = registry.register("my_service", Box::new(service))?;
//!
//! // Get a service
//! let service = registry.get_by_name("my_service")?;
//! ```

#![no_std]
#![allow(dead_code)]

extern crate alloc;

// Core modules
pub mod registry;
pub mod discovery;
pub mod core;
pub mod fs;
pub mod process;
pub mod network;
pub mod ipc;
pub mod types;

// Re-export commonly used items
pub use registry::{ServiceRegistry, ServiceInfo, register_service, unregister_service, get_service, get_stats};
pub use discovery::{ServiceDiscovery, ServiceDescriptor};
pub use core::{Service, ServiceStatus, ServiceStats};
pub use core::{ServiceManager, ServiceConfig};
// Note: fs, process, network, ipc modules are not re-exported to avoid unused import warnings
pub use types::{ServicePriority, ServiceMetrics, ServiceDependency};
pub use types::service_type::*;

/// Initialize the services subsystem
///
/// This function initializes the service registry
/// and discovery mechanisms.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn init_services() -> nos_api::Result<()> {
    // Initialize service registry
    registry::init_registry()?;
    
    // Initialize service discovery
    discovery::init_discovery()?;
    
    Ok(())
}

/// Shutdown the services subsystem
///
/// This function shuts down the service registry
/// and discovery mechanisms.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_services() -> nos_api::Result<()> {
    // Shutdown service discovery
    discovery::shutdown_discovery()?;
    
    // Shutdown service registry
    registry::shutdown_registry()?;
    
    Ok(())
}

/// Get service statistics
///
/// # Returns
///
/// * `ServiceStats` - Service statistics
pub fn get_service_stats() -> ServiceStats {
    return registry::get_stats();
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_stats() {
        let stats = ServiceStats::default();
        assert_eq!(stats.total_services, 0);
        assert_eq!(stats.running_services, 0);
        assert_eq!(stats.stopped_services, 0);
        assert_eq!(stats.error_services, 0);
        assert_eq!(stats.avg_uptime, 0);
        assert_eq!(stats.total_restarts, 0);
    }
}