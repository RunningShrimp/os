//! # Virtio Device Framework
//!
//! This module provides a comprehensive framework for managing Virtio devices,
//! which are the standard I/O virtualization framework used in virtual machines.
//! Virtio devices use a set of Virtqueues (virtual I/O queues) for efficient
//! communication between the guest OS and the hypervisor.
//!
//! ## Architecture
//!
//! The framework consists of:
//! - **Device Discovery**: Detecting and initializing Virtio devices
//! - **Virtqueue Management**: Managing virtqueues for efficient I/O
//! - **Configuration Space**: Handling device configuration
//! - **Interrupt Handling**: Processing device notifications
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::virtio::virtio::{VirtioDevice, VirtioTransport};
//!
//! let mut device = VirtioDevice::new(VirtioTransport::Mmio);
//! device.init();
//! device.configure_queues();
//! ```

use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};

/// Virtio device IDs as specified in the Virtio specification
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioDeviceId {
    Invalid = 0,
    Network = 1,
    Block = 2,
    Console = 3,
    Entropy = 4,
    Balloon = 5,
    IOMemory = 6,
    Rpmsg = 7,
    Scsi = 8,
    NineP = 9,
    Mac80211 = 10,
    RprocSerial = 11,
    CAIF = 12,
    MemoryBalloon = 13,
    GPU = 16,
    Timer = 17,
    Input = 18,
    Socket = 19,
    Crypto = 20,
    SignalDist = 21,
    PStore = 22,
    IOMMU = 23,
    Memory = 24,
}

impl From<u32> for VirtioDeviceId {
    fn from(id: u32) -> Self {
        match id {
            1 => VirtioDeviceId::Network,
            2 => VirtioDeviceId::Block,
            3 => VirtioDeviceId::Console,
            4 => VirtioDeviceId::Entropy,
            5 => VirtioDeviceId::Balloon,
            6 => VirtioDeviceId::IOMemory,
            7 => VirtioDeviceId::Rpmsg,
            8 => VirtioDeviceId::Scsi,
            9 => VirtioDeviceId::NineP,
            10 => VirtioDeviceId::Mac80211,
            11 => VirtioDeviceId::RprocSerial,
            12 => VirtioDeviceId::CAIF,
            13 => VirtioDeviceId::MemoryBalloon,
            16 => VirtioDeviceId::GPU,
            17 => VirtioDeviceId::Timer,
            18 => VirtioDeviceId::Input,
            19 => VirtioDeviceId::Socket,
            20 => VirtioDeviceId::Crypto,
            21 => VirtioDeviceId::SignalDist,
            22 => VirtioDeviceId::PStore,
            23 => VirtioDeviceId::IOMMU,
            24 => VirtioDeviceId::Memory,
            _ => VirtioDeviceId::Invalid,
        }
    }
}

/// Virtqueue descriptor flags
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum VirtqDescFlag {
    Next = 1,      // Buffer continues via the next field
    Write = 2,    // Buffer is write-only (otherwise read-only)
    Indirect = 4, // Buffer contains a list of indirect descriptors
}

/// Virtqueue descriptor for buffer management
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqDesc {
    /// Address (guest-physical)
    pub addr: u64,
    /// Length in bytes
    pub len: u32,
    /// Flags as described in VirtqDescFlag
    pub flags: u16,
    /// Index of the next descriptor if Next flag is set
    pub next: u16,
}

/// Virtqueue available ring (driver -> device)
#[derive(Debug)]
#[repr(C)]
pub struct VirtqAvail {
    /// Flags: currently only 0 or 1 (no interrupt)
    pub flags: u16,
    /// Index into the ring where the next descriptor is available
    pub idx: u16,
    /// The ring of descriptors
    pub ring: [u16; 0], // Flexible array member
    /// Used event (optional)
    pub used_event: u16,
}

/// Virtqueue used ring (device -> driver)
#[derive(Debug)]
#[repr(C)]
pub struct VirtqUsed {
    /// Flags: currently only 0 or 1 (no interrupt)
    pub flags: u16,
    /// Index into the ring where the next used descriptor is
    pub idx: u16,
    /// The ring of used elements
    pub ring: [VirtqUsedElem; 0], // Flexible array member
    /// Available event (optional)
    pub avail_event: u16,
}

/// Element in the used ring
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtqUsedElem {
    /// Index of start of used descriptor chain
    pub id: u32,
    /// Total length of the descriptor chain
    pub len: u32,
}

/// Configuration space common to all Virtio devices
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VirtioCommonConfig {
    /// Device ID
    pub device_id: u32,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device features
    pub device_features: u32,
    /// Device features selector
    pub device_features_sel: u32,
    /// Driver features
    pub driver_features: u32,
    /// Driver features selector
    pub driver_features_sel: u32,
    /// Guest page size
    pub guest_page_size: u32,
    /// Queue select
    pub queue_sel: u32,
    /// Queue size
    pub queue_num: u32,
    /// Queue ready
    pub queue_ready: u32,
    /// Queue notify
    pub queue_notify: u32,
    /// Interrupt status
    pub interrupt_status: u32,
    /// Interrupt acknowledge
    pub interrupt_ack: u32,
    /// Device status
    pub status: u32,
    /// Configuration space
    pub config: [u8; 0], // Flexible array member
}

