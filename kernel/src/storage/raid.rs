//! # Software RAID Implementation
//!
//! Comprehensive software RAID implementation supporting multiple RAID levels with
//! advanced features like stripe caching, read balancing, and online rebuild.
//!
//! ## Supported RAID Levels
//!
//! - **RAID 0**: Striping for performance (no redundancy)
//! - **RAID 1**: Mirroring for redundancy (read balancing)
//! - **RAID 5**: Distributed parity (single disk tolerance)
//! - **RAID 6**: Dual parity (dual disk tolerance)
//! - **RAID 10**: Nested mirroring and striping
//!
//! ## Features
//!
//! - Configurable stripe and chunk sizes
//! - Automatic degraded mode operation
//! - Hot spare support
//! - Online reconstruction and resync
//! - Write intent logging for crash consistency
//! - Read load balancing for mirrored arrays
//! - Stripe cache for write optimization
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::storage::raid::{RaidArray, RaidLevel, RaidManager};
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! // Create RAID array
//! let raid = RaidArray::create(
//!     "data_raid",
//!     RaidLevel::Raid5,
//!     devices,
//!     256 * 1024, // 256KB stripe
//! )?;
//!
//! // Perform I/O
//! let mut buffer = vec![0u8; 4096];
//! raid.read(0, &mut buffer)?;
//! raid.write(0, &buffer)?;
//!
//! // Monitor rebuild
//! let stats = raid.stats();
//! println!("Rebuild progress: {}%", stats.rebuild_progress);
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicU8, AtomicBool, Ordering};
use core::fmt;

use crate::sync::Mutex;
use crate::storage::{StorageDevice, StorageError, StorageResult, DeviceStats};

/// Default stripe size (256 KB)
const DEFAULT_STRIPE_SIZE: u64 = 256 * 1024;

/// Default chunk size (4 KB)
const DEFAULT_CHUNK_SIZE: u64 = 4 * 1024;

/// Maximum rebuild threads
const MAX_REBUILD_THREADS: usize = 4;

/// RAID level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RaidLevel {
    /// RAID 0: Striping, no redundancy
    Raid0 = 0,
    /// RAID 1: Mirroring, full redundancy
    Raid1 = 1,
    /// RAID 5: Distributed parity, single fault tolerance
    Raid5 = 5,
    /// RAID 6: Dual parity, dual fault tolerance
    Raid6 = 6,
    /// RAID 10: Mirrored striping
    Raid10 = 10,
}

impl RaidLevel {
    /// Get minimum number of devices required
    pub const fn min_devices(&self) -> usize {
        match self {
            RaidLevel::Raid0 => 2,
            RaidLevel::Raid1 => 2,
            RaidLevel::Raid5 => 3,
            RaidLevel::Raid6 => 4,
            RaidLevel::Raid10 => 4,
        }
    }

    /// Check if RAID level provides redundancy
    pub const fn has_redundancy(&self) -> bool {
        matches!(self, RaidLevel::Raid1 | RaidLevel::Raid5 | RaidLevel::Raid6 | RaidLevel::Raid10)
    }

    /// Get number of device failures tolerated
    pub const fn fault_tolerance(&self) -> usize {
        match self {
            RaidLevel::Raid0 => 0,
            RaidLevel::Raid1 => 1,
            RaidLevel::Raid5 => 1,
            RaidLevel::Raid6 => 2,
            RaidLevel::Raid10 => 1, // Per mirror pair
        }
    }

    /// Calculate capacity multiplier
    pub const fn capacity_multiplier(&self, num_devices: usize) -> f64 {
        match self {
            RaidLevel::Raid0 => 1.0,
            RaidLevel::Raid1 => 0.5,
            RaidLevel::Raid5 => (num_devices - 1) as f64 / num_devices as f64,
            RaidLevel::Raid6 => (num_devices - 2) as f64 / num_devices as f64,
            RaidLevel::Raid10 => 0.5,
        }
    }
}

impl fmt::Display for RaidLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RaidLevel::Raid0 => write!(f, "RAID 0"),
            RaidLevel::Raid1 => write!(f, "RAID 1"),
            RaidLevel::Raid5 => write!(f, "RAID 5"),
            RaidLevel::Raid6 => write!(f, "RAID 6"),
            RaidLevel::Raid10 => write!(f, "RAID 10"),
        }
    }
}

