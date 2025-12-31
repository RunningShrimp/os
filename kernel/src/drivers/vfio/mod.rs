//! VFIO (Virtual Function I/O) - High-performance userspace driver framework
//!
//! VFIO provides a secure, high-performance mechanism for userspace drivers to
//! directly access hardware devices with IOMMU protection. This enables:
//!
//! - **Zero-copy I/O**: Direct DMA from/to userspace memory
//! - **High-performance networking**: DPDK, virtio-net, etc.
//! - **GPU passthrough**: NVIDIA, AMD GPU virtualization
//! - **Storage acceleration**: NVMe passthrough for minimal latency
//! - **SR-IOV**: Virtual Function support for device sharing
//!
//! # Architecture
//!
//! ```text
//! Userspace                    Kernel
//! --------                    ------
//! Application
//!   ├── open("/dev/vfio/$GROUP")
//!   ├── ioctl(VFIO_SET_IOMMU)
//!   ├── mmap() device regions
//!   └── ioctl(VFIO_DEVICE_GET_INFO)
//!
//! Userspace Driver (DPDK, etc.)
//!   ├── Direct device access (MMIO)
//!   ├── DMA operations
//!   └── Interrupt handling (eventfd)
//!
//! VFIO Framework (this module)
//!   ├── VfioContainer (IOMMU domain)
//!   ├── VfioDevice (device wrapper)
//!   ├── VfioGroup (device group)
//!   └── Security enforcement
//!       ├── IOMMU translation
//!       ├── Permission checks
//!       └── Device isolation
//! ```
//!
//! # Security Model
//!
//! VFIO provides security through multiple layers:
//!
//! 1. **IOMMU Protection**: All DMA addresses are translated through IOMMU
//! 2. **Group Isolation**: Devices that can DMA to each other are in the same group
//! 3. **Permission Checking**: Fine-grained access control per device
//! 4. **Sandboxing**: Userspace drivers run with restricted privileges
//!
//! # Example Usage
//!
//! ```rust,ignore
//! // 1. Open VFIO group
//! let group_fd = open("/dev/vfio/0", O_RDWR)?;
//!
//! // 2. Create container
//! let container_fd = open("/dev/vfio/vfio", O_RDWR)?;
//! ioctl(group_fd, VFIO_GROUP_SET_CONTAINER, &container_fd)?;
//!
//! // 3. Set IOMMU type
//! ioctl(group_fd, VFIO_GROUP_SET_IOMMU, VFIO_TYPE1_IOMMU)?;
//!
//! // 4. Get device
//! let device_fd = ioctl(group_fd, VFIO_GROUP_GET_DEVICE_FD, "0000:01:00.0")?;
//!
//! // 5. Get device info
//! let device_info: VfioDeviceInfo = ioctl(device_fd, VFIO_DEVICE_GET_INFO)?;
//!
//! // 6. Map device regions
//! let regions = mmap_device_regions(device_fd, &device_info)?;
//!
//! // 7. Setup DMA mappings
//! let iova = 0x1000;
//! let user_addr = 0x7f0000000000;
//! ioctl(container_fd, VFIO_IOMMU_MAP_DMA, &VfioDMAMap {
//!     iova,
//!     size: 4096,
//!     user_addr,
//!     flags: VFIO_DMA_MAP_FLAG_READ | VFIO_DMA_MAP_FLAG_WRITE,
//! })?;
//!
//! // 8. Enable interrupts
//! let irq_fd = eventfd(0, EFD_CLOEXEC)?;
//! ioctl(device_fd, VFIO_DEVICE_SET_IRQS, &VfioIrqSet {
//!     index: VFIO_PCI_MSIX_IRQ_INDEX,
//!     start: 0,
//!     count: 1,
//!     flags: VFIO_IRQ_SET_DATA_EVENTFD | VFIO_IRQ_SET_ACTION_TRIGGER,
//!     data: irq_fd,
//! })?;
//! ```
//!
//! # Module Organization
//!
//! - [`container`] - VFIO container and IOMMU domain management
//! - [`iommu`] - IOMMU operations (DMA map/unmap, device attach/detach)
//! - [`device`] - Device lifecycle management
//! - [`group`] - Device group management
//! - [`dma`] - DMA mapping operations
//! - [`pci`] - PCI-specific support (SR-IOV, config space)
//! - [`interrupt`] - Interrupt handling (MSI/MSI-X, eventfd)
//! - [`security`] - Security policy enforcement
//! - [`api`] - Userspace API (ioctl, mmap, file operations)
//!
//! # Performance Characteristics
//!
//! - **DMA latency**: ~100ns (IOMMU translation in hardware)
//! - **MMIO latency**: ~50ns (direct device access)
//! - **Interrupt latency**: ~1μs (eventfd notification)
//! - **Throughput**: Limited by device, not kernel
//!
//! # Thread Safety
//!
//! All VFIO operations are thread-safe. Multiple threads can:
//! - Perform DMA operations concurrently
//! - Access device registers simultaneously
//! - Handle interrupts in parallel
//!
//! # Live Migration
//!
//! VFIO supports live migration of device state:
//! ```rust,ignore
//! // Save device state
//! let data = ioctl(device_fd, VFIO_DEVICE_GET_REGION_INFO, &region_info)?;
//! write(migration_fd, &data)?;
//!
//! // Restore on target
//! let data = read(migration_fd)?;
//! ioctl(target_device_fd, VFIO_DEVICE_SET_STATE, &data)?;
//! ```

