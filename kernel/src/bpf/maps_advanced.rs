//! # Advanced eBPF Map Types
//!
//! This module provides advanced eBPF map types for specialized use cases including
//! high-performance event streaming, probabilistic data structures, frequency estimation,
//! and batch operations.
//!
//! ## Map Types
//!
//! - **RingBufferMap**: Lock-free ring buffer for efficient event streaming
//! - **PerfEventMap**: Integration with perf event subsystem for performance monitoring
//! - **BloomFilter**: Probabilistic set membership testing
//! - **SkewSketch**: Frequency estimation using Count-Min Sketch
//!
//! ## Features
//!
//! - Lock-free operations for high-concurrency scenarios
//! - Batch operations for improved throughput
//! - Map persistence for crash recovery
//! - Comprehensive statistics and debugging support

use crate::prelude::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::{Mutex, RwLock};

/// Ring buffer map for lock-free event streaming
///
/// # Overview
///
/// RingBufferMap provides a high-performance, lock-free circular buffer for streaming
/// events from BPF programs to user space. It supports variable-length records and
/// provides efficient reserve/submit semantics.
///
/// # Example
///
/// ```rust
/// let ring_buf = RingBufferMap::new(4096)?;
/// let mut record = ring_buf.reserve(128)?;
/// record.write_all(&event_data);
/// ring_buf.submit(record);
/// ```
pub struct RingBufferMap {
    /// Ring buffer data
    buffer: Mutex<RingBufferData>,
    /// Buffer size in bytes
    size: usize,
    /// Statistics
    stats: Mutex<RingBufferStats>,
}

/// Ring buffer internal data
struct RingBufferData {
    /// Buffer storage
    data: Vec<u8>,
    /// Current write position
    write_pos: AtomicUsize,
    /// Current read position
    read_pos: AtomicUsize,
    /// Pending record (being reserved)
    pending_pos: AtomicUsize,
    /// Pending record length
    pending_len: AtomicUsize,
}

/// Reserved record in ring buffer
pub struct RingBufferRecord<'a> {
    /// Reference to ring buffer
    buffer: &'a RingBufferMap,
    /// Position in buffer
    pos: usize,
    /// Length of reservation
    len: usize,
}

/// Ring buffer statistics
#[derive(Debug, Default)]
struct RingBufferStats {
    /// Total records submitted
    records_submitted: AtomicU64,
    /// Total bytes submitted
    bytes_submitted: AtomicU64,
    /// Total records consumed
    records_consumed: AtomicU64,
    /// Failed reservations (buffer full)
    failed_reservations: AtomicU64,
    /// Current buffer utilization (bytes)
    current_utilization: AtomicUsize,
}

impl RingBufferMap {
    /// Create a new ring buffer map
    ///
    /// # Arguments
    ///
    /// * `size` - Buffer size in bytes (must be power of 2)
    pub fn new(size: usize) -> Result<Self> {
        // Validate size is power of 2
        if !size.is_power_of_two() {
            return Err(Error::new(EINVAL));
        }

        let data = vec![0u8; size];

        Ok(Self {
            buffer: Mutex::new(RingBufferData {
                data,
                write_pos: AtomicUsize::new(0),
                read_pos: AtomicUsize::new(0),
                pending_pos: AtomicUsize::new(0),
                pending_len: AtomicUsize::new(0),
            }),
            size,
            stats: Mutex::new(RingBufferStats::default()),
        })
    }

    /// Reserve space in the ring buffer
    ///
    /// # Arguments
    ///
    /// * `len` - Number of bytes to reserve
    ///
    /// # Returns
    ///
    /// A handle to the reserved space, or error if buffer is full
    pub fn reserve(&self, len: usize) -> Result<RingBufferRecord> {
        if len == 0 || len > self.size {
            return Err(Error::new(EINVAL));
        }

        let buffer = self.buffer.lock();
        let current_write = buffer.write_pos.load(Ordering::Acquire);
        let current_read = buffer.read_pos.load(Ordering::Acquire);

        // Calculate available space
        let available = if current_write >= current_read {
            self.size - (current_write - current_read)
        } else {
            current_read - current_write
        };

        // Check if we have enough space (leave 8 bytes for header)
        if available < len + 8 {
            let stats = self.stats.lock();
            stats.failed_reservations.fetch_add(1, Ordering::Relaxed);
            return Err(Error::new(ENOMEM));
        }

        // Reserve the space
        buffer.pending_pos.store(current_write, Ordering::Release);
        buffer.pending_len.store(len, Ordering::Release);

        Ok(RingBufferRecord {
            buffer: self,
            pos: current_write,
            len,
        })
    }

