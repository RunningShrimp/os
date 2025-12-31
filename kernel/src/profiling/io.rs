//! I/O Performance Profiling
//!
//! This module provides disk and network I/O profiling capabilities for the NOS kernel,
//! including latency tracking, size distribution, and per-process/device statistics.
//!
//! # Features
//!
//! - Disk I/O profiling
//! - Network I/O profiling
//! - I/O latency tracking
//! - I/O size distribution
//! - I/O by process
//! - I/O by file/socket
//! - I/O operation breakdown
//!
//! # Usage
//!
//! ```rust
//! use kernel::profiling::io::IoProfiler;
//!
//! let profiler = IoProfiler::new();
//! profiler.start().unwrap();
//! // I/O operations will be automatically tracked
//! profiler.stop().unwrap();
//! let report = profiler.generate_report().unwrap();
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use crate::sync::Mutex;

/// I/O profiling error types
#[derive(Debug, Clone, PartialEq)]
pub enum IoProfileError {
    /// Profiler is already running
    AlreadyRunning,
    /// Profiler is not running
    NotRunning,
    /// Invalid I/O size
    InvalidSize(u64),
    /// Buffer overflow
    BufferOverflow,
    /// Operation not found
    OperationNotFound(u64),
}

impl core::fmt::Display for IoProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "I/O profiler is already running"),
            Self::NotRunning => write!(f, "I/O profiler is not running"),
            Self::InvalidSize(size) => write!(f, "Invalid I/O size: {}", size),
            Self::BufferOverflow => write!(f, "Profile buffer overflow"),
            Self::OperationNotFound(id) => write!(f, "Operation not found: {}", id),
        }
    }
}

/// I/O operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperationType {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Sync/flush operation
    Sync,
    /// Seek operation
    Seek,
    /// Memory-mapped I/O
    Mmap,
    /// Unmapped operation
    Munmap,
}

/// I/O device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDeviceType {
    /// Block device (disk)
    BlockDevice,
    /// Network socket
    Network,
    /// Character device
    CharDevice,
    /// Pipe
    Pipe,
    /// Anonymous (unknown)
    Anonymous,
}

/// I/O operation record
#[derive(Debug, Clone)]
pub struct IoOperation {
    /// Unique operation ID
    pub id: u64,
    /// Operation type
    pub op_type: IoOperationType,
    /// Device type
    pub device_type: IoDeviceType,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// File descriptor or handle
    pub fd: u64,
    /// Device identifier (e.g., block device ID, socket address)
    pub device_id: u64,
    /// Offset in file/device
    pub offset: u64,
    /// Size of I/O operation (bytes)
    pub size: u64,
    /// Start timestamp
    pub start_time: u64,
    /// End timestamp
    pub end_time: Option<u64>,
    /// Latency (nanoseconds)
    pub latency_ns: Option<u64>,
    /// Whether operation completed successfully
    pub success: bool,
    /// Error code (if failed)
    pub error: Option<i32>,
}

impl IoOperation {
    /// Create a new I/O operation
    pub fn new(
        id: u64,
        op_type: IoOperationType,
        device_type: IoDeviceType,
        pid: u64,
        tid: u64,
        fd: u64,
        size: u64,
    ) -> Self {
        Self {
            id,
            op_type,
            device_type,
            pid,
            tid,
            fd,
            device_id: 0,
            offset: 0,
            size,
            start_time: Self::now(),
            end_time: None,
            latency_ns: None,
            success: false,
            error: None,
        }
    }

    /// Mark operation as completed
    pub fn complete(&mut self, success: bool, error: Option<i32>) {
        self.end_time = Some(Self::now());
        self.latency_ns = self.end_time.and_then(|end| Some(end.saturating_sub(self.start_time)));
        self.success = success;
        self.error = error;
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }
}

