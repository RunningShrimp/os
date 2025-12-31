//! Cluster Membership Protocol
//!
//! This module implements cluster membership management with gossip-based failure detection,
//! node join/leave protocols, membership view dissemination, and partition handling.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::sync::{Mutex, RwLock};

/// Unique identifier for a node
pub type NodeId = u64;

/// Cluster generation number (changes on membership change)
pub type Generation = u64;

/// Gossip digest
#[derive(Debug, Clone)]
pub struct GossipDigest {
    /// Node ID
    pub node_id: NodeId,
    /// Generation number
    pub generation: Generation,
    /// Heartbeat version
    pub heartbeat_version: u64,
    /// Status
    pub status: NodeStatus,
}

/// Node status in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node is alive
    Alive,
    /// Node is suspected to be down
    Suspect,
    /// Node is confirmed down
    Dead,
    /// Node is leaving
    Leaving,
    /// Node has been removed
    Removed,
}

/// Node information
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node ID
    pub node_id: NodeId,
    /// Node address (e.g., IP:port)
    pub address: String,
    /// Current status
    pub status: NodeStatus,
    /// Generation number when this node joined
    pub generation: Generation,
    /// Heartbeat version
    pub heartbeat_version: u64,
    /// Last heartbeat time
    pub last_heartbeat_ms: u64,
    /// Node capabilities/roles
    pub roles: Vec<String>,
    /// Node metadata
    pub metadata: BTreeMap<String, String>,
    /// Last update time
    pub last_update_ms: u64,
}

/// Cluster configuration
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Seed nodes for bootstrap
    pub seed_nodes: Vec<String>,
    /// Gossip interval
    pub gossip_interval: Duration,
    /// Failure detection timeout
    pub failure_timeout: Duration,
    /// Suspicion timeout before marking as dead
    pub suspicion_timeout: Duration,
    /// Number of gossip peers per round
    pub gossip_peers: usize,
    /// Enable automatic rebalancing
    pub enable_rebalancing: bool,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            seed_nodes: Vec::new(),
            gossip_interval: Duration::from_millis(100),
            failure_timeout: Duration::from_secs(5),
            suspicion_timeout: Duration::from_secs(15),
            gossip_peers: 3,
            enable_rebalancing: true,
        }
    }
}

/// Membership view
#[derive(Debug, Clone)]
pub struct MembershipView {
    /// View generation
    pub generation: Generation,
    /// Member nodes
    pub members: BTreeMap<NodeId, NodeInfo>,
    /// View creation time
    pub created_ms: u64,
}

/// Partition information
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    /// Nodes in this partition
    pub nodes: BTreeSet<NodeId>,
    /// Partition ID
    pub partition_id: u64,
    /// Whether this partition can form quorum
    pub has_quorum: bool,
    /// Leader (if any)
    pub leader: Option<NodeId>,
}

/// Rebalancing trigger
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebalanceTrigger {
    /// No rebalancing needed
    None,
    /// Node joined
    NodeJoined(NodeId),
    /// Node left
    NodeLeft(NodeId),
    /// Partition healed
    PartitionHealed,
    /// Load imbalance detected
    LoadImbalance,
    /// Manual trigger
    Manual,
}

/// Statistics for membership module
#[derive(Debug, Default)]
pub struct MembershipStatistics {
    /// Total joins processed
    pub joins_processed: AtomicU64,
    /// Total leaves processed
    pub leaves_processed: AtomicU64,
    /// Gossip messages sent
    pub gossip_sent: AtomicU64,
    /// Gossip messages received
    pub gossip_received: AtomicU64,
    /// Membership changes
    pub membership_changes: AtomicU64,
    /// Partitions detected
    pub partitions_detected: AtomicU64,
    /// Rebalancing operations
    pub rebalancing_operations: AtomicU64,
    /// Current cluster size
    pub cluster_size: AtomicUsize,
}

