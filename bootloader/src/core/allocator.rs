//! Boot-time allocator for no_std environment
//!
//! This module provides a dual-level allocator suitable for bootloader use:
//! - Large blocks: Managed through a bump allocator
//! - Small blocks: Managed through a segregated free list for efficient reuse
//! 
//! The allocator supports both allocation and deallocation, with blocks
//! aligned to 64-byte boundaries for cache efficiency.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Bootloader heap configuration
pub const BOOTLOADER_HEAP_SIZE: usize = 4 * 1024 * 1024; // 4MB heap
pub const BOOTLOADER_HEAP_ALIGN: usize = 64; // 64-byte alignment for cache efficiency
pub const SMALL_BLOCK_THRESHOLD: usize = 256; // Blocks <= 256 bytes use free list
pub const MAX_FREE_LIST_SIZE: usize = 1024; // Maximum entries in free list

/// Size classes for segregated free lists
const SIZE_CLASSES: [usize; 6] = [0, 16, 32, 64, 128, 256];
/// Number of buckets (size classes)
const NUM_BUCKETS: usize = SIZE_CLASSES.len() - 1;

/// Free list node for deallocated small blocks
#[repr(C)]
struct FreeNode {
    next: Option<*mut FreeNode>,
    size: usize,
}

/// Dual-level allocator for boot-time use
///
/// This allocator combines:
/// 1. Bump allocator for large allocations (> 256 bytes)
/// 2. Segregated free list for small allocations (<= 256 bytes) to support reuse
///
/// Both levels align allocations to 64-byte boundaries.
pub struct DualLevelAllocator {
    heap: UnsafeCell<[u8; BOOTLOADER_HEAP_SIZE]>,
    offset: AtomicUsize,
    /// Array of free lists, one per size class
    free_list: UnsafeCell<[Option<*mut FreeNode>; NUM_BUCKETS]>,
    free_list_size: AtomicUsize,
}

impl DualLevelAllocator {
    /// Create a new dual-level allocator
    pub const fn new() -> Self {
        Self {
            heap: UnsafeCell::new([0; BOOTLOADER_HEAP_SIZE]),
            offset: AtomicUsize::new(0),
            // Initialize all buckets to None
            free_list: UnsafeCell::new([None; NUM_BUCKETS]),
            free_list_size: AtomicUsize::new(0),
        }
    }

    /// Get current allocation offset
    pub fn allocated(&self) -> usize {
        self.offset.load(Ordering::Relaxed)
    }

    /// Get remaining free space
    pub fn free(&self) -> usize {
        BOOTLOADER_HEAP_SIZE - self.allocated()
    }

    /// Check if there's enough space for allocation
    #[allow(dead_code)]
    fn can_allocate(&self, layout: &Layout) -> bool {
        let current = self.allocated();
        let aligned = Self::align_up(current, BOOTLOADER_HEAP_ALIGN);
        let needed = layout.size();

        aligned.checked_add(needed)
            .map(|end| end <= BOOTLOADER_HEAP_SIZE)
            .unwrap_or(false)
    }

