//! Huge pages support for xv6-rust

extern crate alloc;


use core::ptr::null_mut;

use crate::subsystems::mm::buddy;
use crate::subsystems::mm::buddy::BuddyBlock;

// ============================================================================
// Constants
// ============================================================================

/// Huge page sizes
pub const HPAGE_2MB: usize = 0x200000;
pub const HPAGE_1GB: usize = 0x40000000;

/// Maximum number of huge page orders
pub const HPAGE_MAX_ORDERS: usize = 8;

// ============================================================================
// Huge Pages Allocator
// ============================================================================

/// Huge pages allocator
pub struct HugePageAllocator {
    /// Free lists for each huge page size
    free_lists: [*mut BuddyBlock; HPAGE_MAX_ORDERS],
    /// Available huge page sizes in bytes
    hpage_sizes: [usize; HPAGE_MAX_ORDERS],
    /// Number of available huge page sizes
    num_hpage_sizes: usize,
}

impl Default for HugePageAllocator {
    fn default() -> Self {
        let mut allocator = Self {
            free_lists: [null_mut(); HPAGE_MAX_ORDERS],
            hpage_sizes: [0; HPAGE_MAX_ORDERS],
            num_hpage_sizes: 0,
        };
        
        // Initialize with supported huge page sizes
        // For simplicity, we'll only support 2MB and 1GB pages
        
        let mut index = 0;
        if cfg!(feature = "hpage_2mb") {
            allocator.hpage_sizes[index] = HPAGE_2MB;
            index += 1;
        }
        
        if cfg!(feature = "hpage_1gb") {
            allocator.hpage_sizes[index] = HPAGE_1GB;
            index += 1;
        }
        
        allocator.num_hpage_sizes = index;
        
        allocator
    }
}

impl HugePageAllocator {
    /// Create a new huge pages allocator
    pub const fn new() -> Self {
        let mut allocator = Self {
            free_lists: [null_mut(); HPAGE_MAX_ORDERS],
            hpage_sizes: [0; HPAGE_MAX_ORDERS],
            num_hpage_sizes: 0,
        };
        
        // Initialize with supported huge page sizes
        // Support both 2MB and 1GB pages by default
        allocator.hpage_sizes[0] = HPAGE_2MB;
        allocator.hpage_sizes[1] = HPAGE_1GB;
        allocator.num_hpage_sizes = 2;
        
        allocator
    }
    
    /// Initialize the huge pages allocator with a memory region
    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize) {
        // For simplicity, we'll just assume the entire region is available for huge pages
        
        // Round heap_start to the nearest huge page boundary
        // And heap_end down to the nearest huge page boundary
        
        for i in 0..self.num_hpage_sizes {
            let hpage_size = self.hpage_sizes[i];
            
            let aligned_start = (heap_start + hpage_size - 1) & !(hpage_size - 1);
            let aligned_end = heap_end & !(hpage_size - 1);
            
            if aligned_start + hpage_size > aligned_end {
                continue; // Not enough space for this huge page size
            }
            
            // Add all huge pages to the free list
            let mut addr = aligned_start;
            while addr + hpage_size <= aligned_end {
                let block = addr as *mut BuddyBlock;
                
                (*block).size = hpage_size;
                (*block).next = self.free_lists[i];
                self.free_lists[i] = block;
                
                addr += hpage_size;
            }
        }
    }
    
    /// Allocate a huge page
    /// Optimized algorithm: tries to find the best fit (smallest page that satisfies the request)
    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        // Find the smallest huge page that can satisfy the request
        // This minimizes internal fragmentation
        
        let mut best_fit: Option<usize> = None;
        let mut best_fit_size = usize::MAX;
        
        // First pass: find the best fit
        for i in 0..self.num_hpage_sizes {
            let hpage_size = self.hpage_sizes[i];
            
            if hpage_size >= size && hpage_size < best_fit_size {
                if !self.free_lists[i].is_null() {
                    best_fit = Some(i);
                    best_fit_size = hpage_size;
                }
            }
        }
        
        // Allocate from the best fit if found
        if let Some(i) = best_fit {
            let block = self.free_lists[i];
            if !block.is_null() {
                unsafe {
                    self.free_lists[i] = (*block).next;
                    (*block).next = null_mut();
                }
                
                return block as *mut u8;
            }
        }
        
        null_mut() // Out of huge pages
    }
    
    /// Allocate a specific huge page size
    pub fn alloc_size(&mut self, hpage_size: usize) -> *mut u8 {
        // Find the index for this specific size
        for i in 0..self.num_hpage_sizes {
            if self.hpage_sizes[i] == hpage_size {
                let block = self.free_lists[i];
                if !block.is_null() {
                    unsafe {
                        self.free_lists[i] = (*block).next;
                        (*block).next = null_mut();
                    }
                    
                    return block as *mut u8;
                }
                break;
            }
        }
        
        null_mut() // Out of huge pages of this size
    }
    
    /// Deallocate a huge page
    pub fn dealloc(&mut self, ptr: *mut u8, size: usize) {
        if ptr.is_null() {
            return;
        }
        
        // Find the appropriate free list
        
        for i in 0..self.num_hpage_sizes {
            if self.hpage_sizes[i] == size {
                let block = ptr as *mut BuddyBlock;
                
                unsafe {
                    (*block).next = self.free_lists[i];
                    (*block).size = size;
                }
                
                self.free_lists[i] = block;
                
                return;
            }
        }
    }
    
    /// Check if a size is a supported huge page size
    pub fn is_hpage_size(&self, size: usize) -> bool {
        for i in 0..self.num_hpage_sizes {
            if self.hpage_sizes[i] == size {
                return true;
            }
        }
        
        false
    }
    
    /// Get the supported huge page sizes
    pub fn supported_sizes(&self) -> &[usize] {
        &self.hpage_sizes[0..self.num_hpage_sizes]
    }
    
    /// Get statistics about huge page usage
    pub fn get_stats(&self) -> HugePageStats {
        let mut stats = HugePageStats {
            total_2mb_pages: 0,
            free_2mb_pages: 0,
            total_1gb_pages: 0,
            free_1gb_pages: 0,
        };
        
        for i in 0..self.num_hpage_sizes {
            let hpage_size = self.hpage_sizes[i];
            let mut count = 0;
            let mut current = self.free_lists[i];
            
            while !current.is_null() {
                count += 1;
                unsafe {
                    current = (*current).next;
                }
            }
            
            if hpage_size == HPAGE_2MB {
                stats.free_2mb_pages = count;
            } else if hpage_size == HPAGE_1GB {
                stats.free_1gb_pages = count;
            }
        }
        
        stats
    }
}

/// Statistics for huge page usage
#[derive(Debug, Clone, Copy)]
pub struct HugePageStats {
    pub total_2mb_pages: usize,
    pub free_2mb_pages: usize,
    pub total_1gb_pages: usize,
    pub free_1gb_pages: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hugepage_alloc() {
        let mut alloc = HugePageAllocator::default();
        
        // Test with a simulated 1GB memory region (from 0x100000000 to 0x200000000)
        unsafe { alloc.init(0x100000000, 0x200000000); }
        
        // Allocate a 2MB huge page (if supported)
        if cfg!(feature = "hpage_2mb") {
            let ptr = alloc.alloc(HPAGE_2MB);
            assert!(!ptr.is_null());
            
            // Deallocate it
            alloc.dealloc(ptr, HPAGE_2MB);
        }
    }
}
