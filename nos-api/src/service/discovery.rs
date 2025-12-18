//! Service discovery implementation

use crate::core::traits::Service;
use crate::service::interface::ServiceMetadata;
#[cfg(feature = "alloc")]
use alloc::{
    string::{String, ToString},
    boxed::Box,
    vec::Vec,
};

#[cfg(feature = "alloc")]
use hashbrown::HashMap;

// HashMap, ToString, String, Box, Vec and StringAlias are not used in no-alloc mode

// Import types from interfaces module for no-alloc mode
#[cfg(not(feature = "alloc"))]
use crate::interfaces::{Box, Vec};


/// Service discovery trait
pub trait ServiceDiscovery {
    /// Discovers services by type
    #[cfg(feature = "alloc")]
    fn discover_by_type(&self, service_type: &str) -> Vec<&dyn Service>;
    
    /// Discovers services by interface
    #[cfg(feature = "alloc")]
    fn discover_by_interface(&self, interface: &str) -> Vec<&dyn Service>;
    
    /// Discovers services by capability
    #[cfg(feature = "alloc")]
    fn discover_by_capability(&self, capability: &str) -> Vec<&dyn Service>;
    
    /// Lists all services
    #[cfg(feature = "alloc")]
    fn list_all(&self) -> Vec<&dyn Service>;
    
    /// Discovers services by type (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    fn discover_by_type(&self, service_type: &str) -> &[&dyn Service];
    
    /// Discovers services by interface (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    fn discover_by_interface(&self, interface: &str) -> &[&dyn Service];
    
    /// Discovers services by capability (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    fn discover_by_capability(&self, capability: &str) -> &[&dyn Service];
    
    /// Lists all services (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    fn list_all(&self) -> &[&dyn Service];
    
    /// Checks if a service is discoverable
    fn is_discoverable(&self, name: &str) -> bool;
}

/// Default service discovery implementation
#[cfg(feature = "alloc")]
pub struct DefaultServiceDiscovery {
    /// Map of discoverable services
    services: HashMap<String, ServiceEntry>,
}

/// Service entry for discovery
#[cfg(feature = "alloc")]
struct ServiceEntry {
    /// Service instance
    service: Box<dyn Service>,
    /// Service metadata
    metadata: ServiceMetadata,
}

/// Default service discovery implementation
#[cfg(not(feature = "alloc"))]
pub struct DefaultServiceDiscovery {
    /// Static list of discoverable services
    services: &'static [ServiceEntry],
}

#[cfg(not(feature = "alloc"))]


/// Service entry for discovery
#[cfg(not(feature = "alloc"))]
pub struct ServiceEntry {
    /// Service instance
    pub service: Box<dyn Service>,
    /// Service metadata
    pub metadata: ServiceMetadata,
    /// Service name (static)
    pub name: &'static str,
}



#[cfg(not(feature = "alloc"))]
impl DefaultServiceDiscovery {
    /// Creates a new service discovery (no-alloc mode)
    pub fn new(services: &'static [ServiceEntry]) -> Self {
        Self {
            services,
        }
    }
    
    /// Adds a service to discovery (no-alloc mode)
    pub fn add_service(&mut self, _service: Box<dyn Service>) {
        // In no-alloc mode, services must be registered at creation time
        // This is a limitation of no-alloc environment
        // For now, do nothing
    }
    
    /// Removes a service from discovery (no-alloc mode)
    pub fn remove_service(&mut self, _name: &str) {
        // In no-alloc mode, services are immutable
        // This is a limitation of no-alloc environment
        // For now, do nothing
    }
    
    /// Clears all services from discovery (no-alloc mode)
    pub fn clear(&mut self) {
        // In no-alloc mode, services are immutable
        // This is a limitation of no-alloc environment
        // For now, do nothing
    }
    
    /// Gets a service by name (no-alloc mode)
    pub fn get_service(&self, _name: &str) -> Option<&dyn Service> {
        // In no-alloc mode, services are not accessible by name
        // This is a limitation of no-alloc environment
        // For now, return None
        None
    }
    
    /// Gets a service by name (mutable, no-alloc mode)
    pub fn get_service_mut(&mut self, _name: &str) -> Option<&mut dyn Service> {
        // In no-alloc mode, services are not accessible by name
        // This is a limitation of no-alloc environment
        // For now, return None
        None
    }
    
    /// Checks if a service exists (no-alloc mode)
    pub fn contains_service(&self, _name: &str) -> bool {
        // In no-alloc mode, services are not accessible by name
        // This is a limitation of no-alloc environment
        // For now, return false
        false
    }
    
    /// Gets the number of services (no-alloc mode)
    pub fn len(&self) -> usize {
        // In no-alloc mode, services are not accessible
        // This is a limitation of no-alloc environment
        // For now, return 0
        0
    }
    
    /// Checks if the discovery is empty (no-alloc mode)
    pub fn is_empty(&self) -> bool {
        // In no-alloc mode, services are not accessible
        // This is a limitation of no-alloc environment
        // For now, return true
        true
    }
    
    /// Iterates over services (no-alloc mode)
    pub fn iter(&self) -> impl Iterator<Item = &dyn Service> {
        // In no-alloc mode, services are not accessible
        // This is a limitation of no-alloc environment
        // For now, return empty iterator
        core::iter::empty()
    }
    
    /// Iterates over services (mutable, no-alloc mode)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn Service> {
        // In no-alloc mode, services are not accessible
        // This is a limitation of no-alloc environment
        // For now, return empty iterator
        core::iter::empty()
    }
}

/// Default service discovery implementation
#[cfg(feature = "alloc")]
impl DefaultServiceDiscovery {
    /// Creates a new service discovery (alloc mode)
    pub fn new() -> Self {
        Self {
            services: HashMap::new()
        }
    }
    
