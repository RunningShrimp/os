//! # Virtio MMIO Transport Layer
//!
//! This module implements the Virtio MMIO (Memory-Mapped I/O) transport layer,
//! which is used for Virtio devices on ARM and other systems that use MMIO
//! instead of PCI for device communication.
//!
//! ## Architecture
//!
//! The MMIO transport provides:
//! - Device discovery through memory-mapped registers
//! - Interrupt handling for device notifications
//! - Configuration space access
//! - Queue management
//!
//! ## Memory Layout
//!
//! Each Virtio MMIO device has a 4KB region of memory-mapped registers:
//! - 0x000-0x0FF: Registers
//! - 0x100-0xFFF: Configuration space
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::virtio::mmio::{VirtioMmioDevice, VirtioMmioTransport};
//!
//! let mut transport = VirtioMmioTransport::new(0x1A000000);
//! let devices = transport.discover_devices();
//! ```

use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};

use super::virtio::{VirtioDevice, VirtioDeviceId, VirtioError, Virtqueue, VirtioDeviceStatus};

/// Virtio MMIO register offsets
#[repr(usize)]
pub enum VirtioMmioRegister {
    MagicValue = 0x000,
    Version = 0x004,
    DeviceId = 0x008,
    VendorId = 0x00C,
    HostFeatures = 0x010,
    HostFeaturesSel = 0x014,
    GuestFeatures = 0x020,
    GuestFeaturesSel = 0x024,
    GuestPageSize = 0x028,
    QueueSel = 0x030,
    QueueNumMax = 0x034,
    QueueNum = 0x038,
    QueueReady = 0x044,
    QueueNotify = 0x050,
    InterruptStatus = 0x060,
    InterruptAck = 0x064,
    Status = 0x070,
    QueueDescLow = 0x080,
    QueueDescHigh = 0x084,
    QueueDriverLow = 0x090,
    QueueDriverHigh = 0x094,
    QueueDeviceLow = 0x0A0,
    QueueDeviceHigh = 0x0A4,
    ConfigGeneration = 0x0FC,
}

/// Magic value "virt" to identify Virtio devices
pub const VIRTIO_MMIO_MAGIC_VALUE: u32 = 0x74726976;

/// Valid Virtio version
pub const VIRTIO_MMIO_VERSION: u32 = 1;

/// Virtio MMIO device
pub struct VirtioMmioDevice {
    /// Base address of MMIO region
    base_addr: usize,
    /// Device ID
    device_id: VirtioDeviceId,
    /// Vendor ID
    vendor_id: u32,
    /// Current status
    status: u8,
    /// Number of queues
    num_queues: usize,
    /// Virtqueues
    queues: Vec<Option<Virtqueue>>,
    /// Device features
    host_features: u64,
    /// Driver features
    guest_features: u64,
    /// IRQ number for this device
    irq: u32,
}

impl VirtioMmioDevice {
    /// Create a new Virtio MMIO device
    ///
    /// # Arguments
    ///
    /// * `base_addr` - Base address of the MMIO region
    /// * `irq` - IRQ number for this device
    pub fn new(base_addr: usize, irq: u32) -> Result<Self, VirtioError> {
        // Verify magic value
        let magic = unsafe { read_volatile(base_addr as *const u32) };
        if magic != VIRTIO_MMIO_MAGIC_VALUE {
            return Err(VirtioError::InvalidDeviceType);
        }

        // Verify version
        let version = unsafe { read_volatile((base_addr + 0x004) as *const u32) };
        if version != VIRTIO_MMIO_VERSION {
            return Err(VirtioError::InvalidDeviceType);
        }

        // Read device ID
        let device_id_value = unsafe { read_volatile((base_addr + 0x008) as *const u32) };
        let device_id = VirtioDeviceId::from(device_id_value);

        if device_id == VirtioDeviceId::Invalid {
            return Err(VirtioError::InvalidDeviceType);
        }

        // Read vendor ID
        let vendor_id = unsafe { read_volatile((base_addr + 0x00C) as *const u32) };

        Ok(VirtioMmioDevice {
            base_addr,
            device_id,
            vendor_id,
            status: 0,
            num_queues: 0,
            queues: Vec::new(),
            host_features: 0,
            guest_features: 0,
            irq,
        })
    }

