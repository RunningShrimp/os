//! Core system call functionality
//!
//! This module provides the core system call dispatch mechanism,
//! including the dispatcher, registry, and common utilities.

pub mod dispatcher;
pub mod registry;
pub mod traits;

// Re-export commonly used items
#[cfg(feature = "alloc")]
pub use dispatcher::{SyscallDispatcher, init_dispatcher, shutdown_dispatcher};
#[cfg(feature = "alloc")]
pub use dispatcher::get_dispatcher;
#[cfg(feature = "alloc")]
pub use dispatcher::get_dispatcher_mut;
#[cfg(feature = "alloc")]
pub use dispatcher::get_stats;
pub use registry::{SyscallRegistry, get_registry};
#[cfg(feature = "alloc")]
pub use registry::SyscallInfo;
pub use traits::{SyscallHandler, SyscallInterceptor, SyscallFilter};