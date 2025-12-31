//! CPU Optimization Manager Module
//!
//! This module provides the main CPU optimization manager including:
//! - Performance counter integration (perf events)
//! - CPU statistics aggregation
//! - Optimization policy engine
//! - Public API exports
//!
//! This module serves as the main entry point for CPU and scheduler optimization.

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sync::{Mutex, MutexGuard};

use crate::prelude::*;

// Re-export from cpu_optimizer
pub use crate::perf::cpu_optimizer::{
    GovernorType, CState, CpuTopology, CacheTopology,
    CpuInfo, CpuLoadStats, CpuError, GovernorConfig,
    CpuOptimizer, CpuOptimizerStats,
    set_frequency_governor, set_cpu_idle_state, get_cpu_topology,
    balance_cpu_load, get_cpu_info, get_all_cpu_info,
    init_cpu_optimizer, get_cpu_optimizer, get_cpu_optimizer_stats,
};

// Re-export from scheduler_optimizer
pub use crate::perf::scheduler_optimizer::{
    SchedulingPolicy, TaskInfo, TaskState, RunqueueStats,
    SchedError, Pid, CpuId, Priority,
    NumaScheduler, LoadBalancer, RealtimeScheduler,
    SchedulerOptimizer, SchedulerStats,
    nice_to_prio, prio_to_nice,
    NICeness_MIN, NICeness_MAX, NICeness_DEFAULT,
    RT_PRIO_MIN, RT_PRIO_MAX, NORMAL_PRIO_MIN, NORMAL_PRIO_MAX, NORMAL_PRIO_DEFAULT,
    init_scheduler_optimizer, get_scheduler_optimizer,
    optimize_scheduler_latency, set_task_affinity, get_runqueue_stats,
    get_all_runqueue_stats,
};

// Re-export from cache
pub use crate::perf::cache::{
    CacheLevel, CacheInfo, CacheStats,
    AlignedBuffer, CachePadded, CacheAccessResult,
    CacheSimulator, Prefetcher,
    CacheHashTable, StructureOfArrays, AccessPatternAnalyzer,
    AccessPattern, CacheOptimizationAdvisor,
    OptimizationRecommendation,
    CACHE_LINE_SIZE,
};

// Re-export from lock_optimizer
pub use crate::perf::lock_optimizer::{
    LockStats, LockError, AdaptiveSpinlock, AdaptiveSpinlockGuard,
    SeqLock, LockFreeStack, LockFreeQueue, Rcu,
    LockContentionAnalyzer, LockOptimizer, OptimizationRecommendation as LockOptimizationRecommendation,
    init_lock_optimizer, get_lock_optimizer,
    analyze_lock_contention, optimize_lock,
};

// Re-export from instruction
pub use crate::perf::instruction::{
    CpuFeatures, ICacheStats, InstructionOptimizer, OptimizationLevel,
    likely, unlikely,
    simd, loop_unroll, inline_asm, prefetch,
    init_instruction_optimizer, get_instruction_optimizer,
    enable_simd, is_simd_enabled, branch_hint, prefetch_instruction,
};

/// Performance event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PerfEventType {
    /// CPU cycles
    CpuCycles,
    /// Instructions
    Instructions,
    /// Cache references
    CacheReferences,
    /// Cache misses
    CacheMisses,
    /// Branch instructions
    BranchInstructions,
    /// Branch misses
    BranchMisses,
    /// Bus cycles
    BusCycles,
    /// L1 cache access
    L1Access,
    /// L1 cache miss
    L1Miss,
    /// LLC access
    LlcAccess,
    /// LLC miss
    LlcMiss,
    /// TLB access
    TlbAccess,
    /// TLB miss
    TlbMiss,
}

impl PerfEventType {
    /// Get event name
    pub fn name(&self) -> &str {
        match self {
            PerfEventType::CpuCycles => "cpu-cycles",
            PerfEventType::Instructions => "instructions",
            PerfEventType::CacheReferences => "cache-references",
            PerfEventType::CacheMisses => "cache-misses",
            PerfEventType::BranchInstructions => "branch-instructions",
            PerfEventType::BranchMisses => "branch-misses",
            PerfEventType::BusCycles => "bus-cycles",
            PerfEventType::L1Access => "L1-access",
            PerfEventType::L1Miss => "L1-miss",
            PerfEventType::LlcAccess => "LLC-access",
            PerfEventType::LlcMiss => "LLC-miss",
            PerfEventType::TlbAccess => "TLB-access",
            PerfEventType::TlbMiss => "TLB-miss",
        }
    }

