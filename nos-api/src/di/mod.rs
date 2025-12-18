//! Dependency Injection System
//! 
//! This module provides a dependency injection framework for the NOS operating system.
//! It allows for loose coupling between components and makes the system more testable
//! and maintainable.

#[cfg(feature = "alloc")]
use alloc::{
    string::{String, ToString},
    format,
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
    boxed::Box,
};

// ToString is not used in no-alloc mode
#[cfg(not(feature = "alloc"))]
use crate::interfaces::String;

#[cfg(not(feature = "alloc"))]
use crate::collections::BTreeMap;
#[cfg(not(feature = "alloc"))]
use crate::Vec;
#[cfg(not(feature = "alloc"))]
use crate::interfaces::Box;
#[cfg(not(feature = "alloc"))]
use crate::interfaces::Arc;

use core::any::{Any, TypeId};

use spin::{Mutex, RwLock};

use crate::error::Result;

/// Default container alias
pub type DefaultContainer = Container;

/// Dependency injection container
#[cfg(feature = "alloc")]
pub struct Container {
    /// Registered services
    services: RwLock<BTreeMap<TypeId, Box<dyn Any + Send + Sync>>>,
    /// Service factories
    factories: RwLock<BTreeMap<TypeId, Arc<dyn ServiceFactory>>>,
    /// Service metadata
    metadata: RwLock<BTreeMap<TypeId, ServiceMetadata>>,
    /// Service instances (for singletons)
    instances: RwLock<BTreeMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Configuration
    config: DIConfig,
    /// Resolution stack for circular dependency detection
    resolution_stack: Mutex<Vec<TypeId>>,
}

#[cfg(not(feature = "alloc"))]
pub struct Container {
    /// Registered services
    services: RwLock<BTreeMap<TypeId, &'static (dyn Any + Send + Sync)>>,
    /// Service factories
    factories: RwLock<BTreeMap<TypeId, &'static dyn ServiceFactory>>,
    /// Service metadata
    metadata: RwLock<BTreeMap<TypeId, ServiceMetadata>>,
    /// Service instances (for singletons)
    instances: RwLock<BTreeMap<TypeId, &'static (dyn Any + Send + Sync)>>,
    /// Configuration
    config: DIConfig,
    /// Resolution stack for circular dependency detection
    /// Note: In no-alloc mode, we can't track circular dependencies
    resolution_stack: Mutex<&'static [TypeId]>,
}



/// Service metadata
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service description
    pub description: String,
    /// Service dependencies
    pub dependencies: Vec<String>,
    /// Service scope
    pub scope: ServiceScope,
    /// Whether service is lazy initialized
    pub lazy: bool,
}

#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// Service name
    pub name: &'static str,
    /// Service version
    pub version: &'static str,
    /// Service description
    pub description: &'static str,
    /// Service dependencies
    pub dependencies: &'static [&'static str],
    /// Service scope
    pub scope: ServiceScope,
    /// Whether service is lazy initialized
    pub lazy: bool,
}

/// Service scope
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceScope {
    /// Singleton - only one instance exists
    Singleton,
    /// Transient - new instance created each time
    Transient,
    /// Scoped - one instance per scope
    Scoped(String),
}

/// Service factory trait
pub trait ServiceFactory: Send + Sync {
    /// Create a service instance
    fn create(&self, container: &Container) -> Result<Box<dyn Any + Send + Sync>>;
    
    /// Get service type ID
    fn type_id(&self) -> TypeId;
    
    /// Get service metadata
    fn metadata(&self) -> &ServiceMetadata;
}

/// Service resolver trait
pub trait ServiceResolver: Send + Sync {
    /// Resolve a service by type
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>>;
    
    /// Resolve a service by type ID
    fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>>;
    
    /// Check if a service is registered
    fn is_registered<T: 'static + Send + Sync>(&self) -> bool;
    
    /// Get service metadata
    fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata>;
}



/// Service lifetime manager
pub trait ServiceLifetime: Send + Sync {
    /// Create a service instance
    fn create(&self, container: &Container) -> Result<Box<dyn Any + Send + Sync>>;
    
    /// Dispose of a service instance
    fn dispose(&self, instance: Box<dyn Any + Send + Sync>) -> Result<()>;
    
    /// Get service scope
    fn scope(&self) -> ServiceScope;
}

/// Service registration options
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ServiceRegistrationOptions {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service description
    pub description: String,
    /// Service dependencies
    pub dependencies: Vec<String>,
    /// Service scope
    pub scope: ServiceScope,
    /// Whether service is lazy initialized
    pub lazy: bool,
}

