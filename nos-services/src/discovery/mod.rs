//! Service discovery
//! 
//! This module provides service discovery and enumeration functionality.

extern crate alloc;

use nos_api::Result;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::{Mutex, Once};

/// Service discovery
#[allow(clippy::should_implement_trait)]
#[allow(clippy::field_reassign_with_default)]
pub struct ServiceDiscovery {
    /// Discovered services
    services: BTreeMap<String, ServiceDescriptor>,
}

impl ServiceDiscovery {
    /// Create a new service discovery
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
        }
    }

    /// Discover services
    pub fn discover(&mut self) -> Result<Vec<ServiceDescriptor>> {
        // TODO: Implement actual service discovery
        Ok(self.services.values().cloned().collect())
    }

    /// Add a service descriptor
    pub fn add_service(&mut self, descriptor: ServiceDescriptor) {
        self.services.insert(descriptor.name.clone(), descriptor);
    }

    /// Remove a service descriptor
    pub fn remove_service(&mut self, name: &str) {
        self.services.remove(name);
    }

    /// Get a service descriptor by name
    pub fn get_service(&self, name: &str) -> Option<&ServiceDescriptor> {
        self.services.get(name)
    }

    /// List all discovered services
    pub fn list_services(&self) -> Vec<&ServiceDescriptor> {
        self.services.values().collect()
    }
}

/// Service descriptor
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ServiceDescriptor {
    /// Service name
    pub name: String,
    /// Service type
    pub service_type: u32,
    /// Service version
    pub version: String,
    /// Service description
    pub description: String,
    /// Service endpoint
    pub endpoint: String,
    /// Service metadata
    pub metadata: BTreeMap<String, String>,
}


/// Global service discovery
static GLOBAL_DISCOVERY: Once<Mutex<ServiceDiscovery>> = Once::new();

/// Initialize the global service discovery
pub fn init_discovery() -> Result<()> {
    GLOBAL_DISCOVERY.call_once(|| Mutex::new(ServiceDiscovery::new()));
    Ok(())
}

/// Get the global service discovery
pub fn get_discovery() -> &'static Mutex<ServiceDiscovery> {
    GLOBAL_DISCOVERY.get().expect("Discovery not initialized")
}

/// Shutdown the global service discovery
pub fn shutdown_discovery() -> Result<()> {
    // Note: With spin::Once, we cannot reset the global instance
    // This is a limitation of the safe Once pattern
    Ok(())
}

/// Get the current timestamp
pub fn get_timestamp() -> u64 {
    // In a real implementation, this would get the current time
    // For now, return a dummy value
    42
}

/// Discover services
pub fn discover_services() -> Result<alloc::vec::Vec<ServiceDescriptor>> {
    let mut discovery = get_discovery().lock();
    discovery.discover()
}

/// Get a service descriptor by name
pub fn get_service_descriptor(name: &str) -> Option<ServiceDescriptor> {
    let discovery = get_discovery().lock();
    discovery.get_service(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery() {
        let mut discovery = ServiceDiscovery::new();
        
        // Add a test service
        let descriptor = ServiceDescriptor {
            name: "test_service".to_string(),
            service_type: 1,
            version: "1.0.0".to_string(),
            description: "Test service".to_string(),
            endpoint: "local://test_service".to_string(),
            metadata: BTreeMap::new(),
        };
        discovery.add_service(descriptor.clone());
        
        // Get service
        let retrieved = discovery.get_service("test_service").unwrap();
        assert_eq!(retrieved.name, descriptor.name);
        
        // List services
        let services = discovery.list_services();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, descriptor.name);
    }
}