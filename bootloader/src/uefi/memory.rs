//! UEFI Memory Management
//!
//! This module provides UEFI-specific memory management utilities,
//! including memory allocation, pool management, and memory map handling.

use crate::error::{BootError, Result};
use crate::memory::MemoryRegionType;
use core::ptr;
use alloc::vec::Vec;

#[cfg(feature = "uefi_support")]
use uefi::table::boot::{BootServices, MemoryType};

/// UEFI Memory Manager
#[cfg(feature = "uefi_support")]
pub struct UefiMemoryManager {
    boot_services: Option<&'static BootServices>,
    allocated_regions: Vec<MemoryAllocation>,
}

#[cfg(feature = "uefi_support")]
impl UefiMemoryManager {
    /// Create a new UEFI memory manager
    pub fn new() -> Self {
        Self {
            boot_services: None,
            allocated_regions: Vec::new(),
        }
    }

    /// Initialize with boot services
    pub fn initialize(&mut self, boot_services: &'static BootServices) -> Result<()> {
        self.boot_services = Some(boot_services);
        Ok(())
    }

    /// Allocate memory from UEFI pool
    pub fn allocate_pool(&self, memory_type: MemoryType, size: usize) -> Result<*mut u8> {
        let bs = self.boot_services.ok_or(BootError::UefiNotFound)?;

        let ptr = unsafe {
            bs.allocate_pool(memory_type, size)?
        };

        Ok(ptr as *mut u8)
    }

    /// Free memory to UEFI pool
    pub fn free_pool(&self, ptr: *mut u8) -> Result<()> {
        let bs = self.boot_services.ok_or(BootError::UefiNotFound)?;

        unsafe {
            bs.free_pool(ptr)?;
        }

        Ok(())
    }

    /// Allocate pages
    pub fn allocate_pages(&self, memory_type: MemoryType, pages: usize) -> Result<usize> {
        let bs = self.boot_services.ok_or(BootError::UefiNotFound)?;

        let address = unsafe {
            bs.allocate_pages(
                uefi::table::boot::AllocateType::AnyPages,
                memory_type,
                pages,
                0,
            )?
        };

        Ok(address.0)
    }

    /// Free pages
    pub fn free_pages(&self, address: usize, pages: usize) -> Result<()> {
        let bs = self.boot_services.ok_or(BootError::UefiNotFound)?;

        unsafe {
            bs.free_pages(
                address,
                pages,
            )?;
        }

        Ok(())
    }

    /// Get memory map from UEFI
    pub fn get_memory_map(&self) -> Result<uefi::table::boot::MemoryMap<'static>> {
        let bs = self.boot_services.ok_or(BootError::UefiNotFound)?;

        // First, get the memory map size
        let map_size = bs.memory_map_size()?;

        // Allocate buffer for memory map
        let buffer_size = map_size + 1024; // Add some padding
        let mut buffer = vec![0u8; buffer_size];

        // Get the actual memory map
        let memory_map = bs.memory_map(&mut buffer)?;

