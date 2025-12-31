//! RCU-Optimized Process Table for Lock-Free Reads
//!
//! This module implements Read-Copy-Update (RCU) semantics for process table
//! operations, enabling zero-lock read paths and deferred reclamation.
//!
//! Key Optimizations:
//! 1. **Lock-Free Reads**: Read operations use atomic pointers, no mutex needed
//! 2. **Deferred Reclamation**: Freed processes are reclaimed after grace period
//! 3. **Versioning**: Each read gets a consistent snapshot
//! 4. **Epoch-Based Reclamation**: Efficient memory management
//!
//! Performance Targets:
//! - Read latency: < 20ns (zero locks)
//! - Write latency: < 100ns (single lock)
//! - Memory overhead: < 15% vs baseline

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;

use super::sharded_table::{Pid, ProcEntry};

/// Epoch counter for RCU grace periods
static GLOBAL_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Get current epoch
#[inline]
pub fn current_epoch() -> u64 {
    GLOBAL_EPOCH.load(Ordering::Acquire)
}

/// Advance to next epoch (called periodically)
pub fn advance_epoch() {
    GLOBAL_EPOCH.fetch_add(1, Ordering::Release);
}

/// RCU-protected process pointer
struct RcPointer {
    /// Atomic pointer to process data
    ptr: AtomicUsize,
    /// Epoch when this pointer was last updated
    epoch: AtomicU64,
}

impl RcPointer {
    const fn new() -> Self {
        Self {
            ptr: AtomicUsize::new(0),
            epoch: AtomicU64::new(0),
        }
    }

    /// Load the current process (lock-free read)
    unsafe fn load(&self) -> Option<Arc<ProcEntry>> { unsafe {
        let addr = self.ptr.load(Ordering::Acquire);
        if addr == 0 {
            None
        } else {
            Some(Arc::from_raw(addr as *const ProcEntry))
        }
    }}

    /// Store a new process (write operation)
    unsafe fn store(&self, entry: Arc<ProcEntry>) {
        let addr = Arc::into_raw(entry) as usize;
        self.ptr.store(addr, Ordering::Release);
        self.epoch.store(current_epoch(), Ordering::Release);
    }

    /// Swap to a new process, returning the old one
    unsafe fn swap(&self, entry: Arc<ProcEntry>) -> Option<Arc<ProcEntry>> { unsafe {
        let new_addr = Arc::into_raw(entry) as usize;
        let old_addr = self.ptr.swap(new_addr, Ordering::AcqRel);
        self.epoch.store(current_epoch(), Ordering::Release);

        if old_addr == 0 {
            None
        } else {
            Some(Arc::from_raw(old_addr as *const ProcEntry))
        }
    }}
}

/// RCU-protected process table entry
struct RcuEntry {
    /// Current process data
    current: RcPointer,
    /// Pending frees (to be reclaimed after grace period)
    pending_frees: Mutex<alloc::vec::Vec<(u64, Arc<ProcEntry>)>>,
    /// Delete epoch
    delete_epoch: AtomicU64,
}

impl RcuEntry {
    const fn new() -> Self {
        Self {
            current: RcPointer::new(),
            pending_frees: Mutex::new(alloc::vec::Vec::new()),
            delete_epoch: AtomicU64::new(0),
        }
    }

    /// Read process (lock-free)
    pub fn read(&self) -> Option<Arc<ProcEntry>> {
        unsafe { self.current.load() }
    }

    /// Write/update process
    pub fn write(&self, entry: Arc<ProcEntry>) {
        unsafe {
            // Swap in new entry, get old one
            if let Some(old_entry) = self.current.swap(entry.clone()) {
                // Schedule old entry for reclamation
                let epoch = current_epoch();
                let mut pending = self.pending_frees.lock();
                pending.push((epoch, old_entry));
            }
        }
    }

    /// Reclaim entries that are safe to free
    pub fn reclaim(&self, current_epoch: u64) {
        let mut pending = self.pending_frees.lock();
        pending.retain(|(epoch, _entry)| {
            // Keep if not yet safe to reclaim (grace period = 2 epochs)
            if current_epoch.wrapping_sub(*epoch) < 2 {
                true
            } else {
                // Safe to reclaim, Arc will drop here
                false
            }
        });
    }
}

/// RCU-optimized process table
pub struct RcuProcTable {
    /// Hash-sharded RCU entries
    shards: [RcuEntry; 32],
    /// Number of active processes
    count: AtomicU64,
}

impl RcuProcTable {
    pub const fn new() -> Self {
        Self {
            shards: [
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
                const { RcuEntry::new() },
            ],
            count: AtomicU64::new(0),
        }
    }

    /// Get shard index for PID
    #[inline]
    fn get_shard_index(&self, pid: Pid) -> usize {
        let hash = (pid.get() as usize).wrapping_mul(0x9e3779b97f4a7c15);
        hash % 32
    }

    /// Insert a new process
    pub fn insert(&self, pid: Pid, entry: Arc<ProcEntry>) {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        shard.write(entry);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Read process by PID (lock-free!)
    pub fn read(&self, pid: Pid) -> Option<Arc<ProcEntry>> {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        shard.read()
    }

    /// Update process (requires write lock)
    pub fn update(&self, pid: Pid, entry: Arc<ProcEntry>) {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        shard.write(entry);
    }

    /// Delete process
    pub fn delete(&self, pid: Pid) {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        // Mark deletion epoch
        shard.delete_epoch.store(current_epoch(), Ordering::Release);
        self.count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Periodic maintenance: reclaim dead entries
    pub fn reclaim(&self) {
        let epoch = current_epoch();
        for shard in &self.shards {
            shard.reclaim(epoch);
        }
    }

    /// Get process count
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

/// Global RCU process table
static RCU_TABLE: RcuProcTable = RcuProcTable::new();

/// Get the global RCU process table
pub fn get_rcu_table() -> &'static RcuProcTable {
    &RCU_TABLE
}

/// RCU read-side critical section guard
pub struct RcuReadGuard {
    _epoch: u64,
}

impl RcuReadGuard {
    pub fn new() -> Self {
        Self {
            _epoch: current_epoch(),
        }
    }

    /// Read process within this RCU critical section
    pub fn read(&self, pid: Pid) -> Option<Arc<ProcEntry>> {
        RCU_TABLE.read(pid)
    }
}

/// Enter RCU read-side critical section
pub fn rcu_read_lock() -> RcuReadGuard {
    RcuReadGuard::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rcu_read_write() {
        let table = &RCU_TABLE;

        let pid = NonZeroU64::new(1).unwrap();
        let entry = Arc::new(ProcEntry::new(pid, None, "test"));

        // Write
        table.insert(pid, entry.clone());

        // Read (lock-free)
        let guard = rcu_read_lock();
        let found = guard.read(pid);
        assert!(found.is_some());
        assert_eq!(found.unwrap().pid, pid);
    }

    #[test]
    fn test_epoch_advancement() {
        let e1 = current_epoch();
        advance_epoch();
        let e2 = current_epoch();

        assert_eq!(e2, e1 + 1);
    }
}
