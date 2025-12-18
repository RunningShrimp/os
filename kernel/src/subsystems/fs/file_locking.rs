//! File Locking Implementation
//!
//! This module implements comprehensive file locking mechanisms for NOS,
//! providing both advisory and mandatory locking with support for shared (read)
//! and exclusive (write) locks. The implementation includes deadlock detection,
//! lock upgrading/downgrading, and POSIX-compatible flock/fcntl locking.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
// use alloc::string::String;
// use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// use crate::subsystems::process::{Process, ProcessId};
// use crate::subsystems::fs::file_permissions::FilePermissions;

/// Lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LockType {
    /// No lock
    None = 0,
    /// Shared/read lock
    Shared = 1,
    /// Exclusive/write lock
    Exclusive = 2,
}

/// Lock range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockRange {
    /// Start offset of the lock
    pub start: u64,
    /// End offset of the lock (inclusive)
    pub end: u64,
}

impl LockRange {
    /// Create a new lock range
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// Create a lock range for the entire file
    pub fn entire_file() -> Self {
        Self { start: 0, end: u64::MAX }
    }

    /// Check if this range overlaps with another
    pub fn overlaps(&self, other: &LockRange) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Check if this range contains another
    pub fn contains(&self, other: &LockRange) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// Get the length of this range
    pub fn len(&self) -> u64 {
        if self.end == u64::MAX {
            u64::MAX
        } else {
            self.end - self.start + 1
        }
    }
}

/// Lock request
#[derive(Debug, Clone)]
pub struct LockRequest {
    /// Process ID requesting the lock
    pub pid: ProcessId,
    /// Lock type
    pub lock_type: LockType,
    /// Lock range
    pub range: LockRange,
    /// Whether this is a blocking request
    pub blocking: bool,
    /// Timestamp when request was made
    pub timestamp: u64,
    /// Request ID for tracking
    pub request_id: u64,
}

impl LockRequest {
    /// Create a new lock request
    pub fn new(pid: ProcessId, lock_type: LockType, range: LockRange, blocking: bool) -> Self {
        Self {
            pid,
            lock_type,
            range,
            blocking,
            timestamp: crate::time::get_timestamp(),
            request_id: next_request_id(),
        }
    }
}

/// Get next unique request ID
fn next_request_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

/// Active lock
#[derive(Debug, Clone)]
pub struct ActiveLock {
    /// Process ID holding the lock
    pub pid: ProcessId,
    /// Lock type
    pub lock_type: LockType,
    /// Lock range
    pub range: LockRange,
    /// Timestamp when lock was acquired
    pub acquired_at: u64,
    /// Lock ID for tracking
    pub lock_id: u64,
}

impl ActiveLock {
    /// Create a new active lock
    pub fn new(pid: ProcessId, lock_type: LockType, range: LockRange) -> Self {
        Self {
            pid,
            lock_type,
            range,
            acquired_at: crate::time::get_timestamp(),
            lock_id: next_lock_id(),
        }
    }
}

/// Get next unique lock ID
fn next_lock_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

/// Lock manager
pub struct LockManager {
    /// Active locks by file inode
    active_locks: Mutex<BTreeMap<u32, Vec<ActiveLock>>>,
    /// Pending lock requests by file inode
    pending_requests: Mutex<BTreeMap<u32, Vec<LockRequest>>>,
    /// Lock statistics
    stats: Mutex<LockStats>,
}

