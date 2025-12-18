//! Service Types Module
//! 
//! This module defines the types and structures used for service registration
//! and management in the NOS kernel.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;

/// Service ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceId(u64);

impl ServiceId {
    /// Create a new service ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    /// Get the raw ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Service priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServicePriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

/// Service state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Uninitialized,
    Initializing,
    Running,
    Stopping,
    Stopped,
    Error,
}

/// Service type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    Kernel,
    Driver,
    FileSystem,
    Network,
    User,
}

/// Service dependency
#[derive(Debug, Clone)]
pub struct ServiceDependency {
    pub service_id: ServiceId,
    pub version: String,
    pub optional: bool,
}

/// Service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub id: ServiceId,
    pub name: String,
    pub version: String,
    pub description: String,
    pub service_type: ServiceType,
    pub priority: ServicePriority,
    pub state: ServiceState,
    pub dependencies: Vec<ServiceDependency>,
    pub properties: BTreeMap<String, String>,
}

impl ServiceInfo {
    /// Create a new service info
    pub fn new(
        id: ServiceId,
        name: String,
        version: String,
        description: String,
        service_type: ServiceType,
        priority: ServicePriority,
    ) -> Self {
        Self {
            id,
            name,
            version,
            description,
            service_type,
            priority,
            state: ServiceState::Uninitialized,
            dependencies: Vec::new(),
            properties: BTreeMap::new(),
        }
    }
    
    /// Add a dependency
    pub fn add_dependency(&mut self, dependency: ServiceDependency) {
        self.dependencies.push(dependency);
    }
    
    /// Add a property
    pub fn add_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }
    
    /// Set the service state
    pub fn set_state(&mut self, state: ServiceState) {
        self.state = state;
    }
}

/// Service interface
pub trait ServiceInterface: Send + Sync {
    /// Get the service ID
    fn id(&self) -> ServiceId;
    
    /// Get the service name
    fn name(&self) -> &str;
    
    /// Get the service version
    fn version(&self) -> &str;
    
    /// Initialize the service
    fn initialize(&mut self) -> Result<(), crate::error::KernelError>;
    
    /// Start the service
    fn start(&mut self) -> Result<(), crate::error::KernelError>;
    
    /// Stop the service
    fn stop(&mut self) -> Result<(), crate::error::KernelError>;
    
    /// Cleanup the service
    fn cleanup(&mut self) -> Result<(), crate::error::KernelError>;
    
    /// Handle a service request
    fn handle_request(&mut self, request: &[u8]) -> Result<Vec<u8>, crate::error::KernelError>;
    
    /// Get the service state
    fn state(&self) -> ServiceState;
}

/// Service reference
#[derive(Debug, Clone)]
pub struct ServiceRef {
    pub info: ServiceInfo,
    pub interface: Arc<Mutex<dyn ServiceInterface>>,
}

impl ServiceRef {
    /// Create a new service reference
    pub fn new(info: ServiceInfo, interface: Arc<Mutex<dyn ServiceInterface>>) -> Self {
        Self { info, interface }
    }
    
    /// Get the service ID
    pub fn id(&self) -> ServiceId {
        self.info.id
    }
    
    /// Get the service name
    pub fn name(&self) -> &str {
        &self.info.name
    }
    
    /// Get the service version
    pub fn version(&self) -> &str {
        &self.info.version
    }
    
    /// Get the service type
    pub fn service_type(&self) -> ServiceType {
        self.info.service_type
    }
    
    /// Get the service priority
    pub fn priority(&self) -> ServicePriority {
        self.info.priority
    }
    
    /// Get the service state
    pub fn state(&self) -> ServiceState {
        self.info.state
    }
    
    /// Get the service dependencies
    pub fn dependencies(&self) -> &[ServiceDependency] {
        &self.info.dependencies
    }
    
    /// Get the service properties
    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.info.properties
    }
    
    /// Get the service interface
    pub fn interface(&self) -> &Arc<Mutex<dyn ServiceInterface>> {
        &self.interface
    }
}

/// Service event
#[derive(Debug, Clone)]
pub enum ServiceEvent {
    Registered(ServiceRef),
    Unregistered(ServiceId),
    StateChanged(ServiceId, ServiceState),
    PropertyUpdated(ServiceId, String, String),
}

/// Service listener
pub trait ServiceListener: Send + Sync {
    /// Handle a service event
    fn on_event(&self, event: ServiceEvent);
}