//! Dependency Injection Container Implementation
//! 
//! This module provides a concrete implementation of the dependency injection container.

#[cfg(feature = "alloc")]
use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Weak},
    vec::Vec,
    string::String,
    boxed::Box,
    format,
};
#[cfg(not(feature = "alloc"))]

use core::any::{Any, TypeId};
use core::cell::RefCell;

use crate::{
    error::Result,
    di::{
        Container, ServiceFactory, ServiceMetadata, ServiceScope, ServiceLifetime,
        ServiceRegistrationOptions, ServiceResolver, DIConfig,
    },
};

/// Default implementation of dependency injection container
pub struct DefaultContainer {
    /// Registered services
    services: RefCell<BTreeMap<TypeId, Box<dyn Any + Send + Sync>>>,
    /// Service factories
    factories: RefCell<BTreeMap<TypeId, Box<dyn ServiceFactory>>>,
    /// Service metadata
    metadata: RefCell<BTreeMap<TypeId, ServiceMetadata>>,
    /// Service instances for singleton scope
    singletons: RefCell<BTreeMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Service instances for scoped scope
    scoped: RefCell<BTreeMap<String, BTreeMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
    /// Resolution stack for circular dependency detection
    resolution_stack: RefCell<VecDeque<TypeId>>,
    /// Configuration
    config: DIConfig,
}

impl DefaultContainer {
    /// Create a new container with default configuration
    pub fn new() -> Self {
        Self::with_config(DIConfig::default())
    }
    
    /// Create a new container with custom configuration
    pub fn with_config(config: DIConfig) -> Self {
        Self {
            services: RefCell::new(BTreeMap::new()),
            factories: RefCell::new(BTreeMap::new()),
            metadata: RefCell::new(BTreeMap::new()),
            singletons: RefCell::new(BTreeMap::new()),
            scoped: RefCell::new(BTreeMap::new()),
            resolution_stack: RefCell::new(VecDeque::new()),
            config,
        }
    }
    
