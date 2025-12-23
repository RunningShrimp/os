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

/// Memory management statistics
#[derive(Debug, Clone)]
pub struct MemoryManagementStats {
    /// Total physical memory
    pub total_physical_memory: u64,
    /// Available physical memory
    pub available_physical_memory: u64,
    /// Total virtual memory
    pub total_virtual_memory: u64,
    /// Available virtual memory
    pub available_virtual_memory: u64,
    /// Memory usage by type
    pub memory_usage_by_type: alloc::collections::BTreeMap<MemoryType, u64>,
    /// Allocation statistics
    pub allocation_stats: AllocationStats,
    /// NUMA statistics
    pub numa_stats: NumStats,
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryType {
    /// Normal memory
    Normal = 0,
    /// DMA memory
    DMA = 1,
    /// High memory
    HighMem = 2,
    /// Reserved memory
    Reserved = 3,
    /// Kernel memory
    Kernel = 4,
    /// User memory
    User = 5,
}

/// Allocation statistics
#[derive(Debug, Clone)]
pub struct AllocationStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current allocations
    pub current_allocations: u64,
    /// Peak allocations
    pub peak_allocations: u64,
    /// Total allocated bytes
    pub total_allocated_bytes: u64,
    /// Total deallocated bytes
    pub total_deallocated_bytes: u64,
    /// Currently allocated bytes
    pub current_allocated_bytes: u64,
    /// Peak allocated bytes
    pub peak_allocated_bytes: u64,
    /// Allocation failures
    pub allocation_failures: u64,
}

/// NUMA statistics
#[derive(Debug, Clone)]
pub struct NumStats {
    /// Number of NUMA nodes
    pub num_nodes: u32,
    /// Memory per node
    pub memory_per_node: alloc::vec::Vec<u64>,
    /// Allocation statistics per node
    pub allocation_stats_per_node: alloc::vec::Vec<AllocationStats>,
}

impl Default for MemoryManagementStats {
    fn default() -> Self {
        Self {
            total_physical_memory: 0,
            available_physical_memory: 0,
            total_virtual_memory: 0,
            available_virtual_memory: 0,
            memory_usage_by_type: alloc::collections::BTreeMap::new(),
            allocation_stats: AllocationStats::default(),
            numa_stats: NumStats::default(),
        }
    }
}

impl Default for AllocationStats {
    fn default() -> Self {
        Self {
            total_allocations: 0,
            total_deallocations: 0,
            current_allocations: 0,
            peak_allocations: 0,
            total_allocated_bytes: 0,
            total_deallocated_bytes: 0,
            current_allocated_bytes: 0,
            peak_allocated_bytes: 0,
            allocation_failures: 0,
        }
    }
}

impl Default for NumStats {
    fn default() -> Self {
        Self {
            num_nodes: 0,
            memory_per_node: alloc::vec::Vec::new(),
            allocation_stats_per_node: alloc::vec::Vec::new(),
        }
    }
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