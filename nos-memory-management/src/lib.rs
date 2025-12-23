//! NOS Memory Management
//!
//! This crate provides physical and virtual memory management.
//! It includes memory allocators, page tables, and memory mapping.

#![no_std]
#![warn(missing_docs)]
#![warn(clippy::all)]

// Re-export API types
pub use nos_api::*;

// Memory management modules
pub mod physical;
pub mod virtual_mem;
pub mod page_table;
pub mod allocator;
pub mod layout;

// Re-export commonly used types and functions
// Note: We don't re-export initialize()/shutdown() functions directly to avoid name conflicts
pub use physical::{PhysAddr, PAGE_SIZE, PAGE_SHIFT, page_round_up, page_round_down, addr_to_pfn, pfn_to_addr};
pub use virtual_mem::{VirtAddr};
pub use page_table::*;
pub use allocator::{buddy, slab, mempool};
pub use layout::{AddressSpaceLayout, AslrRegionType, kernel_base, user_base, user_stack_top, user_max, page_size, is_kernel_address, is_user_address, phys_to_virt, virt_to_phys, apply_aslr_offset_global as apply_aslr_offset, apply_aslr_offset_enhanced};

// Re-export allocator types for convenience
pub use allocator::buddy::OptimizedBuddyAllocator;
pub use allocator::slab::OptimizedSlabAllocator;
pub use allocator::mempool::MemoryPool;
pub use allocator::tiered::{TieredMemoryAllocator, MemoryUsage, AllocatorStats as TieredAllocatorStats};

/// Memory management initialization
///
/// This function initializes the memory management subsystem.
/// It should be called after kernel core initialization.
///
/// # Returns
/// * `nos_api::Result<()>` - Success or error
pub fn initialize_memory_management() -> nos_api::Result<()> {
    // Initialize physical memory manager
    physical::initialize()?;
    
    // Initialize virtual memory manager
    virtual_mem::initialize()?;
    
    // Initialize page table manager
    page_table::initialize()?;
    
    // Initialize memory allocator
    allocator::initialize()?;
    
    Ok(())
}

/// Memory management shutdown
///
/// This function shuts down the memory management subsystem.
/// It should be called during system shutdown.
///
/// # Returns
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_memory_management() -> nos_api::Result<()> {
    // Shutdown memory allocator
    allocator::shutdown()?;
    
    // Shutdown page table manager
    page_table::shutdown()?;
    
    // Shutdown virtual memory manager
    virtual_mem::shutdown()?;
    
    // Shutdown physical memory manager
    physical::shutdown()?;
    
    Ok(())
}

/// Get memory management information
///
/// # Returns
/// * `MemoryInfo` - Memory management information
pub fn get_memory_info() -> MemoryInfo {
    MemoryInfo {
        total_physical: physical::get_total_memory(),
        available_physical: physical::get_available_memory(),
        total_virtual: virtual_mem::get_total_memory(),
        available_virtual: virtual_mem::get_available_memory(),
        page_size: page_table::get_page_size(),
        total_pages: page_table::get_total_pages(),
        free_pages: page_table::get_free_pages(),
        allocated_pages: page_table::get_allocated_pages(),
    }
}

/// Memory management information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Total physical memory
    pub total_physical: usize,
    /// Available physical memory
    pub available_physical: usize,
    /// Total virtual memory
    pub total_virtual: usize,
    /// Available virtual memory
    pub available_virtual: usize,
    /// Page size
    pub page_size: usize,
    /// Total pages
    pub total_pages: usize,
    /// Free pages
    pub free_pages: usize,
    /// Allocated pages
    pub allocated_pages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_info() {
        let info = get_memory_info();
        
        // Basic sanity checks
        assert!(info.total_physical > 0);
        assert!(info.available_physical <= info.total_physical);
        assert!(info.total_virtual > 0);
        assert!(info.available_virtual <= info.total_virtual);
        assert!(info.page_size > 0);
        assert!(info.total_pages > 0);
        assert!(info.free_pages <= info.total_pages);
        assert!(info.allocated_pages <= info.total_pages);
    }
}