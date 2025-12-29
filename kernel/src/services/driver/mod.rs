//! Driver Service Module
//!
//! This module provides driver-related types and functions for the services subsystem.

use nos_api::Result;

// Re-export from subsystems::drivers if available
#[cfg(feature = "drivers")]
pub use crate::subsystems::drivers::driver_manager::{
    DeviceType, DeviceStatus, DeviceResources, DeviceInfo, DriverInfo,
    DriverManager,
};

/// Device type enumeration
#[cfg(not(feature = "drivers"))]
#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Block,
    Character,
    Network,
    Other,
}

/// Device status enumeration
#[cfg(not(feature = "drivers"))]
#[derive(Debug, Clone, Copy)]
pub enum DeviceStatus {
    Initialized,
    Running,
    Suspended,
    Failed,
}

/// Device resources structure
#[cfg(not(feature = "drivers"))]
#[derive(Debug, Clone)]
pub struct DeviceResources {
    pub memory_regions: Vec<(usize, usize)>,
    pub irq_lines: Vec<u32>,
}

/// Driver manager
#[cfg(not(feature = "drivers"))]
pub struct DriverManager {
    devices: Vec<DeviceRecord>,
}

#[cfg(not(feature = "drivers"))]
#[derive(Debug, Clone)]
struct DeviceRecord {
    name: String,
    device_type: DeviceType,
    status: DeviceStatus,
}

#[cfg(not(feature = "drivers"))]
impl DriverManager {
    pub fn new() -> Self {
        Self { devices: Vec::new() }
    }
}

/// Get driver manager instance
#[cfg(not(feature = "drivers"))]
pub fn get_driver_manager() -> &'static DriverManager {
    static MANAGER: DriverManager = DriverManager::new();
    &MANAGER
}
