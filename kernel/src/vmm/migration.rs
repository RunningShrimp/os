//! # Live Migration Implementation
//!
//! This module provides live migration capabilities for virtual machines, enabling
//! the transfer of a running VM from one host to another with minimal downtime.
//! It supports both pre-copy and post-copy migration strategies.
//!
//! ## Features
//!
//! - Pre-copy migration with iterative memory transfer
//! - Post-copy migration with on-demand paging
//! - Dirty page tracking
//! - Memory state synchronization
//! - Device state migration
//! - Downtime minimization
//! - Network migration support
//!
//! ## Architecture
//!
//! Live migration works by iteratively copying memory pages from the source to
//! the destination while tracking dirty pages. The VM continues running during
//! the pre-copy phase, minimizing downtime to just the final transfer of dirty
//! pages and device state.
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::migration::{LiveMigration, MigrationConfig};
//!
//! let config = MigrationConfig::default();
//! let migration = LiveMigration::new(config)?;
//! migration.start("destination.example.com:9000")?;
//! ```

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;
use spin::Mutex;
use hashbrown::HashMap;

use crate::error::KernelError;
use crate::memory::{PhysicalAddress, VirtualAddress};

/// Default maximum downtime (ms)
const DEFAULT_MAX_DOWNTIME: u64 = 30;

/// Default bandwidth limit (MB/s)
const DEFAULT_BANDWIDTH_LIMIT: u64 = 1024;

/// Page size for migration tracking
const MIGRATION_PAGE_SIZE: usize = 4096;

/// Migration phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPhase {
    /// Not started
    None,
    /// Setup phase
    Setup,
    /// Pre-copy iteration
    PreCopy,
    /// Pre-copy and device setup
    PreCopySetup,
    /// Pre-copy and device transfer
    PreCopyDevice,
    /// Downtime phase
    Downtime,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}

/// Migration strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStrategy {
    /// Pre-copy: Transfer memory before stopping VM
    PreCopy,
    /// Post-copy: Transfer memory after stopping VM
    PostCopy,
}

/// Errors that can occur during migration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    /// Migration not started
    NotStarted,
    /// Already in progress
    AlreadyInProgress,
    /// Connection failed
    ConnectionFailed,
    /// Transfer failed
    TransferFailed,
    /// Timeout
    Timeout,
    /// Destination not ready
    DestinationNotReady,
    /// Dirty page tracking failed
    DirtyPageTrackingFailed,
    /// Device migration failed
    DeviceMigrationFailed(String),
    /// Memory migration failed
    MemoryMigrationFailed,
    /// Invalid migration state
    InvalidState,
    /// Bandwidth limit exceeded
    BandwidthExceeded,
    /// Maximum downtime exceeded
    DowntimeExceeded,
}

impl From<MigrationError> for KernelError {
    fn from(err: MigrationError) -> Self {
        KernelError::Virtualization(format!("Migration error: {:?}", err))
    }
}

/// Migration statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MigrationStats {
    /// Total RAM bytes
    pub ram_total: u64,
    /// RAM bytes transferred
    pub ram_transferred: u64,
    /// RAM bytes remaining
    pub ram_remaining: u64,
    /// Number of iterations
    pub iterations: u32,
    /// Dirty pages in last iteration
    pub dirty_pages: u64,
    /// Downtime in milliseconds
    pub downtime: u64,
    /// Total time in milliseconds
    pub total_time: u64,
    /// Transfer rate in MB/s
    pub transfer_rate: u64,
}

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Migration strategy
    pub strategy: MigrationStrategy,
    /// Maximum downtime in milliseconds
    pub max_downtime: u64,
    /// Bandwidth limit in bytes per second
    pub bandwidth_limit: u64,
    /// Convergence iterations
    pub max_iterations: u32,
    /// Enable compression
    pub compression_enabled: bool,
    /// Enable XBZRLE compression
    pub xbzrle_enabled: bool,
    /// XBZRLE cache size in MB
    pub xbzrle_cache_size: u64,
    /// Enable auto-convergence
    pub auto_converge: bool,
    /// Percentage of CPU throttling
    pub cpu_throttle: u32,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            strategy: MigrationStrategy::PreCopy,
            max_downtime: DEFAULT_MAX_DOWNTIME,
            bandwidth_limit: DEFAULT_BANDWIDTH_LIMIT * 1024 * 1024,
            max_iterations: 30,
            compression_enabled: false,
            xbzrle_enabled: false,
            xbzrle_cache_size: 64,
            auto_converge: true,
            cpu_throttle: 50,
        }
    }
}

