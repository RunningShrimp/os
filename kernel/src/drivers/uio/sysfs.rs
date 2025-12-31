//! Sysfs interface for UIO devices
//!
//! This module provides sysfs integration for device configuration,
//! debugging, and runtime information.

use crate::drivers::uio::{
    device::UioDevice,
    interrupt::UioInterruptStats,
    memory::UioMemRegion,
    UioError, UioResult, UIO_CLASS_NAME, UIO_CLASS_PATH,
};
use alloc::string::String;
use alloc::sync::Arc;

/// UIO sysfs interface
#[derive(Debug)]
pub struct UioSysfs {
    /// Class initialized flag
    initialized: bool,
}

impl UioSysfs {
    /// Create new sysfs interface
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    /// Initialize sysfs class
    pub fn init(&mut self) -> UioResult<()> {
        if self.initialized {
            return Ok(());
        }

        // Create sysfs class directory
        self.create_class()?;

        self.initialized = true;
        Ok(())
    }

    /// Add device to sysfs
    pub fn add_device(&self, device: &UioDevice) -> UioResult<()> {
        if !self.initialized {
            return Err(UioError::InvalidOperation);
        }

        // Create device directory
        let device_path = format!("{}/uio{}", UIO_CLASS_PATH, device.minor());
        self.create_device_dir(&device_path)?;

        // Create standard attributes
        self.create_device_attrs(device, &device_path)?;

        // Create memory region attributes
        self.create_region_attrs(device, &device_path)?;

        // Create interrupt attributes
        self.create_interrupt_attrs(device, &device_path)?;

        Ok(())
    }

    /// Remove device from sysfs
    pub fn remove_device(&self, name: &str) -> UioResult<()> {
        if !self.initialized {
            return Ok(());
        }

        // Remove device directory
        let device_path = format!("{}/{}", UIO_CLASS_PATH, name);
        self.remove_device_dir(&device_path)?;

        Ok(())
    }

    /// Cleanup sysfs
    pub fn cleanup(&self) -> UioResult<()> {
        if !self.initialized {
            return Ok(());
        }

        // Remove class directory
        self.remove_class()?;

        Ok(())
    }

    /// Create sysfs class
    fn create_class(&self) -> UioResult<()> {
        log::debug!("Creating UIO sysfs class at {}", UIO_CLASS_PATH);
        // Platform-specific implementation
        Ok(())
    }

    /// Remove sysfs class
    fn remove_class(&self) -> UioResult<()> {
        log::debug!("Removing UIO sysfs class");
        // Platform-specific implementation
        Ok(())
    }

    /// Create device directory
    fn create_device_dir(&self, path: &str) -> UioResult<()> {
        log::debug!("Creating device directory {}", path);
        // Platform-specific implementation
        Ok(())
    }

    /// Remove device directory
    fn remove_device_dir(&self, path: &str) -> UioResult<()> {
        log::debug!("Removing device directory {}", path);
        // Platform-specific implementation
        Ok(())
    }

    /// Create device attributes
    fn create_device_attrs(&self, device: &UioDevice, path: &str) -> UioResult<()> {
        // Create standard sysfs attributes
        let attrs = UioSysfsAttrs::new(device);

        // Name attribute
        self.create_attr(path, "name", &attrs.name())?;

        // Version attribute
        self.create_attr(path, "version", &attrs.version())?;

        // Maps attribute (list memory regions)
        self.create_attr(path, "maps", &attrs.maps())?;

        // Events attribute (interrupt count)
        self.create_attr(path, "event", &attrs.event())?;

        Ok(())
    }

    /// Create memory region attributes
    fn create_region_attrs(&self, device: &UioDevice, path: &str) -> UioResult<()> {
        // Create attributes for each memory region
        for i in 0..8 {
            if let Ok(region) = device.get_region(i) {
                let region_path = format!("{}/maps/map{}", path, i);

                // Create map directory
                self.create_map_dir(&region_path)?;

                // Region attributes
                self.create_attr(&region_path, "addr", &format!("{:#X}", region.phys_addr()))?;
                self.create_attr(&region_path, "size", &format!("{:#X}", region.size()))?;
                self.create_attr(&region_path, "name", region.name())?;
            }
        }

        Ok(())
    }

