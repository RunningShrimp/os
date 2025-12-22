//! Microkernel service registry
//!
//! Provides service registration and discovery mechanisms for the
//! hybrid architecture. Services can register themselves and be
//! discovered by other components.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use crate::subsystems::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM, EEXIST, ENOENT, EPERM, EBUSY, ETIMEDOUT};

/// Service identifier
pub type ServiceId = u64;

/// Service categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceCategory {
    System,         // Core system services
    Memory,         // Memory management services
    Process,        // Process management services
    FileSystem,     // File system services
    Network,        // Network services
    Device,         // Device driver services
    Security,       // Security services
    Graphics,       // Graphics services
    Audio,          // Audio services
    Application,    // Application services
    Custom(u32),    // Custom category
}

impl ServiceCategory {
    pub fn as_u32(self) -> u32 {
        match self {
            ServiceCategory::System => 0,
            ServiceCategory::Memory => 1,
            ServiceCategory::Process => 2,
            ServiceCategory::FileSystem => 3,
            ServiceCategory::Network => 4,
            ServiceCategory::Device => 5,
            ServiceCategory::Security => 6,
            ServiceCategory::Graphics => 7,
            ServiceCategory::Audio => 8,
            ServiceCategory::Application => 9,
            ServiceCategory::Custom(n) => n,
        }
    }

    pub fn from_u32(n: u32) -> Self {
        match n {
            0 => ServiceCategory::System,
            1 => ServiceCategory::Memory,
            2 => ServiceCategory::Process,
            3 => ServiceCategory::FileSystem,
            4 => ServiceCategory::Network,
            5 => ServiceCategory::Device,
            6 => ServiceCategory::Security,
            7 => ServiceCategory::Graphics,
            8 => ServiceCategory::Audio,
            9 => ServiceCategory::Application,
            n => ServiceCategory::Custom(n),
        }
    }
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Uninitialized,  // Service created but not started
    Starting,       // Service is starting up
    Running,        // Service is running and healthy
    Stopping,       // Service is shutting down
    Stopped,        // Service is stopped
    Error,          // Service encountered an error
    Degraded,       // Service is running but unhealthy
    Maintenance,    // Service is under maintenance
}

/// Service priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServicePriority {
    Critical = 0,   // Critical system services
    High = 1,       // High priority services
    Normal = 2,     // Normal priority services
    Low = 3,        // Low priority services
    Background = 4, // Background services
}

/// Service interface version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl InterfaceVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }

    pub fn is_compatible_with(&self, other: &InterfaceVersion) -> bool {
        // Major version must match
        if self.major != other.major {
            return false;
        }

        // If our minor version is greater, we should be backward compatible
        if self.minor > other.minor {
            return true;
        }

        // If minor versions are equal, patch versions don't matter for compatibility
        self.minor == other.minor
    }

    pub fn as_string(&self) -> String {
        alloc::format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Service capabilities
#[derive(Debug, Clone)]
pub struct ServiceCapabilities {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub create: bool,
    pub delete: bool,
    pub admin: bool,
}

impl ServiceCapabilities {
    pub const fn new() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            create: false,
            delete: false,
            admin: false,
        }
    }

    pub const fn all() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
            create: true,
            delete: true,
            admin: true,
        }
    }

    pub fn as_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.read { flags |= 0x01; }
        if self.write { flags |= 0x02; }
        if self.execute { flags |= 0x04; }
        if self.create { flags |= 0x08; }
        if self.delete { flags |= 0x10; }
        if self.admin { flags |= 0x20; }
        flags
    }
}

/// Service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub id: ServiceId,
    pub name: String,
    pub description: String,
    pub category: ServiceCategory,
    pub status: ServiceStatus,
    pub priority: ServicePriority,
    pub version: InterfaceVersion,
    pub capabilities: ServiceCapabilities,
    pub owner_id: u64,          // Process that owns this service
    pub ipc_channel_id: Option<u64>, // IPC channel for communication
    pub creation_time: u64,
    pub start_time: Option<u64>,
    pub health_check_interval: u64, // Health check interval in nanoseconds
    pub last_health_check: u64,
    pub metrics: ServiceMetrics,
}

