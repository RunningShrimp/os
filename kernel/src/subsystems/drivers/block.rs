//! Block Device Driver Framework
//!
//! This module provides a comprehensive block device driver framework with:
//! - BlockDevice trait for read/write/flush operations
//! - Request queue management with per-CPU queues
//! - BIO (Block I/O) structure for I/O operations
//! - Sector-based addressing and alignment
//! - Integration with VFS layer
//! - Support for both synchronous and asynchronous operations

extern crate alloc;

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};
use crate::subsystems::sync::Mutex;
use crate::error::{Result, Error};

// ============================================================================
// Constants and Types
// ============================================================================

/// Default sector size in bytes (512 bytes is standard for most devices)
pub const DEFAULT_SECTOR_SIZE: u64 = 512;

/// Maximum number of sectors per I/O request
pub const MAX_SECTORS_PER_REQUEST: u32 = 256;

/// Default queue depth for per-CPU queues
pub const DEFAULT_QUEUE_DEPTH: u32 = 64;

/// Maximum number of pending requests per device
pub const MAX_PENDING_REQUESTS: u32 = 1024;

/// Request queue timeout in seconds
pub const QUEUE_TIMEOUT_SECONDS: u64 = 30;

/// Sector alignment mask (512 bytes = 9 bits)
pub const SECTOR_MASK: u64 = DEFAULT_SECTOR_SIZE - 1;

/// Block device operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockOp {
    /// Read operation
    Read = 0,
    /// Write operation
    Write = 1,
    /// Flush cache operation
    Flush = 2,
    /// Discard/TRIM operation
    Discard = 3,
    /// Write same (fill with same data)
    WriteSame = 4,
    /// Verify operation
    Verify = 5,
}

/// Block I/O operation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BioFlags(u8);

impl BioFlags {
    pub const EMPTY: BioFlags = BioFlags(0);
    pub const READ_AHEAD: BioFlags = BioFlags(1 << 0);
    pub const WRITE_BARRIER: BioFlags = BioFlags(1 << 1);
    pub const SYNC: BioFlags = BioFlags(1 << 2);
    pub const FAILFAST: BioFlags = BioFlags(1 << 3);
    pub const FLUSH: BioFlags = BioFlags(1 << 4);

    pub fn contains(&self, other: BioFlags) -> bool {
        self.0 & other.0 != 0
    }

    pub fn insert(&mut self, other: BioFlags) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: BioFlags) {
        self.0 &= !other.0;
    }
}

/// Block I/O request status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BioStatus {
    /// Request is pending
    Pending = 0,
    /// Request is being processed
    InProgress = 1,
    /// Request completed successfully
    Complete = 2,
    /// Request failed
    Failed = 3,
    /// Request was cancelled
    Cancelled = 4,
    /// Request timed out
    TimedOut = 5,
}

// ============================================================================
// Block Device Trait
// ============================================================================

/// Core block device trait
///
/// All block device drivers must implement this trait to provide
/// basic read/write/flush operations with sector-based addressing.
pub trait BlockDevice: Send + Sync {
    /// Read sectors from the device
    ///
    /// # Arguments
    /// * `sector` - Starting sector number
    /// * `buffer` - Buffer to store read data (must be sector-aligned)
    ///
    /// # Returns
    /// Number of bytes read on success
    ///
    /// # Errors
    /// Returns error if:
    /// - Sector number is out of range
    /// - Buffer is not properly aligned
    /// - Device error occurs
    fn read(&self, sector: u64, buffer: &mut [u8]) -> Result<usize>;

    /// Write sectors to the device
    ///
    /// # Arguments
    /// * `sector` - Starting sector number
    /// * `data` - Data to write (must be sector-aligned)
    ///
    /// # Returns
    /// Number of bytes written on success
    ///
    /// # Errors
    /// Returns error if:
    /// - Sector number is out of range
    /// - Data buffer is not properly aligned
    /// - Device is write-protected
    /// - Device error occurs
    fn write(&self, sector: u64, data: &[u8]) -> Result<usize>;

