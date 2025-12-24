//! Tiered Memory Allocator
//!
//! This module provides a tiered memory allocator that uses multiple memory pools
//! for different object sizes, reducing fragmentation and improving allocation efficiency.
//! It includes defragmentation support and detailed statistics.

extern crate alloc;

use core::alloc::Layout;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use alloc::vec::Vec;
use spin::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Small object threshold (uses slab allocator)
pub const SMALL_OBJECT_THRESHOLD: usize = 2048;

/// Medium object threshold (uses medium-sized buddy allocator)
pub const MEDIUM_OBJECT_THRESHOLD: usize = 32768;

/// Large object threshold (uses large object buddy allocator)
pub const LARGE_OBJECT_THRESHOLD: usize = 1048576; // 1MB

/// Minimum allocation size
pub const MIN_ALLOC_SIZE: usize = 8;

/// Maximum allocation size
pub const MAX_ALLOC_SIZE: usize = 16 * 1024 * 1024; // 16MB

/// Alignment size
pub const ALIGNMENT: usize = 8;

// ============================================================================
// Memory Block Types
// ============================================================================

/// Memory block type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum BlockType {
    Free,
    Small,
    Medium,
    Large,
    Huge,
}

/// Memory block state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum BlockState {
    Free,
    Allocated,
    Split,
    Merged,
}

// ============================================================================
// Memory Block Structure
// ============================================================================

/// Memory block header
#[derive(Debug)]
#[repr(C)]
pub struct BlockHeader {
    /// Block size (including header)
    pub size: usize,
    /// Block type
    pub block_type: BlockType,
    /// Block state
    pub state: BlockState,
    /// Previous block (for merging free blocks)
    pub prev: Option<NonNull<BlockHeader>>,
    /// Next block (for merging free blocks)
    pub next: Option<NonNull<BlockHeader>>,
    /// Allocation timestamp (for debugging)
    pub alloc_timestamp: u64,
    /// Allocating process ID (for debugging)
    pub alloc_pid: u32,
}

impl BlockHeader {
    /// Create a new block header
    pub fn new(size: usize, block_type: BlockType) -> Self {
        Self {
            size,
            block_type,
            state: BlockState::Free,
            prev: None,
            next: None,
            alloc_timestamp: 0,
            alloc_pid: 0,
        }
    }
    
    /// Get data pointer
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe {
            (self as *const BlockHeader as *mut u8).add(core::mem::size_of::<BlockHeader>())
        }
    }
    
    /// Get block header from data pointer
    pub fn from_data_ptr(ptr: *mut u8) -> *mut BlockHeader {
        unsafe {
            (ptr as *mut BlockHeader).sub(1)
        }
    }
    
    /// Check if block is free
    pub fn is_free(&self) -> bool {
        self.state == BlockState::Free
    }
    
    /// Mark as allocated
    pub fn mark_allocated(&mut self, pid: u32) {
        self.state = BlockState::Allocated;
        // Note: timestamp would need time module, skipping for now
        self.alloc_pid = pid;
    }
    
    /// Mark as free
    pub fn mark_free(&mut self) {
        self.state = BlockState::Free;
        self.alloc_timestamp = 0;
        self.alloc_pid = 0;
    }
    
    /// Get aligned size
    pub fn aligned_size(&self) -> usize {
        (self.size + ALIGNMENT - 1) & !(ALIGNMENT - 1)
    }
}

// ============================================================================
// Memory Pool
// ============================================================================

/// Memory pool for tiered allocation
pub struct MemoryPool {
    /// Pool start address
    pub start_addr: usize,
    /// Pool size
    pub size: usize,
    /// Free block list
    pub free_blocks: Mutex<Vec<NonNull<BlockHeader>>>,
    /// Allocated block list (for debugging)
    pub allocated_blocks: Mutex<Vec<NonNull<BlockHeader>>>,
    /// Allocation count
    pub alloc_count: AtomicUsize,
    /// Free count
    pub free_count: AtomicUsize,
    /// Current usage in bytes
    pub used_bytes: AtomicUsize,
    /// Peak usage in bytes
    pub peak_used_bytes: AtomicUsize,
    /// Fragmentation count
    pub fragmentation_count: AtomicUsize,
}