impl ServiceInfo {
    pub fn new(id: ServiceId, name: String, description: String, category: ServiceCategory,
               version: InterfaceVersion, owner_id: u64) -> Self {
        let current_time = get_current_time_ns();

        Self {
            id,
            name,
            description,
            category,
            status: ServiceStatus::Uninitialized,
            priority: ServicePriority::Normal,
            version,
            capabilities: ServiceCapabilities::new(),
            owner_id,
            ipc_channel_id: None,
            creation_time: current_time,
            start_time: None,
            health_check_interval: 5_000_000_000, // Default 5 seconds
            last_health_check: current_time,
            metrics: ServiceMetrics::new(),
        }
    }

    pub fn is_healthy(&self) -> bool {
        match self.status {
            ServiceStatus::Running => {
                let current_time = get_current_time_ns();
                current_time - self.last_health_check <= self.health_check_interval * 2
            }
            _ => false,
        }
    }

    pub fn update_health(&mut self, healthy: bool) {
        self.last_health_check = get_current_time_ns();
        if !healthy && self.status == ServiceStatus::Running {
            self.status = ServiceStatus::Error;
        }
    }
}

/// Service performance metrics
#[derive(Debug)]
pub struct ServiceMetrics {
    pub request_count: AtomicU64,
    pub success_count: AtomicU64,
    pub error_count: AtomicU64,
    pub total_response_time: AtomicU64,
    pub memory_usage: AtomicU64,
    pub cpu_usage: AtomicU64,
}

impl Clone for ServiceMetrics {
    fn clone(&self) -> Self {
        Self {
            request_count: AtomicU64::new(self.request_count.load(Ordering::SeqCst)),
            success_count: AtomicU64::new(self.success_count.load(Ordering::SeqCst)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::SeqCst)),
            total_response_time: AtomicU64::new(self.total_response_time.load(Ordering::SeqCst)),
            memory_usage: AtomicU64::new(self.memory_usage.load(Ordering::SeqCst)),
            cpu_usage: AtomicU64::new(self.cpu_usage.load(Ordering::SeqCst)),
        }
    }
}

impl ServiceMetrics {
    pub const fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_response_time: AtomicU64::new(0),
            memory_usage: AtomicU64::new(0),
            cpu_usage: AtomicU64::new(0),
        }
    }

    pub fn record_request(&self, response_time_ns: u64, success: bool) {
        self.request_count.fetch_add(1, Ordering::SeqCst);
        self.total_response_time.fetch_add(response_time_ns, Ordering::SeqCst);

        if success {
            self.success_count.fetch_add(1, Ordering::SeqCst);
        } else {
            self.error_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn get_request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    pub fn get_success_count(&self) -> u64 {
        self.success_count.load(Ordering::SeqCst)
    }

    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::SeqCst)
    }

    pub fn get_average_response_time(&self) -> u64 {
        let request_count = self.get_request_count();
        if request_count == 0 {
            0
        } else {
            self.total_response_time.load(Ordering::SeqCst) / request_count
        }
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.get_request_count();
        if total == 0 {
            0.0
        } else {
            self.get_success_count() as f64 / total as f64
        }
    }
}

/// Service dependency
#[derive(Debug, Clone)]
pub struct ServiceDependency {
    pub service_id: ServiceId,
    pub required_version: InterfaceVersion,
    pub optional: bool,
}