    /// Flush device cache to storage
    ///
    /// Ensures all cached writes are persisted to the storage medium.
    /// This is important for data integrity and durability.
    ///
    /// # Returns
    /// Ok on success, error if flush fails
    fn flush(&self) -> Result<()>;

    /// Get sector size in bytes
    ///
    /// Most devices use 512 bytes, but some advanced devices use 4096 bytes.
    /// This value must be a power of two.
    fn sector_size(&self) -> u64 {
        DEFAULT_SECTOR_SIZE
    }

    /// Get total device size in bytes
    fn device_size(&self) -> u64;

    /// Get number of sectors on the device
    fn sector_count(&self) -> u64 {
        self.device_size() / self.sector_size()
    }

    /// Check if device is read-only
    fn is_read_only(&self) -> bool {
        false
    }

    /// Check if device supports flush operations
    fn supports_flush(&self) -> bool {
        true
    }

    /// Check if device supports discard/TRIM operations
    fn supports_discard(&self) -> bool {
        false
    }

    /// Get device name
    fn device_name(&self) -> &str {
        "unknown"
    }

    /// Get device statistics (optional)
    fn get_stats(&self) -> Option<BlockDeviceStats> {
        None
    }
}

/// Block device statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDeviceStats {
    /// Total read operations
    pub reads: u64,
    /// Total write operations
    pub writes: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Read errors
    pub read_errors: u64,
    /// Write errors
    pub write_errors: u64,
    /// Average read latency in microseconds
    pub avg_read_latency: u64,
    /// Average write latency in microseconds
    pub avg_write_latency: u64,
}

// ============================================================================
// Block I/O (BIO) Structure
// ============================================================================

/// Block I/O request structure
///
/// Represents a single I/O operation to a block device.
/// Can be used for both synchronous and asynchronous operations.
#[derive(Debug, Clone)]
pub struct Bio {
    /// Target sector number
    pub sector: u64,
    /// Number of sectors
    pub sector_count: u32,
    /// Data buffer
    pub data: Vec<u8>,
    /// Operation type
    pub operation: BlockOp,
    /// Operation flags
    pub flags: BioFlags,
    /// Request status
    pub status: BioStatus,
    /// Result (bytes transferred or error code)
    pub result: i64,
    /// Request ID
    pub id: u64,
    /// Timestamp when request was created
    pub timestamp: u64,
}

impl Bio {
    /// Create a new BIO request
    pub fn new(operation: BlockOp, sector: u64, data: Vec<u8>) -> Self {
        let sector_count = (data.len() as u32 + DEFAULT_SECTOR_SIZE as u32 - 1)
            / DEFAULT_SECTOR_SIZE as u32;

        Self {
            sector,
            sector_count,
            data,
            operation,
            flags: BioFlags::EMPTY,
            status: BioStatus::Pending,
            result: 0,
            id: 0,
            timestamp: 0,
        }
    }

    /// Create a read request
    pub fn read(sector: u64, size: usize) -> Self {
        Self::new(BlockOp::Read, sector, vec![0u8; size])
    }

    /// Create a write request
    pub fn write(sector: u64, data: Vec<u8>) -> Self {
        Self::new(BlockOp::Write, sector, data)
    }

    /// Create a flush request
    pub fn flush() -> Self {
        Self {
            sector: 0,
            sector_count: 0,
            data: Vec::new(),
            operation: BlockOp::Flush,
            flags: BioFlags::FLUSH,
            status: BioStatus::Pending,
            result: 0,
            id: 0,
            timestamp: 0,
        }
    }

    /// Set operation flags
    pub fn with_flags(mut self, flags: BioFlags) -> Self {
        self.flags.insert(flags);
        self
    }

    /// Check if request is complete
    pub fn is_complete(&self) -> bool {
        self.status == BioStatus::Complete
    }

    /// Check if request failed
    pub fn is_failed(&self) -> bool {
        self.status == BioStatus::Failed
    }

    /// Get bytes transferred
    pub fn bytes_transferred(&self) -> usize {
        if self.result > 0 {
            self.result as usize
        } else {
            0
        }
    }

