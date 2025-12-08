//! Microkernel core scheduler
//!
//! Provides basic thread scheduling capabilities for the microkernel layer.
//! This is a simplified scheduler focused on efficiency and low latency.

extern crate alloc;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::sync::Mutex;
use crate::reliability::errno::{EINVAL, ESRCH};
use crate::process::thread::{Thread, ThreadState, Tid};

/// Scheduling policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    Normal,      // Normal time-sharing scheduling
    FIFO,        // Real-time FIFO scheduling
    RR,          // Real-time round-robin scheduling
    Idle,        // Low priority idle scheduling
}

/// CPU affinity mask (simplified - up to 64 CPUs)
#[derive(Debug, Clone, Copy)]
pub struct CpuAffinity {
    mask: u64,
}

impl CpuAffinity {
    pub const fn new() -> Self {
        Self { mask: u64::MAX }
    }

    pub const fn single(cpu: u8) -> Self {
        Self { mask: 1u64 << cpu }
    }

    pub fn contains(&self, cpu: u8) -> bool {
        (self.mask & (1u64 << cpu)) != 0
    }

    pub fn set(&mut self, cpu: u8) {
        self.mask |= 1u64 << cpu;
    }

    pub fn clear(&mut self, cpu: u8) {
        self.mask &= !(1u64 << cpu);
    }
}

/// Thread control block for microkernel scheduler
#[derive(Debug)]
pub struct MicroTcb {
    pub tid: Tid,
    pub priority: i32,
    pub policy: SchedulingPolicy,
    pub state: ThreadState,
    pub cpu_affinity: CpuAffinity,
    pub time_slice: u32,      // Remaining time slice in ticks
    pub total_runtime: u64,   // Total runtime in nanoseconds
    pub last_run: u64,        // Last time this thread ran
    pub wake_time: Option<u64>, // Time to wake up from sleep
}

impl MicroTcb {
    pub fn new(tid: Tid, priority: i32, policy: SchedulingPolicy) -> Self {
        Self {
            tid,
            priority,
            policy,
            state: ThreadState::Runnable,
            cpu_affinity: CpuAffinity::new(),
            time_slice: DEFAULT_TIME_SLICE,
            total_runtime: 0,
            last_run: 0,
            wake_time: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state == ThreadState::Runnable &&
        self.wake_time.map_or(true, |wt| wt <= get_current_time())
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self.state, ThreadState::Runnable | ThreadState::Running) &&
        self.wake_time.map_or(true, |wt| wt <= get_current_time())
    }
}

/// Per-CPU scheduler data
#[derive(Debug)]
pub struct CpuScheduler {
    pub cpu_id: u8,
    ready_queue: VecDeque<Tid>,
    current_thread: Option<Tid>,
    idle_thread: Tid,
    last_switch_time: u64,
    load_average: f64,
}

impl CpuScheduler {
    pub fn new(cpu_id: u8, idle_thread: Tid) -> Self {
        Self {
            cpu_id,
            ready_queue: VecDeque::new(),
            current_thread: None,
            idle_thread,
            last_switch_time: get_current_time(),
            load_average: 0.0,
        }
    }

    pub fn enqueue(&mut self, tid: Tid) -> Result<(), i32> {
        if self.ready_queue.contains(&tid) {
            return Err(EINVAL);
        }
        self.ready_queue.push_back(tid);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<Tid> {
        self.ready_queue.pop_front()
    }

    pub fn remove(&mut self, tid: Tid) -> bool {
        let pos = self.ready_queue.iter().position(|&t| t == tid);
        if let Some(index) = pos {
            self.ready_queue.remove(index);
            true
        } else {
            false
        }
    }

    pub fn get_next_thread(&self, tcb_map: &crate::sync::Mutex<alloc::collections::BTreeMap<Tid, MicroTcb>>) -> Option<Tid> {
        // Find highest priority runnable thread
        let mut best_thread = None;
        let mut best_priority = i32::MIN;

        for &tid in &self.ready_queue {
            let tcb_map_guard = tcb_map.lock();
            if let Some(tcb) = tcb_map_guard.get(&tid) {
                if tcb.is_ready() && tcb.priority > best_priority {
                    best_priority = tcb.priority;
                    best_thread = Some(tid);
                }
            }
        }

        best_thread.or(Some(self.idle_thread))
    }

    pub fn update_load_average(&mut self) {
        let current_time = get_current_time();
        let time_delta = (current_time - self.last_switch_time) as f64 / 1_000_000_000.0; // Convert to seconds
        self.last_switch_time = current_time;

        // Exponential moving average with alpha = 0.1
        let alpha = 0.1;
        let current_load = self.ready_queue.len() as f64;
        self.load_average = alpha * current_load + (1.0 - alpha) * self.load_average;
    }
}

/// Microkernel scheduler
pub struct MicroScheduler {
    pub cpu_schedulers: Vec<CpuScheduler>,
    thread_table: Mutex<alloc::collections::BTreeMap<Tid, MicroTcb>>,
    next_tid: AtomicUsize,
    scheduling_enabled: AtomicUsize, // 0 = disabled, 1 = enabled
}

impl MicroScheduler {
    pub fn new(num_cpus: u8) -> Self {
        let mut cpu_schedulers = Vec::with_capacity(num_cpus as usize);

        // Create idle threads for each CPU
        for cpu_id in 0..num_cpus {
            let idle_tid = 1000 + cpu_id as usize; // Special IDs for idle threads
            cpu_schedulers.push(CpuScheduler::new(cpu_id, idle_tid));
        }

        Self {
            cpu_schedulers,
            thread_table: Mutex::new(alloc::collections::BTreeMap::new()),
            next_tid: AtomicUsize::new(1),
            scheduling_enabled: AtomicUsize::new(1),
        }
    }

