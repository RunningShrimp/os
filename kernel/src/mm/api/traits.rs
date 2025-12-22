// Memory module interface contracts
// These traits follow the MM_MODULE_API_BOUNDARIES_DESIGN.md

use super::{AllocError, VmError, PhysicalError};
use super::{MemoryProtection, MapFlags, MemoryMapping, AllocatorStats, PhysicalMemoryStats, PhysicalPage, MemoryPoolStats};


/// Memory allocator trait
/// 
/// All memory allocators must implement this trait
pub trait Allocator {
    /// Allocate memory block
    /// 
    /// # Contract
    /// * Returned memory must be aligned as specified
    /// * Return clear error when allocation fails
    /// * Allocated memory must be initialized to zero
    /// * Must support zero size allocation
    fn allocate(&mut self, size: usize, align: usize) -> Result<*mut u8, AllocError>;
    
    /// Deallocate memory block
    /// 
    /// # Contract
    /// * Can only deallocate memory previously allocated
    /// * Memory cannot be accessed after deallocation
    /// * Must handle double free
    /// * Must support zero size deallocation
    fn deallocate(&mut self, ptr: *mut u8, size: usize) -> Result<(), AllocError>;
    
    /// Reallocate memory block
    /// 
    /// # Contract
    /// * Try to expand at original address
    /// * Allocate new memory and copy data when expansion not possible
    /// * Original memory is automatically freed
    /// * Must handle zero size reallocation
    fn reallocate(&mut self, ptr: *mut u8, old_size: usize, new_size: usize, align: usize) -> Result<*mut u8, AllocError>;
    
    /// Get allocator statistics
    fn get_stats(&self) -> AllocatorStats;
    
    /// Reset allocator statistics
    fn reset_stats(&mut self);
}

/// Virtual memory manager trait
/// 
/// Define virtual memory management standard interface
pub trait VirtualMemoryManager {
    /// Map memory region
    /// 
    /// # Contract
    /// * Address must be page aligned
    /// * Size must be page aligned
    /// * Validate memory permissions
    /// * Update process address space
    /// * Handle address conflict
    fn map_memory(
        &mut self,
        addr: usize,
        size: usize,
        prot: MemoryProtection,
        flags: MapFlags,
    ) -> Result<MemoryMapping, VmError>;
    
    /// Unmap memory region
    /// 
    /// # Contract
    /// * Address and size must match previously mapped region
    /// * Cleanup page table entries
    /// * Release related resources
    /// * Handle partial unmap
    fn unmap_memory(&mut self, addr: usize, size: usize) -> Result<(), VmError>;
    
    /// Change memory protection
    /// 
    /// # Contract
    /// * Validate address range validity
    /// * Update page table entries
    /// * Handle permission conflict
    /// * Support partial region protection change
    fn change_protection(
        &mut self,
        addr: usize,
        size: usize,
        prot: MemoryProtection,
    ) -> Result<(), VmError>;
    
    /// Flush memory mapping
    /// 
    /// # Contract
    /// * Write dirty pages to backing store
    /// * Wait for I/O to complete
    /// * Handle I/O errors
    fn flush_memory(&mut self, addr: usize, size: usize) -> Result<(), VmError>;
    
    /// Get memory mapping information
    fn get_mapping(&self, addr: usize) -> Option<MemoryMapping>;
}

/// Physical memory manager trait
/// 
/// Manage physical memory allocation and tracking
pub trait PhysicalMemoryManager {
    /// Allocate physical pages
    /// 
    /// # Contract
    /// * Allocated pages must be continuous
    /// * Must track allocated pages
    /// * Handle out of memory situation
    /// * Support page reclaim mechanism
    fn allocate_pages(&mut self, count: usize) -> Result<PhysicalPage, PhysicalError>;
    
    /// Free physical pages
    /// 
    /// # Contract
    /// * Can only free previously allocated pages
    /// * Must update allocation status
    /// * Support batch free
    /// * Handle double free
    fn free_pages(&mut self, page: PhysicalPage) -> Result<(), PhysicalError>;
    
    /// Get physical memory statistics
    fn get_stats(&self) -> PhysicalMemoryStats;
    
    /// Get available physical memory
    fn get_available_memory(&self) -> usize;
    
    /// Get total physical memory
    fn get_total_memory(&self) -> usize;
}

/// Memory pool trait
/// 
/// Provide efficient memory pool management
pub trait MemoryPool {
    /// Allocate object from memory pool
    /// 
    /// # Contract
    /// * Allocation time complexity O(1)
    /// * Support fixed size objects
    /// * Automatic handle pool expansion
    /// * Must be thread safe
    fn allocate_object(&mut self, size: usize) -> Result<*mut u8, AllocError>;
    
    /// Free object to memory pool
    /// 
    /// # Contract
    /// * Free time complexity O(1)
    /// * Must validate object validity
    /// * Automatic handle pool contraction
    /// * Must be thread safe
    fn free_object(&mut self, ptr: *mut u8, size: usize) -> Result<(), AllocError>;
    
    /// Get memory pool statistics
    fn get_pool_stats(&self) -> MemoryPoolStats;
}

/// Zero copy memory operations trait
/// 
/// Provide efficient zero copy memory operations
pub trait ZeroCopyOperations {
    /// Copy data between memory regions
    /// 
    /// # Contract
    /// * Must handle address overlap
    /// * Must validate memory permissions
    /// * Optimize for large copy operations
    /// * Must ensure atomicity
    fn copy_memory(&mut self, src: usize, dst: usize, size: usize) -> Result<(), VmError>;
    
    /// Move data between memory regions
    /// 
    /// # Contract
    /// * Must handle address overlap
    /// * Must validate memory permissions
    /// * Source region is invalid after move
    /// * Must ensure atomicity
    fn move_memory(&mut self, src: usize, dst: usize, size: usize) -> Result<(), VmError>;
    
    /// Set memory region to a value
    /// 
    /// # Contract
    /// * Must be word/double-word aligned
    /// * Optimize for large set operations
    /// * Must ensure atomicity
    fn set_memory(&mut self, dst: usize, value: u64, size: usize) -> Result<(), VmError>;
}

