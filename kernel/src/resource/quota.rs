//! Resource quota enforcement
//!
//! This module implements quota enforcement for CPU, memory, and I/O resources.
//! Quotas provide hard limits on resource consumption for processes, cgroups,
//! and resource pools.
//!
//! # Overview
//!
//! Quotas enforce resource limits by:
//! - Tracking resource usage over time windows
//! - Enforcing hard limits on consumption
//! - Throttling or blocking when quotas are exceeded
//! - Providing quota statistics and accounting
//!
//! # Architecture
//!
//! ```text
//! QuotaManager
//!     ├── CpuQuota (CPU time limits)
//!     ├── MemoryQuota (memory limits)
//!     └── IoQuota (I/O bandwidth limits)
//!
//! Each quota type:
//!     ├── Usage tracking
//!     ├── Limit enforcement
//!     ├── Throttling logic
//!     └── Statistics
//! ```
//!
//! # Quota Types
//!
//! ## CPU Quota
//! - CPU time per period (e.g., 50ms per 100ms)
//! - Per-core and aggregate limits
//! - Real-time vs. normal tasks
//!
//! ## Memory Quota
//! - Resident set size (RSS) limits
//! - Virtual memory size limits
//! - Swap usage limits
//!
//! ## I/O Quota
//! - Bytes per second limits
//! - IOPS limits
//! - Per-device and aggregate limits
//!
//! # Examples
//!
//! ```no_run
//! use kernel::resource::quota::{QuotaManager, QuotaType, QuotaConfig};
//!
//! let manager = QuotaManager::new();
//!
//! // Set CPU quota: 50ms per 100ms period
//! manager.set_quota(
//!     QuotaType::Cpu,
//!     QuotaConfig::CpuQuota {
//!         period_micros: 100_000,
//!         quota_micros: 50_000,
//!     }
//! );
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::Error;
use crate::process::ProcessId;
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;

/// Default quota period in microseconds (100ms)
const DEFAULT_QUOTA_PERIOD: u64 = 100_000;

/// Default CPU quota in microseconds per period (unlimited)
const DEFAULT_CPU_QUOTA: u64 = u64::MAX;

/// Default memory quota in bytes (unlimited)
const DEFAULT_MEMORY_QUOTA: u64 = u64::MAX;

/// Default I/O bandwidth quota in bytes/sec (unlimited)
const DEFAULT_IO_QUOTA: u64 = u64::MAX;

/// Default IOPS quota (unlimited)
const DEFAULT_IOPS_QUOTA: u64 = u64::MAX;

/// Quota violation action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuotaAction {
    /// Block the request until quota is available
    Block,

    /// Throttle the resource
    Throttle,

    /// Return an error
    Error,

    /// Kill the process
    Kill,

    /// Log a warning but allow
    Log,
}

/// Quota type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuotaType {
    /// CPU time quota
    Cpu,

    /// Memory quota
    Memory,

    /// I/O bandwidth quota
    IoBandwidth,

    /// I/O operations quota
    IoOps,
}

impl QuotaType {
    /// Get all quota types
    pub fn all() -> &'static [QuotaType] {
        &[
            QuotaType::Cpu,
            QuotaType::Memory,
            QuotaType::IoBandwidth,
            QuotaType::IoOps,
        ]
    }

    /// Get quota type name
    pub fn name(&self) -> &'static str {
        match self {
            QuotaType::Cpu => "cpu",
            QuotaType::Memory => "memory",
            QuotaType::IoBandwidth => "io_bandwidth",
            QuotaType::IoOps => "io_ops",
        }
    }
}

/// CPU quota configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuQuotaConfig {
    /// Period in microseconds
    pub period_micros: u64,

    /// Quota in microseconds per period (None = unlimited)
    pub quota_micros: Option<u64>,

    /// Number of CPU cores
    pub num_cores: u64,

    /// Action when quota exceeded
    pub action: QuotaAction,
}

