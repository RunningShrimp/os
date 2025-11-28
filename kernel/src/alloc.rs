//! Kernel heap allocator - Hybrid approach using Buddy and Slab allocators
//! Reduces fragmentation and improves allocation efficiency

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::Mutex;

// ============================================================================
// Buddy Allocator Implementation
// ============================================================================

/// Buddy allocator that manages memory blocks of power-of-2 sizes
struct BuddyAllocator {
    free_lists: [*mut BuddyBlock; 32],
    heap_start: usize,
    heap_end: usize,
    min_block_size: usize,
    statistics: BuddyStats,
}

struct BuddyBlock {
    next: *mut BuddyBlock,
    size: usize,
}

#[derive(Debug, Clone, Copy)]
struct BuddyStats {
    allocated: usize,
    freed: usize,
}

impl BuddyAllocator {
    const fn new() -> Self {
        Self {
            free_lists: [null_mut(); 32],
            heap_start: 0,
            heap_end: 0,
            min_block_size: 4096,
            statistics: BuddyStats { allocated: 0, freed: 0 },
        }
    }

    unsafe fn init(&mut self, heap_start: usize, heap_end: usize, min_block_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_end;
        self.min_block_size = min_block_size;

        if heap_end > heap_start {
            let block = heap_start as *mut BuddyBlock;
            (*block).size = heap_end - heap_start;
            (*block).next = null_mut();
            
            let order = self.get_order((*block).size);
            if order < 32 {
                (*block).next = self.free_lists[order];
                self.free_lists[order] = block;
            }
        }
    }

    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        
        let required_size = if size < self.min_block_size {
            self.min_block_size
        } else {
            self.round_up_power_of_2(size)
        };

        let order = self.get_order(required_size);
        if order >= 32 {
            return null_mut();
        }

        for i in order..32 {
            if !self.free_lists[i].is_null() {
                let block = self.free_lists[i];
                self.free_lists[i] = unsafe { (*block).next };

                let mut current = block;
                let mut current_order = i;
                while current_order > order {
                    current_order -= 1;
                    let block_size = 1 << (current_order + 12);
                    let buddy = unsafe { (current as usize + block_size) as *mut BuddyBlock };
                    
                    unsafe {
                        (*buddy).size = block_size;
                        (*buddy).next = self.free_lists[current_order];
                    }
                    self.free_lists[current_order] = buddy;
                }

                let aligned_ptr = align_up(current as usize, align) as *mut u8;
                self.statistics.allocated += required_size;
                
                return aligned_ptr;
            }
        }

        null_mut()
    }

    fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
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
    }

    fn get_order(&self, size: usize) -> usize {
        let mut order = 0;
        let mut block_size = self.min_block_size;
        while block_size < size && order < 32 {
            block_size *= 2;
            order += 1;
        }
        order
    }

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
}

// ============================================================================
// Slab Allocator Implementation
// ============================================================================

const SLAB_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

struct SlabAllocator {
    // For simplicity, we'll keep a simple implementation
    // In production, this would have more sophisticated slab management
    heap_ptr: *mut u8,
    heap_size: usize,
    allocated: usize,
}

impl SlabAllocator {
    const fn new() -> Self {
        Self {
            heap_ptr: null_mut(),
            heap_size: 0,
            allocated: 0,
        }
    }

    unsafe fn init(&mut self, heap_ptr: *mut u8, heap_size: usize) {
        self.heap_ptr = heap_ptr;
        self.heap_size = heap_size;
        self.allocated = 0;
    }

    fn alloc(&mut self, _layout: Layout) -> *mut u8 {
        null_mut() // Fallback to buddy allocator
    }

    fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {
        // No-op for now
    }
}

// ============================================================================
// Hybrid Allocator
// ============================================================================

pub struct HybridAllocator {
    slab: Mutex<SlabAllocator>,
    buddy: Mutex<BuddyAllocator>,
    allocation_count: AtomicUsize,
}

impl HybridAllocator {
    pub const fn new() -> Self {
        Self {
            slab: Mutex::new(SlabAllocator::new()),
            buddy: Mutex::new(BuddyAllocator::new()),
            allocation_count: AtomicUsize::new(0),
        }
    }

    pub unsafe fn init(&self, slab_start: usize, slab_size: usize,
                       buddy_start: usize, buddy_size: usize,
                       page_size: usize) {
        let mut slab = self.slab.lock();
        slab.init(slab_start as *mut u8, slab_size);
        drop(slab);

        let mut buddy = self.buddy.lock();
        buddy.init(buddy_start, buddy_start + buddy_size, page_size);
    }

    fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // For now, use buddy allocator for all allocations
        // In production, use slab for sizes <= 2048
        let mut buddy = self.buddy.lock();
        let ptr = buddy.alloc(layout);
        if !ptr.is_null() {
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let mut buddy = self.buddy.lock();
        buddy.dealloc(ptr, layout);
        self.allocation_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> AllocatorStats {
        let buddy = self.buddy.lock();
        AllocatorStats {
            allocated: buddy.statistics.allocated,
            freed: buddy.statistics.freed,
            total_allocations: self.allocation_count.load(Ordering::Relaxed),
        }
    }
}

unsafe impl GlobalAlloc for HybridAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HybridAllocator::alloc(self, layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HybridAllocator::dealloc(self, ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc(layout);
        if !ptr.is_null() {
            core::ptr::write_bytes(ptr, 0, layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = match Layout::from_size_align(new_size, layout.align()) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            let copy_size = layout.size().min(new_size);
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    pub allocated: usize,
    pub freed: usize,
    pub total_allocations: usize,
}

#[global_allocator]
static ALLOCATOR: HybridAllocator = HybridAllocator::new();

/// Initialize the kernel heap allocator
/// # Safety
/// Must be called exactly once with valid heap bounds
pub unsafe fn init(slab_start: usize, slab_size: usize,
                   buddy_start: usize, buddy_size: usize,
                   page_size: usize) {
    ALLOCATOR.init(slab_start, slab_size, buddy_start, buddy_size, page_size);
}

/// Get heap statistics: (allocated, freed, allocation_count)
pub fn heap_stats() -> AllocatorStats {
    ALLOCATOR.stats()
}

/// Align up to the given alignment
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}