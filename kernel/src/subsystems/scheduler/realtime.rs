//! Real-Time Scheduling Implementation
//! 
//! This module provides comprehensive real-time scheduling with priority-based
//! scheduling, deadline monitoring, and resource reservation capabilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::subsystems::process::thread::{Thread, Tid, SchedPolicy, SchedParam};

/// Real-time scheduling policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePolicy {
    /// First-In, First-Out real-time scheduling
    Fifo,
    /// Round-Robin real-time scheduling
    RoundRobin,
    /// Earliest Deadline First scheduling
    EarliestDeadlineFirst,
    /// Rate Monotonic scheduling
    RateMonotonic,
    /// Constant Bandwidth Server
    ConstantBandwidthServer,
}

/// Real-time task parameters
#[derive(Debug, Clone)]
pub struct RealtimeTaskParams {
    /// Task ID
    pub task_id: Tid,
    /// Scheduling policy
    pub policy: RealtimePolicy,
    /// Static priority (1-99, higher = more important)
    pub priority: u8,
    /// Period in milliseconds (for periodic tasks)
    pub period_ms: u32,
    /// Execution time budget in milliseconds
    pub execution_time_ms: u32,
    /// Relative deadline in milliseconds
    pub deadline_ms: u32,
    /// CPU bandwidth reservation (percentage)
    pub bandwidth_percent: u32,
    /// Task is active
    pub active: bool,
    /// Task creation timestamp
    pub creation_time: u64,
    /// Next activation time
    pub next_activation: u64,
    /// Absolute deadline
    pub absolute_deadline: u64,
    /// Remaining execution time
    pub remaining_time: u32,
    /// Time slice for round-robin
    pub timeslice_ms: u32,
    /// Time slice remaining
    pub timeslice_remaining: u32,
}

/// Real-time scheduling statistics
#[derive(Debug, Default, Clone)]
pub struct RealtimeSchedulingStats {
    /// Total context switches
    pub total_context_switches: u64,
    /// Total missed deadlines
    pub total_missed_deadlines: u64,
    /// Total preemptions
    pub total_preemptions: u64,
    /// Average response time in microseconds
    pub avg_response_time_us: f64,
    /// Maximum response time in microseconds
    pub max_response_time_us: u64,
    /// CPU utilization percentage
    pub cpu_utilization_percent: f64,
    /// Real-time task count
    pub rt_task_count: usize,
    /// Overrun count
    pub total_overruns: u64,
}

/// Real-time scheduler
pub struct RealtimeScheduler {
    /// Real-time tasks by priority
    rt_tasks: Mutex<BTreeMap<u8, Vec<RealtimeTaskParams>>>,
    /// Currently running task
    current_task: Mutex<Option<Tid>>,
    /// Task parameters by TID
    task_params: Mutex<BTreeMap<Tid, RealtimeTaskParams>>,
    /// Scheduling statistics
    stats: Mutex<RealtimeSchedulingStats>,
    /// Last context switch time
    last_switch_time: AtomicU64,
    /// CPU bandwidth allocated
    allocated_bandwidth: AtomicUsize,
    /// Maximum allowed bandwidth (80% by default)
    max_bandwidth: AtomicUsize,
    /// Scheduler enabled
    enabled: AtomicBool,
}

impl RealtimeScheduler {
    /// Create a new real-time scheduler
    pub fn new() -> Self {
        Self {
            rt_tasks: Mutex::new(BTreeMap::new()),
            current_task: Mutex::new(None),
            task_params: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(RealtimeSchedulingStats::default()),
            last_switch_time: AtomicU64::new(0),
            allocated_bandwidth: AtomicUsize::new(0),
            max_bandwidth: AtomicUsize::new(80), // 80% of CPU
            enabled: AtomicBool::new(true),
        }
    }

    /// Enable or disable the real-time scheduler
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if the real-time scheduler is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Add a real-time task
    pub fn add_rt_task(&self, task: RealtimeTaskParams) -> Result<(), &'static str> {
        // Check bandwidth availability
        let current_bandwidth = self.allocated_bandwidth.load(Ordering::Relaxed);
        let max_bandwidth = self.max_bandwidth.load(Ordering::Relaxed);
        
        if current_bandwidth + task.bandwidth_percent as usize > max_bandwidth {
            return Err("Insufficient CPU bandwidth");
        }