/// Device status flags
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum VirtioDeviceStatus {
    Acknowledge = 1,
    Driver = 2,
    Failed = 128,
    FeaturesOk = 8,
    DriverOk = 4,
    NeedsReset = 64,
}

/// Virtqueue for managing I/O buffers
pub struct Virtqueue {
    /// Queue index
    index: u16,
    /// Queue size (must be power of 2)
    size: u16,
    /// Descriptor table
    descriptors: *mut VirtqDesc,
    /// Available ring
    avail: *mut VirtqAvail,
    /// Used ring
    used: *mut VirtqUsed,
    /// Free descriptor head
    free_head: u16,
    /// Number of free descriptors
    num_free: u16,
}

unsafe impl Send for Virtqueue {}

impl Virtqueue {
    /// Create a new virtqueue
    ///
    /// # Arguments
    ///
    /// * `index` - Queue index
    /// * `size` - Queue size (must be power of 2)
    /// * `desc_addr` - Physical address of descriptor table
    /// * `avail_addr` - Physical address of available ring
    /// * `used_addr` - Physical address of used ring
    pub fn new(index: u16, size: u16, desc_addr: u64, avail_addr: u64, used_addr: u64) -> Self {
        assert!(size.is_power_of_two(), "Queue size must be power of 2");

        let descriptors = desc_addr as *mut VirtqDesc;
        let avail = avail_addr as *mut VirtqAvail;
        let used = used_addr as *mut VirtqUsed;

        // Initialize descriptors as a free list
        for i in 0..size {
            unsafe {
                let desc = descriptors.add(i as usize);
                write_volatile(&mut (*desc).next, (i + 1) % size);
                write_volatile(&mut (*desc).flags, 0);
            }
        }

        Virtqueue {
            index,
            size,
            descriptors,
            avail,
            used,
            free_head: 0,
            num_free: size,
        }
    }

    /// Add a descriptor to the virtqueue
    ///
    /// # Arguments
    ///
    /// * `addr` - Physical address of the buffer
    /// * `len` - Length of the buffer
    /// * `flags` - Descriptor flags
    ///
    /// # Returns
    ///
    /// Descriptor index or None if no free descriptors
    pub fn add_descriptor(&mut self, addr: u64, len: u32, flags: u16) -> Option<u16> {
        if self.num_free == 0 {
            return None;
        }

        let idx = self.free_head;

        unsafe {
            let desc = self.descriptors.add(idx as usize);
            write_volatile(&mut (*desc).addr, addr);
            write_volatile(&mut (*desc).len, len);
            write_volatile(&mut (*desc).flags, flags);

            // Update free_head
            self.free_head = read_volatile(&(*desc).next);
        }

        self.num_free -= 1;
        Some(idx)
    }

    /// Add a buffer to the available ring
    ///
    /// # Arguments
    ///
    /// * `desc_idx` - Index of the first descriptor
    pub fn add_to_avail_ring(&mut self, desc_idx: u16) {
        unsafe {
            let avail = &*self.avail;
            let idx = read_volatile(&avail.idx) % self.size;
            let ring_ptr = (self.avail as *const u16).add(2 + idx as usize);
            write_volatile(ring_ptr as *mut u16, desc_idx);

            // Update available index
            let idx_ptr = (self.avail as *const u16).add(1);
            write_volatile(idx_ptr as *mut u16, read_volatile(idx_ptr) + 1);
        }
    }

    /// Get used buffer from the used ring
    ///
    /// # Returns
    ///
    /// Option with used element if available
    pub fn get_used_buffer(&mut self) -> Option<VirtqUsedElem> {
        unsafe {
            let used = &*self.used;
            let last_used_idx = 0; // Should track this properly
            let used_idx = read_volatile(&used.idx);

            if last_used_idx == used_idx {
                return None;
            }

            let idx = last_used_idx % self.size;
            let ring_ptr = (self.used as *const u8).add(4 + (idx as usize) * 8) as *const VirtqUsedElem;
            Some(read_volatile(&*ring_ptr))
        }
    }

    /// Notify the device about available buffers
    pub fn notify(&self) {
        // Implementation depends on transport (MMIO/PCI)
        // This would write to the queue notify register
    }

    /// Get the number of free descriptors
    pub fn num_free(&self) -> u16 {
        self.num_free
    }
}

/// Virtio device abstraction
pub struct VirtioDevice {
    /// Device type
    device_type: VirtioDeviceId,
    /// Device status
    status: u8,
    /// Virtqueues
    queues: Vec<Option<Virtqueue>>,
    /// Device features
    device_features: u64,
    /// Driver features
    driver_features: u64,
    /// Configuration space address
    config_addr: usize,
}

impl VirtioDevice {
    /// Create a new Virtio device
    ///
    /// # Arguments
    ///
    /// * `device_type` - Type of the Virtio device
    /// * `config_addr` - Physical address of configuration space
    pub fn new(device_type: VirtioDeviceId, config_addr: usize) -> Self {
        VirtioDevice {
            device_type,
            status: 0,
            queues: Vec::new(),
            device_features: 0,
            driver_features: 0,
            config_addr,
        }
    }

