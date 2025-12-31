//! Load Balancing Strategies
//!
//! This module implements various load balancing algorithms for distributing requests
//! across cluster nodes, including weighted round-robin, least connections, and
//! consistent hashing with adaptive load balancing.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::sync::{Mutex, RwLock};

/// Unique identifier for a node
pub type NodeId = u64;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Simple round-robin
    RoundRobin,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Least connections first
    LeastConnections,
    /// Random selection
    Random,
    /// Consistent hashing
    ConsistentHash,
    /// Adaptive based on load
    Adaptive,
    /// Locality-aware
    LocalityAware,
}

/// Node load information
#[derive(Debug, Clone)]
pub struct NodeLoad {
    /// Node ID
    pub node_id: NodeId,
    /// Current active connections
    pub active_connections: usize,
    /// CPU utilization (0-100)
    pub cpu_utilization: u8,
    /// Memory utilization (0-100)
    pub memory_utilization: u8,
    /// Request rate (requests per second)
    pub request_rate: u64,
    /// Average response time (microseconds)
    pub avg_response_time_us: u64,
    /// Node weight (for weighted algorithms)
    pub weight: u32,
    /// Node health score (0-100)
    pub health_score: u8,
    /// Last update time
    pub last_update_ms: u64,
}

/// Request metadata for routing decisions
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    /// Request ID
    pub request_id: u64,
    /// Client ID (for affinity)
    pub client_id: Option<u64>,
    /// Session ID (for session affinity)
    pub session_id: Option<u64>,
    /// Request size in bytes
    pub size_bytes: usize,
    /// Expected processing time
    pub estimated_duration_us: Option<u64>,
    /// Request priority (0-10, higher is more important)
    pub priority: u8,
}

/// Backpressure information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureState {
    /// No backpressure
    None,
    /// Light backpressure (slow down)
    Light,
    /// Moderate backpressure
    Moderate,
    /// Heavy backpressure (reject requests)
    Heavy,
}

/// Consistent hash ring entry
#[derive(Debug, Clone)]
struct HashRingEntry {
    /// Virtual node ID
    virtual_id: u64,
    /// Physical node ID
    node_id: NodeId,
}

/// Statistics for load balancer
#[derive(Debug, Default)]
pub struct LoadBalanceStatistics {
    /// Total requests routed
    pub total_requests: AtomicU64,
    /// Requests per node
    pub requests_per_node: Mutex<BTreeMap<NodeId, AtomicU64>>,
    /// Rejected requests (due to backpressure)
    pub rejected_requests: AtomicU64,
    /// Average response time
    pub avg_response_time_us: AtomicU64,
    /// Current backpressure rejections
    pub backpressure_rejections: AtomicU64,
    /// Strategy switches
    pub strategy_switches: AtomicU64,
}

/// Errors that can occur during load balancing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BalanceError {
    /// No available nodes
    NoAvailableNodes,
    /// Invalid node configuration
    InvalidConfiguration,
    /// Node not found
    NodeNotFound(NodeId),
    /// All nodes overloaded
    AllNodesOverloaded,
    /// Backpressure threshold exceeded
    BackpressureExceeded,
    /// Invalid request metadata
    InvalidMetadata,
    /// Hash calculation failed
    HashFailed,
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for BalanceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BalanceError::NoAvailableNodes => {
                write!(f, "No available nodes for routing")
            }
            BalanceError::InvalidConfiguration => {
                write!(f, "Invalid load balancer configuration")
            }
            BalanceError::NodeNotFound(node_id) => {
                write!(f, "Node {} not found", node_id)
            }
            BalanceError::AllNodesOverloaded => {
                write!(f, "All nodes are overloaded")
            }
            BalanceError::BackpressureExceeded => {
                write!(f, "Backpressure threshold exceeded")
            }
            BalanceError::InvalidMetadata => {
                write!(f, "Invalid request metadata")
            }
            BalanceError::HashFailed => {
                write!(f, "Failed to calculate hash")
            }
            BalanceError::InternalError(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

/// Load Balancer configuration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Load balancing strategy
    pub strategy: LoadBalanceStrategy,
    /// Number of virtual nodes per physical node (for consistent hashing)
    pub virtual_nodes: usize,
    /// CPU threshold for considering a node overloaded
    pub cpu_threshold: u8,
    /// Memory threshold for considering a node overloaded
    pub memory_threshold: u8,
    /// Maximum connections per node
    pub max_connections: usize,
    /// Enable backpressure
    pub enable_backpressure: bool,
    /// Backpressure threshold (0-100)
    pub backpressure_threshold: u8,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalanceStrategy::Adaptive,
            virtual_nodes: 150,
            cpu_threshold: 80,
            memory_threshold: 80,
            max_connections: 10000,
            enable_backpressure: true,
            backpressure_threshold: 90,
            health_check_interval: Duration::from_secs(5),
        }
    }
}

