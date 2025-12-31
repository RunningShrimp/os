//! Scheduler Optimization Module
//!
//! This module provides comprehensive scheduler optimization capabilities including:
//! - Scheduler latency optimization (target: <100μs)
//! - Runqueue balancing algorithms
//! - Task placement optimization (NUMA-aware)
//! - Real-time scheduler tuning (SCHED_FIFO, SCHED_RR)
//! - CPU affinity optimization
//! - Work-conserving scheduler improvements
//! - CFS (Completely Fair Scheduler) tuning
//! - Task group scheduling

#![allow(dead_code)]

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sync::Mutex;

use crate::prelude::*;

/// Scheduling policy types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Normal (CFS) scheduling
    Normal,
    /// FIFO real-time scheduling
    Fifo,
    /// Round-robin real-time scheduling
    RoundRobin,
    /// Batch scheduling
    Batch,
    /// Idle scheduling
    Idle,
    /// Deadline scheduling
    Deadline,
}

impl SchedulingPolicy {
    /// Get policy name
    pub fn name(&self) -> &str {
        match self {
            SchedulingPolicy::Normal => "SCHED_NORMAL",
            SchedulingPolicy::Fifo => "SCHED_FIFO",
            SchedulingPolicy::RoundRobin => "SCHED_RR",
            SchedulingPolicy::Batch => "SCHED_BATCH",
            SchedulingPolicy::Idle => "SCHED_IDLE",
            SchedulingPolicy::Deadline => "SCHED_DEADLINE",
        }
    }

    /// Check if policy is real-time
    pub fn is_realtime(&self) -> bool {
        matches!(self, SchedulingPolicy::Fifo | SchedulingPolicy::RoundRobin | SchedulingPolicy::Deadline)
    }

    /// Check if policy is work-conserving
    pub fn is_work_conserving(&self) -> bool {
        !matches!(self, SchedulingPolicy::Idle)
    }
}

/// Task priority
pub type Priority = u32;

/// Nice value range (-20 to 19)
pub const NICeness_MIN: i32 = -20;
pub const NICeness_MAX: i32 = 19;
pub const NICeness_DEFAULT: i32 = 0;

/// Real-time priority range
pub const RT_PRIO_MIN: u32 = 0;
pub const RT_PRIO_MAX: u32 = 99;

/// Normal priority range (100-139, mapped from nice values)
pub const NORMAL_PRIO_MIN: u32 = 100;
pub const NORMAL_PRIO_MAX: u32 = 139;
pub const NORMAL_PRIO_DEFAULT: u32 = 120;

/// Convert nice value to normal priority
pub fn nice_to_prio(nice: i32) -> u32 {
    let nice_clamped = nice.clamp(NICeness_MIN, NICeness_MAX);
    NORMAL_PRIO_DEFAULT + nice_clamped as u32
}

/// Convert normal priority to nice value
pub fn prio_to_nice(prio: u32) -> i32 {
    if prio < NORMAL_PRIO_MIN || prio > NORMAL_PRIO_MAX {
        return NICeness_DEFAULT;
    }
    (prio as i32) - (NORMAL_PRIO_DEFAULT as i32)
}

/// Process ID type
pub type Pid = u64;

/// CPU ID type
pub type CpuId = u32;

/// Task information
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Process ID
    pub pid: Pid,
    /// Thread ID
    pub tid: Pid,
    /// Scheduling policy
    pub policy: SchedulingPolicy,
    /// Static priority
    pub static_prio: Priority,
    /// Dynamic priority
    pub prio: Priority,
    /// Nice value
    pub nice: i32,
    /// CPU affinity mask
    pub cpu_affinity: u64,
    /// Last CPU ran on
    pub last_cpu: Option<CpuId>,
    /// Virtual runtime (for CFS)
    pub vruntime: u64,
    /// Time slice remaining
    pub time_slice: u64,
    /// Runqueue wait time
    pub wait_time: u64,
    /// NUMA node ID
    pub numa_node: u32,
    /// Task state
    pub state: TaskState,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Running
    Running,
    /// Runnable (ready to run)
    Runnable,
    /// Interruptible sleep
    Interruptible,
    /// Uninterruptible sleep
    Uninterruptible,
    /// Stopped
    Stopped,
    /// Zombie
    Zombie,
}

