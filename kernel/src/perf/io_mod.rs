//! I/O Optimization Manager
//!
//! This module provides a unified I/O optimization management system including:
//! - I/O scheduler selection and switching
//! - I/O statistics aggregation
//! - Throttling and QoS enforcement
//! - Public API exports for all I/O optimization features
//!
//! # Manager Overview
//!
//! The I/O Optimization Manager coordinates all I/O optimization subsystems:
//! - Block I/O optimization (schedulers, caching, throttling)
//! - Network optimization (zero-copy, batching, interrupt moderation)
//! - Filesystem optimization (caching, extents, journaling)
//! - Asynchronous I/O (AIO, io_uring)
//!
//! # Example
//!
//! ```rust
//! use kernel::perf::io_mod::{
//!     init_io_optimizer, get_io_stats, set_io_throttle,
//!     set_io_scheduler, configure_io_optimization
//! };
//!
//! // Initialize I/O optimizer
//! init_io_optimizer()?;
//!
//! // Configure optimization
//! configure_io_optimization(IoOptimizationLevel::HighPerformance)?;
//!
//! // Get statistics
//! let stats = get_io_stats()?;
//! println!("I/O throughput: {} MB/s", stats.throughput);
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::prelude::*;

use super::block::{IoScheduler, BlockStats, BlockError};
use super::network::InterfaceStats;
use super::filesystem::FsStats;
use super::io::{IoError, IoPriority};

/// I/O optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOptimizationLevel {
    /// Power saving mode (low performance, low power)
    PowerSaving,
    /// Balanced mode (default)
    Balanced,
    /// High performance mode
    HighPerformance,
    /// Maximum performance mode (highest power consumption)
    MaxPerformance,
}

impl IoOptimizationLevel {
    /// Get level description
    pub fn description(&self) -> &str {
        match self {
            IoOptimizationLevel::PowerSaving => "Power saving - minimize power consumption",
            IoOptimizationLevel::Balanced => "Balanced - balance performance and power",
            IoOptimizationLevel::HighPerformance => "High performance - maximize throughput",
            IoOptimizationLevel::MaxPerformance => "Max performance - highest throughput, highest power",
        }
    }
}

/// Unified I/O statistics
#[derive(Debug)]
pub struct UnifiedIoStats {
    /// Block I/O statistics
    pub block_stats: BTreeMap<u32, BlockStats>,
    /// Network interface statistics
    pub network_stats: BTreeMap<u32, InterfaceStats>,
    /// Filesystem statistics
    pub fs_stats: FsStats,
    /// Total bytes read
    pub total_bytes_read: AtomicU64,
    /// Total bytes written
    pub total_bytes_written: AtomicU64,
    /// Total I/O operations
    pub total_io_ops: AtomicU64,
    /// Average throughput (bytes/second)
    pub throughput: f64,
    /// Average latency (microseconds)
    pub latency: f64,
    /// I/O queue depth
    pub queue_depth: AtomicUsize,
}

impl Clone for UnifiedIoStats {
    fn clone(&self) -> Self {
        Self {
            block_stats: self.block_stats.clone(),
            network_stats: self.network_stats.clone(),
            fs_stats: self.fs_stats.clone(),
            total_bytes_read: AtomicU64::new(self.total_bytes_read.load(Ordering::Relaxed)),
            total_bytes_written: AtomicU64::new(self.total_bytes_written.load(Ordering::Relaxed)),
            total_io_ops: AtomicU64::new(self.total_io_ops.load(Ordering::Relaxed)),
            throughput: self.throughput,
            latency: self.latency,
            queue_depth: AtomicUsize::new(self.queue_depth.load(Ordering::Relaxed)),
        }
    }
}

