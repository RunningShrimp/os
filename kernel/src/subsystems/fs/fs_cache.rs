//! File System Cache Implementation
//!
//! This module provides a comprehensive file system cache implementation with multiple
//! cache levels, intelligent eviction policies, and performance optimization. It supports
//! caching of file data, metadata, directory entries, and other file system objects
//! to improve I/O performance and reduce disk access.

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::{BTreeMap, VecDeque};
use crate::collections::HashMap;
// use alloc::sync::Arc;
// use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, AtomicU8, Ordering};
// use crate::subsystems::sync::{Sleeplock, Mutex};
// use crate::subsystems::fs::fs_impl::{Buf, BufFlags, BufCache, CacheKey};
use crate::platform::drivers::BlockDevice;
use nos_nos_error_handling::unified::KernelError;

// ============================================================================
// File System Cache Constants and Types
// ============================================================================

/// Default cache size in bytes
pub const DEFAULT_CACHE_SIZE: u64 = 64 * 1024 * 1024; // 64MB

/// Minimum cache size in bytes
pub const MIN_CACHE_SIZE: u64 = 4 * 1024 * 1024; // 4MB

/// Maximum cache size in bytes
pub const MAX_CACHE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB

/// Default block size in bytes
pub const DEFAULT_BLOCK_SIZE: u32 = 4096; // 4KB

/// Maximum number of cache entries
pub const MAX_CACHE_ENTRIES: u32 = 65536;

/// Cache entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheEntryType {
    /// Data block
    DataBlock = 1,
    /// Metadata block
    MetadataBlock = 2,
    /// Directory entry
    DirectoryEntry = 3,
    /// Inode
    Inode = 4,
    /// Indirect block
    IndirectBlock = 5,
    /// Extended attribute
    ExtendedAttribute = 6,
    /// Journal entry
    JournalEntry = 7,
    /// Bitmap block
    BitmapBlock = 8,
    /// Superblock
    Superblock = 9,
    /// Group descriptor
    GroupDescriptor = 10,
}

/// Cache entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheEntryStatus {
    /// Entry is valid
    Valid = 0,
    /// Entry is being read from disk
    Reading = 1,
    /// Entry is being written to disk
    Writing = 2,
    /// Entry is dirty (needs to be written to disk)
    Dirty = 3,
    /// Entry is invalid
    Invalid = 4,
    /// Entry is being evicted
    Evicting = 5,
}

/// Cache eviction policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheEvictionPolicy {
    /// Least Recently Used (LRU)
    LRU = 0,
    /// Least Frequently Used (LFU)
    LFU = 1,
    /// First In First Out (FIFO)
    FIFO = 2,
    /// Random
    Random = 3,
    /// Clock algorithm
    Clock = 4,
    /// Adaptive Replacement Cache (ARC)
    ARC = 5,
    /// Two Queue (2Q)
    TwoQueue = 6,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Data cache hits
    pub data_hits: u64,
    /// Data cache misses
    pub data_misses: u64,
    /// Metadata cache hits
    pub metadata_hits: u64,
    /// Metadata cache misses
    pub metadata_misses: u64,
    /// Directory cache hits
    pub directory_hits: u64,
    /// Directory cache misses
    pub directory_misses: u64,
    /// Inode cache hits
    pub inode_hits: u64,
    /// Inode cache misses
    pub inode_misses: u64,
    /// Total entries
    pub total_entries: u32,
    /// Used entries
    pub used_entries: u32,
    /// Used cache size in bytes
    pub used_size: u64,
    /// Total cache size in bytes
    pub total_size: u64,
    /// Evicted entries
    pub evicted_entries: u64,
    /// Written back entries
    pub written_back_entries: u64,
    /// Average access time in microseconds
    pub avg_access_time: u64,
    /// Maximum access time in microseconds
    pub max_access_time: u64,
    /// Minimum access time in microseconds
    pub min_access_time: u64,
    /// Cache hit ratio (percentage)
    pub hit_ratio: f32,
    /// Cache utilization (percentage)
    pub utilization: f32,
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Entry key
    pub key: CacheKey,
    /// Entry type
    pub entry_type: CacheEntryType,
    /// Entry status
    pub status: CacheEntryStatus,
    /// Data buffer
    pub data: Vec<u8>,
    /// Entry size in bytes
    pub size: u32,
    /// Last access time
    pub last_access_time: u64,
    /// Creation time
    pub creation_time: u64,
    /// Modification time
    pub modification_time: u64,
    /// Access count
    pub access_count: u64,
    /// Reference count
    pub ref_count: u32,
    /// Dirty flag
    pub dirty: bool,
    /// Valid flag
    pub valid: bool,
    /// Priority
    pub priority: u8,
    /// Checksum
    pub checksum: u32,
    /// Device ID
    pub device_id: u32,
    /// Block number
    pub block_num: u32,
    /// Inode number
    pub inode_num: u32,
    /// Offset within file
    pub offset: u64,
    /// Additional metadata
    pub metadata: BTreeMap<String, Vec<u8>>,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache size in bytes
    pub cache_size: u64,
    /// Block size in bytes
    pub block_size: u32,
    /// Maximum number of entries
    pub max_entries: u32,
    /// Eviction policy
    pub eviction_policy: CacheEvictionPolicy,
    /// Enable write-back caching
    pub enable_writeback: bool,
    /// Enable read-ahead
    pub enable_readahead: bool,
    /// Read-ahead size in blocks
    pub readahead_size: u32,
    /// Write-back delay in milliseconds
    pub writeback_delay: u32,
    /// Maximum dirty entries
    pub max_dirty_entries: u32,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression threshold in bytes
    pub compression_threshold: u32,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Enable checksums
    pub enable_checksums: bool,
    /// Enable statistics
    pub enable_stats: bool,
    /// Statistics update interval in seconds
    pub stats_update_interval: u32,
}

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheLevel {
    /// L1 cache (fastest, smallest)
    L1 = 1,
    /// L2 cache (medium speed, medium size)
    L2 = 2,
    /// L3 cache (slowest, largest)
    L3 = 3,
}

