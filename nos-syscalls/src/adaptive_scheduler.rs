//! Adaptive Scheduling Algorithm
//!
//! This module provides an adaptive scheduling algorithm implementation
//! for optimizing task scheduling in NOS operating system.

use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
    string::{String, ToString},
    boxed::Box,
    format,
};
use spin::Mutex;
use nos_api::Result;
use crate::{SyscallHandler, SyscallDispatcher};
use crate::logging::output_report;
use core::sync::atomic::{AtomicU64, Ordering};

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Real-time priority (highest)
    Realtime = 0,
    /// High priority
    High = 1,
    /// Normal priority
    Normal = 2,
    /// Low priority
    Low = 3,
    /// Idle priority (lowest)
    Idle = 4,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is blocked waiting for an event
    Blocked,
    /// Task has finished execution
    Finished,
    /// Task is terminated
    Terminated,
}

/// Task statistics for adaptive scheduling
#[derive(Debug, Clone)]
pub struct TaskStats {
    /// Task ID
    pub task_id: u64,
    /// Total execution time in microseconds
    pub total_exec_time_us: u64,
    /// Number of time slices used
    pub time_slices: u64,
    /// Average time slice duration in microseconds
    pub avg_slice_time_us: u64,
    /// Number of priority boosts
    pub priority_boosts: u32,
    /// Number of voluntary yields
    pub voluntary_yields: u32,
    /// Number of preemptions
    pub preemptions: u32,
    /// Cache hit rate (0-100)
    pub cache_hit_rate: f32,
}

impl TaskStats {
    /// Create new task statistics
    pub fn new(task_id: u64) -> Self {
        Self {
            task_id,
            total_exec_time_us: 0,
            time_slices: 0,
            avg_slice_time_us: 0,
            priority_boosts: 0,
            voluntary_yields: 0,
            preemptions: 0,
            cache_hit_rate: 0.0,
        }
    }
    
    /// Update statistics after execution
    pub fn update_execution(&mut self, exec_time_us: u64, was_preempted: bool) {
        self.total_exec_time_us += exec_time_us;
        self.time_slices += 1;
        
        // Update average slice time
        if self.time_slices > 0 {
            self.avg_slice_time_us = self.total_exec_time_us / self.time_slices;
        }
        
        if was_preempted {
            self.preemptions += 1;
        } else {
            self.voluntary_yields += 1;
        }
    }
    
    /// Update cache hit rate
    pub fn update_cache_hit_rate(&mut self, hit_rate: f32) {
        self.cache_hit_rate = hit_rate;
    }
    
    /// Get task efficiency score (0-100)
    pub fn efficiency_score(&self) -> f32 {
        if self.total_exec_time_us == 0 {
            return 100.0;
        }
        
        // Calculate efficiency based on cache hit rate and preemption ratio
        let cache_factor = self.cache_hit_rate;
        let preemption_ratio = if self.time_slices > 0 {
            (self.voluntary_yields as f32) / (self.time_slices as f32) * 100.0
        } else {
            100.0
        };
        
        (cache_factor + preemption_ratio) / 2.0
    }
}

/// Task control block for adaptive scheduling
#[derive(Debug, Clone)]
pub struct TaskControlBlock {
    /// Task ID
    pub task_id: u64,
    /// Task name
    pub name: String,
    /// Current priority
    pub priority: TaskPriority,
    /// Original priority (before boosts)
    pub base_priority: TaskPriority,
    /// Current state
    pub state: TaskState,
    /// Task statistics
    pub stats: TaskStats,
    /// Time slice duration in microseconds
    pub time_slice_us: u64,
    /// Last execution time in microseconds
    pub last_exec_time_us: u64,
    /// CPU affinity mask
    pub cpu_affinity: u64,
    /// Dynamic priority boost counter
    pub boost_counter: u32,
    /// Estimated CPU usage percentage (0-100)
    pub cpu_usage_percent: f32,
}

impl TaskControlBlock {
    /// Create a new task control block
    pub fn new(task_id: u64, name: String, priority: TaskPriority) -> Self {
        Self {
            task_id,
            name,
            priority,
            base_priority: priority,
            state: TaskState::Ready,
            stats: TaskStats::new(task_id),
            time_slice_us: Self::calculate_initial_time_slice(priority),
            last_exec_time_us: 0,
            cpu_affinity: 0, // No affinity restriction
            boost_counter: 0,
            cpu_usage_percent: 0.0,
        }
    }
    
