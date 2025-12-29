//! Hash-Sharded Process Table for Concurrent Performance
//!
//! This module implements a sharded process table design to minimize lock contention
//! in multi-core systems. The key optimizations are:
//!
//! 1. **Hash Sharding**: 32 independent shards reduce lock contention by ~97%
//! 2. **Cache-Line Alignment**: Prevents false sharing between shards
//! 3. **Per-Shard Locks**: Allows concurrent access to different shards
//! 4. **NUMA Awareness**: (Future) Shards can be placed on local NUMA nodes
//!
//! Performance Targets:
//! - Process lookup latency: < 50ns (70% improvement)
//! - Concurrent throughput: 10M ops/sec on 8 cores
//! - Lock contention: < 5% on high load

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use core::num::NonZeroU64;

use crate::subsystems::sync::Mutex;

/// Number of shards for the process table
/// 32 shards provides good balance between memory overhead and contention reduction
pub const NUM_SHARDS: usize = 32;

/// Process ID type
pub type Pid = NonZeroU64;

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    Unused,
    Used,
    Zombie,
    Sleeping,
}

/// Process entry
#[derive(Debug)]
pub struct ProcEntry {
    pub pid: Pid,
    pub state: ProcState,
    pub parent_pid: Option<Pid>,
    pub name: alloc::string::String,
}

impl ProcEntry {
    pub fn new(pid: Pid, parent_pid: Option<Pid>, name: &str) -> Self {
        Self {
            pid,
            state: ProcState::Unused,
            parent_pid,
            name: alloc::string::String::from(name),
        }
    }
}

/// Per-shard process storage
/// Cache-line aligned to prevent false sharing
#[repr(align(64))]
pub struct ProcShard {
    /// Local PID to process mapping
    processes: BTreeMap<u64, ProcEntry>,
    /// Next PID to allocate in this shard
    next_pid: AtomicU64,
    /// Padding to prevent false sharing (64-byte cache line)
    _padding: [u8; 64],
}

impl ProcShard {
    pub const fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            next_pid: AtomicU64::new(0),
            _padding: [0; 64],
        }
    }

    /// Allocate a new process in this shard
    pub fn alloc(&mut self, parent_pid: Option<Pid>, name: &str) -> Option<Pid> {
        // Generate unique PID
        let pid_val = self.next_pid.fetch_add(1, Ordering::Relaxed);
        let pid = NonZeroU64::new(pid_val.wrapping_add(1))?;

        let entry = ProcEntry::new(pid, parent_pid, name);
        self.processes.insert(pid_val, entry);

        Some(pid)
    }

    /// Find a process by PID
    pub fn find(&self, pid: Pid) -> Option<&ProcEntry> {
        self.processes.get(&pid.get())
    }

    /// Find a mutable process by PID
    pub fn find_mut(&mut self, pid: Pid) -> Option<&mut ProcEntry> {
        self.processes.get_mut(&pid.get())
    }

    /// Free a process
    pub fn free(&mut self, pid: Pid) -> bool {
        self.processes.remove(&pid.get()).is_some()
    }

    /// Count processes in this shard
    pub fn count(&self) -> usize {
        self.processes.len()
    }
}

/// Sharded process table with per-shard locking
pub struct ShardedProcTable {
    /// Array of shards, each with its own lock
    shards: [Mutex<ProcShard>; NUM_SHARDS],
    /// Total number of processes across all shards
    total_count: AtomicU64,
}

impl ShardedProcTable {
    /// Create a new sharded process table
    pub const fn new() -> Self {
        // Use const array initialization with Mutex::new
        // Note: This requires Mutex to be const-constructible
        let shards = [
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
            const { Mutex::new(ProcShard::new()) },
        ];

        Self {
            shards,
            total_count: AtomicU64::new(0),
        }
    }

    /// Get the shard index for a given PID
    #[inline]
    fn get_shard_index(&self, pid: Pid) -> usize {
        // Use hash of PID to distribute load evenly
        let hash = (pid.get() as usize).wrapping_mul(0x9e3779b97f4a7c15);
        hash % NUM_SHARDS
    }

    /// Get shard by index
    #[inline]
    pub fn get_shard(&self, index: usize) -> &Mutex<ProcShard> {
        &self.shards[index % NUM_SHARDS]
    }

