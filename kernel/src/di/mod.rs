//! Kernel Dependency Injection System
//! 
//! This module provides kernel-specific dependency injection implementation
//! that integrates with the event system and service management.

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
    string::String,
    boxed::Box,
};
use core::any::{Any, TypeId};

use nos_api::{
    di::{Container, ServiceFactory, ServiceMetadata, ServiceScope, ServiceResolver},
    error::Result,
    event::{Event, EventHandler, EventMetadata, EventPriority, EventType},
};


/// Kernel dependency injection container
pub struct KernelDIContainer {
    /// Base container
    base: nos_api::di::DefaultContainer,
    /// Event handlers for DI events
    event_handlers: Vec<Weak<dyn EventHandler>>,
    /// Service lifecycle listeners
    lifecycle_listeners: Vec<Weak<dyn ServiceLifecycleListener>>,
}


/// Service lifecycle events
#[derive(Debug, Clone)]
pub enum ServiceLifecycleEvent {
    /// Service registered
    Registered {
        service_type: String,
        metadata: ServiceMetadata,
    },
    /// Service resolved
    Resolved {
        service_type: String,
        scope: ServiceScope,
    },
    /// Service disposed
    Disposed {
        service_type: String,
    },
}

/// Service lifecycle listener
pub trait ServiceLifecycleListener: Send + Sync {
    /// Handle service lifecycle event
    fn on_lifecycle_event(&self, event: ServiceLifecycleEvent);
}

/// Kernel service factory with event support
pub struct KernelServiceFactory<T: 'static + Send + Sync> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T: 'static + Send + Sync> KernelServiceFactory<T> {
    /// Create a new kernel service factory
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T: 'static + Send + Sync + Default> ServiceFactory for KernelServiceFactory<T> {
    fn create(&self, container: &Container) -> Result<Box<dyn Any + Send + Sync>> {
        // Emit service creation event
        let event = ServiceEvent::created(
            core::any::type_name::<T>(),
            ServiceScope::Transient,
        );
        
        if let Some(kernel_container) = container.as_any().downcast_ref::<KernelDIContainer>() {
            kernel_container.emit_lifecycle_event(ServiceLifecycleEvent::Resolved {
                service_type: core::any::type_name::<T>().to_string(),
                scope: ServiceScope::Transient,
            });
        }
        
        Ok(Box::new(T::default()))
    }
    
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn metadata(&self) -> &ServiceMetadata {
        static METADATA: ServiceMetadata = ServiceMetadata {
            name: core::any::type_name::<T>(),
            version: "1.0.0",
            description: "Kernel service factory",
            dependencies: Vec::new(),
            scope: ServiceScope::Transient,
            lazy: false,
        };
        &METADATA
    }
}

/// Service events for DI system
pub struct ServiceEvent;

impl ServiceEvent {
    /// Create a service registered event
    pub fn registered(service_type: String, metadata: ServiceMetadata) -> Box<dyn Event> {
        Box::new(ServiceRegisteredEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: "di_container".to_string(),
                category: EventType::System,
                priority: EventPriority::Normal,

                tags: alloc::vec!["service", "registered"],
            },
            data: ServiceRegisteredData {
                service_type,
                metadata,
            },
        })
    }
    
    /// Create a service resolved event
    pub fn resolved(service_type: String, scope: ServiceScope) -> Box<dyn Event> {
        Box::new(ServiceResolvedEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: "di_container".to_string(),
                category: EventType::System,
                priority: EventPriority::Normal,

                tags: alloc::vec!["service", "resolved"],
            },
            data: ServiceResolvedData {
                service_type,
                scope,
            },
        })
    }
    
    /// Create a service created event
    pub fn created(service_type: String, scope: ServiceScope) -> Box<dyn Event> {
        Box::new(ServiceCreatedEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: "di_container".to_string(),
                category: EventType::System,
                priority: EventPriority::Normal,

                tags: alloc::vec!["service", "created"],
            },
            data: ServiceCreatedData {
                service_type,
                scope,
            },
        })
    }
    
    /// Create a service disposed event
    pub fn disposed(service_type: String) -> Box<dyn Event> {
        Box::new(ServiceDisposedEvent {
            metadata: EventMetadata {
                id: None,
                timestamp: crate::subsystems::time::get_time_ns(),
                source: "di_container".to_string(),
                category: EventType::System,
                priority: EventPriority::Normal,

                tags: alloc::vec!["service", "disposed"],
            },
            data: ServiceDisposedData {
                service_type,
            },
        })
    }
}

