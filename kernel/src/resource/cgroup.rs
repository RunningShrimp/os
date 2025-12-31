//! Control groups v2 implementation
//!
//! This module implements cgroup v2 (unified hierarchy) for organizing processes
//! and controlling their resource usage. Cgroups allow grouping processes and applying
//! resource constraints to entire groups.
//!
//! # Overview
//!
//! Control Groups (cgroups) provide:
//! - Resource limiting (CPU, memory, I/O)
//! - Prioritization and accounting
//! - Process control (freezing, killing)
//! - Hierarchical organization
//!
//! # Architecture
//!
//! ```text
//! CgroupRoot
//!     └── Cgroup (hierarchical tree)
//!         ├── Controllers (cpu, memory, io)
//!         ├── Processes (attached PIDs)
//!         ├── Statistics (usage metrics)
//!         └── Subtree (child cgroups)
//! ```
//!
//! # Controllers
//!
//! ## CPU Controller
//! - `cpu.max`: Maximum CPU bandwidth (e.g., "max 100000" for 100%)
//! - `cpu.weight`: CPU weight for scheduling (1-10000)
//! - `cpu.stat`: CPU usage statistics
//!
//! ## Memory Controller
//! - `memory.max`: Maximum memory usage
//! - `memory.swap_max`: Maximum swap usage
//! - `memory.stat`: Memory statistics
//! - `memory.events`: OOM and other events
//!
//! ## I/O Controller
//! - `io.max`: Maximum I/O bandwidth per device
//! - `io.weight": I/O weight (1-10000)
//! - `io.stat": I/O statistics
//!
//! # Examples
//!
//! ```no_run
//! use kernel::resource::cgroup::{CgroupManager, CgroupConfig};
//!
//! let manager = CgroupManager::new();
//!
//! // Create a new cgroup
//! let config = CgroupConfig {
//!     name: "myapp".into(),
//!     cpu_max: Some(50000), // 50% CPU
//!     memory_max: Some(1024 * 1024 * 1024), // 1 GB
//!     ..Default::default()
//! };
//!
//! let cgroup = manager.create_cgroup(config).unwrap();
//!
//! // Attach a process
//! cgroup.attach_process(1234);
//! ```
//!
//! # Hierarchy
//!
//! Cgroups form a tree structure where child cgroups inherit constraints from parents:
//! ```text
//! /
//! ├── system
//! │   ├── sshd
//! │   └── nginx
//! └── user
//!     ├── user1000
//!     └── user1001
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use crate::error::Error;
use crate::process::ProcessId;
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

/// Default CPU weight for fair scheduling
pub const DEFAULT_CPU_WEIGHT: u64 = 100;

/// Maximum CPU weight
pub const MAX_CPU_WEIGHT: u64 = 10000;

/// Minimum CPU weight
pub const MIN_CPU_WEIGHT: u64 = 1;

/// Default I/O weight
pub const DEFAULT_IO_WEIGHT: u64 = 100;

/// Maximum I/O weight
pub const MAX_IO_WEIGHT: u64 = 10000;

/// CPU quota period in microseconds (100ms)
pub const CPU_QUOTA_PERIOD: u64 = 100_000;

/// Maximum number of nested cgroup levels
pub const MAX_CGROUP_DEPTH: usize = 32;

/// Maximum number of processes per cgroup
pub const MAX_PROCESSES_PER_CGROUP: usize = 4096;

/// Memory OOM control flag
pub const MEMORY_OOM_CONTROL: u64 = 1;

/// Memory OOM kill flag
pub const MEMORY_OOM_KILL: u64 = 2;

/// CPU controller configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuController {
    /// Maximum CPU bandwidth in microseconds per period (None = unlimited)
    pub max: Option<u64>,

    /// CPU weight for fair scheduling (1-10000)
    pub weight: u64,

    /// Current CPU usage in microseconds
    pub usage: u64,

    /// Number of CPU cycles
    pub cycles: u64,

    /// Number of context switches
    pub switches: u64,

    /// Throttled time in microseconds
    pub throttled: u64,

    /// Throttled count
    pub throttle_count: u64,
}

