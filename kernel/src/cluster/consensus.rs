//! Raft Consensus Implementation
//!
//! This module implements the Raft consensus algorithm for distributed state machine replication.
//! It provides leader election, log replication, safety guarantees, and cluster membership changes.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::sync::{Mutex, RwLock};

/// Unique identifier for a node in the cluster
pub type NodeId = u64;

/// Term number for leader election
pub type Term = u64;

/// Log index
pub type Index = u64;

/// Raft log entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Index in the log
    pub index: Index,
    /// Term when entry was received by leader
    pub term: Term,
    /// Command to apply to state machine
    pub command: Vec<u8>,
    /// Entry type
    pub entry_type: EntryType,
}

/// Type of log entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Normal state machine command
    Normal,
    /// Cluster membership change
    Membership,
    /// Configuration change
    Config,
    /// No-op (used for leadership establishment)
    NoOp,
}

/// Snapshot of the state machine
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Last included index
    pub last_included_index: Index,
    /// Last included term
    pub last_included_term: Term,
    /// State machine data
    pub data: Vec<u8>,
    /// Configuration at snapshot time
    pub configuration: Configuration,
}

/// Cluster configuration
#[derive(Debug, Clone)]
pub struct Configuration {
    /// Current cluster members
    pub voters: BTreeSet<NodeId>,
    /// Nodes being added (joint consensus)
    pub new_voters: Option<BTreeSet<NodeId>>,
}

/// Request vote RPC arguments
#[derive(Debug, Clone)]
pub struct RequestVoteArgs {
    /// Candidate's term
    pub term: Term,
    /// Candidate requesting vote
    pub candidate_id: NodeId,
    /// Index of candidate's last log entry
    pub last_log_index: Index,
    /// Term of candidate's last log entry
    pub last_log_term: Term,
}

/// Request vote RPC reply
#[derive(Debug, Clone)]
pub struct RequestVoteReply {
    /// Current term (for candidate to update itself)
    pub term: Term,
    /// True means candidate received vote
    pub vote_granted: bool,
}

/// Append entries RPC arguments
#[derive(Debug, Clone)]
pub struct AppendEntriesArgs {
    /// Leader's term
    pub term: Term,
    /// Leader's index
    pub leader_id: NodeId,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: Index,
    /// Term of prev_log_index entry
    pub prev_log_term: Term,
    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntry>,
    /// Leader's commit_index
    pub leader_commit: Index,
}

/// Append entries RPC reply
#[derive(Debug, Clone)]
pub struct AppendEntriesReply {
    /// Current term (for leader to update itself)
    pub term: Term,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    /// Conflicting term (if any)
    pub conflict_term: Option<Term>,
    /// First index of conflicting term
    pub conflict_index: Option<Index>,
}

/// Install snapshot RPC arguments
#[derive(Debug, Clone)]
pub struct InstallSnapshotArgs {
    /// Leader's term
    pub term: Term,
    /// Leader's index
    pub leader_id: NodeId,
    /// Snapshot replaces all log entries up to this index
    pub last_included_index: Index,
    /// Term of last_included_index
    pub last_included_term: Term,
    /// Snapshot data
    pub data: Vec<u8>,
    /// Cluster configuration
    pub configuration: Configuration,
}

/// Install snapshot RPC reply
#[derive(Debug, Clone)]
pub struct InstallSnapshotReply {
    /// Current term
    pub term: Term,
    /// True if snapshot was installed
    pub success: bool,
}

/// Server role in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerRole {
    /// Follower state
    Follower,
    /// Candidate state
    Candidate,
    /// Leader state
    Leader,
}

/// Persistent state for each server
#[derive(Debug, Clone)]
struct PersistentState {
    /// Latest term server has seen
    current_term: Term,
    /// Candidate_id that received vote in current term
    voted_for: Option<NodeId>,
    /// Log entries
    log: Vec<LogEntry>,
}

/// Volatile state for all servers
#[derive(Debug, Clone)]
struct VolatileState {
    /// Index of highest log entry known to be committed
    commit_index: Index,
    /// Index of highest log entry applied to state machine
    last_applied: Index,
}

