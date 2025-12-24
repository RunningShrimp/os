//! Memory pool management
//!
//! This module provides a memory pool implementation for fixed-size objects.
//! Memory pools are pre-allocated blocks of memory that can be used to reduce
//! fragmentation and improve allocation performance.

extern crate alloc;


use core::ptr::null_mut;
use alloc::vec::Vec;
use spin::Mutex;


/// Memory pool statistics.
///
/// Tracks usage and capacity information for a memory pool.
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    /// Total number of blocks in the memory pool.
    pub total_blocks: usize,
    /// Number of blocks currently in use.
    pub used_blocks: usize,
    /// Total size of the memory pool in bytes.
    pub pool_size: usize,
}

/// A memory pool for fixed-size objects
pub struct MemoryPool {
    /// Base address of the memory pool
    base: *mut u8,
    /// Block size (including any internal overhead)
    block_size: usize,
    /// Total number of blocks in the pool
    total_blocks: usize,
    /// Number of used blocks
    used_blocks: usize,
    /// Free list of available blocks
    free_list: *mut u8,
}

impl MemoryPool {
    /// Create a new memory pool
    /// 
    /// # Arguments
    ///
    /// * `base` - Base address of the memory pool
    /// * `size` - Total size of the memory pool in bytes
    /// * `block_size` - Size of each block in the pool
    /// * `alignment` - Alignment requirement for each block
    ///
    /// # Safety
    ///
    /// - `base` must point to valid, contiguous, and accessible memory
    /// - The memory region from `base` to `base + size` must be valid and accessible
    /// - `size` must be large enough to fit at least one aligned block
    /// - `alignment` must be a power of 2 and greater than 0
    /// - The caller must ensure no other code accesses this memory region while the pool is in use
    /// - The memory region must be aligned to the specified `alignment`
    pub unsafe fn new(base: *mut u8, size: usize, block_size: usize, alignment: usize) -> Option<Self> {
        unsafe {
            // Calculate aligned block size
            let aligned_block_size = (block_size + alignment - 1) & !(alignment - 1);
            
            // Calculate maximum number of blocks that fit in the pool
            let max_blocks = size / aligned_block_size;
            if max_blocks == 0 {
                return None;
            }
            
            // Initialize free list
            let mut free_list = null_mut();
            
            // Chain all blocks together in the free list
            for i in (0..max_blocks).rev() {
                let block = base.add(i * aligned_block_size);
                *(block as *mut *mut u8) = free_list;
                free_list = block;
            }

            Some(Self {
                base,
                block_size: aligned_block_size,
                total_blocks: max_blocks,
                used_blocks: 0,
                free_list,
            })
        }
    }

    /// Allocate a block from the memory pool
    pub fn alloc(&mut self) -> *mut u8 {
        if self.free_list.is_null() {
            return null_mut(); // Pool is exhausted
        }

        unsafe {
            // Take the first block from the free list
            let block = self.free_list;
            self.free_list = *(block as *mut *mut u8);
            
            self.used_blocks += 1;
            block
        }
    }

    /// Deallocate a block back to the memory pool
    /// 
    /// # Safety
    ///
    /// The pointer must have been allocated by this memory pool.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8) {
        unsafe {
            // Validate the pointer belongs to this pool
            let ptr_usize = ptr as usize;
            let base_usize = self.base as usize;
            
            if ptr_usize < base_usize || ptr_usize >= base_usize + (self.total_blocks * self.block_size) {
                return; // Invalid pointer
            }
            
            // Add back to the free list
            *(ptr as *mut *mut u8) = self.free_list;
            self.free_list = ptr;
            
            self.used_blocks = self.used_blocks.saturating_sub(1);
        }
    }

    /// Get memory pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total_blocks: self.total_blocks,
            used_blocks: self.used_blocks,
            pool_size: self.total_blocks * self.block_size,
        }
    }

    /// Get the block size used by this pool
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.used_blocks == 0
    }

    /// Check if the pool is full
    pub fn is_full(&self) -> bool {
        self.used_blocks == self.total_blocks
    }
}

// Implement Send/Sync for memory pool when protected by Mutex
unsafe impl Send for MemoryPool {}
unsafe impl Sync for MemoryPool {}

/// Global memory pool registry
struct MemoryPoolRegistry {
    pools: Vec<Mutex<MemoryPool>>,
}