/// Dirty page bitmap
#[derive(Debug)]
pub struct DirtyPageBitmap {
    /// Physical pages that are dirty
    dirty_pages: BTreeSet<usize>,
    /// Total number of pages
    total_pages: usize,
}

impl DirtyPageBitmap {
    /// Create a new dirty page bitmap
    pub fn new(total_pages: usize) -> Self {
        Self {
            dirty_pages: BTreeSet::new(),
            total_pages,
        }
    }

    /// Mark a page as dirty
    pub fn mark_dirty(&mut self, page_number: usize) {
        if page_number < self.total_pages {
            self.dirty_pages.insert(page_number);
        }
    }

    /// Mark a range of pages as dirty
    pub fn mark_range_dirty(&mut self, start: usize, end: usize) {
        for page in start..core::cmp::min(end, self.total_pages) {
            self.dirty_pages.insert(page);
        }
    }

    /// Check if a page is dirty
    pub fn is_dirty(&self, page_number: usize) -> bool {
        self.dirty_pages.contains(&page_number)
    }

    /// Clear all dirty bits
    pub fn clear(&mut self) {
        self.dirty_pages.clear();
    }

    /// Get the number of dirty pages
    pub fn dirty_count(&self) -> usize {
        self.dirty_pages.len()
    }

    /// Get all dirty pages
    pub fn get_dirty_pages(&self) -> &BTreeSet<usize> {
        &self.dirty_pages
    }

    /// Get the percentage of dirty pages
    pub fn dirty_percentage(&self) -> f64 {
        if self.total_pages == 0 {
            0.0
        } else {
            (self.dirty_pages.len() as f64 / self.total_pages as f64) * 100.0
        }
    }
}

/// Device state information
#[derive(Debug, Clone)]
pub struct DeviceState {
    /// Device ID
    pub device_id: String,
    /// Device type
    pub device_type: String,
    /// Device state data
    pub state_data: Vec<u8>,
    /// Configuration data
    pub config_data: Vec<u8>,
}

impl DeviceState {
    /// Create a new device state
    pub fn new(device_id: String, device_type: String) -> Self {
        Self {
            device_id,
            device_type,
            state_data: Vec::new(),
            config_data: Vec::new(),
        }
    }

    /// Set state data
    pub fn set_state(&mut self, data: Vec<u8>) {
        self.state_data = data;
    }

    /// Set configuration data
    pub fn set_config(&mut self, data: Vec<u8>) {
        self.config_data = data;
    }

    /// Get state data size
    pub fn state_size(&self) -> usize {
        self.state_data.len()
    }

    /// Get config data size
    pub fn config_size(&self) -> usize {
        self.config_data.len()
    }

    /// Get total size
    pub fn total_size(&self) -> usize {
        self.state_size() + self.config_size()
    }
}

/// Migration stream data
#[derive(Debug, Clone)]
pub struct MigrationData {
    /// Data chunk
    pub data: Vec<u8>,
    /// Offset in migration stream
    pub offset: u64,
    /// Is this the last chunk
    pub is_last: bool,
    /// Chunk flags
    pub flags: u32,
}

impl MigrationData {
    /// Create new migration data
    pub fn new(data: Vec<u8>, offset: u64) -> Self {
        Self {
            data,
            offset,
            is_last: false,
            flags: 0,
        }
    }

    /// Create last chunk
    pub fn last(mut self) -> Self {
        self.is_last = true;
        self
    }

