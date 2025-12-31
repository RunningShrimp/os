//! Resource usage accounting and statistics
//!
//! This module provides comprehensive resource usage tracking for processes,
//! cgroups, and the entire system. It maintains historical data and generates
//! statistics for monitoring, billing, and capacity planning.
//!
//! # Overview
//!
//! The accounting system tracks:
//! - CPU usage (time, cores, utilization)
//! - Memory usage (RSS, virtual, swap, cache)
//! - I/O usage (bytes, operations, bandwidth)
//! - Network usage (packets, bytes, connections)
//! - System calls and context switches
//!
//! # Architecture
//!
//! ```text
//! AccountingManager
//!     ├── ProcessAccounting (per-process)
//!     ├── CgroupAccounting (per-cgroup)
//!     ├── SystemAccounting (global)
//!     └── HistoryBuffer (time-series data)
//! ```
//!
//! # Metrics Collected
//!
//! ## CPU Metrics
//! - User time
//! - System time
//! - Wait time
//! - Number of context switches
//! - CPU utilization percentage
//!
//! ## Memory Metrics
//! - Resident set size (RSS)
//! - Virtual memory size
//! - Shared memory
//! - Page faults
//! - Swap usage
//!
//! ## I/O Metrics
//! - Bytes read/written
//! - Read/write operations
//! - I/O time
//! - Block device utilization
//!
//! # Examples
//!
//! ```no_run
//! use kernel::resource::accounting::{AccountingManager, ProcessStats};
//!
//! let manager = AccountingManager::new();
//!
//! // Get process statistics
//! let stats = manager.get_process_stats(ProcessId::new(1234));
//!
//! // Get system-wide statistics
//! let system_stats = manager.get_system_stats();
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::Error;
use crate::process::ProcessId;
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

/// Default history buffer size (number of samples)
pub const DEFAULT_HISTORY_SIZE: usize = 60;

/// Accounting update interval in milliseconds
pub const ACCOUNTING_INTERVAL_MS: u64 = 1000;

/// Maximum number of tracked processes
const MAX_TRACKED_PROCESSES: usize = 65536;

/// CPU usage statistics
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CpuStats {
    /// User CPU time in nanoseconds
    pub user_time: u64,

    /// System CPU time in nanoseconds
    pub system_time: u64,

    /// Wait time in nanoseconds
    pub wait_time: u64,

    /// Number of voluntary context switches
    pub voluntary_switches: u64,

    /// Number of involuntary context switches
    pub involuntary_switches: u64,

    /// Number of CPU cycles
    pub cycles: u64,

    /// Number of instructions
    pub instructions: u64,

    /// Cache references
    pub cache_references: u64,

    /// Cache misses
    pub cache_misses: u64,

    /// CPU utilization percentage (0-100)
    pub utilization_percent: f64,
}

impl CpuStats {
    /// Get total CPU time
    pub fn total_time(&self) -> u64 {
        self.user_time.saturating_add(self.system_time).saturating_add(self.wait_time)
    }

    /// Add two CPU stats
    pub fn add(&mut self, other: CpuStats) {
        self.user_time = self.user_time.saturating_add(other.user_time);
        self.system_time = self.system_time.saturating_add(other.system_time);
        self.wait_time = self.wait_time.saturating_add(other.wait_time);
        self.voluntary_switches = self.voluntary_switches.saturating_add(other.voluntary_switches);
        self.involuntary_switches =
            self.involuntary_switches.saturating_add(other.involuntary_switches);
        self.cycles = self.cycles.saturating_add(other.cycles);
        self.instructions = self.instructions.saturating_add(other.instructions);
        self.cache_references = self.cache_references.saturating_add(other.cache_references);
        self.cache_misses = self.cache_misses.saturating_add(other.cache_misses);
    }