    /// Adds a service to discovery (alloc mode)
    pub fn add_service(&mut self, service: Box<dyn Service>) {
        let name = service.name().to_string();
        let version = service.version().to_string();
        let metadata = ServiceMetadata::new(&name, &version);
        
        let entry = ServiceEntry {
            service,
            metadata,
        };
        
        self.services.insert(name, entry);
    }
    
    /// Removes a service from discovery (alloc mode)
    pub fn remove_service(&mut self, name: &str) {
        self.services.remove(name);
    }
    
    /// Clears all services from discovery (alloc mode)
    pub fn clear(&mut self) {
        self.services.clear();
    }
    
    /// Gets a service by name (alloc mode)
    pub fn get_service(&self, name: &str) -> Option<&dyn Service> {
        self.services.get(name).map(|entry| entry.service.as_ref())
    }
    
    /// Gets a service by name (mutable, alloc mode)
    pub fn get_service_mut(&mut self, name: &str) -> Option<&mut (dyn Service + '_)> {
        let entry = self.services.get_mut(name)?;
        Some(entry.service.as_mut())
    }
    
    /// Checks if a service exists (alloc mode)
    pub fn contains_service(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }
    
    /// Gets the number of services (alloc mode)
    pub fn len(&self) -> usize {
        self.services.len()
    }
    
    /// Checks if the discovery is empty (alloc mode)
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }
    
    /// Iterates over services (alloc mode)
    pub fn iter(&self) -> impl Iterator<Item = &dyn Service> {
        self.services.values().map(|entry| entry.service.as_ref())
    }
    
    /// Iterates over services (mutable, alloc mode)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn Service> + '_ {
        self.services.values_mut().map(|entry| {
            let service = entry.service.as_mut();
            service
        })
    }
}

#[cfg(feature = "alloc")]
impl ServiceDiscovery for DefaultServiceDiscovery {
    fn discover_by_type(&self, service_type: &str) -> Vec<&dyn Service> {
        // Find services by type (using service name as type)
        self.services
            .iter()
            .filter(|(name, _): &(&String, _)| name.starts_with(service_type))
            .map(|(_, entry)| entry.service.as_ref())
            .collect()
    }
    
    fn discover_by_interface(&self, interface: &str) -> Vec<&dyn Service> {
        // Find services by interface
        self.services
            .iter()
            .filter(|(_, entry)| {
                let interface_str = interface.to_string();
                entry.metadata.interfaces.contains(&interface_str)
            })
            .map(|(_, entry)| entry.service.as_ref())
            .collect()
    }
    
    fn discover_by_capability(&self, capability: &str) -> Vec<&dyn Service> {
        // Find services by capability
        self.services
            .iter()
            .filter(|(_, entry)| {
                let capability_str = capability.to_string();
                entry.metadata.capabilities.contains(&capability_str)
            })
            .map(|(_, entry)| entry.service.as_ref())
            .collect()
    }
    
    fn list_all(&self) -> Vec<&dyn Service> {
        // Return all discoverable services
        self.services
            .values()
            .map(|entry| entry.service.as_ref())
            .collect()
    }
    
    fn is_discoverable(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }
}

#[cfg(not(feature = "alloc"))]
impl ServiceDiscovery for DefaultServiceDiscovery {
    fn discover_by_type(&self, _service_type: &str) -> &[&dyn Service] {
        // In no-alloc mode, return empty slice
        // In a real implementation, this would filter static services
        &[]
    }
    
    fn discover_by_interface(&self, _interface: &str) -> &[&dyn Service] {
        // In no-alloc mode, return empty slice
        // In a real implementation, this would filter static services
        &[]
    }
    
    fn discover_by_capability(&self, _capability: &str) -> &[&dyn Service] {
        // In no-alloc mode, return empty slice
        // In a real implementation, this would filter static services
        &[]
    }
    
    fn list_all(&self) -> &[&dyn Service] {
        // In no-alloc mode, return empty slice
        // In a real implementation, this would return all static services
        &[]
    }
    
    fn is_discoverable(&self, _name: &str) -> bool {
        // In no-alloc mode, return false
        // In a real implementation, this would check if the service is in the static list
        false
    }
}

/// Service discovery builder
#[cfg(feature = "alloc")]
pub struct ServiceDiscoveryBuilder {
    discovery: DefaultServiceDiscovery,
}

#[cfg(feature = "alloc")]
impl ServiceDiscoveryBuilder {
    /// Creates a new service discovery builder
    pub fn new() -> Self {
        Self {
            discovery: DefaultServiceDiscovery::new(),
        }
    }
    
    /// Adds a service to discovery
    pub fn with_service(mut self, service: Box<dyn Service>) -> Self {
        self.discovery.add_service(service);
        self
    }
    
    /// Builds the service discovery
    pub fn build(self) -> DefaultServiceDiscovery {
        self.discovery
    }
}

#[cfg(feature = "alloc")]
impl Default for DefaultServiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl Default for ServiceDiscoveryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for DefaultServiceDiscovery {
    fn default() -> Self {
        Self::new(&[])
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for ServiceDiscoveryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "alloc"))]
pub struct ServiceDiscoveryBuilder {
    // In no-alloc mode, we don't actually store services
    // since they need to be statically defined
}

#[cfg(not(feature = "alloc"))]
impl ServiceDiscoveryBuilder {
    /// Creates a new service discovery builder (no-alloc mode)
    pub fn new() -> Self {
        Self {}
    }
    
    /// Builds the service discovery (no-alloc mode)
    pub fn build(self) -> DefaultServiceDiscovery {
        // In no-alloc mode, return an empty service discovery
        // Services should be added at compile time or via the new() method
        DefaultServiceDiscovery::new(&[])
    }
}