        Ok(memory_map)
    }

    /// Convert UEFI memory type to our MemoryRegionType
    pub fn convert_memory_type(efi_type: MemoryType) -> MemoryRegionType {
        match efi_type {
            MemoryType::RESERVED => MemoryRegionType::Reserved,
            MemoryType::LOADER_CODE => MemoryRegionType::BootloaderCode,
            MemoryType::LOADER_DATA => MemoryRegionType::BootloaderData,
            MemoryType::BOOT_SERVICES_CODE => MemoryRegionType::RuntimeCode,
            MemoryType::BOOT_SERVICES_DATA => MemoryRegionType::RuntimeData,
            MemoryType::CONVENTIONAL => MemoryRegionType::Free,
            MemoryType::UNUSABLE => MemoryRegionType::Unusable,
            MemoryType::ACPI_RECLAIM => MemoryRegionType::ACPI,
            MemoryType::ACPI_NON_VOLATILE => MemoryRegionType::ACPI,
            MemoryType::MMIO => MemoryRegionType::DeviceMemory,
            MemoryType::PAL_CODE => MemoryRegionType::BootloaderCode,
            MemoryType::PERSISTENT_MEMORY => MemoryRegionType::Free,
            _ => MemoryRegionType::Reserved,
        }
    }

    /// Check if memory type is available for allocation
    pub fn is_memory_available(efi_type: MemoryType) -> bool {
        matches!(efi_type,
            MemoryType::CONVENTIONAL |
            MemoryType::PERSISTENT_MEMORY |
            MemoryType::LOADER_CODE |
            MemoryType::LOADER_DATA
        )
    }

    /// Reserve memory region
    pub fn reserve_region(&mut self, base: usize, size: usize, region_type: MemoryRegionType) {
        let allocation = MemoryAllocation {
            base,
            size,
            region_type,
            is_pages: false,
        };

        self.allocated_regions.push(allocation);
    }

    /// Reserve pages
    pub fn reserve_pages(&mut self, base: usize, pages: usize, region_type: MemoryRegionType) {
        let allocation = MemoryAllocation {
            base,
            size: pages * 4096,
            region_type,
            is_pages: true,
        };

        self.allocated_regions.push(allocation);
    }

    /// Get allocated regions
    pub fn get_allocated_regions(&self) -> &[MemoryAllocation] {
        &self.allocated_regions
    }

    /// Find suitable memory region for allocation
    pub fn find_suitable_region(&self, size: usize, alignment: usize) -> Option<usize> {
        // This would search through available memory regions
        // For now, return a simple allocation
        None
    }

    /// Print memory statistics
    pub fn print_memory_stats(&self) -> Result<()> {
        let memory_map = self.get_memory_map()?;

        let mut total_memory = 0;
        let mut available_memory = 0;
        let mut reserved_memory = 0;

        for descriptor in memory_map {
            let descriptor_size = descriptor.page_count() * 4096;
            total_memory += descriptor_size;

            if Self::is_memory_available(descriptor.ty()) {
                available_memory += descriptor_size;
            } else {
                reserved_memory += descriptor_size;
            }
        }

        println!("[memory] Total Memory: {} MB", total_memory / (1024 * 1024));
        println!("[memory] Available Memory: {} MB", available_memory / (1024 * 1024));
        println!("[memory] Reserved Memory: {} MB", reserved_memory / (1024 * 1024));
        println!("[memory] Bootloader Allocations: {}", self.allocated_regions.len());

        Ok(())
    }
}

/// Memory allocation tracking
#[cfg(feature = "uefi_support")]
#[derive(Debug, Clone)]
pub struct MemoryAllocation {
    pub base: usize,
    pub size: usize,
    pub region_type: MemoryRegionType,
    pub is_pages: bool, // true if allocated with allocate_pages, false with allocate_pool
}

/// UEFI Memory Utilities
#[cfg(feature = "uefi_support")]
pub struct UefiMemoryUtils;

#[cfg(feature = "uefi_support")]
impl UefiMemoryUtils {
    /// Align address to page boundary
    pub fn align_to_page(address: usize) -> usize {
        (address + 4095) & !4095
    }

    /// Check if address is page-aligned
    pub fn is_page_aligned(address: usize) -> bool {
        address & 4095 == 0
    }

    /// Convert size to number of pages
    pub fn size_to_pages(size: usize) -> usize {
        (size + 4095) / 4096
    }

    /// Get recommended memory type for kernel loading
    pub fn kernel_memory_type() -> MemoryType {
        MemoryType::LOADER_DATA
    }

    /// Get recommended memory type for bootloader code
    pub fn bootloader_code_memory_type() -> MemoryType {
        MemoryType::LOADER_CODE
    }

    /// Get recommended memory type for runtime data
    pub fn runtime_memory_type() -> MemoryType {
        MemoryType::BOOT_SERVICES_DATA
    }

    /// Get recommended memory type for framebuffer
    pub fn framebuffer_memory_type() -> MemoryType {
        MemoryType::ACPI_RECLAIM
    }
}

