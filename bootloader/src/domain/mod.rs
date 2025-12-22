//! Domain Layer - Core bootloader domain concepts and business rules
//!
//! This module contains pure domain objects that encapsulate the core
//! bootloader logic, independent of infrastructure concerns like hardware
//! access, file I/O, or protocol implementations.
//!
//! Key concepts:
//! - **Entities**: Mutable objects with identity (BootProcess, BootInfo)
//! - **Value Objects**: Immutable objects representing domain values (BootConfig, GraphicsMode)
//! - **Aggregate Roots**: Entry points for consistency (BootInfo aggregates memory map, kernel, framebuffer)
//! - **Domain Services**: Business logic spanning multiple entities
//! - **Domain Events**: Significant changes published to subscribers

pub mod aggregate_root;
pub mod boot_config;
pub mod boot_info;
pub mod boot_services;
pub mod events;
pub mod event_persistence;
pub mod repositories;
pub mod serialization;
pub mod transactions;
pub mod hardware_detection;
pub mod enhanced_event_publisher;
pub mod event_driven_state;

pub use aggregate_root::{
    AggregateRoot, VersionedAggregateRoot, AuditableAggregateRoot,
    SoftDeletableAggregateRoot, AggregateRootBuilder, AggregateRootFactory,
    AggregateRootValidator, EventSourcedAggregateRoot
};
pub use boot_config::{
    BootConfig, GraphicsMode, LogLevel, MemoryRegion, MemoryRegionType,
    KernelInfo, GraphicsInfo, BootPhase
};
pub use boot_info::BootInfo;
pub use boot_services::{
    BootValidator, GraphicsModeSelector, HardwareInfo, GraphicsCapabilities,
    MemoryManager, KernelLoader
};
pub use events::{
    BootPhaseCompletedEvent, DomainEvent, DomainEventPublisher, GraphicsInitializedEvent,
    KernelLoadedEvent, BootPhaseStartedEvent, MemoryInitializedEvent, ValidationFailedEvent,
    DeviceDetectedEvent, EventFilter, SimpleEventFilter, ImprovedEventPublisher,
    SimpleEventPublisher, NamedDomainEventSubscriber, LoggingSubscriber
};
pub use event_persistence::{
    PersistentEventStore, DiagnosticEventReplayer,
    MemoryUsageStats, ReplayStats, ReplayFilter
};
pub use enhanced_event_publisher::{
    EnhancedEventPublisher, PublisherConfig, PublisherStats, DiagnosticReport
};
pub use event_driven_state::{
    EventDrivenStateManager, BootState, StateTransitionContext, StateTransitionStats
};
pub use repositories::{
    BootConfigRepository, KernelImageRepository, DefaultBootConfigRepository,
    Repository, EntityId, TransactionId, RepositoryError, Page, IdGenerator,
    DefaultIdGenerator, SerializationService, SimpleSerializationService,
    BootInfoRepository, MemoryRegionRepository, KernelInfoRepository, GraphicsInfoRepository
};
pub use serialization::{
    SerializationFormat, SerializationContext, SerializationResult, DeserializationResult,
    Serializer, BinarySerializer, JsonSerializer, SerializerRegistry
};
pub use transactions::{
    TransactionOperation, TransactionStatus, TransactionError, Transaction,
    TransactionManager, TransactionStats, TransactionLog, TransactionLogEntry,
    TransactionLogType, TransactionLogStats
};
pub use hardware_detection::{
    HardwareDetectionService, CpuInfo, CpuFeatures, DetectionCapabilities
};

// Re-export all domain types for convenience
pub use events::*;
