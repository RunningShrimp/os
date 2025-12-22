//! Service registry implementation

use crate::error::Result;
use crate::core::traits::Service;
use crate::service::interface::{ServiceRegistry, ServiceStatus, ServiceMetadata};
#[cfg(feature = "alloc")]
use hashbrown::HashMap;
// HashMap is not used in no-alloc mode
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(not(feature = "alloc"))]
use crate::interfaces::Box;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(not(feature = "alloc"))]
use crate::interfaces::Vec;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::string::ToString;




/// Default service registry implementation
#[cfg(feature = "alloc")]
pub struct DefaultServiceRegistry {
    /// Map of registered services
    services: HashMap<String, ServiceEntry>,
}

/// Service entry in the registry
#[cfg(feature = "alloc")]
struct ServiceEntry {
    /// Service instance
    service: Box<dyn Service>,
    /// Service metadata
    metadata: ServiceMetadata,
    /// Service status
    status: ServiceStatus,
}

#[cfg(not(feature = "alloc"))]
pub struct DefaultServiceRegistry {
    /// Map of registered services (using static slices)
    services: &'static [ServiceEntry],
}

/// Service entry in the registry
#[cfg(not(feature = "alloc"))]
pub struct ServiceEntry {
    /// Service instance
    pub service: Box<dyn Service>,
    /// Service metadata
    pub metadata: ServiceMetadata,
    /// Service status
    pub status: ServiceStatus,
    /// Service name (static)
    pub name: &'static str,
}

#[cfg(feature = "alloc")]
impl DefaultServiceRegistry {
    /// Creates a new service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }
}

#[cfg(feature = "alloc")]
impl ServiceRegistry for DefaultServiceRegistry {
    fn register(&mut self, service: Box<dyn Service>) -> Result<()> {
        let name = service.name().to_string();
        
        // Check if service is already registered
        if self.services.contains_key(&name) {
            return Err(crate::error::service_error("Service already registered"));
        }
        
        // Create service metadata
        let version = service.version().to_string();
        let metadata = ServiceMetadata::new(&name, &version);
        
        // Create service entry
        let entry = ServiceEntry {
            service,
            metadata,
            status: ServiceStatus::Uninitialized,
        };
        
        // Add to registry
        self.services.insert(name, entry);
        
        Ok(())
    }
    
    fn unregister(&mut self, name: &str) -> Result<()> {
        if !self.services.contains_key(name) {
            return Err(crate::error::service_error("Service not registered"));
        }
        
        self.services.remove(name);
        Ok(())
    }
    
    fn find(&self, name: &str) -> Option<&dyn Service> {
        // Look up the service by name
        if let Some(entry) = self.services.get(name) {
            Some(entry.service.as_ref())
        } else {
            None
        }
    }
    
    fn find_mut(&mut self, name: &str) -> Option<&mut dyn Service> {
        // Look up the service by name
        if let Some(entry) = self.services.get_mut(name) {
            Some(entry.service.as_mut())
        } else {
            None
        }
    }
    
    fn list(&self) -> Vec<&str> {
        self.services.keys().map(|name: &String| name.as_str()).collect()
    }
    
    fn count(&self) -> usize {
        self.services.len()
    }
    
    fn contains(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }
}

#[cfg(not(feature = "alloc"))]
impl DefaultServiceRegistry {
    /// Creates a new service registry (no-alloc mode - static only)
    pub fn new(_services: &'static [ServiceEntry]) -> Self {
        Self {
            services: _services,
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl ServiceRegistry for DefaultServiceRegistry {
    fn register(&mut self, _service: Box<dyn Service>) -> Result<()> {
        // In no-alloc mode, services must be registered at creation time
        Err(crate::error::service_error("Cannot register services in no-alloc mode"))
    }
    
    fn unregister(&mut self, _name: &str) -> Result<()> {
        // In no-alloc mode, services cannot be unregistered
        Err(crate::error::service_error("Cannot unregister services in no-alloc mode"))
    }
    
    fn find(&self, name: &str) -> Option<&dyn Service> {
        // Look up the service by name
        for entry in self.services {
            if entry.name == name {
                return Some(entry.service.as_ref());
            }
        }
        None
    }
    
    fn find_mut(&mut self, _name: &str) -> Option<&mut dyn Service> {
        // In no-alloc mode, services are immutable
        None
    }
    
    fn list(&self) -> Vec<&str> {
        // In no-alloc mode, return empty slice since we can't collect into static slice
        &[]
    }
    
    fn count(&self) -> usize {
        self.services.len()
    }
    
    fn contains(&self, name: &str) -> bool {
        self.services.iter().any(|entry| entry.name == name)
    }
}

/// Service registry builder
#[cfg(feature = "alloc")]
pub struct ServiceRegistryBuilder {
    registry: DefaultServiceRegistry,
}

#[cfg(feature = "alloc")]
impl Default for ServiceRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl ServiceRegistryBuilder {
    /// Creates a new service registry builder
    pub fn new() -> Self {
        Self {
            registry: DefaultServiceRegistry::new(),
        }
    }
    
    /// Registers a service
    pub fn with_service(mut self, service: Box<dyn Service>) -> Self {
        let _ = self.registry.register(service);
        self
    }
    
    /// Builds the service registry
    pub fn build(self) -> DefaultServiceRegistry {
        self.registry
    }
}

#[cfg(feature = "alloc")]
impl Default for DefaultServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for DefaultServiceRegistry {
    fn default() -> Self {
        Self::new(&[])
    }
}

#[cfg(not(feature = "alloc"))]
pub struct ServiceRegistryBuilder {
    // In no-alloc mode, we don't actually store services
    // since they need to be statically defined
}

#[cfg(not(feature = "alloc"))]
impl ServiceRegistryBuilder {
    /// Creates a new service registry builder (no-alloc mode)
    pub fn new() -> Self {
        Self {}
    }
    
    /// Builds the service registry (no-alloc mode)
    pub fn build(self) -> DefaultServiceRegistry {
        // In no-alloc mode, return an empty registry
        // Services should be added at compile time or via the new() method
        DefaultServiceRegistry::new(&[])
    }
}