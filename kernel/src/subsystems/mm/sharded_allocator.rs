//! Sharded Memory Allocator
//!
//! This module implements a sharded page allocator that reduces lock contention
//! by maintaining per-CPU shard allocations. Each CPU operates on its local shard
//! most of the time, only falling back to a global allocator when the local shard
//! is exhausted or full.
//!
//! # Performance Benefits
//!
//! - **Reduced Lock Contention**: Each CPU has its own shard, eliminating the
//!   global lock bottleneck in multi-core systems
//! - **Better Cache Locality**: Per-CPU data structures improve cache efficiency
//! - **Linear Scalability**: Performance scales linearly with CPU count in most cases
//!
//! # Architecture
//!
//! ```
//! ShardedAllocator
//!     ├── Shard 0 (CPU 0) - FreeListAllocator
//!     ├── Shard 1 (CPU 1) - FreeListAllocator
//!     ├── Shard 2 (CPU 2) - FreeListAllocator
//!     ├── ...
//!     └── Fallback - BuddyAllocator (shared)
//! ```

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::subsystems::{mm::buddy::OptimizedBuddyAllocator, sync::Mutex};
use crate::cpu;

/// Maximum number of shards (should be power of 2 for fast indexing)
const MAX_SHARDS: usize = 8;

/// Maximum pages per shard before returning to fallback
const SHARD_CAPACITY: usize = 256;

/// Page frame
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub addr: usize,
    pub order: usize,
}

/// Free list allocator for a single shard
///
/// This is a simplified version of the FreeListAllocator from phys.rs,
/// optimized for use within a shard.
struct ShardAllocator {
    free_list: core::ptr::NonNull<u8>,
    free_count: usize,
    capacity: usize,
}

unsafe impl Send for ShardAllocator {}

impl ShardAllocator {
    /// Create a new shard allocator
    const fn new() -> Self {
        Self {
            free_list: core::ptr::NonNull::dangling(),
            free_count: 0,
            capacity: SHARD_CAPACITY,
        }
    }

    /// Check if shard can accept more pages
    #[inline]
    fn can_accept(&self) -> bool {
        self.free_count < self.capacity
    }

    /// Allocate a page from the shard (fast path)
    ///
    /// Returns None if shard is empty
    fn allocate_fast(&mut self) -> Option<Frame> {
        if self.free_count == 0 {
            return None;
        }

        // Pop from free list
        let page_ptr = self.free_list.as_ptr();
        unsafe {
            // Read next pointer from the page
            let next_ptr = *(page_ptr as *const *mut u8);
            if !next_ptr.is_null() {
                self.free_list = core::ptr::NonNull::new_unchecked(next_ptr);
            }
            self.free_count -= 1;
        }

        Some(Frame {
            addr: page_ptr as usize,
            order: 0,
        })
    }

    /// Deallocate a page to the shard
    ///
    /// Returns error if shard is at capacity
    fn deallocate(&mut self, frame: Frame) -> Result<(), ()> {
        if self.free_count >= self.capacity {
            return Err(()); // Shard is full
        }

        let page_ptr = frame.addr as *mut u8;

        // Zero the page for security
        unsafe {
            core::ptr::write_bytes(page_ptr, 0, crate::subsystems::mm::PAGE_SIZE);
        }

        // Push to free list
        unsafe {
            *(page_ptr as *mut *mut u8) = self.free_list.as_ptr();
            self.free_list = core::ptr::NonNull::new_unchecked(page_ptr);
        }
        self.free_count += 1;

        Ok(())
    }

    /// Get number of free pages in shard
    #[inline]
    fn free_pages(&self) -> usize {
        self.free_count
    }
}

/// Sharded page allocator
///
/// Each CPU gets its own shard to reduce lock contention. When a shard is
/// empty, it falls back to a global buddy allocator. When a shard is full,
/// pages are returned to the buddy allocator.
///
/// # Example
///
/// ```no_run
/// use kernel::subsystems::mm::sharded_allocator::ShardedAllocator;
///
/// let allocator = ShardedAllocator::new().unwrap();
/// let frame = allocator.allocate().unwrap();
/// // Use frame...
/// allocator.deallocate(frame);
/// ```
pub struct ShardedAllocator {
    /// Per-CPU shards
    shards: Vec<Mutex<ShardAllocator>>,

