//! Memory Performance Profiling
//!
//! This module provides heap profiling, allocation tracking, and memory leak detection
//! for the NOS kernel.
//!
//! # Features
//!
//! - Heap profiling and allocation tracking
//! - Memory leak detection
//! - Heap snapshot generation
//! - Memory usage by allocation site
//! - Fragmentation analysis
//! - Allocation size distribution
//! - Live object tracking
//!
//! # Usage
//!
//! ```rust
//! use kernel::profiling::memory::MemoryProfiler;
//!
//! let profiler = MemoryProfiler::new();
//! profiler.start().unwrap();
//! // ... allocations to profile ...
//! profiler.stop().unwrap();
//! let snapshot = profiler.take_snapshot().unwrap();
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::sync::Mutex;

/// Memory profiling error types
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryProfileError {
    /// Profiler is already running
    AlreadyRunning,
    /// Profiler is not running
    NotRunning,
    /// Invalid allocation size
    InvalidAllocationSize(usize),
    /// Snapshot buffer overflow
    SnapshotOverflow,
    /// Allocation not found
    AllocationNotFound(u64),
    /// Corruption detected
    CorruptionDetected,
}

impl core::fmt::Display for MemoryProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "Memory profiler is already running"),
            Self::NotRunning => write!(f, "Memory profiler is not running"),
            Self::InvalidAllocationSize(size) => write!(f, "Invalid allocation size: {}", size),
            Self::SnapshotOverflow => write!(f, "Snapshot buffer overflow"),
            Self::AllocationNotFound(id) => write!(f, "Allocation not found: {}", id),
            Self::CorruptionDetected => write!(f, "Memory corruption detected"),
        }
    }
}

/// Allocation site identifier (call stack hash)
pub type AllocationSite = u64;

/// Unique allocation ID
pub type AllocationId = u64;

/// Allocation record tracking a single memory allocation
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// Unique allocation ID
    pub id: AllocationId,
    /// Allocation site (caller address or call stack hash)
    pub site: AllocationSite,
    /// Size in bytes
    pub size: usize,
    /// Alignment
    pub alignment: usize,
    /// Allocation timestamp
    pub timestamp: u64,
    /// Thread ID that made the allocation
    pub thread_id: u64,
    /// Stack trace of allocation (optional)
    pub stack_trace: Option<Vec<u64>>,
    /// Whether allocation is freed
    pub freed: bool,
    /// Free timestamp
    pub free_timestamp: Option<u64>,
}

impl AllocationRecord {
    /// Create a new allocation record
    pub fn new(
        id: AllocationId,
        site: AllocationSite,
        size: usize,
        alignment: usize,
        thread_id: u64,
    ) -> Self {
        Self {
            id,
            site,
            size,
            alignment,
            timestamp: Self::now(),
            thread_id,
            stack_trace: None,
            freed: false,
            free_timestamp: None,
        }
    }

    /// Mark allocation as freed
    pub fn mark_freed(&mut self) {
        self.freed = true;
        self.free_timestamp = Some(Self::now());
    }

    /// Check if allocation is live (not freed)
    pub fn is_live(&self) -> bool {
        !self.freed
    }

    /// Get allocation age in nanoseconds
    pub fn age(&self) -> u64 {
        let end = self.free_timestamp.unwrap_or_else(Self::now);
        end.saturating_sub(self.timestamp)
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }
}

/// Statistics for an allocation site
#[derive(Debug)]
pub struct AllocationSiteStats {
    /// Allocation site identifier
    pub site: AllocationSite,
    /// Total allocations
    pub total_allocations: AtomicU64,
    /// Total deallocations
    pub total_deallocations: AtomicU64,
    /// Current live allocations
    pub live_allocations: AtomicU64,
    /// Peak live allocations
    pub peak_allocations: AtomicU64,
    /// Total bytes allocated
    pub total_bytes: AtomicU64,
    /// Current live bytes
    pub live_bytes: AtomicU64,
    /// Peak bytes allocated
    pub peak_bytes: AtomicU64,
    /// Average allocation size
    pub avg_size: AtomicU64,
}

