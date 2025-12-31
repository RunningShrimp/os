//! Cluster Management and Coordination
//!
//! This module provides comprehensive clustering capabilities for the NOS kernel,
//! including distributed locking, consensus, failover, load balancing, and membership.

pub mod balance;
pub mod consensus;
pub mod failover;
pub mod lock;
pub mod membership;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use crate::sync::{Mutex, RwLock};

// Re-export key types
pub use balance::{
    BalanceError, BackpressureState, LoadBalanceStrategy, LoadBalancer, LoadBalancerConfig,
    NodeLoad, RequestMetadata,
};
pub use consensus::{
    AppendEntriesArgs, AppendEntriesReply, Configuration, ConsensusError, ConsensusStatistics,
    EntryType, InstallSnapshotArgs, InstallSnapshotReply, LogEntry, RaftConsensus,
    RequestVoteArgs, RequestVoteReply, ServerRole, Snapshot,
};
pub use failover::{
    Failable, FailoverAction, FailoverError, FailoverManager, FailoverStatistics,
    FailoverState, FailureDetector, FailureDetectorConfig, FencingMethod, FencingResult,
    NodeHealth, NodeHealthInfo,
};
pub use lock::{
    DlmError, DlmStatistics, DistributedLockManager, LockGranularity, LockId, LockInfo,
    LockMode, TransactionId,
};
pub use membership::{
    ClusterConfig, ClusterMembership, ClusterState, ClusterTopology, GossipDigest,
    MembershipError, MembershipStatistics, MembershipView, NodeInfo, NodeStatus, PartitionInfo,
    RebalanceTrigger,
};

/// Unique identifier for a node
pub type NodeId = u64;

