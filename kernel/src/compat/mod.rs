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

pub mod windows;
pub mod macos;
pub mod linux;
pub mod android;
pub mod ios;

/// Target platform enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlatform {
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
    fn initialize(&mut self) -> Result<(), &'static str>;
    fn shutdown(&mut self) -> Result<(), &'static str>;
}

/// Default hasher builder for hash maps
// We use a custom BuildHasher that wraps hashbrown's default.
// This avoids using the type alias which seems to trigger rustc_private checks
// when used with hashbrown::HashMap in some contexts.
#[derive(Default, Clone)]
pub struct DefaultHasherBuilder;

impl core::hash::BuildHasher for DefaultHasherBuilder {
    type Hasher = hashbrown::hash_map::DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        hashbrown::hash_map::DefaultHasher::default()
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