//! Power Management Subsystem
//!
//! This module provides comprehensive power management capabilities for the NOS kernel.
//! It implements a framework similar to Linux's power management subsystem, including:
//!
//! - **CPU Frequency Scaling** (`cpufreq`): Dynamic CPU frequency adjustment with multiple governors
//! - **CPU Idle Management** (`cpuidle`): C-state management for power savings during idle periods
//! - **Device Power Management** (`dev_pm`): Runtime PM and system suspend/resume for devices
//! - **System Sleep States** (`sleep`): S3 (suspend-to-RAM) and S4 (suspend-to-disk) support
//! - **Energy Monitoring** (`energy_monitor`): Real-time power consumption tracking and analysis
//!
//! ## Architecture
//!
//! The power management subsystem is organized into several components:
//!
//! ```text
//! Power Management
//! ├── cpufreq       - CPU frequency scaling (governors, frequency tables)
//! ├── cpuidle       - CPU idle states (C-states)
//! ├── dev_pm        - Device power management (runtime PM, suspend/resume)
//! ├── sleep         - System sleep states (S3, S4, S5)
//! └── energy_monitor - Energy consumption monitoring and statistics
//! ```
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use kernel::power;
//!
//! // Initialize power management
//! power::init()?;
//!
//! // Get CPU frequency manager
//! let cpufreq_mgr = power::cpufreq::get_manager().unwrap();
//!
//! // Set governor to ondemand for dynamic frequency scaling
//! let cpu = cpufreq_mgr.lock().get_cpu_mut(0).unwrap();
//! cpu.set_governor(power::cpufreq::GovernorType::Ondemand)?;
//!
//! // Enable runtime PM for a device
//! use kernel::power::dev_pm::DevicePower;
//! let mut device = DevicePower::new("eth0", 0);
//! device.enable_runtime_pm()?;
//!
//! // Monitor energy consumption
//! use kernel::power::energy_monitor;
//! let current_power = energy_monitor::get_current_power();
//! println!("Current power: {} mW", current_power);
//! ```
//!
//! ## CPU Frequency Scaling
//!
//! The cpufreq module supports multiple governors:
//!
//! - **Performance**: Always uses maximum frequency
//! - **Powersave**: Always uses minimum frequency
//! - **Ondemand**: Dynamically adjusts based on CPU load
//! - **Conservative**: Gradually adjusts based on CPU load
//!
//! ## CPU Idle States
//!
//! The cpuidle module manages C-states (processor idle states):
//!
//! - **C0**: Active state (CPU is running)
//! - **C1**: Halt state (CPU stops executing but maintains context)
//! - **C1E**: Enhanced halt (lower power than C1)
//! - **C3**: Sleep state (cache coherence maintained)
//! - **C6**: Deep power down (context saved in memory)
//!
//! ## Device Power Management
//!
//! Device power states follow the D0-D3 specification:
//!
//! - **D0**: Fully on (device is operational)
//! - **D1**: Partial power (limited functionality, faster wake)
//! - **D2**: Low power (minimal functionality, slower wake)
//! - **D3hot**: Standby (powered but not operational)
//! - **D3cold**: Off (no power)
//!
//! ## System Sleep States
//!
//! The system supports multiple ACPI sleep states:
//!
//! - **S0**: Working (system is fully operational)
//! - **S3**: Suspend-to-RAM (sleep, maintains memory)
//! - **S4**: Suspend-to-Disk (hibernate, all devices off)
//! - **S5**: Soft Off (system powered off)
//!
//! ## Energy Monitoring
//!
//! The energy monitor tracks:
//!
//! - Real-time power consumption (per-component and total)
//! - Energy consumption over time windows
//! - Battery state and charge level
//! - Power efficiency metrics
//! - Power policy recommendations

pub mod cpufreq;
pub mod cpuidle;
pub mod dev_pm;
pub mod sleep;
pub mod energy_monitor;

use crate::error::UnifiedError;