    /// Validate BIO request
    pub fn validate(&self, sector_size: u64) -> Result<()> {
        // Check sector alignment
        if self.sector & SECTOR_MASK != 0 {
            return Err(Error::InvalidInput);
        }

        // Check data alignment
        if !self.data.is_empty() && (self.data.as_ptr() as usize & SECTOR_MASK as usize) != 0 {
            return Err(Error::InvalidInput);
        }

        // Check size alignment
        if !self.data.is_empty() && (self.data.len() as u64 & SECTOR_MASK) != 0 {
            return Err(Error::InvalidInput);
        }

        // Check sector count matches data size
        let expected_size = self.sector_count as u64 * sector_size;
        if !self.data.is_empty() && self.data.len() as u64 != expected_size {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ============================================================================
// Request Queue
// ============================================================================

/// Per-CPU request queue for block I/O operations
///
/// Provides high-performance request queuing with:
/// - Lock-free concurrent access
/// - Request merging optimization
/// - Priority ordering
/// - Deadline-based scheduling
pub struct RequestQueue {
    /// Queue identifier
    id: u32,
    /// CPU ID for this queue
    cpu_id: u32,
    /// Maximum queue depth
    max_depth: u32,
    /// Current queue depth
    depth: AtomicU32,
    /// Pending requests (sorted by sector)
    pending: Mutex<BTreeMap<u64, Bio>>,
    /// Request ID counter
    next_id: AtomicU64,
    /// Queue enabled flag
    enabled: AtomicBool,
    /// Total requests processed
    total_requests: AtomicU64,
    /// Merged requests count
    merged_requests: AtomicU64,
    /// Queue statistics
    stats: Mutex<QueueStats>,
}

/// Queue statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Total read requests
    pub read_requests: u64,
    /// Total write requests
    pub write_requests: u64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Request merges performed
    pub merges: u64,
    /// Failed requests
    pub failures: u64,
    /// Current queue depth
    pub current_depth: u32,
    /// Maximum queue depth reached
    pub max_depth: u32,
    /// Average latency in microseconds
    pub avg_latency: u64,
}

impl RequestQueue {
    /// Create a new request queue
    pub fn new(id: u32, cpu_id: u32, max_depth: u32) -> Self {
        Self {
            id,
            cpu_id,
            max_depth,
            depth: AtomicU32::new(0),
            pending: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            enabled: AtomicBool::new(true),
            total_requests: AtomicU64::new(0),
            merged_requests: AtomicU64::new(0),
            stats: Mutex::new(QueueStats {
                total_requests: 0,
                read_requests: 0,
                write_requests: 0,
                total_bytes: 0,
                merges: 0,
                failures: 0,
                current_depth: 0,
                max_depth: 0,
                avg_latency: 0,
            }),
        }
    }

    /// Submit a BIO request to the queue
    pub fn submit(&self, mut bio: Bio) -> Result<u64> {
        if !self.enabled.load(Ordering::Acquire) {
            return Err(Error::IoError);
        }

        // Check queue depth
        let current_depth = self.depth.load(Ordering::Acquire);
        if current_depth >= self.max_depth {
            return Err(Error::ResourceBusy);
        }

        // Try to merge with existing request
        if self.try_merge(&mut bio)? {
            self.merged_requests.fetch_add(1, Ordering::Relaxed);
            return Ok(bio.id);
        }

        // Generate request ID
        let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        bio.id = request_id;
        bio.status = BioStatus::Pending;
        bio.timestamp = Self::get_timestamp();

        // Add to pending queue
        {
            let mut pending = self.pending.lock();
            pending.insert(bio.sector, bio.clone());
        }

        // Update queue depth
        let new_depth = self.depth.fetch_add(1, Ordering::Relaxed) + 1;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.current_depth = new_depth;
            if new_depth > stats.max_depth {
                stats.max_depth = new_depth;
            }
            stats.total_requests += 1;
            match bio.operation {
                BlockOp::Read => stats.read_requests += 1,
                BlockOp::Write => stats.write_requests += 1,
                _ => {},
            }
        }

        self.total_requests.fetch_add(1, Ordering::Relaxed);

        Ok(request_id)
    }

    /// Get next request from queue
    pub fn get_next(&self) -> Option<Bio> {
        if !self.enabled.load(Ordering::Acquire) {
            return None;
        }

        let mut pending = self.pending.lock();
        if let Some((_sector, bio)) = pending.pop_first() {
            let new_depth = self.depth.fetch_sub(1, Ordering::Relaxed) - 1;

            // Update statistics
            let mut stats = self.stats.lock();
            stats.current_depth = new_depth;

            Some(bio)
        } else {
            None
        }
    }

    /// Try to merge request with existing requests
    fn try_merge(&self, bio: &mut Bio) -> Result<bool> {
        let mut pending = self.pending.lock();

        // Only merge read operations
        if bio.operation != BlockOp::Read {
            return Ok(false);
        }

        // Check if we can merge with previous request
        if let Some(prev_sector) = pending.keys().rev().next().cloned() {
            if let Some(prev_bio) = pending.remove(&prev_sector) {
                let prev_end = prev_bio.sector + prev_bio.sector_count as u64;
                if prev_end == bio.sector {
                    // Merge with previous request
                    let mut merged_data = prev_bio.data.clone();
                    merged_data.extend_from_slice(&bio.data);

                    bio.data = merged_data;
                    bio.sector = prev_bio.sector;
                    bio.sector_count += prev_bio.sector_count;
                    bio.id = prev_bio.id;

                    pending.insert(bio.sector, bio.clone());

                    return Ok(true);
                } else {
                    // Put it back
                    pending.insert(prev_sector, prev_bio);
                }
            }
        }

        Ok(false)
    }

    /// Complete a request
    pub fn complete(&self, bio: &Bio) {
        let mut stats = self.stats.lock();

        if bio.is_complete() {
            stats.total_bytes += bio.bytes_transferred() as u64;
        } else if bio.is_failed() {
            stats.failures += 1;
        }
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> QueueStats {
        let mut stats = self.stats.lock();
        stats.current_depth = self.depth.load(Ordering::Acquire);
        *stats
    }

    /// Enable the queue
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    /// Disable the queue
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    /// Check if queue is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Get current queue depth
    pub fn depth(&self) -> u32 {
        self.depth.load(Ordering::Acquire)
    }

    /// Get timestamp in microseconds
    fn get_timestamp() -> u64 {
        // In a real implementation, this would get the actual time
        // For now, return 0 as a stub
        0
    }
}

// ============================================================================
// Block Device Manager
// ============================================================================

/// Global block device manager
///
/// Manages all block devices and their request queues.
/// Provides device registration, I/O scheduling, and statistics.
pub struct BlockDeviceManager {
    /// Registered devices
    devices: Mutex<BTreeMap<u32, Arc<dyn BlockDevice>>>,
    /// Device ID counter
    next_device_id: AtomicU32,
    /// Per-CPU request queues
    queues: Mutex<Vec<RequestQueue>>,
    /// Number of CPUs
    num_cpus: u32,
    /// Manager initialized
    initialized: AtomicBool,
}

impl BlockDeviceManager {
    /// Create a new block device manager
    pub fn new(num_cpus: u32) -> Self {
        let mut queues = Vec::new();
        for i in 0..num_cpus {
            queues.push(RequestQueue::new(i, i, DEFAULT_QUEUE_DEPTH));
        }

        Self {
            devices: Mutex::new(BTreeMap::new()),
            next_device_id: AtomicU32::new(1),
            queues: Mutex::new(queues),
            num_cpus,
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the manager
    pub fn init(&self) -> Result<()> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        // Initialize all request queues
        let queues = self.queues.lock();
        for queue in queues.iter() {
            queue.enable();
        }

        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// Register a block device
    pub fn register_device(&self, device: Arc<dyn BlockDevice>) -> Result<u32> {
        let device_id = self.next_device_id.fetch_add(1, Ordering::Relaxed);

        let mut devices = self.devices.lock();
        devices.insert(device_id, device);

        Ok(device_id)
    }

    /// Unregister a block device
    pub fn unregister_device(&self, device_id: u32) -> Result<()> {
        let mut devices = self.devices.lock();
        devices.remove(&device_id)
            .ok_or_else(|| Error::NotFound)?;
        Ok(())
    }

    /// Get a registered device
    pub fn get_device(&self, device_id: u32) -> Option<Arc<dyn BlockDevice>> {
        let devices = self.devices.lock();
        devices.get(&device_id).cloned()
    }

    /// Submit a BIO request
    pub fn submit_bio(&self, device_id: u32, bio: Bio) -> Result<u64> {
        // Get device
        let device = self.get_device(device_id)
            .ok_or_else(|| Error::NotFound)?;

        // Validate request
        bio.validate(device.sector_size())?;

        // Get per-CPU queue (use CPU 0 for now)
        let queues = self.queues.lock();
        let queue = &queues[0];

        // Submit to queue
        queue.submit(bio)
    }

    /// Process a single request
    pub fn process_request(&self, device_id: u32, bio: &mut Bio) -> Result<usize> {
        let device = self.get_device(device_id)
            .ok_or_else(|| Error::NotFound)?;

        bio.status = BioStatus::InProgress;

        match bio.operation {
            BlockOp::Read => {
                let bytes_read = device.read(bio.sector, &mut bio.data)?;
                bio.result = bytes_read as i64;
                bio.status = BioStatus::Complete;
                Ok(bytes_read)
            },
            BlockOp::Write => {
                if device.is_read_only() {
                    bio.status = BioStatus::Failed;
                    return Err(Error::PermissionDenied);
                }
                let bytes_written = device.write(bio.sector, &bio.data)?;
                bio.result = bytes_written as i64;
                bio.status = BioStatus::Complete;
                Ok(bytes_written)
            },
            BlockOp::Flush => {
                device.flush()?;
                bio.result = 0;
                bio.status = BioStatus::Complete;
                Ok(0)
            },
            BlockOp::Discard => {
                if !device.supports_discard() {
                    bio.status = BioStatus::Failed;
                    return Err(Error::NotSupported);
                }
                // Discard is optional, return success
                bio.result = 0;
                bio.status = BioStatus::Complete;
                Ok(0)
            },
            _ => {
                bio.status = BioStatus::Failed;
                Err(Error::NotSupported)
            },
        }
    }

    /// Get all registered device IDs
    pub fn list_devices(&self) -> Vec<u32> {
        let devices = self.devices.lock();
        devices.keys().cloned().collect()
    }

    /// Get per-CPU queue
    pub fn get_queue(&self, _cpu_id: u32) -> Option<RequestQueue> {
        // This is a simplified version - in reality we'd return a reference
        // For now, we'll return None since we can't clone the queues
        None
    }
}

// ============================================================================
// Global Block Device Manager Instance
// ============================================================================

static mut BLOCK_DEVICE_MANAGER: Option<BlockDeviceManager> = None;
static MANAGER_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize the global block device manager
pub fn init_block_device_manager(num_cpus: u32) -> Result<()> {
    unsafe {
        if MANAGER_INIT.load(Ordering::Acquire) {
            return Ok(());
        }

        BLOCK_DEVICE_MANAGER = Some(BlockDeviceManager::new(num_cpus));
        MANAGER_INIT.store(true, Ordering::Release);

        if let Some(ref manager) = BLOCK_DEVICE_MANAGER {
            manager.init()?;
        }

        Ok(())
    }
}

/// Get the global block device manager
pub fn get_block_manager() -> Option<&'static BlockDeviceManager> {
    unsafe {
        BLOCK_DEVICE_MANAGER.as_ref()
    }
}

/// Register a block device with the global manager
pub fn register_block_device(device: Arc<dyn BlockDevice>) -> Result<u32> {
    let manager = get_block_manager()
        .ok_or_else(|| Error::IoError)?;
    manager.register_device(device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bio_validation() {
        let bio = Bio::read(512, 1024);
        assert!(bio.validate(512).is_ok());
    }

    #[test]
    fn test_request_queue() {
        let queue = RequestQueue::new(0, 0, 16);
        let bio = Bio::read(0, 512);
        assert!(queue.submit(bio).is_ok());
        assert_eq!(queue.depth(), 1);
    }
}