/// Cluster-wide configuration
#[derive(Debug, Clone)]
pub struct ClusterConfiguration {
    /// Local node ID
    pub local_node_id: NodeId,
    /// Local node address
    pub local_address: String,
    /// Seed nodes for bootstrap
    pub seed_nodes: Vec<String>,
    /// Gossip interval
    pub gossip_interval: Duration,
    /// Failure detection timeout
    pub failure_timeout: Duration,
    /// Election timeout
    pub election_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Enable automatic failover
    pub enable_auto_failover: bool,
    /// Load balancing strategy
    pub load_balance_strategy: LoadBalanceStrategy,
    /// Maximum cluster size
    pub max_cluster_size: usize,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for ClusterConfiguration {
    fn default() -> Self {
        Self {
            local_node_id: 1,
            local_address: "127.0.0.1:8080".to_string(),
            seed_nodes: Vec::new(),
            gossip_interval: Duration::from_millis(100),
            failure_timeout: Duration::from_secs(5),
            election_timeout: Duration::from_millis(150),
            heartbeat_interval: Duration::from_millis(50),
            enable_auto_failover: true,
            load_balance_strategy: LoadBalanceStrategy::Adaptive,
            max_cluster_size: 100,
            enable_metrics: true,
        }
    }
}

/// Node registry information
#[derive(Debug, Clone)]
pub struct NodeRegistryEntry {
    /// Node ID
    pub node_id: NodeId,
    /// Node address
    pub address: String,
    /// Node capabilities
    pub capabilities: Vec<String>,
    /// Node metadata
    pub metadata: BTreeMap<String, String>,
    /// Registration time
    pub registered_ms: u64,
    /// Last heartbeat time
    pub last_heartbeat_ms: u64,
    /// Node status
    pub status: NodeStatus,
}

/// RPC message for cluster operations
#[derive(Debug, Clone)]
pub enum ClusterRpc {
    /// Request vote
    RequestVote(RequestVoteArgs),
    /// Request vote reply
    RequestVoteReply(RequestVoteReply),
    /// Append entries
    AppendEntries(AppendEntriesArgs),
    /// Append entries reply
    AppendEntriesReply(AppendEntriesReply),
    /// Install snapshot
    InstallSnapshot(InstallSnapshotArgs),
    /// Install snapshot reply
    InstallSnapshotReply(InstallSnapshotReply),
    /// Gossip message
    Gossip(Vec<GossipDigest>),
    /// Join request
    JoinRequest(NodeInfo),
    /// Join response
    JoinResponse { accepted: bool, generation: u64 },
    /// Leave request
    LeaveRequest(NodeId),
    /// Heartbeat
    Heartbeat {
        node_id: NodeId,
        generation: u64,
        version: u64,
    },
    /// Lock request
    LockRequest {
        resource: String,
        mode: LockMode,
        transaction_id: Option<u64>,
    },
    /// Lock response
    LockResponse { lock_id: Option<u64> },
    /// Unlock request
    UnlockRequest { lock_id: u64 },
    /// Unlock response
    UnlockResponse { success: bool },
}

/// Cluster statistics
#[derive(Debug, Default)]
pub struct ClusterStatistics {
    /// Total RPCs sent
    pub rpcs_sent: AtomicU64,
    /// Total RPCs received
    pub rpcs_received: AtomicU64,
    /// RPCs by type
    pub rpcs_by_type: Mutex<BTreeMap<String, AtomicU64>>,
    /// Average RPC latency (microseconds)
    pub avg_rpc_latency_us: AtomicU64,
    /// Total data transferred (bytes)
    pub data_transferred: AtomicU64,
    /// Cluster uptime (milliseconds)
    pub uptime_ms: AtomicU64,
    /// Leadership transitions
    pub leadership_transitions: AtomicU64,
    /// Failover events
    pub failover_events: AtomicU64,
    /// Membership changes
    pub membership_changes: AtomicU64,
}

/// Errors that can occur in cluster operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    /// Node not found
    NodeNotFound(NodeId),
    /// Cluster not initialized
    NotInitialized,
    /// Cluster is shutting down
    ShuttingDown,
    /// RPC failure
    RpcFailed(String),
    /// Timeout
    Timeout,
    /// Configuration error
    ConfigurationError(String),
    /// Permission denied
    PermissionDenied,
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for ClusterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ClusterError::NodeNotFound(node_id) => {
                write!(f, "Node {} not found", node_id)
            }
            ClusterError::NotInitialized => {
                write!(f, "Cluster not initialized")
            }
            ClusterError::ShuttingDown => {
                write!(f, "Cluster is shutting down")
            }
            ClusterError::RpcFailed(msg) => {
                write!(f, "RPC failed: {}", msg)
            }
            ClusterError::Timeout => {
                write!(f, "Operation timeout")
            }
            ClusterError::ConfigurationError(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
            ClusterError::PermissionDenied => {
                write!(f, "Permission denied")
            }
            ClusterError::InternalError(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

/// Cluster Manager - main entry point for cluster operations
pub struct ClusterManager {
    /// Configuration
    config: ClusterConfiguration,
    /// Local node ID
    local_node: NodeId,
    /// Consensus module
    consensus: Arc<RaftConsensus>,
    /// Membership module
    membership: Arc<ClusterMembership>,
    /// Failover manager
    failover: Arc<FailoverManager>,
    /// Distributed lock manager
    dlm: Arc<DistributedLockManager>,
    /// Load balancer
    load_balancer: Arc<LoadBalancer>,
    /// Node registry
    node_registry: RwLock<BTreeMap<NodeId, NodeRegistryEntry>>,
    /// Running state
    running: AtomicBool,
    /// Statistics
    stats: Arc<ClusterStatistics>,
    /// Start time
    start_time_ms: AtomicU64,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub fn new(config: ClusterConfiguration) -> Result<Self, ClusterError> {
        let local_node = config.local_node_id;

        // Initialize consensus
        let mut consensus_config = Configuration {
            voters: BTreeSet::new(),
            new_voters: None,
        };
        consensus_config.voters.insert(local_node);

        let consensus = Arc::new(RaftConsensus::new(
            local_node,
            consensus_config,
            config.election_timeout,
            config.heartbeat_interval,
        ));

        // Initialize membership
        let membership_config = ClusterConfig {
            seed_nodes: config.seed_nodes.clone(),
            gossip_interval: config.gossip_interval,
            failure_timeout: config.failure_timeout,
            ..Default::default()
        };

        let membership = Arc::new(ClusterMembership::new(
            local_node,
            config.local_address.clone(),
            membership_config,
        ));

        // Initialize failover
        let failover_config = FailureDetectorConfig {
            heartbeat_interval: config.heartbeat_interval,
            failure_timeout: config.failure_timeout,
            ..Default::default()
        };

        let failover = Arc::new(FailoverManager::new(
            local_node,
            failover_config,
            Duration::from_secs(5),
            Duration::from_secs(10),
        ));

        // Initialize DLM
        let dlm = Arc::new(DistributedLockManager::new(
            local_node,
            10000, // max locks
            Duration::from_secs(30),
        ));

        // Initialize load balancer
        let lb_config = LoadBalancerConfig {
            strategy: config.load_balance_strategy,
            ..Default::default()
        };

        let load_balancer = Arc::new(LoadBalancer::new(lb_config));

        Ok(Self {
            config,
            local_node,
            consensus,
            membership,
            failover,
            dlm,
            load_balancer,
            node_registry: RwLock::new(BTreeMap::new()),
            running: AtomicBool::new(false),
            stats: Arc::new(ClusterStatistics::default()),
            start_time_ms: AtomicU64::new(0),
        })
    }

    /// Initialize and start the cluster
    pub fn initialize(&self) -> Result<(), ClusterError> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Start cluster membership
        self.membership
            .join_cluster()
            .map_err(|e| ClusterError::InternalError(format!("Failed to join cluster: {}", e)))?;

        // Register self in node registry
        self.register_local_node();

        // Start background tasks
        self.running.store(true, Ordering::Relaxed);
        self.start_time_ms
            .store(self.current_time_ms(), Ordering::Relaxed);

        Ok(())
    }

    /// Shutdown the cluster
    pub fn shutdown(&self) -> Result<(), ClusterError> {
        self.running.store(false, Ordering::Relaxed);

        // Leave cluster
        let _ = self.membership.leave_cluster();

        Ok(())
    }

    /// Register local node in registry
    fn register_local_node(&self) {
        let entry = NodeRegistryEntry {
            node_id: self.local_node,
            address: self.config.local_address.clone(),
            capabilities: vec!["storage".to_string(), "compute".to_string()],
            metadata: {
                let mut meta = BTreeMap::new();
                meta.insert("version".to_string(), "1.0".to_string());
                meta
            },
            registered_ms: self.current_time_ms(),
            last_heartbeat_ms: self.current_time_ms(),
            status: NodeStatus::Alive,
        };

        let mut registry = self.node_registry.write();
        registry.insert(self.local_node, entry);
    }

    /// Process an RPC message
    pub fn process_rpc(&self, rpc: ClusterRpc) -> Result<Option<ClusterRpc>, ClusterError> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(ClusterError::ShuttingDown);
        }

        self.stats.rpcs_received.fetch_add(1, Ordering::Relaxed);

        match rpc {
            ClusterRpc::RequestVote(args) => {
                let reply = self
                    .consensus
                    .request_vote(args)
                    .map_err(|e| ClusterError::InternalError(format!("Request vote failed: {}", e)))?;
                Ok(Some(ClusterRpc::RequestVoteReply(reply)))
            }
            ClusterRpc::AppendEntries(args) => {
                let reply = self
                    .consensus
                    .append_entries(args)
                    .map_err(|e| ClusterError::InternalError(format!("Append entries failed: {}", e)))?;
                Ok(Some(ClusterRpc::AppendEntriesReply(reply)))
            }
            ClusterRpc::InstallSnapshot(args) => {
                let reply = self
                    .consensus
                    .install_snapshot(args)
                    .map_err(|e| ClusterError::InternalError(format!("Install snapshot failed: {}", e)))?;
                Ok(Some(ClusterRpc::InstallSnapshotReply(reply)))
            }
            ClusterRpc::Gossip(digests) => {
                let _ = self.membership.process_gossip(digests);
                Ok(None)
            }
            ClusterRpc::JoinRequest(node_info) => {
                let accepted = self
                    .membership
                    .process_join_request(node_info.clone())
                    .is_ok();
                let generation = self.membership.get_topology().generation;
                Ok(Some(ClusterRpc::JoinResponse { accepted, generation }))
            }
            ClusterRpc::Heartbeat {
                node_id,
                generation,
                version,
            } => {
                let _ = self.membership.process_heartbeat(node_id, generation, version);
                let _ = self.failover.process_heartbeat(node_id);
                Ok(None)
            }
            ClusterRpc::LockRequest {
                resource,
                mode,
                transaction_id,
            } => {
                let lock_id = self
                    .dlm
                    .acquire_lock(&resource, mode, None, LockGranularity::File)
                    .ok()
                    .map(|id| id);
                Ok(Some(ClusterRpc::LockResponse { lock_id }))
            }
            ClusterRpc::UnlockRequest { lock_id } => {
                let success = self.dlm.release_lock(lock_id).is_ok();
                Ok(Some(ClusterRpc::UnlockResponse { success }))
            }
            _ => Ok(None),
        }
    }

