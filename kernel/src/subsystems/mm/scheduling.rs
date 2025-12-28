//! NUMA Scheduling Module
//! 
//! This module provides NUMA-aware scheduling functionality, including
//! task affinity, load balancing, and migration support.

use crate::error::unified::UnifiedError;
use crate::subsystems::sync::Mutex;
use crate::numa::topology::{NodeId, NUMATopology, CPUMask};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Task affinity for NUMA systems
#[derive(Debug, Clone)]
pub struct NUMATaskAffinity {
    /// Task ID
    pub task_id: u64,
    /// Preferred NUMA nodes
    pub preferred_nodes: Vec<NodeId>,
    /// CPU mask for task execution
    pub cpu_mask: CPUMask,
    /// Memory allocation policy for this task
    pub memory_policy: crate::numa::memory::MemoryPolicy,
    /// Migration cost threshold
    pub migration_threshold: f32,
    /// Last migration time
    pub last_migration: u64,
}

/// Load balancing policies for NUMA systems
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingPolicy {
    /// No load balancing
    None,
    /// Simple round-robin
    RoundRobin,
    /// Load-based balancing
    LoadBased,
    /// Distance-aware balancing
    DistanceAware,
    /// Hybrid approach
    Hybrid,
}

/// NUMA scheduler
pub struct NUMAScheduler {
    /// NUMA topology
    topology: NUMATopology,
    /// Task affinities
    task_affinities: Mutex<BTreeMap<u64, NUMATaskAffinity>>,
    /// Node load information
    node_loads: Mutex<Vec<NodeLoad>>,
    /// Current load balancing policy
    current_policy: Mutex<LoadBalancingPolicy>,
    /// Round-robin counter
    round_robin_counter: AtomicUsize,
    /// Migration statistics
    migration_stats: Mutex<MigrationStats>,
    /// Load balancing statistics
    load_balance_stats: Mutex<LoadBalanceStats>,
}

/// Node load information
#[derive(Debug, Default, Clone)]
pub struct NodeLoad {
    /// Node ID
    pub node_id: NodeId,
    /// CPU utilization (0.0 to 1.0)
    pub cpu_utilization: f32,
    /// Memory utilization (0.0 to 1.0)
    pub memory_utilization: f32,
    /// Number of running tasks
    pub running_tasks: usize,
    /// Number of runnable tasks
    pub runnable_tasks: usize,
    /// Load score (higher is more loaded)
    pub load_score: f32,
    /// Last update time
    pub last_update: u64,
}

/// Migration statistics
#[derive(Debug, Default)]
pub struct MigrationStats {
    /// Total task migrations
    pub total_migrations: u64,
    /// Successful migrations
    pub successful_migrations: u64,
    /// Failed migrations
    pub failed_migrations: u64,
    /// Migrations by reason
    pub migrations_by_reason: BTreeMap<MigrationReason, u64>,
    /// Average migration time (microseconds)
    pub avg_migration_time_us: u64,
    /// Total migration time (microseconds)
    pub total_migration_time_us: u64,
}

/// Load balance statistics
#[derive(Debug, Default)]
pub struct LoadBalanceStats {
    /// Total load balancing operations
    pub total_balancing_ops: u64,
    /// Load imbalance events
    pub imbalance_events: u64,
    /// Average imbalance score
    pub avg_imbalance_score: f32,
    /// Time spent balancing (microseconds)
    pub total_balancing_time_us: u64,
    /// Average balancing time (microseconds)
    pub avg_balancing_time_us: u64,
}

/// Migration reasons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    /// Load balancing
    LoadBalancing,
    /// Memory locality
    MemoryLocality,
    /// Power management
    PowerManagement,
    /// Thermal management
    ThermalManagement,
    /// Explicit request
    ExplicitRequest,
    /// Affinity violation
    AffinityViolation,
}

impl NUMAScheduler {
    /// Create a new NUMA scheduler
    pub fn new(topology: NUMATopology) -> Self {
        let node_count = topology.node_count();
        let mut node_loads = Vec::with_capacity(node_count);
        
        // Initialize node loads
        for i in 0..node_count {
            node_loads.push(NodeLoad {
                node_id: i,
                cpu_utilization: 0.0,
                memory_utilization: 0.0,
                running_tasks: 0,
                runnable_tasks: 0,
                load_score: 0.0,
                last_update: Self::get_timestamp(),
            });
        }
        
        Self {
            topology,
            task_affinities: Mutex::new(BTreeMap::new()),
            node_loads: Mutex::new(node_loads),
            current_policy: Mutex::new(LoadBalancingPolicy::LoadBased),
            round_robin_counter: AtomicUsize::new(0),
            migration_stats: Mutex::new(MigrationStats::default()),
            load_balance_stats: Mutex::new(LoadBalanceStats::default()),
        }
    }
    
