//! Factory traits for creating implementations

use crate::error::Result;

use crate::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use crate::interfaces::{Arc, Box};
#[cfg(feature = "alloc")]
use spin::Mutex;

/// Factory trait for creating service instances
pub trait ServiceFactory {
    /// Service type
    type Service: crate::core::traits::Service;
    
    /// Creates a new service instance
    fn create_service(&self) -> Result<Self::Service>;
}

/// Factory trait for creating memory manager instances
pub trait MemoryManagerFactory {
    /// Memory manager type
    type MemoryManager: crate::memory::interface::MemoryManager;
    
    /// Creates a new memory manager instance
    fn create_memory_manager(&self) -> Result<Self::MemoryManager>;
}

/// Factory trait for creating process manager instances
pub trait ProcessManagerFactory {
    /// Process manager type
    type ProcessManager: crate::process::interface::ProcessManager;
    
    /// Creates a new process manager instance
    fn create_process_manager(&self) -> Result<Self::ProcessManager>;
}

/// Factory trait for creating syscall dispatcher instances
pub trait SyscallDispatcherFactory {
    /// Syscall dispatcher type
    type SyscallDispatcher: crate::syscall::interface::SyscallDispatcher;
    
    /// Creates a new syscall dispatcher instance
    fn create_syscall_dispatcher(&self) -> Result<Self::SyscallDispatcher>;
}

// 使用对象安全的特征包装器
#[cfg(feature = "alloc")]
pub trait ServiceFactoryObj: Send + Sync {
    fn create_service_box(&self) -> Result<Box<dyn crate::core::traits::Service>>;
}

#[cfg(not(feature = "alloc"))]
pub trait ServiceFactoryObj: Send + Sync {
    fn create_service(&self) -> Result<&'static dyn crate::core::traits::Service>;
}

#[cfg(feature = "alloc")]
pub trait MemoryManagerFactoryObj: Send + Sync {
    fn create_memory_manager_box(&self) -> Result<Box<dyn crate::memory::interface::MemoryManager>>;
}

#[cfg(not(feature = "alloc"))]
pub trait MemoryManagerFactoryObj: Send + Sync {
    fn create_memory_manager(&self) -> Result<&'static dyn crate::memory::interface::MemoryManager>;
}

#[cfg(feature = "alloc")]
pub trait ProcessManagerFactoryObj: Send + Sync {
    fn create_process_manager_box(&self) -> Result<Box<dyn crate::process::interface::ProcessManager>>;
}

#[cfg(not(feature = "alloc"))]
pub trait ProcessManagerFactoryObj: Send + Sync {
    fn create_process_manager(&self) -> Result<&'static dyn crate::process::interface::ProcessManager>;
}

#[cfg(feature = "alloc")]
pub trait SyscallDispatcherFactoryObj: Send + Sync {
    fn create_syscall_dispatcher_box(&self) -> Result<Box<dyn crate::syscall::interface::SyscallDispatcher>>;
}

#[cfg(not(feature = "alloc"))]
pub trait SyscallDispatcherFactoryObj: Send + Sync {
    fn create_syscall_dispatcher(&self) -> Result<&'static dyn crate::syscall::interface::SyscallDispatcher>;
}

/// Registry for factories
pub struct FactoryRegistry {
    #[cfg(feature = "alloc")]
    service_factories: BTreeMap<&'static str, Box<dyn ServiceFactoryObj>>,
    #[cfg(not(feature = "alloc"))]
    service_factories: BTreeMap<&'static str, &'static dyn ServiceFactoryObj>,
    
    #[cfg(feature = "alloc")]
    memory_manager_factories: BTreeMap<&'static str, Box<dyn MemoryManagerFactoryObj>>,
    #[cfg(not(feature = "alloc"))]
    memory_manager_factories: BTreeMap<&'static str, &'static dyn MemoryManagerFactoryObj>,
    
    #[cfg(feature = "alloc")]
    process_manager_factories: BTreeMap<&'static str, Box<dyn ProcessManagerFactoryObj>>,
    #[cfg(not(feature = "alloc"))]
    process_manager_factories: BTreeMap<&'static str, &'static dyn ProcessManagerFactoryObj>,
    