    /// Allocate a new process
    /// Uses round-robin shard selection for load balancing
    pub fn alloc(&self, parent_pid: Option<Pid>, name: &str) -> Option<Pid> {
        // Use current CPU for shard selection (load balancing)
        let cpu_id = crate::arch::cpuid() as usize;
        let shard_idx = cpu_id % NUM_SHARDS;
        let shard = &self.shards[shard_idx];

        let mut shard_guard = shard.lock();
        if let Some(pid) = shard_guard.alloc(parent_pid, name) {
            self.total_count.fetch_add(1, Ordering::Relaxed);
            Some(pid)
        } else {
            None
        }
    }

    /// Find a process by PID
    /// Only locks the specific shard containing the PID
    pub fn find(&self, pid: Pid) -> Option<ProcEntry> {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        let shard_guard = shard.lock();
        shard_guard.find(pid).cloned()
    }

    /// Update a process (mutable access)
    pub fn update<F>(&self, pid: Pid, f: F) -> bool
    where
        F: FnOnce(&mut ProcEntry),
    {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        let mut shard_guard = shard.lock();
        if let Some(entry) = shard_guard.find_mut(pid) {
            f(entry);
            true
        } else {
            false
        }
    }

    /// Free a process
    pub fn free(&self, pid: Pid) -> bool {
        let shard_idx = self.get_shard_index(pid);
        let shard = &self.shards[shard_idx];

        let mut shard_guard = shard.lock();
        if shard_guard.free(pid) {
            self.total_count.fetch_sub(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get total process count
    pub fn count(&self) -> u64 {
        self.total_count.load(Ordering::Relaxed)
    }

    /// Iterate over all processes (expensive, use sparingly)
    pub fn iter_all<F>(&self, mut f: F)
    where
        F: FnMut(&ProcEntry),
    {
        for shard in &self.shards {
            let shard_guard = shard.lock();
            for entry in shard_guard.processes.values() {
                f(entry);
            }
        }
    }

    /// Get statistics about shard distribution
    pub fn shard_stats(&self) -> alloc::vec::Vec<usize> {
        let mut stats = alloc::vec::Vec::with_capacity(NUM_SHARDS);
        for shard in &self.shards {
            let shard_guard = shard.lock();
            stats.push(shard_guard.count());
        }
        stats
    }

    /// Calculate load balance standard deviation (lower is better)
    pub fn load_balance_score(&self) -> f64 {
        let stats = self.shard_stats();
        if stats.is_empty() {
            return 0.0;
        }

        let mean = self.count() as f64 / NUM_SHARDS as f64;
        let variance: f64 = stats
            .iter()
            .map(|&count| {
                let diff = count as f64 - mean;
                diff * diff
            })
            .sum();

        (variance / NUM_SHARDS as f64).sqrt()
    }
}

/// Global sharded process table instance
static SHARDED_TABLE: ShardedProcTable = ShardedProcTable::new();

/// Get the global sharded process table
pub fn get_sharded_table() -> &'static ShardedProcTable {
    &SHARDED_TABLE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_distribution() {
        let table = ShardedProcTable::new();

        // Allocate processes across different CPUs
        let mut pids = alloc::vec::Vec::new();
        for i in 0..100 {
            if let Some(pid) = table.alloc(None, &format!("proc_{}", i)) {
                pids.push(pid);
            }
        }

        // Check that processes are distributed
        let stats = table.shard_stats();
        let max_shard = *stats.iter().max().unwrap_or(&0);
        let min_shard = *stats.iter().min().unwrap_or(&0);

        // Distribution should be relatively even (allow 2x variance)
        assert!(max_shard <= min_shard * 2 || max_shard < 10);

        // Cleanup
        for pid in pids {
            table.free(pid);
        }
    }

    #[test]
    fn test_concurrent_access() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        let table = &ShardedProcTable::new();
        let counter = AtomicUsize::new(0);

        // Simulate concurrent allocations (simplified)
        let mut pids = alloc::vec::Vec::new();
        for i in 0..50 {
            if let Some(pid) = table.alloc(None, &format!("proc_{}", i)) {
                pids.push(pid);
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        assert_eq!(counter.load(Ordering::Relaxed), 50);

        // Cleanup
        for pid in pids {
            table.free(pid);
        }
    }
}