    /// Set task affinity to specific NUMA nodes
    pub fn set_task_affinity(&self, task_id: u64, node_mask: CPUMask) -> Result<(), UnifiedError> {
        // Determine preferred nodes from CPU mask
        let mut preferred_nodes = Vec::new();
        for cpu_id in node_mask.iter() {
            if let Ok(Some(node_id)) = self.topology.find_node_for_cpu(cpu_id) {
                if !preferred_nodes.contains(&node_id) {
                    preferred_nodes.push(node_id);
                }
            }
        }
        
        let affinity = NUMATaskAffinity {
            task_id,
            preferred_nodes,
            cpu_mask: node_mask.clone(),
            memory_policy: crate::numa::memory::MemoryPolicy::Local,
            migration_threshold: 0.8, // Default threshold
            last_migration: 0,
        };
        
        let mut affinities = self.task_affinities.lock();
        affinities.insert(task_id, affinity);
        
        Ok(())
    }
    
    /// Get task affinity
    pub fn get_task_affinity(&self, task_id: u64) -> Result<CPUMask, UnifiedError> {
        let affinities = self.task_affinities.lock();
        
        match affinities.get(&task_id) {
            Some(affinity) => Ok(affinity.cpu_mask.clone()),
            None => {
                // Return default CPU mask (all CPUs)
                let mut mask = CPUMask::with_capacity(self.topology.total_cpu_count());
                for cpu_id in 0..self.topology.total_cpu_count() {
                    mask.set_cpu(cpu_id);
                }
                Ok(mask)
            }
        }
    }
    
    /// Migrate task to a different NUMA node
    pub fn migrate_task(&self, task_id: u64, target_node: NodeId) -> Result<(), UnifiedError> {
        let start_time = Self::get_timestamp();
        
        // Get current task affinity
        let current_affinity = {
            let affinities = self.task_affinities.lock();
            affinities.get(&task_id).cloned()
        };
        
        // Check if task is already on target node
        if let Some(ref affinity) = current_affinity {
            if affinity.preferred_nodes.contains(&target_node) {
                return Ok(()); // Already on target node
            }
        }
        
        // Find a suitable CPU on the target node
        let target_cpu = self.find_cpu_on_node(target_node)?;
        
        // Create new affinity for target node
        let mut new_cpu_mask = CPUMask::with_capacity(self.topology.total_cpu_count());
        new_cpu_mask.set_cpu(target_cpu);
        
        // Update task affinity
        {
            let mut affinities = self.task_affinities.lock();
            
            if let Some(mut affinity) = current_affinity {
                affinity.preferred_nodes.clear();
                affinity.preferred_nodes.push(target_node);
                affinity.cpu_mask = new_cpu_mask;
                affinity.last_migration = start_time;
                affinities.insert(task_id, affinity);
            } else {
                // Create new affinity
                let affinity = NUMATaskAffinity {
                    task_id,
                    preferred_nodes: vec![target_node],
                    cpu_mask: new_cpu_mask,
                    memory_policy: crate::numa::memory::MemoryPolicy::Local,
                    migration_threshold: 0.8,
                    last_migration: start_time,
                };
                affinities.insert(task_id, affinity);
            }
        }
        
        // Update node loads
        self.update_node_loads_for_migration(task_id, target_node)?;
        
        // Update migration statistics
        {
            let mut stats = self.migration_stats.lock();
            stats.total_migrations += 1;
            stats.successful_migrations += 1;
            
            let migration_time = Self::get_timestamp() - start_time;
            stats.total_migration_time_us += migration_time;
            stats.avg_migration_time_us = stats.total_migration_time_us / stats.total_migrations;
            
            *stats.migrations_by_reason.entry(MigrationReason::ExplicitRequest).or_insert(0) += 1;
        }
        
        Ok(())
    }
    
    /// Set load balancing policy
    pub fn set_load_balancing_policy(&self, policy: LoadBalancingPolicy) -> Result<(), UnifiedError> {
        let mut current_policy = self.current_policy.lock();
        *current_policy = policy;
        Ok(())
    }
    