    /// Create interrupt attributes
    fn create_interrupt_attrs(&self, device: &UioDevice, path: &str) -> UioResult<()> {
        // Create interrupt attributes
        self.create_attr(path, "irq", &device.irq().to_string())?;
        Ok(())
    }

    /// Create sysfs attribute
    fn create_attr(&self, path: &str, name: &str, value: &str) -> UioResult<()> {
        let attr_path = format!("{}/{}", path, name);
        log::debug!("Creating attribute {}: {}", attr_path, value);
        // Platform-specific implementation
        Ok(())
    }

    /// Create memory map directory
    fn create_map_dir(&self, path: &str) -> UioResult<()> {
        log::debug!("Creating map directory {}", path);
        // Platform-specific implementation
        Ok(())
    }
}

impl Default for UioSysfs {
    fn default() -> Self {
        Self::new()
    }
}

/// UIO sysfs attributes
#[derive(Debug, Clone)]
pub struct UioSysfsAttrs {
    /// Device reference
    device: Arc<UioDevice>,
}

impl UioSysfsAttrs {
    /// Create new sysfs attributes
    pub fn new(device: &UioDevice) -> Self {
        Self {
            device: Arc::new(device.clone()),
        }
    }

    /// Get device name
    pub fn name(&self) -> String {
        self.device.name().to_string()
    }

    /// Get device version
    pub fn version(&self) -> String {
        self.device.info().version.clone()
    }

    /// Get memory maps
    pub fn maps(&self) -> String {
        let mut result = String::from("Map0\n");

        for i in 0..8 {
            if let Ok(region) = self.device.get_region(i) {
                result.push_str(&format!(
                    "Map{}: addr={:#X} size={:#X} name={}\n",
                    i,
                    region.phys_addr(),
                    region.size(),
                    region.name()
                ));
            }
        }

        result
    }

    /// Get event count
    pub fn event(&self) -> String {
        // This would read from interrupt stats
        "0".to_string()
    }

    /// Get IRQ number
    pub fn irq(&self) -> String {
        self.device.irq().to_string()
    }

    /// Enable device (write-only attribute)
    pub fn enable(&self) -> UioResult<()> {
        self.device.enable()
    }

    /// Disable device (write-only attribute)
    pub fn disable(&self) -> UioResult<()> {
        self.device.disable()
    }

    /// Get device statistics
    pub fn stats(&self) -> UioDeviceStats {
        UioDeviceStats {
            name: self.device.name().to_string(),
            minor: self.device.minor(),
            irq: self.device.irq(),
            num_regions: self.count_regions(),
            enabled: self.device.is_enabled(),
            interrupt_count: 0, // Would come from interrupt stats
        }
    }

    /// Count memory regions
    fn count_regions(&self) -> usize {
        let mut count = 0;
        for i in 0..8 {
            if self.device.get_region(i).is_ok() {
                count += 1;
            }
        }
        count
    }
}

/// UIO device statistics
#[derive(Debug, Clone, Copy)]
pub struct UioDeviceStats {
    /// Device name
    pub name: &'static str,
    /// Minor number
    pub minor: u32,
    /// IRQ number
    pub irq: u32,
    /// Number of memory regions
    pub num_regions: usize,
    /// Device enabled flag
    pub enabled: bool,
    /// Interrupt count
    pub interrupt_count: u64,
}

/// Sysfs attribute operations
pub trait UioSysfsAttrOps: Send + Sync {
    /// Read attribute value
    fn read(&self) -> UioResult<String>;

    /// Write attribute value
    fn write(&mut self, value: &str) -> UioResult<()>;
}

/// Read-only string attribute
#[derive(Debug)]
pub struct UioReadOnlyAttr {
    name: String,
    value: Arc<dyn Fn() -> String + Send + Sync>,
}

impl UioReadOnlyAttr {
    /// Create new read-only attribute
    pub fn new(name: String, value: Arc<dyn Fn() -> String + Send + Sync>) -> Self {
        Self { name, value }
    }
}

impl UioSysfsAttrOps for UioReadOnlyAttr {
    fn read(&self) -> UioResult<String> {
        Ok((self.value)())
    }

