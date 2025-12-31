//! # Memory Pool Management
//!
//! Efficient memory management for tensor operations with:
//! - Pre-allocated memory pools
//! - Zero-copy buffer reuse
//! - Memory alignment optimization
//! - Concurrent access support

use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Memory pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Initial pool size in bytes
    pub initial_size: usize,
    /// Maximum pool size in bytes
    pub max_size: usize,
    /// Alignment for allocations (power of 2)
    pub alignment: usize,
    /// Minimum block size for reuse
    pub min_block_size: usize,
    /// Whether to grow pool dynamically
    pub allow_growth: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            initial_size: 1024 * 1024, // 1 MB
            max_size: 1024 * 1024 * 1024, // 1 GB
            alignment: 64, // Cache line alignment
            min_block_size: 1024, // 1 KB minimum
            allow_growth: true,
        }
    }
}

/// Memory block in the pool
struct MemoryBlock {
    ptr: *mut u8,
    size: usize,
    aligned_size: usize,
    in_use: bool,
}

unsafe impl Send for MemoryBlock {}

/// Memory pool for tensor allocations
pub struct MemoryPool {
    config: PoolConfig,
    blocks: Mutex<Vec<MemoryBlock>>,
    total_allocated: AtomicUsize,
    total_used: AtomicUsize,
    allocation_count: AtomicUsize,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(config: PoolConfig) -> Result<Self, PoolError> {
        let mut pool = Self {
            config: config.clone(),
            blocks: Mutex::new(Vec::new()),
            total_allocated: AtomicUsize::new(0),
            total_used: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
        };

        // Pre-allocate initial blocks
        pool.grow_pool(config.initial_size)?;

        Ok(pool)
    }

    /// Allocate memory from pool
    pub fn allocate(&self, size: usize) -> Result<MemoryGuard<'_>, PoolError> {
        if size == 0 {
            return Err(PoolError::InvalidSize);
        }

        let aligned_size = self.align_size(size);

        // Try to find a free block
        {
            let mut blocks = self.blocks.lock();
            for block in blocks.iter_mut() {
                if !block.in_use && block.aligned_size >= aligned_size {
                    block.in_use = true;
                    self.total_used.fetch_add(aligned_size, Ordering::SeqCst);
                    self.allocation_count.fetch_add(1, Ordering::SeqCst);

                    return Ok(MemoryGuard {
                        pool: self,
                        ptr: block.ptr,
                        size: block.size,
                        aligned_size: block.aligned_size,
                    });
                }
            }
        }