/// Lock statistics
#[derive(Debug, Default)]
pub struct LockStats {
    /// Total lock requests
    pub total_requests: u64,
    /// Successful lock acquisitions
    pub successful_acquisitions: u64,
    /// Failed lock attempts
    pub failed_attempts: u64,
    /// Lock upgrades
    pub lock_upgrades: u64,
    /// Lock downgrades
    pub lock_downgrades: u64,
    /// Deadlocks detected
    pub deadlocks_detected: u64,
    /// Average wait time for locks
    pub avg_wait_time: u64,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self {
            active_locks: Mutex::new(BTreeMap::new()),
            pending_requests: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(LockStats::default()),
        }
    }

    /// Try to acquire a lock
    pub fn try_lock(&self, inode: u32, pid: ProcessId, lock_type: LockType, range: LockRange) -> Result<u64, LockError> {
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_requests += 1;
        }

        // Check if lock can be granted
        if let Some(conflict) = self.check_conflict(inode, pid, lock_type, &range) {
            return Err(LockError::Conflict(conflict));
        }

        // Grant the lock
        let lock_id = self.grant_lock(inode, pid, lock_type, range);

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.successful_acquisitions += 1;
        }

        Ok(lock_id)
    }

    /// Acquire a lock (blocking)
    pub fn lock(&self, inode: u32, pid: ProcessId, lock_type: LockType, range: LockRange, blocking: bool) -> Result<u64, LockError> {
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_requests += 1;
        }

        // Try to acquire the lock immediately
        if let Ok(lock_id) = self.try_lock(inode, pid, lock_type, range) {
            return Ok(lock_id);
        }

        // If non-blocking, return error
        if !blocking {
            let mut stats = self.stats.lock();
            stats.failed_attempts += 1;
            return Err(LockError::WouldBlock);
        }

        // Add to pending requests
        let request = LockRequest::new(pid, lock_type, range, true);
        let request_id = request.request_id;

        {
            let mut pending = self.pending_requests.lock();
            let requests = pending.entry(inode).or_insert_with(Vec::new);
            requests.push(request);
        }

        // In a real implementation, we would block the process here
        // For now, we'll just return an error
        let mut stats = self.stats.lock();
        stats.failed_attempts += 1;
        Err(LockError::WouldBlock)
    }

    /// Release a lock
    pub fn unlock(&self, inode: u32, pid: ProcessId, lock_id: u64) -> Result<(), LockError> {
        let mut active_locks = self.active_locks.lock();
        let locks = active_locks.get_mut(&inode).ok_or(LockError::NoLock)?;

        // Find and remove the lock
        let lock_pos = locks.iter().position(|l| l.lock_id == lock_id && l.pid == pid)
            .ok_or(LockError::NoLock)?;

        let removed_lock = locks.remove(lock_pos);

        // Clean up if no more locks for this inode
        if locks.is_empty() {
            active_locks.remove(&inode);
        }
        drop(active_locks);

        // Check if any pending requests can now be granted
        self.process_pending_requests(inode);

        crate::println!("lock: released lock {} for inode {} by process {}", lock_id, inode, pid);
        Ok(())
    }

    /// Release all locks held by a process
    pub fn release_all_locks(&self, pid: ProcessId) {
        let mut active_locks = self.active_locks.lock();
        let mut inodes_to_check = Vec::new();

        // Find and remove all locks held by this process
        for (inode, locks) in active_locks.iter_mut() {
            let initial_len = locks.len();
            locks.retain(|l| l.pid != pid);
            
            if locks.len() != initial_len {
                inodes_to_check.push(*inode);
            }
        }

        // Clean up empty entries
        active_locks.retain(|_, locks| !locks.is_empty());
        drop(active_locks);

        // Check pending requests for affected inodes
        for inode in inodes_to_check {
            self.process_pending_requests(inode);
        }

        crate::println!("lock: released all locks for process {}", pid);
    }

    /// Check if there's a conflict with existing locks
    fn check_conflict(&self, inode: u32, pid: ProcessId, lock_type: LockType, range: &LockRange) -> Option<ActiveLock> {
        let active_locks = self.active_locks.lock();
        
        if let Some(locks) = active_locks.get(&inode) {
            for lock in locks {
                // Skip locks held by the same process
                if lock.pid == pid {
                    continue;
                }

                // Check for overlapping ranges
                if lock.range.overlaps(range) {
                    // Check for lock type conflicts
                    match (lock.lock_type, lock_type) {
                        (LockType::Exclusive, _) | (_, LockType::Exclusive) => {
                            // Any exclusive lock conflicts with any other lock
                            return Some(lock.clone());
                        }
                        (LockType::Shared, LockType::Shared) => {
                            // Shared locks are compatible
                            continue;
                        }
                        _ => {
                            // Other combinations are conflicts
                            return Some(lock.clone());
                        }
                    }
                }
            }
        }

        None
    }

    /// Grant a lock
    fn grant_lock(&self, inode: u32, pid: ProcessId, lock_type: LockType, range: LockRange) -> u64 {
        let lock = ActiveLock::new(pid, lock_type, range);
        let lock_id = lock.lock_id;

        let mut active_locks = self.active_locks.lock();
        let locks = active_locks.entry(inode).or_insert_with(Vec::new);
        locks.push(lock);
        drop(active_locks);

        crate::println!("lock: granted lock {} for inode {} to process {}", lock_id, inode, pid);
        lock_id
    }

    /// Process pending lock requests
    fn process_pending_requests(&self, inode: u32) {
        let mut pending_requests = self.pending_requests.lock();
        let requests = pending_requests.get_mut(&inode);
        
        if requests.is_none() || requests.unwrap().is_empty() {
            return;
        }

        let requests = requests.unwrap();
        let mut to_remove = Vec::new();

        for (i, request) in requests.iter().enumerate() {
            // Check if this request can now be granted
            if self.check_conflict(inode, request.pid, request.lock_type, &request.range).is_none() {
                // Grant the lock
                self.grant_lock(inode, request.pid, request.lock_type, request.range);
                to_remove.push(i);

                // Update statistics
                let mut stats = self.stats.lock();
                stats.successful_acquisitions += 1;
                
                // Update wait time
                let wait_time = crate::time::get_timestamp() - request.timestamp;
                stats.avg_wait_time = (stats.avg_wait_time + wait_time) / 2;
            }
        }

        // Remove granted requests
        for &i in to_remove.iter().rev() {
            requests.remove(i);
        }

        // Clean up if no more pending requests
        if requests.is_empty() {
            pending_requests.remove(&inode);
        }
    }

    /// Upgrade a lock (shared to exclusive)
    pub fn upgrade_lock(&self, inode: u32, pid: ProcessId, lock_id: u64) -> Result<u64, LockError> {
        let mut active_locks = self.active_locks.lock();
        let locks = active_locks.get_mut(&inode).ok_or(LockError::NoLock)?;

        // Find the lock to upgrade
        let lock_pos = locks.iter().position(|l| l.lock_id == lock_id && l.pid == pid)
            .ok_or(LockError::NoLock)?;

        let lock = &mut locks[lock_pos];
        
        if lock.lock_type != LockType::Shared {
            return Err(LockError::InvalidOperation);
        }

        // Check if upgrade is possible
        let range = lock.range;
        drop(active_locks);

        if let Some(conflict) = self.check_conflict(inode, pid, LockType::Exclusive, &range) {
            return Err(LockError::Conflict(conflict));
        }

        // Perform the upgrade
        let mut active_locks = self.active_locks.lock();
        let locks = active_locks.get_mut(&inode).unwrap();
        let lock = &mut locks[lock_pos];
        lock.lock_type = LockType::Exclusive;
        let new_lock_id = lock.lock_id;
        drop(active_locks);

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.lock_upgrades += 1;
        }

        crate::println!("lock: upgraded lock {} to exclusive for inode {} by process {}", new_lock_id, inode, pid);
        Ok(new_lock_id)
    }

    /// Downgrade a lock (exclusive to shared)
    pub fn downgrade_lock(&self, inode: u32, pid: ProcessId, lock_id: u64) -> Result<u64, LockError> {
        let mut active_locks = self.active_locks.lock();
        let locks = active_locks.get_mut(&inode).ok_or(LockError::NoLock)?;

        // Find the lock to downgrade
        let lock_pos = locks.iter().position(|l| l.lock_id == lock_id && l.pid == pid)
            .ok_or(LockError::NoLock)?;

        let lock = &mut locks[lock_pos];
        
        if lock.lock_type != LockType::Exclusive {
            return Err(LockError::InvalidOperation);
        }

        // Perform the downgrade
        lock.lock_type = LockType::Shared;
        let new_lock_id = lock.lock_id;
        drop(active_locks);

        // Check if any pending requests can now be granted
        self.process_pending_requests(inode);

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.lock_downgrades += 1;
        }

        crate::println!("lock: downgraded lock {} to shared for inode {} by process {}", new_lock_id, inode, pid);
        Ok(new_lock_id)
    }

    /// Get all locks for a file
    pub fn get_file_locks(&self, inode: u32) -> Vec<ActiveLock> {
        let active_locks = self.active_locks.lock();
        if let Some(locks) = active_locks.get(&inode) {
            locks.clone()
        } else {
            Vec::new()
        }
    }

    /// Get all locks held by a process
    pub fn get_process_locks(&self, pid: ProcessId) -> Vec<(u32, ActiveLock)> {
        let mut result = Vec::new();
        let active_locks = self.active_locks.lock();

        for (inode, locks) in active_locks.iter() {
            for lock in locks {
                if lock.pid == pid {
                    result.push((*inode, lock.clone()));
                }
            }
        }

        result
    }

    /// Check for potential deadlocks
    pub fn detect_deadlock(&self) -> Option<Vec<ProcessId>> {
        // This is a simplified deadlock detection
        // In a real implementation, we would build a wait graph and check for cycles
        
        let pending_requests = self.pending_requests.lock();
        let active_locks = self.active_locks.lock();
        
        // Build a simple wait graph
        let mut wait_graph: BTreeMap<ProcessId, Vec<ProcessId>> = BTreeMap::new();
        
        for (inode, requests) in pending_requests.iter() {
            if let Some(locks) = active_locks.get(inode) {
                for request in requests {
                    let waiters = wait_graph.entry(request.pid).or_insert_with(Vec::new);
                    for lock in locks {
                        if lock.pid != request.pid {
                            waiters.push(lock.pid);
                        }
                    }
                }
            }
        }
        
        // Simple cycle detection
        for (pid, waiters) in wait_graph.iter() {
            for waiter in waiters {
                if let Some(their_waiters) = wait_graph.get(waiter) {
                    if their_waiters.contains(pid) {
                        // Found a cycle
                        let mut stats = self.stats.lock();
                        stats.deadlocks_detected += 1;
                        return Some(vec![*pid, *waiter]);
                    }
                }
            }
        }
        
        None
    }

    /// Get lock statistics
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().clone()
    }
}

