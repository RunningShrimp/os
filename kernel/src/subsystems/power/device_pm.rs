//! Device Power Management
//!
//! This module provides device power state management (D-States) for controlling
//! the power consumption of individual devices.

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use spin::Mutex;

/// Device power states (D-States)
///
/// Defines the power consumption states of a device, from fully operational (D0)
/// to deep sleep (D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DevicePowerState {
    /// D0: Fully operational - device is fully powered and functional
    D0 = 0,

    /// D1: Low power state - device context is preserved, but may have reduced functionality
    D1 = 1,

    /// D2: Lower power state - more power savings than D1, but slower wake time
    D2 = 2,

    /// D3hot: Deep sleep - device is off but powered, requires full reinitialization
    D3hot = 3,

    /// D3cold: Off - device is completely powered off
    D3cold = 4,
}

impl DevicePowerState {
    /// Check if the device is fully operational
    pub fn is_operational(&self) -> bool {
        matches!(self, DevicePowerState::D0)
    }

    /// Check if the device is in a low power state
    pub fn is_low_power(&self) -> bool {
        matches!(self, DevicePowerState::D1 | DevicePowerState::D2)
    }

    /// Check if the device is sleeping or off
    pub fn is_sleeping(&self) -> bool {
        matches!(self, DevicePowerState::D3hot | DevicePowerState::D3cold)
    }

    /// Get the state as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            DevicePowerState::D0 => "D0",
            DevicePowerState::D1 => "D1",
            DevicePowerState::D2 => "D2",
            DevicePowerState::D3hot => "D3hot",
            DevicePowerState::D3cold => "D3cold",
        }
    }

    /// Get power consumption estimate (percentage of D0 power)
    pub fn power_consumption(&self) -> u8 {
        match self {
            DevicePowerState::D0 => 100,
            DevicePowerState::D1 => 50,
            DevicePowerState::D2 => 25,
            DevicePowerState::D3hot => 5,
            DevicePowerState::D3cold => 0,
        }
    }

    /// Get wake latency estimate (in microseconds)
    pub fn wake_latency_us(&self) -> u64 {
        match self {
            DevicePowerState::D0 => 0,
            DevicePowerState::D1 => 100,
            DevicePowerState::D2 => 1000,
            DevicePowerState::D3hot => 10000,
            DevicePowerState::D3cold => 100000,
        }
    }
}

/// Error type for device power management operations
#[derive(Debug)]
pub enum PowerError {
    /// Transition not supported
    TransitionNotSupported,

    /// Device busy
    DeviceBusy,

    /// Invalid state
    InvalidState,

    /// Hardware error
    HardwareError,

    /// Timeout
    Timeout,
}

/// Trait for power-manageable devices
///
/// Devices implementing this trait can be controlled by the power manager
/// to reduce power consumption when idle.
pub trait PowerManaged: Send + Sync {
    /// Get device name
    fn name(&self) -> &str;

    /// Set power state
    ///
    /// Transitions the device to the specified power state.
    /// Returns an error if the transition is not supported or fails.
    fn set_power_state(&mut self, state: DevicePowerState) -> Result<(), PowerError>;

    /// Get current power state
    fn get_power_state(&self) -> DevicePowerState;

    /// Wake up the device
    ///
    /// Convenience method to transition the device to D0.
    fn wakeup(&mut self) -> Result<(), PowerError> {
        self.set_power_state(DevicePowerState::D0)
    }

    /// Suspend the device
    ///
    /// Convenience method to transition the device to D3hot.
    fn suspend(&mut self) -> Result<(), PowerError> {
        self.set_power_state(DevicePowerState::D3hot)
    }

    /// Check if device supports a specific power state
    fn supports_state(&self, state: DevicePowerState) -> bool;

    /// Get supported power states
    fn supported_states(&self) -> &[DevicePowerState];
}

/// Power policy for device management
#[derive(Debug, Clone, Copy)]
pub enum PowerPolicy {
    /// Performance mode - keep all devices at D0
    Performance,

    /// Balanced mode - allow idle devices to enter low power states
    Balanced,

    /// Powersave mode - aggressively reduce power consumption
    Powersave,
}

/// Power manager for device power control
///
/// Manages the power states of all registered devices and applies
/// power policies based on system state.
pub struct PowerManager {
    devices: Mutex<Vec<Box<dyn PowerManaged>>>,
    policy: Mutex<PowerPolicy>,
    idle_threshold_ms: u64,
}