    /// Subtract CPU stats (saturating)
    pub fn sub(&mut self, other: CpuStats) {
        self.user_time = self.user_time.saturating_sub(other.user_time);
        self.system_time = self.system_time.saturating_sub(other.system_time);
        self.wait_time = self.wait_time.saturating_sub(other.wait_time);
        self.voluntary_switches = self.voluntary_switches.saturating_sub(other.voluntary_switches);
        self.involuntary_switches =
            self.involuntary_switches.saturating_sub(other.involuntary_switches);
        self.cycles = self.cycles.saturating_sub(other.cycles);
        self.instructions = self.instructions.saturating_sub(other.instructions);
        self.cache_references = self.cache_references.saturating_sub(other.cache_references);
        self.cache_misses = self.cache_misses.saturating_sub(other.cache_misses);
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MemoryStats {
    /// Resident set size in bytes
    pub rss: u64,

    /// Virtual memory size in bytes
    pub virtual_size: u64,

    /// Shared memory size in bytes
    pub shared: u64,

    /// Text (code) size in bytes
    pub text: u64,

    /// Data and stack size in bytes
    pub data: u64,

    /// Number of page faults
    pub page_faults: u64,

    /// Number of major page faults
    pub major_faults: u64,

    /// Swap usage in bytes
    pub swap: u64,

    /// Cache size in bytes
    pub cache: u64,

    /// Huge page usage
    pub huge_pages: u64,

    /// Anonymous memory
    pub anon: u64,

    /// File-backed memory
    pub file: u64,

    /// Memory utilization percentage
    pub utilization_percent: f64,
}

impl MemoryStats {
    /// Get total memory usage
    pub fn total(&self) -> u64 {
        self.rss.saturating_add(self.shared).saturating_add(self.cache)
    }

    /// Add two memory stats
    pub fn add(&mut self, other: MemoryStats) {
        self.rss = self.rss.saturating_add(other.rss);
        self.virtual_size = self.virtual_size.saturating_add(other.virtual_size);
        self.shared = self.shared.saturating_add(other.shared);
        self.text = self.text.saturating_add(other.text);
        self.data = self.data.saturating_add(other.data);
        self.page_faults = self.page_faults.saturating_add(other.page_faults);
        self.major_faults = self.major_faults.saturating_add(other.major_faults);
        self.swap = self.swap.saturating_add(other.swap);
        self.cache = self.cache.saturating_add(other.cache);
        self.huge_pages = self.huge_pages.saturating_add(other.huge_pages);
        self.anon = self.anon.saturating_add(other.anon);
        self.file = self.file.saturating_add(other.file);
    }
}

/// I/O usage statistics
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IoStats {
    /// Bytes read
    pub read_bytes: u64,

    /// Bytes written
    pub write_bytes: u64,

    /// Number of read operations
    pub read_ops: u64,

    /// Number of write operations
    pub write_ops: u64,

    /// Read time in nanoseconds
    pub read_time: u64,

    /// Write time in nanoseconds
    pub write_time: u64,

    /// Sync operations
    pub sync_ops: u64,

    /// Bytes sync'd
    pub sync_bytes: u64,

    /// I/O bandwidth in bytes/sec
    pub bandwidth_bps: u64,

    /// I/O operations per second
    pub iops: u64,
}

impl IoStats {
    /// Get total bytes
    pub fn total_bytes(&self) -> u64 {
        self.read_bytes.saturating_add(self.write_bytes)
    }

    /// Get total operations
    pub fn total_ops(&self) -> u64 {
        self.read_ops.saturating_add(self.write_ops)
    }

    /// Add two I/O stats
    pub fn add(&mut self, other: IoStats) {
        self.read_bytes = self.read_bytes.saturating_add(other.read_bytes);
        self.write_bytes = self.write_bytes.saturating_add(other.write_bytes);
        self.read_ops = self.read_ops.saturating_add(other.read_ops);
        self.write_ops = self.write_ops.saturating_add(other.write_ops);
        self.read_time = self.read_time.saturating_add(other.read_time);
        self.write_time = self.write_time.saturating_add(other.write_time);
        self.sync_ops = self.sync_ops.saturating_add(other.sync_ops);
        self.sync_bytes = self.sync_bytes.saturating_add(other.sync_bytes);
    }
}

/// Network usage statistics
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkStats {
    /// Bytes received
    pub rx_bytes: u64,

    /// Bytes transmitted
    pub tx_bytes: u64,

    /// Packets received
    pub rx_packets: u64,

    /// Packets transmitted
    pub tx_packets: u64,

    /// Receive errors
    pub rx_errors: u64,

    /// Transmit errors
    pub tx_errors: u64,

    /// Packets dropped
    pub dropped: u64,

    /// Active connections
    pub connections: u64,

    /// Network bandwidth in bytes/sec
    pub bandwidth_bps: u64,
}

impl NetworkStats {
    /// Get total bytes
    pub fn total_bytes(&self) -> u64 {
        self.rx_bytes.saturating_add(self.tx_bytes)
    }

    /// Get total packets
    pub fn total_packets(&self) -> u64 {
        self.rx_packets.saturating_add(self.tx_packets)
    }
}

/// Process statistics
#[derive(Debug, Clone, Copy)]
pub struct ProcessStats {
    /// Process ID
    pub pid: ProcessId,

    /// Parent process ID
    pub ppid: ProcessId,

    /// Thread group ID
    pub tg_id: ProcessId,

    /// Process start time
    pub start_time: Duration,

    /// CPU statistics
    pub cpu: CpuStats,

    /// Memory statistics
    pub memory: MemoryStats,

    /// I/O statistics
    pub io: IoStats,

    /// Network statistics
    pub network: NetworkStats,

    /// Number of threads
    pub num_threads: u64,

    /// Number of open file descriptors
    pub num_fds: u64,

    /// Number of system calls
    pub syscalls: u64,

    /// Exit status (if exited)
    pub exit_status: Option<i32>,
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            pid: 0,
            ppid: 0,
            tg_id: 0,
            start_time: Duration::from_secs(0),
            cpu: CpuStats::default(),
            memory: MemoryStats::default(),
            io: IoStats::default(),
            network: NetworkStats::default(),
            num_threads: 0,
            num_fds: 0,
            syscalls: 0,
            exit_status: None,
        }
    }
}

