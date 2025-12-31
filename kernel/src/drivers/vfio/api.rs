//! VFIO Userspace API
//!
//! Implements the VFIO ioctl interface for userspace drivers.
//!
//! # ioctl Commands
//!
//! - Container ioctls (on /dev/vfio/vfio):
//!   - `VFIO_GET_API_VERSION`: Get API version
//!   - `VFIO_CHECK_EXTENSION`: Check feature support
//!   - `VFIO_SET_IOMMU`: Set IOMMU type
//!
//! - Group ioctls (on /dev/vfio/$GROUP):
//!   - `VFIO_GROUP_GET_STATUS`: Get group status
//!   - `VFIO_GROUP_SET_CONTAINER`: Set group's container
//!   - `VFIO_GROUP_UNSET_CONTAINER`: Unset container
//!   - `VFIO_GROUP_GET_DEVICE_FD`: Get device file descriptor
//!
//! - Device ioctls (on device fd):
//!   - `VFIO_DEVICE_GET_INFO`: Get device information
//!   - `VFIO_DEVICE_GET_REGION_INFO`: Get region info
//!   - `VFIO_DEVICE_GET_IRQ_INFO`: Get IRQ info
//!   - `VFIO_DEVICE_SET_IRQS`: Configure interrupts
//!   - `VFIO_DEVICE_RESET`: Reset device
//!   - `VFIO_IOMMU_MAP_DMA`: Map DMA
//!   - `VFIO_IOMMU_UNMAP_DMA`: Unmap DMA
//!
//! # File Operations
//!
//! - `open()`: Open VFIO device file
//! - `close()`: Close file descriptor
//! - `mmap()`: Map device regions into userspace
//! - `read()/write()`: Read/write device regions
//! - `ioctl()`: Control operations

use crate::drivers::vfio::{
    container::ContainerManager,
    device::{IrqInfo, VfioDeviceInfo, VfioRegionInfo},
    dma::DmaMap,
    group::{GroupManager, GroupStatus},
    iommu::IommuType,
    VfioError, VfioResult,
};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

/// VFIO API version
pub const VFIO_API_VERSION: u32 = 0;

/// VFIO base flags
const VFIO_FLAG_BITS: u32 = 0;
const VFIO_FLAG_BITS_UI: u32 = 1;

/// VFIO extension flags
const VFIO_TYPE1_IOMMU: u32 = 1;
const VFIO_TYPE1v2_IOMMU: u32 = 2;
const VFIO_SPAPR_TCE_IOMMU: u32 = 3;

/// Device flags
const VFIO_DEVICE_FLAGS_PCI: u32 = 1 << 0; // PCI device
const VFIO_DEVICE_FLAGS_PLATFORM: u32 = 1 << 1; // Platform device
const VFIO_DEVICE_FLAGS_AMBA: u32 = 1 << 2; // AMBA device
const VFIO_DEVICE_FLAGS_RESET: u32 = 1 << 3; // Device supports reset

/// Region flags
const VFIO_REGION_INFO_FLAG_READ: u32 = 1 << 0;
const VFIO_REGION_INFO_FLAG_WRITE: u32 = 1 << 1;
const VFIO_REGION_INFO_FLAG_MMAP: u32 = 1 << 2;
const VFIO_REGION_INFO_FLAG_CAPS: u32 = 1 << 3;

/// IRQ flags
const VFIO_IRQ_INFO_EVENTFD: u32 = 1 << 0;
const VFIO_IRQ_INFO_MASKABLE: u32 = 1 << 1;
const VFIO_IRQ_INFO_AUTOMASKED: u32 = 1 << 2;
const VFIO_IRQ_INFO_NORESIZE: u32 = 1 << 3;

/// IRQ set action flags
const VFIO_IRQ_SET_ACTION_NONE: u32 = 0;
const VFIO_IRQ_SET_ACTION_MASK: u32 = 1;
const VFIO_IRQ_SET_ACTION_UNMASK: u32 = 2;
const VFIO_IRQ_SET_ACTION_TRIGGER: u32 = 3;

/// IRQ set data flags
const VFIO_IRQ_SET_DATA_NONE: u32 = 0;
const VFIO_IRQ_SET_DATA_BOOL: u32 = 1;
const VFIO_IRQ_SET_DATA_EVENTFD: u32 = 2;