    #[cfg(feature = "alloc")]
    syscall_dispatcher_factories: BTreeMap<&'static str, Box<dyn SyscallDispatcherFactoryObj>>,
    #[cfg(not(feature = "alloc"))]
    syscall_dispatcher_factories: BTreeMap<&'static str, &'static dyn SyscallDispatcherFactoryObj>,
}

// Safety: The BTreeMap and Box types inside FactoryRegistry are all thread-safe,
// and the trait objects are Send + Sync, so the entire struct is Sync.
unsafe impl Sync for FactoryRegistry {}

impl FactoryRegistry {
    /// Creates a new factory registry
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "alloc")]
            service_factories: alloc::collections::BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            service_factories: Default::default(),
            
            #[cfg(feature = "alloc")]
            memory_manager_factories: alloc::collections::BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            memory_manager_factories: Default::default(),
            
            #[cfg(feature = "alloc")]
            process_manager_factories: alloc::collections::BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            process_manager_factories: Default::default(),
            
            #[cfg(feature = "alloc")]
            syscall_dispatcher_factories: alloc::collections::BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            syscall_dispatcher_factories: Default::default(),
        }
    }
    
    /// Registers a service factory
    #[cfg(feature = "alloc")]
    pub fn register_service_factory(&mut self, name: &'static str, factory: Box<dyn ServiceFactoryObj>) {
        self.service_factories.insert(name, factory);
    }
    
    /// Registers a memory manager factory
    #[cfg(feature = "alloc")]
    pub fn register_memory_manager_factory(&mut self, name: &'static str, factory: Box<dyn MemoryManagerFactoryObj>) {
        self.memory_manager_factories.insert(name, factory);
    }
    
    /// Registers a process manager factory
    #[cfg(feature = "alloc")]
    pub fn register_process_manager_factory(&mut self, name: &'static str, factory: Box<dyn ProcessManagerFactoryObj>) {
        self.process_manager_factories.insert(name, factory);
    }
    
    /// Registers a syscall dispatcher factory
    #[cfg(feature = "alloc")]
    pub fn register_syscall_dispatcher_factory(&mut self, name: &'static str, factory: Box<dyn SyscallDispatcherFactoryObj>) {
        self.syscall_dispatcher_factories.insert(name, factory);
    }
    
    /// Creates a service instance
    #[cfg(feature = "alloc")]
    pub fn create_service(&self, name: &str) -> Result<Box<dyn crate::core::traits::Service>> {
        if let Some(factory) = self.service_factories.get(&name) {
            factory.create_service_box()
        } else {
            let error = crate::error::Error::NotFound(format!("Factory '{}' not found", name));
            Err(error)
        }
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn create_service(&self, name: &str) -> Result<&'static dyn crate::core::traits::Service> {
        if let Some(factory) = self.service_factories.get(&name) {
            factory.create_service()
        } else {
            Err(crate::error::Error::NotFound("Factory not found"))
        }
    }
    
    /// Creates a memory manager instance
    #[cfg(feature = "alloc")]
    pub fn create_memory_manager(&self, name: &str) -> Result<Box<dyn crate::memory::interface::MemoryManager>> {
        if let Some(factory) = self.memory_manager_factories.get(&name) {
            factory.create_memory_manager_box()
        } else {
            let error = crate::error::Error::NotFound(format!("Memory manager factory '{}' not found", name));
            Err(error)
        }
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn create_memory_manager(&self, name: &str) -> Result<&'static dyn crate::memory::interface::MemoryManager> {
        if let Some(factory) = self.memory_manager_factories.get(&name) {
            factory.create_memory_manager()
        } else {
            Err(crate::error::Error::NotFound("Memory manager factory not found"))
        }
    }
    
    /// Creates a process manager instance
    #[cfg(feature = "alloc")]
    pub fn create_process_manager(&self, name: &str) -> Result<Box<dyn crate::process::interface::ProcessManager>> {
        if let Some(factory) = self.process_manager_factories.get(&name) {
            factory.create_process_manager_box()
        } else {
            let error = crate::error::Error::NotFound(format!("Process manager factory '{}' not found", name));
            Err(error)
        }
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn create_process_manager(&self, name: &str) -> Result<&'static dyn crate::process::interface::ProcessManager> {
        if let Some(factory) = self.process_manager_factories.get(&name) {
            factory.create_process_manager()
        } else {
            Err(crate::error::Error::NotFound("Process manager factory not found"))
        }
    }
    
    /// Creates a syscall dispatcher instance
    #[cfg(feature = "alloc")]
    pub fn create_syscall_dispatcher(&self, name: &str) -> Result<Box<dyn crate::syscall::interface::SyscallDispatcher>> {
        if let Some(factory) = self.syscall_dispatcher_factories.get(&name) {
            factory.create_syscall_dispatcher_box()
        } else {
            let error = crate::error::Error::NotFound(format!("Syscall dispatcher factory '{}' not found", name));
            Err(error)
        }
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn create_syscall_dispatcher(&self, name: &str) -> Result<&'static dyn crate::syscall::interface::SyscallDispatcher> {
        if let Some(factory) = self.syscall_dispatcher_factories.get(&name) {
            factory.create_syscall_dispatcher()
        } else {
            Err(crate::error::Error::NotFound("Syscall dispatcher factory not found"))
        }
    }
}

