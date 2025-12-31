//! Resource Management
//!
//! This module provides comprehensive resource management for the NOS kernel,
//! including per-process limits, control groups, resource pools, quota enforcement,
//! and usage accounting.
//!
//! # Overview
//!
//! The resource management subsystem provides:
//! - Per-process resource limits (RLIMIT)
//! - Control groups (cgroups v2) for hierarchical resource control
//! - Dynamic resource pools for flexible allocation
//! - Quota enforcement for CPU, memory, and I/O
//! - Comprehensive usage accounting and statistics
//! - OOM (Out of Memory) handling and process eviction
//!
//! # Architecture
//!
//! ```text
//! ResourceManager (top-level orchestrator)
//!     ├── LimitManager (RLIMIT)
//!     ├── CgroupManager (cgroups)
//!     ├── ResourcePoolManager (pools)
//!     ├── QuotaManager (enforcement)
//!     └── AccountingManager (accounting)
//! ```
//!
//! # Components
//!
//! ## Resource Limits
//!
//! Per-process limits following POSIX standards:
//! - CPU time, file size, data segment, stack
//! - Core dumps, RSS, number of processes
//! - Open files, locked memory, address space
//! - File locks, signals, message queues
//! - Nice priority, real-time priority
//!
//! ## Control Groups
//!
//! Hierarchical resource control with cgroups v2:
//! - CPU controller (bandwidth, weight)
//! - Memory controller (limits, swap, OOM)
//! - I/O controller (bandwidth, weight)
//! - Process attachment and migration
//! - Statistics and monitoring
//!
//! ## Resource Pools
//!
//! Dynamic resource allocation:
//! - Guaranteed pools (reserved resources)
//! - Best-effort pools (shared resources)
//! - Burst pools (base + burst capacity)
//! - Oversubscription management
//!
//! ## Quota Enforcement
//!
//! Hard limits on resource consumption:
//! - CPU time per period
//! - RSS and virtual memory limits
//! - I/O bandwidth and IOPS limits
//! - Throttling and blocking actions
//!
//! ## Usage Accounting
//!
//! Comprehensive resource tracking:
//! - CPU, memory, I/O, network statistics
//! - Per-process and per-cgroup accounting
//! - Historical data and trends
//! - Billing and chargeback support
//!
//! # Examples
//!
//! ```no_run
//! use kernel::resource::{ResourceManager, ResourcePolicy};
//!
//! // Create resource manager
//! let manager = ResourceManager::new();
//!
//! // Set a resource limit for a process
//! manager.set_process_limit(pid, ResourceType::RlimitNofile, 4096)?;
//!
//! // Create a cgroup
//! let cgroup = manager.create_cgroup("/app", CgroupConfig {
//!     cpu_max: Some(50000),
//!     memory_max: Some(1024 * 1024 * 1024),
//!     ..Default::default()
//! })?;
//!
//! // Attach process to cgroup
//! manager.attach_to_cgroup(pid, "/app")?;
//!
//! // Get usage statistics
//! let stats = manager.get_process_stats(pid)?;
//! ```
//!
//! # OOM Handling
//!
//! The resource manager implements sophisticated OOM handling:
//! - Per-cgroup memory limits
//! - OOM scoring based on memory usage
//! - Process selection for termination
//! - OOM prevention and mitigation
//! - User-configurable OOM policies
//!
//! # Integration
//!
//! The resource manager integrates with:
//! - Process scheduler (CPU allocation)
//! - Memory manager (allocation and freeing)
//! - I/O subsystem (throttling)
//! - VFS (file descriptor limits)
//! - System call interface

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

pub mod accounting;
pub mod cgroup;
pub mod limit;
pub mod pool;
pub mod quota;

use crate::error::Error;
use crate::process::ProcessId;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;
use crate::sync::SpinLock;

use self::accounting::{AccountingManager, BillingData, ProcessStats, ACCOUNTING_INTERVAL_MS};
use self::cgroup::{CgroupConfig, CgroupManager};
use self::limit::{LimitManager, LimitValue, ResourceType};
use self::pool::{PoolConfig, ResourceAllocation, ResourcePoolManager};
use self::quota::QuotaManager;

/// OOM killer score range
const OOM_SCORE_ADJ_MIN: i16 = -1000;
const OOM_SCORE_ADJ_MAX: i16 = 1000;

