//! Device Power Management Framework
//!
//! This module implements device-level power management similar to Linux's device PM framework.
//! It provides:
//! - Runtime PM (automatic power management based on usage)
//! - System suspend/resume support
//! - Power domain management
//! - Device power state tracking
//!
//! ## Power States
//!
//! - **D0**: Fully on - Device is fully operational
//! - **D1**: Partial power - Limited functionality, faster wake
//! - **D2**: Low power - Minimal functionality, slower wake
//! - **D3hot**: Standby - Powered but not operational
//! - **D3cold**: Off - No power
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::power::dev_pm::{DevicePower, DeviceState};
//!
//! // Create device power manager
//! let mut dev_pm = DevicePower::new("eth0")?;
//!
//! // Enable runtime PM
//! dev_pm.enable_runtime_pm()?;
//!
//! // Suspend device
//! dev_pm.runtime_suspend()?;
//! ```

use crate::error::UnifiedError;
use core::sync::atomic {AtomicBool, AtomicU32, Ordering, Ordering};
use alloc::vec::Vec;
use alloc::boxed::Box;

/// Device name
pub type DeviceName = &'static str;

/// Device identifier
pub type DeviceId = u32;

/// Power state transition latency in microseconds
pub type LatencyUs = u32;

/// Device power state (D0-D3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceState {
    /// D3: Off (no power)
    Off = 0,
    /// D3hot: Standby (powered but not operational)
    Standby = 1,
    /// D2: Low power (minimal functionality)
    LowPower = 2,
    /// D1: Partial power (limited functionality)
    Partial = 3,
    /// D0: Fully on (fully operational)
    On = 4,
}

impl DeviceState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            DeviceState::Off => "D3cold",
            DeviceState::Standby => "D3hot",
            DeviceState::LowPower => "D2",
            DeviceState::Partial => "D1",
            DeviceState::On => "D0",
        }
    }

    /// Check if device is operational
    pub fn is_operational(&self) -> bool {
        *self == DeviceState::On
    }

    /// Check if device has power
    pub fn has_power(&self) -> bool {
        *self != DeviceState::Off
    }
}

/// Runtime PM status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePmStatus {
    /// Runtime PM is active
    Active,
    /// Runtime PM is suspended
    Suspended,
    /// Runtime PM is disabled
    Disabled,
}

/// Device power management callbacks
pub trait DevicePmOps: Send + Sync {
    /// Suspend device callback
    fn suspend(&mut self) -> Result<(), UnifiedError> {
        Ok(())
    }

    /// Resume device callback
    fn resume(&mut self) -> Result<(), UnifiedError> {
        Ok(())
    }

    /// Runtime suspend callback
    fn runtime_suspend(&mut self) -> Result<(), UnifiedError> {
        Ok(())
    }

    /// Runtime resume callback
    fn runtime_resume(&mut self) -> Result<(), UnifiedError> {
        Ok(())
    }

    /// Set power state callback
    fn set_power_state(&mut self, _state: DeviceState) -> Result<(), UnifiedError> {
        Ok(())
    }

    /// Get current power consumption (milliwatts)
    fn get_power_consumption(&self) -> u32 {
        100 // Default 100mW
    }
}

/// Default empty PM operations
pub struct DefaultPmOps;

impl DevicePmOps for DefaultPmOps {}

/// Device power management statistics
#[derive(Debug)]
pub struct DevicePmStats {
    /// Number of suspend operations
    suspend_count: AtomicU32,
    /// Number of resume operations
    resume_count: AtomicU32,
    /// Total time in active state (milliseconds)
    active_time_ms: AtomicU32,
    /// Total time in suspended state (milliseconds)
    suspended_time_ms: AtomicU32,
    /// Last state change time
    last_state_change: AtomicU32,
}

impl DevicePmStats {
    /// Create new statistics
    pub fn new() -> Self {
        DevicePmStats {
            suspend_count: AtomicU32::new(0),
            resume_count: AtomicU32::new(0),
            active_time_ms: AtomicU32::new(0),
            suspended_time_ms: AtomicU32::new(0),
            last_state_change: AtomicU32::new(0),
        }
    }

    /// Get suspend count
    pub fn suspend_count(&self) -> u32 {
        self.suspend_count.load(Ordering::Relaxed)
    }

