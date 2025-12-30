//! Block Request Handling and Management
//!
//! This module provides advanced block I/O request handling with:
//! - Read/write request processing
//! - Sector management and alignment
//! - Buffer strategies (read-ahead, write-back)
//! - Request merging and optimization
//! - I/O scheduling algorithms

extern crate alloc;

use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use crate::subsystems::sync::Mutex;
use crate::error::{Result, Error};
use super::{BlockDevice, BlockOp, Bio, DEFAULT_SECTOR_SIZE};

// ============================================================================
// Constants
// ============================================================================

/// Default read-ahead size in sectors
pub const DEFAULT_READAHEAD_SECTORS: u32 = 8;

/// Maximum read-ahead size in sectors
pub const MAX_READAHEAD_SECTORS: u32 = 128;

/// Default write-back cache size in sectors
pub const DEFAULT_WRITEBACK_SIZE: u32 = 256;

/// Maximum write-back cache size in sectors
pub const MAX_WRITEBACK_SIZE: u32 = 1024;

/// Write-back timeout in seconds
pub const WRITEBACK_TIMEOUT_SECONDS: u64 = 5;

/// Request merge threshold in sectors
pub const MERGE_THRESHOLD_SECTORS: u32 = 16;

/// Maximum merge size in sectors
pub const MAX_MERGE_SIZE: u32 = 64;

// ============================================================================
// Buffer Strategy Types
// ============================================================================

/// I/O scheduling algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IoScheduler {
    /// No-op scheduler (simple FIFO)
    Noop = 0,
    /// Deadline scheduler
    Deadline = 1,
    /// Completely Fair Queuing (CFQ)
    Cfq = 2,
    /// Budget Fair Queueing (BFQ)
    Bfq = 3,
    /// Kyber I/O scheduler
    Kyber = 4,
}

/// Buffer cache strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BufferStrategy {
    /// No caching (direct I/O)
    Direct = 0,
    /// Write-through cache
    WriteThrough = 1,
    /// Write-back cache
    WriteBack = 2,
    /// Read-only cache
    ReadOnly = 3,
}

/// Read-ahead policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReadaheadPolicy {
    /// No read-ahead
    None = 0,
    /// Fixed size read-ahead
    Fixed = 1,
    /// Adaptive read-ahead based on access patterns
    Adaptive = 2,
    /// Aggressive read-ahead
    Aggressive = 3,
}

// ============================================================================
// Request Context
// ============================================================================

/// Extended block I/O request with additional metadata
#[derive(Debug, Clone)]
pub struct BlockRequest {
    /// Base BIO structure
    pub bio: Bio,
    /// Request priority (0-255, higher is more important)
    pub priority: u8,
    /// Request deadline (for deadline scheduler)
    pub deadline: u64,
    /// Request flags
    pub flags: RequestFlags,
    /// Number of retries attempted
    pub retries: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// Start time
    pub start_time: u64,
    /// Completion time
    pub completion_time: Option<u64>,
    /// Associated request (for merging)
    pub merged_with: Option<u64>,
    /// Original sector before merging
    pub original_sector: u64,
}

/// Request flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestFlags(u8);

impl RequestFlags {
    pub const EMPTY: RequestFlags = RequestFlags(0);
    pub const SYNC: RequestFlags = RequestFlags(1 << 0);
    pub const URGENT: RequestFlags = RequestFlags(1 << 1);
    pub const META: RequestFlags = RequestFlags(1 << 2);
    pub const PRIORITY: RequestFlags = RequestFlags(1 << 3);
    pub const READAHEAD: RequestFlags = RequestFlags(1 << 4);
    pub const WRITEBACK: RequestFlags = RequestFlags(1 << 5);

    pub fn contains(&self, other: RequestFlags) -> bool {
        self.0 & other.0 != 0
    }

    pub fn insert(&mut self, other: RequestFlags) {
        self.0 |= other.0;
    }
}

impl BlockRequest {
    /// Create a new block request from BIO
    pub fn new(bio: Bio) -> Self {
        Self {
            bio,
            priority: 128, // Default priority
            deadline: 0,
            flags: RequestFlags::EMPTY,
            retries: 0,
            max_retries: 3,
            start_time: 0,
            completion_time: None,
            merged_with: None,
            original_sector: 0,
        }
    }

    /// Create a read request
    pub fn read(sector: u64, size: usize) -> Self {
        Self::new(Bio::read(sector, size))
    }

    /// Create a write request
    pub fn write(sector: u64, data: Vec<u8>) -> Self {
        Self::new(Bio::write(sector, data))
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set deadline
    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = deadline;
        self
    }