impl Clone for AllocationSiteStats {
    fn clone(&self) -> Self {
        Self {
            site: self.site,
            total_allocations: AtomicU64::new(self.total_allocations.load(Ordering::Relaxed)),
            total_deallocations: AtomicU64::new(self.total_deallocations.load(Ordering::Relaxed)),
            live_allocations: AtomicU64::new(self.live_allocations.load(Ordering::Relaxed)),
            peak_allocations: AtomicU64::new(self.peak_allocations.load(Ordering::Relaxed)),
            total_bytes: AtomicU64::new(self.total_bytes.load(Ordering::Relaxed)),
            live_bytes: AtomicU64::new(self.live_bytes.load(Ordering::Relaxed)),
            peak_bytes: AtomicU64::new(self.peak_bytes.load(Ordering::Relaxed)),
            avg_size: AtomicU64::new(self.avg_size.load(Ordering::Relaxed)),
        }
    }
}

impl AllocationSiteStats {
    /// Create new allocation site statistics
    pub fn new(site: AllocationSite) -> Self {
        Self {
            site,
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            live_allocations: AtomicU64::new(0),
            peak_allocations: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            live_bytes: AtomicU64::new(0),
            peak_bytes: AtomicU64::new(0),
            avg_size: AtomicU64::new(0),
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, size: usize) {
        let count = self.total_allocations.fetch_add(1, Ordering::Relaxed) + 1;
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);
        self.live_bytes.fetch_add(size as u64, Ordering::Relaxed);

        let live = self.live_allocations.fetch_add(1, Ordering::Relaxed) + 1;

        // Update peaks
        let mut peak = self.peak_allocations.load(Ordering::Relaxed);
        while live > peak {
            match self.peak_allocations.compare_exchange_weak(
                peak,
                live,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }

        let mut peak_bytes = self.peak_bytes.load(Ordering::Relaxed);
        let current_bytes = self.live_bytes.load(Ordering::Relaxed);
        while current_bytes > peak_bytes {
            match self.peak_bytes.compare_exchange_weak(
                peak_bytes,
                current_bytes,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak_bytes = new_peak,
            }
        }

        // Update average
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        self.avg_size.store(total_bytes / count, Ordering::Relaxed);
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, size: usize) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.live_allocations.fetch_sub(1, Ordering::Relaxed);
        self.live_bytes.fetch_sub(size as u64, Ordering::Relaxed);
    }

    /// Get current live allocations
    pub fn get_live_allocations(&self) -> u64 {
        self.live_allocations.load(Ordering::Relaxed)
    }

    /// Get current live bytes
    pub fn get_live_bytes(&self) -> u64 {
        self.live_bytes.load(Ordering::Relaxed)
    }

    /// Check for potential leak (more allocations than deallocations)
    pub fn potential_leak(&self, threshold: f64) -> bool {
        let total = self.total_allocations.load(Ordering::Relaxed) as f64;
        let freed = self.total_deallocations.load(Ordering::Relaxed) as f64;

        if total < 100.0 {
            return false; // Too few samples to determine
        }

        let leak_ratio = (total - freed) / total;
        leak_ratio > threshold
    }
}

/// Memory size bucket for allocation distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeBucket {
    Tiny,     // 0-16 bytes
    Small,    // 17-256 bytes
    Medium,   // 257-4KB
    Large,    // 4KB-128KB
    Huge,     // 128KB-1MB
    Gigantic, // >1MB
}

impl SizeBucket {
    /// Classify allocation size into bucket
    pub fn from_size(size: usize) -> Self {
        match size {
            0..=16 => Self::Tiny,
            17..=256 => Self::Small,
            257..=4096 => Self::Medium,
            4097..=131_072 => Self::Large,
            131_073..=1_048_576 => Self::Huge,
            _ => Self::Gigantic,
        }
    }
}

/// Memory profiler configuration
#[derive(Debug, Clone)]
pub struct MemoryProfilerConfig {
    /// Track individual allocations
    pub track_allocations: bool,
    /// Capture stack traces for allocations
    pub capture_stack_traces: bool,
    /// Maximum stack depth to capture
    pub max_stack_depth: usize,
    /// Maximum number of allocation records
    pub max_allocations: usize,
    /// Minimum allocation size to track
    pub min_tracked_size: usize,
    /// Leak detection threshold (ratio of leaked to total)
    pub leak_threshold: f64,
    /// Track by thread
    pub track_by_thread: bool,
}

