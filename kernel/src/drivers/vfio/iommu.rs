//! IOMMU Integration for VFIO
//!
//! Provides IOMMU domain abstraction for DMA address translation and device isolation.
//!
//! # IOMMU Types
//!
//! - **Type1**: Standard IOMMU (Intel VT-d, AMD-Vi)
//! - **Type1v2**: Extended Type1 with more features
//! - **SPAPR**: POWER PC IOMMU
//!
//! # DMA Translation Flow
//!
//! ```text
//! Userspace                   IOMMU                   Device
//! ---------                   -----                   ------
//! user_addr = 0x7f0001000
//!     |
//!     | VFIO_IOMMU_MAP_DMA
//!     v
//! iova = 0x1000
//!     |
//!     | Map to physical pages
//!     v
//! phys_addr = 0x1000000
//!     |
//!     | IOMMU page table update
//!     v
//! [IOVA -> PA mapping in hardware]
//!     |
//!     v
//! Device DMA reads/writes translated through IOMMU
//! ```

use crate::drivers::vfio::{VfioError, VfioResult};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, RwLock};

/// IOMMU type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuType {
    /// Type1 IOMMU (Intel VT-d, AMD-Vi)
    Type1,

    /// Type1 v2 IOMMU (extended features)
    Type1v2,

    /// SPAPR TCE IOMMU (POWER)
    SPAPR,
}

impl IommuType {
    /// Convert to integer (for ioctl)
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Type1 => 1,
            Self::Type1v2 => 2,
            Self::SPAPR => 3,
        }
    }

    /// Convert from integer
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            1 => Some(Self::Type1),
            2 => Some(Self::Type1v2),
            3 => Some(Self::SPAPR),
            _ => None,
        }
    }
}

/// IOMMU domain - Isolated address space for DMA
///
/// Each IOMMU domain maintains its own page table for IOVA -> physical address translation.
pub struct IommuDomain {
    id: u64,
    iommu_type: IommuType,
    page_size: u64,
    mappings: RwLock<BTreeMap<u64, DmaMapping>>,
    attached_devices: Mutex<Vec<DeviceInfo>>,
    stats: DomainStats,
}

/// DMA mapping entry
#[derive(Debug, Clone)]
struct DmaMapping {
    iova: u64,
    user_addr: u64,
    size: u64,
    flags: u32,
}

/// Device attachment info
#[derive(Debug, Clone)]
struct DeviceInfo {
    segment: u16,
    bus: u8,
    device: u8,
    function: u8,
}

