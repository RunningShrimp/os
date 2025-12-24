//! Unified Scheduler Implementation
//!
//! This module provides a unified, high-performance scheduler that replaces
//! the O(n) linear search with O(log n) priority queues and per-CPU run queues.
//!
//! Features:
//! - Priority queues for O(log n) scheduling decisions
//! - Per-CPU run queues to reduce lock contention
//! - Work stealing for load balancing
//! - Support for multiple scheduling policies (FIFO, RR, Normal, Idle)
//! - Real-time scheduling support

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::subsystems::sync::Mutex;
use crate::cpu;
use crate::process::thread::{ThreadState, Tid, SchedPolicy, SchedParam};

/// Maximum number of CPUs supported
const MAX_CPUS: usize = 256;

/// Maximum priority level (higher = more important)
const MAX_PRIORITY: u8 = 255;

/// Minimum priority level
const MIN_PRIORITY: u8 = 0;

/// Real-time priority range (100-199)
const RT_PRIORITY_MIN: u8 = 100;
const RT_PRIORITY_MAX: u8 = 199;

/// Normal priority range (0-99)
const NORMAL_PRIORITY_MIN: u8 = 0;
const NORMAL_PRIORITY_MAX: u8 = 99;

/// Thread entry in priority queue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PriorityQueueEntry {
    /// Thread ID
    tid: Tid,
    /// Effective priority (for tie-breaking)
    effective_priority: u8,
    /// Timestamp when thread became runnable (for FIFO ordering)
    enqueue_time: u64,
}

impl PartialOrd for PriorityQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityQueueEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Lower priority value = higher priority (inverted comparison)
        // For real-time: lower number = higher priority (e.g., 1 > 99)
        match self.effective_priority.cmp(&other.effective_priority) {
            core::cmp::Ordering::Equal => {
                // For same priority, FIFO: earlier enqueue time first
                self.enqueue_time.cmp(&other.enqueue_time)
            }
            // Invert: lower priority number = higher priority
            ordering => ordering.reverse(),
        }
    }
}

/// Priority queue for threads
/// Uses BTreeMap for O(log n) operations
struct PriorityQueue {
    /// Map from priority to list of thread entries
    /// Higher priority threads are at the end (for efficient pop)
    queues: BTreeMap<u8, Vec<PriorityQueueEntry>>,
    /// Total number of threads in queue
    count: AtomicUsize,
    /// Sequence number for FIFO ordering
    sequence: AtomicU64,
    /// Cached lowest (highest-priority) non-empty priority
    min_priority: AtomicU8,
    /// Whether min_priority is valid
    has_min: AtomicBool,
}