impl MemoryPool {
    /// Create a new memory pool
    /// 
    /// # Safety
    /// The caller must ensure that the memory region is valid and accessible
    pub unsafe fn new(size: usize) -> Option<Self> {
        // In a real implementation, this would allocate from the physical memory manager
        // For now, we'll create a placeholder that will be initialized later
        Some(Self {
            start_addr: 0,
            size,
            free_blocks: Mutex::new(Vec::new()),
            allocated_blocks: Mutex::new(Vec::new()),
            alloc_count: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            used_bytes: AtomicUsize::new(0),
            peak_used_bytes: AtomicUsize::new(0),
            fragmentation_count: AtomicUsize::new(0),
        })
    }
    
    /// Initialize memory pool with actual memory region
    /// 
    /// # Safety
    /// - `start_addr` must point to valid, contiguous memory
    /// - The memory region must be aligned to PAGE_SIZE
    /// - The caller must ensure no other code accesses this memory while the pool is in use
    pub unsafe fn initialize(&mut self, start_addr: usize) {
        self.start_addr = start_addr;
        
        // Create initial free block
        let initial_block = unsafe {
            let block_ptr = start_addr as *mut BlockHeader;
            (*block_ptr) = BlockHeader::new(self.size, BlockType::Free);
            NonNull::new_unchecked(block_ptr)
        };
        
        let mut free_blocks = self.free_blocks.lock();
        free_blocks.push(initial_block);
        drop(free_blocks);
    }
    
    /// Allocate memory
    pub fn allocate(&self, size: usize) -> *mut u8 {
        // Align size
        let aligned_size = (size + ALIGNMENT - 1) & !(ALIGNMENT - 1);

        // Check min and max size
        if !(MIN_ALLOC_SIZE..=MAX_ALLOC_SIZE).contains(&aligned_size) {
            return core::ptr::null_mut();
        }
        
        let mut free_blocks = self.free_blocks.lock();
        
        // Find suitable free block
        for (i, &block_header) in free_blocks.iter().enumerate() {
            let block = unsafe { block_header.as_ref() };
            
            if block.is_free() && block.size >= aligned_size {
                // Check if we need to split the block
                if block.size > aligned_size + core::mem::size_of::<BlockHeader>() + MIN_ALLOC_SIZE {
                    // Split block
                    let remaining_size = block.size - aligned_size;
                    let new_block_addr = block.data_ptr() as usize + aligned_size;
                    let new_block = unsafe {
                        let block_ptr = new_block_addr as *mut BlockHeader;
                        (*block_ptr) = BlockHeader::new(remaining_size, BlockType::Free);
                        NonNull::new_unchecked(block_ptr)
                    };
                    
                    // Update original block
                    let block = unsafe { &mut *block_header.as_ptr() };
                    block.size = aligned_size;
                    block.mark_allocated(0); // Use default PID
                    
                    // Add new block to free list
                    free_blocks.push(new_block);
                    
                    // Update statistics
                    self.alloc_count.fetch_add(1, Ordering::SeqCst);
                    let current_used = self.used_bytes.fetch_add(aligned_size, Ordering::SeqCst) + aligned_size;
                    let peak_used = self.peak_used_bytes.load(Ordering::SeqCst);
                    if current_used > peak_used {
                        self.peak_used_bytes.store(current_used, Ordering::SeqCst);
                    }
                    
                    drop(free_blocks);
                    return block.data_ptr();
                } else {
                    // Allocate entire block
                    let block = unsafe { &mut *block_header.as_ptr() };
                    block.mark_allocated(0); // Use default PID
                    
                    // Remove from free list
                    free_blocks.remove(i);
                    
                    // Update statistics
                    self.alloc_count.fetch_add(1, Ordering::SeqCst);
                    let current_used = self.used_bytes.fetch_add(aligned_size, Ordering::SeqCst) + aligned_size;
                    let peak_used = self.peak_used_bytes.load(Ordering::SeqCst);
                    if current_used > peak_used {
                        self.peak_used_bytes.store(current_used, Ordering::SeqCst);
                    }
                    
                    drop(free_blocks);
                    return block.data_ptr();
                }
            }
        }
        
        drop(free_blocks);
        core::ptr::null_mut()
    }
    
