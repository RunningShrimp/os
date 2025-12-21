extern crate alloc;
use alloc::vec::Vec;

// Boot-time memory management
//
// This module provides memory management facilities for the bootloader,
// including physical memory management, heap allocation, and memory map handling.

use crate::arch::Architecture;
use crate::error::{BootError, Result};
use core::ptr;

#[cfg(feature = "bios_support")]
pub mod bios;

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Free,
    Reserved,
    KernelCode,
    KernelData,
    BootloaderCode,
    BootloaderData,
    Framebuffer,
    ACPI,
    DeviceMemory,
    Unusable,
}

/// Memory region descriptor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Physical base address
    pub base: usize,
    /// Size in bytes
    pub size: usize,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Alignment requirement
    pub alignment: usize,
    /// Name for debugging
    pub name: &'static str,
}

impl MemoryRegion {
    /// Create a new memory region
    pub fn new(base: usize, size: usize, region_type: MemoryRegionType) -> Self {
        Self {
            base,
            size,
            region_type,
            alignment: Architecture::current().page_size(),
            name: "Unknown",
        }
    }

    /// Create a named memory region
    pub fn with_name(base: usize, size: usize, region_type: MemoryRegionType, name: &'static str) -> Self {
        Self {
            base,
            size,
            region_type,
            alignment: Architecture::current().page_size(),
            name,
        }
    }

    /// Check if this region is free
    pub fn is_free(&self) -> bool {
        self.region_type == MemoryRegionType::Free
    }

    /// Check if this region can be used for allocation
    pub fn is_usable(&self) -> bool {
        matches!(
            self.region_type,
            MemoryRegionType::Free | MemoryRegionType::BootloaderCode | MemoryRegionType::BootloaderData
        )
    }

    /// Get the end address (exclusive)
    pub fn end(&self) -> usize {
        self.base + self.size
    }

    /// Check if this region overlaps with another
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.base < other.end() && other.base < self.end()
    }

    /// Split this region at the given address
    pub fn split_at(&self, addr: usize) -> Option<(MemoryRegion, MemoryRegion)> {
        if addr <= self.base || addr >= self.end() {
            return None;
        }

        let left = MemoryRegion::with_name(
            self.base,
            addr - self.base,
            self.region_type,
            self.name,
        );

        let right = MemoryRegion::with_name(
            addr,
            self.end() - addr,
            self.region_type,
            self.name,
        );

        Some((left, right))
    }
}

/// Boot-time memory manager
pub struct BootMemoryManager {
    /// Architecture we're running on
    architecture: Architecture,
    /// Memory regions
    regions: Vec<MemoryRegion>,
    /// Simple heap allocator
    heap: SimpleHeap,
    /// Total physical memory
    total_memory: usize,
    /// Available memory
    available_memory: usize,
}

impl BootMemoryManager {
    /// Create a new memory manager
    pub fn new(architecture: Architecture) -> Result<Self> {
        let mut manager = Self {
            architecture,
            regions: Vec::new(),
            heap: SimpleHeap::new(),
            total_memory: 0,
            available_memory: 0,
        };

        // Initialize with firmware-provided memory map
        manager.initialize_memory_map()?;

        Ok(manager)
    }

    /// Initialize the memory map from firmware
    fn initialize_memory_map(&mut self) -> Result<()> {
        // In a real implementation, this would get the memory map from UEFI or BIOS
        // For now, create a simple default memory map

        let page_size = self.architecture.page_size();
        let kernel_base = self.architecture.default_kernel_base();

        // Create a basic memory map (this would be firmware-provided in reality)
        self.add_region(MemoryRegion::with_name(
            0,
            kernel_base,
            MemoryRegionType::Reserved,
            "Low Memory",
        ));

        // Add memory above kernel as free (simplified)
        self.add_region(MemoryRegion::with_name(
            kernel_base + 0x200000, // 2MB after kernel base
            0x10000000 - (kernel_base + 0x200000), // Up to 256MB
            MemoryRegionType::Free,
            "Free Memory",
        ));

        Ok(())
    }