impl PriorityQueue {
    fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            count: AtomicUsize::new(0),
            sequence: AtomicU64::new(0),
            min_priority: AtomicU8::new(MAX_PRIORITY),
            has_min: AtomicBool::new(false),
        }
    }

    /// Enqueue a thread with given priority
    /// Note: Lower priority number = higher priority (e.g., RT priority 1 > RT priority 99)
    fn enqueue(&mut self, tid: Tid, priority: u8) {
        let entry = PriorityQueueEntry {
            tid,
            effective_priority: priority,
            enqueue_time: self.sequence.fetch_add(1, Ordering::Relaxed),
        };

        self.queues.entry(priority).or_insert_with(Vec::new).push(entry);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update cached min priority for fast dequeue
        let has_min = self.has_min.load(Ordering::Relaxed);
        if !has_min || priority < self.min_priority.load(Ordering::Relaxed) {
            self.min_priority.store(priority, Ordering::Relaxed);
            self.has_min.store(true, Ordering::Relaxed);
        }
    }

    /// Dequeue the highest priority thread
    /// Note: Lower priority number = higher priority
    fn dequeue(&mut self) -> Option<Tid> {
        if !self.has_min.load(Ordering::Relaxed) {
            return None;
        }

        let current_min = self.min_priority.load(Ordering::Relaxed);

        // Fast path: use cached min priority
        if let Some(queue) = self.queues.get_mut(&current_min) {
            if let Some(entry) = queue.pop() {
                if queue.is_empty() {
                    self.queues.remove(&current_min);
                }
                self.count.fetch_sub(1, Ordering::Relaxed);

                // Recompute min if needed
                if let Some((&p, q)) = self.queues.iter().find(|(_, q)| !q.is_empty()) {
                    self.min_priority.store(p, Ordering::Relaxed);
                    self.has_min.store(true, Ordering::Relaxed);
                } else {
                    self.has_min.store(false, Ordering::Relaxed);
                }

                return Some(entry.tid);
            }
        }

        // Slow path: scan for next non-empty priority
        for (&priority, queue) in self.queues.iter() {
            if !queue.is_empty() {
                if let Some(queue_mut) = self.queues.get_mut(&priority) {
                    if let Some(entry) = queue_mut.pop() {
                        if queue_mut.is_empty() {
                            self.queues.remove(&priority);
                        }
                        self.count.fetch_sub(1, Ordering::Relaxed);

                        if let Some((&p, q)) = self.queues.iter().find(|(_, q)| !q.is_empty()) {
                            self.min_priority.store(p, Ordering::Relaxed);
                            self.has_min.store(true, Ordering::Relaxed);
                        } else {
                            self.has_min.store(false, Ordering::Relaxed);
                        }

                        return Some(entry.tid);
                    }
                }
            }
        }

        self.has_min.store(false, Ordering::Relaxed);
        None
    }

    /// Peek at the highest priority thread without removing it
    /// Note: Lower priority number = higher priority
    fn peek(&self) -> Option<Tid> {
        // Find lowest priority number (highest priority) non-empty queue
        // BTreeMap iterates in ascending order, so first non-empty is highest priority
        for (&priority, queue) in self.queues.iter() {
            if !queue.is_empty() {
                // Return first thread in FIFO order (first in queue)
                return queue.first().map(|e| e.tid);
            }
        }
        None
    }

    /// Remove a specific thread from the queue
    fn remove(&mut self, tid: Tid) -> bool {
        for (priority, queue) in self.queues.iter_mut() {
            if let Some(pos) = queue.iter().position(|e| e.tid == tid) {
                queue.remove(pos);
                if queue.is_empty() {
                    self.queues.remove(priority);
                }
                self.count.fetch_sub(1, Ordering::Relaxed);

                if self.count.load(Ordering::Relaxed) == 0 {
                    self.has_min.store(false, Ordering::Relaxed);
                } else if let Some((&p, q)) = self.queues.iter().find(|(_, q)| !q.is_empty()) {
                    self.min_priority.store(p, Ordering::Relaxed);
                    self.has_min.store(true, Ordering::Relaxed);
                }
                return true;
            }
        }
        false
    }

    /// Check if queue is empty
    fn is_empty(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 0
    }

    /// Get number of threads in queue
    fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

/// Per-CPU scheduler data
struct PerCpuScheduler {
    /// CPU ID
    cpu_id: usize,
    /// Priority queue for this CPU
    ready_queue: Mutex<PriorityQueue>,
    /// Currently running thread
    current_thread: AtomicUsize,
    /// Idle thread for this CPU
    idle_thread: Tid,
    /// Statistics
    context_switches: AtomicU64,
    last_switch_time: AtomicU64,
}

impl PerCpuScheduler {
    fn new(cpu_id: usize, idle_thread: Tid) -> Self {
        Self {
            cpu_id,
            ready_queue: Mutex::new(PriorityQueue::new()),
            current_thread: AtomicUsize::new(0),
            idle_thread,
            context_switches: AtomicU64::new(0),
            last_switch_time: AtomicU64::new(0),
        }
    }

    /// Enqueue a thread with priority
    fn enqueue(&self, tid: Tid, priority: u8) {
        let mut queue = self.ready_queue.lock();
        queue.enqueue(tid, priority);
    }

    /// Dequeue the highest priority thread
    fn dequeue(&self) -> Option<Tid> {
        let mut queue = self.ready_queue.lock();
        queue.dequeue()
    }

    /// Peek at next thread without removing
    fn peek(&self) -> Option<Tid> {
        let queue = self.ready_queue.lock();
        queue.peek()
    }

    /// Remove a thread from queue
    fn remove(&self, tid: Tid) -> bool {
        let mut queue = self.ready_queue.lock();
        queue.remove(tid)
    }

    /// Check if queue is empty
    fn is_empty(&self) -> bool {
        let queue = self.ready_queue.lock();
        queue.is_empty()
    }

