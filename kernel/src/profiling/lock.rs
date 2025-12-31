//! Lock Contention Profiling
//!
//! This module provides lock contention profiling capabilities for the NOS kernel,
//! including hold time tracking, wait time tracking, and deadlock detection.
//!
//! # Features
//!
//! - Lock contention profiling
//! - Hold time tracking
//! - Wait time tracking
//! - Lock dependency graph
//! - Deadlock detection
//! - Contention analysis by lock and thread
//!
//! # Usage
//!
//! ```rust
//! use kernel::profiling::lock::LockProfiler;
//!
//! let profiler = LockProfiler::new();
//! profiler.start().unwrap();
//! // Lock operations will be automatically tracked
//! profiler.stop().unwrap();
//! let report = profiler.generate_report().unwrap();
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use crate::sync::Mutex;

/// Lock profiling error types
#[derive(Debug, Clone, PartialEq)]
pub enum LockProfileError {
    /// Profiler is already running
    AlreadyRunning,
    /// Profiler is not running
    NotRunning,
    /// Lock not found
    LockNotFound(u64),
    /// Cycle detected in dependency graph
    CycleDetected,
    /// Buffer overflow
    BufferOverflow,
}

impl core::fmt::Display for LockProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "Lock profiler is already running"),
            Self::NotRunning => write!(f, "Lock profiler is not running"),
            Self::LockNotFound(id) => write!(f, "Lock not found: {}", id),
            Self::CycleDetected => write!(f, "Cycle detected in lock dependency graph"),
            Self::BufferOverflow => write!(f, "Profile buffer overflow"),
        }
    }
}

/// Unique lock identifier
pub type LockId = u64;

/// Thread identifier
pub type ThreadId = u64;

/// Lock operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOperation {
    /// Lock acquisition attempt
    Lock,
    /// Lock acquisition success
    Acquired,
    /// Lock release
    Unlock,
    /// Failed acquisition attempt
    TryLockFailed,
}

/// Lock event record
#[derive(Debug, Clone)]
pub struct LockEvent {
    /// Event timestamp
    pub timestamp: u64,
    /// Lock identifier
    pub lock_id: LockId,
    /// Thread identifier
    pub thread_id: ThreadId,
    /// Operation type
    pub operation: LockOperation,
    /// Call site address
    pub call_site: u64,
    /// Stack trace (optional)
    pub stack_trace: Option<Vec<u64>>,
}

impl LockEvent {
    /// Create a new lock event
    pub fn new(
        lock_id: LockId,
        thread_id: ThreadId,
        operation: LockOperation,
        call_site: u64,
    ) -> Self {
        Self {
            timestamp: Self::now(),
            lock_id,
            thread_id,
            operation,
            call_site,
            stack_trace: None,
        }
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }
}

/// Statistics for a single lock
#[derive(Debug)]
pub struct LockStatistics {
    /// Lock identifier
    pub lock_id: LockId,
    /// Total number of acquisitions
    pub total_acquisitions: AtomicU64,
    /// Total number of releases
    pub total_releases: AtomicU64,
    /// Total number of contentions (failed acquisitions)
    pub total_contentions: AtomicU64,
    /// Total wait time (nanoseconds)
    pub total_wait_time: AtomicU64,
    /// Total hold time (nanoseconds)
    pub total_hold_time: AtomicU64,
    /// Maximum wait time (nanoseconds)
    pub max_wait_time: AtomicU64,
    /// Maximum hold time (nanoseconds)
    pub max_hold_time: AtomicU64,
    /// Current holder (if locked)
    pub current_holder: Mutex<Option<ThreadId>>,
    /// Acquisition timestamp (for hold time calculation)
    pub acquire_timestamp: Mutex<Option<u64>>,
    /// Threads waiting for this lock
    pub waiting_threads: Mutex<BTreeSet<ThreadId>>,
}

impl Clone for LockStatistics {
    fn clone(&self) -> Self {
        Self {
            lock_id: self.lock_id,
            total_acquisitions: AtomicU64::new(self.total_acquisitions.load(Ordering::Relaxed)),
            total_releases: AtomicU64::new(self.total_releases.load(Ordering::Relaxed)),
            total_contentions: AtomicU64::new(self.total_contentions.load(Ordering::Relaxed)),
            total_wait_time: AtomicU64::new(self.total_wait_time.load(Ordering::Relaxed)),
            total_hold_time: AtomicU64::new(self.total_hold_time.load(Ordering::Relaxed)),
            max_wait_time: AtomicU64::new(self.max_wait_time.load(Ordering::Relaxed)),
            max_hold_time: AtomicU64::new(self.max_hold_time.load(Ordering::Relaxed)),
            current_holder: Mutex::new(self.current_holder.lock().clone()),
            acquire_timestamp: Mutex::new(self.acquire_timestamp.lock().clone()),
            waiting_threads: Mutex::new(self.waiting_threads.lock().clone()),
        }
    }
}

