//! Services Module
//! 
//! This module provides the service registration and management system for the NOS kernel.
//! It allows kernel components and user-space services to register themselves and
//! discover other services in the system.

pub mod registry;
pub mod discovery;
pub mod manager;
pub mod types;

use crate::error::KernelError;

/// Initialize the services subsystem
pub fn init() -> Result<(), KernelError> {
    // Initialize the service registry
    registry::init()?;
    
    // Initialize the service discovery
    discovery::init()?;
    
    // Initialize the service manager
    manager::init()?;
    
    log::info!("Services subsystem initialized");
    Ok(())
}

/// Get the service registry
pub fn get_registry() -> &'static registry::ServiceRegistry {
    registry::get_registry()
}

/// Get the service discovery
pub fn get_discovery() -> &'static discovery::ServiceDiscovery {
    discovery::get_discovery()
}

/// Get the service manager
pub fn get_manager() -> &'static manager::ServiceManager {
    manager::get_manager()
}