    /// Get current load balancing policy
    pub fn get_current_policy(&self) -> LoadBalancingPolicy {
        *self.current_policy.lock()
    }
    
    /// Get CPU usage for a specific node
    pub fn get_cpu_usage(&self, node_id: NodeId) -> Result<f32, UnifiedError> {
        let node_loads = self.node_loads.lock();
        
        if node_id >= node_loads.len() {
            return Err(UnifiedError::system("Invalid NUMA node ID", None));
        }
        
        Ok(node_loads[node_id].cpu_utilization)
    }
    
    /// Update node loads
    pub fn update_node_loads(&self) -> Result<(), UnifiedError> {
        let mut node_loads = self.node_loads.lock();
        let current_time = Self::get_timestamp();
        
        // Update each node's load
        for (node_id, node_load) in node_loads.iter_mut().enumerate() {
            // In a real implementation, this would query actual CPU and memory usage
            // For now, we'll simulate some values
            node_load.cpu_utilization = self.simulate_cpu_usage(node_id);
            node_load.memory_utilization = self.simulate_memory_usage(node_id);
            node_load.running_tasks = self.count_running_tasks(node_id);
            node_load.runnable_tasks = self.count_runnable_tasks(node_id);
            
            // Calculate load score
            node_load.load_score = self.calculate_load_score(node_load);
            node_load.last_update = current_time;
        }
        
        // Check for load imbalance
        self.check_load_imbalance(&node_loads)?;
        
        Ok(())
    }
    
    /// Perform load balancing
    pub fn balance_loads(&self) -> Result<Vec<TaskMigration>, UnifiedError> {
        let start_time = Self::get_timestamp();
        let policy = *self.current_policy.lock();
        
        let mut migrations = Vec::new();
        
        match policy {
            LoadBalancingPolicy::None => {
                // No load balancing
            },
            LoadBalancingPolicy::RoundRobin => {
                migrations = self.round_robin_balance()?;
            },
            LoadBalancingPolicy::LoadBased => {
                migrations = self.load_based_balance()?;
            },
            LoadBalancingPolicy::DistanceAware => {
                migrations = self.distance_aware_balance()?;
            },
            LoadBalancingPolicy::Hybrid => {
                migrations = self.hybrid_balance()?;
            },
        }
        
        // Update statistics
        {
            let mut stats = self.load_balance_stats.lock();
            stats.total_balancing_ops += 1;
            
            let balancing_time = Self::get_timestamp() - start_time;
            stats.total_balancing_time_us += balancing_time;
            stats.avg_balancing_time_us = stats.total_balancing_time_us / stats.total_balancing_ops;
            
            if !migrations.is_empty() {
                stats.imbalance_events += 1;
            }
        }
        
        Ok(migrations)
    }
    
    /// Find a CPU on a specific node
    fn find_cpu_on_node(&self, node_id: NodeId) -> Result<usize, UnifiedError> {
        if let Ok(node) = self.topology.get_node(node_id) {
            if let Some(cpu_id) = node.cpu_mask.first_cpu() {
                return Ok(cpu_id);
            }
        }
        
        Err(UnifiedError::system("No CPU available on NUMA node", None))
    }
    
    /// Update node loads for task migration
    fn update_node_loads_for_migration(&self, task_id: u64, target_node: NodeId) -> Result<(), UnifiedError> {
        let mut node_loads = self.node_loads.lock();
        
        // Find source node (simplified - just use current node)
        let source_node = self.topology.get_current_node()?;
        
        if source_node >= node_loads.len() || target_node >= node_loads.len() {
            return Err(UnifiedError::system("Invalid NUMA node ID", None));
        }
        
        // Update source node (decrease load)
        node_loads[source_node].running_tasks = node_loads[source_node].running_tasks.saturating_sub(1);
        node_loads[source_node].load_score = self.calculate_load_score(&node_loads[source_node]);
        
        // Update target node (increase load)
        node_loads[target_node].running_tasks += 1;
        node_loads[target_node].load_score = self.calculate_load_score(&node_loads[target_node]);
        
        Ok(())
    }
    
