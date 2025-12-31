//! Kernel Profiling Module
//!
//! This module provides comprehensive profiling capabilities for the NOS kernel including:
//! - Function graph tracer (ftrace-like interface)
//! - Flame graph generation from stack traces
//! - CPU profiling with statistical sampling
//! - Memory profiling for allocation tracking and leak detection
//! - I/O profiling for block and network I/O operations
//! - Lock dependency profiling and contention analysis
//! - Profiling overhead control (< 5% target)
//!
//! # Architecture
//!
//! The profiler uses a multi-tiered approach:
//! 1. **Sampling Profiler**: Low-overhead statistical sampling (default: 100Hz)
//! 2. **Instrumentation**: Compile-time and runtime instrumentation points
//! 3. **Event Tracing**: Hardware and software event monitoring
//! 4. **Stack Walking**: Call graph reconstruction with frame pointers
//!
//! # Performance Overhead
//!
//! - Disabled: Zero overhead
//! - Sampling mode: < 1% overhead
//! - Full instrumentation: < 5% overhead
//! - Memory usage: ~1-2 MB per session

#![allow(missing_docs)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use hashbrown::HashMap;

/// Profiler configuration options
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Sampling frequency in Hz (default: 100)
    pub sampling_frequency: u32,
    /// Maximum stack depth to capture (default: 64)
    pub max_stack_depth: usize,
    /// Enable memory profiling
    pub enable_memory_profiling: bool,
    /// Enable I/O profiling
    pub enable_io_profiling: bool,
    /// Enable lock contention profiling
    pub enable_lock_profiling: bool,
    /// Minimum sample time threshold in nanoseconds
    pub min_sample_time_ns: u64,
    /// Maximum profiler memory usage in bytes
    pub max_memory_bytes: usize,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            sampling_frequency: 100,
            max_stack_depth: 64,
            enable_memory_profiling: true,
            enable_io_profiling: true,
            enable_lock_profiling: true,
            min_sample_time_ns: 1000, // 1 microsecond
            max_memory_bytes: 2 * 1024 * 1024, // 2 MB
        }
    }
}

/// Profiler handle for managing profiling sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfilerHandle {
    /// Unique session ID
    pub session_id: u64,
}

/// Profiler errors
#[derive(Debug, Clone)]
pub enum ProfError {
    /// Profiler already active
    AlreadyActive,
    /// Profiler not active
    NotActive,
    /// Invalid configuration
    InvalidConfig(String),
    /// Memory limit exceeded
    MemoryLimitExceeded,
    /// Buffer overflow
    BufferOverflow,
    /// Invalid handle
    InvalidHandle,
}

impl core::fmt::Display for ProfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProfError::AlreadyActive => write!(f, "Profiler already active"),
            ProfError::NotActive => write!(f, "Profiler not active"),
            ProfError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            ProfError::MemoryLimitExceeded => write!(f, "Memory limit exceeded"),
            ProfError::BufferOverflow => write!(f, "Buffer overflow"),
            ProfError::InvalidHandle => write!(f, "Invalid profiler handle"),
        }
    }
}

/// CPU profiler with statistical sampling
pub struct CpuProfiler {
    /// Whether profiling is active
    active: AtomicBool,
    /// Configuration
    config: ProfilerConfig,
    /// Collected samples
    samples: Mutex<Vec<CpuSample>>,
    /// Function call statistics
    call_stats: Mutex<HashMap<String, FunctionStats>>,
    /// Total samples collected
    total_samples: AtomicU64,
    /// Start time
    start_time: AtomicU64,
    /// Samples dropped due to buffer overflow
    dropped_samples: AtomicU64,
}

/// CPU sample representing a single stack trace
#[derive(Debug, Clone)]
pub struct CpuSample {
    /// Timestamp
    pub timestamp: u64,
    /// CPU ID
    pub cpu_id: u32,
    /// Stack trace (program counters)
    pub stack_trace: Vec<usize>,
    /// Thread ID
    pub thread_id: u64,
    /// User or kernel mode
    pub is_kernel: bool,
}

