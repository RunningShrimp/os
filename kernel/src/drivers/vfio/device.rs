//! VFIO Device Management
//!
//! Manages VFIO device lifecycle including:
//!
//! - Device discovery and enumeration
//! - Device attachment to containers
//! - Device region mapping (MMIO, ROM, config)
//! - Device state management
//! - Live migration support
//!
//! # Device Lifecycle
//!
//! ```text
//! Probe → Add to Container → Initialize → Map Regions → Use → Unmap → Remove
//! ```

use crate::drivers::vfio::{
    container::VfioContainer,
    iommu::IommuDomain,
    pci::{PciDevice, PciError},
    VfioError, VfioResult,
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use spin::{Mutex, RwLock};

/// Device state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device created but not initialized
    Created,

    /// Device initialized and ready
    Initialized,

    /// Device running (active)
    Running,

    /// Device stopped
    Stopped,

    /// Device in error state
    Error,

    /// Device being removed
    Removing,
}

/// Device region information
#[derive(Debug, Clone)]
pub struct VfioRegionInfo {
    /// Region index
    pub index: u32,

    /// Region type
    pub region_type: RegionType,

    /// Region size (bytes)
    pub size: u64,

    /// Region offset in device BAR
    pub offset: u64,

    /// Region flags (capabilities)
    pub flags: u32,

    /// Memory mapping address (if mapped)
    pub mmap_addr: Option<u64>,

    /// Mmap offset (for mmap() syscall)
    pub mmap_offset: u64,
}

/// Region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// MMIO region (BAR)
    Bar,

    /// ROM region
    Rom,

    /// PCI config space
    Config,

    /// VGA region (for GPUs)
    Vga,

    /// Vendor-specific region
    Vendor,
}

/// Device information
#[derive(Debug, Clone)]
pub struct VfioDeviceInfo {
    /// Device name (e.g., "0000:01:00.0")
    pub name: alloc::string::String,

    /// Device ID
    pub device_id: u16,

    /// Vendor ID
    pub vendor_id: u16,

    /// PCI domain/bus/device/function
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,

    /// Number of regions
    pub num_regions: u32,

    /// Number of IRQs
    pub num_irqs: u32,

    /// Device flags
    pub flags: u32,

    /// SR-IOV VF info (if applicable)
    pub is_vf: bool,

    /// PF device ID (if VF)
    pub pf_device: Option<(u16, u8, u8, u8)>,
}

/// VFIO Device - Wrapper for a physical device
pub struct VfioDevice {
    info: VfioDeviceInfo,
    state: AtomicU8,
    container: Mutex<Option<Arc<VfioContainer>>>,
    regions: Mutex<Vec<VfioRegionInfo>>,
    irqs: Mutex<Vec<IrqInfo>>,
    ref_count: AtomicU32,
    pci_device: Mutex<Option<Arc<PciDevice>>>,
    fd: Mutex<Option<i32>>,
}

impl VfioDevice {
    /// Create a new VFIO device
    pub fn new(info: VfioDeviceInfo) -> VfioResult<Self> {
        Ok(Self {
            info,
            state: AtomicU8::new(DeviceState::Created as u8),
            container: Mutex::new(None),
            regions: Mutex::new(Vec::new()),
            irqs: Mutex::new(Vec::new()),
            ref_count: AtomicU32::new(1),
            pci_device: Mutex::new(None),
            fd: Mutex::new(None),
        })
    }

    /// Get device information
    pub fn info(&self) -> &VfioDeviceInfo {
        &self.info
    }

    /// Get device state
    pub fn state(&self) -> DeviceState {
        match self.state.load(Ordering::Relaxed) {
            0 => DeviceState::Created,
            1 => DeviceState::Initialized,
            2 => DeviceState::Running,
            3 => DeviceState::Stopped,
            4 => DeviceState::Error,
            5 => DeviceState::Removing,
            _ => DeviceState::Error,
        }
    }

    /// Set device state
    fn set_state(&self, state: DeviceState) {
        self.state.store(state as u8, Ordering::Relaxed);
    }

    /// Attach to container
    pub fn attach_to_container(&self, container: Arc<VfioContainer>) -> VfioResult<()> {
        let mut container_slot = self.container.lock();

        if container_slot.is_some() {
            return Err(VfioError::DeviceAttached);
        }

        // Attach to IOMMU domain
        let iommu = container.iommu()?;
        iommu.attach_device(
            self.info.segment,
            self.info.bus,
            self.info.device,
            self.info.function,
        )
        .map_err(|_| VfioError::IommuError)?;

        *container_slot = Some(container);

        Ok(())
    }

    /// Detach from container
    pub fn detach_from_container(&self) -> VfioResult<()> {
        let mut container_slot = self.container.lock();

        let container = container_slot
            .take()
            .ok_or(VfioError::InvalidArgument)?;

        // Detach from IOMMU domain
        if let Ok(iommu) = container.iommu() {
            let _ = iommu.detach_device(
                self.info.segment,
                self.info.bus,
                self.info.device,
                self.info.function,
            );
        }

        Ok(())
    }

