//! Advanced Scheduling Optimization Module
//!
//! This module provides intelligent scheduling strategies including:
//! - NUMA-aware scheduling for memory locality
//! - CPU topology-aware task placement
//! - Load balancing across cores and packages
//! - Power-aware scheduling for energy efficiency
//! - Real-time scheduling support
//! - Workload-aware scheduling decisions
//!
//! # Architecture
//!
//! The scheduler uses a multi-layer approach:
//! 1. **Global scheduler**: Load balancing across system
//! 2. **NUMA scheduler**: Optimize memory locality
//! 3. **Package scheduler**: Load balance within CPU package
//! 4. **Core scheduler**: Assign threads to cores
//! 5. **Hyper-thread scheduler**: Optimize SMT utilization
//!
//! # Performance
//!
//! Goals:
//! - Minimize cross-NUMA memory traffic
//! - Balance load for maximum throughput
//! - Minimize power consumption for low load
//! - Meet real-time deadlines when applicable

#![allow(missing_docs)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// NUMA node ID
pub type NumaNodeId = u8;

/// CPU ID
pub type CpuId = u32;

/// Thread/task ID
pub type TaskId = u64;

/// Scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Normal timesharing
    Normal,
    /// Batch (low priority, background)
    Batch,
    /// Real-time FIFO
    RealtimeFifo,
    /// Real-time round-robin
    RealtimeRR,
    /// Idle (only when no other work)
    Idle,
    /// Power-saving (consolidate on few cores)
    PowerSave,
}

/// CPU topology information
#[derive(Debug, Clone)]
pub struct CpuTopology {
    /// Number of NUMA nodes
    pub numa_nodes: u8,
    /// Number of CPU packages (sockets)
    pub packages: u8,
    /// Number of cores per package
    pub cores_per_package: u8,
    /// Threads per core (hyper-threads)
    /// threads per core
    pub threads_per_core: u8,
    /// Total CPUs
    pub total_cpus: u32,
}

impl CpuTopology {
    /// Create new CPU topology
    pub fn new(numa_nodes: u8, packages: u8, cores_per_package: u8, threads_per_core: u8) -> Self {
        let total_cpus = (packages as u32) * (cores_per_package as u32) * (threads_per_core as u32);

        Self {
            numa_nodes,
            packages,
            cores_per_package,
            threads_per_core,
            total_cpus,
        }
    }

    /// Get typical server topology
    pub fn server() -> Self {
        Self::new(2, 2, 16, 2) // 2 NUMA nodes, 2 sockets, 16 cores/socket, 2 threads/core
    }

    /// Get typical desktop topology
    pub fn desktop() -> Self {
        Self::new(1, 1, 8, 2) // 1 NUMA node, 1 socket, 8 cores/socket, 2 threads/core
    }

    /// Get CPU ID for location
    pub fn cpu_id(&self, package: u8, core: u8, thread: u8) -> CpuId {
        (package as u32) * (self.cores_per_package as u32) * (self.threads_per_core as u32)
            + (core as u32) * (self.threads_per_core as u32)
            + (thread as u32)
    }

    /// Get NUMA node for CPU
    pub fn numa_node_for_cpu(&self, cpu_id: CpuId) -> NumaNodeId {
        let cpus_per_node = self.total_cpus / (self.numa_nodes as u32);
        (cpu_id / cpus_per_node) as u8
    }
}

/// CPU utilization statistics
#[derive(Debug, Clone)]
pub struct CpuUtilization {
    /// CPU ID
    pub cpu_id: CpuId,
    /// Current utilization (0.0 - 1.0)
    pub utilization: f64,
    /// Load average (1, 5, 15 minutes)
    pub load_avg: (f64, f64, f64),
    /// Run queue length
    pub run_queue_len: u32,
    /// Context switches per second
    pub context_switches: u64,
    /// CPU frequency (MHz)
    pub frequency_mhz: u32,
}