/// Global factory registry
#[cfg(feature = "alloc")]
use lazy_static::lazy_static;

#[cfg(feature = "alloc")]
lazy_static! {
    /// Thread-safe global factory registry that can be accessed from anywhere
    static ref FACTORY_REGISTRY: Arc<Mutex<Option<FactoryRegistry>>> = Arc::new(Mutex::new(None));
}
/// Initializes the factory registry
#[cfg(feature = "alloc")]
pub fn init_factory_registry() {
    let mut registry = FACTORY_REGISTRY.lock();
    *registry = Some(FactoryRegistry::new());
}

/// Get the global factory registry
#[cfg(feature = "alloc")]
pub fn get_factory_registry() -> spin::MutexGuard<'static, Option<FactoryRegistry>> {
    FACTORY_REGISTRY.lock()
}

/// Convenience function to create a service
#[cfg(feature = "alloc")]
pub fn create_service(name: &str) -> Result<Box<dyn crate::core::traits::Service>> {
    let registry = get_factory_registry();
    (*registry).as_ref()
        .map_or(
            Err(crate::error::Error::NotFound("Factory registry not initialized".to_string())),
            |r: &FactoryRegistry| r.create_service(name)
        )
}

/// Convenience function to create a memory manager
#[cfg(feature = "alloc")]
pub fn create_memory_manager(name: &str) -> Result<Box<dyn crate::memory::interface::MemoryManager>> {
    let registry = get_factory_registry();
    (*registry).as_ref()
        .map_or(
            Err(crate::error::Error::NotFound("Factory registry not initialized".to_string())),
            |r: &FactoryRegistry| r.create_memory_manager(name)
        )
}

/// Convenience function to create a process manager
#[cfg(feature = "alloc")]
pub fn create_process_manager(name: &str) -> Result<Box<dyn crate::process::interface::ProcessManager>> {
    let registry = get_factory_registry();
    (*registry).as_ref()
        .map_or(
            Err(crate::error::Error::NotFound("Factory registry not initialized".to_string())),
            |r: &FactoryRegistry| r.create_process_manager(name)
        )
}

/// Convenience function to create a syscall dispatcher
#[cfg(feature = "alloc")]
pub fn create_syscall_dispatcher(name: &str) -> Result<Box<dyn crate::syscall::interface::SyscallDispatcher>> {
    let registry = get_factory_registry();
    (*registry).as_ref()
        .map_or(
            Err(crate::error::Error::NotFound("Factory registry not initialized".to_string())),
            |r: &FactoryRegistry| r.create_syscall_dispatcher(name)
        )
}