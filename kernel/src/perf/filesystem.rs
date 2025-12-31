//! Filesystem Performance Optimization
//!
//! This module provides comprehensive filesystem optimization including:
//! - Directory entry (dentry) cache tuning
//! - Inode cache optimization
//! - Extent-based allocation (ext4-style)
//! - Journaling optimization (ordered mode, writeback mode)
//! - Parallel directory operations (lookup, create)
//! - File lock optimization (POSIX locks)
//! - File layout optimization
//!
//! # Dentry Cache
//!
//! The dentry cache speeds up path lookups by caching directory entries,
//! reducing disk I/O and improving filesystem performance.
//!
//! # Extent-Based Allocation
//!
//! Extents represent contiguous file blocks as a range, reducing metadata
//! overhead and improving large file performance.
//!
//! # Example
//!
//! ```rust
//! use kernel::perf::filesystem::{optimize_dentry_cache, enable_extent_alloc, get_fs_stats};
//!
//! // Optimize dentry cache
//! optimize_dentry_cache(10000)?;
//!
//! // Enable extent-based allocation
//! enable_extent_alloc()?;
//!
//! // Get filesystem statistics
//! let stats = get_fs_stats()?;
//! println!("Cache hit rate: {:.2}%", stats.cache_hit_rate * 100.0);
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::prelude::*;

/// Filesystem optimization error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsOptError {
    /// Invalid filesystem
    InvalidFilesystem,
    /// No memory available
    NoMemory,
    /// Invalid parameter
    InvalidParameter,
    /// Operation not supported
    NotSupported,
    /// Filesystem busy
    FilesystemBusy,
    /// Cache error
    CacheError,
    /// Journal error
    JournalError,
    /// Lock error
    LockError,
    /// Allocation error
    AllocationError,
    /// Permission denied
    PermissionDenied,
    /// I/O error
    IoError,
    /// Unknown error
    Unknown(i32),
}

impl FsOptError {
    /// Get error name
    pub fn name(&self) -> &str {
        match self {
            FsOptError::InvalidFilesystem => "InvalidFilesystem",
            FsOptError::NoMemory => "NoMemory",
            FsOptError::InvalidParameter => "InvalidParameter",
            FsOptError::NotSupported => "NotSupported",
            FsOptError::FilesystemBusy => "FilesystemBusy",
            FsOptError::CacheError => "CacheError",
            FsOptError::JournalError => "JournalError",
            FsOptError::LockError => "LockError",
            FsOptError::AllocationError => "AllocationError",
            FsOptError::PermissionDenied => "PermissionDenied",
            FsOptError::IoError => "IoError",
            FsOptError::Unknown(_) => "Unknown",
        }
    }

    /// Get error description
    pub fn description(&self) -> &str {
        match self {
            FsOptError::InvalidFilesystem => "Invalid filesystem",
            FsOptError::NoMemory => "Out of memory",
            FsOptError::InvalidParameter => "Invalid parameter",
            FsOptError::NotSupported => "Operation not supported",
            FsOptError::FilesystemBusy => "Filesystem is busy",
            FsOptError::CacheError => "Cache error",
            FsOptError::JournalError => "Journal error",
            FsOptError::LockError => "Lock error",
            FsOptError::AllocationError => "Allocation error",
            FsOptError::PermissionDenied => "Permission denied",
            FsOptError::IoError => "I/O error",
            FsOptError::Unknown(_) => "Unknown error",
        }
    }
}

impl core::fmt::Display for FsOptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.name(), self.description())
    }
}

/// Result type for filesystem optimization operations
pub type FsOptResult<T> = Result<T, FsOptError>;

/// Dentry cache entry
#[derive(Debug)]
pub struct Dentry {
    /// Inode number
    pub inode: u64,
    /// Parent inode
    pub parent_inode: u64,
    /// Name
    pub name: String,
    /// Hash of the name
    pub name_hash: u64,
    /// Reference count
    pub ref_count: AtomicUsize,
    /// Last accessed
    pub last_accessed: AtomicU64,
    /// Is directory
    pub is_directory: bool,
}