/// PCI region indexes
const VFIO_PCI_BAR0_REGION_INDEX: u32 = 0;
const VFIO_PCI_BAR1_REGION_INDEX: u32 = 1;
const VFIO_PCI_BAR2_REGION_INDEX: u32 = 2;
const VFIO_PCI_BAR3_REGION_INDEX: u32 = 3;
const VFIO_PCI_BAR4_REGION_INDEX: u32 = 4;
const VFIO_PCI_BAR5_REGION_INDEX: u32 = 5;
const VFIO_PCI_ROM_REGION_INDEX: u32 = 6;
const VFIO_PCI_CONFIG_REGION_INDEX: u32 = 7;
const VFIO_PCI_VGA_REGION_INDEX: u32 = 8;

/// PCI IRQ indexes
const VFIO_PCI_INTX_IRQ_INDEX: u32 = 0;
const VFIO_PCI_MSI_IRQ_INDEX: u32 = 1;
const VFIO_PCI_MSIX_IRQ_INDEX: u32 = 2;
const VFIO_PCI_ERR_IRQ_INDEX: u32 = 3;
const VFIO_PCI_REQ_IRQ_INDEX: u32 = 4;

/// DMA map flags
const VFIO_DMA_MAP_FLAG_READ: u32 = 1;
const VFIO_DMA_MAP_FLAG_WRITE: u32 = 2;

/// VFIO ioctl command handler
pub struct VfioIoctl {
    container_mgr: &'static ContainerManager,
    group_mgr: &'static GroupManager,
}

impl VfioIoctl {
    /// Create ioctl handler
    pub fn new() -> Self {
        Self {
            container_mgr: ContainerManager::global(),
            group_mgr: GroupManager::global(),
        }
    }

    /// Handle ioctl on container fd
    pub fn handle_container_ioctl(&self, cmd: u32, arg: u64) -> VfioResult<i32> {
        match cmd {
            VFIO_GET_API_VERSION => self.get_api_version(),
            VFIO_CHECK_EXTENSION => self.check_extension(arg),
            VFIO_SET_IOMMU => self.set_iommu(arg),
            _ => Err(VfioError::NotSupported),
        }
    }

    /// Handle ioctl on group fd
    pub fn handle_group_ioctl(
        &self,
        group_id: u32,
        cmd: u32,
        arg: u64,
    ) -> VfioResult<i32> {
        match cmd {
            VFIO_GROUP_GET_STATUS => self.group_get_status(group_id, arg),
            VFIO_GROUP_SET_CONTAINER => self.group_set_container(group_id, arg),
            VFIO_GROUP_UNSET_CONTAINER => self.group_unset_container(group_id),
            VFIO_GROUP_GET_DEVICE_FD => self.group_get_device_fd(group_id, arg),
            _ => Err(VfioError::NotSupported),
        }
    }

    /// Handle ioctl on device fd
    pub fn handle_device_ioctl(&self, _device_id: u64, cmd: u32, arg: u64) -> VfioResult<i32> {
        match cmd {
            VFIO_DEVICE_GET_INFO => self.device_get_info(_device_id, arg),
            VFIO_DEVICE_GET_REGION_INFO => self.device_get_region_info(_device_id, arg),
            VFIO_DEVICE_GET_IRQ_INFO => self.device_get_irq_info(_device_id, arg),
            VFIO_DEVICE_SET_IRQS => self.device_set_irqs(_device_id, arg),
            VFIO_DEVICE_RESET => self.device_reset(_device_id),
            _ => Err(VfioError::NotSupported),
        }
    }

    /// Get API version
    fn get_api_version(&self) -> VfioResult<i32> {
        Ok(VFIO_API_VERSION as i32)
    }

    /// Check extension support
    fn check_extension(&self, arg: u64) -> VfioResult<i32> {
        match arg as u32 {
            VFIO_TYPE1_IOMMU => Ok(1),
            VFIO_TYPE1v2_IOMMU => Ok(1),
            VFIO_SPAPR_TCE_IOMMU => Ok(0), // Not supported
            _ => Ok(0),
        }
    }