    /// Register a service with options
    pub fn register_with_options<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
        options: ServiceRegistrationOptions,
    ) -> Result<()> {
        let type_id = TypeId::of::<T>();
        
        // Check for existing registration
        if self.config.enable_service_validation {
            if self.services.borrow().contains_key(&type_id) {
                return Err(crate::error::Error::DIError(
                    format!("Service {} is already registered", options.name)
                ));
            }
        }
        
        // Store factory and metadata
        self.factories.borrow_mut().insert(type_id, factory);
        self.metadata.borrow_mut().insert(type_id, ServiceMetadata {
            name: options.name,
            version: options.version,
            description: options.description,
            dependencies: options.dependencies,
            scope: options.scope.clone(),
            lazy: options.lazy,
        });
        
        // Pre-create singleton if not lazy
        if !options.lazy && matches!(options.scope, ServiceScope::Singleton) {
            let instance = factory.create(self)?;
            self.singletons.borrow_mut().insert(type_id, instance.into());
        }
        
        Ok(())
    }
    
    /// Register a transient service
    pub fn register_transient<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        self.register_with_options::<T>(
            factory,
            ServiceRegistrationOptions {
                name: core::any::type_name::<T>().to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                dependencies: Vec::new(),
                scope: ServiceScope::Transient,
                lazy: false,
            },
        )
    }
    
    /// Register a singleton service
    pub fn register_singleton<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        self.register_with_options::<T>(
            factory,
            ServiceRegistrationOptions {
                name: core::any::type_name::<T>().to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                dependencies: Vec::new(),
                scope: ServiceScope::Singleton,
                lazy: false,
            },
        )
    }
    
    /// Register a scoped service
    pub fn register_scoped<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
        scope_name: String,
    ) -> Result<()> {
        self.register_with_options::<T>(
            factory,
            ServiceRegistrationOptions {
                name: core::any::type_name::<T>().to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                dependencies: Vec::new(),
                scope: ServiceScope::Scoped(scope_name),
                lazy: false,
            },
        )
    }
    
    /// Check for circular dependencies
    fn check_circular_dependency(&self, type_id: TypeId) -> Result<()> {
        if !self.config.enable_circular_dependency_detection {
            return Ok(());
        }
        
        let stack = self.resolution_stack.borrow();
        if stack.contains(&type_id) {
            let mut cycle = Vec::new();
            for id in stack.iter() {
                if let Some(metadata) = self.metadata.borrow().get(id) {
                    cycle.push(metadata.name.clone());
                }
            }
            cycle.push(format!("Cycle detected: {}", cycle.join(" -> ")));
            return Err(crate::error::Error::DIError(
                format!("Circular dependency detected: {}", cycle.join(" -> "))
            ));
        }
        
        Ok(())
    }
    
    /// Create a service instance
    fn create_instance(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        let factory = self.factories.borrow()
            .get(&type_id)
            .ok_or_else(|| crate::error::Error::DIError(
                format!("Service not registered: {:?}", type_id)
            ))?;
        
        factory.create(self)
    }
    
    /// Get or create a singleton instance
    fn get_singleton(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        let mut singletons = self.singletons.borrow_mut();
        
        if let Some(instance) = singletons.get(&type_id) {
            Ok(instance.clone())
        } else {
            let instance = self.create_instance(type_id)?;
            singletons.insert(type_id, instance.clone());
            Ok(instance)
        }
    }
    
    /// Get or create a scoped instance
    fn get_scoped(&self, type_id: TypeId, scope_name: &str) -> Result<Arc<dyn Any + Send + Sync>> {
        let mut scoped = self.scoped.borrow_mut();
        
        if let Some(scope_instances) = scoped.get_mut(scope_name) {
            if let Some(instance) = scope_instances.get(&type_id) {
                return Ok(instance.clone());
            }
        } else {
            let instance = self.create_instance(type_id)?;
            scope_instances.insert(type_id, instance.clone());
            return Ok(instance);
            }
        }
        
        // Create new scope
        let instance = self.create_instance(type_id)?;
        let mut scope_instances = BTreeMap::new();
        scope_instances.insert(type_id, instance.clone());
        scoped.insert(scope_name.to_string(), scope_instances);
        Ok(instance)
    }
    
    /// Clear a scope
    pub fn clear_scope(&self, scope_name: &str) {
        let mut scoped = self.scoped.borrow_mut();
        scoped.remove(scope_name);
    }
    
    /// Get registered services count
    pub fn service_count(&self) -> usize {
        self.factories.borrow().len()
    }
    
    /// Get service metadata
    pub fn get_service_metadata(&self, type_id: TypeId) -> Option<ServiceMetadata> {
        self.metadata.borrow().get(&type_id).cloned()
    }
    
    /// List all registered services
    pub fn list_services(&self) -> Vec<String> {
        self.metadata.borrow()
            .values()
            .map(|m| m.name.clone())
            .collect()
    }
}

impl Container for DefaultContainer {
    fn register<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        self.register_transient::<T>(factory)
    }
    
    fn register_with_options<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
        options: ServiceRegistrationOptions,
    ) -> Result<()> {
        self.register_with_options::<T>(factory, options)
    }
    
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        let type_id = TypeId::of::<T>();
        
        // Check for circular dependencies
        self.check_circular_dependency(type_id)?;
        
        // Add to resolution stack
        self.resolution_stack.borrow_mut().push_back(type_id);
        
        let result = match self.get_service_metadata(type_id) {
            Some(metadata) => {
                match metadata.scope {
                    ServiceScope::Singleton => {
                        let instance = self.get_singleton(type_id)?;
                        Ok(instance.downcast::<T>().unwrap())
                    }
                    ServiceScope::Transient => {
                        let instance = self.create_instance(type_id)?;
                        Ok(instance.downcast::<T>().unwrap())
                    }
                    ServiceScope::Scoped(ref scope_name) => {
                        let instance = self.get_scoped(type_id, scope_name)?;
                        Ok(instance.downcast::<T>().unwrap())
                    }
                }
            }
            None => Err(crate::error::Error::DIError(
                format!("Service not registered: {:?}", type_id)
            )),
        };
        
        // Remove from resolution stack
        self.resolution_stack.borrow_mut().pop_back();
        
        result
    }
    
    fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        // Check for circular dependencies
        self.check_circular_dependency(type_id)?;
        
        // Add to resolution stack
        self.resolution_stack.borrow_mut().push_back(type_id);
        
        let result = match self.get_service_metadata(type_id) {
            Some(metadata) => {
                match metadata.scope {
                    ServiceScope::Singleton => self.get_singleton(type_id),
                    ServiceScope::Transient => self.create_instance(type_id),
                    ServiceScope::Scoped(ref scope_name) => self.get_scoped(type_id, scope_name),
                }
            }
            None => Err(crate::error::Error::DIError(
                format!("Service not registered: {:?}", type_id)
            )),
        };
        
        // Remove from resolution stack
        self.resolution_stack.borrow_mut().pop_back();
        
        result
    }
    
    fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.factories.borrow().contains_key(&type_id)
    }
    
    fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
        let type_id = TypeId::of::<T>();
        self.metadata.borrow().get(&type_id).cloned()
    }
}