    /// Initialize the device
    pub fn init(&mut self) -> Result<(), VirtioError> {
        // Reset device
        self.write_register_u8(VirtioMmioRegister::Status, 0);

        // Set ACKNOWLEDGE status bit
        self.set_status_bit(VirtioDeviceStatus::Acknowledge);

        // Set DRIVER status bit
        self.set_status_bit(VirtioDeviceStatus::Driver);

        // Read host features
        self.host_features = self.read_host_features();

        // TODO: Negotiate features based on device type
        self.guest_features = self.host_features;

        // Write guest features
        self.write_guest_features(self.guest_features);

        // Set FEATURES_OK status bit
        self.set_status_bit(VirtioDeviceStatus::FeaturesOk);

        // Verify features were accepted
        if !self.check_features_ok() {
            self.set_status_bit(VirtioDeviceStatus::Failed);
            return Err(VirtioError::FeaturesNotAccepted);
        }

        // Discover and initialize queues
        self.discover_queues()?;

        Ok(())
    }

    /// Discover available virtqueues
    fn discover_queues(&mut self) -> Result<(), VirtioError> {
        let mut queue_num = 0;

        loop {
            // Select queue
            self.write_register_u32(VirtioMmioRegister::QueueSel, queue_num as u32);

            // Check if queue exists by reading QueueNumMax
            let queue_num_max = self.read_register_u32(VirtioMmioRegister::QueueNumMax);

            if queue_num_max == 0 {
                break; // No more queues
            }

            // TODO: Allocate and initialize queue
            // For now, just mark as present
            self.queues.push(None);

            queue_num += 1;
        }

        self.num_queues = queue_num;
        Ok(())
    }

    /// Configure a virtqueue
    ///
    /// # Arguments
    ///
    /// * `queue_idx` - Queue index
    /// * `queue_size` - Queue size (must be power of 2 and <= QueueNumMax)
    /// * `desc_addr` - Physical address of descriptor table
    /// * `avail_addr` - Physical address of available ring
    /// * `used_addr` - Physical address of used ring
    pub fn configure_queue(
        &mut self,
        queue_idx: usize,
        queue_size: u16,
        desc_addr: u64,
        avail_addr: u64,
        used_addr: u64,
    ) -> Result<(), VirtioError> {
        if queue_idx >= self.num_queues {
            return Err(VirtioError::QueueInitFailed);
        }

        // Select the queue
        self.write_register_u32(VirtioMmioRegister::QueueSel, queue_idx as u32);

        // Check maximum size
        let max_size = self.read_register_u32(VirtioMmioRegister::QueueNumMax);
        if queue_size > max_size as u16 || !queue_size.is_power_of_two() {
            return Err(VirtioError::QueueInitFailed);
        }

        // Set queue size
        self.write_register_u32(VirtioMmioRegister::QueueNum, queue_size as u32);

        // Set queue addresses
        self.write_register_u32(VirtioMmioRegister::QueueDescLow, (desc_addr & 0xFFFFFFFF) as u32);
        self.write_register_u32(VirtioMmioRegister::QueueDescHigh, (desc_addr >> 32) as u32);

        self.write_register_u32(VirtioMmioRegister::QueueDriverLow, (avail_addr & 0xFFFFFFFF) as u32);
        self.write_register_u32(VirtioMmioRegister::QueueDriverHigh, (avail_addr >> 32) as u32);

        self.write_register_u32(VirtioMmioRegister::QueueDeviceLow, (used_addr & 0xFFFFFFFF) as u32);
        self.write_register_u32(VirtioMmioRegister::QueueDeviceHigh, (used_addr >> 32) as u32);

        // Mark queue as ready
        self.write_register_u32(VirtioMmioRegister::QueueReady, 1);

        // Create virtqueue object
        let virtqueue = Virtqueue::new(queue_idx as u16, queue_size, desc_addr, avail_addr, used_addr);
        self.queues[queue_idx] = Some(virtqueue);

        Ok(())
    }