/// Statistics for a specific file descriptor
#[derive(Debug)]
pub struct FileDescriptorStats {
    /// File descriptor number
    pub fd: u64,
    /// Total read operations
    pub read_ops: AtomicU64,
    /// Total write operations
    pub write_ops: AtomicU64,
    /// Total bytes read
    pub read_bytes: AtomicU64,
    /// Total bytes written
    pub write_bytes: AtomicU64,
    /// Total read latency
    pub total_read_latency: AtomicU64,
    /// Total write latency
    pub total_write_latency: AtomicU64,
    /// Maximum read latency
    pub max_read_latency: AtomicU64,
    /// Maximum write latency
    pub max_write_latency: AtomicU64,
}

impl Clone for FileDescriptorStats {
    fn clone(&self) -> Self {
        Self {
            fd: self.fd,
            read_ops: AtomicU64::new(self.read_ops.load(Ordering::Relaxed)),
            write_ops: AtomicU64::new(self.write_ops.load(Ordering::Relaxed)),
            read_bytes: AtomicU64::new(self.read_bytes.load(Ordering::Relaxed)),
            write_bytes: AtomicU64::new(self.write_bytes.load(Ordering::Relaxed)),
            total_read_latency: AtomicU64::new(self.total_read_latency.load(Ordering::Relaxed)),
            total_write_latency: AtomicU64::new(self.total_write_latency.load(Ordering::Relaxed)),
            max_read_latency: AtomicU64::new(self.max_read_latency.load(Ordering::Relaxed)),
            max_write_latency: AtomicU64::new(self.max_write_latency.load(Ordering::Relaxed)),
        }
    }
}

impl FileDescriptorStats {
    /// Create new file descriptor statistics
    pub fn new(fd: u64) -> Self {
        Self {
            fd,
            read_ops: AtomicU64::new(0),
            write_ops: AtomicU64::new(0),
            read_bytes: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            total_read_latency: AtomicU64::new(0),
            total_write_latency: AtomicU64::new(0),
            max_read_latency: AtomicU64::new(0),
            max_write_latency: AtomicU64::new(0),
        }
    }

