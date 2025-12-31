//! # PCI Device Passthrough
//!
//! This module provides support for PCI device passthrough, allowing direct
//! assignment of physical PCI devices to virtual machines. This enables near-native
//! performance for devices like GPUs, network cards, and storage controllers.
//!
//! ## Features
//!
//! - PCI device enumeration and discovery
//! - IOMMU (VT-d/AMD-Vi) configuration
//! - Device assignment to VMs
//! - MSI/MSI-X remapping
//! - DMA remapping
//! - SR-IOV virtual function support
//! - GPU passthrough support
//!
//! ## Architecture
//!
//! Device passthrough uses hardware IOMMU to safely give a VM direct access to
//! a physical device. The IOMMU translates device addresses to physical addresses,
//! providing memory isolation and preventing unauthorized access.
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::passthrough::{PassthroughManager, DeviceAssignment};
//!
//! let manager = PassthroughManager::new()?;
//! let device = manager.find_device(0x01, 0x00, 0x0)?;
//! let assignment = manager.assign_device(vm_id, device)?;
//! ```

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;

use crate::error::KernelError;

/// PCI device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PciId {
    /// Bus number
    pub bus: u8,
    /// Device number
    pub device: u8,
    /// Function number
    pub function: u8,
}

impl PciId {
    /// Create a new PCI ID
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        Self { bus, device, function }
    }

    /// Convert to u32
    pub fn as_u32(&self) -> u32 {
        ((self.bus as u32) << 8) | ((self.device as u32) << 3) | (self.function as u32)
    }

    /// Format as BDF string
    pub fn to_bdf(&self) -> String {
        alloc::format!("{:02x}:{:02x}.{:x}", self.bus, self.device, self.function)
    }
}

/// PCI device information
#[derive(Debug, Clone)]
pub struct PciDevice {
    /// PCI identifier
    pub id: PciId,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Class code
    pub class: u8,
    /// Subclass
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
    /// BARs (Base Address Registers)
    pub bars: [Option<BarInfo>; 6],
    /// IRQ number
    pub irq: Option<u32>,
    /// Device name
    pub name: String,
    /// Is SR-IOV virtual function
    pub is_vf: bool,
    /// SR-IOV physical function ID (if VF)
    pub pf_id: Option<PciId>,
}

/// BAR (Base Address Register) information
#[derive(Debug, Clone, Copy)]
pub struct BarInfo {
    /// BAR index
    pub index: u8,
    /// Physical address
    pub address: u64,
    /// Size
    pub size: u64,
    /// Is I/O space
    pub is_io: bool,
    /// Is 64-bit
    pub is_64bit: bool,
    /// Is prefetchable
    pub prefetchable: bool,
}

/// IOMMU type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuType {
    /// Intel VT-d
    IntelVtd,
    /// AMD-Vi
    AmdVi,
    /// Unknown IOMMU
    Unknown,
}

/// IOMMU domain
#[derive(Debug)]
pub struct IommuDomain {
    /// Domain ID
    pub id: u32,
    /// IOMMU type
    pub iommu_type: IommuType,
    /// Attached devices
    pub devices: Vec<PciId>,
    /// Page table root
    pub page_table_root: u64,
}

impl IommuDomain {
    /// Create a new IOMMU domain
    pub fn new(id: u32, iommu_type: IommuType) -> Self {
        Self {
            id,
            iommu_type,
            devices: Vec::new(),
            page_table_root: 0,
        }
    }

    /// Attach a device to this domain
    pub fn attach_device(&mut self, device: PciId) {
        self.devices.push(device);
    }