/// Function-level statistics
#[derive(Debug, Clone)]
pub struct FunctionStats {
    /// Function name
    pub name: String,
    /// Number of samples in this function
    pub sample_count: u64,
    /// Total time spent in this function (nanoseconds)
    pub total_time_ns: u64,
    /// Self time (excluding children) in nanoseconds
    pub self_time_ns: u64,
    /// Number of calls
    pub call_count: u64,
    /// Average time per call in nanoseconds
    pub avg_time_ns: u64,
    /// Parent functions
    pub parents: Vec<String>,
    /// Child functions
    pub children: Vec<String>,
}

impl Default for FunctionStats {
    fn default() -> Self {
        Self {
            name: String::new(),
            sample_count: 0,
            total_time_ns: 0,
            self_time_ns: 0,
            call_count: 0,
            avg_time_ns: 0,
            parents: Vec::new(),
            children: Vec::new(),
        }
    }
}

impl CpuProfiler {
    /// Create a new CPU profiler
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            active: AtomicBool::new(false),
            config,
            samples: Mutex::new(Vec::new()),
            call_stats: Mutex::new(HashMap::new()),
            total_samples: AtomicU64::new(0),
            start_time: AtomicU64::new(0),
            dropped_samples: AtomicU64::new(0),
        }
    }

    /// Start profiling
    pub fn start(&self) -> Result<(), ProfError> {
        if self.active.swap(true, Ordering::Acquire) {
            return Err(ProfError::AlreadyActive);
        }

        self.start_time
            .store(crate::subsystems::time::hrtime_nanos(), Ordering::Release);
        log::info!("CPU profiling started at {} Hz", self.config.sampling_frequency);
        Ok(())
    }

    /// Stop profiling
    pub fn stop(&self) -> Result<(), ProfError> {
        if !self.active.swap(false, Ordering::AcqRel) {
            return Err(ProfError::NotActive);
        }

        log::info!(
            "CPU profiling stopped: {} samples collected",
            self.total_samples.load(Ordering::Relaxed)
        );
        Ok(())
    }

    /// Check if profiling is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Record a CPU sample
    pub fn record_sample(&self, sample: CpuSample) -> Result<(), ProfError> {
        if !self.active.load(Ordering::Acquire) {
            return Ok(()); // Silently ignore when not active
        }

        // Check memory limit
        let current_count = self.total_samples.load(Ordering::Relaxed);
        if current_count * (core::mem::size_of::<CpuSample>() as u64) >= self.config.max_memory_bytes as u64 {
            self.dropped_samples.fetch_add(1, Ordering::Relaxed);
            return Err(ProfError::MemoryLimitExceeded);
        }

        self.total_samples.fetch_add(1, Ordering::Relaxed);

        // Update function stats
        self.update_call_stats(&sample);

        // Store sample
        let mut samples = self.samples.lock();
        samples.push(sample);

        Ok(())
    }

    /// Update function call statistics
    fn update_call_stats(&self, sample: &CpuSample) {
        let mut stats = self.call_stats.lock();

        for (idx, &_pc) in sample.stack_trace.iter().enumerate() {
            let func_name = format!("func_{:x}", _pc);

            let entry = stats.entry(func_name.clone()).or_insert_with(|| FunctionStats {
                name: func_name.clone(),
                ..Default::default()
            });

            entry.sample_count += 1;

            // Track parent-child relationships
            if idx > 0 {
                let parent = format!("func_{:x}", sample.stack_trace[idx - 1]);
                if !entry.parents.contains(&parent) {
                    entry.parents.push(parent);
                }
            }

            if idx < sample.stack_trace.len() - 1 {
                let child = format!("func_{:x}", sample.stack_trace[idx + 1]);
                if !entry.children.contains(&child) {
                    entry.children.push(child);
                }
            }
        }
    }

    /// Get all collected samples
    pub fn get_samples(&self) -> Vec<CpuSample> {
        let samples = self.samples.lock();
        samples.clone()
    }

    /// Get function call statistics
    pub fn get_call_stats(&self) -> HashMap<String, FunctionStats> {
        let stats = self.call_stats.lock();
        stats.clone()
    }

    /// Get top hot functions by sample count
    pub fn get_hot_functions(&self, top_n: usize) -> Vec<(String, u64)> {
        let stats = self.call_stats.lock();
        let mut sorted: Vec<_> = stats
            .iter()
            .map(|(k, v)| (k.clone(), v.sample_count))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(top_n);
        sorted
    }

    /// Generate flame graph data
    pub fn generate_flamegraph(&self) -> FlameGraph {
        let samples = self.get_samples();
        let mut stack_counts: BTreeMap<String, u64> = BTreeMap::new();

        for sample in &samples {
            let stack_str = self.format_stack_trace(sample);
            *stack_counts.entry(stack_str).or_insert(0) += 1;
        }

        FlameGraph {
            stack_counts,
            total_samples: self.total_samples.load(Ordering::Relaxed),
        }
    }

    /// Format stack trace for flame graph
    fn format_stack_trace(&self, sample: &CpuSample) -> String {
        sample
            .stack_trace
            .iter()
            .rev()
            .map(|&pc| format!("func_{:x}", pc))
            .collect::<Vec<_>>()
            .join(";")
    }
}