    /// Record a read operation
    pub fn record_read(&self, size: u64, latency_ns: u64) {
        self.read_ops.fetch_add(1, Ordering::Relaxed);
        self.read_bytes.fetch_add(size, Ordering::Relaxed);
        self.total_read_latency.fetch_add(latency_ns, Ordering::Relaxed);

        let mut max = self.max_read_latency.load(Ordering::Relaxed);
        while latency_ns > max {
            match self.max_read_latency.compare_exchange_weak(
                max,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
    }

    /// Record a write operation
    pub fn record_write(&self, size: u64, latency_ns: u64) {
        self.write_ops.fetch_add(1, Ordering::Relaxed);
        self.write_bytes.fetch_add(size, Ordering::Relaxed);
        self.total_write_latency.fetch_add(latency_ns, Ordering::Relaxed);

        let mut max = self.max_write_latency.load(Ordering::Relaxed);
        while latency_ns > max {
            match self.max_write_latency.compare_exchange_weak(
                max,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
    }

    /// Get average read latency
    pub fn avg_read_latency(&self) -> u64 {
        let ops = self.read_ops.load(Ordering::Relaxed);
        let total = self.total_read_latency.load(Ordering::Relaxed);
        if ops > 0 {
            total / ops
        } else {
            0
        }
    }

    /// Get average write latency
    pub fn avg_write_latency(&self) -> u64 {
        let ops = self.write_ops.load(Ordering::Relaxed);
        let total = self.total_write_latency.load(Ordering::Relaxed);
        if ops > 0 {
            total / ops
        } else {
            0
        }
    }
}

/// Statistics for a process
#[derive(Debug)]
pub struct ProcessIoStats {
    /// Process ID
    pub pid: u64,
    /// Total operations
    pub total_ops: AtomicU64,
    /// Total bytes read
    pub read_bytes: AtomicU64,
    /// Total bytes written
    pub write_bytes: AtomicU64,
    /// Total read operations
    pub read_ops: AtomicU64,
    /// Total write operations
    pub write_ops: AtomicU64,
    /// Total I/O time (nanoseconds)
    pub total_io_time: AtomicU64,
}

impl Clone for ProcessIoStats {
    fn clone(&self) -> Self {
        Self {
            pid: self.pid,
            total_ops: AtomicU64::new(self.total_ops.load(Ordering::Relaxed)),
            read_bytes: AtomicU64::new(self.read_bytes.load(Ordering::Relaxed)),
            write_bytes: AtomicU64::new(self.write_bytes.load(Ordering::Relaxed)),
            read_ops: AtomicU64::new(self.read_ops.load(Ordering::Relaxed)),
            write_ops: AtomicU64::new(self.write_ops.load(Ordering::Relaxed)),
            total_io_time: AtomicU64::new(self.total_io_time.load(Ordering::Relaxed)),
        }
    }
}

impl ProcessIoStats {
    /// Create new process I/O statistics
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            total_ops: AtomicU64::new(0),
            read_bytes: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            read_ops: AtomicU64::new(0),
            write_ops: AtomicU64::new(0),
            total_io_time: AtomicU64::new(0),
        }
    }

    /// Record an operation
    pub fn record_operation(&self, op_type: IoOperationType, size: u64, latency_ns: u64) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.total_io_time.fetch_add(latency_ns, Ordering::Relaxed);

        match op_type {
            IoOperationType::Read => {
                self.read_ops.fetch_add(1, Ordering::Relaxed);
                self.read_bytes.fetch_add(size, Ordering::Relaxed);
            }
            IoOperationType::Write => {
                self.write_ops.fetch_add(1, Ordering::Relaxed);
                self.write_bytes.fetch_add(size, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

/// I/O size bucket for distribution analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoSizeBucket {
    Tiny,     // 0-512 bytes
    Small,    // 512B-4KB
    Medium,   // 4KB-64KB
    Large,    // 64KB-1MB
    Huge,     // 1MB-16MB
    Gigantic, // >16MB
}

impl IoSizeBucket {
    /// Classify I/O size into bucket
    pub fn from_size(size: u64) -> Self {
        match size {
            0..=511 => Self::Tiny,
            512..=4095 => Self::Small,
            4096..=65_535 => Self::Medium,
            65_536..=1_048_575 => Self::Large,
            1_048_576..=16_777_215 => Self::Huge,
            _ => Self::Gigantic,
        }
    }
}

/// I/O profiler configuration
#[derive(Debug, Clone)]
pub struct IoProfilerConfig {
    /// Track individual operations
    pub track_operations: bool,
    /// Maximum operations to store
    pub max_operations: usize,
    /// Track by file descriptor
    pub track_by_fd: bool,
    /// Track by process
    pub track_by_process: bool,
    /// Minimum I/O size to track
    pub min_tracked_size: u64,
    /// Track latency percentiles
    pub track_percentiles: bool,
}

impl Default for IoProfilerConfig {
    fn default() -> Self {
        Self {
            track_operations: true,
            max_operations: 100_000,
            track_by_fd: true,
            track_by_process: true,
            min_tracked_size: 0,
            track_percentiles: true,
        }
    }
}

/// I/O profiler state
struct IoProfilerState {
    running: bool,
    start_time: Option<u64>,
    stop_time: Option<u64>,
}

/// I/O profiling report
#[derive(Debug, Clone)]
pub struct IoProfileReport {
    /// Duration of profiling
    pub duration: Duration,
    /// Total operations
    pub total_operations: u64,
    /// Total bytes read
    pub total_read_bytes: u64,
    /// Total bytes written
    pub total_write_bytes: u64,
    /// Average read latency
    pub avg_read_latency: u64,
    /// Average write latency
    pub avg_write_latency: u64,
    /// Size distribution
    pub size_distribution: BTreeMap<IoSizeBucket, u64>,
    /// Per-process statistics
    pub process_stats: BTreeMap<u64, ProcessIoStats>,
    /// Per-file descriptor statistics
    pub fd_stats: BTreeMap<u64, FileDescriptorStats>,
    /// Raw operations
    pub operations: Vec<IoOperation>,
}

/// I/O profiler implementation
pub struct IoProfiler {
    config: IoProfilerConfig,
    state: Mutex<IoProfilerState>,
    operations: Mutex<Vec<IoOperation>>,
    fd_stats: Mutex<BTreeMap<u64, FileDescriptorStats>>,
    process_stats: Mutex<BTreeMap<u64, ProcessIoStats>>,
    size_distribution: Mutex<BTreeMap<IoSizeBucket, AtomicU64>>,
    total_operations: AtomicU64,
    total_read_bytes: AtomicU64,
    total_write_bytes: AtomicU64,
    total_read_latency: AtomicU64,
    total_write_latency: AtomicU64,
    total_read_ops: AtomicU64,
    total_write_ops: AtomicU64,
    next_op_id: AtomicU64,
}

impl IoProfiler {
    /// Create a new I/O profiler
    pub fn new() -> Self {
        Self::with_config(IoProfilerConfig::default())
    }

    /// Create profiler with custom configuration
    pub fn with_config(config: IoProfilerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(IoProfilerState {
                running: false,
                start_time: None,
                stop_time: None,
            }),
            operations: Mutex::new(Vec::with_capacity(10_000)),
            fd_stats: Mutex::new(BTreeMap::new()),
            process_stats: Mutex::new(BTreeMap::new()),
            size_distribution: Mutex::new(Self::init_size_distribution()),
            total_operations: AtomicU64::new(0),
            total_read_bytes: AtomicU64::new(0),
            total_write_bytes: AtomicU64::new(0),
            total_read_latency: AtomicU64::new(0),
            total_write_latency: AtomicU64::new(0),
            total_read_ops: AtomicU64::new(0),
            total_write_ops: AtomicU64::new(0),
            next_op_id: AtomicU64::new(1),
        }
    }

    /// Initialize size distribution map
    fn init_size_distribution() -> BTreeMap<IoSizeBucket, AtomicU64> {
        let mut map = BTreeMap::new();
        map.insert(IoSizeBucket::Tiny, AtomicU64::new(0));
        map.insert(IoSizeBucket::Small, AtomicU64::new(0));
        map.insert(IoSizeBucket::Medium, AtomicU64::new(0));
        map.insert(IoSizeBucket::Large, AtomicU64::new(0));
        map.insert(IoSizeBucket::Huge, AtomicU64::new(0));
        map.insert(IoSizeBucket::Gigantic, AtomicU64::new(0));
        map
    }

    /// Start profiling
    pub fn start(&self) -> Result<(), IoProfileError> {
        let mut state = self.state.lock();

        if state.running {
            return Err(IoProfileError::AlreadyRunning);
        }

        state.running = true;
        state.start_time = Some(Self::now());

        Ok(())
    }

    /// Stop profiling
    pub fn stop(&self) -> Result<(), IoProfileError> {
        let mut state = self.state.lock();

        if !state.running {
            return Err(IoProfileError::NotRunning);
        }

        state.running = false;
        state.stop_time = Some(Self::now());

        Ok(())
    }

    /// Check if profiler is running
    pub fn is_running(&self) -> bool {
        self.state.lock().running
    }

    /// Record an I/O operation
    pub fn record_operation(
        &self,
        op_type: IoOperationType,
        device_type: IoDeviceType,
        pid: u64,
        tid: u64,
        fd: u64,
        size: u64,
    ) -> Result<u64, IoProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(0);
        }
        drop(state);

        if size == 0 {
            return Err(IoProfileError::InvalidSize(size));
        }

        let id = self.next_op_id.fetch_add(1, Ordering::Relaxed);
        let operation = IoOperation::new(id, op_type, device_type, pid, tid, fd, size);

        // Store operation (will be updated when it completes)
        let mut ops = self.operations.lock();
        if ops.len() < self.config.max_operations {
            ops.push(operation.clone());
        }

        Ok(id)
    }

    /// Complete an I/O operation
    pub fn complete_operation(
        &self,
        id: u64,
        success: bool,
        error: Option<i32>,
    ) -> Result<(), IoProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(());
        }
        drop(state);

        let mut ops = self.operations.lock();
        if let Some(op) = ops.iter_mut().find(|op| op.id == id) {
            op.complete(success, error);

            if let Some(latency_ns) = op.latency_ns {
                self.update_statistics(op, latency_ns);
            }

            Ok(())
        } else {
            Err(IoProfileError::OperationNotFound(id))
        }
    }