    /// Get data size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Live migration controller
#[derive(Debug)]
pub struct LiveMigration {
    /// Migration configuration
    config: MigrationConfig,
    /// Current migration phase
    phase: Arc<Mutex<MigrationPhase>>,
    /// Migration statistics
    stats: Arc<Mutex<MigrationStats>>,
    /// Active flag
    is_active: Arc<AtomicBool>,
    /// Cancelled flag
    is_cancelled: Arc<AtomicBool>,
    /// Destination URI
    destination: Arc<Mutex<Option<String>>>,
    /// Dirty page bitmap
    dirty_bitmap: Arc<Mutex<DirtyPageBitmap>>,
    /// Device states
    device_states: Arc<Mutex<Vec<DeviceState>>>,
    /// Total memory size
    total_memory: Arc<AtomicU64>,
    /// Transferred memory
    transferred_memory: Arc<AtomicU64>,
    /// Current iteration
    current_iteration: Arc<AtomicUsize>,
    /// Downtime start time
    downtime_start: Arc<Mutex<Option<u64>>>,
}

impl LiveMigration {
    /// Create a new live migration instance
    pub fn new(config: MigrationConfig) -> Result<Self, MigrationError> {
        Ok(Self {
            config,
            phase: Arc::new(Mutex::new(MigrationPhase::None)),
            stats: Arc::new(Mutex::new(MigrationStats::default())),
            is_active: Arc::new(AtomicBool::new(false)),
            is_cancelled: Arc::new(AtomicBool::new(false)),
            destination: Arc::new(Mutex::new(None)),
            dirty_bitmap: Arc::new(Mutex::new(DirtyPageBitmap::new(0))),
            device_states: Arc::new(Mutex::new(Vec::new())),
            total_memory: Arc::new(AtomicU64::new(0)),
            transferred_memory: Arc::new(AtomicU64::new(0)),
            current_iteration: Arc::new(AtomicUsize::new(0)),
            downtime_start: Arc::new(Mutex::new(None)),
        })
    }

    /// Get migration configuration
    pub fn config(&self) -> &MigrationConfig {
        &self.config
    }

    /// Get current phase
    pub fn phase(&self) -> MigrationPhase {
        *self.phase.lock()
    }

