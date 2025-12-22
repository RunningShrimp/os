//! Boot Information - Aggregate Root Entity
//!
//! Represents the complete bootloader state at handoff to kernel.
//! This is an aggregate root - the consistency boundary for boot state.
//!
//! Contains all state needed for kernel initialization:
//! - Complete memory map from firmware
//! - Kernel image location and size
//! - Graphics framebuffer details
//! - Boot parameters and timing information
//!
//! # Examples
//!
//! ```no_run
//! # use nos_bootloader::domain::boot_info::BootInfo;
//! # use nos_bootloader::protocol::BootProtocolType;
//! let boot_info = BootInfo::new(BootProtocolType::Uefi);
//! assert!(boot_info.validate().is_ok());
//! ```

use crate::protocol::BootProtocolType;
use crate::domain::boot_config::{KernelInfo, MemoryRegion, GraphicsInfo, BootPhase};
use crate::domain::repositories::{EntityId, RepositoryError};
use crate::domain::AggregateRoot;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;

/// Boot Information - Aggregate Root
///
/// Contains all information needed to hand off control to the kernel.
/// This is the root entity of the boot aggregate in DDD terms.
///
/// # Invariants
/// - Kernel information must be valid and consistent
/// - Memory regions must not overlap
/// - Graphics information must be consistent with memory regions
/// - Boot phase must follow valid transition rules
/// - All entities must maintain consistency within the aggregate
///
/// # Validation
/// The `validate()` method checks:
/// - Kernel information validity
/// - Memory region consistency
/// - Graphics information consistency
/// - Boot phase validity
/// - Aggregate-level invariants
#[derive(Debug, Clone)]
pub struct BootInfo {
    /// Unique identifier for this boot information
    pub id: EntityId,
    /// Boot protocol used (UEFI/BIOS/Multiboot2)
    pub protocol_type: BootProtocolType,
    /// Kernel information entity
    pub kernel_info: Option<KernelInfo>,
    /// Memory region entities
    pub memory_regions: Vec<MemoryRegion>,
    /// Graphics information entity
    pub graphics_info: Option<GraphicsInfo>,
    /// Boot timestamp (cycles or milliseconds from firmware)
    pub boot_timestamp: u64,
    /// Current boot phase
    pub current_phase: BootPhase,
}

impl BootInfo {
    /// Create new boot information
    pub fn new(protocol_type: BootProtocolType) -> Self {
        Self {
            id: EntityId::new(0),
            protocol_type,
            kernel_info: None,
            memory_regions: Vec::new(),
            graphics_info: None,
            boot_timestamp: 0,
            current_phase: BootPhase::Initialization,
        }
    }
    
    /// Factory method: Create from configuration
    pub fn from_config(
        config: &crate::domain::boot_config::BootConfig,
        protocol_type: BootProtocolType
    ) -> Result<Self, &'static str> {
        let mut boot_info = Self::new(protocol_type);
        
        // Validate configuration
        config.validate()?;
        
        // Set graphics info if enabled
        if let Some(mode) = config.graphics_mode {
            // This would be handled by a domain service in a real implementation
            boot_info.graphics_info = Some(GraphicsInfo::new(mode, 0, 0)?);
        }
        