    /// Get resume count
    pub fn resume_count(&self) -> u32 {
        self.resume_count.load(Ordering::Relaxed)
    }

    /// Get active time
    pub fn active_time_ms(&self) -> u32 {
        self.active_time_ms.load(Ordering::Relaxed)
    }

    /// Get suspended time
    pub fn suspended_time_ms(&self) -> u32 {
        self.suspended_time_ms.load(Ordering::Relaxed)
    }

    /// Record suspend operation
    fn record_suspend(&self) {
        self.suspend_count.fetch_add(1, Ordering::Relaxed);
        self.last_state_change
            .store(self.get_timestamp(), Ordering::Relaxed);
    }

    /// Record resume operation
    fn record_resume(&self) {
        self.resume_count.fetch_add(1, Ordering::Relaxed);
        self.last_state_change
            .store(self.get_timestamp(), Ordering::Relaxed);
    }

    /// Update active time
    fn update_active_time(&self, duration_ms: u32) {
        self.active_time_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Update suspended time
    fn update_suspended_time(&self, duration_ms: u32) {
        self.suspended_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Get timestamp (milliseconds since boot)
    fn get_timestamp(&self) -> u32 {
        // In a real implementation, this would read from a timer
        0
    }
}

/// Device power management instance
pub struct DevicePower {
    /// Device name
    name: DeviceName,
    /// Device ID
    id: DeviceId,
    /// Current power state
    state: DeviceState,
    /// Target power state
    target_state: DeviceState,
    /// Runtime PM status
    runtime_status: RuntimePmStatus,
    /// Runtime PM enabled flag
    runtime_enabled: AtomicBool,
    /// Usage counter for runtime PM
    usage_count: AtomicU32,
    /// Auto-suspend enabled flag
    autosuspend_enabled: AtomicBool,
    /// Auto-suspend delay (milliseconds)
    autosuspend_delay_ms: u32,
    /// Statistics
    stats: DevicePmStats,
    /// PM operations
    pm_ops: Option<Box<dyn DevicePmOps>>,
}

impl core::fmt::Debug for DevicePower {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DevicePower")
            .field("name", &self.name)
            .field("id", &self.id)
            .field("state", &self.state)
            .field("target_state", &self.target_state)
            .field("runtime_status", &self.runtime_status)
            .field("runtime_enabled", &self.runtime_enabled)
            .field("usage_count", &self.usage_count)
            .field("autosuspend_enabled", &self.autosuspend_enabled)
            .field("autosuspend_delay_ms", &self.autosuspend_delay_ms)
            .field("stats", &self.stats)
            .field("pm_ops", &self.pm_ops.is_some())
            .finish()
    }
}

impl DevicePower {
    /// Create new device power manager
    pub fn new(name: DeviceName, id: DeviceId) -> Self {
        DevicePower {
            name,
            id,
            state: DeviceState::On,
            target_state: DeviceState::On,
            runtime_status: RuntimePmStatus::Active,
            runtime_enabled: AtomicBool::new(false),
            usage_count: AtomicU32::new(0),
            autosuspend_enabled: AtomicBool::new(false),
            autosuspend_delay_ms: 1000,
            stats: DevicePmStats::new(),
            pm_ops: None,
        }
    }

    /// Get device name
    pub fn name(&self) -> DeviceName {
        self.name
    }

    /// Get device ID
    pub fn id(&self) -> DeviceId {
        self.id
    }

    /// Get current power state
    pub fn state(&self) -> DeviceState {
        self.state
    }

    /// Get runtime PM status
    pub fn runtime_status(&self) -> RuntimePmStatus {
        self.runtime_status
    }

    /// Get statistics
    pub fn stats(&self) -> &DevicePmStats {
        &self.stats
    }

    /// Set PM operations
    pub fn set_pm_ops(&mut self, ops: Box<dyn DevicePmOps>) {
        self.pm_ops = Some(ops);
    }

    /// Enable runtime PM
    pub fn enable_runtime_pm(&mut self) -> Result<(), UnifiedError> {
        if self.runtime_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.runtime_enabled.store(true, Ordering::Relaxed);
        self.runtime_status = RuntimePmStatus::Active;
        Ok(())
    }