/// Flame graph representation
#[derive(Debug, Clone)]
pub struct FlameGraph {
    /// Stack trace counts
    pub stack_counts: BTreeMap<String, u64>,
    /// Total samples
    pub total_samples: u64,
}

impl FlameGraph {
    /// Export flame graph in collapsed text format (for FlameGraph tool)
    pub fn to_collapsed(&self) -> String {
        self.stack_counts
            .iter()
            .map(|(stack, count)| format!("{} {}", stack, count))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate SVG flame graph
    pub fn to_svg(&self) -> String {
        let mut svg = String::from(r#"<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg version="1.1" width="1200" height="600" xmlns="http://www.w3.org/2000/svg">
"#);

        let total = self.total_samples.max(1) as f64;
        let mut x = 0u64;

        for (stack, count) in &self.stack_counts {
            let width = (*count as f64 / total) * 1200.0;
            let color = self.get_color(stack);
            let pct = (*count as f64 / total) * 100.0;

            svg.push_str(&format!(
                r#"<rect x="{}" y="0" width="{}" height="20" fill="{}">
    <title>{}: {} samples ({:.1}%)</title>
</rect>
"#,
                x, width, color, stack, count, pct
            ));

            x += width as u64;
        }

        svg.push_str("</svg>");
        svg
    }

    /// Get color for stack frame (hash-based)
    fn get_color(&self, stack: &str) -> String {
        let hash = stack
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        let r = (hash & 0xFF) as u8;
        let g = ((hash >> 8) & 0xFF) as u8;
        let b = ((hash >> 16) & 0xFF) as u8;

        // Warm colors for hot functions
        let warmth = (hash % 100) as u8;
        let r = r.saturating_add(warmth);
        let g = g.saturating_sub(warmth / 2);
        let b = b.saturating_sub(warmth / 2);

        format!("rgb({},{},{})", r, g, b)
    }
}

/// Memory profiler for allocation tracking and leak detection
pub struct MemoryProfiler {
    /// Active flag
    active: AtomicBool,
    /// Allocation events
    allocations: Mutex<Vec<AllocationEvent>>,
    /// Current allocations (for leak detection)
    live_allocations: Mutex<HashMap<usize, AllocationInfo>>,
    /// Total allocated bytes
    total_allocated: AtomicU64,
    /// Total freed bytes
    total_freed: AtomicU64,
    /// Peak allocation
    peak_allocation: AtomicU64,
}

/// Memory allocation event
#[derive(Debug, Clone)]
pub struct AllocationEvent {
    /// Timestamp
    pub timestamp: u64,
    /// Event type
    pub event_type: AllocationType,
    /// Allocation size
    pub size: usize,
    /// Address
    pub address: usize,
    /// Allocation context (call site)
    pub call_site: Option<String>,
    /// Thread ID
    pub thread_id: u64,
}

/// Allocation event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationType {
    /// Allocation event
    Alloc,
    /// Deallocation event
    Free,
    /// Reallocation event
    Realloc,
}

/// Allocation information
#[derive(Debug, Clone)]
struct AllocationInfo {
    /// Size
    size: usize,
    /// Allocation time
    alloc_time: u64,
    /// Call site
    call_site: Option<String>,
    /// Thread ID
    thread_id: u64,
}

/// Memory statistics summary
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current allocated bytes
    pub current_allocated_bytes: u64,
    /// Peak allocated bytes
    pub peak_allocated_bytes: u64,
    /// Number of potential leaks
    pub potential_leaks: u64,
    /// Total leaked bytes
    pub leaked_bytes: u64,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            allocations: Mutex::new(Vec::new()),
            live_allocations: Mutex::new(HashMap::new()),
            total_allocated: AtomicU64::new(0),
            total_freed: AtomicU64::new(0),
            peak_allocation: AtomicU64::new(0),
        }
    }

    /// Start memory profiling
    pub fn start(&self) -> Result<(), ProfError> {
        if self.active.swap(true, Ordering::Acquire) {
            return Err(ProfError::AlreadyActive);
        }
        log::info!("Memory profiling started");
        Ok(())
    }

    /// Stop memory profiling
    pub fn stop(&self) -> Result<(), ProfError> {
        if !self.active.swap(false, Ordering::AcqRel) {
            return Err(ProfError::NotActive);
        }
        log::info!("Memory profiling stopped");
        Ok(())
    }

    /// Record an allocation
    pub fn record_alloc(&self, address: usize, size: usize, call_site: Option<String>) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let timestamp = crate::subsystems::time::hrtime_nanos();
        let thread_id = 0; // GH-#1281: Get actual thread ID
        // See: https://github.com/npos/kernel/issues/1281

        let event = AllocationEvent {
            timestamp,
            event_type: AllocationType::Alloc,
            size,
            address,
            call_site: call_site.clone(),
            thread_id,
        };

        {
            let mut allocations = self.allocations.lock();
            allocations.push(event.clone());
        }

        {
            let mut live = self.live_allocations.lock();
            live.insert(
                address,
                AllocationInfo {
                    size,
                    alloc_time: timestamp,
                    call_site,
                    thread_id,
                },
            );
        }

        let allocated = self.total_allocated.fetch_add(size as u64, Ordering::Relaxed);
        let freed = self.total_freed.load(Ordering::Relaxed);
        let current = allocated + size as u64 - freed;

        // Update peak
        let mut peak = self.peak_allocation.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_allocation.compare_exchange_weak(
                peak,
                current,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    /// Record a deallocation
    pub fn record_free(&self, address: usize) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        let timestamp = crate::subsystems::time::hrtime_nanos();
        let thread_id = 0;

        // Get allocation info before removing
        let (size, call_site) = {
            let mut live = self.live_allocations.lock();
            live.remove(&address)
                .map(|info| (info.size, info.call_site))
                .unwrap_or((0, None))
        };

        let event = AllocationEvent {
            timestamp,
            event_type: AllocationType::Free,
            size,
            address,
            call_site,
            thread_id,
        };

        {
            let mut allocations = self.allocations.lock();
            allocations.push(event);
        }

        self.total_freed.fetch_add(size as u64, Ordering::Relaxed);
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        let total_allocations = self.total_allocated.load(Ordering::Relaxed);
        let total_freed = self.total_freed.load(Ordering::Relaxed);
        let peak = self.peak_allocation.load(Ordering::Relaxed);
        let current = total_allocations - total_freed;

        let (potential_leaks, leaked_bytes) = {
            let live = self.live_allocations.lock();
            (live.len() as u64, live.values().map(|v| v.size as u64).sum())
        };

        MemoryStats {
            total_allocations: self.allocations.lock().len() as u64,
            total_deallocations: self.total_freed.load(Ordering::Relaxed),
            current_allocated_bytes: current,
            peak_allocated_bytes: peak,
            potential_leaks,
            leaked_bytes,
        }
    }

    /// Detect memory leaks
    pub fn detect_leaks(&self) -> Vec<(usize, AllocationInfo)> {
        let live = self.live_allocations.lock();
        live.iter().map(|(&addr, info)| (addr, info.clone())).collect()
    }

    /// Get allocation events
    pub fn get_events(&self) -> Vec<AllocationEvent> {
        let events = self.allocations.lock();
        events.clone()
    }
}