/// Errors that can occur during membership operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipError {
    /// Node not found
    NodeNotFound(NodeId),
    /// Node already exists
    NodeAlreadyExists(NodeId),
    /// Invalid cluster state
    InvalidState,
    /// Cannot form quorum
    NoQuorum,
    /// Network partition detected
    NetworkPartition,
    /// Bootstrap failed
    BootstrapFailed(String),
    /// Join rejected
    JoinRejected(String),
    /// Leave failed
    LeaveFailed(String),
    /// Gossip failure
    GossipFailure(String),
    /// Timeout
    Timeout,
    /// Permission denied
    PermissionDenied,
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for MembershipError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MembershipError::NodeNotFound(node_id) => {
                write!(f, "Node {} not found in cluster", node_id)
            }
            MembershipError::NodeAlreadyExists(node_id) => {
                write!(f, "Node {} already exists in cluster", node_id)
            }
            MembershipError::InvalidState => {
                write!(f, "Invalid cluster state")
            }
            MembershipError::NoQuorum => {
                write!(f, "Cannot form quorum")
            }
            MembershipError::NetworkPartition => {
                write!(f, "Network partition detected")
            }
            MembershipError::BootstrapFailed(msg) => {
                write!(f, "Bootstrap failed: {}", msg)
            }
            MembershipError::JoinRejected(msg) => {
                write!(f, "Join rejected: {}", msg)
            }
            MembershipError::LeaveFailed(msg) => {
                write!(f, "Leave failed: {}", msg)
            }
            MembershipError::GossipFailure(msg) => {
                write!(f, "Gossip failure: {}", msg)
            }
            MembershipError::Timeout => {
                write!(f, "Operation timeout")
            }
            MembershipError::PermissionDenied => {
                write!(f, "Permission denied")
            }
            MembershipError::InternalError(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

/// Cluster Membership Manager
pub struct ClusterMembership {
    /// Local node ID
    local_node: NodeId,
    /// Local node info
    local_info: Mutex<NodeInfo>,
    /// Cluster configuration
    config: ClusterConfig,
    /// Current membership view
    membership: RwLock<MembershipView>,
    /// Pending joins (nodes being added)
    pending_joins: Mutex<BTreeSet<NodeId>>,
    /// Pending leaves (nodes being removed)
    pending_leaves: Mutex<BTreeSet<NodeId>>,
    /// Suspected nodes
    suspected: Mutex<BTreeMap<NodeId, u64>>,
    /// Current generation
    generation: AtomicU64,
    /// Current time (milliseconds)
    current_time_ms: AtomicU64,
    /// Statistics
    stats: Arc<MembershipStatistics>,
    /// Last gossip time
    last_gossip: Mutex<Option<u64>>,
    /// Cluster state
    cluster_state: Mutex<ClusterState>,
}

/// Cluster state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClusterState {
    /// Not initialized
    Uninitialized,
    /// Bootstrapping
    Bootstrapping,
    /// Normal operation
    Normal,
    /// Partitioned
    Partitioned,
    /// Recovering
    Recovering,
}

impl ClusterMembership {
    /// Create a new cluster membership instance
    pub fn new(local_node: NodeId, local_address: String, config: ClusterConfig) -> Self {
        let local_info = NodeInfo {
            node_id: local_node,
            address: local_address,
            status: NodeStatus::Alive,
            generation: 0,
            heartbeat_version: 0,
            last_heartbeat_ms: 0,
            roles: Vec::new(),
            metadata: BTreeMap::new(),
            last_update_ms: 0,
        };

        Self {
            local_node,
            local_info: Mutex::new(local_info.clone()),
            config,
            membership: RwLock::new(MembershipView {
                generation: 0,
                members: {
                    let mut members = BTreeMap::new();
                    members.insert(local_node, local_info);
                    members
                },
                created_ms: 0,
            }),
            pending_joins: Mutex::new(BTreeSet::new()),
            pending_leaves: Mutex::new(BTreeSet::new()),
            suspected: Mutex::new(BTreeMap::new()),
            generation: AtomicU64::new(0),
            current_time_ms: AtomicU64::new(0),
            stats: Arc::new(MembershipStatistics::default()),
            last_gossip: Mutex::new(None),
            cluster_state: Mutex::new(ClusterState::Uninitialized),
        }
    }

    /// Join a cluster
    pub fn join_cluster(&self) -> Result<(), MembershipError> {
        // Check if already in cluster
        let membership = self.membership.read();
        if membership.members.contains_key(&self.local_node) && membership.members.len() > 1 {
            return Ok(());
        }
        drop(membership);

        // Try to contact seed nodes
        if self.config.seed_nodes.is_empty() {
            // No seed nodes, initialize as first node
            return self.bootstrap_cluster();
        }

        *self.cluster_state.lock() = ClusterState::Bootstrapping;

        // In a real implementation, contact seed nodes and request join
        // For now, simulate successful join
        self.increment_generation();
        self.stats.joins_processed.fetch_add(1, Ordering::Relaxed);

        *self.cluster_state.lock() = ClusterState::Normal;

        Ok(())
    }

