//! Domain Layer - Core bootloader domain concepts and business rules
//!
//! This module contains pure domain objects that encapsulate the core
//! bootloader logic, independent of infrastructure concerns like hardware
//! access, file I/O, or protocol implementations.
//!
//! Key concepts:
//! - **Entities**: Mutable objects with identity (BootProcess, BootInfo)
//! - **Value Objects**: Immutable objects representing domain values (BootConfig, GraphicsMode)
//! - **Aggregate Roots**: Entry points for consistency (BootInfo aggregates memory map, kernel,
//!   framebuffer)
//! - **Domain Services**: Business logic spanning multiple entities
//! - **Domain Events**: Significant changes published to subscribers

pub mod aggregate_root;
pub mod boot_config;
pub mod boot_info;
pub mod boot_services;
pub mod enhanced_event_publisher;
pub mod event_driven_state;
pub mod event_persistence;
pub mod events;
pub mod hardware_detection;
pub mod repositories;
pub mod serialization;
pub mod transactions;

pub use aggregate_root::{
    AggregateRoot, AggregateRootBuilder, AggregateRootFactory, AggregateRootValidator,
    AuditableAggregateRoot, EventSourcedAggregateRoot, SoftDeletableAggregateRoot,
    VersionedAggregateRoot,
};
pub use boot_config::{
    BootConfig, BootPhase, GraphicsInfo, GraphicsMode, KernelInfo, LogLevel, MemoryRegion,
    MemoryRegionType,
};
pub use boot_info::BootInfo;
pub use boot_services::{
    BootValidator, GraphicsCapabilities, GraphicsModeSelector, HardwareInfo, KernelLoader,
    MemoryManager,
};
pub use enhanced_event_publisher::{
    DiagnosticReport, EnhancedEventPublisher, PublisherConfig, PublisherStats,
};
pub use event_driven_state::{
    BootState, EventDrivenStateManager, StateTransitionContext, StateTransitionStats,
};
pub use event_persistence::{
    DiagnosticEventReplayer, MemoryUsageStats, PersistentEventStore, ReplayFilter, ReplayStats,
};
// Re-export all domain types for convenience
pub use events::*;
pub use events::{
    BootPhaseCompletedEvent, BootPhaseStartedEvent, DeviceDetectedEvent, DomainEvent,
    DomainEventPublisher, EventFilter, GraphicsInitializedEvent, ImprovedEventPublisher,
    KernelLoadedEvent, LoggingSubscriber, MemoryInitializedEvent, NamedDomainEventSubscriber,
    SimpleEventFilter, SimpleEventPublisher, ValidationFailedEvent,
};
pub use hardware_detection::{
    CpuFeatures, CpuInfo, DetectionCapabilities, HardwareDetectionService,
};
pub use repositories::{
    BootConfigRepository, BootInfoRepository, DefaultBootConfigRepository, DefaultIdGenerator,
    EntityId, GraphicsInfoRepository, IdGenerator, KernelImageRepository, KernelInfoRepository,
    MemoryRegionRepository, Page, Repository, RepositoryError, SerializationService,
    SimpleSerializationService, TransactionId,
};
pub use serialization::{
    BinarySerializer, DeserializationResult, JsonSerializer, SerializationContext,
    SerializationFormat, SerializationResult, Serializer, SerializerRegistry,
};
pub use transactions::{
    Transaction, TransactionError, TransactionLog, TransactionLogEntry, TransactionLogStats,
    TransactionLogType, TransactionManager, TransactionOperation, TransactionStats,
    TransactionStatus,
};
