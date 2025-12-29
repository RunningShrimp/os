//! Driver Service Module
//!
//! This module provides driver-related types and functions for the services subsystem.
//! Device types are now re-exported from platform::device for consistency.

use alloc::string::String;

// Device types are re-exported from services/mod.rs

/// Device information structure
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub device_type: DeviceType,
    pub vendor: String,
    pub version: String,
}

/// Driver information structure
#[derive(Debug, Clone)]
pub struct DriverInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Driver manager
pub struct DriverManager {
    devices: Vec<DeviceRecord>,
}

#[derive(Debug, Clone)]
struct DeviceRecord {
    name: String,
    device_type: DeviceType,
    status: DeviceStatus,
    resources: DeviceResources,
    info: DeviceInfo,
}

impl DriverManager {
    pub fn new() -> Self {
        Self { devices: Vec::new() }
    }
}

/// Get driver manager instance
pub fn get_driver_manager() -> &'static DriverManager {
    static MANAGER: DriverManager = DriverManager::new();
    &MANAGER
}