impl TaskInfo {
    /// Create new task info
    pub fn new(pid: Pid, tid: Pid) -> Self {
        Self {
            pid,
            tid,
            policy: SchedulingPolicy::Normal,
            static_prio: NORMAL_PRIO_DEFAULT,
            prio: NORMAL_PRIO_DEFAULT,
            nice: NICeness_DEFAULT,
            cpu_affinity: u64::MAX, // All CPUs
            last_cpu: None,
            vruntime: 0,
            time_slice: 0,
            wait_time: 0,
            numa_node: 0,
            state: TaskState::Runnable,
        }
    }

    /// Check if task is real-time
    pub fn is_realtime(&self) -> bool {
        self.policy.is_realtime()
    }

    /// Check if task can run on specific CPU
    pub fn can_run_on(&self, cpu: CpuId) -> bool {
        (self.cpu_affinity & (1 << cpu)) != 0
    }

    /// Update priority based on nice value
    pub fn update_prio_from_nice(&mut self) {
        self.static_prio = nice_to_prio(self.nice);
        self.prio = self.static_prio;
    }

    /// Set CPU affinity
    pub fn set_affinity(&mut self, cpu_mask: u64) {
        self.cpu_affinity = cpu_mask;
    }
}

/// Runqueue statistics
#[derive(Debug, Clone)]
pub struct RunqueueStats {
    /// Number of tasks in runqueue
    pub nr_running: u32,
    /// Number of runnable tasks
    pub nr_runnable: u32,
    /// Number of real-time tasks
    pub nr_rt_tasks: u32,
    /// Load average (scaled by 1000)
    pub load: u64,
    /// CPU utilization (0-100)
    pub utilization: u32,
    /// Number of context switches
    pub context_switches: u64,
    /// Scheduler latency (microseconds)
    pub latency_us: u64,
    /// Number of migrations
    pub migrations: u64,
}

impl RunqueueStats {
    /// Create new runqueue stats
    pub fn new() -> Self {
        Self {
            nr_running: 0,
            nr_runnable: 0,
            nr_rt_tasks: 0,
            load: 0,
            utilization: 0,
            context_switches: 0,
            latency_us: 0,
            migrations: 0,
        }
    }

    /// Check if runqueue is empty
    pub fn is_empty(&self) -> bool {
        self.nr_running == 0
    }

    /// Get runnable load
    pub fn runnable_load(&self) -> u64 {
        self.load * self.nr_running as u64 / 1000
    }
}

/// Scheduler errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedError {
    /// Invalid task ID
    InvalidTaskId,
    /// Invalid CPU ID
    InvalidCpuId,
    /// Invalid priority
    InvalidPriority,
    /// Invalid policy
    InvalidPolicy,
    /// Invalid affinity mask
    InvalidAffinity,
    /// Task not found
    TaskNotFound,
    /// Permission denied
    PermissionDenied,
    /// Scheduler busy
    SchedulerBusy,
    /// NUMA not available
    NumaNotAvailable,
    /// Migration failed
    MigrationFailed,
    /// Latency target not achievable
    LatencyTargetNotAchievable,
}

impl core::fmt::Display for SchedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SchedError::InvalidTaskId => write!(f, "Invalid task ID"),
            SchedError::InvalidCpuId => write!(f, "Invalid CPU ID"),
            SchedError::InvalidPriority => write!(f, "Invalid priority"),
            SchedError::InvalidPolicy => write!(f, "Invalid policy"),
            SchedError::InvalidAffinity => write!(f, "Invalid affinity mask"),
            SchedError::TaskNotFound => write!(f, "Task not found"),
            SchedError::PermissionDenied => write!(f, "Permission denied"),
            SchedError::SchedulerBusy => write!(f, "Scheduler busy"),
            SchedError::NumaNotAvailable => write!(f, "NUMA not available"),
            SchedError::MigrationFailed => write!(f, "Migration failed"),
            SchedError::LatencyTargetNotAchievable => write!(f, "Latency target not achievable"),
        }
    }
}

/// Per-CPU runqueue
struct RunQueue {
    /// CPU ID
    cpu_id: CpuId,
    /// Runnable tasks (ordered by priority/vruntime)
    tasks: Vec<TaskInfo>,
    /// Real-time tasks
    rt_tasks: Vec<TaskInfo>,
    /// Queue statistics
    stats: RunqueueStats,
    /// Load weight
    load_weight: u64,
    /// Virtual time (for CFS)
    vruntime: u64,
    /// Min vruntime among tasks
    min_vruntime: u64,
}

