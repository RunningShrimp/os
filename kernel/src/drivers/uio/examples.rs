//! Example UIO drivers
//!
//! This module provides example implementations demonstrating how to
//! use the UIO framework to create userspace I/O drivers.

use crate::drivers::uio::{
    device::{device_add, device_remove, UioDevice},
    interrupt::UioInterruptHandler,
    memory::UioMemRegion,
    UioResult, UIO_MAX_REGIONS,
};

/// Simple UIO example driver
///
/// Demonstrates a minimal UIO device with one memory region and interrupt.
#[derive(Debug)]
pub struct SimpleUioDriver {
    /// UIO device
    device: UioDevice,
    /// Base address
    base_addr: u64,
    /// IRQ number
    irq: u32,
}

impl SimpleUioDriver {
    /// Create a new simple UIO driver
    pub fn new(name: &str, base_addr: u64, irq: u32) -> Self {
        let device = UioDevice::new(name, 0);
        Self {
            device,
            base_addr,
            irq,
        }
    }

    /// Initialize and register the device
    pub fn init(&mut self) -> UioResult<()> {
        // Add memory region (device registers)
        let region = UioMemRegion::mmio(self.base_addr, 0x1000);
        self.device.add_region(region)?;

        // Register interrupt handler
        self.device.register_interrupt(|irq| {
            log::info!("Simple UIO interrupt: {}", irq);
            Ok(())
        })?;

        // Set IRQ
        self.device.set_irq(self.irq);

        // Register with subsystem
        device_add(self.device.clone())?;

        // Enable device
        self.device.enable()?;

        Ok(())
    }

    /// Cleanup and unregister device
    pub fn cleanup(&self) -> UioResult<()> {
        self.device.disable()?;
        device_remove(self.device.name())
    }

    /// Get device reference
    pub fn device(&self) -> &UioDevice {
        &self.device
    }
}

/// UIO PCI driver template
///
/// Demonstrates a PCI-based UIO driver with multiple memory regions.
#[derive(Debug)]
pub struct UioPciDriver {
    /// UIO device
    device: UioDevice,
    /// PCI vendor ID
    vendor_id: u16,
    /// PCI device ID
    device_id: u16,
    /// BAR addresses
    bars: [Option<u64>; 6],
}

impl UioPciDriver {
    /// Create a new UIO PCI driver
    pub fn new(name: &str, vendor_id: u16, device_id: u16) -> Self {
        let device = UioDevice::new(name, 0);
        Self {
            device,
            vendor_id,
            device_id,
            bars: [None; 6],
        }
    }

    /// Initialize PCI driver
    pub fn init(&mut self) -> UioResult<()> {
        // Probe PCI device
        self.probe_pci()?;

        // Add memory regions for each BAR
        for (i, bar) in self.bars.iter().enumerate() {
            if let Some(addr) = bar {
                let region = UioMemRegion::mmio(addr, 0x1000);
                self.device.add_region(region)?;
            }
        }

        // Set up MSI interrupt if available
        self.setup_msi()?;

        // Register with subsystem
        device_add(self.device.clone())?;

        // Enable device
        self.device.enable()?;

        Ok(())
    }

    /// Probe PCI device and read BARs
    fn probe_pci(&mut self) -> UioResult<()> {
        // Platform-specific PCI probing
        // This would read PCI configuration space and BAR addresses

        // Example: Mock BAR addresses
        self.bars[0] = Some(0xF0000000);
        self.bars[1] = Some(0xF0001000);

        log::info!(
            "Probed PCI device {:04X}:{:04X}",
            self.vendor_id,
            self.device_id
        );

        Ok(())
    }

    /// Setup MSI interrupts
    fn setup_msi(&self) -> UioResult<()> {
        // Platform-specific MSI setup
        let irq = 42; // Example IRQ number

        self.device.register_interrupt(|irq| {
            log::info!("PCI UIO MSI interrupt: {}", irq);
            Ok(())
        })?;

        self.device.set_irq(irq);

        Ok(())
    }

    /// Cleanup PCI driver
    pub fn cleanup(&self) -> UioResult<()> {
        self.device.disable()?;
        device_remove(self.device.name())
    }

    /// Get device reference
    pub fn device(&self) -> &UioDevice {
        &self.device
    }
}

