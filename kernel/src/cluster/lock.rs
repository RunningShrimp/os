//! Distributed Lock Manager (DLM)
//!
//! This module implements a distributed lock manager for coordinating access to resources
//! across a cluster. It provides deadlock detection, lock migration, and failover capabilities.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::sync::{Mutex, RwLock};

/// Unique identifier for a lock
pub type LockId = u64;

/// Unique identifier for a node in the cluster
pub type NodeId = u64;

/// Unique identifier for a transaction
pub type TransactionId = u64;

/// Lock mode determines the type of access granted
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockMode {
    /// No lock held
    Null,
    /// Read lock - multiple readers allowed
    Read,
    /// Write lock - exclusive writer, no readers
    Write,
    /// Exclusive lock - complete exclusivity
    Exclusive,
}

impl LockMode {
    /// Check if this mode is compatible with another mode
    pub fn is_compatible(&self, other: LockMode) -> bool {
        match (self, other) {
            (LockMode::Null, _) | (_, LockMode::Null) => true,
            (LockMode::Read, LockMode::Read) => true,
            (LockMode::Write, LockMode::Read) | (LockMode::Read, LockMode::Write) => false,
            (LockMode::Exclusive, _) | (_, LockMode::Exclusive) => false,
            (LockMode::Write, LockMode::Write) => false,
        }
    }

    /// Convert lock mode to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LockMode::Null => "NULL",
            LockMode::Read => "READ",
            LockMode::Write => "WRITE",
            LockMode::Exclusive => "EXCLUSIVE",
        }
    }
}

/// Granularity of the lock
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockGranularity {
    /// Lock an entire file
    File,
    /// Lock a specific record
    Record,
    /// Lock a byte range
    ByteRange,
}

/// Information about a held lock
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// Unique lock identifier
    pub id: LockId,
    /// Resource being locked
    pub resource: String,
    /// Lock mode
    pub mode: LockMode,
    /// Node holding the lock
    pub holder: NodeId,
    /// Transaction ID (if any)
    pub transaction_id: Option<TransactionId>,
    /// Time when lock was acquired
    pub acquired_at: u64,
    /// Lock timeout in milliseconds
    pub timeout_ms: u64,
    /// Lock granularity
    pub granularity: LockGranularity,
}

/// Request waiting for a lock
#[derive(Debug, Clone)]
struct LockRequest {
    /// Requesting node
    node_id: NodeId,
    /// Requested lock mode
    mode: LockMode,
    /// Request timestamp
    timestamp: u64,
    /// Transaction ID
    transaction_id: Option<TransactionId>,
    /// Request timeout
    timeout: Duration,
}

/// Deadlock detection edge
#[derive(Debug, Clone, PartialEq, Eq)]
struct WaitEdge {
    /// Node waiting for lock
    waiting_node: NodeId,
    /// Node holding the lock
    holding_node: NodeId,
    /// Resource being waited on
    resource: String,
}

/// Deadlock cycle detected
#[derive(Debug, Clone)]
pub struct DeadlockCycle {
    /// Nodes involved in the deadlock
    pub nodes: Vec<NodeId>,
    /// Resources involved
    pub resources: Vec<String>,
    /// Cycle length
    pub length: usize,
}

/// Statistics for the distributed lock manager
#[derive(Debug, Default)]
pub struct DlmStatistics {
    /// Total lock acquisitions attempted
    pub total_acquisitions: AtomicU64,
    /// Successful lock acquisitions
    pub successful_acquisitions: AtomicU64,
    /// Lock timeouts
    pub timeouts: AtomicU64,
    /// Deadlocks detected
    pub deadlocks_detected: AtomicU64,
    /// Deadlocks resolved
    pub deadlocks_resolved: AtomicU64,
    /// Lock conversions
    pub conversions: AtomicU64,
    /// Current number of active locks
    pub active_locks: AtomicUsize,
    /// Average wait time for locks (nanoseconds)
    pub avg_wait_time_ns: AtomicU64,
    /// Lock migrations
    pub migrations: AtomicU64,
}