impl Default for MemoryProfilerConfig {
    fn default() -> Self {
        Self {
            track_allocations: true,
            capture_stack_traces: false, // Expensive
            max_stack_depth: 16,
            max_allocations: 1_000_000,
            min_tracked_size: 0,
            leak_threshold: 0.5, // 50%
            track_by_thread: true,
        }
    }
}

/// Memory profiler state
struct MemoryProfilerState {
    running: bool,
    start_time: Option<u64>,
    stop_time: Option<u64>,
}

/// Heap snapshot data
#[derive(Debug, Clone)]
pub struct HeapSnapshot {
    /// Snapshot timestamp
    pub timestamp: u64,
    /// Total live allocations
    pub total_live_allocations: u64,
    /// Total live bytes
    pub total_live_bytes: u64,
    /// Per-site statistics
    pub site_stats: BTreeMap<AllocationSite, AllocationSiteStats>,
    /// Size distribution
    pub size_distribution: BTreeMap<SizeBucket, u64>,
    /// Thread distribution
    pub thread_distribution: BTreeMap<u64, (u64, u64)>, // (allocations, bytes)
    /// Live allocation records
    pub live_allocations: Vec<AllocationRecord>,
}

/// Memory profiler implementation
pub struct MemoryProfiler {
    config: MemoryProfilerConfig,
    state: Mutex<MemoryProfilerState>,
    allocations: Mutex<BTreeMap<AllocationId, AllocationRecord>>,
    site_stats: Mutex<BTreeMap<AllocationSite, AllocationSiteStats>>,
    size_distribution: Mutex<BTreeMap<SizeBucket, AtomicU64>>,
    next_id: AtomicU64,
    total_allocated: AtomicU64,
    total_freed: AtomicU64,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    pub fn new() -> Self {
        Self::with_config(MemoryProfilerConfig::default())
    }

