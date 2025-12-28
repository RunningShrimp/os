//! Memory Protection Unit (MPU) Implementation
//!
//! This module implements MPU (Memory Protection Unit) for hardware-accelerated
//! memory access control and protection:
//! - Memory region configuration with fine-grained permissions
//! - Subregion support for complex memory layouts
//! - MPU enable/disable control
//! - Fault handling and debugging support
//! - Integration with page table isolation and ASLR
//!
//! Features:
//! - Up to 16 memory regions with per-region permissions
//! - Subregion support (up to 8 subregions per region)
//! - Attribute configuration (access permissions, cacheability, shareable)
//! - MPU fault handling with context
//! - Background MPU update support
//! - MPU validation and debugging utilities

use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use crate::subsystems::mm::page_table_isolation::*;
use core::sync::atomic;

// ============================================================================
// MPU Constants
// ============================================================================

/// Maximum number of MPU regions
pub const MPU_MAX_REGIONS: usize = 16;

/// Maximum number of subregions per region
pub const MPU_MAX_SUBREGIONS: usize = 8;

/// MPU region base address alignment (must be power of 2)
pub const MPU_REGION_ALIGNMENT: usize = 32;

/// MPU region size alignment
pub const MPU_SIZE_ALIGNMENT: usize = 256;

/// Invalid region number
pub const MPU_INVALID_REGION: usize = 0xFF;

// ============================================================================
// MPU Attributes
// ============================================================================

/// MPU access permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpuPermissions {
    /// Readable
    pub read: bool,
    
    /// Writable
    pub write: bool,
    
    /// Executable
    pub execute: bool,
    
    /// Privileged access (supervisor mode)
    pub privileged: bool,
}

impl MpuPermissions {
    /// Create new permissions (no access)
    pub fn new() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            privileged: false,
        }
    }
    
    /// Read-only permissions
    pub fn readonly() -> Self {
        Self {
            read: true,
            write: false,
            execute: true,
            privileged: false,
        }
    }
    
    /// Read-write permissions
    pub fn readwrite() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
            privileged: false,
        }
    }
    
    /// Read-write-execute permissions
    pub fn readwriteexecute() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
            privileged: false,
        }
    }
    
    /// Privileged region (read/write/execute in supervisor mode)
    pub fn privileged() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
            privileged: true,
        }
    }
    
    /// Convert to MPU register format
    pub fn to_mpu_rbar(&self) -> u32 {
        let mut rbar = 0u32;
        
        if self.read {
            rbar |= 1 << 0;
        }
        
        if self.write {
            rbar |= 1 << 1;
        }
        
        if self.execute {
            rbar |= 1 << 2;
        }
        
        if self.privileged {
            rbar |= 1 << 3;
        }
        
        rbar
    }
    
    /// Parse from MPU register format
    pub fn from_mpu_rbar(rbar: u32) -> Self {
        Self {
            read: (rbar & (1 << 0)) != 0,
            write: (rbar & (1 << 1)) != 0,
            execute: (rbar & (1 << 2)) != 0,
            privileged: (rbar & (1 << 3)) != 0,
        }
    }
}

/// MPU region attributes
#[derive(Debug, Clone, Copy)]
pub struct MpuRegionAttributes {
    /// Cacheable in L1 cache
    pub cacheable: bool,
    
    /// Shareable across multiple bus masters
    pub shareable: bool,
    
    /// Bufferable (write-back cacheable)
    pub bufferable: bool,
    
    /// Not cacheable (strong ordering requirements)
    pub not_cacheable: bool,
}

impl MpuRegionAttributes {
    /// Create new default attributes
    pub fn new() -> Self {
        Self {
            cacheable: true,
            shareable: false,
            bufferable: false,
            not_cacheable: false,
        }
    }
    
    /// Create non-cacheable region
    pub fn non_cacheable() -> Self {
        Self {
            cacheable: false,
            shareable: false,
            bufferable: false,
            not_cacheable: true,
        }
    }
}

// ============================================================================
// MPU Region Configuration
// ============================================================================

/// MPU memory region configuration
#[derive(Debug, Clone)]
pub struct MpuRegion {
    /// Region number (0-15)
    pub region_number: u8,
    
    /// Region enabled flag
    pub enabled: bool,
    
    /// Base address (must be aligned to MPU_REGION_ALIGNMENT)
    pub base_address: u64,
    
