//! # Storage Management Subsystem
//!
//! Comprehensive storage management for the NOS kernel providing advanced storage features.
//!
//! ## Overview
//!
//! The storage management subsystem provides enterprise-grade storage capabilities including:
//! - **Software RAID**: Multiple RAID levels (0, 1, 5, 6, 10) with advanced features
//! - **Logical Volume Management**: LVM-style volume management with snapshots and thin provisioning
//! - **Storage Pools**: Pool-based storage architecture with vdev management
//! - **Deduplication**: Block-level deduplication with multiple hash algorithms
//! - **Compression**: Transparent compression with multiple algorithms
//!
//! ## Architecture
//!
//! ```
//! Application Layer
//!     ├── File Systems
//!     └── Direct I/O
//!
//! Storage Management Layer
//!     ├── Logical Volumes (LVM)
//!     ├── Storage Pools
//!     ├── Deduplication Engine
//!     └── Compression Engine
//!
//! RAID Layer
//!     ├── RAID 0 (Striping)
//!     ├── RAID 1 (Mirroring)
//!     ├── RAID 5 (Distributed Parity)
//!     ├── RAID 6 (Dual Parity)
//!     └── RAID 10 (Nested)
//!
//! Block Device Layer
//!     ├── Physical Disks
//!     ├── SSD/NVMe
//!     └── Virtual Devices
//! ```
//!
//! ## Usage
//!
//! ### Creating a RAID Array
//!
//! ```no_run
//! use kernel::storage::{RaidArray, RaidLevel, StorageError};
//!
//! # fn main() -> Result<(), StorageError> {
//! // Create a RAID 5 array with 4 devices
//! let devices = vec![device1, device2, device3, device4];
//! let raid = RaidArray::create(
//!     "data_raid",
//!     RaidLevel::Raid5,
//!     devices,
//!     256 * 1024, // 256KB stripe size
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Creating a Logical Volume
//!
//! ```no_run
//! use kernel::storage::{LogicalVolumeManager, VolumeType, StorageError};
//!
//! # fn main() -> Result<(), StorageError> {
//! let lvm = LogicalVolumeManager::new();
//!
//! // Create a volume group
//! let vg = lvm.create_volume_group("vg0", &[pv1, pv2])?;
//!
//! // Create a logical volume
//! let lv = vg.create_logical_volume(
//!     "data",
//!     100 * 1024 * 1024 * 1024, // 100GB
//!     VolumeType::Linear,
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Using Storage Pools
//!
//! ```no_run
//! use kernel::storage::{StoragePool, PoolConfig, StorageError};
//!
//! # fn main() -> Result<(), StorageError> {
//! let config = PoolConfig {
//!     name: "tank".to_string(),
//!     compression: true,
//!     dedup: true,
//!     ..Default::default()
//! };
//!
//! let pool = StoragePool::create(config, vdevs)?;
//! # Ok(())
//! # }
//! ```

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use core::fmt;

use crate::sync::Mutex;

pub mod raid;
pub mod lvm;
pub mod pool;
pub mod dedup;
pub mod compression;

pub use raid::{
    RaidArray, RaidLevel, RaidState, RaidStats,
    RaidDevice, DeviceState, RaidManager,
};
pub use lvm::{
    LogicalVolumeManager, VolumeGroup, LogicalVolume,
    PhysicalVolume, VolumeType, LvState, PvState,
    Snapshot, ThinPool, ThinVolume,
};
pub use pool::{
    StoragePool, PoolConfig, PoolState, PoolStats,
    VirtualDevice, VdevType, VdevState, Metaslab,
    SpaceMap, AllocatorType,
};
pub use dedup::{
    DedupEngine, DedupConfig, DedupStats, DedupTable,
    HashAlgorithm, DedupMode, BlockRef,
};
pub use compression::{
    CompressionEngine, CompressionConfig, CompressionStats,
    CompressionType, CompressionLevel,
};