    /// Simulate CPU usage for a node
    fn simulate_cpu_usage(&self, node_id: NodeId) -> f32 {
        // In a real implementation, this would query actual CPU usage
        // For now, we'll simulate some values
        match node_id % 4 {
            0 => 0.3 + (Self::get_timestamp() % 100) as f32 / 100.0,
            1 => 0.5 + (Self::get_timestamp() % 100) as f32 / 200.0,
            2 => 0.7 + (Self::get_timestamp() % 100) as f32 / 300.0,
            _ => 0.4 + (Self::get_timestamp() % 100) as f32 / 250.0,
        }
    }
    
    /// Simulate memory usage for a node
    fn simulate_memory_usage(&self, node_id: NodeId) -> f32 {
        // In a real implementation, this would query actual memory usage
        // For now, we'll simulate some values
        match node_id % 4 {
            0 => 0.4 + (Self::get_timestamp() % 100) as f32 / 250.0,
            1 => 0.6 + (Self::get_timestamp() % 100) as f32 / 300.0,
            2 => 0.8 + (Self::get_timestamp() % 100) as f32 / 350.0,
            _ => 0.5 + (Self::get_timestamp() % 100) as f32 / 275.0,
        }
    }
    
    /// Count running tasks on a node
    fn count_running_tasks(&self, node_id: NodeId) -> usize {
        // In a real implementation, this would count actual running tasks
        // For now, we'll simulate some values
        (node_id + 1) * 2
    }
    
    /// Count runnable tasks on a node
    fn count_runnable_tasks(&self, node_id: NodeId) -> usize {
        // In a real implementation, this would count actual runnable tasks
        // For now, we'll simulate some values
        (node_id + 1) * 3
    }
    
    /// Calculate load score for a node
    fn calculate_load_score(&self, node_load: &NodeLoad) -> f32 {
        // Combine CPU, memory, and task load into a single score
        let cpu_weight = 0.4;
        let memory_weight = 0.3;
        let task_weight = 0.3;
        
        node_load.cpu_utilization * cpu_weight +
        node_load.memory_utilization * memory_weight +
        (node_load.running_tasks as f32 / 10.0).min(1.0) * task_weight
    }
    
    /// Check for load imbalance
    fn check_load_imbalance(&self, node_loads: &[NodeLoad]) -> Result<(), UnifiedError> {
        if node_loads.is_empty() {
            return Ok(());
        }
        
        let mut min_score = f32::MAX;
        let mut max_score = 0.0;
        
        for node_load in node_loads {
            if node_load.load_score < min_score {
                min_score = node_load.load_score;
            }
            if node_load.load_score > max_score {
                max_score = node_load.load_score;
            }
        }
        
        // Check if imbalance exceeds threshold
        let imbalance_threshold = 0.3;
        if max_score - min_score > imbalance_threshold {
            // Update statistics
            {
                let mut stats = self.load_balance_stats.lock();
                stats.imbalance_events += 1;
                stats.avg_imbalance_score = 
                    (stats.avg_imbalance_score * (stats.imbalance_events - 1) as f32 + (max_score - min_score)) / 
                    stats.imbalance_events as f32;
            }
        }
        
        Ok(())
    }
    
    /// Round-robin load balancing
    fn round_robin_balance(&self) -> Result<Vec<TaskMigration>, UnifiedError> {
        let mut migrations = Vec::new();
        let node_count = self.topology.node_count();
        
        if node_count < 2 {
            return Ok(migrations);
        }
        
        // Find the most loaded and least loaded nodes
        let node_loads = self.node_loads.lock();
        let mut most_loaded = 0;
        let mut least_loaded = 0;
        let mut max_score = 0.0;
        let mut min_score = f32::MAX;
        
        for (i, node_load) in node_loads.iter().enumerate() {
            if node_load.load_score > max_score {
                max_score = node_load.load_score;
                most_loaded = i;
            }
            if node_load.load_score < min_score {
                min_score = node_load.load_score;
                least_loaded = i;
            }
        }
        
        // If imbalance is significant, migrate a task
        if max_score - min_score > 0.2 {
            // Find a task on the most loaded node
            if let Some(task_id) = self.find_task_on_node(most_loaded) {
                migrations.push(TaskMigration {
                    task_id,
                    source_node: most_loaded,
                    target_node: least_loaded,
                    reason: MigrationReason::LoadBalancing,
                    priority: MigrationPriority::Medium,
                });
            }
        }
        
        Ok(migrations)
    }
    
