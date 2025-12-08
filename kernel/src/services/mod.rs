// Service layer for hybrid architecture
// Implements service registration, discovery, and communication

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::{format, vec};
use crate::sync::Mutex;

// Service modules
pub mod memory;
pub mod process;
pub mod fs;
pub mod syscall;
pub mod network;
pub mod driver;
pub mod ipc;

// ============================================================================
// Constants
// ============================================================================

pub const SERVICE_NAME_MAX: usize = 64;
pub const SERVICE_DESC_MAX: usize = 256;

// ============================================================================
// Types
// ============================================================================

/// Service Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Inactive,
    Active,
    Failed,
    Suspended,
}

/// Service Information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub status: ServiceStatus,
    pub service_id: u32,
    pub endpoint: usize,    // IPC endpoint for the service
    pub dependencies: Vec<String>,
    pub last_heartbeat: u64, // Last heartbeat timestamp for health check
}

/// Service Registry
pub struct ServiceRegistry {
    services: Mutex<Vec<ServiceInfo>>,
    next_service_id: Mutex<u32>,
    last_heartbeat_time: Mutex<u64>,
}

// ============================================================================
// Implementation
// ============================================================================

impl ServiceInfo {
    /// Create a new service information entry
    pub fn new(name: &str, description: &str, endpoint: usize) -> Self {
        Self {
            name: String::from(name),
            description: String::from(description),
            status: ServiceStatus::Active,
            service_id: 0,
            endpoint,
            dependencies: Vec::new(),
            last_heartbeat: 0,
        }
    }
}

impl ServiceRegistry {
    /// Create a new service registry
    pub const fn new() -> Self {
        Self {
            services: Mutex::new(Vec::new()),
            next_service_id: Mutex::new(1),
            last_heartbeat_time: Mutex::new(0),
        }
    }

    /// Register a new service
    pub fn register(&self, mut service: ServiceInfo) -> u32 {
        let mut services = self.services.lock();
        let new_id = *self.next_service_id.lock() + 1;
        *self.next_service_id.lock() = new_id;
        
        service.service_id = new_id;
        service.last_heartbeat = *self.last_heartbeat_time.lock();
        
        services.push(service);
        
        new_id
    }

    /// Unregister a service
    pub fn unregister(&self, service_id: u32) -> bool {
        let mut services = self.services.lock();
        let index = services.iter().position(|s| s.service_id == service_id);
        if let Some(idx) = index {
            services.remove(idx);
            true
        } else {
            false
        }
    }

    /// Find a service by name
    pub fn find_by_name(&self, name: &str) -> Option<ServiceInfo> {
        let services = self.services.lock();
        services.iter().find(|s| s.name == name).cloned()
    }

    /// Find a service by ID
    pub fn find_by_id(&self, service_id: u32) -> Option<ServiceInfo> {
        let services = self.services.lock();
        services.iter().find(|s| s.service_id == service_id).cloned()
    }

    /// Get all services
    pub fn all_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.lock();
        services.iter().cloned().collect()
    }

    /// Update service heartbeat
    pub fn update_heartbeat(&self, service_id: u32) -> bool {
        let mut services = self.services.lock();
        if let Some(service) = services.iter_mut().find(|s| s.service_id == service_id) {
            service.last_heartbeat = *self.last_heartbeat_time.lock();
            service.status = ServiceStatus::Active;
            true
        } else {
            false
        }
    }

    /// Update heartbeat time
    pub fn update_heartbeat_time(&self, time: u64) {
        *self.last_heartbeat_time.lock() = time;
    }

    /// Check service health
    pub fn check_health(&self, timeout: u64) -> Vec<u32> {
        let services = self.services.lock();
        let current_time = *self.last_heartbeat_time.lock();
        services
            .iter()
            .filter(|s| s.status == ServiceStatus::Active && 
                   (current_time - s.last_heartbeat) > timeout)
            .map(|s| s.service_id)
            .collect()
    }
}

// ============================================================================
// Global State
// ============================================================================

pub static SERVICE_REGISTRY: ServiceRegistry = ServiceRegistry::new();

// ============================================================================
// Public API
// ============================================================================

/// Initialize service layer
pub fn init() -> Result<(), &'static str> {
    // Initialize individual services
    memory::init().map_err(|_| "Failed to initialize memory service")?;
    process::init().map_err(|_| "Failed to initialize process service")?;
    fs::init().map_err(|_| "Failed to initialize filesystem service")?;
    syscall::init()?;
    network::init()?;
    driver::init()?;
    ipc::init()?;

    crate::println!("[services] Service layer initialized");
    Ok(())
}

/// Register a new service
pub fn service_register(name: &str, description: &str, endpoint: usize) -> u32 {
    let service = ServiceInfo::new(name, description, endpoint);
    SERVICE_REGISTRY.register(service)
}

/// Unregister a service
pub fn service_unregister(service_id: u32) -> bool {
    SERVICE_REGISTRY.unregister(service_id)
}

/// Find service by name
pub fn service_find_by_name(name: &str) -> Option<ServiceInfo> {
    SERVICE_REGISTRY.find_by_name(name)
}

/// Find service by ID
pub fn service_find_by_id(service_id: u32) -> Option<ServiceInfo> {
    SERVICE_REGISTRY.find_by_id(service_id)
}

/// Update service heartbeat
pub fn service_heartbeat(service_id: u32) -> bool {
    SERVICE_REGISTRY.update_heartbeat(service_id)
}

/// Update global heartbeat time
pub fn service_update_heartbeat_time(time: u64) {
    SERVICE_REGISTRY.update_heartbeat_time(time);
}

/// Check service health
pub fn service_check_health(timeout: u64) -> Vec<u32> {
    SERVICE_REGISTRY.check_health(timeout)
}