    /// Complete device initialization
    pub fn finish_init(&mut self) {
        // Set DRIVER_OK status bit
        self.set_status_bit(VirtioDeviceStatus::DriverOk);
    }

    /// Notify the device about available buffers
    ///
    /// # Arguments
    ///
    /// * `queue_idx` - Queue index to notify
    pub fn notify_queue(&self, queue_idx: usize) {
        if queue_idx < self.num_queues {
            self.write_register_u32(VirtioMmioRegister::QueueSel, queue_idx as u32);
            self.write_register_u32(VirtioMmioRegister::QueueNotify, queue_idx as u32);
        }
    }

    /// Get and acknowledge interrupt status
    ///
    /// # Returns
    ///
    /// Interrupt status flags
    pub fn get_interrupt_status(&self) -> u8 {
        self.read_register_u8(VirtioMmioRegister::InterruptStatus)
    }

    /// Acknowledge interrupt
    ///
    /// # Arguments
    ///
    /// * `status` - Interrupt status to acknowledge
    pub fn acknowledge_interrupt(&self, status: u8) {
        self.write_register_u8(VirtioMmioRegister::InterruptAck, status);
    }

    /// Read device configuration space
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in configuration space
    /// * `data` - Buffer to read into
    pub fn read_config(&self, offset: usize, data: &mut [u8]) {
        let config_base = self.base_addr + 0x100;
        for (i, byte) in data.iter_mut().enumerate() {
            unsafe {
                *byte = read_volatile((config_base + offset + i) as *const u8);
            }
        }
    }

    /// Write device configuration space
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in configuration space
    /// * `data` - Data to write
    pub fn write_config(&self, offset: usize, data: &[u8]) {
        let config_base = self.base_addr + 0x100;
        for (i, &byte) in data.iter().enumerate() {
            unsafe {
                write_volatile((config_base + offset + i) as *mut u8, byte);
            }
        }
    }

    /// Get device type
    pub fn device_type(&self) -> VirtioDeviceId {
        self.device_id
    }

    /// Get vendor ID
    pub fn vendor_id(&self) -> u32 {
        self.vendor_id
    }

    /// Get IRQ number
    pub fn irq(&self) -> u32 {
        self.irq
    }

    /// Get virtqueue
    pub fn get_queue(&mut self, index: usize) -> Option<&mut Virtqueue> {
        self.queues.get_mut(index).and_then(|q| q.as_mut())
    }

    // Helper methods

    fn set_status_bit(&mut self, bit: VirtioDeviceStatus) {
        self.status |= bit as u8;
        self.write_register_u8(VirtioMmioRegister::Status, self.status);
    }

    fn check_features_ok(&self) -> bool {
        let status = self.read_register_u8(VirtioMmioRegister::Status);
        status & (VirtioDeviceStatus::FeaturesOk as u8) != 0
    }

    fn read_host_features(&self) -> u64 {
        // Read features low
        self.write_register_u32(VirtioMmioRegister::HostFeaturesSel, 0);
        let features_low = self.read_register_u32(VirtioMmioRegister::HostFeatures);

        // Read features high
        self.write_register_u32(VirtioMmioRegister::HostFeaturesSel, 1);
        let features_high = self.read_register_u32(VirtioMmioRegister::HostFeatures);

        ((features_high as u64) << 32) | (features_low as u64)
    }