    /// Add a memory region to the memory map
    pub fn add_region(&mut self, region: MemoryRegion) {
        self.total_memory += region.size;
        if region.is_free() {
            self.available_memory += region.size;
        }
        self.regions.push(region);
    }

    /// Allocate physical memory with the given size and alignment
    pub fn allocate_physical(&mut self, size: usize, alignment: usize) -> Result<usize> {
        let aligned_size = align_up(size, alignment);

        for region in &mut self.regions {
            if !region.is_usable() {
                continue;
            }

            let aligned_base = align_up(region.base, alignment);
            let aligned_end = aligned_base + aligned_size;

            // Check if the allocation fits in this region
            if aligned_end <= region.end() {
                // Mark the allocated region as used
                let allocated_region = MemoryRegion::with_name(
                    aligned_base,
                    aligned_size,
                    MemoryRegionType::BootloaderData,
                    "Allocated",
                );

                // Split the region and mark part as used
                if let Some((before, after)) = region.split_at(aligned_base) {
                    if let Some((allocated, after_allocated)) = after.split_at(aligned_size) {
                        // Replace the region with before + allocated + after_allocated
                        let region_index = self.regions.iter().position(|r| r == region).unwrap();
                        self.regions.remove(region_index);
                        self.regions.push(before);
                        self.regions.push(allocated_region);
                        self.regions.push(after_allocated);

                        self.available_memory -= aligned_size;
                        return Ok(aligned_base);
                    }
                }
            }
        }

        Err(BootError::OutOfMemory)
    }

    /// Allocate memory for kernel loading
    pub fn allocate_kernel_space(&mut self, size: usize) -> Result<usize> {
        let kernel_base = self.architecture.default_kernel_base();
        let alignment = self.architecture.page_size();

        // Try to allocate at the preferred kernel base
        for region in &mut self.regions {
            if !region.is_usable() {
                continue;
            }

            if region.base <= kernel_base && region.end() >= kernel_base + size {
                // This region contains our preferred kernel base
                self.available_memory -= size;
                return Ok(kernel_base);
            }
        }

        // Fallback: allocate anywhere
        self.allocate_physical(size, alignment)
    }

    /// Mark a memory region as reserved
    pub fn reserve_region(&mut self, base: usize, size: usize) -> Result<()> {
        let end = base + size;

        let mut i = 0;
        while i < self.regions.len() {
            let region = &self.regions[i];
            if region.overlaps(&MemoryRegion::new(base, size, MemoryRegionType::Reserved)) {
                if let Some((before, after)) = region.split_at(base) {
                    if let Some((reserved, after_reserved)) = after.split_at(size) {
                        // Replace with before + reserved + after_reserved
                        self.regions.remove(i);
                        self.regions.push(before);
                        self.regions.push(MemoryRegion::with_name(
                            base,
                            size,
                            MemoryRegionType::Reserved,
                            "Reserved",
                        ));
                        self.regions.push(after_reserved);

                        if region.is_free() {
                            self.available_memory -= size;
                        }
                        return Ok(());
                    }
                }
            }
            i += 1;
        }

        Err(BootError::MemoryMapError)
    }

    /// Allocate memory from the heap
    pub fn allocate(&mut self, size: usize, alignment: usize) -> Result<*mut u8> {
        // First try to allocate from physical memory
        let physical_addr = self.allocate_physical(size, alignment)?;
        let ptr = physical_addr as *mut u8;

        // Add to heap tracking
        self.heap.add_allocation(ptr, size);

        Ok(ptr)
    }

    /// Deallocate memory
    pub fn deallocate(&mut self, ptr: *mut u8) {
        if let Some(size) = self.heap.remove_allocation(ptr) {
            // Mark the memory as free again
            let addr = ptr as usize;
            self.free_physical(addr, size);
        }
    }

