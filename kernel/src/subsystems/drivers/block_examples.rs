//! Block Device Driver Examples
//!
//! This module provides example block device implementations including:
//! - RAM disk driver (in-memory block device)
//! - Virtual block device (file-backed)
//! - Performance test utilities
//! - VFS integration examples

extern crate alloc;

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::subsystems::sync::Mutex;
use crate::error::{Result, Error};
use super::{
    BlockDevice, DEFAULT_SECTOR_SIZE, register_block_device,
};

// ============================================================================
// RAM Disk Driver
// ============================================================================

/// RAM disk configuration
#[derive(Debug, Clone)]
pub struct RamDiskConfig {
    /// Disk size in bytes
    pub size: u64,
    /// Sector size in bytes
    pub sector_size: u64,
    /// Disk name
    pub name: String,
    /// Enable write protection
    pub read_only: bool,
    /// Enable performance monitoring
    pub enable_stats: bool,
}

impl Default for RamDiskConfig {
    fn default() -> Self {
        Self {
            size: 16 * 1024 * 1024, // 16 MB default
            sector_size: DEFAULT_SECTOR_SIZE,
            name: "ramdisk0".to_string(),
            read_only: false,
            enable_stats: true,
        }
    }
}

/// RAM disk driver statistics
#[derive(Debug, Clone)]
pub struct RamDiskStats {
    /// Total read operations
    pub reads: u64,
    /// Total write operations
    pub writes: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average read latency in nanoseconds
    pub avg_read_latency: u64,
    /// Average write latency in nanoseconds
    pub avg_write_latency: u64,
}

/// RAM disk block device implementation
///
/// An in-memory block device that stores data in a vector.
/// Useful for testing and as a high-performance temporary storage.
pub struct RamDisk {
    /// Device configuration
    config: RamDiskConfig,
    /// Data storage
    data: Mutex<Vec<u8>>,
    /// Device statistics
    stats: Mutex<RamDiskStats>,
    /// Read operations counter
    read_count: AtomicU64,
    /// Write operations counter
    write_count: AtomicU64,
    /// Total read time
    total_read_time: AtomicU64,
    /// Total write time
    total_write_time: AtomicU64,
}

impl RamDisk {
    /// Create a new RAM disk with default configuration
    pub fn new() -> Result<Arc<Self>> {
        Self::with_config(RamDiskConfig::default())
    }

    /// Create a new RAM disk with custom configuration
    pub fn with_config(config: RamDiskConfig) -> Result<Arc<Self>> {
        // Validate configuration
        if config.size == 0 || !config.size.is_power_of_two() {
            return Err(Error::InvalidInput);
        }

        if !config.sector_size.is_power_of_two() {
            return Err(Error::InvalidInput);
        }

        let data = vec![0u8; config.size as usize];
        let stats = RamDiskStats {
            reads: 0,
            writes: 0,
            bytes_read: 0,
            bytes_written: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_read_latency: 0,
            avg_write_latency: 0,
        };

        let ramdisk = Arc::new(Self {
            config,
            data: Mutex::new(data),
            stats: Mutex::new(stats),
            read_count: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            total_read_time: AtomicU64::new(0),
            total_write_time: AtomicU64::new(0),
        });

        Ok(ramdisk)
    }