    pub fn create_thread(&mut self, priority: i32, policy: SchedulingPolicy) -> Result<Tid, i32> {
        let tid = self.next_tid.fetch_add(1, Ordering::SeqCst);

        let tcb = MicroTcb::new(tid, priority, policy);

        let mut table = self.thread_table.lock();
        table.insert(tid, tcb);

        // Add to CPU 0 ready queue for now (TODO: CPU affinity)
        if self.cpu_schedulers.len() > 0 {
            self.cpu_schedulers[0].enqueue(tid).map_err(|_| {
                // Remove from table if enqueue fails
                table.remove(&tid);
                EINVAL
            })?;
        }

        Ok(tid)
    }

    pub fn destroy_thread(&mut self, tid: Tid) -> Result<(), i32> {
        let mut table = self.thread_table.lock();

        if table.remove(&tid).is_none() {
            return Err(ESRCH);
        }

        // Remove from all CPU ready queues
        for scheduler in &mut self.cpu_schedulers {
            scheduler.remove(tid);
        }

        Ok(())
    }

    pub fn set_thread_state(&mut self, tid: Tid, state: ThreadState) -> Result<(), i32> {
        let mut table = self.thread_table.lock();

        let tcb = table.get_mut(&tid).ok_or(ESRCH)?;
        tcb.state = state;

        // Add to ready queue if becoming ready
        if state == ThreadState::Runnable {
            if let Some(scheduler) = self.cpu_schedulers.get_mut(0) { // TODO: CPU selection
                if !scheduler.ready_queue.contains(&tid) {
                    scheduler.enqueue(tid)?;
                }
            }
        }

        Ok(())
    }

    pub fn get_thread_state(&self, tid: Tid) -> Result<ThreadState, i32> {
        let table = self.thread_table.lock();
        let tcb = table.get(&tid).ok_or(ESRCH)?;
        Ok(tcb.state)
    }

    pub fn set_priority(&self, tid: Tid, priority: i32) -> Result<(), i32> {
        let mut table = self.thread_table.lock();

        let tcb = table.get_mut(&tid).ok_or(ESRCH)?;
        tcb.priority = priority;

        Ok(())
    }

    pub fn get_priority(&self, tid: Tid) -> Result<i32, i32> {
        let table = self.thread_table.lock();
        let tcb = table.get(&tid).ok_or(ESRCH)?;
        Ok(tcb.priority)
    }

    pub fn set_affinity(&self, tid: Tid, affinity: CpuAffinity) -> Result<(), i32> {
        let mut table = self.thread_table.lock();

        let tcb = table.get_mut(&tid).ok_or(ESRCH)?;
        tcb.cpu_affinity = affinity;

        Ok(())
    }

    pub fn sleep_thread(&mut self, tid: Tid, wake_time: u64) -> Result<(), i32> {
        let mut table = self.thread_table.lock();

        let tcb = table.get_mut(&tid).ok_or(ESRCH)?;
        tcb.wake_time = Some(wake_time);
        tcb.state = ThreadState::Blocked;

        // Remove from ready queue
        for scheduler in &mut self.cpu_schedulers {
            scheduler.remove(tid);
        }

        Ok(())
    }

    pub fn wake_sleeping_threads(&mut self) {
        let mut table = self.thread_table.lock();
        let current_time = get_current_time();

        for (tid, tcb) in table.iter_mut() {
            if tcb.state == ThreadState::Blocked {
                if let Some(wake_time) = tcb.wake_time {
                    if wake_time <= current_time {
                        tcb.state = ThreadState::Runnable;
                        tcb.wake_time = None;

                        // Add to ready queue
                        if let Some(scheduler) = self.cpu_schedulers.get_mut(0) { // TODO: CPU selection
                            let _ = scheduler.enqueue(*tid);
                        }
                    }
                }
            }
        }
    }