/// Storage management error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Invalid input parameters
    InvalidInput,
    /// Device not found
    NotFound,
    /// Device already exists
    AlreadyExists,
    /// Insufficient space
    NoSpace,
    /// I/O error occurred
    IoError,
    /// Device failure
    DeviceFailed,
    /// Invalid state for operation
    InvalidState,
    /// Operation not supported
    NotSupported,
    /// Resource busy
    ResourceBusy,
    /// Operation timeout
    Timeout,
    /// Corrupted data detected
    CorruptedData,
    /// Out of memory
    OutOfMemory,
    /// Feature not enabled
    NotEnabled,
    /// Quota exceeded
    QuotaExceeded,
    /// Checksum mismatch
    ChecksumMismatch,
    /// Operation interrupted
    Interrupted,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::InvalidInput => write!(f, "Invalid input parameters"),
            StorageError::NotFound => write!(f, "Device or resource not found"),
            StorageError::AlreadyExists => write!(f, "Device or resource already exists"),
            StorageError::NoSpace => write!(f, "Insufficient space available"),
            StorageError::IoError => write!(f, "I/O error occurred"),
            StorageError::DeviceFailed => write!(f, "Device failure detected"),
            StorageError::InvalidState => write!(f, "Invalid state for operation"),
            StorageError::NotSupported => write!(f, "Operation not supported"),
            StorageError::ResourceBusy => write!(f, "Resource busy"),
            StorageError::Timeout => write!(f, "Operation timed out"),
            StorageError::CorruptedData => write!(f, "Data corruption detected"),
            StorageError::OutOfMemory => write!(f, "Out of memory"),
            StorageError::NotEnabled => write!(f, "Feature not enabled"),
            StorageError::QuotaExceeded => write!(f, "Quota exceeded"),
            StorageError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            StorageError::Interrupted => write!(f, "Operation interrupted"),
        }
    }
}

impl core::error::Error for StorageError {}

/// Result type for storage operations
pub type StorageResult<T> = core::result::Result<T, StorageError>;

/// Storage device trait
///
/// All physical and virtual storage devices must implement this trait.
pub trait StorageDevice: Send + Sync {
    /// Read data from the device
    fn read(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize>;

    /// Write data to the device
    fn write(&self, offset: u64, data: &[u8]) -> StorageResult<usize>;

    /// Flush cached data to device
    fn flush(&self) -> StorageResult<()>;

    /// Get device size in bytes
    fn size(&self) -> u64;

    /// Get device name
    fn name(&self) -> &str;

    /// Check if device is healthy
    fn is_healthy(&self) -> bool;

    /// Get device statistics
    fn stats(&self) -> DeviceStats;
}

/// Device statistics
#[derive(Debug, Clone, Copy)]
pub struct DeviceStats {
    /// Number of read operations
    pub reads: u64,
    /// Number of write operations
    pub writes: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Read errors
    pub read_errors: u64,
    /// Write errors
    pub write_errors: u64,
    /// Average read latency in nanoseconds
    pub avg_read_latency: u64,
    /// Average write latency in nanoseconds
    pub avg_write_latency: u64,
}

impl Default for DeviceStats {
    fn default() -> Self {
        Self {
            reads: 0,
            writes: 0,
            bytes_read: 0,
            bytes_written: 0,
            read_errors: 0,
            write_errors: 0,
            avg_read_latency: 0,
            avg_write_latency: 0,
        }
    }
}

/// Storage manager - manages all storage subsystems
pub struct StorageManager {
    /// RAID manager
    raid_manager: Arc<RaidManager>,
    /// LVM manager
    lvm_manager: Arc<LogicalVolumeManager>,
    /// Storage pools
    pools: Mutex<BTreeMap<String, Arc<StoragePool>>>,
    /// Deduplication engine
    dedup_engine: Arc<DedupEngine>,
    /// Compression engine
    compression_engine: Arc<CompressionEngine>,
    /// Total managed capacity
    total_capacity: AtomicU64,
    /// Total used capacity
    used_capacity: AtomicU64,
    /// Initialization state
    initialized: AtomicU32,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new() -> Self {
        Self {
            raid_manager: Arc::new(RaidManager::new()),
            lvm_manager: Arc::new(LogicalVolumeManager::new()),
            pools: Mutex::new(BTreeMap::new()),
            dedup_engine: Arc::new(DedupEngine::new(DedupConfig::default())),
            compression_engine: Arc::new(CompressionEngine::new(CompressionConfig::default())),
            total_capacity: AtomicU64::new(0),
            used_capacity: AtomicU64::new(0),
            initialized: AtomicU32::new(0),
        }
    }

    /// Initialize the storage manager
    pub fn init(&self) -> StorageResult<()> {
        if self.initialized.load(Ordering::Acquire) != 0 {
            return Ok(());
        }

        // Initialize subsystems
        self.raid_manager.init()?;
        self.lvm_manager.init()?;

        self.initialized.store(1, Ordering::Release);
        crate::println!("[storage] Storage manager initialized");
        Ok(())
    }

    /// Shutdown the storage manager
    pub fn shutdown(&self) -> StorageResult<()> {
        self.initialized.store(0, Ordering::Release);
        crate::println!("[storage] Storage manager shutdown");
        Ok(())
    }

    /// Get RAID manager
    pub fn raid(&self) -> &Arc<RaidManager> {
        &self.raid_manager
    }

