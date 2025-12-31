//! Main UIO driver implementation
//!
//! This module provides the core UIO driver functionality including device management,
//! registration, and integration with the kernel device model.

use alloc::{string::String, vec::Vec};
use crate::drivers::uio::{
    device::{UioDevice, UioDeviceRegistry, UioInfo},
    interrupt::{UioInterrupt, UioInterruptHandler},
    memory::UioMemRegion,
    sysfs::UioSysfs,
    UioError, UioResult, UIO_CLASS_NAME, UIO_DEVICE_PREFIX, UIO_MAX_DEVICES,
};

/// UIO driver information structure
#[derive(Debug, Clone)]
pub struct UioDriverInfo {
    /// Driver name
    pub name: String,
    /// Driver version
    pub version: String,
    /// Supported device types
    pub device_types: Vec<String>,
    /// Maximum number of supported devices
    pub max_devices: usize,
}

impl Default for UioDriverInfo {
    fn default() -> Self {
        Self {
            name: "uio".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            device_types: vec!["generic".to_string()],
            max_devices: UIO_MAX_DEVICES,
        }
    }
}

/// UIO driver - manages UIO device registration and subsystem
#[derive(Debug)]
pub struct UioDriver {
    /// Driver information
    info: UioDriverInfo,
    /// Device registry
    registry: UioDeviceRegistry,
    /// Sysfs interface
    sysfs: UioSysfs,
    /// Driver initialized flag
    initialized: bool,
}

impl UioDriver {
    /// Create a new UIO driver
    pub fn new() -> Self {
        Self {
            info: UioDriverInfo::default(),
            registry: UioDeviceRegistry::new(),
            sysfs: UioSysfs::new(),
            initialized: false,
        }
    }

    /// Create a UIO driver with custom info
    pub fn with_info(info: UioDriverInfo) -> Self {
        Self {
            info,
            registry: UioDeviceRegistry::new(),
            sysfs: UioSysfs::new(),
            initialized: false,
        }
    }

    /// Initialize the UIO driver
    pub fn init(&mut self) -> UioResult<()> {
        if self.initialized {
            return Err(UioError::DeviceBusy);
        }

        // Initialize sysfs
        self.sysfs.init()?;

        // Initialize device registry
        self.registry.init()?;

        self.initialized = true;
        Ok(())
    }

    /// Register a UIO device
    pub fn register_device(&mut self, device: UioDevice) -> UioResult<()> {
        if !self.initialized {
            return Err(UioError::InvalidOperation);
        }

        // Validate device
        if device.name().is_empty() {
            return Err(UioError::InvalidParam);
        }

        // Allocate device number
        let minor = self.registry.allocate_minor()?;
        device.set_minor(minor);

        // Register with device core
        self.registry.add_device(device.clone())?;

        // Create sysfs entries
        self.sysfs.add_device(&device)?;

        Ok(())
    }

    /// Unregister a UIO device
    pub fn unregister_device(&mut self, name: &str) -> UioResult<()> {
        if !self.initialized {
            return Err(UioError::InvalidOperation);
        }

        // Remove from sysfs
        self.sysfs.remove_device(name)?;

        // Remove from registry
        let device = self.registry.remove_device(name)?;
        device.free_minor();

        Ok(())
    }

    /// Look up a device by name
    pub fn lookup_device(&self, name: &str) -> UioResult<UioDevice> {
        self.registry.lookup(name)
    }

    /// Look up a device by minor number
    pub fn lookup_device_by_minor(&self, minor: u32) -> UioResult<UioDevice> {
        self.registry.lookup_by_minor(minor)
    }

    /// Get all registered devices
    pub fn devices(&self) -> Vec<UioDevice> {
        self.registry.list_devices()
    }

    /// Get driver information
    pub fn info(&self) -> &UioDriverInfo {
        &self.info
    }

    /// Check if driver is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Shutdown the UIO driver
    pub fn shutdown(&mut self) -> UioResult<()> {
        if !self.initialized {
            return Ok(());
        }

        // Remove all devices
        let devices = self.registry.list_devices();
        for device in devices {
            let _ = self.unregister_device(device.name());
        }

        // Cleanup sysfs
        self.sysfs.cleanup()?;

        self.initialized = false;
        Ok(())
    }
}

impl Default for UioDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Global UIO driver instance
static mut GLOBAL_UIO_DRIVER: Option<UioDriver> = None;

/// Initialize the global UIO driver
pub fn init_uio_driver() -> UioResult<()> {
    unsafe {
        if GLOBAL_UIO_DRIVER.is_some() {
            return Err(UioError::DeviceBusy);
        }

        let mut driver = UioDriver::new();
        driver.init()?;
        GLOBAL_UIO_DRIVER = Some(driver);
        Ok(())
    }
}

/// Get the global UIO driver
pub fn get_uio_driver() -> Option<&'static UioDriver> {
    unsafe { GLOBAL_UIO_DRIVER.as_ref() }
}

/// Get the global UIO driver (mutable)
pub fn get_uio_driver_mut() -> Option<&'static mut UioDriver> {
    unsafe { GLOBAL_UIO_DRIVER.as_mut() }
}

/// Shutdown the global UIO driver
pub fn shutdown_uio_driver() -> UioResult<()> {
    unsafe {
        if let Some(driver) = GLOBAL_UIO_DRIVER.as_mut() {
            driver.shutdown()?;
            GLOBAL_UIO_DRIVER = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_info_default() {
        let info = UioDriverInfo::default();
        assert_eq!(info.name, "uio");
        assert_eq!(info.device_types, vec!["generic"]);
    }

    #[test]
    fn test_driver_create() {
        let driver = UioDriver::new();
        assert!(!driver.is_initialized());
        assert_eq!(driver.info().name, "uio");
    }

    #[test]
    fn test_driver_with_info() {
        let info = UioDriverInfo {
            name: "test".to_string(),
            ..Default::default()
        };
        let driver = UioDriver::with_info(info);
        assert_eq!(driver.info().name, "test");
    }
}
