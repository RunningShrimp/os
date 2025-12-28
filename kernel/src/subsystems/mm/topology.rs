//! NUMA Topology Module
//! 
//! This module provides NUMA topology detection and management functionality,
//! including node discovery, distance matrix calculation, and CPU mapping.

use crate::error::unified::UnifiedError;
use crate::subsystems::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// NUMA node ID type
pub type NodeId = usize;

/// CPU mask for NUMA nodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CPUMask {
    /// Bitmask of CPUs belonging to this node
    mask: Vec<u64>,
    /// Number of CPUs in the mask
    cpu_count: usize,
}

impl CPUMask {
    /// Create a new empty CPU mask
    pub fn new() -> Self {
        Self {
            mask: Vec::new(),
            cpu_count: 0,
        }
    }
    
    /// Create a CPU mask with the specified number of CPUs
    pub fn with_capacity(cpu_count: usize) -> Self {
        let mask_size = (cpu_count + 63) / 64;
        Self {
            mask: vec![0; mask_size],
            cpu_count,
        }
    }
    
    /// Set a CPU in the mask
    pub fn set_cpu(&mut self, cpu_id: usize) {
        if cpu_id >= self.cpu_count {
            return;
        }
        
        let word_index = cpu_id / 64;
        let bit_index = cpu_id % 64;
        
        if word_index >= self.mask.len() {
            return;
        }
        
        self.mask[word_index] |= 1 << bit_index;
    }
    
    /// Clear a CPU in the mask
    pub fn clear_cpu(&mut self, cpu_id: usize) {
        if cpu_id >= self.cpu_count {
            return;
        }
        
        let word_index = cpu_id / 64;
        let bit_index = cpu_id % 64;
        
        if word_index >= self.mask.len() {
            return;
        }
        
        self.mask[word_index] &= !(1 << bit_index);
    }
    
    /// Check if a CPU is set in the mask
    pub fn has_cpu(&self, cpu_id: usize) -> bool {
        if cpu_id >= self.cpu_count {
            return false;
        }
        
        let word_index = cpu_id / 64;
        let bit_index = cpu_id % 64;
        
        if word_index >= self.mask.len() {
            return false;
        }
        
        (self.mask[word_index] & (1 << bit_index)) != 0
    }
    
    /// Get the number of CPUs set in the mask
    pub fn count(&self) -> usize {
        self.mask.iter().map(|word| word.count_ones() as usize).sum()
    }
    
    /// Get the first CPU set in the mask
    pub fn first_cpu(&self) -> Option<usize> {
        for (word_index, &word) in self.mask.iter().enumerate() {
            if word != 0 {
                let bit_index = word.trailing_zeros() as usize;
                return Some(word_index * 64 + bit_index);
            }
        }
        None
    }
    
    /// Get an iterator over all CPUs set in the mask
    pub fn iter(&self) -> CPUMaskIterator {
        CPUMaskIterator {
            mask: self,
            current_cpu: 0,
        }
    }
    
    /// Get the raw mask data
    pub fn as_slice(&self) -> &[u64] {
        &self.mask
    }
    
    /// Get the total CPU count
    pub fn cpu_count(&self) -> usize {
        self.cpu_count
    }
}

/// Iterator over CPUs in a CPU mask
pub struct CPUMaskIterator<'a> {
    mask: &'a CPUMask,
    current_cpu: usize,
}

impl<'a> Iterator for CPUMaskIterator<'a> {
    type Item = usize;
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.current_cpu < self.mask.cpu_count {
            if self.mask.has_cpu(self.current_cpu) {
                let cpu = self.current_cpu;
                self.current_cpu += 1;
                return Some(cpu);
            }
            self.current_cpu += 1;
        }
        None
    }
}

/// Distance matrix between NUMA nodes
#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    /// Distance values between nodes
    distances: Vec<Vec<u32>>,
    /// Number of nodes
    node_count: usize,
}

impl DistanceMatrix {
    /// Create a new distance matrix with the specified number of nodes
    pub fn new(node_count: usize) -> Self {
        let mut distances = vec![vec![0; node_count]; node_count];
        
        // Initialize with default distances
        for i in 0..node_count {
            for j in 0..node_count {
                if i == j {
                    distances[i][j] = 10; // Local distance
                } else {
                    distances[i][j] = 20; // Default remote distance
                }
            }
        }
        
        Self {
            distances,
            node_count,
        }
    }
    
    /// Set the distance between two nodes
    pub fn set_distance(&mut self, from: NodeId, to: NodeId, distance: u32) -> Result<(), UnifiedError> {
        if from >= self.node_count || to >= self.node_count {
            return Err(UnifiedError::system("Invalid node ID for distance matrix", None));
        }
        
        self.distances[from][to] = distance;
        Ok(())
    }
    
    /// Get the distance between two nodes
    pub fn get_distance(&self, from: NodeId, to: NodeId) -> Result<u32, UnifiedError> {
        if from >= self.node_count || to >= self.node_count {
            return Err(UnifiedError::system("Invalid node ID for distance matrix", None));
        }
        
        Ok(self.distances[from][to])
    }
    