/// Volatile state for leaders
#[derive(Debug, Clone)]
struct LeaderState {
    /// For each server, index of next log entry to send
    next_index: BTreeMap<NodeId, Index>,
    /// For each server, index of highest log entry known to be replicated
    match_index: BTreeMap<NodeId, Index>,
}

/// Statistics for the consensus module
#[derive(Debug, Default)]
pub struct ConsensusStatistics {
    /// Total entries proposed
    pub total_proposals: AtomicU64,
    /// Successful entries committed
    pub committed_entries: AtomicU64,
    /// Leader elections
    pub elections: AtomicU64,
    /// Number of terms
    pub terms: AtomicU64,
    /// Append entries RPCs sent
    pub append_entries_sent: AtomicU64,
    /// Append entries RPCs received
    pub append_entries_received: AtomicU64,
    /// Request vote RPCs sent
    pub request_vote_sent: AtomicU64,
    /// Request vote RPCs received
    pub request_vote_received: AtomicU64,
    /// Snapshots created
    pub snapshots_created: AtomicU64,
    /// Snapshots installed
    pub snapshots_installed: AtomicU64,
}

/// Errors that can occur in the consensus module
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusError {
    /// Not the leader
    NotLeader(NodeId),
    /// Entry does not match current term
    TermMismatch {
        expected: Term,
        actual: Term,
    },
    /// Log inconsistency detected
    LogInconsistent {
        expected_index: Index,
        actual_index: Index,
    },
    /// Quorum not reached
    QuorumNotReached {
        votes_for: usize,
        votes_needed: usize,
    },
    /// Node not in configuration
    NodeNotInConfiguration(NodeId),
    /// Snapshot installation failed
    SnapshotFailed(String),
    /// Leadership lost during operation
    LeadershipLost,
    /// Timeout
    Timeout,
    /// Communication failure
    CommunicationFailure(String),
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConsensusError::NotLeader(leader_id) => {
                write!(f, "Not the leader (current leader: {})", leader_id)
            }
            ConsensusError::TermMismatch { expected, actual } => {
                write!(f, "Term mismatch: expected {}, got {}", expected, actual)
            }
            ConsensusError::LogInconsistent {
                expected_index,
                actual_index,
            } => {
                write!(
                    f,
                    "Log inconsistency: expected index {}, got {}",
                    expected_index, actual_index
                )
            }
            ConsensusError::QuorumNotReached {
                votes_for,
                votes_needed,
            } => {
                write!(
                    f,
                    "Quorum not reached: {}/{} votes",
                    votes_for, votes_needed
                )
            }
            ConsensusError::NodeNotInConfiguration(node_id) => {
                write!(f, "Node {} not in cluster configuration", node_id)
            }
            ConsensusError::SnapshotFailed(msg) => {
                write!(f, "Snapshot failed: {}", msg)
            }
            ConsensusError::LeadershipLost => {
                write!(f, "Leadership lost during operation")
            }
            ConsensusError::Timeout => write!(f, "Operation timeout"),
            ConsensusError::CommunicationFailure(msg) => {
                write!(f, "Communication failure: {}", msg)
            }
            ConsensusError::InternalError(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

/// Raft consensus implementation
pub struct RaftConsensus {
    /// This node's ID
    node_id: NodeId,
    /// Current cluster configuration
    configuration: RwLock<Configuration>,
    /// Persistent state
    persistent_state: Mutex<PersistentState>,
    /// Volatile state
    volatile_state: Mutex<VolatileState>,
    /// Leader state (only valid if role == Leader)
    leader_state: Mutex<Option<LeaderState>>,
    /// Current role
    role: Mutex<ServerRole>,
    /// Current leader
    current_leader: Mutex<Option<NodeId>>,
    /// Snapshot
    snapshot: Mutex<Option<Snapshot>>,
    /// Last heartbeat time
    last_heartbeat: Mutex<Option<Duration>>,
    /// Election timeout
    election_timeout: Duration,
    /// Heartbeat interval
    heartbeat_interval: Duration,
    /// Statistics
    stats: Arc<ConsensusStatistics>,
    /// Applied log entries (for state machine)
    applied_entries: RwLock<Vec<LogEntry>>,
}

impl RaftConsensus {
    /// Create a new Raft consensus instance
    pub fn new(
        node_id: NodeId,
        initial_config: Configuration,
        election_timeout: Duration,
        heartbeat_interval: Duration,
    ) -> Self {
        Self {
            node_id,
            configuration: RwLock::new(initial_config),
            persistent_state: Mutex::new(PersistentState {
                current_term: 0,
                voted_for: None,
                log: Vec::new(),
            }),
            volatile_state: Mutex::new(VolatileState {
                commit_index: 0,
                last_applied: 0,
            }),
            leader_state: Mutex::new(None),
            role: Mutex::new(ServerRole::Follower),
            current_leader: Mutex::new(None),
            snapshot: Mutex::new(None),
            last_heartbeat: Mutex::new(None),
            election_timeout,
            heartbeat_interval,
            stats: Arc::new(ConsensusStatistics::default()),
            applied_entries: RwLock::new(Vec::new()),
        }
    }

    /// Propose a new entry to the log
    pub fn propose(&self, entry: LogEntry) -> Result<Index, ConsensusError> {
        // Check if we are the leader
        let role = *self.role.lock();
        if role != ServerRole::Leader {
            let leader = *self.current_leader.lock();
            return Err(ConsensusError::NotLeader(leader.unwrap_or(0)));
        }

        self.stats.total_proposals.fetch_add(1, Ordering::Relaxed);

        // Add entry to log
        let index = {
            let mut state = self.persistent_state.lock();
            let index = state.log.len() as Index + 1;
            state.log.push(entry);
            state.current_term;
            index
        };

        Ok(index)
    }

    /// Read a log entry by index
    pub fn read_index(&self, index: Index) -> Result<Option<LogEntry>, ConsensusError> {
        let state = self.persistent_state.lock();
        if index == 0 || index > state.log.len() as Index {
            return Ok(None);
        }
        Ok(state.log.get((index - 1) as usize).cloned())
    }

    /// Request vote RPC
    pub fn request_vote(&self, args: RequestVoteArgs) -> Result<RequestVoteReply, ConsensusError> {
        self.stats.request_vote_received.fetch_add(1, Ordering::Relaxed);

        let mut reply = RequestVoteReply {
            term: args.term,
            vote_granted: false,
        };

        let mut state = self.persistent_state.lock();

        // If term < currentTerm, reject
        if args.term < state.current_term {
            reply.term = state.current_term;
            return Ok(reply);
        }

        // If term > currentTerm, update term and convert to follower
        if args.term > state.current_term {
            state.current_term = args.term;
            state.voted_for = None;
            *self.role.lock() = ServerRole::Follower;
            *self.current_leader.lock() = None;
            self.stats.terms.fetch_add(1, Ordering::Relaxed);
        }

        // Check if we've already voted
        if state.voted_for.is_some() && state.voted_for != Some(args.candidate_id) {
            return Ok(reply);
        }

        // Check if candidate's log is at least as up-to-date as ours
        let last_log_index = state.log.len() as Index;
        let last_log_term = state.log.last().map(|e| e.term).unwrap_or(0);

        let log_ok = if args.last_log_term > last_log_term {
            true
        } else if args.last_log_term == last_log_term {
            args.last_log_index >= last_log_index
        } else {
            false
        };

        if log_ok {
            state.voted_for = Some(args.candidate_id);
            reply.vote_granted = true;
        }

        Ok(reply)
    }

    /// Append entries RPC
    pub fn append_entries(
        &self,
        args: AppendEntriesArgs,
    ) -> Result<AppendEntriesReply, ConsensusError> {
        self.stats.append_entries_received.fetch_add(1, Ordering::Relaxed);

        let mut reply = AppendEntriesReply {
            term: args.term,
            success: false,
            conflict_term: None,
            conflict_index: None,
        };

        let mut state = self.persistent_state.lock();

        // Update term if needed
        if args.term < state.current_term {
            reply.term = state.current_term;
            return Ok(reply);
        }

        if args.term > state.current_term {
            state.current_term = args.term;
            state.voted_for = None;
            *self.role.lock() = ServerRole::Follower;
        }

        // Update leader and heartbeat
        *self.current_leader.lock() = Some(args.leader_id);
        *self.last_heartbeat.lock() = Some(Duration::from_secs(0)); // Dummy duration

        // Check log consistency
        if args.prev_log_index > 0 {
            if state.log.len() < args.prev_log_index as usize {
                return Ok(reply);
            }

            let prev_entry = &state.log[(args.prev_log_index - 1) as usize];
            if prev_entry.term != args.prev_log_term {
                reply.conflict_term = Some(prev_entry.term);
                // Find first index with this term
                reply.conflict_index = state
                    .log
                    .iter()
                    .position(|e| e.term == prev_entry.term)
                    .map(|i| i as Index + 1);
                return Ok(reply);
            }
        }

        // Append new entries
        if !args.entries.is_empty() {
            let mut insert_index = args.prev_log_index as usize;

            for entry in &args.entries {
                if insert_index < state.log.len() {
                    if state.log[insert_index].term != entry.term {
                        // Conflict: delete from this point
                        state.log.truncate(insert_index);
                        state.log.push(entry.clone());
                    }
                } else {
                    state.log.push(entry.clone());
                }
                insert_index += 1;
            }
        }

        reply.success = true;

        // Update commit index
        let mut volatile = self.volatile_state.lock();
        if args.leader_commit > volatile.commit_index {
            volatile.commit_index = core::cmp::min(args.leader_commit, state.log.len() as Index);
        }

        Ok(reply)
    }

    /// Install snapshot RPC
    pub fn install_snapshot(
        &self,
        args: InstallSnapshotArgs,
    ) -> Result<InstallSnapshotReply, ConsensusError> {
        let mut reply = InstallSnapshotReply {
            term: args.term,
            success: false,
        };

        let mut state = self.persistent_state.lock();

        // Update term if needed
        if args.term < state.current_term {
            reply.term = state.current_term;
            return Ok(reply);
        }

        if args.term > state.current_term {
            state.current_term = args.term;
            state.voted_for = None;
            *self.role.lock() = ServerRole::Follower;
        }

        // Update leader
        *self.current_leader.lock() = Some(args.leader_id);

        // Create snapshot
        let snapshot = Snapshot {
            last_included_index: args.last_included_index,
            last_included_term: args.last_included_term,
            data: args.data,
            configuration: args.configuration,
        };

        // Truncate log
        if state.log.len() > snapshot.last_included_index as usize {
            state.log.truncate(snapshot.last_included_index as usize);
        }

        // Discard all log entries up to snapshot
        state.log = state
            .log
            .split_off(snapshot.last_included_index.saturating_sub(1) as usize);

        *self.snapshot.lock() = Some(snapshot);

        reply.success = true;
        self.stats.snapshots_installed.fetch_add(1, Ordering::Relaxed);

        Ok(reply)
    }

    /// Start a new election
    pub fn start_election(&self) -> Result<(), ConsensusError> {
        let mut state = self.persistent_state.lock();

        // Increment term
        state.current_term += 1;
        state.voted_for = Some(self.node_id);

        // Become candidate
        *self.role.lock() = ServerRole::Candidate;
        *self.current_leader.lock() = None;
        self.stats.elections.fetch_add(1, Ordering::Relaxed);
        self.stats.terms.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Become leader after winning election
    pub fn become_leader(&self) -> Result<(), ConsensusError> {
        let config = self.configuration.read();
        let mut leader_state = LeaderState {
            next_index: BTreeMap::new(),
            match_index: BTreeMap::new(),
        };

        let last_log_index = {
            let state = self.persistent_state.lock();
            state.log.len() as Index
        };

        // Initialize leader state for all followers
        for &node_id in &config.voters {
            if node_id != self.node_id {
                leader_state.next_index.insert(node_id, last_log_index + 1);
                leader_state.match_index.insert(node_id, 0);
            }
        }

        *self.leader_state.lock() = Some(leader_state);
        *self.role.lock() = ServerRole::Leader;
        *self.current_leader.lock() = Some(self.node_id);

        Ok(())
    }

    /// Add a new voter to the cluster
    pub fn add_voter(&self, node_id: NodeId) -> Result<(), ConsensusError> {
        // Check if we are the leader
        let role = *self.role.lock();
        if role != ServerRole::Leader {
            return Err(ConsensusError::NotLeader(
                self.current_leader.lock().unwrap_or(0),
            ));
        }

        let mut config = self.configuration.write();

        // Start joint consensus
        if config.new_voters.is_none() {
            let new_voters = config.voters.clone();
            config.new_voters = Some(new_voters);
        }

        // Add new node to new_voters
        if let Some(ref mut new_voters) = config.new_voters {
            new_voters.insert(node_id);
        }

        // Propose configuration change
        let entry = LogEntry {
            index: 0, // Will be set when proposed
            term: self.persistent_state.lock().current_term,
            command: Vec::new(), // Configuration serialization would go here
            entry_type: EntryType::Membership,
        };

        let _index = self.propose(entry)?;

        Ok(())
    }

    /// Remove a voter from the cluster
    pub fn remove_voter(&self, node_id: NodeId) -> Result<(), ConsensusError> {
        let role = *self.role.lock();
        if role != ServerRole::Leader {
            return Err(ConsensusError::NotLeader(
                self.current_leader.lock().unwrap_or(0),
            ));
        }

        let mut config = self.configuration.write();

        // Start joint consensus if needed
        if config.new_voters.is_none() {
            let new_voters = config.voters.clone();
            config.new_voters = Some(new_voters);
        }

        // Remove from both old and new
        config.voters.remove(&node_id);
        if let Some(ref mut new_voters) = config.new_voters {
            new_voters.remove(&node_id);
        }

        // Propose configuration change
        let entry = LogEntry {
            index: 0,
            term: self.persistent_state.lock().current_term,
            command: Vec::new(),
            entry_type: EntryType::Membership,
        };

        let _index = self.propose(entry)?;

        Ok(())
    }

    /// Calculate quorum size
    pub fn calculate_quorum(&self) -> usize {
        let config = self.configuration.read();
        let size = if let Some(ref new_voters) = config.new_voters {
            // Joint consensus: need majority of both old and new
            let old_quorum = (config.voters.len() / 2) + 1;
            let new_quorum = (new_voters.len() / 2) + 1;
            core::cmp::max(old_quorum, new_quorum)
        } else {
            (config.voters.len() / 2) + 1
        };
        size
    }

    /// Check if we have reached quorum
    pub fn has_quorum(&self, votes: &BTreeSet<NodeId>) -> bool {
        let quorum = self.calculate_quorum();
        votes.len() >= quorum
    }

    /// Update commit index based on match_index
    pub fn update_commit_index(&self) -> Result<(), ConsensusError> {
        let role = *self.role.lock();
        if role != ServerRole::Leader {
            return Ok(());
        }

        let leader_state = self.leader_state.lock();
        let leader_state = leader_state.as_ref().ok_or(ConsensusError::NotLeader(0))?;

        let config = self.configuration.read();
        let mut match_indices: Vec<_> = leader_state.match_index.values().copied().collect();
        match_indices.push(self.persistent_state.lock().log.len() as Index);
        match_indices.sort_by(|a, b| b.cmp(a)); // Descending

        let quorum = self.calculate_quorum();
        if match_indices.len() >= quorum {
            let candidate_index = match_indices[quorum - 1];

            let mut volatile = self.volatile_state.lock();
            if candidate_index > volatile.commit_index {
                // Check if entry at candidate_index is from current term
                let state = self.persistent_state.lock();
                if let Some(entry) = state.log.get((candidate_index - 1) as usize) {
                    if entry.term == state.current_term {
                        volatile.commit_index = candidate_index;
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply committed entries to state machine
    pub fn apply_entries(&self) -> Result<usize, ConsensusError> {
        let mut applied = 0;

        loop {
            let commit_index = {
                let volatile = self.volatile_state.lock();
                volatile.commit_index
            };

            let last_applied = {
                let volatile = self.volatile_state.lock();
                volatile.last_applied
            };

            if last_applied >= commit_index {
                break;
            }

            let index = last_applied + 1;
            let entry = {
                let state = self.persistent_state.lock();
                state.log.get((index - 1) as usize).cloned()
            };

            if let Some(entry) = entry {
                // Apply entry to state machine
                {
                    let mut applied_entries = self.applied_entries.write();
                    applied_entries.push(entry.clone());
                }

                // Update last_applied
                {
                    let mut volatile = self.volatile_state.lock();
                    volatile.last_applied = index;
                }

                applied += 1;
                self.stats.committed_entries.fetch_add(1, Ordering::Relaxed);
            } else {
                break;
            }
        }

        Ok(applied)
    }

    /// Create a snapshot
    pub fn create_snapshot(&self) -> Result<Snapshot, ConsensusError> {
        let volatile = self.volatile_state.lock();
        let state = self.persistent_state.lock();

        let snapshot = Snapshot {
            last_included_index: volatile.last_applied,
            last_included_term: state
                .log
                .get(volatile.last_applied.saturating_sub(1) as usize)
                .map(|e| e.term)
                .unwrap_or(0),
            data: Vec::new(), // In real implementation, serialize state machine
            configuration: self.configuration.read().clone(),
        };

        self.stats.snapshots_created.fetch_add(1, Ordering::Relaxed);

        Ok(snapshot)
    }

    /// Compact log using snapshot
    pub fn compact_log(&self) -> Result<(), ConsensusError> {
        let snapshot = self.create_snapshot()?;

        // Remove log entries up to snapshot
        {
            let mut state = self.persistent_state.lock();
            if state.log.len() > snapshot.last_included_index as usize {
                state.log = state
                    .log
                    .split_off(snapshot.last_included_index as usize);
            }
        }

        *self.snapshot.lock() = Some(snapshot);

        Ok(())
    }

    /// Get current term
    pub fn current_term(&self) -> Term {
        self.persistent_state.lock().current_term
    }

    /// Get current role
    pub fn current_role(&self) -> ServerRole {
        *self.role.lock()
    }

    /// Get current leader
    pub fn current_leader(&self) -> Option<NodeId> {
        *self.current_leader.lock()
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &ConsensusStatistics {
        &self.stats
    }

    /// Step down to follower
    pub fn step_down(&self, new_term: Term) -> Result<(), ConsensusError> {
        let mut state = self.persistent_state.lock();
        if new_term > state.current_term {
            state.current_term = new_term;
            state.voted_for = None;
            *self.role.lock() = ServerRole::Follower;
            *self.leader_state.lock() = None;
            *self.current_leader.lock() = None;
        }
        Ok(())
    }

    /// Send heartbeats to all followers
    pub fn send_heartbeats(&self) -> Result<(), ConsensusError> {
        let role = *self.role.lock();
        if role != ServerRole::Leader {
            return Ok(());
        }

        let config = self.configuration.read();
        let term = self.persistent_state.lock().current_term;

        for &node_id in &config.voters {
            if node_id != self.node_id {
                let args = AppendEntriesArgs {
                    term,
                    leader_id: self.node_id,
                    prev_log_index: 0,
                    prev_log_term: 0,
                    entries: Vec::new(),
                    leader_commit: self.volatile_state.lock().commit_index,
                };

                // In real implementation, send RPC
                let _ = args;
                self.stats.append_entries_sent.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Check if election timeout has expired
    pub fn election_timeout_expired(&self, elapsed: Duration) -> bool {
        elapsed > self.election_timeout
    }

    /// Reset election timeout
    pub fn reset_election_timeout(&self) {
        *self.last_heartbeat.lock() = Some(Duration::from_secs(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let mut config = Configuration {
            voters: BTreeSet::new(),
            new_voters: None,
        };
        config.voters.insert(1);
        config.voters.insert(2);
        config.voters.insert(3);

        let raft = RaftConsensus::new(
            1,
            config,
            Duration::from_millis(150),
            Duration::from_millis(50),
        );

        assert_eq!(raft.current_role(), ServerRole::Follower);
        assert_eq!(raft.current_term(), 0);
    }

    #[test]
    fn test_quorum_calculation() {
        let mut config = Configuration {
            voters: BTreeSet::new(),
            new_voters: None,
        };
        config.voters.insert(1);
        config.voters.insert(2);
        config.voters.insert(3);
        config.voters.insert(4);
        config.voters.insert(5);

        let raft = RaftConsensus::new(
            1,
            config,
            Duration::from_millis(150),
            Duration::from_millis(50),
        );

        assert_eq!(raft.calculate_quorum(), 3);
    }
}
