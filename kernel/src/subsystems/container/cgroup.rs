#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Cgroup (Control Group) Resource Limits
//!
//! This module implements cgroup v2 for container resource management:
//! - CPU limits
//! - Memory limits
//! - I/O limits
//! - Device access limits
//!
//! Features:
//! - cgroup v2 hierarchy
//! - Per-cgroup resource controllers
//! - Hierarchical cgroups
//! - Cgroup statistics
//! - Dynamic limit updates

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::collections::BTreeSet;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// Cgroup Constants
// ============================================================================

/// Maximum cgroup depth (hierarchical nesting)
pub const MAX_CGROUP_DEPTH: usize = 32;

/// Maximum cgroup name length
pub const MAX_CGROUP_NAME_LENGTH: usize = 256;

/// Default cgroup update interval (milliseconds)
pub const DEFAULT_UPDATE_INTERVAL_MS: u64 = 100;

// ============================================================================
// Cgroup Controller Types
// ============================================================================

/// Cgroup controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CgroupController {
    /// CPU controller
    Cpu,
    
    /// Memory controller
    Memory,
    
    /// I/O controller
    Io,
    
    /// CPUSET controller
    Cpuset,
    
    /// Device controller
    Devices,
    
    /// Freezer controller
    Freezer,
    
    /// PIDs controller
    Pids,
    
    /// RDMA controller
    Rdma,
    
    /// Net controller (v2)
    Net,
    
    /// Perf controller
    Perf,
    
    /// CPU controller (legacy)
    Cpuacct,
}

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    /// cgroup v1 (legacy)
    V1,
    
    /// cgroup v2 (current)
    V2,
}

// ============================================================================
// CPU Controller
// ============================================================================

/// CPU controller configuration
#[derive(Debug, Clone, Copy)]
pub struct CpuControllerConfig {
    /// CPU shares (relative CPU allocation)
    pub cpu_shares: u32,
    
    /// CPU quota (microseconds per period)
    pub cpu_quota: i64,
    
    /// CPU period (microseconds)
    pub cpu_period: u32,
    
    /// CPU real-time runtime (microseconds)
    pub cpu_rt_runtime: u64,
    
    /// CPU real-time period (microseconds)
    pub cpu_rt_period: u64,
    
    /// CPUs allowed (bitmask)
    pub cpus_allowed: u64,
    
    /// Maximum number of CPUs
    pub max_cpus: u32,
    
    /// Number of CPU cores available
    pub cpu_cores: u32,
}

impl Default for CpuControllerConfig {
    fn default() -> Self {
        Self {
            cpu_shares: 1024,
            cpu_quota: -1, // Unlimited
            cpu_period: 100_000, // 100ms
            cpu_rt_runtime: 0,
            cpu_rt_period: 1_000_000, // 1s
            cpus_allowed: !0u64,
            max_cpus: 0,
            cpu_cores: 0,
        }
    }
}

/// CPU controller statistics
#[derive(Debug, Clone, Copy)]
pub struct CpuControllerStats {
    /// CPU time used (nanoseconds)
    pub cpu_time_ns: u64,
    
    /// CPU time used per CPU
    pub cpu_time_per_cpu: Vec<u64>,
    
    /// Number of context switches
    pub nr_context_switches: u64,
    
    /// Number of voluntary context switches
    pub nr_voluntary_context_switches: u64,
    
    /// Number of involuntary context switches
    pub nr_involuntary_context_switches: u64,
    
    /// Number of processes in cgroup
    pub nr_processes: usize,
    
    /// Number of threads in cgroup
    pub nr_threads: usize,
    
    /// Throttling count (times cgroup was throttled)
    pub nr_throttled: u64,
    
    /// Throttled time (nanoseconds)
    pub throttled_ns: u64,
}

// ============================================================================
// Memory Controller
// ============================================================================

/// Memory controller configuration
#[derive(Debug, Clone, Copy)]
pub struct MemoryControllerConfig {
    /// Memory limit (bytes)
    pub memory_limit: u64,
    