    /// Create profiler with custom configuration
    pub fn with_config(config: MemoryProfilerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(MemoryProfilerState {
                running: false,
                start_time: None,
                stop_time: None,
            }),
            allocations: Mutex::new(BTreeMap::new()),
            site_stats: Mutex::new(BTreeMap::new()),
            size_distribution: Mutex::new(Self::init_size_distribution()),
            next_id: AtomicU64::new(1),
            total_allocated: AtomicU64::new(0),
            total_freed: AtomicU64::new(0),
        }
    }

    /// Initialize size distribution map
    fn init_size_distribution() -> BTreeMap<SizeBucket, AtomicU64> {
        let mut map = BTreeMap::new();
        map.insert(SizeBucket::Tiny, AtomicU64::new(0));
        map.insert(SizeBucket::Small, AtomicU64::new(0));
        map.insert(SizeBucket::Medium, AtomicU64::new(0));
        map.insert(SizeBucket::Large, AtomicU64::new(0));
        map.insert(SizeBucket::Huge, AtomicU64::new(0));
        map.insert(SizeBucket::Gigantic, AtomicU64::new(0));
        map
    }

    /// Start memory profiling
    pub fn start(&self) -> Result<(), MemoryProfileError> {
        let mut state = self.state.lock();

        if state.running {
            return Err(MemoryProfileError::AlreadyRunning);
        }

        state.running = true;
        state.start_time = Some(Self::now());

        Ok(())
    }

    /// Stop memory profiling
    pub fn stop(&self) -> Result<(), MemoryProfileError> {
        let mut state = self.state.lock();

        if !state.running {
            return Err(MemoryProfileError::NotRunning);
        }

        state.running = false;
        state.stop_time = Some(Self::now());

        Ok(())
    }

    /// Check if profiler is running
    pub fn is_running(&self) -> bool {
        self.state.lock().running
    }

    /// Record an allocation
    pub fn record_allocation(
        &self,
        site: AllocationSite,
        size: usize,
        alignment: usize,
    ) -> Result<AllocationId, MemoryProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(0); // Silently ignore when not running
        }
        drop(state);

        if size == 0 || size > 1024 * 1024 * 1024 {
            return Err(MemoryProfileError::InvalidAllocationSize(size));
        }

        // Check allocation limit
        if self.allocations.lock().len() >= self.config.max_allocations {
            return Err(MemoryProfileError::SnapshotOverflow);
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let thread_id = self.current_thread_id();

        let mut record = AllocationRecord::new(id, site, size, alignment, thread_id);

        // Optionally capture stack trace
        if self.config.capture_stack_traces {
            record.stack_trace = Some(self.capture_stack_trace());
        }

        // Store allocation
        self.allocations.lock().insert(id, record.clone());

        // Update site statistics
        let mut stats = self.site_stats.lock();
        let site_stat = stats.entry(site).or_insert_with(|| AllocationSiteStats::new(site));
        site_stat.record_allocation(size);
        drop(stats);

        // Update size distribution
        let bucket = SizeBucket::from_size(size);
        if let Some(counter) = self.size_distribution.lock().get_mut(&bucket) {
            let counter: &AtomicU64 = counter;
            counter.fetch_add(1, Ordering::Relaxed);
        }

        // Update totals
        self.total_allocated.fetch_add(size as u64, Ordering::Relaxed);

        Ok(id)
    }

    /// Record a deallocation
    pub fn record_deallocation(
        &self,
        id: AllocationId,
    ) -> Result<(), MemoryProfileError> {
        let state = self.state.lock();
        if !state.running {
            return Ok(()); // Silently ignore when not running
        }
        drop(state);

        let mut allocations = self.allocations.lock();
        let allocation = allocations
            .get_mut(&id)
            .ok_or(MemoryProfileError::AllocationNotFound(id))?;

        if allocation.freed {
            return Err(MemoryProfileError::CorruptionDetected); // Double free
        }

        allocation.mark_freed();

        let size = allocation.size;
        let site = allocation.site;

        // Update site statistics
        let mut stats = self.site_stats.lock();
        if let Some(site_stat) = stats.get_mut(&site) {
            let site_stat: &mut AllocationSiteStats = site_stat;
            site_stat.record_deallocation(size);
        }

        // Update totals
        self.total_freed.fetch_add(size as u64, Ordering::Relaxed);

        Ok(())
    }

    /// Take a heap snapshot
    pub fn take_snapshot(&self) -> Result<HeapSnapshot, MemoryProfileError> {
        let allocations = self.allocations.lock();
        let site_stats = self.site_stats.lock();
        let size_dist = self.size_distribution.lock();

        // Calculate live allocations and bytes
        let mut total_live_allocations = 0u64;
        let mut total_live_bytes = 0u64;
        let mut live_records: Vec<AllocationRecord> = Vec::new();
        let mut thread_dist: BTreeMap<u64, (u64, u64)> = BTreeMap::new();

        for record in allocations.values() {
            if record.is_live() {
                total_live_allocations += 1;
                total_live_bytes += record.size as u64;
                live_records.push(record.clone());

                // Update thread distribution
                let entry = thread_dist
                    .entry(record.thread_id)
                    .or_insert((0, 0));
                entry.0 += 1;
                entry.1 += record.size as u64;
            }
        }

        // Clone size distribution
        let mut size_distribution = BTreeMap::new();
        for (bucket, counter) in size_dist.iter() {
            size_distribution.insert(*bucket, counter.load(Ordering::Relaxed));
        }

        Ok(HeapSnapshot {
            timestamp: Self::now(),
            total_live_allocations,
            total_live_bytes,
            site_stats: site_stats.clone(),
            size_distribution,
            thread_distribution: thread_dist,
            live_allocations: live_records,
        })
    }

    /// Detect memory leaks
    pub fn detect_leaks(&self) -> Vec<(AllocationSite, AllocationSiteStats)> {
        let site_stats = self.site_stats.lock();
        let threshold = self.config.leak_threshold;

        site_stats
            .iter()
            .filter(|(_, stats)| stats.potential_leak(threshold))
            .map(|(&site, stats)| (site, stats.clone()))
            .collect()
    }

    /// Get fragmentation info
    pub fn fragmentation_info(&self) -> FragmentationInfo {
        let snapshot = self.take_snapshot().unwrap_or_else(|_| HeapSnapshot {
            timestamp: 0,
            total_live_allocations: 0,
            total_live_bytes: 0,
            site_stats: BTreeMap::new(),
            size_distribution: BTreeMap::new(),
            thread_distribution: BTreeMap::new(),
            live_allocations: Vec::new(),
        });

        let total_allocated = self.total_allocated.load(Ordering::Relaxed);
        let total_freed = self.total_freed.load(Ordering::Relaxed);
        let current_live = snapshot.total_live_bytes;

        // Calculate fragmentation metrics
        let fragmentation_ratio = if total_allocated > 0 {
            (total_allocated - current_live) as f64 / total_allocated as f64
        } else {
            0.0
        };

        // Count size buckets
        let size_variety = snapshot.size_distribution.len();

        FragmentationInfo {
            current_live_bytes: current_live,
            total_allocated_bytes: total_allocated,
            total_freed_bytes: total_freed,
            fragmentation_ratio,
            size_bucket_variety: size_variety,
            allocation_count: snapshot.total_live_allocations,
        }
    }

    /// Clear all profiling data
    pub fn clear(&self) {
        self.allocations.lock().clear();
        self.site_stats.lock().clear();
        self.next_id.store(1, Ordering::Relaxed);
        self.total_allocated.store(0, Ordering::Relaxed);
        self.total_freed.store(0, Ordering::Relaxed);

        // Reset size distribution
        let dist = self.size_distribution.lock();
        for counter in dist.values() {
            counter.store(0, Ordering::Relaxed);
        }
    }

    /// Get current timestamp
    fn now() -> u64 {
        // In real implementation, use high-resolution timer
        0
    }

    /// Get current thread ID
    fn current_thread_id(&self) -> u64 {
        // In real implementation, get actual thread ID
        0
    }

    /// Capture stack trace
    fn capture_stack_trace(&self) -> Vec<u64> {
        // In real implementation, unwind stack
        Vec::new()
    }
}