    /// Get all distances from a specific node
    pub fn get_distances_from_node(&self, node: NodeId) -> Result<BTreeMap<NodeId, u32>, UnifiedError> {
        if node >= self.node_count {
            return Err(UnifiedError::system("Invalid node ID for distance matrix", None));
        }
        
        let mut distances = BTreeMap::new();
        for (i, &distance) in self.distances[node].iter().enumerate() {
            distances.insert(i, distance);
        }
        
        Ok(distances)
    }
    
    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.node_count
    }
    
    /// Find the nearest node to the specified node
    pub fn find_nearest_node(&self, node: NodeId, exclude_nodes: &[NodeId]) -> Result<Option<NodeId>, UnifiedError> {
        if node >= self.node_count {
            return Err(UnifiedError::system("Invalid node ID for distance matrix", None));
        }
        
        let mut nearest_node = None;
        let mut min_distance = u32::MAX;
        
        for (i, &distance) in self.distances[node].iter().enumerate() {
            if i == node || exclude_nodes.contains(&i) {
                continue;
            }
            
            if distance < min_distance {
                min_distance = distance;
                nearest_node = Some(i);
            }
        }
        
        Ok(nearest_node)
    }
}

/// NUMA node information
#[derive(Debug, Clone)]
pub struct NUMANode {
    /// Node ID
    pub id: NodeId,
    /// CPU mask for this node
    pub cpu_mask: CPUMask,
    /// Memory size in bytes
    pub memory_size: usize,
    /// Node type
    pub node_type: NodeType,
    /// Physical address range
    pub address_range: (usize, usize),
    /// Node attributes
    pub attributes: BTreeMap<String, String>,
}

/// NUMA node types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// Regular memory node
    Memory,
    /// CPU-only node
    CPU,
    /// GPU node
    GPU,
    /// Accelerator node
    Accelerator,
    /// Mixed node
    Mixed,
}

/// NUMA topology
#[derive(Debug)]
pub struct NUMATopology {
    /// NUMA nodes
    nodes: Vec<NUMANode>,
    /// Distance matrix between nodes
    distance_matrix: DistanceMatrix,
    /// Current node (for the current execution context)
    current_node: Mutex<Option<NodeId>>,
    /// Total CPU count
    total_cpu_count: usize,
    /// Total memory size
    total_memory_size: usize,
}

impl NUMATopology {
    /// Create a new NUMA topology
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            distance_matrix: DistanceMatrix::new(0),
            current_node: Mutex::new(None),
            total_cpu_count: 0,
            total_memory_size: 0,
        }
    }
    
    /// Add a NUMA node to the topology
    pub fn add_node(&mut self, node: NUMANode) -> Result<(), UnifiedError> {
        // Check if node ID already exists
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(UnifiedError::system("Node ID already exists", None));
        }
        
        // Update total CPU count and memory size
        self.total_cpu_count += node.cpu_mask.count();
        self.total_memory_size += node.memory_size;
        
        // Add the node
        self.nodes.push(node);
        
        // Resize distance matrix if needed
        if self.nodes.len() > self.distance_matrix.node_count() {
            let new_size = self.nodes.len();
            let mut new_matrix = DistanceMatrix::new(new_size);
            
            // Copy existing distances
            for i in 0..self.distance_matrix.node_count() {
                for j in 0..self.distance_matrix.node_count() {
                    if let Ok(distance) = self.distance_matrix.get_distance(i, j) {
                        let _ = new_matrix.set_distance(i, j, distance);
                    }
                }
            }
            
            self.distance_matrix = new_matrix;
        }
        
        Ok(())
    }
    
    /// Get a NUMA node by ID
    pub fn get_node(&self, node_id: NodeId) -> Result<&NUMANode, UnifiedError> {
        self.nodes.get(node_id).ok_or_else(|| {
            UnifiedError::system("Node ID not found", None)
        })
    }
    
    /// Get all NUMA nodes
    pub fn get_nodes(&self) -> &[NUMANode] {
        &self.nodes
    }
    
    /// Get the number of NUMA nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Get the total CPU count
    pub fn total_cpu_count(&self) -> usize {
        self.total_cpu_count
    }
    
    /// Get the total memory size
    pub fn total_memory_size(&self) -> usize {
        self.total_memory_size
    }
    
    /// Get the distance matrix
    pub fn get_distance_matrix(&self) -> &DistanceMatrix {
        &self.distance_matrix
    }
    
    /// Set the distance between two nodes
    pub fn set_distance(&mut self, from: NodeId, to: NodeId, distance: u32) -> Result<(), UnifiedError> {
        self.distance_matrix.set_distance(from, to, distance)
    }
    
    /// Get the distance between two nodes
    pub fn get_distance(&self, from: NodeId, to: NodeId) -> Result<u32, UnifiedError> {
        self.distance_matrix.get_distance(from, to)
    }
    
    /// Get all distances from a specific node
    pub fn get_distances_from_node(&self, node: NodeId) -> Result<BTreeMap<NodeId, u32>, UnifiedError> {
        self.distance_matrix.get_distances_from_node(node)
    }
    
    /// Find the nearest node to the specified node
    pub fn find_nearest_node(&self, node: NodeId, exclude_nodes: &[NodeId]) -> Result<Option<NodeId>, UnifiedError> {
        self.distance_matrix.find_nearest_node(node, exclude_nodes)
    }
    
    /// Set the current node
    pub fn set_current_node(&self, node_id: NodeId) -> Result<(), UnifiedError> {
        if node_id >= self.nodes.len() {
            return Err(UnifiedError::system("Invalid node ID", None));
        }
        
        let mut current = self.current_node.lock();
        *current = Some(node_id);
        Ok(())
    }
    
    /// Get the current node
    pub fn get_current_node(&self) -> Result<NodeId, UnifiedError> {
        let current = self.current_node.lock();
        current.ok_or_else(|| UnifiedError::system("Current node not set", None))
    }
    
    /// Find the node containing the specified CPU
    pub fn find_node_for_cpu(&self, cpu_id: usize) -> Result<Option<NodeId>, UnifiedError> {
        for (node_id, node) in self.nodes.iter().enumerate() {
            if node.cpu_mask.has_cpu(cpu_id) {
                return Ok(Some(node_id));
            }
        }
        Ok(None)
    }
    
    /// Find the node containing the specified memory address
    pub fn find_node_for_address(&self, address: usize) -> Result<Option<NodeId>, UnifiedError> {
        for (node_id, node) in self.nodes.iter().enumerate() {
            let (start, end) = node.address_range;
            if address >= start && address < end {
                return Ok(Some(node_id));
            }
        }
        Ok(None)
    }
    
    /// Get nodes sorted by distance from the specified node
    pub fn get_nodes_by_distance(&self, from_node: NodeId) -> Result<Vec<(NodeId, u32)>, UnifiedError> {
        if from_node >= self.nodes.len() {
            return Err(UnifiedError::system("Invalid node ID", None));
        }
        
        let mut nodes_with_distance = Vec::new();
        
        for (node_id, _) in self.nodes.iter().enumerate() {
            if let Ok(distance) = self.distance_matrix.get_distance(from_node, node_id) {
                nodes_with_distance.push((node_id, distance));
            }
        }
        
        // Sort by distance
        nodes_with_distance.sort_by(|a, b| a.1.cmp(&b.1));
        
        Ok(nodes_with_distance)
    }
}