/// Load Balancer
pub struct LoadBalancer {
    /// Configuration
    config: LoadBalancerConfig,
    /// Registered nodes and their load info
    nodes: RwLock<BTreeMap<NodeId, NodeLoad>>,
    /// Hash ring for consistent hashing
    hash_ring: RwLock<Vec<HashRingEntry>>,
    /// Current index for round-robin
    round_robin_index: Mutex<usize>,
    /// Node weights for weighted round-robin
    weighted_nodes: RwLock<VecDeque<NodeId>>,
    /// Current weight index
    weight_index: Mutex<usize>,
    /// Statistics
    stats: Arc<LoadBalanceStatistics>,
    /// Current backpressure state
    backpressure_state: Mutex<BackpressureState>,
    /// Available nodes (not overloaded)
    available_nodes: RwLock<BTreeSet<NodeId>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(config: LoadBalancerConfig) -> Self {
        Self {
            config,
            nodes: RwLock::new(BTreeMap::new()),
            hash_ring: RwLock::new(Vec::new()),
            round_robin_index: Mutex::new(0),
            weighted_nodes: RwLock::new(VecDeque::new()),
            weight_index: Mutex::new(0),
            stats: Arc::new(LoadBalanceStatistics::default()),
            backpressure_state: Mutex::new(BackpressureState::None),
            available_nodes: RwLock::new(BTreeSet::new()),
        }
    }

    /// Add a node to the load balancer
    pub fn add_node(&self, node_load: NodeLoad) -> Result<(), BalanceError> {
        let node_id = node_load.node_id;

        // Add to nodes
        let mut nodes = self.nodes.write();
        nodes.insert(node_id, node_load.clone());
        drop(nodes);

        // Add to available nodes if not overloaded
        if !self.is_overloaded(&node_load) {
            let mut available = self.available_nodes.write();
            available.insert(node_id);
        }

        // Update hash ring if using consistent hashing
        if self.config.strategy == LoadBalanceStrategy::ConsistentHash
            || self.config.strategy == LoadBalanceStrategy::LocalityAware
        {
            self.update_hash_ring(node_id);
        }

        // Update weighted nodes
        if self.config.strategy == LoadBalanceStrategy::WeightedRoundRobin {
            self.update_weighted_nodes(node_id);
        }

        // Initialize statistics
        let mut per_node = self.stats.requests_per_node.lock();
        per_node.insert(node_id, AtomicU64::new(0));

        Ok(())
    }

    /// Remove a node from the load balancer
    pub fn remove_node(&self, node_id: NodeId) -> Result<(), BalanceError> {
        let mut nodes = self.nodes.write();
        if !nodes.remove(&node_id).is_some() {
            return Err(BalanceError::NodeNotFound(node_id));
        }
        drop(nodes);

        // Remove from available nodes
        let mut available = self.available_nodes.write();
        available.remove(&node_id);

        // Rebuild hash ring
        self.rebuild_hash_ring();

        // Update weighted nodes
        self.rebuild_weighted_nodes();

        Ok(())
    }

    /// Update load information for a node
    pub fn update_node_load(&self, node_load: NodeLoad) -> Result<(), BalanceError> {
        let node_id = node_load.node_id;

        let mut nodes = self.nodes.write();
        if !nodes.contains_key(&node_id) {
            return Err(BalanceError::NodeNotFound(node_id));
        }
        nodes.insert(node_id, node_load.clone());
        drop(nodes);

        // Update availability based on load
        let mut available = self.available_nodes.write();
        if self.is_overloaded(&node_load) {
            available.remove(&node_id);
        } else {
            available.insert(node_id);
        }

        Ok(())
    }

    /// Check if a node is overloaded
    fn is_overloaded(&self, node_load: &NodeLoad) -> bool {
        node_load.cpu_utilization > self.config.cpu_threshold
            || node_load.memory_utilization > self.config.memory_threshold
            || node_load.active_connections >= self.config.max_connections
    }