    /// Get container
    pub fn container(&self) -> Option<Arc<VfioContainer>> {
        self.container.lock().clone()
    }

    /// Initialize device
    pub fn initialize(&self) -> VfioResult<()> {
        if self.state() != DeviceState::Created {
            return Err(VfioError::InvalidArgument);
        }

        // Enable PCI device
        if let Some(pci) = self.pci_device.lock().as_ref() {
            pci.enable()?;
        }

        // Discover regions
        self.discover_regions()?;

        // Discover IRQs
        self.discover_irqs()?;

        self.set_state(DeviceState::Initialized);

        Ok(())
    }

    /// Start device
    pub fn start(&self) -> VfioResult<()> {
        if self.state() != DeviceState::Initialized {
            return Err(VfioError::InvalidArgument);
        }

        self.set_state(DeviceState::Running);

        Ok(())
    }

    /// Stop device
    pub fn stop(&self) -> VfioResult<()> {
        if self.state() != DeviceState::Running {
            return Err(VfioError::InvalidArgument);
        }

        self.set_state(DeviceState::Stopped);

        Ok(())
    }

    /// Reset device
    pub fn reset(&self) -> VfioResult<()> {
        // GH-#1000: Send device reset command
        // See: https://github.com/npos/kernel/issues/1000
        Ok(())
    }

    /// Discover device regions
    fn discover_regions(&self) -> VfioResult<()> {
        let mut regions = self.regions.lock();

        // Standard PCI BARs
        for i in 0..6 {
            // GH-#1001: Read BAR info from PCI config space
            // See: https://github.com/npos/kernel/issues/1001
            let region = VfioRegionInfo {
                index: i,
                region_type: RegionType::Bar,
                size: 0, // GH-#1002: Get actual size
                // See: https://github.com/npos/kernel/issues/1002
                offset: 0,
                flags: 0,
                mmap_addr: None,
                mmap_offset: 0,
            };
            regions.push(region);
        }

        // ROM region
        regions.push(VfioRegionInfo {
            index: 6,
            region_type: RegionType::Rom,
            size: 0,
            offset: 0,
            flags: 0,
            mmap_addr: None,
            mmap_offset: 0,
        });

        // Config space
        regions.push(VfioRegionInfo {
            index: 7,
            region_type: RegionType::Config,
            size: 4096,
            offset: 0,
            flags: 0,
            mmap_addr: None,
            mmap_offset: 0,
        });

        Ok(())
    }

    /// Discover IRQs
    fn discover_irqs(&self) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        // GH-#1003: Query device for IRQ info
        // See: https://github.com/npos/kernel/issues/1003
        // For now, add standard IRQ types
        irqs.push(IrqInfo {
            index: 0,
            irq_type: IrqType::Intx,
            count: 1,
            flags: 0,
        });

        irqs.push(IrqInfo {
            index: 1,
            irq_type: IrqType::Msi,
            count: 32,
            flags: 0,
        });

        irqs.push(IrqInfo {
            index: 2,
            irq_type: IrqType::Msix,
            count: 2048,
            flags: 0,
        });