/// Detect NUMA topology
pub fn detect_topology() -> Result<NUMATopology, UnifiedError> {
    // In a real implementation, this would query the hardware
    // For now, we'll create a simple 2-node topology
    
    let mut topology = NUMATopology::new();
    
    // Create node 0
    let node0_cpu_mask = {
        let mut mask = CPUMask::with_capacity(8);
        mask.set_cpu(0);
        mask.set_cpu(1);
        mask.set_cpu(2);
        mask.set_cpu(3);
        mask
    };
    
    let node0 = NUMANode {
        id: 0,
        cpu_mask: node0_cpu_mask,
        memory_size: 8 * 1024 * 1024 * 1024, // 8GB
        node_type: NodeType::Memory,
        address_range: (0, 8 * 1024 * 1024 * 1024),
        attributes: {
            let mut attrs = BTreeMap::new();
            attrs.insert("description".to_string(), "Node 0 - Primary".to_string());
            attrs
        },
    };
    
    // Create node 1
    let node1_cpu_mask = {
        let mut mask = CPUMask::with_capacity(8);
        mask.set_cpu(4);
        mask.set_cpu(5);
        mask.set_cpu(6);
        mask.set_cpu(7);
        mask
    };
    
    let node1 = NUMANode {
        id: 1,
        cpu_mask: node1_cpu_mask,
        memory_size: 8 * 1024 * 1024 * 1024, // 8GB
        node_type: NodeType::Memory,
        address_range: (8 * 1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024),
        attributes: {
            let mut attrs = BTreeMap::new();
            attrs.insert("description".to_string(), "Node 1 - Secondary".to_string());
            attrs
        },
    };
    
    // Add nodes to topology
    topology.add_node(node0)?;
    topology.add_node(node1)?;
    
    // Set distances
    topology.set_distance(0, 0, 10)?; // Local
    topology.set_distance(1, 1, 10)?; // Local
    topology.set_distance(0, 1, 20)?; // Remote
    topology.set_distance(1, 0, 20)?; // Remote
    
    // Set current node to 0
    topology.set_current_node(0)?;
    
    Ok(topology)
}

/// Get the current CPU ID
pub fn get_current_cpu() -> usize {
    // In a real implementation, this would use CPUID or similar
    // For now, we'll use a simple counter
    static CURRENT_CPU: AtomicU64 = AtomicU64::new(0);
    (CURRENT_CPU.fetch_add(1, Ordering::SeqCst) % 8) as usize
}

/// Get the current NUMA node for the current CPU
pub fn get_current_node_for_cpu(topology: &NUMATopology) -> Result<NodeId, UnifiedError> {
    let cpu_id = get_current_cpu();
    topology.find_node_for_cpu(cpu_id)?
        .ok_or_else(|| UnifiedError::system("CPU not assigned to any NUMA node", None))
}