//! Memory Management Interface
//!
//! This module defines the public interface for memory management
//! that can be used by other subsystems.

extern crate alloc;

use alloc::vec::Vec;

/// Physical address type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub const fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Page frame number type
pub type Pfn = usize;

/// Memory management statistics
#[derive(Debug, Clone)]
pub struct MemoryManagementStats {
    pub total_pages: usize,
    pub free_pages: usize,
    pub allocated_pages: usize,
}

impl Default for MemoryManagementStats {
    fn default() -> Self {
        Self {
            total_pages: 0,
            free_pages: 0,
            allocated_pages: 0,
        }
    }
}

/// Convert physical address to page frame number
pub fn addr_to_pfn(addr: PhysAddr) -> Pfn {
    addr.as_usize() / super::phys::PAGE_SIZE
}

/// Convert page frame number to physical address
pub fn pfn_to_addr(pfn: Pfn) -> PhysAddr {
    PhysAddr(pfn * super::phys::PAGE_SIZE)
}

/// Get heap start address
pub fn heap_start() -> usize {
    #[cfg(not(feature = "test"))]
    {
        // In real implementation, this would come from linker
        unsafe extern "C" {
            #[link_name = "heap_start"]
            fn heap_start() -> usize;
        }
        unsafe { heap_start() }
    }
    #[cfg(feature = "test")]
    0
}

/// Get heap end address
pub fn heap_end() -> usize {
    #[cfg(not(feature = "test"))]
    {
        unsafe extern "C" {
            #[link_name = "heap_end"]
            fn heap_end() -> usize;
        }
        unsafe { heap_end() }
    }
    #[cfg(feature = "test")]
    0x10000000
}

/// Get MMIO regions
pub fn mmio_regions() -> Vec<(PhysAddr, usize)> {
    #[cfg(not(feature = "test"))]
    {
        unsafe extern "C" {
            #[link_name = "mmio_regions_start"]
            fn mmio_regions_start() -> *const usize;
            #[link_name = "mmio_regions_end"]
            fn mmio_regions_end() -> *const usize;
        }
        unsafe {
            let start = mmio_regions_start();
            let end = mmio_regions_end();
            if !start.is_null() && !end.is_null() {
                // Return as a simple tuple for now
                {
    let mut v = alloc::vec::Vec::new();
    v.push((PhysAddr(start as usize), end as usize));
    v
}
            } else {
                Vec::new()
            }
        }
    }
    #[cfg(feature = "test")]
    Vec::new()
}

/// Take MMIO statistics snapshot
pub fn mmio_stats_take() -> MemoryManagementStats {
    #[cfg(not(feature = "test"))]
    {
        unsafe extern "C" {
            #[link_name = "mmio_stats_snapshot"]
            fn mmio_stats_snapshot_fn(stats: *mut MemoryManagementStats);
        }
        let mut stats = MemoryManagementStats::default();
        unsafe { mmio_stats_snapshot_fn(&mut stats) };
        stats
    }
    #[cfg(feature = "test")]
    MemoryManagementStats::default()
}

/// Shutdown per-CPU allocators
pub fn shutdown_percpu_allocators() {
    #[cfg(not(feature = "test"))]
    {
        unsafe extern "C" {
            #[link_name = "shutdown_percpu_allocators"]
            fn shutdown_percpu_allocators_fn();
        }
        unsafe { shutdown_percpu_allocators_fn() };
    }
    #[cfg(feature = "test")]
    {}
}

/// Memory move operation
/// 
/// # Safety
/// The caller must ensure that the source and destination ranges are valid
/// and do not overlap.
pub unsafe fn memmove(dst: *mut u8, src: *const u8, count: usize) {
    if src.is_null() || dst.is_null() || count == 0 {
        return;
    }
    
    core::ptr::copy(src, dst, count);
}

/// Memory set operation
///
/// # Safety
/// The caller must ensure that the destination buffer is valid
/// and has enough space for `count` bytes.
pub unsafe fn memset(ptr: *mut u8, value: u8, count: usize) {
    if ptr.is_null() || count == 0 {
        return;
    }
    
    core::ptr::write_bytes(ptr, value, count);
}

/// Get memory statistics
pub fn mem_stats() -> MemoryManagementStats {
    super::unified_stats::get_memory_stats()
}

