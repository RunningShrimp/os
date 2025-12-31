//! Device Power Management
//!
//! This module provides comprehensive device power management interfaces,
//! including:
//! - Runtime power management (autosuspend, autoresume)
//! - System sleep power management (suspend, resume, hibernate)
//! - Device power state transitions
//! - Wakeup event handling
//! - Power quality monitoring
//! - Power policy management

use crate::prelude::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use core::sync::atomic {AtomicU32, AtomicU64, Ordering, Ordering};

// ============================================================================
// Power Management Constants
// ============================================================================

/// Default autosuspend delay (milliseconds)
pub const DEFAULT_AUTOSUSPEND_DELAY_MS: u64 = 2000;

/// Maximum autosuspend delay (milliseconds)
pub const MAX_AUTOSUSPEND_DELAY_MS: u64 = 10000;

/// Power state transition timeout (milliseconds)
pub const POWER_TRANSITION_TIMEOUT_MS: u64 = 5000;

// ============================================================================
// Device Power States
// ============================================================================

/// Device power state (following ACPI D-states)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DevicePowerState {
    /// D0: Fully powered and operational
    D0,
    /// D1: Low power state, fast wake
    D1,
    /// D2: Lower power state, slower wake
    D2,
    /// D3Hot: Powered but context lost
    D3Hot,
    /// D3Cold: Powered off
    D3Cold,
}

impl DevicePowerState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            DevicePowerState::D0 => "D0 (Full Power)",
            DevicePowerState::D1 => "D1 (Low Power)",
            DevicePowerState::D2 => "D2 (Lower Power)",
            DevicePowerState::D3Hot => "D3Hot (Context Lost)",
            DevicePowerState::D3Cold => "D3Cold (Power Off)",
        }
    }

    /// Check if state is operational (can do I/O)
    pub fn is_operational(&self) -> bool {
        matches!(self, DevicePowerState::D0)
    }

    /// Check if state is suspended (not operational)
    pub fn is_suspended(&self) -> bool {
        !self.is_operational()
    }

    /// Get power consumption (percentage of D0)
    pub fn power_consumption(&self) -> u32 {
        match self {
            DevicePowerState::D0 => 100,
            DevicePowerState::D1 => 50,
            DevicePowerState::D2 => 20,
            DevicePowerState::D3Hot => 5,
            DevicePowerState::D3Cold => 0,
        }
    }

    /// Get wake latency (microseconds, approximate)
    pub fn wake_latency_us(&self) -> u64 {
        match self {
            DevicePowerState::D0 => 0,
            DevicePowerState::D1 => 100,
            DevicePowerState::D2 => 1000,
            DevicePowerState::D3Hot => 10000,
            DevicePowerState::D3Cold => 100000,
        }
    }
}

/// System power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPowerState {
    /// System is working
    Working,
    /// System is suspending to RAM
    SuspendToRam,
    /// System is suspending to disk
    SuspendToDisk,
    /// System is hibernating
    Hibernate,
    /// System is shutting down
    Shutdown,
}

// ============================================================================
// Wakeup Events
// ============================================================================

/// Wakeup event source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupSource {
    /// No specific source
    None,
    /// Power button
    PowerButton,
    /// RTC alarm
    RtcAlarm,
    /// LAN wakeup
    Lan,
    /// USB device
    Usb,
    /// PCI device
    Pci,
    /// GPIO pin
    Gpio,
    /// Custom source
    Custom(u32),
}

/// Wakeup event
#[derive(Debug, Clone)]
pub struct WakeupEvent {
    /// Event source
    pub source: WakeupSource,
    /// Device ID that generated the event
    pub device_id: Option<u32>,
    /// Event timestamp
    pub timestamp: u64,
    /// Event data
    pub data: Vec<u8>,
}

impl WakeupEvent {
    /// Create new wakeup event
    pub fn new(source: WakeupSource) -> Self {
        Self {
            source,
            device_id: None,
            timestamp: 0,
            data: Vec::new(),
        }
    }

    /// Set device ID
    pub fn with_device(mut self, device_id: u32) -> Self {
        self.device_id = Some(device_id);
        self
    }
}

// ============================================================================
// Device Power Management
// ============================================================================

/// Runtime power management flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePmFlags {
    /// Runtime PM enabled
    pub enabled: bool,
    /// Autosuspend enabled
    pub autosuspend_enabled: bool,
    /// Device can wake up system
    pub can_wakeup: bool,
    /// Device is remote wakeup capable
    pub remote_wakeup: bool,
    /// Device must remain active
    pub always_active: bool,
    /// Device is in use
    pub in_use: bool,
    /// Device is ignoring children
    pub ignore_children: bool,
}

