//! Service discovery
//!
//! This module provides service discovery and enumeration functionality.

use nos_api::Result;
#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use spin::{Mutex, Once};

/// Service discovery
pub struct ServiceDiscovery {
    /// Discovered services
    #[cfg(feature = "alloc")]
    services: BTreeMap<String, ServiceDescriptor>,
    #[cfg(not(feature = "alloc"))]
    services: core::marker::PhantomData<&'static str>,
}

impl ServiceDiscovery {
    /// Create a new service discovery
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "alloc")]
            services: BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            services: core::marker::PhantomData,
        }
    }

    /// Discover services
    #[cfg(feature = "alloc")]
    pub fn discover(&mut self) -> Result<Vec<ServiceDescriptor>> {
        // TODO: Implement actual service discovery
        Ok(self.services.values().cloned().collect())
    }

    /// Discover services (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn discover(&mut self) -> Result<&'static [ServiceDescriptor]> {
        // TODO: Implement actual service discovery for no-alloc
        Ok(&[])
    }

    /// Add a service descriptor
    #[cfg(feature = "alloc")]
    pub fn add_service(&mut self, descriptor: ServiceDescriptor) {
        self.services.insert(descriptor.name.clone(), descriptor);
    }

    /// Add a service descriptor (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn add_service(&mut self, _descriptor: ServiceDescriptor) {
        // TODO: Implement for no-alloc
    }

    /// Remove a service descriptor
    #[cfg(feature = "alloc")]
    pub fn remove_service(&mut self, name: &str) {
        self.services.remove(name);
    }

    /// Remove a service descriptor (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn remove_service(&mut self, _name: &str) {
        // TODO: Implement for no-alloc
    }

    /// Get a service descriptor by name
    #[cfg(feature = "alloc")]
    pub fn get_service(&self, name: &str) -> Option<&ServiceDescriptor> {
        self.services.get(name)
    }

    /// Get a service descriptor by name (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn get_service(&self, _name: &str) -> Option<&ServiceDescriptor> {
        // TODO: Implement for no-alloc
        None
    }

    /// List all discovered services
    #[cfg(feature = "alloc")]
    pub fn list_services(&self) -> Vec<&ServiceDescriptor> {
        self.services.values().collect()
    }

    /// List all discovered services (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn list_services(&self) -> &'static [&ServiceDescriptor] {
        // TODO: Implement for no-alloc
        &[]
    }
}

/// Service descriptor
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    /// Service name
    #[cfg(feature = "alloc")]
    pub name: String,
    #[cfg(not(feature = "alloc"))]
    pub name: &'static str,
    /// Service type
    pub service_type: u32,
    /// Service version
    #[cfg(feature = "alloc")]
    pub version: String,
    #[cfg(not(feature = "alloc"))]
    pub version: &'static str,
    /// Service description
    #[cfg(feature = "alloc")]
    pub description: String,
    #[cfg(not(feature = "alloc"))]
    pub description: &'static str,
    /// Service endpoint
    #[cfg(feature = "alloc")]
    pub endpoint: String,
    #[cfg(not(feature = "alloc"))]
    pub endpoint: &'static str,
    /// Service metadata
    #[cfg(feature = "alloc")]
    pub metadata: BTreeMap<String, String>,
    #[cfg(not(feature = "alloc"))]
    pub metadata: core::marker::PhantomData<(&'static str, &'static str)>,
}

impl Default for ServiceDescriptor {
    fn default() -> Self {
        Self {
            #[cfg(feature = "alloc")]
            name: String::new(),
            #[cfg(not(feature = "alloc"))]
            name: "",
            service_type: 0,
            #[cfg(feature = "alloc")]
            version: String::new(),
            #[cfg(not(feature = "alloc"))]
            version: "",
            #[cfg(feature = "alloc")]
            description: String::new(),
            #[cfg(not(feature = "alloc"))]
            description: "",
            #[cfg(feature = "alloc")]
            endpoint: String::new(),
            #[cfg(not(feature = "alloc"))]
            endpoint: "",
            #[cfg(feature = "alloc")]
            metadata: BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            metadata: core::marker::PhantomData,
        }
    }
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
#[cfg(feature = "alloc")]
pub fn discover_services() -> Result<alloc::vec::Vec<ServiceDescriptor>> {
    let mut discovery = get_discovery().lock();
    discovery.discover()
}

/// Discover services (no-alloc version)
#[cfg(not(feature = "alloc"))]
pub fn discover_services() -> Result<&'static [ServiceDescriptor]> {
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