/// Fragmentation information
#[derive(Debug, Clone)]
pub struct FragmentationInfo {
    /// Current live bytes
    pub current_live_bytes: u64,
    /// Total allocated bytes (all time)
    pub total_allocated_bytes: u64,
    /// Total freed bytes (all time)
    pub total_freed_bytes: u64,
    /// Fragmentation ratio (0-1)
    pub fragmentation_ratio: f64,
    /// Number of size buckets in use
    pub size_bucket_variety: usize,
    /// Number of live allocations
    pub allocation_count: u64,
}

impl HeapSnapshot {
    /// Export in JSON format
    pub fn export_json(&self) -> alloc::string::String {
        let mut json = alloc::string::String::new();

        json.push_str("{\n");
        json.push_str(&format!("  \"timestamp\": {},\n", self.timestamp));
        json.push_str(&format!("  \"total_live_allocations\": {},\n", self.total_live_allocations));
        json.push_str(&format!("  \"total_live_bytes\": {},\n", self.total_live_bytes));
        json.push_str("  \"site_stats\": {\n");

        let mut sites: Vec<_> = self.site_stats.iter().collect();
        sites.sort_by(|a, b| b.1.get_live_bytes().cmp(&a.1.get_live_bytes()));

        for (i, (site, stats)) in sites.iter().enumerate() {
            json.push_str(&format!("    \"{}\": {{\n", site));
            json.push_str(&format!("      \"live_allocations\": {},\n", stats.get_live_allocations()));
            json.push_str(&format!("      \"live_bytes\": {},\n", stats.get_live_bytes()));
            json.push_str(&format!("      \"total_allocations\": {},\n", stats.total_allocations.load(Ordering::Relaxed)));
            json.push_str(&format!("      \"peak_bytes\": {}\n", stats.peak_bytes.load(Ordering::Relaxed)));
            json.push_str("    }");

            if i < sites.len() - 1 {
                json.push_str(",");
            }
            json.push_str("\n");
        }

        json.push_str("  }\n");
        json.push_str("}\n");

        json
    }

    /// Get top allocation sites by bytes
    pub fn top_sites_by_bytes(&self, n: usize) -> Vec<(AllocationSite, u64, u64)> {
        let mut sites: Vec<_> = self
            .site_stats
            .iter()
            .map(|(&site, stats)| {
                (
                    site,
                    stats.get_live_allocations(),
                    stats.get_live_bytes(),
                )
            })
            .collect();

        sites.sort_by(|a, b| b.2.cmp(&a.2));
        sites.truncate(n);
        sites
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = MemoryProfiler::new();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_profiler_start_stop() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();
        assert!(profiler.is_running());
        profiler.stop().unwrap();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_allocation_tracking() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        let site = 0x1000;
        let id = profiler.record_allocation(site, 1024, 8).unwrap();
        assert!(id > 0);

        let snapshot = profiler.take_snapshot().unwrap();
        assert_eq!(snapshot.total_live_allocations, 1);
        assert_eq!(snapshot.total_live_bytes, 1024);
    }