    /// Create a RAM disk with specific size in MB
    pub fn with_size_mb(size_mb: u64) -> Result<Arc<Self>> {
        let config = RamDiskConfig {
            size: size_mb * 1024 * 1024,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Get device statistics
    pub fn get_statistics(&self) -> RamDiskStats {
        let mut stats = self.stats.lock();

        // Update average latencies
        let read_count = self.read_count.load(Ordering::Relaxed);
        let write_count = self.write_count.load(Ordering::Relaxed);

        if read_count > 0 {
            stats.avg_read_latency = self.total_read_time.load(Ordering::Relaxed) / read_count;
        }

        if write_count > 0 {
            stats.avg_write_latency = self.total_write_time.load(Ordering::Relaxed) / write_count;
        }

        stats.clone()
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        let mut stats = self.stats.lock();
        *stats = RamDiskStats {
            reads: 0,
            writes: 0,
            bytes_read: 0,
            bytes_written: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_read_latency: 0,
            avg_write_latency: 0,
        };

        self.read_count.store(0, Ordering::Relaxed);
        self.write_count.store(0, Ordering::Relaxed);
        self.total_read_time.store(0, Ordering::Relaxed);
        self.total_write_time.store(0, Ordering::Relaxed);
    }

    /// Fill disk with specific pattern
    pub fn fill_pattern(&self, pattern: u8) -> Result<()> {
        let mut data = self.data.lock();
        for byte in data.iter_mut() {
            *byte = pattern;
        }
        Ok(())
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let data = self.data.lock();
        data.len()
    }
}

impl BlockDevice for RamDisk {
    fn read(&self, sector: u64, buffer: &mut [u8]) -> Result<usize> {
        let start_time = Self::get_time_ns();

        // Validate sector alignment
        if sector & (DEFAULT_SECTOR_SIZE - 1) != 0 {
            return Err(Error::InvalidInput);
        }

        // Calculate offset
        let offset = sector as usize;
        let size = buffer.len();

        // Check bounds
        if offset + size > self.config.size as usize {
            return Err(Error::InvalidInput);
        }

        // Read data
        {
            let data = self.data.lock();
            buffer.copy_from_slice(&data[offset..offset + size]);
        }

        // Update statistics
        self.read_count.fetch_add(1, Ordering::Relaxed);
        let elapsed = Self::get_time_ns() - start_time;
        self.total_read_time.fetch_add(elapsed, Ordering::Relaxed);

        {
            let mut stats = self.stats.lock();
            stats.reads += 1;
            stats.bytes_read += size as u64;
        }

        Ok(size)
    }

    fn write(&self, sector: u64, data: &[u8]) -> Result<usize> {
        let start_time = Self::get_time_ns();

        // Check write protection
        if self.config.read_only {
            return Err(Error::PermissionDenied);
        }

        // Validate sector alignment
        if sector & (DEFAULT_SECTOR_SIZE - 1) != 0 {
            return Err(Error::InvalidInput);
        }

        // Calculate offset
        let offset = sector as usize;
        let size = data.len();

        // Check bounds
        if offset + size > self.config.size as usize {
            return Err(Error::InvalidInput);
        }

        // Write data
        {
            let mut disk_data = self.data.lock();
            disk_data[offset..offset + size].copy_from_slice(data);
        }

        // Update statistics
        self.write_count.fetch_add(1, Ordering::Relaxed);
        let elapsed = Self::get_time_ns() - start_time;
        self.total_write_time.fetch_add(elapsed, Ordering::Relaxed);

        {
            let mut stats = self.stats.lock();
            stats.writes += 1;
            stats.bytes_written += size as u64;
        }

        Ok(size)
    }

    fn flush(&self) -> Result<()> {
        // RAM disk doesn't need flushing
        Ok(())
    }

    fn sector_size(&self) -> u64 {
        self.config.sector_size
    }

    fn device_size(&self) -> u64 {
        self.config.size
    }

    fn is_read_only(&self) -> bool {
        self.config.read_only
    }

    fn device_name(&self) -> &str {
        &self.config.name
    }

    fn get_stats(&self) -> Option<super::BlockDeviceStats> {
        if !self.config.enable_stats {
            return None;
        }

        let stats = self.get_statistics();
        Some(super::BlockDeviceStats {
            reads: stats.reads,
            writes: stats.writes,
            bytes_read: stats.bytes_read,
            bytes_written: stats.bytes_written,
            read_errors: 0,
            write_errors: 0,
            avg_read_latency: stats.avg_read_latency / 1000, // Convert ns to us
            avg_write_latency: stats.avg_write_latency / 1000,
        })
    }
}

impl RamDisk {
    /// Get current time in nanoseconds (stub)
    fn get_time_ns() -> u64 {
        // In a real implementation, this would use a proper timer
        // For now, return 0
        0
    }
}

// ============================================================================
// Virtual Block Device
// ============================================================================

/// Virtual block device configuration
#[derive(Debug, Clone)]
pub struct VirtualBlockConfig {
    /// Device size in bytes
    pub size: u64,
    /// Sector size in bytes
    pub sector_size: u64,
    /// Device name
    pub name: String,
    /// Enable delayed allocation
    pub delayed_alloc: bool,
    /// Zero on read (uninitialized sectors return zeros)
    pub zero_on_read: bool,
}

impl Default for VirtualBlockConfig {
    fn default() -> Self {
        Self {
            size: 32 * 1024 * 1024, // 32 MB default
            sector_size: DEFAULT_SECTOR_SIZE,
            name: "vblock0".to_string(),
            delayed_alloc: true,
            zero_on_read: true,
        }
    }
}

/// Virtual block device
///
/// A sparse block device that allocates storage on-demand.
/// Useful for virtual machines and testing scenarios.
pub struct VirtualBlockDevice {
    /// Device configuration
    config: VirtualBlockConfig,
    /// Allocated blocks (sparse storage)
    blocks: Mutex<BTreeMap<u64, Vec<u8>>>,
    /// Total allocated bytes
    allocated: AtomicU64,
}

impl VirtualBlockDevice {
    /// Create a new virtual block device
    pub fn new() -> Result<Arc<Self>> {
        Self::with_config(VirtualBlockConfig::default())
    }

    /// Create a new virtual block device with custom configuration
    pub fn with_config(config: VirtualBlockConfig) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            config,
            blocks: Mutex::new(BTreeMap::new()),
            allocated: AtomicU64::new(0),
        }))
    }

    /// Get allocated size in bytes
    pub fn allocated_size(&self) -> u64 {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Get allocation ratio (allocated / total)
    pub fn allocation_ratio(&self) -> f64 {
        let allocated = self.allocated.load(Ordering::Relaxed) as f64;
        let total = self.config.size as f64;
        allocated / total
    }
}