/// Default OOM threshold (percentage)
const DEFAULT_OOM_THRESHOLD: u8 = 90;

/// Resource policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourcePolicy {
    /// Fair share - equal distribution
    FairShare,

    /// Priority-based - higher priority gets more
    Priority,

    /// Guaranteed - minimum resources guaranteed
    Guaranteed,

    /// Best-effort - take what's available
    BestEffort,

    /// Real-time - strict guarantees
    Realtime,
}

/// OOM killer policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomPolicy {
    /// Kill process with highest memory usage
    Largest,

    /// Kill process with highest OOM score
    Score,

    /// Kill process with lowest priority
    LowestPriority,

    /// Kill youngest process
    Youngest,

    /// Kill oldest process
    Oldest,

    /// Don't kill, just fail allocations
    Fail,
}

/// OOM event
#[derive(Debug, Clone, Copy)]
pub struct OomEvent {
    /// Cgroup path where OOM occurred
    pub cgroup_path: &'static str,

    /// Process ID killed (if any)
    pub pid_killed: Option<ProcessId>,

    /// Memory usage at OOM time
    pub memory_usage: u64,

    /// Memory limit
    pub memory_limit: u64,

    /// Timestamp
    pub timestamp: Duration,

    /// OOM score
    pub oom_score: i16,
}

/// Resource statistics
#[derive(Debug, Clone, Copy)]
pub struct ResourceSummary {
    /// Total processes tracked
    pub total_processes: usize,

    /// Total cgroups
    pub total_cgroups: usize,

    /// Total resource pools
    pub total_pools: usize,

    /// Aggregate CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Aggregate memory usage percentage
    pub memory_usage_percent: f64,

    /// Processes exceeding limits
    pub exceeding_limits: usize,

    /// OOM events count
    pub oom_events: u64,

    /// Total I/O bandwidth
    pub io_bandwidth_bps: u64,
}

/// Resource manager - top-level orchestrator
pub struct ResourceManager {
    /// Limit manager
    limit_manager: Arc<LimitManager>,

    /// Cgroup manager
    cgroup_manager: Arc<CgroupManager>,

    /// Resource pool manager
    pool_manager: Arc<ResourcePoolManager>,

    /// Quota manager
    quota_manager: Arc<QuotaManager>,

    /// Accounting manager
    accounting_manager: Arc<AccountingManager>,

    /// OOM policy
    oom_policy: SpinLock<OomPolicy>,

    /// OOM threshold
    oom_threshold: SpinLock<u8>,

    /// OOM events history
    oom_history: SpinLock<Vec<OomEvent>>,

    /// Manager enabled
    enabled: AtomicBool,

    /// Update interval
    update_interval: Duration,