    /// Calculate initial time slice based on priority
    fn calculate_initial_time_slice(priority: TaskPriority) -> u64 {
        match priority {
            TaskPriority::Realtime => 1000,    // 1ms for real-time tasks
            TaskPriority::High => 5000,      // 5ms for high priority tasks
            TaskPriority::Normal => 10000,    // 10ms for normal tasks
            TaskPriority::Low => 20000,      // 20ms for low priority tasks
            TaskPriority::Idle => 50000,      // 50ms for idle tasks
        }
    }
    
    /// Update task priority with boost
    pub fn boost_priority(&mut self, boost: bool) {
        if boost {
            // Boost priority temporarily
            self.priority = match self.base_priority {
                TaskPriority::Idle => TaskPriority::Low,
                TaskPriority::Low => TaskPriority::Normal,
                TaskPriority::Normal => TaskPriority::High,
                TaskPriority::High => TaskPriority::Realtime,
                TaskPriority::Realtime => TaskPriority::Realtime, // Already highest
            };
            self.boost_counter += 1;
        } else if self.boost_counter > 0 {
            // Decay boost over time
            self.boost_counter = self.boost_counter.saturating_sub(1);
            if self.boost_counter == 0 {
                self.priority = self.base_priority;
            }
        }
    }
    
    /// Update time slice based on task behavior
    pub fn update_time_slice(&mut self) {
        // Adaptive time slice based on task efficiency
        let efficiency = self.stats.efficiency_score();
        
        // Increase time slice for efficient tasks, decrease for inefficient ones
        let adjustment = if efficiency > 80.0 {
            1.2 // Increase by 20%
        } else if efficiency < 50.0 {
            0.8 // Decrease by 20%
        } else {
            1.0 // No change
        };
        
        self.time_slice_us = (self.time_slice_us as f32 * adjustment) as u64;
        
        // Clamp to reasonable bounds
        self.time_slice_us = self.time_slice_us.clamp(1000, 50000);
    }
    
    /// Update CPU usage estimate
    pub fn update_cpu_usage(&mut self, current_time: u64) {
        let time_since_last = current_time.saturating_sub(self.last_exec_time_us);
        
        if time_since_last > 0 {
            // Simple exponential moving average for CPU usage
            let instant_usage = (self.time_slice_us as f32 / time_since_last as f32) * 100.0;
            self.cpu_usage_percent = self.cpu_usage_percent * 0.9 + instant_usage * 0.1;
            self.last_exec_time_us = current_time;
        }
    }
}

/// Adaptive scheduler
#[allow(clippy::should_implement_trait)]
pub struct AdaptiveScheduler {
    /// Ready queue for each priority level
    ready_queues: [Vec<Arc<TaskControlBlock>>; 5],
    /// Currently running task
    current_task: Option<Arc<TaskControlBlock>>,
    /// Task map by ID
    tasks: BTreeMap<u64, Arc<TaskControlBlock>>,
    /// Next task ID
    next_task_id: AtomicU64,
    /// Scheduler statistics
    stats: SchedulerStats,
    /// Adaptive parameters
    adaptive_params: AdaptiveParameters,
}

/// Scheduler statistics
#[derive(Debug, Clone)]
#[allow(clippy::should_implement_trait)]
pub struct SchedulerStats {
    /// Total tasks scheduled
    pub total_scheduled: u64,
    /// Context switches
    pub context_switches: u64,
    /// Average task wait time in microseconds
    pub avg_wait_time_us: u64,
    /// CPU utilization percentage (0-100)
    pub cpu_utilization: f32,
}

impl SchedulerStats {
    /// Create new scheduler statistics
    pub fn new() -> Self {
        Self {
            total_scheduled: 0,
            context_switches: 0,
            avg_wait_time_us: 0,
            cpu_utilization: 0.0,
        }
    }
    
    /// Record a context switch
    pub fn record_context_switch(&mut self) {
        self.context_switches += 1;
    }
    
    /// Update CPU utilization
    pub fn update_cpu_utilization(&mut self, utilization: f32) {
        self.cpu_utilization = self.cpu_utilization * 0.9 + utilization * 0.1;
    }
}

