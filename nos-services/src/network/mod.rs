//! Network services
//!
//! This module provides network related services.

#[cfg(feature = "alloc")]
use nos_api::Result;
#[cfg(feature = "alloc")]
use crate::core::{Service, ServiceStatus};
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// Network service
#[cfg(feature = "alloc")]
pub struct NetworkService {
    name: String,
    interface: String,
    status: ServiceStatus,
}

#[cfg(feature = "alloc")]
impl NetworkService {
    /// Create a new network service
    pub fn new(name: &str, interface: &str) -> Self {
        Self {
            #[cfg(feature = "alloc")]
            name: name.to_string(),
            #[cfg(not(feature = "alloc"))]
            name: name.into(),
            #[cfg(feature = "alloc")]
            interface: interface.to_string(),
            #[cfg(not(feature = "alloc"))]
            interface: interface.into(),
            status: ServiceStatus::Stopped,
        }
    }

    /// Get the network interface
    pub fn interface(&self) -> &str {
        &self.interface
    }
}

#[cfg(feature = "alloc")]
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
#[cfg(feature = "alloc")]
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