    /// Memory soft limit (bytes)
    pub memory_soft_limit: u64,
    
    /// Swap limit (bytes)
    pub memory_swap_limit: u64,
    
    /// Memory reservation (bytes)
    pub memory_reservation: u64,
    
    /// OOM (Out of Memory) control enabled
    pub oom_control_enabled: bool,
    
    /// OOM kill disable
    pub oom_kill_disable: bool,
    
    /// Memory swappiness (0-100)
    pub swappiness: u32,
    
    /// Max number of memory pages to reclaim
    pub max_reclaim_pages: u32,
}

impl Default for MemoryControllerConfig {
    fn default() -> Self {
        Self {
            memory_limit: !0u64, // Unlimited
            memory_soft_limit: !0u64,
            memory_swap_limit: !0u64,
            memory_reservation: 0,
            oom_control_enabled: true,
            oom_kill_disable: false,
            swappiness: 60,
            max_reclaim_pages: 0,
        }
    }
}

/// Memory controller statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryControllerStats {
    /// Memory used (bytes)
    pub memory_usage: u64,
    
    /// Memory max usage (bytes)
    pub memory_max_usage: u64,
    
    /// Memory limit (bytes)
    pub memory_limit: u64,
    
    /// Swap used (bytes)
    pub swap_usage: u64,
    
    /// Swap limit (bytes)
    pub swap_limit: u64,
    
    /// Cache usage (bytes)
    pub cache_usage: u64,
    
    /// RSS (Resident Set Size) (bytes)
    pub rss_usage: u64,
    
    /// File pages (bytes)
    pub file_usage: u64,
    
    /// Number of pages faulted
    pgfault: u64,
    
    /// Number of major page faults
    pgmajfault: u64,
    
    /// Number of OOM kills
    num_oom_kills: u64,
    
    /// Number of times memory limit was reached
    num_memory_limit_hits: u64,
    
    /// Number of times swap limit was reached
    num_swap_limit_hits: u64,
}

// ============================================================================
// I/O Controller
// ============================================================================

/// I/O controller configuration
#[derive(Debug, Clone, Copy)]
pub struct IoControllerConfig {
    /// I/O bandwidth limit (bytes per second)
    pub io_bandwidth_limit: u64,
    
    /// I/O operations limit (operations per second)
    pub io_ops_limit: u64,
    
    /// I/O read bandwidth limit (bytes per second)
    pub read_bandwidth_limit: u64,
    
    /// I/O read operations limit (operations per second)
    pub read_ops_limit: u64,
    
    /// I/O write bandwidth limit (bytes per second)
    pub write_bandwidth_limit: u64,
    
    /// I/O write operations limit (operations per second)
    pub write_ops_limit: u64,
    
    /// I/O weight (relative priority, 1-10000)
    pub io_weight: u32,
    
    /// I/O read weight
    pub read_weight: u32,
    
    /// I/O write weight
    pub write_weight: u32,
}

impl Default for IoControllerConfig {
    fn default() -> Self {
        Self {
            io_bandwidth_limit: !0u64, // Unlimited
            io_ops_limit: !0u64,
            read_bandwidth_limit: !0u64,
            read_ops_limit: !0u64,
            write_bandwidth_limit: !0u64,
            write_ops_limit: !0u64,
            io_weight: 100,
            read_weight: 100,
            write_weight: 100,
        }
    }
}

/// I/O controller statistics
#[derive(Debug, Clone, Copy)]
pub struct IoControllerStats {
    /// Bytes read
    pub read_bytes: u64,
    
    /// Bytes written
    pub write_bytes: u64,
    
    /// Read operations
    pub read_ios: u64,
    
    /// Write operations
    pub write_ios: u64,
    
    /// Time spent reading (milliseconds)
    pub read_time_ms: u64,
    
    /// Time spent writing (milliseconds)
    pub write_time_ms: u64,
    