    /// Region size (must be multiple of MPU_SIZE_ALIGNMENT)
    pub size: usize,
    
    /// Access permissions
    pub permissions: MpuPermissions,
    
    /// Region attributes
    pub attributes: MpuRegionAttributes,
    
    /// Number of subregions
    pub num_subregions: u8,
    
    /// Subregion array (if any)
    pub subregions: [Option<MpuSubregion>; MPU_MAX_SUBREGIONS],
    
    /// MPU violation count (for statistics)
    pub violation_count: AtomicU32,
    
    /// Last access timestamp
    pub last_access: AtomicU64,
}

/// MPU subregion (for finer-grained control)
#[derive(Debug, Clone, Copy)]
pub struct MpuSubregion {
    /// Subregion number within parent region
    pub subregion_number: u8,
    
    /// Enabled flag
    pub enabled: bool,
    
    /// Base address (relative to parent region)
    pub base_address: u64,
    
    /// Size (must not extend beyond parent region)
    pub size: usize,
    
    /// Access permissions (subset of parent)
    pub permissions: MpuPermissions,
}

impl MpuRegion {
    /// Create new MPU region
    pub fn new(region_number: u8, base_address: u64, size: usize, 
               permissions: MpuPermissions) -> Self {
        Self {
            region_number,
            enabled: true,
            base_address,
            size,
            permissions,
            attributes: MpuRegionAttributes::new(),
            num_subregions: 0,
            subregions: [None; MPU_MAX_SUBREGIONS],
            violation_count: AtomicU32::new(0),
            last_access: AtomicU64::new(0),
        }
    }
    
    /// Check if region is valid
    pub fn is_valid(&self) -> bool {
        self.enabled && 
        self.size > 0 &&
        self.size <= (1usize << 32) && // Max 4GB
        self.base_address & (MPU_REGION_ALIGNMENT - 1) == 0 && // Alignment check
        self.size % MPU_SIZE_ALIGNMENT == 0 // Size alignment check
    }
    
    /// Get end address of region
    pub fn end_address(&self) -> u64 {
        self.base_address.wrapping_add(self.size as u64)
    }
    
    /// Check if address is within this region
    pub fn contains(&self, address: u64) -> bool {
        if !self.enabled {
            return false;
        }
        
        let end = self.end_address();
        
        // Handle wraparound
        if end > self.base_address {
            address >= self.base_address && address < end
        } else {
            address >= self.base_address || address < end
        }
    }
    
    /// Record access (for aging)
    pub fn record_access(&self) {
        self.last_access.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);
    }
    
    /// Get last access time
    pub fn get_last_access_time(&self) -> u64 {
        self.last_access.load(Ordering::Relaxed)
    }
    
    /// Check for subregion collision
    pub fn check_subregion_collision(&self, sub: &MpuSubregion) -> bool {
        let sub_end = sub.base_address.wrapping_add(sub.size as u64);
        
        // Subregion must not extend beyond parent region
        let parent_end = self.end_address();
        
        if parent_end > sub.base_address {
            sub_end <= parent_end
        } else {
            // Handle wraparound
            true
        }
    }
    
    /// Add subregion
    pub fn add_subregion(&mut self, sub: MpuSubregion) -> Result<(), MpuError> {
        if self.num_subregions >= MPU_MAX_SUBREGIONS as u8 {
            return Err(MpuError::TooManySubregions);
        }
        
        // Check alignment and bounds
        if sub.base_address & (MPU_REGION_ALIGNMENT - 1) != 0 {
            return Err(MpuError::AlignmentError);
        }
        
        if sub.size == 0 || sub.size > self.size {
            return Err(MpuError::InvalidSubregion);
        }
        
        // Check if subregion extends beyond parent region
        if !self.check_subregion_collision(&sub) {
            return Err(MpuError::SubregionBoundsError);
        }
        
        // Find empty subregion slot
        for i in 0..self.num_subregions {
            if self.subregions[i].is_none() {
                self.subregions[i] = Some(sub);
                self.num_subregions += 1;
                return Ok(());
            }
        }
        
        Err(MpuError::NoFreeSubregionSlots)
    }
    
    /// Remove subregion
    pub fn remove_subregion(&mut self, subregion_number: u8) {
        if subregion_number < self.num_subregions {
            self.subregions[subregion_number as usize] = None;
            self.num_subregions -= 1;
        }
    }
    
    /// Disable region
    pub fn disable(&mut self) {
        self.enabled = false;
        crate::println!("[mpu] Disabled region {}", self.region_number);
    }
    
    /// Enable region
    pub fn enable(&mut self) {
        self.enabled = true;
        crate::println!("[mpu] Enabled region {}", self.region_number);
    }
}