    /// Align address up to 64-byte boundary
    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }

    /// Get memory utilization percentage
    pub fn utilization(&self) -> f32 {
        (self.allocated() as f32) / (BOOTLOADER_HEAP_SIZE as f32) * 100.0
    }

    /// Get the bucket index for a given size
    fn get_bucket_index(size: usize) -> usize {
        // Find the smallest bucket that can fit the size
        for (i, &class) in SIZE_CLASSES.iter().enumerate().skip(1) {
            if size <= class {
                return i - 1;
            }
        }
        NUM_BUCKETS - 1
    }

    /// Try to allocate from segregated free list (for small blocks)
    unsafe fn alloc_from_free_list(&self, size: usize) -> Option<*mut u8> {
        // Find the smallest bucket that can fit the size
        let mut bucket_index = Self::get_bucket_index(size);
        
        // Check all buckets from current to largest
        while bucket_index < NUM_BUCKETS {
            let bucket_ptr = &mut (*self.free_list.get())[bucket_index];
            
            if let Some(mut current) = *bucket_ptr {
                
                let mut prev: Option<*mut FreeNode> = None;
                
                while let Some(node) = current.as_mut() {
                    if node.size >= size {
                        // Found a suitable block - smallest in this bucket
                        let result = current as *mut u8;
                        
                        // Unlink from free list
                        if let Some(prev_node) = prev {
                            // Dereference raw pointer before accessing field
                            (*prev_node).next = node.next;
                        } else {
                            *bucket_ptr = node.next;
                        }
                        
                        self.free_list_size.fetch_sub(1, Ordering::Relaxed);
                        return Some(result);
                    }
                    
                    prev = Some(current);
                    current = match node.next {
                        Some(next) => next,
                        None => break,
                    };
                }
            }
            
            // Move to next larger bucket
            bucket_index += 1;
        }
        
        // No suitable block found
        None
    }

    /// Add block to segregated free list with merging
    unsafe fn add_to_free_list(&self, ptr: *mut u8, size: usize) {
        if self.free_list_size.load(Ordering::Relaxed) >= MAX_FREE_LIST_SIZE {
            // Free list is full, don't track this block
            return;
        }
        
        // Step 1: Check for adjacent free blocks to merge with
        let mut merged_ptr = ptr;
        let mut merged_size = size;
        
        let heap_start = self.heap.get() as *mut u8;
        let _heap_end = heap_start.add(BOOTLOADER_HEAP_SIZE);
        log::trace!("Merging block at heap range");
        
        // Check all buckets for adjacent blocks
        for bucket_index in 0..NUM_BUCKETS {
            let bucket_ptr = &mut (*self.free_list.get())[bucket_index];
            
            let mut current = *bucket_ptr;
            let mut prev: Option<*mut FreeNode> = None;
            
            while let Some(node) = current {
                let node_ptr = node as *mut u8;
                let node_ref = node.as_ref().unwrap();
                let node_end = node_ptr.add(node_ref.size);
                
                // Check if current node is adjacent before merged block
                if node_end == merged_ptr {
                    // Merge with previous node
                    merged_size += node_ref.size;
                    merged_ptr = node_ptr;
                    
                    // Unlink current node from the list
                    if let Some(prev_node) = prev {
                        // Dereference raw pointer before accessing field
                        (*prev_node).next = node_ref.next;
                    } else {
                        *bucket_ptr = node_ref.next;
                    }
                    
                    self.free_list_size.fetch_sub(1, Ordering::Relaxed);
                    
                    // Restart merging check since merged_ptr has changed
                    break;
                } 
                // Check if current node is adjacent after merged block
                else if merged_ptr.add(merged_size) == node_ptr {
                    // Merge with next node
                    merged_size += node_ref.size;
                    
                    // Unlink current node from the list
                    if let Some(prev_node) = prev {
                        // Dereference raw pointer before accessing field
                        (*prev_node).next = node_ref.next;
                    } else {
                        *bucket_ptr = node_ref.next;
                    }
                    
                    self.free_list_size.fetch_sub(1, Ordering::Relaxed);
                    
                    // Update current to check next node after merged block
                    current = node_ref.next;
                } 
                else {
                    // Move to next node in the list
                    prev = Some(node);
                    current = node_ref.next;
                }
            }
        }
        
        // Step 2: Insert merged block into the appropriate bucket
        let new_node = merged_ptr as *mut FreeNode;
        let new_node_ref = new_node.as_mut().unwrap();
        new_node_ref.size = merged_size;
        
        let target_bucket = Self::get_bucket_index(merged_size);
        let bucket_ptr = &mut (*self.free_list.get())[target_bucket];
        
        if bucket_ptr.is_none() {
            // Empty bucket, insert directly
            // Dereference raw pointer before accessing field
            (*new_node).next = None;
            *bucket_ptr = Some(new_node);
        } else {
            // Insert into bucket, maintaining ascending order by size
            let mut current = bucket_ptr.as_ref().copied().unwrap();
            let mut prev: Option<*mut FreeNode> = None;
            
            // Find insertion point
            while current.as_ref().unwrap().size < merged_size {
                prev = Some(current);
                match current.as_ref().unwrap().next {
                    Some(next) => current = next,
                    None => break,
                }
            }
            
            // Insert new_node
            (*new_node).next = Some(current);
            if let Some(prev_node) = prev {
                (*prev_node).next = Some(new_node);
            } else {
                // Insert at the beginning of the bucket
                *bucket_ptr = Some(new_node);
            }
        }
        
        // Increment free list size
        self.free_list_size.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: DualLevelAllocator is safe to send between threads (though bootloader
// typically runs single-threaded). The atomic access ensures visibility.
unsafe impl Send for DualLevelAllocator {}
unsafe impl Sync for DualLevelAllocator {}

// SAFETY: GlobalAlloc implementation
// - Small allocations: Try segregated free list first, then bump
// - Large allocations: Direct bump allocation
// - Deallocation: Add blocks to segregated free list for reuse with merging
// - Alignment: All allocations aligned to 64-byte boundaries
unsafe impl GlobalAlloc for DualLevelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let needed_size = layout.size();
        
        // Try segregated free list for small blocks
        if needed_size <= SMALL_BLOCK_THRESHOLD {
            if let Some(ptr) = self.alloc_from_free_list(needed_size) {
                return ptr;
            }
        }
        
        // Fall back to bump allocator
        let current = self.offset.load(Ordering::Relaxed);
        let heap_ptr = self.heap.get() as *mut u8;
        let aligned = Self::align_up(current, BOOTLOADER_HEAP_ALIGN);
        let new_offset = aligned + needed_size;

        // Check if allocation would exceed heap
        if new_offset > BOOTLOADER_HEAP_SIZE {
            return core::ptr::null_mut();
        }

        // Try to update offset atomically
        match self.offset.compare_exchange(
            current,
            new_offset,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => heap_ptr.add(aligned),
            Err(actual) => {
                // Retry with actual offset
                let aligned = Self::align_up(actual, BOOTLOADER_HEAP_ALIGN);
                let new_offset = aligned + needed_size;

                if new_offset > BOOTLOADER_HEAP_SIZE {
                    return core::ptr::null_mut();
                }

                if self.offset.compare_exchange(
                    actual,
                    new_offset,
                    Ordering::Release,
                    Ordering::Relaxed,
                ).is_ok() {
                    heap_ptr.add(aligned)
                } else {
                    core::ptr::null_mut()
                }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Add small blocks to segregated free list for reuse with merging
        if layout.size() <= SMALL_BLOCK_THRESHOLD {
            self.add_to_free_list(ptr, layout.size());
        }
        // Large blocks are simply discarded (bump allocator doesn't support release)
    }
}

/// Global allocator instance
#[global_allocator]
static ALLOCATOR: DualLevelAllocator = DualLevelAllocator::new();

/// Allocation error handler
#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    // Get allocator stats for debugging
    let utilization = ALLOCATOR.utilization();
    
    panic!(
        "Bootloader memory allocation failed: {} bytes\n\
         Heap usage: {} bytes / {} bytes ({:.1}%)",
        layout.size(),
        ALLOCATOR.allocated(),
        BOOTLOADER_HEAP_SIZE,
        utilization
    );
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation() {
        let allocator = DualLevelAllocator::new();
        
        // Small allocation should succeed
        let layout = Layout::new::<u32>();
        unsafe {
            let ptr = allocator.alloc(layout);
            assert!(!ptr.is_null());
            
            // Write to allocated memory
            *(ptr as *mut u32) = 42;
            assert_eq!(*(ptr as *mut u32), 42);
        }
    }

    #[test]
    fn test_alignment() {
        let allocator = DualLevelAllocator::new();
        
        // 64-byte aligned allocation
        let layout = Layout::from_size_align(32, 64).unwrap();
        unsafe {
            let ptr = allocator.alloc(layout);
            assert!(!ptr.is_null());
            // Allocator aligns to 64 bytes
            assert_eq!(ptr as usize % 64, 0);
        }
    }

    #[test]
    fn test_deallocation_and_reuse() {
        let allocator = DualLevelAllocator::new();
        
        // Allocate small block
        let layout = Layout::new::<u64>();
        let ptr1 = unsafe { allocator.alloc(layout) };
        assert!(!ptr1.is_null());
        
        // Deallocate
        unsafe {
            allocator.dealloc(ptr1, layout);
        }
        
        // Allocate again - should reuse freed block
        let ptr2 = unsafe { allocator.alloc(layout) };
        assert!(!ptr2.is_null());
        assert_eq!(ptr1, ptr2); // Same location
    }

    #[test]
    fn test_free_list_capacity() {
        let allocator = DualLevelAllocator::new();
        let layout = Layout::new::<u32>();
        
        // Allocate and deallocate many small blocks
        let mut pointers = Vec::new();
        for _ in 0..200 {
            let ptr = unsafe { allocator.alloc(layout) };
            pointers.push(ptr);
        }
        
        // Deallocate all
        for ptr in pointers {
            unsafe {
                allocator.dealloc(ptr, layout);
            }
        }
        
        // Free list should be bounded
        assert!(allocator.free_list_size.load(Ordering::Relaxed) <= MAX_FREE_LIST_SIZE);
    }

    #[test]
    fn test_large_block_allocation() {
        let allocator = DualLevelAllocator::new();
        
        // Large allocation (> 256 bytes) should use bump allocator
        let large_layout = Layout::from_size_align(512, 1).unwrap();
        unsafe {
            let ptr = allocator.alloc(large_layout);
            assert!(!ptr.is_null());
            
            // Large block should be aligned to 64 bytes
            assert_eq!(ptr as usize % 64, 0);
        }
    }
    
    #[test]
    fn test_memory_merging() {
        let allocator = DualLevelAllocator::new();
        
        // Allocate two adjacent small blocks
        let layout1 = Layout::from_size_align(64, 64).unwrap();
        let layout2 = Layout::from_size_align(64, 64).unwrap();
        
        unsafe {
            let ptr1 = allocator.alloc(layout1);
            assert!(!ptr1.is_null());
            
            let ptr2 = allocator.alloc(layout2);
            assert!(!ptr2.is_null());
            
            // Check if they are adjacent
            assert_eq!(ptr1.add(64), ptr2);
            
            // Deallocate both
            allocator.dealloc(ptr1, layout1);
            allocator.dealloc(ptr2, layout2);
            
            // Allocate a larger block that would require merging
            let large_layout = Layout::from_size_align(128, 64).unwrap();
            let merged_ptr = allocator.alloc(large_layout);
            assert!(!merged_ptr.is_null());
            assert_eq!(merged_ptr, ptr1); // Should use merged block
        }
    }
}