    /// Bootstrap a new cluster (first node)
    fn bootstrap_cluster(&self) -> Result<(), MembershipError> {
        let mut info = self.local_info.lock();
        info.generation = 0;
        info.status = NodeStatus::Alive;
        drop(info);

        let mut membership = self.membership.write();
        membership.generation = 0;
        membership.members.clear();
        membership
            .members
            .insert(self.local_node, self.local_info.lock().clone());
        drop(membership);

        *self.cluster_state.lock() = ClusterState::Normal;
        self.stats.cluster_size.store(1, Ordering::Relaxed);

        Ok(())
    }

    /// Leave the cluster
    pub fn leave_cluster(&self) -> Result<(), MembershipError> {
        // Update status to leaving
        {
            let mut membership = self.membership.write();
            if let Some(member) = membership.members.get_mut(&self.local_node) {
                member.status = NodeStatus::Leaving;
            }
        }

        // Add to pending leaves
        let mut pending = self.pending_leaves.lock();
        pending.insert(self.local_node);

        // Increment generation
        self.increment_generation();

        // In a real implementation, notify other nodes
        self.stats.leaves_processed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Process a join request from another node
    pub fn process_join_request(&self, node_info: NodeInfo) -> Result<(), MembershipError> {
        let node_id = node_info.node_id;

        // Check if node already exists
        {
            let membership = self.membership.read();
            if membership.members.contains_key(&node_id) {
                return Err(MembershipError::NodeAlreadyExists(node_id));
            }
        }

        // Add to pending joins
        let mut pending = self.pending_joins.lock();
        pending.insert(node_id);

        // In a real implementation, check if we can accept the join
        // For now, accept immediately

        // Add to membership
        self.add_member(node_info)?;

        self.stats.joins_processed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Process a leave request from another node
    pub fn process_leave_request(&self, node_id: NodeId) -> Result<(), MembershipError> {
        // Check if node exists
        {
            let membership = self.membership.read();
            if !membership.members.contains_key(&node_id) {
                return Err(MembershipError::NodeNotFound(node_id));
            }
        }

        // Mark as leaving
        {
            let mut membership = self.membership.write();
            if let Some(member) = membership.members.get_mut(&node_id) {
                member.status = NodeStatus::Leaving;
            }
        }

        // Add to pending leaves
        let mut pending = self.pending_leaves.lock();
        pending.insert(node_id);

        // Increment generation
        self.increment_generation();

        self.stats.leaves_processed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Add a member to the cluster
    fn add_member(&self, node_info: NodeInfo) -> Result<(), MembershipError> {
        let node_id = node_info.node_id;

        // Update membership
        {
            let mut membership = self.membership.write();
            membership.members.insert(node_id, node_info);
            self.stats.cluster_size.store(membership.members.len(), Ordering::Relaxed);
        }

        // Remove from pending joins
        let mut pending = self.pending_joins.lock();
        pending.remove(&node_id);

        // Trigger rebalancing if enabled
        if self.config.enable_rebalancing {
            self.trigger_rebalancing(RebalanceTrigger::NodeJoined(node_id));
        }

        Ok(())
    }

    /// Remove a member from the cluster
    pub fn remove_member(&self, node_id: NodeId) -> Result<(), MembershipError> {
        // Check if node exists
        {
            let membership = self.membership.read();
            if !membership.members.contains_key(&node_id) {
                return Err(MembershipError::NodeNotFound(node_id));
            }
        }

        // Remove from membership
        {
            let mut membership = self.membership.write();
            membership.members.remove(&node_id);
            self.stats.cluster_size.store(membership.members.len(), Ordering::Relaxed);
        }

        // Remove from pending leaves
        let mut pending = self.pending_leaves.lock();
        pending.remove(&node_id);

        // Remove from suspected
        let mut suspected = self.suspected.lock();
        suspected.remove(&node_id);

        // Increment generation
        self.increment_generation();

        self.stats.membership_changes.fetch_add(1, Ordering::Relaxed);

        // Trigger rebalancing if enabled
        if self.config.enable_rebalancing {
            self.trigger_rebalancing(RebalanceTrigger::NodeLeft(node_id));
        }

        Ok(())
    }

    /// Process heartbeat from a node
    pub fn process_heartbeat(
        &self,
        node_id: NodeId,
        generation: Generation,
        heartbeat_version: u64,
    ) -> Result<(), MembershipError> {
        let current_time = self.current_time();

        // Update node info
        let mut membership = self.membership.write();
        if let Some(member) = membership.members.get_mut(&node_id) {
            member.last_heartbeat_ms = current_time;
            member.heartbeat_version = heartbeat_version;
            member.status = NodeStatus::Alive;
            member.last_update_ms = current_time;

            // Remove from suspected if present
            let mut suspected = self.suspected.lock();
            suspected.remove(&node_id);

            return Ok(());
        }

        Err(MembershipError::NodeNotFound(node_id))
    }

    /// Check for failed nodes and update status
    pub fn check_failures(&self) -> Result<Vec<NodeId>, MembershipError> {
        let current_time = self.current_time();
        let mut failed_nodes = Vec::new();

        // Check all members
        let membership = self.membership.read();
        for (node_id, member) in membership.members.iter() {
            if *node_id == self.local_node {
                continue;
            }

            let elapsed = current_time.saturating_sub(member.last_heartbeat_ms);

            if member.status == NodeStatus::Alive && elapsed > self.config.failure_timeout.as_millis() as u64 {
                // Node has failed, mark as suspected
                let mut suspected = self.suspected.lock();
                suspected.insert(*node_id, current_time);

                // Update status
                drop(membership);
                let mut membership = self.membership.write();
                if let Some(member) = membership.members.get_mut(node_id) {
                    member.status = NodeStatus::Suspect;
                }
                drop(membership);
                // Re-acquire for next iteration
                let membership = self.membership.read();

            } else if member.status == NodeStatus::Suspect {
                // Check if should be marked as dead
                let suspected = self.suspected.lock();
                if let Some(&suspected_time) = suspected.get(node_id) {
                    let suspicion_elapsed = current_time.saturating_sub(suspected_time);
                    if suspicion_elapsed > self.config.suspicion_timeout.as_millis() as u64 {
                        failed_nodes.push(*node_id);
                    }
                }
            }
        }

        // Mark failed nodes as dead
        for node_id in failed_nodes.iter() {
            let mut membership = self.membership.write();
            if let Some(member) = membership.members.get_mut(node_id) {
                member.status = NodeStatus::Dead;
            }
        }

        self.stats.membership_changes.fetch_add(failed_nodes.len() as u64, Ordering::Relaxed);

        Ok(failed_nodes)
    }

    /// Send gossip to peers
    pub fn send_gossip(&self) -> Result<Vec<GossipDigest>, MembershipError> {
        let current_time = self.current_time();

        // Check if it's time to gossip
        {
            let last = self.last_gossip.lock();
            if let Some(&last_time) = last.as_ref() {
                let elapsed = current_time.saturating_sub(last_time);
                if elapsed < self.config.gossip_interval.as_millis() as u64 {
                    return Ok(Vec::new());
                }
            }
        }

        // Update last gossip time
        *self.last_gossip.lock() = Some(current_time);

        // Create gossip digest
        let membership = self.membership.read();
        let mut digests = Vec::new();

        for (node_id, member) in membership.members.iter() {
            digests.push(GossipDigest {
                node_id: *node_id,
                generation: member.generation,
                heartbeat_version: member.heartbeat_version,
                status: member.status,
            });
        }

        self.stats.gossip_sent.fetch_add(1, Ordering::Relaxed);

        Ok(digests)
    }

    /// Process received gossip
    pub fn process_gossip(&self, digests: Vec<GossipDigest>) -> Result<Vec<NodeId>, MembershipError> {
        let mut new_nodes = Vec::new();
        let current_time = self.current_time();

        let mut membership = self.membership.write();

        for digest in digests {
            // If we don't know about this node, it might be new
            if !membership.members.contains_key(&digest.node_id) {
                // In a real implementation, we would fetch full node info
                // For now, just track it
                new_nodes.push(digest.node_id);
                continue;
            }

            // Update known nodes
            if let Some(member) = membership.members.get_mut(&digest.node_id) {
                if digest.generation > member.generation
                    || (digest.generation == member.generation
                        && digest.heartbeat_version > member.heartbeat_version)
                {
                    member.status = digest.status;
                    member.generation = digest.generation;
                    member.heartbeat_version = digest.heartbeat_version;
                    member.last_update_ms = current_time;
                }
            }
        }

        self.stats.gossip_received.fetch_add(1, Ordering::Relaxed);

        Ok(new_nodes)
    }

    /// Get current membership view
    pub fn get_members(&self) -> Vec<NodeInfo> {
        let membership = self.membership.read();
        membership.members.values().cloned().collect()
    }

    /// Get information about a specific member
    pub fn get_member(&self, node_id: NodeId) -> Option<NodeInfo> {
        let membership = self.membership.read();
        membership.members.get(&node_id).cloned()
    }

    /// Get cluster topology
    pub fn get_topology(&self) -> ClusterTopology {
        let membership = self.membership.read();

        ClusterTopology {
            generation: membership.generation,
            nodes: membership.members.keys().copied().collect(),
            size: membership.members.len(),
            state: *self.cluster_state.lock(),
        }
    }

    /// Detect network partition
    pub fn detect_partition(&self) -> Option<PartitionInfo> {
        let membership = self.membership.read();
        let alive_nodes: BTreeSet<_> = membership
            .members
            .iter()
            .filter(|(_, m)| m.status == NodeStatus::Alive)
            .map(|(id, _)| *id)
            .collect();

        let quorum = (alive_nodes.len() / 2) + 1;
        let has_quorum = alive_nodes.len() >= quorum;

        if !has_quorum {
            self.stats.partitions_detected.fetch_add(1, Ordering::Relaxed);
            return Some(PartitionInfo {
                nodes: alive_nodes,
                partition_id: self.current_time(),
                has_quorum,
                leader: None, // Would be determined by consensus
            });
        }

        None
    }

    /// Trigger rebalancing
    fn trigger_rebalancing(&self, trigger: RebalanceTrigger) {
        // In a real implementation, this would trigger actual rebalancing
        self.stats.rebalancing_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment generation
    fn increment_generation(&self) {
        let new_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let mut membership = self.membership.write();
        membership.generation = new_gen;
        self.stats.membership_changes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current time in milliseconds
    fn current_time(&self) -> u64 {
        self.current_time_ms.load(Ordering::Relaxed)
    }

    /// Set current time (for testing)
    pub fn set_time(&self, time: u64) {
        self.current_time_ms.store(time, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &MembershipStatistics {
        &self.stats
    }

    /// Check if we can form quorum
    pub fn has_quorum(&self) -> bool {
        let membership = self.membership.read();
        let alive = membership
            .members
            .values()
            .filter(|m| m.status == NodeStatus::Alive)
            .count();
        let quorum = (membership.members.len() / 2) + 1;
        alive >= quorum
    }
}

/// Cluster topology information
#[derive(Debug, Clone)]
pub struct ClusterTopology {
    /// Generation number
    pub generation: Generation,
    /// Nodes in the cluster
    pub nodes: Vec<NodeId>,
    /// Cluster size
    pub size: usize,
    /// Cluster state
    pub state: ClusterState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_cluster() {
        let config = ClusterConfig::default();
        let membership = ClusterMembership::new(1, "127.0.0.1:8080".to_string(), config);

        membership.join_cluster().unwrap();

        let members = membership.get_members();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].node_id, 1);
    }

    #[test]
    fn test_node_join() {
        let config = ClusterConfig::default();
        let membership = ClusterMembership::new(1, "127.0.0.1:8080".to_string(), config);

        membership.join_cluster().unwrap();

        let node_info = NodeInfo {
            node_id: 2,
            address: "127.0.0.1:8081".to_string(),
            status: NodeStatus::Alive,
            generation: 0,
            heartbeat_version: 0,
            last_heartbeat_ms: 0,
            roles: Vec::new(),
            metadata: BTreeMap::new(),
            last_update_ms: 0,
        };

        membership.process_join_request(node_info).unwrap();

        let members = membership.get_members();
        assert_eq!(members.len(), 2);
    }
}
