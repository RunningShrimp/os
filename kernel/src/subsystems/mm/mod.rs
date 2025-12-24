// Re-export nos-mm as base memory management
pub use nos_mm;

// Advanced memory management extensions
pub mod api;
pub mod traits;
pub mod compress;
pub mod hugepage;
pub mod percpu_allocator;
pub mod prefetch;
pub mod numa;
pub mod stats;
pub mod memory_isolation;
pub mod optimized_page_allocator;
pub mod types;
pub mod unified_stats;

// Re-export unified stats to avoid duplication
pub use unified_stats::{
    MemoryManagementStats,
    AllocationStats,
    AtomicAllocationStats,
    NumStats,
    LightweightAllocationStats,
    ExtendedAllocationStats,
};

#[cfg(feature = "kernel_tests")]
pub mod tests;

// pub use optimized_allocator::OptimizedHybridAllocator;

/// Align up to the given alignment
pub const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Align down to the given alignment
pub const fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

/// Check if an address is aligned to the given alignment
pub const fn is_aligned(addr: usize, align: usize) -> bool {
    (addr & (align - 1)) == 0
}

/// Round up to the next power of 2
pub const fn round_up_power_of_2(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        let mut v = n - 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v |= v >> 32;
        v + 1
    }
}

/// Get the log2 of a power-of-2 number
pub const fn log2_pow2(n: usize) -> u32 {
    if n == 0 {
        panic!("log2_pow2(0) is undefined");
    }
    (usize::BITS - 1) - n.leading_zeros()
}

/// Get the order (log2) of a size, rounded up to the nearest power of 2
pub const fn get_order(size: usize, min_order: usize) -> usize {
    let mut order = min_order;
    let mut current_size = 1 << min_order;
    
    while current_size < size {
        current_size *= 2;
        order += 1;
    }
    
    order
}

/// Initialize advanced memory management
///
/// This function initializes advanced memory management features
/// that build on top of the basic nos-mm functionality.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn init_advanced_memory_management() -> nos_api::Result<()> {
    // Initialize NUMA support
    numa::init_numa()?;
    
    // Initialize per-CPU allocators
    percpu_allocator::init_percpu_allocators()?;
    
    // Initialize optimized memory manager
    // optimized_memory_manager::init_optimized_memory_manager()?;
    
    // Initialize memory statistics
    stats::init_memory_stats()?;
    
    Ok(())
}

/// Shutdown advanced memory management
///
/// This function shuts down advanced memory management features.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_advanced_memory_management() -> nos_api::Result<()> {
    // Shutdown memory statistics
    stats::shutdown_memory_stats()?;
    
    // Shutdown optimized memory manager
    // optimized_memory_manager::shutdown_optimized_memory_manager()?;
    
    // Shutdown per-CPU allocators
    percpu_allocator::shutdown_percpu_allocators()?;
    
    // Shutdown NUMA support
    numa::shutdown_numa()?;
    
    Ok(())
}

/// Get memory management statistics
///
/// # Returns
///
/// * `MemoryManagementStats` - Memory management statistics
pub fn get_memory_stats() -> MemoryManagementStats {
    stats::get_memory_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_functions() {
        assert_eq!(align_up(1000, 4096), 4096);
        assert_eq!(align_down(4096, 4096), 4096);
        assert_eq!(is_aligned(4096, 4096), true);
        assert_eq!(is_aligned(1000, 4096), false);
    }

    #[test]
    fn test_power_of_2_functions() {
        assert_eq!(round_up_power_of_2(1000), 1024);
        assert_eq!(round_up_power_of_2(1024), 1024);
        assert_eq!(log2_pow2(1024), 10);
        assert_eq!(get_order(1000, 0), 10);
    }

    #[test]
    fn test_memory_stats() {
        let stats = get_memory_stats();
        assert_eq!(stats.total_physical_memory, 0);
        assert_eq!(stats.available_physical_memory, 0);
        assert_eq!(stats.total_virtual_memory, 0);
        assert_eq!(stats.available_virtual_memory, 0);
        assert!(stats.memory_usage_by_type.is_empty());
    }
}