    /// Get queue length
    fn len(&self) -> usize {
        let queue = self.ready_queue.lock();
        queue.len()
    }

    /// Get total context switches for this CPU
    fn context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }

    /// Set current thread
    fn set_current(&self, tid: Tid) {
        let old = self.current_thread.swap(tid, Ordering::SeqCst);
        if old != tid {
            self.context_switches.fetch_add(1, Ordering::Relaxed);
            self.last_switch_time.store(get_timestamp_ns(), Ordering::Relaxed);
        }
    }

    /// Get current thread
    fn get_current(&self) -> Tid {
        self.current_thread.load(Ordering::Relaxed)
    }
}

/// Thread metadata for scheduling
#[derive(Debug)]
struct ThreadMetadata {
    /// Thread ID
    tid: Tid,
    /// Process ID
    pid: crate::process::Pid,
    /// Current priority
    priority: u8,
    /// Scheduling policy
    policy: SchedPolicy,
    /// CPU affinity mask
    cpu_affinity: u64,
    /// Thread state
    state: ThreadState,
    /// Time slice remaining (for RR policy)
    time_slice: u32,
}

/// Unified scheduler
pub struct UnifiedScheduler {
    /// Per-CPU schedulers (using Vec instead of array for dynamic sizing)
    per_cpu_schedulers: Vec<PerCpuScheduler>,
    /// Thread metadata table
    thread_metadata: Mutex<BTreeMap<Tid, ThreadMetadata>>,
    /// Next thread ID
    next_tid: AtomicUsize,
}

/// Scheduler statistics snapshot
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_context_switches: u64,
    pub runnable_threads: usize,
    pub runqueue_len_total: usize,
    pub runqueue_len_max: usize,
}

impl UnifiedScheduler {
    /// Create a new unified scheduler
    pub fn new(num_cpus: usize) -> Self {
        let num_cpus = num_cpus.min(MAX_CPUS);
        
        // Initialize per-CPU schedulers with idle threads
        let mut per_cpu_schedulers = Vec::with_capacity(num_cpus);

        for cpu_id in 0..num_cpus {
            let idle_tid = 1000 + cpu_id; // Special IDs for idle threads
            per_cpu_schedulers.push(PerCpuScheduler::new(cpu_id, idle_tid));
        }

        Self {
            per_cpu_schedulers,
            thread_metadata: Mutex::new(BTreeMap::new()),
            next_tid: AtomicUsize::new(1),
        }
    }

    /// Register a thread with the scheduler
    pub fn register_thread(
        &self,
        tid: Tid,
        pid: crate::process::Pid,
        priority: u8,
        policy: SchedPolicy,
        cpu_affinity: u64,
    ) {
        let metadata = ThreadMetadata {
            tid,
            pid,
            priority: priority.min(MAX_PRIORITY),
            policy,
            cpu_affinity,
            state: ThreadState::Runnable,
            time_slice: get_default_timeslice(policy),
        };

        let mut table = self.thread_metadata.lock();
        table.insert(tid, metadata);

        // Enqueue on appropriate CPU(s)
        self.enqueue_thread(tid, priority);
    }

    /// Unregister a thread
    pub fn unregister_thread(&self, tid: Tid) {
        let mut table = self.thread_metadata.lock();
        table.remove(&tid);

        // Remove from all CPU queues
        for scheduler in &self.per_cpu_schedulers {
            scheduler.remove(tid);
        }
    }

    /// Enqueue a thread on appropriate CPU(s)
    fn enqueue_thread(&self, tid: Tid, priority: u8) {
        let table = self.thread_metadata.lock();
        if let Some(metadata) = table.get(&tid) {
            let cpu_affinity = metadata.cpu_affinity;
            drop(table);

            // Enqueue on CPUs where thread is allowed to run
            for (cpu_id, scheduler) in self.per_cpu_schedulers.iter().enumerate() {
                if cpu_affinity == 0 || (cpu_affinity & (1u64 << cpu_id)) != 0 {
                    scheduler.enqueue(tid, priority);
                }
            }
        }
    }