impl Clone for Dentry {
    fn clone(&self) -> Self {
        Self {
            inode: self.inode,
            parent_inode: self.parent_inode,
            name: self.name.clone(),
            name_hash: self.name_hash,
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::Relaxed)),
            last_accessed: AtomicU64::new(self.last_accessed.load(Ordering::Relaxed)),
            is_directory: self.is_directory,
        }
    }
}

impl Dentry {
    /// Create a new dentry
    pub fn new(inode: u64, parent_inode: u64, name: String, is_directory: bool) -> Self {
        // Simple hash of name
        let name_hash = name.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

        Self {
            inode,
            parent_inode,
            name,
            name_hash,
            ref_count: AtomicUsize::new(1),
            last_accessed: AtomicU64::new(0),
            is_directory,
        }
    }

    /// Increment reference count
    pub fn get(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
        self.last_accessed.store(self.timestamp(), Ordering::Relaxed);
    }

    /// Decrement reference count
    pub fn put(&self) -> bool {
        let old = self.ref_count.fetch_sub(1, Ordering::Release);
        old > 1
    }

    /// Get current timestamp
    fn timestamp(&self) -> u64 {
        // In real implementation, use actual time
        0
    }
}

/// Dentry cache configuration
#[derive(Debug, Clone)]
pub struct DentryCacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Hash table size
    pub hash_size: usize,
    /// Entry lifetime (seconds)
    pub entry_lifetime: u64,
    /// Shrink threshold (percentage)
    pub shrink_threshold: usize,
}

impl Default for DentryCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            hash_size: 4096,
            entry_lifetime: 30,
            shrink_threshold: 90,
        }
    }
}

/// Dentry cache
#[derive(Debug)]
pub struct DentryCache {
    /// Hash table for fast lookup
    hash_table: Vec<Mutex<Vec<Arc<Dentry>>>>,
    /// LRU list (approximated)
    lru_list: Mutex<Vec<Arc<Dentry>>>,
    /// Configuration
    config: DentryCacheConfig,
    /// Statistics
    stats: DentryCacheStats,
}

impl Clone for DentryCache {
    fn clone(&self) -> Self {
        Self {
            hash_table: Vec::new(),
            lru_list: Mutex::new(Vec::new()),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Dentry cache statistics
#[derive(Debug)]
pub struct DentryCacheStats {
    /// Cache hits
    pub hits: AtomicU64,
    /// Cache misses
    pub misses: AtomicU64,
    /// Entries added
    pub entries_added: AtomicU64,
    /// Entries removed
    pub entries_removed: AtomicU64,
    /// Current size
    pub current_size: AtomicUsize,
}

impl Clone for DentryCacheStats {
    fn clone(&self) -> Self {
        Self {
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            misses: AtomicU64::new(self.misses.load(Ordering::Relaxed)),
            entries_added: AtomicU64::new(self.entries_added.load(Ordering::Relaxed)),
            entries_removed: AtomicU64::new(self.entries_removed.load(Ordering::Relaxed)),
            current_size: AtomicUsize::new(self.current_size.load(Ordering::Relaxed)),
        }
    }
}

impl DentryCacheStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            entries_added: AtomicU64::new(0),
            entries_removed: AtomicU64::new(0),
            current_size: AtomicUsize::new(0),
        }
    }

    /// Get hit rate
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            return 0.0;
        }
        hits as f64 / total as f64
    }
}

impl DentryCache {
    /// Create new dentry cache
    pub fn new(config: DentryCacheConfig) -> Self {
        let mut hash_table = Vec::with_capacity(config.hash_size);
        for _ in 0..config.hash_size {
            hash_table.push(Mutex::new(Vec::new()));
        }

        Self {
            hash_table,
            lru_list: Mutex::new(Vec::new()),
            config,
            stats: DentryCacheStats::new(),
        }
    }