/// Multi-level cache configuration
#[derive(Debug, Clone)]
pub struct MultiLevelCacheConfig {
    /// L1 cache configuration
    pub l1_config: CacheConfig,
    /// L2 cache configuration
    pub l2_config: CacheConfig,
    /// L3 cache configuration
    pub l3_config: CacheConfig,
    /// Enable L1 cache
    pub enable_l1: bool,
    /// Enable L2 cache
    pub enable_l2: bool,
    /// Enable L3 cache
    pub enable_l3: bool,
    /// Cache promotion threshold
    pub promotion_threshold: f32,
    /// Cache demotion threshold
    pub demotion_threshold: f32,
}

// ============================================================================
// File System Cache Implementation
// ============================================================================

/// File system cache
pub struct FsCache {
    /// Cache configuration
    config: CacheConfig,
    /// Cache entries
    entries: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// LRU list for eviction
    lru_list: Mutex<VecDeque<CacheKey>>,
    /// LFU map for eviction
    lfu_map: Mutex<BTreeMap<CacheKey, u64>>,
    /// FIFO queue for eviction
    fifo_queue: Mutex<VecDeque<CacheKey>>,
    /// Clock hand position
    clock_hand: AtomicU32,
    /// ARC T1 (recently evicted once)
    arc_t1: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// ARC T2 (frequently used)
    arc_t2: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// ARC B1 (recently evicted from T1)
    arc_b1: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// ARC B2 (recently evicted from T2)
    arc_b2: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// ARC ghost list sizes
    arc_p: AtomicU32,
    /// 2Q A1in (new entries)
    twoq_a1in: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// 2Q A1out (demoted entries)
    twoq_a1out: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// 2Q Am (frequently used entries)
    twoq_am: Mutex<BTreeMap<CacheKey, CacheEntry>>,
    /// Cache statistics
    stats: Mutex<CacheStats>,
    /// Used cache size
    used_size: AtomicU64,
    /// Used entries
    used_entries: AtomicU32,
    /// Cache enabled
    enabled: AtomicBool,
    /// Last statistics update time
    last_stats_update: AtomicU64,
    /// Total access time
    total_access_time: AtomicU64,
    /// Minimum access time
    min_access_time: AtomicU64,
    /// Maximum access time
    max_access_time: AtomicU64,
    /// Block device for I/O
    block_device: Option<Box<dyn BlockDevice>>,
}