impl LockStatistics {
    /// Create new lock statistics
    pub fn new(lock_id: LockId) -> Self {
        Self {
            lock_id,
            total_acquisitions: AtomicU64::new(0),
            total_releases: AtomicU64::new(0),
            total_contentions: AtomicU64::new(0),
            total_wait_time: AtomicU64::new(0),
            total_hold_time: AtomicU64::new(0),
            max_wait_time: AtomicU64::new(0),
            max_hold_time: AtomicU64::new(0),
            current_holder: Mutex::new(None),
            acquire_timestamp: Mutex::new(None),
            waiting_threads: Mutex::new(BTreeSet::new()),
        }
    }

    /// Record a lock acquisition
    pub fn record_acquisition(&self, thread_id: ThreadId, wait_time_ns: u64) {
        self.total_acquisitions.fetch_add(1, Ordering::Relaxed);
        self.total_wait_time.fetch_add(wait_time_ns, Ordering::Relaxed);

        // Update max wait time
        let mut max = self.max_wait_time.load(Ordering::Relaxed);
        while wait_time_ns > max {
            match self.max_wait_time.compare_exchange_weak(
                max,
                wait_time_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }

        // Set current holder and timestamp
        *self.current_holder.lock() = Some(thread_id);
        *self.acquire_timestamp.lock() = Some(LockEvent::now());
    }

    /// Record a lock release
    pub fn record_release(&self, _thread_id: ThreadId, hold_time_ns: u64) {
        self.total_releases.fetch_add(1, Ordering::Relaxed);
        self.total_hold_time.fetch_add(hold_time_ns, Ordering::Relaxed);

        // Update max hold time
        let max = self.max_hold_time.load(Ordering::Relaxed);
        while hold_time_ns > max {
            match self.max_hold_time.compare_exchange_weak(
                max,
                hold_time_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(mut new_max) => new_max = max,
            }
        }

        // Clear current holder and timestamp
        *self.current_holder.lock() = None;
        *self.acquire_timestamp.lock() = None;
    }

    /// Record a contention event
    pub fn record_contention(&self) {
        self.total_contentions.fetch_add(1, Ordering::Relaxed);
    }

    /// Add a waiting thread
    pub fn add_waiting_thread(&self, thread_id: ThreadId) {
        self.waiting_threads.lock().insert(thread_id);
    }

    /// Remove a waiting thread
    pub fn remove_waiting_thread(&self, thread_id: ThreadId) {
        self.waiting_threads.lock().remove(&thread_id);
    }

    /// Get average wait time
    pub fn avg_wait_time(&self) -> u64 {
        let acquisitions = self.total_acquisitions.load(Ordering::Relaxed);
        let total_wait = self.total_wait_time.load(Ordering::Relaxed);
        if acquisitions > 0 {
            total_wait / acquisitions
        } else {
            0
        }
    }

    /// Get average hold time
    pub fn avg_hold_time(&self) -> u64 {
        let releases = self.total_releases.load(Ordering::Relaxed);
        let total_hold = self.total_hold_time.load(Ordering::Relaxed);
        if releases > 0 {
            total_hold / releases
        } else {
            0
        }
    }

    /// Get contention rate
    pub fn contention_rate(&self) -> f64 {
        let acquisitions = self.total_acquisitions.load(Ordering::Relaxed);
        let contentions = self.total_contentions.load(Ordering::Relaxed);
        if acquisitions > 0 {
            contentions as f64 / acquisitions as f64
        } else {
            0.0
        }
    }
}

/// Lock dependency graph edge
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyEdge {
    /// Source lock (held first)
    pub from_lock: LockId,
    /// Target lock (acquired second)
    pub to_lock: LockId,
    /// Thread that acquired both locks
    pub thread_id: ThreadId,
    /// Number of times this edge was traversed
    pub count: u64,
}

/// Lock profiler configuration
#[derive(Debug, Clone)]
pub struct LockProfilerConfig {
    /// Track individual lock events
    pub track_events: bool,
    /// Maximum events to store
    pub max_events: usize,
    /// Detect deadlocks automatically
    pub detect_deadlocks: bool,
    /// Capture stack traces for lock operations
    pub capture_stack_traces: bool,
    /// Maximum stack depth
    pub max_stack_depth: usize,
}

impl Default for LockProfilerConfig {
    fn default() -> Self {
        Self {
            track_events: true,
            max_events: 100_000,
            detect_deadlocks: true,
            capture_stack_traces: false,
            max_stack_depth: 16,
        }
    }
}

/// Lock profiler state
struct LockProfilerState {
    running: bool,
    start_time: Option<u64>,
    stop_time: Option<u64>,
}

/// Contention analysis result
#[derive(Debug, Clone)]
pub struct ContentionAnalysis {
    /// Lock identifier
    pub lock_id: LockId,
    /// Contention score (higher is worse)
    pub contention_score: f64,
    /// Total contentions
    pub total_contentions: u64,
    /// Average wait time (ns)
    pub avg_wait_time: u64,
    /// Maximum wait time (ns)
    pub max_wait_time: u64,
    /// Top contending threads
    pub top_contenders: Vec<(ThreadId, u64)>,
}

/// Deadlock detection result
#[derive(Debug, Clone)]
pub struct DeadlockInfo {
    /// Threads involved in deadlock
    pub threads: Vec<ThreadId>,
    /// Locks in deadlock cycle
    pub locks: Vec<LockId>,
    /// Cycle length
    pub cycle_length: usize,
}

/// Lock profiling report
#[derive(Debug, Clone)]
pub struct LockProfileReport {
    /// Duration of profiling
    pub duration: Duration,
    /// Total lock events
    pub total_events: u64,
    /// Lock statistics
    pub lock_stats: BTreeMap<LockId, LockStatistics>,
    /// Dependency graph
    pub dependency_graph: Vec<DependencyEdge>,
    /// Contentions by lock
    pub contentions: Vec<ContentionAnalysis>,
    /// Deadlocks detected
    pub deadlocks: Vec<DeadlockInfo>,
    /// Raw events
    pub events: Vec<LockEvent>,
}

/// Lock profiler implementation
pub struct LockProfiler {
    config: LockProfilerConfig,
    state: Mutex<LockProfilerState>,
    locks: Mutex<BTreeMap<LockId, LockStatistics>>,
    events: Mutex<Vec<LockEvent>>,
    dependency_graph: Mutex<Vec<DependencyEdge>>,
    held_locks: Mutex<BTreeMap<ThreadId, Vec<LockId>>>, // Per-thread held locks
    next_lock_id: AtomicU64,
}

impl LockProfiler {
    /// Create a new lock profiler
    pub fn new() -> Self {
        Self::with_config(LockProfilerConfig::default())
    }

