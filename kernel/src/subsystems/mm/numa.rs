//! NUMA-aware memory allocation support
//!
//! This module provides comprehensive NUMA (Non-Uniform Memory Access) support for
//! memory allocation. NUMA allows to kernel to allocate memory from the
//! closest memory node to the CPU, improving performance by reducing
//! memory access latency.

use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use alloc::vec::Vec;
use crate::subsystems::sync::Mutex;
use crate::subsystems::mm::unified_stats::AtomicAllocationStats;
use nos_api::{Result, Error};

/// NUMA node identifier
pub type NodeId = usize;

/// Maximum number of NUMA nodes supported
pub const MAX_NUMA_NODES: usize = 8;

/// Memory zone types
#[derive(Debug, Clone, Copy)]
pub enum MemoryZoneType {
    Normal,       // Normal memory
    HighMem,      // High memory (for systems with >4GB memory)
    DeviceMem,    // Device memory
    HugepageMem,  // Huge page memory
}

/// Memory zone information
pub struct MemoryZone {
    zone_type: MemoryZoneType,
    start_address: usize,
    end_address: usize,
    free_memory: AtomicUsize,
    total_memory: usize,
    page_size: usize,
    allocation_bitmap: Vec<usize>,
    next_free_page: AtomicUsize,
}

/// NUMA node information
pub struct NumaNode {
    node_id: NodeId,
    cpu_mask: u64, // CPUs associated with this node
    memory_zones: Vec<MemoryZone>,
    free_memory: AtomicUsize,
    total_memory: usize,
    distance: [u8; MAX_tomicANUMA_NODES], // Distance to other nodes
    allocation_stats: AllocationStats,
    preferred_zone: Option<MemoryZoneType>,
}
o rt_allocat policd
/// NUMA allocation policy
#[derive(Debug, Clone, Copy)]
pub enum NumaPolicy {
    LocalNode,    // Allocate from the local node (closest to current CPU)
    AnyNode,      // Allocate from any available node
    SpecificNode(NodeId), // Allocate from a specific node
    Interleave,   // Interleave allocation across all nodes
    PreferNode(NodeId), // Prefer a specific node but fallback to others
}

impl NumaNode {
    /// Create a new NUMA node
    pub const fn new(node_id: NodeId, cpu_mask: u64) -> Self {
        Self {
            node_id,
            cpu_mask,
            memory_zones: Vec::new(),
            free_memory: AtomicUsize::new(0),
            total_memory: 0,
            distance: [10; MAXNODES], // Default distance
            allocation_stats: AllocationStats::default(),
            preferred_zone: None,
        }
    }
    
    /// Add a memory zone to this NUMA node
    pub unsafe fn add_memory_zone(&mut self, zone_type: MemoryZoneType, start: usize, end: usize, page_size: usize) {
        let zone_size = end - start;
        let page_count = zone_size / page_size;
        let bitmap_size = (page_count + 63) / 64; // Bits in usize
        
        let mut allocation_bitmap = Vec::with_capacity(bitmap_size);
        for _ in 0..bitmap_size {
            allocation_bitmap.push(0);
        }
        
        let zone = MemoryZone {
            zone_type,
            start_address: start,
            end_address: end,
            free_memory: AtomicUsize::new(zone_size),
            total_memory: zone_size,
            page_size,
            allocation_bitmap,
            next_free_page: AtomicUsize::new(0),
        };
        
        self.memory_zones.push(zone);
        self.total_memory += zone_size;
        self.free_memory.fetch_add(zone_size, Ordering::Relaxed);
    }
    
    /// Get the amount of free memory in this node
    pub fn free_memory(&self) -> usize {
        self.free_memory.load(Ordering::Relaxed)
    }
    
    /// Get the total memory in this node
    pub fn total_memory(&self) -> usize {
        self.total_memory
    }
    
    /// Check if a CPU is associated with this node
    pub fn has_cpu(&self, cpu: usize) -> bool {
        if cpu >= 64 {
            false
        } else {
            (self.cpu_mask & (1 << cpu)) != 0
        }
    }
    
    /// Set the distance to another NUMA node
    pub fn set_distance(&mut self, target_node: NodeId, distance: u8) {
        if target_node < MAX_NUMA_NODES {
            self.distance[target_node] = distance;
        }
    }
    
    /// Get the distance to another NUMA node
    pub fn get_distance(&self, target_node: NodeId) -> u8 {
        if target_node < MAX_NUMA_NODES {
            self.distance[target_node]
        } else {
            255 // Maximum distance for unknown nodes
        }
    }
    