/// Adaptive scheduling parameters
#[derive(Debug, Clone)]
pub struct AdaptiveParameters {
    /// Minimum time slice in microseconds
    pub min_time_slice_us: u64,
    /// Maximum time slice in microseconds
    pub max_time_slice_us: u64,
    /// Priority boost threshold
    pub boost_threshold: f32,
    /// CPU usage threshold for priority boost
    pub cpu_usage_threshold: f32,
    /// Cache hit rate threshold
    pub cache_hit_threshold: f32,
}

impl Default for AdaptiveParameters {
    fn default() -> Self {
        Self {
            min_time_slice_us: 1000,    // 1ms minimum
            max_time_slice_us: 50000,   // 50ms maximum
            boost_threshold: 0.7,        // Boost if efficiency < 70%
            cpu_usage_threshold: 80.0,   // Boost if CPU usage > 80%
            cache_hit_threshold: 60.0,    // Consider cache hit rate < 60% as poor
        }
    }
}

impl AdaptiveScheduler {
    /// Create a new adaptive scheduler
    pub fn new() -> Self {
        Self {
            ready_queues: [
                Vec::new(), // Realtime
                Vec::new(), // High
                Vec::new(), // Normal
                Vec::new(), // Low
                Vec::new(), // Idle
            ],
            current_task: None,
            tasks: BTreeMap::new(),
            next_task_id: AtomicU64::new(1),
            stats: SchedulerStats::new(),
            adaptive_params: AdaptiveParameters::default(),
        }
    }
}

impl Clone for AdaptiveScheduler {
    fn clone(&self) -> Self {
        Self {
            ready_queues: self.ready_queues.clone(),
            current_task: self.current_task.clone(),
            tasks: self.tasks.clone(),
            next_task_id: AtomicU64::new(self.next_task_id.load(Ordering::SeqCst)),
            stats: self.stats.clone(),
            adaptive_params: self.adaptive_params.clone(),
        }
    }
}

impl AdaptiveScheduler {
    /// Add a new task to the scheduler
    pub fn add_task(&mut self, name: String, priority: TaskPriority) -> Result<u64> {
        let task_id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
        let task = Arc::new(TaskControlBlock::new(task_id, name, priority));
        
        self.tasks.insert(task_id, task.clone());
        self.ready_queues[priority as usize].push(task);
        self.stats.total_scheduled += 1;
        
        Ok(task_id)
    }
    
    /// Remove a task from the scheduler
    pub fn remove_task(&mut self, task_id: u64) -> Result<()> {
        if let Some(task) = self.tasks.remove(&task_id) {
            // Remove from ready queue
            let priority = task.priority as usize;
            self.ready_queues[priority].retain(|t| t.task_id != task_id);

            // If it was the current task, clear it
            if let Some(current) = &self.current_task && current.task_id == task_id {
                self.current_task = None;
            }

            Ok(())
        } else {
            Err(nos_api::Error::NotFound(format!("Task {} not found", task_id)))
        }
    }
    
    /// Schedule the next task to run
    pub fn schedule_next(&mut self) -> Option<Arc<TaskControlBlock>> {
        // Find highest priority ready task
        for queue in &mut self.ready_queues {
            if let Some(task) = queue.first() {
                let task = task.clone();
                queue.remove(0);
                
                // Record context switch if changing tasks
                if self.current_task.is_some() {
                    self.stats.record_context_switch();
                }
                
                self.current_task = Some(task.clone());
                return Some(task);
            }
        }
        
        // No tasks ready
        self.current_task = None;
        None
    }
    
    /// Yield the current task
    pub fn yield_current(&mut self) -> Result<()> {
        if let Some(mut task) = self.current_task.take() {
            // Extract values before mutable borrowing
            let time_slice_us = task.time_slice_us;
            let priority = task.priority as usize;
            
            // Update task statistics
            Arc::make_mut(&mut task).stats.update_execution(time_slice_us, false);
            
            // Re-queue with same priority
            self.ready_queues[priority].push(task);
            
            Ok(())
        } else {
            Err(nos_api::Error::InvalidArgument(
                "No current task to yield".to_string()
            ))
        }
    }
    
    /// Preempt the current task
    pub fn preempt_current(&mut self) -> Result<()> {
        if let Some(mut task) = self.current_task.take() {
            // Extract values before mutable borrowing
            let time_slice_us = task.time_slice_us;
            let priority = task.priority as usize;
            
            // Update task statistics
            Arc::make_mut(&mut task).stats.update_execution(time_slice_us, true);
            
            // Re-queue with same priority
            self.ready_queues[priority].push(task);
            
            Ok(())
        } else {
            Err(nos_api::Error::InvalidArgument(
                "No current task to preempt".to_string()
            ))
        }
    }
    