/// System-wide statistics
#[derive(Debug, Clone, Copy)]
pub struct SystemStats {
    /// Total system CPU time
    pub total_cpu_time: u64,

    /// CPU utilization percentage (per-core)
    pub cpu_utilization: [f64; 64],

    /// Number of CPU cores
    pub num_cores: usize,

    /// Total memory
    pub total_memory: u64,

    /// Free memory
    pub free_memory: u64,

    /// Available memory
    pub available_memory: u64,

    /// Buffered memory
    pub buffers: u64,

    /// Cached memory
    pub cached: u64,

    /// Swap total
    pub swap_total: u64,

    /// Swap free
    pub swap_free: u64,

    /// Number of processes
    pub num_processes: usize,

    /// Number of threads
    pub num_threads: usize,

    /// Number of context switches
    pub context_switches: u64,

    /// Boot time
    pub boot_time: Duration,

    /// Uptime in seconds
    pub uptime: u64,

    /// Load averages (1, 5, 15 min)
    pub load_avg: (f64, f64, f64),
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            total_cpu_time: 0,
            cpu_utilization: [0.0; 64],
            num_cores: 1,
            total_memory: 0,
            free_memory: 0,
            available_memory: 0,
            buffers: 0,
            cached: 0,
            swap_total: 0,
            swap_free: 0,
            num_processes: 0,
            num_threads: 0,
            context_switches: 0,
            boot_time: Duration::from_secs(0),
            uptime: 0,
            load_avg: (0.0, 0.0, 0.0),
        }
    }
}

/// Cgroup statistics
#[derive(Debug, Clone, Copy)]
pub struct CgroupStats {
    /// Cgroup path
    pub path: &'static str,

    /// Number of processes
    pub num_processes: usize,

    /// CPU statistics
    pub cpu: CpuStats,

    /// Memory statistics
    pub memory: MemoryStats,

    /// I/O statistics
    pub io: IoStats,

    /// Network statistics
    pub network: NetworkStats,

    /// Number of descendant cgroups
    pub num_children: usize,
}

/// Historical data sample
#[derive(Debug, Clone, Copy)]
pub struct HistorySample {
    /// Timestamp
    pub timestamp: Duration,

    /// CPU usage
    pub cpu: CpuStats,

    /// Memory usage
    pub memory: MemoryStats,

    /// I/O usage
    pub io: IoStats,
}

/// Per-process accounting data
#[derive(Debug)]
pub struct ProcessAccounting {
    /// Process ID
    pid: ProcessId,

    /// Current statistics
    current: ProcessStats,

    /// Previous statistics (for delta calculation)
    previous: ProcessStats,

    /// History buffer
    history: Vec<HistorySample>,

    /// Accounting enabled
    enabled: bool,
}

