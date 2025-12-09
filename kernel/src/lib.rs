//! NOS Kernel Library
//!
//! This crate provides the public API for the NOS (New Operating System) kernel.
//! It exposes kernel components that need to be tested and used by external modules.
//!
//! # Architecture
//!
//! The kernel follows a modular architecture with the following main components:
//!
//! - **System Calls** (`syscalls`): System call interface and dispatch mechanism
//! - **Process Management** (`process`): Process creation, scheduling, and lifecycle management
//! - **Memory Management** (`mm`): Physical and virtual memory management
//! - **File System** (`fs`, `vfs`): Virtual file system and file operations
//! - **Network** (`net`): Network stack and socket interface
//! - **Security** (`security`): Security mechanisms (ASLR, SMAP/SMEP, ACL, Capabilities)
//! - **IPC** (`ipc`): Inter-process communication mechanisms
//!
//! # Usage
//!
//! ```no_run
//! use kernel::syscalls;
//!
//! // Call a system call
//! let pid = syscalls::dispatch(syscalls::SYS_GETPID, &[]);
//! ```
//!
//! # Features
//!
//! - `kernel_tests`: Enables kernel test framework and test cases
//! - `baremetal`: Enables bare-metal boot support (no bootloader)

#![no_std]
#![allow(dead_code)]

#[macro_use]
extern crate alloc;

#[cfg(feature = "kernel_tests")]
#[macro_use]
mod test_macros;

// Re-export commonly used modules for testing
/// System call interface and dispatch mechanism
pub mod syscalls;

/// Process management and scheduling
pub mod process;

/// Reliability and error handling mechanisms
pub mod reliability;

// Re-export key types for external use
/// Boot parameters passed from bootloader to kernel
pub use crate::boot::BootParameters;

/// Error reporting interface for kernel components
pub use crate::error_handling::error_reporting::ErrorReporter;

// Include necessary internal modules for the library
mod arch;
mod boot;
mod cloud_native;
mod compat;
mod collections;
mod types;
mod cpu;
mod debug;
mod drivers;
mod error_handling;
mod formal_verification;
mod fs;
mod ids;
mod ipc;
mod libc;
mod microkernel;
mod mm;
mod sched;
mod net;
mod posix;
mod security;
mod security_audit;
mod services;
mod sync;
mod time;
mod trap;
mod vfs;
mod procfs;

#[cfg(feature = "kernel_tests")]
/// Kernel test framework and test cases
pub mod tests;


/// Initialize the kernel
///
/// This function initializes the kernel subsystems. It should be called
/// once during kernel startup before any other kernel operations.
///
/// # Safety
///
/// This function must be called before any other kernel operations.
/// Calling it multiple times may result in undefined behavior.
pub fn init() {
    // Placeholder for kernel initialization
}