        Ok(())
    }

    /// Get region info
    pub fn get_region_info(&self, index: u32) -> Option<VfioRegionInfo> {
        self.regions.lock().get(index as usize).cloned()
    }

    /// Get all regions
    pub fn get_regions(&self) -> Vec<VfioRegionInfo> {
        self.regions.lock().clone()
    }

    /// Map region into userspace
    pub fn map_region(&self, index: u32, addr: u64) -> VfioResult<()> {
        let mut regions = self.regions.lock();

        let region = regions
            .get_mut(index as usize)
            .ok_or(VfioError::InvalidArgument)?;

        // GH-#1004: Actually mmap the region
        // See: https://github.com/npos/kernel/issues/1004
        region.mmap_addr = Some(addr);

        Ok(())
    }

    /// Unmap region
    pub fn unmap_region(&self, index: u32) -> VfioResult<()> {
        let mut regions = self.regions.lock();

        let region = regions
            .get_mut(index as usize)
            .ok_or(VfioError::InvalidArgument)?;

        // GH-#1005: Actually munmap the region
        // See: https://github.com/npos/kernel/issues/1005
        region.mmap_addr = None;

        Ok(())
    }

    /// Read from device region
    pub fn region_read(&self, index: u32, offset: u64, data: &mut [u8]) -> VfioResult<()> {
        let regions = self.regions.lock();

        let region = regions
            .get(index as usize)
            .ok_or(VfioError::InvalidArgument)?;

        if offset + data.len() as u64 > region.size {
            return Err(VfioError::InvalidArgument);
        }

        // GH-#1006: Actually read from device
        // See: https://github.com/npos/kernel/issues/1006
        // For now, return zeros
        data.fill(0);

        Ok(())
    }

    /// Write to device region
    pub fn region_write(&self, index: u32, offset: u64, data: &[u8]) -> VfioResult<()> {
        let regions = self.regions.lock();

        let region = regions
            .get(index as usize)
            .ok_or(VfioError::InvalidArgument)?;

        if offset + data.len() as u64 > region.size {
            return Err(VfioError::InvalidArgument);
        }

        // GH-#1007: Actually write to device
        // See: https://github.com/npos/kernel/issues/1007

        Ok(())
    }

    /// Get IRQ info
    pub fn get_irq_info(&self, index: u32) -> Option<IrqInfo> {
        self.irqs.lock().get(index as usize).cloned()
    }

    /// Get all IRQs
    pub fn get_irqs(&self) -> Vec<IrqInfo> {
        self.irqs.lock().clone()
    }

    /// Increment reference count
    pub fn ref_count_inc(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement reference count
    pub fn ref_count_dec(&self) -> u32 {
        self.ref_count.fetch_sub(1, Ordering::Relaxed).saturating_sub(1)
    }

    /// Get reference count
    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Relaxed)
    }

    /// Set file descriptor
    pub fn set_fd(&self, fd: i32) {
        *self.fd.lock() = Some(fd);
    }

    /// Get file descriptor
    pub fn fd(&self) -> Option<i32> {
        *self.fd.lock()
    }

    /// Set PCI device
    pub fn set_pci_device(&self, pci: Arc<PciDevice>) {
        *self.pci_device.lock() = Some(pci);
    }

    /// Prepare for removal
    pub fn prepare_remove(&self) -> VfioResult<()> {
        self.set_state(DeviceState::Removing);

        // Detach from container
        let _ = self.detach_from_container();

        // Unmap all regions
        let regions = self.regions.lock();
        for i in 0..regions.len() {
            let _ = self.unmap_region(i as u32);
        }

        Ok(())
    }
}

impl Drop for VfioDevice {
    fn drop(&mut self) {
        // Clean up resources
        let _ = self.prepare_remove();
    }
}

/// IRQ information
#[derive(Debug, Clone)]
pub struct IrqInfo {
    pub index: u32,
    pub irq_type: IrqType,
    pub count: u32,
    pub flags: u32,
}

/// IRQ type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    /// Legacy INTx
    Intx,

    /// Message Signaled Interrupts
    Msi,

    /// Extended MSI
    Msix,

    /// Generic eventfd
    Eventfd,
}

/// Device error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceError {
    NotFound,
    AlreadyExists,
    InvalidState,
    InitializationFailed,
    RegionError,
    IrqError,
    PciError(PciError),
}

impl core::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Device not found"),
            Self::AlreadyExists => write!(f, "Device already exists"),
            Self::InvalidState => write!(f, "Device in invalid state"),
            Self::InitializationFailed => write!(f, "Device initialization failed"),
            Self::RegionError => write!(f, "Device region error"),
            Self::IrqError => write!(f, "Device IRQ error"),
            Self::PciError(e) => write!(f, "PCI error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_create() {
        let info = VfioDeviceInfo {
            name: alloc::string::String::from("test"),
            device_id: 0x1234,
            vendor_id: 0x5678,
            segment: 0,
            bus: 0,
            device: 1,
            function: 0,
            num_regions: 8,
            num_irqs: 3,
            flags: 0,
            is_vf: false,
            pf_device: None,
        };

        let device = VfioDevice::new(info).unwrap();
        assert_eq!(device.state(), DeviceState::Created);
        assert_eq!(device.ref_count(), 1);
    }

    #[test]
    fn test_device_state_transitions() {
        let info = VfioDeviceInfo {
            name: alloc::string::String::from("test"),
            device_id: 0x1234,
            vendor_id: 0x5678,
            segment: 0,
            bus: 0,
            device: 1,
            function: 0,
            num_regions: 8,
            num_irqs: 3,
            flags: 0,
            is_vf: false,
            pf_device: None,
        };

        let device = VfioDevice::new(info).unwrap();

        // Can't start without initializing
        assert!(device.start().is_err());

        device.initialize().unwrap();
        assert_eq!(device.state(), DeviceState::Initialized);

        device.start().unwrap();
        assert_eq!(device.state(), DeviceState::Running);

        device.stop().unwrap();
        assert_eq!(device.state(), DeviceState::Stopped);
    }

    #[test]
    fn test_device_ref_count() {
        let info = VfioDeviceInfo {
            name: alloc::string::String::from("test"),
            device_id: 0x1234,
            vendor_id: 0x5678,
            segment: 0,
            bus: 0,
            device: 1,
            function: 0,
            num_regions: 8,
            num_irqs: 3,
            flags: 0,
            is_vf: false,
            pf_device: None,
        };

        let device = VfioDevice::new(info).unwrap();
        assert_eq!(device.ref_count(), 1);

        device.ref_count_inc();
        assert_eq!(device.ref_count(), 2);

        assert_eq!(device.ref_count_dec(), 1);
        assert_eq!(device.ref_count(), 1);
    }
}
