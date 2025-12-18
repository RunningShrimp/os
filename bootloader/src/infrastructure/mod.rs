//! Infrastructure Layer - Hardware abstraction and external services
//!
//! This layer contains implementations of domain repositories and services
//! that interact with hardware and external systems.

pub mod graphics_backend;
pub mod di_container;
pub mod di_config;
pub mod hardware_detection;
pub mod repositories;
pub mod service_discovery;
pub mod serialization;
pub mod transactions;

pub use graphics_backend::{GraphicsBackend, create_graphics_backend};
pub use di_container::{DIContainer, BootDIContainer, ServiceDescriptor, ServiceFactory, ServiceLifecycle, ServiceCondition, ServiceScope, ServiceStats,
    DefaultBootConfigRepositoryFactory, HardwareDetectionServiceFactory, GraphicsBackendFactory, SimpleEventPublisherFactory};
pub use crate::domain::repositories::DefaultBootConfigRepository;
pub use di_config::{DIContainerConfig, ServiceConfig, ConditionConfig, ConfigParser, ConfigurableServiceFactory, ConfigError};
pub use hardware_detection::{BiosHardwareDetectionService, UefiHardwareDetectionService, create_hardware_detection_service};
pub use repositories::{MemoryRepository, RepositoryFactory, DefaultRepositoryFactory, FactoryError, FactoryStats, RepositoryFactoryConfig, SpecificRepositoryWrapper};
pub use serialization::{SerializerRegistry, SimpleSerializer};
pub use transactions::{MemoryTransactionManager, MemoryTransactionLog, TransactionPool, TransactionLogQuery};
pub use service_discovery::{
    ServiceRegistry, ServiceMetadata, ServiceDiscovery, CoreServiceRegistry,
    BiosServiceRegistry, UefiServiceRegistry, DiscoveryStats
};
