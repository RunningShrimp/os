//! Service interface traits

use crate::error::Result;
use crate::core::traits::Service;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};


// ToString and String imports are not needed in this module

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// Trait for service registry
pub trait ServiceRegistry {
    /// Registers a service
    #[cfg(feature = "alloc")]
    fn register(&mut self, service: Box<dyn Service>) -> Result<()>;
    
    #[cfg(not(feature = "alloc"))]
    fn register(&mut self, service: crate::interfaces::Box<dyn Service>) -> Result<()>;
    
    /// Unregisters a service by name
    fn unregister(&mut self, name: &str) -> Result<()>;
    
    /// Finds a service by name
    fn find(&self, name: &str) -> Option<&dyn Service>;
    
    /// Finds a mutable service by name
    fn find_mut(&mut self, name: &str) -> Option<&mut dyn Service>;
    
    /// Lists all registered services
    #[cfg(feature = "alloc")]
    fn list(&self) -> Vec<&str>;
    
    #[cfg(not(feature = "alloc"))]
    fn list(&self) -> crate::interfaces::Vec<&str>;
    
    /// Returns the number of registered services
    fn count(&self) -> usize;
    
    /// Checks if a service is registered
    fn contains(&self, name: &str) -> bool;
}

/// Trait for service discovery
pub trait ServiceDiscovery {
    /// Discovers services by type
    #[cfg(feature = "alloc")]
    fn discover_by_type(&self, service_type: &str) -> Vec<&dyn Service>;
    
    #[cfg(not(feature = "alloc"))]
    fn discover_by_type(&self, service_type: &str) -> crate::interfaces::Vec<&dyn Service>;
    
    /// Discovers services by interface
    #[cfg(feature = "alloc")]
    fn discover_by_interface(&self, interface: &str) -> Vec<&dyn Service>;
    
    #[cfg(not(feature = "alloc"))]
    fn discover_by_interface(&self, interface: &str) -> crate::interfaces::Vec<&dyn Service>;
    
    /// Discovers services by capability
    #[cfg(feature = "alloc")]
    fn discover_by_capability(&self, capability: &str) -> Vec<&dyn Service>;
    
    #[cfg(not(feature = "alloc"))]
    fn discover_by_capability(&self, capability: &str) -> crate::interfaces::Vec<&dyn Service>;
    
    /// Lists all discoverable services
    #[cfg(feature = "alloc")]
    fn list_all(&self) -> Vec<&dyn Service>;
    
    #[cfg(not(feature = "alloc"))]
    fn list_all(&self) -> crate::interfaces::Vec<&dyn Service>;
    
    /// Checks if a service is discoverable
    fn is_discoverable(&self, name: &str) -> bool;
}

/// Trait for service communication
pub trait ServiceCommunication {
    /// Message type
    type Message;
    
    /// Sends a message to a service
    fn send(&mut self, service_name: &str, message: Self::Message) -> Result<()>;
    
    /// Receives a message from a service
    fn receive(&mut self, service_name: &str) -> Result<Option<Self::Message>>;
    
    /// Broadcasts a message to all services
    fn broadcast(&mut self, message: Self::Message) -> Result<()>;
    
    /// Subscribes to messages from a service
    fn subscribe(&mut self, service_name: &str) -> Result<()>;
    
    /// Unsubscribes from messages from a service
    fn unsubscribe(&mut self, service_name: &str) -> Result<()>;
}

/// Trait for service lifecycle management
pub trait ServiceLifecycle {
    /// Starts a service
    fn start(&mut self, name: &str) -> Result<()>;
    
    /// Stops a service
    fn stop(&mut self, name: &str) -> Result<()>;
    
    /// Restarts a service
    fn restart(&mut self, name: &str) -> Result<()>;
    
    /// Returns the status of a service
    fn status(&self, name: &str) -> ServiceStatus;
    
    /// Lists all running services
    #[cfg(feature = "alloc")]
    fn list_running(&self) -> Vec<&str>;
    
    #[cfg(not(feature = "alloc"))]
    fn list_running(&self) -> crate::interfaces::Vec<&str>;
    
    /// Lists all stopped services
    #[cfg(feature = "alloc")]
    fn list_stopped(&self) -> Vec<&str>;
    
    #[cfg(not(feature = "alloc"))]
    fn list_stopped(&self) -> crate::interfaces::Vec<&str>;
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is not initialized
    Uninitialized,
    /// Service is initializing
    Initializing,
    /// Service is running
    Running,
    /// Service is stopping
    Stopping,
    /// Service is stopped
    Stopped,
    /// Service has failed
    Failed,
    /// Service is in maintenance mode
    Maintenance,
}