    /// Update statistics based on operation
    fn update_statistics(&self, op: &IoOperation, latency_ns: u64) {
        // Update global statistics
        self.total_operations.fetch_add(1, Ordering::Relaxed);

        // Update size distribution
        let bucket = IoSizeBucket::from_size(op.size);
        if let Some(counter) = self.size_distribution.lock().get_mut(&bucket) {
            let counter: &AtomicU64 = counter;
            counter.fetch_add(1, Ordering::Relaxed);
        }

        // Update per-operation type statistics
        match op.op_type {
            IoOperationType::Read => {
                self.total_read_bytes.fetch_add(op.size, Ordering::Relaxed);
                self.total_read_latency.fetch_add(latency_ns, Ordering::Relaxed);
                self.total_read_ops.fetch_add(1, Ordering::Relaxed);
            }
            IoOperationType::Write => {
                self.total_write_bytes.fetch_add(op.size, Ordering::Relaxed);
                self.total_write_latency.fetch_add(latency_ns, Ordering::Relaxed);
                self.total_write_ops.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }

        // Update per-FD statistics
        if self.config.track_by_fd {
            let mut fd_stats = self.fd_stats.lock();
            let fd_stat = fd_stats
                .entry(op.fd)
                .or_insert_with(|| FileDescriptorStats::new(op.fd));

            match op.op_type {
                IoOperationType::Read => {
                    fd_stat.record_read(op.size, latency_ns);
                }
                IoOperationType::Write => {
                    fd_stat.record_write(op.size, latency_ns);
                }
                _ => {}
            }
        }

        // Update per-process statistics
        if self.config.track_by_process {
            let mut proc_stats = self.process_stats.lock();
            let proc_stat = proc_stats
                .entry(op.pid)
                .or_insert_with(|| ProcessIoStats::new(op.pid));

            proc_stat.record_operation(op.op_type, op.size, latency_ns);
        }
    }

    /// Generate profiling report
    pub fn generate_report(&self) -> Result<IoProfileReport, IoProfileError> {
        let state = self.state.lock();
        let start_time = state.start_time.unwrap_or(0);
        let stop_time = state.stop_time.unwrap_or_else(|| Self::now());
        drop(state);

        let duration = Duration::from_nanos(stop_time.saturating_sub(start_time));

        let read_ops = self.total_read_ops.load(Ordering::Relaxed);
        let write_ops = self.total_write_ops.load(Ordering::Relaxed);
        let total_read_latency = self.total_read_latency.load(Ordering::Relaxed);
        let total_write_latency = self.total_write_latency.load(Ordering::Relaxed);

        let avg_read_latency = if read_ops > 0 {
            total_read_latency / read_ops
        } else {
            0
        };

        let avg_write_latency = if write_ops > 0 {
            total_write_latency / write_ops
        } else {
            0
        };

        // Clone size distribution
        let mut size_distribution: BTreeMap<IoSizeBucket, u64> = BTreeMap::new();
        for (bucket, counter) in self.size_distribution.lock().iter() {
            let counter: &AtomicU64 = counter;
            size_distribution.insert(*bucket, counter.load(Ordering::Relaxed));
        }

        Ok(IoProfileReport {
            duration,
            total_operations: self.total_operations.load(Ordering::Relaxed),
            total_read_bytes: self.total_read_bytes.load(Ordering::Relaxed),
            total_write_bytes: self.total_write_bytes.load(Ordering::Relaxed),
            avg_read_latency,
            avg_write_latency,
            size_distribution,
            process_stats: self.process_stats.lock().clone(),
            fd_stats: self.fd_stats.lock().clone(),
            operations: self.operations.lock().clone(),
        })
    }

    /// Get I/O statistics for a specific process
    pub fn get_process_stats(&self, pid: u64) -> Option<ProcessIoStats> {
        self.process_stats.lock().get(&pid).cloned()
    }

    /// Get I/O statistics for a specific file descriptor
    pub fn get_fd_stats(&self, fd: u64) -> Option<FileDescriptorStats> {
        self.fd_stats.lock().get(&fd).cloned()
    }

    /// Clear all profiling data
    pub fn clear(&self) {
        self.operations.lock().clear();
        self.fd_stats.lock().clear();
        self.process_stats.lock().clear();

        self.total_operations.store(0, Ordering::Relaxed);
        self.total_read_bytes.store(0, Ordering::Relaxed);
        self.total_write_bytes.store(0, Ordering::Relaxed);
        self.total_read_latency.store(0, Ordering::Relaxed);
        self.total_write_latency.store(0, Ordering::Relaxed);
        self.total_read_ops.store(0, Ordering::Relaxed);
        self.total_write_ops.store(0, Ordering::Relaxed);

        // Reset size distribution
        for counter in self.size_distribution.lock().values() {
            let counter: &AtomicU64 = counter;
            counter.store(0, Ordering::Relaxed);
        }
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }
}

impl IoProfileReport {
    /// Get top I/O processes by bytes
    pub fn top_processes_by_bytes(&self, n: usize) -> Vec<(u64, u64, u64)> {
        let mut processes: Vec<_> = self
            .process_stats
            .iter()
            .map(|(&pid, stats)| {
                let read_bytes = stats.read_bytes.load(Ordering::Relaxed);
                let write_bytes = stats.write_bytes.load(Ordering::Relaxed);
                (pid, read_bytes, write_bytes)
            })
            .collect();

        processes.sort_by(|a, b| {
            let total_a = a.1 + a.2;
            let total_b = b.1 + b.2;
            total_b.cmp(&total_a)
        });

        processes.truncate(n);
        processes
    }