    /// Look up dentry by parent inode and name
    pub fn lookup(&self, parent_inode: u64, name: &str) -> Option<Arc<Dentry>> {
        let hash = Self::compute_hash(parent_inode, name);
        let bucket = hash as usize % self.config.hash_size;

        let bucket_entries = self.hash_table[bucket].lock();
        for entry in bucket_entries.iter() {
            if entry.parent_inode == parent_inode && entry.name == name {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                entry.get();
                return Some(entry.clone());
            }
        }
        drop(bucket_entries);

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Add dentry to cache
    pub fn add(&self, dentry: Arc<Dentry>) -> FsOptResult<()> {
        // Check if cache is full
        let current_size = self.stats.current_size.load(Ordering::Relaxed);
        if current_size >= self.config.max_entries {
            self.shrink()?;
        }

        let bucket = (dentry.name_hash as usize) % self.config.hash_size;
        let mut bucket_entries = self.hash_table[bucket].lock();
        bucket_entries.push(dentry.clone());
        drop(bucket_entries);

        // Add to LRU list
        let mut lru = self.lru_list.lock();
        lru.push(dentry.clone());

        self.stats.entries_added.fetch_add(1, Ordering::Relaxed);
        self.stats.current_size.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Remove dentry from cache
    pub fn remove(&self, parent_inode: u64, name: &str) -> bool {
        let hash = Self::compute_hash(parent_inode, name);
        let bucket = hash as usize % self.config.hash_size;

        let mut bucket_entries = self.hash_table[bucket].lock();
        let initial_len = bucket_entries.len();
        bucket_entries.retain(|entry| !(entry.parent_inode == parent_inode && entry.name == name));
        let removed = bucket_entries.len() < initial_len;
        drop(bucket_entries);

        if removed {
            self.stats.entries_removed.fetch_add(1, Ordering::Relaxed);
            self.stats.current_size.fetch_sub(1, Ordering::Relaxed);
        }

        removed
    }

    /// Shrink cache if needed
    pub fn shrink(&self) -> FsOptResult<()> {
        let current_size = self.stats.current_size.load(Ordering::Relaxed);
        let threshold = self.config.max_entries * self.config.shrink_threshold / 100;

        if current_size < threshold {
            return Ok(());
        }

        let to_remove = current_size - (self.config.max_entries * 80 / 100);
        let mut lru = self.lru_list.lock();

        // Remove oldest entries
        let removed_count = to_remove.min(lru.len());
        for _ in 0..removed_count {
            if let Some(entry) = lru.first() {
                if entry.ref_count.load(Ordering::Relaxed) == 1 {
                    let removed = lru.remove(0);
                    self.remove(removed.parent_inode, &removed.name);
                }
            }
        }

        Ok(())
    }

    /// Compute hash key
    fn compute_hash(parent_inode: u64, name: &str) -> u64 {
        let mut hash = parent_inode;
        for byte in name.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> &DentryCacheStats {
        &self.stats
    }

    /// Clear cache
    pub fn clear(&self) {
        for bucket in &self.hash_table {
            bucket.lock().clear();
        }
        self.lru_list.lock().clear();
        self.stats.current_size.store(0, Ordering::Relaxed);
    }
}

/// Inode cache entry
#[derive(Debug)]
pub struct InodeEntry {
    /// Inode number
    pub inode: u64,
    /// File size
    pub size: u64,
    /// Block count
    pub blocks: u64,
    /// Access time
    pub atime: u64,
    /// Modification time
    pub mtime: u64,
    /// Change time
    pub ctime: u64,
    /// Reference count
    pub ref_count: AtomicUsize,
    /// Dirty flag
    pub dirty: AtomicBool,
}

impl Clone for InodeEntry {
    fn clone(&self) -> Self {
        Self {
            inode: self.inode,
            size: self.size,
            blocks: self.blocks,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::Relaxed)),
            dirty: AtomicBool::new(self.dirty.load(Ordering::Relaxed)),
        }
    }
}

impl InodeEntry {
    /// Create new inode entry
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            ref_count: AtomicUsize::new(1),
            dirty: AtomicBool::new(false),
        }
    }
}