    /// Find a suitable memory zone for allocation
    pub fn find_suitable_zone(&self, size: usize, zone_type: Option<MemoryZoneType>) -> Option<usize> {
        for (zone_idx, zone) in self.memory_zones.iter().enumerate() {
            // Check if zone type matches preference
            if let Some(preferred_type) = zone_type {
                if zone.zone_type != preferred_type {
                    continue;
                }
            }
            
            // Check if zone has enough free memory
            if zone.free_memory.load(Ordering::Relaxed) >= size {
                return Some(zone_idx);
            }
        }
        None
    }
    
    /// Allocate pages from a specific zone
    pub fn allocate_from_zone(&self, zone_idx: usize, page_count: usize) -> Result<*mut u8> {
        if zone_idx >= self.memory_zones.len() {
            return Err(Error::InvalidArgument);
        }
        
        let zone = &self.memory_zones[zone_idx];
        let size = page_count * zone.page_size;
        
        // Check if we have enough free memory
        if zone.free_memory.load(Ordering::Relaxed) < size {
            self.allocation_stats.allocation_failures.fetch_add(1, Ordering::Relaxed);
            return Err(Error::OutOfMemory);
        }
        
        // Find contiguous pages
        let start_page = self.find_contiguous_pages(zone_idx, page_count)?;
        let address = zone.start_address + start_page * zone.page_size;
        
        // Update statistics
        zone.free_memory.fetch_sub(size, Ordering::Relaxed);
        self.free_memory.fetch_sub(size, Ordering::Relaxed);
        
        self.allocation_stats.total_allocations.fetch_add(1, Ordering::Relaxed);
        self.allocation_stats.current_allocations.fetch_add(1, Ordering::Relaxed);
        self.allocation_stats.total_allocated_bytes.fetch_add(size, Ordering::Relaxed);
        self.allocation_stats.current_allocated_bytes.fetch_add(size, Ordering::Relaxed);
        
        // Update peak allocations
        let current = self.allocation_stats.current_allocations.load(Ordering::Relaxed);
        let peak = self.allocation_stats.peak_allocations.load(Ordering::Relaxed);
        if current > peak {
            self.allocation_stats.peak_allocations.store(current, Ordering::Relaxed);
        }
        
        let current_bytes = self.allocation_stats.current_allocated_bytes.load(Ordering::Relaxed);
        let peak_bytes = self.allocation_stats.peak_allocated_bytes.load(Ordering::Relaxed);
        if current_bytes > peak_bytes {
            self.allocation_stats.peak_allocated_bytes.store(current_bytes, Ordering::Relaxed);
        }
        
        Ok(address as *mut u8)
    }
    
    /// Find contiguous pages in a zone
    fn find_contiguous_pages(&self, zone_idx: usize, page_count: usize) -> Result<usize> {
        if zone_idx >= self.memory_zones.len() {
            return Err(Error::InvalidArgument);
        }
        
        let zone = &self.memory_zones[zone_idx];
        let total_pages = zone.total_memory / zone.page_size;
        
        // Simple linear search for contiguous pages
        // In a real implementation, we would use a more sophisticated allocator
        let mut consecutive = 0;
        let mut start_page = 0;
        
        for page in 0..total_pages {
            let bitmap_idx = page / 64;
            let bit_idx = page % 64;
            
            if (zone.allocation_bitmap[bitmap_idx] & (1 << bit_idx)) == 0 {
                if consecutive == 0 {
                    start_page = page;
                }
                consecutive += 1;
                
                if consecutive >= page_count {
                    return Ok(start_page);
                }
            } else {
                consecutive = 0;
            }
        }
        
        Err(Error::OutOfMemory)
    }
    