impl BlockDevice for VirtualBlockDevice {
    fn read(&self, sector: u64, buffer: &mut [u8]) -> Result<usize> {
        let block_size = self.config.sector_size as usize;
        let block_num = sector / self.config.sector_size;

        let blocks = self.blocks.lock();

        if let Some(block_data) = blocks.get(&block_num) {
            // Block is allocated, read actual data
            let offset = (sector % self.config.sector_size) as usize;
            let bytes_to_read = block_size.min(buffer.len());
            buffer[..bytes_to_read].copy_from_slice(
                &block_data[offset..offset + bytes_to_read]
            );
        } else if self.config.zero_on_read {
            // Block not allocated, return zeros
            buffer.fill(0);
        } else {
            return Err(Error::IoError);
        }

        Ok(buffer.len())
    }

    fn write(&self, sector: u64, data: &[u8]) -> Result<usize> {
        let block_size = self.config.sector_size as usize;
        let block_num = sector / self.config.sector_size;
        let offset = (sector % self.config.sector_size) as usize;

        let mut blocks = self.blocks.lock();

        if !blocks.contains_key(&block_num) {
            // Allocate new block
            let mut block_data = vec![0u8; block_size];
            block_data[offset..offset + data.len()].copy_from_slice(data);
            blocks.insert(block_num, block_data);
            self.allocated.fetch_add(block_size as u64, Ordering::Relaxed);
        } else {
            // Block already allocated, update it
            if let Some(block_data) = blocks.get_mut(&block_num) {
                let end = (offset + data.len()).min(block_size);
                block_data[offset..end].copy_from_slice(data);
            }
        }

        Ok(data.len())
    }

    fn flush(&self) -> Result<()> {
        // Virtual device doesn't need flushing
        Ok(())
    }

    fn sector_size(&self) -> u64 {
        self.config.sector_size
    }

    fn device_size(&self) -> u64 {
        self.config.size
    }

    fn device_name(&self) -> &str {
        &self.config.name
    }
}

// ============================================================================
// Performance Testing Utilities
// ============================================================================

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerfTestConfig {
    /// Test name
    pub name: String,
    /// Read test size in bytes
    pub read_size: u64,
    /// Write test size in bytes
    pub write_size: u64,
    /// Number of iterations
    pub iterations: u32,
    /// Enable sequential access pattern
    pub sequential: bool,
    /// Block size for testing
    pub block_size: u64,
}

impl Default for PerfTestConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            read_size: 1024 * 1024, // 1 MB
            write_size: 1024 * 1024, // 1 MB
            iterations: 100,
            sequential: true,
            block_size: DEFAULT_SECTOR_SIZE,
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerfTestResults {
    /// Test name
    pub test_name: String,
    /// Total read throughput in MB/s
    pub read_throughput: f64,
    /// Total write throughput in MB/s
    pub write_throughput: f64,
    /// Average read latency in microseconds
    pub avg_read_latency: f64,
    /// Average write latency in microseconds
    pub avg_write_latency: f64,
    /// Total operations performed
    pub total_ops: u64,
    /// Test duration in seconds
    pub duration: f64,
}

