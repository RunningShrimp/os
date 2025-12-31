#![allow(dead_code)]
//! Platform-specific compatibility modules
//!
//! This module contains platform-specific compatibility implementations:
//! - Windows API compatibility layer
//! - macOS frameworks compatibility
//! - Android runtime compatibility
//! - iOS framework compatibility
//! - Linux binary compatibility

extern crate alloc;
extern crate hashbrown;

use alloc::boxed::Box;
use alloc::vec::Vec;

/// Compatibility error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityError {
    /// Invalid binary format
    InvalidBinaryFormat,
    /// Unsupported architecture
    UnsupportedArchitecture,
    /// Unsupported API version
    UnsupportedApi,
    /// Memory error
    MemoryError,
    /// Not found
    NotFound,
    /// I/O error
    IoError,
    /// Operation not supported
    NotSupported,
}

pub type Result<T, E = CompatibilityError> = core::result::Result<T, E>;

pub mod android;
pub mod ios;
pub mod linux;
pub mod macos;
pub mod windows;
pub mod loader;
pub mod abi;
pub mod graphics;
pub mod memory;

// Explicitly load the MemoryPermissions module (uppercase filename)
#[path = "MemoryPermissions.rs"]
mod memory_permissions_impl;

// Re-export memory types and permissions
pub use memory::{MemoryRegion, MemoryRegionType};

// Re-export MemoryPermissions
pub use memory_permissions_impl::MemoryPermissions;

pub mod package_manager;
pub mod sandbox;
pub mod syscall_translator;

/// Memory manager for compatibility layer
#[derive(Debug)]
pub struct MemoryManager {
    pub regions: Vec<MemoryRegion>,
    pub next_addr: usize,
    pub stats: MemoryStats,
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub peak_allocation: usize,
    pub allocation_count: usize,
}

/// Target platform enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetPlatform {
    Nos,
    Windows,
    MacOS,
    Linux,
    Android,
    IOS,
    Unknown,
}

/// Platform module trait
pub trait PlatformModule {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn is_supported(&self) -> bool;
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}

/// Default hasher builder for hash maps
// We use core::hash::BuildHasherDefault with hashbrown::DefaultHasher
// hashbrown::DefaultHasher has a new() method but not Default trait
// We use a custom builder that calls new()
#[derive(Clone, Copy)]
pub struct DefaultHasherBuilder;

// Re-export hashbrown's DefaultHashBuilder for convenience
pub use hashbrown::DefaultHashBuilder;

impl Default for DefaultHasherBuilder {
    fn default() -> Self {
        Self
    }
}

impl core::hash::BuildHasher for DefaultHasherBuilder {
    type Hasher = hashbrown::DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        hashbrown::DefaultHashBuilder::default().build_hasher()
    }
}

// Platform module factory

/// Create platform module for given target
pub fn create_platform_module(platform: TargetPlatform) -> Option<Box<dyn PlatformModule>> {
    match platform {
        TargetPlatform::Windows => Some(Box::new(windows::WindowsModule::new())),
        TargetPlatform::MacOS => Some(Box::new(macos::MacOSModule::new())),
        TargetPlatform::Linux => Some(Box::new(linux::LinuxModule::new())),
        TargetPlatform::Android => Some(Box::new(android::AndroidModule::new())),
        TargetPlatform::IOS => Some(Box::new(ios::IOSModule::new())),
        _ => None,
    }
}

/// Initialize compatibility subsystem
pub fn init() -> Result<()> {
    // Initialize all compatibility layers
    Ok(())
}

/// Shutdown compatibility subsystem
pub fn shutdown() -> Result<()> {
    // Cleanup compatibility layers
    Ok(())
}