/// Task information for scheduling
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Task ID
    pub task_id: TaskId,
    /// Task priority (0-255, higher = more important)
    pub priority: u8,
    /// Scheduling policy
    pub policy: SchedulingPolicy,
    /// Current CPU
    pub current_cpu: Option<CpuId>,
    /// Preferred NUMA node
    pub preferred_numa_node: Option<NumaNodeId>,
    /// CPU usage (0.0 - 1.0)
    pub cpu_usage: f64,
    /// Memory affinity
    pub memory_affinity: Vec<NumaNodeId>,
    /// Last execution time (nanoseconds)
    pub last_runtime_ns: u64,
    /// Total runtime (nanoseconds)
    pub total_runtime_ns: u64,
}

impl TaskInfo {
    /// Create new task info
    pub fn new(task_id: TaskId, priority: u8, policy: SchedulingPolicy) -> Self {
        Self {
            task_id,
            priority,
            policy,
            current_cpu: None,
            preferred_numa_node: None,
            memory_affinity: Vec::new(),
            cpu_usage: 0.0,
            last_runtime_ns: 0,
            total_runtime_ns: 0,
        }
    }

    /// Get scheduling score (higher = better)
    pub fn scheduling_score(&self) -> f64 {
        let policy_score = match self.policy {
            SchedulingPolicy::RealtimeFifo | SchedulingPolicy::RealtimeRR => 1000.0,
            SchedulingPolicy::Normal => 500.0,
            SchedulingPolicy::Batch => 100.0,
            SchedulingPolicy::Idle => 0.0,
            SchedulingPolicy::PowerSave => 200.0,
        };

        policy_score + (self.priority as f64) + (self.cpu_usage * 100.0)
    }
}

/// NUMA-aware scheduler
pub struct NumaScheduler {
    /// Topology
    topology: CpuTopology,
    /// Per-NUMA node utilization
    node_utilization: Mutex<Vec<AtomicU64>>,
    /// NUMA memory affinity
    memory_affinity: Mutex<BTreeMap<TaskId, NumaNodeId>>,
    /// Active flag
    active: AtomicBool,
}

impl NumaScheduler {
    /// Create new NUMA scheduler
    pub fn new(topology: CpuTopology) -> Self {
        let node_utilization = (0..topology.numa_nodes)
            .map(|_| AtomicU64::new(0))
            .collect();

        Self {
            topology,
            node_utilization: Mutex::new(node_utilization),
            memory_affinity: Mutex::new(BTreeMap::new()),
            active: AtomicBool::new(true),
        }
    }

    /// Set NUMA memory affinity for task
    pub fn set_affinity(&self, task_id: TaskId, node: NumaNodeId) {
        let mut affinity = self.memory_affinity.lock();
        affinity.insert(task_id, node);
    }

    /// Get best NUMA node for task
    pub fn select_node(&self, task: &TaskInfo) -> NumaNodeId {
        // Use preferred node if set
        if let Some(node) = task.preferred_numa_node {
            return node;
        }

        // Check affinity cache
        {
            let affinity = self.memory_affinity.lock();
            if let Some(&node) = affinity.get(&task.task_id) {
                return node;
            }
        }

        // Select least loaded node
        let utilization = self.node_utilization.lock();
        let (best_node, _) = utilization.iter()
            .enumerate()
            .min_by_key(|(_, load)| load.load(Ordering::Relaxed))
            .unwrap();

        best_node as u8
    }

    /// Record task execution on node
    pub fn record_execution(&self, _node: NumaNodeId, _duration_ns: u64) {
        // Update utilization (simplified)
        // In real implementation, use exponential moving average
    }

    /// Get node utilization
    pub fn get_node_utilization(&self) -> Vec<f64> {
        let utilization = self.node_utilization.lock();
        // Normalize to 0.0-1.0
        utilization.iter()
            .map(|load| {
                let val = load.load(Ordering::Relaxed);
                (val as f64 / 1_000_000_000.0).min(1.0)
            })
            .collect()
    }
}

/// Load balancer
pub struct LoadBalancer {
    /// Topology
    topology: CpuTopology,
    /// Per-CPU load
    cpu_load: Mutex<Vec<AtomicU64>>,
    /// Load balancing threshold
    imbalance_threshold: f64,
    /// Active flag
    active: AtomicBool,
}

impl LoadBalancer {
    /// Create new load balancer
    pub fn new(topology: CpuTopology, imbalance_threshold: f64) -> Self {
        let cpu_load = (0..topology.total_cpus)
            .map(|_| AtomicU64::new(0))
            .collect();

        Self {
            topology,
            cpu_load: Mutex::new(cpu_load),
            imbalance_threshold,
            active: AtomicBool::new(true),
        }
    }

