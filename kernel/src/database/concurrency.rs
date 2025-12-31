//! # Concurrency Control
//!
//! Provides concurrency control mechanisms:
//! - Lock manager (shared/exclusive locks, lock escalation)
//! - Latch management (buffer pool latching, B-tree latching)
//! - Deadlock detection (wait-for graph, victim selection)
//! - Lock timeout and retry logic

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use super::{DatabaseError, Result};

/// Lock mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockMode {
    /// Intention shared
    IS,

    /// Intention exclusive
    IX,

    /// Shared
    S,

    /// Shared exclusive (six)
    SIX,

    /// Exclusive
    X,
}

impl LockMode {
    /// Check if this mode is compatible with another
    pub fn is_compatible(&self, other: &LockMode) -> bool {
        match (self, other) {
            (LockMode::IS, LockMode::IS) => true,
            (LockMode::IS, LockMode::IX) => true,
            (LockMode::IS, LockMode::S) => true,
            (LockMode::IX, LockMode::IS) => true,
            (LockMode::IX, LockMode::IX) => true,
            (LockMode::IX, LockMode::SIX) => true,
            (LockMode::S, LockMode::IS) => true,
            (LockMode::S, LockMode::S) => true,
            (LockMode::SIX, LockMode::IX) => true,
            (LockMode::SIX, LockMode::IS) => true,
            _ => false,
        }
    }

    /// Convert lock mode to string
    pub fn as_str(&self) -> &str {
        match self {
            LockMode::IS => "IS",
            LockMode::IX => "IX",
            LockMode::S => "S",
            LockMode::SIX => "SIX",
            LockMode::X => "X",
        }
    }
}

/// Lock request
#[derive(Debug, Clone)]
pub struct LockRequest {
    /// Transaction requesting the lock
    pub txn_id: u64,

    /// Lock mode
    pub mode: LockMode,

    /// Resource being locked
    pub resource: String,

    /// Whether the lock is granted
    pub granted: bool,

    /// Timestamp for timeout
    pub timestamp: u64,
}

/// Lock manager
pub struct LockManager {
    /// Lock table: resource -> list of lock requests
    lock_table: BTreeMap<String, Vec<LockRequest>>,

    /// Transaction locks: txn_id -> list of resources
    txn_locks: BTreeMap<u64, BTreeSet<String>>,

    /// Wait-for graph for deadlock detection
    wait_graph: WaitForGraph,

    /// Lock timeout in milliseconds
    lock_timeout_ms: u64,

    /// Next timestamp
    next_timestamp: AtomicU64,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new(lock_timeout_ms: u64) -> Result<Self> {
        Ok(Self {
            lock_table: BTreeMap::new(),
            txn_locks: BTreeMap::new(),
            wait_graph: WaitForGraph::new(),
            lock_timeout_ms,
            next_timestamp: AtomicU64::new(1),
        })
    }

    /// Acquire a lock
    pub fn acquire_lock(&mut self, txn_id: u64, resource: String, mode: LockMode) -> Result<()> {
        let timestamp = self.next_timestamp.fetch_add(1, AtomicOrdering::SeqCst);

        let request = LockRequest {
            txn_id,
            mode,
            resource: resource.clone(),
            granted: false,
            timestamp,
        };

        // Check if lock can be granted immediately
        let can_grant = if let Some(locks) = self.lock_table.get(&resource) {
            locks.iter().all(|lock| {
                lock.granted && mode.is_compatible(&lock.mode)
            })
        } else {
            true
        };

        if can_grant {
            // Grant lock immediately
            self.grant_lock(request)?;
        } else {
            // Add to wait queue
            self.add_to_wait_queue(request)?;

            // Check for deadlock
            if self.wait_graph.has_deadlock() {
                // Select victim and abort
                let _victim = self.select_victim()?;
                return Err(DatabaseError::Deadlock);
            }
        }

        Ok(())
    }

    /// Grant a lock
    fn grant_lock(&mut self, mut request: LockRequest) -> Result<()> {
        request.granted = true;

        self.lock_table
            .entry(request.resource.clone())
            .or_insert_with(Vec::new)
            .push(request.clone());

        self.txn_locks
            .entry(request.txn_id)
            .or_insert_with(BTreeSet::new)
            .insert(request.resource);

        // Update wait graph
        self.wait_graph.add_node(request.txn_id);
        self.wait_graph.remove_waiting(request.txn_id);

        Ok(())
    }

    /// Add request to wait queue
    fn add_to_wait_queue(&mut self, request: LockRequest) -> Result<()> {
        self.lock_table
            .entry(request.resource.clone())
            .or_insert_with(Vec::new)
            .push(request.clone());

        // Update wait graph
        if let Some(locks) = self.lock_table.get(&request.resource) {
            for lock in locks {
                if lock.granted && lock.txn_id != request.txn_id {
                    self.wait_graph.add_edge(request.txn_id, lock.txn_id);
                }
            }
        }

        Ok(())
    }

