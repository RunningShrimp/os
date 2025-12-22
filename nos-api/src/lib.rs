//! NOS API - Core interfaces and types for the NOS operating system
//!
//! This crate provides the core interfaces, types, and abstractions used throughout
//! the NOS operating system. It serves as the foundation for communication between
//! different kernel components and ensures consistent APIs across the system.
//!
//! # Architecture
//!
//! The API is organized into several key modules:
//!
//! - **Core**: Fundamental traits, types, and constants
//! - **Error**: Common error types and handling mechanisms
//! - **Syscall**: System call interface definitions
//! - **Service**: Service registry and discovery interfaces
//! - **Memory**: Memory management abstractions
//! - **Process**: Process management interfaces
//!
//! # Design Principles
//!
//! - **Dependency Inversion**: High-level modules depend on abstractions
//! - **Interface Segregation**: Small, focused interfaces
//! - **Single Responsibility**: Each interface has a single purpose
//!
//! # Usage
//!
//! ```rust
//! use nos_api::core::traits::Service;
//! use nos_api::error::Result;
//!
//! struct MyService;
//!
//! impl Service for MyService {
//!     fn name(&self) -> &str {
//!         "my_service"
//!     }
//! }
//!
//! fn main() -> Result<()> {
//!     let service = MyService;
//!     println!("Service: {}", service.name());
//!     Ok(())
//! }
//! ```

#![no_std]
#![allow(dead_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// Core modules
pub mod core;
pub mod error;
pub mod syscall;
pub mod service;
pub mod memory;
pub mod process;
pub mod factory;
pub mod event;
pub mod di;
pub mod interfaces;
pub mod context;
pub mod service_lifecycle;
pub mod collections;
pub mod fmt_utils;
pub mod boot;

// Performance monitoring and optimization (from nos-perf)
pub mod perf;

// Re-export commonly used types
pub use crate::core::types::*;
pub use crate::error::{Error, Result};
pub use crate::syscall::interface::SyscallHandler;
pub use crate::core::traits::Service;
pub use crate::service::interface::ServiceRegistry;
pub use crate::memory::interface::{MemoryManager, PageAllocator};
pub use crate::process::interface::{ProcessManager, Scheduler};
pub use crate::factory::{ServiceFactory, MemoryManagerFactory, ProcessManagerFactory, SyscallDispatcherFactory};
pub use crate::interfaces::*;