    /// Free physical memory
    fn free_physical(&mut self, base: usize, size: usize) {
        // Mark the region as free
        for region in &mut self.regions {
            if region.base == base && region.size == size {
                region.region_type = MemoryRegionType::Free;
                self.available_memory += size;
                return;
            }
        }

        // If we don't find an exact match, we need to handle this more carefully
        // For now, just update available memory count
        self.available_memory += size;
    }

    /// Get the memory map for passing to the kernel
    pub fn get_memory_map(&self) -> Vec<MemoryRegion> {
        self.regions.clone()
    }

    /// Get total memory size
    pub fn total_memory(&self) -> usize {
        self.total_memory
    }

    /// Get available memory size
    pub fn available_memory(&self) -> usize {
        self.available_memory
    }

    /// Find a suitable region for a given purpose
    pub fn find_suitable_region(&self, min_size: usize, alignment: usize) -> Option<&MemoryRegion> {
        self.regions.iter().find(|region| {
            region.is_free()
                && region.size >= min_size
                && (region.base & (alignment - 1)) == 0
        })
    }

    /// Validate the memory map
    pub fn validate_memory_map(&self) -> Result<()> {
        // Check for overlapping regions
        for (i, region1) in self.regions.iter().enumerate() {
            for region2 in self.regions.iter().skip(i + 1) {
                if region1.overlaps(region2) {
                    return Err(BootError::MemoryMapError);
                }
            }
        }

        // Check for reasonable totals
        if self.total_memory == 0 {
            return Err(BootError::MemoryMapError);
        }

        Ok(())
    }
}

/// Simple heap tracking for the bootloader
struct SimpleHeap {
    allocations: Vec<(*mut u8, usize)>,
}

impl SimpleHeap {
    fn new() -> Self {
        Self {
            allocations: Vec::new(),
        }
    }

    fn add_allocation(&mut self, ptr: *mut u8, size: usize) {
        self.allocations.push((ptr, size));
    }

    fn remove_allocation(&mut self, ptr: *mut u8) -> Option<usize> {
        if let Some(pos) = self.allocations.iter().position(|&(p, _)| p == ptr) {
            let (_, size) = self.allocations.remove(pos);
            Some(size)
        } else {
            None
        }
    }
}

/// Memory allocation utilities
pub fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

pub fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

pub fn is_aligned(value: usize, alignment: usize) -> bool {
    (value & (alignment - 1)) == 0
}

/// Convert memory region type to protocol memory type
pub fn region_type_to_protocol_type(region_type: MemoryRegionType) -> crate::protocol::MemoryType {
    match region_type {
        MemoryRegionType::Free => crate::protocol::MemoryType::Usable,
        MemoryRegionType::Reserved => crate::protocol::MemoryType::Reserved,
        MemoryRegionType::KernelCode => crate::protocol::MemoryType::RuntimeCode,
        MemoryRegionType::KernelData => crate::protocol::MemoryType::RuntimeData,
        MemoryRegionType::BootloaderCode => crate::protocol::MemoryType::BootloaderCode,
        MemoryRegionType::BootloaderData => crate::protocol::MemoryType::BootloaderData,
        MemoryRegionType::Framebuffer => crate::protocol::MemoryType::UnconventionalMemory,
        MemoryRegionType::ACPI => crate::protocol::MemoryType::ACPIReclaimable,
        MemoryRegionType::DeviceMemory => crate::protocol::MemoryType::Reserved,
        MemoryRegionType::Unusable => crate::protocol::MemoryType::BadMemory,
    }
}

/// Global memory allocator helper
#[macro_export]
macro_rules! allocate {
    ($manager:expr, $size:expr) => {
        $manager.allocate($size, 8)
    };
}

#[macro_export]
macro_rules! allocate_aligned {
    ($manager:expr, $size:expr, $align:expr) => {
        $manager.allocate($size, $align)
    };
}

#[macro_export]
macro_rules! deallocate {
    ($manager:expr, $ptr:expr) => {
        $manager.deallocate($ptr)
    };
}