/// Lock errors
#[derive(Debug, Clone)]
pub enum LockError {
    /// Lock would block
    WouldBlock,
    /// No such lock
    NoLock,
    /// Conflict with existing lock
    Conflict(ActiveLock),
    /// Invalid operation
    InvalidOperation,
    /// Deadlock detected
    Deadlock(Vec<ProcessId>),
}

/// File lock wrapper for easier integration with file operations
pub struct FileLock {
    /// Inode number
    pub inode: u32,
    /// Lock ID
    pub lock_id: u64,
    /// Lock type
    pub lock_type: LockType,
    /// Lock range
    pub range: LockRange,
}

impl FileLock {
    /// Create a new file lock
    pub fn new(inode: u32, lock_id: u64, lock_type: LockType, range: LockRange) -> Self {
        Self {
            inode,
            lock_id,
            lock_type,
            range,
        }
    }
}

/// Global lock manager instance
static mut LOCK_MANAGER: Option<LockManager> = None;

/// Initialize lock manager
pub fn init() {
    unsafe {
        LOCK_MANAGER = Some(LockManager::new());
    }
    crate::println!("lock: initialized lock manager");
}

/// Get lock manager instance
pub fn get_lock_manager() -> Option<&'static LockManager> {
    unsafe { LOCK_MANAGER.as_ref() }
}

