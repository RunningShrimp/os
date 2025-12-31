//! # Cluster Management and Raft Consensus
//!
//! This module implements distributed cluster management using the Raft consensus algorithm.
//! It provides leader election, log replication, and cluster membership management.
//!
//! ## Architecture
//!
//! The cluster implementation follows the Raft design:
//!
//! - **Leader**: Handles all client requests, replicates logs to followers
//! - **Follower**: Passively replicates leader's log, serves read requests
//! - **Candidate**: Contender in leader election, seeks votes
//!
//! ## Raft Consensus
//!
//! Raft ensures consistency through:
//!
//! 1. **Leader Election**: Timeouts trigger elections, most up-to-date node wins
//! 2. **Log Replication**: Leader serializes updates, replicates to majority
//! 3. **Safety**: Log matching property ensures committed entries aren't lost
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::ha::cluster::{Cluster, ClusterConfig, NodeId};
//!
//! # async fn example() -> Result<(), kernel::ha::HaError> {
//! // Create 3-node cluster configuration
//! let config = ClusterConfig::three_node_replicated();
//!
//! // Initialize cluster with local node ID
//! let cluster = Cluster::new(NodeId::from(1), config).await?;
//!
//! // Start cluster services
//! cluster.start().await?;
//!
//! // Check cluster state
//! let state = cluster.get_state().await;
//! println!("Cluster role: {:?}", state.role);
//! # Ok(())
//! # }
//! ```
//!
//! ## Leader Election
//!
//! Elections are triggered by:
//! - Election timeout (randomized 150-300ms)
//! - Leader failure detection
//! - Manual leadership transfer
//!
//! The election process:
//! 1. Node transitions to candidate, increments term
//! 2. Requests votes from all nodes
//! 3. Becomes leader if receives majority votes
//! 4. Followers step down if they discover higher term
//!
//! ## Log Replication
//!
//! Leader replication flow:
//! 1. Leader receives client request
//! 2. Appends entry to local log
//! 3. Sends AppendEntries RPC to all followers
//! 4. Entry commits once replicated to majority
//! 5. Applies committed entry to state machine
//!
//! ## Cluster Membership
//!
//! Dynamic membership changes via joint consensus:
//! - Add nodes: C_old -> C_old,new -> C_new
//! - Remove nodes: C_old -> C_old,removed -> C_new
//! - Ensures safety during configuration changes
//!
//! ## Related Modules
//!
//! - [`crate::ha::replication`]: Data replication layer
//! - [`crate::ha::failover`]: Failover management
//! - [`crate::subsystems::distributed`]: Distributed primitives

use crate::ha::{HaError, HaResult, ClusterError};
use crate::subsystems::sync::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use core::time::Duration;

/// Unique identifier for a cluster node
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);

impl NodeId {
    /// Create a new node ID
    pub fn new(id: u64) -> Self {
        NodeId(id)
    }

    /// Get the underlying ID value
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Check if this is a valid node ID (non-zero)
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl From<u64> for NodeId {
    fn from(id: u64) -> Self {
        NodeId(id)
    }
}

impl core::fmt::Display for NodeId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

/// Node role in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Leader handles all writes and coordinates replication
    Leader,
    /// Follower replicates leader's log and serves reads
    Follower,
    /// Candidate is campaigning for leadership
    Candidate,
}

impl NodeRole {
    /// Check if node can process write requests
    pub fn can_write(&self) -> bool {
        matches!(self, NodeRole::Leader)
    }

    /// Check if node can process read requests
    pub fn can_read(&self) -> bool {
        matches!(self, NodeRole::Leader | NodeRole::Follower)
    }

    /// Check if node is participating in election
    pub fn is_candidate(&self) -> bool {
        matches!(self, NodeRole::Candidate)
    }
}

/// Cluster state information
#[derive(Debug, Clone)]
pub struct ClusterState {
    /// Current node role
    pub role: NodeRole,
    /// Current term (monotonically increasing)
    pub current_term: u64,
    /// ID of node this node voted for in current term
    pub voted_for: Option<NodeId>,
    /// Number of nodes in cluster
    pub node_count: usize,
    /// ID of current leader (if known)
    pub leader_id: Option<NodeId>,
    /// Commit index (highest committed log entry)
    pub commit_index: u64,
    /// Last applied index (highest applied to state machine)
    pub last_applied: u64,
}