impl ServiceResolver for DefaultContainer {
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        Container::resolve(self)
    }
    
    fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        Container::resolve_by_id(self, type_id)
    }
    
    fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
        Container::is_registered(self)
    }
    
    fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
        Container::get_metadata(self)
    }
}

impl ServiceResolver for Arc<dyn Container> {
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        Container::resolve(self.as_ref())
    }
    
    fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        Container::resolve_by_id(self.as_ref(), type_id)
    }
    
    fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
        Container::is_registered(self.as_ref())
    }
    
    fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
        Container::get_metadata(self.as_ref())
    }
}

/// Simple service factory implementation
pub struct SimpleServiceFactory<T: 'static + Send + Sync> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T: 'static + Send + Sync> SimpleServiceFactory<T> {
    /// Create a new simple service factory
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T: 'static + Send + Sync + Default> ServiceFactory for SimpleServiceFactory<T> {
    fn create(&self, _container: &Container) -> Result<Box<dyn Any + Send + Sync>> {
        Ok(Box::new(T::default()))
    }
    
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn metadata(&self) -> &ServiceMetadata {
        static METADATA: ServiceMetadata = ServiceMetadata {
            name: core::any::type_name::<T>(),
            version: "1.0.0",
            description: "Simple service factory",
            dependencies: Vec::new(),
            scope: ServiceScope::Transient,
            lazy: false,
        };
        &METADATA
    }
}

/// Function-based service factory
pub struct FnServiceFactory<T: 'static + Send + Sync> {
    factory_fn: fn(&Container) -> Result<T>,
    type_id: TypeId,
    metadata: ServiceMetadata,
}

impl<T: 'static + Send + Sync> FnServiceFactory<T> {
    /// Create a new function-based service factory
    pub fn new(
        factory_fn: fn(&Container) -> Result<T>,
        metadata: ServiceMetadata,
    ) -> Self {
        Self {
            factory_fn,
            type_id: TypeId::of::<T>(),
            metadata,
        }
    }
}

impl<T: 'static + Send + Sync> ServiceFactory for FnServiceFactory<T> {
    fn create(&self, container: &Container) -> Result<Box<dyn Any + Send + Sync>> {
        let instance = (self.factory_fn)(container)?;
        Ok(Box::new(instance))
    }
    
    fn type_id(&self) -> TypeId {
        self.type_id
    }
    
    fn metadata(&self) -> &ServiceMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    
    #[derive(Debug, Default)]
    struct TestService {
        value: i32,
    }
    
    #[test]
    fn test_container_registration() {
        let container = DefaultContainer::new();
        let factory = SimpleServiceFactory::<TestService>::new();
        
        assert!(container.register::<TestService>(Box::new(factory)).is_ok());
        assert!(container.is_registered::<TestService>());
    }
    
    #[test]
    fn test_service_resolution() {
        let container = DefaultContainer::new();
        let factory = SimpleServiceFactory::<TestService>::new();
        
        container.register::<TestService>(Box::new(factory)).unwrap();
        
        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 0);
    }
    
    #[test]
    fn test_singleton_scope() {
        let container = DefaultContainer::new();
        let factory = SimpleServiceFactory::<TestService>::new();
        
        container.register_with_options::<TestService>(
            Box::new(factory),
            ServiceRegistrationOptions {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: "Test service".to_string(),
                dependencies: Vec::new(),
                scope: ServiceScope::Singleton,
                lazy: false,
            },
        ).unwrap();
        
        let service1 = container.resolve::<TestService>().unwrap();
        let service2 = container.resolve::<TestService>().unwrap();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&service1, &service2));
    }
}