    /// Set IOMMU type
    fn set_iommu(&self, arg: u64) -> VfioResult<i32> {
        let iommu_type = match IommuType::from_u32(arg as u32) {
            Some(t) => t,
            None => return Err(VfioError::InvalidArgument),
        };

        // TODO: Set IOMMU type for container
        // This needs container context

        Ok(0)
    }

    /// Get group status
    fn group_get_status(&self, group_id: u32, arg: u64) -> VfioResult<i32> {
        let group = self
            .group_mgr
            .get_group(group_id)
            .ok_or(VfioError::GroupNotAvailable)?;

        let status = group.get_status();

        // Write status to userspace
        // TODO: Implement proper copy_to_user
        let flags = match status {
            GroupStatus::Viable => VFIO_GROUP_FLAGS_VIABLE,
            GroupStatus::NotViable => 0,
        };

        // *(arg as *mut u32) = flags;

        Ok(0)
    }

    /// Set group's container
    fn group_set_container(&self, group_id: u32, arg: u64) -> VfioResult<i32> {
        let container_id = arg as u64;

        let group = self
            .group_mgr
            .get_group(group_id)
            .ok_or(VfioError::GroupNotAvailable)?;

        let container = self
            .container_mgr
            .get_container(container_id)
            .ok_or(VfioError::ContainerNotFound)?;

        group.set_container(container_id)?;

        container.add_group(group_id)?;

        Ok(0)
    }

    /// Unset group's container
    fn group_unset_container(&self, group_id: u32) -> VfioResult<i32> {
        let group = self
            .group_mgr
            .get_group(group_id)
            .ok_or(VfioError::GroupNotAvailable)?;

        group.unset_container()?;

        Ok(0)
    }

    /// Get device file descriptor
    fn group_get_device_fd(&self, _group_id: u32, _arg: u64) -> VfioResult<i32> {
        // TODO: Get device name from arg
        // TODO: Create device fd
        Ok(0)
    }

    /// Get device info
    fn device_get_info(&self, _device_id: u64, _arg: u64) -> VfioResult<i32> {
        // TODO: Write device info to userspace
        Ok(0)
    }

    /// Get region info
    fn device_get_region_info(&self, _device_id: u64, _arg: u64) -> VfioResult<i32> {
        // TODO: Write region info to userspace
        Ok(0)
    }

    /// Get IRQ info
    fn device_get_irq_info(&self, _device_id: u64, _arg: u64) -> VfioResult<i32> {
        // TODO: Write IRQ info to userspace
        Ok(0)
    }

    /// Set IRQs
    fn device_set_irqs(&self, _device_id: u64, _arg: u64) -> VfioResult<i32> {
        // TODO: Configure interrupts
        Ok(0)
    }

    /// Reset device
    fn device_reset(&self, _device_id: u64) -> VfioResult<i32> {
        // TODO: Reset device
        Ok(0)
    }
}

/// VFIO file operations
pub struct VfioFileOps {
    ioctl: VfioIoctl,
}

impl VfioFileOps {
    pub fn new() -> Self {
        Self {
            ioctl: VfioIoctl::new(),
        }
    }

    /// Open VFIO file
    pub fn open(&self, path: &str) -> VfioResult<i32> {
        // TODO: Implement file opening
        Ok(0)
    }

    /// Close VFIO file
    pub fn close(&self, fd: i32) -> VfioResult<()> {
        // TODO: Implement file closing
        Ok(())
    }

    /// mmap device region
    pub fn mmap(
        &self,
        fd: i32,
        offset: u64,
        size: usize,
        prot: u32,
        flags: u32,
    ) -> VfioResult<u64> {
        // TODO: Implement mmap
        Ok(0)
    }

    /// Read from device
    pub fn read(&self, fd: i32, buf: &mut [u8], offset: u64) -> VfioResult<usize> {
        // TODO: Implement read
        Ok(0)
    }

    /// Write to device
    pub fn write(&self, fd: i32, buf: &[u8], offset: u64) -> VfioResult<usize> {
        // TODO: Implement write
        Ok(0)
    }
}

/// VFIO API structure
#[derive(Clone)]
pub struct VfioApi {
    dma_map: DmaMap,
}

