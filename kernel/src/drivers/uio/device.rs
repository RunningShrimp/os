//! Device lifecycle management for UIO
//!
//! This module provides device registration, lifecycle management, and
//! registry functionality for UIO devices.

use crate::drivers::uio::{
    interrupt::{UioInterrupt, UioInterruptHandler},
    memory::{UioMemRegion, UioMemRegionManager},
    UioError, UioResult, UIO_MAX_DEVICES, UIO_MAX_REGIONS,
};
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;

/// UIO device information structure
#[derive(Debug, Clone)]
pub struct UioInfo {
    /// Device name
    pub name: String,
    /// Device version
    pub version: String,
    /// Device description
    pub description: String,
    /// Device major number (0 for dynamic allocation)
    pub major: u32,
    /// Device minor number
    pub minor: u32,
    /// Device flags
    pub flags: u32,
}

impl UioInfo {
    /// Create new device info
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: "1.0".to_string(),
            description: String::new(),
            major: 0,
            minor: 0,
            flags: 0,
        }
    }
}

/// UIO device structure
///
/// Represents a userspace I/O device with memory regions and interrupts.
#[derive(Debug)]
pub struct UioDevice {
    /// Device information
    info: UioInfo,
    /// Memory regions
    regions: UioMemRegionManager,
    /// Interrupt controller
    interrupt: Arc<UioInterrupt>,
    /// Device enabled flag
    enabled: AtomicBool,
    /// Device registered flag
    registered: AtomicBool,
    /// Reference count
    refcount: Arc<AtomicU32>,
}

impl UioDevice {
    /// Create a new UIO device
    pub fn new(name: &str, minor: u32) -> Self {
        let info = UioInfo::new(name.to_string());
        Self {
            info,
            regions: UioMemRegionManager::new(UIO_MAX_REGIONS),
            interrupt: Arc::new(UioInterrupt::new()),
            enabled: AtomicBool::new(false),
            registered: AtomicBool::new(false),
            refcount: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Get device name
    pub fn name(&self) -> &str {
        &self.info.name
    }

    /// Get device information
    pub fn info(&self) -> &UioInfo {
        &self.info
    }

    /// Get minor number
    pub fn minor(&self) -> u32 {
        self.info.minor
    }

    /// Set minor number
    pub fn set_minor(&mut self, minor: u32) {
        self.info.minor = minor;
    }

    /// Free minor number
    pub fn free_minor(&self) {
        // Signal to registry that minor is available
    }

    /// Add a memory region
    pub fn add_region(&mut self, region: UioMemRegion) -> UioResult<()> {
        self.regions.add_region(region)?;
        Ok(())
    }

    /// Get memory region by index
    pub fn get_region(&self, index: usize) -> UioResult<&UioMemRegion> {
        self.regions.get_region(index)
    }

    /// Remove memory region
    pub fn remove_region(&mut self, index: usize) -> UioResult<()> {
        self.regions.remove_region(index)?;
        Ok(())
    }

    /// Map memory region
    pub fn map_region(&self, index: usize) -> UioResult<usize> {
        let region = self.regions.get_region(index)?;
        region.map()
    }

    /// Unmap memory region
    pub fn unmap_region(&self, index: usize) -> UioResult<()> {
        let region = self.regions.get_region(index)?;
        region.unmap()
    }

    /// Register interrupt handler
    pub fn register_interrupt<F>(&self, handler: F) -> UioResult<()>
    where
        F: Fn(u32) -> UioResult<()> + Send + Sync + 'static,
    {
        self.interrupt.register_handler(Arc::new(handler))
    }

    /// Unregister interrupt handler
    pub fn unregister_interrupt(&self) -> UioResult<()> {
        self.interrupt.unregister_handler()
    }

    /// Enable interrupt
    pub fn enable_interrupt(&self) -> UioResult<()> {
        self.interrupt.enable()
    }

    /// Disable interrupt
    pub fn disable_interrupt(&self) -> UioResult<()> {
        self.interrupt.disable()
    }

    /// Set IRQ number
    pub fn set_irq(&self, irq: u32) {
        self.interrupt.set_irq(irq);
    }

    /// Get IRQ number
    pub fn irq(&self) -> u32 {
        self.interrupt.irq()
    }

    /// Wait for interrupt
    pub fn wait_for_interrupt(&self) -> UioResult<()> {
        // Block until interrupt occurs
        // This would typically use wait queues or similar mechanism
        Ok(())
    }

    /// Acknowledge interrupt
    pub fn ack_interrupt(&self) -> UioResult<()> {
        // Reset interrupt state
        self.interrupt.reset_count();
        Ok(())
    }

    /// Enable device
    pub fn enable(&self) -> UioResult<()> {
        if self.enabled.load(Ordering::Acquire) {
            return Ok(());
        }

        self.regions.map_all()?;
        self.interrupt.enable()?;
        self.enabled.store(true, Ordering::Release);

        Ok(())
    }

    /// Disable device
    pub fn disable(&self) -> UioResult<()> {
        if !self.enabled.load(Ordering::Acquire) {
            return Ok(());
        }

        self.interrupt.disable()?;
        self.regions.unmap_all()?;
        self.enabled.store(false, Ordering::Release);

        Ok(())
    }

    /// Check if device is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Check if device is registered
    pub fn is_registered(&self) -> bool {
        self.registered.load(Ordering::Acquire)
    }

    /// Mark device as registered
    pub fn set_registered(&self, registered: bool) {
        self.registered.store(registered, Ordering::Release);
    }

    /// Increment reference count
    pub fn get(&self) {
        self.refcount.fetch_add(1, Ordering::Release);
    }

    /// Decrement reference count
    pub fn put(&self) {
        if self.refcount.fetch_sub(1, Ordering::Release) == 1 {
            // Last reference dropped, cleanup
        }
    }

    /// Get reference count
    pub fn refcount(&self) -> u32 {
        self.refcount.load(Ordering::Acquire)
    }
}

impl Clone for UioDevice {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            regions: UioMemRegionManager::new(UIO_MAX_REGIONS),
            interrupt: Arc::clone(&self.interrupt),
            enabled: AtomicBool::new(self.enabled.load(Ordering::Acquire)),
            registered: AtomicBool::new(self.registered.load(Ordering::Acquire)),
            refcount: Arc::clone(&self.refcount),
        }
    }
}