        // No free block found, try to grow pool
        if self.config.allow_growth {
            let grow_size = aligned_size.max(self.config.initial_size);
            self.grow_pool(grow_size)?;

            // Try allocation again
            {
                let mut blocks = self.blocks.lock();
                for block in blocks.iter_mut() {
                    if !block.in_use && block.aligned_size >= aligned_size {
                        block.in_use = true;
                        self.total_used.fetch_add(aligned_size, Ordering::SeqCst);
                        self.allocation_count.fetch_add(1, Ordering::SeqCst);

                        return Ok(MemoryGuard {
                            pool: self,
                            ptr: block.ptr,
                            size: block.size,
                            aligned_size: block.aligned_size,
                        });
                    }
                }
            }

            Err(PoolError::OutOfMemory)
        } else {
            Err(PoolError::PoolFull)
        }
    }

    /// Deallocate memory back to pool
    fn deallocate(&self, ptr: *mut u8, aligned_size: usize) {
        let mut blocks = self.blocks.lock();
        for block in blocks.iter_mut() {
            if block.ptr == ptr {
                block.in_use = false;
                self.total_used.fetch_sub(aligned_size, Ordering::SeqCst);
                return;
            }
        }
    }

    /// Grow pool by allocating more memory
    fn grow_pool(&self, size: usize) -> Result<(), PoolError> {
        let current_allocated = self.total_allocated.load(Ordering::SeqCst);
        let aligned_size = self.align_size(size);

        if current_allocated + aligned_size > self.config.max_size {
            return Err(PoolError::MaxSizeExceeded);
        }

        // Allocate aligned memory
        let layout = unsafe {
            alloc::alloc::Layout::from_size_align_unchecked(
                aligned_size,
                self.config.alignment,
            )
        };

        let ptr = unsafe { alloc::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(PoolError::AllocationFailed);
        }

        let block = MemoryBlock {
            ptr,
            size: aligned_size,
            aligned_size,
            in_use: false,
        };

        self.blocks.lock().push(block);
        self.total_allocated.fetch_add(aligned_size, Ordering::SeqCst);

        Ok(())
    }

    /// Align size to pool alignment
    fn align_size(&self, size: usize) -> usize {
        let alignment = self.config.alignment;
        ((size + alignment - 1) / alignment) * alignment
    }

    /// Get total allocated memory
    pub fn total_allocated(&self) -> usize {
        self.total_allocated.load(Ordering::SeqCst)
    }

    /// Get total used memory
    pub fn total_used(&self) -> usize {
        self.total_used.load(Ordering::SeqCst)
    }

    /// Get allocation count
    pub fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::SeqCst)
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let blocks = self.blocks.lock();
        let free_blocks = blocks.iter().filter(|b| !b.in_use).count();
        let used_blocks = blocks.iter().filter(|b| b.in_use).count();

        PoolStats {
            total_allocated: self.total_allocated(),
            total_used: self.total_used(),
            free_blocks,
            used_blocks,
            allocation_count: self.allocation_count(),
            utilization: if self.total_allocated() > 0 {
                self.total_used() as f64 / self.total_allocated() as f64
            } else {
                0.0
            },
        }
    }

    /// Reset pool (deallocate all memory)
    pub fn reset(&self) {
        let blocks = self.blocks.lock();
        for block in blocks.iter() {
            unsafe {
                let layout = alloc::alloc::Layout::from_size_align_unchecked(
                    block.aligned_size,
                    self.config.alignment,
                );
                alloc::alloc::dealloc(block.ptr, layout);
            }
        }
        self.blocks.lock().clear();
        self.total_allocated.store(0, Ordering::SeqCst);
        self.total_used.store(0, Ordering::SeqCst);
        self.allocation_count.store(0, Ordering::SeqCst);
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        self.reset();
    }
}

/// Memory guard that automatically returns memory to pool
pub struct MemoryGuard<'a> {
    pool: &'a MemoryPool,
    ptr: *mut u8,
    size: usize,
    aligned_size: usize,
}

impl<'a> MemoryGuard<'a> {
    /// Get pointer to allocated memory
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Get mutable slice
    pub fn as_slice_mut(&mut self, len: usize) -> &mut [u8] {
        assert!(len <= self.size, "Slice length exceeds allocation size");
        unsafe { core::slice::from_raw_parts_mut(self.ptr, len) }
    }

    /// Get slice
    pub fn as_slice(&self, len: usize) -> &[u8] {
        assert!(len <= self.size, "Slice length exceeds allocation size");
        unsafe { core::slice::from_raw_parts(self.ptr, len) }
    }

    /// Get allocation size
    pub fn size(&self) -> usize {
        self.size
    }
}

impl<'a> Drop for MemoryGuard<'a> {
    fn drop(&mut self) {
        self.pool.deallocate(self.ptr, self.aligned_size);
    }
}

unsafe impl<'a> Send for MemoryGuard<'a> {}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_allocated: usize,
    pub total_used: usize,
    pub free_blocks: usize,
    pub used_blocks: usize,
    pub allocation_count: usize,
    pub utilization: f64,
}

/// Pool errors
#[derive(Debug, Clone)]
pub enum PoolError {
    InvalidSize,
    PoolFull,
    OutOfMemory,
    MaxSizeExceeded,
    AllocationFailed,
}

impl core::fmt::Display for PoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PoolError::InvalidSize => write!(f, "Invalid allocation size"),
            PoolError::PoolFull => write!(f, "Memory pool is full"),
            PoolError::OutOfMemory => write!(f, "Out of memory"),
            PoolError::MaxSizeExceeded => write!(f, "Maximum pool size exceeded"),
            PoolError::AllocationFailed => write!(f, "Memory allocation failed"),
        }
    }
}

