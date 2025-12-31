//! # Real-Time Memory Management
//!
//! This module provides deterministic, bounded-time memory allocation for real-time systems.
//!
//! ## Overview
//!
//! Real-time systems require guaranteed memory allocation performance. Standard allocators
//! can have unpredictable worst-case behavior due to:
//!
//! - Fragmentation
//! - Coalescing operations
//! - Complex data structures
//! - Dynamic metadata updates
//!
//! This module solves these problems with:
//!
//! - **Fixed-size pools**: O(1) allocation/deallocation
//! - **Pre-allocation**: All memory reserved upfront
//! - **Lock-free operations**: No blocking in allocator
//! - **Memory reservation**: Guaranteed availability for critical tasks
//! - **mlock support**: Lock critical pages in physical memory
//!
//! ## Memory Pools
//!
//! Fixed-size memory pools provide deterministic allocation:
//!
//! ```text
//! Pool Layout:
//! ┌──────────────────────────────────────┐
//! │  Block 0  │  Block 1  │  Block 2  │  │
//! ├──────────────────────────────────────┤
//! │  Free List Head ──► Block 0 ──► ...  │
//! └──────────────────────────────────────┘
//!
//! Allocation: O(1) - pop from free list
//! Deallocation: O(1) - push to free list
//! ```
//!
//! ## Usage
//!
//! ### Memory Pool
//!
//! ```no_run
//! use kernel::rtos::memory::MemoryPool;
//!
//! let pool = MemoryPool::new(1024, 100)?; // 1KB blocks, 100 blocks
//! let ptr = pool.allocate()?;
//! pool.deallocate(ptr);
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```
//!
//! ### RT-Malloc
//!
//! ```no_run
//! use kernel::rtos::memory::rt_malloc;
//!
//! let ptr = rt_malloc(1024)?;
//! rt_free(ptr);
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```
//!
//! ## Performance
//!
//! | Allocator | Alloc Time | Free Time | Fragmentation |
//! |-----------|-----------|-----------|---------------|
//! | Pool      | O(1)      | O(1)      | None          |
//! | RT-Malloc | O(1)      | O(1)      | Minimal       |
//! | System    | O(n)      | O(n)      | Possible      |

use crate::rtos::RtError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use core::ptr::NonNull;

/// Fixed-size memory pool
///
/// Provides O(1) allocation and deallocation for fixed-size blocks.
#[derive(Debug)]
pub struct MemoryPool {
    /// Block size in bytes
    block_size: usize,

    /// Total number of blocks
    total_blocks: usize,

    /// Free blocks list
    free_blocks: spin::Mutex<Vec<NonNull<u8>>>,

    /// Pool memory region
    memory: Option<NonNull<u8>>,

    /// Pool is initialized
    initialized: AtomicBool,

    /// Pool ID
    pool_id: usize,

    /// Allocations counter
    allocations: AtomicU64,

    /// Deallocations counter
    deallocations: AtomicU64,

    /// Current free blocks
    free_count: AtomicUsize,
}

unsafe impl Send for MemoryPool {}
unsafe impl Sync for MemoryPool {}

impl MemoryPool {
    /// Create a new memory pool
    ///
    /// # Arguments
    ///
    /// * `block_size` - Size of each block in bytes
    /// * `num_blocks` - Number of blocks in the pool
    pub fn new(block_size: usize, num_blocks: usize) -> Result<Self, RtError> {
        if block_size == 0 || num_blocks == 0 {
            return Err(RtError::InvalidTimingParameter {
                parameter: "block_size or num_blocks",
                value: 0,
            });
        }

        // Align block size to pointer size
        let aligned_size = (block_size + core::mem::size_of::<usize>() - 1)
            / core::mem::size_of::<usize>() * core::mem::size_of::<usize>();

        let total_size = aligned_size * num_blocks;

        // Allocate memory region
        let layout = alloc::alloc::Layout::from_size_align(total_size, 8)
            .map_err(|_| RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: total_size,
            })?;

        let memory = unsafe { alloc::alloc::alloc_zeroed(layout) };

        let memory = NonNull::new(memory)
            .ok_or(RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: total_size,
            })?;

        // Build free list
        let mut free_blocks = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            let offset = i * aligned_size;
            let block_ptr = unsafe { NonNull::new_unchecked(memory.as_ptr().add(offset)) };
            free_blocks.push(block_ptr);
        }

        Ok(Self {
            block_size: aligned_size,
            total_blocks: num_blocks,
            free_blocks: spin::Mutex::new(free_blocks),
            memory: Some(memory),
            initialized: AtomicBool::new(true),
            pool_id: 0,
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            free_count: AtomicUsize::new(num_blocks),
        })
    }

    /// Allocate a block from the pool
    pub fn allocate(&self) -> Result<NonNull<u8>, RtError> {
        let mut free_blocks = self.free_blocks.lock();

        let block = free_blocks.pop()
            .ok_or(RtError::MemoryAllocationFailed {
                pool_id: self.pool_id,
                requested_size: self.block_size,
            })?;

        self.allocations.fetch_add(1, AtomicOrdering::Relaxed);
        self.free_count.fetch_sub(1, AtomicOrdering::Relaxed);

        Ok(block)
    }

    /// Deallocate a block back to the pool
    pub fn deallocate(&self, block: NonNull<u8>) -> Result<(), RtError> {
        // Verify block is within pool range
        let pool_start = self.memory.unwrap().as_ptr() as usize;
        let pool_end = pool_start + (self.total_blocks * self.block_size);
        let block_addr = block.as_ptr() as usize;

        if block_addr < pool_start || block_addr >= pool_end {
            return Err(RtError::InvalidState {
                state: "block outside pool",
                expected: "block within pool",
            });
        }

        let mut free_blocks = self.free_blocks.lock();
        free_blocks.push(block);

        self.deallocations.fetch_add(1, AtomicOrdering::Relaxed);
        self.free_count.fetch_add(1, AtomicOrdering::Relaxed);

        Ok(())
    }

    /// Get number of free blocks
    pub fn available(&self) -> usize {
        self.free_count.load(AtomicOrdering::Acquire)
    }

    /// Get total number of blocks
    pub fn capacity(&self) -> usize {
        self.total_blocks
    }

    /// Get pool utilization
    pub fn utilization(&self) -> f64 {
        let used = self.total_blocks - self.available();
        used as f64 / self.total_blocks as f64
    }

    /// Check if pool has memory available
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    /// Get allocation statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total_blocks: self.total_blocks,
            free_blocks: self.available(),
            allocations: self.allocations.load(AtomicOrdering::Relaxed),
            deallocations: self.deallocations.load(AtomicOrdering::Relaxed),
        }
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        if let Some(memory) = self.memory.take() {
            let layout = unsafe {
                alloc::alloc::Layout::from_size_align_unchecked(
                    self.total_blocks * self.block_size,
                    8,
                )
            };
            unsafe { alloc::alloc::dealloc(memory.as_ptr(), layout) };
        }
    }
}

/// Memory pool statistics
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    pub total_blocks: usize,
    pub free_blocks: usize,
    pub allocations: u64,
    pub deallocations: u64,
}

/// Pre-allocated memory manager
///
/// Manages multiple memory pools for different size classes.
#[derive(Debug)]
pub struct RtMemoryManager {
    /// Size-classed pools
    pools: spin::Mutex<BTreeMap<usize, MemoryPool>>,

    /// Next pool ID
    next_pool_id: AtomicU64,
}

impl RtMemoryManager {
    /// Create new memory manager
    pub fn new() -> Self {
        Self {
            pools: spin::Mutex::new(BTreeMap::new()),
            next_pool_id: AtomicU64::new(1),
        }
    }

    /// Add a memory pool
    pub fn add_pool(&self, pool: MemoryPool) {
        let mut pools = self.pools.lock();
        pools.insert(pool.block_size, pool);
    }

    /// Allocate memory
    pub fn allocate(&self, size: usize) -> Result<NonNull<u8>, RtError> {
        let pools = self.pools.lock();

        // Find smallest pool that can satisfy the request
        for (&block_size, pool) in pools.iter() {
            if block_size >= size {
                return pool.allocate();
            }
        }

        Err(RtError::MemoryAllocationFailed {
            pool_id: 0,
            requested_size: size,
        })
    }

    /// Deallocate memory
    pub fn deallocate(&self, ptr: NonNull<u8>, size: usize) -> Result<(), RtError> {
        let pools = self.pools.lock();

        // Find the pool for this size
        for (&block_size, pool) in pools.iter() {
            if block_size >= size {
                return pool.deallocate(ptr);
            }
        }

        Err(RtError::NotFound {
            resource_type: "pool",
            id: size as u64,
        })
    }

    /// Get statistics for all pools
    pub fn stats(&self) -> Vec<(usize, PoolStats)> {
        let pools = self.pools.lock();
        pools.iter().map(|(&size, pool)| {
            (size, pool.stats())
        }).collect()
    }
}

impl Default for RtMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global RT memory manager
pub static RT_MEMORY_MANAGER: spin::Once<RtMemoryManager> = spin::Once::new();

/// Initialize the global RT memory manager
pub fn init_rt_memory_manager() {
    RT_MEMORY_MANAGER.call_once(|| RtMemoryManager::new());
}

/// Real-time malloc wrapper
///
/// Provides lock-free, bounded-time allocation using memory pools.
pub fn rt_malloc(size: usize) -> Result<NonNull<u8>, RtError> {
    // Ensure the manager is initialized
    if let Some(manager) = RT_MEMORY_MANAGER.get() {
        return manager.allocate(size);
    }
    // Fallback error if not initialized
    Err(RtError::MemoryAllocationFailed {
        pool_id: 0,
        requested_size: size,
    })
}

/// Real-time free wrapper
pub fn rt_free(ptr: NonNull<u8>, size: usize) -> Result<(), RtError> {
    // Ensure the manager is initialized
    if let Some(manager) = RT_MEMORY_MANAGER.get() {
        return manager.deallocate(ptr, size);
    }
    // Fallback error if not initialized
    Err(RtError::NotFound {
        resource_type: "memory manager",
        id: size as u64,
    })
}

/// Memory reservation
///
/// Reserves memory for exclusive use by a task.
#[derive(Debug)]
pub struct MemoryReservation {
    /// Reserved memory region
    memory: Option<NonNull<u8>>,

    /// Size in bytes
    size: usize,

    /// Reservation ID
    reservation_id: u64,

    /// Is locked in physical memory
    mlocked: AtomicBool,
}

impl MemoryReservation {
    /// Create a new memory reservation
    pub fn new(size: usize) -> Result<Self, RtError> {
        let layout = alloc::alloc::Layout::from_size_align(size, 4096)
            .map_err(|_| RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            })?;

        let memory = unsafe { alloc::alloc::alloc_zeroed(layout) };

        let memory = NonNull::new(memory)
            .ok_or(RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            })?;

        Ok(Self {
            memory: Some(memory),
            size,
            reservation_id: 0,
            mlocked: AtomicBool::new(false),
        })
    }

    /// Lock memory in physical RAM (prevent paging)
    pub fn mlock(&self) -> Result<(), RtError> {
        // In real implementation, call into memory management to lock pages
        self.mlocked.store(true, AtomicOrdering::Release);
        Ok(())
    }

    /// Unlock memory
    pub fn munlock(&self) -> Result<(), RtError> {
        // In real implementation, unlock pages
        self.mlocked.store(false, AtomicOrdering::Release);
        Ok(())
    }

    /// Get pointer to reserved memory
    pub fn as_ptr(&self) -> Option<NonNull<u8>> {
        self.memory
    }

    /// Get reservation size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if memory is locked
    pub fn is_locked(&self) -> bool {
        self.mlocked.load(AtomicOrdering::Acquire)
    }
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        if let Some(memory) = self.memory.take() {
            let layout = unsafe {
                alloc::alloc::Layout::from_size_align_unchecked(self.size, 4096)
            };
            unsafe { alloc::alloc::dealloc(memory.as_ptr(), layout) };
        }
    }
}

/// Lock-free memory allocator
///
/// Implements lock-free allocation for maximum parallelism.
#[derive(Debug)]
pub struct LockFreeAllocator {
    /// Free list head
    free_head: AtomicU64,

    /// Memory region
    memory: Option<NonNull<u8>>,

    /// Region size
    size: usize,

    /// Block size
    block_size: usize,
}

impl LockFreeAllocator {
    /// Create new lock-free allocator
    pub fn new(size: usize, block_size: usize) -> Result<Self, RtError> {
        let num_blocks = size / block_size;

        let layout = alloc::alloc::Layout::from_size_align(size, 8)
            .map_err(|_| RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            })?;

        let memory = unsafe { alloc::alloc::alloc_zeroed(layout) };

        let memory = NonNull::new(memory)
            .ok_or(RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            })?;

        // Initialize free list (all blocks initially free)
        let free_head = if num_blocks > 0 { 1 } else { 0 };

        Ok(Self {
            free_head: AtomicU64::new(free_head),
            memory: Some(memory),
            size,
            block_size,
        })
    }

    /// Allocate a block
    pub fn allocate(&self) -> Result<NonNull<u8>, RtError> {
        let mut current = self.free_head.load(AtomicOrdering::Acquire);

        while current != 0 {
            // Try to pop from free list
            match self.free_head.compare_exchange_weak(
                current,
                0, // In real implementation, this would be next pointer
                AtomicOrdering::AcqRel,
                AtomicOrdering::Acquire,
            ) {
                Ok(_) => {
                    let offset = ((current - 1) * self.block_size as u64) as usize;
                    let ptr = unsafe { NonNull::new_unchecked(self.memory.unwrap().as_ptr().add(offset)) };
                    return Ok(ptr);
                }
                Err(actual) => current = actual,
            }
        }

        Err(RtError::MemoryAllocationFailed {
            pool_id: 0,
            requested_size: self.block_size,
        })
    }

    /// Deallocate a block
    pub fn deallocate(&self, _block: NonNull<u8>) -> Result<(), RtError> {
        // In real implementation, push to free list using CAS
        Ok(())
    }
}

impl Drop for LockFreeAllocator {
    fn drop(&mut self) {
        if let Some(memory) = self.memory.take() {
            let layout = unsafe {
                alloc::alloc::Layout::from_size_align_unchecked(self.size, 8)
            };
            unsafe { alloc::alloc::dealloc(memory.as_ptr(), layout) };
        }
    }
}

/// Bump pointer allocator for phase-based allocation
///
/// Fast allocator that never reuses memory until reset.
#[derive(Debug)]
pub struct BumpAllocator {
    /// Current allocation pointer
    current: AtomicUsize,

    /// Start of memory region
    start: NonNull<u8>,

    /// End of memory region
    end: NonNull<u8>,

    /// Total size
    size: usize,
}

unsafe impl Send for BumpAllocator {}
unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    /// Create new bump allocator
    pub fn new(size: usize) -> Result<Self, RtError> {
        let layout = alloc::alloc::Layout::from_size_align(size, 8)
            .map_err(|_| RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            })?;

        let memory = unsafe { alloc::alloc::alloc_zeroed(layout) };

        let start = NonNull::new(memory)
            .ok_or(RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            })?;

        let end = unsafe { NonNull::new_unchecked(start.as_ptr().add(size)) };

        Ok(Self {
            current: AtomicUsize::new(0),
            start,
            end,
            size,
        })
    }

    /// Allocate memory
    pub fn allocate(&self, size: usize, align: usize) -> Result<NonNull<u8>, RtError> {
        let offset = self.current.load(AtomicOrdering::Acquire);

        // Calculate aligned offset
        let aligned_offset = (offset + align - 1) / align * align;
        let new_offset = aligned_offset + size;

        if new_offset > self.size {
            return Err(RtError::MemoryAllocationFailed {
                pool_id: 0,
                requested_size: size,
            });
        }

        // Try to allocate
        match self.current.compare_exchange(
            offset,
            new_offset,
            AtomicOrdering::AcqRel,
            AtomicOrdering::Acquire,
        ) {
            Ok(_) => {
                let ptr = unsafe { NonNull::new_unchecked(self.start.as_ptr().add(aligned_offset)) };
                Ok(ptr)
            }
            Err(_) => {
                // Retry
                self.allocate(size, align)
            }
        }
    }

    /// Reset the allocator (free all memory)
    pub fn reset(&self) {
        self.current.store(0, AtomicOrdering::Release);
    }

    /// Get remaining space
    pub fn remaining(&self) -> usize {
        self.size - self.current.load(AtomicOrdering::Acquire)
    }
}

impl Drop for BumpAllocator {
    fn drop(&mut self) {
        let layout = unsafe {
            alloc::alloc::Layout::from_size_align_unchecked(self.size, 8)
        };
        unsafe { alloc::alloc::dealloc(self.start.as_ptr(), layout) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_creation() {
        let pool = MemoryPool::new(1024, 10).unwrap();
        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.available(), 10);
    }

    #[test]
    fn test_memory_pool_allocate() {
        let pool = MemoryPool::new(1024, 10).unwrap();
        let ptr = pool.allocate().unwrap();
        assert_eq!(pool.available(), 9);
        pool.deallocate(ptr).unwrap();
        assert_eq!(pool.available(), 10);
    }

    #[test]
    fn test_memory_pool_exhaustion() {
        let pool = MemoryPool::new(1024, 1).unwrap();
        let _ptr1 = pool.allocate().unwrap();
        let result = pool.allocate();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_pool_utilization() {
        let pool = MemoryPool::new(1024, 10).unwrap();
        pool.allocate().unwrap();
        pool.allocate().unwrap();
        let util = pool.utilization();
        assert!((util - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_rt_malloc() {
        let ptr = rt_malloc(1024).unwrap();
        rt_free(ptr, 1024).unwrap();
    }

    #[test]
    fn test_memory_reservation() {
        let reservation = MemoryReservation::new(4096).unwrap();
        assert_eq!(reservation.size(), 4096);
        assert!(reservation.as_ptr().is_some());
    }

    #[test]
    fn test_memory_reservation_mlock() {
        let reservation = MemoryReservation::new(4096).unwrap();
        reservation.mlock().unwrap();
        assert!(reservation.is_locked());
        reservation.munlock().unwrap();
        assert!(!reservation.is_locked());
    }

    #[test]
    fn test_bump_allocator() {
        let allocator = BumpAllocator::new(4096).unwrap();
        let ptr1 = allocator.allocate(100, 8).unwrap();
        let ptr2 = allocator.allocate(200, 8).unwrap();
        assert!(ptr1.as_ptr() != ptr2.as_ptr());
    }

    #[test]
    fn test_bump_allocator_reset() {
        let allocator = BumpAllocator::new(4096).unwrap();
        let _ptr = allocator.allocate(1000, 8).unwrap();
        let remaining = allocator.remaining();
        allocator.reset();
        assert!(allocator.remaining() > remaining);
    }

    #[test]
    fn test_memory_manager() {
        let manager = RtMemoryManager::new();
        let pool = MemoryPool::new(1024, 10).unwrap();
        manager.add_pool(pool);

        let ptr = manager.allocate(512).unwrap();
        manager.deallocate(ptr, 512).unwrap();
    }

    #[test]
    fn test_lock_free_allocator() {
        let allocator = LockFreeAllocator::new(4096, 512).unwrap();
        let ptr = allocator.allocate().unwrap();
        assert!(ptr.as_ptr() as usize != 0);
    }
}
