//! Kernel heap allocator - Hybrid approach using Buddy and Slab allocators
//!
//! Reduces fragmentation and improves allocation efficiency

extern crate alloc;

use alloc::vec::Vec;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

// ============================================================================
// Re-export the allocator modules
// ============================================================================

// pub mod buddy;
// pub mod slab;
// pub mod compress;
use crate::{
    subsystems::{
        mm::{
            buddy::{AllocatorStats as BuddyStats, OptimizedBuddyAllocator},
            hugepage,
            hugepage::HugePageAllocator,
            slab::{AllocatorStats as SlabStats, OptimizedSlabAllocator},
            traits::{AllocatorWithStats, UnifiedAllocator},
        },
        sync::Mutex,
    },
};

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
            slab: Mutex::new(OptimizedSlabAllocator::uninitialized()),
            buddy: Mutex::new(OptimizedBuddyAllocator::new()),
            hugepage: Mutex::new(HugePageAllocator::new()),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
            peak_allocated_bytes: AtomicUsize::new(0),
            current_allocated_bytes: AtomicUsize::new(0),
            failed_allocations: AtomicUsize::new(0),
        }
    }
    pub unsafe fn init(
        &self,
        slab_start: usize,
        slab_size: usize,
        buddy_start: usize,
        buddy_size: usize,
        page_size: usize,
    ) {
        // 使用 SpinLock 替代 Mutex 初始化分配器
        let mut slab = self.slab.lock();
        unsafe {
            slab.init(slab_start as *mut u8, slab_size);
        }
        drop(slab);

        let mut buddy = self.buddy.lock();
        unsafe {
            buddy.init(buddy_start, buddy_start + buddy_size, page_size);
        }
        drop(buddy);

        // Initialize huge page allocator with a portion of the buddy region
        // Reserve 10% of buddy region for huge pages
        let hugepage_start = buddy_start + (buddy_size * 9 / 10);
        let _hugepage_size = buddy_size / 10;
        let mut hugepage = self.hugepage.lock();
        unsafe {
            hugepage.init(hugepage_start, buddy_start + buddy_size);
        }
    }

    fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();

        // 快速路径：小对象直接从slab分配（内联优化）
        #[inline(always)]
        fn fast_path_alloc(
            slab: &Mutex<OptimizedSlabAllocator>,
            layout: Layout,
            size: usize,
            alloc_count: &AtomicUsize,
            tracker: &HybridAllocator,
        ) -> *mut u8 {
            if size <= 2048 {
                let mut slab_guard = slab.lock();
                let ptr = unsafe { slab_guard.alloc(layout) };
                if !ptr.is_null() {
                    alloc_count.fetch_add(1, Ordering::Relaxed);
                    tracker.track_allocation(size);
                }
                ptr
            } else {
                core::ptr::null_mut()
            }
        }

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
        let ptr = fast_path_alloc(&self.slab, layout, size, &self.allocation_count, self);
        if !ptr.is_null() {
            return ptr;
        }

        // Fallback to buddy allocator (SpinLock for faster access)
        let mut buddy = self.buddy.lock();
        let ptr = unsafe { buddy.alloc(layout) };
        if !ptr.is_null() {
            self.allocation_count.fetch_add(1, Ordering::Relaxed);
            self.track_allocation(size);
        } else {
            self.failed_allocations.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    /// 快速路径分配（专用于小对象）
    ///
    /// 针对小对象分配优化的快速路径，减少锁竞争。
    #[inline(always)]
    pub fn allocate_fast(&self, size: usize, align: usize) -> *mut u8 {
        use core::alloc::Layout;

        // 创建布局
        let layout = match Layout::from_size_align(size, align) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };

        // 快速路径：仅处理小对象
        if size <= 2048 {
            let mut slab = self.slab.lock();
            let ptr = unsafe { slab.alloc(layout) };
            if !ptr.is_null() {
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                self.track_allocation(size);
            }
            ptr
        } else {
            // 大对象走常规路径
            self.alloc(layout)
        }
    }

    fn track_allocation(&self, size: usize) {
        let current = self
            .current_allocated_bytes
            .fetch_add(size, Ordering::Relaxed)
            + size;
        // Update peak if necessary (use simple compare-and-swap loop)
        loop {
            let peak = self.peak_allocated_bytes.load(Ordering::Relaxed);
            if current <= peak {
                break;
            }
            if self
                .peak_allocated_bytes
                .compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
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
        self.current_allocated_bytes
            .fetch_sub(size, Ordering::Relaxed);

        // Check if this is a huge page deallocation
        if size >= hugepage::HPAGE_2MB {
            let mut hugepage = self.hugepage.lock();
            hugepage.dealloc(ptr, size);
            self.allocation_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Try slab allocator first for small objects
        if size <= 2048 {
            // Matches SLAB_SIZES defined in slab.rs
            let mut slab = self.slab.lock(); // SpinLock for faster access
            unsafe {
                slab.dealloc(ptr, layout);
            }
            self.allocation_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Fallback to buddy allocator (SpinLock for faster access)
        let mut buddy = self.buddy.lock();
        unsafe {
            buddy.dealloc(ptr, layout);
        }
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
        let ptr = unsafe { self.allocate(layout) };
        if !ptr.is_null() {
            unsafe {
                core::ptr::write_bytes(ptr, 0, layout.size());
            }
        }
        ptr
    }

    unsafe fn reallocate(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        let new_ptr = unsafe { self.allocate(new_layout) };
        if new_ptr.is_null() {
            return null_mut();
        }

        let copy_size = old_layout.size().min(new_size);
        unsafe {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
            self.deallocate(ptr, old_layout);
        }

        new_ptr
    }
}

// Implement AllocatorWithStats for HybridAllocator
impl AllocatorWithStats for HybridAllocator {
    fn stats(&self) -> crate::subsystems::mm::traits::AllocatorStats {
        let total_allocations = self.allocation_count.load(Ordering::Relaxed);
        let total_deallocations = self.deallocation_count.load(Ordering::Relaxed);
        let current_allocated_bytes = self.current_allocated_bytes.load(Ordering::Relaxed);
        let peak_allocated_bytes = self.peak_allocated_bytes.load(Ordering::Relaxed);
        let failed_allocations = self.failed_allocations.load(Ordering::Relaxed);

        crate::subsystems::mm::traits::AllocatorStats {
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
            unsafe {
                core::ptr::write_bytes(ptr, 0, layout.size());
            }
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
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
                self.dealloc(ptr, layout);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: HybridAllocator = HybridAllocator::new();

/// Get total memory size from system
fn get_total_memory() -> usize {
    // Try to get memory size from platform-specific sources
    #[cfg(target_arch = "x86_64")]
    {
        // For x86_64, we could read from BIOS or use a fixed value for now
        // In a real implementation, this would query the hardware
        512 * 1024 * 1024 // 512 MB default for x86_64
    }

    #[cfg(target_arch = "aarch64")]
    {
        // For aarch64, we could read from device tree
        // In a real implementation, this would query the device tree
        512 * 1024 * 1024 // 512 MB default for aarch64
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        // Default fallback for other architectures
        256 * 1024 * 1024 // 256 MB default
    }
}

/// Calculate memory pressure as (current_allocated / total) * 10000
pub fn calculate_memory_pressure() -> u64 {
    let current = ALLOCATOR.current_allocated_bytes.load(Ordering::Relaxed);
    let total = get_total_memory();

    // If total memory is 0, return maximum pressure
    if total == 0 {
        return 10000; // Maximum pressure
    }

    // Convert to u64 for calculation to avoid overflow
    let current_u64 = current as u64;
    let total_u64 = total as u64;

    (current_u64 * 10000) / total_u64
}

/// Expose memory metrics for monitoring
pub fn get_memory_metrics() -> (usize, usize, usize) {
    let current = ALLOCATOR.current_allocated_bytes.load(Ordering::Relaxed);
    let peak = ALLOCATOR.peak_allocated_bytes.load(Ordering::Relaxed);
    let failed = ALLOCATOR.failed_allocations.load(Ordering::Relaxed);

    (current, peak, failed)
}

/// Get reference to the global allocator
pub fn get_global_allocator() -> &'static HybridAllocator {
    &ALLOCATOR
}

/// Initialize the kernel heap allocator
/// # Safety
/// Must be called exactly once with valid heap bounds
pub unsafe fn init(
    slab_start: usize,
    slab_size: usize,
    buddy_start: usize,
    buddy_size: usize,
    page_size: usize,
) {
    unsafe {
        ALLOCATOR.init(slab_start, slab_size, buddy_start, buddy_size, page_size);
    }
}

/// Get heap statistics
pub fn heap_stats() -> (BuddyStats, SlabStats) {
    let buddy = ALLOCATOR.buddy.lock();
    let slab = ALLOCATOR.slab.lock();
    (buddy.stats(), slab.stats())
}

/// Align up to the given alignment
// Implement Send/Sync for the allocator types since they are thread-safe
unsafe impl Send for HybridAllocator {}
unsafe impl Sync for HybridAllocator {}