impl UnifiedIoStats {
    /// Create new unified I/O statistics
    pub fn new() -> Self {
        Self {
            block_stats: BTreeMap::new(),
            network_stats: BTreeMap::new(),
            fs_stats: FsStats {
                cache_hit_rate: 0.0,
                inode_hit_rate: 0.0,
                extent_enabled: false,
                avg_extents: 0.0,
                lock_stats: crate::perf::filesystem::LockStats::new(),
            },
            total_bytes_read: AtomicU64::new(0),
            total_bytes_written: AtomicU64::new(0),
            total_io_ops: AtomicU64::new(0),
            throughput: 0.0,
            latency: 0.0,
            queue_depth: AtomicUsize::new(0),
        }
    }

    /// Update read statistics
    pub fn update_read(&self, bytes: u64, ops: u64) {
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.total_io_ops.fetch_add(ops, Ordering::Relaxed);
    }

    /// Update write statistics
    pub fn update_write(&self, bytes: u64, ops: u64) {
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.total_io_ops.fetch_add(ops, Ordering::Relaxed);
    }
}

/// I/O throttle configuration
#[derive(Debug)]
pub struct IoThrottleConfig {
    /// Device ID
    pub device_id: u32,
    /// Maximum IOPS (0 = unlimited)
    pub max_iops: u32,
    /// Maximum bandwidth in bytes/second (0 = unlimited)
    pub max_bw: u64,
    /// Burst allowance
    pub burst_bytes: u64,
    /// Throttling enabled
    pub enabled: AtomicBool,
    /// Statistics
    pub stats: ThrottleStats,
}

impl Clone for IoThrottleConfig {
    fn clone(&self) -> Self {
        Self {
            device_id: self.device_id,
            max_iops: self.max_iops,
            max_bw: self.max_bw,
            burst_bytes: self.burst_bytes,
            enabled: AtomicBool::new(self.enabled.load(Ordering::Relaxed)),
            stats: self.stats.clone(),
        }
    }
}

impl IoThrottleConfig {
    /// Create new throttle configuration
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            max_iops: 0,
            max_bw: 0,
            burst_bytes: 0,
            enabled: AtomicBool::new(false),
            stats: ThrottleStats::new(),
        }
    }

    /// Enable throttling
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable throttling
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Check if throttling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Set IOPS limit
    pub fn set_iops_limit(&mut self, iops: u32) {
        self.max_iops = iops;
    }

    /// Set bandwidth limit
    pub fn set_bandwidth_limit(&mut self, bw: u64) {
        self.max_bw = bw;
    }
}

/// Throttle statistics
#[derive(Debug)]
pub struct ThrottleStats {
    /// Throttled operations
    pub throttled_ops: AtomicU64,
    /// Throttled bytes
    pub throttled_bytes: AtomicU64,
}

impl Clone for ThrottleStats {
    fn clone(&self) -> Self {
        Self {
            throttled_ops: AtomicU64::new(self.throttled_ops.load(Ordering::Relaxed)),
            throttled_bytes: AtomicU64::new(self.throttled_bytes.load(Ordering::Relaxed)),
        }
    }
}

impl ThrottleStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            throttled_ops: AtomicU64::new(0),
            throttled_bytes: AtomicU64::new(0),
        }
    }

    /// Record throttled operation
    pub fn record_throttle(&self, bytes: u64) {
        self.throttled_ops.fetch_add(1, Ordering::Relaxed);
        self.throttled_bytes.fetch_add(bytes, Ordering::Relaxed);
    }
}

/// I/O QoS (Quality of Service) configuration
#[derive(Debug, Clone)]
pub struct IoQosConfig {
    /// Minimum IOPS guarantee
    pub min_iops: u32,
    /// Minimum bandwidth guarantee (bytes/second)
    pub min_bw: u64,
    /// Maximum latency target (microseconds)
    pub max_latency_us: u64,
}

impl IoQosConfig {
    /// Create new QoS configuration
    pub fn new() -> Self {
        Self {
            min_iops: 0,
            min_bw: 0,
            max_latency_us: 0,
        }
    }

    /// Set minimum IOPS guarantee
    pub fn set_min_iops(&mut self, iops: u32) {
        self.min_iops = iops;
    }

    /// Set minimum bandwidth guarantee
    pub fn set_min_bw(&mut self, bw: u64) {
        self.min_bw = bw;
    }