/// MPU subregion implementation
impl MpuSubregion {
    /// Create new subregion
    pub fn new(subregion_number: u8, base_address: u64, size: usize, 
               permissions: MpuPermissions) -> Self {
        Self {
            subregion_number,
            enabled: true,
            base_address,
            size,
            permissions,
        }
    }
}

// ============================================================================
// MPU Manager
// ============================================================================

/// MPU configuration and control
pub struct MpuManager {
    /// MPU regions
    regions: [Option<MpuRegion>; MPU_MAX_REGIONS],
    
    /// MPU enabled flag
    mpu_enabled: AtomicBool,
    
    /// Background MPU update flag
    background_update: AtomicBool,
    
    /// MPU configuration registers (mock)
    pub mpu_rnr: AtomicU8,     // Region Number Register
    pub mpu_rbar: AtomicU32,  // Region Base Address Register
    pub mpu_rlar: AtomicU64,  // Region Limit Address Register
    pub mpu_rdr: AtomicU32,  // Region Size Register
    pub mpu_rcr: AtomicU32,      // Region Attribute Register
    
    /// Violation statistics
    pub total_violations: AtomicU32,
    
    /// Last MPU update timestamp
    pub last_update: AtomicU64,
}

/// MPU error types
#[derive(Debug, Clone)]
pub enum MpuError {
    /// Region number already in use
    RegionInUse {
        region_number: u8,
    },
    
    /// Invalid region configuration
    InvalidRegionConfig,
    
    /// Alignment error
    AlignmentError,
    
    /// Subregion error
    SubregionError {
        sub_error: SubregionErrorType,
    },
    
    /// MPU not enabled
    MpuNotEnabled,
    
    /// Address out of bounds
    AddressOutOfBounds,
}

/// Subregion error types
#[derive(Debug, Clone, Copy)]
pub enum SubregionErrorType {
    TooManySubregions,
    InvalidSubregion,
    SubregionBoundsError,
}

impl MpuManager {
    /// Create new MPU manager
    pub fn new() -> Self {
        Self {
            regions: [None; MPU_MAX_REGIONS],
            mpu_enabled: AtomicBool::new(false),
            background_update: AtomicBool::new(false),
            mpu_rnr: AtomicU8::new(0),
            mpu_rbar: AtomicU32::new(0),
            mpu_rlar: AtomicU64::new(0),
            mpu_rdr: AtomicU32::new(0),
            mpu_rcr: AtomicU32::new(0),
            total_violations: AtomicU32::new(0),
            last_update: AtomicU64::new(0),
        }
    }
    
    /// Enable MPU
    pub fn enable(&self) {
        self.mpu_enabled.store(true, Ordering::Release);
        
        // In real implementation, would write to MPU enable register
        crate::println!("[mpu] MPU enabled");
    }
    
    /// Disable MPU
    pub fn disable(&self) {
        self.mpu_enabled.store(false, Ordering::Release);
        
        // In real implementation, would write to MPU disable register
        crate::println!("[mpu] MPU disabled");
    }
    
    /// Check if MPU is enabled
    pub fn is_enabled(&self) -> bool {
        self.mpu_enabled.load(Ordering::Relaxed)
    }
    
    /// Configure MPU region
    pub fn configure_region(&self, region: MpuRegion) -> Result<(), MpuError> {
        // Check if MPU is enabled
        if !self.is_enabled() {
            return Err(MpuError::MpuNotEnabled);
        }
        
        // Check if region number is valid
        if region.region_number >= MPU_MAX_REGIONS as u8 {
            return Err(MpuError::RegionInUse { region_number: MPU_INVALID_REGION });
        }
        
        // Check if region slot is available
        if self.regions[region.region_number as usize].is_some() {
            return Err(MpuError::RegionInUse { region_number: region.region_number });
        }
        
        // Validate region configuration
        if !region.is_valid() {
            return Err(MpuError::InvalidRegionConfig);
        }
        
        // Check alignment
        if region.base_address & (MPU_REGION_ALIGNMENT - 1) != 0 {
            return Err(MpuError::AlignmentError);
        }
        
        // Insert region
        self.regions[region.region_number as usize] = Some(region);
        
        crate::println!("[mpu] Configured region {}: base={:#x}, size={}, permissions={:?}",
                        region.region_number, region.base_address, region.size, region.permissions);
        
        // In real implementation, would program MPU registers here
        
        Ok(())
    }
    