// Event types for service lifecycle
use nos_api::event::{SystemEvent, SystemEventData};

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
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
    
    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented("Deserialization not implemented".to_string()))
    }
}

impl Event for ServiceResolvedEvent {
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
    
    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented("Deserialization not implemented".to_string()))
    }
}

impl Event for ServiceCreatedEvent {
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
    
    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented("Deserialization not implemented".to_string()))
    }
}

impl Event for ServiceDisposedEvent {
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
    
    fn deserialize(_data: &[u8]) -> Result<Self> {
        Err(nos_api::error::Error::NotImplemented("Deserialization not implemented".to_string()))
    }
}

impl KernelDIContainer {
    /// Create a new kernel DI container
    pub fn new() -> Self {
        Self {
            base: nos_api::di::DefaultContainer::new(),
            event_handlers: Vec::new(),
            lifecycle_listeners: Vec::new(),
        }
    }
    
    /// Create a new kernel DI container with custom configuration
    pub fn with_config(config: nos_api::di::DIConfig) -> Self {
        Self {
            base: nos_api::di::DefaultContainer::with_config(config),
            event_handlers: Vec::new(),
            lifecycle_listeners: Vec::new(),
        }
    }
    
    /// Register a service with event emission
    pub fn register_with_events<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        let result = self.base.register_with_options::<T>(
            Arc::from(factory),
            nos_api::di::ServiceRegistrationOptions {
                name: core::any::type_name::<T>().to_string(),
                version: "1.0.0".to_string(),
                description: "Kernel service".to_string(),
                dependencies: Vec::new(),
                scope: nos_api::di::ServiceScope::Transient,
                lazy: false,
            },
        );
        
        if result.is_ok() {
            // Emit service registered event
            let event = ServiceEvent::registered(
                core::any::type_name::<T>().to_string(),
                factory.metadata().clone(),
            );
            
            self.emit_service_event(&*event);
        }
        