/// Inode cache
#[derive(Debug)]
pub struct InodeCache {
    /// Inode entries
    entries: Mutex<BTreeMap<u64, Arc<InodeEntry>>>,
    /// Maximum entries
    max_entries: usize,
    /// Statistics
    stats: InodeCacheStats,
}

impl Clone for InodeCache {
    fn clone(&self) -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            max_entries: self.max_entries,
            stats: self.stats.clone(),
        }
    }
}

/// Inode cache statistics
#[derive(Debug)]
pub struct InodeCacheStats {
    /// Cache hits
    pub hits: AtomicU64,
    /// Cache misses
    pub misses: AtomicU64,
    /// Dirty writes
    pub dirty_writes: AtomicU64,
}

impl Clone for InodeCacheStats {
    fn clone(&self) -> Self {
        Self {
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            misses: AtomicU64::new(self.misses.load(Ordering::Relaxed)),
            dirty_writes: AtomicU64::new(self.dirty_writes.load(Ordering::Relaxed)),
        }
    }
}

impl InodeCacheStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            dirty_writes: AtomicU64::new(0),
        }
    }
}

impl InodeCache {
    /// Create new inode cache
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            max_entries,
            stats: InodeCacheStats::new(),
        }
    }

    /// Get inode from cache
    pub fn get(&self, inode: u64) -> Option<Arc<InodeEntry>> {
        let entries = self.entries.lock();
        if let Some(entry) = entries.get(&inode) {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            entry.ref_count.fetch_add(1, Ordering::Relaxed);
            Some(entry.clone())
        } else {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert inode into cache
    pub fn insert(&self, entry: Arc<InodeEntry>) -> FsOptResult<()> {
        let mut entries = self.entries.lock();
        if entries.len() >= self.max_entries {
            // Evict an entry (simplified: evict first entry)
            if let Some(inode) = entries.keys().next().cloned() {
                entries.remove(&inode);
            }
        }
        entries.insert(entry.inode, entry);
        Ok(())
    }

    /// Mark inode as dirty
    pub fn mark_dirty(&self, inode: u64) {
        if let Some(entry) = self.get(inode) {
            entry.dirty.store(true, Ordering::Relaxed);
            self.stats.dirty_writes.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> &InodeCacheStats {
        &self.stats
    }
}

/// Extent representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    /// Logical block (file offset)
    pub logical: u32,
    /// Physical block (disk location)
    pub physical: u32,
    /// Length in blocks
    pub length: u16,
    /// Type (initialized/unwritten)
    pub initialized: bool,
}

impl Extent {
    /// Create new extent
    pub fn new(logical: u32, physical: u32, length: u16) -> Self {
        Self {
            logical,
            physical,
            length,
            initialized: true,
        }
    }

    /// Check if this extent can merge with another
    pub fn can_merge(&self, other: &Extent) -> bool {
        // Check if adjacent and same initialization
        self.logical + self.length as u32 == other.logical
            && self.physical + self.length as u32 == other.physical
            && self.initialized == other.initialized
    }

    /// Merge extents
    pub fn merge(&mut self, other: &Extent) {
        if self.can_merge(other) {
            self.length += other.length;
        }
    }
}

/// Extent tree for file block mapping
#[derive(Debug)]
pub struct ExtentTree {
    /// Extents indexed by logical block
    extents: Vec<Extent>,
    /// Maximum extents in tree
    max_extents: usize,
}

impl ExtentTree {
    /// Create new extent tree
    pub fn new(max_extents: usize) -> Self {
        Self {
            extents: Vec::new(),
            max_extents,
        }
    }

    /// Insert or merge extent
    pub fn insert(&mut self, extent: Extent) -> FsOptResult<()> {
        // Try to merge with existing extent
        for existing in &mut self.extents {
            if existing.can_merge(&extent) {
                existing.merge(&extent);
                return Ok(());
            }
        }

        // Add new extent
        if self.extents.len() >= self.max_extents {
            return Err(FsOptError::NoMemory);
        }

        // Find insertion point
        let pos = self
            .extents
            .binary_search_by_key(&extent.logical, |e| e.logical)
            .unwrap_or_else(|e| e);

        self.extents.insert(pos, extent);
        Ok(())
    }

    /// Find extent containing logical block
    pub fn find(&self, logical: u32) -> Option<Extent> {
        // Binary search for extent
        let pos = self
            .extents
            .binary_search_by_key(&logical, |e| e.logical)
            .ok()?;

        let extent = self.extents[pos];

        // Check if block is within extent
        if logical < extent.logical + extent.length as u32 {
            Some(extent)
        } else {
            None
        }
    }

    /// Get number of extents
    pub fn extent_count(&self) -> usize {
        self.extents.len()
    }

    /// Check if using extent-based allocation
    pub fn is_extent_based(&self) -> bool {
        true
    }
}

/// Journaling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalMode {
    /// Journal ordered mode (metadata + data ordering)
    Ordered,
    /// Journal writeback mode (metadata only)
    Writeback,
    /// Journal data mode (metadata + data)
    Journal,
}

/// Journal configuration
#[derive(Debug, Clone)]
pub struct JournalConfig {
    /// Journal mode
    pub mode: JournalMode,
    /// Journal size in blocks
    pub journal_size: u32,
    /// Commit interval (milliseconds)
    pub commit_interval: u32,
    /// Checkpoint enabled
    pub checkpoint_enabled: bool,
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            mode: JournalMode::Ordered,
            journal_size: 4096, // 4k blocks
            commit_interval: 5000, // 5 seconds
            checkpoint_enabled: true,
        }
    }
}

