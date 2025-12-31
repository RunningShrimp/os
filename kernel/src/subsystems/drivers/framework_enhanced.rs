//! Enhanced Driver Framework
//!
//! This module provides an enhanced driver framework with advanced features,
//! including:
//! - Enhanced driver registration and matching
//! - Improved device lifecycle management
//! - Enhanced error handling and recovery
//! - Driver capabilities and requirements
//! - Dependency management
//! - Driver telemetry and monitoring

use crate::prelude::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::Mutex;
use core::sync::atomic {AtomicU32, AtomicU64, Ordering, Ordering};

// ============================================================================
// Driver Capability Flags
// ============================================================================

/// Driver capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverCapabilities {
    /// Driver supports MSI
    pub msi: bool,
    /// Driver supports MSI-X
    pub msix: bool,
    /// Driver supports DMA
    pub dma: bool,
    /// Driver supports scatter-gather DMA
    pub sg_dma: bool,
    /// Driver supports hotplug
    pub hotplug: bool,
    /// Driver supports runtime PM
    pub runtime_pm: bool,
    /// Driver supports system sleep PM
    pub system_pm: bool,
    /// Driver supports wakeup
    pub wakeup: bool,
    /// Driver supports interrupts
    pub interrupts: bool,
    /// Driver supports polling mode
    pub polling: bool,
    /// Driver supports async I/O
    pub async_io: bool,
    /// Driver supports memory mapping
    pub mmap: bool,
    /// Driver is privileged
    pub privileged: bool,
    /// Reserved flags
    pub reserved: u32,
}

impl Default for DriverCapabilities {
    fn default() -> Self {
        Self {
            msi: false,
            msix: false,
            dma: false,
            sg_dma: false,
            hotplug: false,
            runtime_pm: false,
            system_pm: false,
            wakeup: false,
            interrupts: true,
            polling: false,
            async_io: false,
            mmap: false,
            privileged: false,
            reserved: 0,
        }
    }
}

/// Driver requirements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverRequirements {
    /// Minimum firmware version
    pub min_fw_version: Option<String>,
    /// Required kernel version
    pub min_kernel_version: Option<String>,
    /// Required other drivers (dependencies)
    pub dependencies: Vec<String>,
    /// Exclusive access required
    pub exclusive: bool,
    /// Physical address requirements
    pub phys_addr: Option<u64>,
    /// I/O port requirements
    pub io_ports: Option<(u16, u16)>, // (start, count)
    /// Memory size requirement
    pub memory_size: Option<usize>,
    /// IRQ requirement
    pub irq_required: bool,
}

impl Default for DriverRequirements {
    fn default() -> Self {
        Self {
            min_fw_version: None,
            min_kernel_version: None,
            dependencies: Vec::new(),
            exclusive: false,
            phys_addr: None,
            io_ports: None,
            memory_size: None,
            irq_required: false,
        }
    }
}

// ============================================================================
// Driver Registration
// ============================================================================

/// Driver registration status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverRegistrationStatus {
    /// Driver is not registered
    NotRegistered,
    /// Driver is registered but not initialized
    Registered,
    /// Driver is initialized and ready
    Initialized,
    /// Driver initialization failed
    InitFailed,
    /// Driver is being unregistered
    Unregistering,
}

/// Driver descriptor
#[derive(Debug)]
pub struct DriverDescriptor {
    /// Driver name
    pub name: String,
    /// Driver version
    pub version: String,
    /// Driver author
    pub author: String,
    /// Driver description
    pub description: String,
    /// Supported device types (vendor_id, device_id)
    pub supported_devices: Vec<(u16, u16)>,
    /// Driver capabilities
    pub capabilities: DriverCapabilities,
    /// Driver requirements
    pub requirements: DriverRequirements,
    /// Driver priority (for matching)
    pub priority: i32,
    /// Registration status
    pub status: DriverRegistrationStatus,
    /// Registration timestamp
    pub registration_time: u64,
    /// Device count
    pub device_count: AtomicU32,
    /// Error count
    pub error_count: AtomicU64,
}

// Manual implementation of Clone for DriverDescriptor
impl Clone for DriverDescriptor {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            version: self.version.clone(),
            author: self.author.clone(),
            description: self.description.clone(),
            supported_devices: self.supported_devices.clone(),
            capabilities: self.capabilities,
            requirements: self.requirements.clone(),
            priority: self.priority,
            status: self.status,
            registration_time: self.registration_time,
            device_count: AtomicU32::new(self.device_count.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
        }
    }
}

