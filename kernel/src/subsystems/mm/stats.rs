//! Memory statistics
//!
//! This module provides memory statistics collection and reporting functionality.

use nos_api::Result;
use spin::Mutex;

/// Memory statistics collector
pub struct MemoryStatsCollector {
    /// Memory statistics
    stats: Mutex<MemoryManagementStats>,
}

impl MemoryStatsCollector {
    /// Create a new memory statistics collector
    pub fn new() -> Self {
        Self {
            stats: Mutex::new(MemoryManagementStats::default()),
        }
    }

    /// Update total physical memory
    pub fn update_total_physical_memory(&self, size: u64) {
        let mut stats = self.stats.lock();
        stats.total_physical_memory = size;
    }

    /// Update available physical memory
    pub fn update_available_physical_memory(&self, size: u64) {
        let mut stats = self.stats.lock();
        stats.available_physical_memory = size;
    }

    /// Update total virtual memory
    pub fn update_total_virtual_memory(&self, size: u64) {
        let mut stats = self.stats.lock();
        stats.total_virtual_memory = size;
    }

    /// Update available virtual memory
    pub fn update_available_virtual_memory(&self, size: u64) {
        let mut stats = self.stats.lock();
        stats.available_virtual_memory = size;
    }

    /// Record allocation
    pub fn record_allocation(&self, mem_type: super::MemoryType, size: usize) {
        let mut stats = self.stats.lock();
        
        // Update allocation statistics
        stats.allocation_stats.total_allocations += 1;
        stats.allocation_stats.current_allocations += 1;
        stats.allocation_stats.total_allocated_bytes += size as u64;
        stats.allocation_stats.current_allocated_bytes += size as u64;
        
        // Update peak allocations
        if stats.allocation_stats.current_allocations > stats.allocation_stats.peak_allocations {
            stats.allocation_stats.peak_allocations = stats.allocation_stats.current_allocations;
        }
        
        if stats.allocation_stats.current_allocated_bytes > stats.allocation_stats.peak_allocated_bytes {
            stats.allocation_stats.peak_allocated_bytes = stats.allocation_stats.current_allocated_bytes;
        }
        
        // Update memory usage by type
        *stats.memory_usage_by_type.entry(mem_type).or_insert(0) += size as u64;
    }

    /// Record deallocation
    pub fn record_deallocation(&self, mem_type: super::MemoryType, size: usize) {
        let mut stats = self.stats.lock();
        
        // Update allocation statistics
        stats.allocation_stats.total_deallocations += 1;
        stats.allocation_stats.current_allocations -= 1;
        stats.allocation_stats.total_deallocated_bytes += size as u64;
        stats.allocation_stats.current_allocated_bytes -= size as u64;
    }

    /// Record allocation failure
    pub fn record_allocation_failure(&self) {
        let mut stats = self.stats.lock();
        stats.allocation_stats.allocation_failures += 1;
    }

    /// Update NUMA statistics
    pub fn update_numa_stats(&self, node_id: u32, total_memory: u64, available_memory: u64) {
        let mut stats = self.stats.lock();
        
        // Ensure we have enough entries
        while stats.numa_stats.memory_per_node.len() <= node_id as usize {
            stats.numa_stats.memory_per_node.push(0);
            stats.numa_stats.allocation_stats_per_node.push(super::AllocationStats::default());
        }
        
        // Update NUMA node memory
        stats.numa_stats.memory_per_node[node_id as usize] = total_memory;
        
        // Update number of nodes
        stats.numa_stats.num_nodes = stats.numa_stats.memory_per_node.len() as u32;
    }

    /// Record NUMA allocation
    pub fn record_numa_allocation(&self, node_id: u32, size: usize) {
        let mut stats = self.stats.lock();
        
        // Ensure we have enough entries
        while stats.numa_stats.allocation_stats_per_node.len() <= node_id as usize {
            stats.numa_stats.allocation_stats_per_node.push(super::AllocationStats::default());
        }
        
        // Update NUMA allocation statistics
        let numa_stats = &mut stats.numa_stats.allocation_stats_per_node[node_id as usize];
        numa_stats.total_allocations += 1;
        numa_stats.current_allocations += 1;
        numa_stats.total_allocated_bytes += size as u64;
        numa_stats.current_allocated_bytes += size as u64;
        
        // Update peak allocations
        if numa_stats.current_allocations > numa_stats.peak_allocations {
            numa_stats.peak_allocations = numa_stats.current_allocations;
        }
        
        if numa_stats.current_allocated_bytes > numa_stats.peak_allocated_bytes {
            numa_stats.peak_allocated_bytes = numa_stats.current_allocated_bytes;
        }
    }

    /// Record NUMA deallocation
    pub fn record_numa_deallocation(&self, node_id: u32, size: usize) {
        let mut stats = self.stats.lock();
        
        // Ensure we have enough entries
        while stats.numa_stats.allocation_stats_per_node.len() <= node_id as usize {
            stats.numa_stats.allocation_stats_per_node.push(super::AllocationStats::default());
        }
        
        // Update NUMA allocation statistics
        let numa_stats = &mut stats.numa_stats.allocation_stats_per_node[node_id as usize];
        numa_stats.total_deallocations += 1;
        numa_stats.current_allocations -= 1;
        numa_stats.total_deallocated_bytes += size as u64;
        numa_stats.current_allocated_bytes -= size as u64;
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> super::MemoryManagementStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = super::MemoryManagementStats::default();
    }
}

