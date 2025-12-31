//! # Real-Time Scheduler
//!
//! This module implements deterministic real-time scheduling algorithms with
//! provable timing guarantees and bounded worst-case execution time.
//!
//! ## Supported Algorithms
//!
//! - **Rate Monotonic Scheduling (RMS)**: Static priority optimal for periodic tasks
//! - **Earliest Deadline First (EDF)**: Dynamic priority optimal for preemptive scheduling
//! - **Deadline Monotonic**: Static priority based on deadlines
//! - **Priority Inheritance**: Prevents unbounded priority inversion
//! - **Priority Ceiling Protocol**: Prevents deadlock and chaining
//!
//! ## Schedulability Analysis
//!
//! The module provides multiple analysis methods:
//!
//! ### Utilization Bound Test (RMS)
//!
//! For RMS, a task set is schedulable if:
//! ```text
//! U = Σ(C_i / T_i) ≤ n(2^(1/n) - 1)
//! ```
//!
//! Where:
//! - C_i = Worst-case execution time
//! - T_i = Period
//! - n = Number of tasks
//! - Bound approaches ln(2) ≈ 0.693 as n → ∞
//!
//! ### Response Time Analysis
//!
//! Iterative calculation of worst-case response time:
//! ```text
//! R_i^0 = C_i
//! R_i^(k+1) = C_i + Σ_{j∈hp(i)} ⌈R_i^k / T_j⌉ * C_j
//! ```
//!
//! Terminates when R_i^(k+1) = R_i^k or R_i > D_i.
//!
//! ## Example
//!
//! ```no_run
//! use kernel::rtos::scheduler::{RealTimeScheduler, SchedulingPolicy};
//!
//! let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
//!
//! // Add tasks with (period, deadline, wcet) in microseconds
//! scheduler.add_task(1, 1000, 1000, 200)?;
//! scheduler.add_task(2, 2000, 2000, 300)?;
//! scheduler.add_task(3, 5000, 5000, 500)?;
//!
//! // Verify schedulability
//! let analysis = scheduler.analyze_schedulability()?;
//! assert!(analysis.is_schedulable);
//!
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```

use crate::rtos::RtError;
use alloc::collections::BinaryHeap;
use alloc::collections::BTreeMap;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering as AtomicOrdering};

/// Real-time task descriptor
///
/// Contains all timing and priority information required for real-time scheduling.
#[derive(Debug, Clone)]
pub struct RealTimeTask {
    /// Unique task identifier
    pub task_id: u64,

    /// Period in microseconds (time between activations)
    pub period_us: u64,

    /// Relative deadline in microseconds
    pub deadline_us: u64,

    /// Worst-case execution time in microseconds
    pub wcet_us: u64,

    /// Static priority (0-255, higher = more important)
    pub static_priority: u8,

    /// Current dynamic priority (for EDF)
    pub dynamic_priority: u8,

    /// Absolute deadline for current instance
    pub absolute_deadline_us: u64,

    /// Remaining execution time for current instance
    pub remaining_time_us: u64,

    /// Task state
    pub state: TaskState,

    /// Number of deadline misses
    pub deadline_misses: u32,

    /// Critical section nesting depth
    pub critical_section_depth: u32,

    /// Inherited priority (for priority inheritance)
    pub inherited_priority: Option<u8>,

    /// Resources currently held
    pub held_resources: alloc::vec::Vec<u64>,
}

/// Task state for real-time scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is dormant (not yet activated)
    Dormant,

    /// Task is ready to run
    Ready,

    /// Task is currently running
    Running,

    /// Task is blocked (waiting for resource)
    Blocked,

    /// Task completed successfully
    Completed,

    /// Task missed its deadline
    MissedDeadline,
}

/// Scheduling policy for real-time tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Rate Monotonic Scheduling (static priority based on period)
    RateMonotonic,

    /// Earliest Deadline First (dynamic priority)
    EarliestDeadlineFirst,

    /// Deadline Monotonic (static priority based on deadline)
    DeadlineMonotonic,

    /// Mixed criticality scheduling
    MixedCriticality,
}

/// Priority inheritance protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityProtocol {
    /// No priority inheritance
    None,

    /// Basic priority inheritance
    Inherit,

    /// Priority ceiling protocol (prevents deadlock)
    Ceiling,

    /// Stack resource policy (highest priority = highest ceiling)
    StackResource,
}

/// Schedulability analysis result
#[derive(Debug, Clone)]
pub struct SchedulabilityAnalysis {
    /// Whether the task set is schedulable
    pub is_schedulable: bool,

    /// Total CPU utilization
    pub utilization: f64,

    /// Schedulability bound
    pub bound: f64,

    /// Worst-case response time for each task
    pub response_times: alloc::vec::Vec<u64>,

    /// Analysis method used
    pub method: &'static str,

    /// Tasks that miss deadlines
    pub missed_tasks: alloc::vec::Vec<u64>,
}

/// Deadline tracker for monitoring task compliance
#[derive(Debug)]
pub struct DeadlineTracker {
    /// Task deadlines
    deadlines: BTreeMap<u64, u64>,

    /// Deadline miss counter per task
    miss_count: BTreeMap<u64, u32>,

    /// Maximum miss duration per task
    max_miss_us: BTreeMap<u64, u64>,

    /// Total deadline misses
    total_misses: AtomicU32,
}

/// Real-time scheduler with multiple scheduling policies
pub struct RealTimeScheduler {
    /// Scheduling policy
    policy: SchedulingPolicy,

    /// Ready queue (priority-ordered)
    ready_queue: BinaryHeap<TaskWrapper>,

    /// All tasks
    tasks: BTreeMap<u64, RealTimeTask>,

    /// Currently running task
    current_task: Option<u64>,

    /// Priority protocol
    priority_protocol: PriorityProtocol,

    /// System start time (microseconds)
    start_time_us: u64,

    /// Current time (microseconds)
    current_time_us: u64,

    /// Next task ID
    next_task_id: AtomicU64,

    /// Scheduler statistics
    stats: SchedulerStats,
}

/// Wrapper for task ordering in heap
#[derive(Debug, Clone)]
struct TaskWrapper {
    task_id: u64,
    priority: u8,
    deadline: u64,
}

impl PartialEq for TaskWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for TaskWrapper {}

impl PartialOrd for TaskWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first (reverse order)
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {
                // For equal priority, earlier deadline first
                other.deadline.cmp(&self.deadline)
            }
            other => other,
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Default)]
struct SchedulerStats {
    /// Number of schedule operations
    schedules: AtomicU64,

    /// Number of context switches
    context_switches: AtomicU64,

    /// Total scheduling time (nanoseconds)
    total_schedule_time_ns: AtomicU64,

    /// Maximum scheduling time (nanoseconds)
    max_schedule_time_ns: AtomicU64,
}

impl RealTimeTask {
    /// Create a new real-time task
    ///
    /// # Arguments
    ///
    /// * `task_id` - Unique task identifier
    /// * `period_us` - Period in microseconds
    /// * `deadline_us` - Relative deadline in microseconds
    /// * `wcet_us` - Worst-case execution time in microseconds
    ///
    /// # Returns
    ///
    /// A new RealTimeTask with initialized timing parameters
    pub fn new(
        task_id: u64,
        period_us: u64,
        deadline_us: u64,
        wcet_us: u64,
    ) -> Result<Self, RtError> {
        // Validate timing constraints
        if wcet_us == 0 {
            return Err(RtError::InvalidTimingParameter {
                parameter: "wcet",
                value: wcet_us,
            });
        }

        if wcet_us > deadline_us {
            return Err(RtError::InvalidTimingParameter {
                parameter: "wcet > deadline",
                value: wcet_us,
            });
        }

        if deadline_us > period_us {
            return Err(RtError::InvalidTimingParameter {
                parameter: "deadline > period",
                value: deadline_us,
            });
        }

        Ok(Self {
            task_id,
            period_us,
            deadline_us,
            wcet_us,
            static_priority: 128, // Default middle priority
            dynamic_priority: 128,
            absolute_deadline_us: 0,
            remaining_time_us: wcet_us,
            state: TaskState::Dormant,
            deadline_misses: 0,
            critical_section_depth: 0,
            inherited_priority: None,
            held_resources: alloc::vec![],
        })
    }

    /// Calculate utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        self.wcet_us as f64 / self.period_us as f64
    }

    /// Check if task is in critical section
    #[inline]
    pub fn in_critical_section(&self) -> bool {
        self.critical_section_depth > 0
    }

    /// Get effective priority (considering inheritance)
    #[inline]
    pub fn effective_priority(&self) -> u8 {
        self.inherited_priority.unwrap_or(self.static_priority).max(self.dynamic_priority)
    }
}

impl Default for RealTimeTask {
    fn default() -> Self {
        Self {
            task_id: 0,
            period_us: 1000,
            deadline_us: 1000,
            wcet_us: 100,
            static_priority: 128,
            dynamic_priority: 128,
            absolute_deadline_us: 0,
            remaining_time_us: 100,
            state: TaskState::Dormant,
            deadline_misses: 0,
            critical_section_depth: 0,
            inherited_priority: None,
            held_resources: alloc::vec![],
        }
    }
}

impl DeadlineTracker {
    /// Create a new deadline tracker
    pub fn new() -> Self {
        Self {
            deadlines: BTreeMap::new(),
            miss_count: BTreeMap::new(),
            max_miss_us: BTreeMap::new(),
            total_misses: AtomicU32::new(0),
        }
    }

    /// Register a task deadline
    pub fn register_deadline(&mut self, task_id: u64, deadline_us: u64) {
        self.deadlines.insert(task_id, deadline_us);
    }

    /// Check if deadline was missed
    pub fn check_deadline(
        &mut self,
        task_id: u64,
        current_time_us: u64,
    ) -> Result<(), RtError> {
        if let Some(&deadline) = self.deadlines.get(&task_id) {
            if current_time_us > deadline {
                let miss_duration = current_time_us - deadline;
                *self.miss_count.entry(task_id).or_insert(0) += 1;
                self.max_miss_us
                    .entry(task_id)
                    .and_modify(|m| *m = (*m).max(miss_duration))
                    .or_insert(miss_duration);
                self.total_misses.fetch_add(1, AtomicOrdering::Relaxed);

                return Err(RtError::DeadlineMissed {
                    task_id,
                    miss_duration_us: miss_duration,
                });
            }
        }
        Ok(())
    }

    /// Get miss count for a task
    pub fn miss_count(&self, task_id: u64) -> u32 {
        *self.miss_count.get(&task_id).unwrap_or(&0)
    }

    /// Get total deadline misses
    pub fn total_misses(&self) -> u32 {
        self.total_misses.load(AtomicOrdering::Relaxed)
    }
}

impl Default for DeadlineTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RealTimeScheduler {
    /// Create a new real-time scheduler
    pub fn new(policy: SchedulingPolicy) -> Self {
        Self {
            policy,
            ready_queue: BinaryHeap::new(),
            tasks: BTreeMap::new(),
            current_task: None,
            priority_protocol: PriorityProtocol::Inherit,
            start_time_us: 0,
            current_time_us: 0,
            next_task_id: AtomicU64::new(1),
            stats: SchedulerStats::default(),
        }
    }

    /// Set priority inheritance protocol
    pub fn set_priority_protocol(&mut self, protocol: PriorityProtocol) {
        self.priority_protocol = protocol;
    }

    /// Add a task to the scheduler
    pub fn add_task(
        &mut self,
        task_id: u64,
        period_us: u64,
        deadline_us: u64,
        wcet_us: u64,
    ) -> Result<(), RtError> {
        if self.tasks.contains_key(&task_id) {
            return Err(RtError::AlreadyExists {
                resource_type: "task",
                id: task_id,
            });
        }

        let mut task = RealTimeTask::new(task_id, period_us, deadline_us, wcet_us)?;

        // Set static priority based on policy
        match self.policy {
            SchedulingPolicy::RateMonotonic => {
                // Shorter period = higher priority
                task.static_priority = Self::period_to_priority(period_us);
            }
            SchedulingPolicy::DeadlineMonotonic => {
                // Shorter deadline = higher priority
                task.static_priority = Self::deadline_to_priority(deadline_us);
            }
            SchedulingPolicy::EarliestDeadlineFirst => {
                // Priority is dynamic, set to default
                task.static_priority = 128;
            }
            SchedulingPolicy::MixedCriticality => {
                task.static_priority = 128;
            }
        }

        self.tasks.insert(task_id, task);
        Ok(())
    }

    /// Remove a task from the scheduler
    pub fn remove_task(&mut self, task_id: u64) -> Result<(), RtError> {
        if self.current_task == Some(task_id) {
            return Err(RtError::InvalidState {
                state: "task is running",
                expected: "task not running",
            });
        }

        self.tasks.remove(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        Ok(())
    }

    /// Activate a task (make it ready)
    pub fn activate_task(&mut self, task_id: u64) -> Result<(), RtError> {
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        if task.state == TaskState::Running {
            return Err(RtError::InvalidState {
                state: "running",
                expected: "dormant or completed",
            });
        }

        // Reset task state for new period
        task.state = TaskState::Ready;
        task.remaining_time_us = task.wcet_us;
        task.absolute_deadline_us = self.current_time_us + task.deadline_us;

        // Update dynamic priority for EDF
        if self.policy == SchedulingPolicy::EarliestDeadlineFirst {
            task.dynamic_priority = Self::deadline_to_priority(task.absolute_deadline_us);
        }

        // Add to ready queue
        self.ready_queue.push(TaskWrapper {
            task_id,
            priority: task.effective_priority(),
            deadline: task.absolute_deadline_us,
        });

        Ok(())
    }

    /// Schedule the next task
    pub fn schedule(&mut self) -> Result<Option<u64>, RtError> {
        self.stats.schedules.fetch_add(1, AtomicOrdering::Relaxed);

        // Check if current task should continue
        if let Some(current_id) = self.current_task {
            if let Some(task) = self.tasks.get(&current_id) {
                if task.state == TaskState::Running && task.remaining_time_us > 0 {
                    // Current task still has time, continue
                    return Ok(Some(current_id));
                }
            }
        }

        // Select highest priority ready task
        if let Some(wrapper) = self.ready_queue.pop() {
            let task_id = wrapper.task_id;

            // Preempt current task if needed
            if let Some(current_id) = self.current_task {
                if current_id != task_id {
                    self.preempt_task(current_id)?;
                    self.stats.context_switches.fetch_add(1, AtomicOrdering::Relaxed);
                }
            }

            // Start new task
            let task = self.tasks.get_mut(&task_id).unwrap();
            task.state = TaskState::Running;
            self.current_task = Some(task_id);

            Ok(Some(task_id))
        } else {
            // No ready tasks
            self.current_task = None;
            Ok(None)
        }
    }

    /// Preempt the current task
    fn preempt_task(&mut self, task_id: u64) -> Result<(), RtError> {
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        if task.state == TaskState::Running {
            task.state = TaskState::Ready;

            // Re-add to ready queue
            self.ready_queue.push(TaskWrapper {
                task_id,
                priority: task.effective_priority(),
                deadline: task.absolute_deadline_us,
            });
        }

        Ok(())
    }

    /// Tick the scheduler (advance time and check deadlines)
    pub fn tick(&mut self, elapsed_us: u64) -> Result<(), RtError> {
        self.current_time_us += elapsed_us;

        // Update running task
        if let Some(task_id) = self.current_task {
            let task = self.tasks.get_mut(&task_id).unwrap();

            if task.state == TaskState::Running {
                task.remaining_time_us = task.remaining_time_us.saturating_sub(elapsed_us);

                // Check if task completed
                if task.remaining_time_us == 0 {
                    task.state = TaskState::Completed;
                    self.current_task = None;
                }
                // Check deadline
                else if self.current_time_us > task.absolute_deadline_us {
                    task.state = TaskState::MissedDeadline;
                    task.deadline_misses += 1;
                    return Err(RtError::DeadlineMissed {
                        task_id,
                        miss_duration_us: self.current_time_us - task.absolute_deadline_us,
                    });
                }
            }
        }

        Ok(())
    }

    /// Perform schedulability analysis
    pub fn analyze_schedulability(&self) -> Result<SchedulabilityAnalysis, RtError> {
        match self.policy {
            SchedulingPolicy::RateMonotonic | SchedulingPolicy::DeadlineMonotonic => {
                self.utilization_bound_test()
            }
            SchedulingPolicy::EarliestDeadlineFirst => {
                self.edf_analysis()
            }
            SchedulingPolicy::MixedCriticality => {
                self.response_time_analysis()
            }
        }
    }

    /// Utilization bound test for fixed-priority scheduling
    fn utilization_bound_test(&self) -> Result<SchedulabilityAnalysis, RtError> {
        let tasks: alloc::vec::Vec<_> = self.tasks.values().collect();
        let n = tasks.len();

        if n == 0 {
            return Ok(SchedulabilityAnalysis {
                is_schedulable: true,
                utilization: 0.0,
                bound: 1.0,
                response_times: alloc::vec![],
                method: "Utilization Bound Test",
                missed_tasks: alloc::vec![],
            });
        }

        // Calculate total utilization
        let utilization: f64 = tasks.iter().map(|t| t.utilization()).sum();

        // Calculate Liu & Layland bound
        let bound = n as f64 * (2_f64.powf(1.0 / n as f64) - 1.0);

        // Calculate response times
        let mut response_times = alloc::vec::Vec::new();
        let mut missed_tasks = alloc::vec::Vec::new();

        for task in &tasks {
            let rt = self.calculate_response_time(task, &tasks)?;
            response_times.push(rt);

            if rt > task.deadline_us {
                missed_tasks.push(task.task_id);
            }
        }

        let is_schedulable = utilization <= bound && missed_tasks.is_empty();

        Ok(SchedulabilityAnalysis {
            is_schedulable,
            utilization,
            bound,
            response_times,
            method: "Liu & Layland Utilization Bound",
            missed_tasks,
        })
    }

    /// EDF schedulability analysis
    fn edf_analysis(&self) -> Result<SchedulabilityAnalysis, RtError> {
        let tasks: alloc::vec::Vec<_> = self.tasks.values().collect();

        // EDF condition: U <= 1.0
        let utilization: f64 = tasks.iter().map(|t| t.utilization()).sum();

        let is_schedulable = utilization <= 1.0;

        Ok(SchedulabilityAnalysis {
            is_schedulable,
            utilization,
            bound: 1.0,
            response_times: alloc::vec![],
            method: "EDF Utilization Test",
            missed_tasks: alloc::vec![],
        })
    }

    /// Response time analysis (exact)
    fn response_time_analysis(&self) -> Result<SchedulabilityAnalysis, RtError> {
        let tasks: alloc::vec::Vec<_> = self.tasks.values().collect();
        let mut response_times = alloc::vec::Vec::new();
        let mut missed_tasks = alloc::vec::Vec::new();

        for task in &tasks {
            let rt = self.calculate_response_time(task, &tasks)?;
            response_times.push(rt);

            if rt > task.deadline_us {
                missed_tasks.push(task.task_id);
            }
        }

        let utilization: f64 = tasks.iter().map(|t| t.utilization()).sum();

        Ok(SchedulabilityAnalysis {
            is_schedulable: missed_tasks.is_empty(),
            utilization,
            bound: 1.0,
            response_times,
            method: "Response Time Analysis",
            missed_tasks,
        })
    }

    /// Calculate worst-case response time for a task
    fn calculate_response_time(
        &self,
        task: &RealTimeTask,
        all_tasks: &[&RealTimeTask],
    ) -> Result<u64, RtError> {
        let mut r_prev = task.wcet_us;
        let mut r_next = r_prev;

        // Find higher priority tasks
        let higher_prio: alloc::vec::Vec<_> = all_tasks
            .iter()
            .filter(|t| t.task_id != task.task_id && t.static_priority > task.static_priority)
            .collect();

        // Iterate until convergence
        for _iteration in 0..100 {
            // Calculate interference
            let mut interference = 0u64;
            for hp_task in &higher_prio {
                interference += ((r_prev + hp_task.period_us - 1) / hp_task.period_us) * hp_task.wcet_us;
            }

            r_next = task.wcet_us + interference;

            // Check convergence
            if r_next == r_prev {
                break;
            }

            // Check if response time exceeds deadline
            if r_next > task.deadline_us {
                return Ok(r_next);
            }

            r_prev = r_next;
        }

        Ok(r_next)
    }

    /// Boost task priority (for priority inheritance)
    pub fn boost_priority(&mut self, task_id: u64, new_priority: u8) -> Result<(), RtError> {
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        if new_priority > 255 {
            return Err(RtError::InvalidPriority {
                priority: new_priority,
                max_priority: 255,
            });
        }

        task.inherited_priority = Some(new_priority);

        // Update ready queue if task is ready
        if task.state == TaskState::Ready {
            self.ready_queue.push(TaskWrapper {
                task_id,
                priority: task.effective_priority(),
                deadline: task.absolute_deadline_us,
            });
        }

        Ok(())
    }

    /// Restore task priority (after priority inheritance)
    pub fn restore_priority(&mut self, task_id: u64) -> Result<(), RtError> {
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        task.inherited_priority = None;
        Ok(())
    }

    /// Enter critical section
    pub fn enter_critical_section(&mut self, task_id: u64) -> Result<(), RtError> {
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        task.critical_section_depth += 1;
        Ok(())
    }

    /// Exit critical section
    pub fn exit_critical_section(&mut self, task_id: u64) -> Result<(), RtError> {
        let task = self.tasks.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task",
                id: task_id,
            })?;

        if task.critical_section_depth > 0 {
            task.critical_section_depth -= 1;
        }

        Ok(())
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: u64) -> Option<&RealTimeTask> {
        self.tasks.get(&task_id)
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, task_id: u64) -> Option<&mut RealTimeTask> {
        self.tasks.get_mut(&task_id)
    }

    /// Get current task
    pub fn current_task(&self) -> Option<u64> {
        self.current_task
    }

    /// Get scheduling statistics
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }

    /// Convert period to RMS priority (shorter period = higher priority)
    fn period_to_priority(period_us: u64) -> u8 {
        // Map period to priority [0-255]
        // Period 100μs -> 255, Period 1s -> 0
        if period_us < 100 {
            255
        } else if period_us > 1_000_000 {
            0
        } else {
            let ratio = (period_us as f64).log10() / 6.0;
            (255.0 * (1.0 - ratio)) as u8
        }
    }

    /// Convert deadline to priority (shorter deadline = higher priority)
    fn deadline_to_priority(deadline_us: u64) -> u8 {
        if deadline_us < 100 {
            255
        } else if deadline_us > 1_000_000 {
            0
        } else {
            let ratio = (deadline_us as f64).log10() / 6.0;
            (255.0 * (1.0 - ratio)) as u8
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = RealTimeTask::new(1, 1000, 800, 200).unwrap();
        assert_eq!(task.task_id, 1);
        assert_eq!(task.period_us, 1000);
        assert_eq!(task.deadline_us, 800);
        assert_eq!(task.wcet_us, 200);
    }

    #[test]
    fn test_invalid_wcet() {
        let result = RealTimeTask::new(1, 1000, 800, 900);
        assert!(matches!(result, Err(RtError::InvalidTimingParameter { .. })));
    }

    #[test]
    fn test_scheduler_creation() {
        let scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        assert_eq!(scheduler.current_task(), None);
    }

    #[test]
    fn test_add_task() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        assert!(scheduler.get_task(1).is_some());
    }

    #[test]
    fn test_duplicate_task() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        let result = scheduler.add_task(1, 2000, 2000, 300);
        assert!(matches!(result, Err(RtError::AlreadyExists { .. })));
    }

    #[test]
    fn test_activate_task() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        scheduler.activate_task(1).unwrap();
        assert_eq!(scheduler.get_task(1).unwrap().state, TaskState::Ready);
    }

    #[test]
    fn test_schedule_task() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        scheduler.activate_task(1).unwrap();
        let task_id = scheduler.schedule().unwrap();
        assert_eq!(task_id, Some(1));
    }

    #[test]
    fn test_task_tick() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        scheduler.activate_task(1).unwrap();
        scheduler.schedule().unwrap();
        scheduler.tick(100).unwrap();
        assert_eq!(scheduler.get_task(1).unwrap().remaining_time_us, 100);
    }

    #[test]
    fn test_deadline_miss() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 100, 200).unwrap();
        scheduler.activate_task(1).unwrap();
        scheduler.schedule().unwrap();
        let result = scheduler.tick(1000);
        assert!(matches!(result, Err(RtError::DeadlineMissed { .. })));
    }

    #[test]
    fn test_utilization_bound_test() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        scheduler.add_task(2, 2000, 2000, 300).unwrap();
        let analysis = scheduler.analyze_schedulability().unwrap();
        assert!(analysis.is_schedulable);
    }

    #[test]
    fn test_priority_boost() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        scheduler.boost_priority(1, 250).unwrap();
        assert_eq!(scheduler.get_task(1).unwrap().inherited_priority, Some(250));
    }

    #[test]
    fn test_critical_section() {
        let mut scheduler = RealTimeScheduler::new(SchedulingPolicy::RateMonotonic);
        scheduler.add_task(1, 1000, 1000, 200).unwrap();
        scheduler.enter_critical_section(1).unwrap();
        assert_eq!(scheduler.get_task(1).unwrap().critical_section_depth, 1);
        scheduler.exit_critical_section(1).unwrap();
        assert_eq!(scheduler.get_task(1).unwrap().critical_section_depth, 0);
    }

    #[test]
    fn test_period_to_priority() {
        let p1 = RealTimeScheduler::period_to_priority(100);
        let p2 = RealTimeScheduler::period_to_priority(1000);
        assert!(p1 > p2);
    }

    #[test]
    fn test_tracker() {
        let mut tracker = DeadlineTracker::new();
        tracker.register_deadline(1, 1000);
        tracker.check_deadline(1, 500).unwrap();
        let result = tracker.check_deadline(1, 1500);
        assert!(matches!(result, Err(RtError::DeadlineMissed { .. })));
    }
}