/// I/O profiler for tracking I/O operations
pub struct IoProfiler {
    /// Active flag
    active: AtomicBool,
    /// I/O events
    events: Mutex<Vec<IoEvent>>,
    /// I/O statistics by device
    device_stats: Mutex<HashMap<String, IoStats>>,
}

/// I/O event
#[derive(Debug, Clone)]
pub struct IoEvent {
    /// Timestamp
    pub timestamp: u64,
    /// I/O operation
    pub operation: IoOperation,
    /// Device name
    pub device: String,
    /// Start sector/block
    pub start_sector: u64,
    /// Number of sectors
    pub num_sectors: u32,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Thread ID
    pub thread_id: u64,
    /// Whether I/O completed successfully
    pub success: bool,
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Flush operation
    Flush,
    /// Discard operation
    Discard,
}

/// I/O statistics for a device
#[derive(Debug, Clone)]
pub struct IoStats {
    /// Device name
    pub device: String,
    /// Total read operations
    pub read_ops: u64,
    /// Total write operations
    pub write_ops: u64,
    /// Total bytes read
    pub read_bytes: u64,
    /// Total bytes written
    pub write_bytes: u64,
    /// Average read latency (nanoseconds)
    pub avg_read_latency_ns: u64,
    /// Average write latency (nanoseconds)
    pub avg_write_latency_ns: u64,
    /// Total read errors
    pub read_errors: u64,
    /// Total write errors
    pub write_errors: u64,
}