    /// Set maximum latency target
    pub fn set_max_latency(&mut self, latency_us: u64) {
        self.max_latency_us = latency_us;
    }
}

/// I/O optimization configuration
#[derive(Debug, Clone)]
pub struct IoOptimizationConfig {
    /// Optimization level
    pub level: IoOptimizationLevel,
    /// Block device configurations
    pub block_configs: BTreeMap<u32, BlockDeviceConfig>,
    /// Network configurations
    pub network_configs: BTreeMap<u32, NetworkDeviceConfig>,
    /// Filesystem configuration
    pub fs_config: FilesystemConfig,
    /// Throttle configurations
    pub throttle_configs: BTreeMap<u32, IoThrottleConfig>,
    /// QoS configurations
    pub qos_configs: BTreeMap<u32, IoQosConfig>,
}

/// Block device configuration
#[derive(Debug, Clone)]
pub struct BlockDeviceConfig {
    /// Device ID
    pub device_id: u32,
    /// I/O scheduler
    pub scheduler: IoScheduler,
    /// Readahead size
    pub readahead_size: u32,
    /// Write-back enabled
    pub writeback_enabled: bool,
    /// Queue depth
    pub queue_depth: usize,
}

impl BlockDeviceConfig {
    /// Create new block device configuration
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            scheduler: IoScheduler::Deadline,
            readahead_size: 256,
            writeback_enabled: true,
            queue_depth: 128,
        }
    }
}

/// Network device configuration
#[derive(Debug, Clone)]
pub struct NetworkDeviceConfig {
    /// Interface ID
    pub interface_id: u32,
    /// RSS enabled
    pub rss_enabled: bool,
    /// Number of RSS queues
    pub rss_queues: u32,
    /// Interrupt moderation enabled
    pub int_moderation: bool,
    /// Busy polling enabled
    pub busy_poll: bool,
    /// Zero-copy enabled
    pub zero_copy: bool,
}

impl NetworkDeviceConfig {
    /// Create new network device configuration
    pub fn new(interface_id: u32) -> Self {
        Self {
            interface_id,
            rss_enabled: false,
            rss_queues: 1,
            int_moderation: false,
            busy_poll: false,
            zero_copy: false,
        }
    }
}

/// Filesystem configuration
#[derive(Debug, Clone)]
pub struct FilesystemConfig {
    /// Dentry cache size
    pub dentry_cache_size: usize,
    /// Inode cache size
    pub inode_cache_size: usize,
    /// Extent allocation enabled
    pub extent_enabled: bool,
    /// Journaling mode
    pub journal_mode: JournalMode,
}

/// Journaling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalMode {
    /// Ordered mode
    Ordered,
    /// Writeback mode
    Writeback,
    /// Journal mode
    Journal,
}

impl FilesystemConfig {
    /// Create new filesystem configuration
    pub fn new() -> Self {
        Self {
            dentry_cache_size: 10000,
            inode_cache_size: 5000,
            extent_enabled: true,
            journal_mode: JournalMode::Ordered,
        }
    }
}

/// I/O optimization manager
#[derive(Debug)]
pub struct IoOptimizer {
    /// Configuration
    config: Mutex<IoOptimizationConfig>,
    /// Unified statistics
    stats: Mutex<UnifiedIoStats>,
    /// Optimizer initialized
    initialized: AtomicBool,
    /// Current optimization level
    opt_level: Mutex<IoOptimizationLevel>,
}

impl Clone for IoOptimizer {
    fn clone(&self) -> Self {
        Self {
            config: Mutex::new(IoOptimizationConfig {
                level: IoOptimizationLevel::Balanced,
                block_configs: BTreeMap::new(),
                network_configs: BTreeMap::new(),
                fs_config: FilesystemConfig::new(),
                throttle_configs: BTreeMap::new(),
                qos_configs: BTreeMap::new(),
            }),
            stats: Mutex::new(UnifiedIoStats::new()),
            initialized: AtomicBool::new(self.initialized.load(Ordering::Relaxed)),
            opt_level: Mutex::new(*self.opt_level.lock()),
        }
    }
}