    /// Propose a value to the cluster
    pub fn propose(&self, entry: LogEntry) -> Result<u64, ClusterError> {
        self.consensus
            .propose(entry)
            .map_err(|e| ClusterError::InternalError(format!("Propose failed: {}", e)))
    }

    /// Read from the log
    pub fn read(&self, index: u64) -> Result<Option<LogEntry>, ClusterError> {
        self.consensus
            .read_index(index)
            .map_err(|e| ClusterError::InternalError(format!("Read failed: {}", e)))
    }

    /// Acquire a distributed lock
    pub fn acquire_lock(
        &self,
        resource: &str,
        mode: LockMode,
        timeout: Option<Duration>,
    ) -> Result<LockId, ClusterError> {
        self.dlm
            .acquire_lock(resource, mode, timeout, LockGranularity::File)
            .map_err(|e| ClusterError::InternalError(format!("Lock acquisition failed: {}", e)))
    }

    /// Release a distributed lock
    pub fn release_lock(&self, lock_id: LockId) -> Result<(), ClusterError> {
        self.dlm
            .release_lock(lock_id)
            .map_err(|e| ClusterError::InternalError(format!("Lock release failed: {}", e)))
    }

    /// Select a node for load balancing
    pub fn select_node(&self, metadata: Option<&RequestMetadata>) -> Result<NodeId, ClusterError> {
        self.load_balancer
            .select_node(metadata)
            .map_err(|e| ClusterError::InternalError(format!("Node selection failed: {}", e)))
    }