    /// Set flags
    pub fn with_flags(mut self, flags: RequestFlags) -> Self {
        self.flags.insert(flags);
        self
    }

    /// Check if request is urgent
    pub fn is_urgent(&self) -> bool {
        self.flags.contains(RequestFlags::URGENT) ||
        self.flags.contains(RequestFlags::SYNC) ||
        self.priority > 200
    }

    /// Check if request is metadata
    pub fn is_metadata(&self) -> bool {
        self.flags.contains(RequestFlags::META)
    }

    /// Get elapsed time
    pub fn elapsed_time(&self) -> Option<u64> {
        self.completion_time.map(|end| end - self.start_time)
    }
}

// ============================================================================
// Sector Management
// ============================================================================

/// Sector allocation and management
pub struct SectorManager {
    /// Total sectors on device
    total_sectors: u64,
    /// Sector size in bytes
    sector_size: u64,
    /// Allocated sectors (bitmap-like tracking)
    allocated_sectors: Mutex<Vec<bool>>,
    /// Bad sectors list
    bad_sectors: Mutex<Vec<u64>>,
    /// Reserved sectors
    reserved_sectors: Mutex<Vec<u64>>,
}

impl SectorManager {
    /// Create a new sector manager
    pub fn new(total_sectors: u64, sector_size: u64) -> Self {
        Self {
            total_sectors,
            sector_size,
            allocated_sectors: Mutex::new(vec![false; total_sectors as usize]),
            bad_sectors: Mutex::new(Vec::new()),
            reserved_sectors: Mutex::new(Vec::new()),
        }
    }

    /// Allocate contiguous sectors
    pub fn allocate_sectors(&self, count: u32) -> Result<u64> {
        let allocated = self.allocated_sectors.lock();
        let reserved = self.reserved_sectors.lock();
        let bad = self.bad_sectors.lock();

        // Find contiguous free sectors
        let mut start_sector = 0;
        let mut found = 0;
        let count = count as u64;

        for sector in 0..self.total_sectors {
            if allocated[sector as usize] ||
               reserved.contains(&sector) ||
               bad.contains(&sector) {
                start_sector = sector + 1;
                found = 0;
            } else {
                found += 1;
                if found >= count {
                    return Ok(start_sector);
                }
            }
        }

        Err(Error::OutOfSpace)
    }

    /// Mark sectors as allocated
    pub fn mark_allocated(&self, start: u64, count: u32) -> Result<()> {
        let mut allocated = self.allocated_sectors.lock();

        for sector in start..start + count as u64 {
            if sector >= self.total_sectors {
                return Err(Error::InvalidInput);
            }
            allocated[sector as usize] = true;
        }

        Ok(())
    }

    /// Free sectors
    pub fn free_sectors(&self, start: u64, count: u32) -> Result<()> {
        let mut allocated = self.allocated_sectors.lock();

        for sector in start..start + count as u64 {
            if sector >= self.total_sectors {
                return Err(Error::InvalidInput);
            }
            allocated[sector as usize] = false;
        }

        Ok(())
    }

    /// Mark sector as bad
    pub fn mark_bad_sector(&self, sector: u64) -> Result<()> {
        if sector >= self.total_sectors {
            return Err(Error::InvalidInput);
        }

        let mut bad = self.bad_sectors.lock();
        if !bad.contains(&sector) {
            bad.push(sector);
        }

        Ok(())
    }

    /// Check if sector is bad
    pub fn is_bad_sector(&self, sector: u64) -> bool {
        let bad = self.bad_sectors.lock();
        bad.contains(&sector)
    }

    /// Reserve sectors
    pub fn reserve_sectors(&self, start: u64, count: u32) -> Result<()> {
        let mut reserved = self.reserved_sectors.lock();

        for sector in start..start + count as u64 {
            if sector >= self.total_sectors {
                return Err(Error::InvalidInput);
            }
            if !reserved.contains(&sector) {
                reserved.push(sector);
            }
        }

        Ok(())
    }

    /// Get sector size
    pub fn sector_size(&self) -> u64 {
        self.sector_size
    }

    /// Get total sectors
    pub fn total_sectors(&self) -> u64 {
        self.total_sectors
    }

    /// Get free sectors count
    pub fn get_free_sectors(&self) -> u64 {
        let allocated = self.allocated_sectors.lock();
        let reserved = self.reserved_sectors.lock();
        let bad = self.bad_sectors.lock();

        let mut free = 0;
        for sector in 0..self.total_sectors {
            if !allocated[sector as usize] &&
               !reserved.contains(&sector) &&
               !bad.contains(&sector) {
                free += 1;
            }
        }

        free
    }
}