    /// Find least loaded CPU
    pub fn find_least_loaded_cpu(&self, candidate_cpus: &[CpuId]) -> Option<CpuId> {
        let load = self.cpu_load.lock();

        candidate_cpus.iter()
            .min_by_key(|&&cpu_id| {
                load.get(cpu_id as usize)
                    .map(|l| l.load(Ordering::Relaxed))
                    .unwrap_or(u64::MAX)
            })
            .copied()
    }

    /// Check if load balancing is needed
    pub fn needs_balancing(&self) -> bool {
        let load = self.cpu_load.lock();

        let loads: Vec<u64> = load.iter()
            .map(|l| l.load(Ordering::Relaxed))
            .collect();

        if loads.is_empty() {
            return false;
        }

        let max = *loads.iter().max().unwrap() as f64;
        let min = *loads.iter().min().unwrap() as f64;

        if min == 0.0 {
            return false;
        }

        (max - min) / min > self.imbalance_threshold
    }

    /// Record task execution
    pub fn record_execution(&self, cpu_id: CpuId, duration_ns: u64) {
        let load = self.cpu_load.lock();
        if let Some(counter) = load.get(cpu_id as usize) {
            counter.fetch_add(duration_ns, Ordering::Relaxed);
        }
    }

    /// Get per-CPU load
    pub fn get_cpu_load(&self) -> Vec<f64> {
        let load = self.cpu_load.lock();
        load.iter()
            .map(|l| {
                let val = l.load(Ordering::Relaxed);
                (val as f64 / 1_000_000_000.0).min(1.0)
            })
            .collect()
    }
}

/// Power-aware scheduler
pub struct PowerScheduler {
    /// Topology
    topology: CpuTopology,
    /// Power state per CPU
    cpu_power_state: Mutex<Vec<PowerState>>,
    /// Current power policy
    policy: Mutex<PowerPolicy>,
}

/// CPU power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    /// Full power
    Full,
    /// Reduced frequency
    Reduced { frequency_percent: u8 },
    /// Deep sleep (C-state)
    DeepSleep,
}

/// Power policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPolicy {
    /// Maximum performance
    Performance,
    /// Balance performance and power
    Balanced,
    /// Power saving
    PowerSave,
    /// Dynamic based on load
    Dynamic,
}

impl PowerScheduler {
    /// Create new power scheduler
    pub fn new(topology: CpuTopology) -> Self {
        let cpu_power_state = (0..topology.total_cpus)
            .map(|_| PowerState::Full)
            .collect();

        Self {
            topology,
            cpu_power_state: Mutex::new(cpu_power_state),
            policy: Mutex::new(PowerPolicy::Balanced),
        }
    }

    /// Set power policy
    pub fn set_policy(&self, policy: PowerPolicy) {
        let mut current_policy = self.policy.lock();
        *current_policy = policy;
    }

    /// Get optimal CPU for task based on power policy
    pub fn select_cpu(&self, available_cpus: &[CpuId], task: &TaskInfo) -> Option<CpuId> {
        let policy = *self.policy.lock();

        match policy {
            PowerPolicy::Performance => {
                // Select any available CPU for max performance
                available_cpus.first().copied()
            }
            PowerPolicy::PowerSave => {
                // Consolidate on fewer CPUs
                self.find_least_used_cpu(available_cpus)
            }
            PowerPolicy::Balanced => {
                // Balance based on task priority
                if task.priority > 128 {
                    available_cpus.first().copied()
                } else {
                    self.find_least_used_cpu(available_cpus)
                }
            }
            PowerPolicy::Dynamic => {
                // Select based on system load
                if self.is_high_load() {
                    available_cpus.first().copied()
                } else {
                    self.find_least_used_cpu(available_cpus)
                }
            }
        }
    }

    /// Find least used CPU
    fn find_least_used_cpu(&self, available_cpus: &[CpuId]) -> Option<CpuId> {
        let states = self.cpu_power_state.lock();

        available_cpus.iter()
            .filter(|&&cpu_id| {
                states.get(cpu_id as usize)
                    .map(|s| *s == PowerState::Full)
                    .unwrap_or(false)
            })
            .copied()
            .next()
    }