#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ServiceRegistrationOptions {
    /// Service name
    pub name: &'static str,
    /// Service version
    pub version: &'static str,
    /// Service description
    pub description: &'static str,
    /// Service dependencies
    pub dependencies: &'static [&'static str],
    /// Service scope
    pub scope: ServiceScope,
    /// Whether service is lazy initialized
    pub lazy: bool,
}

#[cfg(feature = "alloc")]
impl Default for ServiceRegistrationOptions {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "1.0.0".to_string(),
            description: String::new(),
            dependencies: Vec::new(),
            scope: ServiceScope::Transient,
            lazy: false,
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for ServiceRegistrationOptions {
    fn default() -> Self {
        Self {
            name: "",
            version: "1.0.0",
            description: "",
            dependencies: &[],
            scope: ServiceScope::Transient,
            lazy: false,
        }
    }
}

/// Service registration builder
pub struct ServiceRegistrationBuilder {
    options: ServiceRegistrationOptions,
}

impl ServiceRegistrationBuilder {
    /// Create a new service registration builder
    pub fn new() -> Self {
        Self {
            options: ServiceRegistrationOptions::default(),
        }
    }
    
    /// Set service name
    #[cfg(feature = "alloc")]
    pub fn name(mut self, name: String) -> Self {
        self.options.name = name;
        self
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn name(mut self, name: &'static str) -> Self {
        self.options.name = name;
        self
    }
    
    /// Set service version
    #[cfg(feature = "alloc")]
    pub fn version(mut self, version: String) -> Self {
        self.options.version = version;
        self
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn version(mut self, version: &'static str) -> Self {
        self.options.version = version;
        self
    }
    
    /// Set service description
    #[cfg(feature = "alloc")]
    pub fn description(mut self, description: String) -> Self {
        self.options.description = description;
        self
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn description(mut self, description: &'static str) -> Self {
        self.options.description = description;
        self
    }
    
    /// Add a dependency
    #[cfg(feature = "alloc")]
    pub fn depends_on(mut self, dependency: String) -> Self {
        self.options.dependencies.push(dependency);
        self
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn depends_on(self, _dependency: &'static str) -> Self {
        // In no-alloc mode, we can't modify the static slice
        self
    }
    
    /// Set service scope
    pub fn scope(mut self, scope: ServiceScope) -> Self {
        self.options.scope = scope;
        self
    }
    
    /// Set lazy initialization
    pub fn lazy(mut self, lazy: bool) -> Self {
        self.options.lazy = lazy;
        self
    }
    
    /// Build service registration options
    pub fn build(self) -> ServiceRegistrationOptions {
        self.options
    }
}

impl Default for ServiceRegistrationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Service locator for resolving dependencies
pub struct ServiceLocator {
    container: Arc<Container>,
}

impl ServiceLocator {
    /// Create a new service locator
    pub fn new(container: Arc<Container>) -> Self {
        Self { container }
    }
    
    /// Get the underlying container
    pub fn container(&self) -> &Container {
        &self.container
    }
    
    /// Resolve a service by type
    pub fn get<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        use crate::di::ServiceResolver;
        self.container.resolve()
    }
    
    /// Try to resolve a service by type
    pub fn try_get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        use crate::di::ServiceResolver;
        self.container.resolve().ok()
    }
}

/// Dependency injection attribute macro
#[macro_export]
macro_rules! inject {
    ($field:ident: $type:ty) => {
        $field: Option<Arc<$type>>
    };
}

/// Dependency injection constructor macro
#[macro_export]
macro_rules! injectable {
    ($struct_name:ident { $($field:ident: $type:ty),* }) => {
        impl $struct_name {
            pub fn new(container: &crate::di::Container) -> Result<Self> {
                Ok(Self {
                    $($field: Some(container.resolve()?),)*
                })
            }
        }
    };
}

/// Module for dependency injection
pub mod module {
    use super::*;
    
    /// Service registration
    #[derive(Clone)]
    pub struct ServiceRegistration {
        /// Service type ID
        pub type_id: TypeId,
        /// Service factory
        pub factory: Arc<dyn ServiceFactory>,
        /// Service metadata
        pub metadata: ServiceMetadata,
    }
    
    /// Dependency injection module
    #[cfg(feature = "alloc")]
    pub struct Module {
        /// Module name
        pub name: String,
        /// Service registrations
        pub services: Vec<ServiceRegistration>,
        /// Module dependencies
        pub dependencies: Vec<String>,
    }
    
    /// Dependency injection module (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub struct Module {
        /// Module name
        pub name: &'static str,
        /// Service registrations
        pub services: &'static [ServiceRegistration],
        /// Module dependencies
        pub dependencies: &'static [&'static str],
    }
    
