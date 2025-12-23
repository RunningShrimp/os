//! Network services
//!
//! This module provides network related services.

use nos_api::Result;
use crate::core::{Service, ServiceStatus};
use alloc::string::String;
use alloc::string::ToString;
use alloc::boxed::Box;

/// Network service
pub struct NetworkService {
    name: String,
    interface: String,
    status: ServiceStatus,
}

impl NetworkService {
    /// Create a new network service
    pub fn new(name: &str, interface: &str) -> Self {
        Self {
            name: name.to_string(),
            interface: interface.to_string(),
            status: ServiceStatus::Stopped,
        }
    }

    /// Get the network interface
    pub fn interface(&self) -> &str {
        &self.interface
    }
}

impl Service for NetworkService {
    fn start(&self) -> Result<()> {
        // TODO: Implement actual network service start
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        // TODO: Implement actual network service stop
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn service_type(&self) -> u32 {
        crate::types::service_type::NETWORK
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }
}

/// Register network services
pub fn register_network_services() -> Result<()> {
    use crate::registry;
    
    let mut registry = registry::get_registry().lock();
    
    // Register loopback network service
    let loopback_service = NetworkService::new("loopback", "lo");
    registry.register("loopback", Box::new(loopback_service))?;
    
    // Register Ethernet network service
    let ethernet_service = NetworkService::new("ethernet", "eth0");
    registry.register("ethernet", Box::new(ethernet_service))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_service() {
        let service = NetworkService::new("test_network", "eth0");
        
        assert_eq!(service.name(), "test_network");
        assert_eq!(service.interface(), "eth0");
        assert_eq!(service.service_type(), crate::types::service_type::NETWORK);
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }
}