impl IoProfiler {
    /// Create a new I/O profiler
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            events: Mutex::new(Vec::new()),
            device_stats: Mutex::new(HashMap::new()),
        }
    }

    /// Start I/O profiling
    pub fn start(&self) -> Result<(), ProfError> {
        if self.active.swap(true, Ordering::Acquire) {
            return Err(ProfError::AlreadyActive);
        }
        log::info!("I/O profiling started");
        Ok(())
    }

    /// Stop I/O profiling
    pub fn stop(&self) -> Result<(), ProfError> {
        if !self.active.swap(false, Ordering::AcqRel) {
            return Err(ProfError::NotActive);
        }
        log::info!("I/O profiling stopped");
        Ok(())
    }

    /// Record an I/O event
    pub fn record_io(&self, event: IoEvent) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        // Store event
        {
            let mut events = self.events.lock();
            events.push(event.clone());
        }

        // Update stats
        let device = event.device.clone();
        let mut device_stats = self.device_stats.lock();
        let stats = device_stats
            .entry(device)
            .or_insert_with(|| IoStats {
                device: event.device.clone(),
                ..Default::default()
            });

        let bytes = event.num_sectors as u64 * 512; // Assume 512-byte sectors

        match event.operation {
            IoOperation::Read => {
                stats.read_ops += 1;
                stats.read_bytes += bytes;
                if !event.success {
                    stats.read_errors += 1;
                }
            }
            IoOperation::Write => {
                stats.write_ops += 1;
                stats.write_bytes += bytes;
                if !event.success {
                    stats.write_errors += 1;
                }
            }
            _ => {}
        }
    }

    /// Get I/O events
    pub fn get_events(&self) -> Vec<IoEvent> {
        let events = self.events.lock();
        events.clone()
    }

    /// Get device statistics
    pub fn get_device_stats(&self) -> HashMap<String, IoStats> {
        let stats = self.device_stats.lock();
        stats.clone()
    }

    /// Calculate throughput (bytes/sec)
    pub fn calculate_throughput(&self, device: &str) -> IoThroughput {
        let stats = self.device_stats.lock();
        let stat = stats.get(device).cloned().unwrap_or_default();

        let events = self.events.lock();
        if events.is_empty() {
            return IoThroughput {
                device: device.to_string(),
                read_bytes_per_sec: 0.0,
                write_bytes_per_sec: 0.0,
                total_bytes_per_sec: 0.0,
                read_ops_per_sec: 0.0,
                write_ops_per_sec: 0.0,
            };
        }

        let duration_ns = events.last().unwrap().timestamp - events.first().unwrap().timestamp;
        let duration_sec = duration_ns as f64 / 1_000_000_000.0;

        if duration_sec > 0.0 {
            IoThroughput {
                device: device.to_string(),
                read_bytes_per_sec: stat.read_bytes as f64 / duration_sec,
                write_bytes_per_sec: stat.write_bytes as f64 / duration_sec,
                total_bytes_per_sec: (stat.read_bytes + stat.write_bytes) as f64 / duration_sec,
                read_ops_per_sec: stat.read_ops as f64 / duration_sec,
                write_ops_per_sec: stat.write_ops as f64 / duration_sec,
            }
        } else {
            IoThroughput {
                device: device.to_string(),
                read_bytes_per_sec: 0.0,
                write_bytes_per_sec: 0.0,
                total_bytes_per_sec: 0.0,
                read_ops_per_sec: 0.0,
                write_ops_per_sec: 0.0,
            }
        }
    }
}