        Ok(boot_info)
    }
    
    /// Set kernel information
    pub fn set_kernel_info(&mut self, kernel_info: KernelInfo) -> Result<(), &'static str> {
        // Validate kernel info
        if !kernel_info.is_valid() {
            return Err("Invalid kernel information");
        }
        
        // Check if kernel conflicts with memory regions
        for region in &self.memory_regions {
            if region.overlaps(&MemoryRegion {
                id: 0, // Temporary ID for overlapping check
                start: kernel_info.address,
                end: kernel_info.end_address(),
                region_type: crate::domain::boot_config::MemoryRegionType::Available,
            }) && !region.is_available() {
                return Err("Kernel memory conflicts with reserved memory region");
            }
        }
        
        self.kernel_info = Some(kernel_info);
        Ok(())
    }
    
    /// Add memory region
    pub fn add_memory_region(&mut self, region: MemoryRegion) -> Result<(), &'static str> {
        // Check for overlaps with existing regions
        for existing in &self.memory_regions {
            if region.overlaps(existing) {
                return Err("Memory region overlaps with existing region");
            }
        }
        
        self.memory_regions.push(region);
        Ok(())
    }
    
    /// Set graphics information
    pub fn set_graphics_info(&mut self, graphics_info: GraphicsInfo) -> Result<(), &'static str> {
        if !graphics_info.is_valid() {
            return Err("Invalid graphics information");
        }
        
        self.graphics_info = Some(graphics_info);
        Ok(())
    }
    
    /// Advance boot phase
    pub fn advance_phase(&mut self, new_phase: BootPhase) -> Result<(), &'static str> {
        if !self.current_phase.can_transition_to(&new_phase) {
            return Err("Invalid boot phase transition");
        }
        
        self.current_phase = new_phase;
        Ok(())
    }
    
    /// Validate boot information completeness and consistency
    pub fn inner_validate(&self) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // Check kernel information
        if self.kernel_info.is_none() {
            errors.push("Kernel information is required");
        }
        
        // Check memory regions
        if self.memory_regions.is_empty() {
            errors.push("At least one memory region is required");
        }
        
        // Validate graphics if present
        if let Some(ref graphics) = self.graphics_info {
            if !graphics.is_valid() {
                errors.push("Invalid graphics information");
            }
        }
        
        // Check boot phase
        if self.current_phase == BootPhase::Initialization {
            errors.push("Boot phase cannot be in initialization state");
        }
        
        // Validate aggregate invariants
        if let Some(ref kernel) = self.kernel_info {
            // Check kernel memory conflicts
            for region in &self.memory_regions {
                if region.overlaps(&MemoryRegion {
                    id: 0, // Temporary ID for overlapping check
                    start: kernel.address,
                    end: kernel.end_address(),
                    region_type: crate::domain::boot_config::MemoryRegionType::Available,
                }) && !region.is_available() {
                    errors.push("Kernel memory conflicts with reserved memory region");
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Check if system is ready to boot kernel
    pub fn is_ready_for_kernel(&self) -> bool {
        self.kernel_info.is_some() &&
        self.current_phase == BootPhase::KernelLoadComplete &&
        self.inner_validate().is_ok()
    }
    
    /// Get total available memory
    pub fn total_available_memory(&self) -> u64 {
        self.memory_regions
            .iter()
            .filter(|region| region.is_available())
            .map(|region| region.size())
            .sum()
    }
    
    /// Check if graphics output is enabled
    pub fn has_graphics(&self) -> bool {
        self.graphics_info.is_some()
    }
    
    /// Get framebuffer size in bytes
    pub fn framebuffer_size(&self) -> usize {
        if let Some(ref graphics) = self.graphics_info {
            graphics.total_framebuffer_size()
        } else {
            0
        }
    }
    
    /// Get command line as slice
    pub fn get_cmdline(&self) -> &[u8] {
        if let Some(ref kernel) = self.kernel_info {
            kernel.get_cmdline()
        } else {
            &[]
        }
    }
    
    /// Get kernel address
    pub fn kernel_address(&self) -> Option<u64> {
        self.kernel_info.as_ref().map(|k| k.address)
    }
    
    /// Get kernel size
    pub fn kernel_size(&self) -> Option<u64> {
        self.kernel_info.as_ref().map(|k| k.size)
    }
    
    /// Get kernel entry point
    pub fn kernel_entry_point(&self) -> Option<u64> {
        self.kernel_info.as_ref().map(|k| k.entry_point)
    }
}

impl fmt::Display for BootInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Boot Information (ID: {})", self.id)?;
        writeln!(f, "  Protocol: {:?}", self.protocol_type)?;
        writeln!(f, "  Boot Phase: {}", self.current_phase)?;
        
        if let Some(ref kernel) = self.kernel_info {
            writeln!(f, "  Kernel: {:#x} ({} bytes)", kernel.address, kernel.size)?;
            writeln!(f, "  Entry Point: {:#x}", kernel.entry_point)?;
            if kernel.signature_verified {
                writeln!(f, "  Signature: Verified")?;
            }
        } else {
            writeln!(f, "  Kernel: Not loaded")?;
        }

        if let Some(ref graphics) = self.graphics_info {
            writeln!(f, "  Graphics: {}x{}x{}",
                     graphics.mode.width, graphics.mode.height, graphics.mode.bits_per_pixel)?;
            writeln!(f, "  Framebuffer: {:#x} ({} bytes)",
                     graphics.framebuffer_address, graphics.framebuffer_size)?;
        } else {
            writeln!(f, "  Graphics: Disabled")?;
        }

        writeln!(f, "  Memory Regions: {}", self.memory_regions.len())?;
        writeln!(f, "  Available Memory: {} MB",
                 self.total_available_memory() / (1024 * 1024))?;
        writeln!(f, "  Timestamp: {} ms", self.boot_timestamp)?;

        Ok(())
    }
}

impl AggregateRoot for BootInfo {
    fn clone_aggregate(&self) -> Box<dyn AggregateRoot> {
        Box::new(self.clone())
    }
    
    fn id(&self) -> EntityId {
        self.id
    }

    fn set_id(&mut self, id: EntityId) {
        self.id = id;
    }