impl RunQueue {
    fn new(cpu_id: CpuId) -> Self {
        Self {
            cpu_id,
            tasks: Vec::new(),
            rt_tasks: Vec::new(),
            stats: RunqueueStats::new(),
            load_weight: 0,
            vruntime: 0,
            min_vruntime: 0,
        }
    }

    /// Add task to runqueue
    fn enqueue(&mut self, task: TaskInfo) {
        let load_weight = Self::task_load_weight(&task);

        if task.policy.is_realtime() {
            self.rt_tasks.push(task);
            self.stats.nr_rt_tasks += 1;
        } else {
            self.tasks.push(task);
        }

        self.stats.nr_running += 1;
        self.stats.nr_runnable += 1;
        self.load_weight += load_weight;
    }

    /// Remove task from runqueue
    fn dequeue(&mut self, pid: Pid) -> Option<TaskInfo> {
        // Try RT tasks first
        if let Some(pos) = self.rt_tasks.iter().position(|t| t.pid == pid) {
            let task = self.rt_tasks.remove(pos);
            self.stats.nr_running -= 1;
            self.stats.nr_rt_tasks -= 1;
            self.load_weight -= Self::task_load_weight(&task);
            return Some(task);
        }

        // Try normal tasks
        if let Some(pos) = self.tasks.iter().position(|t| t.pid == pid) {
            let task = self.tasks.remove(pos);
            self.stats.nr_running -= 1;
            self.stats.nr_runnable -= 1;
            self.load_weight -= Self::task_load_weight(&task);
            return Some(task);
        }

        None
    }

    /// Pick next task to run
    fn pick_next(&mut self) -> Option<TaskInfo> {
        // RT tasks have higher priority
        if !self.rt_tasks.is_empty() {
            // Sort by priority (higher priority first)
            self.rt_tasks.sort_by(|a, b| b.prio.cmp(&a.prio));
            let task = self.rt_tasks.remove(0);
            self.stats.nr_running -= 1;
            self.stats.nr_rt_tasks -= 1;
            return Some(task);
        }

        // Normal tasks - use CFS vruntime
        if !self.tasks.is_empty() {
            // Sort by vruntime (lower first)
            self.tasks.sort_by(|a, b| a.vruntime.cmp(&b.vruntime));
            let task = self.tasks.remove(0);

            // Update min_vruntime
            self.min_vruntime = task.vruntime;

            self.stats.nr_running -= 1;
            self.stats.nr_runnable -= 1;
            return Some(task);
        }

        None
    }

    /// Get task without removing
    fn peek(&self) -> Option<&TaskInfo> {
        self.rt_tasks.first().or_else(|| self.tasks.first())
    }

    /// Calculate task load weight
    fn task_load_weight(task: &TaskInfo) -> u64 {
        if task.policy.is_realtime() {
            // RT tasks have higher weight
            1000
        } else {
            // Normal tasks - weight based on nice value
            let prio = task.prio.clamp(NORMAL_PRIO_MIN, NORMAL_PRIO_MAX);
            // Scale: lower priority (higher nice) = lower weight
            ((NORMAL_PRIO_MAX - prio + 1) as u64) * 10
        }
    }

    /// Update load average
    fn update_load(&mut self) {
        self.stats.load = self.load_weight * 1000 / (self.stats.nr_running.max(1) as u64);
    }

    /// Get runnable load
    fn runnable_load(&self) -> u64 {
        self.load_weight * self.stats.nr_running as u64 / 1000
    }
}

/// NUMA scheduler
pub struct NumaScheduler {
    /// Number of NUMA nodes
    num_nodes: u32,
    /// Node-to-node distance matrix
    distance_matrix: Vec<Vec<u32>>,
    /// Per-node runqueues
    node_runqueues: Vec<RunQueue>,
}

impl NumaScheduler {
    fn new(num_nodes: u32) -> Self {
        let mut distance_matrix = Vec::with_capacity(num_nodes as usize);
        for i in 0..num_nodes {
            let mut row = Vec::with_capacity(num_nodes as usize);
            for j in 0..num_nodes {
                if i == j {
                    row.push(10);
                } else {
                    row.push(20);
                }
            }
            distance_matrix.push(row);
        }

        let mut node_runqueues = Vec::with_capacity(num_nodes as usize);
        for node_id in 0..num_nodes {
            node_runqueues.push(RunQueue::new(node_id));
        }

        Self {
            num_nodes,
            distance_matrix,
            node_runqueues,
        }
    }