    /// Check if event is cache-related
    pub fn is_cache_event(&self) -> bool {
        matches!(
            self,
            PerfEventType::CacheReferences |
            PerfEventType::CacheMisses |
            PerfEventType::L1Access |
            PerfEventType::L1Miss |
            PerfEventType::LlcAccess |
            PerfEventType::LlcMiss
        )
    }
}

/// Performance counter value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerfCounterValue {
    /// Counter value
    pub value: u64,
    /// Time enabled
    pub time_enabled: u64,
    /// Time running
    pub time_running: u64,
}

impl PerfCounterValue {
    /// Create new counter value
    pub fn new(value: u64) -> Self {
        Self {
            value,
            time_enabled: 1,
            time_running: 1,
        }
    }

    /// Get scaled value (accounting for counter time)
    pub fn scaled(&self) -> u64 {
        if self.time_running == 0 {
            return 0;
        }
        (self.value * self.time_enabled) / self.time_running
    }
}

/// Per-CPU performance counters
pub struct PerCpuPerformanceCounters {
    /// CPU ID
    cpu_id: CpuId,
    /// Counter values
    counters: BTreeMap<PerfEventType, PerfCounterValue>,
    /// Enabled flag
    enabled: AtomicBool,
}

impl PerCpuPerformanceCounters {
    /// Create new per-CPU counters
    pub fn new(cpu_id: CpuId) -> Self {
        Self {
            cpu_id,
            counters: BTreeMap::new(),
            enabled: AtomicBool::new(false),
        }
    }

    /// Enable counters
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable counters
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Read counter value
    pub fn read(&self, event: PerfEventType) -> Option<PerfCounterValue> {
        if !self.is_enabled() {
            return None;
        }

        // In real implementation, read from hardware PMU
        // For now, return placeholder value
        self.counters.get(&event).copied()
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        for value in self.counters.values_mut() {
            *value = PerfCounterValue::new(0);
        }
    }
}

/// Performance counter manager
pub struct PerformanceCounterManager {
    /// Per-CPU counters
    cpu_counters: Vec<Mutex<PerCpuPerformanceCounters>>,
    /// Global enabled flag
    enabled: AtomicBool,
}

impl PerformanceCounterManager {
    /// Create new performance counter manager
    pub fn new(num_cpus: u32) -> Self {
        let mut cpu_counters = Vec::with_capacity(num_cpus as usize);
        for cpu_id in 0..num_cpus {
            cpu_counters.push(Mutex::new(PerCpuPerformanceCounters::new(cpu_id)));
        }

        Self {
            cpu_counters,
            enabled: AtomicBool::new(false),
        }
    }

    /// Enable all counters
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        for counters in &self.cpu_counters {
            counters.lock().enable();
        }
    }

    /// Disable all counters
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        for counters in &self.cpu_counters {
            counters.lock().disable();
        }
    }

    /// Read counter from specific CPU
    pub fn read_cpu_counter(&self, cpu_id: CpuId, event: PerfEventType) -> Option<PerfCounterValue> {
        if cpu_id as usize >= self.cpu_counters.len() {
            return None;
        }

        self.cpu_counters[cpu_id as usize].lock().read(event)
    }

    /// Read counter from current CPU
    pub fn read_current_cpu_counter(&self, event: PerfEventType) -> Option<PerfCounterValue> {
        let cpu_id = crate::cpu::cpuid() as CpuId;
        self.read_cpu_counter(cpu_id, event)
    }

    /// Aggregate counter values across all CPUs
    pub fn aggregate_counter(&self, event: PerfEventType) -> u64 {
        let mut total = 0u64;

        for counters in &self.cpu_counters {
            if let Some(value) = counters.lock().read(event) {
                total += value.value;
            }
        }

        total
    }
}

/// CPU statistics aggregator
pub struct CpuStatisticsAggregator {
    /// CPU optimizer reference
    cpu_optimizer: Option<&'static CpuOptimizer>,
    /// Scheduler optimizer reference
    scheduler_optimizer: Option<&'static SchedulerOptimizer>,
    /// Performance counter manager
    perf_counter_manager: Mutex<Option<PerformanceCounterManager>>,
    /// Aggregated statistics
    stats: Mutex<AggregatedCpuStats>,
}

