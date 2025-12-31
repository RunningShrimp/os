//! Memory region management for UIO devices
//!
//! This module provides memory region descriptors and mapping functionality
//! for safe access to device memory from userspace.

use crate::drivers::uio::{UioError, UioResult};
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};

/// UIO memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UioMemRegionType {
    /// Memory-mapped I/O region
    Mmio = 0,

    /// Port I/O region (x86 specific)
    PortIo = 1,

    /// Custom region type
    Custom = 2,

    /// Reserved/invalid region
    Invalid = 3,
}

impl UioMemRegionType {
    /// Convert from integer
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Mmio,
            1 => Self::PortIo,
            2 => Self::Custom,
            _ => Self::Invalid,
        }
    }

    /// Convert to integer
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Memory access permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UioMemPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission (typically false for MMIO)
    pub execute: bool,
}

impl UioMemPermissions {
    /// Create new permissions
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    /// Read-only permission
    pub fn read_only() -> Self {
        Self::new(true, false, false)
    }

    /// Write-only permission
    pub fn write_only() -> Self {
        Self::new(false, true, false)
    }

    /// Read-write permission
    pub fn read_write() -> Self {
        Self::new(true, true, false)
    }

    /// No access
    pub fn none() -> Self {
        Self::new(false, false, false)
    }

    /// Convert to flags
    pub fn as_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.read {
            flags |= 0x01;
        }
        if self.write {
            flags |= 0x02;
        }
        if self.execute {
            flags |= 0x04;
        }
        flags
    }

    /// Create from flags
    pub fn from_flags(flags: u32) -> Self {
        Self {
            read: (flags & 0x01) != 0,
            write: (flags & 0x02) != 0,
            execute: (flags & 0x04) != 0,
        }
    }
}

/// UIO memory region descriptor
///
/// Describes a memory region that can be mapped to userspace.
#[derive(Debug, Clone)]
pub struct UioMemRegion {
    /// Region type
    region_type: UioMemRegionType,
    /// Physical address
    phys_addr: u64,
    /// Region size
    size: usize,
    /// Access permissions
    permissions: UioMemPermissions,
    /// Region name
    name: String,
    /// Internal address (kernel virtual address)
    internal_addr: Option<u64>,
    /// Mapped flag
    mapped: AtomicBool,
}

impl UioMemRegion {
    /// Create a new memory region
    pub fn new(
        region_type: UioMemRegionType,
        phys_addr: u64,
        size: usize,
        permissions: UioMemPermissions,
        name: String,
    ) -> Self {
        Self {
            region_type,
            phys_addr,
            size,
            permissions,
            name,
            internal_addr: None,
            mapped: AtomicBool::new(false),
        }
    }

    /// Create an MMIO region
    pub fn mmio(phys_addr: u64, size: usize) -> Self {
        Self::new(
            UioMemRegionType::Mmio,
            phys_addr,
            size,
            UioMemPermissions::read_write(),
            "mmio".to_string(),
        )
    }

    /// Create a Port I/O region
    pub fn port_io(base_port: u64, num_ports: usize) -> Self {
        Self::new(
            UioMemRegionType::PortIo,
            base_port,
            num_ports,
            UioMemPermissions::read_write(),
            "portio".to_string(),
        )
    }

    /// Create a custom region
    pub fn custom(phys_addr: u64, size: usize, name: String) -> Self {
        Self::new(
            UioMemRegionType::Custom,
            phys_addr,
            size,
            UioMemPermissions::read_write(),
            name,
        )
    }

    /// Get region type
    pub fn region_type(&self) -> UioMemRegionType {
        self.region_type
    }

    /// Get physical address
    pub fn phys_addr(&self) -> u64 {
        self.phys_addr
    }

    /// Get region size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get permissions
    pub fn permissions(&self) -> UioMemPermissions {
        self.permissions
    }

    /// Set permissions
    pub fn set_permissions(&mut self, perms: UioMemPermissions) {
        self.permissions = perms;
    }

    /// Get region name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set region name
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Get internal kernel virtual address
    pub fn internal_addr(&self) -> Option<u64> {
        self.internal_addr
    }

    /// Set internal kernel virtual address
    pub fn set_internal_addr(&mut self, addr: u64) {
        self.internal_addr = Some(addr);
    }

    /// Check if region is mapped
    pub fn is_mapped(&self) -> bool {
        self.mapped.load(Ordering::Acquire)
    }

    /// Set mapped status
    pub fn set_mapped(&self, mapped: bool) {
        self.mapped.store(mapped, Ordering::Release);
    }

