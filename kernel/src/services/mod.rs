// Service layer for hybrid architecture
// Implements service registration, discovery, and communication

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::{format, vec};
use crate::sync::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::services::traits::ServiceCapabilities;

// Service modules
pub mod memory;
pub mod process;
pub mod fs;
pub mod syscall;
pub mod network;
pub mod driver;
pub mod ipc;
pub mod traits;

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
    pub version: Option<(u16,u16)>,
    pub capabilities: Option<ServiceCapabilities>,
}

/// Service Registry
pub struct ServiceRegistry {
    services: Mutex<Vec<ServiceInfo>>,
    next_service_id: Mutex<u32>,
    last_heartbeat_time: Mutex<u64>,
    registrations: AtomicU64,
    lookups: AtomicU64,
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
            version: None,
            capabilities: None,
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
            registrations: AtomicU64::new(0),
            lookups: AtomicU64::new(0),
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
        self.registrations.fetch_add(1, Ordering::SeqCst);
        
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
        self.lookups.fetch_add(1, Ordering::SeqCst);
        services.iter().find(|s| s.name == name).cloned()
    }

    /// Find a service by ID
    pub fn find_by_id(&self, service_id: u32) -> Option<ServiceInfo> {
        let services = self.services.lock();
        self.lookups.fetch_add(1, Ordering::SeqCst);
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

pub fn service_register_with_meta(
    name: &str,
    description: &str,
    endpoint: usize,
    version: Option<(u16,u16)>,
    capabilities: Option<ServiceCapabilities>,
) -> u32 {
    let mut service = ServiceInfo::new(name, description, endpoint);
    service.version = version;
    service.capabilities = capabilities;
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

pub fn service_stats_registrations() -> u64 {
    SERVICE_REGISTRY.registrations.load(Ordering::SeqCst)
}

pub fn service_stats_lookups() -> u64 {
    SERVICE_REGISTRY.lookups.load(Ordering::SeqCst)
}

pub fn service_stats_string() -> alloc::string::String {
    use alloc::string::ToString;
    let regs = SERVICE_REGISTRY.registrations.load(Ordering::SeqCst);
    let lookups = SERVICE_REGISTRY.lookups.load(Ordering::SeqCst);
    let services = SERVICE_REGISTRY.all_services();
    let mut s = alloc::string::String::new();
    s.push_str("# Service Registry Stats\n");
    s.push_str(&("registrations: ".to_string() + &regs.to_string() + "\n"));
    s.push_str(&("lookups: ".to_string() + &lookups.to_string() + "\n"));
    s.push_str(&("services: ".to_string() + &services.len().to_string() + "\n\n"));
    for info in services {
        let ver = info.version.map(|(maj,min)| format!("{}.{}", maj, min)).unwrap_or_else(|| "n/a".to_string());
        let caps = info.capabilities.map(|c| format!("0x{:08x}", c.bits())).unwrap_or_else(|| "n/a".to_string());
        s.push_str(&format!(
            "- [{}] id={} name={} ver={} caps={} desc={} deps={:?}\n",
            match info.status { ServiceStatus::Active => "active", ServiceStatus::Inactive => "inactive", ServiceStatus::Failed => "failed", ServiceStatus::Suspended => "suspended" },
            info.service_id,
            info.name,
            ver,
            caps,
            info.description,
            info.dependencies,
        ));
    }
    s
}