    /// Schedule next thread for current CPU
    pub fn schedule(&self) -> Option<Tid> {
        let cpu_id = cpu::cpuid() % MAX_CPUS;
        let scheduler = &self.per_cpu_schedulers[cpu_id];

        // Try to get next thread from this CPU's queue
        if let Some(next_tid) = scheduler.dequeue() {
            scheduler.set_current(next_tid);
            return Some(next_tid);
        }

        // Optimized work stealing with load-aware selection
        self.try_steal_work(cpu_id, scheduler)
    }

    /// Try to steal work from other CPUs with optimized algorithm
    fn try_steal_work(&self, local_cpu_id: usize, local_scheduler: &PerCpuScheduler) -> Option<Tid> {
        let local_load = local_scheduler.len();

        // Don't steal if local CPU has enough work (anti-thrashing)
        if local_load > 2 {
            return None;
        }

        // Get random starting point for fair stealing (avoid stealing bias)
        let num_cpus = self.per_cpu_schedulers.len();
        let random_start = self.get_random_offset() as usize % num_cpus;

        // Adaptive steal probability based on load imbalance
        // Steal more aggressively when load is very imbalanced
        let steal_probability = if local_load == 0 { 80 }
                              else if local_load == 1 { 60 }
                              else { 30 };

        // Work stealing with load awareness and probability control
        let mut steal_attempts = 0;
        let max_attempts = num_cpus.saturating_sub(1);

        for i in 0..max_attempts {
            steal_attempts += 1;

            // Early exit based on steal probability
            let random_threshold = self.get_random_offset() as u32 % 100;
            if steal_attempts > 2 && random_threshold > steal_probability {
                break;
            }

            let steal_cpu_id = (random_start + i) % num_cpus;

            // Skip local CPU
            if steal_cpu_id == local_cpu_id {
                continue;
            }

            let steal_scheduler = &self.per_cpu_schedulers[steal_cpu_id];
            let steal_load = steal_scheduler.len();

            // Only steal from CPUs with significantly higher load (load balancing)
            // Threshold increases as steal attempts increase
            let threshold = local_load + steal_attempts;
            if steal_load <= threshold {
                continue;
            }

            // Try to steal from this CPU
            if let Some(stolen_tid) = steal_scheduler.dequeue() {
                // Steal successful, update stolen thread's CPU affinity
                if let Some(mut metadata) = self.thread_metadata.lock().get_mut(&stolen_tid).copied() {
                    metadata.last_cpu = Some(local_cpu_id);
                }
                local_scheduler.set_current(stolen_tid);
                return Some(stolen_tid);
            }
        }

        // No work to steal, return idle thread
        local_scheduler.set_current(local_scheduler.idle_thread);
        Some(local_scheduler.idle_thread)
    }

    /// Get random offset for work stealing (using RDRAND when available)
    fn get_random_offset(&self) -> u32 {
        // Try to use RDRAND for better randomness
        #[cfg(target_arch = "x86_64")]
        {
            unsafe {
                let mut value: u32 = 0;
                let success: bool;
                core::arch::asm!(
                    "rdrand {0:e}",
                    out(reg) value,
                    setne(success),
                    options(nostack, pure)
                );
                if success {
                    return value;
                }
            }
        }

        // Fallback: Use a simple hash of timestamp + CPU ID as random source
        let timestamp = crate::subsystems::time::get_ticks();
        let cpu_id = cpu::cpuid();
        let combined = timestamp.wrapping_mul(31).wrapping_add(cpu_id as u64);
        (combined as u32)
    }

    /// Set thread state
    pub fn set_thread_state(&self, tid: Tid, state: ThreadState) -> Result<(), &'static str> {
        let mut table = self.thread_metadata.lock();
        let metadata = table.get_mut(&tid).ok_or("Thread not found")?;
        
        let old_state = metadata.state;
        metadata.state = state;