    /// Remove MPU region
    pub fn remove_region(&self, region_number: u8) -> Result<(), MpuError> {
        if region_number >= MPU_MAX_REGIONS as u8 {
            return Err(MpuError::InvalidRegionConfig);
        }
        
        self.regions[region_number as usize] = None;
        
        crate::println!("[mpu] Removed region {}", region_number);
        
        // In real implementation, would disable MPU region in hardware
        
        Ok(())
    }
    
    /// Check address against MPU regions
    pub fn check_address(&self, address: u64) -> MpuCheckResult {
        let mut result = MpuCheckResult {
            allowed: true,
            violating_region: MPU_INVALID_REGION,
            violation_type: MpuViolationType::None,
        };
        
        for i in 0..MPU_MAX_REGIONS {
            if let Some(region) = &self.regions[i] {
                if region.enabled && region.contains(address) {
                    result.allowed = false;
                    result.violating_region = region.region_number;
                    result.violation_type = match region.permissions {
                        MpuPermissions { read: false, .. } => MpuViolationType::ReadViolation,
                        MpuPermissions { write: false, .. } => MpuViolationType::WriteViolation,
                        MpuPermissions { execute: false, .. } => MpuViolationType::ExecuteViolation,
                        _ => MpuViolationType::PermissionViolation,
                    };
                    region.violation_count.fetch_add(1, Ordering::Relaxed);
                    
                    // Don't check other regions
                    break;
                }
            }
        }
        
        result
    }
    
    /// Enable background MPU updates
    pub fn enable_background_updates(&self) {
        self.background_update.store(true, Ordering::Release);
        crate::println!("[mpu] Background MPU updates enabled");
    }
    
    /// Disable background MPU updates
    pub fn disable_background_updates(&self) {
        self.background_update.store(false, Ordering::Release);
        crate::println!("[mpu] Background MPU updates disabled");
    }
    
    /// Get MPU statistics
    pub fn get_stats(&self) -> MpuStats {
        let mut active_regions = 0;
        let mut total_violations = 0;
        
        for i in 0..MPU_MAX_REGIONS {
            if let Some(region) = &self.regions[i] {
                if region.enabled {
                    active_regions += 1;
                    total_violations += region.violation_count.load(Ordering::Relaxed);
                }
            }
        }
        
        MpuStats {
            mpu_enabled: self.is_enabled(),
            active_regions,
            total_violations: self.total_violations.load(Ordering::Relaxed),
            last_update: self.last_update.load(Ordering::Relaxed),
        }
    }
    
    /// Update MPU registers (in background)
    pub fn update_mpu_registers(&self) {
        if !self.background_update.load(Ordering::Relaxed) {
            return;
        }
        
        // In real implementation, would iterate through all regions
        // and update MPU registers with their current configuration
        
        self.last_update.store(crate::subsystems::time::timestamp_nanos(), Ordering::Release);
    }
}

/// MPU check result
#[derive(Debug, Clone)]
pub struct MpuCheckResult {
    /// Access is allowed
    pub allowed: bool,
    
    /// Region that caused violation (if any)
    pub violating_region: u8,
    
    /// Type of violation
    pub violation_type: MpuViolationType,
}

/// MPU violation types
#[derive(Debug, Clone, Copy)]
pub enum MpuViolationType {
    /// No violation
    None,
    
    /// Read violation (read from read-only region)
    ReadViolation,
    
    /// Write violation (write to read-only region)
    WriteViolation,
    
    /// Execute violation (execute from non-executable region)
    ExecuteViolation,
    
    /// General permission violation
    PermissionViolation,
}

/// MPU statistics
#[derive(Debug, Clone, Copy)]
pub struct MpuStats {
    pub mpu_enabled: bool,
    pub active_regions: usize,
    pub total_violations: u32,
    pub last_update: u64,
}

