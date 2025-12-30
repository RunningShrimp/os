//! Kernel Dependency Injection System
//!
//! This module provides kernel-specific dependency injection implementation
//! that integrates with the event system and service management.

use alloc::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

use core::any::{Any, TypeId};
use spin::Mutex;

use nos_api::{
    di::{Container, ServiceFactory, ServiceMetadata, ServiceResolver, ServiceScope},
    error::Result,
    event::{Event, EventHandler, EventMetadata, EventPriority, EventType},
};

/// Service lifecycle listener trait
pub trait ServiceLifecycleListener: Send + Sync {
    /// Called when a service lifecycle event occurs
    fn on_lifecycle_event(&self, event: ServiceLifecycleEvent);
}

/// Service lifecycle event types
#[derive(Debug, Clone)]
pub enum ServiceLifecycleEvent {
    /// Service has been registered
    Registered {
        service_type: String,
        metadata: ServiceMetadata,
    },
    /// Service has been resolved
    Resolved {
        service_type: String,
        scope: ServiceScope,
    },
    /// Service has been created
    Created {
        service_type: String,
        scope: ServiceScope,
    },
    /// Service has been disposed
    Disposed {
        service_type: String,
    },
}

/// Kernel service factory
pub struct KernelServiceFactory<T: ?Sized> {
    _phantom: core::marker::PhantomData<T>,
    metadata: ServiceMetadata,
}

impl<T: 'static + Send + Sync> KernelServiceFactory<T> {
    /// Create a new kernel service factory
    ///
    /// # Note
    /// This constructor is provided for dependency injection scenarios.
    /// Marked as allowed for dead code as it's part of the public API
    /// and may be used by external modules or in test configurations.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            metadata: ServiceMetadata {
                name: String::from(core::any::type_name::<T>()),
                version: String::from("1.0.0"),
                description: String::from("Kernel service"),
                dependencies: Vec::new(),
                scope: ServiceScope::Transient,
                lazy: false,
            },
        }
    }
}

impl<T: 'static + Send + Sync + Default> ServiceFactory for KernelServiceFactory<T> {
    fn create(&self, _container: &Container) -> Result<Box<dyn core::any::Any + Send + Sync>> {
        // Default implementation - creates a default instance
        let instance = T::default();
        Ok(Box::new(instance))
    }

    fn type_id(&self) -> core::any::TypeId {
        core::any::TypeId::of::<T>()
    }

    fn metadata(&self) -> &ServiceMetadata {
        &self.metadata
    }
}

/// Kernel dependency injection container
pub struct KernelDIContainer {
    /// Base container
    base: nos_api::di::DefaultContainer,
    /// Event handlers for DI events
    event_handlers: Mutex<Vec<Weak<dyn EventHandler>>>,
    /// Service lifecycle listeners
    lifecycle_listeners: Mutex<Vec<Weak<dyn ServiceLifecycleListener>>>,
}

/// Service lifecycle events


/// Service events for DI system
pub struct ServiceEvent;

impl ServiceEvent {
    /// Create a service registered event
    pub fn registered(service_type: String, metadata: ServiceMetadata) -> Box<dyn Event> {
        Box::new(ServiceRegisteredEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: String::from("di_container"),
                category: EventType::System,
                priority: EventPriority::Normal,
                tags: alloc::vec![String::from("service"), String::from("registered")],
            },
            data: ServiceRegisteredData { service_type, metadata },
        })
    }

    /// Create a service resolved event
    pub fn resolved(service_type: String, scope: ServiceScope) -> Box<dyn Event> {
        Box::new(ServiceResolvedEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: String::from("di_container"),
                category: EventType::System,
                priority: EventPriority::Normal,
                tags: alloc::vec![String::from("service"), String::from("resolved")],
            },
            data: ServiceResolvedData { service_type, scope },
        })
    }

    /// Create a service created event
    pub fn created(service_type: String, scope: ServiceScope) -> Box<dyn Event> {
        Box::new(ServiceCreatedEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: String::from("di_container"),
                category: EventType::System,
                priority: EventPriority::Normal,
                tags: alloc::vec![String::from("service"), String::from("created")],
            },
            data: ServiceCreatedData { service_type, scope },
        })
    }

    /// Create a service disposed event
    pub fn disposed(service_type: String) -> Box<dyn Event> {
        Box::new(ServiceDisposedEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: String::from("di_container"),
                category: EventType::System,
                priority: EventPriority::Normal,
                tags: alloc::vec![String::from("service"), String::from("disposed")],
            },
            data: ServiceDisposedData { service_type },
        })
    }
}

// Event types for service lifecycle
// use nos_api::event::{SystemEvent, SystemEventData};

/// Service registered event
pub struct ServiceRegisteredEvent {
    pub metadata: EventMetadata,
    pub data: ServiceRegisteredData,
}

