//! Core system call interface definitions
//!
//! This module provides trait definitions for system call handlers,
//! dispatchers, and related components. Actual implementations are in
//! kernel/subsystems/syscalls.

pub mod traits;
pub mod registry; // Registry trait definitions only

// Note: dispatcher module contains implementation details that should be
// in kernel, not here. It is deprecated.
#[cfg(feature = "alloc")]
#[deprecated(note = "Dispatcher implementation should be in kernel/subsystems/syscalls")]
pub mod dispatcher;

// Re-export trait definitions
pub use traits::{
    SyscallHandler, SyscallValidator, SyscallLogger,
    SyscallInterceptor, SyscallFilter, SyscallContext, SyscallStats
};
pub use registry::{SyscallRegistry, SyscallInfo};