//! Driver Service Module
//!
//! This module provides driver-related types and functions for the services subsystem.

// Re-export from subsystems::drivers
pub use crate::subsystems::drivers::driver_manager::{
    DeviceType, DeviceStatus, DeviceResources, DeviceInfo, DriverInfo,
    DriverManager, get_driver_manager,
};