    fn write(&mut self, _value: &str) -> UioResult<()> {
        Err(UioError::PermissionDenied)
    }
}

/// Write-only boolean attribute
#[derive(Debug)]
pub struct UioWriteOnlyBoolAttr {
    name: String,
    handler: Arc<dyn Fn(bool) -> UioResult<()> + Send + Sync>,
}

impl UioWriteOnlyBoolAttr {
    /// Create new write-only boolean attribute
    pub fn new(
        name: String,
        handler: Arc<dyn Fn(bool) -> UioResult<()> + Send + Sync>,
    ) -> Self {
        Self { name, handler }
    }
}

impl UioSysfsAttrOps for UioWriteOnlyBoolAttr {
    fn read(&self) -> UioResult<String> {
        Ok("<write-only>".to_string())
    }

    fn write(&mut self, value: &str) -> UioResult<()> {
        let bool_val = value.parse::<bool>().map_err(|_| UioError::InvalidParam)?;
        (self.handler)(bool_val)
    }
}

/// Read-write integer attribute
#[derive(Debug)]
pub struct UioReadWriteIntAttr {
    name: String,
    value: Arc<dyn Fn() -> u64 + Send + Sync>,
    setter: Arc<dyn Fn(u64) -> UioResult<()> + Send + Sync>,
}

impl UioReadWriteIntAttr {
    /// Create new read-write integer attribute
    pub fn new(
        name: String,
        value: Arc<dyn Fn() -> u64 + Send + Sync>,
        setter: Arc<dyn Fn(u64) -> UioResult<()> + Send + Sync>,
    ) -> Self {
        Self { name, value, setter }
    }
}

impl UioSysfsAttrOps for UioReadWriteIntAttr {
    fn read(&self) -> UioResult<String> {
        Ok((self.value)().to_string())
    }

    fn write(&mut self, value: &str) -> UioResult<()> {
        let int_val = value.parse::<u64>().map_err(|_| UioError::InvalidParam)?;
        (self.setter)(int_val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;

    #[test]
    fn test_sysfs_new() {
        let sysfs = UioSysfs::new();
        assert!(!sysfs.initialized);
    }

    #[test]
    fn test_sysfs_attrs_new() {
        let device = UioDevice::new("test", 0);
        let attrs = UioSysfsAttrs::new(&device);

        assert_eq!(attrs.name(), "test");
        assert_eq!(attrs.version(), "1.0");
        assert_eq!(attrs.irq(), "0");
    }

    #[test]
    fn test_read_only_attr() {
        let attr = UioReadOnlyAttr::new(
            "test".to_string(),
            Arc::new(|| "test_value".to_string()),
        );

        assert_eq!(attr.read().unwrap(), "test_value");
        assert!(attr.write("ignored").is_err());
    }

    #[test]
    fn test_write_only_bool_attr() {
        let called = Arc::new(core::sync::atomic::AtomicBool::new(false));

        let attr = UioWriteOnlyBoolAttr::new(
            "test".to_string(),
            Arc::new({
                let called = called.clone();
                move |val| {
                    called.store(val, core::sync::atomic::Ordering::Release);
                    Ok(())
                }
            }),
        );

        assert_eq!(attr.read().unwrap(), "<write-only>");

        attr.write("true").unwrap();
        assert!(called.load(core::sync::atomic::Ordering::Acquire));

        attr.write("false").unwrap();
        assert!(!called.load(core::sync::atomic::Ordering::Acquire));
    }

    #[test]
    fn test_read_write_int_attr() {
        let value = Arc::new(core::sync::atomic::AtomicU64::new(42));

        let attr = UioReadWriteIntAttr::new(
            "test".to_string(),
            Arc::new({
                let value = value.clone();
                move || value.load(core::sync::atomic::Ordering::Acquire)
            }),
            Arc::new({
                let value = value.clone();
                move |val| {
                    value.store(val, core::sync::atomic::Ordering::Release);
                    Ok(())
                }
            }),
        );

        assert_eq!(attr.read().unwrap(), "42");
        attr.write("100").unwrap();
        assert_eq!(attr.read().unwrap(), "100");
    }
}
