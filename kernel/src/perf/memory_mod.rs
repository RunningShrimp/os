//! Memory Optimization Manager - Track EK
//!
//! Unified memory optimization manager coordinating all memory optimization subsystems.
//!
//! ## Features
//!
//! - **Unified Management**: Coordinate all memory optimization modules
//! - **Memory Statistics**: Comprehensive tracking and profiling
//! - **Optimization Policy**: Adaptive optimization strategies
//! - **NUMA Optimization**: NUMA-aware memory placement
//! - **Public API**: Clean interface for memory optimizations
//!
//! ## Architecture
//!
//! The memory optimization manager provides a unified interface:
//! 1. **Coordinator**: Manage all optimization modules
//! 2. **Statistics Collector**: Aggregate statistics from all modules
//! 3. **Policy Engine**: Adaptive optimization policies
//! 4. **NUMA Manager**: NUMA topology awareness
//!
//! ## Performance Targets
//!
//! - Memory allocation throughput: > 1M allocs/sec
//! - Memory overhead: < 5%
//! - NUMA locality: > 90% local allocations
//! - Optimization overhead: < 2% CPU

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::prelude::*;
use crate::subsystems::mm::PAGE_SIZE;

/// Memory optimization errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemOptError {
    /// Not initialized
    NotInitialized,
    /// Optimization failed
    OptimizationFailed,
    /// Invalid policy
    InvalidPolicy,
    /// NUMA error
    NumaError,
    /// Statistics error
    StatisticsError,
}

impl core::fmt::Display for MemOptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MemOptError::NotInitialized => write!(f, "Memory optimizer not initialized"),
            MemOptError::OptimizationFailed => write!(f, "Optimization failed"),
            MemOptError::InvalidPolicy => write!(f, "Invalid optimization policy"),
            MemOptError::NumaError => write!(f, "NUMA error"),
            MemOptError::StatisticsError => write!(f, "Statistics error"),
        }
    }
}

/// Memory optimization policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPolicy {
    /// Performance-optimized
    Performance,
    /// Memory-optimized
    Memory,
    /// Balanced
    Balanced,
    /// Power-optimized
    Power,
}

/// NUMA node information
#[derive(Debug)]
pub struct NumaNode {
    /// Node ID
    pub node_id: usize,
    /// CPU list for this node
    pub cpus: Vec<usize>,
    /// Memory size
    pub memory_size: usize,
    /// Free memory
    pub free_memory: AtomicUsize,
    /// Distance to other nodes
    pub distances: Vec<usize>,
}

impl Clone for NumaNode {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            cpus: self.cpus.clone(),
            memory_size: self.memory_size,
            free_memory: AtomicUsize::new(self.free_memory.load(Ordering::Relaxed)),
            distances: self.distances.clone(),
        }
    }
}

impl NumaNode {
    /// Create a new NUMA node
    pub fn new(node_id: usize, memory_size: usize) -> Self {
        Self {
            node_id,
            cpus: Vec::new(),
            memory_size,
            free_memory: AtomicUsize::new(memory_size),
            distances: Vec::new(),
        }
    }

    /// Allocate memory from this node
    pub fn allocate(&self, size: usize) -> Result<(), MemOptError> {
        let mut current = self.free_memory.load(Ordering::Relaxed);

        loop {
            if current < size {
                return Err(MemOptError::NumaError);
            }

            let new = current - size;

            match self.free_memory.compare_exchange_weak(
                current,
                new,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(c) => current = c,
            }
        }
    }

    /// Free memory to this node
    pub fn free(&self, size: usize) {
        let current = self.free_memory.fetch_add(size, Ordering::Relaxed);
        // Cap at memory_size
        if current + size > self.memory_size {
            self.free_memory.store(self.memory_size, Ordering::Relaxed);
        }
    }

    /// Get free memory
    pub fn get_free(&self) -> usize {
        self.free_memory.load(Ordering::Relaxed)
    }
}

/// NUMA topology manager
pub struct NumaManager {
    /// NUMA nodes
    nodes: Vec<NumaNode>,
    /// Preferred node for current CPU
    preferred_node: AtomicUsize,
    /// Migration count
    migrations: AtomicU64,
    /// Local allocations
    local_allocations: AtomicU64,
    /// Remote allocations
    remote_allocations: AtomicU64,
}