    fn validate(&self) -> Result<(), RepositoryError> {
        // Check kernel information
        if self.kernel_info.is_none() {
            return Err(RepositoryError::InvalidEntity("Kernel information is required"));
        }
        
        // Check memory regions
        if self.memory_regions.is_empty() {
            return Err(RepositoryError::InvalidEntity("At least one memory region is required"));
        }
        
        // Validate graphics if present
        if let Some(ref graphics) = self.graphics_info {
            if !graphics.is_valid() {
                return Err(RepositoryError::InvalidEntity("Invalid graphics information"));
            }
        }
        
        // Check boot phase
        if self.current_phase == BootPhase::Initialization {
            return Err(RepositoryError::InvalidEntity("Boot phase cannot be in initialization state"));
        }
        
        // Validate aggregate invariants
        if let Some(ref kernel) = self.kernel_info {
            // Check kernel memory conflicts
            for region in &self.memory_regions {
                if region.overlaps(&MemoryRegion {
                    id: 0, // Temporary ID for overlapping check
                    start: kernel.address,
                    end: kernel.end_address(),
                    region_type: crate::domain::boot_config::MemoryRegionType::Available,
                }) && !region.is_available() {
                    return Err(RepositoryError::InvalidEntity("Kernel memory conflicts with reserved memory region"));
                }
            }
        }

        Ok(())
    }

    fn entity_type() -> &'static str {
        "BootInfo"
    }

    fn entity_type_dyn(&self) -> &'static str {
        Self::entity_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::boot_config::{KernelInfo, MemoryRegion, MemoryRegionType};

    #[test]
    fn test_boot_info_validation_missing_kernel() {
        let info = BootInfo::new(BootProtocolType::Bios);
        assert!(info.inner_validate().is_err());
    }

    #[test]
    fn test_boot_info_validation_valid() {
        let mut info = BootInfo::new(BootProtocolType::Uefi);
        let kernel_info = KernelInfo::new(0x100000, 0x500000, 0x100000).unwrap();
        info.set_kernel_info(kernel_info).unwrap();
        
        let mem_region = MemoryRegion::new(0, 0x1000000, MemoryRegionType::Available).unwrap();
        info.add_memory_region(mem_region).unwrap();
        
        info.advance_phase(BootPhase::KernelLoadComplete).unwrap();
        assert!(info.inner_validate().is_ok());
    }

    #[test]
    fn test_boot_info_memory_region_overlap() {
        let mut info = BootInfo::new(BootProtocolType::Bios);
        
        let region1 = MemoryRegion::new(0x100000, 0x200000, MemoryRegionType::Available).unwrap();
        let region2 = MemoryRegion::new(0x150000, 0x250000, MemoryRegionType::Reserved).unwrap();
        
        info.add_memory_region(region1).unwrap();
        assert!(info.add_memory_region(region2).is_err()); // Overlapping regions
    }

    #[test]
    fn test_boot_info_kernel_memory_conflict() {
        let mut info = BootInfo::new(BootProtocolType::Uefi);
        
        // Add a reserved memory region
        let reserved_region = MemoryRegion::new(0x100000, 0x200000, MemoryRegionType::Reserved).unwrap();
        info.add_memory_region(reserved_region).unwrap();
        
        // Try to set kernel that conflicts with reserved region
        let kernel_info = KernelInfo::new(0x150000, 0x50000, 0x150000).unwrap();
        assert!(info.set_kernel_info(kernel_info).is_err());
    }

    #[test]
    fn test_boot_info_phase_transitions() {
        let mut info = BootInfo::new(BootProtocolType::Uefi);
        
        // Valid transitions
        assert!(info.advance_phase(BootPhase::HardwareDetection).is_ok());
        assert!(info.advance_phase(BootPhase::MemoryInitialization).is_ok());
        assert!(info.advance_phase(BootPhase::KernelLoading).is_ok());
        assert!(info.advance_phase(BootPhase::KernelLoadComplete).is_ok());
        assert!(info.advance_phase(BootPhase::ReadyForKernel).is_ok());
        
        // Invalid transition
        assert!(info.advance_phase(BootPhase::Initialization).is_err());
    }

    #[test]
    fn test_boot_info_total_available_memory() {
        let mut info = BootInfo::new(BootProtocolType::Uefi);
        
        let region1 = MemoryRegion::new(0, 0x1000000, MemoryRegionType::Available).unwrap();
        let region2 = MemoryRegion::new(0x1000000, 0x2000000, MemoryRegionType::Reserved).unwrap();
        let region3 = MemoryRegion::new(0x2000000, 0x3000000, MemoryRegionType::Available).unwrap();
        
        info.add_memory_region(region1).unwrap();
        info.add_memory_region(region2).unwrap();
        info.add_memory_region(region3).unwrap();
        
        // Should only count available regions
        assert_eq!(info.total_available_memory(), 0x2000000); // 32MB
    }
}