impl IoOptimizer {
    /// Create new I/O optimizer
    pub fn new() -> Self {
        Self {
            config: Mutex::new(IoOptimizationConfig {
                level: IoOptimizationLevel::Balanced,
                block_configs: BTreeMap::new(),
                network_configs: BTreeMap::new(),
                fs_config: FilesystemConfig::new(),
                throttle_configs: BTreeMap::new(),
                qos_configs: BTreeMap::new(),
            }),
            stats: Mutex::new(UnifiedIoStats::new()),
            initialized: AtomicBool::new(false),
            opt_level: Mutex::new(IoOptimizationLevel::Balanced),
        }
    }

    /// Initialize optimizer
    pub fn init(&self) -> Result<(), IoOptError> {
        if self.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        log::info!("Initializing I/O optimization manager");

        // Initialize subsystems
        self.init_block_devices()?;
        self.init_network_devices()?;
        self.init_filesystem()?;

        self.initialized.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Initialize block devices
    fn init_block_devices(&self) -> Result<(), IoOptError> {
        log::debug!("Initializing block device optimizations");

        // In real implementation, enumerate and configure block devices
        Ok(())
    }

    /// Initialize network devices
    fn init_network_devices(&self) -> Result<(), IoOptError> {
        log::debug!("Initializing network device optimizations");

        // In real implementation, enumerate and configure network devices
        Ok(())
    }

    /// Initialize filesystem optimizations
    fn init_filesystem(&self) -> Result<(), IoOptError> {
        log::debug!("Initializing filesystem optimizations");

        // Initialize caches
        super::filesystem::optimize_dentry_cache(10000)
            .map_err(|_| IoOptError::OperationFailed)?;
        super::filesystem::optimize_inode_cache(5000)
            .map_err(|_| IoOptError::OperationFailed)?;
        super::filesystem::enable_extent_alloc()
            .map_err(|_| IoOptError::OperationFailed)?;
        super::filesystem::init_lock_manager()
            .map_err(|_| IoOptError::OperationFailed)?;

        Ok(())
    }

    /// Set optimization level
    pub fn set_optimization_level(&self, level: IoOptimizationLevel) -> Result<(), IoOptError> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Err(IoOptError::NotInitialized);
        }

        log::info!("Setting I/O optimization level: {:?}", level);

        let mut config = self.config.lock();
        config.level = level;

        // Apply level-specific settings
        match level {
            IoOptimizationLevel::PowerSaving => {
                // Power saving: disable aggressive optimizations
                for (_, block_cfg) in config.block_configs.iter_mut() {
                    block_cfg.readahead_size = 64;
                    block_cfg.queue_depth = 32;
                }
            }
            IoOptimizationLevel::Balanced => {
                // Balanced: moderate optimizations
                for (_, block_cfg) in config.block_configs.iter_mut() {
                    block_cfg.readahead_size = 256;
                    block_cfg.queue_depth = 128;
                }
            }
            IoOptimizationLevel::HighPerformance => {
                // High performance: aggressive optimizations
                for (_, block_cfg) in config.block_configs.iter_mut() {
                    block_cfg.readahead_size = 512;
                    block_cfg.queue_depth = 256;
                }
            }
            IoOptimizationLevel::MaxPerformance => {
                // Maximum performance: most aggressive settings
                for (_, block_cfg) in config.block_configs.iter_mut() {
                    block_cfg.readahead_size = 1024;
                    block_cfg.queue_depth = 512;
                }
            }
        }

        let mut opt_level = self.opt_level.lock();
        *opt_level = level;