/// Performance testing utilities
pub struct PerfTest;

impl PerfTest {
    /// Run sequential read performance test
    pub fn test_sequential_read(device: &dyn BlockDevice, config: &PerfTestConfig) -> Result<PerfTestResults> {
        let start_time = Self::get_time_sec();
        let mut total_bytes: u64 = 0;
        let mut total_ops: u64 = 0;
        let mut total_latency: u64 = 0;

        for _ in 0..config.iterations {
            let sector = (total_ops * config.block_size) % device.device_size();
            let mut buffer = vec![0u8; config.block_size as usize];

            let op_start = Self::get_time_ns();
            match device.read(sector, &mut buffer) {
                Ok(_) => {
                    let latency = Self::get_time_ns() - op_start;
                    total_latency += latency / 1000; // Convert to microseconds
                    total_bytes += config.block_size;
                    total_ops += 1;
                },
                Err(_) => break,
            }
        }

        let duration = Self::get_time_sec() - start_time;
        let read_throughput = (total_bytes as f64) / (1024.0 * 1024.0 * duration);
        let avg_latency = if total_ops > 0 {
            (total_latency as f64) / (total_ops as f64)
        } else {
            0.0
        };

        Ok(PerfTestResults {
            test_name: config.name.clone(),
            read_throughput,
            write_throughput: 0.0,
            avg_read_latency: avg_latency,
            avg_write_latency: 0.0,
            total_ops,
            duration,
        })
    }

    /// Run sequential write performance test
    pub fn test_sequential_write(device: &dyn BlockDevice, config: &PerfTestConfig) -> Result<PerfTestResults> {
        let start_time = Self::get_time_sec();
        let mut total_bytes: u64 = 0;
        let mut total_ops: u64 = 0;
        let mut total_latency: u64 = 0;

        let data = vec![0xAA_u8; config.block_size as usize];

        for _ in 0..config.iterations {
            let sector = (total_ops * config.block_size) % device.device_size();

            let op_start = Self::get_time_ns();
            match device.write(sector, &data) {
                Ok(_) => {
                    let latency = Self::get_time_ns() - op_start;
                    total_latency += latency / 1000; // Convert to microseconds
                    total_bytes += config.block_size;
                    total_ops += 1;
                },
                Err(_) => break,
            }
        }

        let duration = Self::get_time_sec() - start_time;
        let write_throughput = (total_bytes as f64) / (1024.0 * 1024.0 * duration);
        let avg_latency = if total_ops > 0 {
            (total_latency as f64) / (total_ops as f64)
        } else {
            0.0
        };

        Ok(PerfTestResults {
            test_name: config.name.clone(),
            read_throughput: 0.0,
            write_throughput,
            avg_read_latency: 0.0,
            avg_write_latency: avg_latency,
            total_ops,
            duration,
        })
    }

    /// Run full performance test suite
    pub fn run_full_test(device: &dyn BlockDevice) -> Result<Vec<PerfTestResults>> {
        let mut results = Vec::new();

        // Sequential read test
        let read_config = PerfTestConfig {
            name: "sequential_read".to_string(),
            ..Default::default()
        };
        results.push(Self::test_sequential_read(device, &read_config)?);

        // Sequential write test
        let write_config = PerfTestConfig {
            name: "sequential_write".to_string(),
            ..Default::default()
        };
        results.push(Self::test_sequential_write(device, &write_config)?);

        Ok(results)
    }

    /// Print test results
    pub fn print_results(results: &[PerfTestResults]) {
        crate::println!("=== Block Device Performance Test Results ===");
        for result in results {
            crate::println!("Test: {}", result.test_name);
            crate::println!("  Read Throughput: {:.2} MB/s", result.read_throughput);
            crate::println!("  Write Throughput: {:.2} MB/s", result.write_throughput);
            crate::println!("  Avg Read Latency: {:.2} us", result.avg_read_latency);
            crate::println!("  Avg Write Latency: {:.2} us", result.avg_write_latency);
            crate::println!("  Total Operations: {}", result.total_ops);
            crate::println!("  Duration: {:.2} seconds", result.duration);
            crate::println!();
        }
    }

    /// Get time in seconds (stub)
    fn get_time_sec() -> f64 {
        0.0
    }

    /// Get time in nanoseconds (stub)
    fn get_time_ns() -> u64 {
        0
    }
}

