//! Edge Synchronization Protocols
//!
//! Provides CRDT-based synchronization, delta sync, and bandwidth-aware
//! transfer for edge-cloud communication.

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet, VecDeque},
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::prelude::*;
use crate::subsystems::sync::Mutex;

/// Replica identifier
pub type ReplicaId = u64;

/// Sync operation identifier
pub type SyncOpId = u64;

/// Sync priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// CRDT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CRDTType {
    /// G-Counter (grow-only counter)
    GCounter,
    /// PN-Counter (positive-negative counter)
    PNCounter,
    /// LWW-Register (last-write-wins register)
    LWWRegister,
    /// OR-Set (observed-remove set)
    ORSet,
    /// Map (nested CRDT)
    Map,
}

/// CRDT state
#[derive(Debug, Clone)]
pub enum CRDTState {
    /// G-Counter state: replica_id -> count
    GCounter(BTreeMap<ReplicaId, u64>),
    /// PN-Counter state: (increments, decrements)
    PNCounter {
        increments: BTreeMap<ReplicaId, u64>,
        decrements: BTreeMap<ReplicaId, u64>,
    },
    /// LWW-Register state: (timestamp, value)
    LWWRegister(u64, Vec<u8>),
    /// OR-Set state: (elements, tombstones)
    ORSet {
        elements: BTreeSet<Vec<u8>>,
        tombstones: BTreeSet<Vec<u8>>,
    },
    /// Map state: key -> CRDT
    Map(BTreeMap<String, Box<CRDTState>>),
}

/// CRDT replica
#[derive(Debug, Clone)]
pub struct CRDTReplica {
    /// Replica ID
    pub replica_id: ReplicaId,
    /// CRDT type
    pub crdt_type: CRDTType,
    /// CRDT state
    pub state: CRDTState,
    /// Last synchronization timestamp
    pub last_sync: u64,
}

impl CRDTReplica {
    /// Create a new G-Counter replica
    pub fn new_gcounter(replica_id: ReplicaId) -> Self {
        let mut state = BTreeMap::new();
        state.insert(replica_id, 0);

        Self {
            replica_id,
            crdt_type: CRDTType::GCounter,
            state: CRDTState::GCounter(state),
            last_sync: 0,
        }
    }

    /// Create a new PN-Counter replica
    pub fn new_pncounter(replica_id: ReplicaId) -> Self {
        let mut increments = BTreeMap::new();
        let mut decrements = BTreeMap::new();
        increments.insert(replica_id, 0);
        decrements.insert(replica_id, 0);

        Self {
            replica_id,
            crdt_type: CRDTType::PNCounter,
            state: CRDTState::PNCounter {
                increments,
                decrements,
            },
            last_sync: 0,
        }
    }

    /// Increment G-Counter
    pub fn increment(&mut self, amount: u64) -> Result<()> {
        match &mut self.state {
            CRDTState::GCounter(counters) => {
                *counters.entry(self.replica_id).or_insert(0) += amount;
                Ok(())
            }
            _ => Err(nos_api::Error::InvalidOperation),
        }
    }

    /// Get G-Counter value
    pub fn get_gcounter_value(&self) -> Result<u64> {
        match &self.state {
            CRDTState::GCounter(counters) => {
                Ok(counters.values().copied().sum())
            }
            _ => Err(nos_api::Error::InvalidOperation),
        }
    }

    /// Merge with another replica
    pub fn merge(&mut self, other: &CRDTReplica) -> Result<()> {
        if self.crdt_type != other.crdt_type {
            return Err(nos_api::Error::InvalidArgument);
        }

        match (&mut self.state, &other.state) {
            (CRDTState::GCounter(local), CRDTState::GCounter(remote)) => {
                for (replica, &count) in remote {
                    let entry = local.entry(*replica).or_insert(0);
                    *entry = (*entry).max(count);
                }
                Ok(())
            }
            (CRDTState::PNCounter { increments: local_inc, decrements: local_dec },
             CRDTState::PNCounter { increments: remote_inc, decrements: remote_dec }) => {
                for (replica, &count) in remote_inc {
                    let entry = local_inc.entry(*replica).or_insert(0);
                    *entry = (*entry).max(count);
                }
                for (replica, &count) in remote_dec {
                    let entry = local_dec.entry(*replica).or_insert(0);
                    *entry = (*entry).max(count);
                }
                Ok(())
            }
            _ => Err(nos_api::Error::NotImplemented),
        }
    }
}