    /// Validate region parameters
    pub fn validate(&self) -> UioResult<()> {
        if self.size == 0 {
            return Err(UioError::InvalidRegion);
        }

        // Check alignment based on type
        match self.region_type {
            UioMemRegionType::Mmio => {
                if self.phys_addr % 0x1000 != 0 {
                    // Page-aligned for MMIO
                    return Err(UioError::InvalidRegion);
                }
            }
            UioMemRegionType::PortIo => {
                if self.phys_addr > 0xFFFF {
                    // Port I/O only 16-bit address space
                    return Err(UioError::InvalidRegion);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Map region to userspace
    pub fn map(&self) -> UioResult<usize> {
        if self.is_mapped() {
            return Err(UioError::RegionMapped);
        }

        self.validate()?;

        // Platform-specific mapping implementation
        log::debug!(
            "Mapping region {} at {:#X} (size: {:#X})",
            self.name,
            self.phys_addr,
            self.size
        );

        Ok(self.phys_addr as usize)
    }

    /// Unmap region
    pub fn unmap(&self) -> UioResult<()> {
        if !self.is_mapped() {
            return Ok(());
        }

        // Platform-specific unmapping
        log::debug!("Unmapping region {}", self.name);

        self.set_mapped(false);
        Ok(())
    }
}

/// Memory region manager
///
/// Manages multiple memory regions for a UIO device.
#[derive(Debug)]
pub struct UioMemRegionManager {
    regions: alloc::vec::Vec<Option<UioMemRegion>>,
    max_regions: usize,
}

impl UioMemRegionManager {
    /// Create new region manager
    pub fn new(max_regions: usize) -> Self {
        Self {
            regions: alloc::vec![None; max_regions],
            max_regions,
        }
    }

    /// Add a memory region
    pub fn add_region(&mut self, mut region: UioMemRegion) -> UioResult<usize> {
        // Find free slot
        let index = self
            .regions
            .iter()
            .position(|r| r.is_none())
            .ok_or(UioError::NoMemory)?;

        region.validate()?;
        region.set_mapped(false);

        self.regions[index] = Some(region);
        Ok(index)
    }

    /// Remove a memory region
    pub fn remove_region(&mut self, index: usize) -> UioResult<UioMemRegion> {
        if index >= self.max_regions {
            return Err(UioError::InvalidRegion);
        }

        self.regions[index]
            .take()
            .ok_or(UioError::InvalidRegion)
    }

    /// Get region by index
    pub fn get_region(&self, index: usize) -> UioResult<&UioMemRegion> {
        if index >= self.max_regions {
            return Err(UioError::InvalidRegion);
        }

        self.regions[index]
            .as_ref()
            .ok_or(UioError::InvalidRegion)
    }

    /// Get mutable region by index
    pub fn get_region_mut(&mut self, index: usize) -> UioResult<&mut UioMemRegion> {
        if index >= self.max_regions {
            return Err(UioError::InvalidRegion);
        }

        self.regions[index]
            .as_mut()
            .ok_or(UioError::InvalidRegion)
    }

    /// Get all regions
    pub fn regions(&self) -> alloc::vec::Vec<Option<&UioMemRegion>> {
        self.regions.iter().map(|r| r.as_ref()).collect()
    }

    /// Get number of active regions
    pub fn count(&self) -> usize {
        self.regions.iter().filter(|r| r.is_some()).count()
    }

    /// Map all regions
    pub fn map_all(&self) -> UioResult<()> {
        for (i, region) in self.regions.iter().enumerate() {
            if let Some(ref region) = region {
                region.map()?;
                log::debug!("Mapped region {}", i);
            }
        }
        Ok(())
    }

    /// Unmap all regions
    pub fn unmap_all(&self) -> UioResult<()> {
        for region in self.regions.iter().flatten() {
            region.unmap()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_type_conversions() {
        assert_eq!(UioMemRegionType::from_u8(0), UioMemRegionType::Mmio);
        assert_eq!(UioMemRegionType::Mmio.as_u8(), 0);
    }

    #[test]
    fn test_permissions_read_only() {
        let perms = UioMemPermissions::read_only();
        assert!(perms.read);
        assert!(!perms.write);
        assert!(!perms.execute);
    }

    #[test]
    fn test_permissions_read_write() {
        let perms = UioMemPermissions::read_write();
        assert!(perms.read);
        assert!(perms.write);
        assert!(!perms.execute);
    }

    #[test]
    fn test_permissions_flags() {
        let perms = UioMemPermissions::read_write();
        let flags = perms.as_flags();
        assert_eq!(flags, 0x03);

        let perms2 = UioMemPermissions::from_flags(flags);
        assert_eq!(perms2.read, perms.read);
        assert_eq!(perms2.write, perms.write);
    }

    #[test]
    fn test_region_mmio() {
        let region = UioMemRegion::mmio(0xF0000000, 0x1000);
        assert_eq!(region.region_type(), UioMemRegionType::Mmio);
        assert_eq!(region.phys_addr(), 0xF0000000);
        assert_eq!(region.size(), 0x1000);
        assert_eq!(region.name(), "mmio");
    }

    #[test]
    fn test_region_port_io() {
        let region = UioMemRegion::port_io(0x3F8, 8);
        assert_eq!(region.region_type(), UioMemRegionType::PortIo);
        assert_eq!(region.phys_addr(), 0x3F8);
        assert_eq!(region.size(), 8);
    }

    #[test]
    fn test_region_custom() {
        let region = UioMemRegion::custom(0xE0000000, 0x2000, "custom".to_string());
        assert_eq!(region.region_type(), UioMemRegionType::Custom);
        assert_eq!(region.name(), "custom");
    }

    #[test]
    fn test_region_validate() {
        let region = UioMemRegion::mmio(0xF0000000, 0x1000);
        assert!(region.validate().is_ok());

        let bad_region = UioMemRegion::mmio(0xF0000001, 0x1000);
        assert!(bad_region.validate().is_err());
    }

    #[test]
    fn test_region_manager() {
        let mut manager = UioMemRegionManager::new(8);
        assert_eq!(manager.count(), 0);

        let region = UioMemRegion::mmio(0xF0000000, 0x1000);
        let idx = manager.add_region(region).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(manager.count(), 1);

        let retrieved = manager.get_region(idx).unwrap();
        assert_eq!(retrieved.phys_addr(), 0xF0000000);

        manager.remove_region(idx).unwrap();
        assert_eq!(manager.count(), 0);
    }
}