    #[cfg(feature = "alloc")]
    impl Module {
        /// Create a new module
        pub fn new(name: String) -> Self {
            Self {
                name,
                services: Vec::new(),
                dependencies: Vec::new(),
            }
        }
        
        /// Register a service
        pub fn register_service<T: 'static + Send + Sync>(
            mut self,
            factory: Box<dyn ServiceFactory>,
        ) -> Self {
            let registration = ServiceRegistration {
                type_id: TypeId::of::<T>(),
                factory: Arc::from(factory),
                metadata: ServiceMetadata {
                    name: core::any::type_name::<T>().to_string(),
                    version: "1.0.0".to_string(),
                    description: "".to_string(),
                    dependencies: Vec::new(),
                    scope: ServiceScope::Transient,
                    lazy: false,
                },
            };
            
            self.services.push(registration);
            self
        }
        
        /// Add a module dependency
        pub fn depends_on(mut self, dependency: String) -> Self {
            self.dependencies.push(dependency);
            self
        }
    }
    
    #[cfg(not(feature = "alloc"))]
    impl Module {
        /// Create a new module
        pub fn new(name: &'static str) -> Self {
            Self {
                name,
                services: &[],
                dependencies: &[],
            }
        }
        
        /// Register a service (no-op in no-alloc mode)
        pub fn register_service<T: 'static + Send + Sync>(
            self,
            _factory: Box<dyn ServiceFactory>,
        ) -> Self {
            // In no-alloc mode, we can't modify the static slice
            self
        }
        
        /// Add a module dependency (no-op in no-alloc mode)
        pub fn depends_on(self, _dependency: &'static str) -> Self {
            // In no-alloc mode, we can't modify the static slice
            self
        }
    }
}

/// Configuration for dependency injection
#[derive(Debug, Clone)]
pub struct DIConfig {
    /// Whether to enable circular dependency detection
    pub enable_circular_dependency_detection: bool,
    /// Whether to enable service validation
    pub enable_service_validation: bool,
    /// Maximum depth for dependency resolution
    pub max_resolution_depth: usize,
}

impl Default for DIConfig {
    fn default() -> Self {
        Self {
            enable_circular_dependency_detection: true,
            enable_service_validation: true,
            max_resolution_depth: 100,
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    /// Create a new container with default configuration
    pub fn new() -> Self {
        Self::with_config(DIConfig::default())
    }
    
    /// Create a new container with custom configuration
    pub fn with_config(config: DIConfig) -> Self {
        #[cfg(feature = "alloc")] {
            Self {
                services: RwLock::new(BTreeMap::new()),
                factories: RwLock::new(BTreeMap::new()),
                metadata: RwLock::new(BTreeMap::new()),
                instances: RwLock::new(BTreeMap::new()),
                config,
                resolution_stack: Mutex::new(Vec::new()),
            }
        }
        #[cfg(not(feature = "alloc"))] {
            Self {
                services: RwLock::new(BTreeMap::new()),
                factories: RwLock::new(BTreeMap::new()),
                metadata: RwLock::new(BTreeMap::new()),
                instances: RwLock::new(BTreeMap::new()),
                config,
                resolution_stack: Mutex::new(&[]),
            }
        }
    }
    
