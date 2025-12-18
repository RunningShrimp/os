//! Boot-time memory management (minimal stub for P0)

use crate::arch::Architecture;

pub mod bios;

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Free,
    Reserved,
}

/// Memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base: usize,
    pub size: usize,
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    pub fn new(base: usize, size: usize, region_type: MemoryRegionType) -> Self {
        Self {
            base,
            size,
            region_type,
        }
    }

    pub fn is_free(&self) -> bool {
        self.region_type == MemoryRegionType::Free
    }

    pub fn end(&self) -> usize {
        self.base + self.size
    }
}

/// Boot-time memory manager
pub struct BootMemoryManager;

impl BootMemoryManager {
    pub fn new(arch: Architecture) -> Self {
        log::debug!("Initializing boot memory manager for architecture: {:?}", arch);
        // Validate architecture before initializing memory manager
        match arch {
            Architecture::X86_64 => {
                log::info!("Initializing x86_64 memory manager");
            }
            Architecture::AArch64 => {
                log::info!("Initializing AArch64 memory manager");
            }
            Architecture::RiscV64 => {
                log::info!("Initializing RISC-V 64-bit memory manager");
            }
        }
        Self
    }
}
