//! NOS Kernel Library
//!
//! This crate provides the public API for the NOS (New Operating System) kernel.
//! It acts as an integration layer for the various kernel components.
//!
//! # Architecture
//!
//! The kernel follows a modular architecture with the following main components:
//!
//! - **System Calls** (`nos-syscalls`): System call interface and dispatch mechanism
//! - **Services** (`nos-services`): Service management and discovery framework
//! - **Error Handling** (`nos-error-handling`): Comprehensive error handling and recovery framework
//! - **Memory Management** (`nos-mm`): Physical and virtual memory management
//! - **Process Management** (`process`): Process creation, scheduling, and lifecycle management
//! - **File System** (`fs`, `vfs`): Virtual file system and file operations
//! - **Network** (`net`): Network stack and socket interface
//! - **Security** (`security`): Security mechanisms (ASLR, SMAP/SMEP, ACL, Capabilities)
//! - **IPC** (`ipc`): Inter-process communication mechanisms
//!
//! # Usage
//!
//! ```no_run
//! use kernel::init_kernel;
//!
//! // Initialize the kernel
//! let boot_params = kernel::BootParameters::default();
//! init_kernel(boot_params)?;
//! ```
//!
//! # Features
//!
//! - `kernel_tests`: Enables kernel test framework and test cases
//! - `baremetal`: Enables bare-metal boot support (no bootloader)
//! - `syscalls`: Enables system call support (via nos-syscalls crate)
//! - `services`: Enables service management (via nos-services crate)
//! - `error_handling`: Enables error handling (via nos-error-handling crate)

#![no_std]
#![allow(dead_code)]
#![allow(missing_docs)]

#[macro_use]
extern crate alloc;

#[cfg(feature = "kernel_tests")]
#[macro_use]
mod test_macros;

// Logging macros (stub implementations for no_std environments)
#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_debug { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_info { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_warn { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

// API layer - public interfaces for kernel subsystems
pub mod api;

// VFS interface layer - breaks circular dependency between VFS and FS
pub mod vfs_interface;

// Core kernel functionality (from nos-kernel-core)
pub mod core;

// Error handling module
pub mod error;

// Kernel factory for creating and managing internal modules
mod kernel_factory;

// Include necessary internal modules for library
// pub mod arch;
pub mod subsystems;
pub mod platform;

// Re-export key types for external use
/// Boot parameters passed from bootloader to kernel
pub use crate::boot::BootParameters;

/// Kernel factory and components
pub use kernel_factory::*;

/// Core kernel functionality
pub use core::*;

/// Performance monitoring
pub use perf::*;

/// POSIX types and constants
pub use posix::*;

// Re-export moved modules to maintain compatibility
pub use subsystems::fs;
pub use subsystems::vfs;
pub use subsystems::ipc;
pub use subsystems::process;
pub use subsystems::sync;
pub use subsystems::time;

// Re-export external crates when features are enabled
#[cfg(feature = "syscalls")]
pub use nos_syscalls as syscalls;

#[cfg(feature = "services")]
pub use nos_services as services;

#[cfg(feature = "error_handling")]
pub use nos_error_handling as error_handling;

#[cfg(feature = "net_stack")]
pub use subsystems::net;

#[cfg(not(feature = "net_stack"))]
pub mod net {
    #[allow(dead_code)]
    pub enum NetworkError { Unsupported }
    #[allow(dead_code)]
    pub enum Packet {}

    pub mod socket {
        #[allow(dead_code)]
        pub struct SocketAddr;
        #[allow(dead_code)]
        pub enum Socket {}
        #[allow(dead_code)]
        pub enum SocketOptions {}
        #[allow(dead_code)]
        pub enum SocketType {}
        #[allow(dead_code)]
        pub enum ProtocolFamily {}
        #[allow(dead_code)]
        pub struct TcpSocketWrapper;
        impl TcpSocketWrapper { pub fn new(_opts: super::SocketOptions) -> Self { TcpSocketWrapper } }
        #[allow(dead_code)]
        pub struct UdpSocketWrapper;
        #[allow(dead_code)]
        pub enum SocketError {}
        #[allow(dead_code)]
        pub struct RawSocketWrapper;
        impl RawSocketWrapper { pub fn new(_opts: super::SocketOptions) -> Self { RawSocketWrapper } }
    }

    pub mod tcp {
        pub mod manager {
            #[allow(dead_code)]
            pub struct TcpOptions;
            #[allow(dead_code)]
            pub struct ConnectionId(pub u64);
            #[allow(dead_code)]
            pub struct TcpConnectionManager;
            #[allow(dead_code)]
            pub enum TcpError {}
        }
    }

    pub mod ipv4 {
        #[allow(dead_code)]
        pub struct Ipv4Addr(pub u8, pub u8, pub u8, pub u8);
    }
}

pub use platform::arch;
pub use platform::boot;
pub use platform::drivers;
pub use platform::trap;

mod compat;
mod collections;
mod syscall_interface;
mod types;
mod cpu;
#[cfg(feature = "debug_subsystems")]
mod debug;
mod event;
mod di;
mod ids;
mod libc;
// Legacy modules - now accessed through subsystems
mod sched;
mod monitoring;
mod perf;
pub mod posix;
mod security;
#[cfg(feature = "security_audit")]
mod security_audit;
mod procfs;

#[cfg(not(feature = "cloud_native"))]
mod cloud_native {
    pub mod namespaces {
        use alloc::string::String;
        #[derive(Debug)]
        pub enum NamespaceType { Mount, UTS, IPC, Network, PID, User }