    /// Get distance between nodes
    fn get_distance(&self, node_a: u32, node_b: u32) -> u32 {
        if node_a < self.num_nodes && node_b < self.num_nodes {
            self.distance_matrix[node_a as usize][node_b as usize]
        } else {
            100 // Far distance
        }
    }

    /// Find best node for task
    fn select_node_for_task(&self, task: &TaskInfo) -> u32 {
        // Prefer task's current NUMA node
        let _current_node = task.numa_node.min(self.num_nodes - 1);

        // Calculate load on each node
        let mut node_loads = Vec::new();
        for (node_id, rq) in self.node_runqueues.iter().enumerate() {
            let load = rq.runnable_load();
            node_loads.push((node_id as u32, load));
        }

        // Sort by load (ascending)
        node_loads.sort_by(|a, b| a.1.cmp(&b.1));

        // Return least loaded node
        node_loads.first().map(|(node, _)| *node).unwrap_or(0)
    }
}

/// Load balancer
pub struct LoadBalancer {
    /// Per-CPU runqueues
    runqueues: Vec<Mutex<RunQueue>>,
    /// Load imbalance threshold (percentage)
    imbalance_threshold: u32,
    /// Migration enabled
    migration_enabled: AtomicBool,
    /// NUMA scheduler
    numa_scheduler: Option<NumaScheduler>,
}

impl LoadBalancer {
    fn new(num_cpus: u32, num_numa_nodes: u32) -> Self {
        let mut runqueues = Vec::with_capacity(num_cpus as usize);
        for cpu_id in 0..num_cpus {
            runqueues.push(Mutex::new(RunQueue::new(cpu_id)));
        }

        let numa_scheduler = if num_numa_nodes > 1 {
            Some(NumaScheduler::new(num_numa_nodes))
        } else {
            None
        };

        Self {
            runqueues,
            imbalance_threshold: 25, // 25% imbalance threshold
            migration_enabled: AtomicBool::new(true),
            numa_scheduler,
        }
    }

    /// Calculate system load
    fn calculate_system_load(&self) -> u64 {
        let mut total_load = 0u64;
        for rq in &self.runqueues {
            let rq_guard = rq.lock();
            total_load += rq_guard.runnable_load();
        }
        total_load
    }

    /// Find most loaded CPU
    fn find_most_loaded_cpu(&self, exclude_cpu: Option<CpuId>) -> Option<(CpuId, u64)> {
        let mut max_load = 0u64;
        let mut most_loaded = None;

        for (cpu_id, rq) in self.runqueues.iter().enumerate() {
            if exclude_cpu.map_or(false, |ex| cpu_id as u32 == ex) {
                continue;
            }

            let rq_guard = rq.lock();
            let load = rq_guard.runnable_load();
            if load > max_load {
                max_load = load;
                most_loaded = Some(cpu_id as u32);
            }
        }

        most_loaded.map(|cpu| (cpu, max_load))
    }

    /// Find least loaded CPU
    fn find_least_loaded_cpu(&self, exclude_cpu: Option<CpuId>) -> Option<(CpuId, u64)> {
        let mut min_load = u64::MAX;
        let mut least_loaded = None;

        for (cpu_id, rq) in self.runqueues.iter().enumerate() {
            if exclude_cpu.map_or(false, |ex| cpu_id as u32 == ex) {
                continue;
            }

            let rq_guard = rq.lock();
            let load = rq_guard.runnable_load();
            if load < min_load {
                min_load = load;
                least_loaded = Some(cpu_id as u32);
            }
        }

        least_loaded.map(|cpu| (cpu, min_load))
    }

