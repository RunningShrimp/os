//! Optimized buddy allocator with proper bitmap usage and improved coalescing

extern crate alloc;

use core::alloc::Layout;
use core::ptr::null_mut;
use alloc::vec::Vec;

/// Buddy allocator that manages memory blocks of power-of-2 sizes
pub struct OptimizedBuddyAllocator {
    /// Free lists for each power of 2: [16, 32, 64, 128, 256, ...]
    free_lists: [*mut BuddyBlock; 32],
    heap_start: usize,
    heap_end: usize,
    /// Bitmap to track allocated blocks (1 bit per minimum block)
    bitmap: Vec<u64>,
    min_block_size: usize, // Usually PAGE_SIZE
    min_order: usize,
    max_order: usize,
    total_blocks: usize,
    statistics: AllocatorStats,
}

/// A block in the buddy allocator free list.
/// 
/// This represents a free memory block that can be allocated or split into smaller blocks.
pub struct BuddyBlock {
    /// Pointer to the next free block in the same size class.
    pub next: *mut BuddyBlock,
    /// Size of this block in bytes (must be a power of 2).
    pub size: usize,
}

/// Allocator statistics for the buddy allocator.
///
/// Tracks memory usage, freed memory, and fragmentation statistics.
#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    /// Total number of bytes allocated.
    pub allocated: usize,
    /// Total number of bytes freed.
    pub freed: usize,
    /// Percentage of memory fragmentation (0-100).
    pub fragmentation: usize,
}

impl OptimizedBuddyAllocator {
    /// Create a new buddy allocator
    pub const fn new() -> Self {
        Self {
            free_lists: [null_mut(); 32],
            heap_start: 0,
            heap_end: 0,
            bitmap: Vec::new(),
            min_block_size: 4096, // PAGE_SIZE
            min_order: 0,
            max_order: 31,
            total_blocks: 0,
            statistics: AllocatorStats {
                allocated: 0,
                freed: 0,
                fragmentation: 0,
            },
        }
    }

    /// Initialize the allocator with a memory region
    ///
    /// # Safety
    /// - `heap_start` and `heap_end` must point to valid, contiguous memory that is accessible
    /// - The memory region must be aligned to `min_block_size` boundaries
    /// - `min_block_size` must be a power of 2 and greater than 0
    /// - The caller must ensure no other code accesses this memory region while the allocator is in use
    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize, min_block_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_end;
        self.min_block_size = min_block_size;

        // Calculate the total number of minimum-sized blocks
        let total_size = heap_end - heap_start;
        self.total_blocks = total_size / min_block_size;
        
        // Initialize bitmap
        let bitmap_size = self.total_blocks.div_ceil(64);
        self.bitmap = Vec::with_capacity(bitmap_size);
        for _ in 0..bitmap_size {
            self.bitmap.push(0);
        }

        // Initialize order range
        self.min_order = 0;
        self.max_order = 31;

