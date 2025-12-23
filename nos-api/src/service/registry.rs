//! Service registry implementation

use crate::error::Result;
use crate::core::traits::Service;
use crate::service::interface::{ServiceRegistry, ServiceStatus, ServiceMetadata};
use hashbrown::HashMap;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;




/// Default service registry implementation
pub struct DefaultServiceRegistry {
    /// Map of registered services
    services: HashMap<String, ServiceEntry>,
}

/// Service entry in the registry
struct ServiceEntry {
    /// Service instance
    service: Box<dyn Service>,
    /// Service metadata
    metadata: ServiceMetadata,
    /// Service status
    status: ServiceStatus,
}

impl DefaultServiceRegistry {
    /// Creates a new service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }
}

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

/// Service registry builder
pub struct ServiceRegistryBuilder {
    registry: DefaultServiceRegistry,
}

impl Default for ServiceRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

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

impl Default for DefaultServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultServiceRegistry {
    /// Builds the service registry
    pub fn build(self) -> DefaultServiceRegistry {
        DefaultServiceRegistry::new()
    }
}