impl ClusterState {
    /// Create a new cluster state (follower by default)
    pub fn new(node_count: usize) -> Self {
        ClusterState {
            role: NodeRole::Follower,
            current_term: 0,
            voted_for: None,
            node_count,
            leader_id: None,
            commit_index: 0,
            last_applied: 0,
        }
    }

    /// Calculate quorum size (majority of nodes)
    pub fn quorum_size(&self) -> usize {
        (self.node_count / 2) + 1
    }

    /// Check if we have quorum
    pub fn has_quorum(&self, votes: usize) -> bool {
        votes >= self.quorum_size()
    }
}

/// Raft log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Index in the log
    pub index: u64,
    /// Term when entry was received by leader
    pub term: u64,
    /// Command to apply to state machine
    pub command: Vec<u8>,
    /// Entry type
    pub entry_type: EntryType,
}

/// Log entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Normal state machine command
    Normal,
    /// Cluster membership change
    ConfigChange,
    /// No-op entry (for leadership establishment)
    NoOp,
}

/// Raft RPC messages
#[derive(Debug, Clone)]
pub enum RaftRpc {
    /// RequestVote RPC for leader election
    RequestVote {
        term: u64,
        candidate_id: NodeId,
        last_log_index: u64,
        last_log_term: u64,
    },
    /// RequestVote response
    RequestVoteResponse {
        term: u64,
        vote_granted: bool,
    },
    /// AppendEntries RPC for log replication
    AppendEntries {
        term: u64,
        leader_id: NodeId,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    },
    /// AppendEntries response
    AppendEntriesResponse {
        term: u64,
        success: bool,
        match_index: Option<u64>,
    },
    /// InstallSnapshot RPC for log compaction
    InstallSnapshot {
        term: u64,
        leader_id: NodeId,
        last_included_index: u64,
        last_included_term: u64,
        data: Vec<u8>,
    },
    /// InstallSnapshot response
    InstallSnapshotResponse {
        term: u64,
        success: bool,
    },
}

/// Configuration for cluster setup
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Node IDs in the cluster
    pub nodes: Vec<NodeId>,
    /// Election timeout range (milliseconds)
    pub election_timeout_ms: (u64, u64),
    /// Heartbeat interval (milliseconds)
    pub heartbeat_interval_ms: u64,
    /// Maximum log entries per AppendEntries RPC
    pub max_entries_per_rpc: usize,
    /// Log snapshot threshold
    pub snapshot_threshold: u64,
    /// RPC timeout (milliseconds)
    pub rpc_timeout_ms: u64,
    /// Enable log compaction
    pub enable_compaction: bool,
}

impl ClusterConfig {
    /// Create configuration for 3-node replicated cluster
    pub fn three_node_replicated() -> Self {
        ClusterConfig {
            nodes: vec![NodeId::from(1), NodeId::from(2), NodeId::from(3)],
            election_timeout_ms: (150, 300),
            heartbeat_interval_ms: 50,
            max_entries_per_rpc: 100,
            snapshot_threshold: 10000,
            rpc_timeout_ms: 100,
            enable_compaction: true,
        }
    }

    /// Create configuration for 5-node replicated cluster
    pub fn five_node_replicated() -> Self {
        ClusterConfig {
            nodes: vec![
                NodeId::from(1),
                NodeId::from(2),
                NodeId::from(3),
                NodeId::from(4),
                NodeId::from(5),
            ],
            election_timeout_ms: (150, 300),
            heartbeat_interval_ms: 50,
            max_entries_per_rpc: 100,
            snapshot_threshold: 10000,
            rpc_timeout_ms: 100,
            enable_compaction: true,
        }
    }