/// Service registry
pub struct ServiceRegistry {
    pub services: Mutex<BTreeMap<ServiceId, ServiceInfo>>,
    pub name_index: Mutex<BTreeMap<String, ServiceId>>,
    pub category_index: Mutex<BTreeMap<ServiceCategory, Vec<ServiceId>>>,
    pub dependency_graph: Mutex<BTreeMap<ServiceId, Vec<ServiceDependency>>>,
    pub next_service_id: AtomicU64,
    pub total_services: AtomicUsize,
    pub running_services: AtomicUsize,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Mutex::new(BTreeMap::new()),
            name_index: Mutex::new(BTreeMap::new()),
            category_index: Mutex::new(BTreeMap::new()),
            dependency_graph: Mutex::new(BTreeMap::new()),
            next_service_id: AtomicU64::new(1),
            total_services: AtomicUsize::new(0),
            running_services: AtomicUsize::new(0),
        }
    }

    pub fn register_service(&self, mut service: ServiceInfo) -> Result<ServiceId, i32> {
        let mut services = self.services.lock();
        let mut name_index = self.name_index.lock();
        let mut category_index = self.category_index.lock();

        // Check if service name already exists
        if name_index.contains_key(&service.name) {
            return Err(EEXIST);
        }

        // Generate service ID if not provided
        if service.id == 0 {
            service.id = self.next_service_id.fetch_add(1, Ordering::SeqCst);
        }

        // Add to registry
        services.insert(service.id, service.clone());
        name_index.insert(service.name.clone(), service.id);

        // Add to category index
        category_index.entry(service.category)
            .or_insert_with(Vec::new)
            .push(service.id);

        self.total_services.fetch_add(1, Ordering::SeqCst);

        Ok(service.id)
    }

    pub fn unregister_service(&self, service_id: ServiceId) -> Result<(), i32> {
        let mut services = self.services.lock();
        let mut name_index = self.name_index.lock();
        let mut category_index = self.category_index.lock();

        let service = services.get(&service_id).ok_or(ENOENT)?;
        let service_status = service.status;

        // Remove from name index
        name_index.remove(&service.name);

        // Remove from category index
        if let Some(services_in_category) = category_index.get_mut(&service.category) {
            services_in_category.retain(|&id| id != service_id);
        }

        // Remove from main registry
        if services.remove(&service_id).is_some() {
            self.total_services.fetch_sub(1, Ordering::SeqCst);
            if service_status == ServiceStatus::Running {
                self.running_services.fetch_sub(1, Ordering::SeqCst);
            }
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    pub fn find_service_by_name(&self, name: &str) -> Option<ServiceInfo> {
        let name_index = self.name_index.lock();
        let services = self.services.lock();

        name_index.get(name)
            .and_then(|&id| services.get(&id).cloned())
    }

    pub fn find_service_by_id(&self, service_id: ServiceId) -> Option<ServiceInfo> {
        let services = self.services.lock();
        services.get(&service_id).cloned()
    }

    pub fn find_services_by_category(&self, category: ServiceCategory) -> Vec<ServiceInfo> {
        let category_index = self.category_index.lock();
        let services = self.services.lock();

        category_index.get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|&id| services.get(&id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn update_service_status(&self, service_id: ServiceId, status: ServiceStatus) -> Result<(), i32> {
        let mut services = self.services.lock();
        let service = services.get_mut(&service_id).ok_or(ENOENT)?;

        let old_status = service.status;
        service.status = status;

        // Update running services count
        match (old_status, status) {
            (ServiceStatus::Running, _) => {
                self.running_services.fetch_sub(1, Ordering::SeqCst);
            }
            (_, ServiceStatus::Running) => {
                service.start_time = Some(get_current_time_ns());
                self.running_services.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }

        Ok(())
    }

    pub fn set_service_priority(&self, service_id: ServiceId, priority: ServicePriority) -> Result<(), i32> {
        let mut services = self.services.lock();
        let service = services.get_mut(&service_id).ok_or(ENOENT)?;
        service.priority = priority;
        Ok(())
    }

    pub fn set_service_ipc_channel(&self, service_id: ServiceId, channel_id: u64) -> Result<(), i32> {
        let mut services = self.services.lock();
        let service = services.get_mut(&service_id).ok_or(ENOENT)?;
        service.ipc_channel_id = Some(channel_id);
        Ok(())
    }

    pub fn add_service_dependency(&self, service_id: ServiceId, dependency: ServiceDependency) -> Result<(), i32> {
        let services = self.services.lock();
        let mut dependency_graph = self.dependency_graph.lock();

        // Check if service exists
        if !services.contains_key(&service_id) {
            return Err(ENOENT);
        }

        // Check if dependency service exists
        if !services.contains_key(&dependency.service_id) {
            return Err(ENOENT);
        }

        // Check for circular dependencies
        if self.would_create_circular_dependency(service_id, &dependency) {
            return Err(EINVAL);
        }

        dependency_graph.entry(service_id)
            .or_insert_with(Vec::new)
            .push(dependency);

        Ok(())
    }

    pub fn remove_service_dependency(&self, service_id: ServiceId, dependency_id: ServiceId) -> Result<(), i32> {
        let mut dependency_graph = self.dependency_graph.lock();

        if let Some(dependencies) = dependency_graph.get_mut(&service_id) {
            dependencies.retain(|dep| dep.service_id != dependency_id);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    pub fn get_service_dependencies(&self, service_id: ServiceId) -> Vec<ServiceDependency> {
        let dependency_graph = self.dependency_graph.lock();
        dependency_graph.get(&service_id).cloned().unwrap_or_default()
    }

    pub fn get_dependent_services(&self, service_id: ServiceId) -> Vec<ServiceId> {
        let dependency_graph = self.dependency_graph.lock();
        let mut dependents = Vec::new();

        for (&id, dependencies) in dependency_graph.iter() {
            if dependencies.iter().any(|dep| dep.service_id == service_id) {
                dependents.push(id);
            }
        }

        dependents
    }

    fn would_create_circular_dependency(&self, service_id: ServiceId, new_dependency: &ServiceDependency) -> bool {
        // Simple cycle detection using DFS
        let mut visited = alloc::collections::BTreeSet::new();
        self.has_cycle_recursive(new_dependency.service_id, service_id, &mut visited)
    }

    fn has_cycle_recursive(&self, current: ServiceId, target: ServiceId, visited: &mut alloc::collections::BTreeSet<ServiceId>) -> bool {
        if current == target {
            return true;
        }

        if visited.contains(&current) {
            return false;
        }

        visited.insert(current);

        let dependency_graph = self.dependency_graph.lock();
        if let Some(dependencies) = dependency_graph.get(&current) {
            for dep in dependencies {
                if self.has_cycle_recursive(dep.service_id, target, visited) {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_running_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.lock();
        services.values()
            .filter(|s| s.status == ServiceStatus::Running)
            .cloned()
            .collect()
    }

    pub fn get_healthy_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.lock();
        services.values()
            .filter(|s| s.is_healthy())
            .cloned()
            .collect()
    }

    pub fn get_services_by_priority(&self, priority: ServicePriority) -> Vec<ServiceInfo> {
        let services = self.services.lock();
        services.values()
            .filter(|s| s.priority == priority)
            .cloned()
            .collect()
    }

    pub fn perform_health_checks(&self) -> Vec<(ServiceId, bool)> {
        let mut services = self.services.lock();
        let mut results = Vec::new();

        for (id, service) in services.iter_mut() {
            if service.status == ServiceStatus::Running {
                let healthy = service.is_healthy();
                service.update_health(healthy);
                results.push((*id, healthy));
                
                // Auto-mark as Error if unhealthy for too long
                if !healthy {
                    let current_time = get_current_time_ns();
                    if current_time - service.last_health_check > service.health_check_interval * 3 {
                        service.status = ServiceStatus::Error;
                    }
                }
            }
        }

        results
    }

    /// Perform health check for a specific service
    pub fn check_service_health(&self, service_id: ServiceId) -> Result<bool, i32> {
        let mut services = self.services.lock();
        let service = services.get_mut(&service_id).ok_or(ENOENT)?;
        
        if service.status != ServiceStatus::Running {
            return Ok(false);
        }
        
        let healthy = service.is_healthy();
        service.update_health(healthy);
        Ok(healthy)
    }

    /// Auto-register a service with default settings
    pub fn auto_register_service(
        &self,
        name: String,
        description: String,
        category: ServiceCategory,
        owner_id: u64,
    ) -> Result<ServiceId, i32> {
        let version = InterfaceVersion::new(1, 0, 0);
        let service = ServiceInfo::new(0, name, description, category, version, owner_id);
        self.register_service(service)
    }

    /// Auto-register a service and start it immediately
    pub fn auto_register_and_start(
        &self,
        name: String,
        description: String,
        category: ServiceCategory,
        owner_id: u64,
    ) -> Result<ServiceId, i32> {
        let service_id = self.auto_register_service(name, description, category, owner_id)?;
        self.update_service_status(service_id, ServiceStatus::Running)?;
        Ok(service_id)
    }

    /// Find services by name pattern (prefix match)
    pub fn find_services_by_name_pattern(&self, pattern: &str) -> Vec<ServiceInfo> {
        let name_index = self.name_index.lock();
        let services = self.services.lock();
        
        name_index.iter()
            .filter(|(name, _)| name.starts_with(pattern))
            .filter_map(|(_, &id)| services.get(&id).cloned())
            .collect()
    }

    /// Find services by owner
    pub fn find_services_by_owner(&self, owner_id: u64) -> Vec<ServiceInfo> {
        let services = self.services.lock();
        services.values()
            .filter(|s| s.owner_id == owner_id)
            .cloned()
            .collect()
    }

    /// Get services that need health checks (running services with expired health check)
    pub fn get_services_needing_health_check(&self) -> Vec<ServiceId> {
        let services = self.services.lock();
        let current_time = get_current_time_ns();
        
        services.iter()
            .filter(|(_, s)| {
                s.status == ServiceStatus::Running &&
                (current_time - s.last_health_check) >= s.health_check_interval
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Set health check interval for a service
    pub fn set_health_check_interval(&self, service_id: ServiceId, interval_ns: u64) -> Result<(), i32> {
        let mut services = self.services.lock();
        let service = services.get_mut(&service_id).ok_or(ENOENT)?;
        service.health_check_interval = interval_ns;
        Ok(())
    }

    /// Start a service (transition from Uninitialized/Stopped to Running)
    pub fn start_service(&self, service_id: ServiceId) -> Result<(), i32> {
        let services = self.services.lock();
        let service = services.get(&service_id).ok_or(ENOENT)?;
        
        // Check current status
        match service.status {
            ServiceStatus::Running => return Ok(()), // Already running
            ServiceStatus::Starting => return Err(EBUSY), // Already starting
            ServiceStatus::Stopping => return Err(EBUSY), // Currently stopping
            _ => {}
        }
        
        // Check dependencies
        let dependencies = self.get_service_dependencies(service_id);
        for dep in dependencies {
            if !dep.optional {
                let dep_service = services.get(&dep.service_id).ok_or(ENOENT)?;
                if dep_service.status != ServiceStatus::Running {
                    return Err(EINVAL); // Dependency not running
                }
                // Check version compatibility
                if !service.version.is_compatible_with(&dep.required_version) {
                    return Err(EINVAL); // Version mismatch
                }
            }
        }
        
        drop(services);
        self.update_service_status(service_id, ServiceStatus::Starting)?;
        
        // Try to start the service
        // Note: In a full implementation, we would look up the service implementation
        // and call its start() method. For now, we'll use a callback mechanism.
        // Services should register their lifecycle callbacks during registration.
        
        // Simulate service startup (in real implementation, call service.start())
        // For now, mark as running after a brief delay simulation
        self.update_service_status(service_id, ServiceStatus::Running)?;
        
        // Update health check timestamp
        {
            let mut services = self.services.lock();
            if let Some(service) = services.get_mut(&service_id) {
                service.last_health_check = get_current_time_ns();
            }
        }
        
        Ok(())
    }

    /// Stop a service (transition to Stopped)
    pub fn stop_service(&self, service_id: ServiceId) -> Result<(), i32> {
        let services = self.services.lock();
        let service = services.get(&service_id).ok_or(ENOENT)?;
        
        // Check current status
        match service.status {
            ServiceStatus::Stopped => return Ok(()), // Already stopped
            ServiceStatus::Stopping => return Err(EBUSY), // Already stopping
            ServiceStatus::Uninitialized => {
                drop(services);
                return self.update_service_status(service_id, ServiceStatus::Stopped);
            }
            _ => {}
        }
        
        // Check if any services depend on this one
        let dependents = self.get_dependent_services(service_id);
        drop(services);
        
        if !dependents.is_empty() {
            // Check if any dependent services are running
            let services = self.services.lock();
            for dep_id in dependents {
                if let Some(dep_service) = services.get(&dep_id) {
                    if dep_service.status == ServiceStatus::Running {
                        return Err(EBUSY); // Cannot stop, has running dependents
                    }
                }
            }
            drop(services);
        }
        
        self.update_service_status(service_id, ServiceStatus::Stopping)?;
        
        // Try to stop the service
        // Note: In a full implementation, we would look up the service implementation
        // and call its stop() method. For now, we'll simulate stopping.
        
        // Wait for service to fully stop (with timeout)
        const STOP_TIMEOUT_NS: u64 = 5_000_000_000; // 5 seconds
        let start_time = get_current_time_ns();
        
        loop {
            let services = self.services.lock();
            let service = services.get(&service_id).ok_or(ENOENT)?;
            
            // Check if service has stopped
            if service.status == ServiceStatus::Stopped {
                drop(services);
                return Ok(());
            }
            
            // Check timeout
            let elapsed = get_current_time_ns() - start_time;
            if elapsed > STOP_TIMEOUT_NS {
                drop(services);
                // Force stop on timeout
                self.update_service_status(service_id, ServiceStatus::Stopped)?;
                return Err(ETIMEDOUT);
            }
            
            drop(services);
            
            // Brief delay before checking again
            crate::subsystems::time::sleep_ms(10); // 10ms delay
        }
    }

    /// Restart a service
    pub fn restart_service(&self, service_id: ServiceId) -> Result<(), i32> {
        self.stop_service(service_id)?;
        
        // Ensure service is fully stopped before restarting
        let services = self.services.lock();
        let service = services.get(&service_id).ok_or(ENOENT)?;
        if service.status != ServiceStatus::Stopped {
            drop(services);
            return Err(EBUSY); // Service didn't stop properly
        }
        drop(services);
        
        self.start_service(service_id)
    }
    
    /// Perform health check on a service
    pub fn perform_health_check(&self, service_id: ServiceId) -> Result<bool, i32> {
        let mut services = self.services.lock();
        let service = services.get_mut(&service_id).ok_or(ENOENT)?;
        
        if service.status != ServiceStatus::Running {
            return Ok(false);
        }
        
        // Update health check timestamp
        service.last_health_check = get_current_time_ns();
        
        // In a full implementation, we would call service.health_check()
        // For now, we'll check if the service is still responsive
        // by checking if it's been updated recently
        
        let is_healthy = service.is_healthy();
        
        if !is_healthy {
            service.status = ServiceStatus::Degraded;
            self.running_services.fetch_sub(1, Ordering::SeqCst);
        }
        
        Ok(is_healthy)
    }
    
    /// Perform health checks on all services that need them
    pub fn perform_all_health_checks(&self) -> usize {
        let services_to_check = self.get_services_needing_health_check();
        let mut unhealthy_count = 0;
        
        for service_id in services_to_check {
            if let Ok(false) = self.perform_health_check(service_id) {
                unhealthy_count += 1;
            }
        }
        
        unhealthy_count
    }

    pub fn get_service_metrics(&self, service_id: ServiceId) -> Option<ServiceMetrics> {
        let services = self.services.lock();
        services.get(&service_id).map(|s| s.metrics.clone())
    }

    pub fn get_stats(&self) -> RegistryStats {
        RegistryStats {
            total_services: self.total_services.load(Ordering::SeqCst),
            running_services: self.running_services.load(Ordering::SeqCst),
            unhealthy_services: {
                let services = self.services.lock();
                services.values()
                    .filter(|s| s.status == ServiceStatus::Running && !s.is_healthy())
                    .count()
            },
        }
    }
}

/// Registry statistics
#[derive(Debug)]
pub struct RegistryStats {
    pub total_services: usize,
    pub running_services: usize,
    pub unhealthy_services: usize,
}

/// Global service registry
static mut GLOBAL_SERVICE_REGISTRY: Option<ServiceRegistry> = None;
static REGISTRY_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize service registry
pub fn init() -> Result<(), i32> {
    if REGISTRY_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    let registry = ServiceRegistry::new();

    unsafe {
        GLOBAL_SERVICE_REGISTRY = Some(registry);
    }

    REGISTRY_INIT.store(true, Ordering::SeqCst);
    Ok(())
}

/// Get global service registry
pub fn get_service_registry() -> Option<&'static ServiceRegistry> {
    unsafe {
        GLOBAL_SERVICE_REGISTRY.as_ref()
    }
}

/// Get mutable global service registry
pub fn get_service_registry_mut() -> Option<&'static mut ServiceRegistry> {
    unsafe {
        GLOBAL_SERVICE_REGISTRY.as_mut()
    }
}

/// Get current time in nanoseconds
fn get_current_time_ns() -> u64 {
    crate::subsystems::time::get_time_ns()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_version() {
        let v1 = InterfaceVersion::new(1, 0, 0);
        let v2 = InterfaceVersion::new(1, 1, 0);
        let v3 = InterfaceVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v1));
        assert!(v2.is_compatible_with(&v1)); // Higher minor version compatible
        assert!(!v1.is_compatible_with(&v2)); // Lower minor version not compatible
        assert!(!v3.is_compatible_with(&v1)); // Different major version

        assert_eq!(v1.as_string(), "1.0.0");
    }

    #[test]
    fn test_service_capabilities() {
        let caps = ServiceCapabilities::new();
        assert_eq!(caps.as_flags(), 0);

        let mut caps = ServiceCapabilities::new();
        caps.read = true;
        caps.write = true;
        assert_eq!(caps.as_flags(), 0x03);

        let all_caps = ServiceCapabilities::all();
        assert_eq!(all_caps.as_flags(), 0x3F);
    }

    #[test]
    fn test_service_info() {
        let version = InterfaceVersion::new(1, 0, 0);
        let mut service = ServiceInfo::new(
            1,
            "test-service".to_string(),
            "Test service".to_string(),
            ServiceCategory::System,
            version,
            100
        );

        assert_eq!(service.id, 1);
        assert_eq!(service.name, "test-service");
        assert_eq!(service.status, ServiceStatus::Uninitialized);

        service.status = ServiceStatus::Running;
        service.start_time = Some(get_current_time_ns());

        assert!(service.is_healthy());
    }

    #[test]
    fn test_service_metrics() {
        let metrics = ServiceMetrics::new();

        assert_eq!(metrics.get_request_count(), 0);
        assert_eq!(metrics.get_success_rate(), 0.0);

        metrics.record_request(1000, true);
        metrics.record_request(2000, false);

        assert_eq!(metrics.get_request_count(), 2);
        assert_eq!(metrics.get_success_count(), 1);
        assert_eq!(metrics.get_error_count(), 1);
        assert_eq!(metrics.get_average_response_time(), 1500);
        assert_eq!(metrics.get_success_rate(), 0.5);
    }

    #[test]
    fn test_service_registry() {
        let registry = ServiceRegistry::new();

        let version = InterfaceVersion::new(1, 0, 0);
        let service = ServiceInfo::new(
            0,
            "test-service".to_string(),
            "Test service".to_string(),
            ServiceCategory::System,
            version,
            100
        );

        // Test registration
        let service_id = registry.register_service(service).unwrap();
        assert!(service_id > 0);

        // Test lookup by name
        let found = registry.find_service_by_name("test-service");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-service");

        // Test lookup by ID
        let found = registry.find_service_by_id(service_id);
        assert!(found.is_some());

        // Test status update
        assert_eq!(registry.update_service_status(service_id, ServiceStatus::Running), Ok(()));
        let found = registry.find_service_by_id(service_id).unwrap();
        assert_eq!(found.status, ServiceStatus::Running);

        // Test category lookup
        let system_services = registry.find_services_by_category(ServiceCategory::System);
        assert_eq!(system_services.len(), 1);

        // Test unregistration
        assert_eq!(registry.unregister_service(service_id), Ok(()));
        let found = registry.find_service_by_name("test-service");
        assert!(found.is_none());
    }

    #[test]
    fn test_service_dependency() {
        let registry = ServiceRegistry::new();

        let version = InterfaceVersion::new(1, 0, 0);

        // Create two services
        let service1 = ServiceInfo::new(
            0,
            "service1".to_string(),
            "Service 1".to_string(),
            ServiceCategory::System,
            version,
            100
        );

        let service2 = ServiceInfo::new(
            0,
            "service2".to_string(),
            "Service 2".to_string(),
            ServiceCategory::System,
            version,
            101
        );

        let id1 = registry.register_service(service1).unwrap();
        let id2 = registry.register_service(service2).unwrap();

        // Add dependency
        let dependency = ServiceDependency {
            service_id: id1,
            required_version: version,
            optional: false,
        };

        assert_eq!(registry.add_service_dependency(id2, dependency), Ok(()));

        // Test dependency lookup
        let dependencies = registry.get_service_dependencies(id2);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].service_id, id1);

        // Test dependent services lookup
        let dependents = registry.get_dependent_services(id1);
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], id2);

        // Remove dependency
        assert_eq!(registry.remove_service_dependency(id2, id1), Ok(()));
        let dependencies = registry.get_service_dependencies(id2);
        assert_eq!(dependencies.len(), 0);
    }
}