    pub fn schedule(&mut self, cpu_id: u8) -> Option<Tid> {
        if self.scheduling_enabled.load(Ordering::SeqCst) == 0 {
            return None;
        }

        if let Some(scheduler) = self.cpu_schedulers.get_mut(cpu_id as usize) {
            let next_thread = scheduler.get_next_thread(&self.thread_table);

            if next_thread != scheduler.current_thread {
                // Context switch needed
                scheduler.current_thread = next_thread;
                scheduler.update_load_average();

                // Update statistics
                super::MICROKERNEL_STATS.scheduler_runs.fetch_add(1, Ordering::SeqCst);
            }

            scheduler.current_thread
        } else {
            None
        }
    }

    pub fn get_current_thread(&self, cpu_id: u8) -> Option<Tid> {
        self.cpu_schedulers.get(cpu_id as usize)
            .and_then(|s| s.current_thread)
    }

    pub fn enable_scheduling(&self) {
        self.scheduling_enabled.store(1, Ordering::SeqCst);
    }

    pub fn disable_scheduling(&self) {
        self.scheduling_enabled.store(0, Ordering::SeqCst);
    }

    pub fn get_load_average(&self, cpu_id: u8) -> f64 {
        self.cpu_schedulers.get(cpu_id as usize)
            .map(|s| s.load_average)
            .unwrap_or(0.0)
    }

    pub fn get_ready_queue_len(&self, cpu_id: u8) -> usize {
        self.cpu_schedulers.get(cpu_id as usize)
            .map(|s| s.ready_queue.len())
            .unwrap_or(0)
    }
}

/// Default time slice in ticks (typically 10ms)
const DEFAULT_TIME_SLICE: u32 = 10;

/// Get current time in nanoseconds
fn get_current_time() -> u64 {
    crate::time::get_time_ns()
}

/// Global scheduler instance
static mut GLOBAL_SCHEDULER: Option<MicroScheduler> = None;
static SCHEDULER_INIT: AtomicUsize = AtomicUsize::new(0);

/// Initialize the microkernel scheduler
pub fn init() -> Result<(), i32> {
    if SCHEDULER_INIT.load(Ordering::SeqCst) != 0 {
        return Ok(());
    }

    // Detect number of CPUs (simplified)
    let num_cpus = if cfg!(target_arch = "x86_64") { 1 } else { 1 };

    let scheduler = MicroScheduler::new(num_cpus as u8);

    unsafe {
        GLOBAL_SCHEDULER = Some(scheduler);
    }

    SCHEDULER_INIT.store(1, Ordering::SeqCst);
    Ok(())
}

/// Get global scheduler instance
pub fn get_scheduler() -> Option<&'static mut MicroScheduler> {
    unsafe {
        GLOBAL_SCHEDULER.as_mut()
    }
}

/// Yield current CPU
pub fn yield_cpu() {
    if let Some(scheduler) = get_scheduler() {
        // For simplicity, assume CPU 0
        let _ = scheduler.schedule(0);
    }
}

/// Preempt current thread (called from timer interrupt)
pub fn preempt_current(cpu_id: u8) {
    if let Some(scheduler) = get_scheduler() {
        // Wake any sleeping threads first
        scheduler.wake_sleeping_threads();

        // Schedule next thread
        let _ = scheduler.schedule(cpu_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_affinity() {
        let mut affinity = CpuAffinity::new();
        assert!(affinity.contains(0));

        affinity.clear(0);
        assert!(!affinity.contains(0));

        affinity.set(1);
        assert!(affinity.contains(1));
        assert!(!affinity.contains(0));
    }

    #[test]
    fn test_micro_tcb_creation() {
        let tcb = MicroTcb::new(1, 10, SchedulingPolicy::Normal);
        assert_eq!(tcb.tid, 1);
        assert_eq!(tcb.priority, 10);
        assert_eq!(tcb.policy, SchedulingPolicy::Normal);
        assert!(tcb.is_ready());
    }

    #[test]
    fn test_cpu_scheduler() {
        let idle_tid = 999;
        let mut scheduler = CpuScheduler::new(0, idle_tid);

        let tid1 = 1;
        let tid2 = 2;

        assert_eq!(scheduler.enqueue(tid1), Ok(()));
        assert_eq!(scheduler.enqueue(tid2), Ok(()));

        assert_eq!(scheduler.dequeue(), Some(tid1));
        assert_eq!(scheduler.dequeue(), Some(tid2));
        assert_eq!(scheduler.dequeue(), None);
    }
}