    /// Time spent syncing (milliseconds)
    pub sync_time_ms: u64,
    
    /// Number of I/O throttles
    nr_throttles: u64,
    
    /// I/O bandwidth (bytes per second)
    pub io_bandwidth: u64,
    
    /// I/O operations per second
    pub io_ops: u64,
}

// ============================================================================
// Cgroup
// ============================================================================

/// Container cgroup
#[derive(Debug, Clone)]
pub struct Cgroup {
    /// Cgroup ID
    pub cgroup_id: u32,
    
    /// Cgroup name (path-like)
    pub name: String,
    
    /// Cgroup version
    pub version: CgroupVersion,
    
    /// Parent cgroup (for hierarchical cgroups)
    pub parent_cgroup: Option<u32>,
    
    /// Child cgroups
    pub child_cgroups: Mutex<BTreeSet<u32>>,
    
    /// Depth in hierarchy
    pub depth: usize,
    
    /// Enabled controllers
    pub enabled_controllers: Mutex<BTreeSet<CgroupController>>,
    
    /// CPU controller configuration
    pub cpu_config: Mutex<Option<CpuControllerConfig>>,
    
    /// CPU controller statistics
    pub cpu_stats: Mutex<CpuControllerStats>,
    
    /// Memory controller configuration
    pub memory_config: Mutex<Option<MemoryControllerConfig>>,
    
    /// Memory controller statistics
    pub memory_stats: Mutex<MemoryControllerStats>,
    
    /// I/O controller configuration
    pub io_config: Mutex<Option<IoControllerConfig>>,
    
    /// I/O controller statistics
    pub io_stats: Mutex<IoControllerStats>,
    
    /// Processes in cgroup
    pub processes: Mutex<BTreeSet<usize>>,
    
    /// Number of processes
    pub num_processes: AtomicUsize,
    
    /// Cgroup is active
    pub active: AtomicBool,
    
    /// Cgroup statistics
    pub stats: Mutex<CgroupStats>,
    
    /// Last update time (for periodic updates)
    pub last_update: AtomicU64,
}

/// Cgroup statistics
#[derive(Debug, Clone, Copy)]
pub struct CgroupStats {
    /// Total processes
    pub total_processes: usize,
    
    /// Total children
    pub total_children: usize,
    
    /// Total memory used (bytes)
    pub total_memory_usage: u64,
    
    /// Total CPU time (nanoseconds)
    pub total_cpu_time_ns: u64,
    
    /// Number of limit violations
    pub limit_violations: u64,
    
    /// Cgroup creation time
    pub created_at: u64,
    
    /// Cgroup last activity time
    pub last_activity: u64,
}

impl Default for CgroupStats {
    fn default() -> Self {
        Self {
            total_processes: 0,
            total_children: 0,
            total_memory_usage: 0,
            total_cpu_time_ns: 0,
            limit_violations: 0,
            created_at: crate::subsystems::time::timestamp_nanos(),
            last_activity: crate::subsystems::time::timestamp_nanos(),
        }
    }
}