        result
    }
    
    /// Register a singleton service with event emission
    pub fn register_singleton_with_events<T: 'static + Send + Sync>(
        &self,
        factory: Box<dyn ServiceFactory>,
    ) -> Result<()> {
        let result = self.base.register_with_options::<T>(
            Arc::from(factory),
            nos_api::di::ServiceRegistrationOptions {
                name: core::any::type_name::<T>().to_string(),
                version: "1.0.0".to_string(),
                description: "Kernel singleton service".to_string(),
                dependencies: Vec::new(),
                scope: nos_api::di::ServiceScope::Singleton,
                lazy: false,
            },
        );
        
        if result.is_ok() {
            // Emit service registered event
            let event = ServiceEvent::registered(
                core::any::type_name::<T>().to_string(),
                factory.metadata().clone(),
            );
            
            self.emit_service_event(&*event);
        }
        
        result
    }
    
    /// Resolve a service with event emission
    pub fn resolve_with_events<T: 'static + Send + Sync>(&self) -> Result<Arc<T>> {
        let service_type = core::any::type_name::<T>();
        
        // Emit service resolved event
        let event = ServiceEvent::resolved(
            service_type.to_string(),
            nos_api::di::ServiceScope::Transient,
        );
        
        self.emit_service_event(&*event);
        
        self.base.resolve::<T>()
    }
    
    /// Add a lifecycle listener
    pub fn add_lifecycle_listener(&mut self, listener: Weak<dyn ServiceLifecycleListener>) {
        self.lifecycle_listeners.push(listener);
    }
    
    /// Remove a lifecycle listener
    pub fn remove_lifecycle_listener(&mut self, listener: &Weak<dyn ServiceLifecycleListener>) {
        self.lifecycle_listeners.retain(|l| !Weak::ptr_eq(l, listener));
    }
    
    /// Emit a service lifecycle event
    fn emit_service_event(&self, event: &dyn Event) {
        // Notify lifecycle listeners
        self.lifecycle_listeners.retain(|listener| {
            if let Some(strong_listener) = listener.upgrade() {
                let lifecycle_event = match event.category() {
                    nos_api::event::EventType::System => {
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
                    }
                    _ => None,
                };
                
                if let Some(lifecycle_event) = lifecycle_event {
                    strong_listener.on_lifecycle_event(lifecycle_event);
                }
                
                true // Keep listener
            } else {
                false // Remove weak listener
            }
        });
        
        // Emit to event system
        for handler in &self.event_handlers {
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

// impl core::any::Any for KernelDIContainer {
//    fn type_id(&self) -> TypeId {
//        TypeId::of::<Self>()
//    }
// }


/// Global DI container instance
static mut GLOBAL_DI_CONTAINER: Option<KernelDIContainer> = None;
static DI_CONTAINER_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Initialize the global DI container
pub fn init() -> Result<()> {
    if DI_CONTAINER_INIT.compare_exchange(false, true, core::sync::atomic::Ordering::SeqCst, core::sync::atomic::Ordering::Relaxed).is_ok() {
        unsafe {
            GLOBAL_DI_CONTAINER = Some(KernelDIContainer::new());
        }
        Ok(())
    } else {
        Err(nos_api::error::Error::DIError("DI container already initialized".to_string()))
    }
}

/// Get the global DI container
pub fn get() -> &'static mut KernelDIContainer {
    unsafe {
        GLOBAL_DI_CONTAINER.as_mut().expect("DI container not initialized")
    }
}

/// Check if the global DI container is initialized
pub fn is_initialized() -> bool {
    DI_CONTAINER_INIT.load(core::sync::atomic::Ordering::SeqCst)
}

/// Register a service in the global DI container
pub fn register_service<T: 'static + Send + Sync>(
    factory: Box<dyn ServiceFactory>,
) -> Result<()> {
    get().register_with_events::<T>(factory)
}

/// Register a singleton service in the global DI container
pub fn register_singleton<T: 'static + Send + Sync>(
    factory: Box<dyn ServiceFactory>,
) -> Result<()> {
    get().register_singleton_with_events::<T>(factory)
}

/// Resolve a service from the global DI container
pub fn resolve<T: 'static + Send + Sync>() -> Result<Arc<T>> {
    get().resolve_with_events::<T>()
}

/// Check if a service is registered in the global DI container
pub fn is_registered<T: 'static + Send + Sync>() -> bool {
    get().is_registered::<T>()
}

/// Get service metadata from the global DI container
pub fn get_metadata<T: 'static + Send + Sync>() -> Option<ServiceMetadata> {
    get().get_metadata::<T>()
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
    fn test_kernel_di_container() {
        let mut container = KernelDIContainer::new();
        let factory = KernelServiceFactory::<TestService>::new();
        
        assert!(container.register_with_events::<TestService>(Box::new(factory)).is_ok());
        assert!(container.is_registered::<TestService>());
    }
    
    #[test]
    fn test_service_resolution_with_events() {
        let mut container = KernelDIContainer::new();
        let factory = KernelServiceFactory::<TestService>::new();
        
        container.register_with_events::<TestService>(Box::new(factory)).unwrap();
        
        let service = container.resolve_with_events::<TestService>().unwrap();
        assert_eq!(service.value, 0);
    }
}