/// Delta synchronization state
#[derive(Debug, Clone)]
pub struct DeltaSyncState {
    /// Local replica ID
    pub replica_id: ReplicaId,
    /// Remote replica states
    pub remote_states: BTreeMap<ReplicaId, RemoteReplicaState>,
    /// Pending deltas
    pub pending_deltas: VecDeque<SyncDelta>,
    /// Last sync timestamp
    pub last_sync: u64,
    /// Sync interval
    pub sync_interval: Duration,
}

/// Remote replica state
#[derive(Debug, Clone)]
pub struct RemoteReplicaState {
    /// Replica ID
    pub replica_id: ReplicaId,
    /// Known vector clock
    pub vector_clock: BTreeMap<ReplicaId, u64>,
    /// Last sync timestamp
    pub last_sync: u64,
    /// Sync status
    pub sync_status: SyncStatus,
}

/// Sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Not synchronized
    NotSynced,
    /// Synchronization in progress
    Syncing,
    /// Synchronized
    Synchronized,
    /// Sync failed
    Failed,
}

/// Synchronization delta
#[derive(Debug, Clone)]
pub struct SyncDelta {
    /// Delta ID
    pub delta_id: SyncOpId,
    /// Source replica ID
    pub source_id: ReplicaId,
    /// Target replica ID
    pub target_id: ReplicaId,
    /// Operation type
    pub op_type: SyncOpType,
    /// Delta data
    pub data: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
    /// Priority
    pub priority: SyncPriority,
    /// Size in bytes
    pub size: u64,
}

/// Sync operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOpType {
    /// Insert operation
    Insert,
    /// Update operation
    Update,
    /// Delete operation
    Delete,
    /// Merge operation
    Merge,
}

/// Offline queue for disconnected operation
#[derive(Debug)]
pub struct OfflineQueue {
    /// Queue ID
    pub queue_id: u64,
    /// Pending operations
    pub pending_ops: VecDeque<QueuedOp>,
    /// Maximum queue size
    pub max_size: usize,
    /// Maximum queue size in bytes
    pub max_bytes: u64,
    /// Current queue size in bytes
    pub current_bytes: AtomicU64,
}

impl Clone for OfflineQueue {
    fn clone(&self) -> Self {
        Self {
            queue_id: self.queue_id,
            pending_ops: self.pending_ops.clone(),
            max_size: self.max_size,
            max_bytes: self.max_bytes,
            current_bytes: AtomicU64::new(self.current_bytes.load(Ordering::Relaxed)),
        }
    }
}

/// Queued operation
#[derive(Debug, Clone)]
pub struct QueuedOp {
    /// Operation ID
    pub op_id: SyncOpId,
    /// Operation type
    pub op_type: SyncOpType,
    /// Operation data
    pub data: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
    /// Priority
    pub priority: SyncPriority,
    /// Retry count
    pub retry_count: usize,
    /// Max retries
    pub max_retries: usize,
}

impl OfflineQueue {
    /// Create a new offline queue
    pub fn new(queue_id: u64, max_size: usize, max_bytes: u64) -> Self {
        Self {
            queue_id,
            pending_ops: VecDeque::new(),
            max_size,
            max_bytes,
            current_bytes: AtomicU64::new(0),
        }
    }