/// Aggregated CPU statistics
#[derive(Debug, Clone)]
pub struct AggregatedCpuStats {
    /// Total CPU utilization (0-100)
    pub total_utilization: u32,
    /// Per-CPU utilization
    pub per_cpu_utilization: Vec<u32>,
    /// Context switches per second
    pub context_switches_per_sec: u64,
    /// Scheduler latency (microseconds)
    pub scheduler_latency_us: u64,
    /// Cache hit rate (0-100)
    pub cache_hit_rate: f64,
    /// Instructions per cycle (IPC)
    pub ipc: f64,
    /// Branch prediction accuracy (0-100)
    pub branch_accuracy: f64,
    /// Number of migrations
    pub migrations: u64,
}

impl AggregatedCpuStats {
    /// Create new aggregated stats
    pub fn new(num_cpus: u32) -> Self {
        Self {
            total_utilization: 0,
            per_cpu_utilization: vec![0; num_cpus as usize],
            context_switches_per_sec: 0,
            scheduler_latency_us: 0,
            cache_hit_rate: 100.0,
            ipc: 1.0,
            branch_accuracy: 95.0,
            migrations: 0,
        }
    }

    /// Calculate average utilization
    pub fn average_utilization(&self) -> u32 {
        if self.per_cpu_utilization.is_empty() {
            return 0;
        }

        let sum: u32 = self.per_cpu_utilization.iter().sum();
        sum / self.per_cpu_utilization.len() as u32
    }
}

impl CpuStatisticsAggregator {
    /// Create new CPU statistics aggregator
    pub fn new() -> Self {
        Self {
            cpu_optimizer: get_cpu_optimizer(),
            scheduler_optimizer: get_scheduler_optimizer(),
            perf_counter_manager: Mutex::new(None),
            stats: Mutex::new(AggregatedCpuStats::new(1)),
        }
    }

    /// Initialize with number of CPUs
    pub fn init(&self, num_cpus: u32) {
        *self.stats.lock() = AggregatedCpuStats::new(num_cpus);
        *self.perf_counter_manager.lock() = Some(PerformanceCounterManager::new(num_cpus));
    }

    /// Update aggregated statistics
    pub fn update(&self) {
        let mut stats = self.stats.lock();

        // Update CPU utilization
        if let Some(cpu_opt) = self.cpu_optimizer {
            let cpu_info = cpu_opt.get_all_cpu_info();
            stats.per_cpu_utilization = cpu_info.iter().map(|info| info.utilization).collect();
            stats.total_utilization = stats.average_utilization();

            // Get optimizer stats
            let opt_stats = cpu_opt.get_stats();
            stats.migrations = opt_stats.hotplug_events;
        }

        // Update scheduler statistics
        if let Some(sched_opt) = self.scheduler_optimizer {
            stats.scheduler_latency_us = sched_opt.get_average_latency();

            let all_stats = sched_opt.get_all_runqueue_stats();
            let mut total_cs = 0u64;
            for rq_stats in all_stats {
                total_cs += rq_stats.context_switches;
            }
            stats.context_switches_per_sec = total_cs;
        }

        // Update performance counter statistics
        if let Some(perf_mgr) = self.perf_counter_manager.lock().as_ref() {
            let instructions = perf_mgr.aggregate_counter(PerfEventType::Instructions);
            let cycles = perf_mgr.aggregate_counter(PerfEventType::CpuCycles);

            if cycles > 0 {
                stats.ipc = instructions as f64 / cycles as f64;
            }

            let cache_refs = perf_mgr.aggregate_counter(PerfEventType::CacheReferences);
            let cache_misses = perf_mgr.aggregate_counter(PerfEventType::CacheMisses);

            if cache_refs > 0 {
                stats.cache_hit_rate = ((cache_refs - cache_misses) as f64 / cache_refs as f64) * 100.0;
            }

            let branch_instrs = perf_mgr.aggregate_counter(PerfEventType::BranchInstructions);
            let branch_misses = perf_mgr.aggregate_counter(PerfEventType::BranchMisses);

            if branch_instrs > 0 {
                stats.branch_accuracy = ((branch_instrs - branch_misses) as f64 / branch_instrs as f64) * 100.0;
            }
        }
    }