impl VfioApi {
    pub fn new() -> Self {
        Self {
            dma_map: DmaMap::new(),
        }
    }

    /// Map DMA for userspace
    pub fn map_dma(&self, iova: u64, user_addr: u64, size: u64, flags: u32) -> VfioResult<()> {
        self.dma_map.map_user_pages_at(iova, user_addr, size, flags)
    }

    /// Unmap DMA
    pub fn unmap_dma(&self, iova: u64, size: u64) -> VfioResult<()> {
        self.dma_map.unmap_user_pages(iova, size)
    }
}

// ioctl command numbers
const VFIO_GET_API_VERSION: u32 = 0x3B64;
const VFIO_CHECK_EXTENSION: u32 = 0x3B65;
const VFIO_SET_IOMMU: u32 = 0x3B66;

const VFIO_GROUP_GET_STATUS: u32 = 0x3B67;
const VFIO_GROUP_SET_CONTAINER: u32 = 0x3B68;
const VFIO_GROUP_UNSET_CONTAINER: u32 = 0x3B69;
const VFIO_GROUP_GET_DEVICE_FD: u32 = 0x3B6A;

const VFIO_DEVICE_GET_INFO: u32 = 0x3B6B;
const VFIO_DEVICE_GET_REGION_INFO: u32 = 0x3B6C;
const VFIO_DEVICE_GET_IRQ_INFO: u32 = 0x3B6D;
const VFIO_DEVICE_SET_IRQS: u32 = 0x3B6E;
const VFIO_DEVICE_RESET: u32 = 0x3B6F;

const VFIO_IOMMU_MAP_DMA: u32 = 0x3B71;
const VFIO_IOMMU_UNMAP_DMA: u32 = 0x3B72;

const VFIO_GROUP_FLAGS_VIABLE: u32 = 1 << 0;

/// VFIO device info (userspace ABI)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfioDeviceInfoRaw {
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
}

/// VFIO region info (userspace ABI)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfioRegionInfoRaw {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}

/// VFIO IRQ info (userspace ABI)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfioIrqInfoRaw {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub count: u32,
}

/// VFIO IRQ set (userspace ABI)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfioIrqSetRaw {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub start: u32,
    pub count: u32,
}

/// VFIO DMA map (userspace ABI)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfioDmaMapRaw {
    pub argsz: u32,
    pub flags: u32,
    pub iova: u64,
    pub user_addr: u64,
    pub size: u64,
}

/// VFIO DMA unmap (userspace ABI)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VfioDmaUnmapRaw {
    pub argsz: u32,
    pub flags: u32,
    pub iova: u64,
    pub size: u64,
}

impl Default for VfioIoctl {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VfioFileOps {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VfioApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ioctl_create() {
        let ioctl = VfioIoctl::new();
        let version = ioctl.get_api_version().unwrap();
        assert_eq!(version, VFIO_API_VERSION as i32);
    }

    #[test]
    fn test_check_extension() {
        let ioctl = VfioIoctl::new();

        assert_eq!(ioctl.check_extension(VFIO_TYPE1_IOMMU as u64).unwrap(), 1);
        assert_eq!(ioctl.check_extension(VFIO_TYPE1v2_IOMMU as u64).unwrap(), 1);
        assert_eq!(ioctl.check_extension(VFIO_SPAPR_TCE_IOMMU as u64).unwrap(), 0);
    }

    #[test]
    fn test_iommu_type_conversion() {
        assert_eq!(IommuType::Type1.as_u32(), 1);
        assert_eq!(IommuType::Type1v2.as_u32(), 2);
        assert_eq!(IommuType::from_u32(1), Some(IommuType::Type1));
        assert_eq!(IommuType::from_u32(999), None);
    }

    #[test]
    fn test_api_map_unmap_dma() {
        let api = VfioApi::new();

        let user_addr = 0x7f0000000000;
        let iova = 0x1000;
        let size = 0x1000;
        let flags = VFIO_DMA_MAP_FLAG_READ | VFIO_DMA_MAP_FLAG_WRITE;

        api.map_dma(iova, user_addr, size, flags).unwrap();
        api.unmap_dma(iova, size).unwrap();
    }
}