    /// Deallocate pages to a specific zone
    pub fn deallocate_to_zone(&self, zone_idx: usize, ptr: *mut u8, page_count: usize) -> Result<()> {
        if zone_idx >= self.memory_zones.len() {
            return Err(Error::InvalidArgument);
        }
        
        let zone = &self.memory_zones[zone_idx];
        let address = ptr as usize;
        
        // Verify address is within zone bounds
        if address < zone.start_address || address >= zone.end_address {
            return Err(Error::InvalidArgument);
        }
        
        let start_page = (address - zone.start_address) / zone.page_size;
        let size = page_count * zone.page_size;
        
        // Mark pages as free in bitmap
        for page in start_page..start_page + page_count {
            let bitmap_idx = page / 64;
            let bit_idx = page % 64;
            zone.allocation_bitmap[bitmap_idx] &= !(1 << bit_idx);
        }
        
        // Update statistics
        zone.free_memory.fetch_add(size, Ordering::Relaxed);
        self.free_memory.fetch_add(size, Ordering::Relaxed);
        
        self.allocation_stats.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.allocation_stats.current_allocations.fetch_sub(1, Ordering::Relaxed);
        self.allocation_stats.total_deallocated_bytes.fetch_add(size, Ordering::Relaxed);
        self.allocation_stats.current_allocated_bytes.fetch_sub(size, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Get allocation statistics
    pub fn get_allocation_stats(&self) -> AllocationStats {
        AllocationStats {
            total_allocations: AtomicUsize::new(self.allocation_stats.total_allocations.load(Ordering::Relaxed)),
            total_deallocations: AtomicUsize::new(self.allocation_stats.total_deallocations.load(Ordering::Relaxed)),
            current_allocations: AtomicUsize::new(self.allocation_stats.current_allocations.load(Ordering::Relaxed)),
            peak_allocations: AtomicUsize::new(self.allocation_stats.peak_allocations.load(Ordering::Relaxed)),
            total_allocated_bytes: AtomicUsize::new(self.allocation_stats.total_allocated_bytes.load(Ordering::Relaxed)),
            total_deallocated_bytes: AtomicUsize::new(self.allocation_stats.total_deallocated_bytes.load(Ordering::Relaxed)),
            current_allocated_bytes: AtomicUsize::new(self.allocation_stats.current_allocated_bytes.load(Ordering::Relaxed)),
            peak_allocated_bytes: AtomicUsize::new(self.allocation_stats.peak_allocated_bytes.load(Ordering::Relaxed)),
            allocation_failures: AtomicUsize::new(self.allocation_stats.allocation_failures.load(Ordering::Relaxed)),
        }
    }
}

/// NUMA controller
pub struct NumaController {
    nodes: Vec<Mutex<NumaNode>>,
    current_cpu: AtomicUsize,
    default_policy: AtomicPtr<NumaPolicy>,
    interleave_counter: AtomicUsize,
    node_distances: [[u8; MAX_NUMA_NODES]; MAX_NUMA_NODES],
}

impl NumaController {
    /// Create a new NUMA controller
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            current_cpu: AtomicUsize::new(0),
            default_policy: AtomicPtr::new(null_mut()),
            interleave_counter: AtomicUsize::new(0),
            node_distances: [[10; MAX_NUMA_NODES]; MAX_NUMA_NODES], // Default distances
        }
    }
    
    /// Add a NUMA node to the controller
    pub fn add_node(&mut self, node: NumaNode) {
        self.nodes.push(Mutex::new(node));
    }
    
    /// Set the current CPU for NUMA allocation decisions
    pub fn set_current_cpu(&self, cpu: usize) {
        self.current_cpu.store(cpu, Ordering::Relaxed);
    }
    
    /// Get the current CPU
    pub fn current_cpu(&self) -> usize {
        self.current_cpu.load(Ordering::Relaxed)
    }
    
    /// Set the default NUMA allocation policy
    pub fn set_default_policy(&self, policy: NumaPolicy) {
        // Store the policy on the heap and update the atomic pointer
        // This is a simplified approach - in a real implementation we would use more sophisticated methods
        let policy_ptr = Box::into_raw(Box::new(policy));
        self.default_policy.store(policy_ptr, Ordering::Relaxed);
    }
    
    /// Get the default NUMA allocation policy
    pub fn get_default_policy(&self) -> NumaPolicy {
        let policy_ptr = self.default_policy.load(Ordering::Relaxed);
        if policy_ptr.is_null() {
            NumaPolicy::LocalNode
        } else {
            unsafe { *policy_ptr }
        }
    }
    
    /// Get the NUMA node for the current CPU using LocalNode policy
    pub fn get_local_node_id(&self) -> Option<NodeId> {
        let current_cpu = self.current_cpu();
        
        for (node_id, node) in self.nodes.iter().enumerate() {
            let node_guard = node.lock();
            if node_guard.has_cpu(current_cpu) {
                return Some(node_id);
            }
        }
        
        None // No node found for current CPU
    }
    