    /// Get LVM manager
    pub fn lvm(&self) -> &Arc<LogicalVolumeManager> {
        &self.lvm_manager
    }

    /// Create a storage pool
    pub fn create_pool(
        &self,
        config: PoolConfig,
        vdevs: Vec<Arc<VirtualDevice>>,
    ) -> StorageResult<Arc<StoragePool>> {
        let pool = Arc::new(StoragePool::create(config, vdevs)?);

        let mut pools = self.pools.lock();
        pools.insert(pool.config.name.clone(), pool.clone());

        // Update capacity
        self.total_capacity.fetch_add(pool.capacity(), Ordering::Relaxed);

        Ok(pool)
    }

    /// Get a storage pool
    pub fn get_pool(&self, name: &str) -> Option<Arc<StoragePool>> {
        let pools = self.pools.lock();
        pools.get(name).cloned()
    }

    /// Delete a storage pool
    pub fn delete_pool(&self, name: &str) -> StorageResult<()> {
        let mut pools = self.pools.lock();
        let pool = pools.remove(name).ok_or(StorageError::NotFound)?;

        // Update capacity
        self.total_capacity.fetch_sub(pool.capacity(), Ordering::Relaxed);

        Ok(())
    }

    /// List all pools
    pub fn list_pools(&self) -> Vec<String> {
        let pools = self.pools.lock();
        pools.keys().cloned().collect()
    }

    /// Get deduplication engine
    pub fn dedup(&self) -> &Arc<DedupEngine> {
        &self.dedup_engine
    }

    /// Get compression engine
    pub fn compression(&self) -> &Arc<CompressionEngine> {
        &self.compression_engine
    }

    /// Get total managed capacity
    pub fn total_capacity(&self) -> u64 {
        self.total_capacity.load(Ordering::Relaxed)
    }

    /// Get total used capacity
    pub fn used_capacity(&self) -> u64 {
        self.used_capacity.load(Ordering::Relaxed)
    }

    /// Get storage statistics
    pub fn stats(&self) -> StorageManagerStats {
        let pools = self.pools.lock();
        let mut total_reads = 0u64;
        let mut total_writes = 0u64;
        let mut total_bytes_read = 0u64;
        let mut total_bytes_written = 0u64;

        for pool in pools.values() {
            let stats = pool.stats();
            total_reads += stats.reads;
            total_writes += stats.writes;
            total_bytes_read += stats.bytes_read;
            total_bytes_written += stats.bytes_written;
        }

        StorageManagerStats {
            total_capacity: self.total_capacity.load(Ordering::Relaxed),
            used_capacity: self.used_capacity.load(Ordering::Relaxed),
            total_reads,
            total_writes,
            total_bytes_read,
            total_bytes_written,
            pool_count: pools.len() as u32,
        }
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage manager statistics
#[derive(Debug, Clone, Copy)]
pub struct StorageManagerStats {
    /// Total managed capacity in bytes
    pub total_capacity: u64,
    /// Used capacity in bytes
    pub used_capacity: u64,
    /// Total read operations
    pub total_reads: u64,
    /// Total write operations
    pub total_writes: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Number of storage pools
    pub pool_count: u32,
}

/// Global storage manager instance
static STORAGE_MANAGER: spin::Once<Arc<StorageManager>> = spin::Once::new();

/// Get the global storage manager
pub fn storage_manager() -> &'static Arc<StorageManager> {
    STORAGE_MANAGER.call_once(|| Arc::new(StorageManager::new()))
}

/// Initialize the storage subsystem
pub fn init() -> StorageResult<()> {
    storage_manager().init()
}

/// Shutdown the storage subsystem
pub fn shutdown() -> StorageResult<()> {
    storage_manager().shutdown()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_display() {
        let err = StorageError::NotFound;
        assert_eq!(format!("{}", err), "Device or resource not found");
    }

    #[test]
    fn test_device_stats_default() {
        let stats = DeviceStats::default();
        assert_eq!(stats.reads, 0);
        assert_eq!(stats.writes, 0);
    }

    #[test]
    fn test_storage_manager_creation() {
        let manager = StorageManager::new();
        assert_eq!(manager.total_capacity(), 0);
        assert_eq!(manager.used_capacity(), 0);
    }

    #[test]
    fn test_storage_manager_init() {
        let manager = StorageManager::new();
        assert!(manager.init().is_ok());
        assert!(manager.init().is_ok()); // Should be idempotent
    }

    #[test]
    fn test_storage_manager_stats() {
        let manager = StorageManager::new();
        let stats = manager.stats();
        assert_eq!(stats.total_capacity, 0);
        assert_eq!(stats.used_capacity, 0);
        assert_eq!(stats.pool_count, 0);
    }
}