    /// Release a lock
    pub fn release_lock(&mut self, txn_id: u64, resource: &str) -> Result<()> {
        // Remove lock from lock table
        if let Some(locks) = self.lock_table.get_mut(resource) {
            locks.retain(|lock| lock.txn_id != txn_id || !lock.granted);
        }

        // Remove from transaction's lock set
        if let Some(ref mut locks) = self.txn_locks.get_mut(&txn_id) {
            locks.remove(resource);
        }

        // Grant next waiting lock if compatible
        self.grant_next_waiting_lock(resource)?;

        // Update wait graph
        self.wait_graph.remove_node(txn_id);

        Ok(())
    }

    /// Release all locks held by a transaction
    pub fn release_all_locks(&mut self, txn_id: u64) -> Result<()> {
        if let Some(resources) = self.txn_locks.remove(&txn_id) {
            for resource in resources {
                self.release_lock(txn_id, &resource)?;
            }
        }
        Ok(())
    }

    /// Grant next waiting lock if compatible
    fn grant_next_waiting_lock(&mut self, resource: &str) -> Result<()> {
        if let Some(locks) = self.lock_table.get_mut(resource) {
            // Find first waiting request
            let waiting_pos = locks.iter().position(|lock| !lock.granted);

            if let Some(pos) = waiting_pos {
                let waiting = &locks[pos];

                // Check if compatible with all granted locks
                let compatible = locks.iter().all(|lock| {
                    !lock.granted || waiting.mode.is_compatible(&lock.mode)
                });

                if compatible {
                    // Grant the waiting lock
                    let txn_id = waiting.txn_id;
                    let mut request = waiting.clone();
                    request.granted = true;
                    locks[pos] = request.clone();

                    self.txn_locks
                        .entry(txn_id)
                        .or_insert_with(BTreeSet::new)
                        .insert(String::from(resource));

                    self.wait_graph.remove_waiting(txn_id);
                }
            }
        }

        Ok(())
    }

    /// Check if a transaction holds a lock
    pub fn has_lock(&self, txn_id: u64, resource: &str) -> bool {
        self.txn_locks
            .get(&txn_id)
            .map(|locks| locks.contains(resource))
            .unwrap_or(false)
    }

    /// Select a victim for deadlock resolution
    fn select_victim(&self) -> Result<u64> {
        // Simple strategy: select the transaction with the highest ID (youngest)
        let mut victims = Vec::new();

        for (txn, _) in &self.txn_locks {
            if self.wait_graph.is_in_cycle(*txn) {
                victims.push(*txn);
            }
        }

        if victims.is_empty() {
            return Err(DatabaseError::InternalError(
                String::from("No victim found for deadlock")
            ));
        }

        // Select youngest transaction
        victims.sort();
        Ok(*victims.last().unwrap())
    }

    /// Get number of locks held by a transaction
    pub fn get_lock_count(&self, txn_id: u64) -> usize {
        self.txn_locks
            .get(&txn_id)
            .map(|locks| locks.len())
            .unwrap_or(0)
    }

    /// Get all lock information
    pub fn get_lock_info(&self) -> Vec<LockRequest> {
        let mut result = Vec::new();

        for locks in self.lock_table.values() {
            for lock in locks {
                if lock.granted {
                    result.push(lock.clone());
                }
            }
        }

        result
    }
}

/// Wait-for graph for deadlock detection
#[derive(Debug, Clone)]
pub struct WaitForGraph {
    /// Graph: txn_id -> set of transactions it's waiting for
    graph: BTreeMap<u64, BTreeSet<u64>>,

    /// All nodes in the graph
    nodes: BTreeSet<u64>,
}

impl WaitForGraph {
    /// Create a new wait-for graph
    pub fn new() -> Self {
        Self {
            graph: BTreeMap::new(),
            nodes: BTreeSet::new(),
        }
    }

    /// Add a node (transaction)
    pub fn add_node(&mut self, txn_id: u64) {
        self.nodes.insert(txn_id);
        self.graph.entry(txn_id).or_insert_with(BTreeSet::new);
    }

    /// Remove a node
    pub fn remove_node(&mut self, txn_id: u64) {
        self.nodes.remove(&txn_id);
        self.graph.remove(&txn_id);

        // Remove all edges pointing to this node
        for waits in self.graph.values_mut() {
            waits.remove(&txn_id);
        }
    }