    /// Update scheduler state
    pub fn update(&mut self, current_time: u64) {
        // Update current task
        if let Some(task) = &self.current_task {
            let task = Arc::as_ref(task) as *const TaskControlBlock as *mut TaskControlBlock;
            unsafe {
                (*task).update_cpu_usage(current_time);
                (*task).update_time_slice();
                
                // Check if task needs priority boost
                let efficiency = (*task).stats.efficiency_score();
                if efficiency < self.adaptive_params.boost_threshold {
                    (*task).boost_priority(true);
                } else {
                    (*task).boost_priority(false);
                }
            }
        }
        
        // Update scheduler statistics
        let total_tasks = self.tasks.len() as u64;
        if total_tasks > 0 {
            let ready_tasks = self.ready_queues.iter().map(|q| q.len()).sum::<usize>() as u64;
            let utilization = if ready_tasks > 0 {
                (ready_tasks as f32 - 1.0) / total_tasks as f32 * 100.0
            } else {
                0.0
            };
            self.stats.update_cpu_utilization(utilization);
        }
    }
    
    /// Get scheduler statistics
    pub fn get_stats(&self) -> &SchedulerStats {
        &self.stats
    }
    
    /// Get task by ID
    pub fn get_task(&self, task_id: u64) -> Option<&Arc<TaskControlBlock>> {
        self.tasks.get(&task_id)
    }
    
    /// Get current task
    pub fn get_current_task(&self) -> Option<&Arc<TaskControlBlock>> {
        self.current_task.as_ref()
    }
    
    /// Get ready queue sizes
    pub fn get_ready_queue_sizes(&self) -> [usize; 5] {
        [
            self.ready_queues[0].len(),
            self.ready_queues[1].len(),
            self.ready_queues[2].len(),
            self.ready_queues[3].len(),
            self.ready_queues[4].len(),
        ]
    }
}

/// System call handler for task management
#[allow(clippy::should_implement_trait)]
pub struct TaskSchedulerHandler {
    scheduler: Arc<Mutex<AdaptiveScheduler>>,
}

impl TaskSchedulerHandler {
    /// Create a new task scheduler handler
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(Mutex::new(AdaptiveScheduler::new())),
        }
    }
    
    pub fn new_with_scheduler(scheduler: Arc<Mutex<AdaptiveScheduler>>) -> Self {
        Self { scheduler }
    }
}

impl SyscallHandler for TaskSchedulerHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_SCHED_YIELD
    }
    
    fn name(&self) -> &str {
        "sched_yield"
    }
    
    fn execute(&self, _args: &[usize]) -> Result<isize> {
        self.scheduler.lock().yield_current()?;
        Ok(0)
    }
}

/// Register adaptive scheduler system call handlers
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Create adaptive scheduler
    let scheduler = Arc::new(Mutex::new(AdaptiveScheduler::new()));
    
    // Register task yield system call
    let yield_handler = TaskSchedulerHandler::new_with_scheduler(scheduler.clone());
    dispatcher.register_handler(crate::types::SYS_SCHED_YIELD, Box::new(yield_handler));
    
    // Print scheduler report
    let report = get_scheduler_report(&scheduler.lock());
    output_report(&report);
    
    Ok(())
}

/// Get scheduler report
pub fn get_scheduler_report(scheduler: &AdaptiveScheduler) -> String {
    let mut report = String::from("=== Adaptive Scheduler Report ===\n");
    
    let stats = scheduler.get_stats();
    report.push_str(&format!("Total tasks scheduled: {}\n", stats.total_scheduled));
    report.push_str(&format!("Context switches: {}\n", stats.context_switches));
    report.push_str(&format!("CPU utilization: {:.1}%\n", stats.cpu_utilization));
    
    let queue_sizes = scheduler.get_ready_queue_sizes();
    report.push_str(&format!("Ready queue sizes: [{}, {}, {}, {}, {}]\n",
        queue_sizes[0], queue_sizes[1], queue_sizes[2], queue_sizes[3], queue_sizes[4]));
    
    if let Some(current) = scheduler.get_current_task() {
        report.push_str(&format!("Current task: {} (ID: {}, Priority: {:?})\n",
            current.name, current.task_id, current.priority));
    }
    
    report
}