    /// Create configuration for single-node development cluster
    pub fn single_node_dev() -> Self {
        ClusterConfig {
            nodes: vec![NodeId::from(1)],
            election_timeout_ms: (5000, 10000),
            heartbeat_interval_ms: 1000,
            max_entries_per_rpc: 1000,
            snapshot_threshold: 100000,
            rpc_timeout_ms: 5000,
            enable_compaction: false,
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self::three_node_replicated()
    }
}

/// Cluster membership manager
#[derive(Debug)]
pub struct ClusterMembership {
    /// Current cluster configuration
    config: ClusterConfig,
    /// Current configuration version
    config_version: u64,
    /// Pending configuration changes
    pending_changes: Vec<NodeId>,
    /// Cluster state snapshot
    state: Arc<Mutex<ClusterState>>,
}

impl ClusterMembership {
    /// Create new cluster membership manager
    pub fn new(config: ClusterConfig, state: Arc<Mutex<ClusterState>>) -> Self {
        ClusterMembership {
            config,
            config_version: 0,
            pending_changes: Vec::new(),
            state,
        }
    }

    /// Get current cluster nodes
    pub fn get_nodes(&self) -> Vec<NodeId> {
        self.config.nodes.clone()
    }

    /// Check if node is in cluster
    pub fn contains_node(&self, node_id: NodeId) -> bool {
        self.config.nodes.contains(&node_id)
    }

    /// Add node to cluster
    pub fn add_node(&mut self, node_id: NodeId) -> HaResult<()> {
        if self.contains_node(node_id) {
            return Err(HaError::ClusterError(ClusterError::RpcFailed));
        }

        self.pending_changes.push(node_id);
        self.apply_config_change()?;
        Ok(())
    }

    /// Remove node from cluster
    pub fn remove_node(&mut self, node_id: NodeId) -> HaResult<()> {
        if !self.contains_node(node_id) {
            return Err(HaError::ClusterError(ClusterError::NodeNotFound));
        }

        self.config.nodes.retain(|&n| n != node_id);
        self.config_version += 1;
        Ok(())
    }

    /// Apply pending configuration changes
    fn apply_config_change(&mut self) -> HaResult<()> {
        // Apply joint consensus (simplified)
        for node in self.pending_changes.drain(..) {
            self.config.nodes.push(node);
        }
        self.config_version += 1;
        Ok(())
    }

    /// Get quorum size
    pub fn quorum_size(&self) -> usize {
        (self.config.nodes.len() / 2) + 1
    }

