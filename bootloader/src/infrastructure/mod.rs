//! Infrastructure Layer - Hardware abstraction and external services
//!
//! This layer contains implementations of domain repositories and services
//! that interact with hardware and external systems.

pub mod di_config;
pub mod di_container;
pub mod graphics_backend;
pub mod hardware_detection;
pub mod repositories;
pub mod serialization;
pub mod service_discovery;
pub mod transactions;

pub use di_config::{
    ConditionConfig, ConfigError, ConfigParser, ConfigurableServiceFactory, DIContainerConfig,
    ServiceConfig,
};
pub use di_container::{
    BootDIContainer, DIContainer, DefaultBootConfigRepositoryFactory, GraphicsBackendFactory,
    HardwareDetectionServiceFactory, ServiceCondition, ServiceDescriptor, ServiceFactory,
    ServiceLifecycle, ServiceScope, ServiceStats, SimpleEventPublisherFactory,
};
pub use graphics_backend::{GraphicsBackend, create_graphics_backend};
pub use hardware_detection::{
    BiosHardwareDetectionService, UefiHardwareDetectionService, create_hardware_detection_service,
};
pub use repositories::{
    DefaultRepositoryFactory, FactoryError, FactoryStats, MemoryRepository, RepositoryFactory,
    RepositoryFactoryConfig, SpecificRepositoryWrapper,
};
pub use serialization::{SerializerRegistry, SimpleSerializer};
pub use service_discovery::{
    BiosServiceRegistry, CoreServiceRegistry, DiscoveryStats, ServiceDiscovery, ServiceMetadata,
    ServiceRegistry, UefiServiceRegistry,
};
pub use transactions::{
    MemoryTransactionLog, MemoryTransactionManager, TransactionLogQuery, TransactionPool,
};

pub use crate::domain::repositories::DefaultBootConfigRepository;