    /// Create profiler with custom configuration
    pub fn with_config(config: LockProfilerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(LockProfilerState {
                running: false,
                start_time: None,
                stop_time: None,
            }),
            locks: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::with_capacity(10_000)),
            dependency_graph: Mutex::new(Vec::new()),
            held_locks: Mutex::new(BTreeMap::new()),
            next_lock_id: AtomicU64::new(1),
        }
    }

    /// Start profiling
    pub fn start(&self) -> Result<(), LockProfileError> {
        let mut state = self.state.lock();

        if state.running {
            return Err(LockProfileError::AlreadyRunning);
        }

        state.running = true;
        state.start_time = Some(Self::now());

        Ok(())
    }

    /// Stop profiling
    pub fn stop(&self) -> Result<(), LockProfileError> {
        let mut state = self.state.lock();

        if !state.running {
            return Err(LockProfileError::NotRunning);
        }

        state.running = false;
        state.stop_time = Some(Self::now());

        Ok(())
    }

    /// Check if profiler is running
    pub fn is_running(&self) -> bool {
        self.state.lock().running
    }

    /// Register a lock for profiling
    pub fn register_lock(&self) -> LockId {
        let id = self.next_lock_id.fetch_add(1, Ordering::Relaxed);
        let stats = LockStatistics::new(id);
        self.locks.lock().insert(id, stats);
        id
    }

    /// Unregister a lock
    pub fn unregister_lock(&self, lock_id: LockId) {
        self.locks.lock().remove(&lock_id);
    }

    /// Record a lock acquisition attempt
    pub fn record_lock(
        &self,
        lock_id: LockId,
        thread_id: ThreadId,
        call_site: u64,
    ) -> Result<(), LockProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(());
        }
        drop(state);

        // Record event
        let event = LockEvent::new(lock_id, thread_id, LockOperation::Lock, call_site);
        self.add_event(event);

        // Add to waiting threads
        let locks = self.locks.lock();
        if let Some(stats) = locks.get(&lock_id) {
            let stats: &LockStatistics = stats;
            stats.add_waiting_thread(thread_id);
        }

        Ok(())
    }

    /// Record successful lock acquisition
    pub fn record_acquired(
        &self,
        lock_id: LockId,
        thread_id: ThreadId,
        wait_time_ns: u64,
        call_site: u64,
    ) -> Result<(), LockProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(());
        }
        drop(state);

        // Record event
        let event = LockEvent::new(lock_id, thread_id, LockOperation::Acquired, call_site);
        self.add_event(event);

        // Update statistics
        let locks = self.locks.lock();
        if let Some(stats) = locks.get(&lock_id) {
            let stats: &LockStatistics = stats;
            stats.remove_waiting_thread(thread_id);
            stats.record_acquisition(thread_id, wait_time_ns);
        }

        // Update held locks
        let mut held = self.held_locks.lock();
        held.entry(thread_id).or_insert_with(Vec::new).push(lock_id);

        Ok(())
    }

    /// Record lock release
    pub fn record_unlock(
        &self,
        lock_id: LockId,
        thread_id: ThreadId,
        call_site: u64,
    ) -> Result<(), LockProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(());
        }
        drop(state);

        // Calculate hold time
        let hold_time_ns = {
            let locks = self.locks.lock();
            if let Some(stats) = locks.get(&lock_id) {
                let timestamp = stats.acquire_timestamp.lock();
                if let Some(ts) = *timestamp {
                    LockEvent::now().saturating_sub(ts)
                } else {
                    0
                }
            } else {
                0
            }
        };

        // Record event
        let event = LockEvent::new(lock_id, thread_id, LockOperation::Unlock, call_site);
        self.add_event(event);

        // Update statistics
        let locks = self.locks.lock();
        if let Some(stats) = locks.get(&lock_id) {
            let stats: &LockStatistics = stats;
            stats.record_release(thread_id, hold_time_ns);
        }

        // Update held locks
        let mut held = self.held_locks.lock();
        if let Some(locks_vec) = held.get_mut(&thread_id) {
            let locks_vec: &mut Vec<LockId> = locks_vec;
            locks_vec.retain(|&id| id != lock_id);
        }

        Ok(())
    }

    /// Record failed lock acquisition
    pub fn record_trylock_failed(
        &self,
        lock_id: LockId,
        thread_id: ThreadId,
        call_site: u64,
    ) -> Result<(), LockProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(());
        }
        drop(state);

        // Record event
        let event = LockEvent::new(lock_id, thread_id, LockOperation::TryLockFailed, call_site);
        self.add_event(event);

        // Record contention
        let locks = self.locks.lock();
        if let Some(stats) = locks.get(&lock_id) {
            let stats: &LockStatistics = stats;
            stats.record_contention();
        }

        Ok(())
    }

    /// Record lock dependency (thread holds lock A and tries to acquire lock B)
    pub fn record_dependency(
        &self,
        from_lock: LockId,
        to_lock: LockId,
        thread_id: ThreadId,
    ) -> Result<(), LockProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(());
        }
        drop(state);

        let mut graph = self.dependency_graph.lock();

        // Find existing edge or create new one
        if let Some(edge) = graph.iter_mut().find(|e| {
            e.from_lock == from_lock && e.to_lock == to_lock && e.thread_id == thread_id
        }) {
            edge.count += 1;
        } else {
            graph.push(DependencyEdge {
                from_lock,
                to_lock,
                thread_id,
                count: 1,
            });
        }

        // Check for deadlock if enabled
        if self.config.detect_deadlocks {
            self.detect_deadlock_internal(&graph);
        }

        Ok(())
    }

    /// Detect deadlock in dependency graph
    fn detect_deadlock_internal(&self, graph: &[DependencyEdge]) -> Result<(), LockProfileError> {
        // Build adjacency list
        let mut adj: BTreeMap<LockId, Vec<LockId>> = BTreeMap::new();
        for edge in graph {
            adj.entry(edge.from_lock).or_insert_with(Vec::new).push(edge.to_lock);
        }

        // Detect cycles using DFS
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();

        for &node in adj.keys() {
            if self.has_cycle(&adj, node, &mut visited, &mut rec_stack) {
                return Err(LockProfileError::CycleDetected);
            }
        }

        Ok(())
    }

    /// DFS cycle detection helper
    fn has_cycle(
        &self,
        adj: &BTreeMap<LockId, Vec<LockId>>,
        node: LockId,
        visited: &mut BTreeSet<LockId>,
        rec_stack: &mut BTreeSet<LockId>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = adj.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if self.has_cycle(adj, neighbor, visited, rec_stack) {
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

    /// Generate profiling report
    pub fn generate_report(&self) -> Result<LockProfileReport, LockProfileError> {
        let state = self.state.lock();
        let start_time = state.start_time.unwrap_or(0);
        let stop_time = state.stop_time.unwrap_or_else(|| Self::now());
        drop(state);

        let duration = Duration::from_nanos(stop_time.saturating_sub(start_time));

        let locks = self.locks.lock();
        let events = self.events.lock();
        let graph = self.dependency_graph.lock();

        let lock_stats = locks.clone();
        let total_events = events.len() as u64;
        let dependency_graph = graph.clone();

        // Analyze contentions
        let contentions = self.analyze_contentions(&lock_stats);

        // Detect deadlocks
        let deadlocks = if self.config.detect_deadlocks {
            self.detect_deadlocks(&dependency_graph)
        } else {
            Vec::new()
        };

        Ok(LockProfileReport {
            duration,
            total_events,
            lock_stats,
            dependency_graph,
            contentions,
            deadlocks,
            events: events.clone(),
        })
    }

    /// Analyze lock contentions
    fn analyze_contentions(&self, lock_stats: &BTreeMap<LockId, LockStatistics>) -> Vec<ContentionAnalysis> {
        let mut contentions = Vec::new();

        for (&lock_id, stats) in lock_stats {
            let total_contentions = stats.total_contentions.load(Ordering::Relaxed);
            if total_contentions > 0 {
                let contention_score = stats.contention_rate() * stats.avg_wait_time() as f64;

                contentions.push(ContentionAnalysis {
                    lock_id,
                    contention_score,
                    total_contentions,
                    avg_wait_time: stats.avg_wait_time(),
                    max_wait_time: stats.max_wait_time.load(Ordering::Relaxed),
                    top_contenders: Vec::new(), // Would need per-thread tracking
                });
            }
        }

        contentions.sort_by(|a, b| {
            b.contention_score
                .partial_cmp(&a.contention_score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        contentions
    }

    /// Detect deadlocks in dependency graph
    fn detect_deadlocks(&self, graph: &[DependencyEdge]) -> Vec<DeadlockInfo> {
        let mut deadlocks = Vec::new();

        // Build adjacency list
        let mut adj: BTreeMap<LockId, Vec<LockId>> = BTreeMap::new();
        for edge in graph {
            adj.entry(edge.from_lock).or_insert_with(Vec::new).push(edge.to_lock);
        }

        // Find all cycles
        let mut visited = BTreeSet::new();
        let mut path = Vec::new();

        for &node in adj.keys() {
            if !visited.contains(&node) {
                if let Some(cycle) = self.find_cycle(&adj, node, &mut visited, &mut path) {
                    deadlocks.push(DeadlockInfo {
                        threads: Vec::new(), // Would need thread tracking
                        locks: cycle.clone(),
                        cycle_length: cycle.len(),
                    });
                }
            }
        }

        deadlocks
    }

    /// Find cycle in graph
    fn find_cycle(
        &self,
        adj: &BTreeMap<LockId, Vec<LockId>>,
        node: LockId,
        visited: &mut BTreeSet<LockId>,
        path: &mut Vec<LockId>,
    ) -> Option<Vec<LockId>> {
        visited.insert(node);
        path.push(node);

        if let Some(neighbors) = adj.get(&node) {
            for &neighbor in neighbors {
                if let Some(pos) = path.iter().position(|&n| n == neighbor) {
                    // Found a cycle
                    return Some(path[pos..].to_vec());
                }

                if !visited.contains(&neighbor) {
                    if let Some(cycle) = self.find_cycle(adj, neighbor, visited, path) {
                        return Some(cycle);
                    }
                }
            }
        }

        path.pop();
        None
    }

    /// Add event to buffer
    fn add_event(&self, event: LockEvent) {
        let mut events = self.events.lock();
        if events.len() < self.config.max_events {
            events.push(event);
        }
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }

    /// Clear all profiling data
    pub fn clear(&self) {
        self.events.lock().clear();
        self.dependency_graph.lock().clear();
        self.held_locks.lock().clear();

        // Reset all lock statistics
        for stats in self.locks.lock().values() {
            stats.total_acquisitions.store(0, Ordering::Relaxed);
            stats.total_releases.store(0, Ordering::Relaxed);
            stats.total_contentions.store(0, Ordering::Relaxed);
            stats.total_wait_time.store(0, Ordering::Relaxed);
            stats.total_hold_time.store(0, Ordering::Relaxed);
            stats.max_wait_time.store(0, Ordering::Relaxed);
            stats.max_hold_time.store(0, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = LockProfiler::new();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_profiler_start_stop() {
        let profiler = LockProfiler::new();
        profiler.start().unwrap();
        assert!(profiler.is_running());
        profiler.stop().unwrap();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_lock_registration() {
        let profiler = LockProfiler::new();
        let lock_id = profiler.register_lock();
        assert!(lock_id > 0);

        profiler.unregister_lock(lock_id);
    }

    #[test]
    fn test_lock_acquisition_tracking() {
        let profiler = LockProfiler::new();
        profiler.start().unwrap();

        let lock_id = profiler.register_lock();
        let thread_id = 1;

        profiler.record_lock(lock_id, thread_id, 0x1000).unwrap();
        profiler.record_acquired(lock_id, thread_id, 100, 0x1000).unwrap();

        let stats = profiler.locks.lock();
        let lock_stats = stats.get(&lock_id).unwrap();
        assert_eq!(lock_stats.total_acquisitions.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_lock_release_tracking() {
        let profiler = LockProfiler::new();
        profiler.start().unwrap();

        let lock_id = profiler.register_lock();
        let thread_id = 1;

        profiler.record_lock(lock_id, thread_id, 0x1000).unwrap();
        profiler.record_acquired(lock_id, thread_id, 100, 0x1000).unwrap();
        profiler.record_unlock(lock_id, thread_id, 0x1000).unwrap();

        let stats = profiler.locks.lock();
        let lock_stats = stats.get(&lock_id).unwrap();
        assert_eq!(lock_stats.total_releases.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_contention_tracking() {
        let profiler = LockProfiler::new();
        profiler.start().unwrap();

        let lock_id = profiler.register_lock();
        profiler.record_trylock_failed(lock_id, 1, 0x1000).unwrap();

        let stats = profiler.locks.lock();
        let lock_stats = stats.get(&lock_id).unwrap();
        assert_eq!(lock_stats.total_contentions.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_dependency_tracking() {
        let profiler = LockProfiler::new();
        profiler.start().unwrap();

        let lock1 = profiler.register_lock();
        let lock2 = profiler.register_lock();

        profiler.record_dependency(lock1, lock2, 1).unwrap();

        let graph = profiler.dependency_graph.lock();
        assert_eq!(graph.len(), 1);
        assert_eq!(graph[0].from_lock, lock1);
        assert_eq!(graph[0].to_lock, lock2);
    }

    #[test]
    fn test_report_generation() {
        let profiler = LockProfiler::new();
        profiler.start().unwrap();

        let lock_id = profiler.register_lock();
        profiler.record_lock(lock_id, 1, 0x1000).unwrap();
        profiler.record_acquired(lock_id, 1, 100, 0x1000).unwrap();

        let report = profiler.generate_report().unwrap();
        assert_eq!(report.total_events, 2);
        assert!(!report.lock_stats.is_empty());
    }

    #[test]
    fn test_avg_wait_time() {
        let stats = LockStatistics::new(1);
        stats.record_acquisition(1, 100);
        stats.record_acquisition(2, 200);

        assert_eq!(stats.avg_wait_time(), 150);
    }

    #[test]
    fn test_avg_hold_time() {
        let stats = LockStatistics::new(1);
        stats.record_release(1, 50);
        stats.record_release(2, 150);

        assert_eq!(stats.avg_hold_time(), 100);
    }

    #[test]
    fn test_contention_rate() {
        let stats = LockStatistics::new(1);
        stats.total_acquisitions.store(10, Ordering::Relaxed);
        stats.total_contentions.store(3, Ordering::Relaxed);

        assert!((stats.contention_rate() - 0.3).abs() < 0.01);
    }
}