impl MemoryPoolRegistry {
    const fn new() -> Self {
        Self {
            pools: Vec::new(),
        }
    }
}

/// Global memory pool registry instance
static MEMORY_POOL_REGISTRY: Mutex<MemoryPoolRegistry> = Mutex::new(MemoryPoolRegistry::new());

/// Initialize a memory pool and register it globally
///
/// # Safety
///
/// - `base` must point to valid, contiguous, and accessible memory
/// - The memory region from `base` to `base + size` must be valid and accessible
/// - `size` must be large enough to fit at least one aligned block
/// - `alignment` must be a power of 2 and greater than 0
/// - The caller must ensure no other code accesses this memory region while the pool is in use
/// - The memory region must be aligned to the specified `alignment`
/// - The caller must ensure that the memory region remains valid for the lifetime of the registered pool
pub unsafe fn mempool_create(base: *mut u8, size: usize, block_size: usize, alignment: usize) -> Option<usize> {
    if let Some(pool) = unsafe { MemoryPool::new(base, size, block_size, alignment) } {
        let mut registry = MEMORY_POOL_REGISTRY.lock();
        let index = registry.pools.len();
        registry.pools.push(Mutex::new(pool));
        Some(index)
    } else {
        None
    }
}

/// Allocate from a registered memory pool
pub fn mempool_alloc(pool_id: usize) -> *mut u8 {
    let registry = MEMORY_POOL_REGISTRY.lock();
    if pool_id < registry.pools.len() {
        registry.pools[pool_id].lock().alloc()
    } else {
        null_mut()
    }
}

/// Deallocate from a registered memory pool
///
/// # Safety
///
/// - `pool_id` must be a valid pool identifier returned by a previous call to `mempool_create`
/// - `ptr` must have been allocated by the memory pool identified by `pool_id`
/// - `ptr` must not be null
/// - `ptr` must not be double-freed
pub unsafe fn mempool_dealloc(pool_id: usize, ptr: *mut u8) {
    let registry = MEMORY_POOL_REGISTRY.lock();
    if pool_id < registry.pools.len() {
        unsafe {
            registry.pools[pool_id].lock().dealloc(ptr);
        }
    }
}

/// Get statistics for a registered memory pool
pub fn mempool_stats(pool_id: usize) -> Option<PoolStats> {
    let registry = MEMORY_POOL_REGISTRY.lock();
    if pool_id < registry.pools.len() {
        Some(registry.pools[pool_id].lock().stats())
    } else {
        None
    }
}

/// Allocate from a memory pool with specified block size
/// 
/// This function will find or create a memory pool with the requested block size
pub fn mempool_alloc_sized(_block_size: usize, _alignment: usize) -> *mut u8 {
    // For now, we just return null. In a more sophisticated implementation,
    // we would search for an existing pool or create a new one.
    null_mut()
}

/// Deallocate from a memory pool with specified block size
///
/// This function will find the appropriate memory pool based on the block size and deallocate the pointer.
///
/// # Safety
///
/// - `ptr` must have been allocated by a memory pool with the specified `block_size`
/// - `ptr` must not be null
/// - `ptr` must not be double-freed
/// - `block_size` must match the block size of the memory pool that allocated `ptr`
pub unsafe fn mempool_dealloc_sized(_ptr: *mut u8, _block_size: usize) {
    // For now, we just do nothing. In a more sophisticated implementation,
    // we would find the appropriate pool to return the block to.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool() {
        // Allocate a buffer for testing
        let mut buffer = Vec::with_capacity(1024);
        buffer.resize(1024, 0);
        
        unsafe {
            // Create a memory pool with 64-byte blocks
            let mut pool = MemoryPool::new(buffer.as_mut_ptr(), buffer.len(), 64, 8).unwrap();
            
            // Allocate some blocks
            let ptr1 = pool.alloc();
            let ptr2 = pool.alloc();
            let ptr3 = pool.alloc();
            
            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());
            assert!(!ptr3.is_null());
            
            // Check that pool stats are correct
            let stats = pool.stats();
            assert_eq!(stats.used_blocks, 3);
            
            // Deallocate a block
            pool.dealloc(ptr2);
            let stats2 = pool.stats();
            assert_eq!(stats2.used_blocks, 2);
            
            // Allocate another block
            let ptr4 = pool.alloc();
            assert!(!ptr4.is_null());
        }
    }
}