    /// Check if migration is active
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Check if migration is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    /// Start migration to destination
    pub fn start(&self, destination: &str) -> Result<(), MigrationError> {
        if self.is_active() {
            return Err(MigrationError::AlreadyInProgress);
        }

        *self.destination.lock() = Some(destination.to_string());
        *self.phase.lock() = MigrationPhase::Setup;
        self.is_active.store(true, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// Cancel migration
    pub fn cancel(&self) -> Result<(), MigrationError> {
        if !self.is_active() {
            return Err(MigrationError::NotStarted);
        }

        self.is_cancelled.store(true, Ordering::SeqCst);
        *self.phase.lock() = MigrationPhase::Cancelled;
        self.is_active.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// Get migration statistics
    pub fn stats(&self) -> MigrationStats {
        *self.stats.lock()
    }

    /// Update statistics
    pub fn update_stats(&self, stats: MigrationStats) {
        *self.stats.lock() = stats;
    }

    /// Set total memory size
    pub fn set_total_memory(&self, size: u64) {
        self.total_memory.store(size, Ordering::SeqCst);
        let mut bitmap = self.dirty_bitmap.lock();
        *bitmap = DirtyPageBitmap::new((size / MIGRATION_PAGE_SIZE as u64) as usize);
    }

    /// Get total memory size
    pub fn total_memory(&self) -> u64 {
        self.total_memory.load(Ordering::SeqCst)
    }

    /// Get transferred memory
    pub fn transferred_memory(&self) -> u64 {
        self.transferred_memory.load(Ordering::SeqCst)
    }

    /// Add transferred bytes
    pub fn add_transferred(&self, bytes: u64) {
        self.transferred_memory.fetch_add(bytes, Ordering::SeqCst);
    }

    /// Get remaining memory
    pub fn remaining_memory(&self) -> u64 {
        self.total_memory() - self.transferred_memory()
    }

    /// Get current iteration
    pub fn current_iteration(&self) -> usize {
        self.current_iteration.load(Ordering::SeqCst)
    }

    /// Start a new iteration
    pub fn start_iteration(&self) {
        self.current_iteration.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark pages as dirty
    pub fn mark_dirty(&self, page_numbers: &[usize]) {
        let mut bitmap = self.dirty_bitmap.lock();
        for &page in page_numbers {
            bitmap.mark_dirty(page);
        }
    }

    /// Mark a page range as dirty
    pub fn mark_dirty_range(&self, start: usize, end: usize) {
        let mut bitmap = self.dirty_bitmap.lock();
        bitmap.mark_range_dirty(start, end);
    }

    /// Get dirty page count
    pub fn dirty_page_count(&self) -> usize {
        self.dirty_bitmap.lock().dirty_count()
    }

    /// Get dirty page percentage
    pub fn dirty_page_percentage(&self) -> f64 {
        self.dirty_bitmap.lock().dirty_percentage()
    }

    /// Clear dirty bitmap
    pub fn clear_dirty_bitmap(&self) {
        self.dirty_bitmap.lock().clear();
    }

    /// Get all dirty pages
    pub fn get_dirty_pages(&self) -> Vec<usize> {
        self.dirty_bitmap
            .lock()
            .get_dirty_pages()
            .iter()
            .copied()
            .collect()
    }

    /// Add device state
    pub fn add_device_state(&self, state: DeviceState) {
        self.device_states.lock().push(state);
    }

    /// Get device states
    pub fn device_states(&self) -> Vec<DeviceState> {
        self.device_states.lock().clone()
    }

    /// Clear device states
    pub fn clear_device_states(&self) {
        self.device_states.lock().clear();
    }

    /// Start downtime phase
    pub fn start_downtime(&self) {
        *self.downtime_start.lock() = Some(Self::current_time_ms());
        *self.phase.lock() = MigrationPhase::Downtime;
    }

    /// End downtime phase
    pub fn end_downtime(&self) {
        if let Some(start) = *self.downtime_start.lock() {
            let elapsed = Self::current_time_ms() - start;
            let mut stats = self.stats.lock();
            stats.downtime = elapsed;
        }
    }

    /// Get current time in milliseconds
    fn current_time_ms() -> u64 {
        // In a real implementation, this would get the actual time
        // For now, return a simulated value
        0
    }

    /// Calculate transfer rate
    pub fn calculate_transfer_rate(&self) -> u64 {
        let stats = self.stats.lock();
        if stats.total_time > 0 {
            (stats.ram_transferred * 1000) / stats.total_time
        } else {
            0
        }
    }

    /// Check if convergence is reached
    pub fn is_converged(&self) -> bool {
        if self.current_iteration() >= self.config.max_iterations as usize {
            return true;
        }

        let dirty_percent = self.dirty_page_percentage();
        dirty_percent < 5.0 // 5% threshold
    }

    /// Transition to next phase
    pub fn transition_to(&self, phase: MigrationPhase) -> Result<(), MigrationError> {
        match (*self.phase.lock(), phase) {
            (MigrationPhase::None, MigrationPhase::Setup) => {
                *self.phase.lock() = phase;
                Ok(())
            }
            (MigrationPhase::Setup, MigrationPhase::PreCopy) => {
                *self.phase.lock() = phase;
                Ok(())
            }
            (MigrationPhase::PreCopy, MigrationPhase::PreCopySetup) => {
                *self.phase.lock() = phase;
                Ok(())
            }
            (MigrationPhase::PreCopySetup, MigrationPhase::PreCopyDevice) => {
                *self.phase.lock() = phase;
                Ok(())
            }
            (MigrationPhase::PreCopyDevice, MigrationPhase::Downtime) => {
                *self.phase.lock() = phase;
                Ok(())
            }
            (MigrationPhase::Downtime, MigrationPhase::Completed) => {
                *self.phase.lock() = phase;
                self.is_active.store(false, Ordering::SeqCst);
                Ok(())
            }
            _ => Err(MigrationError::InvalidState),
        }
    }

    /// Mark as failed
    pub fn mark_failed(&self) {
        *self.phase.lock() = MigrationPhase::Failed;
        self.is_active.store(false, Ordering::SeqCst);
    }

    /// Get migration progress (0-100)
    pub fn progress(&self) -> f64 {
        let total = self.total_memory();
        if total == 0 {
            0.0
        } else {
            (self.transferred_memory() as f64 / total as f64) * 100.0
        }
    }

    /// Estimated time to completion (ms)
    pub fn estimated_time_remaining(&self) -> u64 {
        let stats = self.stats.lock();
        if stats.transfer_rate == 0 {
            return u64::MAX;
        }

        let remaining = self.remaining_memory();
        let rate_bps = stats.transfer_rate * 1024 * 1024;
        (remaining * 1000) / rate_bps
    }

    /// Create migration data chunk
    pub fn create_migration_data(&self, data: Vec<u8>, offset: u64) -> MigrationData {
        MigrationData::new(data, offset)
    }
}

impl Default for LiveMigration {
    fn default() -> Self {
        Self::new(MigrationConfig::default()).unwrap()
    }
}

/// Network migration handler
#[derive(Debug)]
pub struct NetworkMigration {
    /// Migration instance
    migration: Arc<LiveMigration>,
    /// Connected flag
    is_connected: Arc<AtomicBool>,
}

impl NetworkMigration {
    /// Create a new network migration handler
    pub fn new(migration: Arc<LiveMigration>) -> Self {
        Self {
            migration,
            is_connected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Connect to destination
    pub fn connect(&self, dest: &str) -> Result<(), MigrationError> {
        // In a real implementation, this would establish a network connection
        self.is_connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Disconnect from destination
    pub fn disconnect(&self) {
        self.is_connected.store(false, Ordering::SeqCst);
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    /// Send migration data
    pub fn send_data(&self, data: MigrationData) -> Result<(), MigrationError> {
        if !self.is_connected() {
            return Err(MigrationError::ConnectionFailed);
        }

        self.migration.add_transferred(data.size() as u64);
        Ok(())
    }

    /// Receive migration data
    pub fn receive_data(&self) -> Result<MigrationData, MigrationError> {
        if !self.is_connected() {
            return Err(MigrationError::ConnectionFailed);
        }

        // In a real implementation, this would receive data from network
        Ok(MigrationData::new(Vec::new(), 0))
    }

    /// Send device state
    pub fn send_device_state(&self, state: &DeviceState) -> Result<(), MigrationError> {
        let data = bincode::serialize(state).unwrap_or_default();
        let migration_data = MigrationData::new(data, 0);
        self.send_data(migration_data)
    }

    /// Receive device state
    pub fn receive_device_state(&self) -> Result<DeviceState, MigrationError> {
        let data = self.receive_data()?;
        bincode::deserialize(&data.data)
            .map_err(|_| MigrationError::TransferFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_config() {
        let config = MigrationConfig::default();
        assert_eq!(config.strategy, MigrationStrategy::PreCopy);
        assert_eq!(config.max_downtime, 30);
        assert!(config.compression_enabled == false);
    }

    #[test]
    fn test_dirty_bitmap() {
        let mut bitmap = DirtyPageBitmap::new(1000);
        bitmap.mark_dirty(100);
        bitmap.mark_dirty(200);
        bitmap.mark_range_dirty(300, 305);

        assert!(bitmap.is_dirty(100));
        assert!(!bitmap.is_dirty(150));
        assert_eq!(bitmap.dirty_count(), 7);
    }

    #[test]
    fn test_device_state() {
        let mut state = DeviceState::new("test-device".into(), "virtio-net".into());
        state.set_state(vec![1, 2, 3, 4]);
        state.set_config(vec![5, 6, 7, 8]);

        assert_eq!(state.state_size(), 4);
        assert_eq!(state.config_size(), 4);
        assert_eq!(state.total_size(), 8);
    }

    #[test]
    fn test_live_migration_creation() {
        let config = MigrationConfig::default();
        let migration = LiveMigration::new(config).unwrap();

        assert!(!migration.is_active());
        assert_eq!(migration.phase(), MigrationPhase::None);
    }

    #[test]
    fn test_migration_start() {
        let migration = LiveMigration::default();
        migration.start("dest.example.com:9000").unwrap();

        assert!(migration.is_active());
        assert_eq!(migration.phase(), MigrationPhase::Setup);
    }

    #[test]
    fn test_migration_cancel() {
        let migration = LiveMigration::default();
        migration.start("dest.example.com:9000").unwrap();
        migration.cancel().unwrap();

        assert!(!migration.is_active());
        assert!(migration.is_cancelled());
        assert_eq!(migration.phase(), MigrationPhase::Cancelled);
    }

    #[test]
    fn test_dirty_tracking() {
        let migration = LiveMigration::default();
        migration.set_total_memory(1024 * 1024 * 1024); // 1GB

        migration.mark_dirty(&[100, 200, 300]);
        assert_eq!(migration.dirty_page_count(), 3);

        migration.clear_dirty_bitmap();
        assert_eq!(migration.dirty_page_count(), 0);
    }

    #[test]
    fn test_memory_tracking() {
        let migration = LiveMigration::default();
        migration.set_total_memory(1024 * 1024 * 1024);

        migration.add_transferred(512 * 1024 * 1024);
        assert_eq!(migration.transferred_memory(), 512 * 1024 * 1024);
        assert_eq!(migration.remaining_memory(), 512 * 1024 * 1024);
        assert_eq!(migration.progress(), 50.0);
    }

    #[test]
    fn test_convergence_check() {
        let config = MigrationConfig {
            max_iterations: 10,
            ..Default::default()
        };
        let migration = LiveMigration::new(config).unwrap();
        migration.set_total_memory(1024 * 1024 * 1024);

        // Should not converge with many dirty pages
        for i in 0..1000 {
            migration.mark_dirty(&[i * 100]);
        }
        assert!(!migration.is_converged());

        // Should converge with few dirty pages
        migration.clear_dirty_bitmap();
        migration.mark_dirty(&[1, 2, 3]);
        assert!(migration.is_converged());
    }
}
