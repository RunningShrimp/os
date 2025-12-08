//! Platform-specific compatibility modules
//!
//! This module contains platform-specific compatibility implementations:
//! - Windows API compatibility layer
//! - macOS frameworks compatibility
//! - Android runtime compatibility
//! - iOS framework compatibility
//! - Linux binary compatibility

pub mod windows;
pub mod macos;
pub mod linux;
pub mod android;
pub mod ios;

extern crate alloc;

// Platform module factory
// Re-export platform types from parent module
pub use crate::compat::{TargetPlatform, PlatformModule};
use alloc::boxed::Box;

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