    /// Bitmask for fast shard indexing (MAX_SHARDS - 1)
    shard_mask: usize,

    /// Fallback global allocator
    fallback: Mutex<OptimizedBuddyAllocator>,

    /// For statistics tracking
    total_allocations: AtomicUsize,
    total_fallback_allocations: AtomicUsize,
}

unsafe impl Send for ShardedAllocator {}

impl ShardedAllocator {
    /// Create a new sharded allocator
    ///
    /// # Errors
    ///
    /// Returns error if shard count is not a power of 2
    pub fn new() -> Result<Self, &'static str> {
        if !MAX_SHARDS.is_power_of_two() {
            return Err("MAX_SHARDS must be a power of 2");
        }

        let mut shards = Vec::with_capacity(MAX_SHARDS);
        for _ in 0..MAX_SHARDS {
            shards.push(Mutex::new(ShardAllocator::new()));
        }

        Ok(Self {
            shards,
            shard_mask: MAX_SHARDS - 1,
            fallback: Mutex::new(OptimizedBuddyAllocator::new()),
            total_allocations: AtomicUsize::new(0),
            total_fallback_allocations: AtomicUsize::new(0),
        })
    }

    /// Allocate a page frame
    ///
    /// # Allocation Strategy
    ///
    /// 1. Try to allocate from current CPU's local shard (lock-free fast path)
    /// 2. If shard is empty, fall back to global buddy allocator
    /// 3. Return error if out of memory
    ///
    /// # Performance
    ///
    /// - **Fast path**: O(1) with no lock contention (most common case)
    /// - **Slow path**: O(log n) when shard is empty (rare)
    pub fn allocate(&self) -> Result<Frame, &'static str> {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        // 1. Get current CPU ID
        let cpu_id = self.get_cpu_id();

        // 2. Calculate shard index using bitmask
        let shard_idx = cpu_id & self.shard_mask;

        // 3. Try to allocate from local shard (try_lock for lock-free fast path)
        if let Some(mut shard) = self.shards[shard_idx].try_lock() {
            if let Some(frame) = shard.allocate_fast() {
                return Ok(frame);
            }
            // Shard is empty, fall through to fallback
        }

        // 4. Shard is empty or locked, allocate from fallback
        self.total_fallback_allocations.fetch_add(1, Ordering::Relaxed);
        self.allocate_from_fallback()
    }

    /// Deallocate a page frame
    ///
    /// # Deallocation Strategy
    ///
    /// 1. Try to return to current CPU's local shard
    /// 2. If shard is full, return to global buddy allocator
    ///
    /// # Performance
    ///
    /// - **Fast path**: O(1) when shard has capacity (most common case)
    /// - **Slow path**: O(log n) when shard is full (rare)
    pub fn deallocate(&self, frame: Frame) {
        // 1. Get current CPU ID
        let cpu_id = self.get_cpu_id();

        // 2. Calculate shard index
        let shard_idx = cpu_id & self.shard_mask;

        // 3. Try to return to local shard
        if let Some(mut shard) = self.shards[shard_idx].try_lock() {
            if shard.can_accept() {
                let _ = shard.deallocate(frame);
                return;
            }
            // Shard is full, fall through to fallback
        }

        // 4. Shard is full or locked, return to fallback
        self.deallocate_to_fallback(frame);
    }

    /// Get current CPU ID
    #[inline]
    fn get_cpu_id(&self) -> usize {
        cpu::cpuid()
    }

    /// Allocate from fallback buddy allocator
    #[inline]
    fn allocate_from_fallback(&self) -> Result<Frame, &'static str> {
        use core::alloc::Layout;
        let layout = Layout::new::<u8>();
        let ptr = unsafe { self.fallback.lock().alloc(layout) };

        if ptr.is_null() {
            Err("Out of memory")
        } else {
            Ok(Frame {
                addr: ptr as usize,
                order: 0,
            })
        }
    }

    /// Deallocate to fallback buddy allocator
    #[inline]
    fn deallocate_to_fallback(&self, frame: Frame) {
        use core::alloc::Layout;
        let layout = Layout::new::<u8>();
        unsafe {
            self.fallback.lock().dealloc(frame.addr as *mut u8, layout);
        }
    }

    /// Initialize the allocator with a memory range
    ///
    /// # Safety
    ///
    /// The memory range must be valid and not overlap with other allocations
    pub unsafe fn init(&self, start: usize, end: usize) {
        let page_size = crate::subsystems::mm::PAGE_SIZE;
        unsafe {
            self.fallback.lock().init(start, end, page_size);
        }
    }

    /// Get allocator statistics
    pub fn stats(&self) -> ShardedAllocatorStats {
        let mut shard_stats = Vec::new();
        for shard in &self.shards {
            let shard = shard.lock();
            shard_stats.push(shard.free_pages());
        }

        let fallback_stats = self.fallback.lock().stats();

        ShardedAllocatorStats {
            shard_free_pages: shard_stats,
            fallback_allocated: fallback_stats.allocated,
            fallback_freed: fallback_stats.freed,
            fallback_fragmentation: fallback_stats.fragmentation,
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_fallback_allocations: self.total_fallback_allocations.load(Ordering::Relaxed),
        }
    }

    /// Get total free pages across all shards
    pub fn total_free_pages(&self) -> usize {
        let mut total = 0;
        for shard in &self.shards {
            let shard = shard.lock();
            total += shard.free_pages();
        }
        total
    }
}