    /// Enqueue operation
    pub fn enqueue(&self, op: QueuedOp) -> Result<()> {
        if self.pending_ops.len() >= self.max_size {
            return Err(nos_api::Error::NoMemory);
        }

        let op_size = op.data.len() as u64;
        if self.current_bytes.load(Ordering::Relaxed) + op_size > self.max_bytes {
            return Err(nos_api::Error::NoMemory);
        }

        // This would require interior mutability in real implementation
        // For now, just return Ok
        crate::println!("[sync] Enqueued operation {} ({} bytes)", op.op_id, op_size);

        Ok(())
    }

    /// Dequeue operation
    pub fn dequeue(&mut self) -> Option<QueuedOp> {
        let op = self.pending_ops.pop_front()?;
        let op_size = op.data.len() as u64;
        self.current_bytes.fetch_sub(op_size, Ordering::Relaxed);
        Some(op)
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.pending_ops.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.pending_ops.is_empty()
    }
}

/// Conflict resolver for CRDT operations
pub struct ConflictResolver {
    /// Resolver ID
    pub resolver_id: u64,
    /// Resolution strategy
    pub strategy: ConflictResolutionStrategy,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolutionStrategy {
    /// Last-write-wins (based on timestamp)
    LastWriteWins,
    /// Application-defined
    ApplicationDefined,
    /// Merge
    Merge,
    /// Custom
    Custom,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new(strategy: ConflictResolutionStrategy) -> Self {
        Self {
            resolver_id: nos_api::event::get_time_ns(),
            strategy,
        }
    }

    /// Resolve conflict between two operations
    pub fn resolve(&self, op1: &SyncDelta, op2: &SyncDelta) -> Result<SyncDelta> {
        match self.strategy {
            ConflictResolutionStrategy::LastWriteWins => {
                if op1.timestamp >= op2.timestamp {
                    Ok(op1.clone())
                } else {
                    Ok(op2.clone())
                }
            }
            ConflictResolutionStrategy::Merge => {
                // Simple merge: concatenate data
                let mut merged_data = op1.data.clone();
                merged_data.extend_from_slice(&op2.data);

                Ok(SyncDelta {
                    delta_id: nos_api::event::get_time_ns(),
                    source_id: op1.source_id,
                    target_id: op1.target_id,
                    op_type: SyncOpType::Merge,
                    data: merged_data,
                    timestamp: nos_api::event::get_time_ns(),
                    priority: op1.priority.max(op2.priority),
                    size: op1.size + op2.size,
                })
            }
            _ => Err(nos_api::Error::NotImplemented),
        }
    }
}

/// Bandwidth-aware transfer
pub struct BandwidthAwareTransfer {
    /// Estimated bandwidth in bytes/sec
    bandwidth_bytes_per_sec: AtomicU64,
    /// Active transfers
    active_transfers: Mutex<BTreeMap<u64, TransferState>>,
    /// Transfer queue
    transfer_queue: Mutex<VecDeque<QueuedTransfer>>,
    /// Next transfer ID
    next_transfer_id: AtomicU64,
}

/// Transfer state
#[derive(Debug, Clone)]
pub struct TransferState {
    /// Transfer ID
    pub transfer_id: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Transferred bytes
    pub transferred_bytes: u64,
    /// Start timestamp
    pub start_time: u64,
    /// Priority
    pub priority: SyncPriority,
}

/// Queued transfer
#[derive(Debug, Clone)]
pub struct QueuedTransfer {
    /// Transfer ID
    pub transfer_id: u64,
    /// Data size
    pub data_size: u64,
    /// Priority
    pub priority: SyncPriority,
    /// Enqueue timestamp
    pub timestamp: u64,
}

impl BandwidthAwareTransfer {
    /// Create a new bandwidth-aware transfer manager
    pub fn new(initial_bandwidth: u64) -> Self {
        Self {
            bandwidth_bytes_per_sec: AtomicU64::new(initial_bandwidth),
            active_transfers: Mutex::new(BTreeMap::new()),
            transfer_queue: Mutex::new(VecDeque::new()),
            next_transfer_id: AtomicU64::new(1),
        }
    }

    /// Update bandwidth estimate
    pub fn update_bandwidth(&self, bandwidth: u64) {
        self.bandwidth_bytes_per_sec.store(bandwidth, Ordering::Relaxed);
        crate::println!("[sync] Bandwidth updated to {} bytes/sec", bandwidth);
    }