        // Validate parameters
        if task.priority == 0 || task.priority > 99 {
            return Err("Invalid priority (must be 1-99)");
        }

        if task.execution_time_ms > task.deadline_ms {
            return Err("Execution time exceeds deadline");
        }

        // Add task to appropriate priority queue
        {
            let mut rt_tasks = self.rt_tasks.lock();
            rt_tasks.entry(task.priority).or_insert_with(Vec::new).push(task.clone());
        }

        // Store task parameters
        {
            let mut task_params = self.task_params.lock();
            task_params.insert(task.task_id, task);
        }

        // Update allocated bandwidth
        self.allocated_bandwidth.fetch_add(task.bandwidth_percent as usize, Ordering::Relaxed);

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.rt_task_count += 1;
        }

        Ok(())
    }

    /// Remove a real-time task
    pub fn remove_rt_task(&self, task_id: Tid) -> Result<(), &'static str> {
        // Get task parameters
        let task_params = {
            let mut params = self.task_params.lock();
            params.remove(&task_id).ok_or("Task not found")?
        };

        // Remove from priority queue
        {
            let mut rt_tasks = self.rt_tasks.lock();
            if let Some(tasks) = rt_tasks.get_mut(&task_params.priority) {
                tasks.retain(|t| t.task_id != task_id);
                
                // Remove empty priority queue
                if tasks.is_empty() {
                    rt_tasks.remove(&task_params.priority);
                }
            }
        }

        // Update allocated bandwidth
        self.allocated_bandwidth.fetch_sub(task_params.bandwidth_percent as usize, Ordering::Relaxed);

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.rt_task_count = stats.rt_task_count.saturating_sub(1);
        }

        Ok(())
    }

    /// Update real-time task parameters
    pub fn update_rt_task(&self, task_id: Tid, new_params: RealtimeTaskParams) -> Result<(), &'static str> {
        // Remove old task
        self.remove_rt_task(task_id)?;
        
        // Add new task
        self.add_rt_task(new_params)
    }

    /// Get the next real-time task to run
    pub fn pick_next_rt_task(&self, current_time: u64) -> Option<Tid> {
        if !self.is_enabled() {
            return None;
        }

        let rt_tasks = self.rt_tasks.lock();
        if rt_tasks.is_empty() {
            return None;
        }

        match self.select_policy() {
            RealtimePolicy::Fifo => self.pick_fifo_task(&rt_tasks, current_time),
            RealtimePolicy::RoundRobin => self.pick_rr_task(&rt_tasks, current_time),
            RealtimePolicy::EarliestDeadlineFirst => self.pick_edf_task(&rt_tasks, current_time),
            RealtimePolicy::RateMonotonic => self.pick_rm_task(&rt_tasks, current_time),
            RealtimePolicy::ConstantBandwidthServer => self.pick_cbs_task(&rt_tasks, current_time),
        }
    }

    /// Select the scheduling policy based on highest priority tasks
    fn select_policy(&self) -> RealtimePolicy {
        let rt_tasks = self.rt_tasks.lock();
        
        // Find the highest priority with active tasks
        for (&priority, tasks) in rt_tasks.iter().rev() {
            if tasks.iter().any(|t| t.active) {
                // Return the policy of the first active task at this priority
                if let Some(task) = tasks.iter().find(|t| t.active) {
                    return task.policy;
                }
            }
        }
        
        RealtimePolicy::Fifo // Default
    }

    /// Pick next task using FIFO policy
    fn pick_fifo_task(&self, rt_tasks: &BTreeMap<u8, Vec<RealtimeTaskParams>>, _current_time: u64) -> Option<Tid> {
        // Find highest priority with active tasks
        for (&priority, tasks) in rt_tasks.iter().rev() {
            if let Some(task) = tasks.iter().find(|t| t.active) {
                return Some(task.task_id);
            }
        }
        None
    }

    /// Pick next task using Round-Robin policy
    fn pick_rr_task(&self, rt_tasks: &BTreeMap<u8, Vec<RealtimeTaskParams>>, _current_time: u64) -> Option<Tid> {
        // Find highest priority with active tasks
        for (&priority, tasks) in rt_tasks.iter().rev() {
            if let Some(task) = tasks.iter().find(|t| t.active && t.timeslice_remaining > 0) {
                return Some(task.task_id);
            }
            
            // If all tasks at this priority have exhausted their timeslice, reset and pick first
            if let Some(task) = tasks.iter().find(|t| t.active) {
                return Some(task.task_id);
            }
        }
        None
    }

    /// Pick next task using Earliest Deadline First policy
    fn pick_edf_task(&self, rt_tasks: &BTreeMap<u8, Vec<RealtimeTaskParams>>, current_time: u64) -> Option<Tid> {
        let mut earliest_deadline = None;
        let mut selected_task = None;

        for tasks in rt_tasks.values() {
            for task in tasks {
                if task.active && task.policy == RealtimePolicy::EarliestDeadlineFirst {
                    let deadline = if task.absolute_deadline > 0 {
                        task.absolute_deadline
                    } else {
                        current_time + task.deadline_ms as u64 * 1000 // Convert to microseconds
                    };

                    if let Some(earliest) = earliest_deadline {
                        if deadline < earliest {
                            earliest_deadline = Some(deadline);
                            selected_task = Some(task.task_id);
                        }
                    } else {
                        earliest_deadline = Some(deadline);
                        selected_task = Some(task.task_id);
                    }
                }
            }
        }

        selected_task
    }

    /// Pick next task using Rate Monotonic policy
    fn pick_rm_task(&self, rt_tasks: &BTreeMap<u8, Vec<RealtimeTaskParams>>, current_time: u64) -> Option<Tid> {
        // Rate Monotonic: higher priority = shorter period
        let mut shortest_period = None;
        let mut selected_task = None;

        for tasks in rt_tasks.values() {
            for task in tasks {
                if task.active && task.policy == RealtimePolicy::RateMonotonic {
                    if let Some(shortest) = shortest_period {
                        if task.period_ms < shortest {
                            shortest_period = Some(task.period_ms);
                            selected_task = Some(task.task_id);
                        }
                    } else {
                        shortest_period = Some(task.period_ms);
                        selected_task = Some(task.task_id);
                    }
                }
            }
        }

        selected_task
    }

    /// Pick next task using Constant Bandwidth Server policy
    fn pick_cbs_task(&self, rt_tasks: &BTreeMap<u8, Vec<RealtimeTaskParams>>, current_time: u64) -> Option<Tid> {
        // CBS: similar to EDF but with bandwidth enforcement
        let mut earliest_deadline = None;
        let mut selected_task = None;

        for tasks in rt_tasks.values() {
            for task in tasks {
                if task.active && task.policy == RealtimePolicy::ConstantBandwidthServer {
                    // Check if task has remaining budget
                    if task.remaining_time > 0 {
                        let deadline = if task.absolute_deadline > 0 {
                            task.absolute_deadline
                        } else {
                            current_time + task.deadline_ms as u64 * 1000
                        };

                        if let Some(earliest) = earliest_deadline {
                            if deadline < earliest {
                                earliest_deadline = Some(deadline);
                                selected_task = Some(task.task_id);
                            }
                        } else {
                            earliest_deadline = Some(deadline);
                            selected_task = Some(task.task_id);
                        }
                    }
                }
            }
        }

        selected_task
    }

    /// Handle context switch to a real-time task
    pub fn handle_context_switch(&self, new_task_id: Tid, current_time: u64) {
        let old_task_id = {
            let mut current = self.current_task.lock();
            let old = *current;
            *current = Some(new_task_id);
            old
        };

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_context_switches += 1;

            // Calculate response time
            if let Some(old_id) = old_task_id {
                if let Some(old_params) = self.get_task_params(old_id) {
                    let response_time = current_time - old_params.creation_time;
                    stats.avg_response_time_us = 
                        (stats.avg_response_time_us * (stats.total_context_switches - 1) as f64 + response_time as f64) 
                        / stats.total_context_switches as f64;
                    stats.max_response_time_us = stats.max_response_time_us.max(response_time);
                }
            }
        }

        // Update last switch time
        self.last_switch_time.store(current_time, Ordering::Relaxed);

        // Update task state
        if let Some(mut task_params) = self.get_task_params_mut(new_task_id) {
            task_params.creation_time = current_time;
            
            // Reset timeslice for RR tasks
            if task_params.policy == RealtimePolicy::RoundRobin {
                task_params.timeslice_remaining = task_params.timeslice_ms;
            }
        }
    }

    /// Update task execution time
    pub fn update_task_execution(&self, task_id: Tid, elapsed_ms: u32, current_time: u64) {
        if let Some(mut task_params) = self.get_task_params_mut(task_id) {
            // Update remaining execution time
            if task_params.remaining_time >= elapsed_ms {
                task_params.remaining_time -= elapsed_ms;
            } else {
                // Task overran its budget
                task_params.remaining_time = 0;
                
                // Update statistics
                let mut stats = self.stats.lock();
                stats.total_overruns += 1;
            }

            // Update timeslice for RR tasks
            if task_params.policy == RealtimePolicy::RoundRobin {
                if task_params.timeslice_remaining >= elapsed_ms {
                    task_params.timeslice_remaining -= elapsed_ms;
                } else {
                    task_params.timeslice_remaining = 0;
                }
            }

            // Check for deadline miss
            if current_time > task_params.absolute_deadline && task_params.absolute_deadline > 0 {
                let mut stats = self.stats.lock();
                stats.total_missed_deadlines += 1;
            }
        }
    }

    /// Activate a real-time task
    pub fn activate_task(&self, task_id: Tid, current_time: u64) -> Result<(), &'static str> {
        if let Some(mut task_params) = self.get_task_params_mut(task_id) {
            task_params.active = true;
            task_params.next_activation = current_time;
            
            // Set absolute deadline
            if task_params.deadline_ms > 0 {
                task_params.absolute_deadline = current_time + task_params.deadline_ms as u64 * 1000;
            }
            
            // Reset execution time budget
            task_params.remaining_time = task_params.execution_time_ms;
            
            // Reset timeslice
            task_params.timeslice_remaining = task_params.timeslice_ms;
            
            Ok(())
        } else {
            Err("Task not found")
        }
    }

    /// Deactivate a real-time task
    pub fn deactivate_task(&self, task_id: Tid) -> Result<(), &'static str> {
        if let Some(mut task_params) = self.get_task_params_mut(task_id) {
            task_params.active = false;
            task_params.absolute_deadline = 0;
            Ok(())
        } else {
            Err("Task not found")
        }
    }

    /// Get task parameters
    fn get_task_params(&self, task_id: Tid) -> Option<RealtimeTaskParams> {
        let task_params = self.task_params.lock();
        task_params.get(&task_id).cloned()
    }

    /// Get mutable task parameters
    fn get_task_params_mut(&self, task_id: Tid) -> Option<impl core::ops::DerefMut<Target = RealtimeTaskParams> + '_> {
        use core::ops::DerefMut;
        
        struct TaskRef<'a> {
            params: &'a mut RealtimeTaskParams,
        }
        
        impl<'a> DerefMut for TaskRef<'a> {
            type Target = RealtimeTaskParams;
            
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.params
            }
        }
        
        let mut task_params = self.task_params.lock();
        if task_params.contains_key(&task_id) {
            Some(TaskRef {
                params: task_params.get_mut(&task_id).unwrap(),
            })
        } else {
            None
        }
    }

    /// Get scheduling statistics
    pub fn get_stats(&self) -> RealtimeSchedulingStats {
        let mut stats = self.stats.lock();
        
        // Calculate CPU utilization
        let current_time = crate::subsystems::time::timestamp_nanos();
        let last_switch = self.last_switch_time.load(Ordering::Relaxed);
        
        if current_time > last_switch {
            let elapsed = current_time - last_switch;
            let allocated_bandwidth = self.allocated_bandwidth.load(Ordering::Relaxed);
            stats.cpu_utilization_percent = (allocated_bandwidth as f64 * elapsed as f64) / (100.0 * 1000000.0);
        }
        
        stats.clone()
    }

    /// Reset scheduling statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = RealtimeSchedulingStats::default();
    }

    /// Set maximum CPU bandwidth for real-time tasks
    pub fn set_max_bandwidth(&self, max_percent: u32) {
        self.max_bandwidth.store(max_percent as usize, Ordering::Relaxed);
    }

    /// Get current CPU bandwidth allocation
    pub fn get_allocated_bandwidth(&self) -> usize {
        self.allocated_bandwidth.load(Ordering::Relaxed)
    }

    /// Get maximum allowed CPU bandwidth
    pub fn get_max_bandwidth(&self) -> usize {
        self.max_bandwidth.load(Ordering::Relaxed)
    }

    /// Check if a task can be admitted based on schedulability analysis
    pub fn check_admission(&self, new_task: &RealtimeTaskParams) -> bool {
        let current_bandwidth = self.allocated_bandwidth.load(Ordering::Relaxed);
        let max_bandwidth = self.max_bandwidth.load(Ordering::Relaxed);
        
        // Simple utilization test
        if current_bandwidth + new_task.bandwidth_percent as usize > max_bandwidth {
            return false;
        }

        // For EDF tasks, do exact schedulability test
        if new_task.policy == RealtimePolicy::EarliestDeadlineFirst {
            let task_params = self.task_params.lock();
            let mut total_utilization = 0.0;
            
            // Add existing EDF tasks
            for task in task_params.values() {
                if task.policy == RealtimePolicy::EarliestDeadlineFirst {
                    total_utilization += task.execution_time_ms as f64 / task.deadline_ms as f64;
                }
            }
            
            // Add new task
            total_utilization += new_task.execution_time_ms as f64 / new_task.deadline_ms as f64;
            
            // EDF schedulability test: utilization <= 1.0
            return total_utilization <= 1.0;
        }

        // For Rate Monotonic tasks, do Liu & Layland test
        if new_task.policy == RealtimePolicy::RateMonotonic {
            let task_params = self.task_params.lock();
            let mut tasks = Vec::new();
            
            // Collect existing RM tasks
            for task in task_params.values() {
                if task.policy == RealtimePolicy::RateMonotonic {
                    tasks.push(task);
                }
            }
            
            // Add new task
            tasks.push(new_task);
            
            // Sort by period (shorter period = higher priority)
            tasks.sort_by(|a, b| a.period_ms.cmp(&b.period_ms));
            
            // Liu & Layland test
            let mut total_utilization = 0.0;
            for (i, task) in tasks.iter().enumerate() {
                total_utilization += task.execution_time_ms as f64 / task.period_ms as f64;
                
                let n = i + 1;
                let bound = n as f64 * ((n as f64).powf(1.0 / n as f64) - 1.0);
                
                if total_utilization > bound {
                    return false;
                }
            }
            
            return true;
        }

        // Default: admit if bandwidth is available
        true
    }
}