impl DriverDescriptor {
    /// Create new driver descriptor
    pub fn new(
        name: String,
        version: String,
        author: String,
        description: String,
    ) -> Self {
        Self {
            name,
            version,
            author,
            description,
            supported_devices: Vec::new(),
            capabilities: DriverCapabilities::default(),
            requirements: DriverRequirements::default(),
            priority: 0,
            status: DriverRegistrationStatus::NotRegistered,
            registration_time: 0,
            device_count: AtomicU32::new(0),
            error_count: AtomicU64::new(0),
        }
    }

    /// Add supported device
    pub fn with_supported_device(mut self, vendor_id: u16, device_id: u16) -> Self {
        self.supported_devices.push((vendor_id, device_id));
        self
    }

    /// Set capabilities
    pub fn with_capabilities(mut self, caps: DriverCapabilities) -> Self {
        self.capabilities = caps;
        self
    }

    /// Set requirements
    pub fn with_requirements(mut self, reqs: DriverRequirements) -> Self {
        self.requirements = reqs;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if device is supported
    pub fn supports_device(&self, vendor_id: u16, device_id: u16) -> bool {
        self.supported_devices.contains(&(vendor_id, device_id))
    }

    /// Increment device count
    pub fn increment_device_count(&self) {
        self.device_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement device count
    pub fn decrement_device_count(&self) {
        self.device_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get device count
    pub fn get_device_count(&self) -> u32 {
        self.device_count.load(Ordering::Relaxed)
    }

    /// Increment error count
    pub fn increment_error_count(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get error count
    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Device Lifecycle
// ============================================================================

/// Device lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceLifecycleState {
    /// Device is unknown
    Unknown,
    /// Device is discovered
    Discovered,
    /// Device is being probed
    Probing,
    /// Device is probed and bound
    Probed,
    /// Device is initialized
    Initialized,
    /// Device is active (running)
    Active,
    /// Device is suspended
    Suspended,
    /// Device is being removed
    Removing,
    /// Device is removed
    Removed,
    /// Device has failed
    Failed,
}

/// Device lifecycle descriptor
#[derive(Debug)]
pub struct DeviceLifecycle {
    /// Device ID
    pub device_id: u32,
    /// Current state
    pub state: DeviceLifecycleState,
    /// Driver name
    pub driver_name: Option<String>,
    /// State change count
    pub state_changes: AtomicU64,
    /// Last state change timestamp
    pub last_state_change: AtomicU64,
    /// Device uptime (nanoseconds)
    pub uptime_ns: AtomicU64,
    /// Error count
    pub error_count: AtomicU64,
}

// Manual implementation of Clone for DeviceLifecycle
impl Clone for DeviceLifecycle {
    fn clone(&self) -> Self {
        Self {
            device_id: self.device_id,
            state: self.state,
            driver_name: self.driver_name.clone(),
            state_changes: AtomicU64::new(self.state_changes.load(Ordering::Relaxed)),
            last_state_change: AtomicU64::new(self.last_state_change.load(Ordering::Relaxed)),
            uptime_ns: AtomicU64::new(self.uptime_ns.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
        }
    }
}

impl DeviceLifecycle {
    /// Create new device lifecycle
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            state: DeviceLifecycleState::Unknown,
            driver_name: None,
            state_changes: AtomicU64::new(0),
            last_state_change: AtomicU64::new(0),
            uptime_ns: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }

    /// Set state
    pub fn set_state(&mut self, new_state: DeviceLifecycleState) {
        self.state = new_state;
        self.state_changes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get state change count
    pub fn get_state_changes(&self) -> u64 {
        self.state_changes.load(Ordering::Relaxed)
    }

    /// Update uptime
    pub fn update_uptime(&self, delta_ns: u64) {
        self.uptime_ns.fetch_add(delta_ns, Ordering::Relaxed);
    }

    /// Get uptime
    pub fn get_uptime_ns(&self) -> u64 {
        self.uptime_ns.load(Ordering::Relaxed)
    }

    /// Increment error count
    pub fn increment_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get error count
    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Driver Framework Statistics
// ============================================================================

/// Driver framework statistics
#[derive(Debug, Default, Clone)]
pub struct FrameworkStats {
    /// Total registered drivers
    pub total_drivers: u32,
    /// Total active devices
    pub active_devices: u32,
    /// Total driver probes
    pub total_probes: u64,
    /// Successful probes
    pub successful_probes: u64,
    /// Failed probes
    pub failed_probes: u64,
    /// Device state transitions
    pub state_transitions: u64,
    /// Total errors
    pub total_errors: u64,
}

// ============================================================================
// Enhanced Driver Framework
// ============================================================================

/// Enhanced driver framework
pub struct EnhancedDriverFramework {
    /// Registered drivers (driver_name -> descriptor)
    drivers: Mutex<BTreeMap<String, DriverDescriptor>>,
    /// Device lifecycles (device_id -> lifecycle)
    device_lifecycles: Mutex<BTreeMap<u32, DeviceLifecycle>>,
    /// Driver to device mapping (driver_name -> device_ids)
    driver_devices: Mutex<BTreeMap<String, Vec<u32>>>,
    /// Statistics
    stats: Mutex<FrameworkStats>,
    /// Next device ID
    next_device_id: AtomicU32,
    /// Framework enabled
    enabled: AtomicU32,
}

impl EnhancedDriverFramework {
    /// Create new enhanced driver framework
    pub fn new() -> Self {
        Self {
            drivers: Mutex::new(BTreeMap::new()),
            device_lifecycles: Mutex::new(BTreeMap::new()),
            driver_devices: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(FrameworkStats::default()),
            next_device_id: AtomicU32::new(1),
            enabled: AtomicU32::new(1),
        }
    }

    /// Enable framework
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Release);
        crate::println!("driver-framework: enabled");
    }

    /// Disable framework
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Release);
        crate::println!("driver-framework: disabled");
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) != 0
    }

    /// Register driver
    pub fn register_driver(&self, descriptor: DriverDescriptor) -> Result<()> {
        if !self.is_enabled() {
            return Err(KernelError::InvalidState);
        }

        let name = descriptor.name.clone();

        // Check dependencies
        for dep in &descriptor.requirements.dependencies {
            if !self.is_driver_loaded(dep) {
                return Err(KernelError::NotFound);
            }
        }

        {
            let mut drivers = self.drivers.lock();
            if drivers.contains_key(&name) {
                return Err(KernelError::AlreadyExists);
            }

            drivers.insert(name.clone(), descriptor.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_drivers += 1;
        }

        crate::println!(
            "driver-framework: registered driver '{}' (version: {})",
            name,
            descriptor.version
        );

        Ok(())
    }

    /// Unregister driver
    pub fn unregister_driver(&self, name: &str) -> Result<()> {
        // Check if driver has devices
        {
            let driver_devices = self.driver_devices.lock();
            if let Some(devices) = driver_devices.get(name) {
                if !devices.is_empty() {
                    return Err(KernelError::ResourceBusy);
                }
            }
        }

        {
            let mut drivers = self.drivers.lock();
            drivers.remove(name).ok_or(KernelError::NotFound)?;
        }

        // Remove from device mapping
        {
            let mut driver_devices = self.driver_devices.lock();
            driver_devices.remove(name);
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_drivers = stats.total_drivers.saturating_sub(1);
        }

        crate::println!("driver-framework: unregistered driver '{}'", name);

        Ok(())
    }

    /// Check if driver is loaded
    pub fn is_driver_loaded(&self, name: &str) -> bool {
        let drivers = self.drivers.lock();
        drivers.contains_key(name)
    }

    /// Get driver descriptor
    pub fn get_driver(&self, name: &str) -> Option<DriverDescriptor> {
        let drivers = self.drivers.lock();
        drivers.get(name).cloned()
    }

    /// List all drivers
    pub fn list_drivers(&self) -> Vec<String> {
        let drivers = self.drivers.lock();
        drivers.keys().cloned().collect()
    }

    /// Probe device with matching drivers
    pub fn probe_device(&self, vendor_id: u16, device_id: u16) -> Result<u32> {
        let device_id_value = self.next_device_id.fetch_add(1, Ordering::SeqCst);

        // Create device lifecycle
        let mut lifecycle = DeviceLifecycle::new(device_id_value);
        lifecycle.set_state(DeviceLifecycleState::Probing);

        {
            let mut lifecycles = self.device_lifecycles.lock();
            lifecycles.insert(device_id_value, lifecycle);
        }

        // Find matching driver
        let matching_driver = {
            let drivers = self.drivers.lock();
            let mut matches: Vec<(nos_api::String, DriverDescriptor)> = drivers.iter()
                .filter(|(_, d)| d.supports_device(vendor_id, device_id))
                .map(|(name, desc)| (name.clone(), desc.clone()))
                .collect();
            matches.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));
            matches.first().cloned()
        };

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_probes += 1;
        }