impl Cgroup {
    /// Create new cgroup
    pub fn new(cgroup_id: u32, name: String, version: CgroupVersion,
               parent: Option<u32>) -> Self {
        
        let depth = if parent.is_some() {
            // In real implementation, would get parent depth
            1
        } else {
            0
        };
        
        Self {
            cgroup_id,
            name,
            version,
            parent_cgroup: parent,
            child_cgroups: Mutex::new(BTreeSet::new()),
            depth,
            enabled_controllers: Mutex::new(BTreeSet::new()),
            cpu_config: Mutex::new(None),
            cpu_stats: Mutex::new(CpuControllerStats {
                cpu_time_ns: 0,
                cpu_time_per_cpu: Vec::new(),
                nr_context_switches: 0,
                nr_voluntary_context_switches: 0,
                nr_involuntary_context_switches: 0,
                nr_processes: 0,
                nr_threads: 0,
                nr_throttled: 0,
                throttled_ns: 0,
            }),
            memory_config: Mutex::new(None),
            memory_stats: Mutex::new(MemoryControllerStats {
                memory_usage: 0,
                memory_max_usage: 0,
                memory_limit: !0u64,
                swap_usage: 0,
                swap_limit: !0u64,
                cache_usage: 0,
                rss_usage: 0,
                file_usage: 0,
                pgfault: 0,
                pgmajfault: 0,
                num_oom_kills: 0,
                num_memory_limit_hits: 0,
                num_swap_limit_hits: 0,
            }),
            io_config: Mutex::new(None),
            io_stats: Mutex::new(IoControllerStats {
                read_bytes: 0,
                write_bytes: 0,
                read_ios: 0,
                write_ios: 0,
                read_time_ms: 0,
                write_time_ms: 0,
                sync_time_ms: 0,
                nr_throttles: 0,
                io_bandwidth: 0,
                io_ops: 0,
            }),
            processes: Mutex::new(BTreeSet::new()),
            num_processes: AtomicUsize::new(0),
            active: AtomicBool::new(true),
            stats: Mutex::new(CgroupStats::default()),
            last_update: AtomicU64::new(crate::subsystems::time::timestamp_nanos()),
        }
    }
    
    /// Enable controller
    pub fn enable_controller(&self, controller: CgroupController) 
        -> Result<(), CgroupError> {
        
        let mut controllers = self.enabled_controllers.lock();
        
        if controllers.contains(&controller) {
            return Err(CgroupError::ControllerAlreadyEnabled {
                controller,
                cgroup_id: self.cgroup_id,
            });
        }
        
        controllers.insert(controller);
        
        crate::println!("[cgroup] Enabled controller {:?} in cgroup {}",
                        controller, self.cgroup_id);
        
        Ok(())
    }
    
    /// Disable controller
    pub fn disable_controller(&self, controller: CgroupController) 
        -> Result<(), CgroupError> {
        
        let mut controllers = self.enabled_controllers.lock();
        
        if !controllers.remove(&controller) {
            return Err(CgroupError::ControllerNotEnabled {
                controller,
                cgroup_id: self.cgroup_id,
            });
        }
        
        crate::println!("[cgroup] Disabled controller {:?} in cgroup {}",
                        controller, self.cgroup_id);
        
        Ok(())
    }
    
    /// Set CPU controller configuration
    pub fn set_cpu_config(&self, config: CpuControllerConfig) 
        -> Result<(), CgroupError> {
        
        let cpu_enabled = self.enabled_controllers.lock().contains(&CgroupController::Cpu);
        
        if !cpu_enabled {
            return Err(CgroupError::ControllerNotEnabled {
                controller: CgroupController::Cpu,
                cgroup_id: self.cgroup_id,
            });
        }
        
        *self.cpu_config.lock() = Some(config);
        
        crate::println!("[cgroup] Set CPU config in cgroup {}", self.cgroup_id);
        
        Ok(())
    }
    
    /// Set memory controller configuration
    pub fn set_memory_config(&self, config: MemoryControllerConfig) 
        -> Result<(), CgroupError> {
        
        let mem_enabled = self.enabled_controllers.lock().contains(&CgroupController::Memory);
        
        if !mem_enabled {
            return Err(CgroupError::ControllerNotEnabled {
                controller: CgroupController::Memory,
                cgroup_id: self.cgroup_id,
            });
        }
        
        *self.memory_config.lock() = Some(config);
        
        crate::println!("[cgroup] Set memory config in cgroup {}", self.cgroup_id);
        
        Ok(())
    }
    
    /// Set I/O controller configuration
    pub fn set_io_config(&self, config: IoControllerConfig) 
        -> Result<(), CgroupError> {
        
        let io_enabled = self.enabled_controllers.lock().contains(&CgroupController::Io);
        
        if !io_enabled {
            return Err(CgroupError::ControllerNotEnabled {
                controller: CgroupController::Io,
                cgroup_id: self.cgroup_id,
            });
        }
        
        *self.io_config.lock() = Some(config);
        
        crate::println!("[cgroup] Set I/O config in cgroup {}", self.cgroup_id);
        
        Ok(())
    }
    