        Ok(())
    }

    /// Get optimization level
    pub fn get_optimization_level(&self) -> IoOptimizationLevel {
        *self.opt_level.lock()
    }

    /// Configure block device
    pub fn configure_block_device(&self, config: BlockDeviceConfig) -> Result<(), IoOptError> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Err(IoOptError::NotInitialized);
        }

        log::debug!(
            "Configuring block device {}: scheduler={:?}, readahead={}",
            config.device_id,
            config.scheduler,
            config.readahead_size
        );

        let mut global_config = self.config.lock();
        global_config.block_configs.insert(config.device_id, config);

        Ok(())
    }

    /// Configure network device
    pub fn configure_network_device(&self, config: NetworkDeviceConfig) -> Result<(), IoOptError> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Err(IoOptError::NotInitialized);
        }

        log::debug!(
            "Configuring network interface {}: rss={}, queues={}",
            config.interface_id,
            config.rss_enabled,
            config.rss_queues
        );

        let mut global_config = self.config.lock();
        global_config.network_configs.insert(config.interface_id, config);

        Ok(())
    }

    /// Set I/O throttle
    pub fn set_throttle(&self, device_id: u32, config: IoThrottleConfig) -> Result<(), IoOptError> {
        log::debug!(
            "Setting I/O throttle for device {}: iops={}, bw={}",
            device_id,
            config.max_iops,
            config.max_bw
        );

        let mut global_config = self.config.lock();
        global_config.throttle_configs.insert(device_id, config);

        Ok(())
    }

    /// Set I/O QoS
    pub fn set_qos(&self, device_id: u32, config: IoQosConfig) -> Result<(), IoOptError> {
        log::debug!(
            "Setting I/O QoS for device {}: min_iops={}, min_bw={}",
            device_id,
            config.min_iops,
            config.min_bw
        );

        let mut global_config = self.config.lock();
        global_config.qos_configs.insert(device_id, config);

        Ok(())
    }

    /// Get unified statistics
    pub fn get_stats(&self) -> Result<UnifiedIoStats, IoOptError> {
        if !self.initialized.load(Ordering::Relaxed) {
            return Err(IoOptError::NotInitialized);
        }

        let stats = self.stats.lock();
        Ok(UnifiedIoStats {
            block_stats: stats.block_stats.clone(),
            network_stats: stats.network_stats.clone(),
            fs_stats: stats.fs_stats.clone(),
            total_bytes_read: AtomicU64::new(stats.total_bytes_read.load(Ordering::Relaxed)),
            total_bytes_written: AtomicU64::new(stats.total_bytes_written.load(Ordering::Relaxed)),
            total_io_ops: AtomicU64::new(stats.total_io_ops.load(Ordering::Relaxed)),
            throughput: stats.throughput,
            latency: stats.latency,
            queue_depth: AtomicUsize::new(stats.queue_depth.load(Ordering::Relaxed)),
        })
    }

    /// Update statistics
    pub fn update_stats(&self, bytes_read: u64, bytes_written: u64, ops: u64) {
        let stats = self.stats.lock();
        stats.update_read(bytes_read, ops);
        stats.update_write(bytes_written, ops);
    }
}

/// I/O optimization error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOptError {
    /// Not initialized
    NotInitialized,
    /// Invalid configuration
    InvalidConfig,
    /// Device not found
    DeviceNotFound,
    /// Operation failed
    OperationFailed,
    /// Not supported
    NotSupported,
    /// No memory
    NoMemory,
}

impl core::fmt::Display for IoOptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IoOptError::NotInitialized => write!(f, "I/O optimizer not initialized"),
            IoOptError::InvalidConfig => write!(f, "Invalid configuration"),
            IoOptError::DeviceNotFound => write!(f, "Device not found"),
            IoOptError::OperationFailed => write!(f, "Operation failed"),
            IoOptError::NotSupported => write!(f, "Operation not supported"),
            IoOptError::NoMemory => write!(f, "Out of memory"),
        }
    }
}

/// Global I/O optimizer instance
static GLOBAL_OPTIMIZER: Mutex<Option<IoOptimizer>> = Mutex::new(None);

/// Initialize I/O optimizer
pub fn init_io_optimizer() -> Result<(), IoOptError> {
    let mut global = GLOBAL_OPTIMIZER.lock();

    if global.is_some() {
        return Ok(());
    }

    let optimizer = IoOptimizer::new();
    optimizer.init()?;

    *global = Some(optimizer);

    log::info!("I/O optimization manager initialized");
    Ok(())
}