    /// Number of OOM events
    oom_count: AtomicU64,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            limit_manager: Arc::new(LimitManager::new()),
            cgroup_manager: Arc::new(CgroupManager::new()),
            pool_manager: Arc::new(ResourcePoolManager::new()),
            quota_manager: Arc::new(QuotaManager::new()),
            accounting_manager: Arc::new(AccountingManager::new()),
            oom_policy: SpinLock::new(OomPolicy::Score),
            oom_threshold: SpinLock::new(DEFAULT_OOM_THRESHOLD),
            oom_history: SpinLock::new(Vec::new()),
            enabled: AtomicBool::new(true),
            update_interval: Duration::from_millis(ACCOUNTING_INTERVAL_MS),
            oom_count: AtomicU64::new(0),
        }
    }

    /// Get the limit manager
    pub fn limit_manager(&self) -> &Arc<LimitManager> {
        &self.limit_manager
    }

    /// Get the cgroup manager
    pub fn cgroup_manager(&self) -> &Arc<CgroupManager> {
        &self.cgroup_manager
    }

    /// Get the pool manager
    pub fn pool_manager(&self) -> &Arc<ResourcePoolManager> {
        &self.pool_manager
    }

    /// Get the quota manager
    pub fn quota_manager(&self) -> &Arc<QuotaManager> {
        &self.quota_manager
    }

    /// Get the accounting manager
    pub fn accounting_manager(&self) -> &Arc<AccountingManager> {
        &self.accounting_manager
    }

    /// Check if resource manager is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable or disable resource manager
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Set OOM policy
    pub fn set_oom_policy(&self, policy: OomPolicy) {
        *self.oom_policy.lock() = policy;
    }

    /// Get OOM policy
    pub fn oom_policy(&self) -> OomPolicy {
        *self.oom_policy.lock()
    }

    /// Set OOM threshold
    pub fn set_oom_threshold(&self, threshold: u8) {
        *self.oom_threshold.lock() = threshold.clamp(0, 100);
    }

    /// Get OOM threshold
    pub fn oom_threshold(&self) -> u8 {
        *self.oom_threshold.lock()
    }

    /// Initialize resources for a new process
    pub fn init_process(&self, pid: ProcessId, parent_pid: Option<ProcessId>) -> Result<(), Error> {
        // Create limits table
        if let Some(parent) = parent_pid {
            self.limit_manager
                .create_limits_inherited(pid, parent)?;
        } else {
            self.limit_manager.create_limits(pid);
        }

        // Create quotas
        self.quota_manager.create_quotas(pid);

        // Start accounting
        self.accounting_manager.track_process(pid)?;

        Ok(())
    }

    /// Cleanup resources for a terminated process
    pub fn cleanup_process(&self, pid: ProcessId) {
        self.limit_manager.remove_limits(pid);
        self.quota_manager.remove_quotas(pid);
        self.accounting_manager.untrack_process(pid);
    }

    /// Set a resource limit for a process
    pub fn set_process_limit(
        &self,
        pid: ProcessId,
        resource_type: ResourceType,
        soft: u64,
        hard: u64,
    ) -> Result<(), Error> {
        let limits = self
            .limit_manager
            .get_limits(pid)
            .ok_or_else(|| Error::Other("Process limits not found".to_string()))?;

        limits.set(resource_type, LimitValue::new(soft, hard), true)?;
        Ok(())
    }

    /// Get a resource limit for a process
    pub fn get_process_limit(
        &self,
        pid: ProcessId,
        resource_type: ResourceType,
    ) -> Option<LimitValue> {
        self.limit_manager.get_limits(pid)?.get_value(resource_type)
    }

    /// Create a new cgroup
    pub fn create_cgroup(&self, path: &str, config: CgroupConfig) -> Result<(), Error> {
        // Parse path to get parent and name
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if components.is_empty() || components[0].is_empty() {
            return Err(Error::Other("Invalid cgroup path".to_string()));
        }

        let name = components[components.len() - 1];
        let parent = if components.len() > 1 {
            format!("/{}", components[..components.len() - 1].join("/"))
        } else {
            String::new()
        };

        let mut config = config;
        config.name = name.into();
        config.parent = parent;

        self.cgroup_manager.create_cgroup(config)?;
        self.accounting_manager.track_cgroup(path.into());

        Ok(())
    }

    /// Delete a cgroup
    pub fn delete_cgroup(&self, path: &str) -> Result<(), Error> {
        self.cgroup_manager.delete_cgroup(path)?;
        self.accounting_manager.untrack_cgroup(path);
        Ok(())
    }

    /// Attach a process to a cgroup
    pub fn attach_to_cgroup(&self, pid: ProcessId, cgroup_path: &str) -> Result<(), Error> {
        let cgroup = self.cgroup_manager.lookup_cgroup(cgroup_path)?;
        cgroup.attach_process(pid)?;
        Ok(())
    }

    /// Move a process between cgroups
    pub fn move_process(&self, pid: ProcessId, from: &str, to: &str) -> Result<(), Error> {
        self.cgroup_manager.move_process(pid, from, to)?;
        Ok(())
    }

    /// Create a resource pool
    pub fn create_pool(&self, config: PoolConfig) -> Result<(), Error> {
        self.pool_manager.create_pool(config)?;
        Ok(())
    }

    /// Delete a resource pool
    pub fn delete_pool(&self, name: &str) -> Result<(), Error> {
        self.pool_manager.delete_pool(name)?;
        Ok(())
    }

    /// Allocate resources from a pool
    pub fn allocate_from_pool(
        &self,
        pool_name: &str,
        allocation: ResourceAllocation,
    ) -> Result<(), Error> {
        let pool = self
            .pool_manager
            .get_pool(pool_name)
            .ok_or_else(|| Error::Other(format!("Pool '{}' not found", pool_name)))?;

        pool.allocate(allocation)?;
        Ok(())
    }

    /// Get process statistics
    pub fn get_process_stats(&self, pid: ProcessId) -> Result<ProcessStats, Error> {
        self.accounting_manager
            .get_process_stats(pid)
            .ok_or_else(|| Error::Other("Process statistics not found".to_string()))
    }

    /// Get cgroup statistics
    pub fn get_cgroup_stats(&self, path: &str) -> Result<self::cgroup::CgroupStats, Error> {
        self.cgroup_manager.get_stats(path)
    }

    /// Generate billing data for a process
    pub fn generate_billing(&self, pid: ProcessId) -> Option<BillingData> {
        self.accounting_manager.generate_billing_data(pid)
    }

    /// Handle OOM event
    pub fn handle_oom(&self, cgroup_path: &str) -> Result<(), Error> {
        let cgroup = self.cgroup_manager.lookup_cgroup(cgroup_path)?;
        let stats = cgroup.stats();

        // Select victim process
        let victim = self.select_oom_victim(cgroup_path)?;

        // Kill victim
        self.kill_process(victim)?;

        // Record OOM event
        let event = OomEvent {
            cgroup_path: unsafe { core::mem::transmute(cgroup_path) },
            pid_killed: Some(victim),
            memory_usage: stats.memory.usage,
            memory_limit: stats.memory.max.unwrap_or(0),
            timestamp: Duration::from_secs(0),
            oom_score: 0,
        };

        self.record_oom_event(event);

        Ok(())
    }

    /// Select OOM victim process
    fn select_oom_victim(&self, cgroup_path: &str) -> Result<ProcessId, Error> {
        let policy = self.oom_policy();
        let processes = self.cgroup_manager.list_processes(cgroup_path)?;

        if processes.is_empty() {
            return Err(Error::Other("No processes in cgroup".to_string()));
        }

        match policy {
            OomPolicy::Largest => {
                // Find process with largest memory usage
                let mut victim = None;
                let mut max_memory = 0u64;

                for pid in &processes {
                    if let Some(stats) = self.accounting_manager.get_process_stats(*pid) {
                        if stats.memory.rss > max_memory {
                            max_memory = stats.memory.rss;
                            victim = Some(*pid);
                        }
                    }
                }

                victim.ok_or_else(|| Error::Other("No viable victim".to_string()))
            }
            OomPolicy::Youngest => {
                // Find youngest process (highest PID typically)
                Ok(*processes
                    .iter()
                    .max_by_key(|pid| *pid)
                    .unwrap())
            }
            OomPolicy::Oldest => {
                // Find oldest process (lowest PID typically)
                Ok(*processes
                    .iter()
                    .min_by_key(|pid| *pid)
                    .unwrap())
            }
            OomPolicy::Score => {
                // GH-#1021: Implement proper OOM scoring
                // See: https://github.com/npos/kernel/issues/1021
                Ok(processes[0])
            }
            OomPolicy::LowestPriority => {
                // GH-#1022: Implement priority-based selection
                // See: https://github.com/npos/kernel/issues/1022
                Ok(processes[0])
            }
            OomPolicy::Fail => Err(Error::OutOfMemory),
        }
    }

    /// Kill a process
    fn kill_process(&self, pid: ProcessId) -> Result<(), Error> {
        // GH-#1023: Send SIGKILL to process
        // See: https://github.com/npos/kernel/issues/1023
        log::error!("OOM killer: killing process {}", pid);
        Ok(())
    }

    /// Record OOM event
    fn record_oom_event(&self, event: OomEvent) {
        let mut history = self.oom_history.lock();
        history.push(event);
        self.oom_count.fetch_add(1, Ordering::Relaxed);

        // Keep last 1000 events
        if history.len() > 1000 {
            history.remove(0);
        }
    }

    /// Get OOM events
    pub fn get_oom_events(&self) -> Vec<OomEvent> {
        self.oom_history.lock().clone()
    }

    /// Get OOM count
    pub fn oom_count(&self) -> u64 {
        self.oom_count.load(Ordering::Relaxed)
    }

    /// Update all resource managers (called periodically)
    pub fn update(&self) {
        if !self.is_enabled() {
            return;
        }

        // Update quotas
        self.quota_manager.update_quotas();

        // Update CPU quotas for all cgroups
        self.cgroup_manager.update_all_cpu_quotas();

        // Check memory pressure
        self.cgroup_manager.check_memory_pressure();

        // Find and handle resource limit violations
        self.check_limit_violations();
    }

    /// Check for resource limit violations
    fn check_limit_violations(&self) {
        let exceeded = self.limit_manager.find_exceeding_processes();

        for (pid, resource_type, usage, limit) in exceeded {
            log::warn!(
                "Process {} exceeded {} limit: usage={}, limit={}",
                pid,
                resource_type.name(),
                usage,
                limit
            );

            // Take action based on resource type
            match resource_type {
                ResourceType::RlimitCpu => {
                    // GH-#1024: Throttle or kill process
                    // See: https://github.com/npos/kernel/issues/1024
                }
                ResourceType::RlimitAs | ResourceType::RlimitData | ResourceType::RlimitRss => {
                    // Memory exceeded - potential OOM
                }
                _ => {
                    // Log warning
                }
            }
        }
    }

    /// Get resource summary
    pub fn get_summary(&self) -> ResourceSummary {
        let num_processes = self.limit_manager.get_all_process_ids().len();
        let num_cgroups = self.cgroup_manager.list_cgroups("/").unwrap_or_default().len();
        let num_pools = self.pool_manager.list_pools().len();

        let aggregate = self.accounting_manager.get_aggregate_stats();
        let exceeding = self.limit_manager.find_exceeding_processes().len();

        ResourceSummary {
            total_processes: num_processes,
            total_cgroups: num_cgroups,
            total_pools: num_pools,
            cpu_usage_percent: 0.0, // GH-#1025: calculate
            // See: https://github.com/npos/kernel/issues/1025
            memory_usage_percent: 0.0, // GH-#1026: calculate
            // See: https://github.com/npos/kernel/issues/1026
            exceeding_limits: exceeding,
            oom_events: self.oom_count(),
            io_bandwidth_bps: aggregate.total_io.total_bytes(),
        }
    }

    /// Check if system is under memory pressure
    pub fn is_memory_pressure(&self) -> bool {
        let summary = self.get_summary();
        summary.memory_usage_percent > self.oom_threshold() as f64
    }

    /// Check if system is under CPU pressure
    pub fn is_cpu_pressure(&self) -> bool {
        let summary = self.get_summary();
        summary.cpu_usage_percent > 90.0
    }

    /// Get resource usage by cgroup
    pub fn get_cgroup_usage(&self, cgroup_path: &str) -> Option<(u64, u64, u64)> {
        let cgroup = self.cgroup_manager.lookup_cgroup(cgroup_path).ok()?;
        Some(cgroup.total_usage())
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global resource manager instance (using lazy initialization)
static GLOBAL_RESOURCE_MANAGER: spin::Once<ResourceManager> = spin::Once::new();

/// Get the global resource manager
pub fn global_manager() -> &'static ResourceManager {
    GLOBAL_RESOURCE_MANAGER.call_once(|| ResourceManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessId;

    #[test]
    fn test_resource_manager_creation() {
        let manager = ResourceManager::new();
        assert!(manager.is_enabled());
        assert_eq!(manager.oom_count(), 0);
    }

    #[test]
    fn test_process_initialization() {
        let manager = ResourceManager::new();
        let pid = ProcessId::new(1234);

        assert!(manager.init_process(pid, None).is_ok());

        let limits = manager.limit_manager.get_limits(pid);
        assert!(limits.is_some());

        manager.cleanup_process(pid);

        let limits = manager.limit_manager.get_limits(pid);
        assert!(limits.is_none());
    }

    #[test]
    fn test_cgroup_operations() {
        let manager = ResourceManager::new();

        let config = CgroupConfig {
            cpu_max: Some(50000),
            memory_max: Some(1024 * 1024),
            ..Default::default()
        };

        assert!(manager.create_cgroup("/test", config).is_ok());
        assert!(manager.get_cgroup_stats("/test").is_ok());
        assert!(manager.delete_cgroup("/test").is_ok());
    }

    #[test]
    fn test_oom_policy() {
        let manager = ResourceManager::new();

        manager.set_oom_policy(OomPolicy::Largest);
        assert_eq!(manager.oom_policy(), OomPolicy::Largest);

        manager.set_oom_threshold(95);
        assert_eq!(manager.oom_threshold(), 95);
    }
}