    /// Add an edge (txn1 waits for txn2)
    pub fn add_edge(&mut self, txn1: u64, txn2: u64) {
        self.add_node(txn1);
        self.add_node(txn2);
        self.graph
            .entry(txn1)
            .or_insert_with(BTreeSet::new)
            .insert(txn2);
    }

    /// Remove waiting status for a transaction
    pub fn remove_waiting(&mut self, txn_id: u64) {
        self.graph.insert(txn_id, BTreeSet::new());
    }

    /// Check if there's a deadlock (cycle)
    pub fn has_deadlock(&self) -> bool {
        for &node in &self.nodes {
            if self.is_in_cycle(node) {
                return true;
            }
        }
        false
    }

    /// Check if a node is part of a cycle
    pub fn is_in_cycle(&self, start: u64) -> bool {
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();
        self.has_cycle_util(start, &mut visited, &mut rec_stack)
    }

    /// Utility function for cycle detection
    fn has_cycle_util(
        &self,
        node: u64,
        visited: &mut BTreeSet<u64>,
        rec_stack: &mut BTreeSet<u64>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = self.graph.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if self.has_cycle_util(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }
}

/// Latch mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchMode {
    /// Shared latch (for reading)
    Shared,

    /// Exclusive latch (for writing)
    Exclusive,
}

/// Latch for protecting internal data structures
pub struct Latch {
    /// Current mode
    mode: LatchMode,

    /// Number of holders
    holders: usize,

    /// Wait queue
    wait_queue: Vec<u64>,
}

impl Latch {
    /// Create a new latch
    pub fn new() -> Self {
        Self {
            mode: LatchMode::Shared,
            holders: 0,
            wait_queue: Vec::new(),
        }
    }

    /// Acquire latch
    pub fn acquire(&mut self, mode: LatchMode) -> Result<()> {
        match mode {
            LatchMode::Shared => {
                if self.mode == LatchMode::Exclusive && self.holders > 0 {
                    return Err(DatabaseError::LockTimeout);
                }
                self.mode = LatchMode::Shared;
                self.holders += 1;
            }
            LatchMode::Exclusive => {
                if self.holders > 0 {
                    return Err(DatabaseError::LockTimeout);
                }
                self.mode = LatchMode::Exclusive;
                self.holders = 1;
            }
        }
        Ok(())
    }

    /// Release latch
    pub fn release(&mut self) -> Result<()> {
        if self.holders == 0 {
            return Err(DatabaseError::InternalError(
                String::from("No holders to release")
            ));
        }
        self.holders -= 1;
        if self.holders == 0 {
            self.mode = LatchMode::Shared;
        }
        Ok(())
    }

    /// Check if latch is held
    pub fn is_held(&self) -> bool {
        self.holders > 0
    }
}

/// Latch manager
pub struct LatchManager {
    /// Latches for buffer pool pages
    buffer_latches: BTreeMap<usize, Latch>,

    /// Latches for B-tree nodes
    btree_latches: BTreeMap<Vec<u8>, Latch>,
}

impl LatchManager {
    /// Create a new latch manager
    pub fn new() -> Self {
        Self {
            buffer_latches: BTreeMap::new(),
            btree_latches: BTreeMap::new(),
        }
    }

    /// Acquire buffer pool latch
    pub fn acquire_buffer_latch(&mut self, page_id: usize, mode: LatchMode) -> Result<()> {
        let latch = self
            .buffer_latches
            .entry(page_id)
            .or_insert_with(Latch::new);
        latch.acquire(mode)
    }

    /// Release buffer pool latch
    pub fn release_buffer_latch(&mut self, page_id: usize) -> Result<()> {
        if let Some(latch) = self.buffer_latches.get_mut(&page_id) {
            latch.release()?;
        }
        Ok(())
    }

    /// Acquire B-tree latch
    pub fn acquire_btree_latch(&mut self, node_id: Vec<u8>, mode: LatchMode) -> Result<()> {
        let latch = self
            .btree_latches
            .entry(node_id)
            .or_insert_with(Latch::new);
        latch.acquire(mode)
    }

    /// Release B-tree latch
    pub fn release_btree_latch(&mut self, node_id: &[u8]) -> Result<()> {
        if let Some(latch) = self.btree_latches.get_mut(node_id) {
            latch.release()?;
        }
        Ok(())
    }
}