    /// Export as JSON
    pub fn export_json(&self) -> alloc::string::String {
        let mut json = alloc::string::String::new();

        json.push_str("{\n");
        json.push_str(&format!("  \"duration_ns\": {},\n", self.duration.as_nanos()));
        json.push_str(&format!("  \"total_operations\": {},\n", self.total_operations));
        json.push_str(&format!("  \"total_read_bytes\": {},\n", self.total_read_bytes));
        json.push_str(&format!("  \"total_write_bytes\": {},\n", self.total_write_bytes));
        json.push_str(&format!("  \"avg_read_latency_ns\": {},\n", self.avg_read_latency));
        json.push_str(&format!("  \"avg_write_latency_ns\": {},\n", self.avg_write_latency));
        json.push_str("  \"size_distribution\": {\n");

        let mut buckets: Vec<_> = self.size_distribution.iter().collect();
        buckets.sort_by(|a, b| a.0.cmp(b.0));

        for (i, (bucket, count)) in buckets.iter().enumerate() {
            json.push_str(&format!("    \"{:?}\": {}", bucket, count));
            if i < buckets.len() - 1 {
                json.push_str(",");
            }
            json.push_str("\n");
        }

        json.push_str("  }\n");
        json.push_str("}\n");

        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = IoProfiler::new();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_profiler_start_stop() {
        let profiler = IoProfiler::new();
        profiler.start().unwrap();
        assert!(profiler.is_running());
        profiler.stop().unwrap();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_io_operation_tracking() {
        let profiler = IoProfiler::new();
        profiler.start().unwrap();

        let id = profiler
            .record_operation(IoOperationType::Read, IoDeviceType::BlockDevice, 1, 2, 3, 4096)
            .unwrap();

        profiler.complete_operation(id, true, None).unwrap();

        let report = profiler.generate_report().unwrap();
        assert_eq!(report.total_operations, 1);
        assert_eq!(report.total_read_bytes, 4096);
    }

    #[test]
    fn test_size_classification() {
        assert_eq!(IoSizeBucket::from_size(256), IoSizeBucket::Tiny);
        assert_eq!(IoSizeBucket::from_size(1024), IoSizeBucket::Small);
        assert_eq!(IoSizeBucket::from_size(8192), IoSizeBucket::Medium);
        assert_eq!(IoSizeBucket::from_size(100000), IoSizeBucket::Large);
        assert_eq!(IoSizeBucket::from_size(2000000), IoSizeBucket::Huge);
        assert_eq!(IoSizeBucket::from_size(20000000), IoSizeBucket::Gigantic);
    }

    #[test]
    fn test_fd_statistics() {
        let stats = FileDescriptorStats::new(42);
        stats.record_read(4096, 1000);
        stats.record_write(2048, 500);

        assert_eq!(stats.read_ops.load(Ordering::Relaxed), 1);
        assert_eq!(stats.write_ops.load(Ordering::Relaxed), 1);
        assert_eq!(stats.read_bytes.load(Ordering::Relaxed), 4096);
        assert_eq!(stats.write_bytes.load(Ordering::Relaxed), 2048);
        assert_eq!(stats.avg_read_latency(), 1000);
        assert_eq!(stats.avg_write_latency(), 500);
    }

    #[test]
    fn test_process_statistics() {
        let stats = ProcessIoStats::new(123);
        stats.record_operation(IoOperationType::Read, 4096, 1000);
        stats.record_operation(IoOperationType::Write, 2048, 500);

        assert_eq!(stats.total_ops.load(Ordering::Relaxed), 2);
        assert_eq!(stats.read_bytes.load(Ordering::Relaxed), 4096);
        assert_eq!(stats.write_bytes.load(Ordering::Relaxed), 2048);
    }

    #[test]
    fn test_report_generation() {
        let profiler = IoProfiler::new();
        profiler.start().unwrap();

        let id = profiler
            .record_operation(IoOperationType::Read, IoDeviceType::BlockDevice, 1, 2, 3, 4096)
            .unwrap();
        profiler.complete_operation(id, true, None).unwrap();

        let report = profiler.generate_report().unwrap();
        assert_eq!(report.total_operations, 1);
        assert_eq!(report.total_read_bytes, 4096);
    }

    #[test]
    fn test_size_distribution() {
        let profiler = IoProfiler::new();
        profiler.start().unwrap();

        let id1 = profiler
            .record_operation(IoOperationType::Read, IoDeviceType::BlockDevice, 1, 2, 3, 256)
            .unwrap();
        profiler.complete_operation(id1, true, None).unwrap();

        let id2 = profiler
            .record_operation(IoOperationType::Read, IoDeviceType::BlockDevice, 1, 2, 3, 8192)
            .unwrap();
        profiler.complete_operation(id2, true, None).unwrap();

        let report = profiler.generate_report().unwrap();
        assert_eq!(report.size_distribution.get(&IoSizeBucket::Tiny), Some(&1));
        assert_eq!(report.size_distribution.get(&IoSizeBucket::Medium), Some(&1));
    }

    #[test]
    fn test_invalid_size() {
        let profiler = IoProfiler::new();
        profiler.start().unwrap();

        let result = profiler.record_operation(
            IoOperationType::Read,
            IoDeviceType::BlockDevice,
            1,
            2,
            3,
            0,
        );
        assert!(matches!(result, Err(IoProfileError::InvalidSize(0))));
    }
}