impl FsCache {
    /// Create a new file system cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: Mutex::new(BTreeMap::new()),
            lru_list: Mutex::new(VecDeque::new()),
            lfu_map: Mutex::new(BTreeMap::new()),
            fifo_queue: Mutex::new(VecDeque::new()),
            clock_hand: AtomicU32::new(0),
            arc_t1: Mutex::new(BTreeMap::new()),
            arc_t2: Mutex::new(BTreeMap::new()),
            arc_b1: Mutex::new(BTreeMap::new()),
            arc_b2: Mutex::new(BTreeMap::new()),
            arc_p: AtomicU32::new(0),
            twoq_a1in: Mutex::new(BTreeMap::new()),
            twoq_a1out: Mutex::new(BTreeMap::new()),
            twoq_am: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(CacheStats {
                total_hits: 0,
                total_misses: 0,
                data_hits: 0,
                data_misses: 0,
                metadata_hits: 0,
                metadata_misses: 0,
                directory_hits: 0,
                directory_misses: 0,
                inode_hits: 0,
                inode_misses: 0,
                total_entries: 0,
                used_entries: 0,
                used_size: 0,
                total_size: 0,
                evicted_entries: 0,
                written_back_entries: 0,
                avg_access_time: 0,
                max_access_time: 0,
                min_access_time: u64::MAX,
                hit_ratio: 0.0,
                utilization: 0.0,
            }),
            used_size: AtomicU64::new(0),
            used_entries: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
            last_stats_update: AtomicU64::new(0),
            total_access_time: AtomicU64::new(0),
            min_access_time: AtomicU64::new(u64::MAX),
            max_access_time: AtomicU64::new(0),
            block_device: None,
        }
    }

    /// Initialize the cache
    pub fn init(&mut self) -> Result<(), KernelError> {
        // Initialize statistics
        {
            let mut stats = self.stats.lock();
            stats.total_size = self.config.cache_size;
            stats.total_entries = self.config.max_entries;
        }

        crate::println!("fs_cache: initialized with size {}MB, block size {}B, max entries {}",
                      self.config.cache_size / (1024 * 1024),
                      self.config.block_size,
                      self.config.max_entries);
        
        Ok(())
    }

    /// Set block device
    pub fn set_block_device(&mut self, device: Box<dyn BlockDevice>) {
        self.block_device = Some(device);
    }

    /// Get cache configuration
    pub fn get_config(&self) -> CacheConfig {
        self.config.clone()
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.update_stats();
        self.stats.lock().clone()
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        stats.total_hits = 0;
        stats.total_misses = 0;
        stats.data_hits = 0;
        stats.data_misses = 0;
        stats.metadata_hits = 0;
        stats.metadata_misses = 0;
        stats.directory_hits = 0;
        stats.directory_misses = 0;
        stats.inode_hits = 0;
        stats.inode_misses = 0;
        stats.evicted_entries = 0;
        stats.written_back_entries = 0;
        stats.avg_access_time = 0;
        stats.max_access_time = 0;
        stats.min_access_time = u64::MAX;
        stats.hit_ratio = 0.0;
        
        self.total_access_time.store(0, Ordering::SeqCst);
        self.min_access_time.store(u64::MAX, Ordering::SeqCst);
        self.max_access_time.store(0, Ordering::SeqCst);
    }

    /// Enable/disable cache
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get cache entry
    pub fn get(&self, key: &CacheKey) -> Option<CacheEntry> {
        if !self.enabled.load(Ordering::SeqCst) {
            return None;
        }

        let start_time = self.get_current_time();
        
        let entry = {
            let entries = self.entries.lock();
            entries.get(key).cloned()
        };

        if let Some(mut entry) = entry {
            // Update access statistics
            entry.last_access_time = self.get_current_time();
            entry.access_count += 1;
            
            // Update LRU list
            self.update_lru(key);
            
            // Update LFU map
            self.update_lfu(key, entry.access_count);
            
            // Update statistics
            self.update_hit_stats(&entry);
            
            // Update access time statistics
            let access_time = self.get_current_time() - start_time;
            self.update_access_time_stats(access_time);
            
            Some(entry)
        } else {
            // Update miss statistics
            self.update_miss_stats(key);
            
            None
        }
    }

    /// Put cache entry
    pub fn put(&self, key: CacheKey, entry: CacheEntry) -> Result<(), KernelError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(KernelError::InvalidState);
        }

        // Check if we need to evict entries
        self.ensure_space(entry.size as u64)?;

        // Add entry to cache
        {
            let mut entries = self.entries.lock();
            entries.insert(key.clone(), entry.clone());
        }

        // Update LRU list
        self.add_to_lru(&key);
        
        // Update LFU map
        self.add_to_lfu(&key, 1);
        
        // Update FIFO queue
        self.add_to_fifo(&key);
        
        // Update used size and entries
        self.used_size.fetch_add(entry.size as u64, Ordering::SeqCst);
        self.used_entries.fetch_add(1, Ordering::SeqCst);
        
        // Update statistics
        self.update_put_stats(&entry);
        
        Ok(())
    }

    /// Remove cache entry
    pub fn remove(&self, key: &CacheKey) -> Option<CacheEntry> {
        if !self.enabled.load(Ordering::SeqCst) {
            return None;
        }

        let entry = {
            let mut entries = self.entries.lock();
            entries.remove(key)
        };

        if let Some(entry) = entry {
            // Update used size and entries
            self.used_size.fetch_sub(entry.size as u64, Ordering::SeqCst);
            self.used_entries.fetch_sub(1, Ordering::SeqCst);
            
            // Remove from LRU list
            self.remove_from_lru(key);
            
            // Remove from LFU map
            self.remove_from_lfu(key);
            
            // Remove from FIFO queue
            self.remove_from_fifo(key);
            
            // Update statistics
            self.update_remove_stats(&entry);
            
            Some(entry)
        } else {
            None
        }
    }

    /// Flush dirty entries
    pub fn flush_dirty(&self) -> Result<(), KernelError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(KernelError::InvalidState);
        }

        let mut dirty_entries = Vec::new();
        
        // Collect dirty entries
        {
            let entries = self.entries.lock();
            for (key, entry) in entries.iter() {
                if entry.dirty && entry.status == CacheEntryStatus::Dirty {
                    dirty_entries.push((key.clone(), entry.clone()));
                }
            }
        }

        // Write dirty entries to disk
        for (key, entry) in dirty_entries {
            if let Err(e) = self.write_entry_to_disk(&entry) {
                crate::println!("fs_cache: failed to write entry to disk: {:?}", e);
                continue;
            }
            
            // Mark entry as clean
            {
                let mut entries = self.entries.lock();
                if let Some(entry) = entries.get_mut(key) {
                    entry.dirty = false;
                    entry.status = CacheEntryStatus::Valid;
                }
            }
            
            // Update statistics
            {
                let mut stats = self.stats.lock();
                stats.written_back_entries += 1;
            }
        }

        Ok(())
    }

    /// Evict entries to make space
    pub fn evict_entries(&self, required_size: u64) -> Result<(), KernelError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(KernelError::InvalidState);
        }

        let mut freed_size = 0u64;
        let mut evicted_count = 0u32;
        
        // Evict entries based on policy
        match self.config.eviction_policy {
            CacheEvictionPolicy::LRU => {
                while freed_size < required_size {
                    if let Some(key) = self.get_lru_key() {
                        if let Some(entry) = self.remove(&key) {
                            freed_size += entry.size as u64;
                            evicted_count += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
            CacheEvictionPolicy::LFU => {
                while freed_size < required_size {
                    if let Some(key) = self.get_lfu_key() {
                        if let Some(entry) = self.remove(&key) {
                            freed_size += entry.size as u64;
                            evicted_count += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
            CacheEvictionPolicy::FIFO => {
                while freed_size < required_size {
                    if let Some(key) = self.get_fifo_key() {
                        if let Some(entry) = self.remove(&key) {
                            freed_size += entry.size as u64;
                            evicted_count += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
            CacheEvictionPolicy::Clock => {
                while freed_size < required_size {
                    if let Some(key) = self.get_clock_key() {
                        if let Some(entry) = self.remove(&key) {
                            freed_size += entry.size as u64;
                            evicted_count += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
            CacheEvictionPolicy::ARC => {
                self.arc_evict(required_size, &mut freed_size, &mut evicted_count);
            }
            CacheEvictionPolicy::TwoQueue => {
                self.twoq_evict(required_size, &mut freed_size, &mut evicted_count);
            }
            CacheEvictionPolicy::Random => {
                while freed_size < required_size {
                    if let Some(key) = self.get_random_key() {
                        if let Some(entry) = self.remove(&key) {
                            freed_size += entry.size as u64;
                            evicted_count += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.evicted_entries += evicted_count as u64;
        }

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<(), KernelError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(KernelError::InvalidState);
        }

        // Flush dirty entries first
        self.flush_dirty()?;

        // Clear all entries
        {
            let mut entries = self.entries.lock();
            entries.clear();
        }

        // Clear LRU list
        {
            let mut lru_list = self.lru_list.lock();
            lru_list.clear();
        }

        // Clear LFU map
        {
            let mut lfu_map = self.lfu_map.lock();
            lfu_map.clear();
        }

        // Clear FIFO queue
        {
            let mut fifo_queue = self.fifo_queue.lock();
            fifo_queue.clear();
        }

        // Reset used size and entries
        self.used_size.store(0, Ordering::SeqCst);
        self.used_entries.store(0, Ordering::SeqCst);

        Ok(())
    }

    /// Ensure enough space for new entry
    fn ensure_space(&self, required_size: u64) -> Result<(), KernelError> {
        let current_size = self.used_size.load(Ordering::SeqCst);
        let max_size = self.config.cache_size;
        
        if current_size + required_size <= max_size {
            return Ok(());
        }
        
        // Need to evict entries
        let needed = current_size + required_size - max_size;
        self.evict_entries(needed)
    }

    /// Update LRU list
    fn update_lru(&self, key: &CacheKey) {
        let mut lru_list = self.lru_list.lock();
        
        // Remove key from current position
        lru_list.retain(|k| k != key);
        
        // Add key to the end (most recently used)
        lru_list.push_back(key.clone());
    }

    /// Add to LRU list
    fn add_to_lru(&self, key: &CacheKey) {
        let mut lru_list = self.lru_list.lock();
        lru_list.push_back(key.clone());
    }

    /// Remove from LRU list
    fn remove_from_lru(&self, key: &CacheKey) {
        let mut lru_list = self.lru_list.lock();
        lru_list.retain(|k| k != key);
    }

    /// Get LRU key
    fn get_lru_key(&self) -> Option<CacheKey> {
        let mut lru_list = self.lru_list.lock();
        lru_list.pop_front()
    }

    /// Update LFU map
    fn update_lfu(&self, key: &CacheKey, count: u64) {
        let mut lfu_map = self.lfu_map.lock();
        lfu_map.insert(key.clone(), count);
    }

    /// Add to LFU map
    fn add_to_lfu(&self, key: &CacheKey, count: u64) {
        let mut lfu_map = self.lfu_map.lock();
        lfu_map.insert(key.clone(), count);
    }

    /// Remove from LFU map
    fn remove_from_lfu(&self, key: &CacheKey) {
        let mut lfu_map = self.lfu_map.lock();
        lfu_map.remove(key);
    }

    /// Get LFU key
    fn get_lfu_key(&self) -> Option<CacheKey> {
        let mut lfu_map = self.lfu_map.lock();
        
        if let Some((key, _)) = lfu_map.iter().min_by_key(|(_, &count)| count) {
            Some(key.clone())
        } else {
            None
        }
    }

    /// Add to FIFO queue
    fn add_to_fifo(&self, key: &CacheKey) {
        let mut fifo_queue = self.fifo_queue.lock();
        fifo_queue.push_back(key.clone());
    }

    /// Remove from FIFO queue
    fn remove_from_fifo(&self, key: &CacheKey) {
        let mut fifo_queue = self.fifo_queue.lock();
        fifo_queue.retain(|k| k != key);
    }

    /// Get FIFO key
    fn get_fifo_key(&self) -> Option<CacheKey> {
        let mut fifo_queue = self.fifo_queue.lock();
        fifo_queue.pop_front()
    }

    /// Get clock key
    fn get_clock_key(&self) -> Option<CacheKey> {
        let entries = self.entries.lock();
        let entry_count = entries.len();
        
        if entry_count == 0 {
            return None;
        }
        
        let hand = self.clock_hand.fetch_add(1, Ordering::SeqCst) as usize % entry_count;
        
        if let Some((key, _)) = entries.iter().nth(hand) {
            Some(key.clone())
        } else {
            None
        }
    }

    /// Get random key
    fn get_random_key(&self) -> Option<CacheKey> {
        let entries = self.entries.lock();
        let entry_count = entries.len();
        
        if entry_count == 0 {
            return None;
        }
        
        // Simple pseudo-random selection
        let index = (self.get_current_time() as usize) % entry_count;
        
        if let Some((key, _)) = entries.iter().nth(index) {
            Some(key.clone())
        } else {
            None
        }
    }

    /// ARC eviction
    fn arc_evict(&self, required_size: u64, freed_size: &mut u64, evicted_count: &mut u32) {
        // Simplified ARC implementation
        // In a real implementation, this would implement the full ARC algorithm
        
        // First try to evict from T1 (recently evicted once)
        while *freed_size < required_size {
            let mut t1 = self.arc_t1.lock();
            if let Some((key, entry)) = t1.iter().next() {
                let key = key.clone();
                let entry = entry.clone();
                drop(t1);
                
                self.arc_t1.lock().remove(&key);
                *freed_size += entry.size as u64;
                *evicted_count += 1;
            } else {
                break;
            }
        }
        
        // Then try to evict from T2 (frequently used)
        while *freed_size < required_size {
            let mut t2 = self.arc_t2.lock();
            if let Some((key, entry)) = t2.iter().next() {
                let key = key.clone();
                let entry = entry.clone();
                drop(t2);
                
                self.arc_t2.lock().remove(&key);
                *freed_size += entry.size as u64;
                *evicted_count += 1;
            } else {
                break;
            }
        }
    }

    /// 2Q eviction
    fn twoq_evict(&self, required_size: u64, freed_size: &mut u64, evicted_count: &mut u32) {
        // Simplified 2Q implementation
        // In a real implementation, this would implement the full 2Q algorithm
        
        // First try to evict from A1out (demoted entries)
        while *freed_size < required_size {
            let mut a1out = self.twoq_a1out.lock();
            if let Some((key, entry)) = a1out.iter().next() {
                let key = key.clone();
                let entry = entry.clone();
                drop(a1out);
                
                self.twoq_a1out.lock().remove(&key);
                *freed_size += entry.size as u64;
                *evicted_count += 1;
            } else {
                break;
            }
        }
        
        // Then try to evict from A1in (new entries)
        while *freed_size < required_size {
            let mut a1in = self.twoq_a1in.lock();
            if let Some((key, entry)) = a1in.iter().next() {
                let key = key.clone();
                let entry = entry.clone();
                drop(a1in);
                
                self.twoq_a1in.lock().remove(&key);
                *freed_size += entry.size as u64;
                *evicted_count += 1;
            } else {
                break;
            }
        }
    }

    /// Write entry to disk
    fn write_entry_to_disk(&self, entry: &CacheEntry) -> Result<(), KernelError> {
        if let Some(ref block_device) = self.block_device {
            let block_size = block_device.block_size();
            let block_num = entry.block_num as usize;
            
            // Ensure data size matches block size
            let mut data = entry.data.clone();
            if data.len() < block_size {
                data.resize(block_size, 0);
            } else if data.len() > block_size {
                data.truncate(block_size);
            }
            
            block_device.write(block_num, &data);
            Ok(())
        } else {
            Err(KernelError::InvalidState)
        }
    }

    /// Update hit statistics
    fn update_hit_stats(&self, entry: &CacheEntry) {
        let mut stats = self.stats.lock();
        
        stats.total_hits += 1;
        
        match entry.entry_type {
            CacheEntryType::DataBlock => stats.data_hits += 1,
            CacheEntryType::MetadataBlock => stats.metadata_hits += 1,
            CacheEntryType::DirectoryEntry => stats.directory_hits += 1,
            CacheEntryType::Inode => stats.inode_hits += 1,
            _ => {}
        }
        
        // Update hit ratio
        if stats.total_hits + stats.total_misses > 0 {
            stats.hit_ratio = stats.total_hits as f32 / (stats.total_hits + stats.total_misses) as f32;
        }
    }

    /// Update miss statistics
    fn update_miss_stats(&self, key: &CacheKey) {
        let mut stats = self.stats.lock();
        
        stats.total_misses += 1;
        
        // Determine entry type from key
        // This is a simplified approach; in a real implementation,
        // we would have more sophisticated type detection
        if key.block_num < 1000 {
            stats.metadata_misses += 1;
        } else if key.block_num < 10000 {
            stats.inode_misses += 1;
        } else {
            stats.data_misses += 1;
        }
        
        // Update hit ratio
        if stats.total_hits + stats.total_misses > 0 {
            stats.hit_ratio = stats.total_hits as f32 / (stats.total_hits + stats.total_misses) as f32;
        }
    }

    /// Update put statistics
    fn update_put_stats(&self, entry: &CacheEntry) {
        let mut stats = self.stats.lock();
        
        stats.used_entries = self.used_entries.load(Ordering::SeqCst);
        stats.used_size = self.used_size.load(Ordering::SeqCst);
        
        // Update utilization
        if stats.total_size > 0 {
            stats.utilization = stats.used_size as f32 / stats.total_size as f32;
        }
    }

    /// Update remove statistics
    fn update_remove_stats(&self, entry: &CacheEntry) {
        let mut stats = self.stats.lock();
        
        stats.used_entries = self.used_entries.load(Ordering::SeqCst);
        stats.used_size = self.used_size.load(Ordering::SeqCst);
        
        // Update utilization
        if stats.total_size > 0 {
            stats.utilization = stats.used_size as f32 / stats.total_size as f32;
        }
    }

    /// Update access time statistics
    fn update_access_time_stats(&self, access_time: u64) {
        self.total_access_time.fetch_add(access_time, Ordering::SeqCst);
        
        // Update min access time
        let current_min = self.min_access_time.load(Ordering::SeqCst);
        if access_time < current_min {
            self.min_access_time.store(access_time, Ordering::SeqCst);
        }
        
        // Update max access time
        let current_max = self.max_access_time.load(Ordering::SeqCst);
        if access_time > current_max {
            self.max_access_time.store(access_time, Ordering::SeqCst);
        }
        
        // Update average access time
        let total_time = self.total_access_time.load(Ordering::SeqCst);
        let total_hits = self.stats.lock().total_hits;
        if total_hits > 0 {
            let mut stats = self.stats.lock();
            stats.avg_access_time = total_time / total_hits;
            stats.min_access_time = self.min_access_time.load(Ordering::SeqCst);
            stats.max_access_time = self.max_access_time.load(Ordering::SeqCst);
        }
    }

    /// Update statistics
    fn update_stats(&self) {
        if !self.config.enable_stats {
            return;
        }
        
        let current_time = self.get_current_time();
        let last_update = self.last_stats_update.load(Ordering::SeqCst);
        
        if current_time - last_update < self.config.stats_update_interval as u64 {
            return;
        }
        
        self.last_stats_update.store(current_time, Ordering::SeqCst);
        
        let mut stats = self.stats.lock();
        
        // Update current usage
        stats.used_entries = self.used_entries.load(Ordering::SeqCst);
        stats.used_size = self.used_size.load(Ordering::SeqCst);
        
        // Update utilization
        if stats.total_size > 0 {
            stats.utilization = stats.used_size as f32 / stats.total_size as f32;
        }
        
        // Update hit ratio
        if stats.total_hits + stats.total_misses > 0 {
            stats.hit_ratio = stats.total_hits as f32 / (stats.total_hits + stats.total_misses) as f32;
        }
        
        // Update access time statistics
        let total_time = self.total_access_time.load(Ordering::SeqCst);
        let total_hits = stats.total_hits;
        if total_hits > 0 {
            stats.avg_access_time = total_time / total_hits;
            stats.min_access_time = self.min_access_time.load(Ordering::SeqCst);
            stats.max_access_time = self.max_access_time.load(Ordering::SeqCst);
        }
    }

    /// Get current time in microseconds
    fn get_current_time(&self) -> u64 {
        // In a real implementation, this would get the current time
        // from the system clock
        0
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_size: DEFAULT_CACHE_SIZE,
            block_size: DEFAULT_BLOCK_SIZE,
            max_entries: MAX_CACHE_ENTRIES,
            eviction_policy: CacheEvictionPolicy::LRU,
            enable_writeback: true,
            enable_readahead: true,
            readahead_size: 8,
            writeback_delay: 1000, // 1 second
            max_dirty_entries: 1024,
            enable_compression: false,
            compression_threshold: 1024, // 1KB
            enable_encryption: false,
            enable_checksums: true,
            enable_stats: true,
            stats_update_interval: 10, // 10 seconds
        }
    }
}

/// Initialize file system cache
pub fn init() {
    crate::println!("fs_cache: initializing file system cache");
    
    // In a real implementation, this would initialize the file system cache
    // with appropriate configuration based on system resources
    
    crate::println!("fs_cache: file system cache initialized");
}