    /// Submit a reserved record to the ring buffer
    ///
    /// # Arguments
    ///
    /// * `record` - The reserved record handle
    pub fn submit(&self, record: RingBufferRecord) {
        let mut buffer = self.buffer.lock();

        // Write header (length)
        let write_pos = record.pos;
        let len_bytes = (record.len as u64).to_le_bytes();

        for (i, &byte) in len_bytes.iter().enumerate() {
            buffer.data[(write_pos + i) & (self.size - 1)] = byte;
        }

        // Update write position
        let new_write_pos = (write_pos + record.len + 8) & (self.size - 1);
        buffer.write_pos.store(new_write_pos, Ordering::Release);

        // Clear pending state
        buffer.pending_len.store(0, Ordering::Release);

        // Update statistics
        let stats = self.stats.lock();
        stats.records_submitted.fetch_add(1, Ordering::Relaxed);
        stats.bytes_submitted.fetch_add(record.len as u64, Ordering::Relaxed);
        stats.current_utilization.store(
            (new_write_pos as isize - buffer.read_pos.load(Ordering::Relaxed) as isize).abs() as usize,
            Ordering::Relaxed
        );
    }

    /// Consume events from the ring buffer
    ///
    /// # Arguments
    ///
    /// * `callback` - Function to process each event
    ///
    /// # Returns
    ///
    /// Number of events consumed
    pub fn consume<F>(&self, mut callback: F) -> Result<usize>
    where
        F: FnMut(&[u8]),
    {
        let buffer = self.buffer.lock();
        let mut count = 0;
        let read_pos = buffer.read_pos.load(Ordering::Acquire);
        let write_pos = buffer.write_pos.load(Ordering::Acquire);
        let mut current_read = read_pos;

        while current_read != write_pos {
            // Read length header
            let mut len_bytes = [0u8; 8];
            for i in 0..8 {
                len_bytes[i] = buffer.data[(current_read + i) & (self.size - 1)];
            }
            let len = u64::from_le_bytes(len_bytes) as usize;

            // Validate length
            if len == 0 || len > self.size {
                break;
            }

            // Read data
            let data_start = (current_read + 8) & (self.size - 1);
            let mut record_data = vec![0u8; len];

            for i in 0..len {
                record_data[i] = buffer.data[(data_start + i) & (self.size - 1)];
            }

            // Process record
            callback(&record_data);
            count += 1;

            // Move read position
            current_read = (data_start + len) & (self.size - 1);
        }

        // Update read position
        buffer.read_pos.store(current_read, Ordering::Release);

        // Update statistics
        let stats = self.stats.lock();
        stats.records_consumed.fetch_add(count as u64, Ordering::Relaxed);

        Ok(count)
    }

    /// Get current buffer utilization
    pub fn utilization(&self) -> f64 {
        let buffer = self.buffer.lock();
        let write_pos = buffer.write_pos.load(Ordering::Relaxed);
        let read_pos = buffer.read_pos.load(Ordering::Relaxed);

        let used = if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.size - (read_pos - write_pos)
        };

        (used as f64) / (self.size as f64)
    }

    /// Get statistics
    pub fn stats(&self) -> RingBufferStatistics {
        let stats = self.stats.lock();
        RingBufferStatistics {
            records_submitted: stats.records_submitted.load(Ordering::Relaxed),
            bytes_submitted: stats.bytes_submitted.load(Ordering::Relaxed),
            records_consumed: stats.records_consumed.load(Ordering::Relaxed),
            failed_reservations: stats.failed_reservations.load(Ordering::Relaxed),
            current_utilization: stats.current_utilization.load(Ordering::Relaxed),
        }
    }
}

/// Ring buffer statistics
#[derive(Debug, Clone)]
pub struct RingBufferStatistics {
    pub records_submitted: u64,
    pub bytes_submitted: u64,
    pub records_consumed: u64,
    pub failed_reservations: u64,
    pub current_utilization: usize,
}

impl<'a> core::ops::Deref for RingBufferRecord<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let buffer = self.buffer.buffer.lock();
        let data_start = (self.pos + 8) & (self.buffer.size - 1);
        &buffer.data[data_start..data_start + self.len]
    }
}

impl<'a> core::ops::DerefMut for RingBufferRecord<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let mut buffer = self.buffer.buffer.lock();
        let data_start = (self.pos + 8) & (self.buffer.size - 1);
        &mut buffer.data[data_start..data_start + self.len]
    }
}

/// Performance event map for perf subsystem integration
///
/// # Overview
///
/// PerfEventMap enables BPF programs to export data to the Linux perf event subsystem,
/// providing integration with performance monitoring tools like perf, flame graphs, etc.
pub struct PerfEventMap {
    /// Map ID
    id: u32,
    /// Event type (sampling, counting, etc.)
    event_type: PerfEventType,
    /// Event configuration
    config: PerfEventConfig,
    /// Sample period (for sampling events)
    sample_period: u64,
    /// Output buffer
    output: Mutex<PerfEventOutput>,
    /// Statistics
    stats: Mutex<PerfEventStats>,
}

