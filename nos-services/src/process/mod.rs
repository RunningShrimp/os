//! Process services
//!
//! This module provides process related services.

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

/// Process service
#[cfg(feature = "alloc")]
pub struct ProcessService {
    name: String,
    status: ServiceStatus,
}

#[cfg(feature = "alloc")]
impl ProcessService {
    /// Create a new process service
    pub fn new(name: &str) -> Self {
        Self {
            #[cfg(feature = "alloc")]
            name: name.to_string(),
            #[cfg(not(feature = "alloc"))]
            name: name.into(),
            status: ServiceStatus::Stopped,
        }
    }
}

#[cfg(feature = "alloc")]
impl Service for ProcessService {
    fn start(&self) -> Result<()> {
        // TODO: Implement actual process service start
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        // TODO: Implement actual process service stop
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn service_type(&self) -> u32 {
        crate::types::service_type::PROCESS
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }
}

/// Register process services
#[cfg(feature = "alloc")]
pub fn register_process_services() -> Result<()> {
    use crate::registry;
    
    let mut registry = registry::get_registry().lock();
    
    // Register init process service
    let init_service = ProcessService::new("init");
    registry.register("init", Box::new(init_service))?;
    
    // Register scheduler service
    let scheduler_service = ProcessService::new("scheduler");
    registry.register("scheduler", Box::new(scheduler_service))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_service() {
        let service = ProcessService::new("test_process");
        
        assert_eq!(service.name(), "test_process");
        assert_eq!(service.service_type(), crate::types::service_type::PROCESS);
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }
}