impl Default for CpuQuotaConfig {
    fn default() -> Self {
        Self {
            period_micros: DEFAULT_QUOTA_PERIOD,
            quota_micros: None,
            num_cores: 1,
            action: QuotaAction::Throttle,
        }
    }
}

/// Memory quota configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryQuotaConfig {
    /// Maximum RSS in bytes (None = unlimited)
    pub max_rss: Option<u64>,

    /// Maximum virtual memory in bytes (None = unlimited)
    pub max_virtual: Option<u64>,

    /// Maximum swap in bytes (None = unlimited)
    pub max_swap: Option<u64>,

    /// Action when quota exceeded
    pub action: QuotaAction,
}

impl Default for MemoryQuotaConfig {
    fn default() -> Self {
        Self {
            max_rss: None,
            max_virtual: None,
            max_swap: None,
            action: QuotaAction::Kill,
        }
    }
}

/// I/O quota configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoQuotaConfig {
    /// Maximum read bandwidth in bytes/sec (None = unlimited)
    pub max_read_bw: Option<u64>,

    /// Maximum write bandwidth in bytes/sec (None = unlimited)
    pub max_write_bw: Option<u64>,

    /// Maximum read IOPS (None = unlimited)
    pub max_read_iops: Option<u64>,

    /// Maximum write IOPS (None = unlimited)
    pub max_write_iops: Option<u64>,

    /// Action when quota exceeded
    pub action: QuotaAction,
}

impl Default for IoQuotaConfig {
    fn default() -> Self {
        Self {
            max_read_bw: None,
            max_write_bw: None,
            max_read_iops: None,
            max_write_iops: None,
            action: QuotaAction::Throttle,
        }
    }
}

/// Quota configuration (union type)
#[derive(Debug, Clone, Copy)]
pub enum QuotaConfig {
    Cpu(CpuQuotaConfig),
    Memory(MemoryQuotaConfig),
    Io(IoQuotaConfig),
}

/// CPU quota state
#[derive(Debug, Clone, Copy)]
pub struct CpuQuotaState {
    /// Configuration
    config: CpuQuotaConfig,

    /// CPU usage in current period (microseconds)
    period_usage: u64,

    /// Total CPU usage (microseconds)
    total_usage: u64,

    /// Number of throttles
    throttles: u64,

    /// Throttled time (microseconds)
    throttled_time: u64,

    /// Period start time
    period_start: Duration,

    /// Whether quota is active
    active: bool,
}

impl CpuQuotaState {
    /// Create a new CPU quota state
    pub fn new(config: CpuQuotaConfig) -> Self {
        Self {
            config,
            period_usage: 0,
            total_usage: 0,
            throttles: 0,
            throttled_time: 0,
            period_start: Duration::from_secs(0),
            active: true,
        }
    }

    /// Check if CPU time is available
    pub fn can_run(&self, time: u64) -> bool {
        if !self.active {
            return true;
        }

        if let Some(quota) = self.config.quota_micros {
            self.period_usage.saturating_add(time) <= quota
        } else {
            true
        }
    }

    /// Add CPU usage
    pub fn add_usage(&mut self, time: u64) -> Result<(), Error> {
        if !self.active {
            return Ok(());
        }

        self.total_usage = self.total_usage.saturating_add(time);

        if let Some(quota) = self.config.quota_micros {
            let new_usage = self.period_usage.saturating_add(time);

            if new_usage > quota {
                match self.config.action {
                    QuotaAction::Throttle => {
                        self.throttles += 1;
                        return Err(Error::QuotaExceeded);
                    }
                    QuotaAction::Kill => {
                        return Err(Error::ProcessError(crate::error::unified::ProcessError::ProcessKilled));
                    }
                    QuotaAction::Error => {
                        return Err(Error::QuotaExceeded);
                    }
                    _ => {}
                }
            }

            self.period_usage = new_usage;
        } else {
            self.period_usage = self.period_usage.saturating_add(time);
        }

        Ok(())
    }

    /// Reset period usage (called at end of period)
    pub fn reset_period(&mut self) {
        self.period_usage = 0;
        self.period_start = Duration::from_secs(0);
    }