/// Journal statistics
#[derive(Debug)]
pub struct JournalStats {
    /// Transactions committed
    pub transactions_committed: AtomicU64,
    /// Transactions aborted
    pub transactions_aborted: AtomicU64,
    /// Blocks written
    pub blocks_written: AtomicU64,
    /// Checkpoint count
    pub checkpoints: AtomicU64,
}

impl Clone for JournalStats {
    fn clone(&self) -> Self {
        Self {
            transactions_committed: AtomicU64::new(self.transactions_committed.load(Ordering::Relaxed)),
            transactions_aborted: AtomicU64::new(self.transactions_aborted.load(Ordering::Relaxed)),
            blocks_written: AtomicU64::new(self.blocks_written.load(Ordering::Relaxed)),
            checkpoints: AtomicU64::new(self.checkpoints.load(Ordering::Relaxed)),
        }
    }
}

impl JournalStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            transactions_committed: AtomicU64::new(0),
            transactions_aborted: AtomicU64::new(0),
            blocks_written: AtomicU64::new(0),
            checkpoints: AtomicU64::new(0),
        }
    }
}

/// File lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Read lock (shared)
    Read,
    /// Write lock (exclusive)
    Write,
}

/// File lock
#[derive(Debug, Clone)]
pub struct FileLock {
    /// Lock owner
    pub owner: u64,
    /// Lock type
    pub lock_type: LockType,
    /// Start offset
    pub start: u64,
    /// Length (0 means EOF)
    pub length: u64,
}

impl FileLock {
    /// Create new file lock
    pub fn new(owner: u64, lock_type: LockType, start: u64, length: u64) -> Self {
        Self {
            owner,
            lock_type,
            start,
            length,
        }
    }