        pub struct NamespaceParameters {
            pub mount_params: Option<()>,
            pub network_params: Option<()>,
            pub user_params: Option<()>,
            pub uts_params: Option<()>,
        }

        pub struct NamespaceConfig {
            pub ns_type: NamespaceType,
            pub new_namespace: bool,
            pub existing_path: Option<String>,
        }

        pub fn create_namespace(_config: NamespaceConfig) -> Result<u64, ()> { Ok(0) }
        pub fn join_namespace(_path: &str) -> Result<u64, ()> { Ok(0) }
    }
}

/// Initialize the kernel
///
/// This function initializes all kernel subsystems and prepares the system
/// for operation.
/// It uses the same core initialization logic as `rust_main_with_boot_info`,
/// ensuring consistency between bootloader-based and library-based startup.
///
/// # Arguments
///
/// * `boot_params` - Boot parameters passed from the bootloader
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn init_kernel(boot_params: BootParameters) -> nos_api::Result<()> {
    // Use the same core initialization function as bootloader entry
    // This ensures consistency between different entry points
    core::init::init_kernel_core(Some(&boot_params));
    
    log_info!("NOS Kernel initialized successfully");
    
    Ok(())
}

/// Shutdown the kernel
///
/// This function shuts down all kernel subsystems in a controlled manner.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_kernel() -> nos_api::Result<()> {
    log_info!("Shutting down NOS Kernel");
    
    // Shutdown performance monitoring
    // perf::shutdown_performance_monitor()?; // Function not found
    
    // Shutdown scheduler
    // sched::shutdown_scheduler()?; // Function not found
    
    // Shutdown security
    // security::shutdown_security()?; // Function not found
    
    // Shutdown network stack (if enabled)
    // #[cfg(feature = "net_stack")]
    // {
    //     subsystems::net::shutdown_network_stack()?;
    // }
    
    // Shutdown error handling (if enabled)
    #[cfg(feature = "error_handling")]
    {
        nos_error_handling::shutdown_error_handling()?;
    }
    
    // Shutdown services (if enabled)
    #[cfg(feature = "services")]
    {
        nos_services::shutdown_services()?;
    }
    
    // Shutdown system calls (if enabled)
    #[cfg(all(feature = "syscalls", feature = "alloc"))]
    {
        nos_syscalls::shutdown_syscalls()?;
    }
    
    // Shutdown IPC
    // subsystems::ipc::shutdown_ipc()?; // Function not found
    
    // Shutdown file system
    // subsystems::fs::shutdown_file_system()?; // Function not found
    
    // Shutdown process management
    // subsystems::process::shutdown_process_management()?; // Function not found
    
    // Shutdown memory management
    mm::shutdown_advanced_memory_management()?;
    
    // Shutdown platform
    platform::shutdown_platform()?;
    
    log_info!("NOS Kernel shutdown complete");
    
    Ok(())
}

/// Get kernel version
///
/// # Returns
///
/// * `&'static str` - Kernel version string
pub fn get_kernel_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get kernel build information
///
/// # Returns
///
/// * `KernelBuildInfo` - Kernel build information
pub fn get_kernel_build_info() -> KernelBuildInfo {
    KernelBuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        build_time: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
        git_commit: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
        target_triple: option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or_else(|| {
            option_env!("CARGO_BUILD_TARGET").unwrap_or("unknown")
        }),
        profile: if cfg!(debug_assertions) { "debug" } else { "release" },
        features: get_enabled_features(),
    }
}

/// Kernel build information
#[derive(Debug, Clone)]
pub struct KernelBuildInfo {
    /// Kernel version
    pub version: &'static str,
    /// Build timestamp
    pub build_time: &'static str,
    /// Git commit hash
    pub git_commit: &'static str,
    /// Target triple
    pub target_triple: &'static str,
    /// Build profile
    pub profile: &'static str,
    /// Enabled features
    pub features: alloc::vec::Vec<&'static str>,
}

/// Get enabled features
fn get_enabled_features() -> alloc::vec::Vec<&'static str> {
    let mut features = alloc::vec::Vec::new();
    
    if cfg!(feature = "baremetal") {
        features.push("baremetal");
    }
    if cfg!(feature = "kernel_tests") {
        features.push("kernel_tests");
    }
    if cfg!(feature = "syscalls") {
        features.push("syscalls");
    }
    if cfg!(feature = "services") {
        features.push("services");
    }
    if cfg!(feature = "error_handling") {
        features.push("error_handling");
    }
    if cfg!(feature = "net_stack") {
        features.push("net_stack");
    }
    if cfg!(feature = "posix_layer") {
        features.push("posix_layer");
    }
    if cfg!(feature = "debug_subsystems") {
        features.push("debug_subsystems");
    }
    if cfg!(feature = "security_audit") {
        features.push("security_audit");
    }
    if cfg!(feature = "formal_verification") {
        features.push("formal_verification");
    }
    if cfg!(feature = "cloud_native") {
        features.push("cloud_native");
    }
    
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_version() {
        let version = get_kernel_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_kernel_build_info() {
        let build_info = get_kernel_build_info();
        assert!(!build_info.version.is_empty());
        assert!(!build_info.build_time.is_empty());
        assert!(!build_info.git_commit.is_empty());
        assert!(!build_info.target_triple.is_empty());
        assert!(!build_info.profile.is_empty());
        assert!(!build_info.features.is_empty());
    }

    #[test]
    fn test_enabled_features() {
        let features = get_enabled_features();
        assert!(!features.is_empty());
    }
}