    /// Get the NUMA node with the most free memory using AnyNode policy
    pub fn get_node_with_most_free(&self) -> Option<NodeId> {
        let mut best_node = None;
        let mut max_free = 0;
        
        for (node_id, node) in self.nodes.iter().enumerate() {
            let node_guard = node.lock();
            let free = node_guard.free_memory();
            
            if free > max_free {
                max_free = free;
                best_node = Some(node_id);
            }
        }
        
        best_node
    }
    
    /// Get the next node for interleaved allocation
    pub fn get_interleave_node(&self) -> Option<NodeId> {
        if self.nodes.is_empty() {
            return None;
        }
        
        let node_count = self.nodes.len();
        let counter = self.interleave_counter.fetch_add(1, Ordering::Relaxed);
        let node_id = counter % node_count;
        
        Some(node_id)
    }
    
    /// Select a NUMA node based on policy
    pub fn select_node(&self, policy: NumaPolicy, size: usize) -> Option<NodeId> {
        match policy {
            NumaPolicy::LocalNode => self.get_local_node_id(),
            NumaPolicy::AnyNode => self.get_node_with_most_free(),
            NumaPolicy::SpecificNode(node_id) => {
                if node_id < self.nodes.len() {
                    Some(node_id)
                } else {
                    None
                }
            }
            NumaPolicy::Interleave => self.get_interleave_node(),
            NumaPolicy::PreferNode(node_id) => {
                // Try preferred node first, fallback to any node
                if node_id < self.nodes.len() {
                    let node_guard = self.nodes[node_id].lock();
                    if node_guard.free_memory() >= size {
                        return Some(node_id);
                    }
                }
                self.get_node_with_most_free()
            }
        }
    }
    
    /// Allocate memory with NUMA awareness
    pub fn allocate(&self, size: usize, align: usize, policy: NumaPolicy) -> Result<*mut u8> {
        if size == 0 {
            return Ok(null_mut());
        }
        
        // Select a node based on policy
        let node_id = self.select_node(policy, size).ok_or(Error::OutOfMemory)?;
        
        // Get the node
        let node_guard = self.nodes[node_id].lock();
        
        // Find a suitable zone
        let zone_idx = node_guard.find_suitable_zone(size, None).ok_or(Error::OutOfMemory)?;
        
        // Calculate page count (round up to nearest page)
        let page_size = 4096; // Default page size
        let page_count = (size + page_size - 1) / page_size;
        
        // Allocate from the zone
        node_guard.allocate_from_zone(zone_idx, page_count)
    }
    
    /// Deallocate memory
    pub fn deallocate(&self, ptr: *mut u8, size: usize) -> Result<()> {
        if ptr.is_null() || size == 0 {
            return Ok(());
        }
        
        // Find which node contains this address
        let address = ptr as usize;
        
        for (node_id, node) in self.nodes.iter().enumerate() {
            let node_guard = node.lock();
            
            // Check each zone in this node
            for (zone_idx, zone) in node_guard.memory_zones.iter().enumerate() {
                if address >= zone.start_address && address < zone.end_address {
                    // Found the zone, deallocate from it
                    let page_size = zone.page_size;
                    let page_count = (size + page_size - 1) / page_size;
                    return node_guard.deallocate_to_zone(zone_idx, ptr, page_count);
                }
            }
        }
        
        Err(Error::InvalidArgument)
    }
    
    /// Get NUMA statistics
    pub fn get_numa_stats(&self) -> NumStats {
        let mut stats = NumStats::default();
        stats.num_nodes = self.nodes.len() as u32;
        
        for node in &self.nodes {
            let node_guard = node.lock();
            stats.memory_per_node.push(node_guard.total_memory() as u64);
            stats.allocation_stats_per_node.push(node_guard.get_allocation_stats());
        }
        
        stats
    }
    
    /// Set distance between nodes
    pub fn set_node_distance(&mut self, from_node: NodeId, to_node: NodeId, distance: u8) {
        if from_node < MAX_NUMA_NODES && to_node < MAX_NUMA_NODES {
            self.node_distances[from_node][to_node] = distance;
            
            // Also update the node's distance table
            if from_node < self.nodes.len() {
                let mut node_guard = self.nodes[from_node].lock();
                node_guard.set_distance(to_node, distance);
            }
        }
    }
    
    /// Get distance between nodes
    pub fn get_node_distance(&self, from_node: NodeId, to_node: NodeId) -> u8 {
        if from_node < MAX_NUMA_NODES && to_node < MAX_NUMA_NODES {
            self.node_distances[from_node][to_node]
        } else {
            255 // Maximum distance for unknown nodes
        }
    }
}

