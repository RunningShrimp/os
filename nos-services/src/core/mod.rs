//! Core service interfaces and types
//!
//! This module provides core service interfaces and common types.

use nos_api::Result;
use alloc::string::String;
use alloc::string::ToString;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::boxed::Box;

/// Service trait
pub trait Service: Send + Sync {
    /// Start the service
    fn start(&self) -> Result<()>;
    
    /// Stop the service
    fn stop(&self) -> Result<()>;
    
    /// Get the service name
    fn name(&self) -> &str;
    
    /// Get the service type
    fn service_type(&self) -> u32;
    
    /// Get the service status
    fn status(&self) -> ServiceStatus {
        ServiceStatus::Stopped
    }
    
    /// Check if the service is healthy
    fn is_healthy(&self) -> bool {
        true
    }
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is stopped
    Stopped,
    /// Service is starting
    Starting,
    /// Service is running
    Running,
    /// Service is stopping
    Stopping,
    /// Service has an error
    Error,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Service type
    pub service_type: u32,
    /// Service version
    pub version: String,
    /// Service description
    pub description: String,
    /// Service parameters
    pub parameters: BTreeMap<String, String>,
    /// Service dependencies
    pub dependencies: Vec<String>,
    /// Auto-start flag
    pub auto_start: bool,
    /// Restart on failure
    pub restart_on_failure: bool,
    /// Maximum restart attempts
    pub max_restart_attempts: u32,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            service_type: 0,
            version: "1.0.0".to_string(),
            description: String::new(),
            parameters: BTreeMap::new(),
            dependencies: Vec::new(),
            auto_start: false,
            restart_on_failure: false,
            max_restart_attempts: 3,
        }
    }
}

/// Service manager trait
pub trait ServiceManager: Send + Sync {
    /// Register a service
    fn register_service(&mut self, name: &str, service: Box<dyn Service>) -> Result<u32>;
    
    /// Unregister a service
    fn unregister_service(&mut self, id: u32) -> Result<()>;
    
    /// Start a service
    fn start_service(&mut self, id: u32) -> Result<()>;
    
    /// Stop a service
    fn stop_service(&mut self, id: u32) -> Result<()>;
    
    /// Get a service by ID
    fn get_service(&self, id: u32) -> Option<&dyn Service>;
    
    /// Get a service by name
    fn get_service_by_name(&self, name: &str) -> Option<&dyn Service>;
    
    /// List all services
    fn list_services(&self) -> alloc::vec::Vec<ServiceInfo>;
    
    /// Get service statistics
    fn get_stats(&self) -> ServiceStats;
}

/// Service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service ID
    pub id: u32,
    /// Service name
    pub name: String,
    /// Service type
    pub service_type: u32,
    /// Service status
    pub status: ServiceStatus,
    /// Registration time
    pub registration_time: u64,
    /// Start time
    pub start_time: Option<u64>,
    /// Restart count
    pub restart_count: u32,
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// Total number of services
    pub total_services: u64,
    /// Number of running services
    pub running_services: u64,
    /// Number of stopped services
    pub stopped_services: u64,
    /// Number of services with errors
    pub error_services: u64,
    /// Average service uptime (seconds)
    pub avg_uptime: u64,
    /// Total number of service restarts
    pub total_restarts: u64,
}

impl Default for ServiceStats {
    fn default() -> Self {
        Self {
            total_services: 0,
            running_services: 0,
            stopped_services: 0,
            error_services: 0,
            avg_uptime: 0,
            total_restarts: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        name: &'static str,
        service_type: u32,
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
            self.service_type
        }
    }

    #[test]
    fn test_service() {
        let service = TestService {
            name: "test_service",
            service_type: 1,
        };
        
        assert_eq!(service.name(), "test_service");
        assert_eq!(service.service_type(), 1);
        assert_eq!(service.status(), ServiceStatus::Stopped);
        assert!(service.is_healthy());
    }

    #[test]
    fn test_service_config() {
        let config = ServiceConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.max_restart_attempts, 3);
        assert!(!config.auto_start);
        assert!(!config.restart_on_failure);
    }
}