impl PowerManager {
    /// Create a new power manager
    ///
    /// # Arguments
    ///
    /// * `idle_threshold_ms` - Time in milliseconds before considering a device idle
    pub fn new(idle_threshold_ms: u64) -> Self {
        Self {
            devices: Mutex::new(Vec::new()),
            policy: Mutex::new(PowerPolicy::Balanced),
            idle_threshold_ms,
        }
    }

    /// Register a device with the power manager
    ///
    /// # Arguments
    ///
    /// * `device` - Device to register
    pub fn register_device(&self, device: Box<dyn PowerManaged>) {
        let mut devices = self.devices.lock();
        log::info!("PowerManager: Registered device {}", device.name());
        devices.push(device);
    }

    /// Unregister a device by name
    ///
    /// # Arguments
    ///
    /// * `name` - Device name to unregister
    pub fn unregister_device(&self, name: &str) -> bool {
        let mut devices = self.devices.lock();
        if let Some(pos) = devices.iter().position(|d| d.name() == name) {
            let device = devices.remove(pos);
            log::info!("PowerManager: Unregistered device {}", device.name());
            true
        } else {
            false
        }
    }

    /// Get number of registered devices
    pub fn device_count(&self) -> usize {
        self.devices.lock().len()
    }

    /// Set power policy
    ///
    /// # Arguments
    ///
    /// * `policy` - Power policy to apply
    pub fn set_policy(&self, policy: PowerPolicy) {
        let mut current = self.policy.lock();
        *current = policy;
        log::info!("PowerManager: Policy changed to {:?}", policy);
    }

    /// Get current power policy
    pub fn get_policy(&self) -> PowerPolicy {
        *self.policy.lock()
    }

    /// Put all devices to sleep
    ///
    /// Transitions all non-essential devices to D3hot for power savings.
    pub fn idle(&self) {
        let policy = *self.policy.lock();

        match policy {
            PowerPolicy::Performance => {
                // Keep devices at D0 for performance
                return;
            }
            PowerPolicy::Balanced => {
                self.set_all_state(DevicePowerState::D3hot);
            }
            PowerPolicy::Powersave => {
                // Aggressive power saving
                self.set_all_state(DevicePowerState::D3cold);
            }
        }
    }

    /// Wake up all devices
    ///
    /// Transitions all devices to D0 for full operation.
    pub fn busy(&self) {
        self.set_all_state(DevicePowerState::D0);
    }

    /// Set all devices to a specific power state
    ///
    /// # Arguments
    ///
    /// * `state` - Target power state
    fn set_all_state(&self, state: DevicePowerState) {
        let mut devices = self.devices.lock();
        for device in devices.iter_mut() {
            if device.supports_state(state) {
                if let Err(e) = device.set_power_state(state) {
                    log::warn!(
                        "PowerManager: Failed to set {} to {:?}: {:?}",
                        device.name(),
                        state,
                        e
                    );
                }
            }
        }
    }

    /// Set specific device power state
    ///
    /// # Arguments
    ///
    /// * `name` - Device name
    /// * `state` - Target power state
    pub fn set_device_state(&self, name: &str, state: DevicePowerState) -> Result<(), PowerError> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.iter_mut().find(|d| d.name() == name) {
            if !device.supports_state(state) {
                return Err(PowerError::TransitionNotSupported);
            }
            device.set_power_state(state)
        } else {
            Err(PowerError::InvalidState)
        }
    }

    /// Get device power state
    ///
    /// # Arguments
    ///
    /// * `name` - Device name
    pub fn get_device_state(&self, name: &str) -> Option<DevicePowerState> {
        let devices = self.devices.lock();
        devices.iter().find(|d| d.name() == name).map(|d| d.get_power_state())
    }

    /// Suspend all devices (before system sleep)
    ///
    /// Returns the number of devices successfully suspended
    pub fn suspend_all(&self) -> usize {
        let mut devices = self.devices.lock();
        let mut count = 0;

        for device in devices.iter_mut() {
            if device.set_power_state(DevicePowerState::D3hot).is_ok() {
                count += 1;
            } else {
                log::warn!("PowerManager: Failed to suspend {}", device.name());
            }
        }

        log::info!("PowerManager: Suspended {} devices", count);
        count
    }

    /// Resume all devices (after system wake)
    ///
    /// Returns the number of devices successfully resumed
    pub fn resume_all(&self) -> usize {
        let mut devices = self.devices.lock();
        let mut count = 0;

        for device in devices.iter_mut() {
            if device.set_power_state(DevicePowerState::D0).is_ok() {
                count += 1;
            } else {
                log::warn!("PowerManager: Failed to resume {}", device.name());
            }
        }

        log::info!("PowerManager: Resumed {} devices", count);
        count
    }

    /// Get total power consumption estimate (percentage of max power)
    pub fn total_power_consumption(&self) -> u8 {
        let devices = self.devices.lock();
        let total: u32 = devices.iter().map(|d| d.get_power_state().power_consumption() as u32).sum();

        if devices.is_empty() {
            return 0;
        }

        ((total as usize) / devices.len()) as u8
    }

    /// Get devices in a specific power state
    pub fn devices_in_state(&self, state: DevicePowerState) -> Vec<String> {
        let devices = self.devices.lock();
        devices
            .iter()
            .filter(|d| d.get_power_state() == state)
            .map(|d| d.name().to_string())
            .collect()
    }
}

