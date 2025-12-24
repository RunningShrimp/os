//! nos_services stub module
//! This provides stub implementations for the nos_services crate

use alloc::string::String;

// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Stopped,
    Running,
    Paused,
    Error,
}

// Service trait
pub trait Service: core::any::Any {
    fn name(&self) -> &str;
    fn status(&self) -> ServiceStatus;
    fn start(&mut self) -> Result<(), ()>;
    fn stop(&mut self) -> Result<(), ()>;
}

// BaseService trait
pub trait BaseService: Service {
    fn version(&self) -> Version;
}

// SyscallService trait
pub trait SyscallService: BaseService {
    fn handle_syscall(&mut self, args: &[usize]) -> isize;
}

// Version type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

// ServiceRegistry stub
pub struct ServiceRegistry {
    services: alloc::collections::BTreeMap<String, alloc::boxed::Box<dyn SyscallService>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: alloc::collections::BTreeMap::new(),
        }
    }

    pub fn register(&mut self, name: String, service: alloc::boxed::Box<dyn SyscallService>) {
        self.services.insert(name, service);
    }

    pub fn get(&self, name: &str) -> Option<&dyn SyscallService> {
        self.services.get(name).map(|s| s.as_ref())
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn SyscallService> {
        self.services.get_mut(name).map(|s| s.as_mut())
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Traits module
pub mod traits {
    pub use super::{Service, BaseService, SyscallService};
}

// Registry module
pub mod registry {
    pub use super::{ServiceRegistry, Version};
}