impl IommuDomain {
    /// Create a new IOMMU domain
    pub fn new(iommu_type: IommuType) -> Result<Self, IommuError> {
        use core::sync::atomic::{AtomicU64, Ordering};

        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Ok(Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            iommu_type,
            page_size: 4096, // Default 4KB pages
            mappings: RwLock::new(BTreeMap::new()),
            attached_devices: Mutex::new(Vec::new()),
            stats: DomainStats::new(),
        })
    }

    /// Get domain ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get IOMMU type
    pub fn iommu_type(&self) -> IommuType {
        self.iommu_type
    }

    /// Get page size
    pub fn page_size(&self) -> u64 {
        self.page_size
    }

    /// Map userspace memory to IOVA
    ///
    /// # Arguments
    ///
    /// - `iova`: IO virtual address (must be page-aligned)
    /// - `user_addr`: Userspace virtual address
    /// - `size`: Size to map (must be page-aligned)
    /// - `flags`: Mapping flags (read/write)
    pub fn map(&self, iova: u64, user_addr: u64, size: u64, flags: u32) -> Result<(), IommuError> {
        // Validate alignment
        if iova % self.page_size != 0 || user_addr % self.page_size != 0 || size % self.page_size != 0
        {
            return Err(IommuError::InvalidArgument);
        }

        // Check for overlap
        let mappings = self.mappings.read();
        for mapping in mappings.values() {
            let iova_end = mapping.iova + mapping.size;
            let new_end = iova + size;

            if !(iova >= iova_end || new_end <= mapping.iova) {
                return Err(IommuError::Overlap);
            }
        }
        drop(mappings);

        // Get physical pages for user address
        let phys_addrs = self.get_phys_pages(user_addr, size)?;

        // Create mapping entries (one per page)
        let mut mappings = self.mappings.write();

        let num_pages = (size / self.page_size) as usize;
        for i in 0..num_pages {
            let page_iova = iova + (i as u64 * self.page_size);
            let page_user = user_addr + (i as u64 * self.page_size);
            let page_phys = if i < phys_addrs.len() {
                phys_addrs[i]
            } else {
                return Err(IommuError::InvalidArgument);
            };

            mappings.insert(
                page_iova,
                DmaMapping {
                    iova: page_iova,
                    user_addr: page_user,
                    size: self.page_size,
                    flags,
                },
            );

            // TODO: Program hardware IOMMU
            // iommu_hw_map_page(self.id, page_iova, page_phys, flags);
        }

        self.stats.mapped_pages.fetch_add(num_pages as u64, Ordering::Relaxed);
        self.stats.mapped_bytes.fetch_add(size, Ordering::Relaxed);

        Ok(())
    }

    /// Unmap IOVA range
    pub fn unmap(&self, iova: u64, size: u64) -> Result<(), IommuError> {
        if size % self.page_size != 0 {
            return Err(IommuError::InvalidArgument);
        }

        let mut mappings = self.mappings.write();
        let mut unmapped_pages = 0u64;

        let num_pages = (size / self.page_size) as usize;
        for i in 0..num_pages {
            let page_iova = iova + (i as u64 * self.page_size);

            if mappings.remove(&page_iova).is_some() {
                unmapped_pages += 1;

                // TODO: Unprogram hardware IOMMU
                // iommu_hw_unmap_page(self.id, page_iova);
            }
        }

        self.stats.unmapped_pages.fetch_add(unmapped_pages, Ordering::Relaxed);
        self.stats.unmapped_bytes.fetch_add(size, Ordering::Relaxed);

        Ok(())
    }

    /// Attach a device to this domain
    ///
    /// This sets the device's IOMMU to use this domain's page table.
    pub fn attach_device(&self, segment: u16, bus: u8, device: u8, function: u8) -> Result<(), IommuError> {
        // Check if device already attached
        let mut devices = self.attached_devices.lock();
        for dev in devices.iter() {
            if dev.segment == segment && dev.bus == bus && dev.device == device && dev.function == function
            {
                return Err(IommuError::DeviceAttached);
            }
        }

        // TODO: Attach device to IOMMU domain in hardware
        // iommu_hw_attach_device(self.id, segment, bus, device, function)?;

        devices.push(DeviceInfo {
            segment,
            bus,
            device,
            function,
        });

        self.stats.devices_attached.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Detach a device from this domain
    pub fn detach_device(&self, segment: u16, bus: u8, device: u8, function: u8) -> Result<(), IommuError> {
        let mut devices = self.attached_devices.lock();
        let pos = devices
            .iter()
            .position(|dev| {
                dev.segment == segment && dev.bus == bus && dev.device == device && dev.function == function
            });

        if pos.is_none() {
            return Err(IommuError::DeviceNotFound);
        }

        // TODO: Detach device from IOMMU domain in hardware
        // iommu_hw_detach_device(self.id, segment, bus, device, function)?;

        devices.remove(pos.unwrap());

        self.stats.devices_detached.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get all attached devices
    pub fn get_attached_devices(&self) -> Vec<(u16, u8, u8, u8)> {
        self.attached_devices
            .lock()
            .iter()
            .map(|dev| (dev.segment, dev.bus, dev.device, dev.function))
            .collect()
    }

    /// Check if device is attached
    pub fn is_device_attached(&self, segment: u16, bus: u8, device: u8, function: u8) -> bool {
        self.attached_devices
            .lock()
            .iter()
            .any(|dev| {
                dev.segment == segment && dev.bus == bus && dev.device == device && dev.function == function
            })
    }

    /// Get mapping info
    pub fn get_mapping(&self, iova: u64) -> Option<DmaMapping> {
        self.mappings.read().get(&iova).cloned()
    }

    /// Get all mappings
    pub fn get_mappings(&self) -> Vec<DmaMapping> {
        self.mappings.read().values().cloned().collect()
    }

    /// Get domain statistics
    pub fn get_stats(&self) -> DomainStatsSnapshot {
        DomainStatsSnapshot {
            id: self.id,
            iommu_type: self.iommu_type,
            attached_devices: self.attached_devices.lock().len(),
            active_mappings: self.mappings.read().len(),
            mapped_pages: self.stats.mapped_pages.load(Ordering::Relaxed),
            mapped_bytes: self.stats.mapped_bytes.load(Ordering::Relaxed),
            unmapped_pages: self.stats.unmapped_pages.load(Ordering::Relaxed),
            unmapped_bytes: self.stats.unmapped_bytes.load(Ordering::Relaxed),
            faults: self.stats.faults.load(Ordering::Relaxed),
        }
    }

    /// Handle IOMMU fault
    pub fn handle_fault(&self, fault: IommuFault) -> Result<(), IommuError> {
        self.stats.faults.fetch_add(1, Ordering::Relaxed);

        // Log fault
        log::error!(
            "IOMMU fault: domain={}, addr=0x{:x}, reason={:?}",
            self.id,
            fault.address,
            fault.reason
        );

        // TODO: Report fault to userspace
        // send_fault_event(fault);

        Err(IommuError::Fault(fault.reason))
    }

    /// Get physical pages for userspace address range
    ///
    /// This is a stub - real implementation would walk page tables
    /// or use get_user_pages() equivalent.
    fn get_phys_pages(&self, user_addr: u64, size: u64) -> Result<Vec<u64>, IommuError> {
        // TODO: Implement proper page table walk or use MMU notifiers
        // For now, return identity mapping (not safe!)

        let num_pages = (size / self.page_size) as usize;
        let mut pages = Vec::with_capacity(num_pages);

        // Identity map (INSECURE - for testing only!)
        for i in 0..num_pages {
            pages.push(user_addr + (i as u64 * self.page_size));
        }

        Ok(pages)
    }
}

/// IOMMU fault information
#[derive(Debug, Clone, Copy)]
pub struct IommuFault {
    /// Faulting address
    pub address: u64,

    /// Fault reason
    pub reason: FaultReason,

    /// Flags
    pub flags: u32,
}

/// IOMMU fault reasons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultReason {
    /// Read access to unmapped address
    ReadUnmapped,

    /// Write access to unmapped address
    WriteUnmapped,

    /// Permission violation
    Permission,

    /// External request failure
    External,

    /// Unknown reason
    Unknown,
}