/// Network card UIO driver example
///
/// Demonstrates a network card UIO driver with register access
/// and interrupt handling for packet reception.
#[derive(Debug)]
pub struct UioNetDriver {
    /// UIO device
    device: UioDevice,
    /// Register base
    reg_base: u64,
    /// RX descriptor base
    rx_desc_base: u64,
    /// TX descriptor base
    tx_desc_base: u64,
}

impl UioNetDriver {
    /// Create a new network UIO driver
    pub fn new(name: &str, reg_base: u64) -> Self {
        let device = UioDevice::new(name, 0);
        Self {
            device,
            reg_base,
            rx_desc_base: reg_base + 0x1000,
            tx_desc_base: reg_base + 0x2000,
        }
    }

    /// Initialize network driver
    pub fn init(&mut self) -> UioResult<()> {
        // Add register regions
        let reg_region = UioMemRegion::mmio(self.reg_base, 0x1000);
        self.device.add_region(reg_region)?;

        let rx_region = UioMemRegion::mmio(self.rx_desc_base, 0x1000);
        self.device.add_region(rx_region)?;

        let tx_region = UioMemRegion::mmio(self.tx_desc_base, 0x1000);
        self.device.add_region(tx_region)?;

        // Register interrupt handler
        self.device.register_interrupt(|irq| {
            log::info!("Network UIO interrupt: {}", irq);
            // Handle RX/TX completion
            Ok(())
        })?;

        // Set IRQ
        self.device.set_irq(43);

        // Register device
        device_add(self.device.clone())?;

        // Enable device
        self.device.enable()?;

        Ok(())
    }

    /// Cleanup network driver
    pub fn cleanup(&self) -> UioResult<()> {
        self.device.disable()?;
        device_remove(self.device.name())
    }

    /// Get device reference
    pub fn device(&self) -> &UioDevice {
        &self.device
    }
}

/// UIO GPIO driver example
///
/// Demonstrates a GPIO controller UIO driver with pin control.
#[derive(Debug)]
pub struct UioGpioDriver {
    /// UIO device
    device: UioDevice,
    /// GPIO register base
    reg_base: u64,
    /// Number of GPIO pins
    num_pins: u32,
}

impl UioGpioDriver {
    /// Create a new GPIO UIO driver
    pub fn new(name: &str, reg_base: u64, num_pins: u32) -> Self {
        let device = UioDevice::new(name, 0);
        Self {
            device,
            reg_base,
            num_pins,
        }
    }

    /// Initialize GPIO driver
    pub fn init(&mut self) -> UioResult<()> {
        // Add register region
        let region = UioMemRegion::mmio(self.reg_base, 0x1000);
        self.device.add_region(region)?;

        // Register interrupt handler for GPIO events
        self.device.register_interrupt(|irq| {
            log::info!("GPIO UIO interrupt: {}", irq);
            // Handle GPIO edge events
            Ok(())
        })?;

        // Set IRQ
        self.device.set_irq(44);

        // Register device
        device_add(self.device.clone())?;

        // Enable device
        self.device.enable()?;

        Ok(())
    }

    /// Cleanup GPIO driver
    pub fn cleanup(&self) -> UioResult<()> {
        self.device.disable()?;
        device_remove(self.device.name())
    }

    /// Get device reference
    pub fn device(&self) -> &UioDevice {
        &self.device
    }
}

/// Custom UIO device with specific requirements
///
/// Example showing how to create a custom UIO device with
/// non-standard memory regions and interrupt handling.
#[derive(Debug)]
pub struct CustomUioDevice {
    /// UIO device
    device: UioDevice,
    /// Custom configuration
    config: CustomConfig,
}

/// Custom device configuration
#[derive(Debug, Clone)]
pub struct CustomConfig {
    /// Device name
    pub name: String,
    /// Memory regions
    pub regions: Vec<(u64, usize)>,
    /// IRQ number
    pub irq: u32,
    /// Interrupt handler name
    pub handler_name: String,
}

impl CustomUioDevice {
    /// Create a custom UIO device
    pub fn new(config: CustomConfig) -> Self {
        let device = UioDevice::new(&config.name, 0);
        Self { device, config }
    }