/// Global NUMA controller instance
static NUMA_CONTROLLER: Mutex<NumaController> = Mutex::new(NumaController::new());

/// Initialize the NUMA controller
pub fn init_numa() -> Result<()> {
    // Detect NUMA nodes from hardware information
    // For now, we'll assume a single NUMA node with all CPUs
    let mut node = NumaNode::new(0, 0xffffffffffffffff); // All CPUs
    
    // Add memory zones to the node
    // In a real implementation, we would detect these from hardware
    unsafe {
        // Normal memory zone (0 - 2GB)
        node.add_memory_zone(MemoryZoneType::Normal, 0x00000000, 0x80000000, 4096);
        
        // High memory zone (2GB - 4GB)
        node.add_memory_zone(MemoryZoneType::HighMem, 0x80000000, 0x100000000, 4096);
    }
    
    let mut controller = NUMA_CONTROLLER.lock();
    controller.add_node(node);
    
    // Set default policy to local node allocation
    controller.set_default_policy(NumaPolicy::LocalNode);
    
    Ok(())
}

/// Shutdown the NUMA controller
pub fn shutdown_numa() -> Result<()> {
    // Clean up NUMA controller resources
    let controller = NUMA_CONTROLLER.lock();
    
    // In a real implementation, we would clean up allocated memory
    // and other resources here
    
    Ok(())
}

/// Allocate memory with NUMA awareness
pub unsafe fn numa_alloc(size: usize, policy: NumaPolicy) -> *mut u8 {
    let controller = NUMA_CONTROLLER.lock();
    match controller.allocate(size, 8, policy) {
        Ok(ptr) => ptr,
        Err(_) => null_mut(),
    }
}

/// Allocate memory with a specific alignment and NUMA policy
pub unsafe fn numa_alloc_aligned(size: usize, align: usize, policy: NumaPolicy) -> *mut u8 {
    let controller = NUMA_CONTROLLER.lock();
    match controller.allocate(size, align, policy) {
        Ok(ptr) => ptr,
        Err(_) => null_mut(),
    }
}

/// Allocate zero-initialized memory with NUMA awareness
pub unsafe fn numa_alloc_zeroed(size: usize, policy: NumaPolicy) -> *mut u8 {
    let ptr = numa_alloc(size, policy);
    if !ptr.is_null() {
        // Zero initialize the memory
        core::ptr::write_bytes(ptr, 0, size);
    }
    ptr
}

/// Deallocate memory allocated with NUMA-aware allocation
pub unsafe fn numa_dealloc(ptr: *mut u8, size: usize) {
    let controller = NUMA_CONTROLLER.lock();
    let _ = controller.deallocate(ptr, size);
}

/// Get the NUMA node for a given memory address
pub fn numa_node_for_address(addr: *mut u8) -> NodeId {
    let address = addr as usize;
    let controller = NUMA_CONTROLLER.lock();
    
    // Find which node contains this address
    for (node_id, node) in controller.nodes.iter().enumerate() {
        let node_guard = node.lock();
        
        // Check each zone in this node
        for zone in &node_guard.memory_zones {
            if address >= zone.start_address && address < zone.end_address {
                return node_id;
            }
        }
    }
    
    0 // Default to node 0 if not found
}

/// Set the default NUMA allocation policy
pub fn numa_set_default_policy(policy: NumaPolicy) {
    let controller = NUMA_CONTROLLER.lock();
    controller.set_default_policy(policy);
}

/// Get the default NUMA allocation policy
pub fn numa_get_default_policy() -> NumaPolicy {
    let controller = NUMA_CONTROLLER.lock();
    controller.get_default_policy()
}

/// Set the current CPU for NUMA allocation decisions
pub fn numa_set_current_cpu(cpu: usize) {
    let controller = NUMA_CONTROLLER.lock();
    controller.set_current_cpu(cpu);
}

/// Get the current CPU
pub fn numa_get_current_cpu() -> usize {
    let controller = NUMA_CONTROLLER.lock();
    controller.current_cpu()
}

/// Get NUMA statistics
pub fn numa_get_stats() -> NumStats {
    let controller = NUMA_CONTROLLER.lock();
    controller.get_numa_stats()
}

/// Get distance between NUMA nodes
pub fn numa_get_node_distance(from_node: NodeId, to_node: NodeId) -> u8 {
    let controller = NUMA_CONTROLLER.lock();
    controller.get_node_distance(from_node, to_node)
}