/// Get global I/O optimizer
pub fn get_io_optimizer() -> Option<IoOptimizer> {
    GLOBAL_OPTIMIZER.lock().as_ref().cloned()
}

/// Configure I/O optimization
pub fn configure_io_optimization(level: IoOptimizationLevel) -> Result<(), IoOptError> {
    let optimizer = get_io_optimizer().ok_or(IoOptError::NotInitialized)?;
    optimizer.set_optimization_level(level)
}

/// Set I/O scheduler for device
pub fn set_io_scheduler(device_id: u32, scheduler: IoScheduler) -> Result<(), BlockError> {
    super::block::set_io_scheduler(device_id, scheduler)
}

/// Set I/O throttle
pub fn set_io_throttle(device_id: u32, max_iops: u32, max_bw: u64) -> Result<(), IoOptError> {
    let optimizer = get_io_optimizer().ok_or(IoOptError::NotInitialized)?;

    let mut config = IoThrottleConfig::new(device_id);
    config.set_iops_limit(max_iops);
    config.set_bandwidth_limit(max_bw);
    config.enable();

    optimizer.set_throttle(device_id, config)
}

/// Get unified I/O statistics
pub fn get_io_stats() -> Result<UnifiedIoStats, IoOptError> {
    let optimizer = get_io_optimizer().ok_or(IoOptError::NotInitialized)?;
    optimizer.get_stats()
}

/// Configure block device
pub fn configure_block_device(
    device_id: u32,
    scheduler: IoScheduler,
    readahead: u32,
    queue_depth: usize,
) -> Result<(), IoOptError> {
    let optimizer = get_io_optimizer().ok_or(IoOptError::NotInitialized)?;

    let config = BlockDeviceConfig {
        device_id,
        scheduler,
        readahead_size: readahead,
        writeback_enabled: true,
        queue_depth,
    };

    optimizer.configure_block_device(config)
}

/// Configure network device
pub fn configure_network_device(
    interface_id: u32,
    rss_enabled: bool,
    rss_queues: u32,
    busy_poll: bool,
) -> Result<(), IoOptError> {
    let optimizer = get_io_optimizer().ok_or(IoOptError::NotInitialized)?;

    let config = NetworkDeviceConfig {
        interface_id,
        rss_enabled,
        rss_queues,
        int_moderation: true,
        busy_poll,
        zero_copy: false,
    };

    optimizer.configure_network_device(config)
}

/// Set I/O priority
pub fn set_io_priority_global(pid: u32, priority: IoPriority) -> Result<(), IoError> {
    super::io::set_io_priority(pid, priority)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_init() {
        let optimizer = IoOptimizer::new();
        assert!(optimizer.init().is_ok());
        assert!(optimizer.initialized.load(Ordering::Relaxed));
    }

    #[test]
    fn test_optimization_levels() {
        assert_eq!(
            IoOptimizationLevel::Balanced.description(),
            "Balanced - balance performance and power"
        );
    }

    #[test]
    fn test_throttle_config() {
        let mut config = IoThrottleConfig::new(0);
        config.set_iops_limit(1000);
        config.set_bandwidth_limit(1024 * 1024 * 100); // 100 MB/s

        assert_eq!(config.max_iops, 1000);
        assert_eq!(config.max_bw, 1024 * 1024 * 100);

        config.enable();
        assert!(config.is_enabled());
    }

    #[test]
    fn test_unified_stats() {
        let stats = UnifiedIoStats::new();

        stats.update_read(4096, 1);
        stats.update_write(4096, 1);

        assert_eq!(stats.total_bytes_read.load(Ordering::Relaxed), 4096);
        assert_eq!(stats.total_bytes_written.load(Ordering::Relaxed), 4096);
        assert_eq!(stats.total_io_ops.load(Ordering::Relaxed), 2);
    }
}