    #[test]
    fn test_deallocation_tracking() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        let site = 0x1000;
        let id = profiler.record_allocation(site, 1024, 8).unwrap();
        profiler.record_deallocation(id).unwrap();

        let snapshot = profiler.take_snapshot().unwrap();
        assert_eq!(snapshot.total_live_allocations, 0);
        assert_eq!(snapshot.total_live_bytes, 0);
    }

    #[test]
    fn test_site_statistics() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        let site = 0x1000;
        profiler.record_allocation(site, 1024, 8).unwrap();
        profiler.record_allocation(site, 2048, 8).unwrap();

        let snapshot = profiler.take_snapshot().unwrap();
        let site_stats = snapshot.site_stats.get(&site).unwrap();

        assert_eq!(site_stats.get_live_allocations(), 2);
        assert_eq!(site_stats.get_live_bytes(), 3072);
    }

    #[test]
    fn test_size_classification() {
        assert_eq!(SizeBucket::from_size(8), SizeBucket::Tiny);
        assert_eq!(SizeBucket::from_size(100), SizeBucket::Small);
        assert_eq!(SizeBucket::from_size(1000), SizeBucket::Medium);
        assert_eq!(SizeBucket::from_size(10000), SizeBucket::Large);
        assert_eq!(SizeBucket::from_size(200000), SizeBucket::Huge);
        assert_eq!(SizeBucket::from_size(2000000), SizeBucket::Gigantic);
    }

    #[test]
    fn test_size_distribution() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        profiler.record_allocation(0x1000, 8, 8).unwrap();   // Tiny
        profiler.record_allocation(0x2000, 100, 8).unwrap(); // Small
        profiler.record_allocation(0x3000, 1000, 8).unwrap(); // Medium

        let snapshot = profiler.take_snapshot().unwrap();

        assert_eq!(snapshot.size_distribution.get(&SizeBucket::Tiny), Some(&1));
        assert_eq!(snapshot.size_distribution.get(&SizeBucket::Small), Some(&1));
        assert_eq!(snapshot.size_distribution.get(&SizeBucket::Medium), Some(&1));
    }

    #[test]
    fn test_double_free_detection() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        let id = profiler.record_allocation(0x1000, 1024, 8).unwrap();
        profiler.record_deallocation(id).unwrap();

        let result = profiler.record_deallocation(id);
        assert!(matches!(result, Err(MemoryProfileError::CorruptionDetected)));
    }

    #[test]
    fn test_leak_detection() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        let site = 0x1000;
        // Allocate many times without freeing
        for _ in 0..100 {
            profiler.record_allocation(site, 1024, 8).unwrap();
        }

        let leaks = profiler.detect_leaks();
        // Should detect this as a potential leak
        assert!(!leaks.is_empty());
    }

    #[test]
    fn test_fragmentation_info() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        profiler.record_allocation(0x1000, 1024, 8).unwrap();
        profiler.record_allocation(0x2000, 2048, 8).unwrap();

        let info = profiler.fragmentation_info();
        assert_eq!(info.allocation_count, 2);
        assert_eq!(info.current_live_bytes, 3072);
    }

    #[test]
    fn test_top_sites() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        profiler.record_allocation(0x1000, 4096, 8).unwrap();
        profiler.record_allocation(0x2000, 1024, 8).unwrap();
        profiler.record_allocation(0x3000, 2048, 8).unwrap();

        let snapshot = profiler.take_snapshot().unwrap();
        let top = snapshot.top_sites_by_bytes(2);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, 0x1000); // Most bytes
        assert_eq!(top[0].2, 4096);
    }

    #[test]
    fn test_snapshot_export() {
        let profiler = MemoryProfiler::new();
        profiler.start().unwrap();

        profiler.record_allocation(0x1000, 1024, 8).unwrap();

        let snapshot = profiler.take_snapshot().unwrap();
        let json = snapshot.export_json();

        assert!(json.contains("total_live_allocations"));
        assert!(json.contains("total_live_bytes"));
    }
}