impl Default for RuntimePmFlags {
    fn default() -> Self {
        Self {
            enabled: true,
            autosuspend_enabled: false,
            can_wakeup: false,
            remote_wakeup: false,
            always_active: false,
            in_use: false,
            ignore_children: false,
        }
    }
}

/// Device power management info
#[derive(Debug)]
pub struct DevicePowerInfo {
    /// Device ID
    pub device_id: u32,
    /// Current power state
    pub current_state: DevicePowerState,
    /// Target power state (for transitions)
    pub target_state: Option<DevicePowerState>,
    /// Supported power states
    pub supported_states: Vec<DevicePowerState>,
    /// Runtime PM flags
    pub runtime_flags: RuntimePmFlags,
    /// Autosuspend delay (milliseconds)
    pub autosuspend_delay: u64,
    /// Last activity timestamp
    pub last_activity: AtomicU64,
    /// Time in current state (nanoseconds)
    pub state_time_ns: AtomicU64,
    /// State transition count
    pub transitions: AtomicU64,
    /// Wakeup events count
    pub wakeup_count: AtomicU64,
    /// Power consumption (microwatts)
    pub power_consumption_uw: AtomicU64,
}

// Manual implementation of Clone for DevicePowerInfo
impl Clone for DevicePowerInfo {
    fn clone(&self) -> Self {
        Self {
            device_id: self.device_id,
            current_state: self.current_state,
            target_state: self.target_state,
            supported_states: self.supported_states.clone(),
            runtime_flags: self.runtime_flags,
            autosuspend_delay: self.autosuspend_delay,
            last_activity: AtomicU64::new(self.last_activity.load(Ordering::Relaxed)),
            state_time_ns: AtomicU64::new(self.state_time_ns.load(Ordering::Relaxed)),
            transitions: AtomicU64::new(self.transitions.load(Ordering::Relaxed)),
            wakeup_count: AtomicU64::new(self.wakeup_count.load(Ordering::Relaxed)),
            power_consumption_uw: AtomicU64::new(self.power_consumption_uw.load(Ordering::Relaxed)),
        }
    }
}

impl DevicePowerInfo {
    /// Create new device power info
    pub fn new(device_id: u32, supported_states: Vec<DevicePowerState>) -> Self {
        Self {
            device_id,
            current_state: DevicePowerState::D0,
            target_state: None,
            supported_states,
            runtime_flags: RuntimePmFlags::default(),
            autosuspend_delay: DEFAULT_AUTOSUSPEND_DELAY_MS,
            last_activity: AtomicU64::new(0),
            state_time_ns: AtomicU64::new(0),
            transitions: AtomicU64::new(0),
            wakeup_count: AtomicU64::new(0),
            power_consumption_uw: AtomicU64::new(0),
        }
    }

    /// Set runtime flags
    pub fn with_runtime_flags(mut self, flags: RuntimePmFlags) -> Self {
        self.runtime_flags = flags;
        self
    }

    /// Set autosuspend delay
    pub fn with_autosuspend_delay(mut self, delay: u64) -> Self {
        self.autosuspend_delay = delay.min(MAX_AUTOSUSPEND_DELAY_MS);
        self
    }

    /// Check if state is supported
    pub fn supports_state(&self, state: DevicePowerState) -> bool {
        self.supported_states.contains(&state)
    }

    /// Update activity timestamp
    pub fn update_activity(&self) {
        self.last_activity.fetch_add(1, Ordering::Relaxed);
    }

    /// Get state transition count
    pub fn get_transitions(&self) -> u64 {
        self.transitions.load(Ordering::Relaxed)
    }

    /// Increment transition count
    pub fn increment_transition(&self) {
        self.transitions.fetch_add(1, Ordering::Relaxed);
    }

    /// Get wakeup count
    pub fn get_wakeup_count(&self) -> u64 {
        self.wakeup_count.load(Ordering::Relaxed)
    }

