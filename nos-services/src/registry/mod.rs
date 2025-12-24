//! Service registry
//!
//! This module provides service registration and lookup functionality.

use crate::core::Service;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::format;
use alloc::vec::Vec;
use spin::Mutex;
use nos_api::Result;

/// Service registry
#[allow(clippy::should_implement_trait)]
#[allow(clippy::field_reassign_with_default)]
pub struct ServiceRegistry {
    /// Registered services
    services: BTreeMap<u32, ServiceInfo>,
    /// Services by name
    services_by_name: BTreeMap<String, u32>,
    /// Next available service ID
    next_id: u32,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
            services_by_name: BTreeMap::new(),
            next_id: 1000, // Start from 1000 to avoid conflicts
        }
    }

    /// Register a service
    pub fn register(&mut self, name: &str, service: Box<dyn Service>) -> Result<u32> {
        // Check if service already exists
        if self.services_by_name.contains_key(name) {
            return Err(nos_api::Error::InvalidState(format!("Service {} already exists", name)));
        }

        let id = self.next_id;
        self.next_id += 1;

        let info = ServiceInfo {
            id,
            name: name.to_string(),
            service: Arc::from(service),
            status: ServiceStatus::Registered,
            registration_time: crate::discovery::get_timestamp(),
        };

        self.services.insert(id, info);
        self.services_by_name.insert(name.to_string(), id);

        Ok(id)
    }

    /// Unregister a service
    pub fn unregister(&mut self, id: u32) -> Result<()> {
        let info = self.services.remove(&id)
            .ok_or_else(|| nos_api::Error::NotFound(format!("Service {} not found", id)))?;
        
        self.services_by_name.remove(&info.name);
        
        Ok(())
    }

    /// Get a service by ID
    pub fn get(&self, id: u32) -> Option<&ServiceInfo> {
        self.services.get(&id)
    }

    /// Get a service by name
    pub fn get_by_name(&self, name: &str) -> Option<&ServiceInfo> {
        self.services_by_name.get(name)
            .and_then(|id| self.services.get(id))
    }

    /// List all registered services
    pub fn list(&self) -> Vec<&ServiceInfo> {
        self.services.values().collect()
    }

    /// Start a service
    pub fn start(&mut self, id: u32) -> Result<()> {
        let info = self.services.get_mut(&id)
            .ok_or_else(|| nos_api::Error::NotFound(format!("Service {} not found", id)))?;
        
        if info.status != ServiceStatus::Registered {
            return Err(nos_api::Error::InvalidState(format!("Service {} is not registered", id)));
        }
        
        // Start the service
        info.service.start()?;
        info.status = ServiceStatus::Running;
        
        Ok(())
    }

    /// Stop a service
    pub fn stop(&mut self, id: u32) -> Result<()> {
        let info = self.services.get_mut(&id)
            .ok_or_else(|| nos_api::Error::NotFound(format!("Service {} not found", id)))?;
        
        if info.status != ServiceStatus::Running {
            return Err(nos_api::Error::InvalidState(format!("Service {} is not running", id)));
        }
        
        // Stop the service
        info.service.stop()?;
        info.status = ServiceStatus::Stopped;
        
        Ok(())
    }
}

/// Service information
pub struct ServiceInfo {
    /// Service ID
    pub id: u32,
    /// Service name
    pub name: String,
    /// Service implementation
    pub service: Arc<dyn Service>,
    /// Service status
    pub status: ServiceStatus,
    /// Registration time
    pub registration_time: u64,
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is registered but not started
    Registered,
    /// Service is running
    Running,
    /// Service is stopped
    Stopped,
    /// Service has an error
    Error,
}

/// Global service registry
static GLOBAL_REGISTRY: spin::Once<Mutex<ServiceRegistry>> = spin::Once::new();

/// Initialize the global service registry
pub fn init_registry() -> Result<()> {
    GLOBAL_REGISTRY.call_once(|| Mutex::new(ServiceRegistry::new()));
    Ok(())
}

/// Get global service registry
pub fn get_registry() -> Result<&'static Mutex<ServiceRegistry>> {
    GLOBAL_REGISTRY.get().ok_or_else(|| {
        nos_api::Error::InvalidState("Registry not initialized. Call init_registry() first.".to_string())
    })
}

/// Shutdown the global service registry
pub fn shutdown_registry() -> Result<()> {
    // In this implementation, we don't actually shut down the registry
    // since it's statically allocated and can't be deallocated
    Ok(())
}

/// Register a service
pub fn register_service(name: &str, service: Box<dyn Service>) -> Result<u32> {
    let registry = get_registry()?;
    let mut registry = registry.lock();
    registry.register(name, service)
}

/// Get a service by name
pub fn get_service_by_name(name: &str) -> Result<Arc<dyn Service>> {
    let registry = get_registry()?;
    let registry = registry.lock();
    let service_info = registry.get_by_name(name)
        .ok_or_else(|| nos_api::Error::NotFound(format!("Service {} not found", name)))?;

    Ok(Arc::clone(&service_info.service))
}

/// Get a service by name (alias for get_service_by_name)
pub fn get_service(name: &str) -> Result<Arc<dyn Service>> {
    get_service_by_name(name)
}

/// Unregister a service by name
pub fn unregister_service(name: &str) -> Result<()> {
    let registry = get_registry()?;
    let mut registry = registry.lock();
    if let Some(id) = registry.services_by_name.get(name).copied() {
        registry.unregister(id)
    } else {
        Err(nos_api::Error::NotFound(format!("Service {} not found", name)))
    }
}

/// Get service statistics
#[allow(clippy::field_reassign_with_default)]
pub fn get_stats() -> crate::ServiceStats {
    if let Ok(registry) = get_registry() {
        let registry = registry.lock();
        let services = registry.list();

        let mut stats = crate::ServiceStats::default();
        stats.total_services = services.len() as u64;

        for info in services {
            if info.status == ServiceStatus::Running {
                stats.running_services += 1;
            } else if info.status == ServiceStatus::Error {
                stats.error_services += 1;
            }
        }

        stats
    } else {
        crate::ServiceStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        name: &'static str,
    }

    impl Service for TestService {
        fn start(&self) -> Result<()> {
            Ok(())
        }
        
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        
        fn name(&self) -> &str {
            self.name
        }
        
        fn service_type(&self) -> u32 {
            1
        }
    }

    #[test]
    fn test_registry() {
        let mut registry = ServiceRegistry::new();
        
        // Register a test service
        let service = TestService {
            name: "test_service",
        };
        let id = registry.register("test_service", Box::new(service)).unwrap();
        
        // Get service
        let info = registry.get(id).unwrap();
        assert_eq!(info.name, "test_service");
        
        // Get by name
        let info = registry.get_by_name("test_service").unwrap();
        assert_eq!(info.id, id);
    }
}