/// Domain statistics
struct DomainStats {
    mapped_pages: AtomicU64,
    mapped_bytes: AtomicU64,
    unmapped_pages: AtomicU64,
    unmapped_bytes: AtomicU64,
    devices_attached: AtomicU64,
    devices_detached: AtomicU64,
    faults: AtomicU64,
}

impl DomainStats {
    fn new() -> Self {
        Self {
            mapped_pages: AtomicU64::new(0),
            mapped_bytes: AtomicU64::new(0),
            unmapped_pages: AtomicU64::new(0),
            unmapped_bytes: AtomicU64::new(0),
            devices_attached: AtomicU64::new(0),
            devices_detached: AtomicU64::new(0),
            faults: AtomicU64::new(0),
        }
    }
}

/// Domain statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct DomainStatsSnapshot {
    pub id: u64,
    pub iommu_type: IommuType,
    pub attached_devices: usize,
    pub active_mappings: usize,
    pub mapped_pages: u64,
    pub mapped_bytes: u64,
    pub unmapped_pages: u64,
    pub unmapped_bytes: u64,
    pub faults: u64,
}

/// IOMMU error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuError {
    /// IOMMU type not supported
    NotSupported,

    /// Invalid argument
    InvalidArgument,

    /// Device already attached to domain
    DeviceAttached,

    /// Device not found
    DeviceNotFound,

    /// DMA mapping overlap
    Overlap,

    /// IOMMU fault occurred
    Fault(FaultReason),

    /// Hardware error
    HardwareError,

    /// Resource exhausted
    ResourceExhausted,

    /// Internal error
    InternalError,
}