    /// Load-based load balancing
    fn load_based_balance(&self) -> Result<Vec<TaskMigration>, UnifiedError> {
        let mut migrations = Vec::new();
        let node_loads = self.node_loads.lock();
        
        // Sort nodes by load score
        let mut sorted_nodes: Vec<(NodeId, f32)> = node_loads.iter()
            .enumerate()
            .map(|(i, load)| (i, load.load_score))
            .collect();
        
        sorted_nodes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        // Consider migrating tasks from the most loaded nodes to the least loaded nodes
        if sorted_nodes.len() >= 2 {
            let most_loaded = sorted_nodes.last().unwrap().0;
            let least_loaded = sorted_nodes.first().unwrap().0;
            
            let load_diff = sorted_nodes.last().unwrap().1 - sorted_nodes.first().unwrap().1;
            
            if load_diff > 0.3 {
                // Find a task on the most loaded node
                if let Some(task_id) = self.find_task_on_node(most_loaded) {
                    migrations.push(TaskMigration {
                        task_id,
                        source_node: most_loaded,
                        target_node: least_loaded,
                        reason: MigrationReason::LoadBalancing,
                        priority: MigrationPriority::High,
                    });
                }
            }
        }
        
        Ok(migrations)
    }
    
    /// Distance-aware load balancing
    fn distance_aware_balance(&self) -> Result<Vec<TaskMigration>, UnifiedError> {
        let mut migrations = Vec::new();
        let node_loads = self.node_loads.lock();
        
        // Find nodes with high load
        let mut high_load_nodes = Vec::new();
        let mut low_load_nodes = Vec::new();
        
        for (i, node_load) in node_loads.iter().enumerate() {
            if node_load.load_score > 0.7 {
                high_load_nodes.push(i);
            } else if node_load.load_score < 0.4 {
                low_load_nodes.push(i);
            }
        }
        
        // For each high load node, find the nearest low load node
        for high_node in high_load_nodes {
            if let Some(nearest_low_node) = self.topology.find_nearest_node(high_node, &[]) {
                if low_load_nodes.contains(&nearest_low_node) {
                    // Find a task on the high load node
                    if let Some(task_id) = self.find_task_on_node(high_node) {
                        migrations.push(TaskMigration {
                            task_id,
                            source_node: high_node,
                            target_node: nearest_low_node,
                            reason: MigrationReason::LoadBalancing,
                            priority: MigrationPriority::Medium,
                        });
                    }
                }
            }
        }
        
        Ok(migrations)
    }
    
    /// Hybrid load balancing
    fn hybrid_balance(&self) -> Result<Vec<TaskMigration>, UnifiedError> {
        // Combine multiple strategies
        let mut migrations = Vec::new();
        
        // Try load-based balancing first
        let load_migrations = self.load_based_balance()?;
        
        // If no migrations from load-based, try distance-aware
        if load_migrations.is_empty() {
            let distance_migrations = self.distance_aware_balance()?;
            migrations.extend(distance_migrations);
        } else {
            migrations.extend(load_migrations);
        }
        
        Ok(migrations)
    }
    
    /// Find a task on a specific node
    fn find_task_on_node(&self, node_id: NodeId) -> Option<u64> {
        let affinities = self.task_affinities.lock();
        
        for (task_id, affinity) in affinities.iter() {
            if affinity.preferred_nodes.contains(&node_id) {
                return Some(*task_id);
            }
        }
        
        None
    }
    
    /// Get current timestamp
    fn get_timestamp() -> u64 {
        // In a real implementation, this would get the actual timestamp
        // For now, we'll use a simple counter
        static TIMESTAMP: AtomicU64 = AtomicU64::new(1);
        TIMESTAMP.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Get migration statistics
    pub fn get_migration_stats(&self) -> MigrationStats {
        let stats = self.migration_stats.lock();
        stats.clone()
    }
    
    /// Get load balance statistics
    pub fn get_load_balance_stats(&self) -> LoadBalanceStats {
        let stats = self.load_balance_stats.lock();
        stats.clone()
    }
}

/// Task migration information
#[derive(Debug, Clone)]
pub struct TaskMigration {
    /// Task ID
    pub task_id: u64,
    /// Source node
    pub source_node: NodeId,
    /// Target node
    pub target_node: NodeId,
    /// Reason for migration
    pub reason: MigrationReason,
    /// Migration priority
    pub priority: MigrationPriority,
}

/// Migration priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Initialize NUMA scheduler
pub fn init_numa_scheduler(topology: &NUMATopology) -> Result<NUMAScheduler, UnifiedError> {
    Ok(NUMAScheduler::new(topology.clone()))
}