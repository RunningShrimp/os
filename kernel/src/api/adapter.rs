//! API Type Adapter
//!
//! This module bridges nos-api crate types with kernel-internal types.
//! It provides type aliases and re-exports to maintain compatibility
//! while using the correct nos_api paths.

#![allow(dead_code)]

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

// Re-export nos_api Result and Error for convenience
pub use nos_api::error::{Error as ApiError, Result as ApiResult};

// ============================================================================
// Service Type Adapters
// ============================================================================

// Service trait - re-exported from nos_api::core::traits::Service
pub use nos_api::core::traits::Service;

// Service information - adapter for InterfaceServiceInfo
pub type ServiceInfo = nos_api::interfaces::InterfaceServiceInfo;

// Service manager - adapter for InterfaceServiceManager
pub type ServiceManager = nos_api::interfaces::InterfaceServiceManager;

// Service status - adapter for InterfaceServiceStatus
pub type ServiceStatus = nos_api::interfaces::InterfaceServiceStatus;

// Service request - adapter for InterfaceServiceRequest
pub type ServiceRequest = nos_api::interfaces::InterfaceServiceRequest;

// Service response - adapter for InterfaceServiceResponse
pub type ServiceResponse = nos_api::interfaces::InterfaceServiceResponse;

// Service stats - adapter for InterfaceServiceStats
pub type ServiceStats = nos_api::interfaces::InterfaceServiceStats;

// ============================================================================
// Syscall Type Adapters
// ============================================================================

// Syscall handler - re-export from interfaces
pub use nos_api::interfaces::{
    InterfaceSyscallHandler as SyscallHandler,
    InterfaceSyscallDispatcher as SyscallDispatcher,
};

// Syscall stats
pub type SyscallStats = nos_api::interfaces::SyscallStats;

// ============================================================================
// Event Type Adapters
// ============================================================================

// Event publisher - adapter for InterfaceEventPublisher
pub type EventPublisher = nos_api::interfaces::InterfaceEventPublisher;

// Event subscriber - adapter for InterfaceEventSubscriber
pub type EventSubscriber = nos_api::interfaces::InterfaceEventSubscriber;

// Basic event from nos_api event module
pub use nos_api::event::BasicEvent;

// ============================================================================
// Context Type Adapters
// ============================================================================

// Context manager - adapter for InterfaceContextManager
pub type ContextManager = nos_api::interfaces::InterfaceContextManager;

// Context - adapter for InterfaceContext
pub type Context = nos_api::interfaces::InterfaceContext;

// Context type - adapter for InterfaceContextType
pub type ContextType = nos_api::interfaces::InterfaceContextType;

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
    result.map_err(|e| crate::error::Error::SystemError(format!("{:?}", e)))
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