impl core::fmt::Display for IommuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotSupported => write!(f, "IOMMU type not supported"),
            Self::InvalidArgument => write!(f, "Invalid argument"),
            Self::DeviceAttached => write!(f, "Device already attached"),
            Self::DeviceNotFound => write!(f, "Device not found"),
            Self::Overlap => write!(f, "DMA mapping overlap"),
            Self::Fault(reason) => write!(f, "IOMMU fault: {:?}", reason),
            Self::HardwareError => write!(f, "IOMMU hardware error"),
            Self::ResourceExhausted => write!(f, "IOMMU resource exhausted"),
            Self::InternalError => write!(f, "Internal IOMMU error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IommuError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iommu_type_conversion() {
        assert_eq!(IommuType::Type1.as_u32(), 1);
        assert_eq!(IommuType::Type1v2.as_u32(), 2);
        assert_eq!(IommuType::SPAPR.as_u32(), 3);

        assert_eq!(IommuType::from_u32(1), Some(IommuType::Type1));
        assert_eq!(IommuType::from_u32(999), None);
    }

    #[test]
    fn test_domain_create() {
        let domain = IommuDomain::new(IommuType::Type1).unwrap();
        assert_eq!(domain.iommu_type(), IommuType::Type1);
        assert_eq!(domain.page_size(), 4096);
    }

    #[test]
    fn test_domain_map_unmap() {
        let domain = IommuDomain::new(IommuType::Type1).unwrap();
        let iova = 0x1000;
        let user_addr = 0x7f0000000000;
        let size = 0x1000;

        domain.map(iova, user_addr, size, 0x3).unwrap();

        let mapping = domain.get_mapping(iova);
        assert!(mapping.is_some());
        assert_eq!(mapping.unwrap().iova, iova);

        domain.unmap(iova, size).unwrap();

        assert!(domain.get_mapping(iova).is_none());
    }

    #[test]
    fn test_domain_map_overlap() {
        let domain = IommuDomain::new(IommuType::Type1).unwrap();

        domain.map(0x1000, 0x7f0000000000, 0x1000, 0x3).unwrap();

        // Overlapping mapping should fail
        assert!(domain.map(0x1000, 0x7f0000001000, 0x1000, 0x3).is_err());
    }

    #[test]
    fn test_domain_device_attach() {
        let domain = IommuDomain::new(IommuType::Type1).unwrap();

        domain.attach_device(0, 0, 1, 0).unwrap();

        assert!(domain.is_device_attached(0, 0, 1, 0));

        // Duplicate attach should fail
        assert!(domain.attach_device(0, 0, 1, 0).is_err());
    }

    #[test]
    fn test_domain_device_detach() {
        let domain = IommuDomain::new(IommuType::Type1).unwrap();

        domain.attach_device(0, 0, 1, 0).unwrap();
        domain.detach_device(0, 0, 1, 0).unwrap();

        assert!(!domain.is_device_attached(0, 0, 1, 0));

        // Detach non-existent should fail
        assert!(domain.detach_device(0, 0, 1, 0).is_err());
    }

    #[test]
    fn test_domain_stats() {
        let domain = IommuDomain::new(IommuType::Type1).unwrap();

        let stats = domain.get_stats();
        assert_eq!(stats.iommu_type, IommuType::Type1);
        assert_eq!(stats.attached_devices, 0);
        assert_eq!(stats.active_mappings, 0);
    }
}
