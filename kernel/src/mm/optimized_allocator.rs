//! Optimized Hybrid Allocator using Optimized Buddy and Slab allocators

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use crate::sync::Mutex;

// ============================================================================
// Re-export the allocator modules
// ============================================================================

use crate::mm::buddy::OptimizedBuddyAllocator;
use crate::mm::slab::OptimizedSlabAllocator;
use crate::mm::hugepage::HugePageAllocator;
use crate::mm::traits::{UnifiedAllocator, AllocatorWithStats, AllocatorStats};

// ============================================================================
// Optimized Hybrid Allocator
// ============================================================================

pub struct OptimizedHybridAllocator {
    slab: Mutex<OptimizedSlabAllocator>,
    buddy: Mutex<OptimizedBuddyAllocator>,
    hugepage: Mutex<HugePageAllocator>,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
    peak_allocated: AtomicUsize,
    failed_allocations: AtomicUsize,
}

impl OptimizedHybridAllocator {
    pub const fn new() -> Self {
        Self {
            slab: Mutex::new(OptimizedSlabAllocator::uninitialized()),
            buddy: Mutex::new(OptimizedBuddyAllocator::new()),
            hugepage: Mutex::new(HugePageAllocator::new()),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
            peak_allocated: AtomicUsize::new(0),
            failed_allocations: AtomicUsize::new(0),
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
        drop(buddy);
        
        // Initialize huge page allocator with a portion of the buddy region
        // Reserve 10% of buddy region for huge pages
        let hugepage_start = buddy_start + (buddy_size * 9 / 10);
        let _hugepage_size = buddy_size / 10;
        let mut hugepage = self.hugepage.lock();
        hugepage.init(hugepage_start, buddy_start + buddy_size);
    }

    fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // Check if this is a huge page allocation (>= 2MB)
        if size >= crate::mm::hugepage::HPAGE_2MB {
            let mut hugepage = self.hugepage.lock();
            let ptr = hugepage.alloc(size);
            if !ptr.is_null() {
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                return ptr;
            }
            // Fallback to buddy if hugepage allocation fails
        }
        
        // Try optimized slab allocator first for small objects
        if size <= 2048 { // Matches SLAB_SIZES defined in optimized_slab.rs
            let mut slab = self.slab.lock();
            let ptr = slab.alloc(layout);
            if !ptr.is_null() {
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                return ptr;
            }
        }

        // Fallback to optimized buddy allocator
        let mut buddy = self.buddy.lock();
        let ptr = buddy.alloc(layout);
        if !ptr.is_null() {
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_allocations.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let size = layout.size();
        
        // Check if this is a huge page deallocation
        if size >= crate::mm::hugepage::HPAGE_2MB {
            let mut hugepage = self.hugepage.lock();
            hugepage.dealloc(ptr, size);
            self.deallocation_count.fetch_add(1, Ordering::Relaxed);
            return;
        }
        
        // Try optimized slab allocator first for small objects
        if size <= 2048 { // Matches SLAB_SIZES defined in optimized_slab.rs
            let mut slab = self.slab.lock();
            unsafe { slab.dealloc(ptr, layout); }
            self.deallocation_count.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Fallback to optimized buddy allocator
        let mut buddy = self.buddy.lock();
        buddy.dealloc(ptr, layout);
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> (crate::mm::buddy::AllocatorStats, crate::mm::slab::AllocatorStats) {
        let buddy = self.buddy.lock();
        let slab_stats = self.slab.lock().stats();

        (
            buddy.stats(),
            slab_stats
        )
    }
    
    /// Get supported huge page sizes
    pub fn get_hugepage_sizes(&self) -> Vec<usize> {
        let hugepage = self.hugepage.lock();
        hugepage.supported_sizes().to_vec()
    }
}

// Implement UnifiedAllocator for OptimizedHybridAllocator
unsafe impl UnifiedAllocator for OptimizedHybridAllocator {
    unsafe fn allocate(&self, layout: Layout) -> *mut u8 {
        OptimizedHybridAllocator::alloc(self, layout)
    }

    unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        OptimizedHybridAllocator::dealloc(self, ptr, layout)
    }

    unsafe fn allocate_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.allocate(layout);
        if !ptr.is_null() {
            core::ptr::write_bytes(ptr, 0, layout.size());
        }
        ptr
    }

    unsafe fn reallocate(
        &self,
        ptr: *mut u8,
        old_layout: Layout,
        new_size: usize,
    ) -> *mut u8 {
        let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        let new_ptr = self.allocate(new_layout);
        if new_ptr.is_null() {
            return null_mut();
        }

        let copy_size = old_layout.size().min(new_size);
        core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        self.deallocate(ptr, old_layout);

        new_ptr
    }
}

// Implement AllocatorWithStats for OptimizedHybridAllocator
impl AllocatorWithStats for OptimizedHybridAllocator {
    fn stats(&self) -> crate::mm::traits::AllocatorStats {
        let (buddy_stats, slab_stats) = self.stats();
        let total_allocations = self.allocation_count.load(Ordering::Relaxed);
        let total_deallocations = self.deallocation_count.load(Ordering::Relaxed);
        let failed_allocations = self.failed_allocations.load(Ordering::Relaxed);
        
        let current_allocated = buddy_stats.allocated - buddy_stats.freed + slab_stats.used;
        
        // Update peak allocated
        let mut peak = self.peak_allocated.load(Ordering::Acquire);
        while current_allocated > peak {
            if self.peak_allocated.compare_exchange(peak, current_allocated, Ordering::Release, Ordering::Acquire).is_ok() {
                peak = current_allocated;
            }
            peak = self.peak_allocated.load(Ordering::Acquire);
        }
        
        crate::mm::traits::AllocatorStats {
            total_allocations,
            total_deallocations,
            current_allocated_bytes: current_allocated,
            peak_allocated_bytes: peak,
            failed_allocations,
        }
    }
}

// Extend slab::Stats to match the expected interface
// Note: This is a local stats structure, different from traits::AllocatorStats

unsafe impl GlobalAlloc for OptimizedHybridAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        OptimizedHybridAllocator::alloc(self, layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        OptimizedHybridAllocator::dealloc(self, ptr, layout)
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