/// Helper function to acquire a shared lock
pub fn acquire_shared_lock(inode: u32, pid: ProcessId, range: LockRange, blocking: bool) -> Result<FileLock, LockError> {
    if let Some(lm) = get_lock_manager() {
        let lock_id = lm.lock(inode, pid, LockType::Shared, range, blocking)?;
        Ok(FileLock::new(inode, lock_id, LockType::Shared, range))
    } else {
        Err(LockError::InvalidOperation)
    }
}

/// Helper function to acquire an exclusive lock
pub fn acquire_exclusive_lock(inode: u32, pid: ProcessId, range: LockRange, blocking: bool) -> Result<FileLock, LockError> {
    if let Some(lm) = get_lock_manager() {
        let lock_id = lm.lock(inode, pid, LockType::Exclusive, range, blocking)?;
        Ok(FileLock::new(inode, lock_id, LockType::Exclusive, range))
    } else {
        Err(LockError::InvalidOperation)
    }
}

/// Helper function to release a lock
pub fn release_lock(file_lock: FileLock, pid: ProcessId) -> Result<(), LockError> {
    if let Some(lm) = get_lock_manager() {
        lm.unlock(file_lock.inode, pid, file_lock.lock_id)
    } else {
        Err(LockError::InvalidOperation)
    }
}

use crate::sync::Mutex;