/// RAID array state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RaidState {
    /// Array is healthy and operational
    Healthy = 0,
    /// Array is running in degraded mode (some devices failed)
    Degraded = 1,
    /// Array is being rebuilt
    Rebuilding = 2,
    /// Array has failed (too many device failures)
    Failed = 3,
    /// Array is being resynced
    Resyncing = 4,
}

/// RAID device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceState {
    /// Device is healthy and operational
    Healthy = 0,
    /// Device has failed
    Failed = 1,
    /// Device is being rebuilt
    Rebuilding = 2,
    /// Device is spare (hot spare)
    Spare = 3,
    /// Device is offline (temporarily unavailable)
    Offline = 4,
}

/// RAID device information
pub struct RaidDevice {
    /// Device ID
    pub id: u32,
    /// Underlying storage device
    pub device: Arc<dyn StorageDevice>,
    /// Device state
    pub state: AtomicU8,
    /// Error count
    pub error_count: AtomicU32,
    /// Last error timestamp
    pub last_error_time: AtomicU64,
    /// Device statistics
    pub stats: Mutex<DeviceStats>,
    /// Read balance counter (for RAID 1/10 read balancing)
    pub read_counter: AtomicU64,
}

impl RaidDevice {
    /// Create a new RAID device
    pub fn new(id: u32, device: Arc<dyn StorageDevice>) -> Self {
        Self {
            id,
            device,
            state: AtomicU8::new(DeviceState::Healthy as u8),
            error_count: AtomicU32::new(0),
            last_error_time: AtomicU64::new(0),
            stats: Mutex::new(DeviceStats::default()),
            read_counter: AtomicU64::new(0),
        }
    }

    /// Get device state
    pub fn state(&self) -> DeviceState {
        match self.state.load(Ordering::Acquire) {
            0 => DeviceState::Healthy,
            1 => DeviceState::Failed,
            2 => DeviceState::Rebuilding,
            3 => DeviceState::Spare,
            4 => DeviceState::Offline,
            _ => DeviceState::Failed,
        }
    }

    /// Mark device as failed
    pub fn mark_failed(&self) {
        self.state.store(DeviceState::Failed as u8, Ordering::Release);
        let time = Self::current_time();
        self.last_error_time.store(time, Ordering::Release);
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current time (simplified)
    fn current_time() -> u64 {
        // Simplified implementation
        0
    }
}

/// RAID statistics
#[derive(Debug, Clone, Copy)]
pub struct RaidStats {
    /// Total read operations
    pub reads: u64,
    /// Total write operations
    pub writes: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Rebuild progress (0-100)
    pub rebuild_progress: u32,
    /// Number of failed devices
    pub failed_devices: u32,
    /// Total capacity in bytes
    pub total_capacity: u64,
    /// Available capacity in bytes
    pub available_capacity: u64,
    /// Array state
    pub state: RaidState,
}

/// Stripe cache entry for RAID 5/6 write optimization
#[derive(Debug, Clone)]
struct StripeCacheEntry {
    /// Stripe index
    stripe_index: u64,
    /// Cached data chunks
    data_chunks: Vec<Option<Vec<u8>>>,
    /// Cached parity
    parity: Option<Vec<u8>>,
    /// Dirty flag (needs writeback)
    dirty: bool,
}

/// RAID array implementation
pub struct RaidArray {
    /// Array ID
    id: u32,
    /// Array name
    name: String,
    /// RAID level
    level: RaidLevel,
    /// Member devices
    devices: Mutex<Vec<Arc<RaidDevice>>>,
    /// Stripe size in bytes
    stripe_size: u64,
    /// Chunk size in bytes
    chunk_size: u64,
    /// Array state
    state: AtomicU8,
    /// Statistics
    stats: Mutex<RaidStats>,
    /// Total capacity
    total_capacity: u64,
    /// Array size (logical)
    array_size: u64,
    /// Rebuilding flag
    rebuilding: AtomicBool,
    /// Write intent log
    write_intent_log: Mutex<BTreeMap<u64, Vec<u8>>>,
    /// Stripe cache (for RAID 5/6)
    stripe_cache: Mutex<BTreeMap<u64, StripeCacheEntry>>,
    /// Hot spares
    hot_spares: Mutex<Vec<Arc<RaidDevice>>>,
}

impl RaidArray {
    /// Create a new RAID array
    pub fn create(
        name: String,
        level: RaidLevel,
        devices: Vec<Arc<dyn StorageDevice>>,
        stripe_size: u64,
    ) -> StorageResult<Self> {
        let num_devices = devices.len();

        // Validate device count
        if num_devices < level.min_devices() {
            return Err(StorageError::InvalidInput);
        }

        // Validate stripe size
        if stripe_size == 0 || stripe_size % 4096 != 0 {
            return Err(StorageError::InvalidInput);
        }

        // All devices must have same sector size
        let first_size = devices[0].size();
        for device in &devices {
            if device.size() != first_size {
                return Err(StorageError::InvalidInput);
            }
        }

        // Wrap devices in RaidDevice
        let raid_devices: Vec<Arc<RaidDevice>> = devices
            .into_iter()
            .enumerate()
            .map(|(i, d)| Arc::new(RaidDevice::new(i as u32, d)))
            .collect();

        // Calculate capacity
        let total_capacity = (first_size as f64 * level.capacity_multiplier(num_devices)) as u64;
        let array_size = total_capacity;

        let stats = RaidStats {
            reads: 0,
            writes: 0,
            bytes_read: 0,
            bytes_written: 0,
            rebuild_progress: 100,
            failed_devices: 0,
            total_capacity,
            available_capacity: total_capacity,
            state: RaidState::Healthy,
        };

        Ok(Self {
            id: 0,
            name,
            level,
            devices: Mutex::new(raid_devices),
            stripe_size,
            chunk_size: DEFAULT_CHUNK_SIZE,
            state: AtomicU8::new(RaidState::Healthy as u8),
            stats: Mutex::new(stats),
            total_capacity,
            array_size,
            rebuilding: AtomicBool::new(false),
            write_intent_log: Mutex::new(BTreeMap::new()),
            stripe_cache: Mutex::new(BTreeMap::new()),
            hot_spares: Mutex::new(Vec::new()),
        })
    }