/// Example power-manageable device
#[derive(Debug)]
pub struct ExamplePowerDevice {
    name: String,
    current_state: DevicePowerState,
    supported_states: Vec<DevicePowerState>,
}

impl ExamplePowerDevice {
    /// Create a new example device
    ///
    /// # Arguments
    ///
    /// * `name` - Device name
    /// * `supported_states` - Supported power states
    pub fn new(name: &str, supported_states: Vec<DevicePowerState>) -> Self {
        Self {
            name: String::from(name),
            current_state: DevicePowerState::D0,
            supported_states,
        }
    }
}

impl PowerManaged for ExamplePowerDevice {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_power_state(&mut self, state: DevicePowerState) -> Result<(), PowerError> {
        if !self.supported_states.contains(&state) {
            return Err(PowerError::TransitionNotSupported);
        }

        log::info!(
            "Device {}: {:?} -> {:?}",
            self.name,
            self.current_state,
            state
        );

        self.current_state = state;
        Ok(())
    }

    fn get_power_state(&self) -> DevicePowerState {
        self.current_state
    }

    fn supports_state(&self, state: DevicePowerState) -> bool {
        self.supported_states.contains(&state)
    }

    fn supported_states(&self) -> &[DevicePowerState] {
        &self.supported_states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_power_state() {
        assert!(DevicePowerState::D0.is_operational());
        assert!(DevicePowerState::D1.is_low_power());
        assert!(DevicePowerState::D3hot.is_sleeping());

        assert_eq!(DevicePowerState::D0.power_consumption(), 100);
        assert_eq!(DevicePowerState::D3cold.power_consumption(), 0);

        assert_eq!(DevicePowerState::D0.wake_latency_us(), 0);
        assert_eq!(DevicePowerState::D3cold.wake_latency_us(), 100000);
    }

    #[test]
    fn test_power_manager() {
        let manager = PowerManager::new(1000);

        let device = ExamplePowerDevice::new(
            "test_device",
            vec![
                DevicePowerState::D0,
                DevicePowerState::D1,
                DevicePowerState::D3hot,
            ],
        );

        manager.register_device(Box::new(device));
        assert_eq!(manager.device_count(), 1);

        manager.set_policy(PowerPolicy::Balanced);
        assert_eq!(manager.get_policy(), PowerPolicy::Balanced);

        manager.idle();
        assert_eq!(
            manager.get_device_state("test_device"),
            Some(DevicePowerState::D3hot)
        );

        manager.busy();
        assert_eq!(
            manager.get_device_state("test_device"),
            Some(DevicePowerState::D0)
        );
    }

    #[test]
    fn test_example_device() {
        let mut device = ExamplePowerDevice::new(
            "eth0",
            vec![
                DevicePowerState::D0,
                DevicePowerState::D1,
                DevicePowerState::D3hot,
            ],
        );

        assert_eq!(device.get_power_state(), DevicePowerState::D0);
        assert!(device.supports_state(DevicePowerState::D0));

        assert!(device.set_power_state(DevicePowerState::D3hot).is_ok());
        assert_eq!(device.get_power_state(), DevicePowerState::D3hot);

        assert!(device.set_power_state(DevicePowerState::D3cold).is_err());
    }

    #[test]
    fn test_power_consumption() {
        let manager = PowerManager::new(1000);

        let device1 = ExamplePowerDevice::new("dev1", vec![DevicePowerState::D0, DevicePowerState::D3hot]);
        let device2 = ExamplePowerDevice::new("dev2", vec![DevicePowerState::D0, DevicePowerState::D3hot]);

        manager.register_device(Box::new(device1));
        manager.register_device(Box::new(device2));

        // Both at D0 (100% power)
        assert_eq!(manager.total_power_consumption(), 100);

        // Set one to D3hot (5% power)
        manager.set_device_state("dev1", DevicePowerState::D3hot).ok();
        assert_eq!(manager.total_power_consumption(), 52); // (100 + 5) / 2
    }
}