/// Global real-time scheduler instance
static mut RT_SCHEDULER: Option<RealtimeScheduler> = None;
static RT_SCHEDULER_INIT: spin::Once = spin::Once::new();

/// Initialize the real-time scheduler
pub fn init_rt_scheduler() {
    RT_SCHEDULER_INIT.call_once(|| {
        unsafe {
            RT_SCHEDULER = Some(RealtimeScheduler::new());
        }
    });
}

/// Get the global real-time scheduler
pub fn get_rt_scheduler() -> Option<&'static RealtimeScheduler> {
    unsafe {
        RT_SCHEDULER.as_ref()
    }
}

/// Convert thread scheduling policy to real-time policy
pub fn thread_policy_to_rt(policy: SchedPolicy) -> RealtimePolicy {
    match policy {
        SchedPolicy::Fifo => RealtimePolicy::Fifo,
        SchedPolicy::RoundRobin => RealtimePolicy::RoundRobin,
        _ => RealtimePolicy::Fifo, // Default
    }
}

/// Convert real-time policy to thread scheduling policy
pub fn rt_policy_to_thread(policy: RealtimePolicy) -> SchedPolicy {
    match policy {
        RealtimePolicy::Fifo => SchedPolicy::Fifo,
        RealtimePolicy::RoundRobin => SchedPolicy::RoundRobin,
        _ => SchedPolicy::Fifo, // Default
    }
}

/// Check if a scheduling policy is real-time
pub fn is_realtime_policy(policy: SchedPolicy) -> bool {
    matches!(policy, SchedPolicy::Fifo | SchedPolicy::RoundRobin)
}

/// Check if a real-time policy is valid
pub fn is_valid_rt_policy(policy: RealtimePolicy) -> bool {
    matches!(policy, 
        RealtimePolicy::Fifo | 
        RealtimePolicy::RoundRobin | 
        RealtimePolicy::EarliestDeadlineFirst | 
        RealtimePolicy::RateMonotonic | 
        RealtimePolicy::ConstantBandwidthServer
    )
}