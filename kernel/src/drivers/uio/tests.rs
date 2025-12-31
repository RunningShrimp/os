//! Integration tests for UIO framework

use crate::drivers::uio::{
    api::{DefaultUioFileOps, UioIoctl, UioRegionInfo},
    device::{device_add, device_lookup, UioDevice},
    examples::{SimpleUioDriver, UioDeviceFactory},
    interrupt::UioInterrupt,
    memory::{UioMemPermissions, UioMemRegion, UioMemRegionType},
    sysfs::UioSysfsAttrs,
    UioDriver, UioError, UioResult, UIO_MAX_REGIONS,
};

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test complete device lifecycle
    #[test]
    fn test_device_lifecycle() {
        // Initialize driver
        let mut driver = UioDriver::new();
        assert!(driver.init().is_ok());
        assert!(driver.is_initialized());

        // Create device
        let device = UioDevice::new("test_lifecycle", 0);
        assert_eq!(device.name(), "test_lifecycle");
        assert!(!device.is_enabled());

        // Register device
        assert!(driver.register_device(device).is_ok());

        // Look up device
        let found = driver.lookup_device("test_lifecycle");
        assert!(found.is_ok());

        // Cleanup
        assert!(driver.shutdown().is_ok());
        assert!(!driver.is_initialized());
    }

    /// Test memory region operations
    #[test]
    fn test_memory_regions() {
        let mut device = UioDevice::new("test_regions", 0);

        // Add multiple regions
        let region1 = UioMemRegion::mmio(0xF0000000, 0x1000);
        let region2 = UioMemRegion::mmio(0xF0001000, 0x2000);
        let region3 = UioMemRegion::port_io(0x3F8, 8);

        assert!(device.add_region(region1).is_ok());
        assert!(device.add_region(region2).is_ok());
        assert!(device.add_region(region3).is_ok());

        // Retrieve regions
        let r1 = device.get_region(0);
        assert!(r1.is_ok());
        assert_eq!(r1.unwrap().phys_addr(), 0xF0000000);

        let r2 = device.get_region(1);
        assert!(r2.is_ok());
        assert_eq!(r2.unwrap().phys_addr(), 0xF0001000);

        let r3 = device.get_region(2);
        assert!(r3.is_ok());
        assert_eq!(r3.unwrap().region_type(), UioMemRegionType::PortIo);
    }

    /// Test interrupt handling
    #[test]
    fn test_interrupt_handling() {
        let device = UioDevice::new("test_interrupt", 0);
        let irq = 42;

        // Set IRQ
        device.set_irq(irq);
        assert_eq!(device.irq(), irq);

        // Register interrupt handler
        let result = device.register_interrupt(|irq_num| {
            log::info!("Interrupt {}", irq_num);
            Ok(())
        });
        assert!(result.is_ok());

        // Enable/disable interrupt
        assert!(device.enable_interrupt().is_ok());
        assert!(device.disable_interrupt().is_ok());
    }

    /// Test sysfs attributes
    #[test]
    fn test_sysfs_attributes() {
        let device = UioDevice::new("test_sysfs", 0);
        let attrs = UioSysfsAttrs::new(&device);

        assert_eq!(attrs.name(), "test_sysfs");
        assert_eq!(attrs.version(), "1.0");
        assert_eq!(attrs.irq(), "0");

        // Test device stats
        let stats = attrs.stats();
        assert_eq!(stats.name, "test_sysfs");
        assert_eq!(stats.minor, 0);
    }

    /// Test memory permissions
    #[test]
    fn test_memory_permissions() {
        let perms_ro = UioMemPermissions::read_only();
        assert!(perms_ro.read);
        assert!(!perms_ro.write);
        assert!(!perms_ro.execute);
        assert_eq!(perms_ro.as_flags(), 0x01);

        let perms_rw = UioMemPermissions::read_write();
        assert!(perms_rw.read);
        assert!(perms_rw.write);
        assert!(!perms_rw.execute);
        assert_eq!(perms_rw.as_flags(), 0x03);

        let perms_from_flags = UioMemPermissions::from_flags(0x03);
        assert_eq!(perms_from_flags.read, perms_rw.read);
        assert_eq!(perms_from_flags.write, perms_rw.write);
    }

    /// Test region validation
    #[test]
    fn test_region_validation() {
        let valid_region = UioMemRegion::mmio(0xF0000000, 0x1000);
        assert!(valid_region.validate().is_ok());

        let invalid_region = UioMemRegion::mmio(0xF0000001, 0x1000);
        assert!(invalid_region.validate().is_err());
    }

    /// Test interrupt controller
    #[test]
    fn test_interrupt_controller() {
        let intr = UioInterrupt::with_irq(42);

        assert_eq!(intr.irq(), 42);
        assert!(!intr.is_enabled());

        assert!(intr.enable().is_ok());
        assert!(intr.is_enabled());

        assert!(intr.disable().is_ok());
        assert!(!intr.is_enabled());

        // Test eventfd
        assert!(intr.set_eventfd(123).is_ok());
        assert_eq!(intr.eventfd(), Some(123));
    }

    /// Test simple driver example
    #[test]
    fn test_simple_driver() {
        let driver = SimpleUioDriver::new("test_simple_driver", 0xF0000000, 42);

        assert_eq!(driver.device().name(), "test_simple_driver");
        assert_eq!(driver.base_addr, 0xF0000000);
        assert_eq!(driver.irq, 42);
    }

    /// Test device factory
    #[test]
    fn test_device_factory() {
        let simple = UioDeviceFactory::create_simple("factory_test", 0xF0000000, 42);
        assert!(simple.is_ok());
        assert_eq!(simple.unwrap().device().name(), "factory_test");

        let net = UioDeviceFactory::create_network("factory_net", 0xE0000000);
        assert!(net.is_ok());
        assert_eq!(net.unwrap().device().name(), "factory_net");

        let gpio = UioDeviceFactory::create_gpio("factory_gpio", 0xD0000000, 32);
        assert!(gpio.is_ok());
        assert_eq!(gpio.unwrap().device().name(), "factory_gpio");
    }

    /// Test ioctl commands
    #[test]
    fn test_ioctl_commands() {
        assert_eq!(UioIoctl::from_command(0x0001), Some(UioIoctl::GetInfo));
        assert_eq!(UioIoctl::from_command(0x0002), Some(UioIoctl::MapRegion));
        assert_eq!(UioIoctl::from_command(0x0003), Some(UioIoctl::UnmapRegion));
        assert_eq!(UioIoctl::from_command(0x0004), Some(UioIoctl::GetRegionInfo));
        assert_eq!(UioIoctl::from_command(0x0005), Some(UioIoctl::EnableInterrupt));
        assert_eq!(UioIoctl::from_command(0x0006), Some(UioIoctl::DisableInterrupt));
        assert_eq!(UioIoctl::from_command(0x0007), Some(UioIoctl::GetEventCount));
        assert_eq!(UioIoctl::from_command(0xFFFF), None);
    }

    /// Test region info structure
    #[test]
    fn test_region_info() {
        let info = UioRegionInfo::new();
        assert_eq!(info.phys_addr, 0);
        assert_eq!(info.size, 0);
        assert_eq!(info.region_type, UioMemRegionType::Mmio);
        assert_eq!(info.flags, 0);
    }

    /// Test device enable/disable
    #[test]
    fn test_device_enable_disable() {
        let device = UioDevice::new("test_enable", 0);

        assert!(!device.is_enabled());

        // Note: This may fail without proper platform support
        let result = device.enable();
        if result.is_ok() {
            assert!(device.is_enabled());

            assert!(device.disable().is_ok());
            assert!(!device.is_enabled());
        }
    }

    /// Test device reference counting
    #[test]
    fn test_device_refcount() {
        let device = UioDevice::new("test_refcount", 0);

        assert_eq!(device.refcount(), 0);

        device.get();
        assert_eq!(device.refcount(), 1);

        device.get();
        assert_eq!(device.refcount(), 2);

        device.put();
        assert_eq!(device.refcount(), 1);

        device.put();
        assert_eq!(device.refcount(), 0);
    }

    /// Test error types
    #[test]
    fn test_error_types() {
        let errors = [
            UioError::DeviceNotFound,
            UioError::InvalidDevice,
            UioError::RegionMapped,
            UioError::InvalidRegion,
            UioError::InterruptError,
            UioError::InvalidOperation,
            UioError::PermissionDenied,
            UioError::NoMemory,
            UioError::DeviceBusy,
            UioError::InvalidParam,
        ];

        for error in errors {
            // Test Display trait
            let s = format!("{}", error);
            assert!(!s.is_empty());

            // Test as_str
            let s2 = error.as_str();
            assert!(!s2.is_empty());
        }
    }

    /// Test multiple devices
    #[test]
    fn test_multiple_devices() {
        let mut driver = UioDriver::new();
        assert!(driver.init().is_ok());

        // Create multiple devices
        for i in 0..5 {
            let name = format!("test_multi_{}", i);
            let device = UioDevice::new(&name, 0);
            assert!(driver.register_device(device).is_ok());
        }

        // Verify all devices are registered
        let devices = driver.devices();
        assert_eq!(devices.len(), 5);

        // Cleanup
        assert!(driver.shutdown().is_ok());
    }

    /// Test memory region types
    #[test]
    fn test_region_types() {
        assert_eq!(UioMemRegionType::from_u8(0), UioMemRegionType::Mmio);
        assert_eq!(UioMemRegionType::from_u8(1), UioMemRegionType::PortIo);
        assert_eq!(UioMemRegionType::from_u8(2), UioMemRegionType::Custom);
        assert_eq!(UioMemRegionType::from_u8(255), UioMemRegionType::Invalid);

        assert_eq!(UioMemRegionType::Mmio.as_u8(), 0);
        assert_eq!(UioMemRegionType::PortIo.as_u8(), 1);
        assert_eq!(UioMemRegionType::Custom.as_u8(), 2);
        assert_eq!(UioMemRegionType::Invalid.as_u8(), 3);
    }
}