// ============================================================================
// VFS Integration Utilities
// ============================================================================

/// Mount a block device as a filesystem
///
/// This function demonstrates how to integrate a block device with the VFS layer.
/// In a real implementation, this would parse filesystem headers and mount the device.
pub fn mount_block_device(device: Arc<dyn BlockDevice>, mount_point: &str) -> Result<()> {
    crate::println!("Mounting block device: {} at {}", device.device_name(), mount_point);
    crate::println!("  Device size: {} MB", device.device_size() / (1024 * 1024));
    crate::println!("  Sector size: {} bytes", device.sector_size());
    crate::println!("  Sector count: {}", device.sector_count());

    // In a real implementation, this would:
    // 1. Detect filesystem type (ext4, FAT32, etc.)
    // 2. Initialize filesystem superblock
    // 3. Register with VFS mount table
    // 4. Create root inode

    Ok(())
}

/// Create a block device file in /dev
///
/// This function creates device nodes for accessing block devices from userspace.
pub fn create_device_node(device: Arc<dyn BlockDevice>, device_id: u32) -> Result<()> {
    let major = 8; // Block device major number
    let minor = device_id;

    crate::println!("Creating device node: {} ({}:{})",
                    device.device_name(), major, minor);

    // In a real implementation, this would:
    // 1. Create character/block device file in /dev
    // 2. Register device with device filesystem (devtmpfs)
    // 3. Set permissions and ownership

    Ok(())
}

// ============================================================================
// Initialization and Example Usage
// ============================================================================

/// Initialize example block devices
pub fn init_example_devices() -> Result<()> {
    crate::println!("Initializing example block devices...");

    // Create and register RAM disk
    let ramdisk = RamDisk::with_size_mb(16)?;
    let ramdisk_id = register_block_device(ramdisk.clone())?;
    crate::println!("  Registered RAM disk: {} (ID: {})", ramdisk.device_name(), ramdisk_id);

    // Create device node
    create_device_node(ramdisk.clone(), ramdisk_id)?;

    // Mount device
    mount_block_device(ramdisk.clone(), "/mnt/ramdisk")?;

    // Create and register virtual block device
    let vblock = VirtualBlockDevice::new()?;
    let vblock_id = register_block_device(vblock.clone())?;
    crate::println!("  Registered virtual block device: {} (ID: {})", vblock.device_name(), vblock_id);

    // Run performance tests on RAM disk
    crate::println!("Running performance tests on RAM disk...");
    let results = PerfTest::run_full_test(&*ramdisk)?;
    PerfTest::print_results(&results);

    crate::println!("Example block devices initialized successfully");

    Ok(())
}

/// Demonstrate basic block device operations
pub fn demonstrate_basic_operations() -> Result<()> {
    crate::println!("=== Block Device Basic Operations Demo ===");

    // Create a small RAM disk
    let ramdisk = RamDisk::with_size_mb(1)?;
    crate::println!("Created RAM disk: {}", ramdisk.device_name());

    // Write some data
    let write_data = b"Hello, Block Device World!".to_vec();
    let write_sector = 0;
    ramdisk.write(write_sector, &write_data)?;
    crate::println!("Written {} bytes at sector {}", write_data.len(), write_sector);

    // Read the data back
    let mut read_data = vec![0u8; write_data.len()];
    ramdisk.read(write_sector, &mut read_data)?;
    crate::println!("Read {} bytes: {:?}", read_data.len(),
                    core::str::from_utf8(&read_data).unwrap_or("(invalid utf-8)"));

    // Flush the device
    ramdisk.flush()?;
    crate::println!("Flushed device");

    // Get statistics
    let stats = ramdisk.get_statistics();
    crate::println!("Statistics: {} reads, {} writes, {} bytes read, {} bytes written",
                    stats.reads, stats.writes, stats.bytes_read, stats.bytes_written);

    crate::println!("=== Demo Complete ===");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ramdisk_creation() {
        let ramdisk = RamDisk::new();
        assert!(ramdisk.is_ok());
    }

    #[test]
    fn test_ramdisk_read_write() {
        let ramdisk = RamDisk::new().unwrap();
        let data = vec![0xABu8; 512];
        assert!(ramdisk.write(0, &data).is_ok());
    }

    #[test]
    fn test_virtual_block_device() {
        let vblock = VirtualBlockDevice::new();
        assert!(vblock.is_ok());
    }
}