    /// Add process to cgroup
    pub fn add_process(&self, pid: usize) -> Result<(), CgroupError> {
        let mut processes = self.processes.lock();
        
        if processes.contains(&pid) {
            return Err(CgroupError::ProcessAlreadyInCgroup {
                pid,
                cgroup_id: self.cgroup_id,
            });
        }
        
        processes.insert(pid);
        self.num_processes.fetch_add(1, Ordering::Relaxed);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_processes = processes.len();
        stats.last_activity = crate::subsystems::time::timestamp_nanos();
        
        crate::println!("[cgroup] Added process {} to cgroup {}",
                        pid, self.cgroup_id);
        
        Ok(())
    }
    
    /// Remove process from cgroup
    pub fn remove_process(&self, pid: usize) -> Result<(), CgroupError> {
        let mut processes = self.processes.lock();
        
        if !processes.remove(&pid) {
            return Err(CgroupError::ProcessNotInCgroup {
                pid,
                cgroup_id: self.cgroup_id,
            });
        }
        
        self.num_processes.fetch_sub(1, Ordering::Relaxed);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_processes = processes.len();
        
        crate::println!("[cgroup] Removed process {} from cgroup {}",
                        pid, self.cgroup_id);
        
        Ok(())
    }
    
    /// Add child cgroup
    pub fn add_child_cgroup(&self, child_id: u32) -> Result<(), CgroupError> {
        let mut children = self.child_cgroups.lock();
        
        if children.contains(&child_id) {
            return Err(CgroupError::ChildAlreadyExists {
                child_id,
                parent_id: self.cgroup_id,
            });
        }
        
        // Check depth limit
        if self.depth >= MAX_CGROUP_DEPTH {
            return Err(CgroupError::MaxDepthExceeded { depth: self.depth });
        }
        
        children.insert(child_id);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_children = children.len();
        
        crate::println!("[cgroup] Added child cgroup {} to cgroup {}",
                        child_id, self.cgroup_id);
        
        Ok(())
    }
    
    /// Get all processes in cgroup
    pub fn get_processes(&self) -> Vec<usize> {
        let processes = self.processes.lock();
        processes.iter().cloned().collect()
    }
    
    /// Check if cgroup is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
    
    /// Activate cgroup
    pub fn activate(&self) {
        self.active.store(true, Ordering::Release);
        crate::println!("[cgroup] Activated cgroup {}", self.cgroup_id);
    }
    
    /// Deactivate cgroup
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
        crate::println!("[cgroup] Deactivated cgroup {}", self.cgroup_id);
    }
    
    /// Get cgroup statistics
    pub fn get_stats(&self) -> CgroupStats {
        let mut stats = self.stats.lock();
        
        stats.total_processes = self.num_processes.load(Ordering::Relaxed);
        stats.last_activity = crate::subsystems::time::timestamp_nanos();
        
        *stats
    }
}

/// Cgroup error
#[derive(Debug, Clone)]
pub enum CgroupError {
    /// Controller already enabled
    ControllerAlreadyEnabled {
        controller: CgroupController,
        cgroup_id: u32,
    },
    
    /// Controller not enabled
    ControllerNotEnabled {
        controller: CgroupController,
        cgroup_id: u32,
    },
    
    /// Process already in cgroup
    ProcessAlreadyInCgroup {
        pid: usize,
        cgroup_id: u32,
    },
    
    /// Process not in cgroup
    ProcessNotInCgroup {
        pid: usize,
        cgroup_id: u32,
    },
    
    /// Child cgroup already exists
    ChildAlreadyExists {
        child_id: u32,
        parent_id: u32,
    },
    
