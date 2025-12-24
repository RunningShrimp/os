use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::mm::types::MemoryType;

pub use crate::subsystems::mm::types::MemoryType;

/// Memory management statistics
#[derive(Debug, Clone)]
pub struct MemoryManagementStats {
    pub total_physical_memory: u64,
    pub available_physical_memory: u64,
    pub total_virtual_memory: u64,
    pub available_virtual_memory: u64,
    pub memory_usage_by_type: BTreeMap<MemoryType, u64>,
    pub allocation_stats: AllocationStats,
    pub numa_stats: NumStats,
}

impl Default for MemoryManagementStats {
    fn default() -> Self {
        Self {
            total_physical_memory: 0,
            available_physical_memory: 0,
            total_virtual_memory: 0,
            available_virtual_memory: 0,
            memory_usage_by_type: BTreeMap::new(),
            allocation_stats: AllocationStats::default(),
            numa_stats: NumStats::default(),
        }
    }
}

/// Allocation statistics - unified version supporting both atomic and non-atomic use cases
#[derive(Debug, Clone)]
pub struct AllocationStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub current_allocations: u64,
    pub peak_allocations: u64,
    pub total_allocated_bytes: u64,
    pub total_deallocated_bytes: u64,
    pub current_allocated_bytes: u64,
    pub peak_allocated_bytes: u64,
    pub allocation_failures: u64,
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

impl AllocationStats {
    pub fn record_allocation(&mut self, size: u64) {
        self.total_allocations += 1;
        self.current_allocations += 1;
        self.total_allocated_bytes += size;
        self.current_allocated_bytes += size;
        
        if self.current_allocations > self.peak_allocations {
            self.peak_allocations = self.current_allocations;
        }
        if self.current_allocated_bytes > self.peak_allocated_bytes {
            self.peak_allocated_bytes = self.current_allocated_bytes;
        }
    }
    
    pub fn record_deallocation(&mut self, size: u64) {
        self.total_deallocations += 1;
        self.current_allocations = self.current_allocations.saturating_sub(1);
        self.total_deallocated_bytes += size;
        self.current_allocated_bytes = self.current_allocated_bytes.saturating_sub(size);
    }
    
    pub fn record_failure(&mut self) {
        self.allocation_failures += 1;
    }
}

/// Atomic allocation statistics for concurrent access
#[derive(Debug)]
pub struct AtomicAllocationStats {
    pub total_allocations: AtomicU64,
    pub total_deallocations: AtomicU64,
    pub current_allocations: AtomicU64,
    pub peak_allocations: AtomicU64,
    pub total_allocated_bytes: AtomicU64,
    pub total_deallocated_bytes: AtomicU64,
    pub current_allocated_bytes: AtomicU64,
    pub peak_allocated_bytes: AtomicU64,
    pub allocation_failures: AtomicU64,
}

impl Default for AtomicAllocationStats {
    fn default() -> Self {
        Self {
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            current_allocations: AtomicU64::new(0),
            peak_allocations: AtomicU64::new(0),
            total_allocated_bytes: AtomicU64::new(0),
            total_deallocated_bytes: AtomicU64::new(0),
            current_allocated_bytes: AtomicU64::new(0),
            peak_allocated_bytes: AtomicU64::new(0),
            allocation_failures: AtomicU64::new(0),
        }
    }
}

impl AtomicAllocationStats {
    pub fn record_allocation(&self, size: u64) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_allocations.fetch_add(1, Ordering::Relaxed) + 1;
        self.total_allocated_bytes.fetch_add(size, Ordering::Relaxed);
        let current_bytes = self.current_allocated_bytes.fetch_add(size, Ordering::Relaxed) + size;
        