impl NumaManager {
    /// Create a new NUMA manager
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            preferred_node: AtomicUsize::new(0),
            migrations: AtomicU64::new(0),
            local_allocations: AtomicU64::new(0),
            remote_allocations: AtomicU64::new(0),
        }
    }

    /// Add NUMA node
    pub fn add_node(&mut self, node: NumaNode) {
        self.nodes.push(node);
    }

    /// Get optimal node for allocation
    pub fn get_optimal_node(&self) -> Result<usize, MemOptError> {
        let preferred = self.preferred_node.load(Ordering::Relaxed);

        if let Some(node) = self.nodes.get(preferred) {
            if node.get_free() > PAGE_SIZE {
                self.local_allocations.fetch_add(1, Ordering::Relaxed);
                return Ok(preferred);
            }
        }

        // Find node with most free memory
        let mut best_node = 0;
        let mut max_free = 0;

        for (idx, node) in self.nodes.iter().enumerate() {
            let free = node.get_free();
            if free > max_free {
                max_free = free;
                best_node = idx;
            }
        }

        if best_node != preferred {
            self.remote_allocations.fetch_add(1, Ordering::Relaxed);
        }

        Ok(best_node)
    }

    /// Allocate memory on specific node
    pub fn allocate_on_node(&self, node_id: usize, size: usize) -> Result<(), MemOptError> {
        if let Some(node) = self.nodes.get(node_id) {
            node.allocate(size)
        } else {
            Err(MemOptError::NumaError)
        }
    }

    /// Set preferred node
    pub fn set_preferred_node(&self, node_id: usize) {
        self.preferred_node.store(node_id, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> NumaStats {
        NumaStats {
            num_nodes: self.nodes.len(),
            migrations: self.migrations.load(Ordering::Relaxed),
            local_allocations: self.local_allocations.load(Ordering::Relaxed),
            remote_allocations: self.remote_allocations.load(Ordering::Relaxed),
        }
    }
}

/// NUMA statistics
#[derive(Debug, Clone)]
pub struct NumaStats {
    pub num_nodes: usize,
    pub migrations: u64,
    pub local_allocations: u64,
    pub remote_allocations: u64,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStatistics {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current memory usage
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Zero page references
    pub zero_page_refs: u64,
    /// Huge pages allocated
    pub huge_pages: u64,
    /// VMA count
    pub vma_count: usize,
    /// TLB flushes
    pub tlb_flushes: u64,
    /// NUMA local allocation ratio
    pub numa_local_ratio: f64,
}

/// Memory profiling data
#[derive(Debug, Clone)]
pub struct MemoryProfile {
    /// Allocation size histogram
    pub size_histogram: BTreeMap<usize, u64>,
    /// Allocation latency (ns)
    pub avg_latency_ns: u64,
    /// Peak throughput (allocs/sec)
    pub peak_throughput: u64,
    /// Fragmentation ratio
    pub fragmentation_ratio: f64,
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
}

/// Optimization policy engine
pub struct PolicyEngine {
    /// Current policy
    current_policy: AtomicUsize, // Stores OptimizationPolicy as usize
    /// Policy changes
    policy_changes: AtomicU64,
    /// Adaptation count
    adaptations: AtomicU64,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            current_policy: AtomicUsize::new(OptimizationPolicy::Balanced as usize),
            policy_changes: AtomicU64::new(0),
            adaptations: AtomicU64::new(0),
        }
    }

    /// Get current policy
    pub fn get_policy(&self) -> OptimizationPolicy {
        match self.current_policy.load(Ordering::Relaxed) {
            0 => OptimizationPolicy::Performance,
            1 => OptimizationPolicy::Memory,
            2 => OptimizationPolicy::Balanced,
            3 => OptimizationPolicy::Power,
            _ => OptimizationPolicy::Balanced,
        }
    }

    /// Set policy
    pub fn set_policy(&self, policy: OptimizationPolicy) {
        let old = self.current_policy.swap(policy as usize, Ordering::Relaxed);
        if old != policy as usize {
            self.policy_changes.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Adapt policy based on conditions
    pub fn adapt(&self, memory_pressure: f64, cpu_load: f64) {
        self.adaptations.fetch_add(1, Ordering::Relaxed);

        let new_policy = if memory_pressure > 0.8 {
            OptimizationPolicy::Memory
        } else if cpu_load > 0.8 {
            OptimizationPolicy::Power
        } else {
            OptimizationPolicy::Performance
        };

        self.set_policy(new_policy);
    }

    /// Get statistics
    pub fn stats(&self) -> PolicyStats {
        PolicyStats {
            current_policy: self.get_policy(),
            policy_changes: self.policy_changes.load(Ordering::Relaxed),
            adaptations: self.adaptations.load(Ordering::Relaxed),
        }
    }
}

/// Policy statistics
#[derive(Debug, Clone)]
pub struct PolicyStats {
    pub current_policy: OptimizationPolicy,
    pub policy_changes: u64,
    pub adaptations: u64,
}

/// Main memory optimization manager
pub struct MemoryOptimizationManager {
    /// NUMA manager
    numa: NumaManager,
    /// Policy engine
    policy: PolicyEngine,
    /// Memory statistics
    stats: MemoryStatistics,
    /// Memory profile
    profile: MemoryProfile,
    /// Enabled flag
    enabled: AtomicBool,
}

impl MemoryOptimizationManager {
    /// Create a new memory optimization manager
    pub fn new() -> Self {
        Self {
            numa: NumaManager::new(),
            policy: PolicyEngine::new(),
            stats: MemoryStatistics {
                total_allocations: 0,
                total_deallocations: 0,
                current_usage: 0,
                peak_usage: 0,
                zero_page_refs: 0,
                huge_pages: 0,
                vma_count: 0,
                tlb_flushes: 0,
                numa_local_ratio: 1.0,
            },
            profile: MemoryProfile {
                size_histogram: BTreeMap::new(),
                avg_latency_ns: 0,
                peak_throughput: 0,
                fragmentation_ratio: 0.0,
                cache_hit_ratio: 0.0,
            },
            enabled: AtomicBool::new(true),
        }
    }

    /// Enable/disable optimizations
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Record allocation
    pub fn record_allocation(&mut self, size: usize) {
        self.stats.total_allocations += 1;
        self.stats.current_usage += size;

        if self.stats.current_usage > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.current_usage;
        }

        // Update size histogram
        *self.profile.size_histogram.entry(size).or_insert(0) += 1;
    }

    /// Record deallocation
    pub fn record_deallocation(&mut self, size: usize) {
        self.stats.total_deallocations += 1;
        self.stats.current_usage = self.stats.current_usage.saturating_sub(size);
    }

    /// Get NUMA manager (immutable)
    pub fn get_numa(&self) -> &NumaManager {
        &self.numa
    }

    /// Get NUMA manager (mutable)
    pub fn get_numa_mut(&mut self) -> &mut NumaManager {
        &mut self.numa
    }

    /// Get policy engine
    pub fn get_policy(&self) -> &PolicyEngine {
        &self.policy
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> &MemoryStatistics {
        &self.stats
    }

    /// Get memory profile
    pub fn get_profile(&self) -> &MemoryProfile {
        &self.profile
    }

    /// Update statistics from subsystems
    pub fn update_stats(&mut self, stats_update: MemoryStatsUpdate) {
        if let Some(zero_refs) = stats_update.zero_page_refs {
            self.stats.zero_page_refs = zero_refs;
        }

        if let Some(huge_pages) = stats_update.huge_pages {
            self.stats.huge_pages = huge_pages;
        }

        if let Some(vma_count) = stats_update.vma_count {
            self.stats.vma_count = vma_count;
        }

        if let Some(tlb_flushes) = stats_update.tlb_flushes {
            self.stats.tlb_flushes = tlb_flushes;
        }

        if let Some(local_allocs) = stats_update.numa_local {
            let remote = stats_update.numa_remote.unwrap_or(0);
            let total = local_allocs + remote;
            if total > 0 {
                self.stats.numa_local_ratio = local_allocs as f64 / total as f64;
            }
        }

        if let Some(hit_ratio) = stats_update.cache_hit_ratio {
            self.profile.cache_hit_ratio = hit_ratio;
        }
    }

    /// Adapt optimization policy
    pub fn adapt_policy(&self, memory_pressure: f64, cpu_load: f64) {
        self.policy.adapt(memory_pressure, cpu_load);
    }

    /// Get comprehensive report
    pub fn get_report(&self) -> MemoryOptimizationReport {
        MemoryOptimizationReport {
            statistics: self.stats.clone(),
            profile: self.profile.clone(),
            numa: self.numa.stats(),
            policy: self.policy.stats(),
            enabled: self.enabled.load(Ordering::Relaxed),
        }
    }
}

/// Memory statistics update
pub struct MemoryStatsUpdate {
    pub zero_page_refs: Option<u64>,
    pub huge_pages: Option<u64>,
    pub vma_count: Option<usize>,
    pub tlb_flushes: Option<u64>,
    pub numa_local: Option<u64>,
    pub numa_remote: Option<u64>,
    pub cache_hit_ratio: Option<f64>,
}

/// Memory optimization report
#[derive(Debug, Clone)]
pub struct MemoryOptimizationReport {
    pub statistics: MemoryStatistics,
    pub profile: MemoryProfile,
    pub numa: NumaStats,
    pub policy: PolicyStats,
    pub enabled: bool,
}

/// Global memory optimization manager instance
static GLOBAL_MEMORY_OPTIMIZER: Mutex<Option<MemoryOptimizationManager>> = Mutex::new(None);

/// Initialize memory optimization manager
pub fn init_memory_optimization() {
    log::info!("Initializing memory optimization manager...");

    let mut manager = MemoryOptimizationManager::new();

    // Initialize NUMA topology (simplified - single node)
    let node = NumaNode::new(0, 1024 * 1024 * 1024); // 1GB
    manager.get_numa_mut().add_node(node);

    *GLOBAL_MEMORY_OPTIMIZER.lock() = Some(manager);

    log::info!("Memory optimization manager initialized");
}

/// Get global memory optimization manager
pub fn get_memory_optimizer() -> Option<&'static Mutex<Option<MemoryOptimizationManager>>> {
    Some(&GLOBAL_MEMORY_OPTIMIZER)
}

/// Enable memory optimizations
pub fn enable_optimizations() {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        optimizer.set_enabled(true);
    }
}

