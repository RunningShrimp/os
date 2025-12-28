//! 服务模块
//! 
//! 本模块提供服务管理功能，使用nos-services crate作为统一实现。
//! 提供服务注册、发现和生命周期管理功能。

// Re-export from nos-services crate
pub use nos_services as services;

// Re-export commonly used types
pub use services::{
    ServiceRegistry, Service, ServiceStatus, ServiceInfo, ServiceCapabilities,
    ServiceManager, ServiceError, ServiceResult
};

/// Initialize services
pub fn init() -> Result<(), &'static str> {
    // Initialize the service registry
    let mut registry = services::ServiceRegistry::new();
    
    // Register core services
    register_core_services(&mut registry)?;
    
    crate::println!("[services] Service registry initialized");
    Ok(())
}

/// Register core services
fn register_core_services(registry: &mut services::ServiceRegistry) -> Result<(), &'static str> {
    // Register process service
    let process_service = services::process::ProcessService::new();
    registry.register("process", Box::new(process_service))
        .map_err(|_| "Failed to register process service")?;
    
    // Register memory service
    let memory_service = services::memory::MemoryService::new();
    registry.register("memory", Box::new(memory_service))
        .map_err(|_| "Failed to register memory service")?;
    
    // Register file system service
    let fs_service = services::fs::FileSystemService::new();
    registry.register("filesystem", Box::new(fs_service))
        .map_err(|_| "Failed to register filesystem service")?;
    
    // Register network service
    let network_service = services::network::NetworkService::new();
    registry.register("network", Box::new(network_service))
        .map_err(|_| "Failed to register network service")?;
    
    // Register IPC service
    let ipc_service = services::ipc::IpcService::new();
    registry.register("ipc", Box::new(ipc_service))
        .map_err(|_| "Failed to register IPC service")?;
    
    crate::println!("[services] Core services registered");
    Ok(())
}

/// Get service by name
pub fn get_service(name: &str) -> Option<Box<dyn services::Service>> {
    // This would need to be implemented based on the actual service registry
    // For now, return None as placeholder
    None
}

/// Get service statistics
pub fn get_service_stats() -> services::ServiceStats {
    // This would need to be implemented based on the actual service registry
    // For now, return default stats
    services::ServiceStats::default()
}