//! Kernel heap allocator - Hybrid approach using Buddy and Slab allocators
//!
//! Reduces fragmentation and improves allocation efficiency

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use crate::sync::Mutex;

// ============================================================================
// Re-export the allocator modules
// ============================================================================

// pub mod buddy;
// pub mod slab;
// pub mod compress;


use crate::mm::optimized_buddy;
use crate::mm::optimized_slab;
use crate::mm::optimized_buddy::AllocatorStats as BuddyStats;
use crate::mm::optimized_slab::AllocatorStats as SlabStats;
use crate::mm::hugepage;
use crate::mm::optimized_buddy::OptimizedBuddyAllocator;
use crate::mm::optimized_slab::OptimizedSlabAllocator;
use crate::mm::hugepage::HugePageAllocator;
use crate::mm::traits::{UnifiedAllocator, AllocatorWithStats, AllocatorStats};

// ============================================================================
// Hybrid Allocator
// ============================================================================

pub struct HybridAllocator {
    // 使用 Mutex 管理分配器实例
    slab: Mutex<OptimizedSlabAllocator>,
    buddy: Mutex<OptimizedBuddyAllocator>,
    hugepage: Mutex<HugePageAllocator>,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
    peak_allocated_bytes: AtomicUsize,
    current_allocated_bytes: AtomicUsize,
    failed_allocations: AtomicUsize,
}

impl HybridAllocator {
    pub const fn new() -> Self {
        Self {
            slab: Mutex::new(OptimizedSlabAllocator::new()),
            buddy: Mutex::new(OptimizedBuddyAllocator::new()),
            hugepage: Mutex::new(HugePageAllocator::new()),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
            peak_allocated_bytes: AtomicUsize::new(0),
            current_allocated_bytes: AtomicUsize::new(0),
            failed_allocations: AtomicUsize::new(0),
        }
    }
pub unsafe fn init(&self, slab_start: usize, slab_size: usize,
                   buddy_start: usize, buddy_size: usize,
                   page_size: usize) {
    // 使用 SpinLock 替代 Mutex 初始化分配器
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
        if size >= hugepage::HPAGE_2MB {
            let mut hugepage = self.hugepage.lock();
            let ptr = hugepage.alloc(size);
            if !ptr.is_null() {
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                self.track_allocation(size);
                return ptr;
            }
            // Fallback to buddy if hugepage allocation fails
        }
        
        // Try slab allocator first for small objects
        if size <= 2048 { // Matches SLAB_SIZES defined in slab.rs
            let mut slab = self.slab.lock(); // SpinLock for faster access
            let ptr = slab.alloc(layout);
            if !ptr.is_null() {
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                self.track_allocation(size);
                return ptr;
            }
        }

        // Fallback to buddy allocator (SpinLock for faster access)
        let mut buddy = self.buddy.lock();
        let ptr = buddy.alloc(layout);
        if !ptr.is_null() {
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
            self.track_allocation(size);
        } else {
            self.failed_allocations.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }
    
    fn track_allocation(&self, size: usize) {
        let current = self.current_allocated_bytes.fetch_add(size, Ordering::Relaxed) + size;
        // Update peak if necessary (use simple compare-and-swap loop)
        loop {
            let peak = self.peak_allocated_bytes.load(Ordering::Relaxed);
            if current <= peak {
                break;
            }
            if self.peak_allocated_bytes.compare_exchange_weak(
                peak, current, Ordering::Relaxed, Ordering::Relaxed
            ).is_ok() {
                break;
            }
        }
    }

    fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let size = layout.size();
        
        // Track deallocation
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
        self.current_allocated_bytes.fetch_sub(size, Ordering::Relaxed);
        
        // Check if this is a huge page deallocation
        if size >= hugepage::HPAGE_2MB {
            let mut hugepage = self.hugepage.lock();
            hugepage.dealloc(ptr, size);
            self.allocation_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }
        
        // Try slab allocator first for small objects
        if size <= 2048 { // Matches SLAB_SIZES defined in slab.rs
            let mut slab = self.slab.lock(); // SpinLock for faster access
            slab.dealloc(ptr, layout);
            self.allocation_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Fallback to buddy allocator (SpinLock for faster access)
        let mut buddy = self.buddy.lock();
        buddy.dealloc(ptr, layout);
        self.allocation_count.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> (BuddyStats, SlabStats) {
        let buddy = self.buddy.lock();
        let slab_stats = self.slab.lock().stats();
        (buddy.stats(), slab_stats)
    }
    
    /// Get supported huge page sizes
    pub fn get_hugepage_sizes(&self) -> Vec<usize> {
        let hugepage = self.hugepage.lock();
        hugepage.supported_sizes().to_vec()
    }
}

// Implement UnifiedAllocator for HybridAllocator
unsafe impl UnifiedAllocator for HybridAllocator {
    unsafe fn allocate(&self, layout: Layout) -> *mut u8 {
        HybridAllocator::alloc(self, layout)
    }

    unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        HybridAllocator::dealloc(self, ptr, layout)
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

// Implement AllocatorWithStats for HybridAllocator
impl AllocatorWithStats for HybridAllocator {
    fn stats(&self) -> crate::mm::traits::AllocatorStats {
        let total_allocations = self.allocation_count.load(Ordering::Relaxed);
        let total_deallocations = self.deallocation_count.load(Ordering::Relaxed);
        let current_allocated_bytes = self.current_allocated_bytes.load(Ordering::Relaxed);
        let peak_allocated_bytes = self.peak_allocated_bytes.load(Ordering::Relaxed);
        let failed_allocations = self.failed_allocations.load(Ordering::Relaxed);
        
        crate::mm::traits::AllocatorStats {
            total_allocations,
            total_deallocations,
            current_allocated_bytes,
            peak_allocated_bytes,
            failed_allocations,
        }
    }
}

// Extend slab::Stats to match the expected interface
// Note: This is a local stats structure, different from traits::AllocatorStats
pub struct LocalAllocatorStats {
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

/// Get reference to the global allocator
pub fn get_global_allocator() -> &'static HybridAllocator {
    &ALLOCATOR
}

/// Initialize the kernel heap allocator
/// # Safety
/// Must be called exactly once with valid heap bounds
pub unsafe fn init(slab_start: usize, slab_size: usize,
                   buddy_start: usize, buddy_size: usize,
                   page_size: usize) {
    ALLOCATOR.init(slab_start, slab_size, buddy_start, buddy_size, page_size);
}

/// Get heap statistics
pub fn heap_stats() -> (BuddyStats, SlabStats) {
    let buddy = ALLOCATOR.buddy.lock();
    let slab = ALLOCATOR.slab.lock();
    (buddy.stats(), slab.stats())
}

/// Align up to the given alignment
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

// Implement Send/Sync for the allocator types since they are thread-safe
unsafe impl Send for HybridAllocator {}
unsafe impl Sync for HybridAllocator {}