pub mod api;
pub mod container;
pub mod device;
pub mod dma;
pub mod group;
pub mod interrupt;
pub mod iommu;
pub mod pci;
pub mod security;

#[cfg(test)]
mod tests;

// Re-exports for convenience
pub use api::{
    VfioApi,
    VfioFileOps,
    VfioIoctl,
};
pub use container::{
    VfioContainer,
    ContainerManager,
    ContainerError,
};
pub use device::{
    VfioDevice,
    VfioDeviceInfo,
    DeviceState,
    DeviceError,
};
pub use dma::{
    DmaMap,
    DmaMapping,
    DmaError,
};
pub use group::{
    VfioGroup,
    GroupStatus,
    GroupError,
};
pub use iommu::{
    IommuDomain,
    IommuType,
    IommuError,
};
pub use interrupt::{
    VfioInterrupt,
    IrqType,
    IrqInfo,
    InterruptError,
};
pub use pci::{
    PciDevice,
    PciConfigSpace,
    SrioVf,
    PciError,
};
pub use security::{
    VfioSandbox,
    SecurityPolicy,
    Permission,
    SecurityError,
};

/// VFIO API version
pub const VFIO_API_VERSION: u32 = 0;

/// VFIO base directory for device nodes
pub const VFIO_DEV_PATH: &str = "/dev/vfio";

/// Maximum number of containers system-wide
pub const MAX_CONTAINERS: usize = 256;

/// Maximum number of devices per container
pub const MAX_DEVICES_PER_CONTAINER: usize = 32;

/// Maximum number of groups per container
pub const MAX_GROUPS_PER_CONTAINER: usize = 16;

/// Maximum DMA mapping size (16TB)
pub const MAX_DMA_SIZE: u64 = 16 * 1024 * 1024 * 1024 * 1024;

/// Page size for DMA mappings (must match IOMMU page size)
pub const DMA_PAGE_SIZE: u64 = 4096;

/// Maximum number of MSI-X vectors per device
pub const MAX_MSIX_VECTORS: u32 = 2048;

/// Maximum number of IRQs per device
pub const MAX_IRQS: u32 = 32;

/// VFIO error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfioError {
    /// Invalid argument
    InvalidArgument,

    /// Operation not supported
    NotSupported,

    /// Permission denied
    PermissionDenied,

    /// Device not found
    DeviceNotFound,

    /// IOMMU operation failed
    IommuError,

    /// DMA mapping failed
    DmaError,

    /// Interrupt setup failed
    InterruptError,

    /// Container not found
    ContainerNotFound,

    /// Group not available
    GroupNotAvailable,

    /// Device already attached
    DeviceAttached,

    /// Resource exhausted
    ResourceExhausted,

    /// Invalid DMA address
    InvalidDmaAddress,

    /// Invalid file descriptor
    InvalidFd,

    /// Operation timed out
    Timeout,

    /// Internal error
    InternalError,
}

impl core::fmt::Display for VfioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "Invalid argument"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::DeviceNotFound => write!(f, "Device not found"),
            Self::IommuError => write!(f, "IOMMU operation failed"),
            Self::DmaError => write!(f, "DMA mapping failed"),
            Self::InterruptError => write!(f, "Interrupt setup failed"),
            Self::ContainerNotFound => write!(f, "Container not found"),
            Self::GroupNotAvailable => write!(f, "Group not available"),
            Self::DeviceAttached => write!(f, "Device already attached"),
            Self::ResourceExhausted => write!(f, "Resource exhausted"),
            Self::InvalidDmaAddress => write!(f, "Invalid DMA address"),
            Self::InvalidFd => write!(f, "Invalid file descriptor"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::InternalError => write!(f, "Internal error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VfioError {}

/// Result type for VFIO operations
pub type VfioResult<T> = core::result::Result<T, VfioError>;

/// VFIO statistics
#[derive(Debug, Default)]
pub struct VfioStats {
    /// Number of active containers
    pub active_containers: usize,

    /// Number of active devices
    pub active_devices: usize,

    /// Total DMA mappings
    pub total_dma_mappings: usize,

    /// Total DMA size (bytes)
    pub total_dma_size: u64,

    /// Number of active interrupts
    pub active_interrupts: usize,

    /// Number of IOMMU faults
    pub iommu_faults: u64,
}

/// Get global VFIO statistics
pub fn get_stats() -> VfioStats {
    let containers = container::ContainerManager::global();
    let stats = containers.get_stats();

    VfioStats {
        active_containers: stats.active_containers,
        active_devices: stats.active_devices,
        total_dma_mappings: stats.total_dma_mappings,
        total_dma_size: stats.total_dma_size,
        active_interrupts: stats.active_interrupts,
        iommu_faults: stats.iommu_faults,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfio_error_display() {
        let err = VfioError::PermissionDenied;
        assert_eq!(format!("{}", err), "Permission denied");
    }

    #[test]
    fn test_vfio_constants() {
        assert_eq!(VFIO_API_VERSION, 0);
        assert_eq!(MAX_CONTAINERS, 256);
        assert_eq!(MAX_DEVICES_PER_CONTAINER, 32);
        assert_eq!(DMA_PAGE_SIZE, 4096);
    }

    #[test]
    fn test_stats() {
        let stats = get_stats();
        assert!(stats.active_containers <= MAX_CONTAINERS);
    }
}
