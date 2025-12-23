//! NOS Kernel Core
//!
//! This crate provides core kernel functionality and abstractions.
//! It includes kernel initialization, interrupt handling, and basic types.

#![no_std]
#![warn(missing_docs)]
#![warn(clippy::all)]

// Re-export API types
pub use nos_api::*;

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

// Core modules
pub mod init;
pub mod interrupt;
pub mod arch;
pub mod types;
pub mod sync;

// Re-export modules
pub use init::*;
pub use interrupt::*;
pub use arch::*;
pub use types::*;
pub use sync::*;

/// Kernel version information
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const KERNEL_NAME: &str = "NOS";
pub const KERNEL_VERSION_MAJOR: u32 = 0;
pub const KERNEL_VERSION_MINOR: u32 = 1;
pub const KERNEL_VERSION_PATCH: u32 = 0;

/// Kernel initialization
///
/// This function initializes the kernel core.
/// It should be called early in the boot process.
///
/// # Returns
/// * `nos_api::Result<()>` - Success or error
pub fn initialize_kernel() -> nos_api::Result<()> {
    // Initialize architecture-specific code
    arch::initialize()?;
    
    // Initialize interrupt handling
    interrupt::initialize()?;
    
    // Initialize synchronization primitives
    sync::initialize()?;
    
    Ok(())
}

/// Kernel shutdown
///
/// This function shuts down the kernel core.
/// It should be called during system shutdown.
///
/// # Returns
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_kernel() -> nos_api::Result<()> {
    // Shutdown interrupt handling
    interrupt::shutdown()?;
    
    // Shutdown architecture-specific code
    arch::shutdown()?;
    
    Ok(())
}

/// Get kernel information
///
/// # Returns
/// * `KernelInfo` - Kernel information
pub fn get_kernel_info() -> KernelInfo {
    KernelInfo {
        name: KERNEL_NAME.to_string(),
        version: KERNEL_VERSION.to_string(),
        major: KERNEL_VERSION_MAJOR,
        minor: KERNEL_VERSION_MINOR,
        patch: KERNEL_VERSION_PATCH,
        build_date: option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown").to_string(),
        build_time: option_env!("VERGEN_BUILD_TIME").unwrap_or("unknown").to_string(),
        commit_hash: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown").to_string(),
        target_triple: option_env!("VERGEN_TARGET_TRIPLE").unwrap_or("unknown").to_string(),
        features: get_enabled_features(),
    }
}

/// Get enabled features
///
/// # Returns
/// * `Vec<String>` - Enabled features
fn get_enabled_features() -> Vec<String> {
    let mut features = Vec::new();
    
    #[cfg(feature = "std")]
    features.push("std".to_string());
    
    #[cfg(feature = "log")]
    features.push("log".to_string());
    
    #[cfg(feature = "debug_subsystems")]
    features.push("debug_subsystems".to_string());
    
    #[cfg(feature = "formal_verification")]
    features.push("formal_verification".to_string());
    
    #[cfg(feature = "security_audit")]
    features.push("security_audit".to_string());
    
    features
}

/// Kernel information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInfo {
    /// Kernel name
    pub name: String,
    /// Kernel version
    pub version: String,
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
    /// Build date
    pub build_date: String,
    /// Build time
    pub build_time: String,
    /// Commit hash
    pub commit_hash: String,
    /// Target triple
    pub target_triple: String,
    /// Enabled features
    pub features: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_info() {
        let info = get_kernel_info();
        assert_eq!(info.name, KERNEL_NAME);
        assert_eq!(info.version, KERNEL_VERSION);
        assert_eq!(info.major, KERNEL_VERSION_MAJOR);
        assert_eq!(info.minor, KERNEL_VERSION_MINOR);
        assert_eq!(info.patch, KERNEL_VERSION_PATCH);
    }

    #[test]
    fn test_enabled_features() {
        let features = get_enabled_features();
        
        // At least the default features should be present
        assert!(!features.is_empty());
    }
}