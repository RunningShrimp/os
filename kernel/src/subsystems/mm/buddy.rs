#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Buddy System Memory Allocator
//! 
//! This module implements the buddy system allocator for physical memory management.

use core::alloc::Layout;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Block size levels for buddy allocator
pub const MAX_ORDER: usize = 10; // Support up to 2^10 * PAGE_SIZE blocks

/// Block header for buddy allocator
#[derive(Debug, Clone)]
pub struct BuddyBlock {
    pub size: usize,
    pub allocated: bool,
    pub next: *mut BuddyBlock,
}

/// Statistics for the buddy allocator
#[derive(Debug, Clone)]
pub struct AllocatorStats {
    pub allocated: usize,
    pub freed: usize,
    pub fragmentation: usize,
}

impl AllocatorStats {
    pub const fn new() -> Self {
        Self {
            allocated: 0,
            freed: 0,
            fragmentation: 0,
        }
    }
}

impl Default for AllocatorStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized Buddy Allocator
pub struct OptimizedBuddyAllocator {
    free_lists: [*mut BuddyBlock; MAX_ORDER + 1],
    total_memory: AtomicUsize,
    allocated_memory: AtomicUsize,
    stats: AllocatorStats,
}

unsafe impl Send for OptimizedBuddyAllocator {}

impl OptimizedBuddyAllocator {
    pub const fn new() -> Self {
        // Initialize free lists with null pointers
        const NULL_PTR: *mut BuddyBlock = core::ptr::null_mut();
        let free_lists = [NULL_PTR; MAX_ORDER + 1];
        
        Self {
            free_lists,
            total_memory: AtomicUsize::new(0),
            allocated_memory: AtomicUsize::new(0),
            stats: AllocatorStats::new(),
        }
    }

    /// Initialize the buddy allocator with a memory range
    pub unsafe fn init(&mut self, start: usize, end: usize) {
        let total_size = end - start;
        self.total_memory.store(total_size, Ordering::SeqCst);
        
        // Add entire memory to the largest free block
        let block_ptr = start as *mut BuddyBlock;
        (*block_ptr).size = total_size;
        (*block_ptr).allocated = false;
        (*block_ptr).next = core::ptr::null_mut();
        
        // Find the appropriate order for this block
        let order = self.size_to_order(total_size);
        if order <= MAX_ORDER {
            self.free_lists[order] = block_ptr;
        }
    }

    /// Allocate memory of given layout
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let order = self.size_to_order(size);
        
        // Find the smallest free block that can satisfy the request
        for current_order in order..=MAX_ORDER {
            if !self.free_lists[current_order].is_null() {
                let block = self.free_lists[current_order];
                self.free_lists[current_order] = (*block).next;
                
                // Split the block if necessary
                let final_block = if current_order > order {
                    self.split_block(block, current_order, order)
                } else {
                    block
                };
                
                (*final_block).allocated = true;
                self.allocated_memory.fetch_add(size, Ordering::SeqCst);
                self.stats.allocated += size;
                
                return final_block.add(core::mem::size_of::<BuddyBlock>()) as *mut u8;
            }
        }
        
        // No suitable block found
        core::ptr::null_mut()
    }

    /// Deallocate memory
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        
        let size = layout.size();
        let block_ptr = (ptr as usize - core::mem::size_of::<BuddyBlock>()) as *mut BuddyBlock;
        
        if !(*block_ptr).allocated {
            // Double-free or invalid free
            return;
        }
        
        (*block_ptr).allocated = false;
        self.allocated_memory.fetch_sub(size, Ordering::SeqCst);
        self.stats.freed += size;
        
        // Coalesce with buddy if possible
        let order = self.size_to_order(size);
        self.coalesce_block(block_ptr, order);
    }

    /// Get allocator statistics
    pub fn stats(&self) -> AllocatorStats {
        // Calculate fragmentation as percentage
        let total = self.total_memory.load(Ordering::SeqCst);
        let allocated = self.allocated_memory.load(Ordering::SeqCst);
        let fragmentation = if total > 0 {
            ((total - allocated) * 100) / total
        } else {
            0
        };
        
        AllocatorStats {
            allocated: self.stats.allocated,
            freed: self.stats.freed,
            fragmentation,
        }
    }

    /// Convert size to buddy order
    fn size_to_order(&self, size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        
        let mut order = 0;
        let mut block_size = 1;
        while block_size < size {
            block_size *= 2;
            order += 1;
        }
        order
    }

    /// Split a block to the target order
    unsafe fn split_block(&mut self, block: *mut BuddyBlock, current_order: usize, target_order: usize) -> *mut BuddyBlock {
        if current_order == target_order {
            return block;
        }
        
        let size = (*block).size / 2;
        let second_half = (block as usize + size) as *mut BuddyBlock;
        
        // Create split blocks
        (*block).size = size;
        (*second_half).size = size;
        (*second_half).allocated = false;
        (*second_half).next = core::ptr::null_mut();
        
        // Add second half to free list at current_order - 1
        self.free_lists[current_order - 1] = second_half;
        
        // Recursively split first half
        self.split_block(block, current_order - 1, target_order)
    }

    /// Coalesce buddy blocks
    unsafe fn coalesce_block(&mut self, block: *mut BuddyBlock, order: usize) {
        if order >= MAX_ORDER {
            // Add to free list at current order
            (*block).next = self.free_lists[order];
            self.free_lists[order] = block;
            return;
        }
        
        // Find buddy block
        let block_addr = block as usize;
        let buddy_addr = block_addr ^ (1 << order);
        let buddy = buddy_addr as *mut BuddyBlock;
        
        // Check if buddy is free and same size
        if !self.is_block_in_free_list(buddy, order) {
            // Buddy not free, add current block to free list
            (*block).next = self.free_lists[order];
            self.free_lists[order] = block;
            return;
        }
        
        // Remove buddy from free list
        self.remove_from_free_list(buddy, order);
        
        // Merge blocks
        let merged_block = if block_addr < buddy_addr {
            block
        } else {
            buddy
        };
        (*merged_block).size *= 2;
        
        // Recursively coalesce
        self.coalesce_block(merged_block, order + 1);
    }

    /// Check if a block is in a free list
    unsafe fn is_block_in_free_list(&self, block: *mut BuddyBlock, order: usize) -> bool {
        let mut current = self.free_lists[order];
        while !current.is_null() {
            if current == block {
                return true;
            }
            current = (*current).next;
        }
        false
    }

    /// Remove a block from a free list
    unsafe fn remove_from_free_list(&mut self, block: *mut BuddyBlock, order: usize) {
        let mut current = self.free_lists[order];
        let mut prev: *mut BuddyBlock = core::ptr::null_mut();
        
        while !current.is_null() {
            if current == block {
                if prev.is_null() {
                    self.free_lists[order] = (*current).next;
                } else {
                    (*prev).next = (*current).next;
                }
                return;
            }
            prev = current;
            current = (*current).next;
        }
    }
}

impl Default for OptimizedBuddyAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// Add next pointer to BuddyBlock
#[derive(Debug, Clone)]
pub struct BuddyBlockWithNext {
    pub size: usize,
    pub allocated: bool,
    pub next: *mut BuddyBlockWithNext,
}