    /// Disable runtime PM
    pub fn disable_runtime_pm(&mut self) -> Result<(), UnifiedError> {
        if !self.runtime_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Resume if suspended
        if self.runtime_status == RuntimePmStatus::Suspended {
            self.runtime_resume()?;
        }

        self.runtime_enabled.store(false, Ordering::Relaxed);
        self.runtime_status = RuntimePmStatus::Disabled;
        Ok(())
    }

    /// Check if runtime PM is enabled
    pub fn is_runtime_enabled(&self) -> bool {
        self.runtime_enabled.load(Ordering::Relaxed)
    }

    /// Get usage counter
    pub fn usage_count(&self) -> u32 {
        self.usage_count.load(Ordering::Relaxed)
    }

    /// Increment usage counter (prevent auto-suspend)
    pub fn get(&self) {
        self.usage_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement usage counter (may allow auto-suspend)
    pub fn put(&self) {
        self.usage_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Enable auto-suspend
    pub fn enable_autosuspend(&mut self, delay_ms: u32) {
        self.autosuspend_enabled.store(true, Ordering::Relaxed);
        self.autosuspend_delay_ms = delay_ms;
    }

    /// Disable auto-suspend
    pub fn disable_autosuspend(&mut self) {
        self.autosuspend_enabled.store(false, Ordering::Relaxed);
    }

    /// Check if auto-suspend is enabled
    pub fn autosuspend_enabled(&self) -> bool {
        self.autosuspend_enabled.load(Ordering::Relaxed)
    }

    /// Set power state
    pub fn set_power_state(&mut self, state: DeviceState) -> Result<(), UnifiedError> {
        if state == self.state {
            return Ok(());
        }

        self.target_state = state;

        // Call PM operations if available
        if let Some(ops) = &mut self.pm_ops {
            let ops: &mut dyn DevicePmOps = ops.as_mut();
            ops.set_power_state(state)?;
        }

        self.state = state;
        Ok(())
    }

    /// Runtime suspend
    pub fn runtime_suspend(&mut self) -> Result<(), UnifiedError> {
        if !self.runtime_enabled.load(Ordering::Relaxed) {
            return Err(UnifiedError::Other("Runtime PM not enabled".into()));
        }

        if self.runtime_status != RuntimePmStatus::Active {
            return Ok(());
        }

        if self.usage_count.load(Ordering::Relaxed) > 0 {
            return Err(UnifiedError::Other(
                "Device in use, cannot suspend".into(),
            ));
        }

        // Call PM operations
        if let Some(ops) = &mut self.pm_ops {
            let ops: &mut dyn DevicePmOps = ops.as_mut();
            ops.runtime_suspend()?;
        }

        // Set to low power state
        self.set_power_state(DeviceState::LowPower)?;

        self.runtime_status = RuntimePmStatus::Suspended;
        self.stats.record_suspend();

        Ok(())
    }

    /// Runtime resume
    pub fn runtime_resume(&mut self) -> Result<(), UnifiedError> {
        if !self.runtime_enabled.load(Ordering::Relaxed) {
            return Err(UnifiedError::Other("Runtime PM not enabled".into()));
        }

        if self.runtime_status != RuntimePmStatus::Suspended {
            return Ok(());
        }

        // Set to on state
        self.set_power_state(DeviceState::On)?;

        // Call PM operations
        if let Some(ops) = &mut self.pm_ops {
            let ops: &mut dyn DevicePmOps = ops.as_mut();
            ops.runtime_resume()?;
        }

        self.runtime_status = RuntimePmStatus::Active;
        self.stats.record_resume();

        Ok(())
    }

    /// System suspend
    pub fn suspend(&mut self) -> Result<(), UnifiedError> {
        if self.state == DeviceState::Off {
            return Ok(());
        }

        // Disable runtime PM temporarily
        let was_runtime_enabled = self.runtime_enabled.load(Ordering::Relaxed);
        if was_runtime_enabled {
            self.runtime_enabled.store(false, Ordering::Relaxed);
        }

        // Call PM operations
        if let Some(ops) = &mut self.pm_ops {
            let ops: &mut dyn DevicePmOps = ops.as_mut();
            ops.suspend()?;
        }

        // Set to low power state
        self.set_power_state(DeviceState::Standby)?;
        self.stats.record_suspend();

        Ok(())
    }

    /// System resume
    pub fn resume(&mut self) -> Result<(), UnifiedError> {
        if self.state == DeviceState::On {
            return Ok(());
        }

        // Set to on state
        self.set_power_state(DeviceState::On)?;

        // Call PM operations
        if let Some(ops) = &mut self.pm_ops {
            let ops: &mut dyn DevicePmOps = ops.as_mut();
            ops.resume()?;
        }

        // Re-enable runtime PM if it was enabled
        if !self.runtime_enabled.load(Ordering::Relaxed) {
            self.runtime_enabled.store(true, Ordering::Relaxed);
        }

        self.runtime_status = RuntimePmStatus::Active;
        self.stats.record_resume();

        Ok(())
    }

    /// Get power consumption (milliwatts)
    pub fn get_power_consumption(&self) -> u32 {
        if let Some(ops) = &self.pm_ops {
            let ops: &dyn DevicePmOps = ops.as_ref();
            ops.get_power_consumption()
        } else {
            // Estimate based on state
            match self.state {
                DeviceState::On => 100,
                DeviceState::Partial => 60,
                DeviceState::LowPower => 30,
                DeviceState::Standby => 10,
                DeviceState::Off => 0,
            }
        }
    }
}

/// Power domain - groups devices that share a power rail
#[derive(Debug)]
pub struct PowerDomain {
    /// Domain name
    name: DeviceName,
    /// Domain ID
    id: u32,
    /// Devices in this domain
    devices: Vec<DevicePower>,
    /// Domain power state
    state: DeviceState,
}

impl PowerDomain {
    /// Create new power domain
    pub fn new(name: DeviceName, id: u32) -> Self {
        PowerDomain {
            name,
            id,
            devices: Vec::new(),
            state: DeviceState::On,
        }
    }

    /// Get domain name
    pub fn name(&self) -> DeviceName {
        self.name
    }

    /// Get domain ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get current state
    pub fn state(&self) -> DeviceState {
        self.state
    }

    /// Add device to domain
    pub fn add_device(&mut self, device: DevicePower) {
        self.devices.push(device);
    }

    /// Get devices
    pub fn devices(&self) -> &[DevicePower] {
        &self.devices
    }

    /// Get devices (mutable)
    pub fn devices_mut(&mut self) -> &mut [DevicePower] {
        &mut self.devices
    }

    /// Suspend entire domain
    pub fn suspend(&mut self) -> Result<(), UnifiedError> {
        // Suspend all devices
        for device in &mut self.devices {
            device.suspend()?;
        }

        self.state = DeviceState::Standby;
        Ok(())
    }

    /// Resume entire domain
    pub fn resume(&mut self) -> Result<(), UnifiedError> {
        // Resume all devices
        for device in &mut self.devices {
            device.resume()?;
        }

        self.state = DeviceState::On;
        Ok(())
    }

    /// Calculate total power consumption (milliwatts)
    pub fn total_power_consumption(&self) -> u32 {
        self.devices.iter().map(|d| d.get_power_consumption()).sum()
    }
}

/// Device power manager - manages all devices and power domains
#[derive(Debug)]
pub struct DevicePowerManager {
    /// All devices
    devices: Vec<DevicePower>,
    /// Power domains
    domains: Vec<PowerDomain>,
    /// Next device ID
    next_device_id: DeviceId,
}

impl DevicePowerManager {
    /// Create new device power manager
    pub fn new() -> Self {
        DevicePowerManager {
            devices: Vec::new(),
            domains: Vec::new(),
            next_device_id: 0,
        }
    }

    /// Register device
    pub fn register_device(&mut self, name: DeviceName) -> Result<DeviceId, UnifiedError> {
        let id = self.next_device_id;
        self.next_device_id += 1;

        let device = DevicePower::new(name, id);
        self.devices.push(device);

        Ok(id)
    }

    /// Unregister device
    pub fn unregister_device(&mut self, id: DeviceId) -> Result<(), UnifiedError> {
        let idx = self
            .devices
            .iter()
            .position(|d| d.id() == id)
            .ok_or_else(|| UnifiedError::Other(format!("Device {} not found", id)))?;

        self.devices.remove(idx);
        Ok(())
    }

    /// Get device
    pub fn get_device(&self, id: DeviceId) -> Option<&DevicePower> {
        self.devices.iter().find(|d| d.id() == id)
    }

    /// Get device (mutable)
    pub fn get_device_mut(&mut self, id: DeviceId) -> Option<&mut DevicePower> {
        self.devices.iter_mut().find(|d| d.id() == id)
    }

    /// Create power domain
    pub fn create_domain(&mut self, name: DeviceName) -> Result<u32, UnifiedError> {
        let id = self.domains.len() as u32;
        let domain = PowerDomain::new(name, id);
        self.domains.push(domain);
        Ok(id)
    }

    /// Get domain
    pub fn get_domain(&self, id: u32) -> Option<&PowerDomain> {
        self.domains.get(id as usize)
    }

    /// Get domain (mutable)
    pub fn get_domain_mut(&mut self, id: u32) -> Option<&mut PowerDomain> {
        self.domains.get_mut(id as usize)
    }

    /// Suspend all devices
    pub fn suspend_all(&mut self) -> Result<(), UnifiedError> {
        for device in &mut self.devices {
            device.suspend()?;
        }
        Ok(())
    }

    /// Resume all devices
    pub fn resume_all(&mut self) -> Result<(), UnifiedError> {
        for device in &mut self.devices {
            device.resume()?;
        }
        Ok(())
    }

    /// Calculate total system power consumption (milliwatts)
    pub fn total_power_consumption(&self) -> u32 {
        self.devices.iter().map(|d| d.get_power_consumption()).sum()
    }

    /// Get all devices in runtime suspended state
    pub fn get_suspended_devices(&self) -> Vec<&DevicePower> {
        self.devices
            .iter()
            .filter(|d| d.runtime_status() == RuntimePmStatus::Suspended)
            .collect()
    }

    /// Get all devices in active state
    pub fn get_active_devices(&self) -> Vec<&DevicePower> {
        self.devices
            .iter()
            .filter(|d| d.runtime_status() == RuntimePmStatus::Active)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_power_creation() {
        let device = DevicePower::new("test", 0);
        assert_eq!(device.name(), "test");
        assert_eq!(device.id(), 0);
        assert_eq!(device.state(), DeviceState::On);
    }

    #[test]
    fn test_power_state_transition() {
        let mut device = DevicePower::new("test", 0);
        device.set_power_state(DeviceState::LowPower).unwrap();
        assert_eq!(device.state(), DeviceState::LowPower);
    }

    #[test]
    fn test_runtime_pm() {
        let mut device = DevicePower::new("test", 0);
        device.enable_runtime_pm().unwrap();
        assert!(device.is_runtime_enabled());

        device.runtime_suspend().unwrap();
        assert_eq!(device.runtime_status(), RuntimePmStatus::Suspended);

        device.runtime_resume().unwrap();
        assert_eq!(device.runtime_status(), RuntimePmStatus::Active);
    }

    #[test]
    fn test_usage_counter() {
        let device = DevicePower::new("test", 0);
        device.enable_runtime_pm().unwrap();

        assert_eq!(device.usage_count(), 0);

        device.get();
        assert_eq!(device.usage_count(), 1);

        device.put();
        assert_eq!(device.usage_count(), 0);
    }

    #[test]
    fn test_suspend_with_usage() {
        let mut device = DevicePower::new("test", 0);
        device.enable_runtime_pm().unwrap();

        device.get();
        assert!(device.runtime_suspend().is_err());

        device.put();
        assert!(device.runtime_suspend().is_ok());
    }

    #[test]
    fn test_power_domain() {
        let mut domain = PowerDomain::new("test_domain", 0);
        let device1 = DevicePower::new("dev1", 0);
        let device2 = DevicePower::new("dev2", 1);

        domain.add_device(device1);
        domain.add_device(device2);

        assert_eq!(domain.devices().len(), 2);

        domain.suspend().unwrap();
        assert_eq!(domain.state(), DeviceState::Standby);

        domain.resume().unwrap();
        assert_eq!(domain.state(), DeviceState::On);
    }

    #[test]
    fn test_device_power_manager() {
        let mut manager = DevicePowerManager::new();

        let id1 = manager.register_device("eth0").unwrap();
        let id2 = manager.register_device("wifi0").unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);

        assert!(manager.get_device(id1).is_some());
        assert!(manager.get_device(id2).is_some());
    }
}
