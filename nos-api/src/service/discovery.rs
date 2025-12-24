use crate::core::traits::Service;
use crate::service::interface::ServiceMetadata;
use alloc::{
    string::{String, ToString},
    vec::Vec,
    sync::Arc,
};

use hashbrown::HashMap;

/// Service discovery trait
pub trait ServiceDiscovery {
    /// Discovers services by type
    fn discover_by_type(&self, service_type: &str) -> Vec<Arc<dyn Service>>;

    /// Discovers services by interface
    fn discover_by_interface(&self, interface: &str) -> Vec<Arc<dyn Service>>;

    /// Discovers services by capability
    fn discover_by_capability(&self, capability: &str) -> Vec<Arc<dyn Service>>;

    /// Lists all services
    fn list_all(&self) -> Vec<Arc<dyn Service>>;

    /// Checks if a service is discoverable
    fn is_discoverable(&self, name: &str) -> bool;
}

/// Default service discovery implementation
pub struct DefaultServiceDiscovery {
    /// Map of discoverable services
    services: HashMap<String, ServiceEntry>,
}

/// Service entry for discovery
struct ServiceEntry {
    /// Service instance
    service: Arc<dyn Service>,
    /// Service metadata
    metadata: ServiceMetadata,
}

impl DefaultServiceDiscovery {
    /// Creates a new service discovery
    pub fn new() -> Self {
        Self {
            services: HashMap::new()
        }
    }

    /// Adds a service to discovery (alloc mode)
    pub fn add_service(&mut self, service: Arc<dyn Service>) {
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
    pub fn get_service(&self, name: &str) -> Option<Arc<dyn Service>> {
        self.services.get(name).map(|entry| entry.service.clone())
    }

    /// Checks if a service exists (alloc mode)
    pub fn contains_service(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }

    /// Gets number of services (alloc mode)
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Checks if discovery is empty (alloc mode)
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Iterates over services (alloc mode)
    pub fn iter(&self) -> impl Iterator<Item = Arc<dyn Service>> + '_ {
        self.services.values().map(|entry| entry.service.clone())
    }
}

impl ServiceDiscovery for DefaultServiceDiscovery {
    fn discover_by_type(&self, service_type: &str) -> Vec<Arc<dyn Service>> {
        self.services
            .iter()
            .filter(|(name, _): &(&String, _)| name.starts_with(service_type))
            .map(|(_, entry)| entry.service.clone())
            .collect()
    }

    fn discover_by_interface(&self, interface: &str) -> Vec<Arc<dyn Service>> {
        self.services
            .iter()
            .filter(|(_, entry)| {
                entry.metadata.interfaces.iter().any(|i| i == interface)
            })
            .map(|(_, entry)| entry.service.clone())
            .collect()
    }

    fn discover_by_capability(&self, capability: &str) -> Vec<Arc<dyn Service>> {
        self.services
            .iter()
            .filter(|(_, entry)| {
                entry.metadata.capabilities.iter().any(|c| c == capability)
            })
            .map(|(_, entry)| entry.service.clone())
            .collect()
    }

    fn list_all(&self) -> Vec<Arc<dyn Service>> {
        self.services
            .values()
            .map(|entry| entry.service.clone())
            .collect()
    }

    fn is_discoverable(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }
}

/// Service discovery builder
pub struct ServiceDiscoveryBuilder {
    discovery: DefaultServiceDiscovery,
}

impl ServiceDiscoveryBuilder {
    /// Creates a new service discovery builder
    pub fn new() -> Self {
        Self {
            discovery: DefaultServiceDiscovery::new(),
        }
    }

    /// Adds a service to discovery
    pub fn with_service(mut self, service: Arc<dyn Service>) -> Self {
        self.discovery.add_service(service);
        self
    }

    /// Builds service discovery
    pub fn build(self) -> DefaultServiceDiscovery {
        self.discovery
    }
}

impl Default for DefaultServiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ServiceDiscoveryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