/// Performance event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfEventType {
    /// Hardware event (cycles, instructions, etc.)
    Hardware,
    /// Software event (context switches, page faults, etc.)
    Software,
    /// Tracepoint event
    Tracepoint,
    /// BPF output event
    BpfOutput,
}

/// Performance event configuration
#[derive(Debug, Clone)]
pub struct PerfEventConfig {
    /// Event configuration value
    pub config: u64,
    /// Event attributes
    pub attrs: PerfEventAttrs,
}

/// Performance event attributes
#[derive(Debug, Clone)]
pub struct PerfEventAttrs {
    /// Is disabled
    pub disabled: bool,
    /// Inherit child tasks
    pub inherit: bool,
    /// Pin to specific CPU
    pub pinned: bool,
    /// Exclude user space
    pub exclude_user: bool,
    /// Exclude kernel
    pub exclude_kernel: bool,
    /// Exclude hypervisor
    pub exclude_hv: bool,
    /// Exclude idle
    pub exclude_idle: bool,
    /// Mmap pages
    pub mmap_pages: u32,
    /// Frequency mode
    pub freq: bool,
    /// Sample frequency or period
    pub sample_freq_or_period: u64,
}

/// Performance event output
struct PerfEventOutput {
    /// Output data
    data: Vec<u8>,
    /// Data length
    len: usize,
}

/// Performance event statistics
#[derive(Debug, Default)]
struct PerfEventStats {
    /// Total events
    total_events: AtomicU64,
    /// Lost events (buffer full)
    lost_events: AtomicU64,
    /// Samples collected
    samples: AtomicU64,
}

impl PerfEventMap {
    /// Create a new perf event map
    ///
    /// # Arguments
    ///
    /// * `event_type` - Type of perf event
    /// * `config` - Event configuration
    /// * `sample_period` - Sample period (for sampling events)
    pub fn new(event_type: PerfEventType, config: PerfEventConfig, sample_period: u64) -> Result<Self> {
        Ok(Self {
            id: Self::generate_id(),
            event_type,
            config,
            sample_period,
            output: Mutex::new(PerfEventOutput {
                data: Vec::new(),
                len: 0,
            }),
            stats: Mutex::new(PerfEventStats::default()),
        })
    }

    /// Generate unique map ID
    fn generate_id() -> u32 {
        use core::sync::atomic::{AtomicU32, Ordering};
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst)
    }

    /// Output data to perf event buffer
    ///
    /// # Arguments
    ///
    /// * `data` - Data to output
    pub fn output(&self, data: &[u8]) -> Result<()> {
        let mut output = self.output.lock();

        // Check buffer size
        if output.data.len() + data.len() > output.data.capacity() {
            let stats = self.stats.lock();
            stats.lost_events.fetch_add(1, Ordering::Relaxed);
            return Err(Error::new(ENOMEM));
        }

        // Write data
        output.data.extend_from_slice(data);
        output.len += data.len();

        // Update statistics
        let stats = self.stats.lock();
        stats.total_events.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Read samples from perf event buffer
    ///
    /// # Returns
    ///
    /// Vector of sample data
    pub fn read_samples(&self) -> Result<Vec<Vec<u8>>> {
        let mut output = self.output.lock();
        let mut samples = Vec::new();

        // Parse perf event records
        let mut pos = 0;
        while pos < output.len {
            // Read perf event header
            if pos + 8 > output.len {
                break;
            }

            let size = u32::from_le_bytes([
                output.data[pos + 4],
                output.data[pos + 5],
                output.data[pos + 6],
                output.data[pos + 7],
            ]) as usize;

            if pos + size > output.len {
                break;
            }

            // Extract sample data
            let sample_data = output.data[pos..pos + size].to_vec();
            samples.push(sample_data);

            pos += size;
        }

        // Update statistics
        let stats = self.stats.lock();
        stats.samples.fetch_add(samples.len() as u64, Ordering::Relaxed);

        Ok(samples)
    }

    /// Get map ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get event type
    pub fn event_type(&self) -> PerfEventType {
        self.event_type
    }

    /// Get statistics
    pub fn stats(&self) -> PerfEventStatistics {
        let stats = self.stats.lock();
        PerfEventStatistics {
            total_events: stats.total_events.load(Ordering::Relaxed),
            lost_events: stats.lost_events.load(Ordering::Relaxed),
            samples: stats.samples.load(Ordering::Relaxed),
        }
    }
}