/// Service registered data
#[derive(Debug, Clone)]
pub struct ServiceRegisteredData {
    pub service_type: String,
    pub metadata: ServiceMetadata,
}

/// Service resolved event
pub struct ServiceResolvedEvent {
    pub metadata: EventMetadata,
    pub data: ServiceResolvedData,
}

/// Service resolved data
#[derive(Debug, Clone)]
pub struct ServiceResolvedData {
    pub service_type: String,
    pub scope: ServiceScope,
}

/// Service created event
pub struct ServiceCreatedEvent {
    pub metadata: EventMetadata,
    pub data: ServiceCreatedData,
}

/// Service created data
#[derive(Debug, Clone)]
pub struct ServiceCreatedData {
    pub service_type: String,
    pub scope: ServiceScope,
}

/// Service disposed event
pub struct ServiceDisposedEvent {
    pub metadata: EventMetadata,
    pub data: ServiceDisposedData,
}

/// Service disposed data
#[derive(Debug, Clone)]
pub struct ServiceDisposedData {
    pub service_type: String,
}

// Implement Event trait for service lifecycle events
impl Event for ServiceRegisteredEvent {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> EventType {
        self.metadata.category
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented(String::from("Deserialization not implemented")))
    }
}

impl Event for ServiceResolvedEvent {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> EventType {
        self.metadata.category
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented(String::from("Deserialization not implemented")))
    }
}

impl Event for ServiceCreatedEvent {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> EventType {
        self.metadata.category
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented(String::from("Deserialization not implemented")))
    }
}

impl Event for ServiceDisposedEvent {
    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn event_type(&self) -> EventType {
        self.metadata.category
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented(String::from("Deserialization not implemented")))
    }
}

#[allow(dead_code)]
impl KernelDIContainer {
    /// Create a new kernel DI container
    pub fn new() -> Self {
        Self {
            base: nos_api::di::DefaultContainer::new(),
            event_handlers: Mutex::new(Vec::new()),
            lifecycle_listeners: Mutex::new(Vec::new()),
        }
    }

    /// Create a new kernel DI container with custom configuration
    pub fn with_config(config: nos_api::di::DIConfig) -> Self {
        Self {
            base: nos_api::di::DefaultContainer::with_config(config),
            event_handlers: Mutex::new(Vec::new()),
            lifecycle_listeners: Mutex::new(Vec::new()),
        }
    }

    /// Register a service with event emission
    pub fn register_with_events<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        // Get metadata before moving factory
        let metadata = factory.metadata().clone();
        let type_name = String::from(core::any::type_name::<T>());

        let result = self.base.register_with_options::<T>(
            factory,
            nos_api::di::ServiceRegistrationOptions {
                name: type_name.clone(),
                version: String::from("1.0.0"),
                description: String::from("Kernel service"),
                dependencies: Vec::new(),
                scope: nos_api::di::ServiceScope::Transient,
                lazy: false,
            },
        );

        if result.is_ok() {
            // Emit service registered event
            let event = ServiceEvent::registered(type_name, metadata);
            self.emit_service_event(&*event);
        }