    /// Max depth exceeded
    MaxDepthExceeded {
        depth: usize,
    },
    
    /// Cgroup not found
    CgroupNotFound {
        cgroup_id: u32,
    },
    
    /// Invalid parameters
    InvalidParameters,
    
    /// Configuration failed
    ConfigurationFailed {
        reason: String,
    },
}

// ============================================================================
// Cgroup Manager
// ============================================================================

/// Cgroup manager
pub struct CgroupManager {
    /// All cgroups
    pub cgroups: Mutex<BTreeMap<u32, Arc<Cgroup>>>,
    
    /// Cgroups by controller
    pub cgroups_by_controller: Mutex<BTreeMap<CgroupController, BTreeSet<u32>>>,
    
    /// Root cgroups (no parent)
    pub root_cgroups: Mutex<BTreeSet<u32>>,
    
    /// Next cgroup ID
    pub next_cgroup_id: AtomicU32,
    
    /// Total cgroups
    pub total_cgroups: AtomicUsize,
    
    /// Active cgroups
    pub active_cgroups: AtomicUsize,
    
    /// Manager statistics
    pub stats: Mutex<CgroupManagerStats>,
}

/// Cgroup manager statistics
#[derive(Debug, Clone, Copy)]
pub struct CgroupManagerStats {
    pub total_cgroups: usize,
    pub active_cgroups: usize,
    pub total_processes: usize,
    pub total_memory_usage: u64,
    pub total_cpu_time_ns: u64,
}

impl Default for CgroupManagerStats {
    fn default() -> Self {
        Self {
            total_cgroups: 0,
            active_cgroups: 0,
            total_processes: 0,
            total_memory_usage: 0,
            total_cpu_time_ns: 0,
        }
    }
}

impl CgroupManager {
    /// Create new cgroup manager
    pub fn new() -> Self {
        Self {
            cgroups: Mutex::new(BTreeMap::new()),
            cgroups_by_controller: Mutex::new(BTreeMap::new()),
            root_cgroups: Mutex::new(BTreeSet::new()),
            next_cgroup_id: AtomicU32::new(1),
            total_cgroups: AtomicUsize::new(0),
            active_cgroups: AtomicUsize::new(0),
            stats: Mutex::new(CgroupManagerStats::default()),
        }
    }
    
    /// Create cgroup
    pub fn create_cgroup(&self, name: String, version: CgroupVersion,
                       parent: Option<u32>) -> Result<u32, CgroupError> {
        
        let cgroup_id = self.next_cgroup_id.fetch_add(1, Ordering::Relaxed);
        
        let cgroup = Arc::new(Cgroup::new(cgroup_id, name, version, parent));
        
        let mut cgroups = self.cgroups.lock();
        cgroups.insert(cgroup_id, cgroup);
        self.total_cgroups.fetch_add(1, Ordering::Relaxed);
        
        // Update root cgroups
        if parent.is_none() {
            let mut roots = self.root_cgroups.lock();
            roots.insert(cgroup_id);
        }
        
        crate::println!("[cgroup] Created cgroup {} ({})", cgroup_id, name);
        
        Ok(cgroup_id)
    }
    
    /// Get cgroup by ID
    pub fn get_cgroup(&self, cgroup_id: u32) -> Option<Arc<Cgroup>> {
        let cgroups = self.cgroups.lock();
        cgroups.get(&cgroup_id).cloned()
    }
    
    /// Delete cgroup
    pub fn delete_cgroup(&self, cgroup_id: u32) -> Result<(), CgroupError> {
        let mut cgroups = self.cgroups.lock();
        
        if let Some(cgroup) = cgroups.remove(&cgroup_id) {
            // Update root cgroups
            let mut roots = self.root_cgroups.lock();
            roots.remove(&cgroup_id);
            
            crate::println!("[cgroup] Deleted cgroup {}", cgroup_id);
            
            Ok(())
        } else {
            Err(CgroupError::CgroupNotFound { cgroup_id })
        }
    }
    