impl ProcessAccounting {
    /// Create a new process accounting
    pub fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            current: ProcessStats::default(),
            previous: ProcessStats::default(),
            history: Vec::with_capacity(DEFAULT_HISTORY_SIZE),
            enabled: true,
        }
    }

    /// Get process ID
    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    /// Update CPU usage
    pub fn update_cpu(&mut self, stats: CpuStats) {
        self.previous.cpu = self.current.cpu;
        self.current.cpu = stats;
    }

    /// Update memory usage
    pub fn update_memory(&mut self, stats: MemoryStats) {
        self.previous.memory = self.current.memory;
        self.current.memory = stats;
    }

    /// Update I/O usage
    pub fn update_io(&mut self, stats: IoStats) {
        self.previous.io = self.current.io;
        self.current.io = stats;
    }

    /// Update network usage
    pub fn update_network(&mut self, stats: NetworkStats) {
        self.current.network = stats;
    }

    /// Get current statistics
    pub fn current(&self) -> &ProcessStats {
        &self.current
    }

    /// Get delta (change since previous sample)
    pub fn delta(&self) -> ProcessStats {
        let mut delta = self.current;
        delta.cpu.user_time = delta
            .cpu
            .user_time
            .saturating_sub(self.previous.cpu.user_time);
        delta.cpu.system_time = delta
            .cpu
            .system_time
            .saturating_sub(self.previous.cpu.system_time);
        delta.cpu.cycles = delta.cpu.cycles.saturating_sub(self.previous.cpu.cycles);

        delta.io.read_bytes = delta
            .io
            .read_bytes
            .saturating_sub(self.previous.io.read_bytes);
        delta.io.write_bytes = delta
            .io
            .write_bytes
            .saturating_sub(self.previous.io.write_bytes);

        delta
    }

    /// Add a history sample
    pub fn add_history_sample(&mut self, sample: HistorySample) {
        self.history.push(sample);
        if self.history.len() > DEFAULT_HISTORY_SIZE {
            self.history.remove(0);
        }
    }

    /// Get history
    pub fn history(&self) -> &[HistorySample] {
        &self.history
    }

    /// Get average CPU usage over history
    pub fn avg_cpu_usage(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let total: u64 = self.history.iter().map(|s| s.cpu.total_time()).sum();
        let avg = total / self.history.len() as u64;
        avg as f64
    }

    /// Get peak memory usage
    pub fn peak_memory(&self) -> u64 {
        self.history
            .iter()
            .map(|s| s.memory.rss)
            .max()
            .unwrap_or(0)
    }

    /// Enable or disable accounting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if accounting is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Per-cgroup accounting data
#[derive(Debug)]
pub struct CgroupAccounting {
    /// Cgroup path
    path: String,

    /// Current statistics
    current: CgroupStats,

    /// History buffer
    history: Vec<HistorySample>,
}

impl CgroupAccounting {
    /// Create a new cgroup accounting
    pub fn new(path: String) -> Self {
        Self {
            path,
            current: CgroupStats {
                path: "",
                num_processes: 0,
                cpu: CpuStats::default(),
                memory: MemoryStats::default(),
                io: IoStats::default(),
                network: NetworkStats::default(),
                num_children: 0,
            },
            history: Vec::with_capacity(DEFAULT_HISTORY_SIZE),
        }
    }

    /// Get cgroup path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Update statistics
    pub fn update(&mut self, stats: CgroupStats) {
        self.current = stats;
    }

    /// Get current statistics
    pub fn current(&self) -> &CgroupStats {
        &self.current
    }

    /// Add history sample
    pub fn add_history_sample(&mut self, sample: HistorySample) {
        self.history.push(sample);
        if self.history.len() > DEFAULT_HISTORY_SIZE {
            self.history.remove(0);
        }
    }
}

/// Accounting manager
pub struct AccountingManager {
    /// Per-process accounting
    process_accounting: SpinLock<BTreeMap<ProcessId, Arc<SpinLock<ProcessAccounting>>>>,

    /// Per-cgroup accounting
    cgroup_accounting: SpinLock<BTreeMap<String, Arc<SpinLock<CgroupAccounting>>>>,

    /// System statistics
    system_stats: SpinLock<SystemStats>,

    /// Number of tracked processes
    num_tracked: AtomicU64,
}

impl AccountingManager {
    /// Create a new accounting manager
    pub fn new() -> Self {
        Self {
            process_accounting: SpinLock::new(BTreeMap::new()),
            cgroup_accounting: SpinLock::new(BTreeMap::new()),
            system_stats: SpinLock::new(SystemStats::default()),
            num_tracked: AtomicU64::new(0),
        }
    }