/// Lock escalation: convert multiple low-level locks to a higher-level lock
pub fn escalate_locks(
    lock_manager: &mut LockManager,
    txn_id: u64,
    resources: Vec<String>,
    target_mode: LockMode,
) -> Result<()> {
    // Release all existing locks
    for resource in &resources {
        lock_manager.release_lock(txn_id, resource)?;
    }

    // Acquire escalated lock (in a real implementation, this would be on a parent resource)
    // For now, just reacquire the first resource with the target mode
    if !resources.is_empty() {
        lock_manager.acquire_lock(txn_id, resources[0].clone(), target_mode)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_compatibility() {
        assert!(LockMode::S.is_compatible(&LockMode::S));
        assert!(LockMode::S.is_compatible(&LockMode::IS));
        assert!(!LockMode::X.is_compatible(&LockMode::S));
        assert!(!LockMode::X.is_compatible(&LockMode::X));
    }

    #[test]
    fn test_lock_manager_creation() {
        let lm = LockManager::new(5000).unwrap();
        // Lock manager created successfully
    }

    #[test]
    fn test_acquire_shared_lock() {
        let mut lm = LockManager::new(5000).unwrap();
        lm.acquire_lock(1, String::from("table1"), LockMode::S).unwrap();
        assert!(lm.has_lock(1, "table1"));
    }

    #[test]
    fn test_acquire_exclusive_lock() {
        let mut lm = LockManager::new(5000).unwrap();
        lm.acquire_lock(1, String::from("table1"), LockMode::X).unwrap();
        assert!(lm.has_lock(1, "table1"));
    }

    #[test]
    fn test_release_lock() {
        let mut lm = LockManager::new(5000).unwrap();
        lm.acquire_lock(1, String::from("table1"), LockMode::S).unwrap();
        lm.release_lock(1, "table1").unwrap();
        assert!(!lm.has_lock(1, "table1"));
    }

    #[test]
    fn test_release_all_locks() {
        let mut lm = LockManager::new(5000).unwrap();
        lm.acquire_lock(1, String::from("table1"), LockMode::S).unwrap();
        lm.acquire_lock(1, String::from("table2"), LockMode::S).unwrap();
        lm.release_all_locks(1).unwrap();
        assert!(!lm.has_lock(1, "table1"));
        assert!(!lm.has_lock(1, "table2"));
    }

    #[test]
    fn test_wait_for_graph() {
        let mut graph = WaitForGraph::new();

        graph.add_node(1);
        graph.add_node(2);
        graph.add_edge(1, 2);

        assert!(!graph.has_deadlock());
    }

    #[test]
    fn test_deadlock_detection() {
        let mut graph = WaitForGraph::new();

        graph.add_node(1);
        graph.add_node(2);
        graph.add_node(3);

        // Create a cycle: 1 -> 2 -> 3 -> 1
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);
        graph.add_edge(3, 1);

        assert!(graph.has_deadlock());
    }

    #[test]
    fn test_latch_acquire_release() {
        let mut latch = Latch::new();

        latch.acquire(LatchMode::Shared).unwrap();
        assert!(latch.is_held());

        latch.release().unwrap();
        assert!(!latch.is_held());
    }

    #[test]
    fn test_latch_exclusive() {
        let mut latch = Latch::new();

        latch.acquire(LatchMode::Exclusive).unwrap();
        assert!(latch.is_held());

        // Second acquire should fail
        assert!(latch.acquire(LatchMode::Shared).is_err());
    }

    #[test]
    fn test_latch_manager() {
        let mut manager = LatchManager::new();

        manager.acquire_buffer_latch(1, LatchMode::Shared).unwrap();
        manager.release_buffer_latch(1).unwrap();
    }

    #[test]
    fn test_lock_count() {
        let mut lm = LockManager::new(5000).unwrap();

        lm.acquire_lock(1, String::from("table1"), LockMode::S).unwrap();
        lm.acquire_lock(1, String::from("table2"), LockMode::S).unwrap();

        assert_eq!(lm.get_lock_count(1), 2);
    }

    #[test]
    fn test_lock_mode_display() {
        assert_eq!(LockMode::S.as_str(), "S");
        assert_eq!(LockMode::X.as_str(), "X");
        assert_eq!(LockMode::IS.as_str(), "IS");
        assert_eq!(LockMode::IX.as_str(), "IX");
        assert_eq!(LockMode::SIX.as_str(), "SIX");
    }

    #[test]
    fn test_incompatible_locks() {
        let mut lm = LockManager::new(5000).unwrap();

        // Transaction 1 gets exclusive lock
        lm.acquire_lock(1, String::from("table1"), LockMode::X).unwrap();

        // Transaction 2 tries to get shared lock - should wait or fail
        // In this simplified implementation, it will wait
        lm.acquire_lock(2, String::from("table1"), LockMode::S).unwrap();

        // Transaction 1 still holds the lock
        assert!(lm.has_lock(1, "table1"));
    }

    #[test]
    fn test_remove_node_from_wait_graph() {
        let mut graph = WaitForGraph::new();

        graph.add_node(1);
        graph.add_node(2);
        graph.add_edge(1, 2);

        graph.remove_node(1);

        assert!(!graph.nodes.contains(&1));
        assert!(graph.nodes.contains(&2));
    }
}
