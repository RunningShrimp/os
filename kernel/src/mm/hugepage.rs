//! Huge pages support for xv6-rust

extern crate alloc;


use core::ptr::null_mut;

use crate::mm::buddy;
use crate::mm::buddy::BuddyBlock;

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
        // For now, we'll support 2MB pages by default
        // 1GB pages can be enabled via feature flag
        allocator.hpage_sizes[0] = HPAGE_2MB;
        allocator.num_hpage_sizes = 1;
        
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
    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        // Find the smallest huge page that can satisfy the request
        
        for i in 0..self.num_hpage_sizes {
            let hpage_size = self.hpage_sizes[i];
            
            if hpage_size >= size {
                let block = self.free_lists[i];
                if !block.is_null() {
                    unsafe {
                        self.free_lists[i] = (*block).next;
                        (*block).next = null_mut();
                    }
                    
                    return block as *mut u8;
                }
            }
        }
        
        null_mut() // Out of huge pages
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
