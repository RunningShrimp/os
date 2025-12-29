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
//! use nos_api::{core::traits::Service, error::Result};
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

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

pub use hashbrown::HashMap;

// Core modules
pub mod boot;
pub mod collections;
pub mod context;
pub mod core;
pub mod di;
pub mod error;
pub mod event;
pub mod factory;
pub mod fmt_utils;
pub mod interfaces;
pub mod memory;
pub mod process;
pub mod service;
pub mod service_lifecycle;
pub mod syscall;

pub use crate::fmt_utils::format;

// Performance monitoring and optimization (from nos-perf)
pub mod perf;

// Re-export commonly used types
pub use crate::{
    core::{traits::Service, types::*},
    error::{Error, Result},
    factory::{
        MemoryManagerFactory, ProcessManagerFactory, ServiceFactory, SyscallDispatcherFactory,
    },
    interfaces::*,
    memory::interface::{MemoryManager, PageAllocator},
    process::interface::{ProcessManager, Scheduler},
    service::interface::ServiceRegistry,
    syscall::interface::{SyscallDispatcher, SyscallHandler},
};