/// Memory allocation strategy
#[cfg(feature = "uefi_support")]
pub enum AllocationStrategy {
    /// Use allocate_pages for large allocations
    Pages,
    /// Use allocate_pool for small allocations
    Pool,
    /// Choose automatically based on size
    Auto,
}

/// Memory allocation builder
#[cfg(feature = "uefi_support")]
pub struct MemoryAllocationBuilder {
    manager: Option<*const UefiMemoryManager>,
    memory_type: MemoryType,
    strategy: AllocationStrategy,
    alignment: usize,
}

#[cfg(feature = "uefi_support")]
impl MemoryAllocationBuilder {
    /// Create a new allocation builder
    pub fn new() -> Self {
        Self {
            manager: None,
            memory_type: MemoryType::LOADER_DATA,
            strategy: AllocationStrategy::Auto,
            alignment: 8,
        }
    }

    /// Set the memory manager
    pub fn manager(mut self, manager: &UefiMemoryManager) -> Self {
        self.manager = Some(manager);
        self
    }

    /// Set the memory type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = memory_type;
        self
    }

    /// Set the allocation strategy
    pub fn strategy(mut self, strategy: AllocationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the alignment
    pub fn alignment(mut self, alignment: usize) -> Self {
        self.alignment = alignment;
        self
    }

    /// Allocate memory
    pub fn allocate(self, size: usize) -> Result<*mut u8> {
        let manager = self.manager.ok_or(BootError::UefiNotFound)?;

        match self.strategy {
            AllocationStrategy::Pages => {
                let pages = UefiMemoryUtils::size_to_pages(size + self.alignment - 1);
                let address = unsafe { (*manager).allocate_pages(self.memory_type, pages)? };
                Ok(address as *mut u8)
            }
            AllocationStrategy::Pool => {
                let aligned_size = size + self.alignment - 1;
                unsafe { (*manager).allocate_pool(self.memory_type, aligned_size) }
            }
            AllocationStrategy::Auto => {
                if size >= 4096 {
                    let pages = UefiMemoryUtils::size_to_pages(size);
                    let address = unsafe { (*manager).allocate_pages(self.memory_type, pages)? };
                    Ok(address as *mut u8)
                } else {
                    unsafe { (*manager).allocate_pool(self.memory_type, size) }
                }
            }
        }
    }
}

/// Non-UEFI stub implementations
#[cfg(not(feature = "uefi_support"))]
pub struct UefiMemoryManager;

#[cfg(not(feature = "uefi_support"))]
impl UefiMemoryManager {
    pub fn new() -> Self {
        Self
    }

    pub fn initialize(&mut self, _boot_services: &()) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }

    pub fn allocate_pool(&self, _memory_type: (), _size: usize) -> Result<*mut u8> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }

    pub fn free_pool(&self, _ptr: *mut u8) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }

    pub fn allocate_pages(&self, _memory_type: (), _pages: usize) -> Result<usize> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }

    pub fn free_pages(&self, _address: usize, _pages: usize) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }

    pub fn get_memory_map(&self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }

    pub fn print_memory_stats(&self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }
}

#[cfg(not(feature = "uefi_support"))]
pub struct UefiMemoryUtils;

#[cfg(not(feature = "uefi_support"))]
impl UefiMemoryUtils {
    pub fn align_to_page(address: usize) -> usize {
        (address + 4095) & !4095
    }

    pub fn is_page_aligned(address: usize) -> bool {
        address & 4095 == 0
    }

    pub fn size_to_pages(size: usize) -> usize {
        (size + 4095) / 4096
    }
}

#[cfg(not(feature = "uefi_support"))]
pub enum AllocationStrategy {
    Pages,
    Pool,
    Auto,
}

#[cfg(not(feature = "uefi_support"))]
pub struct MemoryAllocationBuilder {
    _private: (),
}

#[cfg(not(feature = "uefi_support"))]
impl MemoryAllocationBuilder {
    pub fn new() -> Self {
        Self { _private: () }
    }

extern crate alloc;
    pub fn allocate(self, _size: usize) -> Result<*mut u8> {
        Err(BootError::FeatureNotEnabled("UEFI memory management"))
    }
}