    /// Get aggregated statistics
    pub fn get_stats(&self) -> AggregatedCpuStats {
        self.stats.lock().clone()
    }

    /// Get performance counter manager
    pub fn get_perf_counter_manager(&self) -> Option<MutexGuard<'_, PerformanceCounterManager>> {
        // Note: This returns a lock guard, so use carefully
        None // Placeholder - would need to return the lock
    }
}

/// Optimization policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPolicy {
    /// Performance-focused (maximize throughput)
    Performance,
    /// Power-focused (minimize power consumption)
    Power,
    /// Balanced (performance/power trade-off)
    Balanced,
    /// Latency-focused (minimize latency)
    Latency,
}

impl OptimizationPolicy {
    /// Get policy name
    pub fn name(&self) -> &str {
        match self {
            OptimizationPolicy::Performance => "performance",
            OptimizationPolicy::Power => "power",
            OptimizationPolicy::Balanced => "balanced",
            OptimizationPolicy::Latency => "latency",
        }
    }

    /// Check if policy is performance-focused
    pub fn is_performance_focused(&self) -> bool {
        matches!(self, OptimizationPolicy::Performance | OptimizationPolicy::Latency)
    }

    /// Check if policy is power-focused
    pub fn is_power_focused(&self) -> bool {
        matches!(self, OptimizationPolicy::Power)
    }
}

/// Optimization policy engine
pub struct OptimizationPolicyEngine {
    /// Current policy
    current_policy: Mutex<OptimizationPolicy>,
    /// Statistics aggregator
    aggregator: CpuStatisticsAggregator,
    /// Policy evaluation interval (milliseconds)
    eval_interval_ms: u32,
}

impl OptimizationPolicyEngine {
    /// Create new optimization policy engine
    pub fn new() -> Self {
        Self {
            current_policy: Mutex::new(OptimizationPolicy::Balanced),
            aggregator: CpuStatisticsAggregator::new(),
            eval_interval_ms: 1000,
        }
    }

    /// Initialize policy engine
    pub fn init(&mut self, num_cpus: u32) {
        self.aggregator.init(num_cpus);
    }

    /// Set optimization policy
    pub fn set_policy(&self, policy: OptimizationPolicy) {
        let mut current = self.current_policy.lock();
        *current = policy;

        // Apply policy settings
        self.apply_policy(policy);
    }

    /// Get current policy
    pub fn get_policy(&self) -> OptimizationPolicy {
        *self.current_policy.lock()
    }

    /// Apply policy settings
    fn apply_policy(&self, policy: OptimizationPolicy) {
        match policy {
            OptimizationPolicy::Performance => {
                // Set performance governor on all CPUs
                if let Some(cpu_opt) = get_cpu_optimizer() {
                    let num_cpus = cpu_opt.get_cpu_topology().total_cpus;
                    for cpu_id in 0..num_cpus {
                        let _ = cpu_opt.set_frequency_governor(cpu_id, GovernorType::Performance);
                    }
                }

                // Enable turbo boost
                if let Some(cpu_opt) = get_cpu_optimizer() {
                    let _ = cpu_opt.enable_turbo_boost(true);
                }
            }

            OptimizationPolicy::Power => {
                // Set powersave governor on all CPUs
                if let Some(cpu_opt) = get_cpu_optimizer() {
                    let num_cpus = cpu_opt.get_cpu_topology().total_cpus;
                    for cpu_id in 0..num_cpus {
                        let _ = cpu_opt.set_frequency_governor(cpu_id, GovernorType::Powersave);
                    }
                }

                // Disable turbo boost
                if let Some(cpu_opt) = get_cpu_optimizer() {
                    let _ = cpu_opt.enable_turbo_boost(false);
                }
            }

            OptimizationPolicy::Balanced => {
                // Set ondemand governor on all CPUs
                if let Some(cpu_opt) = get_cpu_optimizer() {
                    let num_cpus = cpu_opt.get_cpu_topology().total_cpus;
                    for cpu_id in 0..num_cpus {
                        let _ = cpu_opt.set_frequency_governor(
                            cpu_id,
                            GovernorType::Ondemand {
                                up_threshold: 80,
                                down_threshold: 20,
                            },
                        );
                    }
                }
            }

            OptimizationPolicy::Latency => {
                // Set schedutil governor for scheduler-driven frequency scaling
                if let Some(cpu_opt) = get_cpu_optimizer() {
                    let num_cpus = cpu_opt.get_cpu_topology().total_cpus;
                    for cpu_id in 0..num_cpus {
                        let _ = cpu_opt.set_frequency_governor(cpu_id, GovernorType::Schedutil);
                    }
                }

                // Optimize scheduler for low latency
                if let Some(sched_opt) = get_scheduler_optimizer() {
                    let _ = sched_opt.optimize_scheduler_latency();
                }
            }
        }
    }