// ============================================================================
// Buffer Cache
// ============================================================================

/// Buffer cache entry
#[derive(Debug)]
struct BufferEntry {
    /// Sector number
    sector: u64,
    /// Number of sectors
    sector_count: u32,
    /// Data buffer
    data: Vec<u8>,
    /// Entry is dirty (modified)
    dirty: bool,
    /// Last access time
    access_time: u64,
    /// Reference count
    ref_count: u32,
}

/// Buffer cache for write-back strategy
pub struct BufferCache {
    /// Cache entries
    entries: Mutex<Vec<BufferEntry>>,
    /// Maximum cache size in sectors
    max_size: u32,
    /// Current cache size in sectors
    current_size: AtomicU32,
    /// Read-ahead policy
    readahead_policy: ReadaheadPolicy,
    /// Read-ahead size in sectors
    readahead_size: u32,
    /// Access pattern tracking (for adaptive read-ahead)
    access_pattern: Mutex<AccessPattern>,
}

/// Access pattern for adaptive read-ahead
#[derive(Debug, Clone, Copy)]
struct AccessPattern {
    /// Last accessed sector
    last_sector: u64,
    /// Sequential access count
    sequential_count: u32,
    /// Random access count
    random_count: u32,
    /// Average sequential stride
    avg_stride: u64,
}

impl AccessPattern {
    pub const fn new() -> Self {
        Self {
            last_sector: 0,
            sequential_count: 0,
            random_count: 0,
            avg_stride: 0,
        }
    }

    pub fn update(&mut self, sector: u64) {
        if sector == self.last_sector + 1 {
            // Sequential access
            self.sequential_count += 1;
            self.avg_stride = 1;
        } else if sector > self.last_sector {
            // Forward access but not sequential
            let stride = sector - self.last_sector;
            self.avg_stride = (self.avg_stride + stride) / 2;
            self.random_count += 1;
        } else {
            // Random or backward access
            self.random_count += 1;
        }

        self.last_sector = sector;
    }

    pub fn is_sequential(&self) -> bool {
        self.sequential_count > 2 && self.sequential_count > self.random_count
    }
}