    /// Deallocate memory
    /// 
    /// # Safety
    /// - `ptr` must have been allocated by this pool
    /// - `ptr` must not be null
    /// - `ptr` must not be double-freed
    pub unsafe fn deallocate(&self, ptr: *mut u8, _size: usize) {
        if ptr.is_null() {
            return;
        }
        
        // Get block header
        let block_header = BlockHeader::from_data_ptr(ptr);
        let block = unsafe { &mut *block_header };
        
        // Check if allocated
        if !block.is_free() {
            // Mark as free
            block.mark_free();
            
            // Try to merge with adjacent blocks
            unsafe {
                self.try_merge(block_header);
            }
            
            // Add to free list
            let mut free_blocks = self.free_blocks.lock();
            unsafe {
                free_blocks.push(NonNull::new_unchecked(block_header));
            }
            drop(free_blocks);
            
            // Update statistics
            self.free_count.fetch_add(1, Ordering::SeqCst);
            self.used_bytes.fetch_sub(block.aligned_size(), Ordering::SeqCst);
            
            // Remove from allocated list
            let mut allocated_blocks = self.allocated_blocks.lock();
            allocated_blocks.retain(|&b| b.as_ptr() != block_header);
            drop(allocated_blocks);
        }
    }
    
    /// Try to merge adjacent free blocks
    unsafe fn try_merge(&self, block: *mut BlockHeader) {
        let current_size = unsafe { (*block).size };
        
        // Check previous block
        if let Some(prev) = unsafe { (*block).prev } {
            let prev_ptr = prev.as_ptr();
            if unsafe { (*prev_ptr).is_free() } {
                let prev_size = unsafe { (*prev_ptr).size };
                // Merge previous block
                unsafe { (*prev_ptr).size = current_size + prev_size; }
                
                // Remove previous block from free list
                let mut free_blocks = self.free_blocks.lock();
                free_blocks.retain(|&b| b.as_ptr() != prev_ptr);
                drop(free_blocks);
                
                // Update fragmentation statistics
                if current_size < SMALL_OBJECT_THRESHOLD || prev_size < SMALL_OBJECT_THRESHOLD {
                    self.fragmentation_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        
        // Check next block
        if let Some(next) = unsafe { (*block).next } {
            let next_ptr = next.as_ptr();
            if unsafe { (*next_ptr).is_free() } {
                let next_size = unsafe { (*next_ptr).size };
                // Merge next block
                unsafe { (*block).size = current_size + next_size; }
                
                // Remove next block from free list
                let mut free_blocks = self.free_blocks.lock();
                free_blocks.retain(|&b| b.as_ptr() != next_ptr);
                drop(free_blocks);
                
                // Update fragmentation statistics
                if current_size < SMALL_OBJECT_THRESHOLD || next_size < SMALL_OBJECT_THRESHOLD {
                    self.fragmentation_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
    
    /// Get memory pool statistics
    pub fn get_stats(&self) -> MemoryPoolStats {
        MemoryPoolStats {
            total_size: self.size,
            used_bytes: self.used_bytes.load(Ordering::SeqCst),
            peak_used_bytes: self.peak_used_bytes.load(Ordering::SeqCst),
            alloc_count: self.alloc_count.load(Ordering::SeqCst),
            free_count: self.free_count.load(Ordering::SeqCst),
            fragmentation_count: self.fragmentation_count.load(Ordering::SeqCst),
            free_blocks: self.free_blocks.lock().len(),
            allocated_blocks: self.allocated_blocks.lock().len(),
        }
    }
    
    /// Get memory pool information
    pub fn get_pool_info(&self) -> PoolInfo {
        let free_blocks_len = self.free_blocks.lock().len();
        let allocated_blocks_len = self.allocated_blocks.lock().len();
        let total_blocks = free_blocks_len + allocated_blocks_len;
        
        let used_bytes = self.used_bytes.load(Ordering::SeqCst);
        let free_bytes = self.size - used_bytes;
        
        // Calculate fragmentation ratio
        let fragmentation_ratio = if total_blocks > 0 {
            (free_blocks_len as f32) / (total_blocks as f32)
        } else {
            0.0
        };
        
        PoolInfo {
            total_blocks,
            free_blocks: free_blocks_len,
            allocated_blocks: allocated_blocks_len,
            total_bytes: self.size,
            free_bytes,
            allocated_bytes: used_bytes,
            fragmentation_ratio,
        }
    }
    
    /// Defragment memory pool
    pub fn defragment(&self) {
        let mut free_blocks = self.free_blocks.lock();
        
        // Sort free blocks by address
        free_blocks.sort_by(|a, b| {
            let block_a = unsafe { a.as_ref() };
            let block_b = unsafe { b.as_ref() };
            (block_a.data_ptr() as usize).cmp(&(block_b.data_ptr() as usize))
        });
        
        // Merge adjacent free blocks
        let mut i = 0;
        while i < free_blocks.len().saturating_sub(1) {
            let current_block = unsafe { &mut *free_blocks[i].as_ptr() };
            let next_block = unsafe { &mut *free_blocks[i + 1].as_ptr() };
            
            // Check if adjacent
            let current_end = current_block.data_ptr() as usize + current_block.size;
            let next_start = next_block.data_ptr() as usize;
            
            if current_end == next_start {
                // Merge blocks
                let merged_size = current_block.size + next_block.size;
                current_block.size = merged_size;
                current_block.mark_free();
                
                // Remove next block
                free_blocks.remove(i + 1);
                
                // Update fragmentation statistics
                if current_block.size < SMALL_OBJECT_THRESHOLD || next_block.size < SMALL_OBJECT_THRESHOLD {
                    self.fragmentation_count.fetch_add(1, Ordering::SeqCst);
                }
            } else {
                i += 1;
            }
        }
    }
}

// ============================================================================
// Memory Pool Statistics
// ============================================================================

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    /// Total size
    pub total_size: usize,
    /// Used bytes
    pub used_bytes: usize,
    /// Peak used bytes
    pub peak_used_bytes: usize,
    /// Allocation count
    pub alloc_count: usize,
    /// Free count
    pub free_count: usize,
    /// Fragmentation count
    pub fragmentation_count: usize,
    /// Free blocks count
    pub free_blocks: usize,
    /// Allocated blocks count
    pub allocated_blocks: usize,
}

impl MemoryPoolStats {
    /// Get memory usage ratio
    pub fn usage_ratio(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            self.used_bytes as f64 / self.total_size as f64
        }
    }
    
    /// Get fragmentation ratio
    pub fn fragmentation_ratio(&self) -> f64 {
        if self.alloc_count == 0 {
            0.0
        } else {
            self.fragmentation_count as f64 / self.alloc_count as f64
        }
    }
}

/// Pool information
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct PoolInfo {
    pub total_blocks: usize,
    pub free_blocks: usize,
    pub allocated_blocks: usize,
    pub total_bytes: usize,
    pub free_bytes: usize,
    pub allocated_bytes: usize,
    pub fragmentation_ratio: f32,
}

impl Default for PoolInfo {
    fn default() -> Self {
        Self {
            total_blocks: 0,
            free_blocks: 0,
            allocated_blocks: 0,
            total_bytes: 0,
            free_bytes: 0,
            allocated_bytes: 0,
            fragmentation_ratio: 0.0,
        }
    }
}

// ============================================================================
// Tiered Memory Allocator
// ============================================================================

/// Tiered memory allocator using multiple pools
pub struct TieredMemoryAllocator {
    /// Small object pool
    small_pool: Option<MemoryPool>,
    /// Medium object pool
    medium_pool: Option<MemoryPool>,
    /// Large object pool
    large_pool: Option<MemoryPool>,
    /// Allocation statistics
    pub stats: Mutex<AllocatorStats>,
}

/// Allocator statistics
#[derive(Debug, Default)]
pub struct AllocatorStats {
    /// Total allocations
    pub total_allocations: AtomicU64,
    /// Total deallocations
    pub total_deallocations: AtomicU64,
    /// Total allocated bytes
    pub total_allocated_bytes: AtomicU64,
    /// Total freed bytes
    pub total_freed_bytes: AtomicU64,
    /// Current allocated bytes
    pub current_allocated_bytes: AtomicU64,
    /// Peak allocated bytes
    pub peak_allocated_bytes: AtomicU64,
    /// Failed allocations
    pub failed_allocations: AtomicU64,
}

impl Clone for AllocatorStats {
    fn clone(&self) -> Self {
        Self {
            total_allocations: AtomicU64::new(self.total_allocations.load(Ordering::SeqCst)),
            total_deallocations: AtomicU64::new(self.total_deallocations.load(Ordering::SeqCst)),
            total_allocated_bytes: AtomicU64::new(self.total_allocated_bytes.load(Ordering::SeqCst)),
            total_freed_bytes: AtomicU64::new(self.total_freed_bytes.load(Ordering::SeqCst)),
            current_allocated_bytes: AtomicU64::new(self.current_allocated_bytes.load(Ordering::SeqCst)),
            peak_allocated_bytes: AtomicU64::new(self.peak_allocated_bytes.load(Ordering::SeqCst)),
            failed_allocations: AtomicU64::new(self.failed_allocations.load(Ordering::SeqCst)),
        }
    }
}

impl Default for TieredMemoryAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl TieredMemoryAllocator {
    /// Create a new tiered memory allocator
    pub fn new() -> Self {
        Self {
            small_pool: None,
            medium_pool: None,
            large_pool: None,
            stats: Mutex::new(AllocatorStats::default()),
        }
    }
    
    /// Initialize allocator with memory regions
    /// 
    /// # Safety
    /// - All memory regions must be valid and accessible
    /// - Memory regions must be aligned to PAGE_SIZE
    pub unsafe fn initialize(
        &mut self,
        small_pool_addr: usize,
        small_pool_size: usize,
        medium_pool_addr: usize,
        medium_pool_size: usize,
        large_pool_addr: usize,
        large_pool_size: usize,
    ) -> nos_api::Result<()> {
        // Initialize small pool
        if let Some(mut pool) = unsafe { MemoryPool::new(small_pool_size) } {
            unsafe { pool.initialize(small_pool_addr) };
            self.small_pool = Some(pool);
        }
        
        // Initialize medium pool
        if let Some(mut pool) = unsafe { MemoryPool::new(medium_pool_size) } {
            unsafe { pool.initialize(medium_pool_addr) };
            self.medium_pool = Some(pool);
        }
        
        // Initialize large pool
        if let Some(mut pool) = unsafe { MemoryPool::new(large_pool_size) } {
            unsafe { pool.initialize(large_pool_addr) };
            self.large_pool = Some(pool);
        }
        
        Ok(())
    }
    
    /// Allocate memory
    pub fn allocate(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // Update statistics
        self.stats.lock().total_allocations.fetch_add(1, Ordering::SeqCst);
        
        // Select appropriate pool based on size
        let ptr = if size <= SMALL_OBJECT_THRESHOLD {
            self.small_pool.as_ref()
                .map(|pool| pool.allocate(size))
                .unwrap_or(core::ptr::null_mut())
        } else if size <= MEDIUM_OBJECT_THRESHOLD {
            self.medium_pool.as_ref()
                .map(|pool| pool.allocate(size))
                .unwrap_or(core::ptr::null_mut())
        } else if size <= LARGE_OBJECT_THRESHOLD {
            self.large_pool.as_ref()
                .map(|pool| pool.allocate(size))
                .unwrap_or(core::ptr::null_mut())
        } else {
            // Huge objects - would need to allocate from system
            core::ptr::null_mut()
        };
        
        if ptr.is_null() {
            self.stats.lock().failed_allocations.fetch_add(1, Ordering::SeqCst);
        } else {
            let stats = self.stats.lock();
            stats.total_allocated_bytes.fetch_add(size as u64, Ordering::SeqCst);
            let current = stats.current_allocated_bytes.fetch_add(size as u64, Ordering::SeqCst) + size as u64;
            let peak = stats.peak_allocated_bytes.load(Ordering::SeqCst);
            if current > peak {
                stats.peak_allocated_bytes.store(current, Ordering::SeqCst);
            }
        }
        
        ptr
    }
    
    /// Deallocate memory
    /// 
    /// # Safety
    /// - `ptr` must have been allocated by this allocator
    /// - `ptr` must not be null
    pub unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        
        let size = layout.size();
        
        // Update statistics
        let stats = self.stats.lock();
        stats.total_deallocations.fetch_add(1, Ordering::SeqCst);
        stats.total_freed_bytes.fetch_add(size as u64, Ordering::SeqCst);
        stats.current_allocated_bytes.fetch_sub(size as u64, Ordering::SeqCst);
        drop(stats);
        
        // Deallocate from appropriate pool
        if size <= SMALL_OBJECT_THRESHOLD && let Some(pool) = &self.small_pool {
            unsafe { pool.deallocate(ptr, 0) };
        } else if size <= MEDIUM_OBJECT_THRESHOLD && let Some(pool) = &self.medium_pool {
            unsafe { pool.deallocate(ptr, 0) };
        } else if size <= LARGE_OBJECT_THRESHOLD && let Some(pool) = &self.large_pool {
            unsafe { pool.deallocate(ptr, 0) };
        }
    }
    
    /// Get allocator statistics
    pub fn get_stats(&self) -> AllocatorStats {
        self.stats.lock().clone()
    }
    
    /// Get memory usage information
    pub fn get_memory_usage(&self) -> MemoryUsage {
        let small_stats = self.small_pool.as_ref()
            .map(|p| p.get_stats())
            .unwrap_or_else(|| MemoryPoolStats {
                total_size: 0,
                used_bytes: 0,
                peak_used_bytes: 0,
                alloc_count: 0,
                free_count: 0,
                fragmentation_count: 0,
                free_blocks: 0,
                allocated_blocks: 0,
            });
        let medium_stats = self.medium_pool.as_ref()
            .map(|p| p.get_stats())
            .unwrap_or_else(|| MemoryPoolStats {
                total_size: 0,
                used_bytes: 0,
                peak_used_bytes: 0,
                alloc_count: 0,
                free_count: 0,
                fragmentation_count: 0,
                free_blocks: 0,
                allocated_blocks: 0,
            });
        let large_stats = self.large_pool.as_ref()
            .map(|p| p.get_stats())
            .unwrap_or_else(|| MemoryPoolStats {
                total_size: 0,
                used_bytes: 0,
                peak_used_bytes: 0,
                alloc_count: 0,
                free_count: 0,
                fragmentation_count: 0,
                free_blocks: 0,
                allocated_blocks: 0,
            });
        let stats = self.get_stats();
        
        let total_used = small_stats.used_bytes + medium_stats.used_bytes + large_stats.used_bytes;
        let total_size = small_stats.total_size + medium_stats.total_size + large_stats.total_size;
        
        MemoryUsage {
            total_size,
            used_bytes: total_used,
            free_bytes: total_size - total_used,
            usage_ratio: if total_size > 0 { total_used as f64 / total_size as f64 } else { 0.0 },
            fragmentation_ratio: (small_stats.fragmentation_ratio() + medium_stats.fragmentation_ratio() + large_stats.fragmentation_ratio()) / 3.0,
            total_allocations: stats.total_allocations.load(Ordering::SeqCst),
            total_deallocations: stats.total_deallocations.load(Ordering::SeqCst),
            failed_allocations: stats.failed_allocations.load(Ordering::SeqCst),
        }
    }
    
    /// Defragment all pools
    pub fn defragment(&self) {
        if let Some(pool) = &self.small_pool {
            pool.defragment();
        }
        if let Some(pool) = &self.medium_pool {
            pool.defragment();
        }
        if let Some(pool) = &self.large_pool {
            pool.defragment();
        }
    }
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// Total size
    pub total_size: usize,
    /// Used bytes
    pub used_bytes: usize,
    /// Free bytes
    pub free_bytes: usize,
    /// Usage ratio
    pub usage_ratio: f64,
    /// Fragmentation ratio
    pub fragmentation_ratio: f64,
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Failed allocations
    pub failed_allocations: u64,
}