    /// Check if system is under high load
    fn is_high_load(&self) -> bool {
        let states = self.cpu_power_state.lock();
        let full_power_count = states.iter()
            .filter(|s| **s == PowerState::Full)
            .count();

        full_power_count > (states.len() / 2)
    }

    /// Set CPU power state
    pub fn set_cpu_power_state(&self, cpu_id: CpuId, state: PowerState) {
        let mut states = self.cpu_power_state.lock();
        if let Some(cpu_state) = states.get_mut(cpu_id as usize) {
            *cpu_state = state;
        }
    }

    /// Get CPU power state
    pub fn get_cpu_power_state(&self, cpu_id: CpuId) -> Option<PowerState> {
        let states = self.cpu_power_state.lock();
        states.get(cpu_id as usize).copied()
    }
}

/// Real-time scheduler
pub struct RealtimeScheduler {
    /// Ready queue for FIFO tasks
    fifo_queue: Mutex<Vec<TaskInfo>>,
    /// Ready queue for RR tasks
    rr_queue: Mutex<Vec<TaskInfo>>,
    /// Currently running tasks
    running: Mutex<BTreeMap<CpuId, TaskId>>,
}

impl RealtimeScheduler {
    /// Create new real-time scheduler
    pub fn new() -> Self {
        Self {
            fifo_queue: Mutex::new(Vec::new()),
            rr_queue: Mutex::new(Vec::new()),
            running: Mutex::new(BTreeMap::new()),
        }
    }

    /// Enqueue real-time task
    pub fn enqueue(&self, task: TaskInfo) {
        match task.policy {
            SchedulingPolicy::RealtimeFifo => {
                let mut queue = self.fifo_queue.lock();
                queue.push(task);
                queue.sort_by(|a, b| b.priority.cmp(&a.priority));
            }
            SchedulingPolicy::RealtimeRR => {
                let mut queue = self.rr_queue.lock();
                queue.push(task);
                queue.sort_by(|a, b| b.priority.cmp(&a.priority));
            }
            _ => {}
        }
    }

    /// Select next real-time task
    pub fn select_next(&self, cpu_id: CpuId) -> Option<TaskInfo> {
        // Check FIFO queue first
        {
            let mut fifo_queue = self.fifo_queue.lock();
            if !fifo_queue.is_empty() {
                let task = fifo_queue.remove(0);
                let mut running = self.running.lock();
                running.insert(cpu_id, task.task_id);
                return Some(task);
            }
        }

        // Check RR queue
        {
            let mut rr_queue = self.rr_queue.lock();
            if !rr_queue.is_empty() {
                let task = rr_queue.remove(0);
                // Re-queue at end for round-robin
                let task_clone = task.clone();
                rr_queue.push(task_clone);

                let mut running = self.running.lock();
                running.insert(cpu_id, task.task_id);
                return Some(task);
            }
        }

        None
    }

    /// Remove task from scheduler
    pub fn remove(&self, task_id: TaskId) {
        let mut fifo_queue = self.fifo_queue.lock();
        fifo_queue.retain(|t| t.task_id != task_id);

        let mut rr_queue = self.rr_queue.lock();
        rr_queue.retain(|t| t.task_id != task_id);

        let mut running = self.running.lock();
        running.retain(|_, &mut tid| tid != task_id);
    }
}

/// Unified scheduler
pub struct UnifiedScheduler {
    /// CPU topology
    topology: CpuTopology,
    /// NUMA scheduler
    numa: NumaScheduler,
    /// Load balancer
    load_balancer: LoadBalancer,
    /// Power scheduler
    power: PowerScheduler,
    /// Real-time scheduler
    realtime: RealtimeScheduler,
    /// Runnable tasks
    runnable_tasks: Mutex<Vec<TaskInfo>>,
    /// Active flag
    active: AtomicBool,
}