    /// Evaluate and adjust policy
    pub fn evaluate(&self) {
        // Update statistics
        self.aggregator.update();

        let stats = self.aggregator.get_stats();
        let policy = self.get_policy();

        // Auto-adjust based on conditions
        match policy {
            OptimizationPolicy::Balanced => {
                // If utilization is low, switch to power mode
                if stats.total_utilization < 30 {
                    log_debug!("Low utilization, switching to power policy");
                    self.set_policy(OptimizationPolicy::Power);
                }
                // If utilization is high, switch to performance mode
                else if stats.total_utilization > 80 {
                    log_debug!("High utilization, switching to performance policy");
                    self.set_policy(OptimizationPolicy::Performance);
                }
            }

            _ => {
                // Keep current policy
            }
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> AggregatedCpuStats {
        self.aggregator.get_stats()
    }

    /// Set evaluation interval
    pub fn set_eval_interval(&mut self, interval_ms: u32) {
        self.eval_interval_ms = interval_ms;
    }
}

/// CPU optimization manager - main entry point
pub struct CpuOptimizationManager {
    /// Policy engine
    policy_engine: OptimizationPolicyEngine,
    /// Statistics aggregator
    aggregator: CpuStatisticsAggregator,
    /// Initialization status
    initialized: AtomicBool,
}

impl CpuOptimizationManager {
    /// Create new CPU optimization manager
    pub fn new() -> Self {
        Self {
            policy_engine: OptimizationPolicyEngine::new(),
            aggregator: CpuStatisticsAggregator::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize CPU optimization manager
    pub fn init(&mut self, num_cpus: u32, num_numa_nodes: u32) {
        if self.initialized.load(Ordering::Relaxed) {
            return;
        }

        // Initialize subsystems
        init_cpu_optimizer(num_cpus);
        init_scheduler_optimizer(num_cpus, num_numa_nodes);
        init_lock_optimizer();
        init_instruction_optimizer();

        // Initialize policy engine
        self.policy_engine.init(num_cpus);
        self.aggregator.init(num_cpus);

        // Apply default policy
        self.policy_engine.set_policy(OptimizationPolicy::Balanced);

        self.initialized.store(true, Ordering::Relaxed);

        log_info!("CPU optimization manager initialized with {} CPUs", num_cpus);
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Get policy engine
    pub fn get_policy_engine(&self) -> &OptimizationPolicyEngine {
        &self.policy_engine
    }

    /// Get statistics aggregator
    pub fn get_aggregator(&self) -> &CpuStatisticsAggregator {
        &self.aggregator
    }

    /// Run optimization pass
    pub fn optimize(&self) {
        if !self.is_initialized() {
            return;
        }

        // Update governors
        if let Some(cpu_opt) = get_cpu_optimizer() {
            cpu_opt.update_governors();
        }

        // Evaluate policy
        self.policy_engine.evaluate();

        // Balance CPU load
        if let Some(cpu_opt) = get_cpu_optimizer() {
            let _ = cpu_opt.balance_cpu_load();
        }

        // Trigger scheduler load balancing
        if let Some(sched_opt) = get_scheduler_optimizer() {
            let _ = sched_opt.trigger_load_balance();
        }

        // Update statistics
        self.aggregator.update();
    }

    /// Get optimization report
    pub fn get_report(&self) -> OptimizationReport {
        let stats = self.aggregator.get_stats();
        let policy = self.policy_engine.get_policy();

        OptimizationReport {
            policy: policy.name().to_string(),
            total_utilization: stats.total_utilization,
            per_cpu_utilization: stats.per_cpu_utilization.clone(),
            scheduler_latency_us: stats.scheduler_latency_us,
            cache_hit_rate: stats.cache_hit_rate,
            ipc: stats.ipc,
            branch_accuracy: stats.branch_accuracy,
            migrations: stats.migrations,
            context_switches_per_sec: stats.context_switches_per_sec,
        }
    }
}

/// Optimization report
#[derive(Debug, Clone)]
pub struct OptimizationReport {
    /// Current policy
    pub policy: String,
    /// Total CPU utilization (0-100)
    pub total_utilization: u32,
    /// Per-CPU utilization
    pub per_cpu_utilization: Vec<u32>,
    /// Scheduler latency (microseconds)
    pub scheduler_latency_us: u64,
    /// Cache hit rate (0-100)
    pub cache_hit_rate: f64,
    /// Instructions per cycle
    pub ipc: f64,
    /// Branch prediction accuracy (0-100)
    pub branch_accuracy: f64,
    /// Number of migrations
    pub migrations: u64,
    /// Context switches per second
    pub context_switches_per_sec: u64,
}

impl OptimizationReport {
    /// Format report as string
    pub fn format(&self) -> String {
        format!(
            "CPU Optimization Report:\n\
             Policy: {}\n\
             Total Utilization: {}%\n\
             Scheduler Latency: {} μs\n\
             Cache Hit Rate: {:.1}%\n\
             IPC: {:.2}\n\
             Branch Accuracy: {:.1}%\n\
             Migrations: {}\n\
             Context Switches/sec: {}\n",
            self.policy,
            self.total_utilization,
            self.scheduler_latency_us,
            self.cache_hit_rate,
            self.ipc,
            self.branch_accuracy,
            self.migrations,
            self.context_switches_per_sec
        )
    }
}

/// Global CPU optimization manager instance
static mut GLOBAL_CPU_OPT_MANAGER: Option<CpuOptimizationManager> = None;
static CPU_OPT_MANAGER_INIT: Mutex<bool> = Mutex::new(false);

/// Initialize global CPU optimization manager
pub fn init_cpu_optimization_manager(num_cpus: u32, num_numa_nodes: u32) {
    let mut is_init = CPU_OPT_MANAGER_INIT.lock();
    if *is_init {
        return;
    }

    let mut manager = CpuOptimizationManager::new();
    manager.init(num_cpus, num_numa_nodes);

    unsafe {
        GLOBAL_CPU_OPT_MANAGER = Some(manager);
    }
    *is_init = true;

    log_info!("Global CPU optimization manager initialized");
}

/// Get global CPU optimization manager
pub fn get_cpu_optimization_manager() -> Option<&'static CpuOptimizationManager> {
    unsafe { GLOBAL_CPU_OPT_MANAGER.as_ref() }
}

/// Run optimization pass (convenience function)
pub fn run_optimization() {
    if let Some(manager) = get_cpu_optimization_manager() {
        manager.optimize();
    }
}

/// Get optimization report (convenience function)
pub fn get_optimization_report() -> Option<OptimizationReport> {
    if let Some(manager) = get_cpu_optimization_manager() {
        Some(manager.get_report())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_event_type() {
        assert_eq!(PerfEventType::CpuCycles.name(), "cpu-cycles");
        assert!(PerfEventType::CacheMisses.is_cache_event());
    }

    #[test]
    fn test_perf_counter_value() {
        let value = PerfCounterValue::new(100);
        assert_eq!(value.value, 100);
        assert_eq!(value.scaled(), 100);
    }

    #[test]
    fn test_optimization_policy() {
        let policy = OptimizationPolicy::Performance;
        assert!(policy.is_performance_focused());
        assert!(!policy.is_power_focused());
        assert_eq!(policy.name(), "performance");
    }

    #[test]
    fn test_aggregated_stats() {
        let stats = AggregatedCpuStats::new(4);
        stats.per_cpu_utilization = vec![25, 50, 75, 100];
        assert_eq!(stats.average_utilization(), 62);
    }

    #[test]
    fn test_optimization_report() {
        let report = OptimizationReport {
            policy: "performance".to_string(),
            total_utilization: 75,
            per_cpu_utilization: vec![70, 80],
            scheduler_latency_us: 50,
            cache_hit_rate: 95.0,
            ipc: 1.5,
            branch_accuracy: 97.0,
            migrations: 10,
            context_switches_per_sec: 1000,
        };

        let formatted = report.format();
        assert!(formatted.contains("performance"));
        assert!(formatted.contains("75%"));
    }
}