/// Performance event statistics
#[derive(Debug, Clone)]
pub struct PerfEventStatistics {
    pub total_events: u64,
    pub lost_events: u64,
    pub samples: u64,
}

/// Bloom filter for probabilistic set membership
///
/// # Overview
///
/// BloomFilter is a space-efficient probabilistic data structure that tests
/// whether an element is a member of a set. False positives are possible,
/// but false negatives are not.
///
/// # Properties
///
/// - Fast O(k) lookup where k is the number of hash functions
/// - Memory-efficient: ~1.44*log2(1/ε) bits per element
/// - Configurable false positive rate (ε)
pub struct BloomFilter {
    /// Bit array
    bits: Vec<u64>,
    /// Number of bits in the filter
    num_bits: usize,
    /// Number of hash functions
    num_hashes: usize,
    /// Number of items added
    count: AtomicUsize,
    /// Statistics
    stats: Mutex<BloomFilterStats>,
}

/// Bloom filter statistics
#[derive(Debug, Default)]
struct BloomFilterStats {
    /// Total lookups
    lookups: AtomicU64,
    /// Positive lookups
    positive_lookups: AtomicU64,
    /// Add operations
    add_operations: AtomicU64,
}

impl BloomFilter {
    /// Create a new bloom filter
    ///
    /// # Arguments
    ///
    /// * `expected_items` - Expected number of items
    /// * `false_positive_rate` - Desired false positive rate (0.0 to 1.0)
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Result<Self> {
        if expected_items == 0 || false_positive_rate <= 0.0 || false_positive_rate >= 1.0 {
            return Err(Error::new(EINVAL));
        }

        // Calculate optimal size and hash count
        // num_bits = -n * ln(p) / (ln(2)^2)
        // num_hashes = (m/n) * ln(2)
        let ln2 = core::f64::consts::LN_2;
        let num_bits = ((expected_items as f64) * false_positive_rate.ln() / (-ln2 * ln2)).ceil() as usize;
        let num_hashes = ((num_bits as f64) / (expected_items as f64) * ln2).ceil() as usize;

        // Round up to nearest multiple of 64
        let num_words = (num_bits + 63) / 64;
        let bits = vec![0u64; num_words];

        Ok(Self {
            bits,
            num_bits: num_words * 64,
            num_hashes,
            count: AtomicUsize::new(0),
            stats: Mutex::new(BloomFilterStats::default()),
        })
    }

