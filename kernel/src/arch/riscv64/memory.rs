//! RISC-V 64 memory management implementation
//!
//! This module provides RISC-V 64-specific memory management mechanisms,
//! including integration with the paging system and memory allocation.

use crate::arch::riscv64::paging::{PageTable, PageTableFlags, enable_paging, make_page_table, flush_tlb_all};

/// Kernel page table root (will be initialized during boot)
static mut KERNEL_PAGE_TABLE: *mut PageTable = core::ptr::null_mut();

/// Initialize RISC-V 64 memory management
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("riscv64: Initializing memory management");

    // Create kernel page table
    let page_table = make_page_table();

    // Store the page table pointer
    unsafe {
        KERNEL_PAGE_TABLE = page_table;
    }

    // Map kernel memory regions
    // In production, this would map:
    // - Kernel code (executable, read-only)
    // - Kernel data (read-write)
    // - Kernel heap (read-write)
    // - Device memory (uncached, device attributes)
    // - Stack regions

    crate::println!("riscv64: Setting up kernel page tables");

    // Enable paging with kernel page table
    // Use ASID 0 for kernel
    let pt_pa = page_table as u64;
    enable_paging(pt_pa, 0);

    crate::println!("riscv64: Paging enabled, Sv48 mode active");

    Ok(())
}

/// Shutdown RISC-V 64 memory management
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("riscv64: Shutting down memory management");

    // Flush all TLB entries before shutdown
    flush_tlb_all();

    Ok(())
}

/// Get kernel page table
pub fn get_kernel_page_table() -> Option<*mut PageTable> {
    let pt = unsafe { KERNEL_PAGE_TABLE };
    if pt.is_null() {
        None
    } else {
        Some(pt)
    }
}

/// Map kernel memory region
pub fn map_kernel_region(va: usize, pa: usize, size: usize, flags: PageTableFlags) {
    if let Some(pt) = get_kernel_page_table() {
        unsafe {
            let pt_ref = &mut *pt;
            let num_pages = (size + 4095) / 4096; // Round up to page boundary
            pt_ref.map_range(va, pa, num_pages, flags);
        }
    }
}

/// Unmap kernel memory region
pub fn unmap_kernel_region(va: usize, size: usize) {
    if let Some(pt) = get_kernel_page_table() {
        unsafe {
            let pt_ref = &mut *pt;
            let num_pages = (size + 4095) / 4096;
            pt_ref.unmap_range(va, num_pages);
        }
    }
}

/// Memory region types for kernel mapping
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Kernel code segment
    Code,
    /// Kernel read-only data
    ReadOnlyData,
    /// Kernel read-write data
    ReadWriteData,
    /// Kernel heap
    Heap,
    /// Kernel stack
    Stack,
    /// Device memory (MMIO)
    Device,
    /// DMA memory
    DMA,
}

impl MemoryRegionType {
    /// Get page table flags for this region type
    pub fn flags(&self) -> PageTableFlags {
        match self {
            MemoryRegionType::Code => {
                PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Execute
            }
            MemoryRegionType::ReadOnlyData => {
                PageTableFlags::Valid | PageTableFlags::Read
            }
            MemoryRegionType::ReadWriteData => {
                PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Write
            }
            MemoryRegionType::Heap => {
                PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Write
            }
            MemoryRegionType::Stack => {
                PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Write
            }
            MemoryRegionType::Device => {
                PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Write
            }
            MemoryRegionType::DMA => {
                PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Write
            }
        }
    }
}

/// Initialize kernel memory layout
pub fn init_kernel_memory_layout() {
    // Map kernel code (typically at 0x8000_0000 or higher)
    // In production, these addresses would come from the linker script

    let kernel_start = 0x8000_0000usize;
    let kernel_size = 0x1000_0000; // 256 MB

    map_kernel_region(
        kernel_start,
        kernel_start, // Identity mapping for kernel
        kernel_size,
        PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Execute,
    );

    crate::println!("riscv64: Mapped kernel region {:#x} - {:#x}",
        kernel_start, kernel_start + kernel_size);
}

/// Physical memory management
pub mod physical {
    /// Physical frame allocator
    ///
    /// Manages allocation of 4KB physical frames
    pub struct FrameAllocator {
        /// Bitmap of free frames
        free_map: *mut u8,
        /// Total number of frames
        total_frames: usize,
        /// Base physical address
        base_addr: usize,
    }

    impl FrameAllocator {
        /// Create new frame allocator
        pub fn new(base_addr: usize, size: usize) -> Self {
            let total_frames = size / 4096;
            let bitmap_size = (total_frames + 7) / 8;

            // Allocate bitmap (simplified - in production use proper allocator)
            let free_map = alloc::vec::Vec::from_iter(core::iter::repeat(0u8).take(bitmap_size))
                .leak()
                .as_mut_ptr();

            Self {
                free_map,
                total_frames,
                base_addr,
            }
        }

        /// Allocate a frame
        pub fn allocate_frame(&self) -> Option<usize> {
            // Find first free frame
            for i in 0..self.total_frames {
                let byte_idx = i / 8;
                let bit_idx = i % 8;

                unsafe {
                    let byte = *self.free_map.add(byte_idx);
                    if byte & (1 << bit_idx) == 0 {
                        // Frame is free, mark as used
                        *(self.free_map.add(byte_idx)) |= 1 << bit_idx;
                        return Some(self.base_addr + (i * 4096));
                    }
                }
            }
            None
        }

        /// Free a frame
        pub fn free_frame(&self, frame_addr: usize) {
            let offset = frame_addr - self.base_addr;
            let frame_idx = offset / 4096;

            if frame_idx < self.total_frames {
                let byte_idx = frame_idx / 8;
                let bit_idx = frame_idx % 8;

                unsafe {
                    *(self.free_map.add(byte_idx)) &= !(1 << bit_idx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_flags() {
        assert_eq!(
            MemoryRegionType::Code.flags(),
            PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Execute
        );

        assert_eq!(
            MemoryRegionType::ReadWriteData.flags(),
            PageTableFlags::Valid | PageTableFlags::Read | PageTableFlags::Write
        );

        assert!(!MemoryRegionType::Code.flags().contains(PageTableFlags::Write));
        assert!(!MemoryRegionType::ReadOnlyData.flags().contains(PageTableFlags::Write));
    }

    #[test]
    fn test_frame_allocator() {
        let base = 0x8000_0000;
        let size = 0x1000_0000; // 256 MB

        let allocator = physical::FrameAllocator::new(base, size);

        // Allocate a frame
        let frame1 = allocator.allocate_frame();
        assert!(frame1.is_some());

        // Allocate another frame
        let frame2 = allocator.allocate_frame();
        assert!(frame2.is_some());

        // Free the first frame
        if let Some(frame) = frame1 {
            allocator.free_frame(frame);
        }

        // Should be able to allocate again
        let frame3 = allocator.allocate_frame();
        assert!(frame3.is_some());
        assert_eq!(frame3, frame1);
    }
}