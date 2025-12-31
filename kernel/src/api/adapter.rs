//! API Type Adapter
//!
//! This module bridges nos-api crate types with kernel-internal types.
//! It provides type aliases and re-exports to maintain compatibility
//! while using the correct nos_api paths.

#![allow(dead_code)]

// Re-export nos_api Result and Error for convenience
pub use nos_api::error::{Error as ApiError, Result as ApiResult};

// ============================================================================
// Service Type Adapters
// ============================================================================

// Service trait - re-exported from nos_api::core::traits::Service
pub use nos_api::core::traits::Service;

// Service information - adapter for InterfaceServiceInfo
pub use nos_api::interfaces::InterfaceServiceInfo as ServiceInfo;

// Service manager - adapter for InterfaceServiceManager
pub use nos_api::interfaces::InterfaceServiceManager as ServiceManager;

// Service status - adapter for InterfaceServiceStatus
pub use nos_api::interfaces::InterfaceServiceStatus as ServiceStatus;

// Service request - adapter for InterfaceServiceRequest
pub use nos_api::interfaces::InterfaceServiceRequest as ServiceRequest;

// Service response - adapter for InterfaceServiceResponse
pub use nos_api::interfaces::InterfaceServiceResponse as ServiceResponse;

// Service stats - adapter for InterfaceServiceStats
pub use nos_api::interfaces::InterfaceServiceStats as ServiceStats;

// ============================================================================
// Syscall Type Adapters
// ============================================================================

// Syscall handler - re-export from interfaces
pub use nos_api::interfaces::{
    InterfaceSyscallHandler as SyscallHandler,
    InterfaceSyscallDispatcher as SyscallDispatcher,
};

// Syscall stats
pub use nos_api::interfaces::SyscallStats as SyscallStats;

// ============================================================================
// Event Type Adapters
// ============================================================================

// Event publisher - adapter for InterfaceEventPublisher
pub use nos_api::interfaces::InterfaceEventPublisher as EventPublisher;

// Event subscriber - adapter for InterfaceEventSubscriber
pub use nos_api::interfaces::InterfaceEventSubscriber as EventSubscriber;

// Basic event from nos_api event module
pub use nos_api::event::BasicEvent;

// ============================================================================
// Context Type Adapters
// ============================================================================

// Context manager - adapter for InterfaceContextManager
pub use nos_api::interfaces::InterfaceContextManager as ContextManager;

// Context - adapter for InterfaceContext
pub use nos_api::interfaces::InterfaceContext as Context;

// Context type - adapter for InterfaceContextType
pub use nos_api::interfaces::InterfaceContextType as ContextType;

// ============================================================================
// Additional Type Aliases for Common Patterns
// ============================================================================

// Service registry from nos_api::service
pub use nos_api::service::interface::ServiceRegistry;

// Service discovery from nos_api::service
pub use nos_api::service::interface::ServiceDiscovery;

// Process manager from nos_api::process
pub use nos_api::process::interface::ProcessManager;

// Scheduler from nos_api::process
pub use nos_api::process::interface::Scheduler;

// Memory manager from nos_api::memory
pub use nos_api::memory::interface::MemoryManager;

// Page allocator from nos_api::memory
pub use nos_api::memory::interface::PageAllocator;

// ============================================================================
// Helper Functions for Common Conversions
// ============================================================================

// Convert to API result
#[inline]
pub fn to_api_result<T>(result: core::result::Result<T, crate::error::Error>) -> ApiResult<T> {
    result.map_err(|e| ApiError::SystemError(format!("{:?}", e)))
}

// Convert from API result
#[inline]
pub fn from_api_result<T>(result: ApiResult<T>) -> core::result::Result<T, crate::error::Error> {
    result.map_err(|e| crate::error::Error::Other(format!("{:?}", e)))
}

// ============================================================================
// Type Conversion Traits
// ============================================================================

// Trait for converting kernel types to API types
pub trait ToApi<T> {
    /// Convert to API type
    fn to_api(self) -> T;
}

// Trait for converting API types to kernel types
pub trait ToKernel<T> {
    /// Convert to kernel type
    fn to_kernel(self) -> T;
}

// ============================================================================
// Documentation
// ============================================================================

// # API Type Adapter
//
// This module provides a unified interface to nos-api types, resolving
// path inconsistencies and providing convenient type aliases.
//
// ## Usage
//
// ```rust
// use kernel::api::adapter::{
//     Service, ServiceInfo, ServiceManager, ServiceStatus,
//     SyscallHandler, SyscallDispatcher,
// };
//
// // Use the types with short, convenient names
// fn register_service(manager: &dyn ServiceManager, service: Arc<dyn Service>) {
//     // Service registration logic
// }
// ```
//
// ## Type Mapping
//
// - `Service` → `nos_api::core::traits::Service`
// - `ServiceInfo` → `nos_api::interfaces::InterfaceServiceInfo`
// - `ServiceManager` → `nos_api::interfaces::InterfaceServiceManager`
// - `ServiceStatus` → `nos_api::interfaces::InterfaceServiceStatus`
// - `SyscallHandler` → `nos_api::interfaces::InterfaceSyscallHandler`
// - `SyscallDispatcher` → `nos_api::interfaces::InterfaceSyscallDispatcher`
// - `EventPublisher` → `nos_api::interfaces::InterfaceEventPublisher`