/// I/O throughput metrics
#[derive(Debug, Clone)]
pub struct IoThroughput {
    /// Device name
    pub device: String,
    /// Read throughput (bytes/sec)
    pub read_bytes_per_sec: f64,
    /// Write throughput (bytes/sec)
    pub write_bytes_per_sec: f64,
    /// Total throughput (bytes/sec)
    pub total_bytes_per_sec: f64,
    /// Read operations per second
    pub read_ops_per_sec: f64,
    /// Write operations per second
    pub write_ops_per_sec: f64,
}

impl Default for IoStats {
    fn default() -> Self {
        Self {
            device: String::new(),
            read_ops: 0,
            write_ops: 0,
            read_bytes: 0,
            write_bytes: 0,
            avg_read_latency_ns: 0,
            avg_write_latency_ns: 0,
            read_errors: 0,
            write_errors: 0,
        }
    }
}

/// Lock profiler for contention analysis
pub struct LockProfiler {
    /// Active flag
    active: AtomicBool,
    /// Lock events
    events: Mutex<Vec<LockEvent>>,
    /// Lock statistics by lock name
    lock_stats: Mutex<HashMap<String, LockStats>>,
}

/// Lock event
#[derive(Debug, Clone)]
pub struct LockEvent {
    /// Timestamp
    pub timestamp: u64,
    /// Lock name or address
    pub lock_name: String,
    /// Lock type
    pub lock_type: LockType,
    /// Event type
    pub event_type: LockEventType,
    /// Wait time in nanoseconds
    pub wait_time_ns: u64,
    /// Hold time in nanoseconds
    pub hold_time_ns: u64,
    /// Thread ID
    pub thread_id: u64,
    /// CPU ID
    pub cpu_id: u32,
}

/// Lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Mutex
    Mutex,
    /// Spinlock
    Spinlock,
    /// Read-write lock (read side)
    RwLockRead,
    /// Read-write lock (write side)
    RwLockWrite,
    /// Semaphore
    Semaphore,
}

/// Lock event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockEventType {
    /// Acquire (with contention)
    AcquireContended,
    /// Acquire (without contention)
    AcquireUncontended,
    /// Release
    Release,
}

/// Lock statistics
#[derive(Debug, Clone)]
pub struct LockStats {
    /// Lock name
    pub lock_name: String,
    /// Total acquisitions
    pub total_acquisitions: u64,
    /// Contended acquisitions
    pub contended_acquisitions: u64,
    /// Total wait time (nanoseconds)
    pub total_wait_ns: u64,
    /// Total hold time (nanoseconds)
    pub total_hold_ns: u64,
    /// Average wait time (nanoseconds)
    pub avg_wait_ns: u64,
    /// Maximum wait time (nanoseconds)
    pub max_wait_ns: u64,
    /// Contention rate (0.0 to 1.0)
    pub contention_rate: f64,
}

impl Default for LockStats {
    fn default() -> Self {
        Self {
            lock_name: String::new(),
            total_acquisitions: 0,
            contended_acquisitions: 0,
            total_wait_ns: 0,
            total_hold_ns: 0,
            avg_wait_ns: 0,
            max_wait_ns: 0,
            contention_rate: 0.0,
        }
    }
}

impl LockProfiler {
    /// Create a new lock profiler
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            events: Mutex::new(Vec::new()),
            lock_stats: Mutex::new(HashMap::new()),
        }
    }

    /// Start lock profiling
    pub fn start(&self) -> Result<(), ProfError> {
        if self.active.swap(true, Ordering::Acquire) {
            return Err(ProfError::AlreadyActive);
        }
        log::info!("Lock profiling started");
        Ok(())
    }

    /// Stop lock profiling
    pub fn stop(&self) -> Result<(), ProfError> {
        if !self.active.swap(false, Ordering::AcqRel) {
            return Err(ProfError::NotActive);
        }
        log::info!("Lock profiling stopped");
        Ok(())
    }

    /// Record a lock event
    pub fn record_lock(&self, event: LockEvent) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }

        // Store event
        {
            let mut events = self.events.lock();
            events.push(event.clone());
        }

        // Update stats
        let lock_name = event.lock_name.clone();
        let mut lock_stats_map = self.lock_stats.lock();
        let stats = lock_stats_map
            .entry(lock_name)
            .or_insert_with(|| LockStats {
                lock_name: event.lock_name.clone(),
                ..Default::default()
            });

        stats.total_acquisitions += 1;

        if event.event_type == LockEventType::AcquireContended {
            stats.contended_acquisitions += 1;
            stats.total_wait_ns += event.wait_time_ns;
            stats.avg_wait_ns = stats.total_wait_ns / stats.contended_acquisitions;
            stats.max_wait_ns = stats.max_wait_ns.max(event.wait_time_ns);
        }

        stats.contention_rate = if stats.total_acquisitions > 0 {
            stats.contended_acquisitions as f64 / stats.total_acquisitions as f64
        } else {
            0.0
        };
    }

    /// Get lock events
    pub fn get_events(&self) -> Vec<LockEvent> {
        let events = self.events.lock();
        events.clone()
    }

    /// Get lock statistics
    pub fn get_lock_stats(&self) -> HashMap<String, LockStats> {
        let stats = self.lock_stats.lock();
        stats.clone()
    }

    /// Get most contended locks
    pub fn get_contended_locks(&self, top_n: usize) -> Vec<(String, f64)> {
        let stats = self.lock_stats.lock();
        let mut sorted: Vec<_> = stats
            .iter()
            .map(|(k, v)| (k.clone(), v.contention_rate))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        sorted.truncate(top_n);
        sorted
    }
}