/// Power management version
pub const VERSION: &str = "0.1.0";

/// Initialize the power management subsystem
///
/// This function initializes all power management components:
/// - CPU frequency manager
/// - CPU idle manager
/// - Sleep state manager
/// - Energy monitor
///
/// # Errors
///
/// Returns an error if initialization of any component fails.
pub fn init() -> Result<(), UnifiedError> {
    // Initialize CPU frequency management
    cpufreq::init()?;

    // Initialize CPU idle management
    cpuidle::init()?;

    // Initialize sleep state management
    sleep::init()?;

    // Initialize energy monitoring
    energy_monitor::init()?;

    log::info!("Power management subsystem initialized (version {})", VERSION);

    Ok(())
}

/// Power management configuration
pub struct PowerConfig {
    /// CPU frequency scaling enabled
    pub cpufreq_enabled: bool,
    /// CPU idle management enabled
    pub cpuidle_enabled: bool,
    /// Device runtime PM enabled
    pub runtime_pm_enabled: bool,
    /// Sleep states enabled
    pub sleep_enabled: bool,
    /// Energy monitoring enabled
    pub energy_monitor_enabled: bool,
}

impl PowerConfig {
    /// Get default configuration
    pub fn default() -> Self {
        PowerConfig {
            cpufreq_enabled: true,
            cpuidle_enabled: true,
            runtime_pm_enabled: true,
            sleep_enabled: true,
            energy_monitor_enabled: true,
        }
    }

    /// Get performance-oriented configuration
    pub fn performance() -> Self {
        PowerConfig {
            cpufreq_enabled: true,
            cpuidle_enabled: false, // Keep CPUs active
            runtime_pm_enabled: false, // Keep devices active
            sleep_enabled: true,
            energy_monitor_enabled: true,
        }
    }

    /// Get power-saving configuration
    pub fn powersave() -> Self {
        PowerConfig {
            cpufreq_enabled: true,
            cpuidle_enabled: true,
            runtime_pm_enabled: true,
            sleep_enabled: true,
            energy_monitor_enabled: true,
        }
    }
}

/// Get current power state summary
pub struct PowerStateSummary {
    /// Total power consumption (milliwatts)
    pub total_power_mw: u32,
    /// Number of active CPUs
    pub active_cpus: usize,
    /// Number of CPUs in idle states
    pub idle_cpus: usize,
    /// Current sleep state
    pub sleep_state: sleep::SleepState,
    /// Battery capacity (percentage, if available)
    pub battery_capacity: Option<u8>,
}

/// Get current power state summary
pub fn get_state_summary() -> PowerStateSummary {
    let total_power_mw = energy_monitor::get_current_power();
    let battery_info = energy_monitor::get_battery_info();
    let battery_capacity = if battery_info.is_present() {
        Some(battery_info.capacity)
    } else {
        None
    };

    PowerStateSummary {
        total_power_mw,
        active_cpus: 0, // TODO: Get from cpuidle
        idle_cpus: 0,   // TODO: Get from cpuidle
        sleep_state: sleep::get_current_state(),
        battery_capacity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_config_defaults() {
        let config = PowerConfig::default();
        assert!(config.cpufreq_enabled);
        assert!(config.cpuidle_enabled);
        assert!(config.runtime_pm_enabled);
    }

    #[test]
    fn test_performance_config() {
        let config = PowerConfig::performance();
        assert!(config.cpufreq_enabled);
        assert!(!config.cpuidle_enabled);
        assert!(!config.runtime_pm_enabled);
    }

    #[test]
    fn test_powersave_config() {
        let config = PowerConfig::powersave();
        assert!(config.cpufreq_enabled);
        assert!(config.cpuidle_enabled);
        assert!(config.runtime_pm_enabled);
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_state_summary() {
        let summary = get_state_summary();
        // Just verify it doesn't panic
        assert_eq!(summary.sleep_state, sleep::SleepState::S0);
    }
}