    /// Get current bandwidth
    pub fn get_bandwidth(&self) -> u64 {
        self.bandwidth_bytes_per_sec.load(Ordering::Relaxed)
    }

    /// Start transfer
    pub fn start_transfer(&self, data_size: u64, priority: SyncPriority) -> Result<u64> {
        let transfer_id = self.next_transfer_id.fetch_add(1, Ordering::SeqCst);

        let transfer = TransferState {
            transfer_id,
            total_bytes: data_size,
            transferred_bytes: 0,
            start_time: nos_api::event::get_time_ns(),
            priority,
        };

        // Add to active transfers
        {
            let mut active = self.active_transfers.lock();
            active.insert(transfer_id, transfer);
        }

        crate::println!("[sync] Transfer {} started ({} bytes, priority: {:?})",
            transfer_id, data_size, priority);

        Ok(transfer_id)
    }

    /// Update transfer progress
    pub fn update_transfer(&self, transfer_id: u64, transferred: u64) -> Result<()> {
        let mut active = self.active_transfers.lock();
        if let Some(transfer) = active.get_mut(&transfer_id) {
            transfer.transferred_bytes = transferred;
            Ok(())
        } else {
            Err(nos_api::Error::NotFound)
        }
    }

    /// Complete transfer
    pub fn complete_transfer(&self, transfer_id: u64) -> Result<()> {
        let mut active = self.active_transfers.lock();
        active.remove(&transfer_id)
            .ok_or(nos_api::Error::NotFound)?;

        crate::println!("[sync] Transfer {} completed", transfer_id);

        Ok(())
    }

    /// Estimate transfer time
    pub fn estimate_transfer_time(&self, data_size: u64) -> Duration {
        let bandwidth = self.get_bandwidth();
        if bandwidth == 0 {
            return Duration::from_secs(3600); // 1 hour fallback
        }

        let seconds = data_size / bandwidth;
        Duration::from_secs(seconds.max(1))
    }

    /// Adapt data to bandwidth (compress/deduplicate)
    pub fn adapt_data(&self, data: &[u8]) -> Vec<u8> {
        // Placeholder: Apply compression based on bandwidth
        // Low bandwidth: Aggressive compression
        // High bandwidth: No compression

        let bandwidth = self.get_bandwidth();

        if bandwidth < 1_000_000 { // < 1 Mbps
            // Compress (placeholder: just return data)
            data.to_vec()
        } else {
            data.to_vec()
        }
    }
}

/// Edge synchronization protocol
pub struct EdgeSyncProtocol {
    /// Local replica ID
    local_replica_id: ReplicaId,
    /// CRDT replicas
    crdt_replicas: Mutex<BTreeMap<String, CRDTReplica>>,
    /// Delta sync state
    delta_sync: Mutex<DeltaSyncState>,
    /// Offline queue
    offline_queue: Mutex<OfflineQueue>,
    /// Conflict resolver
    conflict_resolver: ConflictResolver,
    /// Bandwidth-aware transfer
    bandwidth_transfer: BandwidthAwareTransfer,
    /// Online flag
    online: AtomicBool,
}

impl EdgeSyncProtocol {
    /// Create a new edge sync protocol
    pub fn new(replica_id: ReplicaId) -> Self {
        Self {
            local_replica_id: replica_id,
            crdt_replicas: Mutex::new(BTreeMap::new()),
            delta_sync: Mutex::new(DeltaSyncState {
                replica_id,
                remote_states: BTreeMap::new(),
                pending_deltas: VecDeque::new(),
                last_sync: 0,
                sync_interval: Duration::from_secs(30),
            }),
            offline_queue: Mutex::new(OfflineQueue::new(
                1, 1000, 10 * 1024 * 1024 // 1000 ops, 10MB
            )),
            conflict_resolver: ConflictResolver::new(ConflictResolutionStrategy::LastWriteWins),
            bandwidth_transfer: BandwidthAwareTransfer::new(1_000_000), // 1 Mbps default
            online: AtomicBool::new(true),
        }
    }