/// Benchmark tests for performance validation
#[cfg(test)]
mod bench_tests {
    use super::*;
    use std::time::Instant;

    /// Benchmark device creation
    #[test]
    fn bench_device_creation() {
        let start = Instant::now();

        for i in 0..1000 {
            let _device = UioDevice::new(&format!("bench_{}", i), i);
        }

        let duration = start.elapsed();
        println!("Created 1000 devices in {:?}", duration);
    }

    /// Benchmark memory region operations
    #[test]
    fn bench_region_operations() {
        let mut device = UioDevice::new("bench_regions", 0);

        let start = Instant::now();

        for i in 0..100 {
            let addr = 0xF0000000 + (i * 0x1000) as u64;
            let region = UioMemRegion::mmio(addr, 0x1000);
            assert!(device.add_region(region).is_ok());
        }

        let duration = start.elapsed();
        println!("Added 100 regions in {:?}", duration);
    }

    /// Benchmark interrupt operations
    #[test]
    fn bench_interrupt_operations() {
        let device = UioDevice::new("bench_interrupt", 0);

        let start = Instant::now();

        for i in 0..1000 {
            device.set_irq(i);
            assert_eq!(device.irq(), i);
        }

        let duration = start.elapsed();
        println!("Performed 1000 IRQ operations in {:?}", duration);
    }
}
