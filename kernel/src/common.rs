//! Common utility functions
//!
//! This module provides common utility functions used throughout the kernel.

/// Get current timestamp in milliseconds since boot
///
/// This is a convenience wrapper around the time subsystem's get_timestamp function.
pub fn get_timestamp() -> u64 {
    crate::subsystems::time::get_timestamp()
}

/// Get current timestamp in nanoseconds since boot
///
/// High-precision timestamp for performance measurements.
pub fn get_timestamp_nanos() -> u64 {
    crate::subsystems::time::timestamp_nanos()
}