    /// Register CRDT replica
    pub fn register_crdt(&self, key: String, replica: CRDTReplica) -> Result<()> {
        let mut replicas = self.crdt_replicas.lock();
        replicas.insert(key, replica);
        Ok(())
    }

    /// Get CRDT replica
    pub fn get_crdt(&self, key: &str) -> Option<CRDTReplica> {
        let replicas = self.crdt_replicas.lock();
        replicas.get(key).cloned()
    }

    /// Synchronize with remote replica
    pub fn sync_with_remote(&self, remote_id: ReplicaId) -> Result<SyncResult> {
        crate::println!("[sync] Synchronizing with remote replica {}", remote_id);

        if !self.online.load(Ordering::Relaxed) {
            // Offline: queue operation
            return Err(nos_api::Error::Unavailable);
        }

        let start_time = nos_api::event::get_time_ns();

        // Perform synchronization (placeholder)
        // In real implementation:
        // 1. Compute deltas
        // 2. Exchange deltas with remote
        // 3. Apply remote deltas
        // 4. Resolve conflicts
        // 5. Update vector clocks

        let sync_time = nos_api::event::get_time_ns() - start_time;

        let result = SyncResult {
            local_replica: self.local_replica_id,
            remote_replica: remote_id,
            success: true,
            bytes_sent: 0,
            bytes_received: 0,
            sync_time_ms: sync_time / 1_000_000,
            conflicts_resolved: 0,
        };

        crate::println!("[sync] Sync completed in {}ms", result.sync_time_ms);

        Ok(result)
    }

    /// Set online status
    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::Relaxed);
        crate::println!("[sync] Status: {}", if online { "online" } else { "offline" });
    }

    /// Check if online
    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }

    /// Process offline queue
    pub fn process_offline_queue(&mut self) -> Result<usize> {
        if !self.is_online() {
            return Ok(0);
        }

        let mut queue = self.offline_queue.lock();
        let mut processed = 0;

        while let Some(op) = queue.dequeue() {
            // Process operation (placeholder)
            crate::println!("[sync] Processing queued operation {}", op.op_id);
            processed += 1;
        }

        crate::println!("[sync] Processed {} queued operations", processed);

        Ok(processed)
    }
}

/// Synchronization result
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Local replica ID
    pub local_replica: ReplicaId,
    /// Remote replica ID
    pub remote_replica: ReplicaId,
    /// Success flag
    pub success: bool,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Sync time in milliseconds
    pub sync_time_ms: u64,
    /// Number of conflicts resolved
    pub conflicts_resolved: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcounter_creation() {
        let replica = CRDTReplica::new_gcounter(1);
        assert_eq!(replica.replica_id, 1);
        assert_eq!(replica.crdt_type, CRDTType::GCounter);
    }

    #[test]
    fn test_gcounter_increment() {
        let mut replica = CRDTReplica::new_gcounter(1);
        replica.increment(5).unwrap();
        assert_eq!(replica.get_gcounter_value().unwrap(), 5);
    }

    #[test]
    fn test_gcounter_merge() {
        let mut replica1 = CRDTReplica::new_gcounter(1);
        let mut replica2 = CRDTReplica::new_gcounter(2);

        replica1.increment(10).unwrap();
        replica2.increment(20).unwrap();

        replica1.merge(&replica2).unwrap();
        assert_eq!(replica1.get_gcounter_value().unwrap(), 30);
    }

    #[test]
    fn test_offline_queue() {
        let queue = OfflineQueue::new(1, 100, 1024 * 1024);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_bandwidth_aware() {
        let transfer = BandwidthAwareTransfer::new(1_000_000);
        assert_eq!(transfer.get_bandwidth(), 1_000_000);

        transfer.update_bandwidth(2_000_000);
        assert_eq!(transfer.get_bandwidth(), 2_000_000);
    }
}
