//! Service Discovery Module
//! 
//! This module provides service discovery functionality for finding and
//! connecting to services in the NOS kernel.

use crate::error::KernelError;
use crate::services::types::{
    ServiceId, ServiceRef, ServiceType, ServicePriority,
};
use crate::services::registry::get_registry;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

/// Service query
#[derive(Debug, Clone)]
pub struct ServiceQuery {
    pub name: Option<String>,
    pub service_type: Option<ServiceType>,
    pub priority: Option<ServicePriority>,
    pub properties: BTreeMap<String, String>,
    pub limit: Option<usize>,
}

impl ServiceQuery {
    /// Create a new service query
    pub fn new() -> Self {
        Self {
            name: None,
            service_type: None,
            priority: None,
            properties: BTreeMap::new(),
            limit: None,
        }
    }
    
    /// Set the service name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    /// Set the service type
    pub fn with_type(mut self, service_type: ServiceType) -> Self {
        self.service_type = Some(service_type);
        self
    }
    
    /// Set the service priority
    pub fn with_priority(mut self, priority: ServicePriority) -> Self {
        self.priority = Some(priority);
        self
    }
    
    /// Add a property filter
    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }
    
    /// Set the result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Service discovery
pub struct ServiceDiscovery {
    query_cache: BTreeMap<String, Vec<ServiceRef>>,
    cache_timeout: u64,
}

impl ServiceDiscovery {
    /// Create a new service discovery
    pub fn new() -> Self {
        Self {
            query_cache: BTreeMap::new(),
            cache_timeout: 5000, // 5 seconds
        }
    }
    
    /// Find services matching a query
    pub fn find_services(&mut self, query: ServiceQuery) -> Result<Vec<ServiceRef>, KernelError> {
        // Check cache first
        let cache_key = self.query_to_cache_key(&query);
        if let Some(cached_services) = self.query_cache.get(&cache_key) {
            return Ok(cached_services.clone());
        }
        
        // Get all services from registry
        let registry = get_registry();
        let all_services = registry.list_services();
        
        // Filter services based on query
        let mut matching_services = Vec::new();
        
        for service in all_services {
            if self.matches_query(&service, &query) {
                matching_services.push(service);
            }
        }
        
        // Sort by priority
        matching_services.sort_by(|a, b| a.priority().cmp(&b.priority()));
        
        // Apply limit if specified
        if let Some(limit) = query.limit {
            matching_services.truncate(limit);
        }
        
        // Cache the result
        self.query_cache.insert(cache_key, matching_services.clone());
        
        Ok(matching_services)
    }
    
    /// Find a single service matching a query
    pub fn find_service(&mut self, query: ServiceQuery) -> Result<Option<ServiceRef>, KernelError> {
        let mut limited_query = query;
        limited_query.limit = Some(1);
        
        let services = self.find_services(limited_query)?;
        Ok(services.into_iter().next())
    }
    
    /// Find a service by name
    pub fn find_service_by_name(&mut self, name: &str) -> Result<Option<ServiceRef>, KernelError> {
        let query = ServiceQuery::new().with_name(name.to_string());
        self.find_service(query)
    }
    
    /// Find services by type
    pub fn find_services_by_type(&mut self, service_type: ServiceType) -> Result<Vec<ServiceRef>, KernelError> {
        let query = ServiceQuery::new().with_type(service_type);
        self.find_services(query)
    }
    
    /// Find services by priority
    pub fn find_services_by_priority(&mut self, priority: ServicePriority) -> Result<Vec<ServiceRef>, KernelError> {
        let query = ServiceQuery::new().with_priority(priority);
        self.find_services(query)
    }
    
    /// Find services by property
    pub fn find_services_by_property(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<Vec<ServiceRef>, KernelError> {
        let query = ServiceQuery::new().with_property(key.to_string(), value.to_string());
        self.find_services(query)
    }
    
    /// Clear the query cache
    pub fn clear_cache(&mut self) {
        self.query_cache.clear();
    }
    
    /// Check if a service matches a query
    fn matches_query(&self, service: &ServiceRef, query: &ServiceQuery) -> bool {
        // Check name
        if let Some(ref name) = query.name {
            if service.name() != name {
                return false;
            }
        }
        
        // Check service type
        if let Some(service_type) = query.service_type {
            if service.service_type() != service_type {
                return false;
            }
        }
        
        // Check priority
        if let Some(priority) = query.priority {
            if service.priority() != priority {
                return false;
            }
        }
        
        // Check properties
        for (key, value) in &query.properties {
            if let Some(service_value) = service.properties().get(key) {
                if service_value != value {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        true
    }
    
    /// Convert a query to a cache key
    fn query_to_cache_key(&self, query: &ServiceQuery) -> String {
        let mut key = String::new();
        
        if let Some(ref name) = query.name {
            key.push_str("name:");
            key.push_str(name);
            key.push(';');
        }
        
        if let Some(service_type) = query.service_type {
            key.push_str("type:");
            key.push_str(&format!("{:?}", service_type));
            key.push(';');
        }
        
        if let Some(priority) = query.priority {
            key.push_str("priority:");
            key.push_str(&format!("{:?}", priority));
            key.push(';');
        }
        
        for (key, value) in &query.properties {
            key.push_str("prop:");
            key.push_str(key);
            key.push(':');
            key.push_str(value);
            key.push(';');
        }
        
        key
    }
}

/// Global service discovery instance
static SERVICE_DISCOVERY: Mutex<Option<ServiceDiscovery>> = Mutex::new(None);

/// Initialize the service discovery
pub fn init() -> Result<(), KernelError> {
    let mut discovery = SERVICE_DISCOVERY.lock();
    *discovery = Some(ServiceDiscovery::new());
    log::info!("Service discovery initialized");
    Ok(())
}

/// Get the global service discovery
pub fn get_discovery() -> &'static ServiceDiscovery {
    // This is a simplified implementation
    // In a real implementation, we would use a more sophisticated approach
    // to avoid the lifetime issue
    unsafe {
        static mut DISCOVERY_PTR: *const ServiceDiscovery = core::ptr::null();
        
        if DISCOVERY_PTR.is_null() {
            let discovery = SERVICE_DISCOVERY.lock();
            if let Some(ref disc) = *discovery {
                DISCOVERY_PTR = disc as *const ServiceDiscovery;
            }
        }
        
        &*DISCOVERY_PTR
    }
}