    /// Get all cgroups
    pub fn get_all_cgroups(&self) -> Vec<Arc<Cgroup>> {
        let cgroups = self.cgroups.lock();
        cgroups.values().cloned().collect()
    }
    
    /// Get root cgroups
    pub fn get_root_cgroups(&self) -> Vec<Arc<Cgroup>> {
        let roots = self.root_cgroups.lock();
        let cgroups = self.cgroups.lock();
        roots.iter().filter_map(|id| cgroups.get(id).cloned()).collect()
    }
    
    /// Get manager statistics
    pub fn get_stats(&self) -> CgroupManagerStats {
        let mut stats = self.stats.lock();
        
        stats.total_cgroups = self.total_cgroups.load(Ordering::Relaxed);
        stats.active_cgroups = self.active_cgroups.load(Ordering::Relaxed);
        
        let cgroups = self.cgroups.lock();
        
        let mut total_processes = 0usize;
        let mut total_memory = 0u64;
        
        for cgroup in cgroups.values() {
            let cgroup_stats = cgroup.get_stats();
            total_processes += cgroup_stats.total_processes;
            total_memory += cgroup_stats.total_memory_usage;
        }
        
        stats.total_processes = total_processes;
        stats.total_memory_usage = total_memory;
        
        *stats
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_controller_config() {
        let config = CpuControllerConfig::default();
        
        assert_eq!(config.cpu_shares, 1024);
        assert_eq!(config.cpu_quota, -1);
        assert_eq!(config.cpu_period, 100_000);
    }

    #[test]
    fn test_memory_controller_config() {
        let config = MemoryControllerConfig::default();
        
        assert_eq!(config.memory_limit, !0u64);
        assert_eq!(config.oom_control_enabled, true);
        assert_eq!(config.swappiness, 60);
    }

    #[test]
    fn test_io_controller_config() {
        let config = IoControllerConfig::default();
        
        assert_eq!(config.io_bandwidth_limit, !0u64);
        assert_eq!(config.io_weight, 100);
    }

    #[test]
    fn test_cgroup_creation() {
        let cgroup = Cgroup::new(
            1,
            String::from("test"),
            CgroupVersion::V2,
            None
        );
        
        assert_eq!(cgroup.cgroup_id, 1);
        assert_eq!(cgroup.name, "test");
        assert_eq!(cgroup.depth, 0);
    }

    #[test]
    fn test_cgroup_controllers() {
        let cgroup = Cgroup::new(
            1,
            String::from("test"),
            CgroupVersion::V2,
            None
        );
        
        cgroup.enable_controller(CgroupController::Cpu).unwrap();
        cgroup.enable_controller(CgroupController::Memory).unwrap();
        
        let config = CpuControllerConfig::default();
        cgroup.set_cpu_config(config).unwrap();
        
        let mem_config = MemoryControllerConfig::default();
        cgroup.set_memory_config(mem_config).unwrap();
    }

    #[test]
    fn test_cgroup_processes() {
        let cgroup = Cgroup::new(
            1,
            String::from("test"),
            CgroupVersion::V2,
            None
        );
        
        cgroup.add_process(100).unwrap();
        cgroup.add_process(101).unwrap();
        
        assert_eq!(cgroup.num_processes.load(Ordering::Relaxed), 2);
        
        let processes = cgroup.get_processes();
        assert_eq!(processes.len(), 2);
        assert!(processes.contains(&100));
        assert!(processes.contains(&101));
    }

    #[test]
    fn test_cgroup_manager() {
        let manager = CgroupManager::new();
        
        let cgroup_id = manager.create_cgroup(
            String::from("test"),
            CgroupVersion::V2,
            None
        ).unwrap();
        
        let cgroup = manager.get_cgroup(cgroup_id).unwrap();
        assert_eq!(cgroup.cgroup_id, cgroup_id);
        
        let stats = manager.get_stats();
        assert_eq!(stats.total_cgroups, 1);
    }
}