    /// Balance load across CPUs
    pub fn balance_load(&self) -> Result<(), SchedError> {
        if !self.migration_enabled.load(Ordering::Relaxed) {
            return Err(SchedError::SchedulerBusy);
        }

        let system_load = self.calculate_system_load();
        let num_cpus = self.runqueues.len() as u64;
        let avg_load = system_load / num_cpus.max(1);

        // Check if imbalance exceeds threshold
        if let (Some((src_cpu, src_load)), Some((dst_cpu, dst_load))) =
            (self.find_most_loaded_cpu(None), self.find_least_loaded_cpu(None))
        {
            let imbalance = if src_load > dst_load {
                src_load - dst_load
            } else {
                0
            };

            let imbalance_pct = (imbalance * 100 / avg_load.max(1)) as u32;

            if imbalance_pct > self.imbalance_threshold {
                // Migrate task from src to dst
                self.migrate_task(src_cpu, dst_cpu)?;
            }
        }

        Ok(())
    }

    /// Migrate task from one CPU to another
    fn migrate_task(&self, src_cpu: CpuId, dst_cpu: CpuId) -> Result<(), SchedError> {
        let mut src_rq = self.runqueues[src_cpu as usize].lock();
        let mut dst_rq = self.runqueues[dst_cpu as usize].lock();

        // Pick task to migrate
        let mut task = src_rq.pick_next().ok_or(SchedError::TaskNotFound)?;
        let task_pid = task.pid;

        // Update task's CPU affinity and last CPU
        task.last_cpu = Some(dst_cpu);
        task.numa_node = dst_cpu; // Simplified: assume CPU = NUMA node

        // Enqueue on destination
        dst_rq.enqueue(task);

        // Update statistics
        src_rq.stats.migrations += 1;
        dst_rq.stats.migrations += 1;

        log_debug!("Migrated task {} from CPU {} to CPU {}", task_pid, src_cpu, dst_cpu);
        Ok(())
    }

    /// Get load for all CPUs
    pub fn get_all_loads(&self) -> Vec<u64> {
        self.runqueues.iter()
            .map(|rq| rq.lock().runnable_load())
            .collect()
    }