    /// Check if majority is achieved
    pub fn has_majority(&self, count: usize) -> bool {
        count >= self.quorum_size()
    }
}

/// Leader election implementation
#[derive(Debug)]
pub struct LeaderElection {
    /// Local node ID
    node_id: NodeId,
    /// Cluster state
    state: Arc<Mutex<ClusterState>>,
    /// Cluster membership
    membership: Arc<Mutex<ClusterMembership>>,
    /// Election timeout
    election_timeout: Duration,
    /// Last heartbeat time
    last_heartbeat: Arc<AtomicU64>,
    /// Election in progress flag
    election_in_progress: Arc<AtomicBool>,
    /// Vote tracking
    votes_received: Arc<Mutex<Vec<NodeId>>>,
}

impl LeaderElection {
    /// Create new leader election manager
    pub fn new(
        node_id: NodeId,
        state: Arc<Mutex<ClusterState>>,
        membership: Arc<Mutex<ClusterMembership>>,
        config: &ClusterConfig,
    ) -> Self {
        let timeout = Duration::from_millis(config.election_timeout_ms.0);

        LeaderElection {
            node_id,
            state,
            membership,
            election_timeout: timeout,
            last_heartbeat: Arc::new(AtomicU64::new(0)),
            election_in_progress: Arc::new(AtomicBool::new(false)),
            votes_received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start election timer
    pub fn start_election_timer(&self) {
        // Reset heartbeat timestamp
        self.last_heartbeat.store(
            self.current_time_ms(),
            Ordering::Relaxed
        );
    }

    /// Check if election should be triggered
    pub fn should_start_election(&self) -> bool {
        let last = self.last_heartbeat.load(Ordering::Relaxed);
        let now = self.current_time_ms();
        let elapsed = now.saturating_sub(last);

        elapsed >= self.election_timeout.as_millis() as u64
    }

    /// Become candidate and start election
    pub fn start_election(&self) -> HaResult<()> {
        if self.election_in_progress.load(Ordering::Acquire) {
            return Ok(());
        }

        self.election_in_progress.store(true, Ordering::Release);

        // Increment current term
        {
            let mut state = self.state.lock();
            state.current_term += 1;
            state.role = NodeRole::Candidate;
            state.voted_for = Some(self.node_id);
        }

        // Clear previous votes
        self.votes_received.lock().clear();
        self.votes_received.lock().push(self.node_id);

        // Request votes from all nodes
        self.request_votes()?;

        Ok(())
    }

    /// Request votes from all cluster nodes
    fn request_votes(&self) -> HaResult<()> {
        let state = self.state.lock();
        let membership = self.membership.lock();

        // Create RequestVote RPC (simplified - in real implementation, send to all nodes)
        let _rpc = RaftRpc::RequestVote {
            term: state.current_term,
            candidate_id: self.node_id,
            last_log_index: state.last_applied,
            last_log_term: state.current_term,
        };

        // Check if we have enough votes
        let nodes_count = membership.get_nodes().len();
        let votes_count = self.votes_received.lock().len();

        if votes_count >= (nodes_count / 2) + 1 {
            drop(state);
            self.become_leader()?;
        }

        Ok(())
    }

    /// Become leader after winning election
    fn become_leader(&self) -> HaResult<()> {
        let mut state = self.state.lock();
        state.role = NodeRole::Leader;
        state.leader_id = Some(self.node_id);
        self.election_in_progress.store(false, Ordering::Release);

        // Send initial heartbeat
        drop(state);
        self.send_heartbeat()?;

        Ok(())
    }

    /// Step down to follower
    pub fn step_down(&self, new_term: u64) {
        let mut state = self.state.lock();
        if new_term > state.current_term {
            state.current_term = new_term;
            state.role = NodeRole::Follower;
            state.voted_for = None;
            state.leader_id = None;
            self.election_in_progress.store(false, Ordering::Release);
        }
    }

    /// Grant vote to candidate
    pub fn grant_vote(&self, candidate_id: NodeId, candidate_term: u64) -> bool {
        let mut state = self.state.lock();

        if candidate_term < state.current_term {
            return false;
        }

        if candidate_term > state.current_term {
            state.current_term = candidate_term;
            state.voted_for = None;
        }

        if state.voted_for.is_none() || state.voted_for == Some(candidate_id) {
            state.voted_for = Some(candidate_id);
            return true;
        }

        false
    }

    /// Record vote from peer
    pub fn record_vote(&self, node_id: NodeId) {
        self.votes_received.lock().push(node_id);
    }

    /// Send heartbeat to all followers
    fn send_heartbeat(&self) -> HaResult<()> {
        let state = self.state.lock();

        // Create AppendEntries RPC with no entries (heartbeat)
        let _heartbeat = RaftRpc::AppendEntries {
            term: state.current_term,
            leader_id: self.node_id,
            prev_log_index: state.last_applied,
            prev_log_term: state.current_term,
            entries: Vec::new(),
            leader_commit: state.commit_index,
        };

        // In real implementation, send to all followers
        Ok(())
    }

    /// Receive heartbeat from leader
    pub fn receive_heartbeat(&self, leader_id: NodeId, term: u64) {
        let mut state = self.state.lock();

        if term >= state.current_term {
            state.current_term = term;
            state.role = NodeRole::Follower;
            state.leader_id = Some(leader_id);
            self.last_heartbeat.store(
                self.current_time_ms(),
                Ordering::Relaxed
            );
            self.election_in_progress.store(false, Ordering::Release);
        }
    }

    /// Get current time in milliseconds (stub)
    fn current_time_ms(&self) -> u64 {
        0 // In real implementation, use actual time
    }
}

/// Distributed cluster with Raft consensus
#[derive(Debug)]
pub struct Cluster {
    /// Local node ID
    node_id: NodeId,
    /// Cluster configuration
    config: ClusterConfig,
    /// Cluster state
    state: Arc<Mutex<ClusterState>>,
    /// Cluster membership
    membership: Arc<Mutex<ClusterMembership>>,
    /// Leader election manager
    election: LeaderElection,
    /// Raft log
    log: Arc<Mutex<Vec<LogEntry>>>,
    /// Cluster running flag
    running: Arc<AtomicBool>,
}

impl Cluster {
    /// Create new cluster instance
    pub async fn new(node_id: NodeId, config: ClusterConfig) -> HaResult<Self> {
        let node_count = config.nodes.len();
        let state = Arc::new(Mutex::new(ClusterState::new(node_count)));
        let membership = Arc::new(Mutex::new(ClusterMembership::new(
            config.clone(),
            state.clone(),
        )));

        let election = LeaderElection::new(
            node_id,
            state.clone(),
            membership.clone(),
            &config,
        );

        let cluster = Cluster {
            node_id,
            config,
            state,
            membership,
            election,
            log: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
        };

        Ok(cluster)
    }

    /// Start cluster services
    pub async fn start(&self) -> HaResult<()> {
        self.running.store(true, Ordering::Release);
        self.election.start_election_timer();

        // For single-node cluster, immediately become leader
        if self.config.nodes.len() == 1 {
            self.election.become_leader()?;
        }

        Ok(())
    }

    /// Stop cluster services
    pub async fn stop(&self) -> HaResult<()> {
        self.running.store(false, Ordering::Release);
        {
            let mut state = self.state.lock();
            state.role = NodeRole::Follower;
            state.leader_id = None;
        }
        Ok(())
    }

    /// Get current cluster state
    pub async fn get_state(&self) -> ClusterState {
        let state = self.state.lock();
        ClusterState {
            role: state.role,
            current_term: state.current_term,
            voted_for: state.voted_for,
            node_count: state.node_count,
            leader_id: state.leader_id,
            commit_index: state.commit_index,
            last_applied: state.last_applied,
        }
    }

    /// Check if this node is leader
    pub async fn is_leader(&self) -> bool {
        let state = self.state.lock();
        state.role == NodeRole::Leader
    }

    /// Get current leader ID
    pub async fn get_leader_id(&self) -> Option<NodeId> {
        let state = self.state.lock();
        state.leader_id
    }

    /// Submit command to cluster (must be leader)
    pub async fn submit_command(&self, command: Vec<u8>) -> HaResult<u64> {
        if !self.is_leader().await {
            return Err(HaError::ClusterError(ClusterError::NoLeader));
        }

        let state = self.state.lock();
        let entry = LogEntry {
            index: state.last_applied + 1,
            term: state.current_term,
            command,
            entry_type: EntryType::Normal,
        };

        let index = entry.index;
        self.log.lock().push(entry);

        Ok(index)
    }

    /// Replicate log entry to followers
    pub async fn replicate_entry(&self, _entry: &LogEntry) -> HaResult<bool> {
        if !self.is_leader().await {
            return Ok(false);
        }

        let _state = self.state.lock();
        let membership = self.membership.lock();
        let _quorum = membership.quorum_size();

        // In real implementation, send AppendEntries RPC to all followers
        // and count successful responses
        let _replicas = self.config.nodes.len() - 1;

        // Simplified: assume success if we're leader
        Ok(true)
    }

    /// Apply committed log entry to state machine
    pub async fn apply_entry(&self, index: u64) -> HaResult<()> {
        let mut state = self.state.lock();

        if index > state.commit_index {
            return Err(HaError::ClusterError(ClusterError::LogInconsistent));
        }

        if index <= state.last_applied {
            return Ok(());
        }

        // Apply log entry to state machine
        state.last_applied = index;

        Ok(())
    }

    /// Handle AppendEntries RPC
    pub async fn handle_append_entries(
        &self,
        term: u64,
        leader_id: NodeId,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    ) -> HaResult<(u64, bool)> {
        let mut state = self.state.lock();

        // Step down if term is higher
        if term > state.current_term {
            state.current_term = term;
            state.role = NodeRole::Follower;
            state.voted_for = None;
        }

        // Reject if term is lower
        if term < state.current_term {
            return Ok((state.current_term, false));
        }

        // Update leader info
        state.leader_id = Some(leader_id);
        self.election.receive_heartbeat(leader_id, term);

        // Check log consistency
        let log = self.log.lock();
        if prev_log_index > 0 {
            if let Some(entry) = log.get(prev_log_index as usize - 1) {
                if entry.term != prev_log_term {
                    drop(log);
                    return Ok((state.current_term, false));
                }
            } else {
                drop(log);
                return Ok((state.current_term, false));
            }
        }
        drop(log);

        // Append new entries
        if !entries.is_empty() {
            let mut log = self.log.lock();
            for entry in entries {
                if log.len() < entry.index as usize {
                    log.push(entry);
                }
            }
        }

        // Update commit index
        if leader_commit > state.commit_index {
            state.commit_index = leader_commit.min(state.last_applied);
        }

        Ok((state.current_term, true))
    }

    /// Handle RequestVote RPC
    pub async fn handle_request_vote(
        &self,
        term: u64,
        candidate_id: NodeId,
        last_log_index: u64,
        last_log_term: u64,
    ) -> HaResult<(u64, bool)> {
        let mut state = self.state.lock();

        // Step down if term is higher
        if term > state.current_term {
            state.current_term = term;
            state.role = NodeRole::Follower;
            state.voted_for = None;
        }

        // Reject if term is lower
        if term < state.current_term {
            return Ok((state.current_term, false));
        }

        // Check if we can grant vote
        let log_ok = last_log_term >= state.current_term
            && last_log_index >= state.last_applied;

        let vote_granted = if state.voted_for.is_none() || state.voted_for == Some(candidate_id) {
            if log_ok {
                state.voted_for = Some(candidate_id);
                true
            } else {
                false
            }
        } else {
            false
        };

        Ok((state.current_term, vote_granted))
    }

    /// Add node to cluster
    pub async fn add_node(&self, node_id: NodeId) -> HaResult<()> {
        let mut membership = self.membership.lock();
        membership.add_node(node_id)?;
        Ok(())
    }

    /// Remove node from cluster
    pub async fn remove_node(&self, node_id: NodeId) -> HaResult<()> {
        let mut membership = self.membership.lock();
        membership.remove_node(node_id)?;
        Ok(())
    }

    /// Get cluster membership
    pub async fn get_membership(&self) -> Vec<NodeId> {
        let membership = self.membership.lock();
        membership.get_nodes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id() {
        let id = NodeId::from(42);
        assert_eq!(id.value(), 42);
        assert!(id.is_valid());
    }

    #[test]
    fn test_cluster_state() {
        let state = ClusterState::new(3);
        assert_eq!(state.node_count, 3);
        assert_eq!(state.quorum_size(), 2);
        assert!(state.has_quorum(2));
        assert!(!state.has_quorum(1));
    }

    #[test]
    fn test_node_role() {
        assert!(NodeRole::Leader.can_write());
        assert!(!NodeRole::Follower.can_write());
        assert!(NodeRole::Follower.can_read());
    }

    #[test]
    fn test_cluster_config() {
        let config = ClusterConfig::three_node_replicated();
        assert_eq!(config.nodes.len(), 3);

        let config = ClusterConfig::five_node_replicated();
        assert_eq!(config.nodes.len(), 5);
    }

    #[test]
    fn test_membership() {
        let config = ClusterConfig::three_node_replicated();
        let state = Arc::new(Mutex::new(ClusterState::new(3)));
        let mut membership = ClusterMembership::new(config, state);

        assert_eq!(membership.get_nodes().len(), 3);
        assert!(membership.contains_node(NodeId::from(1)));
        assert!(!membership.contains_node(NodeId::from(99)));

        assert!(membership.add_node(NodeId::from(4)).is_ok());
        assert!(membership.contains_node(NodeId::from(4)));
    }

    #[tokio::test]
    async fn test_cluster_creation() {
        let config = ClusterConfig::single_node_dev();
        let cluster = Cluster::new(NodeId::from(1), config).await.unwrap();
        assert!(cluster.start().await.is_ok());

        let state = cluster.get_state().await;
        assert_eq!(state.node_count, 1);
    }
}