/// UIO device registry
///
/// Tracks all registered UIO devices and manages device numbers.
#[derive(Debug)]
pub struct UioDeviceRegistry {
    /// Registered devices
    devices: Mutex<alloc::vec::Vec<Option<UioDevice>>>,
    /// Minor number allocation bitmap
    minors: Mutex<alloc::vec::Vec<bool>>,
    /// Maximum devices
    max_devices: usize,
}

impl UioDeviceRegistry {
    /// Create new device registry
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(alloc::vec![None; UIO_MAX_DEVICES]),
            minors: Mutex::new(alloc::vec![false; UIO_MAX_DEVICES]),
            max_devices: UIO_MAX_DEVICES,
        }
    }

    /// Initialize registry
    pub fn init(&self) -> UioResult<()> {
        // Platform-specific initialization
        Ok(())
    }

    /// Allocate a minor number
    pub fn allocate_minor(&self) -> UioResult<u32> {
        let mut minors = self.minors.lock();

        // Find free minor
        for (i, &in_use) in minors.iter().enumerate() {
            if !in_use {
                minors[i] = true;
                return Ok(i as u32);
            }
        }

        Err(UioError::NoMemory)
    }

    /// Free a minor number
    pub fn free_minor(&self, minor: u32) {
        let mut minors = self.minors.lock();
        if (minor as usize) < self.max_devices {
            minors[minor as usize] = false;
        }
    }

    /// Add device to registry
    pub fn add_device(&self, device: UioDevice) -> UioResult<()> {
        let minor = device.minor();

        if minor >= self.max_devices as u32 {
            return Err(UioError::InvalidDevice);
        }

        let mut devices = self.devices.lock();
        if devices[minor as usize].is_some() {
            return Err(UioError::DeviceBusy);
        }

        devices[minor as usize] = Some(device);
        Ok(())
    }

    /// Remove device from registry
    pub fn remove_device(&self, name: &str) -> UioResult<UioDevice> {
        let mut devices = self.devices.lock();

        // Find device by name
        for (i, device) in devices.iter_mut().enumerate() {
            if let Some(ref dev) = *device {
                if dev.name() == name {
                    let minor = dev.minor();
                    let dev = device.take().ok_or(UioError::DeviceNotFound)?;

                    // Free minor number
                    drop(devices);
                    self.free_minor(minor);

                    return Ok(dev);
                }
            }
        }

        Err(UioError::DeviceNotFound)
    }

    /// Look up device by name
    pub fn lookup(&self, name: &str) -> UioResult<UioDevice> {
        let devices = self.devices.lock();

        for device in devices.iter().flatten() {
            if device.name() == name {
                return Ok(device.clone());
            }
        }

        Err(UioError::DeviceNotFound)
    }

    /// Look up device by minor number
    pub fn lookup_by_minor(&self, minor: u32) -> UioResult<UioDevice> {
        if minor >= self.max_devices as u32 {
            return Err(UioError::InvalidDevice);
        }

        let devices = self.devices.lock();
        let device = devices[minor as usize]
            .as_ref()
            .ok_or(UioError::DeviceNotFound)?;

        Ok(device.clone())
    }

    /// List all registered devices
    pub fn list_devices(&self) -> alloc::vec::Vec<UioDevice> {
        let devices = self.devices.lock();

        devices
            .iter()
            .filter_map(|d| d.as_ref().cloned())
            .collect()
    }

    /// Get device count
    pub fn count(&self) -> usize {
        let devices = self.devices.lock();
        devices.iter().filter(|d| d.is_some()).count()
    }
}