    fn write_guest_features(&self, features: u64) {
        // Write features low
        self.write_register_u32(VirtioMmioRegister::GuestFeaturesSel, 0);
        self.write_register_u32(VirtioMmioRegister::GuestFeatures, (features & 0xFFFFFFFF) as u32);

        // Write features high
        self.write_register_u32(VirtioMmioRegister::GuestFeaturesSel, 1);
        self.write_register_u32(
            VirtioMmioRegister::GuestFeatures,
            ((features >> 32) & 0xFFFFFFFF) as u32,
        );
    }

    fn read_register_u8(&self, reg: VirtioMmioRegister) -> u8 {
        unsafe { read_volatile((self.base_addr + reg as usize) as *const u8) }
    }

    fn read_register_u32(&self, reg: VirtioMmioRegister) -> u32 {
        unsafe { read_volatile((self.base_addr + reg as usize) as *const u32) }
    }

    fn write_register_u8(&self, reg: VirtioMmioRegister, value: u8) {
        unsafe { write_volatile((self.base_addr + reg as usize) as *mut u8, value) }
    }

    fn write_register_u32(&self, reg: VirtioMmioRegister, value: u32) {
        unsafe { write_volatile((self.base_addr + reg as usize) as *mut u32, value) }
    }
}

/// Virtio MMIO transport manager
pub struct VirtioMmioTransport {
    /// Base address for MMIO device region
    base_addr: usize,
    /// Number of devices to probe
    num_devices: usize,
    /// Device stride (typically 4KB)
    stride: usize,
}

impl VirtioMmioTransport {
    /// Create a new Virtio MMIO transport manager
    ///
    /// # Arguments
    ///
    /// * `base_addr` - Base address of the MMIO region
    /// * `num_devices` - Number of devices to probe
    /// * `stride` - Device stride (default 0x1000 for 4KB)
    pub fn new(base_addr: usize, num_devices: usize, stride: usize) -> Self {
        VirtioMmioTransport {
            base_addr,
            num_devices,
            stride: stride.max(0x1000),
        }
    }

    /// Discover all Virtio MMIO devices
    ///
    /// # Returns
    ///
    /// Vector of discovered devices
    pub fn discover_devices(&self) -> Vec<VirtioMmioDevice> {
        let mut devices = Vec::new();

        for i in 0..self.num_devices {
            let device_base = self.base_addr + i * self.stride;

            // Try to create device at this address
            // IRQs are typically assigned sequentially starting from a base
            let irq = 32 + i as u32; // Example: start from IRQ 32

            if let Ok(device) = VirtioMmioDevice::new(device_base, irq) {
                devices.push(device);
            }
        }

        devices
    }

    /// Initialize all discovered devices
    ///
    /// # Arguments
    ///
    /// * `devices` - Devices to initialize
    pub fn init_devices(&self, devices: &mut [VirtioMmioDevice]) -> Result<(), VirtioError> {
        for device in devices {
            device.init()?;
            device.finish_init();
        }
        Ok(())
    }
}

/// Interrupt handler for Virtio MMIO devices
///
/// # Arguments
///
/// * `irq` - IRQ number
/// * `devices` - List of devices to check
pub fn handle_virtio_irq(irq: u32, devices: &mut [VirtioMmioDevice]) {
    for device in devices {
        if device.irq() == irq {
            let status = device.get_interrupt_status();

            // Handle used buffer interrupt
            if status & 0x01 != 0 {
                // TODO: Process used buffers
            }

            // Handle configuration change interrupt
            if status & 0x02 != 0 {
                // TODO: Handle configuration change
            }

            // Acknowledge interrupt
            device.acknowledge_interrupt(status);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_value() {
        assert_eq!(VIRTIO_MMIO_MAGIC_VALUE, 0x74726976);
    }

    #[test]
    fn test_transport_creation() {
        let transport = VirtioMmioTransport::new(0x1A000000, 4, 0x1000);
        assert_eq!(transport.base_addr, 0x1A000000);
        assert_eq!(transport.num_devices, 4);
        assert_eq!(transport.stride, 0x1000);
    }
}