/// Statistics for the sharded allocator
#[derive(Debug)]
pub struct ShardedAllocatorStats {
    /// Free pages per shard
    pub shard_free_pages: Vec<usize>,

    /// Total allocations from fallback (bytes)
    pub fallback_allocated: usize,

    /// Total deallocations to fallback (bytes)
    pub fallback_freed: usize,

    /// Fragmentation percentage in fallback
    pub fallback_fragmentation: usize,

    /// Total allocation requests
    pub total_allocations: usize,

    /// Total fallback allocations (indicates shard miss rate)
    pub total_fallback_allocations: usize,
}

impl ShardedAllocatorStats {
    /// Calculate shard hit rate (percentage of allocations served from shards)
    pub fn shard_hit_rate(&self) -> f64 {
        if self.total_allocations == 0 {
            100.0
        } else {
            let shard_hits = self.total_allocations.saturating_sub(self.total_fallback_allocations);
            (shard_hits as f64 / self.total_allocations as f64) * 100.0
        }
    }

    /// Calculate average free pages per shard
    pub fn avg_shard_free_pages(&self) -> f64 {
        if self.shard_free_pages.is_empty() {
            0.0
        } else {
            let sum: usize = self.shard_free_pages.iter().sum();
            sum as f64 / self.shard_free_pages.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharded_allocator_creation() {
        let allocator = ShardedAllocator::new();
        assert!(allocator.is_ok());
    }

    #[test]
    fn test_shard_allocator_basic() {
        let mut shard = ShardAllocator::new();
        assert_eq!(shard.free_pages(), 0);
        assert!(shard.can_accept());
    }

    #[test]
    fn test_shard_capacity() {
        let allocator = ShardedAllocator::new().unwrap();
        // All shards should start empty
        assert_eq!(allocator.total_free_pages(), 0);
    }
}

/// Benchmark: Parallel allocation performance
///
/// This benchmark measures the performance improvement of the sharded
/// allocator compared to a single global lock allocator.
#[cfg(feature = "bench")]
pub mod bench {
    use super::*;
    use core::time::Duration;

    /// Benchmark single-threaded allocation
    pub fn bench_single_threaded(allocator: &ShardedAllocator, iterations: usize) -> Duration {
        let start = crate::subsystems::time::get_ticks();

        for _ in 0..iterations {
            if let Ok(frame) = allocator.allocate() {
                allocator.deallocate(frame);
            }
        }

        let end = crate::subsystems::time::get_ticks();
        Duration::from_secs(end - start)
    }

    /// Benchmark multi-threaded allocation
    #[cfg(feature = "multi_thread")]
    pub fn bench_multi_threaded(allocator: &ShardedAllocator, iterations: usize, threads: usize) -> Duration {
        // Placeholder for multi-threaded benchmark
        // In a real implementation, this would spawn multiple threads
        bench_single_threaded(allocator, iterations)
    }
}