        match state {
            ThreadState::Runnable => {
                // Add to ready queue
                self.enqueue_thread(tid, metadata.priority);
            }
            ThreadState::Blocked | ThreadState::Zombie => {
                // Remove from all queues
                for scheduler in &self.per_cpu_schedulers {
                    scheduler.remove(tid);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Set thread priority
    pub fn set_priority(&self, tid: Tid, priority: u8) -> Result<(), &'static str> {
        let mut table = self.thread_metadata.lock();
        let metadata = table.get_mut(&tid).ok_or("Thread not found")?;
        
        let old_priority = metadata.priority;
        metadata.priority = priority.min(MAX_PRIORITY);

        // Re-enqueue with new priority if thread is runnable
        if metadata.state == ThreadState::Runnable {
            drop(table);
            // Remove from old position
            for scheduler in &self.per_cpu_schedulers {
                scheduler.remove(tid);
            }
            // Add with new priority
            self.enqueue_thread(tid, metadata.priority);
        }

        Ok(())
    }

    /// Get current thread for CPU
    pub fn get_current_thread(&self, cpu_id: usize) -> Tid {
        let cpu_id = cpu_id % MAX_CPUS;
        self.per_cpu_schedulers[cpu_id].get_current()
    }

    /// Get queue length for CPU
    pub fn get_queue_len(&self, cpu_id: usize) -> usize {
        let cpu_id = cpu_id % MAX_CPUS;
        self.per_cpu_schedulers[cpu_id].len()
    }

    /// Collect aggregated scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let mut total_ctx = 0u64;
        let mut total_len = 0usize;
        let mut max_len = 0usize;

        for sched in &self.per_cpu_schedulers {
            let len = sched.len();
            total_len += len;
            max_len = max_len.max(len);
            total_ctx += sched.context_switches();
        }

        // Count runnable threads from metadata
        let table = self.thread_metadata.lock();
        let runnable = table
            .values()
            .filter(|m| m.state == ThreadState::Runnable)
            .count();
        drop(table);

        SchedulerStats {
            total_context_switches: total_ctx,
            runnable_threads: runnable,
            runqueue_len_total: total_len,
            runqueue_len_max: max_len,
        }
    }
}

/// Get default time slice for scheduling policy
fn get_default_timeslice(policy: SchedPolicy) -> u32 {
    match policy {
        SchedPolicy::Fifo => u32::MAX, // FIFO runs until blocked
        SchedPolicy::RoundRobin => 10,  // 10ms for RR
        SchedPolicy::Normal => 10,      // 10ms for normal
        SchedPolicy::Batch => 50,      // 50ms for batch
        SchedPolicy::Idle => 100,      // 100ms for idle
    }
}

/// Get current timestamp in nanoseconds
fn get_timestamp_ns() -> u64 {
    crate::subsystems::time::get_time_ns()
}

/// Global unified scheduler instance
static GLOBAL_SCHEDULER: Mutex<Option<UnifiedScheduler>> = Mutex::new(None);

/// Initialize the unified scheduler
pub fn init_unified_scheduler(num_cpus: usize) {
    let mut scheduler_guard = GLOBAL_SCHEDULER.lock();
    *scheduler_guard = Some(UnifiedScheduler::new(num_cpus));
}

/// Get the global unified scheduler
pub fn get_unified_scheduler() -> Option<&'static Mutex<Option<UnifiedScheduler>>> {
    Some(&GLOBAL_SCHEDULER)
}

/// Get scheduler statistics if initialized
pub fn get_scheduler_stats() -> Option<SchedulerStats> {
    get_unified_scheduler().and_then(|m| {
        let guard = m.lock();
        guard.as_ref().map(|sched| sched.get_stats())
    })
}

/// Schedule next thread (replacement for old schedule function)
pub fn unified_schedule() -> Option<Tid> {
    if let Some(scheduler_guard) = get_unified_scheduler() {
        let scheduler = scheduler_guard.lock();
        if let Some(ref scheduler) = *scheduler {
            scheduler.schedule()
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_queue() {
        let mut queue = PriorityQueue::new();
        
        queue.enqueue(1, 10);
        queue.enqueue(2, 20);
        queue.enqueue(3, 15);
        
        // Should dequeue highest priority first
        assert_eq!(queue.dequeue(), Some(2)); // Priority 20
        assert_eq!(queue.dequeue(), Some(3)); // Priority 15
        assert_eq!(queue.dequeue(), Some(1)); // Priority 10
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn test_fifo_ordering() {
        let mut queue = PriorityQueue::new();
        
        queue.enqueue(1, 10);
        queue.enqueue(2, 10); // Same priority
        queue.enqueue(3, 10); // Same priority
        
        // Should dequeue in FIFO order
        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2));
        assert_eq!(queue.dequeue(), Some(3));
    }
}