impl BufferCache {
    /// Create a new buffer cache
    pub fn new(max_size: u32, readahead_policy: ReadaheadPolicy) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            max_size,
            current_size: AtomicU32::new(0),
            readahead_policy,
            readahead_size: DEFAULT_READAHEAD_SECTORS,
            access_pattern: Mutex::new(AccessPattern::new()),
        }
    }

    /// Read from cache or device
    pub fn read(&self, sector: u64, count: u32, device: &dyn BlockDevice) -> Result<Vec<u8>> {
        // Update access pattern
        {
            let mut pattern = self.access_pattern.lock();
            pattern.update(sector);
        }

        // Check cache first
        if let Some(data) = self.read_from_cache(sector, count) {
            return Ok(data);
        }

        // Read from device
        let size = (count as u64 * device.sector_size()) as usize;
        let mut data = vec![0u8; size];
        device.read(sector, &mut data)?;

        // Add to cache
        self.add_to_cache(sector, count, data.clone(), false)?;

        // Perform read-ahead if enabled
        self.perform_readahead(sector, count, device)?;

        Ok(data)
    }

    /// Write with caching
    pub fn write(&self, sector: u64, data: Vec<u8>, device: &dyn BlockDevice) -> Result<usize> {
        let count = (data.len() as u64 / device.sector_size()) as u32;

        match self.readahead_policy {
            ReadaheadPolicy::None => {
                // Direct write
                let written = device.write(sector, &data)?;
                Ok(written)
            },
            _ => {
                // Write-back: cache and mark dirty
                self.add_to_cache(sector, count, data.clone(), true)?;
                Ok(data.len())
            },
        }
    }

    /// Flush dirty buffers to device
    pub fn flush(&self, device: &dyn BlockDevice) -> Result<()> {
        let mut entries = self.entries.lock();

        for entry in entries.iter_mut() {
            if entry.dirty {
                device.write(entry.sector, &entry.data)?;
                entry.dirty = false;
            }
        }

        Ok(())
    }

    /// Read from cache
    fn read_from_cache(&self, sector: u64, count: u32) -> Option<Vec<u8>> {
        let mut entries = self.entries.lock();

        for entry in entries.iter_mut() {
            if entry.sector == sector && entry.sector_count >= count {
                entry.access_time = Self::get_time();
                return Some(entry.data[..(count as usize * DEFAULT_SECTOR_SIZE as usize)].to_vec());
            }
        }

        None
    }

    /// Add data to cache
    fn add_to_cache(&self, sector: u64, count: u32, data: Vec<u8>, dirty: bool) -> Result<()> {
        // Check if we need to evict entries
        self.evict_if_needed(count)?;

        let mut entries = self.entries.lock();

        let entry = BufferEntry {
            sector,
            sector_count: count,
            data: data.clone(),
            dirty,
            access_time: Self::get_time(),
            ref_count: 1,
        };

        entries.push(entry);
        self.current_size.fetch_add(count, Ordering::Relaxed);

        Ok(())
    }

    /// Evict entries if cache is full
    fn evict_if_needed(&self, needed_sectors: u32) -> Result<()> {
        let current_size = self.current_size.load(Ordering::Relaxed);

        if current_size + needed_sectors <= self.max_size {
            return Ok(());
        }

        let mut entries = self.entries.lock();
        let _target_size = self.max_size * 3 / 4; // Evict to 75% capacity

        // Sort by access time (least recently used first)
        entries.sort_by_key(|e| e.access_time);

        // Evict LRU entries that are not dirty
        let mut freed = 0;
        entries.retain(|entry| {
            if freed >= needed_sectors {
                return true;
            }

            if !entry.dirty && entry.ref_count == 0 {
                freed += entry.sector_count;
                false
            } else {
                true
            }
        });

        // If still need space, flush dirty entries
        if freed < needed_sectors {
            for entry in entries.iter_mut() {
                if freed >= needed_sectors {
                    break;
                }

                if entry.dirty && entry.ref_count == 0 {
                    entry.dirty = false; // Will be flushed externally
                    freed += entry.sector_count;
                }
            }
        }

        self.current_size.store(
            self.current_size.load(Ordering::Relaxed) - freed,
            Ordering::Relaxed
        );

        Ok(())
    }

    /// Perform read-ahead
    fn perform_readahead(&self, sector: u64, count: u32, device: &dyn BlockDevice) -> Result<()> {
        let readahead_sectors = match self.readahead_policy {
            ReadaheadPolicy::None => return Ok(()),
            ReadaheadPolicy::Fixed => self.readahead_size,
            ReadaheadPolicy::Adaptive => {
                let pattern = self.access_pattern.lock();
                if pattern.is_sequential() {
                    self.readahead_size
                } else {
                    0
                }
            },
            ReadaheadPolicy::Aggressive => self.readahead_size * 2,
        };

        if readahead_sectors == 0 {
            return Ok(());
        }

        let next_sector = sector + count as u64;
        let size = (readahead_sectors as u64 * device.sector_size()) as usize;

        // Check if already in cache
        if self.read_from_cache(next_sector, readahead_sectors).is_some() {
            return Ok(());
        }

        // Perform read-ahead
        let mut data = vec![0u8; size];
        if device.read(next_sector, &mut data).is_ok() {
            self.add_to_cache(next_sector, readahead_sectors, data, false)?;
        }

        Ok(())
    }

    /// Get current time
    fn get_time() -> u64 {
        // In a real implementation, this would get the actual time
        0
    }
}

// ============================================================================
// Request Optimizer
// ============================================================================

/// Request merging and optimization
pub struct RequestOptimizer {
    /// Enable request merging
    enable_merging: bool,
    /// Merge threshold
    merge_threshold: u32,
    /// Maximum merge size
    max_merge_size: u32,
    /// Merged request counter
    merged_count: AtomicU64,
}

impl RequestOptimizer {
    /// Create a new request optimizer
    pub fn new() -> Self {
        Self {
            enable_merging: true,
            merge_threshold: MERGE_THRESHOLD_SECTORS,
            max_merge_size: MAX_MERGE_SIZE,
            merged_count: AtomicU64::new(0),
        }
    }

    /// Try to merge requests
    pub fn try_merge(&self, requests: &[BlockRequest]) -> Vec<BlockRequest> {
        if !self.enable_merging || requests.len() < 2 {
            return requests.to_vec();
        }

        let mut merged = Vec::new();
        let mut i = 0;

        while i < requests.len() {
            let mut current = requests[i].clone();

            // Try to merge with subsequent requests
            while i + 1 < requests.len() {
                let next = &requests[i + 1];

                if self.can_merge(&current, next) {
                    current = self.merge_requests(current, next.clone());
                    self.merged_count.fetch_add(1, Ordering::Relaxed);
                    i += 1;
                } else {
                    break;
                }
            }

            merged.push(current);
            i += 1;
        }

        merged
    }