        if let Some((driver_name, _descriptor)) = matching_driver {
            // Bind device to driver
            {
                let mut lifecycles = self.device_lifecycles.lock();
                if let Some(lifecycle) = lifecycles.get_mut(&device_id_value) {
                    lifecycle.set_state(DeviceLifecycleState::Probed);
                    lifecycle.driver_name = Some(driver_name.clone());
                }
            }

            // Add to driver's device list
            {
                let mut driver_devices = self.driver_devices.lock();
                driver_devices.entry(driver_name.clone())
                    .or_insert_with(Vec::new)
                    .push(device_id_value);
            }

            // Increment driver device count
            {
                let drivers = self.drivers.lock();
                if let Some(driver) = drivers.get(driver_name.as_str()) {
                    driver.increment_device_count();
                }
            }

            // Update statistics
            {
                let mut stats = self.stats.lock();
                stats.successful_probes += 1;
                stats.active_devices += 1;
            }

            crate::println!(
                "driver-framework: probed device {} ({:04X}:{:04X}) with driver '{}'",
                device_id_value,
                vendor_id,
                device_id,
                driver_name
            );

            Ok(device_id_value)
        } else {
            // No matching driver
            {
                let mut lifecycles = self.device_lifecycles.lock();
                if let Some(lifecycle) = lifecycles.get_mut(&device_id_value) {
                    lifecycle.set_state(DeviceLifecycleState::Failed);
                }
            }

            // Update statistics
            {
                let mut stats = self.stats.lock();
                stats.failed_probes += 1;
            }

            crate::println!(
                "driver-framework: no driver found for device ({:04X}:{:04X})",
                vendor_id,
                device_id
            );

            Err(KernelError::NotFound)
        }
    }

    /// Remove device
    pub fn remove_device(&self, device_id: u32) -> Result<()> {
        let driver_name = {
            let mut lifecycles = self.device_lifecycles.lock();
            let lifecycle = lifecycles.get_mut(&device_id).ok_or(KernelError::NotFound)?;
            lifecycle.set_state(DeviceLifecycleState::Removing);
            lifecycle.driver_name.clone()
        };

        // Remove from driver's device list
        if let Some(ref name) = driver_name {
            {
                let mut driver_devices = self.driver_devices.lock();
                if let Some(devices) = driver_devices.get_mut(name) {
                    devices.retain(|&id| id != device_id);
                }
            }

            // Decrement driver device count
            {
                let drivers = self.drivers.lock();
                if let Some(driver) = drivers.get(name) {
                    driver.decrement_device_count();
                }
            }
        }

        // Remove lifecycle
        {
            let mut lifecycles = self.device_lifecycles.lock();
            let lifecycle = lifecycles.get_mut(&device_id).ok_or(KernelError::NotFound)?;
            lifecycle.set_state(DeviceLifecycleState::Removed);
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.active_devices = stats.active_devices.saturating_sub(1);
            stats.state_transitions += 1;
        }

        crate::println!(
            "driver-framework: removed device {} (driver: {:?})",
            device_id,
            driver_name
        );

        Ok(())
    }

    /// Get device lifecycle
    pub fn get_device_lifecycle(&self, device_id: u32) -> Option<DeviceLifecycle> {
        let lifecycles = self.device_lifecycles.lock();
        lifecycles.get(&device_id).cloned()
    }

    /// Set device state
    pub fn set_device_state(&self, device_id: u32, new_state: DeviceLifecycleState) -> Result<()> {
        let mut lifecycles = self.device_lifecycles.lock();
        let lifecycle = lifecycles.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        let old_state = lifecycle.state;
        lifecycle.set_state(new_state);

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.state_transitions += 1;
        }

        crate::println!(
            "driver-framework: device {} state: {:?} -> {:?}",
            device_id,
            old_state,
            new_state
        );

        Ok(())
    }

    /// Report device error
    pub fn report_error(&self, device_id: u32, _error: KernelError) -> Result<()> {
        let lifecycles = self.device_lifecycles.lock();
        let lifecycle = lifecycles.get(&device_id).ok_or(KernelError::NotFound)?;

        lifecycle.increment_error();

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_errors += 1;
        }

        crate::println!(
            "driver-framework: error on device {} (total errors: {})",
            device_id,
            lifecycle.get_error_count()
        );

        Ok(())
    }

    /// Get devices for driver
    pub fn get_driver_devices(&self, driver_name: &str) -> Vec<u32> {
        let driver_devices = self.driver_devices.lock();
        driver_devices.get(driver_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Get statistics
    pub fn get_stats(&self) -> FrameworkStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = FrameworkStats::default();
    }
}