    /// Detach a device from this domain
    pub fn detach_device(&mut self, device: PciId) -> bool {
        if let Some(pos) = self.devices.iter().position(|&d| d == device) {
            self.devices.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if device is attached
    pub fn has_device(&self, device: PciId) -> bool {
        self.devices.contains(&device)
    }

    /// Get number of attached devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

/// MSI/MSI-X configuration
#[derive(Debug, Clone, Copy)]
pub struct MsiConfig {
    /// Is MSI-X
    pub is_msix: bool,
    /// Number of vectors
    pub num_vectors: u16,
    /// Table offset (for MSI-X)
    pub table_offset: u32,
    /// PBA offset (for MSI-X)
    pub pba_offset: u32,
}

impl Default for MsiConfig {
    fn default() -> Self {
        Self {
            is_msix: false,
            num_vectors: 1,
            table_offset: 0,
            pba_offset: 0,
        }
    }
}

/// Device assignment configuration
#[derive(Debug, Clone)]
pub struct DeviceAssignment {
    /// PCI device
    pub device: PciDevice,
    /// VM ID
    pub vm_id: u32,
    /// IOMMU domain
    pub domain: Option<u32>,
    /// MSI configuration
    pub msi_config: MsiConfig,
    /// Assigned flags
    pub flags: AssignmentFlags,
    /// Guest physical address for BARs
    pub guest_bars: [Option<u64>; 6],
}

bitflags::bitflags! {
    /// Device assignment flags
    #[derive(Debug, Clone, Copy)]
    pub struct AssignmentFlags: u32 {
        /// No flags
        const NONE = 0;
        /// Enable INTx
        const INTX = 1 << 0;
        /// Enable MSI
        const MSI = 1 << 1;
        /// Enable MSI-X
        const MSIX = 1 << 2;
        /// Enable write-combining
        const WRITE_COMBINE = 1 << 3;
        /// Relaxed ordering
        const RELAXED_ORDERING = 1 << 4;
        /// Use per-domain mapping
        const PER_DOMAIN = 1 << 5;
    }
}

impl Default for AssignmentFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// SR-IOV virtual function info
#[derive(Debug, Clone)]
pub struct VirtualFunction {
    /// VF PCI ID
    pub id: PciId,
    /// Physical function ID
    pub pf_id: PciId,
    /// VF number
    pub vf_index: u16,
    /// VF device ID
    pub device_id: u16,
}

/// Errors that can occur during passthrough operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassthroughError {
    /// Device not found
    DeviceNotFound(PciId),
    /// Device already assigned
    DeviceAlreadyAssigned(PciId),
    /// IOMMU not available
    IommuNotAvailable,
    /// IOMMU operation failed
    IommuOperationFailed(String),
    /// DMA remapping failed
    DmaRemappingFailed,
    /// MSI configuration failed
    MsiConfigurationFailed,
    /// Invalid device configuration
    InvalidConfiguration,
    /// Resource not available
    ResourceUnavailable,
    /// SR-IOV not supported
    SriovNotSupported,
    /// VF not available
    VfNotAvailable,
}

impl From<PassthroughError> for KernelError {
    fn from(err: PassthroughError) -> Self {
        KernelError::Virtualization(format!("Passthrough error: {:?}", err))
    }
}

/// Device passthrough manager
#[derive(Debug)]
pub struct PassthroughManager {
    /// IOMMU type
    iommu_type: IommuType,
    /// IOMMU enabled
    iommu_enabled: Arc<AtomicBool>,
    /// PCI devices
    pci_devices: Arc<Mutex<BTreeMap<PciId, PciDevice>>>,
    /// Device assignments
    assignments: Arc<Mutex<BTreeMap<u32, Vec<DeviceAssignment>>>>,
    /// IOMMU domains
    domains: Arc<Mutex<BTreeMap<u32, IommuDomain>>>,
    /// Next domain ID
    next_domain_id: Arc<AtomicU32>,
    /// SR-IOV VFs
    virtual_functions: Arc<Mutex<Vec<VirtualFunction>>>,
}

impl PassthroughManager {
    /// Create a new passthrough manager
    pub fn new() -> Result<Self, PassthroughError> {
        // Detect IOMMU type
        let iommu_type = Self::detect_iommu();

        Ok(Self {
            iommu_type,
            iommu_enabled: Arc::new(AtomicBool::new(false)),
            pci_devices: Arc::new(Mutex::new(BTreeMap::new())),
            assignments: Arc::new(Mutex::new(BTreeMap::new())),
            domains: Arc::new(Mutex::new(BTreeMap::new())),
            next_domain_id: Arc::new(AtomicU32::new(1)),
            virtual_functions: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Detect IOMMU type
    fn detect_iommu() -> IommuType {
        // In a real implementation, this would detect the actual IOMMU
        // For now, assume Intel VT-d
        IommuType::IntelVtd
    }

    /// Get IOMMU type
    pub fn iommu_type(&self) -> IommuType {
        self.iommu_type
    }

    /// Check if IOMMU is enabled
    pub fn iommu_enabled(&self) -> bool {
        self.iommu_enabled.load(Ordering::SeqCst)
    }

    /// Enable IOMMU
    pub fn enable_iommu(&self) -> Result<(), PassthroughError> {
        self.iommu_enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Disable IOMMU
    pub fn disable_iommu(&self) {
        self.iommu_enabled.store(false, Ordering::SeqCst);
    }

    /// Add a PCI device
    pub fn add_device(&self, device: PciDevice) -> Result<(), PassthroughError> {
        let mut devices = self.pci_devices.lock();
        devices.insert(device.id, device);
        Ok(())
    }

    /// Remove a PCI device
    pub fn remove_device(&self, id: PciId) -> Result<(), PassthroughError> {
        let mut devices = self.pci_devices.lock();
        if !devices.remove(&id).is_some() {
            return Err(PassthroughError::DeviceNotFound(id));
        }
        Ok(())
    }

    /// Find a PCI device
    pub fn find_device(&self, bus: u8, device: u8, function: u8) -> Result<PciDevice, PassthroughError> {
        let id = PciId::new(bus, device, function);
        let devices = self.pci_devices.lock();

        devices
            .get(&id)
            .cloned()
            .ok_or(PassthroughError::DeviceNotFound(id))
    }

    /// List all PCI devices
    pub fn list_devices(&self) -> Vec<PciDevice> {
        let devices = self.pci_devices.lock();
        devices.values().cloned().collect()
    }

    /// Create an IOMMU domain
    pub fn create_domain(&self) -> Result<u32, PassthroughError> {
        if !self.iommu_enabled() {
            return Err(PassthroughError::IommuNotAvailable);
        }

        let domain_id = self.next_domain_id.fetch_add(1, Ordering::SeqCst);
        let domain = IommuDomain::new(domain_id, self.iommu_type);

        let mut domains = self.domains.lock();
        domains.insert(domain_id, domain);

        Ok(domain_id)
    }

    /// Destroy an IOMMU domain
    pub fn destroy_domain(&self, domain_id: u32) -> Result<(), PassthroughError> {
        let mut domains = self.domains.lock();

        if !domains.remove(&domain_id).is_some() {
            return Err(PassthroughError::IommuOperationFailed(
                "Domain not found".into(),
            ));
        }

        Ok(())
    }

    /// Get IOMMU domain
    pub fn get_domain(&self, domain_id: u32) -> Option<IommuDomain> {
        let domains = self.domains.lock();
        domains.get(&domain_id).cloned()
    }

    /// Attach device to domain
    pub fn attach_device_to_domain(
        &self,
        domain_id: u32,
        device: PciId,
    ) -> Result<(), PassthroughError> {
        let mut domains = self.domains.lock();

        if let Some(domain) = domains.get_mut(&domain_id) {
            domain.attach_device(device);
            Ok(())
        } else {
            Err(PassthroughError::IommuOperationFailed(
                "Domain not found".into(),
            ))
        }
    }

    /// Detach device from domain
    pub fn detach_device_from_domain(
        &self,
        domain_id: u32,
        device: PciId,
    ) -> Result<(), PassthroughError> {
        let mut domains = self.domains.lock();

        if let Some(domain) = domains.get_mut(&domain_id) {
            if domain.detach_device(device) {
                Ok(())
            } else {
                Err(PassthroughError::IommuOperationFailed(
                    "Device not in domain".into(),
                ))
            }
        } else {
            Err(PassthroughError::IommuOperationFailed(
                "Domain not found".into(),
            ))
        }
    }

    /// Assign a device to a VM
    pub fn assign_device(
        &self,
        vm_id: u32,
        device_id: PciId,
    ) -> Result<DeviceAssignment, PassthroughError> {
        if !self.iommu_enabled() {
            return Err(PassthroughError::IommuNotAvailable);
        }

        // Get device
        let device = self.find_device(device_id.bus, device_id.device, device_id.function)?;

        // Check if device is already assigned
        let assignments = self.assignments.lock();
        for (_, vm_devices) in assignments.iter() {
            for assignment in vm_devices {
                if assignment.device.id == device_id {
                    return Err(PassthroughError::DeviceAlreadyAssigned(device_id));
                }
            }
        }
        drop(assignments);

        // Create IOMMU domain for this device
        let domain_id = self.create_domain()?;
        self.attach_device_to_domain(domain_id, device_id)?;

        // Create assignment
        let assignment = DeviceAssignment {
            device,
            vm_id,
            domain: Some(domain_id),
            msi_config: MsiConfig::default(),
            flags: AssignmentFlags::MSI | AssignmentFlags::MSIX,
            guest_bars: [None, None, None, None, None, None],
        };

        // Store assignment
        let mut assignments = self.assignments.lock();
        assignments
            .entry(vm_id)
            .or_insert_with(Vec::new)
            .push(assignment.clone());

        Ok(assignment)
    }

    /// Deassign a device from a VM
    pub fn deassign_device(&self, vm_id: u32, device_id: PciId) -> Result<(), PassthroughError> {
        let mut assignments = self.assignments.lock();

        if let Some(vm_devices) = assignments.get_mut(&vm_id) {
            if let Some(pos) = vm_devices.iter().position(|a| a.device.id == device_id) {
                let assignment = vm_devices.remove(pos);

                // Cleanup IOMMU domain
                if let Some(domain_id) = assignment.domain {
                    let mut domains = self.domains.lock();
                    if let Some(domain) = domains.get_mut(&domain_id) {
                        domain.detach_device(device_id);
                    }
                    domains.remove(&domain_id);
                }

                return Ok(());
            }
        }

        Err(PassthroughError::DeviceNotFound(device_id))
    }

    /// Get all device assignments for a VM
    pub fn get_assignments(&self, vm_id: u32) -> Vec<DeviceAssignment> {
        let assignments = self.assignments.lock();

        assignments
            .get(&vm_id)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Configure MSI for a device
    pub fn configure_msi(
        &self,
        device_id: PciId,
        msi_config: MsiConfig,
    ) -> Result<(), PassthroughError> {
        // In a real implementation, this would configure the MSI registers
        Ok(())
    }

    /// Enable SR-IOV for a device
    pub fn enable_sriov(&self, pf_id: PciId, num_vfs: u16) -> Result<(), PassthroughError> {
        // In a real implementation, this would enable SR-IOV and create VFs
        for i in 0..num_vfs {
            let vf = VirtualFunction {
                id: PciId::new(pf_id.bus, pf_id.device + 1 + (i as u8) / 8, (i as u8) % 8),
                pf_id,
                vf_index: i,
                device_id: 0xFFFF, // Would be actual VF device ID
            };

            let mut vfs = self.virtual_functions.lock();
            vfs.push(vf);
        }

        Ok(())
    }

    /// Get available virtual functions
    pub fn get_virtual_functions(&self, pf_id: PciId) -> Vec<VirtualFunction> {
        let vfs = self.virtual_functions.lock();

        vfs.iter()
            .filter(|vf| vf.pf_id == pf_id)
            .cloned()
            .collect()
    }

    /// Check if device is a GPU
    pub fn is_gpu(&self, device: &PciDevice) -> bool {
        // GPU class codes
        matches!(device.class, 0x03)
    }

    /// Check if device is a network device
    pub fn is_network(&self, device: &PciDevice) -> bool {
        // Network class codes
        matches!(device.class, 0x02)
    }

    /// Check if device is a storage device
    pub fn is_storage(&self, device: &PciDevice) -> bool {
        // Storage class codes
        matches!(device.class, 0x01)
    }

    /// Get GPU devices
    pub fn get_gpu_devices(&self) -> Vec<PciDevice> {
        self.list_devices()
            .into_iter()
            .filter(|d| self.is_gpu(d))
            .collect()
    }

    /// Get network devices
    pub fn get_network_devices(&self) -> Vec<PciDevice> {
        self.list_devices()
            .into_iter()
            .filter(|d| self.is_network(d))
            .collect()
    }

    /// Get storage devices
    pub fn get_storage_devices(&self) -> Vec<PciDevice> {
        self.list_devices()
            .into_iter()
            .filter(|d| self.is_storage(d))
            .collect()
    }
}

impl Default for PassthroughManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pci_id() {
        let id = PciId::new(0, 31, 0);
        assert_eq!(id.bus, 0);
        assert_eq!(id.device, 31);
        assert_eq!(id.function, 0);
    }

    #[test]
    fn test_pci_id_to_bdf() {
        let id = PciId::new(0, 31, 0);
        let bdf = id.to_bdf();
        assert_eq!(bdf, "00:1f.0");
    }

    #[test]
    fn test_passthrough_manager_creation() {
        let manager = PassthroughManager::new().unwrap();
        assert!(!manager.iommu_enabled());
    }

    #[test]
    fn test_enable_iommu() {
        let manager = PassthroughManager::new().unwrap();
        manager.enable_iommu().unwrap();
        assert!(manager.iommu_enabled());
    }

    #[test]
    fn test_add_remove_device() {
        let manager = PassthroughManager::new().unwrap();

        let device = PciDevice {
            id: PciId::new(0, 1, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: 0x02,
            subclass: 0x00,
            prog_if: 0x00,
            bars: [None, None, None, None, None, None],
            irq: Some(16),
            name: "Test Device".into(),
            is_vf: false,
            pf_id: None,
        };

        manager.add_device(device.clone()).unwrap();
        let found = manager.find_device(0, 1, 0).unwrap();
        assert_eq!(found.device_id, 0x1234);

        manager.remove_device(device.id).unwrap();
    }

    #[test]
    fn test_domain_creation() {
        let manager = PassthroughManager::new().unwrap();
        manager.enable_iommu().unwrap();

        let domain_id = manager.create_domain().unwrap();
        assert_eq!(domain_id, 1);

        let domain = manager.get_domain(domain_id).unwrap();
        assert_eq!(domain.id, domain_id);
    }

    #[test]
    fn test_domain_attach_detach() {
        let manager = PassthroughManager::new().unwrap();
        manager.enable_iommu().unwrap();

        let domain_id = manager.create_domain().unwrap();
        let device_id = PciId::new(0, 1, 0);

        manager.attach_device_to_domain(domain_id, device_id).unwrap();

        let domain = manager.get_domain(domain_id).unwrap();
        assert!(domain.has_device(device_id));

        manager.detach_device_from_domain(domain_id, device_id).unwrap();

        let domain = manager.get_domain(domain_id).unwrap();
        assert!(!domain.has_device(device_id));
    }
}