/// Errors that can occur in the distributed lock manager
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlmError {
    /// Lock request timeout
    LockTimeout,
    /// Deadlock detected
    DeadlockDetected(DeadlockCycle),
    /// Invalid lock mode conversion
    InvalidConversion {
        from: LockMode,
        to: LockMode,
    },
    /// Lock not found
    LockNotFound(LockId),
    /// Resource not found
    ResourceNotFound(String),
    /// Node not available
    NodeUnavailable(NodeId),
    /// Transaction aborted
    TransactionAborted(TransactionId),
    /// Permission denied
    PermissionDenied,
    /// Resource quota exceeded
    QuotaExceeded,
    /// Invalid lock range
    InvalidRange,
    /// Lock table full
    LockTableFull,
    /// Communication failure
    CommunicationFailure(String),
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for DlmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DlmError::LockTimeout => write!(f, "Lock acquisition timeout"),
            DlmError::DeadlockDetected(cycle) => {
                write!(f, "Deadlock detected with {} nodes", cycle.length)
            }
            DlmError::InvalidConversion { from, to } => {
                write!(f, "Invalid lock conversion from {:?} to {:?}", from, to)
            }
            DlmError::LockNotFound(id) => write!(f, "Lock {} not found", id),
            DlmError::ResourceNotFound(resource) => {
                write!(f, "Resource '{}' not found", resource)
            }
            DlmError::NodeUnavailable(node_id) => {
                write!(f, "Node {} unavailable", node_id)
            }
            DlmError::TransactionAborted(tx_id) => {
                write!(f, "Transaction {} aborted", tx_id)
            }
            DlmError::PermissionDenied => write!(f, "Permission denied"),
            DlmError::QuotaExceeded => write!(f, "Resource quota exceeded"),
            DlmError::InvalidRange => write!(f, "Invalid lock range"),
            DlmError::LockTableFull => write!(f, "Lock table full"),
            DlmError::CommunicationFailure(msg) => {
                write!(f, "Communication failure: {}", msg)
            }
            DlmError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Distributed Lock Manager
pub struct DistributedLockManager {
    /// Current node ID
    node_id: NodeId,
    /// Lock table: resource -> locks
    lock_table: RwLock<BTreeMap<String, Vec<LockInfo>>>,
    /// Lock requests by resource
    pending_requests: RwLock<BTreeMap<String, VecDeque<LockRequest>>>,
    /// Node to locks mapping
    node_locks: RwLock<BTreeMap<NodeId, BTreeSet<LockId>>>,
    /// Transaction to locks mapping
    transaction_locks: RwLock<BTreeMap<TransactionId, BTreeSet<LockId>>>,
    /// Wait-for graph for deadlock detection
    wait_graph: RwLock<Vec<WaitEdge>>,
    /// Next lock ID
    next_lock_id: AtomicU64,
    /// Statistics
    stats: Arc<DlmStatistics>,
    /// Maximum locks allowed
    max_locks: usize,
    /// Default lock timeout
    default_timeout: Duration,
}

impl DistributedLockManager {
    /// Create a new distributed lock manager
    pub fn new(node_id: NodeId, max_locks: usize, default_timeout: Duration) -> Self {
        Self {
            node_id,
            lock_table: RwLock::new(BTreeMap::new()),
            pending_requests: RwLock::new(BTreeMap::new()),
            node_locks: RwLock::new(BTreeMap::new()),
            transaction_locks: RwLock::new(BTreeMap::new()),
            wait_graph: RwLock::new(Vec::new()),
            next_lock_id: AtomicU64::new(1),
            stats: Arc::new(DlmStatistics::default()),
            max_locks,
            default_timeout,
        }
    }

    /// Acquire a lock on a resource
    pub fn acquire_lock(
        &self,
        resource: &str,
        mode: LockMode,
        timeout: Option<Duration>,
        granularity: LockGranularity,
    ) -> Result<LockId, DlmError> {
        self.stats.total_acquisitions.fetch_add(1, Ordering::Relaxed);

        let timeout = timeout.unwrap_or(self.default_timeout);
        let node_id = self.node_id;
        let timestamp = self.current_time_ms();

        // Check if we can grant the lock immediately
        let lock_table = self.lock_table.read();

        if let Some(locks) = lock_table.get(resource) {
            // Check compatibility with existing locks
            for existing_lock in locks {
                if !mode.is_compatible(existing_lock.mode) {
                    drop(lock_table);
                    return self.enqueue_lock_request(
                        resource.to_string(),
                        mode,
                        node_id,
                        None,
                        timeout,
                        granularity,
                    );
                }
            }
        }
        drop(lock_table);

        // Grant the lock immediately
        self.grant_lock(resource, mode, node_id, None, granularity, timeout, timestamp)
    }

    /// Acquire a lock within a transaction
    pub fn acquire_lock_transactional(
        &self,
        resource: &str,
        mode: LockMode,
        transaction_id: TransactionId,
        timeout: Option<Duration>,
        granularity: LockGranularity,
    ) -> Result<LockId, DlmError> {
        let timeout = timeout.unwrap_or(self.default_timeout);
        let node_id = self.node_id;
        let timestamp = self.current_time_ms();

        // Check transaction state
        let tx_locks = self.transaction_locks.read();
        if let Some(_) = tx_locks.get(&transaction_id) {
            // Transaction exists, proceed
        }
        drop(tx_locks);

        // Try to grant immediately or enqueue
        let lock_table = self.lock_table.read();
        if let Some(locks) = lock_table.get(resource) {
            for existing_lock in locks {
                if !mode.is_compatible(existing_lock.mode) {
                    drop(lock_table);
                    return self.enqueue_lock_request(
                        resource.to_string(),
                        mode,
                        node_id,
                        Some(transaction_id),
                        timeout,
                        granularity,
                    );
                }
            }
        }
        drop(lock_table);

        self.grant_lock(
            resource,
            mode,
            node_id,
            Some(transaction_id),
            granularity,
            timeout,
            timestamp,
        )
    }

    /// Grant a lock to a node
    fn grant_lock(
        &self,
        resource: &str,
        mode: LockMode,
        holder: NodeId,
        transaction_id: Option<TransactionId>,
        granularity: LockGranularity,
        timeout: Duration,
        timestamp: u64,
    ) -> Result<LockId, DlmError> {
        // Check if lock table is full
        {
            let lock_table = self.lock_table.read();
            let total_locks: usize = lock_table.values().map(|v| v.len()).sum();
            if total_locks >= self.max_locks {
                return Err(DlmError::LockTableFull);
            }
        }

        // Generate lock ID
        let lock_id = self.next_lock_id.fetch_add(1, Ordering::SeqCst);

        // Create lock info
        let lock_info = LockInfo {
            id: lock_id,
            resource: resource.to_string(),
            mode,
            holder,
            transaction_id,
            acquired_at: timestamp,
            timeout_ms: timeout.as_millis() as u64,
            granularity,
        };

        // Add to lock table
        {
            let mut lock_table = self.lock_table.write();
            lock_table
                .entry(resource.to_string())
                .or_insert_with(Vec::new)
                .push(lock_info.clone());
        }

        // Update node locks mapping
        {
            let mut node_locks = self.node_locks.write();
            node_locks
                .entry(holder)
                .or_insert_with(BTreeSet::new)
                .insert(lock_id);
        }

        // Update transaction locks mapping if applicable
        if let Some(tx_id) = transaction_id {
            let mut tx_locks = self.transaction_locks.write();
            tx_locks
                .entry(tx_id)
                .or_insert_with(BTreeSet::new)
                .insert(lock_id);
        }

        // Update statistics
        self.stats.successful_acquisitions.fetch_add(1, Ordering::Relaxed);
        self.stats.active_locks.fetch_add(1, Ordering::Relaxed);

        Ok(lock_id)
    }

    /// Enqueue a lock request when immediate grant is not possible
    fn enqueue_lock_request(
        &self,
        resource: String,
        mode: LockMode,
        node_id: NodeId,
        transaction_id: Option<TransactionId>,
        timeout: Duration,
        granularity: LockGranularity,
    ) -> Result<LockId, DlmError> {
        // Add wait-for graph edge
        {
            let lock_table = self.lock_table.read();
            if let Some(locks) = lock_table.get(&resource) {
                let mut wait_graph = self.wait_graph.write();
                for lock in locks {
                    wait_graph.push(WaitEdge {
                        waiting_node: node_id,
                        holding_node: lock.holder,
                        resource: resource.clone(),
                    });
                }
            }
        }

        // Check for deadlock
        if let Some(cycle) = self.detect_deadlock(node_id) {
            self.stats.deadlocks_detected.fetch_add(1, Ordering::Relaxed);
            return Err(DlmError::DeadlockDetected(cycle));
        }

        // Enqueue the request
        let request = LockRequest {
            node_id,
            mode,
            timestamp: self.current_time_ms(),
            transaction_id,
            timeout,
        };

        {
            let mut pending = self.pending_requests.write();
            pending
                .entry(resource.clone())
                .or_insert_with(VecDeque::new)
                .push_back(request);
        }

        // In a real implementation, we would wait here
        // For now, return timeout
        self.stats.timeouts.fetch_add(1, Ordering::Relaxed);
        Err(DlmError::LockTimeout)
    }

    /// Release a lock
    pub fn release_lock(&self, lock_id: LockId) -> Result<(), DlmError> {
        // Find and remove the lock
        let (resource, holder, transaction_id) = {
            let mut lock_table = self.lock_table.write();
            let mut found = None;

            for (res, locks) in lock_table.iter_mut() {
                if let Some(pos) = locks.iter().position(|l| l.id == lock_id) {
                    let lock_info = locks.remove(pos);
                    found = Some((
                        res.clone(),
                        lock_info.holder,
                        lock_info.transaction_id,
                    ));
                    break;
                }
            }

            found.ok_or(DlmError::LockNotFound(lock_id))?
        };

        // Remove from node locks mapping
        {
            let mut node_locks = self.node_locks.write();
            if let Some(locks) = node_locks.get_mut(&holder) {
                locks.remove(&lock_id);
                if locks.is_empty() {
                    node_locks.remove(&holder);
                }
            }
        }

        // Remove from transaction locks if applicable
        if let Some(tx_id) = transaction_id {
            let mut tx_locks = self.transaction_locks.write();
            if let Some(locks) = tx_locks.get_mut(&tx_id) {
                locks.remove(&lock_id);
                if locks.is_empty() {
                    tx_locks.remove(&tx_id);
                }
            }
        }

        // Update statistics
        self.stats.active_locks.fetch_sub(1, Ordering::Relaxed);

        // Remove from wait-for graph
        {
            let mut wait_graph = self.wait_graph.write();
            wait_graph.retain(|edge| edge.resource != resource);
        }

        // Process pending requests for this resource
        self.process_pending_requests(&resource);

        Ok(())
    }

    /// Convert a lock to a different mode
    pub fn convert_lock(
        &self,
        lock_id: LockId,
        new_mode: LockMode,
    ) -> Result<(), DlmError> {
        // Find the lock
        let (resource, old_mode, holder) = {
            let lock_table = self.lock_table.read();
            let mut found = None;

            for (res, locks) in lock_table.iter() {
                if let Some(lock) = locks.iter().find(|l| l.id == lock_id) {
                    found = Some((res.clone(), lock.mode, lock.holder));
                    break;
                }
            }

            found.ok_or(DlmError::LockNotFound(lock_id))?
        };

        // Validate conversion
        if !Self::is_valid_conversion(old_mode, new_mode) {
            return Err(DlmError::InvalidConversion {
                from: old_mode,
                to: new_mode,
            });
        }

        // Update lock mode
        {
            let mut lock_table = self.lock_table.write();
            if let Some(locks) = lock_table.get_mut(&resource) {
                if let Some(lock) = locks.iter_mut().find(|l| l.id == lock_id) {
                    lock.mode = new_mode;
                }
            }
        }

        // Update statistics
        self.stats.conversions.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Check if lock mode conversion is valid
    fn is_valid_conversion(from: LockMode, to: LockMode) -> bool {
        match (from, to) {
            (LockMode::Null, _) => true,
            (LockMode::Read, LockMode::Write | LockMode::Exclusive) => true,
            (LockMode::Write, LockMode::Exclusive) => true,
            _ => false,
        }
    }

    /// Process pending lock requests for a resource
    fn process_pending_requests(&self, resource: &str) {
        let mut pending = self.pending_requests.write();
        if let Some(requests) = pending.get_mut(resource) {
            let lock_table = self.lock_table.read();
            let current_locks = lock_table.get(resource).cloned().unwrap_or_default();
            drop(lock_table);

            let mut to_grant = Vec::new();
            let mut to_remove = Vec::new();

            for (i, request) in requests.iter().enumerate() {
                let can_grant = current_locks.iter().all(|lock| {
                    request.mode.is_compatible(lock.mode)
                });

                if can_grant {
                    to_grant.push((i, request.clone()));
                    to_remove.push(i);
                }
            }

            // Remove granted requests
            to_remove.sort_by(|a, b| b.cmp(a));
            for i in to_remove {
                requests.remove(i);
            }

            // Grant locks
            for (i, request) in to_grant {
                let _ = self.grant_lock(
                    resource,
                    request.mode,
                    request.node_id,
                    request.transaction_id,
                    LockGranularity::File,
                    request.timeout,
                    request.timestamp,
                );
            }
        }
    }

    /// Detect deadlock using wait-for graph
    fn detect_deadlock(&self, start_node: NodeId) -> Option<DeadlockCycle> {
        let wait_graph = self.wait_graph.read();
        let mut visited = BTreeMap::new();
        let mut path = Vec::new();
        let mut resources = Vec::new();

        if self.dfs_detect_cycle(start_node, &wait_graph, &mut visited, &mut path, &mut resources) {
            Some(DeadlockCycle {
                nodes: path,
                resources,
                length: path.len(),
            })
        } else {
            None
        }
    }

    /// DFS for cycle detection in wait-for graph
    fn dfs_detect_cycle(
        &self,
        node: NodeId,
        graph: &[WaitEdge],
        visited: &mut BTreeMap<NodeId, bool>,
        path: &mut Vec<NodeId>,
        resources: &mut Vec<String>,
    ) -> bool {
        if let Some(&in_path) = visited.get(&node) {
            if in_path {
                // Found a cycle
                return true;
            }
            return false;
        }

        visited.insert(node, true);
        path.push(node);

        // Find edges from this node
        for edge in graph {
            if edge.waiting_node == node {
                resources.push(edge.resource.clone());
                if self.dfs_detect_cycle(edge.holding_node, graph, visited, path, resources) {
                    return true;
                }
                resources.pop();
            }
        }

        path.pop();
        visited.insert(node, false);
        false
    }

    /// Resolve a deadlock by aborting a transaction
    pub fn resolve_deadlock(&self, cycle: &DeadlockCycle) -> Result<(), DlmError> {
        // Choose a victim (youngest transaction)
        let victim_node = *cycle.nodes.first().unwrap_or(&0);

        // Release all locks held by the victim
        let node_locks = self.node_locks.read();
        if let Some(lock_ids) = node_locks.get(&victim_node) {
            let lock_ids: Vec<_> = lock_ids.iter().copied().collect();
            drop(node_locks);

            for lock_id in lock_ids {
                let _ = self.release_lock(lock_id);
            }
        }

        self.stats.deadlocks_resolved.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Migrate locks from one node to another
    pub fn migrate_locks(
        &self,
        from_node: NodeId,
        to_node: NodeId,
    ) -> Result<usize, DlmError> {
        let mut migrated = 0;

        // Get all locks from the source node
        let lock_ids: Vec<LockId> = {
            let node_locks = self.node_locks.read();
            node_locks
                .get(&from_node)
                .map(|locks| locks.iter().copied().collect())
                .unwrap_or_default()
        };

        // Migrate each lock
        for lock_id in lock_ids {
            let (resource, mode, transaction_id, granularity, timeout, acquired_at) = {
                let mut lock_table = self.lock_table.write();
                let mut found = None;

                for (res, locks) in lock_table.iter_mut() {
                    if let Some(lock) = locks.iter_mut().find(|l| l.id == lock_id) {
                        lock.holder = to_node;
                        found = Some((
                            res.clone(),
                            lock.mode,
                            lock.transaction_id,
                            lock.granularity,
                            lock.timeout_ms,
                            lock.acquired_at,
                        ));
                        break;
                    }
                }

                found.ok_or(DlmError::LockNotFound(lock_id))?
            };

            // Update node locks mapping
            {
                let mut node_locks = self.node_locks.write();
                node_locks
                    .entry(from_node)
                    .or_insert_with(BTreeSet::new)
                    .remove(&lock_id);
                node_locks
                    .entry(to_node)
                    .or_insert_with(BTreeSet::new)
                    .insert(lock_id);
            }

            migrated += 1;
        }

        self.stats.migrations.fetch_add(migrated as u64, Ordering::Relaxed);
        Ok(migrated)
    }

    /// Get information about a lock
    pub fn get_lock_info(&self, lock_id: LockId) -> Result<LockInfo, DlmError> {
        let lock_table = self.lock_table.read();

        for locks in lock_table.values() {
            if let Some(lock) = locks.iter().find(|l| l.id == lock_id) {
                return Ok(lock.clone());
            }
        }

        Err(DlmError::LockNotFound(lock_id))
    }

    /// Get all locks held by a node
    pub fn get_node_locks(&self, node_id: NodeId) -> Vec<LockInfo> {
        let lock_ids: Vec<LockId> = {
            let node_locks = self.node_locks.read();
            node_locks
                .get(&node_id)
                .map(|locks| locks.iter().copied().collect())
                .unwrap_or_default()
        };

        let mut result = Vec::new();
        let lock_table = self.lock_table.read();

        for locks in lock_table.values() {
            for lock in locks {
                if lock_ids.contains(&lock.id) {
                    result.push(lock.clone());
                }
            }
        }

        result
    }

    /// Get all locks for a transaction
    pub fn get_transaction_locks(&self, transaction_id: TransactionId) -> Vec<LockInfo> {
        let lock_ids: Vec<LockId> = {
            let tx_locks = self.transaction_locks.read();
            tx_locks
                .get(&transaction_id)
                .map(|locks| locks.iter().copied().collect())
                .unwrap_or_default()
        };

        let mut result = Vec::new();
        let lock_table = self.lock_table.read();

        for locks in lock_table.values() {
            for lock in locks {
                if lock_ids.contains(&lock.id) {
                    result.push(lock.clone());
                }
            }
        }

        result
    }

    /// Release all locks for a transaction
    pub fn release_transaction_locks(&self, transaction_id: TransactionId) -> Result<usize, DlmError> {
        let lock_ids: Vec<LockId> = {
            let tx_locks = self.transaction_locks.read();
            tx_locks
                .get(&transaction_id)
                .map(|locks| locks.iter().copied().collect())
                .unwrap_or_default()
        };

        let mut released = 0;
        for lock_id in lock_ids {
            if self.release_lock(lock_id).is_ok() {
                released += 1;
            }
        }

        Ok(released)
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &DlmStatistics {
        &self.stats
    }

    /// Get current time in milliseconds
    fn current_time_ms(&self) -> u64 {
        // In a real implementation, this would use the actual time
        // For now, return a dummy value
        0
    }

    /// Cleanup expired locks
    pub fn cleanup_expired_locks(&self) -> Result<usize, DlmError> {
        let current_time = self.current_time_ms();
        let mut expired_locks = Vec::new();

        {
            let lock_table = self.lock_table.read();
            for locks in lock_table.values() {
                for lock in locks {
                    let elapsed = current_time.saturating_sub(lock.acquired_at);
                    if elapsed > lock.timeout_ms {
                        expired_locks.push(lock.id);
                    }
                }
            }
        }

        let mut cleaned = 0;
        for lock_id in expired_locks {
            if self.release_lock(lock_id).is_ok() {
                cleaned += 1;
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_mode_compatibility() {
        assert!(LockMode::Read.is_compatible(LockMode::Read));
        assert!(!LockMode::Read.is_compatible(LockMode::Write));
        assert!(!LockMode::Write.is_compatible(LockMode::Read));
        assert!(!LockMode::Exclusive.is_compatible(LockMode::Read));
    }

    #[test]
    fn test_acquire_and_release_lock() {
        let dlm = DistributedLockManager::new(1, 1000, Duration::from_secs(5));

        let lock_id = dlm
            .acquire_lock("test_resource", LockMode::Read, None, LockGranularity::File)
            .unwrap();

        assert!(dlm.get_lock_info(lock_id).is_ok());

        dlm.release_lock(lock_id).unwrap();
        assert!(dlm.get_lock_info(lock_id).is_err());
    }

    #[test]
    fn test_lock_conversion() {
        let dlm = DistributedLockManager::new(1, 1000, Duration::from_secs(5));

        let lock_id = dlm
            .acquire_lock("test_resource", LockMode::Read, None, LockGranularity::File)
            .unwrap();

        assert!(dlm.convert_lock(lock_id, LockMode::Write).is_ok());
        assert!(dlm.convert_lock(lock_id, LockMode::Exclusive).is_ok());
    }
}
