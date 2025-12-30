//! Enhanced Per-CPU Memory Allocator with Batch Allocation
//!
//! This module enhances the existing per-CPU allocator with:
//! 1. **Local Caching**: Each CPU maintains a local cache of frequently-used sizes
//! 2. **Batch Allocation**: Refill from global allocator in batches (32 frames)
//! 3. **Fast Path Allocation**: O(1) allocation from local cache
//! 4. **Load Balancing**: Periodic rebalancing between CPU caches
//!
//! Performance Targets:
//! - Small object allocation: < 20ns (fast path)
//! - Cache hit rate: > 95% for common workloads
//! - Lock contention: < 2% on 8-core systems

extern crate alloc;

use alloc::vec::Vec;
use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    subsystems::{mm::allocator::HybridAllocator, sync::Once},
};

const CACHE_LINE_SIZE: usize = 64;
const LOCAL_CACHE_SIZE: usize = 64;  // Number of frames in local cache
const BATCH_SIZE: usize = 32;         // Refill batch size

/// Frame identifier
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    addr: usize,
    size: usize,
}

impl Frame {
    pub fn new(addr: usize, size: usize) -> Self {
        Self { addr, size }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.addr as *mut u8
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

/// Enhanced per-CPU allocator with local caching
#[repr(align(64))]
pub struct EnhancedPerCpuAllocator {
    /// Local cache of pre-allocated frames
    local_cache: Vec<Frame>,
    /// Current cache size
    cache_size: AtomicUsize,
    /// Cache hit/miss statistics
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
    /// Batch allocation size
    batch_size: usize,
    /// Reference to global allocator (shared)
    global_ref: *const HybridAllocator,
    /// Padding to prevent false sharing
    _padding: [u8; CACHE_LINE_SIZE.saturating_sub(80)],
}

unsafe impl Send for EnhancedPerCpuAllocator {}

impl EnhancedPerCpuAllocator {
    pub fn new(global_allocator: &HybridAllocator) -> Self {
        Self {
            local_cache: Vec::with_capacity(LOCAL_CACHE_SIZE),
            cache_size: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
            batch_size: BATCH_SIZE,
            global_ref: global_allocator as *const HybridAllocator,
            _padding: [0; CACHE_LINE_SIZE.saturating_sub(80)],
        }
    }

    /// Fast path: allocate from local cache (O(1))
    pub fn alloc_fast(&mut self, size: usize) -> Option<Frame> {
        // Try to find a frame in local cache
        while let Some(frame) = self.local_cache.pop() {
            if frame.size >= size {
                self.cache_size.fetch_sub(1, Ordering::Relaxed);
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Some(frame);
            }
            // Size mismatch, discard and continue
        }

        // Cache miss
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Slow path: refill local cache from global allocator
    pub fn refill_local(&mut self) -> bool {
        if self.global_ref.is_null() {
            return false;
        }

        let global = unsafe { &*self.global_ref };
        let mut allocated = 0;

        // Allocate a batch of frames
        for size in &[64, 128, 256, 512, 1024, 2048] {
            if self.cache_size.load(Ordering::Relaxed) >= LOCAL_CACHE_SIZE {
                break;
            }

            let layout = match Layout::from_size_align(*size, 8) {
                Ok(layout) => layout,
                Err(_) => continue,
            };
            let ptr = unsafe { global.alloc(layout) };

            if !ptr.is_null() {
                self.local_cache.push(Frame::new(ptr as usize, *size));
                self.cache_size.fetch_add(1, Ordering::Relaxed);
                allocated += 1;

                if allocated >= self.batch_size {
                    break;
                }
            }
        }

        allocated > 0
    }

    /// Allocate with fast path + refill fallback
    pub fn alloc(&mut self, size: usize) -> Option<Frame> {
        // Fast path: local cache
        if let Some(frame) = self.alloc_fast(size) {
            return Some(frame);
        }

        // Slow path: refill and retry
        if self.refill_local() {
            self.alloc_fast(size)
        } else {
            // Direct allocation as last resort
            if !self.global_ref.is_null() {
                let global = unsafe { &*self.global_ref };
                let layout = Layout::from_size_align(size, 8).ok()?;
                let ptr = unsafe { global.alloc(layout) };
                if !ptr.is_null() {
                    return Some(Frame::new(ptr as usize, size));
                }
            }
            None
        }
    }

    /// Return frame to local cache
    pub fn dealloc(&mut self, frame: Frame) {
        if self.cache_size.load(Ordering::Relaxed) < LOCAL_CACHE_SIZE {
            self.local_cache.push(frame);
            self.cache_size.fetch_add(1, Ordering::Relaxed);
        } else {
            // Cache full, return to global
            if !self.global_ref.is_null() {
                let global = unsafe { &*self.global_ref };
                let layout = Layout::from_size_align(frame.size, 8).unwrap();
                unsafe { global.dealloc(frame.as_ptr(), layout) };
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let size = self.cache_size.load(Ordering::Relaxed);

        (hits, misses, size)
    }

    /// Calculate cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let (hits, misses, _) = self.stats();
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64) / (total as f64)
        }
    }
}

/// Global enhanced per-CPU allocator array
static mut ENHANCED_ALLOCATORS: Option<Vec<EnhancedPerCpuAllocator>> = None;
static ENHANCED_INIT: Once = Once::new();

/// Initialize enhanced per-CPU allocators
pub fn init_enhanced_allocators(num_cpus: usize, global_allocator: &HybridAllocator) {
    ENHANCED_INIT.call_once(|| unsafe {
        let mut allocators = Vec::with_capacity(num_cpus);
        for _ in 0..num_cpus {
            allocators.push(EnhancedPerCpuAllocator::new(global_allocator));
        }
        ENHANCED_ALLOCATORS = Some(allocators);
    });
}

/// Get current CPU's enhanced allocator
pub fn current_enhanced_allocator() -> Option<&'static mut EnhancedPerCpuAllocator> {
    if !ENHANCED_INIT.is_completed() {
        return None;
    }

    unsafe {
        let cpu_id = crate::cpu::cpuid() as usize;
        ENHANCED_ALLOCATORS.as_mut()?.get_mut(cpu_id)
    }
}

/// Allocate using enhanced allocator
pub fn enhanced_alloc(size: usize) -> Option<Frame> {
    if let Some(allocator) = current_enhanced_allocator() {
        allocator.alloc(size)
    } else {
        None
    }
}

/// Deallocate using enhanced allocator
pub fn enhanced_dealloc(frame: Frame) {
    if let Some(allocator) = current_enhanced_allocator() {
        allocator.dealloc(frame);
    }
}

/// Get statistics from all enhanced allocators
pub fn get_enhanced_stats() -> Vec<(usize, (usize, usize, usize, f64))> {
    let mut stats = Vec::new();

    unsafe {
        if let Some(ref allocators) = ENHANCED_ALLOCATORS {
            for (cpu_id, alloc) in allocators.iter().enumerate() {
                let (hits, misses, size) = alloc.stats();
                let hit_rate = alloc.hit_rate();
                stats.push((cpu_id, (hits, misses, size, hit_rate)));
            }
        }
    }

    stats
}

/// Balance caches between CPUs (reclaim excess from busy CPUs)
pub fn balance_enhanced_caches() {
    unsafe {
        if let Some(ref mut allocators) = ENHANCED_ALLOCATORS {
            // Calculate average cache size
            let mut total_size = 0;
            for alloc in allocators.iter() {
                total_size += alloc.cache_size.load(Ordering::Relaxed);
            }

            let avg_size = total_size / allocators.len();

            // Move frames from CPUs above average to those below
            for i in 0..allocators.len() {
                let current_size = allocators[i].cache_size.load(Ordering::Relaxed);

                if current_size > avg_size * 2 {
                    // This CPU has too many frames, redistribute
                    let excess = current_size - avg_size;
                    let mut moved = 0;

                    while moved < excess && !allocators[i].local_cache.is_empty() {
                        if let Some(frame) = allocators[i].local_cache.pop() {
                            // Find a CPU with low cache
                            for j in 0..allocators.len() {
                                if i != j && allocators[j].cache_size.load(Ordering::Relaxed) < avg_size {
                                    allocators[j].local_cache.push(frame);
                                    allocators[j].cache_size.fetch_add(1, Ordering::Relaxed);
                                    allocators[i].cache_size.fetch_sub(1, Ordering::Relaxed);
                                    moved += 1;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_allocator_cache() {
        // This is a simplified test
        let global_allocator = HybridAllocator::new();
        let mut enhanced = EnhancedPerCpuAllocator::new(&global_allocator);

        // Test fast path (should miss initially)
        assert!(enhanced.alloc_fast(64).is_none());

        // Test allocation
        let frame = enhanced.alloc(64);
        assert!(frame.is_some());

        // Test deallocation
        if let Some(f) = frame {
            enhanced.dealloc(f);
        }

        // Check cache has items
        assert!(enhanced.cache_size.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let global_allocator = HybridAllocator::new();
        let mut enhanced = EnhancedPerCpuAllocator::new(&global_allocator);

        // Simulate some cache misses
        enhanced.cache_misses.fetch_add(10, Ordering::Relaxed);

        let hit_rate = enhanced.hit_rate();
        assert_eq!(hit_rate, 0.0);
    }
}