    /// Get CPU usage percentage
    pub fn usage_percent(&self) -> f64 {
        if let Some(quota) = self.config.quota_micros {
            if quota > 0 {
                (self.period_usage as f64 / quota as f64) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

/// Memory quota state
#[derive(Debug, Clone, Copy)]
pub struct MemoryQuotaState {
    /// Configuration
    config: MemoryQuotaConfig,

    /// Current RSS usage
    rss_usage: u64,

    /// Current virtual memory usage
    virtual_usage: u64,

    /// Current swap usage
    swap_usage: u64,

    /// Number of OOM events
    oom_events: u64,

    /// Whether quota is active
    active: bool,
}

impl MemoryQuotaState {
    /// Create a new memory quota state
    pub fn new(config: MemoryQuotaConfig) -> Self {
        Self {
            config,
            rss_usage: 0,
            virtual_usage: 0,
            swap_usage: 0,
            oom_events: 0,
            active: true,
        }
    }

    /// Check if memory can be allocated
    pub fn can_allocate(&self, size: u64) -> bool {
        if !self.active {
            return true;
        }

        if let Some(max_rss) = self.config.max_rss {
            if self.rss_usage.saturating_add(size) > max_rss {
                return false;
            }
        }

        if let Some(max_virtual) = self.config.max_virtual {
            if self.virtual_usage.saturating_add(size) > max_virtual {
                return false;
            }
        }

        true
    }

    /// Add memory usage
    pub fn add_usage(&mut self, rss: u64, virtual_mem: u64) -> Result<(), Error> {
        if !self.active {
            return Ok(());
        }

        // Check RSS limit
        if let Some(max_rss) = self.config.max_rss {
            let new_rss = self.rss_usage.saturating_add(rss);
            if new_rss > max_rss {
                self.oom_events += 1;
                return Err(Error::QuotaExceeded);
            }
            self.rss_usage = new_rss;
        } else {
            self.rss_usage = self.rss_usage.saturating_add(rss);
        }

        // Check virtual memory limit
        if let Some(max_virtual) = self.config.max_virtual {
            let new_virtual = self.virtual_usage.saturating_add(virtual_mem);
            if new_virtual > max_virtual {
                self.oom_events += 1;
                return Err(Error::QuotaExceeded);
            }
            self.virtual_usage = new_virtual;
        } else {
            self.virtual_usage = self.virtual_usage.saturating_add(virtual_mem);
        }

        Ok(())
    }

    /// Free memory
    pub fn free(&mut self, rss: u64, virtual_mem: u64) {
        self.rss_usage = self.rss_usage.saturating_sub(rss);
        self.virtual_usage = self.virtual_usage.saturating_sub(virtual_mem);
    }

    /// Add swap usage
    pub fn add_swap(&mut self, swap: u64) -> Result<(), Error> {
        if let Some(max_swap) = self.config.max_swap {
            let new_swap = self.swap_usage.saturating_add(swap);
            if new_swap > max_swap {
                self.oom_events += 1;
                return Err(Error::QuotaExceeded);
            }
            self.swap_usage = new_swap;
        } else {
            self.swap_usage = self.swap_usage.saturating_add(swap);
        }
        Ok(())
    }

    /// Check if at limit
    pub fn at_limit(&self) -> bool {
        if let Some(max_rss) = self.config.max_rss {
            if self.rss_usage >= max_rss {
                return true;
            }
        }
        if let Some(max_virtual) = self.config.max_virtual {
            if self.virtual_usage >= max_virtual {
                return true;
            }
        }
        false
    }
}

/// I/O quota state
#[derive(Debug, Clone, Copy)]
pub struct IoQuotaState {
    /// Configuration
    config: IoQuotaConfig,

    /// Read bytes in current window
    read_bytes: u64,

    /// Write bytes in current window
    write_bytes: u64,

    /// Read IOPS in current window
    read_iops: u64,

    /// Write IOPS in current window
    write_iops: u64,

    /// Total read bytes
    total_read_bytes: u64,

    /// Total write bytes
    total_write_bytes: u64,

    /// Number of throttles
    throttles: u64,

    /// Window start time
    window_start: Duration,

    /// Whether quota is active
    active: bool,
}

impl IoQuotaState {
    /// Create a new I/O quota state
    pub fn new(config: IoQuotaConfig) -> Self {
        Self {
            config,
            read_bytes: 0,
            write_bytes: 0,
            read_iops: 0,
            write_iops: 0,
            total_read_bytes: 0,
            total_write_bytes: 0,
            throttles: 0,
            window_start: Duration::from_secs(0),
            active: true,
        }
    }

    /// Check if I/O can be performed
    pub fn can_io(&self, read_bytes: u64, write_bytes: u64) -> bool {
        if !self.active {
            return true;
        }

        if let Some(max_read) = self.config.max_read_bw {
            if self.read_bytes.saturating_add(read_bytes) > max_read {
                return false;
            }
        }

        if let Some(max_write) = self.config.max_write_bw {
            if self.write_bytes.saturating_add(write_bytes) > max_write {
                return false;
            }
        }

        true
    }

    /// Add I/O usage
    pub fn add_io(
        &mut self,
        read_bytes: u64,
        write_bytes: u64,
        read_ops: u64,
        write_ops: u64,
    ) -> Result<(), Error> {
        if !self.active {
            return Ok(());
        }

        // Check read bandwidth
        if let Some(max_read) = self.config.max_read_bw {
            let new_read = self.read_bytes.saturating_add(read_bytes);
            if new_read > max_read {
                self.throttles += 1;
                return Err(Error::IoQuotaExceeded {
                    operation: "read".to_string(),
                    usage: new_read,
                    quota: max_read,
                });
            }
            self.read_bytes = new_read;
        } else {
            self.read_bytes = self.read_bytes.saturating_add(read_bytes);
        }

        // Check write bandwidth
        if let Some(max_write) = self.config.max_write_bw {
            let new_write = self.write_bytes.saturating_add(write_bytes);
            if new_write > max_write {
                self.throttles += 1;
                return Err(Error::IoQuotaExceeded {
                    operation: "write".to_string(),
                    usage: new_write,
                    quota: max_write,
                });
            }
            self.write_bytes = new_write;
        } else {
            self.write_bytes = self.write_bytes.saturating_add(write_bytes);
        }

        // Check read IOPS
        if let Some(max_read_iops) = self.config.max_read_iops {
            let new_iops = self.read_iops.saturating_add(read_ops);
            if new_iops > max_read_iops {
                self.throttles += 1;
                return Err(Error::IoQuotaExceeded {
                    operation: "read_iops".to_string(),
                    usage: new_iops,
                    quota: max_read_iops,
                });
            }
            self.read_iops = new_iops;
        } else {
            self.read_iops = self.read_iops.saturating_add(read_ops);
        }

        // Check write IOPS
        if let Some(max_write_iops) = self.config.max_write_iops {
            let new_iops = self.write_iops.saturating_add(write_ops);
            if new_iops > max_write_iops {
                self.throttles += 1;
                return Err(Error::IoQuotaExceeded {
                    operation: "write_iops".to_string(),
                    usage: new_iops,
                    quota: max_write_iops,
                });
            }
            self.write_iops = new_iops;
        } else {
            self.write_iops = self.write_iops.saturating_add(write_ops);
        }

        self.total_read_bytes = self.total_read_bytes.saturating_add(read_bytes);
        self.total_write_bytes = self.total_write_bytes.saturating_add(write_bytes);

        Ok(())
    }

    /// Reset window (called periodically)
    pub fn reset_window(&mut self) {
        self.read_bytes = 0;
        self.write_bytes = 0;
        self.read_iops = 0;
        self.write_iops = 0;
        self.window_start = Duration::from_secs(0);
    }
}

/// Quota statistics
#[derive(Debug, Clone, Copy)]
pub struct QuotaStats {
    /// Quota type
    pub quota_type: QuotaType,

    /// Current usage
    pub current_usage: u64,

    /// Quota limit
    pub quota_limit: Option<u64>,

    /// Usage percentage
    pub usage_percent: f64,

    /// Number of violations
    pub violations: u64,

    /// Whether quota is active
    pub active: bool,
}

/// Per-process quotas
pub struct ProcessQuotas {
    /// CPU quota
    cpu: SpinLock<CpuQuotaState>,

    /// Memory quota
    memory: SpinLock<MemoryQuotaState>,

    /// I/O quota
    io: SpinLock<IoQuotaState>,

    /// Process ID
    pid: ProcessId,
}

impl core::fmt::Debug for ProcessQuotas {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProcessQuotas")
            .field("pid", &self.pid)
            .finish()
    }
}

impl ProcessQuotas {
    /// Create a new process quotas
    pub fn new(
        pid: ProcessId,
        cpu_config: CpuQuotaConfig,
        memory_config: MemoryQuotaConfig,
        io_config: IoQuotaConfig,
    ) -> Self {
        Self {
            cpu: SpinLock::new(CpuQuotaState::new(cpu_config)),
            memory: SpinLock::new(MemoryQuotaState::new(memory_config)),
            io: SpinLock::new(IoQuotaState::new(io_config)),
            pid,
        }
    }

    /// Get process ID
    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    /// Get CPU quota
    pub fn cpu(&self) -> &SpinLock<CpuQuotaState> {
        &self.cpu
    }

    /// Get memory quota
    pub fn memory(&self) -> &SpinLock<MemoryQuotaState> {
        &self.memory
    }

    /// Get I/O quota
    pub fn io(&self) -> &SpinLock<IoQuotaState> {
        &self.io
    }

    /// Get all quota statistics
    pub fn stats(&self) -> Vec<QuotaStats> {
        vec![
            QuotaStats {
                quota_type: QuotaType::Cpu,
                current_usage: self.cpu.lock().period_usage,
                quota_limit: self.cpu.lock().config.quota_micros,
                usage_percent: self.cpu.lock().usage_percent(),
                violations: self.cpu.lock().throttles,
                active: self.cpu.lock().active,
            },
            QuotaStats {
                quota_type: QuotaType::Memory,
                current_usage: self.memory.lock().rss_usage,
                quota_limit: self.memory.lock().config.max_rss,
                usage_percent: {
                    let mem = self.memory.lock();
                    if let Some(max) = mem.config.max_rss {
                        if max > 0 {
                            (mem.rss_usage as f64 / max as f64) * 100.0
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    }
                },
                violations: self.memory.lock().oom_events,
                active: self.memory.lock().active,
            },
            QuotaStats {
                quota_type: QuotaType::IoBandwidth,
                current_usage: self.io.lock().read_bytes.saturating_add(self.io.lock().write_bytes),
                quota_limit: self
                    .io
                    .lock()
                    .config
                    .max_read_bw
                    .or(self.io.lock().config.max_write_bw),
                usage_percent: 0.0, // Complex to compute for I/O
                violations: self.io.lock().throttles,
                active: self.io.lock().active,
            },
        ]
    }
}

/// Quota manager
pub struct QuotaManager {
    /// Per-process quotas
    process_quotas: SpinLock<BTreeMap<ProcessId, Arc<ProcessQuotas>>>,

    /// Default CPU quota config
    default_cpu: CpuQuotaConfig,

    /// Default memory quota config
    default_memory: MemoryQuotaConfig,

    /// Default I/O quota config
    default_io: IoQuotaConfig,
}

impl QuotaManager {
    /// Create a new quota manager
    pub fn new() -> Self {
        Self {
            process_quotas: SpinLock::new(BTreeMap::new()),
            default_cpu: CpuQuotaConfig::default(),
            default_memory: MemoryQuotaConfig::default(),
            default_io: IoQuotaConfig::default(),
        }
    }

    /// Create quotas for a process
    pub fn create_quotas(&self, pid: ProcessId) -> Arc<ProcessQuotas> {
        let quotas = Arc::new(ProcessQuotas::new(
            pid,
            self.default_cpu,
            self.default_memory,
            self.default_io,
        ));

        self.process_quotas.lock().insert(pid, quotas.clone());
        quotas
    }

    /// Get quotas for a process
    pub fn get_quotas(&self, pid: ProcessId) -> Option<Arc<ProcessQuotas>> {
        self.process_quotas.lock().get(&pid).cloned()
    }

    /// Remove quotas for a process
    pub fn remove_quotas(&self, pid: ProcessId) {
        self.process_quotas.lock().remove(&pid);
    }

    /// Set default CPU quota
    pub fn set_default_cpu_quota(&mut self, config: CpuQuotaConfig) {
        self.default_cpu = config;
    }

    /// Set default memory quota
    pub fn set_default_memory_quota(&mut self, config: MemoryQuotaConfig) {
        self.default_memory = config;
    }

    /// Set default I/O quota
    pub fn set_default_io_quota(&mut self, config: IoQuotaConfig) {
        self.default_io = config;
    }

    /// Update all quotas (called periodically)
    pub fn update_quotas(&self) {
        for quotas in self.process_quotas.lock().values() {
            quotas.cpu.lock().reset_period();
            quotas.io.lock().reset_window();
        }
    }

    /// Get all quota statistics
    pub fn get_all_stats(&self) -> Vec<(ProcessId, Vec<QuotaStats>)> {
        self.process_quotas
            .lock()
            .iter()
            .map(|(&pid, quotas)| (pid, quotas.stats()))
            .collect()
    }

    /// Find processes exceeding quotas
    pub fn find_exceeding(&self) -> Vec<(ProcessId, QuotaType, u64, u64)> {
        let mut results = Vec::new();

        for (&pid, quotas) in self.process_quotas.lock().iter() {
            let cpu = quotas.cpu.lock();
            if let Some(quota) = cpu.config.quota_micros {
                if cpu.period_usage > quota {
                    results.push((pid, QuotaType::Cpu, cpu.period_usage, quota));
                }
            }
            drop(cpu);

            let memory = quotas.memory.lock();
            if let Some(quota) = memory.config.max_rss {
                if memory.rss_usage > quota {
                    results.push((pid, QuotaType::Memory, memory.rss_usage, quota));
                }
            }
        }

        results
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_quota() {
        let config = CpuQuotaConfig {
            period_micros: 100_000,
            quota_micros: Some(50_000),
            ..Default::default()
        };

        let mut quota = CpuQuotaState::new(config);

        assert!(quota.can_run(30_000));
        assert!(quota.add_usage(30_000).is_ok());
        assert!(quota.can_run(20_000));
        assert!(quota.add_usage(20_000).is_ok());

        assert!(!quota.can_run(10_000));
        assert!(quota.add_usage(10_000).is_err());
    }

    #[test]
    fn test_memory_quota() {
        let config = MemoryQuotaConfig {
            max_rss: Some(1024),
            ..Default::default()
        };

        let mut quota = MemoryQuotaState::new(config);

        assert!(quota.can_allocate(512));
        assert!(quota.add_usage(512, 512).is_ok());
        assert_eq!(quota.rss_usage, 512);

        assert!(quota.can_allocate(512));
        assert!(quota.add_usage(512, 512).is_ok());

        assert!(!quota.can_allocate(100));
        assert!(quota.add_usage(100, 100).is_err());
    }

    #[test]
    fn test_io_quota() {
        let config = IoQuotaConfig {
            max_read_bw: Some(1000),
            max_write_bw: Some(1000),
            ..Default::default()
        };

        let mut quota = IoQuotaState::new(config);

        assert!(quota.can_io(500, 500));
        assert!(quota.add_io(500, 500, 10, 10).is_ok());

        assert!(!quota.can_io(600, 0));
        assert!(quota.add_io(600, 0, 0, 0).is_err());
    }
}