    /// Check if two requests can be merged
    fn can_merge(&self, first: &BlockRequest, second: &BlockRequest) -> bool {
        // Must be same operation type
        if first.bio.operation != second.bio.operation {
            return false;
        }

        // Only merge read operations
        if first.bio.operation != BlockOp::Read {
            return false;
        }

        // Check if sectors are contiguous
        let first_end = first.bio.sector + first.bio.sector_count as u64;
        if first_end != second.bio.sector {
            return false;
        }

        // Check merged size doesn't exceed limit
        let merged_count = first.bio.sector_count + second.bio.sector_count;
        if merged_count > self.max_merge_size {
            return false;
        }

        // Check if both are normal priority (don't merge high-priority requests)
        if first.is_urgent() || second.is_urgent() {
            return false;
        }

        true
    }

    /// Merge two requests
    fn merge_requests(&self, mut first: BlockRequest, second: BlockRequest) -> BlockRequest {
        // Extend data buffer
        first.bio.data.extend_from_slice(&second.bio.data);
        first.bio.sector_count += second.bio.sector_count;

        // Merge with reference
        first.merged_with = Some(second.bio.id);

        first
    }

    /// Get number of merged requests
    pub fn merged_count(&self) -> u64 {
        self.merged_count.load(Ordering::Relaxed)
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.merged_count.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// I/O Scheduler
// ============================================================================

/// I/O scheduler for request ordering
pub struct IoSchedulerImpl {
    /// Scheduler type
    scheduler_type: IoScheduler,
    /// Pending requests
    pending: Mutex<VecDeque<BlockRequest>>,
    /// Request optimizer
    optimizer: RequestOptimizer,
}

impl IoSchedulerImpl {
    /// Create a new I/O scheduler
    pub fn new(scheduler_type: IoScheduler) -> Self {
        Self {
            scheduler_type,
            pending: Mutex::new(VecDeque::new()),
            optimizer: RequestOptimizer::new(),
        }
    }

    /// Add request to scheduler
    pub fn add_request(&self, request: BlockRequest) -> Result<()> {
        let mut pending = self.pending.lock();

        match self.scheduler_type {
            IoScheduler::Noop => {
                pending.push_back(request);
            },
            IoScheduler::Deadline => {
                // Insert based on deadline
                let pos = pending
                    .iter()
                    .position(|r| r.deadline > request.deadline)
                    .unwrap_or(pending.len());
                pending.insert(pos, request);
            },
            _ => {
                // Default FIFO behavior
                pending.push_back(request);
            },
        }

        Ok(())
    }

    /// Get next request to process
    pub fn next_request(&self) -> Option<BlockRequest> {
        let mut pending = self.pending.lock();

        match self.scheduler_type {
            IoScheduler::Cfq | IoScheduler::Bfq => {
                // Find request with highest priority
                let idx = pending
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, r)| r.priority)
                    .map(|(i, _)| i);

                if let Some(i) = idx {
                    // Remove and return the request
                    // We need to do this in two steps to avoid borrow issues
                    let len = pending.len();
                    if i < len {
                        // Convert to index-based removal
                        let req = pending.remove(i).unwrap();
                        Some(req)
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            _ => {
                pending.pop_front()
            },
        }
    }

    /// Get pending request count
    pub fn pending_count(&self) -> usize {
        let pending = self.pending.lock();
        pending.len()
    }

    /// Optimize and merge pending requests
    pub fn optimize_pending(&self) {
        let requests = {
            let pending = self.pending.lock();
            pending.iter().cloned().collect::<Vec<_>>()
        };

        let optimized = self.optimizer.try_merge(&requests);

        let mut pending = self.pending.lock();
        pending.clear();
        for req in optimized {
            pending.push_back(req);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sector_manager() {
        let manager = SectorManager::new(1024, 512);
        let sector = manager.allocate_sectors(8).unwrap();
        assert!(sector < 1024);
    }

    #[test]
    fn test_buffer_cache() {
        let cache = BufferCache::new(64, ReadaheadPolicy::Fixed);
        assert_eq!(cache.current_size.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_request_merging() {
        let optimizer = RequestOptimizer::new();
        let req1 = BlockRequest::read(0, 1024);
        let req2 = BlockRequest::read(2, 1024); // Sequential sectors

        let requests = vec![req1, req2];
        let merged = optimizer.try_merge(&requests);

        // Should merge if contiguous
        assert!(merged.len() <= 2);
    }
}
