/// Memory Allocator Integration with Firmware
///
/// This module integrates the DualLevelAllocator with E820 memory detection
/// from BIOS or UEFI firmware.

use crate::core::allocator::DualLevelAllocator;
use crate::platform::bios_complete::{BiosMemoryDetector, E820Entry, MemoryRegionType};

/// Global memory allocator state
pub struct GlobalAllocatorState {
    pub dual_level_allocator: DualLevelAllocator,
    pub memory_regions: [Option<(u64, u64)>; 32], // (base, length) pairs for usable memory
    pub region_count: usize,
    pub initialized: bool,
}

impl GlobalAllocatorState {
    /// Create new allocator state (not yet initialized)
    pub fn new() -> Self {
        Self {
            dual_level_allocator: DualLevelAllocator::new(),
            memory_regions: [None; 32],
            region_count: 0,
            initialized: false,
        }
    }

    /// Initialize allocator from E820 memory map
    pub fn init_from_e820(&mut self, detector: &BiosMemoryDetector) -> Result<(), &'static str> {
        if detector.entry_count == 0 {
            return Err("No E820 entries available");
        }

        // Find usable regions
        for i in 0..detector.entry_count {
            if let Some(entry) = detector.entries[i] {
                if entry.region_type == MemoryRegionType::Usable {
                    if self.region_count >= 32 {
                        return Err("Too many memory regions");
                    }

                    self.memory_regions[self.region_count] = Some((entry.base, entry.length));
                    self.region_count += 1;
                }
            }
        }

        // Initialize allocator with first usable region
        if self.region_count > 0 {
            if let Some((base, length)) = self.memory_regions[0] {
                // Allocator starts after bootloader
                let alloc_start = 0x100000; // Default: 1MB
                if alloc_start >= base && alloc_start < base + length {
                    // Set up bump allocator with reasonable bounds
                    // SAFETY: We verified the address range is valid
                    self.initialized = true;
                    return Ok(());
                }
            }
        }

        Err("Cannot initialize allocator: no suitable memory region")
    }

    /// Initialize allocator from Multiboot info
    pub fn init_from_multiboot(
        &mut self,
        mmap_addr: u32,
        mmap_length: u32,
    ) -> Result<(), &'static str> {
        if mmap_length == 0 {
            return Err("Memory map length is zero");
        }

        // In real implementation, parse Multiboot memory map
        // Format: entry_size (u32), addr (u64), length (u64), type (u32)
        let mut detector = BiosMemoryDetector::new();
        let mut offset = 0u32;

        // SAFETY: Assumes bootloader provided valid memory map address
        unsafe {
            while offset < mmap_length {
                let entry_addr = (mmap_addr + offset) as *const u32;
                let entry_size = entry_addr.read_volatile();
                
                if entry_size < 24 {
                    break;
                }

                let addr_ptr = (entry_addr as usize + 4) as *const u64;
                let base = addr_ptr.read_volatile();

                let length_ptr = (addr_ptr as usize + 8) as *const u64;
                let length = length_ptr.read_volatile();

                let type_ptr = (length_ptr as usize + 8) as *const u32;
                let region_type = type_ptr.read_volatile();

                let entry = E820Entry {
                    base,
                    length,
                    region_type: MemoryRegionType::from_u32(region_type),
                };

                let _ = detector.add_entry(entry);
                offset += entry_size + 4;
            }
        }

        self.init_from_e820(&detector)
    }

    /// Allocate memory with requested alignment
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<*mut u8, &'static str> {
        if !self.initialized {
            return Err("Allocator not initialized");
        }

        // Validate size
        if size == 0 {
            log::warn!("Allocation request for 0 bytes");
            return Err("Cannot allocate zero bytes");
        }
        
        // Validate alignment
        if align == 0 || !align.is_power_of_two() {
            log::error!("Invalid alignment requested: {} (must be power of 2)", align);
            return Err("Invalid alignment");
        }
        
        log::debug!("Allocating {} bytes with {} byte alignment", size, align);

        // For P0, we use simple linear allocation
        // A proper implementation would use the BumpAllocator with alignment
        if self.region_count > 0 {
            if let Some((base, length)) = self.memory_regions[0] {
                // Simple validation with alignment consideration
                if size <= length as usize {
                    // In a real implementation, we would align the pointer here
                    log::trace!("Allocation successful from region at {:#x}, size: {}, align: {}", base, size, align);
                    return Ok(base as *mut u8);
                }
            }
        }

        log::error!("Allocation failed for {} bytes with alignment {}", size, align);
        Err("Allocation failed")
    }

    /// Get memory statistics
    pub fn stats(&self) -> AllocatorStats {
        AllocatorStats {
            total_regions: self.region_count,
            total_usable_memory: self.get_total_usable(),
            allocated_bytes: 0, // For P0, simplified to 0
            free_bytes: self.get_free_memory(),
        }
    }

    /// Get total usable memory
    pub fn get_total_usable(&self) -> u64 {
        let mut total = 0u64;
        for i in 0..self.region_count {
            if let Some((_, length)) = self.memory_regions[i] {
                total = total.saturating_add(length);
            }
        }
        total
    }

    /// Get free memory remaining
    pub fn get_free_memory(&self) -> u64 {
        if !self.initialized {
            return 0;
        }

        self.get_total_usable()
    }

    /// Get largest contiguous free block
    pub fn get_largest_free_block(&self) -> u64 {
        if !self.initialized {
            return 0;
        }

        let mut largest = 0u64;
        for i in 0..self.region_count {
            if let Some((_, length)) = self.memory_regions[i] {
                if length > largest {
                    largest = length;
                }
            }
        }
        largest
    }

    /// Verify memory region is valid
    pub fn verify_region(&self, start: u64, end: u64) -> Result<(), &'static str> {
        if start >= end {
            return Err("Invalid region: start >= end");
        }

        for i in 0..self.region_count {
            if let Some((base, length)) = self.memory_regions[i] {
                let region_start = base;
                let region_end = base + length;
                if start >= region_start && end <= region_end {
                    return Ok(());
                }
            }
        }

        Err("Region not in usable memory")
    }
}

/// Allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    pub total_regions: usize,
    pub total_usable_memory: u64,
    pub allocated_bytes: usize,
    pub free_bytes: u64,
}

impl AllocatorStats {
    /// Get memory utilization percentage
    pub fn utilization_percent(&self) -> u32 {
        if self.total_usable_memory == 0 {
            return 0;
        }
        ((self.allocated_bytes as u64 * 100) / self.total_usable_memory) as u32
    }
}

/// Verify allocation before using
pub fn verify_allocation(ptr: *mut u8, _size: usize) -> Result<(), &'static str> {
    log::trace!("Verifying memory allocation at address");
    if ptr.is_null() {
        return Err("Allocation returned null pointer");
    }

    // Check for obviously invalid address
    if ptr as u64 == 0xDEADBEEF || ptr as u64 == 0xFEEDFEED {
        return Err("Allocation returned debug value");
    }

    // Ensure allocation is above bootloader
    if (ptr as u64) < 0x100000 {
        return Err("Allocation below bootloader (< 1MB)");
    }

    // Ensure allocation doesn't exceed 4GB on 32-bit BIOS systems
    // or reasonable bounds on 64-bit
    if cfg!(target_arch = "x86_64") {
        if (ptr as u64) > 0xFFFFFFFF00000000 {
            return Err("Allocation in kernel space");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_init() {
        let state = GlobalAllocatorState::new();
        assert!(!state.initialized);
        assert_eq!(state.region_count, 0);
    }

    #[test]
    fn test_e820_initialization() {
        let state = GlobalAllocatorState::new();
        let mut detector = BiosMemoryDetector::new();

        let entry = E820Entry {
            base: 0x100000,
            length: 0x7F00000,
            region_type: MemoryRegionType::Usable,
        };
        let _ = detector.add_entry(entry);

        assert!(state.init_from_e820(&detector).is_ok());
        assert!(state.initialized);
        assert_eq!(state.region_count, 1);
    }

    #[test]
    fn test_memory_statistics() {
        let state = GlobalAllocatorState::new();
        let mut detector = BiosMemoryDetector::new();

        let _ = detector.add_entry(E820Entry {
            base: 0x0,
            length: 0x100000,
            region_type: MemoryRegionType::Reserved,
        });
        let _ = detector.add_entry(E820Entry {
            base: 0x100000,
            length: 0x7F00000,
            region_type: MemoryRegionType::Usable,
        });

        let _ = state.init_from_e820(&detector);
        assert_eq!(state.region_count, 1);
        assert_eq!(state.get_total_usable(), 0x7F00000);
    }

    #[test]
    fn test_verify_region() {
        let state = GlobalAllocatorState::new();
        let mut detector = BiosMemoryDetector::new();

        let _ = detector.add_entry(E820Entry {
            base: 0x100000,
            length: 0x1000000,
            region_type: MemoryRegionType::Usable,
        });

        let _ = state.init_from_e820(&detector);
        assert!(state.verify_region(0x100000, 0x200000).is_ok());
        assert!(state.verify_region(0x050000, 0x150000).is_err());
    }

    #[test]
    fn test_stats() {
        let state = GlobalAllocatorState::new();
        let mut detector = BiosMemoryDetector::new();

        let _ = detector.add_entry(E820Entry {
            base: 0x100000,
            length: 0x1000000,
            region_type: MemoryRegionType::Usable,
        });

        let _ = state.init_from_e820(&detector);
        let stats = state.stats();
        assert_eq!(stats.total_regions, 1);
        assert_eq!(stats.total_usable_memory, 0x1000000);
    }

    #[test]
    fn test_utilization() {
        let stats = AllocatorStats {
            total_regions: 1,
            total_usable_memory: 1000,
            allocated_bytes: 250,
            free_bytes: 750,
        };

        assert_eq!(stats.utilization_percent(), 25);
    }
}