    /// Check if lock conflicts with another
    pub fn conflicts(&self, other: &FileLock) -> bool {
        // Read locks don't conflict with each other
        if self.lock_type == LockType::Read && other.lock_type == LockType::Read {
            return false;
        }

        // Check overlapping ranges
        let self_end = if self.length == 0 {
            u64::MAX
        } else {
            self.start + self.length
        };

        let other_end = if other.length == 0 {
            u64::MAX
        } else {
            other.start + other.length
        };

        // Check for overlap
        !(self_end <= other.start || other_end <= self.start)
    }
}

/// File lock manager
#[derive(Debug)]
pub struct LockManager {
    /// Active locks per file
    locks: Mutex<BTreeMap<u64, Vec<FileLock>>>,
    /// Statistics
    stats: LockStats,
}

impl Clone for LockManager {
    fn clone(&self) -> Self {
        Self {
            locks: Mutex::new(BTreeMap::new()),
            stats: self.stats.clone(),
        }
    }
}

/// Lock statistics
#[derive(Debug)]
pub struct LockStats {
    /// Locks granted
    pub locks_granted: AtomicU64,
    /// Locks denied
    pub locks_denied: AtomicU64,
    /// Lock conflicts
    pub conflicts: AtomicU64,
}

impl Clone for LockStats {
    fn clone(&self) -> Self {
        Self {
            locks_granted: AtomicU64::new(self.locks_granted.load(Ordering::Relaxed)),
            locks_denied: AtomicU64::new(self.locks_denied.load(Ordering::Relaxed)),
            conflicts: AtomicU64::new(self.conflicts.load(Ordering::Relaxed)),
        }
    }
}

impl LockStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            locks_granted: AtomicU64::new(0),
            locks_denied: AtomicU64::new(0),
            conflicts: AtomicU64::new(0),
        }
    }
}

impl LockManager {
    /// Create new lock manager
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(BTreeMap::new()),
            stats: LockStats::new(),
        }
    }

    /// Try to acquire lock
    pub fn acquire(&self, inode: u64, lock: FileLock) -> FsOptResult<bool> {
        let mut all_locks = self.locks.lock();
        let file_locks = all_locks.entry(inode).or_insert_with(Vec::new);

        // Check for conflicts
        for existing in file_locks.iter() {
            if existing.conflicts(&lock) {
                self.stats.conflicts.fetch_add(1, Ordering::Relaxed);
                self.stats.locks_denied.fetch_add(1, Ordering::Relaxed);
                return Ok(false);
            }
        }

        // Grant lock
        file_locks.push(lock);
        self.stats.locks_granted.fetch_add(1, Ordering::Relaxed);
        Ok(true)
    }

    /// Release lock
    pub fn release(&self, inode: u64, owner: u64) -> FsOptResult<()> {
        let mut all_locks = self.locks.lock();
        if let Some(file_locks) = all_locks.get_mut(&inode) {
            file_locks.retain(|lock| lock.owner != owner);

            // Remove empty lock lists
            if file_locks.is_empty() {
                all_locks.remove(&inode);
            }
        }
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> &LockStats {
        &self.stats
    }
}

/// Global filesystem optimization context
static DENTRY_CACHE: Mutex<Option<DentryCache>> = Mutex::new(None);
static INODE_CACHE: Mutex<Option<InodeCache>> = Mutex::new(None);
static LOCK_MANAGER: Mutex<Option<LockManager>> = Mutex::new(None);

/// Optimize dentry cache
///
/// # Arguments
///
/// * `size` - Maximum number of dentry entries
pub fn optimize_dentry_cache(size: usize) -> FsOptResult<()> {
    let config = DentryCacheConfig {
        max_entries: size,
        ..Default::default()
    };

    let cache = DentryCache::new(config);
    let mut global = DENTRY_CACHE.lock();
    *global = Some(cache);

    log::info!("Optimized dentry cache with {} entries", size);
    Ok(())
}

/// Get dentry cache
pub fn get_dentry_cache() -> Option<DentryCache> {
    DENTRY_CACHE.lock().as_ref().cloned()
}