impl Default for EnhancedDriverFramework {
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
    fn test_driver_descriptor() {
        let descriptor = DriverDescriptor::new(
            "test-driver".to_string(),
            "1.0.0".to_string(),
            "Test Author".to_string(),
            "Test driver".to_string(),
        )
        .with_supported_device(0x8086, 0x1234)
        .with_supported_device(0x8086, 0x5678)
        .with_priority(10);

        assert_eq!(descriptor.name, "test-driver");
        assert_eq!(descriptor.supported_devices.len(), 2);
        assert!(descriptor.supports_device(0x8086, 0x1234));
        assert!(descriptor.supports_device(0x8086, 0x5678));
        assert!(!descriptor.supports_device(0x8086, 0xABCD));
        assert_eq!(descriptor.priority, 10);

        descriptor.increment_device_count();
        assert_eq!(descriptor.get_device_count(), 1);

        descriptor.increment_error_count();
        assert_eq!(descriptor.get_error_count(), 1);
    }

    #[test]
    fn test_device_lifecycle() {
        let mut lifecycle = DeviceLifecycle::new(100);
        assert_eq!(lifecycle.device_id, 100);
        assert_eq!(lifecycle.state, DeviceLifecycleState::Unknown);

        lifecycle.set_state(DeviceLifecycleState::Probing);
        assert_eq!(lifecycle.state, DeviceLifecycleState::Probing);
        assert_eq!(lifecycle.get_state_changes(), 1);

        lifecycle.update_uptime(1000);
        assert_eq!(lifecycle.get_uptime_ns(), 1000);

        lifecycle.increment_error();
        assert_eq!(lifecycle.get_error_count(), 1);
    }