        self.peak_allocations.fetch_max(current, Ordering::Relaxed);
        self.peak_allocated_bytes.fetch_max(current_bytes, Ordering::Relaxed);
    }
    
    pub fn record_deallocation(&self, size: u64) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_allocations.fetch_sub(1, Ordering::Relaxed);
        self.total_deallocated_bytes.fetch_add(size, Ordering::Relaxed);
        self.current_allocated_bytes.fetch_sub(size, Ordering::Relaxed);
    }
    
    pub fn record_failure(&self) {
        self.allocation_failures.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn snapshot(&self) -> AllocationStats {
        AllocationStats {
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.total_deallocations.load(Ordering::Relaxed),
            current_allocations: self.current_allocations.load(Ordering::Relaxed),
            peak_allocations: self.peak_allocations.load(Ordering::Relaxed),
            total_allocated_bytes: self.total_allocated_bytes.load(Ordering::Relaxed),
            total_deallocated_bytes: self.total_deallocated_bytes.load(Ordering::Relaxed),
            current_allocated_bytes: self.current_allocated_bytes.load(Ordering::Relaxed),
            peak_allocated_bytes: self.peak_allocated_bytes.load(Ordering::Relaxed),
            allocation_failures: self.allocation_failures.load(Ordering::Relaxed),
        }
    }
}

/// NUMA statistics
#[derive(Debug, Clone)]
pub struct NumStats {
    pub num_nodes: u32,
    pub memory_per_node: Vec<u64>,
    pub allocation_stats_per_node: Vec<AllocationStats>,
}

impl Default for NumStats {
    fn default() -> Self {
        Self {
            num_nodes: 0,
            memory_per_node: Vec::new(),
            allocation_stats_per_node: Vec::new(),
        }
    }
}

/// Lightweight allocation statistics for performance-critical paths
#[derive(Debug, Default)]
pub struct LightweightAllocationStats {
    pub fast_path_hits: AtomicUsize,
    pub slow_path_allocations: AtomicUsize,
    pub failed_allocations: AtomicUsize,
}

impl LightweightAllocationStats {
    pub fn record_fast_path(&self) {
        self.fast_path_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_slow_path(&self) {
        self.slow_path_allocations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_failure(&self) {
        self.failed_allocations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.fast_path_hits.load(Ordering::Relaxed),
            self.slow_path_allocations.load(Ordering::Relaxed),
            self.failed_allocations.load(Ordering::Relaxed),
        )
    }
    
    pub fn hit_ratio(&self) -> f64 {
        let hits = self.fast_path_hits.load(Ordering::Relaxed) as f64;
        let total = (hits + self.slow_path_allocations.load(Ordering::Relaxed) as f64);
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }
}

/// Extended allocation statistics with defragmentation tracking
#[derive(Debug)]
pub struct ExtendedAllocationStats {
    pub base: AtomicAllocationStats,
    pub fast_path_hits: AtomicUsize,
    pub slow_path_allocations: AtomicUsize,
    pub defragmentation_runs: AtomicUsize,
}

impl Default for ExtendedAllocationStats {
    fn default() -> Self {
        Self {
            base: AtomicAllocationStats::default(),
            fast_path_hits: AtomicUsize::new(0),
            slow_path_allocations: AtomicUsize::new(0),
            defragmentation_runs: AtomicUsize::new(0),
        }
    }
}

impl ExtendedAllocationStats {
    pub fn record_fast_path(&self, size: u64) {
        self.fast_path_hits.fetch_add(1, Ordering::Relaxed);
        self.base.record_allocation(size);
    }
    
    pub fn record_slow_path(&self, size: u64) {
        self.slow_path_allocations.fetch_add(1, Ordering::Relaxed);
        self.base.record_allocation(size);
    }
    
    pub fn record_deallocation(&self, size: u64) {
        self.base.record_deallocation(size);
    }
    
    pub fn record_failure(&self) {
        self.base.record_failure();
    }
    
    pub fn record_defragmentation(&self) {
        self.defragmentation_runs.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn snapshot(&self) -> AllocationStats {
        self.base.snapshot()
    }
    
    pub fn get_extended_stats(&self) -> (AllocationStats, usize, usize, usize) {
        (
            self.snapshot(),
            self.fast_path_hits.load(Ordering::Relaxed),
            self.slow_path_allocations.load(Ordering::Relaxed),
            self.defragmentation_runs.load(Ordering::Relaxed),
        )
    }
}