/// Set distance between NUMA nodes
pub fn numa_set_node_distance(from_node: NodeId, to_node: NodeId, distance: u8) {
    let mut controller = NUMA_CONTROLLER.lock();
    controller.set_node_distance(from_node, to_node, distance);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_node() {
        let mut node = NumaNode::new(0, 0b1111); // CPUs 0-3
        
        // Check that CPUs 0-3 are associated with this node
        assert!(node.has_cpu(0));
        assert!(node.has_cpu(1));
        assert!(node.has_cpu(2));
        assert!(node.has_cpu(3));
        
        // Check that other CPUs are not associated
        assert!(!node.has_cpu(4));
        assert!(!node.has_cpu(5));
        
        // Test distance functionality
        node.set_distance(1, 20);
        assert_eq!(node.get_distance(1), 20);
        assert_eq!(node.get_distance(2), 10); // Default distance
    }

    #[test]
    fn test_numa_controller() {
        let mut controller = NumaController::new();
        
        // Add a node with CPUs 0-3
        let mut node0 = NumaNode::new(0, 0b1111);
        unsafe {
            node0.add_memory_zone(MemoryZoneType::Normal, 0x10000000, 0x20000000, 4096);
        }
        controller.add_node(node0);
        
        // Add a node with CPUs 4-7
        let mut node1 = NumaNode::new(1, 0b11110000);
        unsafe {
            node1.add_memory_zone(MemoryZoneType::Normal, 0x20000000, 0x30000000, 4096);
        }
        controller.add_node(node1);
        
        // Test current CPU setting
        controller.set_current_cpu(2);
        
        // Get local node for CPU 2 (should be node 0)
        let local_node = controller.get_local_node_id();
        assert_eq!(local_node, Some(0));
        
        // Get local node for CPU 5 (should be node 1)
        controller.set_current_cpu(5);
        let local_node = controller.get_local_node_id();
        assert_eq!(local_node, Some(1));
        
        // Test policy selection
        controller.set_current_cpu(2);
        let selected = controller.select_node(NumaPolicy::LocalNode, 4096);
        assert_eq!(selected, Some(0));
        
        let selected = controller.select_node(NumaPolicy::SpecificNode(1), 4096);
        assert_eq!(selected, Some(1));
        
        // Test interleave policy
        let selected1 = controller.select_node(NumaPolicy::Interleave, 4096);
        let selected2 = controller.select_node(NumaPolicy::Interleave, 4096);
        assert_ne!(selected1, selected2); // Should be different nodes
    }
    
    #[test]
    fn test_numa_allocation() {
        // Initialize NUMA
        let _ = init_numa();
        
        // Test allocation with different policies
        unsafe {
            let ptr1 = numa_alloc(4096, NumaPolicy::LocalNode);
            assert!(!ptr1.is_null());
            
            let ptr2 = numa_alloc_aligned(8192, 8192, NumaPolicy::AnyNode);
            assert!(!ptr2.is_null());
            assert_eq!((ptr2 as usize) % 8192, 0);
            
            let ptr3 = numa_alloc_zeroed(4096, NumaPolicy::LocalNode);
            assert!(!ptr3.is_null());
            
            // Verify zero-initialized memory
            let slice = core::slice::from_raw_parts(ptr3, 4096);
            for &byte in slice {
                assert_eq!(byte, 0);
            }
            
            // Test deallocation
            numa_dealloc(ptr1, 4096);
            numa_dealloc(ptr2, 8192);
            numa_dealloc(ptr3, 4096);
        }
        
        // Cleanup
        let _ = shutdown_numa();
    }
    
    #[test]
    fn test_numa_stats() {
        // Initialize NUMA
        let _ = init_numa();
        
        // Get statistics
        let stats = numa_get_stats();
        assert_eq!(stats.num_nodes, 1);
        assert!(!stats.memory_per_node.is_empty());
        assert!(!stats.allocation_stats_per_node.is_empty());
        
        // Test allocation and stats update
        unsafe {
            let ptr = numa_alloc(4096, NumaPolicy::LocalNode);
            assert!(!ptr.is_null());
            
            let stats_after = numa_get_stats();
            let node_stats = &stats_after.allocation_stats_per_node[0];
            assert!(node_stats.total_allocations.load(core::sync::atomic::Ordering::Relaxed) > 0);
            
            numa_dealloc(ptr, 4096);
        }
        
        // Cleanup
        let _ = shutdown_numa();
    }
}