// ============================================================================
// MPU Fault Handler
// ============================================================================

/// MPU fault context
#[derive(Debug, Clone)]
pub struct MpuFaultContext {
    /// Faulting address
    pub fault_address: u64,
    
    /// Access type (read/write/execute)
    pub access_type: MpuAccessType,
    
    /// Region that caused violation
    pub violating_region: u8,
    
    /// Timestamp of fault
    pub timestamp: u64,
    
    /// Process ID that caused fault
    pub process_id: usize,
}

/// MPU access types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpuAccessType {
    Read,
    Write,
    Execute,
}

/// MPU fault handler
pub struct MpuFaultHandler {
    // Empty struct - methods are in impl block
}

impl MpuFaultHandler {
    /// Handle MPU fault
    pub fn handle_fault(&self, context: MpuFaultContext) -> FaultAction {
        crate::println!("[mpu] MPU fault: addr={:#x}, access={:?}, region={}",
                        context.fault_address, context.access_type, 
                        context.violating_region);
        
        // Check if fault is due to subregion
        let region_opt = self.mpu().check_subregion(context.fault_address);
        
        match region_opt {
            Ok(subregion) => {
                // Check if access is allowed for subregion
                if subregion.enabled && subregion.permissions_allows(context.access_type) {
                    FaultAction::Continue
                } else {
                    FaultAction::Terminate { signal: 11 } // SIGSEGV
                }
            }
            Err(_) => {
                // Check if access is allowed for main region
                if let Some(region) = self.mpu().regions.get(context.violating_region as usize) {
                    if region.enabled && region.permissions_allows(context.access_type) {
                        FaultAction::Continue
                    } else {
                        FaultAction::Terminate { signal: 11 } // SIGSEGV
                    }
                } else {
                    // No region found - terminate
                    FaultAction::Terminate { signal: 11 } // SIGSEGV
                }
            }
        }
    }
    
    /// Check if permissions allow access
    fn mpu(&self) -> &'static MpuManager {
        // In real implementation, would return global MPU manager
        unimplemented!()
    }
}

/// Fault action
#[derive(Debug, Clone, Copy)]
pub enum FaultAction {
    /// Continue execution
    Continue,
    
    /// Terminate process with signal
    Terminate {
        signal: i32,
    },
    
    /// Deliver fault signal and restart
    Restart {
        signal: i32,
    },
}

impl MpuPermissions {
    /// Check if permissions allow specific access type
    fn permissions_allows(&self, access_type: MpuAccessType) -> bool {
        match access_type {
            MpuAccessType::Read => self.read,
            MpuAccessType::Write => self.write,
            MpuAccessType::Execute => self.execute,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpu_permissions() {
        let ro = MpuPermissions::readonly();
        let rw = MpuPermissions::readwrite();
        let rwx = MpuPermissions::readwriteexecute();
        
        assert!(ro.read);
        assert!(!ro.write);
        assert!(ro.execute);
        
        assert!(rw.read);
        assert!(rw.write);
        assert!(!rw.execute);
        
        assert!(rwx.read);
        assert!(rwx.write);
        assert!(rwx.execute);
    }

    #[test]
    fn test_mpu_region() {
        let region = MpuRegion::new(
            0, 0x1000_0000, 0x10000, MpuPermissions::readonly()
        );
        
        assert!(region.is_valid());
        assert_eq!(region.region_number, 0);
        assert_eq!(region.end_address(), 0x10010000);
        assert!(region.contains(0x10000000));
        assert!(region.contains(0x10001000));
        assert!(!region.contains(0x10010000));
    }

    #[test]
    fn test_mpu_subregion() {
        let parent = MpuRegion::new(
            0, 0x1000_0000, 0x10000, MpuPermissions::readwrite()
        );
        
        let sub = MpuSubregion::new(
            0, 0x1000_2000, 0x2000, MpuPermissions::readonly()
        );
        
        assert_eq!(parent.add_subregion(sub).is_ok(), ());
        assert_eq!(parent.num_subregions, 1);
    }

    #[test]
    fn test_mpu_manager() {
        let manager = MpuManager::new();
        
        assert!(manager.regions[0].is_none());
        assert!(!manager.is_enabled());
        assert_eq!(manager.total_violations.load(Ordering::Relaxed), 0);
    }
}