/// Disable memory optimizations
pub fn disable_optimizations() {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        optimizer.set_enabled(false);
    }
}

/// Get memory statistics
pub fn get_memory_statistics() -> Result<MemoryStatistics, MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        Ok(optimizer.get_stats().clone())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Get memory profile
pub fn get_memory_profile() -> Result<MemoryProfile, MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        Ok(optimizer.get_profile().clone())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Get NUMA statistics
pub fn get_numa_statistics() -> Result<NumaStats, MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        Ok(optimizer.get_numa().stats())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Get optimization policy
pub fn get_optimization_policy() -> Result<OptimizationPolicy, MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        Ok(optimizer.get_policy().get_policy())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Set optimization policy
pub fn set_optimization_policy(policy: OptimizationPolicy) -> Result<(), MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        optimizer.get_policy().set_policy(policy);
        Ok(())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Allocate NUMA-aware memory
pub fn numa_allocate(size: usize) -> Result<usize, MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        let numa = optimizer.get_numa();
        let node_id = numa.get_optimal_node()?;
        numa.allocate_on_node(node_id, size)?;
        Ok(node_id)
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Update memory statistics
pub fn update_statistics(update: MemoryStatsUpdate) -> Result<(), MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_mut() {
        optimizer.update_stats(update);
        Ok(())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Get comprehensive report
pub fn get_optimization_report() -> Result<MemoryOptimizationReport, MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        Ok(optimizer.get_report())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Adapt optimization policy
pub fn adapt_optimization_policy(memory_pressure: f64, cpu_load: f64) -> Result<(), MemOptError> {
    if let Some(optimizer) = GLOBAL_MEMORY_OPTIMIZER.lock().as_ref() {
        optimizer.adapt_policy(memory_pressure, cpu_load);
        Ok(())
    } else {
        Err(MemOptError::NotInitialized)
    }
}

/// Public API exports
pub mod api {
    pub use super::{
        init_memory_optimization,
        enable_optimizations,
        disable_optimizations,
        get_memory_statistics,
        get_memory_profile,
        get_numa_statistics,
        get_optimization_policy,
        set_optimization_policy,
        numa_allocate,
        update_statistics,
        get_optimization_report,
        adapt_optimization_policy,
        OptimizationPolicy,
        MemoryStatistics,
        MemoryProfile,
        NumaStats,
        MemoryOptimizationReport,
        MemOptError,
    };
}