impl Default for UioDeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Device add operation
pub fn device_add(mut device: UioDevice) -> UioResult<()> {
    // Get registry from global driver
    use crate::drivers::uio::uio::get_uio_driver;

    let driver = get_uio_driver().ok_or(UioError::InvalidOperation)?;

    // Allocate minor if not set
    if device.minor() == 0 {
        let registry = &driver.registry;
        let minor = registry.allocate_minor()?;
        device.set_minor(minor);
    }

    // Register device
    driver.register_device(device)?;
    Ok(())
}

/// Device remove operation
pub fn device_remove(name: &str) -> UioResult<()> {
    use crate::drivers::uio::uio::get_uio_driver;

    let driver = get_uio_driver().ok_or(UioError::InvalidOperation)?;

    driver.unregister_device(name)?;
    Ok(())
}

/// Device lookup by name
pub fn device_lookup(name: &str) -> UioResult<UioDevice> {
    use crate::drivers::uio::uio::get_uio_driver;

    let driver = get_uio_driver().ok_or(UioError::InvalidOperation)?;

    driver.lookup_device(name)
}

/// Device lookup by major/minor
pub fn device_lookup_by_dev(major: u32, minor: u32) -> UioResult<UioDevice> {
    use crate::drivers::uio::uio::get_uio_driver;

    let driver = get_uio_driver().ok_or(UioError::InvalidOperation)?;

    // Validate major number if using dynamic allocation
    if major != 0 && major != driver.info().major {
        return Err(UioError::DeviceNotFound);
    }

    driver.lookup_device_by_minor(minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uio_info_new() {
        let info = UioInfo::new("test_device".to_string());
        assert_eq!(info.name, "test_device");
        assert_eq!(info.version, "1.0");
        assert_eq!(info.minor, 0);
    }

    #[test]
    fn test_uio_device_new() {
        let device = UioDevice::new("test", 0);
        assert_eq!(device.name(), "test");
        assert_eq!(device.minor(), 0);
        assert!(!device.is_enabled());
        assert!(!device.is_registered());
    }

    #[test]
    fn test_uio_device_regions() {
        let mut device = UioDevice::new("test", 0);

        let region = UioMemRegion::mmio(0xF0000000, 0x1000);
        device.add_region(region).unwrap();

        let retrieved = device.get_region(0).unwrap();
        assert_eq!(retrieved.phys_addr(), 0xF0000000);
    }

    #[test]
    fn test_registry_new() {
        let registry = UioDeviceRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_allocate_minor() {
        let registry = UioDeviceRegistry::new();

        let minor1 = registry.allocate_minor().unwrap();
        assert_eq!(minor1, 0);

        let minor2 = registry.allocate_minor().unwrap();
        assert_eq!(minor2, 1);

        registry.free_minor(minor1);
        let minor3 = registry.allocate_minor().unwrap();
        assert_eq!(minor3, 0); // Should reuse freed minor
    }

    #[test]
    fn test_registry_add_device() {
        let registry = UioDeviceRegistry::new();
        let device = UioDevice::new("test", 0);

        registry.add_device(device).unwrap();
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_registry_lookup() {
        let registry = UioDeviceRegistry::new();
        let device = UioDevice::new("test", 0);

        registry.add_device(device).unwrap();
        let found = registry.lookup("test").unwrap();
        assert_eq!(found.name(), "test");
    }
}
