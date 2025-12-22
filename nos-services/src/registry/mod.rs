//! Service registry
//!
//! This module provides service registration and lookup functionality.

use crate::core::Service;
#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use spin::Mutex;
#[cfg(feature = "alloc")]
use nos_api::Result;

/// Service registry
#[cfg(feature = "alloc")]
pub struct ServiceRegistry {
    /// Registered services
    services: BTreeMap<u32, ServiceInfo>,
    /// Services by name
    services_by_name: BTreeMap<String, u32>,
    /// Next available service ID
    next_id: u32,
}

#[cfg(feature = "alloc")]
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
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidState(format!("Service {} already exists", name)));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidState("Service already exists".into()));
        }
        
        let id = self.next_id;
        self.next_id += 1;
        
        let info = ServiceInfo {
            id,
            name: name.to_string(),
            service,
            status: ServiceStatus::Registered,
            registration_time: crate::discovery::get_timestamp(),
        };
        
        self.services.insert(id, info);
        #[cfg(feature = "alloc")]
        self.services_by_name.insert(name.to_string(), id);
        #[cfg(not(feature = "alloc"))]
        self.services_by_name.insert(name.into(), id);
        
        Ok(id)
    }

    /// Unregister a service
    pub fn unregister(&mut self, id: u32) -> Result<()> {
        let info = self.services.remove(&id)
            .ok_or_else(|| {
                #[cfg(feature = "alloc")]
                return nos_api::Error::NotFound(format!("Service {} not found", id));
                #[cfg(not(feature = "alloc"))]
                nos_api::Error::NotFound("Service not found".into())
            })?;
        
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
    #[cfg(feature = "alloc")]
    pub fn list(&self) -> Vec<&ServiceInfo> {
        self.services.values().collect()
    }

    /// Start a service
    pub fn start(&mut self, id: u32) -> Result<()> {
        let info = self.services.get_mut(&id)
            .ok_or_else(|| {
                #[cfg(feature = "alloc")]
                return nos_api::Error::NotFound(format!("Service {} not found", id));
                #[cfg(not(feature = "alloc"))]
                nos_api::Error::NotFound("Service not found".into())
            })?;
        
        if info.status != ServiceStatus::Registered {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidState(format!("Service {} is not registered", id)));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidState("Service is not registered".into()));
        }
        
        // Start the service
        info.service.start()?;
        info.status = ServiceStatus::Running;
        
        Ok(())
    }

    /// Stop a service
    pub fn stop(&mut self, id: u32) -> Result<()> {
        let info = self.services.get_mut(&id)
            .ok_or_else(|| {
                #[cfg(feature = "alloc")]
                return nos_api::Error::NotFound(format!("Service {} not found", id));
                #[cfg(not(feature = "alloc"))]
                nos_api::Error::NotFound("Service not found".into())
            })?;
        
        if info.status != ServiceStatus::Running {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidState(format!("Service {} is not running", id)));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidState("Service is not running".into()));
        }
        
        // Stop the service
        info.service.stop()?;
        info.status = ServiceStatus::Stopped;
        
        Ok(())
    }
}

/// Service information
#[cfg(feature = "alloc")]
pub struct ServiceInfo {
    /// Service ID
    pub id: u32,
    /// Service name
    pub name: String,
    /// Service implementation
    pub service: Box<dyn Service>,
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
#[cfg(feature = "alloc")]
static GLOBAL_REGISTRY: spin::Mutex<core::mem::MaybeUninit<Mutex<ServiceRegistry>>> = spin::Mutex::new(core::mem::MaybeUninit::uninit());
static REGISTRY_INIT: spin::Once = spin::Once::new();

/// Initialize the global service registry
#[cfg(feature = "alloc")]
pub fn init_registry() -> Result<()> {
    REGISTRY_INIT.call_once(|| {
        let mut registry = GLOBAL_REGISTRY.lock();
        // SAFETY: We're writing to an uninitialized MaybeUninit, which is safe
        // and we're in a call_once so no concurrent access
        unsafe {
            registry.write(Mutex::new(ServiceRegistry::new()));
        }
    });
    Ok(())
}

/// Get the global service registry
#[cfg(feature = "alloc")]
pub fn get_registry() -> &'static Mutex<ServiceRegistry> {
    // SAFETY: We've already initialized the registry in init_registry
    unsafe {
        GLOBAL_REGISTRY.lock().assume_init_ref()
    }
}

/// Shutdown the global service registry
#[cfg(feature = "alloc")]
pub fn shutdown_registry() -> Result<()> {
    // In this implementation, we don't actually shut down the registry
    // since it's statically allocated and can't be deallocated
    Ok(())
}

/// Register a service
#[cfg(feature = "alloc")]
pub fn register_service(name: &str, service: Box<dyn Service>) -> Result<u32> {
    let mut registry = get_registry().lock();
    registry.register(name, service)
}

/// Get a service by name
#[cfg(feature = "alloc")]
pub fn get_service_by_name(name: &str) -> Option<&'static dyn Service> {
    // In a real implementation, this would return a reference to a global service
    // For now, we'll return None to indicate the service needs to be implemented
    None
}

/// Get a service by name (alias for get_service_by_name)
#[cfg(feature = "alloc")]
pub fn get_service(name: &str) -> Option<&'static dyn Service> {
    get_service_by_name(name)
}

/// Unregister a service by name
#[cfg(feature = "alloc")]
pub fn unregister_service(name: &str) -> Result<()> {
    let mut registry = get_registry().lock();
    if let Some(id) = registry.services_by_name.get(name).copied() {
        registry.unregister(id)
    } else {
        #[cfg(feature = "alloc")]
        return Err(nos_api::Error::NotFound(format!("Service {} not found", name)));
        #[cfg(not(feature = "alloc"))]
        Err(nos_api::Error::NotFound("Service not found".into()))
    }
}

/// Get service statistics
#[cfg(feature = "alloc")]
pub fn get_stats() -> crate::ServiceStats {
    let registry = get_registry().lock();
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
        #[cfg(feature = "alloc")]
        let id = registry.register("test_service", Box::new(service)).unwrap();
        
        // Get service
        #[cfg(feature = "alloc")]
        {
            let info = registry.get(id).unwrap();
            assert_eq!(info.name, "test_service");
            
            // Get by name
            let info = registry.get_by_name("test_service").unwrap();
            assert_eq!(info.id, id);
        }
    }
}