        // For simplicity, we'll manage the initial free block
        if heap_end > heap_start {
            let block = heap_start as *mut BuddyBlock;
            unsafe {
                (*block).size = heap_end - heap_start;
                (*block).next = null_mut();
            }
            
            // Find the appropriate free list for this block
            let order = unsafe { self.get_order((*block).size) };
            if order < 32 {
                unsafe {
                    (*block).next = self.free_lists[order];
                }
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
                let mut current_order = i;
                let current_block = block;
                
                while current_order > order {
                    current_order -= 1;
                    let block_size = 1 << (self.min_order + current_order);
                    
                    // Calculate buddy address
                    let buddy_addr = (current_block as usize) + block_size;
                    let buddy_block = buddy_addr as *mut BuddyBlock;
                    
                    // Split the block - mark both as free in their respective lists
                    unsafe {
                        (*current_block).size = block_size;
                        (*buddy_block).size = block_size;
                        
                        // Add buddy to current order free list
                        (*buddy_block).next = self.free_lists[current_order];
                        self.free_lists[current_order] = buddy_block;
                    }
                }
                
                // Mark block as allocated in bitmap
                let block_idx = (current_block as usize - self.heap_start) / self.min_block_size;
                let num_blocks = required_size / self.min_block_size;
                
                // Update bitmap - set all bits for this block
                for i in 0..num_blocks {
                    let bit_idx = block_idx + i;
                    let word_idx = bit_idx / 64;
                    let bit = bit_idx % 64;
                    self.bitmap[word_idx] |= 1 << bit;
                }
                
                // Align the returned pointer
                let aligned_ptr = align_up(current_block as usize, align) as *mut u8;
                
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
        
        // Mark block as free in bitmap
        let block_idx = (block as usize - self.heap_start) / self.min_block_size;
        let num_blocks = block_size / self.min_block_size;
        
        // Update bitmap - clear all bits for this block
        for i in 0..num_blocks {
            let bit_idx = block_idx + i;
            let word_idx = bit_idx / 64;
            let bit = bit_idx % 64;
            self.bitmap[word_idx] &= !(1 << bit);
        }
        
        // Add back to free list
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
        while order < self.max_order {
            let block = self.free_lists[order];
            if block.is_null() {
                return;
            }

            let buddy = self.find_buddy(block, order);
            if buddy.is_null() {
                return;
            }
            
            // Check if buddy is actually free in bitmap
            let buddy_idx = (buddy as usize - self.heap_start) / self.min_block_size;
            let buddy_in_bitmap = self.is_block_free(buddy_idx, order + 1);
            if !buddy_in_bitmap {
                return;
            }

            // Remove both blocks from current free list
            self.remove_from_free_list(block, order);
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
        let block_size = 1 << (self.min_order + order);
        let buddy_addr = (block as usize) ^ block_size;
        
        if buddy_addr >= self.heap_start && buddy_addr < self.heap_end {
            buddy_addr as *mut BuddyBlock
        } else {
            null_mut()
        }
    }

    /// Check if a block is free in the bitmap
    fn is_block_free(&self, block_idx: usize, order: usize) -> bool {
        let num_blocks = 1 << (order - self.min_order);
        
        for i in 0..num_blocks {
            let bit_idx = block_idx + i;
            let word_idx = bit_idx / 64;
            let bit = bit_idx % 64;
            
            if self.bitmap[word_idx] & (1 << bit) != 0 {
                return false;
            }
        }
        
        true
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
        let mut current_size = self.min_block_size;
        
        // Skip the loop if size is exactly the minimum block size
        if size == self.min_block_size {
            return order;
        }
        
        // Find the order where current_size >= size
        while current_size < size && order < 31 {
            current_size *= 2;
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
        // Calculate fragmentation
        let used_memory = self.statistics.allocated - self.statistics.freed;
        let total_memory = self.heap_end - self.heap_start;
        let fragmentation = if total_memory > 0 {
            ((total_memory - used_memory) * 100) / total_memory
        } else {
            0
        };

        AllocatorStats {
            allocated: self.statistics.allocated,
            freed: self.statistics.freed,
            fragmentation,
        }
    }
}

impl Default for OptimizedBuddyAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Align up to the given alignment
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

// Implement Send/Sync for buddy allocator types since they are thread-safe when protected by Mutex
unsafe impl Send for BuddyBlock {}
unsafe impl Sync for BuddyBlock {}
unsafe impl Send for OptimizedBuddyAllocator {}
unsafe impl Sync for OptimizedBuddyAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buddy_alloc() {
        let mut alloc = OptimizedBuddyAllocator::new();
        unsafe { alloc.init(0x1000, 0x10000, 0x1000); }
        
        let layout = Layout::from_size_align(256, 8).unwrap();
        let ptr = alloc.alloc(layout);
        assert!(!ptr.is_null());
    }
}

/// Alias for backward compatibility
pub type BuddyAllocator = OptimizedBuddyAllocator;