impl Default for CpuController {
    fn default() -> Self {
        Self {
            max: None, // Unlimited
            weight: DEFAULT_CPU_WEIGHT,
            usage: 0,
            cycles: 0,
            switches: 0,
            throttled: 0,
            throttle_count: 0,
        }
    }
}

impl CpuController {
    /// Create a new CPU controller
    pub fn new(max: Option<u64>, weight: u64) -> Self {
        Self {
            max,
            weight: weight.clamp(MIN_CPU_WEIGHT, MAX_CPU_WEIGHT),
            ..Default::default()
        }
    }

    /// Check if CPU bandwidth is available
    pub fn can_run(&self) -> bool {
        if let Some(max) = self.max {
            self.usage < max
        } else {
            true
        }
    }

    /// Add CPU usage
    pub fn add_usage(&mut self, usage: u64) {
        self.usage = self.usage.saturating_add(usage);
    }

    /// Reset usage (called every quota period)
    pub fn reset_usage(&mut self) {
        self.usage = 0;
    }

    /// Check if throttled
    pub fn is_throttled(&self) -> bool {
        if let Some(max) = self.max {
            self.usage >= max
        } else {
            false
        }
    }

    /// Get CPU usage percentage
    pub fn usage_percent(&self) -> f64 {
        if let Some(max) = self.max {
            (self.usage as f64 / max as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Memory controller configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryController {
    /// Maximum memory in bytes (None = unlimited)
    pub max: Option<u64>,

    /// Maximum swap in bytes (None = unlimited)
    pub swap_max: Option<u64>,

    /// Current memory usage in bytes
    pub usage: u64,

    /// Current swap usage in bytes
    pub swap_usage: u64,

    /// Number of page faults
    pub faults: u64,

    /// Number of major page faults
    pub major_faults: u64,

    /// Memory low watermark
    pub low: Option<u64>,

    /// Memory high watermark
    pub high: Option<u64>,

    /// OOM control flags
    pub oom_control: u64,

    /// OOM kill count
    pub oom_kill: u64,

    /// Cache usage
    pub cache: u64,

    /// RSS usage
    pub rss: u64,

    /// Shmem usage
    pub shmem: u64,

    /// File usage
    pub file: u64,

    /// Anon usage
    pub anon: u64,
}

impl Default for MemoryController {
    fn default() -> Self {
        Self {
            max: None,
            swap_max: None,
            usage: 0,
            swap_usage: 0,
            faults: 0,
            major_faults: 0,
            low: None,
            high: None,
            oom_control: 0,
            oom_kill: 0,
            cache: 0,
            rss: 0,
            shmem: 0,
            file: 0,
            anon: 0,
        }
    }
}

impl MemoryController {
    /// Create a new memory controller
    pub fn new(max: Option<u64>, swap_max: Option<u64>) -> Self {
        Self {
            max,
            swap_max,
            ..Default::default()
        }
    }

    /// Check if memory can be allocated
    pub fn can_allocate(&self, size: u64) -> bool {
        if let Some(max) = self.max {
            self.usage.saturating_add(size) <= max
        } else {
            true
        }
    }

    /// Add memory usage
    pub fn add_usage(&mut self, size: u64) -> Result<(), Error> {
        if let Some(max) = self.max {
            let new_usage = self.usage.saturating_add(size);
            if new_usage > max {
                return Err(Error::OutOfMemory);
            }
            self.usage = new_usage;
        } else {
            self.usage = self.usage.saturating_add(size);
        }
        Ok(())
    }

    /// Free memory
    pub fn sub_usage(&mut self, size: u64) {
        self.usage = self.usage.saturating_sub(size);
    }

    /// Check if at memory limit
    pub fn at_limit(&self) -> bool {
        if let Some(max) = self.max {
            self.usage >= max
        } else {
            false
        }
    }

    /// Check if OOM should be triggered
    pub fn should_oom(&self) -> bool {
        self.at_limit() && (self.oom_control & MEMORY_OOM_CONTROL != 0)
    }

    /// Get memory usage percentage
    pub fn usage_percent(&self) -> f64 {
        if let Some(max) = self.max {
            (self.usage as f64 / max as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if above high watermark
    pub fn above_high(&self) -> bool {
        if let Some(high) = self.high {
            self.usage >= high
        } else {
            false
        }
    }

    /// Check if below low watermark
    pub fn below_low(&self) -> bool {
        if let Some(low) = self.low {
            self.usage <= low
        } else {
            false
        }
    }
}

/// I/O controller configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoController {
    /// Maximum I/O per device (device major:minor -> bytes per second)
    pub max: Option<(u64, u64)>,

    /// I/O weight (1-10000)
    pub weight: u64,

    /// Current I/O bytes read
    pub read_bytes: u64,

    /// Current I/O bytes written
    pub write_bytes: u64,

    /// Number of read operations
    pub read_ios: u64,

    /// Number of write operations
    pub write_ios: u64,

    /// Throttled read time in microseconds
    pub read_throttled: u64,

    /// Throttled write time in microseconds
    pub write_throttled: u64,
}

impl Default for IoController {
    fn default() -> Self {
        Self {
            max: None,
            weight: DEFAULT_IO_WEIGHT,
            read_bytes: 0,
            write_bytes: 0,
            read_ios: 0,
            write_ios: 0,
            read_throttled: 0,
            write_throttled: 0,
        }
    }
}

impl IoController {
    /// Create a new I/O controller
    pub fn new(max: Option<(u64, u64)>, weight: u64) -> Self {
        Self {
            max,
            weight: weight.clamp(MIN_CPU_WEIGHT, MAX_IO_WEIGHT),
            ..Default::default()
        }
    }

    /// Check if I/O can be performed
    pub fn can_io(&self) -> bool {
        // I/O limiting not yet fully implemented
        true
    }

    /// Add read bytes
    pub fn add_read(&mut self, bytes: u64, ios: u64) {
        self.read_bytes = self.read_bytes.saturating_add(bytes);
        self.read_ios = self.read_ios.saturating_add(ios);
    }

    /// Add write bytes
    pub fn add_write(&mut self, bytes: u64, ios: u64) {
        self.write_bytes = self.write_bytes.saturating_add(bytes);
        self.write_ios = self.write_ios.saturating_add(ios);
    }

    /// Get total I/O bytes
    pub fn total_bytes(&self) -> u64 {
        self.read_bytes.saturating_add(self.write_bytes)
    }

    /// Get total I/O operations
    pub fn total_ios(&self) -> u64 {
        self.read_ios.saturating_add(self.write_ios)
    }
}

/// cgroup statistics
#[derive(Debug, Clone, Copy)]
pub struct CgroupStats {
    /// Number of processes in cgroup
    pub num_processes: usize,

    /// CPU statistics
    pub cpu: CpuController,

    /// Memory statistics
    pub memory: MemoryController,

    /// I/O statistics
    pub io: IoController,

    /// Number of descendant cgroups
    pub num_children: usize,

    /// Cgroup depth in hierarchy
    pub depth: usize,
}

/// cgroup configuration
#[derive(Debug, Clone)]
pub struct CgroupConfig {
    /// Cgroup name
    pub name: String,

    /// Parent cgroup path (empty for root)
    pub parent: String,

    /// CPU controller settings
    pub cpu_max: Option<u64>,
    pub cpu_weight: Option<u64>,

    /// Memory controller settings
    pub memory_max: Option<u64>,
    pub memory_swap_max: Option<u64>,
    pub memory_low: Option<u64>,
    pub memory_high: Option<u64>,

    /// I/O controller settings
    pub io_max: Option<(u64, u64)>,
    pub io_weight: Option<u64>,

    /// Enable OOM killer
    pub enable_oom_killer: bool,
}

impl Default for CgroupConfig {
    fn default() -> Self {
        Self {
            name: "cgroup".into(),
            parent: String::new(),
            cpu_max: None,
            cpu_weight: None,
            memory_max: None,
            memory_swap_max: None,
            memory_low: None,
            memory_high: None,
            io_max: None,
            io_weight: None,
            enable_oom_killer: true,
        }
    }
}

/// Control group
pub struct Cgroup {
    /// Cgroup name
    name: String,

    /// Full path from root
    path: String,

    /// Depth in hierarchy
    depth: usize,

    /// CPU controller
    cpu: Arc<SpinLock<CpuController>>,

    /// Memory controller
    memory: Arc<SpinLock<MemoryController>>,

    /// I/O controller
    io: Arc<SpinLock<IoController>>,

    /// Attached process IDs
    processes: Arc<SpinLock<BTreeSet<ProcessId>>>,

    /// Child cgroups
    children: Arc<SpinLock<BTreeMap<String, Arc<Cgroup>>>>,

    /// Parent cgroup reference
    parent: Option<Arc<Cgroup>>,

    /// Whether cgroup is enabled
    enabled: AtomicBool,

    /// Creation time
    created_at: Duration,
}

impl core::fmt::Debug for Cgroup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cgroup")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("depth", &self.depth)
            .field("process_count", &self.processes.lock().len())
            .field("children_count", &self.children.lock().len())
            .field("enabled", &self.enabled.load(core::sync::atomic::Ordering::Relaxed))
            .finish()
    }
}

impl Cgroup {
    /// Create a new cgroup
    pub fn new(name: String, path: String, config: CgroupConfig, depth: usize, parent: Option<Arc<Cgroup>>) -> Self {
        let cpu = CpuController::new(config.cpu_max, config.cpu_weight.unwrap_or(DEFAULT_CPU_WEIGHT));

        let mut memory =
            MemoryController::new(config.memory_max, config.memory_swap_max);
        memory.low = config.memory_low;
        memory.high = config.memory_high;
        if config.enable_oom_killer {
            memory.oom_control |= MEMORY_OOM_CONTROL;
        }

        let io = IoController::new(config.io_max, config.io_weight.unwrap_or(DEFAULT_IO_WEIGHT));

        Self {
            name,
            path,
            depth,
            cpu: Arc::new(SpinLock::new(cpu)),
            memory: Arc::new(SpinLock::new(memory)),
            io: Arc::new(SpinLock::new(io)),
            processes: Arc::new(SpinLock::new(BTreeSet::new())),
            children: Arc::new(SpinLock::new(BTreeMap::new())),
            parent,
            enabled: AtomicBool::new(true),
            created_at: Duration::from_secs(0),
        }
    }

    /// Get cgroup name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get cgroup path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get cgroup depth
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Check if cgroup is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable or disable cgroup
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get CPU controller
    pub fn cpu(&self) -> &Arc<SpinLock<CpuController>> {
        &self.cpu
    }

    /// Get memory controller
    pub fn memory(&self) -> &Arc<SpinLock<MemoryController>> {
        &self.memory
    }

    /// Get I/O controller
    pub fn io(&self) -> &Arc<SpinLock<IoController>> {
        &self.io
    }

    /// Attach a process to this cgroup
    pub fn attach_process(&self, pid: ProcessId) -> Result<(), Error> {
        let mut processes = self.processes.lock();

        if processes.len() >= MAX_PROCESSES_PER_CGROUP {
            return Err(Error::QuotaExceeded);
        }

        processes.insert(pid);
        Ok(())
    }

    /// Detach a process from this cgroup
    pub fn detach_process(&self, pid: ProcessId) {
        self.processes.lock().remove(&pid);
    }

    /// Get all processes in this cgroup
    pub fn processes(&self) -> Vec<ProcessId> {
        self.processes.lock().iter().copied().collect()
    }

    /// Get number of processes
    pub fn num_processes(&self) -> usize {
        self.processes.lock().len()
    }

    /// Add a child cgroup
    pub fn add_child(&self, child: Arc<Cgroup>) -> Result<(), Error> {
        if child.depth() > MAX_CGROUP_DEPTH {
            return Err(Error::Other("Maximum cgroup depth exceeded".to_string()));
        }

        self.children.lock().insert(child.name().into(), child);
        Ok(())
    }

    /// Remove a child cgroup
    pub fn remove_child(&self, name: &str) {
        self.children.lock().remove(name);
    }

    /// Get a child cgroup
    pub fn get_child(&self, name: &str) -> Option<Arc<Cgroup>> {
        self.children.lock().get(name).cloned()
    }

    /// Get all children
    pub fn children(&self) -> Vec<Arc<Cgroup>> {
        self.children.lock().values().cloned().collect()
    }

    /// Set parent cgroup
    pub fn set_parent(&mut self, parent: Arc<Cgroup>) {
        self.parent = Some(parent);
    }

    /// Get parent cgroup
    pub fn parent(&self) -> Option<&Arc<Cgroup>> {
        self.parent.as_ref()
    }

    /// Get cgroup statistics
    pub fn stats(&self) -> CgroupStats {
        CgroupStats {
            num_processes: self.num_processes(),
            cpu: *self.cpu.lock(),
            memory: *self.memory.lock(),
            io: *self.io.lock(),
            num_children: self.children.lock().len(),
            depth: self.depth,
        }
    }

    /// Kill all processes in cgroup
    pub fn kill_all(&self) {
        let processes = self.processes.lock();
        for pid in processes.iter() {
            // GH-#1154: Send SIGKILL to process
            // See: https://github.com/npos/kernel/issues/1154
            log::warn!("Killing process {} in cgroup {}", pid, self.path);
        }
    }

    /// Freeze all processes in cgroup
    pub fn freeze(&self) {
        // GH-#1155: Implement process freezing
        // See: https://github.com/npos/kernel/issues/1155
        log::info!("Freezing cgroup {}", self.path);
    }

    /// Thaw all processes in cgroup
    pub fn thaw(&self) {
        // GH-#1156: Implement process thawing
        // See: https://github.com/npos/kernel/issues/1156
        log::info!("Thawing cgroup {}", self.path);
    }

    /// Check if cgroup is at memory limit
    pub fn is_at_memory_limit(&self) -> bool {
        self.memory.lock().at_limit()
    }

    /// Handle OOM for this cgroup
    pub fn handle_oom(&self) -> Result<(), Error> {
        let mut memory = self.memory.lock();

        if !memory.should_oom() {
            return Ok(());
        }

        memory.oom_kill += 1;

        // OOM killer logic: kill process with largest memory usage
        drop(memory);

        // GH-#1157: Implement proper OOM killer
        // See: https://github.com/npos/kernel/issues/1157
        log::error!("OOM in cgroup {}, killing processes", self.path);
        self.kill_all();

        Ok(())
    }

    /// Update CPU quota (called periodically)
    pub fn update_cpu_quota(&self) {
        self.cpu.lock().reset_usage();
    }

    /// Get total resource usage (including children)
    pub fn total_usage(&self) -> (u64, u64, u64) {
        let cpu_usage = self.cpu.lock().usage;
        let mem_usage = self.memory.lock().usage;
        let io_usage = self.io.lock().total_bytes();

        let mut total_cpu = cpu_usage;
        let mut total_mem = mem_usage;
        let mut total_io = io_usage;

        for child in self.children() {
            let (child_cpu, child_mem, child_io) = child.total_usage();
            total_cpu = total_cpu.saturating_add(child_cpu);
            total_mem = total_mem.saturating_add(child_mem);
            total_io = total_io.saturating_add(child_io);
        }

        (total_cpu, total_mem, total_io)
    }
}

/// cgroup manager
pub struct CgroupManager {
    /// Root cgroup
    root: Arc<Cgroup>,

    /// Next cgroup ID
    next_id: AtomicU64,

    /// Enabled controllers
    enabled_controllers: SpinLock<BTreeSet<String>>,
}

impl CgroupManager {
    /// Create a new cgroup manager
    pub fn new() -> Self {
        let root = Arc::new(Cgroup::new(
            "root".into(),
            "/".into(),
            CgroupConfig::default(),
            0,
            None,
        ));

        let mut enabled_controllers = BTreeSet::new();
        enabled_controllers.insert("cpu".into());
        enabled_controllers.insert("memory".into());
        enabled_controllers.insert("io".into());

        Self {
            root,
            next_id: AtomicU64::new(1),
            enabled_controllers: SpinLock::new(enabled_controllers),
        }
    }

    /// Get root cgroup
    pub fn root(&self) -> &Arc<Cgroup> {
        &self.root
    }

    /// Create a new cgroup
    pub fn create_cgroup(&self, config: CgroupConfig) -> Result<Arc<Cgroup>, Error> {
        let parent = if config.parent.is_empty() {
            self.root.clone()
        } else {
            self.lookup_cgroup(&config.parent)?
        };

        if parent.depth() >= MAX_CGROUP_DEPTH {
            return Err(Error::Other("Maximum cgroup depth exceeded".to_string()));
        }

        let path = if config.parent.is_empty() {
            format!("/{}", config.name)
        } else {
            format!("{}/{}", config.parent, config.name)
        };

        let cgroup = Arc::new(Cgroup::new(config.name.clone(), path, config, parent.depth() + 1, Some(parent.clone())));
        parent.add_child(cgroup.clone())?;

        Ok(cgroup)
    }

    /// Look up a cgroup by path
    pub fn lookup_cgroup(&self, path: &str) -> Result<Arc<Cgroup>, Error> {
        if path == "/" || path.is_empty() {
            return Ok(self.root.clone());
        }

        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = self.root.clone();

        for component in components {
            if component.is_empty() {
                continue;
            }

            current = current
                .get_child(component)
                .ok_or_else(|| Error::Other(format!("Cgroup '{}' not found", component)))?;
        }

        Ok(current)
    }

    /// Delete a cgroup
    pub fn delete_cgroup(&self, path: &str) -> Result<(), Error> {
        let cgroup = self.lookup_cgroup(path)?;

        // Cannot delete root
        if cgroup.path() == "/" {
            return Err(Error::PermissionDenied);
        }

        // Cannot delete if it has children
        if !cgroup.children().is_empty() {
            return Err(Error::Other("Cgroup has children".to_string()));
        }

        // Cannot delete if it has processes
        if cgroup.num_processes() > 0 {
            return Err(Error::Other("Cgroup has processes".to_string()));
        }

        // Remove from parent
        if let Some(parent) = cgroup.parent() {
            parent.remove_child(cgroup.name());
        }

        Ok(())
    }

    /// Get cgroup statistics
    pub fn get_stats(&self, path: &str) -> Result<CgroupStats, Error> {
        let cgroup = self.lookup_cgroup(path)?;
        Ok(cgroup.stats())
    }

    /// Get all cgroups
    pub fn list_cgroups(&self, path: &str) -> Result<Vec<String>, Error> {
        let cgroup = self.lookup_cgroup(path)?;

        let mut children = cgroup.children();
        children.sort_by(|a, b| a.name().cmp(b.name()));

        Ok(children.iter().map(|c| c.path().into()).collect())
    }

    /// Move a process to a different cgroup
    pub fn move_process(&self, pid: ProcessId, from: &str, to: &str) -> Result<(), Error> {
        let from_cgroup = self.lookup_cgroup(from)?;
        let to_cgroup = self.lookup_cgroup(to)?;

        from_cgroup.detach_process(pid);
        to_cgroup.attach_process(pid)?;

        Ok(())
    }

    /// Get all processes in a cgroup subtree
    pub fn list_processes(&self, path: &str) -> Result<Vec<ProcessId>, Error> {
        let cgroup = self.lookup_cgroup(path)?;
        Ok(cgroup.processes())
    }

    /// Enable a controller
    pub fn enable_controller(&self, controller: &str) {
        self.enabled_controllers.lock().insert(controller.into());
    }

    /// Disable a controller
    pub fn disable_controller(&self, controller: &str) {
        self.enabled_controllers.lock().remove(controller);
    }

    /// Check if a controller is enabled
    pub fn is_controller_enabled(&self, controller: &str) -> bool {
        self.enabled_controllers.lock().contains(controller)
    }

    /// Get all enabled controllers
    pub fn enabled_controllers(&self) -> Vec<String> {
        self.enabled_controllers.lock().iter().cloned().collect()
    }

    /// Update CPU quotas for all cgroups (called periodically)
    pub fn update_all_cpu_quotas(&self) {
        fn update_recursive(cgroup: &Cgroup) {
            cgroup.update_cpu_quota();
            for child in cgroup.children() {
                update_recursive(&child);
            }
        }

        update_recursive(&self.root);
    }

    /// Check for memory pressure and trigger actions
    pub fn check_memory_pressure(&self) {
        fn check_recursive(cgroup: &Cgroup) {
            if cgroup.is_at_memory_limit() {
                let _ = cgroup.handle_oom();
            }

            for child in cgroup.children() {
                check_recursive(&child);
            }
        }

        check_recursive(&self.root);
    }

    /// Get global cgroup statistics
    pub fn global_stats(&self) -> CgroupStats {
        self.root.stats()
    }
}

impl Default for CgroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgroup_creation() {
        let manager = CgroupManager::new();

        let config = CgroupConfig {
            name: "test".into(),
            cpu_max: Some(50000),
            memory_max: Some(1024 * 1024 * 1024),
            ..Default::default()
        };

        let cgroup = manager.create_cgroup(config).unwrap();
        assert_eq!(cgroup.name(), "test");
        assert_eq!(cgroup.path(), "/test");
        assert_eq!(cgroup.depth(), 1);
    }

    #[test]
    fn test_cgroup_hierarchy() {
        let manager = CgroupManager::new();

        let parent_config = CgroupConfig {
            name: "parent".into(),
            ..Default::default()
        };
        let parent = manager.create_cgroup(parent_config).unwrap();

        let child_config = CgroupConfig {
            name: "child".into(),
            parent: "/parent".into(),
            ..Default::default()
        };
        let child = manager.create_cgroup(child_config).unwrap();

        assert_eq!(child.path(), "/parent/child");
        assert_eq!(child.depth(), 2);
        assert!(parent.get_child("child").is_some());
    }

    #[test]
    fn test_process_attachment() {
        let manager = CgroupManager::new();

        let config = CgroupConfig {
            name: "test".into(),
            ..Default::default()
        };
        let cgroup = manager.create_cgroup(config).unwrap();

        assert!(cgroup.attach_process(ProcessId::new(1)).is_ok());
        assert!(cgroup.attach_process(ProcessId::new(2)).is_ok());
        assert_eq!(cgroup.num_processes(), 2);

        cgroup.detach_process(ProcessId::new(1));
        assert_eq!(cgroup.num_processes(), 1);
    }

    #[test]
    fn test_cpu_controller() {
        let cpu = CpuController::new(Some(50000), 100);

        assert!(cpu.can_run());
        cpu.add_usage(30000);
        assert!(cpu.can_run());
        cpu.add_usage(30000);
        assert!(!cpu.can_run());
        assert!(cpu.is_throttled());

        cpu.reset_usage();
        assert!(cpu.can_run());
    }

    #[test]
    fn test_memory_controller() {
        let memory = MemoryController::new(Some(1024), None);

        assert!(memory.can_allocate(512).is_ok());
        assert!(memory.add_usage(512).is_ok());
        assert!(memory.can_allocate(256).is_ok());
        assert!(memory.add_usage(256).is_err());

        memory.sub_usage(100);
        assert_eq!(memory.usage(), 412);
        assert!(memory.at_limit());
    }
}