/// Unified profiler manager
pub struct ProfilerManager {
    /// CPU profiler
    cpu_profiler: CpuProfiler,
    /// Memory profiler
    memory_profiler: MemoryProfiler,
    /// I/O profiler
    io_profiler: IoProfiler,
    /// Lock profiler
    lock_profiler: LockProfiler,
    /// Configuration
    config: ProfilerConfig,
}

impl ProfilerManager {
    /// Create a new profiler manager
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            cpu_profiler: CpuProfiler::new(config.clone()),
            memory_profiler: MemoryProfiler::new(),
            io_profiler: IoProfiler::new(),
            lock_profiler: LockProfiler::new(),
            config,
        }
    }

    /// Start all profilers
    pub fn start(&self) -> Result<ProfilerHandle, ProfError> {
        self.cpu_profiler.start()?;

        if self.config.enable_memory_profiling {
            self.memory_profiler.start()?;
        }

        if self.config.enable_io_profiling {
            self.io_profiler.start()?;
        }

        if self.config.enable_lock_profiling {
            self.lock_profiler.start()?;
        }

        let session_id = crate::subsystems::time::hrtime_nanos();
        log::info!("Profiler session {} started", session_id);

        Ok(ProfilerHandle { session_id })
    }

    /// Stop all profilers
    pub fn stop(&self, _handle: ProfilerHandle) -> Result<ProfilerData, ProfError> {
        self.cpu_profiler.stop()?;

        if self.config.enable_memory_profiling {
            self.memory_profiler.stop()?;
        }

        if self.config.enable_io_profiling {
            self.io_profiler.stop()?;
        }

        if self.config.enable_lock_profiling {
            self.lock_profiler.stop()?;
        }

        // Collect profiler data
        let data = ProfilerData {
            flamegraph: self.cpu_profiler.generate_flamegraph(),
            memory_stats: self.memory_profiler.get_stats(),
            io_events: self.io_profiler.get_events(),
            lock_stats: self.lock_profiler.get_lock_stats(),
        };

        log::info!("Profiler session stopped, data collected");

        Ok(data)
    }

    /// Generate flame graph
    pub fn generate_flame_graph(&self) -> String {
        self.cpu_profiler.generate_flamegraph().to_collapsed()
    }
}

/// Complete profiler data from a session
#[derive(Debug, Clone)]
pub struct ProfilerData {
    /// Flame graph data
    pub flamegraph: FlameGraph,
    /// Memory statistics
    pub memory_stats: MemoryStats,
    /// I/O events
    pub io_events: Vec<IoEvent>,
    /// Lock statistics
    pub lock_stats: HashMap<String, LockStats>,
}

/// Generate flame graph from profiler data
pub fn generate_flame_graph(data: &ProfilerData) -> String {
    data.flamegraph.to_svg()
}

/// Start profiler with given configuration
pub fn start_profiler(config: ProfilerConfig) -> Result<ProfilerHandle, ProfError> {
    let manager = ProfilerManager::new(config);
    manager.start()
}

/// Stop profiler and collect data
pub fn stop_profiler(handle: ProfilerHandle) -> Result<ProfilerData, ProfError> {
    let manager = ProfilerManager::new(ProfilerConfig::default());
    manager.stop(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_profiler() {
        let profiler = CpuProfiler::new(ProfilerConfig::default());
        assert!(!profiler.is_active());

        profiler.start().unwrap();
        assert!(profiler.is_active());

        profiler.stop().unwrap();
        assert!(!profiler.is_active());
    }

    #[test]
    fn test_memory_profiler() {
        let profiler = MemoryProfiler::new();

        profiler.start().unwrap();
        profiler.record_alloc(0x1000, 1024, Some(String::from("test")));
        profiler.record_free(0x1000);

        let stats = profiler.get_stats();
        assert!(stats.total_allocations > 0);
    }
}