    /// Get cluster members
    pub fn get_members(&self) -> Vec<NodeInfo> {
        self.membership.get_members()
    }

    /// Get cluster topology
    pub fn get_topology(&self) -> ClusterTopology {
        self.membership.get_topology()
    }

    /// Check if cluster is healthy
    pub fn is_healthy(&self) -> bool {
        self.membership.has_quorum() && !self.failover.is_failover_in_progress()
    }

    /// Get consensus module
    pub fn consensus(&self) -> Arc<RaftConsensus> {
        Arc::clone(&self.consensus)
    }

    /// Get membership module
    pub fn membership(&self) -> Arc<ClusterMembership> {
        Arc::clone(&self.membership)
    }

    /// Get failover manager
    pub fn failover(&self) -> Arc<FailoverManager> {
        Arc::clone(&self.failover)
    }

    /// Get distributed lock manager
    pub fn dlm(&self) -> Arc<DistributedLockManager> {
        Arc::clone(&self.dlm)
    }

    /// Get load balancer
    pub fn load_balancer(&self) -> Arc<LoadBalancer> {
        Arc::clone(&self.load_balancer)
    }

    /// Get cluster statistics
    pub fn get_statistics(&self) -> &ClusterStatistics {
        &self.stats
    }

    /// Run periodic maintenance tasks
    pub fn run_maintenance(&self) -> Result<(), ClusterError> {
        if !self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Update uptime
        let uptime = self.current_time_ms().saturating_sub(self.start_time_ms.load(Ordering::Relaxed));
        self.stats.uptime_ms.store(uptime, Ordering::Relaxed);

        // Check for failures
        if let Ok(failed_nodes) = self.failover.detector().check_node_failure(self.local_node) {
            if failed_nodes {
                // Handle failure
            }
        }

        // Send heartbeats
        let _ = self.consensus.send_heartbeats();

        // Apply committed entries
        let _ = self.consensus.apply_entries();

        // Process gossip
        let _ = self.membership.send_gossip();

        Ok(())
    }