    #[test]
    fn test_driver_registration() {
        let framework = EnhancedDriverFramework::new();

        let descriptor = DriverDescriptor::new(
            "test-driver".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test".to_string(),
        )
        .with_supported_device(0x8086, 0x1234);

        framework.register_driver(descriptor).unwrap();

        assert!(framework.is_driver_loaded("test-driver"));
        assert!(!framework.is_driver_loaded("nonexistent"));

        let retrieved = framework.get_driver("test-driver").unwrap();
        assert_eq!(retrieved.name, "test-driver");

        let drivers = framework.list_drivers();
        assert_eq!(drivers.len(), 1);
        assert!(drivers.contains(&"test-driver".to_string()));
    }

    #[test]
    fn test_device_probe() {
        let framework = EnhancedDriverFramework::new();

        let descriptor = DriverDescriptor::new(
            "test-driver".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test".to_string(),
        )
        .with_supported_device(0x8086, 0x1234)
        .with_priority(10);

        framework.register_driver(descriptor).unwrap();

        // Probe device
        let device_id = framework.probe_device(0x8086, 0x1234).unwrap();
        assert_eq!(device_id, 1);

        let lifecycle = framework.get_device_lifecycle(device_id).unwrap();
        assert_eq!(lifecycle.state, DeviceLifecycleState::Probed);
        assert_eq!(lifecycle.driver_name, Some("test-driver".to_string()));

        // Check driver's device list
        let devices = framework.get_driver_devices("test-driver");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0], device_id);

        // Check statistics
        let stats = framework.get_stats();
        assert_eq!(stats.total_probes, 1);
        assert_eq!(stats.successful_probes, 1);
        assert_eq!(stats.active_devices, 1);
    }

    #[test]
    fn test_device_remove() {
        let framework = EnhancedDriverFramework::new();

        let descriptor = DriverDescriptor::new(
            "test-driver".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test".to_string(),
        )
        .with_supported_device(0x8086, 0x1234);

        framework.register_driver(descriptor).unwrap();

        let device_id = framework.probe_device(0x8086, 0x1234).unwrap();
        framework.remove_device(device_id).unwrap();

        let lifecycle = framework.get_device_lifecycle(device_id).unwrap();
        assert_eq!(lifecycle.state, DeviceLifecycleState::Removed);

        let devices = framework.get_driver_devices("test-driver");
        assert_eq!(devices.len(), 0);

        let stats = framework.get_stats();
        assert_eq!(stats.active_devices, 0);
    }

    #[test]
    fn test_no_matching_driver() {
        let framework = EnhancedDriverFramework::new();

        let descriptor = DriverDescriptor::new(
            "test-driver".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test".to_string(),
        )
        .with_supported_device(0x8086, 0x1234);

        framework.register_driver(descriptor).unwrap();

        // Try to probe unsupported device
        let result = framework.probe_device(0x8086, 0x5678);
        assert!(result.is_err());

        let stats = framework.get_stats();
        assert_eq!(stats.failed_probes, 1);
    }
}