impl UnifiedScheduler {
    /// Create new unified scheduler
    pub fn new(topology: CpuTopology) -> Self {
        Self {
            numa: NumaScheduler::new(topology.clone()),
            load_balancer: LoadBalancer::new(topology.clone(), 0.2),
            power: PowerScheduler::new(topology.clone()),
            realtime: RealtimeScheduler::new(),
            topology,
            runnable_tasks: Mutex::new(Vec::new()),
            active: AtomicBool::new(true),
        }
    }

    /// Enqueue task for scheduling
    pub fn enqueue(&self, task: TaskInfo) {
        match task.policy {
            SchedulingPolicy::RealtimeFifo | SchedulingPolicy::RealtimeRR => {
                self.realtime.enqueue(task);
            }
            _ => {
                let mut tasks = self.runnable_tasks.lock();
                tasks.push(task);
            }
        }
    }

    /// Select next task for CPU
    pub fn select_next_task(&self, cpu_id: CpuId) -> Option<TaskInfo> {
        if !self.active.load(Ordering::Acquire) {
            return None;
        }

        // Check real-time tasks first
        if let Some(task) = self.realtime.select_next(cpu_id) {
            return Some(task);
        }

        // Select from normal tasks
        let mut tasks = self.runnable_tasks.lock();

        // Get NUMA node for this CPU
        let _numa_node = self.topology.numa_node_for_cpu(cpu_id);

        // Filter tasks that can run on this CPU
        let candidates: Vec<_> = tasks.drain(..)
            .filter(|t| {
                t.current_cpu.is_none() || t.current_cpu == Some(cpu_id)
            })
            .collect();

        // Sort by scheduling score
        let mut sorted: Vec<_> = candidates.into_iter()
            .map(|mut t| {
                t.current_cpu = Some(cpu_id);
                t
            })
            .collect();

        sorted.sort_by(|a, b| {
            b.scheduling_score()
                .partial_cmp(&a.scheduling_score())
                .unwrap()
        });

        sorted.into_iter().next()
    }

    /// Update task statistics
    pub fn update_task_stats(&self, task_id: TaskId, runtime_ns: u64) {
        // Update load balancer
        if let Some(cpu_id) = self.get_task_cpu(task_id) {
            self.load_balancer.record_execution(cpu_id, runtime_ns);
        }
    }

    /// Get task's current CPU
    fn get_task_cpu(&self, task_id: TaskId) -> Option<CpuId> {
        let running = self.realtime.running.lock();
        running.iter()
            .find(|(_, tid)| **tid == task_id)
            .map(|(&cpu_id, _)| cpu_id)
    }

    /// Get scheduling statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let runnable_count = self.runnable_tasks.lock().len();
        let cpu_load = self.load_balancer.get_cpu_load();
        let numa_utilization = self.numa.get_node_utilization();

        SchedulerStats {
            runnable_tasks: runnable_count as u64,
            average_load: if cpu_load.is_empty() {
                0.0
            } else {
                cpu_load.iter().sum::<f64>() / cpu_load.len() as f64
            },
            max_load: cpu_load.iter().cloned().fold(0.0f64, f64::max),
            min_load: cpu_load.iter().cloned().fold(1.0f64, f64::min),
            numa_utilization,
            load_balancing_needed: self.load_balancer.needs_balancing(),
        }
    }

    /// Start scheduler
    pub fn start(&self) {
        self.active.store(true, Ordering::Release);
        log::info!("Scheduler started");
    }

    /// Stop scheduler
    pub fn stop(&self) {
        self.active.store(false, Ordering::Release);
        log::info!("Scheduler stopped");
    }
}

/// Scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    /// Number of runnable tasks
    pub runnable_tasks: u64,
    /// Average CPU load
    pub average_load: f64,
    /// Maximum CPU load
    pub max_load: f64,
    /// Minimum CPU load
    pub min_load: f64,
    /// Per-NUMA node utilization
    pub numa_utilization: Vec<f64>,
    /// Whether load balancing is needed
    pub load_balancing_needed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_topology() {
        let topology = CpuTopology::server();
        assert_eq!(topology.numa_nodes, 2);
        assert_eq!(topology.packages, 2);
        assert_eq!(topology.total_cpus, 64); // 2*2*16*2

        let cpu_id = topology.cpu_id(0, 0, 0);
        assert_eq!(cpu_id, 0);

        let numa_node = topology.numa_node_for_cpu(0);
        assert_eq!(numa_node, 0);
    }

    #[test]
    fn test_task_info() {
        let task = TaskInfo::new(123, 100, SchedulingPolicy::Normal);
        assert_eq!(task.task_id, 123);
        assert_eq!(task.priority, 100);

        let score = task.scheduling_score();
        assert!(score > 0.0);
    }

    #[test]
    fn test_numa_scheduler() {
        let topology = CpuTopology::desktop();
        let scheduler = NumaScheduler::new(topology);

        let mut task = TaskInfo::new(1, 50, SchedulingPolicy::Normal);
        task.preferred_numa_node = Some(0);

        let node = scheduler.select_node(&task);
        assert_eq!(node, 0);
    }

    #[test]
    fn test_load_balancer() {
        let topology = CpuTopology::desktop();
        let balancer = LoadBalancer::new(topology, 0.3);

        let candidates = vec![0, 1, 2, 3];
        let cpu = balancer.find_least_loaded_cpu(&candidates);

        assert!(cpu.is_some());
        assert!(candidates.contains(&cpu.unwrap()));
    }

    #[test]
    fn test_load_balancing_needed() {
        let topology = CpuTopology::desktop();
        let balancer = LoadBalancer::new(topology, 0.3);

        // Record some load
        balancer.record_execution(0, 1_000_000);
        balancer.record_execution(1, 100_000_000);

        // Should need balancing
        assert!(balancer.needs_balancing());
    }

    #[test]
    fn test_power_scheduler() {
        let topology = CpuTopology::desktop();
        let scheduler = PowerScheduler::new(topology);

        scheduler.set_policy(PowerPolicy::PowerSave);

        let task = TaskInfo::new(1, 50, SchedulingPolicy::Normal);
        let available = vec![0, 1, 2];

        let cpu = scheduler.select_cpu(&available, &task);
        assert!(cpu.is_some());
    }

    #[test]
    fn test_realtime_scheduler() {
        let scheduler = RealtimeScheduler::new();

        let task1 = TaskInfo::new(1, 200, SchedulingPolicy::RealtimeFifo);
        let task2 = TaskInfo::new(2, 150, SchedulingPolicy::RealtimeFifo);

        scheduler.enqueue(task1);
        scheduler.enqueue(task2);

        let next = scheduler.select_next(0);
        assert!(next.is_some());
        assert_eq!(next.unwrap().task_id, 1); // Higher priority
    }

    #[test]
    fn test_unified_scheduler() {
        let topology = CpuTopology::desktop();
        let scheduler = UnifiedScheduler::new(topology);

        scheduler.start();

        let task = TaskInfo::new(1, 100, SchedulingPolicy::Normal);
        scheduler.enqueue(task);

        let next = scheduler.select_next_task(0);
        assert!(next.is_some());

        let stats = scheduler.get_stats();
        assert_eq!(stats.runnable_tasks, 0); // Task was selected
    }

    #[test]
    fn test_power_state() {
        let topology = CpuTopology::desktop();
        let scheduler = PowerScheduler::new(topology);

        scheduler.set_cpu_power_state(0, PowerState::Reduced { frequency_percent: 50 });

        let state = scheduler.get_cpu_power_state(0);
        assert_eq!(state, Some(PowerState::Reduced { frequency_percent: 50 }));
    }

    #[test]
    fn test_numa_affinity() {
        let topology = CpuTopology::server();
        let scheduler = NumaScheduler::new(topology);

        scheduler.set_affinity(123, 1);

        let mut task = TaskInfo::new(123, 50, SchedulingPolicy::Normal);
        task.preferred_numa_node = None;

        let node = scheduler.select_node(&task);
        assert_eq!(node, 1); // Should use affinity
    }

    #[test]
    fn test_scheduling_policy_score() {
        let rt_task = TaskInfo::new(1, 100, SchedulingPolicy::RealtimeFifo);
        let normal_task = TaskInfo::new(2, 100, SchedulingPolicy::Normal);
        let batch_task = TaskInfo::new(3, 100, SchedulingPolicy::Batch);

        assert!(rt_task.scheduling_score() > normal_task.scheduling_score());
        assert!(normal_task.scheduling_score() > batch_task.scheduling_score());
    }
}
