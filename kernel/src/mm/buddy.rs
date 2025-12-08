//! Buddy allocator for kernel heap
//! Provides efficient memory allocation with minimal fragmentation

extern crate alloc;

use core::alloc::Layout;
use core::ptr::null_mut;
use alloc::vec::Vec;

/// Buddy allocator that manages memory blocks of power-of-2 sizes
pub struct BuddyAllocator {
    /// Free lists for each power of 2: [16, 32, 64, 128, 256, ...]
    free_lists: [*mut BuddyBlock; 32],
    heap_start: usize,
    heap_end: usize,
    /// Bitmap to track allocated blocks (1 bit per minimum block)
    bitmap: Vec<u8>,
    min_block_size: usize, // Usually PAGE_SIZE
    statistics: AllocatorStats,
}

pub struct BuddyBlock {
    pub next: *mut BuddyBlock,
    pub size: usize, // Size of this block in bytes
}

#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    pub allocated: usize,
    pub freed: usize,
    pub fragmentation: usize,
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    pub const fn new() -> Self {
        Self {
            free_lists: [null_mut(); 32],
            heap_start: 0,
            heap_end: 0,
            bitmap: Vec::new(),
            min_block_size: 4096, // PAGE_SIZE
            statistics: AllocatorStats {
                allocated: 0,
                freed: 0,
                fragmentation: 0,
            },
        }
    }

    /// Initialize the allocator with a memory region
    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize, min_block_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_end;
        self.min_block_size = min_block_size;

        // Initialize the bitmap for tracking allocated blocks
        let total_blocks = (heap_end - heap_start) / min_block_size;
        let _bitmap_size = (total_blocks + 7) / 8;
        
        // For simplicity, we'll manage the initial free block
        // In a real implementation, this would be more sophisticated
        if heap_end > heap_start {
            let block = heap_start as *mut BuddyBlock;
            (*block).size = heap_end - heap_start;
            (*block).next = null_mut();
            
            // Find the appropriate free list for this block
            let order = self.get_order((*block).size);
            if order < 32 {
                (*block).next = self.free_lists[order];
                self.free_lists[order] = block;
            }
        }
    }

    /// Allocate memory with given layout
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        
        // Round up to minimum block size
        let required_size = if size < self.min_block_size {
            self.min_block_size
        } else {
            self.round_up_power_of_2(size)
        };

        let order = self.get_order(required_size);
        if order >= 32 {
            return null_mut();
        }

        // Try to find a free block at the requested order or higher
        for i in order..32 {
            if !self.free_lists[i].is_null() {
                let block = self.free_lists[i];
                self.free_lists[i] = unsafe { (*block).next };

                // Split blocks if necessary
                let mut current = block;
                let mut current_order = i;
                while current_order > order {
                    current_order -= 1;
                    let block_size = 1 << (current_order + 12); // Assuming base 4KB
                    let buddy = unsafe { (current as usize + block_size) as *mut BuddyBlock };
                    
                    unsafe {
                        (*buddy).size = block_size;
                        (*buddy).next = self.free_lists[current_order];
                    }
                    self.free_lists[current_order] = buddy;
                }

                // Align the returned pointer
                let aligned_ptr = align_up(current as usize, align) as *mut u8;
                self.statistics.allocated += required_size;
                
                return aligned_ptr;
            }
        }

        null_mut() // Out of memory
    }

    /// Deallocate memory
    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let size = layout.size();
        let block_size = self.round_up_power_of_2(if size < self.min_block_size {
            self.min_block_size
        } else {
            size
        });

        let order = self.get_order(block_size);
        if order >= 32 {
            return;
        }

        let block = ptr as *mut BuddyBlock;
        unsafe {
            (*block).size = block_size;
            (*block).next = self.free_lists[order];
        }
        self.free_lists[order] = block;
        self.statistics.freed += block_size;

        // Coalesce with buddy blocks
        self.coalesce(order);
    }

    /// Coalesce with buddy blocks to reduce fragmentation
    fn coalesce(&mut self, mut order: usize) {
        while order < 31 {
            let block = self.free_lists[order];
            if block.is_null() {
                return;
            }

            let buddy = self.find_buddy(block, order);
            if buddy.is_null() {
                return;
            }

            // Remove buddy from free list
            self.remove_from_free_list(buddy, order);

            // Merge blocks
            let merged = if (block as usize) < (buddy as usize) {
                block
            } else {
                buddy
            };

            unsafe {
                (*merged).size *= 2;
                (*merged).next = self.free_lists[order + 1];
            }
            self.free_lists[order + 1] = merged;

            order += 1;
        }
    }

    /// Find the buddy of a block
    fn find_buddy(&self, block: *mut BuddyBlock, order: usize) -> *mut BuddyBlock {
        let block_size = 1 << (order + 12);
        let buddy_addr = (block as usize) ^ block_size;
        
        if buddy_addr >= self.heap_start && buddy_addr < self.heap_end {
            buddy_addr as *mut BuddyBlock
        } else {
            null_mut()
        }
    }

    /// Remove a block from the free list
    fn remove_from_free_list(&mut self, target: *mut BuddyBlock, order: usize) {
        let mut current = self.free_lists[order];
        let mut prev: *mut *mut BuddyBlock = &mut self.free_lists[order];

        while !current.is_null() {
            if current == target {
                unsafe {
                    *prev = (*current).next;
                }
                return;
            }
            prev = unsafe { &mut (*current).next };
            current = unsafe { (*current).next };
        }
    }

    /// Get the order (power of 2) for a given size
    fn get_order(&self, size: usize) -> usize {
        let mut order = 0;
        let mut block_size = self.min_block_size;
        while block_size < size && order < 32 {
            block_size *= 2;
            order += 1;
        }
        order
    }

    /// Round up to the next power of 2
    fn round_up_power_of_2(&self, mut n: usize) -> usize {
        n -= 1;
        n |= n >> 1;
        n |= n >> 2;
        n |= n >> 4;
        n |= n >> 8;
        n |= n >> 16;
        n |= n >> 32;
        n + 1
    }

    /// Get allocator statistics
    pub fn stats(&self) -> AllocatorStats {
        self.statistics
    }
}

/// Align up to the given alignment
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

// Implement Send/Sync for buddy allocator types since they are thread-safe when protected by Mutex
unsafe impl Send for BuddyBlock {}
unsafe impl Sync for BuddyBlock {}
unsafe impl Send for BuddyAllocator {}
unsafe impl Sync for BuddyAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buddy_alloc() {
        let mut alloc = BuddyAllocator::new();
        unsafe { alloc.init(0x1000, 0x10000, 0x1000); }
        
        let layout = Layout::from_size_align(256, 8).unwrap();
        let ptr = alloc.alloc(layout);
        assert!(!ptr.is_null());
    }
}