/// Global memory statistics collector
static mut GLOBAL_STATS: Option<MemoryStatsCollector> = None;
static STATS_INIT: spin::Once = spin::Once::new();

/// Initialize memory statistics
pub fn init_memory_stats() -> Result<()> {
    STATS_INIT.call_once(|| {
        unsafe {
            GLOBAL_STATS = Some(MemoryStatsCollector::new());
        }
    });
    Ok(())
}

/// Get global memory statistics collector
pub fn get_stats_collector() -> &'static MemoryStatsCollector {
    unsafe {
        GLOBAL_STATS.as_ref().unwrap()
    }
}

/// Shutdown memory statistics
pub fn shutdown_memory_stats() -> Result<()> {
    unsafe {
        GLOBAL_STATS = None;
    }
    Ok(())
}

/// Get memory statistics
pub fn get_memory_stats() -> super::MemoryManagementStats {
    get_stats_collector().get_stats()
}

/// Update total physical memory
pub fn update_total_physical_memory(size: u64) {
    get_stats_collector().update_total_physical_memory(size);
}

/// Update available physical memory
pub fn update_available_physical_memory(size: u64) {
    get_stats_collector().update_available_physical_memory(size);
}

/// Update total virtual memory
pub fn update_total_virtual_memory(size: u64) {
    get_stats_collector().update_total_virtual_memory(size);
}

/// Update available virtual memory
pub fn update_available_virtual_memory(size: u64) {
    get_stats_collector().update_available_virtual_memory(size);
}

/// Record allocation
pub fn record_allocation(mem_type: super::MemoryType, size: usize) {
    get_stats_collector().record_allocation(mem_type, size);
}

/// Record deallocation
pub fn record_deallocation(mem_type: super::MemoryType, size: usize) {
    get_stats_collector().record_deallocation(mem_type, size);
}

/// Record allocation failure
pub fn record_allocation_failure() {
    get_stats_collector().record_allocation_failure();
}

/// Update NUMA statistics
pub fn update_numa_stats(node_id: u32, total_memory: u64, available_memory: u64) {
    get_stats_collector().update_numa_stats(node_id, total_memory, available_memory);
}

/// Record NUMA allocation
pub fn record_numa_allocation(node_id: u32, size: usize) {
    get_stats_collector().record_numa_allocation(node_id, size);
}

/// Record NUMA deallocation
pub fn record_numa_deallocation(node_id: u32, size: usize) {
    get_stats_collector().record_numa_deallocation(node_id, size);
}

/// Reset statistics
pub fn reset_stats() {
    get_stats_collector().reset_stats();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_stats_collector() {
        let collector = MemoryStatsCollector::new();
        
        // Record some allocations
        collector.record_allocation(MemoryType::Normal, 1024);
        collector.record_allocation(MemoryType::DMA, 2048);
        
        // Check statistics
        let stats = collector.get_stats();
        assert_eq!(stats.allocation_stats.total_allocations, 2);
        assert_eq!(stats.allocation_stats.current_allocations, 2);
        assert_eq!(stats.allocation_stats.total_allocated_bytes, 3072);
        assert_eq!(stats.allocation_stats.current_allocated_bytes, 3072);
        
        // Record deallocations
        collector.record_deallocation(MemoryType::Normal, 1024);
        
        // Check statistics
        let stats = collector.get_stats();
        assert_eq!(stats.allocation_stats.total_deallocations, 1);
        assert_eq!(stats.allocation_stats.current_allocations, 1);
        assert_eq!(stats.allocation_stats.total_deallocated_bytes, 1024);
        assert_eq!(stats.allocation_stats.current_allocated_bytes, 2048);
    }

    #[test]
    fn test_memory_stats_functions() {
        init_memory_stats().unwrap();
        
        // Update memory information
        update_total_physical_memory(1024 * 1024 * 1024); // 1GB
        update_available_physical_memory(512 * 1024 * 1024); // 512MB
        
        // Record allocations
        record_allocation(MemoryType::Normal, 1024);
        record_allocation(MemoryType::DMA, 2048);
        
        // Check statistics
        let stats = get_memory_stats();
        assert_eq!(stats.total_physical_memory, 1024 * 1024 * 1024);
        assert_eq!(stats.available_physical_memory, 512 * 1024 * 1024);
        assert_eq!(stats.allocation_stats.total_allocations, 2);
        assert_eq!(stats.allocation_stats.current_allocations, 2);
        assert_eq!(stats.allocation_stats.total_allocated_bytes, 3072);
        assert_eq!(stats.allocation_stats.current_allocated_bytes, 3072);
        
        // Record deallocations
        record_deallocation(MemoryType::Normal, 1024);
        
        // Check statistics
        let stats = get_memory_stats();
        assert_eq!(stats.allocation_stats.total_deallocations, 1);
        assert_eq!(stats.allocation_stats.current_allocations, 1);
        assert_eq!(stats.allocation_stats.total_deallocated_bytes, 1024);
        assert_eq!(stats.allocation_stats.current_allocated_bytes, 2048);
    }
}