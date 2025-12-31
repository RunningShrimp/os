//! Power Management Subsystem
//!
//! This module provides comprehensive power management support including:
//! - CPU frequency scaling (CPUFreq)
//! - Device power management (D-States)
//! - System sleep states (S3/S4)
//! - ACPI integration
//!
//! # Architecture
//!
//! The power management subsystem is organized into several components:
//! - **CPUFreq**: Dynamic CPU frequency scaling with multiple governors
//! - **Device PM**: Device power state management (D0-D3)
//! - **Sleep**: System suspend/resume (S3/S4)
//! - **ACPI**: ACPI table parsing and power management
//!
//! # Example
//!
//! ```rust
//! use kernel::subsystems::power::{CpufreqGovernor, PerformanceGovernor};
//!
//! // Create a performance governor
//! let governor = PerformanceGovernor::new(3_000_000); // 3GHz
//!
//! // Adjust frequency based on load
//! governor.adjust(85); // 85% load -> max frequency
//! ```

pub mod cpufreq;
pub mod device_pm;
pub mod sleep;
pub mod acpi_pm;

pub use cpufreq::{
    CpufreqGovernor,
    PerformanceGovernor,
    PowersaveGovernor,
    OndemandGovernor,
    ConservativeGovernor,
};

pub use device_pm::{
    DevicePowerState,
    PowerManaged,
    PowerManager,
    PowerPolicy,
};

pub use sleep::{
    SleepState,
    SleepManager,
    SuspendResult,
};

pub use acpi_pm::{
    AcpiPowerManager,
    PState,
    CState,
};

// Re-exports for convenience
pub use core::time::Duration;
