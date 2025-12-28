//! Service Registry Module
//! 
//! This module provides the service registry for registering and managing services
//! in the NOS kernel.

use crate::error::KernelError;
use crate::services::types::{
    ServiceId, ServiceRef, ServiceEvent, ServiceListener, ServiceState,
};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

/// Service registry
pub struct ServiceRegistry {
    services: BTreeMap<ServiceId, ServiceRef>,
    services_by_name: BTreeMap<alloc::string::String, ServiceId>,
    listeners: Vec<Arc<dyn ServiceListener>>,
    next_service_id: u64,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
            services_by_name: BTreeMap::new(),
            listeners: Vec::new(),
            next_service_id: 1,
        }
    }
    
    /// Register a service
    pub fn register_service(
        &mut self,
        service: ServiceRef,
    ) -> Result<(), KernelError> {
        let service_id = service.id();
        let service_name = service.name().to_string();
        
        // Check if service already exists
        if self.services.contains_key(&service_id) {
            return Err(KernelError::AlreadyExists("Service already registered".into()));
        }
        
        if self.services_by_name.contains_key(&service_name) {
            return Err(KernelError::AlreadyExists("Service with this name already registered".into()));
        }
        
        // Add service to registry
        self.services.insert(service_id, service.clone());
        self.services_by_name.insert(service_name, service_id);
        
        // Notify listeners
        let event = ServiceEvent::Registered(service);
        self.notify_listeners(event);
        
        log::info!("Service registered: {}", service_id.value());
        Ok(())
    }
    
    /// Unregister a service
    pub fn unregister_service(&mut self, service_id: ServiceId) -> Result<(), KernelError> {
        // Check if service exists
        let service = match self.services.get(&service_id) {
            Some(service) => service.clone(),
            None => return Err(KernelError::NotFound("Service not found".into())),
        };
        
        let service_name = service.name().to_string();
        
        // Remove service from registry
        self.services.remove(&service_id);
        self.services_by_name.remove(&service_name);
        
        // Notify listeners
        let event = ServiceEvent::Unregistered(service_id);
        self.notify_listeners(event);
        
        log::info!("Service unregistered: {}", service_id.value());
        Ok(())
    }
    
    /// Get a service by ID
    pub fn get_service(&self, service_id: ServiceId) -> Option<ServiceRef> {
        self.services.get(&service_id).cloned()
    }
    
    /// Get a service by name
    pub fn get_service_by_name(&self, name: &str) -> Option<ServiceRef> {
        self.services_by_name.get(name).and_then(|id| self.services.get(id).cloned())
    }
    
    /// List all services
    pub fn list_services(&self) -> Vec<ServiceRef> {
        self.services.values().cloned().collect()
    }
    
    /// List services by type
    pub fn list_services_by_type(&self, service_type: crate::services::types::ServiceType) -> Vec<ServiceRef> {
        self.services
            .values()
            .filter(|service| service.service_type() == service_type)
            .cloned()
            .collect()
    }
    
    /// Update service state
    pub fn update_service_state(
        &mut self,
        service_id: ServiceId,
        state: ServiceState,
    ) -> Result<(), KernelError> {
        // Check if service exists
        let mut service = match self.services.get_mut(&service_id) {
            Some(service) => service,
            None => return Err(KernelError::NotFound("Service not found".into())),
        };
        
        // Update state
        service.info.state = state;
        
        // Notify listeners
        let event = ServiceEvent::StateChanged(service_id, state);
        self.notify_listeners(event);
        
        log::info!("Service state updated: {} -> {:?}", service_id.value(), state);
        Ok(())
    }
    
    /// Update service property
    pub fn update_service_property(
        &mut self,
        service_id: ServiceId,
        key: alloc::string::String,
        value: alloc::string::String,
    ) -> Result<(), KernelError> {
        // Check if service exists
        let mut service = match self.services.get_mut(&service_id) {
            Some(service) => service,
            None => return Err(KernelError::NotFound("Service not found".into())),
        };
        
        // Update property
        service.info.properties.insert(key.clone(), value.clone());
        
        // Notify listeners
        let event = ServiceEvent::PropertyUpdated(service_id, key, value);
        self.notify_listeners(event);
        
        log::info!("Service property updated: {}", service_id.value());
        Ok(())
    }
    
    /// Add a service listener
    pub fn add_listener(&mut self, listener: Arc<dyn ServiceListener>) {
        self.listeners.push(listener);
    }
    
    /// Remove a service listener
    pub fn remove_listener(&mut self, listener: &Arc<dyn ServiceListener>) {
        self.listeners.retain(|l| !Arc::ptr_eq(l, listener));
    }
    
    /// Notify all listeners of an event
    fn notify_listeners(&self, event: ServiceEvent) {
        for listener in &self.listeners {
            listener.on_event(event.clone());
        }
    }
    
    /// Generate a new service ID
    pub fn generate_service_id(&mut self) -> ServiceId {
        let id = ServiceId::new(self.next_service_id);
        self.next_service_id += 1;
        id
    }
}

/// Global service registry instance
static SERVICE_REGISTRY: Mutex<Option<ServiceRegistry>> = Mutex::new(None);

/// Initialize the service registry
pub fn init() -> Result<(), KernelError> {
    let mut registry = SERVICE_REGISTRY.lock();
    *registry = Some(ServiceRegistry::new());
    log::info!("Service registry initialized");
    Ok(())
}

/// Get the global service registry
pub fn get_registry() -> &'static ServiceRegistry {
    // This is a simplified implementation
    // In a real implementation, we would use a more sophisticated approach
    // to avoid the lifetime issue
    unsafe {
        static mut REGISTRY_PTR: *const ServiceRegistry = core::ptr::null();
        
        if REGISTRY_PTR.is_null() {
            let registry = SERVICE_REGISTRY.lock();
            if let Some(ref reg) = *registry {
                REGISTRY_PTR = reg as *const ServiceRegistry;
            }
        }
        
        &*REGISTRY_PTR
    }
}