    /// Initialize the device
    pub fn init(&mut self) -> Result<(), VirtioError> {
        // Reset device
        self.write_status(0);

        // Set ACKNOWLEDGE status bit
        self.set_status(VirtioDeviceStatus::Acknowledge as u8);

        // Set DRIVER status bit
        self.set_status(VirtioDeviceStatus::Driver as u8);

        // Read device features
        self.device_features = self.read_device_features();

        // GH-#1239: Negotiate features
        // See: https://github.com/npos/kernel/issues/1239
        self.driver_features = self.device_features;

        // Write driver features
        self.write_driver_features(self.driver_features);

        // Set FEATURES_OK status bit
        self.set_status(VirtioDeviceStatus::FeaturesOk as u8);

        // Verify features were accepted
        if !self.check_features_ok() {
            self.set_status(VirtioDeviceStatus::Failed as u8);
            return Err(VirtioError::FeaturesNotAccepted);
        }

        // Initialize virtqueues
        self.init_queues()?;

        Ok(())
    }

    /// Configure and initialize virtqueues
    fn init_queues(&mut self) -> Result<(), VirtioError> {
        // GH-#1240: Allocate memory for queues
        // See: https://github.com/npos/kernel/issues/1240
        // For now, just create placeholder queues
        self.queues.push(None);
        Ok(())
    }

    /// Complete device initialization
    pub fn finish_init(&mut self) {
        // Set DRIVER_OK status bit
        self.set_status(VirtioDeviceStatus::DriverOk as u8);
    }

    /// Read device status
    pub fn read_status(&self) -> u8 {
        unsafe {
            let config = self.config_addr as *const u8;
            // Offset 0x14 in common config is status
            read_volatile(config.add(0x14))
        }
    }

    /// Write device status
    fn write_status(&self, status: u8) {
        unsafe {
            let config = self.config_addr as *mut u8;
            write_volatile(config.add(0x14), status);
        }
    }

    /// Set status bit
    fn set_status(&mut self, bit: u8) {
        self.status |= bit;
        self.write_status(self.status);
    }

    /// Read device features
    fn read_device_features(&self) -> u64 {
        unsafe {
            let config = self.config_addr as *const u32;
            let features_low = read_volatile(config.add(0x10));
            let features_high = read_volatile(config.add(0x14));
            (features_high as u64) << 32 | (features_low as u64)
        }
    }

    /// Write driver features
    fn write_driver_features(&self, features: u64) {
        unsafe {
            let config = self.config_addr as *mut u32;
            write_volatile(config.add(0x04), (features & 0xFFFFFFFF) as u32);
            write_volatile(config.add(0x08), ((features >> 32) & 0xFFFFFFFF) as u32);
        }
    }

    /// Check if features were accepted
    fn check_features_ok(&self) -> bool {
        self.read_status() & (VirtioDeviceStatus::FeaturesOk as u8) != 0
    }

    /// Get device type
    pub fn device_type(&self) -> VirtioDeviceId {
        self.device_type
    }

    /// Get device configuration space
    pub fn get_config(&self, offset: usize, size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);
        unsafe {
            let config = self.config_addr as *const u8;
            // Configuration space starts at offset 0x100
            let config_base = config.add(0x100 + offset);
            for i in 0..size {
                data.push(read_volatile(config_base.add(i)));
            }
        }
        data
    }

    /// Set device configuration
    pub fn set_config(&self, offset: usize, data: &[u8]) {
        unsafe {
            let config = self.config_addr as *mut u8;
            let config_base = config.add(0x100 + offset);
            for (i, &byte) in data.iter().enumerate() {
                write_volatile(config_base.add(i), byte);
            }
        }
    }
}

/// Virtio errors
#[derive(Debug)]
pub enum VirtioError {
    /// Features not accepted by device
    FeaturesNotAccepted,
    /// Queue initialization failed
    QueueInitFailed,
    /// Invalid device type
    InvalidDeviceType,
    /// Transport-specific error
    TransportError,
    /// Device not ready
    DeviceNotReady,
}

/// Discover Virtio devices on the system
pub fn discover_virtio_devices() -> Vec<VirtioDevice> {
    let mut devices = Vec::new();

    // This is a placeholder implementation
    // In a real system, this would scan the MMIO or PCI space for Virtio devices

    devices.push(VirtioDevice::new(
        VirtioDeviceId::Network,
        0, // Placeholder address
    ));

    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_conversion() {
        assert_eq!(VirtioDeviceId::from(1), VirtioDeviceId::Network);
        assert_eq!(VirtioDeviceId::from(2), VirtioDeviceId::Block);
        assert_eq!(VirtioDeviceId::from(999), VirtioDeviceId::Invalid);
    }

    #[test]
    fn test_virtqueue_requires_power_of_two() {
        // This should panic
        let result = std::panic::catch_unwind(|| {
            Virtqueue::new(0, 15, 0x1000, 0x2000, 0x3000);
        });
        assert!(result.is_err());
    }
}
