//! IPC services
//!
//! This module provides inter-process communication related services.

use nos_api::Result;
use crate::core::{Service, ServiceStatus};
use alloc::string::String;
use alloc::string::ToString;
use alloc::boxed::Box;

/// IPC service
pub struct IpcService {
    name: String,
    status: ServiceStatus,
}

impl IpcService {
    /// Create a new IPC service
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: ServiceStatus::Stopped,
        }
    }
}

impl Service for IpcService {
    fn start(&self) -> Result<()> {
        // TODO: Implement actual IPC service start
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        // TODO: Implement actual IPC service stop
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn service_type(&self) -> u32 {
        crate::types::service_type::IPC
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }
}

/// Register IPC services
pub fn register_ipc_services() -> Result<()> {
    use crate::registry;

    let mut registry = registry::get_registry()?.lock();
    
    // Register message queue service
    let mq_service = IpcService::new("message_queue");
    registry.register("message_queue", Box::new(mq_service))?;
    
    // Register semaphore service
    let semaphore_service = IpcService::new("semaphore");
    registry.register("semaphore", Box::new(semaphore_service))?;
    
    // Register shared memory service
    let shm_service = IpcService::new("shared_memory");
    registry.register("shared_memory", Box::new(shm_service))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_service() {
        let service = IpcService::new("test_ipc");
        
        assert_eq!(service.name(), "test_ipc");
        assert_eq!(service.service_type(), crate::types::service_type::IPC);
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }
}