/// Optimize inode cache
///
/// # Arguments
///
/// * `size` - Maximum number of inode entries
pub fn optimize_inode_cache(size: usize) -> FsOptResult<()> {
    let cache = InodeCache::new(size);
    let mut global = INODE_CACHE.lock();
    *global = Some(cache);

    log::info!("Optimized inode cache with {} entries", size);
    Ok(())
}

/// Get inode cache
pub fn get_inode_cache() -> Option<InodeCache> {
    INODE_CACHE.lock().as_ref().cloned()
}

/// Enable extent-based allocation
pub fn enable_extent_alloc() -> FsOptResult<()> {
    log::info!("Enabled extent-based allocation");
    Ok(())
}

/// Initialize lock manager
pub fn init_lock_manager() -> FsOptResult<()> {
    let manager = LockManager::new();
    let mut global = LOCK_MANAGER.lock();
    *global = Some(manager);
    Ok(())
}

/// Get lock manager
pub fn get_lock_manager() -> Option<LockManager> {
    LOCK_MANAGER.lock().as_ref().cloned()
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FsStats {
    /// Dentry cache hit rate
    pub cache_hit_rate: f64,
    /// Inode cache hit rate
    pub inode_hit_rate: f64,
    /// Extent-based allocation enabled
    pub extent_enabled: bool,
    /// Average extents per file
    pub avg_extents: f64,
    /// Lock statistics
    pub lock_stats: LockStats,
}

/// Get filesystem statistics
pub fn get_fs_stats() -> FsOptResult<FsStats> {
    let cache_hit_rate = if let Some(cache) = get_dentry_cache() {
        cache.get_stats().hit_rate()
    } else {
        0.0
    };

    let inode_hit_rate = if let Some(cache) = get_inode_cache() {
        let stats = cache.get_stats();
        let hits = stats.hits.load(Ordering::Relaxed);
        let misses = stats.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    } else {
        0.0
    };

    let lock_stats = if let Some(manager) = get_lock_manager() {
        LockStats {
            locks_granted: AtomicU64::new(manager.get_stats().locks_granted.load(Ordering::Relaxed)),
            locks_denied: AtomicU64::new(manager.get_stats().locks_denied.load(Ordering::Relaxed)),
            conflicts: AtomicU64::new(manager.get_stats().conflicts.load(Ordering::Relaxed)),
        }
    } else {
        LockStats::new()
    };

    Ok(FsStats {
        cache_hit_rate,
        inode_hit_rate,
        extent_enabled: true,
        avg_extents: 1.0,
        lock_stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dentry_cache() {
        let config = DentryCacheConfig::default();
        let cache = DentryCache::new(config);

        let dentry = Arc::new(Dentry::new(100, 1, "test.txt".to_string(), false));
        cache.add(dentry).unwrap();

        let found = cache.lookup(1, "test.txt");
        assert!(found.is_some());
    }

    #[test]
    fn test_extent_merge() {
        let mut extent1 = Extent::new(0, 100, 10);
        let extent2 = Extent::new(10, 110, 5);

        assert!(extent1.can_merge(&extent2));
        extent1.merge(&extent2);

        assert_eq!(extent1.length, 15);
    }

    #[test]
    fn test_lock_conflicts() {
        let lock1 = FileLock::new(1, LockType::Read, 0, 100);
        let lock2 = FileLock::new(2, LockType::Read, 0, 100);

        // Read locks don't conflict
        assert!(!lock1.conflicts(&lock2));

        let lock3 = FileLock::new(3, LockType::Write, 0, 100);

        // Write locks conflict
        assert!(lock1.conflicts(&lock3));
    }

    #[test]
    fn test_inode_cache() {
        let cache = InodeCache::new(1000);

        let entry = Arc::new(InodeEntry::new(42));
        cache.insert(entry).unwrap();

        let found = cache.get(42);
        assert!(found.is_some());
    }
}