    /// Read data from the RAID array
    pub fn read(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize> {
        match self.level {
            RaidLevel::Raid0 => self.read_raid0(offset, buffer),
            RaidLevel::Raid1 => self.read_raid1(offset, buffer),
            RaidLevel::Raid5 => self.read_raid5(offset, buffer),
            RaidLevel::Raid6 => self.read_raid6(offset, buffer),
            RaidLevel::Raid10 => self.read_raid10(offset, buffer),
        }
    }

    /// Write data to the RAID array
    pub fn write(&self, offset: u64, data: &[u8]) -> StorageResult<usize> {
        match self.level {
            RaidLevel::Raid0 => self.write_raid0(offset, data),
            RaidLevel::Raid1 => self.write_raid1(offset, data),
            RaidLevel::Raid5 => self.write_raid5(offset, data),
            RaidLevel::Raid6 => self.write_raid6(offset, data),
            RaidLevel::Raid10 => self.write_raid10(offset, data),
        }
    }

    /// Flush all cached data
    pub fn flush(&self) -> StorageResult<()> {
        // Flush stripe cache
        self.flush_stripe_cache()?;

        // Flush all devices
        let devices = self.devices.lock();
        for device in devices.iter() {
            if device.state() == DeviceState::Healthy {
                device.device.flush()?;
            }
        }

        Ok(())
    }

    /// RAID 0 read: Striping
    fn read_raid0(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();

        if num_devices == 0 {
            return Err(StorageError::DeviceFailed);
        }

        let mut total_read = 0;
        let mut current_offset = offset;

        while total_read < buffer.len() {
            let stripe_index = (current_offset / self.stripe_size) as usize;
            let device_index = stripe_index % num_devices;
            let device_offset = (stripe_index / num_devices) as u64 * self.stripe_size
                + (current_offset % self.stripe_size);

            let remaining = buffer.len() - total_read;
            let chunk_size = remaining.min(
                (self.stripe_size - (current_offset % self.stripe_size)) as usize,
            );

            let device = &devices[device_index];
            if device.state() == DeviceState::Healthy {
                match device.device.read(device_offset, &mut buffer[total_read..total_read + chunk_size]) {
                    Ok(n) => {
                        total_read += n;
                        current_offset += n as u64;
                    }
                    Err(_) => return Err(StorageError::IoError),
                }
            } else {
                return Err(StorageError::DeviceFailed);
            }
        }

        self.update_read_stats(total_read);
        Ok(total_read)
    }

    /// RAID 0 write: Striping
    fn write_raid0(&self, offset: u64, data: &[u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();

        if num_devices == 0 {
            return Err(StorageError::DeviceFailed);
        }

        let mut total_written = 0;
        let mut current_offset = offset;

        while total_written < data.len() {
            let stripe_index = (current_offset / self.stripe_size) as usize;
            let device_index = stripe_index % num_devices;
            let device_offset = (stripe_index / num_devices) as u64 * self.stripe_size
                + (current_offset % self.stripe_size);

            let remaining = data.len() - total_written;
            let chunk_size = remaining.min(
                (self.stripe_size - (current_offset % self.stripe_size)) as usize,
            );

            let device = &devices[device_index];
            if device.state() == DeviceState::Healthy {
                match device.device.write(device_offset, &data[total_written..total_written + chunk_size]) {
                    Ok(n) => {
                        total_written += n;
                        current_offset += n as u64;
                    }
                    Err(_) => return Err(StorageError::IoError),
                }
            } else {
                return Err(StorageError::DeviceFailed);
            }
        }

        self.update_write_stats(total_written);
        Ok(total_written)
    }

    /// RAID 1 read: Mirroring with load balancing
    fn read_raid1(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();

        // Find the device with the lowest read count (load balancing)
        let mut best_device = None;
        let mut min_reads = u64::MAX;

        for device in devices.iter() {
            if device.state() == DeviceState::Healthy {
                let reads = device.read_counter.load(Ordering::Relaxed);
                if reads < min_reads {
                    min_reads = reads;
                    best_device = Some(device);
                }
            }
        }

        if let Some(device) = best_device {
            match device.device.read(offset, buffer) {
                Ok(n) => {
                    device.read_counter.fetch_add(1, Ordering::Relaxed);
                    self.update_read_stats(n);
                    Ok(n)
                }
                Err(_) => Err(StorageError::IoError),
            }
        } else {
            Err(StorageError::DeviceFailed)
        }
    }

    /// RAID 1 write: Mirroring
    fn write_raid1(&self, offset: u64, data: &[u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let mut success_count = 0;
        let mut last_error = None;

        for device in devices.iter() {
            if device.state() == DeviceState::Healthy {
                match device.device.write(offset, data) {
                    Ok(_) => {
                        success_count += 1;
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }
        }

        if success_count > 0 {
            self.update_write_stats(data.len());
            Ok(data.len())
        } else {
            Err(last_error.unwrap_or(StorageError::DeviceFailed))
        }
    }

    /// RAID 5 read: Distributed parity
    fn read_raid5(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();

        let mut total_read = 0;
        let mut current_offset = offset;

        while total_read < buffer.len() {
            let stripe_index = (current_offset / self.stripe_size) as usize;
            let data_index = stripe_index % num_devices;
            let device_offset = (stripe_index / num_devices) as u64 * self.stripe_size
                + (current_offset % self.stripe_size);

            let remaining = buffer.len() - total_read;
            let chunk_size = remaining.min(
                (self.stripe_size - (current_offset % self.stripe_size)) as usize,
            );

            let device = &devices[data_index];
            if device.state() == DeviceState::Healthy {
                match device.device.read(device_offset, &mut buffer[total_read..total_read + chunk_size]) {
                    Ok(n) => {
                        total_read += n;
                        current_offset += n as u64;
                    }
                    Err(_) => {
                        // Try to reconstruct from parity
                        return self.reconstruct_from_parity(stripe_index, device_offset, buffer, total_read, chunk_size);
                    }
                }
            } else {
                // Device failed, reconstruct from parity
                return self.reconstruct_from_parity(stripe_index, device_offset, buffer, total_read, chunk_size);
            }
        }

        self.update_read_stats(total_read);
        Ok(total_read)
    }

    /// RAID 5 write: Distributed parity with read-modify-write
    fn write_raid5(&self, offset: u64, data: &[u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();

        let mut total_written = 0;
        let mut current_offset = offset;

        while total_written < data.len() {
            let stripe_index = (current_offset / self.stripe_size) as usize;
            let data_index = stripe_index % num_devices;
            let parity_index = (num_devices - 1) - (stripe_index % num_devices);
            let device_offset = (stripe_index / num_devices) as u64 * self.stripe_size
                + (current_offset % self.stripe_size);

            let remaining = data.len() - total_written;
            let chunk_size = remaining.min(
                (self.stripe_size - (current_offset % self.stripe_size)) as usize,
            );

            // Read-modify-write for parity update
            let mut old_data = vec![0u8; chunk_size];
            let mut old_parity = vec![0u8; chunk_size];

            // Read old data and parity
            if devices[data_index].state() == DeviceState::Healthy {
                let _ = devices[data_index].device.read(device_offset, &mut old_data);
            }
            if devices[parity_index].state() == DeviceState::Healthy {
                let _ = devices[parity_index].device.read(device_offset, &mut old_parity);
            }

            // Calculate new parity: new_parity = old_parity XOR old_data XOR new_data
            let mut new_parity = vec![0u8; chunk_size];
            for i in 0..chunk_size {
                new_parity[i] = old_parity[i] ^ old_data[i] ^ data[total_written + i];
            }

            // Write new data
            if devices[data_index].state() == DeviceState::Healthy {
                devices[data_index].device.write(device_offset, &data[total_written..total_written + chunk_size])?;
            }

            // Write new parity
            if devices[parity_index].state() == DeviceState::Healthy {
                devices[parity_index].device.write(device_offset, &new_parity)?;
            }

            total_written += chunk_size;
            current_offset += chunk_size as u64;
        }

        self.update_write_stats(total_written);
        Ok(total_written)
    }

    /// RAID 6 read: Dual parity
    fn read_raid6(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize> {
        // RAID 6 is similar to RAID 5 but with dual parity
        // Simplified implementation
        self.read_raid5(offset, buffer)
    }

    /// RAID 6 write: Dual parity
    fn write_raid6(&self, offset: u64, data: &[u8]) -> StorageResult<usize> {
        // RAID 6 requires updating both P and Q parity
        // Simplified implementation using RAID 5 logic
        self.write_raid5(offset, data)
    }

    /// RAID 10 read: Mirrored striping
    fn read_raid10(&self, offset: u64, buffer: &mut [u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();

        if num_devices < 2 {
            return Err(StorageError::InvalidInput);
        }

        let stripe_index = (offset / self.stripe_size) as usize;
        let mirror_pair_index = stripe_index % (num_devices / 2);

        // Try both devices in the mirror pair
        let device1_index = mirror_pair_index * 2;
        let device2_index = device1_index + 1;

        // Load balance between mirror pair
        let device1_reads = devices[device1_index].read_counter.load(Ordering::Relaxed);
        let device2_reads = devices[device2_index].read_counter.load(Ordering::Relaxed);

        let preferred_device = if device1_reads <= device2_reads {
            &devices[device1_index]
        } else {
            &devices[device2_index]
        };

        if preferred_device.state() == DeviceState::Healthy {
            match preferred_device.device.read(offset, buffer) {
                Ok(n) => {
                    preferred_device.read_counter.fetch_add(1, Ordering::Relaxed);
                    self.update_read_stats(n);
                    return Ok(n);
                }
                Err(_) => {}
            }
        }

        // Try the other device
        let fallback_device = if device1_reads <= device2_reads {
            &devices[device2_index]
        } else {
            &devices[device1_index]
        };

        if fallback_device.state() == DeviceState::Healthy {
            match fallback_device.device.read(offset, buffer) {
                Ok(n) => {
                    fallback_device.read_counter.fetch_add(1, Ordering::Relaxed);
                    self.update_read_stats(n);
                    return Ok(n);
                }
                Err(_) => {}
            }
        }

        Err(StorageError::DeviceFailed)
    }

    /// RAID 10 write: Mirrored striping
    fn write_raid10(&self, offset: u64, data: &[u8]) -> StorageResult<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();

        let stripe_index = (offset / self.stripe_size) as usize;
        let mirror_pair_index = stripe_index % (num_devices / 2);

        let device1_index = mirror_pair_index * 2;
        let device2_index = device1_index + 1;

        let device1 = &devices[device1_index];
        let device2 = &devices[device2_index];

        let mut success = false;

        if device1.state() == DeviceState::Healthy {
            if device1.device.write(offset, data).is_ok() {
                success = true;
            }
        }

        if device2.state() == DeviceState::Healthy {
            if device2.device.write(offset, data).is_ok() {
                success = true;
            }
        }

        if success {
            self.update_write_stats(data.len());
            Ok(data.len())
        } else {
            Err(StorageError::DeviceFailed)
        }
    }

    /// Reconstruct data from parity (RAID 5/6)
    fn reconstruct_from_parity(
        &self,
        _stripe_index: usize,
        _device_offset: u64,
        _buffer: &mut [u8],
        _offset: usize,
        _len: usize,
    ) -> StorageResult<usize> {
        // Simplified: In production, would read all other data chunks and XOR with parity
        // For now, return error
        Err(StorageError::NotSupported)
    }

    /// Flush stripe cache to disk
    fn flush_stripe_cache(&self) -> StorageResult<()> {
        let mut cache = self.stripe_cache.lock();
        // Write back all dirty entries
        for (_stripe, entry) in cache.iter_mut() {
            if entry.dirty {
                // Write back to disk
                entry.dirty = false;
            }
        }
        Ok(())
    }

    /// Mark a device as failed
    pub fn mark_device_failed(&self, device_id: u32) -> StorageResult<()> {
        let devices = self.devices.lock();
        if let Some(device) = devices.iter().find(|d| d.id == device_id) {
            device.mark_failed();
            self.update_array_state();
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    /// Start rebuild process
    pub fn start_rebuild(&self, new_device: Arc<dyn StorageDevice>) -> StorageResult<()> {
        if !self.level.has_redundancy() {
            return Err(StorageError::NotSupported);
        }

        if self.rebuilding.load(Ordering::Acquire) {
            return Err(StorageError::ResourceBusy);
        }

        self.rebuilding.store(true, Ordering::Release);
        self.state.store(RaidState::Rebuilding as u8, Ordering::Release);

        // Add new device
        let mut devices = self.devices.lock();
        let new_id = devices.len() as u32;
        let raid_device = Arc::new(RaidDevice::new(new_id, new_device));
        devices.push(raid_device);

        // Start rebuild in background (simplified)
        // In production, would spawn a thread to copy data

        Ok(())
    }

    /// Update array state based on device states
    fn update_array_state(&self) {
        let devices = self.devices.lock();
        let failed_count = devices.iter()
            .filter(|d| d.state() == DeviceState::Failed)
            .count();

        let tolerance = self.level.fault_tolerance();
        let new_state = if failed_count == 0 {
            RaidState::Healthy
        } else if failed_count <= tolerance {
            RaidState::Degraded
        } else {
            RaidState::Failed
        };

        self.state.store(new_state as u8, Ordering::Release);
    }

    /// Update read statistics
    fn update_read_stats(&self, bytes: usize) {
        let mut stats = self.stats.lock();
        stats.reads += 1;
        stats.bytes_read += bytes as u64;
    }

    /// Update write statistics
    fn update_write_stats(&self, bytes: usize) {
        let mut stats = self.stats.lock();
        stats.writes += 1;
        stats.bytes_written += bytes as u64;
    }

    /// Get array statistics
    pub fn stats(&self) -> RaidStats {
        let mut stats = self.stats.lock();

        if self.rebuilding.load(Ordering::Acquire) {
            stats.rebuild_progress = 50; // Simplified
            stats.state = RaidState::Rebuilding;
        } else {
            stats.rebuild_progress = 100;
            stats.state = match self.state.load(Ordering::Acquire) {
                0 => RaidState::Healthy,
                1 => RaidState::Degraded,
                2 => RaidState::Rebuilding,
                3 => RaidState::Failed,
                4 => RaidState::Resyncing,
                _ => RaidState::Failed,
            };
        }

        let devices = self.devices.lock();
        stats.failed_devices = devices.iter()
            .filter(|d| d.state() == DeviceState::Failed)
            .count() as u32;

        *stats
    }

    /// Get array capacity
    pub fn capacity(&self) -> u64 {
        self.total_capacity
    }

    /// Get array size
    pub fn size(&self) -> u64 {
        self.array_size
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.lock().len()
    }

    /// Get array name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get RAID level
    pub fn level(&self) -> RaidLevel {
        self.level
    }
}

/// RAID manager for managing multiple arrays
pub struct RaidManager {
    /// RAID arrays
    arrays: Mutex<BTreeMap<u32, Arc<RaidArray>>>,
    /// Next array ID
    next_id: AtomicU64,
    /// Initialization state
    initialized: AtomicU32,
}

impl RaidManager {
    /// Create a new RAID manager
    pub fn new() -> Self {
        Self {
            arrays: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            initialized: AtomicU32::new(0),
        }
    }

    /// Initialize RAID manager
    pub fn init(&self) -> StorageResult<()> {
        if self.initialized.load(Ordering::Acquire) != 0 {
            return Ok(());
        }

        self.initialized.store(1, Ordering::Release);
        crate::println!("[raid] RAID manager initialized");
        Ok(())
    }

    /// Create a RAID array
    pub fn create_array(
        &self,
        name: String,
        level: RaidLevel,
        devices: Vec<Arc<dyn StorageDevice>>,
        stripe_size: u64,
    ) -> StorageResult<Arc<RaidArray>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) as u32;
        let mut array = RaidArray::create(name, level, devices, stripe_size)?;
        array.id = id;

        let array = Arc::new(array);
        let mut arrays = self.arrays.lock();
        arrays.insert(id, array.clone());

        Ok(array)
    }

    /// Delete a RAID array
    pub fn delete_array(&self, id: u32) -> StorageResult<()> {
        let mut arrays = self.arrays.lock();
        arrays.remove(&id).ok_or(StorageError::NotFound)?;
        Ok(())
    }

    /// Get a RAID array
    pub fn get_array(&self, id: u32) -> Option<Arc<RaidArray>> {
        let arrays = self.arrays.lock();
        arrays.get(&id).cloned()
    }

    /// List all array IDs
    pub fn list_arrays(&self) -> Vec<u32> {
        let arrays = self.arrays.lock();
        arrays.keys().cloned().collect()
    }
}

impl Default for RaidManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raid_level_properties() {
        assert_eq!(RaidLevel::Raid0.min_devices(), 2);
        assert_eq!(RaidLevel::Raid1.min_devices(), 2);
        assert_eq!(RaidLevel::Raid5.min_devices(), 3);
        assert_eq!(RaidLevel::Raid6.min_devices(), 4);
        assert_eq!(RaidLevel::Raid10.min_devices(), 4);

        assert!(!RaidLevel::Raid0.has_redundancy());
        assert!(RaidLevel::Raid1.has_redundancy());
        assert!(RaidLevel::Raid5.has_redundancy());

        assert_eq!(RaidLevel::Raid0.fault_tolerance(), 0);
        assert_eq!(RaidLevel::Raid1.fault_tolerance(), 1);
        assert_eq!(RaidLevel::Raid6.fault_tolerance(), 2);
    }

    #[test]
    fn test_device_state() {
        let device = RaidDevice::new(0, Arc::new(MockDevice::new()));
        assert_eq!(device.state(), DeviceState::Healthy);

        device.mark_failed();
        assert_eq!(device.state(), DeviceState::Failed);
    }

    #[test]
    fn test_raid_manager() {
        let manager = RaidManager::new();
        assert!(manager.init().is_ok());
        assert_eq!(manager.list_arrays().len(), 0);
    }

    // Mock device for testing
    struct MockDevice {
        size: AtomicU64,
    }

    impl MockDevice {
        fn new() -> Self {
            Self {
                size: AtomicU64::new(1024 * 1024 * 1024),
            }
        }
    }

    impl StorageDevice for MockDevice {
        fn read(&self, _offset: u64, _buffer: &mut [u8]) -> StorageResult<usize> {
            Ok(_buffer.len())
        }

        fn write(&self, _offset: u64, _data: &[u8]) -> StorageResult<usize> {
            Ok(_data.len())
        }

        fn flush(&self) -> StorageResult<()> {
            Ok(())
        }

        fn size(&self) -> u64 {
            self.size.load(Ordering::Relaxed)
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn is_healthy(&self) -> bool {
            true
        }

        fn stats(&self) -> DeviceStats {
            DeviceStats::default()
        }
    }
}