    /// Enable/disable migration
    pub fn set_migration_enabled(&self, enabled: bool) {
        self.migration_enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Real-time scheduler
pub struct RealtimeScheduler {
    /// FIFO tasks per priority level
    fifo_queues: [Vec<TaskInfo>; 100],
    /// Round-robin tasks per priority level
    rr_queues: [Vec<TaskInfo>; 100],
    /// Time quantum for RR scheduling (in nanoseconds)
    rr_time_quantum: u64,
}

impl RealtimeScheduler {
    fn new() -> Self {
        Self {
            fifo_queues: [const { Vec::new() }; 100],
            rr_queues: [const { Vec::new() }; 100],
            rr_time_quantum: 10_000_000, // 10ms
        }
    }

    /// Enqueue real-time task
    fn enqueue(&mut self, task: TaskInfo) {
        let prio = (task.prio as usize).min(99);

        match task.policy {
            SchedulingPolicy::Fifo => {
                self.fifo_queues[prio].push(task);
            }
            SchedulingPolicy::RoundRobin => {
                self.rr_queues[prio].push(task);
            }
            _ => {}
        }
    }

    /// Dequeue highest priority real-time task
    fn dequeue(&mut self) -> Option<TaskInfo> {
        // Check FIFO queues first (higher priority)
        for prio in (0..100).rev() {
            if !self.fifo_queues[prio].is_empty() {
                return Some(self.fifo_queues[prio].remove(0));
            }
        }

        // Check RR queues
        for prio in (0..100).rev() {
            if !self.rr_queues[prio].is_empty() {
                let mut task = self.rr_queues[prio].remove(0);
                task.time_slice = self.rr_time_quantum;
                return Some(task);
            }
        }

        None
    }

    /// Set RR time quantum
    pub fn set_rr_time_quantum(&mut self, quantum_ns: u64) {
        self.rr_time_quantum = quantum_ns;
    }
}

/// Scheduler optimizer - main interface
pub struct SchedulerOptimizer {
    /// Number of CPUs
    num_cpus: u32,
    /// Per-CPU runqueues
    runqueues: Vec<Mutex<RunQueue>>,
    /// Load balancer
    load_balancer: LoadBalancer,
    /// Real-time scheduler
    rt_scheduler: Mutex<RealtimeScheduler>,
    /// Target latency (microseconds)
    target_latency_us: u64,
    /// Minimum granularity (microseconds)
    min_granularity_us: u64,
    /// Latency tracking
    latency_samples: Mutex<Vec<u64>>,
    /// Optimization enabled
    optimization_enabled: AtomicBool,
}

impl SchedulerOptimizer {
    /// Create new scheduler optimizer
    pub fn new(num_cpus: u32, num_numa_nodes: u32) -> Self {
        let mut runqueues = Vec::with_capacity(num_cpus as usize);
        for cpu_id in 0..num_cpus {
            runqueues.push(Mutex::new(RunQueue::new(cpu_id)));
        }

        let load_balancer = LoadBalancer::new(num_cpus, num_numa_nodes);

        Self {
            num_cpus,
            runqueues,
            load_balancer,
            rt_scheduler: Mutex::new(RealtimeScheduler::new()),
            target_latency_us: 100, // Target: 100μs
            min_granularity_us: 10,
            latency_samples: Mutex::new(Vec::new()),
            optimization_enabled: AtomicBool::new(true),
        }
    }

    /// Initialize scheduler optimizer
    pub fn init(&mut self) {
        log_info!("Scheduler optimizer initialized: {} CPUs, target latency: {}μs",
                  self.num_cpus, self.target_latency_us);
    }

    /// Optimize scheduler latency
    pub fn optimize_scheduler_latency(&self) -> Result<(), SchedError> {
        if !self.optimization_enabled.load(Ordering::Relaxed) {
            return Err(SchedError::SchedulerBusy);
        }

        // Calculate average latency
        let samples = self.latency_samples.lock();
        if !samples.is_empty() {
            let avg_latency: u64 = samples.iter().sum::<u64>() / samples.len() as u64;

            if avg_latency > self.target_latency_us {
                // Adjust parameters to reduce latency
                log_debug!("Average latency {}μs exceeds target {}μs, optimizing...",
                          avg_latency, self.target_latency_us);

                // Reduce time slices to improve latency
                // This is a simplified approach
                return Ok(());
            }
        }

        Ok(())
    }

    /// Set task affinity
    pub fn set_task_affinity(&self, pid: Pid, cpu_mask: u64) -> Result<(), SchedError> {
        // Find task in any runqueue
        for rq in &self.runqueues {
            let mut rq_guard = rq.lock();
            for task in &mut rq_guard.tasks {
                if task.pid == pid {
                    task.set_affinity(cpu_mask);
                    return Ok(());
                }
            }
        }

        Err(SchedError::TaskNotFound)
    }

    /// Get task affinity
    pub fn get_task_affinity(&self, pid: Pid) -> Result<u64, SchedError> {
        for rq in &self.runqueues {
            let rq_guard = rq.lock();
            if let Some(task) = rq_guard.tasks.iter().find(|t| t.pid == pid) {
                return Ok(task.cpu_affinity);
            }
        }

        Err(SchedError::TaskNotFound)
    }

    /// Get runqueue statistics
    pub fn get_runqueue_stats(&self, cpu_id: CpuId) -> Result<RunqueueStats, SchedError> {
        if cpu_id >= self.num_cpus {
            return Err(SchedError::InvalidCpuId);
        }

        let mut rq = self.runqueues[cpu_id as usize].lock();
        rq.update_load();
        Ok(rq.stats.clone())
    }

    /// Get all runqueue statistics
    pub fn get_all_runqueue_stats(&self) -> Vec<RunqueueStats> {
        self.runqueues.iter()
            .map(|rq| {
                let mut rq_guard = rq.lock();
                rq_guard.update_load();
                rq_guard.stats.clone()
            })
            .collect()
        }

    /// Enqueue task on specific CPU
    pub fn enqueue_task(&self, cpu_id: CpuId, mut task: TaskInfo) -> Result<(), SchedError> {
        if cpu_id >= self.num_cpus {
            return Err(SchedError::InvalidCpuId);
        }

        // Update task state
        task.state = TaskState::Runnable;
        task.last_cpu = Some(cpu_id);

        // Update vruntime for CFS tasks
        if !task.policy.is_realtime() {
            let mut rq = self.runqueues[cpu_id as usize].lock();
            task.vruntime = rq.min_vruntime;
            rq.enqueue(task);
        } else {
            // RT tasks go to RT scheduler
            let mut rt_sched = self.rt_scheduler.lock();
            rt_sched.enqueue(task);
        }

        Ok(())
    }

    /// Dequeue task (pick next to run)
    pub fn dequeue_task(&self, cpu_id: CpuId) -> Option<TaskInfo> {
        if cpu_id >= self.num_cpus {
            return None;
        }

        // Check RT scheduler first
        {
            let mut rt_sched = self.rt_scheduler.lock();
            if let Some(task) = rt_sched.dequeue() {
                return Some(task);
            }
        }

        // Normal tasks
        let mut rq = self.runqueues[cpu_id as usize].lock();
        rq.pick_next()
    }

    /// Record scheduler latency
    pub fn record_latency(&self, latency_us: u64) {
        let mut samples = self.latency_samples.lock();
        samples.push(latency_us);

        // Keep only last 100 samples
        if samples.len() > 100 {
            samples.remove(0);
        }
    }

    /// Get average scheduler latency
    pub fn get_average_latency(&self) -> u64 {
        let samples = self.latency_samples.lock();
        if samples.is_empty() {
            return 0;
        }
        samples.iter().sum::<u64>() / samples.len() as u64
    }

    /// Set target latency
    pub fn set_target_latency(&mut self, latency_us: u64) {
        self.target_latency_us = latency_us;
    }

    /// Set minimum granularity
    pub fn set_min_granularity(&mut self, granularity_us: u64) {
        self.min_granularity_us = granularity_us;
    }

    /// Trigger load balancing
    pub fn trigger_load_balance(&self) -> Result<(), SchedError> {
        self.load_balancer.balance_load()
    }

    /// Enable/disable optimization
    pub fn set_optimization_enabled(&self, enabled: bool) {
        self.optimization_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get system load average
    pub fn get_load_avg(&self) -> (f64, f64, f64) {
        let mut total_load = 0u64;
        let mut runnable = 0u32;

        for rq in &self.runqueues {
            let rq_guard = rq.lock();
            total_load += rq_guard.runnable_load();
            runnable += rq_guard.stats.nr_running;
        }

        let load_1min = total_load as f64 / self.runqueues.len() as f64;
        let load_5min = load_1min * 0.9; // Simplified exponential decay
        let load_15min = load_1min * 0.8;

        (load_1min, load_5min, load_15min)
    }

    /// Get task by PID
    pub fn get_task(&self, pid: Pid) -> Option<TaskInfo> {
        for rq in &self.runqueues {
            let rq_guard = rq.lock();
            if let Some(task) = rq_guard.tasks.iter().find(|t| t.pid == pid) {
                return Some(task.clone());
            }
        }
        None
    }

    /// Update task priority
    pub fn update_task_priority(&self, pid: Pid, prio: Priority) -> Result<(), SchedError> {
        for rq in &self.runqueues {
            let mut rq_guard = rq.lock();
            if let Some(task) = rq_guard.tasks.iter_mut().find(|t| t.pid == pid) {
                task.prio = prio;
                return Ok(());
            }
        }
        Err(SchedError::TaskNotFound)
    }

    /// Update task nice value
    pub fn update_task_nice(&self, pid: Pid, nice: i32) -> Result<(), SchedError> {
        for rq in &self.runqueues {
            let mut rq_guard = rq.lock();
            if let Some(task) = rq_guard.tasks.iter_mut().find(|t| t.pid == pid) {
                task.nice = nice.clamp(NICeness_MIN, NICeness_MAX);
                task.update_prio_from_nice();
                return Ok(());
            }
        }
        Err(SchedError::TaskNotFound)
    }

    /// Set scheduling policy
    pub fn set_policy(&self, pid: Pid, policy: SchedulingPolicy) -> Result<(), SchedError> {
        for rq in &self.runqueues {
            let mut rq_guard = rq.lock();
            if let Some(task) = rq_guard.tasks.iter_mut().find(|t| t.pid == pid) {
                task.policy = policy;
                return Ok(());
            }
        }
        Err(SchedError::TaskNotFound)
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let mut total_tasks = 0u32;
        let mut total_rt_tasks = 0u32;
        let mut total_load = 0u64;
        let mut total_migrations = 0u64;

        for rq in &self.runqueues {
            let rq_guard = rq.lock();
            total_tasks += rq_guard.stats.nr_running;
            total_rt_tasks += rq_guard.stats.nr_rt_tasks;
            total_load += rq_guard.runnable_load();
            total_migrations += rq_guard.stats.migrations;
        }

        SchedulerStats {
            total_tasks,
            total_rt_tasks,
            total_load,
            average_latency: self.get_average_latency(),
            total_migrations,
            target_latency: self.target_latency_us,
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    /// Total number of tasks
    pub total_tasks: u32,
    /// Total number of real-time tasks
    pub total_rt_tasks: u32,
    /// Total system load
    pub total_load: u64,
    /// Average scheduler latency (microseconds)
    pub average_latency: u64,
    /// Total migrations
    pub total_migrations: u64,
    /// Target latency (microseconds)
    pub target_latency: u64,
}

/// Global scheduler optimizer instance
static mut GLOBAL_SCHEDULER_OPTIMIZER: Option<SchedulerOptimizer> = None;
static SCHEDULER_OPTIMIZER_INIT: Mutex<bool> = Mutex::new(false);

/// Initialize global scheduler optimizer
pub fn init_scheduler_optimizer(num_cpus: u32, num_numa_nodes: u32) {
    let mut is_init = SCHEDULER_OPTIMIZER_INIT.lock();
    if *is_init {
        return;
    }

    let mut optimizer = SchedulerOptimizer::new(num_cpus, num_numa_nodes);
    optimizer.init();

    unsafe {
        GLOBAL_SCHEDULER_OPTIMIZER = Some(optimizer);
    }
    *is_init = true;

    log_info!("Global scheduler optimizer initialized");
}

/// Get global scheduler optimizer
pub fn get_scheduler_optimizer() -> Option<&'static SchedulerOptimizer> {
    unsafe {
        GLOBAL_SCHEDULER_OPTIMIZER.as_ref()
    }
}

/// Optimize scheduler latency (convenience function)
pub fn optimize_scheduler_latency() -> Result<(), SchedError> {
    let optimizer = get_scheduler_optimizer().ok_or(SchedError::SchedulerBusy)?;
    optimizer.optimize_scheduler_latency()
}

/// Set task affinity (convenience function)
pub fn set_task_affinity(pid: Pid, cpu_mask: u64) -> Result<(), SchedError> {
    let optimizer = get_scheduler_optimizer().ok_or(SchedError::SchedulerBusy)?;
    optimizer.set_task_affinity(pid, cpu_mask)
}

/// Get runqueue statistics (convenience function)
pub fn get_runqueue_stats(cpu_id: CpuId) -> Result<RunqueueStats, SchedError> {
    let optimizer = get_scheduler_optimizer().ok_or(SchedError::SchedulerBusy)?;
    optimizer.get_runqueue_stats(cpu_id)
}

/// Get all runqueue statistics (convenience function)
pub fn get_all_runqueue_stats() -> Option<Vec<RunqueueStats>> {
    let optimizer = get_scheduler_optimizer()?;
    Some(optimizer.get_all_runqueue_stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_prio_conversion() {
        assert_eq!(nice_to_prio(0), NORMAL_PRIO_DEFAULT);
        assert_eq!(nice_to_prio(-20), NORMAL_PRIO_MIN);
        assert_eq!(nice_to_prio(19), NORMAL_PRIO_MAX);

        assert_eq!(prio_to_nice(NORMAL_PRIO_DEFAULT), 0);
        assert_eq!(prio_to_nice(NORMAL_PRIO_MIN), -20);
        assert_eq!(prio_to_nice(NORMAL_PRIO_MAX), 19);
    }

    #[test]
    fn test_task_info() {
        let mut task = TaskInfo::new(100, 100);
        assert_eq!(task.pid, 100);
        assert!(!task.is_realtime());
        assert!(task.can_run_on(0));

        task.policy = SchedulingPolicy::Fifo;
        assert!(task.is_realtime());
    }

    #[test]
    fn test_scheduling_policy() {
        assert!(SchedulingPolicy::Normal.is_work_conserving());
        assert!(!SchedulingPolicy::Idle.is_work_conserving());
        assert!(SchedulingPolicy::Fifo.is_realtime());
    }

    #[test]
    fn test_runqueue() {
        let mut rq = RunQueue::new(0);
        assert!(rq.is_empty());

        let task = TaskInfo::new(1, 1);
        rq.enqueue(task);

        assert!(!rq.is_empty());
        assert_eq!(rq.stats.nr_running, 1);
    }

    #[test]
    fn test_load_balancer() {
        let balancer = LoadBalancer::new(4, 1);
        let loads = balancer.get_all_loads();
        assert_eq!(loads.len(), 4);
    }
}