    /// Select a node for the given request
    pub fn select_node(
        &self,
        metadata: Option<&RequestMetadata>,
    ) -> Result<NodeId, BalanceError> {
        // Check backpressure
        if self.config.enable_backpressure {
            let backpressure = *self.backpressure_state.lock();
            if backpressure == BackpressureState::Heavy {
                self.stats.backpressure_rejections.fetch_add(1, Ordering::Relaxed);
                return Err(BalanceError::BackpressureExceeded);
            }
        }

        let node_id = match self.config.strategy {
            LoadBalanceStrategy::RoundRobin => self.select_round_robin()?,
            LoadBalanceStrategy::WeightedRoundRobin => self.select_weighted_round_robin()?,
            LoadBalanceStrategy::LeastConnections => self.select_least_connections()?,
            LoadBalanceStrategy::Random => self.select_random()?,
            LoadBalanceStrategy::ConsistentHash => {
                self.select_consistent_hash(metadata.ok_or(BalanceError::InvalidMetadata)?)?
            }
            LoadBalanceStrategy::Adaptive => self.select_adaptive()?,
            LoadBalanceStrategy::LocalityAware => {
                self.select_locality_aware(metadata.ok_or(BalanceError::InvalidMetadata)?)?
            }
        };

        // Update statistics
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);
        {
            let per_node = self.stats.requests_per_node.lock();
            if let Some(counter) = per_node.get(&node_id) {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(node_id)
    }

    /// Round-robin selection
    fn select_round_robin(&self) -> Result<NodeId, BalanceError> {
        let available = self.available_nodes.read();
        if available.is_empty() {
            return Err(BalanceError::NoAvailableNodes);
        }

        let nodes: Vec<_> = available.iter().copied().collect();
        let mut index = self.round_robin_index.lock();
        let node_id = nodes[*index % nodes.len()];
        *index = (*index + 1) % nodes.len();

        Ok(node_id)
    }

    /// Weighted round-robin selection
    fn select_weighted_round_robin(&self) -> Result<NodeId, BalanceError> {
        let weighted = self.weighted_nodes.read();
        if weighted.is_empty() {
            return Err(BalanceError::NoAvailableNodes);
        }

        let mut index = self.weight_index.lock();
        let node_id = weighted[*index % weighted.len()].ok_or(BalanceError::NoAvailableNodes)?;
        *index = (*index + 1) % weighted.len();

        Ok(node_id)
    }

    /// Least connections selection
    fn select_least_connections(&self) -> Result<NodeId, BalanceError> {
        let nodes = self.nodes.read();
        let available = self.available_nodes.read();

        if available.is_empty() {
            return Err(BalanceError::NoAvailableNodes);
        }

        let mut min_connections = usize::MAX;
        let mut selected_node = None;

        for &node_id in available {
            if let Some(load) = nodes.get(&node_id) {
                if load.active_connections < min_connections {
                    min_connections = load.active_connections;
                    selected_node = Some(node_id);
                }
            }
        }

        selected_node.ok_or(BalanceError::NoAvailableNodes)
    }

    /// Random selection
    fn select_random(&self) -> Result<NodeId, BalanceError> {
        let available = self.available_nodes.read();
        if available.is_empty() {
            return Err(BalanceError::NoAvailableNodes);
        }

        let nodes: Vec<_> = available.iter().copied().collect();
        // Simple pseudo-random selection based on total requests
        let index = (self.stats.total_requests.fetch_add(1, Ordering::Relaxed) as usize) % nodes.len();
        Ok(nodes[index])
    }

    /// Consistent hash selection
    fn select_consistent_hash(&self, metadata: &RequestMetadata) -> Result<NodeId, BalanceError> {
        let available = self.available_nodes.read();
        if available.is_empty() {
            return Err(BalanceError::NoAvailableNodes);
        }

        let hash = self.calculate_hash(metadata);
        let ring = self.hash_ring.read();

        // Find the first node with hash >= our hash
        for entry in ring.iter() {
            if entry.virtual_id >= hash && available.contains(&entry.node_id) {
                return Ok(entry.node_id);
            }
        }

        // Wrap around to the first node
        if let Some(entry) = ring.first() {
            if available.contains(&entry.node_id) {
                return Ok(entry.node_id);
            }
        }

        Err(BalanceError::NoAvailableNodes)
    }

    /// Adaptive selection (based on multiple factors)
    fn select_adaptive(&self) -> Result<NodeId, BalanceError> {
        let nodes = self.nodes.read();
        let available = self.available_nodes.read();

        if available.is_empty() {
            return Err(BalanceError::NoAvailableNodes);
        }

        let mut best_node = None;
        let mut best_score = i64::MIN;

        for &node_id in available {
            if let Some(load) = nodes.get(&node_id) {
                // Calculate score: higher is better
                let cpu_score = (100 - load.cpu_utilization as i64) * 2;
                let mem_score = (100 - load.memory_utilization as i64);
                let conn_score = -(load.active_connections as i64);
                let health_score = load.health_score as i64;
                let weight_score = load.weight as i64;

                let total_score = cpu_score + mem_score + conn_score + health_score + weight_score;

                if total_score > best_score {
                    best_score = total_score;
                    best_node = Some(node_id);
                }
            }
        }

        best_node.ok_or(BalanceError::NoAvailableNodes)
    }

    /// Locality-aware selection
    fn select_locality_aware(&self, metadata: &RequestMetadata) -> Result<NodeId, BalanceError> {
        // Try consistent hash first for session affinity
        if let Some(session_id) = metadata.session_id {
            let mut meta = metadata.clone();
            meta.session_id = Some(session_id);
            return self.select_consistent_hash(&meta);
        }

        // Fall back to adaptive if no session affinity
        self.select_adaptive()
    }

    /// Calculate hash for consistent hashing
    fn calculate_hash(&self, metadata: &RequestMetadata) -> u64 {
        // Simple hash function (in production, use a proper hash like xxHash)
        let mut hash = metadata.request_id;
        if let Some(client_id) = metadata.client_id {
            hash ^= client_id;
        }
        if let Some(session_id) = metadata.session_id {
            hash ^= session_id;
        }
        hash
    }

    /// Update hash ring for a node
    fn update_hash_ring(&self, node_id: NodeId) {
        let mut ring = self.hash_ring.write();
        let weight = self
            .nodes
            .read()
            .get(&node_id)
            .map(|n| n.weight)
            .unwrap_or(1);

        let virtual_count = (self.config.virtual_nodes as u32 * weight) as usize;

        for i in 0..virtual_count {
            let virtual_id = self.calculate_virtual_node_id(node_id, i);
            ring.push(HashRingEntry {
                virtual_id,
                node_id,
            });
        }

        // Sort by virtual ID
        ring.sort_by(|a, b| a.virtual_id.cmp(&b.virtual_id));
    }

    /// Calculate virtual node ID
    fn calculate_virtual_node_id(&self, node_id: NodeId, index: usize) -> u64 {
        // Simple hash combining node_id and index
        node_id.wrapping_mul(31).wrapping_add(index as u64)
    }

    /// Rebuild hash ring from scratch
    fn rebuild_hash_ring(&self) {
        let mut ring = self.hash_ring.write();
        ring.clear();

        let nodes = self.nodes.read();
        for (&node_id, load) in nodes.iter() {
            let virtual_count = (self.config.virtual_nodes as u32 * load.weight) as usize;

            for i in 0..virtual_count {
                let virtual_id = self.calculate_virtual_node_id(node_id, i);
                ring.push(HashRingEntry {
                    virtual_id,
                    node_id,
                });
            }
        }

        ring.sort_by(|a, b| a.virtual_id.cmp(&b.virtual_id));
    }

    /// Update weighted nodes deque
    fn update_weighted_nodes(&self, node_id: NodeId) {
        let weight = self
            .nodes
            .read()
            .get(&node_id)
            .map(|n| n.weight)
            .unwrap_or(1);

        let mut weighted = self.weighted_nodes.write();
        for _ in 0..weight {
            weighted.push_back(node_id);
        }
    }

    /// Rebuild weighted nodes from scratch
    fn rebuild_weighted_nodes(&self) {
        let mut weighted = self.weighted_nodes.write();
        weighted.clear();

        let nodes = self.nodes.read();
        for (&node_id, load) in nodes.iter() {
            for _ in 0..load.weight {
                weighted.push_back(node_id);
            }
        }
    }

    /// Update backpressure state
    pub fn update_backpressure(&self, state: BackpressureState) {
        *self.backpressure_state.lock() = state;
    }

    /// Get current backpressure state
    pub fn get_backpressure_state(&self) -> BackpressureState {
        *self.backpressure_state.lock()
    }

    /// Get load information for all nodes
    pub fn get_all_node_loads(&self) -> Vec<NodeLoad> {
        let nodes = self.nodes.read();
        nodes.values().cloned().collect()
    }

    /// Get load information for a specific node
    pub fn get_node_load(&self, node_id: NodeId) -> Option<NodeLoad> {
        let nodes = self.nodes.read();
        nodes.get(&node_id).cloned()
    }

    /// Get available nodes (not overloaded)
    pub fn get_available_nodes(&self) -> Vec<NodeId> {
        let available = self.available_nodes.read();
        available.iter().copied().collect()
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &LoadBalanceStatistics {
        &self.stats
    }

    /// Change load balancing strategy
    pub fn set_strategy(&self, strategy: LoadBalanceStrategy) -> Result<(), BalanceError> {
        self.config.strategy = strategy;
        self.stats.strategy_switches.fetch_add(1, Ordering::Relaxed);

        // Rebuild data structures if needed
        match strategy {
            LoadBalanceStrategy::ConsistentHash | LoadBalanceStrategy::LocalityAware => {
                self.rebuild_hash_ring();
            }
            LoadBalanceStrategy::WeightedRoundRobin => {
                self.rebuild_weighted_nodes();
            }
            _ => {}
        }

        Ok(())
    }

    /// Get current strategy
    pub fn get_strategy(&self) -> LoadBalanceStrategy {
        self.config.strategy
    }

    /// Increment connection count for a node
    pub fn increment_connections(&self, node_id: NodeId) -> Result<(), BalanceError> {
        let mut nodes = self.nodes.write();
        if let Some(load) = nodes.get_mut(&node_id) {
            load.active_connections += 1;
            return Ok(());
        }
        Err(BalanceError::NodeNotFound(node_id))
    }

    /// Decrement connection count for a node
    pub fn decrement_connections(&self, node_id: NodeId) -> Result<(), BalanceError> {
        let mut nodes = self.nodes.write();
        if let Some(load) = nodes.get_mut(&node_id) {
            load.active_connections = load.active_connections.saturating_sub(1);
            return Ok(());
        }
        Err(BalanceError::NodeNotFound(node_id))
    }

    /// Record response time for a node
    pub fn record_response_time(&self, node_id: NodeId, duration_us: u64) -> Result<(), BalanceError> {
        let mut nodes = self.nodes.write();
        if let Some(load) = nodes.get_mut(&node_id) {
            // Exponential moving average
            let alpha = 0.2;
            let current_avg = load.avg_response_time_us;
            load.avg_response_time_us = ((alpha * duration_us as f64)
                + ((1.0 - alpha) * current_avg as f64)) as u64;
            return Ok(());
        }
        Err(BalanceError::NodeNotFound(node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin_selection() {
        let config = LoadBalancerConfig {
            strategy: LoadBalanceStrategy::RoundRobin,
            ..Default::default()
        };
        let lb = LoadBalancer::new(config);

        let node1 = NodeLoad {
            node_id: 1,
            active_connections: 0,
            cpu_utilization: 50,
            memory_utilization: 50,
            request_rate: 100,
            avg_response_time_us: 1000,
            weight: 1,
            health_score: 100,
            last_update_ms: 0,
        };

        let node2 = NodeLoad {
            node_id: 2,
            active_connections: 0,
            cpu_utilization: 50,
            memory_utilization: 50,
            request_rate: 100,
            avg_response_time_us: 1000,
            weight: 1,
            health_score: 100,
            last_update_ms: 0,
        };

        lb.add_node(node1).unwrap();
        lb.add_node(node2).unwrap();

        let selected1 = lb.select_node(None).unwrap();
        let selected2 = lb.select_node(None).unwrap();

        assert_ne!(selected1, selected2);
    }

    #[test]
    fn test_least_connections() {
        let config = LoadBalancerConfig {
            strategy: LoadBalanceStrategy::LeastConnections,
            ..Default::default()
        };
        let lb = LoadBalancer::new(config);

        let node1 = NodeLoad {
            node_id: 1,
            active_connections: 10,
            cpu_utilization: 50,
            memory_utilization: 50,
            request_rate: 100,
            avg_response_time_us: 1000,
            weight: 1,
            health_score: 100,
            last_update_ms: 0,
        };

        let node2 = NodeLoad {
            node_id: 2,
            active_connections: 5,
            cpu_utilization: 50,
            memory_utilization: 50,
            request_rate: 100,
            avg_response_time_us: 1000,
            weight: 1,
            health_score: 100,
            last_update_ms: 0,
        };

        lb.add_node(node1).unwrap();
        lb.add_node(node2).unwrap();

        let selected = lb.select_node(None).unwrap();
        assert_eq!(selected, 2); // Should select node with fewer connections
    }
}