    /// Get current time in milliseconds
    fn current_time_ms(&self) -> u64 {
        // In a real implementation, this would use the actual time
        // For now, return a dummy value
        0
    }

    /// Check if cluster is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get local node ID
    pub fn local_node_id(&self) -> NodeId {
        self.local_node
    }

    /// Get cluster configuration
    pub fn config(&self) -> &ClusterConfiguration {
        &self.config
    }

    /// Add a voter to the cluster
    pub fn add_voter(&self, node_id: NodeId) -> Result<(), ClusterError> {
        self.consensus
            .add_voter(node_id)
            .map_err(|e| ClusterError::InternalError(format!("Add voter failed: {}", e)))
    }

    /// Remove a voter from the cluster
    pub fn remove_voter(&self, node_id: NodeId) -> Result<(), ClusterError> {
        self.consensus
            .remove_voter(node_id)
            .map_err(|e| ClusterError::InternalError(format!("Remove voter failed: {}", e)))
    }

    /// Trigger failover for a node
    pub fn trigger_failover(&self, node_id: NodeId) -> Result<(), ClusterError> {
        self.failover
            .trigger_failover(node_id)
            .map_err(|e| ClusterError::InternalError(format!("Failover failed: {}", e)))
    }

    /// Fence a node
    pub fn fence_node(&self, node_id: NodeId) -> Result<FencingResult, ClusterError> {
        self.failover
            .fence_node(node_id, FencingMethod::PowerOff)
            .map_err(|e| ClusterError::InternalError(format!("Fencing failed: {}", e)))
    }

    /// Update load information for a node
    pub fn update_node_load(&self, node_load: NodeLoad) -> Result<(), ClusterError> {
        self.load_balancer
            .update_node_load(node_load)
            .map_err(|e| ClusterError::InternalError(format!("Update load failed: {}", e)))
    }
}

/// Trait for cluster RPC handling
pub trait ClusterRpcHandler: Send + Sync {
    /// Handle an RPC message
    fn handle_rpc(&self, rpc: ClusterRpc) -> Result<Option<ClusterRpc>, ClusterError>;

    /// Send an RPC message to a node
    fn send_rpc(&self, node_id: NodeId, rpc: ClusterRpc) -> Result<Option<ClusterRpc>, ClusterError>;
}

/// Default RPC handler implementation
impl ClusterRpcHandler for ClusterManager {
    fn handle_rpc(&self, rpc: ClusterRpc) -> Result<Option<ClusterRpc>, ClusterError> {
        self.process_rpc(rpc)
    }

    fn send_rpc(&self, _node_id: NodeId, _rpc: ClusterRpc) -> Result<Option<ClusterRpc>, ClusterError> {
        // In a real implementation, this would send the RPC over the network
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_initialization() {
        let config = ClusterConfiguration {
            local_node_id: 1,
            local_address: "127.0.0.1:8080".to_string(),
            ..Default::default()
        };

        let manager = ClusterManager::new(config).unwrap();

        assert!(!manager.is_running());

        manager.initialize().unwrap();

        assert!(manager.is_running());
        assert_eq!(manager.local_node_id(), 1);
    }

    #[test]
    fn test_cluster_topology() {
        let config = ClusterConfiguration::default();
        let manager = ClusterManager::new(config).unwrap();

        manager.initialize().unwrap();

        let topology = manager.get_topology();
        assert!(topology.nodes.contains(&1));
    }
}
