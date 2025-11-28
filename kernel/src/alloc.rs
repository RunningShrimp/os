//! Kernel heap allocator - Hybrid approach using Buddy and Slab allocators
//! Reduces fragmentation and improves allocation efficiency

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::Mutex;

// ============================================================================
// Re-export the allocator modules
// ============================================================================

pub mod buddy;
pub mod slab;
pub mod compress;


use self::buddy::BuddyAllocator;
use self::slab::SlabAllocator;

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
        
        // Try slab allocator first for small objects
        if size <= 2048 { // Matches SLAB_SIZES defined in slab.rs
            let mut slab = self.slab.lock();
            let ptr = slab.alloc(layout);
            if !ptr.is_null() {
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                return ptr;
            }
        }

        // Fallback to buddy allocator
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

        let size = layout.size();
        
        // Try slab allocator first for small objects
        if size <= 2048 { // Matches SLAB_SIZES defined in slab.rs
            let mut slab = self.slab.lock();
            slab.dealloc(ptr, layout);
            self.allocation_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Fallback to buddy allocator
        let mut buddy = self.buddy.lock();
        buddy.dealloc(ptr, layout);
        self.allocation_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> (buddy::AllocatorStats, slab::AllocatorStats) {
        let buddy = self.buddy.lock();
        let (slab_used, slab_allocated) = self.slab.lock().stats();
        
        (
            buddy.stats(),
            slab::AllocatorStats { used: slab_used, allocated: slab_allocated }
        )
    }
}

// Extend slab::Stats to match the expected interface
pub struct AllocatorStats {
    pub used: usize,
    pub allocated: usize,
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

/// Get heap statistics
pub fn heap_stats() -> (buddy::AllocatorStats, slab::AllocatorStats) {
    ALLOCATOR.stats()
}

/// Align up to the given alignment
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}