/// Service priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServicePriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Service dependency
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ServiceDependency {
    /// Name of the dependent service
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Whether the dependency is optional
    pub optional: bool,
}

#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ServiceDependency {
    /// Name of the dependent service
    pub name: &'static str,
    /// Version requirement
    pub version: &'static str,
    /// Whether the dependency is optional
    pub optional: bool,
}

#[cfg(feature = "alloc")]
impl ServiceDependency {
    /// Creates a new service dependency
    pub fn new(name: &str, version: &str, optional: bool) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            optional,
        }
    }
    
    /// Creates a required dependency
    pub fn required(name: &str, version: &str) -> Self {
        Self::new(name, version, false)
    }
    
    /// Creates an optional dependency
    pub fn optional(name: &str, version: &str) -> Self {
        Self::new(name, version, true)
    }
}

#[cfg(not(feature = "alloc"))]
impl ServiceDependency {
    /// Creates a new service dependency
    pub fn new(name: &'static str, version: &'static str, optional: bool) -> Self {
        Self {
            name,
            version,
            optional,
        }
    }
    
    /// Creates a required dependency
    pub fn required(name: &'static str, version: &'static str) -> Self {
        Self::new(name, version, false)
    }
    
    /// Creates an optional dependency
    pub fn optional(name: &'static str, version: &'static str) -> Self {
        Self::new(name, version, true)
    }
}

/// Service metadata
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// Name of the service
    pub name: String,
    /// Version of the service
    pub version: String,
    /// Description of the service
    pub description: String,
    /// Author of the service
    pub author: String,
    /// License of the service
    pub license: String,
    /// Priority of the service
    pub priority: ServicePriority,
    /// Dependencies of the service
    pub dependencies: Vec<ServiceDependency>,
    /// Capabilities provided by the service
    pub capabilities: Vec<String>,
    /// Interfaces implemented by the service
    pub interfaces: Vec<String>,
}

#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// Name of the service
    pub name: &'static str,
    /// Version of the service
    pub version: &'static str,
    /// Description of the service
    pub description: &'static str,
    /// Author of the service
    pub author: &'static str,
    /// License of the service
    pub license: &'static str,
    /// Priority of the service
    pub priority: ServicePriority,
    /// Dependencies of the service
    pub dependencies: &'static [ServiceDependency],
    /// Capabilities provided by the service
    pub capabilities: &'static [&'static str],
    /// Interfaces implemented by the service
    pub interfaces: &'static [&'static str],
}

#[cfg(feature = "alloc")]
impl ServiceMetadata {
    /// Creates new service metadata
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: String::new(),
            author: String::new(),
            license: String::new(),
            priority: ServicePriority::Normal,
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            interfaces: Vec::new(),
        }
    }
    
    /// Sets the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }
    
    /// Sets the author
    pub fn with_author(mut self, author: &str) -> Self {
        self.author = author.to_string();
        self
    }
    
    /// Sets the license
    pub fn with_license(mut self, license: &str) -> Self {
        self.license = license.to_string();
        self
    }
    
    /// Sets the priority
    pub fn with_priority(mut self, priority: ServicePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Adds a dependency
    pub fn with_dependency(mut self, dependency: ServiceDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }
    
    /// Adds a capability
    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capabilities.push(capability.to_string());
        self
    }
    
    /// Adds an interface
    pub fn with_interface(mut self, interface: &str) -> Self {
        self.interfaces.push(interface.to_string());
        self
    }
}

#[cfg(not(feature = "alloc"))]
impl ServiceMetadata {
    /// Creates new service metadata
    pub fn new(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            description: "",
            author: "",
            license: "",
            priority: ServicePriority::Normal,
            dependencies: &[],
            capabilities: &[],
            interfaces: &[],
        }
    }
    
    /// Sets the description
    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }
    
    /// Sets the author
    pub fn with_author(mut self, author: &'static str) -> Self {
        self.author = author;
        self
    }
    
    /// Sets the license
    pub fn with_license(mut self, license: &'static str) -> Self {
        self.license = license;
        self
    }
    
    /// Sets the priority
    pub fn with_priority(mut self, priority: ServicePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Adds a dependency (not available in no-alloc mode)
    pub fn with_dependency(self, _dependency: ServiceDependency) -> Self {
        // In no-alloc mode, dependencies must be set at creation time
        self
    }
    
    /// Adds a capability (not available in no-alloc mode)
    pub fn with_capability(self, _capability: &'static str) -> Self {
        // In no-alloc mode, capabilities must be set at creation time
        self
    }
    
    /// Adds an interface (not available in no-alloc mode)
    pub fn with_interface(self, _interface: &'static str) -> Self {
        // In no-alloc mode, interfaces must be set at creation time
        self
    }
}