    /// Initialize custom device
    pub fn init(&mut self) -> UioResult<()> {
        // Add all memory regions
        for (addr, size) in &self.config.regions {
            let region = UioMemRegion::custom(*addr, *size, "custom".to_string());
            self.device.add_region(region)?;
        }

        // Register custom interrupt handler
        let handler_name = self.config.handler_name.clone();
        self.device.register_interrupt(move |irq| {
            log::info!("{} interrupt: {}", handler_name, irq);
            Ok(())
        })?;

        // Set IRQ
        self.device.set_irq(self.config.irq);

        // Register device
        device_add(self.device.clone())?;

        // Enable device
        self.device.enable()?;

        Ok(())
    }

    /// Cleanup custom device
    pub fn cleanup(&self) -> UioResult<()> {
        self.device.disable()?;
        device_remove(self.device.name())
    }

    /// Get device reference
    pub fn device(&self) -> &UioDevice {
        &self.device
    }
}

/// UIO device factory
///
/// Factory pattern for creating different types of UIO devices.
#[derive(Debug)]
pub struct UioDeviceFactory;

impl UioDeviceFactory {
    /// Create a simple UIO device
    pub fn create_simple(name: &str, base: u64, irq: u32) -> UioResult<SimpleUioDriver> {
        let driver = SimpleUioDriver::new(name, base, irq);
        Ok(driver)
    }

    /// Create a PCI UIO device
    pub fn create_pci(name: &str, vendor: u16, device: u16) -> UioResult<UioPciDriver> {
        let driver = UioPciDriver::new(name, vendor, device);
        Ok(driver)
    }

    /// Create a network UIO device
    pub fn create_network(name: &str, base: u64) -> UioResult<UioNetDriver> {
        let driver = UioNetDriver::new(name, base);
        Ok(driver)
    }

    /// Create a GPIO UIO device
    pub fn create_gpio(name: &str, base: u64, pins: u32) -> UioResult<UioGpioDriver> {
        let driver = UioGpioDriver::new(name, base, pins);
        Ok(driver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_driver_create() {
        let driver = SimpleUioDriver::new("test_simple", 0xF0000000, 42);
        assert_eq!(driver.device.name(), "test_simple");
        assert_eq!(driver.base_addr, 0xF0000000);
        assert_eq!(driver.irq, 42);
    }

    #[test]
    fn test_pci_driver_create() {
        let driver = UioPciDriver::new("test_pci", 0x1234, 0x5678);
        assert_eq!(driver.device.name(), "test_pci");
        assert_eq!(driver.vendor_id, 0x1234);
        assert_eq!(driver.device_id, 0x5678);
    }

    #[test]
    fn test_net_driver_create() {
        let driver = UioNetDriver::new("test_net", 0xE0000000);
        assert_eq!(driver.device.name(), "test_net");
        assert_eq!(driver.reg_base, 0xE0000000);
    }

    #[test]
    fn test_gpio_driver_create() {
        let driver = UioGpioDriver::new("test_gpio", 0xD0000000, 32);
        assert_eq!(driver.device.name(), "test_gpio");
        assert_eq!(driver.reg_base, 0xD0000000);
        assert_eq!(driver.num_pins, 32);
    }

    #[test]
    fn test_factory_create() {
        let simple = UioDeviceFactory::create_simple("factory_test", 0xF0000000, 42);
        assert!(simple.is_ok());

        let pci = UioDeviceFactory::create_pci("factory_pci", 0x1234, 0x5678);
        assert!(pci.is_ok());

        let net = UioDeviceFactory::create_network("factory_net", 0xE0000000);
        assert!(net.is_ok());

        let gpio = UioDeviceFactory::create_gpio("factory_gpio", 0xD0000000, 32);
        assert!(gpio.is_ok());
    }

    #[test]
    fn test_custom_device_create() {
        let config = CustomConfig {
            name: "custom_test".to_string(),
            regions: vec![(0xF0000000, 0x1000), (0xF0001000, 0x2000)],
            irq: 42,
            handler_name: "custom_handler".to_string(),
        };

        let device = CustomUioDevice::new(config);
        assert_eq!(device.device.name(), "custom_test");
        assert_eq!(device.config.regions.len(), 2);
        assert_eq!(device.config.irq, 42);
    }
}