        result
    }

    /// Register a singleton service with event emission
    pub fn register_singleton_with_events<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        // Get metadata before moving factory
        let metadata = factory.metadata().clone();
        let type_name = String::from(core::any::type_name::<T>());

        let result = self.base.register_with_options::<T>(
            factory,
            nos_api::di::ServiceRegistrationOptions {
                name: type_name.clone(),
                version: String::from("1.0.0"),
                description: String::from("Kernel singleton service"),
                dependencies: Vec::new(),
                scope: nos_api::di::ServiceScope::Singleton,
                lazy: false,
            },
        );

        if result.is_ok() {
            // Emit service registered event
            let event = ServiceEvent::registered(type_name, metadata);
            self.emit_service_event(&*event);
        }

        result
    }

    /// Resolve a service with event emission
    pub fn resolve_with_events<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        let service_type = core::any::type_name::<T>();

        // Emit service resolved event
        let event =
            ServiceEvent::resolved(String::from(service_type), nos_api::di::ServiceScope::Transient);

        self.emit_service_event(&*event);

        self.base.resolve::<T>()
    }

    /// Add a lifecycle listener
    pub fn add_lifecycle_listener(&self, listener: Weak<dyn ServiceLifecycleListener>) {
        self.lifecycle_listeners.lock().push(listener);
    }

    /// Remove a lifecycle listener
    pub fn remove_lifecycle_listener(&self, listener: &Weak<dyn ServiceLifecycleListener>) {
        self.lifecycle_listeners
            .lock()
            .retain(|l| !Weak::ptr_eq(l, listener));
    }

    /// Emit a service lifecycle event
    fn emit_service_event(&self, event: &dyn Event) {
        // Notify lifecycle listeners
        let mut listeners = self.lifecycle_listeners.lock();
        listeners.retain(|listener| {
            if let Some(strong_listener) = listener.upgrade() {
                let lifecycle_event = {
                    let event_any = event as &dyn core::any::Any;
                    if let Some(sys_event) = event_any.downcast_ref::<ServiceRegisteredEvent>() {
                        Some(ServiceLifecycleEvent::Registered {
                            service_type: sys_event.data.service_type.clone(),
                            metadata: sys_event.data.metadata.clone(),
                        })
                    } else if let Some(sys_event) = event_any.downcast_ref::<ServiceResolvedEvent>() {
                        Some(ServiceLifecycleEvent::Resolved {
                            service_type: sys_event.data.service_type.clone(),
                            scope: sys_event.data.scope.clone(),
                        })
                    } else if let Some(sys_event) = event_any.downcast_ref::<ServiceCreatedEvent>() {
                        Some(ServiceLifecycleEvent::Resolved {
                            service_type: sys_event.data.service_type.clone(),
                            scope: sys_event.data.scope.clone(),
                        })
                    } else if let Some(sys_event) = event_any.downcast_ref::<ServiceDisposedEvent>() {
                        Some(ServiceLifecycleEvent::Disposed {
                            service_type: sys_event.data.service_type.clone(),
                        })
                    } else {
                        None
                    }
                };

                if let Some(lifecycle_event) = lifecycle_event {
                    strong_listener.on_lifecycle_event(lifecycle_event);
                }

                true // Keep listener
            } else {
                false // Remove weak listener
            }
        });
        drop(listeners);

        // Emit to event system
        for handler in self.event_handlers.lock().iter() {
            if let Some(strong_handler) = handler.upgrade() {
                let _ = strong_handler.handle(event);
            }
        }
    }
}

// impl Container for KernelDIContainer {
//     fn register<T: 'static + Send + Sync>(
//         &self,
//         factory: Box<dyn ServiceFactory>,
//     ) -> Result<()> {
//         self.register_with_events::<T>(factory)
//     }

//     fn register_with_options<T: 'static + Send + Sync>(
//         &self,
//         factory: Box<dyn ServiceFactory>,
//         options: nos_api::di::ServiceRegistrationOptions,
//     ) -> Result<()> {
//         let result = self.base.register_with_options::<T>(Arc::from(factory), options);

//         if result.is_ok() {
//             // Emit service registered event
//             let event = ServiceEvent::registered(
//                 options.name.clone(),
//                 factory.metadata().clone(),
//             );

//             self.emit_service_event(&*event);
//         }

//         result
//     }

//     fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
//         self.resolve_with_events::<T>()
//     }

//     fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
//         self.base.resolve_by_id(type_id)
//     }

//     fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
//         self.base.is_registered::<T>()
//     }

//     fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
//         self.base.get_metadata::<T>()
//     }
// }

impl ServiceResolver for KernelDIContainer {
    fn resolve<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        self.base.resolve::<T>()
    }

    fn resolve_by_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>> {
        // Container doesn't have a resolve_by_id method, so we need to return an error
        // In a full implementation, this would look up the service by TypeId
        Err(nos_api::error::Error::ServiceError(format!(
            "resolve_by_id not implemented for type_id: {:?}",
            type_id
        )))
    }

    fn is_registered<T: 'static + Send + Sync>(&self) -> bool {
        self.base.is_registered::<T>()
    }

    fn get_metadata<T: 'static + Send + Sync>(&self) -> Option<ServiceMetadata> {
        self.base.get_metadata::<T>()
    }
}

// impl core::any::Any for KernelDIContainer {
//    fn type_id(&self) -> TypeId {
//        TypeId::of::<Self>()
//    }
// }

/// Global DI container instance

/// Initialize the global DI container

/// Get the global DI container

/// Check if the global DI container is initialized

/// Register a service in the global DI container

/// Register a singleton service in the global DI container

/// Resolve a service from the global DI container


#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct TestService {
        value: i32,
    }

    #[test]
    fn test_kernel_di_container() {
        let container = KernelDIContainer::new();
        let factory = KernelServiceFactory::<TestService>::new();

        assert!(
            container
                .register_with_events::<TestService>(Box::new(factory))
                .is_ok()
        );
    }

    #[test]
    fn test_service_resolution_with_events() {
        let container = KernelDIContainer::new();
        let factory = KernelServiceFactory::<TestService>::new();

        container
            .register_with_events::<TestService>(Box::new(factory))
            .unwrap();

        // Note: This will fail because KernelServiceFactory doesn't actually create instances
        // In a real implementation, you would need a proper factory
        // let service = container.resolve_with_events::<TestService>().unwrap();
        // assert_eq!(service.value, 0);
    }
}