    /// Start tracking a process
    pub fn track_process(&self, pid: ProcessId) -> Result<(), Error> {
        let mut accounting = self.process_accounting.lock();

        if accounting.len() >= MAX_TRACKED_PROCESSES {
            return Err(Error::QuotaExceeded);
        }

        if accounting.contains_key(&pid) {
            return Ok(());
        }

        accounting.insert(pid, Arc::new(SpinLock::new(ProcessAccounting::new(pid))));
        self.num_tracked.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Stop tracking a process
    pub fn untrack_process(&self, pid: ProcessId) {
        let mut accounting = self.process_accounting.lock();
        if accounting.remove(&pid).is_some() {
            self.num_tracked.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Update process CPU statistics
    pub fn update_process_cpu(&self, pid: ProcessId, stats: CpuStats) {
        if let Some(accounting) = self.process_accounting.lock().get(&pid) {
            accounting.lock().update_cpu(stats);
        }
    }

    /// Update process memory statistics
    pub fn update_process_memory(&self, pid: ProcessId, stats: MemoryStats) {
        if let Some(accounting) = self.process_accounting.lock().get(&pid) {
            accounting.lock().update_memory(stats);
        }
    }

    /// Update process I/O statistics
    pub fn update_process_io(&self, pid: ProcessId, stats: IoStats) {
        if let Some(accounting) = self.process_accounting.lock().get(&pid) {
            accounting.lock().update_io(stats);
        }
    }

    /// Get process statistics
    pub fn get_process_stats(&self, pid: ProcessId) -> Option<ProcessStats> {
        self.process_accounting
            .lock()
            .get(&pid)
            .map(|acc| *acc.lock().current())
    }

    /// Get all process statistics
    pub fn get_all_process_stats(&self) -> Vec<ProcessStats> {
        self.process_accounting
            .lock()
            .values()
            .map(|acc| *acc.lock().current())
            .collect()
    }

    /// Start tracking a cgroup
    pub fn track_cgroup(&self, path: String) {
        let mut accounting = self.cgroup_accounting.lock();
        if !accounting.contains_key(&path) {
            let path_clone = path.clone();
            accounting.insert(path, Arc::new(SpinLock::new(CgroupAccounting::new(path_clone))));
        }
    }

    /// Stop tracking a cgroup
    pub fn untrack_cgroup(&self, path: &str) {
        self.cgroup_accounting.lock().remove(path);
    }

    /// Update cgroup statistics
    pub fn update_cgroup_stats(&self, path: &str, stats: CgroupStats) {
        if let Some(accounting) = self.cgroup_accounting.lock().get(path) {
            accounting.lock().update(stats);
        }
    }

    /// Get cgroup statistics
    pub fn get_cgroup_stats(&self, path: &str) -> Option<CgroupStats> {
        self.cgroup_accounting
            .lock()
            .get(path)
            .map(|acc| *acc.lock().current())
    }

    /// Update system statistics
    pub fn update_system_stats(&self, stats: SystemStats) {
        *self.system_stats.lock() = stats;
    }

    /// Get system statistics
    pub fn get_system_stats(&self) -> SystemStats {
        *self.system_stats.lock()
    }

    /// Generate billing data for a process
    pub fn generate_billing_data(&self, pid: ProcessId) -> Option<BillingData> {
        let process_accounting = self.process_accounting.lock();
        let accounting = process_accounting.get(&pid)?;
        let acc = accounting.lock();

        let cpu_time = acc.current.cpu.total_time();
        let avg_memory = acc.avg_cpu_usage() as u64;
        let io_bytes = acc.current.io.total_bytes();

        Some(BillingData {
            pid,
            cpu_time_ns: cpu_time,
            avg_memory_bytes: avg_memory,
            io_bytes,
            network_bytes: acc.current.network.total_bytes(),
            uptime: Duration::from_secs(0), // GH-#1271: track actual uptime
            // See: https://github.com/npos/kernel/issues/1271
        })
    }

    /// Get aggregate statistics for all processes
    pub fn get_aggregate_stats(&self) -> AggregateStats {
        let processes = self.process_accounting.lock();

        let mut total_cpu = CpuStats::default();
        let mut total_memory = MemoryStats::default();
        let mut total_io = IoStats::default();
        let mut total_network = NetworkStats::default();

        for acc in processes.values() {
            let acc_locked = acc.lock();
            let current = acc_locked.current();
            total_cpu.add(current.cpu);
            total_memory.add(current.memory);
            total_io.add(current.io);
            total_network.rx_bytes = total_network.rx_bytes.saturating_add(current.network.rx_bytes);
            total_network.tx_bytes = total_network.tx_bytes.saturating_add(current.network.tx_bytes);
        }

        AggregateStats {
            total_cpu,
            total_memory,
            total_io,
            total_network,
            num_processes: processes.len(),
        }
    }

    /// Get top consumers by CPU
    pub fn top_cpu_consumers(&self, n: usize) -> Vec<(ProcessId, u64)> {
        let mut consumers: Vec<(ProcessId, u64)> = self
            .process_accounting
            .lock()
            .iter()
            .map(|(&pid, acc)| (pid, acc.lock().current().cpu.total_time()))
            .collect();

        consumers.sort_by(|a, b| b.1.cmp(&a.1));
        consumers.truncate(n);
        consumers
    }

    /// Get top consumers by memory
    pub fn top_memory_consumers(&self, n: usize) -> Vec<(ProcessId, u64)> {
        let mut consumers: Vec<(ProcessId, u64)> = self
            .process_accounting
            .lock()
            .iter()
            .map(|(&pid, acc)| (pid, acc.lock().current().memory.rss))
            .collect();

        consumers.sort_by(|a, b| b.1.cmp(&a.1));
        consumers.truncate(n);
        consumers
    }

    /// Get number of tracked processes
    pub fn num_tracked(&self) -> usize {
        self.process_accounting.lock().len()
    }
}

impl Default for AccountingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Billing data for a process
#[derive(Debug, Clone, Copy)]
pub struct BillingData {
    /// Process ID
    pub pid: ProcessId,

    /// Total CPU time in nanoseconds
    pub cpu_time_ns: u64,

    /// Average memory usage in bytes
    pub avg_memory_bytes: u64,

    /// Total I/O bytes
    pub io_bytes: u64,

    /// Total network bytes
    pub network_bytes: u64,

    /// Process uptime
    pub uptime: Duration,
}

/// Aggregate statistics
#[derive(Debug, Clone, Copy)]
pub struct AggregateStats {
    /// Total CPU statistics
    pub total_cpu: CpuStats,

    /// Total memory statistics
    pub total_memory: MemoryStats,

    /// Total I/O statistics
    pub total_io: IoStats,

    /// Total network statistics
    pub total_network: NetworkStats,

    /// Number of processes
    pub num_processes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_stats() {
        let mut stats1 = CpuStats {
            user_time: 100,
            system_time: 50,
            ..Default::default()
        };

        let stats2 = CpuStats {
            user_time: 200,
            system_time: 100,
            ..Default::default()
        };

        stats1.add(stats2);

        assert_eq!(stats1.user_time, 300);
        assert_eq!(stats1.system_time, 150);
        assert_eq!(stats1.total_time(), 450);
    }

    #[test]
    fn test_memory_stats() {
        let mut stats = MemoryStats {
            rss: 1024,
            virtual_size: 4096,
            ..Default::default()
        };

        assert_eq!(stats.total(), 1024);

        stats.add(MemoryStats {
            rss: 2048,
            ..Default::default()
        });

        assert_eq!(stats.rss, 3072);
    }

    #[test]
    fn test_io_stats() {
        let mut stats = IoStats {
            read_bytes: 1000,
            write_bytes: 2000,
            read_ops: 10,
            write_ops: 20,
            ..Default::default()
        };

        assert_eq!(stats.total_bytes(), 3000);
        assert_eq!(stats.total_ops(), 30);
    }

    #[test]
    fn test_process_accounting() {
        let mut acc = ProcessAccounting::new(ProcessId::new(1234));

        acc.update_cpu(CpuStats {
            user_time: 1000,
            system_time: 500,
            ..Default::default()
        });

        assert_eq!(acc.current().cpu.user_time, 1000);

        acc.update_cpu(CpuStats {
            user_time: 1500,
            system_time: 700,
            ..Default::default()
        });

        let delta = acc.delta();
        assert_eq!(delta.cpu.user_time, 500); // 1500 - 1000
    }
}