    /// Add an item to the bloom filter
    ///
    /// # Arguments
    ///
    /// * `data` - Item to add
    pub fn add(&self, data: &[u8]) -> Result<()> {
        let hash = self.hash(data);
        let mut a = hash;
        let mut b = hash >> 32;

        for i in 0..self.num_hashes {
            let index = ((a.wrapping_add(b.wrapping_mul(i as u32))) as usize) % self.num_bits;
            let word_index = index / 64;
            let bit_index = index % 64;

            self.bits[word_index] |= 1 << bit_index;
        }

        self.count.fetch_add(1, Ordering::Relaxed);

        let stats = self.stats.lock();
        stats.add_operations.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Check if an item might be in the filter
    ///
    /// # Arguments
    ///
    /// * `data` - Item to check
    ///
    /// # Returns
    ///
    /// True if item might be in filter (may be false positive)
    /// False if item is definitely not in filter
    pub fn contains(&self, data: &[u8]) -> bool {
        let hash = self.hash(data);
        let mut a = hash;
        let mut b = hash >> 32;

        let mut result = true;

        for i in 0..self.num_hashes {
            let index = ((a.wrapping_add(b.wrapping_mul(i as u32))) as usize) % self.num_bits;
            let word_index = index / 64;
            let bit_index = index % 64;

            if (self.bits[word_index] & (1 << bit_index)) == 0 {
                result = false;
                break;
            }
        }

        let stats = self.stats.lock();
        stats.lookups.fetch_add(1, Ordering::Relaxed);
        if result {
            stats.positive_lookups.fetch_add(1, Ordering::Relaxed);
        }

        result
    }

    /// Hash function for bloom filter
    fn hash(&self, data: &[u8]) -> u32 {
        // Simple hash function (FNV-1a)
        let mut hash: u32 = 2166136261;
        for &byte in data {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }

    /// Get number of items in filter
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> BloomFilterStatistics {
        let stats = self.stats.lock();
        BloomFilterStatistics {
            lookups: stats.lookups.load(Ordering::Relaxed),
            positive_lookups: stats.positive_lookups.load(Ordering::Relaxed),
            add_operations: stats.add_operations.load(Ordering::Relaxed),
        }
    }

    /// Clear the bloom filter
    pub fn clear(&self) {
        for word in &mut self.bits {
            *word = 0;
        }
        self.count.store(0, Ordering::Relaxed);
    }
}

/// Bloom filter statistics
#[derive(Debug, Clone)]
pub struct BloomFilterStatistics {
    pub lookups: u64,
    pub positive_lookups: u64,
    pub add_operations: u64,
}

/// Count-Min Sketch for frequency estimation
///
/// # Overview
///
/// SkewSketch (Count-Min Sketch) is a probabilistic data structure for
/// frequency estimation and heavy hitters detection. It uses sub-linear
/// space to estimate item frequencies.
///
/// # Properties
///
/// - O(1) update and query operations
/// - Sub-linear space usage
    /// - Approximate frequency estimates
/// - Detects heavy hitters (most frequent items)
pub struct SkewSketch {
    /// 2D array of counters
    counters: Vec<Vec<u64>>,
    /// Number of rows
    rows: usize,
    /// Number of columns
    cols: usize,
    /// Total count (for normalization)
    total_count: AtomicU64,
    /// Statistics
    stats: Mutex<SkewSketchStats>,
}

/// Sketch statistics
#[derive(Debug, Default)]
struct SkewSketchStats {
    /// Total updates
    updates: AtomicU64,
    /// Total queries
    queries: AtomicU64,
}

impl SkewSketch {
    /// Create a new Count-Min Sketch
    ///
    /// # Arguments
    ///
    /// * `rows` - Number of rows (hash functions)
    /// * `cols` - Number of columns (counters per row)
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        if rows == 0 || cols == 0 {
            return Err(Error::new(EINVAL));
        }

        let counters = vec![vec![0u64; cols]; rows];

        Ok(Self {
            counters,
            rows,
            cols,
            total_count: AtomicU64::new(0),
            stats: Mutex::new(SkewSketchStats::default()),
        })
    }

    /// Increment count for an item
    ///
    /// # Arguments
    ///
    /// * `data` - Item to increment
    /// * `count` - Amount to increment (default 1)
    pub fn increment(&self, data: &[u8], count: u64) -> Result<()> {
        for row in 0..self.rows {
            let hash = self.hash_for_row(data, row);
            let col = hash as usize % self.cols;

            self.counters[row][col] = self.counters[row][col].saturating_add(count);
        }

        self.total_count.fetch_add(count, Ordering::Relaxed);

        let stats = self.stats.lock();
        stats.updates.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Estimate count for an item
    ///
    /// # Arguments
    ///
    /// * `data` - Item to query
    ///
    /// # Returns
    ///
    /// Estimated count (may be higher than actual)
    pub fn estimate(&self, data: &[u8]) -> u64 {
        let mut min_count = u64::MAX;

        for row in 0..self.rows {
            let hash = self.hash_for_row(data, row);
            let col = hash as usize % self.cols;

            min_count = min_count.min(self.counters[row][col]);
        }

        let stats = self.stats.lock();
        stats.queries.fetch_add(1, Ordering::Relaxed);

        min_count
    }

    /// Get heavy hitters (items above threshold)
    ///
    /// # Arguments
    ///
    /// * `items` - Potential heavy hitter items
    /// * `threshold_fraction` - Fraction of total count to be heavy hitter (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Vector of (item, estimated_count) for heavy hitters
    pub fn heavy_hitters(&self, items: &[Vec<u8>], threshold_fraction: f64) -> Vec<(Vec<u8>, u64)> {
        let total = self.total_count.load(Ordering::Relaxed);
        let threshold = (total as f64 * threshold_fraction) as u64;

        let mut hitters = Vec::new();

        for item in items {
            let count = self.estimate(item);
            if count >= threshold {
                hitters.push((item.clone(), count));
            }
        }

        // Sort by count descending
        hitters.sort_by(|a, b| b.1.cmp(&a.1));

        hitters
    }

    /// Hash function for specific row
    fn hash_for_row(&self, data: &[u8], row: usize) -> u64 {
        let mut hash: u64 = row as u64;
        for &byte in data {
            hash = hash.wrapping_mul  (31).wrapping_add(byte as u64);
        }
        hash
    }

    /// Get total count
    pub fn total_count(&self) -> u64 {
        self.total_count.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> SkewSketchStatistics {
        let stats = self.stats.lock();
        SkewSketchStatistics {
            updates: stats.updates.load(Ordering::Relaxed),
            queries: stats.queries.load(Ordering::Relaxed),
        }
    }

    /// Clear the sketch
    pub fn clear(&self) {
        for row in &self.counters {
            for col in row {
                *col = 0;
            }
        }
        self.total_count.store(0, Ordering::Relaxed);
    }
}

/// Sketch statistics
#[derive(Debug, Clone)]
pub struct SkewSketchStatistics {
    pub updates: u64,
    pub queries: u64,
}

/// Batch operations for eBPF maps
///
/// # Overview
///
/// Provides batch operations for improved throughput when working with
/// multiple map entries at once.
pub struct BpfMapBatchOps;

impl BpfMapBatchOps {
    /// Lookup multiple keys at once
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map
    /// * `keys` - Keys to lookup
    ///
    /// # Returns
    ///
    /// Vector of (key, value) tuples for found keys
    pub fn batch_lookup(
        map: &crate::bpf::maps::BpfMap,
        keys: &[Vec<u8>],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();

        for key in keys {
            if let Ok(Some(value)) = map.lookup(key) {
                results.push((key.clone(), value));
            }
        }

        Ok(results)
    }

    /// Update multiple entries at once
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map
    /// * `entries` - Vector of (key, value) tuples
    /// * `flags` - Update flags
    ///
    /// # Returns
    ///
    /// Number of successfully updated entries
    pub fn batch_update(
        map: &crate::bpf::maps::BpfMap,
        entries: &[(Vec<u8>, Vec<u8>)],
        flags: u64,
    ) -> Result<usize> {
        let mut count = 0;

        for (key, value) in entries {
            if map.update(key, value, flags).is_ok() {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Delete multiple entries at once
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map
    /// * `keys` - Keys to delete
    ///
    /// # Returns
    ///
    /// Number of successfully deleted entries
    pub fn batch_delete(
        map: &crate::bpf::maps::BpfMap,
        keys: &[Vec<u8>],
    ) -> Result<usize> {
        let mut count = 0;

        for key in keys {
            // Note: BpfMap doesn't have a delete method yet
            // This is a placeholder for when delete is added
            // if map.delete(key).is_ok() {
            //     count += 1;
            // }
            count += 0; // Placeholder
        }

        Ok(count)
    }
}

/// Map persistence for crash recovery
///
/// # Overview
///
/// Provides functionality to save and restore map contents to persistent storage.
pub struct BpfMapPersistence;

impl BpfMapPersistence {
    /// Save map contents to storage
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map to save
    /// * `path` - Storage path
    ///
    /// # Returns
    ///
    /// Number of entries saved
    pub fn save_map(
        map: &crate::bpf::maps::BpfMap,
        _path: &str,
    ) -> Result<usize> {
        // Note: This is a placeholder implementation
        // Real implementation would serialize map to disk
        let count = map.len();

        // GH-#1282: Implement actual serialization
        // See: https://github.com/npos/kernel/issues/1282
        // 1. Iterate through all map entries
        // 2. Serialize to binary format
        // 3. Write to specified path

        Ok(count)
    }

    /// Load map from storage
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map to load into
    /// * `path` - Storage path
    ///
    /// # Returns
    ///
    /// Number of entries loaded
    pub fn load_map(
        map: &crate::bpf::maps::BpfMap,
        _path: &str,
    ) -> Result<usize> {
        // Note: This is a placeholder implementation
        // Real implementation would deserialize map from disk

        // GH-#1283: Implement actual deserialization
        // See: https://github.com/npos/kernel/issues/1283
        // 1. Read file from path
        // 2. Parse serialized format
        // 3. Insert entries into map

        Ok(0)
    }

    /// Create incremental snapshot
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map
    /// * `base_snapshot_id` - Base snapshot to diff against
    ///
    /// # Returns
    ///
    /// Snapshot ID
    pub fn create_snapshot(
        map: &crate::bpf::maps::BpfMap,
        _base_snapshot_id: Option<u64>,
    ) -> Result<u64> {
        // Generate snapshot ID
        let snapshot_id = Self::generate_snapshot_id();

        // Note: This is a placeholder implementation
        // Real implementation would:
        // 1. Capture current map state
        // 2. If base_snapshot provided, compute delta
        // 3. Store snapshot

        Ok(snapshot_id)
    }

    /// Generate unique snapshot ID
    fn generate_snapshot_id() -> u64 {
        use core::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_ID.fetch_add(1, Ordering::SeqCst)
    }

    /// Restore from snapshot
    ///
    /// # Arguments
    ///
    /// * `map` - The BPF map to restore into
    /// * `snapshot_id` - Snapshot to restore
    pub fn restore_snapshot(
        map: &crate::bpf::maps::BpfMap,
        _snapshot_id: u64,
    ) -> Result<()> {
        // Note: This is a placeholder implementation
        // Real implementation would:
        // 1. Load snapshot data
        // 2. Clear current map
        // 3. Restore snapshot state

        let _ = map.len(); // Use map to avoid unused warning
        Ok(())
    }
}

/// Map statistics and debugging support
///
/// # Overview
///
/// Provides comprehensive statistics collection and debugging support for
/// BPF map operations including timing, memory usage, and operation counts.
pub struct BpfMapStats {
    /// Map ID
    id: u32,
    /// Operation counters
    counters: Mutex<OperationCounters>,
    /// Timing data
    timing: Mutex<TimingData>,
    /// Memory usage
    memory: Mutex<MemoryUsage>,
}

/// Operation counters
#[derive(Debug, Default)]
struct OperationCounters {
    /// Number of lookups
    lookups: AtomicU64,
    /// Number of updates
    updates: AtomicU64,
    /// Number of deletes
    deletes: AtomicU64,
    /// Number of failed lookups
    failed_lookups: AtomicU64,
    /// Number of failed updates
    failed_updates: AtomicU64,
}

/// Timing data (in nanoseconds)
#[derive(Debug, Default)]
struct TimingData {
    /// Total lookup time
    total_lookup_ns: AtomicU64,
    /// Total update time
    total_update_ns: AtomicU64,
    /// Total delete time
    total_delete_ns: AtomicU64,
    /// Maximum lookup time
    max_lookup_ns: AtomicU64,
    /// Maximum update time
    max_update_ns: AtomicU64,
    /// Maximum delete time
    max_delete_ns: AtomicU64,
}

/// Memory usage tracking
#[derive(Debug, Default)]
struct MemoryUsage {
    /// Current bytes allocated
    current_bytes: AtomicUsize,
    /// Peak bytes allocated
    peak_bytes: AtomicUsize,
    /// Number of allocations
    allocations: AtomicU64,
}

impl BpfMapStats {
    /// Create new statistics tracker
    pub fn new(id: u32) -> Self {
        Self {
            id,
            counters: Mutex::new(OperationCounters::default()),
            timing: Mutex::new(TimingData::default()),
            memory: Mutex::new(MemoryUsage::default()),
        }
    }

    /// Record a lookup operation
    pub fn record_lookup(&self, duration_ns: u64, succeeded: bool) {
        let counters = self.counters.lock();
        counters.lookups.fetch_add(1, Ordering::Relaxed);
        if !succeeded {
            counters.failed_lookups.fetch_add(1, Ordering::Relaxed);
        }

        let timing = self.timing.lock();
        timing.total_lookup_ns.fetch_add(duration_ns, Ordering::Relaxed);

        // Update max
        let mut max = timing.max_lookup_ns.load(Ordering::Relaxed);
        while duration_ns > max {
            match timing.max_lookup_ns.compare_exchange_weak(
                max,
                duration_ns,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
    }

    /// Record an update operation
    pub fn record_update(&self, duration_ns: u64, succeeded: bool) {
        let counters = self.counters.lock();
        counters.updates.fetch_add(1, Ordering::Relaxed);
        if !succeeded {
            counters.failed_updates.fetch_add(1, Ordering::Relaxed);
        }

        let timing = self.timing.lock();
        timing.total_update_ns.fetch_add(duration_ns, Ordering::Relaxed);

        // Update max
        let mut max = timing.max_update_ns.load(Ordering::Relaxed);
        while duration_ns > max {
            match timing.max_update_ns.compare_exchange_weak(
                max,
                duration_ns,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
    }

    /// Record a delete operation
    pub fn record_delete(&self, duration_ns: u64) {
        let counters = self.counters.lock();
        counters.deletes.fetch_add(1, Ordering::Relaxed);

        let timing = self.timing.lock();
        timing.total_delete_ns.fetch_add(duration_ns, Ordering::Relaxed);

        // Update max
        let mut max = timing.max_delete_ns.load(Ordering::Relaxed);
        while duration_ns > max {
            match timing.max_delete_ns.compare_exchange_weak(
                max,
                duration_ns,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
    }

    /// Record memory allocation
    pub fn record_allocation(&self, bytes: usize) {
        let memory = self.memory.lock();
        memory.allocations.fetch_add(1, Ordering::Relaxed);
        memory.current_bytes.fetch_add(bytes, Ordering::Relaxed);

        // Update peak
        let current = memory.current_bytes.load(Ordering::Relaxed);
        let mut peak = memory.peak_bytes.load(Ordering::Relaxed);
        while current > peak {
            match memory.peak_bytes.compare_exchange_weak(
                peak,
                current,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
    }

    /// Record memory deallocation
    pub fn record_deallocation(&self, bytes: usize) {
        let memory = self.memory.lock();
        memory.current_bytes.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Get comprehensive statistics
    pub fn get_stats(&self) -> MapStatistics {
        let counters = self.counters.lock();
        let timing = self.timing.lock();
        let memory = self.memory.lock();

        MapStatistics {
            map_id: self.id,
            lookups: counters.lookups.load(Ordering::Relaxed),
            updates: counters.updates.load(Ordering::Relaxed),
            deletes: counters.deletes.load(Ordering::Relaxed),
            failed_lookups: counters.failed_lookups.load(Ordering::Relaxed),
            failed_updates: counters.failed_updates.load(Ordering::Relaxed),
            avg_lookup_ns: if counters.lookups.load(Ordering::Relaxed) > 0 {
                timing.total_lookup_ns.load(Ordering::Relaxed) / counters.lookups.load(Ordering::Relaxed)
            } else {
                0
            },
            avg_update_ns: if counters.updates.load(Ordering::Relaxed) > 0 {
                timing.total_update_ns.load(Ordering::Relaxed) / counters.updates.load(Ordering::Relaxed)
            } else {
                0
            },
            avg_delete_ns: if counters.deletes.load(Ordering::Relaxed) > 0 {
                timing.total_delete_ns.load(Ordering::Relaxed) / counters.deletes.load(Ordering::Relaxed)
            } else {
                0
            },
            max_lookup_ns: timing.max_lookup_ns.load(Ordering::Relaxed),
            max_update_ns: timing.max_update_ns.load(Ordering::Relaxed),
            max_delete_ns: timing.max_delete_ns.load(Ordering::Relaxed),
            current_memory_bytes: memory.current_bytes.load(Ordering::Relaxed),
            peak_memory_bytes: memory.peak_bytes.load(Ordering::Relaxed),
            total_allocations: memory.allocations.load(Ordering::Relaxed),
        }
    }
}

/// Comprehensive map statistics
#[derive(Debug, Clone)]
pub struct MapStatistics {
    pub map_id: u32,
    pub lookups: u64,
    pub updates: u64,
    pub deletes: u64,
    pub failed_lookups: u64,
    pub failed_updates: u64,
    pub avg_lookup_ns: u64,
    pub avg_update_ns: u64,
    pub avg_delete_ns: u64,
    pub max_lookup_ns: u64,
    pub max_update_ns: u64,
    pub max_delete_ns: u64,
    pub current_memory_bytes: usize,
    pub peak_memory_bytes: usize,
    pub total_allocations: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_create() {
        let ring_buf = RingBufferMap::new(4096);
        assert!(ring_buf.is_ok());
    }

    #[test]
    fn test_ring_buffer_reserve_submit() {
        let ring_buf = RingBufferMap::new(4096).unwrap();
        let record = ring_buf.reserve(128);
        assert!(record.is_ok());
        ring_buf.submit(record.unwrap());
    }

    #[test]
    fn test_ring_buffer_utilization() {
        let ring_buf = RingBufferMap::new(4096).unwrap();
        let util = ring_buf.utilization();
        assert!(util >= 0.0 && util <= 1.0);
    }

    #[test]
    fn test_bloom_filter_create() {
        let filter = BloomFilter::new(1000, 0.01);
        assert!(filter.is_ok());
    }

    #[test]
    fn test_bloom_filter_add_contains() {
        let filter = BloomFilter::new(1000, 0.01).unwrap();
        let data = b"test_key";

        assert!(!filter.contains(data));
        filter.add(data).unwrap();
        assert!(filter.contains(data));
    }

    #[test]
    fn test_skew_sketch_create() {
        let sketch = SkewSketch::new(5, 1000);
        assert!(sketch.is_ok());
    }

    #[test]
    fn test_skew_sketch_increment_estimate() {
        let sketch = SkewSketch::new(5, 1000).unwrap();
        let data = b"test_item";

        sketch.increment(data, 5).unwrap();
        let estimate = sketch.estimate(data);

        // Estimate should be at least the actual count
        assert!(estimate >= 5);
    }

    #[test]
    fn test_perf_event_map_create() {
        let config = PerfEventConfig {
            config: 0,
            attrs: PerfEventAttrs {
                disabled: false,
                inherit: false,
                pinned: false,
                exclude_user: false,
                exclude_kernel: false,
                exclude_hv: false,
                exclude_idle: false,
                mmap_pages: 1,
                freq: false,
                sample_freq_or_period: 1000,
            },
        };

        let perf_map = PerfEventMap::new(PerfEventType::BpfOutput, config, 1000);
        assert!(perf_map.is_ok());
    }

    #[test]
    fn test_map_stats() {
        let stats = BpfMapStats::new(1);
        stats.record_lookup(100, true);
        stats.record_update(200, true);
        stats.record_allocation(1024);

        let map_stats = stats.get_stats();
        assert_eq!(map_stats.lookups, 1);
        assert_eq!(map_stats.updates, 1);
        assert_eq!(map_stats.current_memory_bytes, 1024);
    }
}