/// Shared memory pool reference
pub type SharedMemoryPool = Arc<MemoryPool>;

/// Tensor pool for managing tensor allocations
pub struct TensorPool {
    memory_pool: SharedMemoryPool,
}

impl TensorPool {
    /// Create a new tensor pool
    pub fn new(config: PoolConfig) -> Result<Self, PoolError> {
        let memory_pool = Arc::new(MemoryPool::new(config)?);
        Ok(Self { memory_pool })
    }

    /// Allocate tensor buffer
    pub fn allocate_tensor_buffer(&self, size: usize) -> Result<MemoryGuard<'_>, PoolError> {
        self.memory_pool.allocate(size)
    }

    /// Get memory pool statistics
    pub fn stats(&self) -> PoolStats {
        self.memory_pool.stats()
    }

    /// Get shared memory pool
    pub fn memory_pool(&self) -> &SharedMemoryPool {
        &self.memory_pool
    }

    /// Pre-allocate buffers for common sizes
    pub fn preallocate(&self, sizes: &[usize]) -> Result<(), PoolError> {
        for &size in sizes {
            let _guard = self.memory_pool.allocate(size)?;
            // Guard is dropped immediately, returning buffer to pool
        }
        Ok(())
    }
}

/// Thread-local memory pool for fast allocations
pub struct LocalPool {
    buffers: Vec<Vec<u8>>,
    max_buffer_size: usize,
}

impl LocalPool {
    /// Create a new local pool
    pub fn new(capacity: usize, max_buffer_size: usize) -> Self {
        Self {
            buffers: Vec::with_capacity(capacity),
            max_buffer_size,
        }
    }

    /// Get or allocate a buffer
    pub fn get_buffer(&mut self, min_size: usize) -> Result<&mut Vec<u8>, PoolError> {
        if min_size > self.max_buffer_size {
            return Err(PoolError::InvalidSize);
        }

        // Find a free buffer
        for buffer in &mut self.buffers {
            if buffer.capacity() >= min_size && buffer.is_empty() {
                return Ok(buffer);
            }
        }

        // Allocate new buffer
        let mut buffer = Vec::with_capacity(min_size);
        self.buffers.push(buffer);
        Ok(self.buffers.last_mut().unwrap())
    }

    /// Reset all buffers
    pub fn reset(&mut self) {
        for buffer in &mut self.buffers {
            buffer.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let config = PoolConfig::default();
        let pool = MemoryPool::new(config);
        assert!(pool.is_ok());
        let pool = pool.unwrap();
        assert!(pool.total_allocated() > 0);
    }

    #[test]
    fn test_allocation() {
        let config = PoolConfig::default();
        let pool = MemoryPool::new(config).unwrap();

        let guard = pool.allocate(1024);
        assert!(guard.is_ok());
        let guard = guard.unwrap();
        assert_eq!(guard.size(), 1024);

        let slice = guard.as_slice(1024);
        assert_eq!(slice.len(), 1024);
    }

    #[test]
    fn test_multiple_allocations() {
        let config = PoolConfig {
            initial_size: 4096,
            ..Default::default()
        };
        let pool = MemoryPool::new(config).unwrap();

        let alloc1 = pool.allocate(1024);
        let alloc2 = pool.allocate(2048);
        let alloc3 = pool.allocate(1024);

        assert!(alloc1.is_ok());
        assert!(alloc2.is_ok());
        assert!(alloc3.is_ok());

        let stats = pool.stats();
        assert_eq!(stats.used_blocks, 3);
    }

    #[test]
    fn test_pool_stats() {
        let config = PoolConfig::default();
        let pool = MemoryPool::new(config).unwrap();

        let _guard = pool.allocate(1024).unwrap();
        let stats = pool.stats();

        assert!(stats.total_used > 0);
        assert!(stats.utilization > 0.0);
    }

    #[test]
    fn test_tensor_pool() {
        let config = PoolConfig::default();
        let tensor_pool = TensorPool::new(config).unwrap();

        let buffer = tensor_pool.allocate_tensor_buffer(2048);
        assert!(buffer.is_ok());
    }
}