    /// Increment wakeup count
    pub fn increment_wakeup(&self) {
        self.wakeup_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update power consumption
    pub fn update_power_consumption(&self, consumption_uw: u64) {
        self.power_consumption_uw.store(consumption_uw, Ordering::Relaxed);
    }

    /// Get current power consumption
    pub fn get_power_consumption(&self) -> u64 {
        self.power_consumption_uw.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Power Management Statistics
// ============================================================================

/// Power management statistics
#[derive(Debug, Default, Clone)]
pub struct PowerStats {
    /// Total devices under PM control
    pub total_devices: u32,
    /// Devices in D0 state
    pub devices_d0: u32,
    /// Devices in D1 state
    pub devices_d1: u32,
    /// Devices in D2 state
    pub devices_d2: u32,
    /// Devices in D3Hot state
    pub devices_d3hot: u32,
    /// Devices in D3Cold state
    pub devices_d3cold: u32,
    /// Total state transitions
    pub total_transitions: u64,
    /// Total wakeup events
    pub total_wakeups: u64,
    /// Total power consumption (microwatts)
    pub total_power_uw: u64,
    /// Energy saved (microwatt-hours)
    pub energy_saved_uwh: u64,
}

// ============================================================================
// Power Manager
// ============================================================================

/// Power manager
pub struct PowerManager {
    /// Device power info (device_id -> info)
    device_info: Mutex<BTreeMap<u32, DevicePowerInfo>>,
    /// System power state
    system_state: Mutex<SystemPowerState>,
    /// Wakeup events
    wakeup_events: Mutex<Vec<WakeupEvent>>,
    /// Statistics
    stats: Mutex<PowerStats>,
    /// Runtime PM enabled
    runtime_enabled: AtomicU32,
}

impl PowerManager {
    /// Create new power manager
    pub fn new() -> Self {
        Self {
            device_info: Mutex::new(BTreeMap::new()),
            system_state: Mutex::new(SystemPowerState::Working),
            wakeup_events: Mutex::new(Vec::new()),
            stats: Mutex::new(PowerStats::default()),
            runtime_enabled: AtomicU32::new(1), // Enabled by default
        }
    }

    /// Enable runtime power management
    pub fn enable_runtime_pm(&self) {
        self.runtime_enabled.store(1, Ordering::Release);
        crate::println!("power: runtime PM enabled");
    }

    /// Disable runtime power management
    pub fn disable_runtime_pm(&self) {
        self.runtime_enabled.store(0, Ordering::Release);
        crate::println!("power: runtime PM disabled");
    }

    /// Check if runtime PM is enabled
    pub fn is_runtime_enabled(&self) -> bool {
        self.runtime_enabled.load(Ordering::Acquire) != 0
    }

    /// Register device for power management
    pub fn register_device(&self, info: DevicePowerInfo) -> Result<()> {
        let device_id = info.device_id;

        {
            let mut devices = self.device_info.lock();
            devices.insert(device_id, info.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_devices += 1;
            stats.devices_d0 += 1;
        }

        crate::println!(
            "power: registered device {} for PM (states: {:?})",
            device_id,
            info.supported_states
        );

        Ok(())
    }

    /// Unregister device from power management
    pub fn unregister_device(&self, device_id: u32) -> Result<()> {
        let info = {
            let mut devices = self.device_info.lock();
            devices.remove(&device_id).ok_or(KernelError::NotFound)?
        };

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_devices = stats.total_devices.saturating_sub(1);
            match info.current_state {
                DevicePowerState::D0 => stats.devices_d0 = stats.devices_d0.saturating_sub(1),
                DevicePowerState::D1 => stats.devices_d1 = stats.devices_d1.saturating_sub(1),
                DevicePowerState::D2 => stats.devices_d2 = stats.devices_d2.saturating_sub(1),
                DevicePowerState::D3Hot => stats.devices_d3hot = stats.devices_d3hot.saturating_sub(1),
                DevicePowerState::D3Cold => stats.devices_d3cold = stats.devices_d3cold.saturating_sub(1),
            }
        }

        crate::println!("power: unregistered device {}", device_id);

        Ok(())
    }

    /// Set device power state
    pub fn set_power_state(&self, device_id: u32, new_state: DevicePowerState) -> Result<()> {
        let mut devices = self.device_info.lock();
        let device = devices.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        // Check if state is supported
        if !device.supports_state(new_state) {
            return Err(KernelError::NotSupported);
        }

        let old_state = device.current_state;

        // Update state
        device.current_state = new_state;
        device.increment_transition();

        // Update statistics
        {
            let mut stats = self.stats.lock();
            // Decrement old state count
            match old_state {
                DevicePowerState::D0 => stats.devices_d0 = stats.devices_d0.saturating_sub(1),
                DevicePowerState::D1 => stats.devices_d1 = stats.devices_d1.saturating_sub(1),
                DevicePowerState::D2 => stats.devices_d2 = stats.devices_d2.saturating_sub(1),
                DevicePowerState::D3Hot => stats.devices_d3hot = stats.devices_d3hot.saturating_sub(1),
                DevicePowerState::D3Cold => stats.devices_d3cold = stats.devices_d3cold.saturating_sub(1),
            }
            // Increment new state count
            match new_state {
                DevicePowerState::D0 => stats.devices_d0 += 1,
                DevicePowerState::D1 => stats.devices_d1 += 1,
                DevicePowerState::D2 => stats.devices_d2 += 1,
                DevicePowerState::D3Hot => stats.devices_d3hot += 1,
                DevicePowerState::D3Cold => stats.devices_d3cold += 1,
            }
            stats.total_transitions += 1;
        }

        crate::println!(
            "power: device {} state: {} -> {}",
            device_id,
            old_state.name(),
            new_state.name()
        );

        Ok(())
    }

    /// Get device power state
    pub fn get_power_state(&self, device_id: u32) -> Result<DevicePowerState> {
        let devices = self.device_info.lock();
        let device = devices.get(&device_id).ok_or(KernelError::NotFound)?;
        Ok(device.current_state)
    }

    /// Suspend device (runtime)
    pub fn runtime_suspend(&self, device_id: u32) -> Result<()> {
        let devices = self.device_info.lock();
        let device = devices.get(&device_id).ok_or(KernelError::NotFound)?;

        if !device.runtime_flags.enabled {
            return Err(KernelError::NotSupported);
        }

        drop(devices);

        // Transition to lowest supported state
        self.set_power_state(device_id, DevicePowerState::D3Hot)?;

        crate::println!("power: runtime suspended device {}", device_id);

        Ok(())
    }

    /// Resume device (runtime)
    pub fn runtime_resume(&self, device_id: u32) -> Result<()> {
        let devices = self.device_info.lock();
        let device = devices.get(&device_id).ok_or(KernelError::NotFound)?;

        if !device.runtime_flags.enabled {
            return Err(KernelError::NotSupported);
        }

        drop(devices);

        // Transition to D0
        self.set_power_state(device_id, DevicePowerState::D0)?;

        crate::println!("power: runtime resumed device {}", device_id);

        Ok(())
    }

    /// Suspend all devices for system sleep
    pub fn system_suspend(&self) -> Result<()> {
        crate::println!("power: system suspend started");

        // Update system state
        {
            let mut system_state = self.system_state.lock();
            *system_state = SystemPowerState::SuspendToRam;
        }

        // Suspend all devices
        let device_ids = {
            let devices = self.device_info.lock();
            devices.keys().cloned().collect::<Vec<_>>()
        };

        for device_id in device_ids {
            // Try to suspend to D3Cold first, then D3Hot
            let result = self.set_power_state(device_id, DevicePowerState::D3Cold);
            if result.is_err() {
                let _ = self.set_power_state(device_id, DevicePowerState::D3Hot);
            }
        }

        crate::println!("power: system suspend completed");

        Ok(())
    }

    /// Resume all devices from system sleep
    pub fn system_resume(&self) -> Result<()> {
        crate::println!("power: system resume started");

        // Update system state
        {
            let mut system_state = self.system_state.lock();
            *system_state = SystemPowerState::Working;
        }

        // Resume all devices to D0
        let device_ids = {
            let devices = self.device_info.lock();
            devices.keys().cloned().collect::<Vec<_>>()
        };

        for device_id in device_ids {
            let _ = self.set_power_state(device_id, DevicePowerState::D0);
        }

        crate::println!("power: system resume completed");

        Ok(())
    }

    /// Report wakeup event
    pub fn report_wakeup(&self, event: WakeupEvent) -> Result<()> {
        // Update device wakeup count if applicable
        if let Some(device_id) = event.device_id {
            let devices = self.device_info.lock();
            if let Some(device) = devices.get(&device_id) {
                device.increment_wakeup();
            }

            // Update statistics
            let mut stats = self.stats.lock();
            stats.total_wakeups += 1;
        }

        // Add to wakeup events
        {
            let mut events = self.wakeup_events.lock();
            events.push(event.clone());
        }

        crate::println!(
            "power: wakeup event reported (source: {:?}, device: {:?})",
            event.source,
            event.device_id
        );

        Ok(())
    }

    /// Enable wakeup for device
    pub fn enable_wakeup(&self, device_id: u32) -> Result<()> {
        let mut devices = self.device_info.lock();
        let device = devices.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        device.runtime_flags.can_wakeup = true;

        crate::println!("power: enabled wakeup for device {}", device_id);

        Ok(())
    }

    /// Disable wakeup for device
    pub fn disable_wakeup(&self, device_id: u32) -> Result<()> {
        let mut devices = self.device_info.lock();
        let device = devices.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        device.runtime_flags.can_wakeup = false;

        crate::println!("power: disabled wakeup for device {}", device_id);

        Ok(())
    }

    /// Update device activity (for autosuspend)
    pub fn update_activity(&self, device_id: u32) -> Result<()> {
        let devices = self.device_info.lock();
        let device = devices.get(&device_id).ok_or(KernelError::NotFound)?;

        device.update_activity();

        Ok(())
    }

    /// Get device power info
    pub fn get_device_info(&self, device_id: u32) -> Option<DevicePowerInfo> {
        let devices = self.device_info.lock();
        devices.get(&device_id).cloned()
    }

    /// Get statistics
    pub fn get_stats(&self) -> PowerStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = PowerStats::default();
    }
}

impl Default for PowerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_state() {
        assert!(DevicePowerState::D0.is_operational());
        assert!(!DevicePowerState::D0.is_suspended());
        assert!(!DevicePowerState::D3Hot.is_operational());
        assert!(DevicePowerState::D3Hot.is_suspended());

        assert_eq!(DevicePowerState::D0.power_consumption(), 100);
        assert_eq!(DevicePowerState::D3Cold.power_consumption(), 0);

        assert_eq!(DevicePowerState::D0.wake_latency_us(), 0);
        assert!(DevicePowerState::D2.wake_latency_us() > 0);
    }

    #[test]
    fn test_device_power_info() {
        let supported = vec![
            DevicePowerState::D0,
            DevicePowerState::D3Hot,
            DevicePowerState::D3Cold,
        ];

        let info = DevicePowerInfo::new(100, supported)
            .with_autosuspend_delay(5000);

        assert_eq!(info.device_id, 100);
        assert_eq!(info.current_state, DevicePowerState::D0);
        assert!(info.supports_state(DevicePowerState::D0));
        assert!(info.supports_state(DevicePowerState::D3Hot));
        assert!(!info.supports_state(DevicePowerState::D1));
        assert_eq!(info.autosuspend_delay, 5000);

        info.update_activity();
        assert_eq!(info.get_transitions(), 0);

        info.increment_transition();
        assert_eq!(info.get_transitions(), 1);

        info.increment_wakeup();
        assert_eq!(info.get_wakeup_count(), 1);
    }

    #[test]
    fn test_power_manager() {
        let manager = PowerManager::new();

        let supported = vec![
            DevicePowerState::D0,
            DevicePowerState::D3Hot,
        ];

        let info = DevicePowerInfo::new(100, supported);
        manager.register_device(info).unwrap();

        // Check initial state
        let state = manager.get_power_state(100).unwrap();
        assert_eq!(state, DevicePowerState::D0);

        // Set new state
        manager.set_power_state(100, DevicePowerState::D3Hot).unwrap();
        let state = manager.get_power_state(100).unwrap();
        assert_eq!(state, DevicePowerState::D3Hot);

        // Runtime suspend/resume
        manager.runtime_suspend(100).unwrap();
        manager.runtime_resume(100).unwrap();

        let state = manager.get_power_state(100).unwrap();
        assert_eq!(state, DevicePowerState::D0);

        // Check statistics
        let stats = manager.get_stats();
        assert_eq!(stats.total_devices, 1);
        assert_eq!(stats.devices_d0, 1);
        assert!(stats.total_transitions > 0);
    }

    #[test]
    fn test_wakeup() {
        let manager = PowerManager::new();

        let supported = vec![DevicePowerState::D0, DevicePowerState::D3Hot];
        let info = DevicePowerInfo::new(100, supported);
        manager.register_device(info).unwrap();

        // Enable wakeup
        manager.enable_wakeup(100).unwrap();

        let device_info = manager.get_device_info(100).unwrap();
        assert!(device_info.runtime_flags.can_wakeup);

        // Report wakeup event
        let event = WakeupEvent::new(WakeupSource::PowerButton)
            .with_device(100);
        manager.report_wakeup(event).unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_wakeups, 1);

        let device_info = manager.get_device_info(100).unwrap();
        assert_eq!(device_info.get_wakeup_count(), 1);
    }

    #[test]
    fn test_system_suspend_resume() {
        let manager = PowerManager::new();

        let supported = vec![
            DevicePowerState::D0,
            DevicePowerState::D3Hot,
            DevicePowerState::D3Cold,
        ];

        for i in 0..3 {
            let info = DevicePowerInfo::new(i, supported.clone());
            manager.register_device(info).unwrap();
        }

        // System suspend
        manager.system_suspend().unwrap();

        let stats = manager.get_stats();
        // Devices should be in low power states
        assert!(stats.devices_d0 == 0);

        // System resume
        manager.system_resume().unwrap();

        let stats = manager.get_stats();
        // All devices should be in D0
        assert_eq!(stats.devices_d0, 3);
    }
}