    /// Register a service instance
    pub fn register_instance<T: 'static + Send + Sync>(&self, instance: Arc<T>) -> Result<()> {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "alloc")] {
            self.instances.write().insert(type_id, instance);
        }
        #[cfg(not(feature = "alloc"))] {
            // In no-alloc mode, we need to convert Arc<T> to &'static (dyn Any + Send + Sync)
            let instance_ptr = instance.as_ptr() as *const (dyn Any + Send + Sync);
            // Safe because we're leaking the Arc to make it static
            let static_instance = unsafe {
                core::mem::transmute::<*const (dyn Any + Send + Sync), &'static (dyn Any + Send + Sync)>(instance_ptr)
            };
            self.instances.write().insert(type_id, static_instance);
        }
        Ok(())
    }
    
    /// Register a service factory
    pub fn register_factory<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "alloc")] {
            self.factories.write().insert(type_id, Arc::from(factory));
        }
        #[cfg(not(feature = "alloc"))] {
            // In no-alloc mode, convert Box to static reference
            let factory_ptr = factory.as_ptr() as *const dyn ServiceFactory;
            // Safe because we're leaking the Box to make it static
            let static_factory = unsafe {
                core::mem::transmute::<*const dyn ServiceFactory, &'static dyn ServiceFactory>(factory_ptr)
            };
            self.factories.write().insert(type_id, static_factory);
        }
        Ok(())
    }
    
    /// Register a service with options
    pub fn register_with_options<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
        options: ServiceRegistrationOptions,
    ) -> Result<()> {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "alloc")] {
            self.factories.write().insert(type_id, Arc::from(factory));
        }
        #[cfg(not(feature = "alloc"))] {
            // In no-alloc mode, convert Box to static reference
            let factory_ptr = factory.as_ptr() as *const dyn ServiceFactory;
            // Safe because we're leaking the Box to make it static
            let static_factory = unsafe {
                core::mem::transmute::<*const dyn ServiceFactory, &'static dyn ServiceFactory>(factory_ptr)
            };
            self.factories.write().insert(type_id, static_factory);
        }
        self.metadata.write().insert(type_id, ServiceMetadata {
            name: options.name,
            version: options.version,
            description: options.description,
            dependencies: options.dependencies,
            scope: options.scope,
            lazy: options.lazy,
        });
        Ok(())
    }


    
    /// Resolve a service by type
    pub fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        let type_id = TypeId::of::<T>();
        
        // Check for circular dependencies
        if self.config.enable_circular_dependency_detection {
            #[cfg(feature = "alloc")] {
                let mut stack = self.resolution_stack.lock();
                if stack.contains(&type_id) {
                    return Err(crate::error::Error::CircularDependency(
                        format!("Circular dependency detected for type: {:?}", type_id)
                    ));
                }
                stack.push(type_id);
            }
            #[cfg(not(feature = "alloc"))] {
                // In no-alloc mode, we can't track circular dependencies
            }
        }
        
        // Check if we already have an instance (for singletons)
        // Note: we use read lock first to check
        if let Some(instance) = self.instances.read().get(&type_id) {
            if self.config.enable_circular_dependency_detection {
                #[cfg(feature = "alloc")] {
                    let mut stack = self.resolution_stack.lock();
                    stack.pop();
                }
                #[cfg(not(feature = "alloc"))] {
                    // In no-alloc mode, we can't pop from a static slice
                }
            }
            #[cfg(feature = "alloc")]
            return instance.clone().downcast::<T>()
                .map_err(|_| crate::error::Error::ServiceError(
                    "Failed to downcast service instance".to_string()
                ));
            #[cfg(not(feature = "alloc"))] {
                // In no-alloc mode, instance is &'static (dyn Any + ...)
                // We need to convert it to &'static T first
                if TypeId::of::<T>() == instance.type_id() {
                    // Safe because we checked the type ID
                    let typed_ref = unsafe {
                        &*(instance as *const dyn Any as *const T)
                    };
                    return Ok(Arc::new(typed_ref));
                } else {
                    return Err(crate::error::Error::ServiceError(
                        "Failed to downcast service instance"
                    ));
                }
            }
        }
        
        // Create a new instance using factory
        // Note: we might need write lock if we create singleton
        // But we first check factories with read lock
        let factory_opt = self.factories.read().get(&type_id).cloned();
        
        if let Some(factory) = factory_opt {
            // Check scope
            let is_singleton = if let Some(metadata) = self.metadata.read().get(&type_id) {
                matches!(metadata.scope, ServiceScope::Singleton)
            } else {
                false
            };

            // We can't hold factory read lock while calling create() because create() might recursively call resolve() 
            // which might need to access factories again. RwLock allows multiple readers, so this is fine for recursive reads.
            // BUT if create() tries to register a new service (unlikely but possible), it would need write lock, causing deadlock.
            // Assuming create() only resolves other services.
            
            let instance = factory.create(self)?;
            let arc_instance: Arc<dyn Any + Send + Sync> = instance.into();
            
            #[cfg(feature = "alloc")]
            let typed_instance = arc_instance.clone().downcast::<T>()
                .map_err(|_| crate::error::Error::ServiceError(
                    "Failed to downcast service instance".to_string()
                ))?;
            #[cfg(not(feature = "alloc"))]
            let typed_instance = arc_instance.clone().downcast::<T>()
                .map_err(|_| crate::error::Error::ServiceError(
                    "Failed to downcast service instance"
                ))?;
            
            // Store instance if it's a singleton
            if is_singleton {
                #[cfg(feature = "alloc")] {
                    self.instances.write().insert(type_id, arc_instance);
                }
                #[cfg(not(feature = "alloc"))] {
                    // In no-alloc mode, we store the static reference directly
                    // This is safe because arc_instance contains a static reference
                    let static_ref = unsafe {
                    core::mem::transmute::<*const (dyn Any + Send + Sync), &'static (dyn Any + Send + Sync)>(arc_instance.as_ptr())
                };
                    self.instances.write().insert(type_id, static_ref);
                }
            }
            
            if self.config.enable_circular_dependency_detection {
                #[cfg(feature = "alloc")] {
                    let mut stack = self.resolution_stack.lock();
                    stack.pop();
                }
                #[cfg(not(feature = "alloc"))] {
                    // In no-alloc mode, we can't track circular dependencies
                }
            }
            
            Ok(typed_instance)
        } else {

            if self.config.enable_circular_dependency_detection {
                #[cfg(feature = "alloc")] {
                    let mut stack = self.resolution_stack.lock();
                    stack.pop();
                }
                #[cfg(not(feature = "alloc"))] {
                    // In no-alloc mode, we can't track circular dependencies
                }
            }
            #[cfg(feature = "alloc")]
            return Err(crate::error::Error::ServiceError(
                format!("Service not registered for type: {:?}", type_id)
            ));
            #[cfg(not(feature = "alloc"))]
            return Err(crate::error::Error::ServiceError(
                "Service not registered"
            ));
        }
    }
    
    /// Get service metadata
    pub fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
        let type_id = TypeId::of::<T>();
        self.metadata.read().get(&type_id).cloned()
    }
    
    /// Check if a service is registered
    pub fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.factories.read().contains_key(&type_id)
    }
    
    /// Get all registered services
    pub fn get_registered_services(&self) -> Vec<ServiceMetadata> {
        #[cfg(feature = "alloc")]
        return self.metadata.read().values().cloned().collect();
        #[cfg(not(feature = "alloc"))]
        {
            // In no-alloc mode, we can't collect into a Vec
            &[]
        }
    }
    
    /// Validate all dependencies
    pub fn validate_dependencies(&self) -> Result<()> {
        if !self.config.enable_service_validation {
            return Ok(());
        }
        
        let metadata_map = self.metadata.read();
        
        for (_type_id, metadata) in metadata_map.iter() {
            #[cfg(feature = "alloc")] {
                for dependency in &metadata.dependencies {
                    let mut found = false;
                    for (_, dep_metadata) in metadata_map.iter() {
                        if dep_metadata.name == *dependency {
                            found = true;
                            break;
                        }
                    }
                    
                    if !found {
                        return Err(crate::error::Error::ServiceError(
                            format!("Dependency '{}' not found for service '{}'", 
                                    dependency, metadata.name)
                        ));
                    }
                }
            }
            #[cfg(not(feature = "alloc"))] {
                for dependency in metadata.dependencies {
                    let mut found = false;
                    for (_, dep_metadata) in metadata_map.iter() {
                        if dep_metadata.name == *dependency {
                            found = true;
                            break;
                        }
                    }
                    
                    if !found {
                        return Err(crate::error::Error::ServiceError(
                            "Dependency not found"
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }

}

impl ServiceResolver for Arc<Container> {
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        // Just call container's resolve since Arc<Container> derefs to Container
        (**self).resolve()
    }

    
    fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        // Check if we already have an instance
        if let Some(instance) = self.instances.read().get(&type_id) {
            #[cfg(feature = "alloc")]
            return Ok(instance.clone());
            #[cfg(not(feature = "alloc"))]
            {
                // In no-alloc mode, we need to convert &'static T to Arc<T>
                // This is safe because we know the reference is static
                let instance_ptr = instance as *const (dyn Any + Send + Sync);
                unsafe {
                    // Create a new Arc that holds the static reference
                    let arc_instance = Arc::new(&*instance_ptr);
                    return Ok(arc_instance);
                }
            }
        }
        
        // Create a new instance using factory
        let factory_opt = self.factories.read().get(&type_id).cloned();
        if let Some(factory) = factory_opt {
            let instance = factory.create(self)?;
            #[cfg(feature = "alloc")]
            {
                let arc_instance: Arc<dyn Any + Send + Sync> = instance.into();
                Ok(arc_instance)
            }
            #[cfg(not(feature = "alloc"))]
            {
                // In no-alloc mode, we need to convert Box to Arc
                let arc_instance: Arc<dyn Any + Send + Sync> = instance.into();
                Ok(arc_instance)
            }
        } else {
            #[cfg(feature = "alloc")]
            return Err(crate::error::Error::ServiceError(
                format!("Service not registered for type: {:?}", type_id)
            ));
            #[cfg(not(feature = "alloc"))]
            return Err(crate::error::Error::ServiceError(
                "Service not registered"
            ));
        }
    }
    
    fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.factories.read().contains_key(&type_id)
    }
    
    fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
        let type_id = TypeId::of::<T>();
        self.metadata.read().get(&type_id).cloned()
    }
}
