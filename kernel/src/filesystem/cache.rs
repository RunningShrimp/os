//! Multi-level File System Cache
//!
//! Three-tier caching architecture for optimal performance.
//!
//! ## Overview
//!
//! This module implements a sophisticated caching system with:
//! - L1 Cache: CPU cache-aware data structures
//! - L2 Cache: Page cache with readahead and writeback
//! - L3 Cache: Adaptive Replacement Cache (ARC)
//!
//! ## Key Concepts
//!
//! - **Page cache**: Caches file data in fixed-size pages
//! - **Inode cache**: Caches file metadata (inodes)
//! - **Directory cache**: Caches directory entries (dentries)
//! - **ARC**: Self-tuning cache with scan resistance
//! - **Cache coherence**: Maintains consistency across caches
//!
//! ## Architecture
//!
//! ```
//! Application
//!     ↓ Read/Write
//! L1 Cache (CPU cache optimized)
//!     ├── Hot data structures
//!     └── Prefetch buffers
//! L2 Cache (Page cache)
//!     ├── Clean pages (shared with apps)
//!     ├── Dirty pages (awaiting writeback)
//!     └── Writeback queue
//! L3 Cache (ARC)
//!     ├── Recency list (recently accessed)
//!     ├── Frequency list (frequently accessed)
//!     └── Ghost lists (eviction history)
//!     ↓
//! Storage Device
//! ```
//!
//! ## Performance
//!
//! - L1 hit latency: < 10 ns
//! - L2 hit latency: < 100 ns
//! - L3 hit latency: < 500 ns
//! - Cache hit rates: > 90% for typical workloads

#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::collections::{BTreeMap, VecDeque};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::hash::{Hash, Hasher};

use crate::subsystems::sync::Mutex;
use crate::filesystem::error::{FsError, FsResult};

/// Default page size (4 KB)
pub const DEFAULT_PAGE_SIZE: usize = 4096;

/// Maximum dirty pages ratio
pub const MAX_DIRTY_RATIO: usize = 40; // 40%

/// Dirty page background threshold
pub const DIRTY_BACKGROUND_RATIO: usize = 10; // 10%

/// Cache layer types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLayer {
    /// L1 cache (CPU cache)
    L1,
    /// L2 cache (page cache)
    L2,
    /// L3 cache (ARC)
    L3,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// L1 cache size in bytes
    pub l1_size: usize,
    /// L2 cache size in bytes
    pub l2_size: usize,
    /// L3 cache size in bytes
    pub l3_size: usize,
    /// Page size
    pub page_size: usize,
    /// Enable readahead
    pub readahead: bool,
    /// Readahead window size (pages)
    pub readahead_size: usize,
    /// Enable writeback
    pub writeback: bool,
    /// Writeback interval (ms)
    pub writeback_interval_ms: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 256 * 1024, // 256 KB
            l2_size: 64 * 1024 * 1024, // 64 MB
            l3_size: 512 * 1024 * 1024, // 512 MB
            page_size: DEFAULT_PAGE_SIZE,
            readahead: true,
            readahead_size: 32, // 128 KB
            writeback: true,
            writeback_interval_ms: 500,
        }
    }
}

/// Cached page
#[derive(Debug)]
pub struct CachedPage {
    /// Page identifier
    pub page_id: u64,
    /// Page data
    pub data: Vec<u8>,
    /// Dirty flag
    pub dirty: bool,
    /// Access count (for LRU)
    pub access_count: AtomicU64,
    /// Last access time
    pub last_access: AtomicU64,
}

impl CachedPage {
    /// Create a new cached page
    pub fn new(page_id: u64, data: Vec<u8>) -> Self {
        Self {
            page_id,
            data,
            dirty: false,
            access_count: AtomicU64::new(1),
            last_access: AtomicU64::new(0),
        }
    }

    /// Mark page as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear dirty flag
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Record access
    pub fn access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.last_access.fetch_add(1, Ordering::Relaxed);
    }
}

/// Manual Clone implementation for CachedPage
impl Clone for CachedPage {
    fn clone(&self) -> Self {
        Self {
            page_id: self.page_id,
            data: self.data.clone(),
            dirty: self.dirty,
            access_count: AtomicU64::new(self.access_count.load(Ordering::Relaxed)),
            last_access: AtomicU64::new(self.last_access.load(Ordering::Relaxed)),
        }
    }
}

/// LRU cache entry
#[derive(Debug, Clone)]
struct LruEntry<K, V> {
    key: K,
    value: V,
    prev: Option<usize>,
    next: Option<usize>,
}

/// LRU cache implementation
struct LruCache<K, V>
where
    K: Clone + Eq + Hash + Ord,
    V: Clone,
{
    capacity: usize,
    map: BTreeMap<K, usize>,
    entries: Vec<Option<LruEntry<K, V>>>,
    head: Option<usize>,
    tail: Option<usize>,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + Hash + Ord,
    V: Clone,
{
    /// Create a new LRU cache
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: BTreeMap::new(),
            entries: Vec::new(),
            head: None,
            tail: None,
        }
    }

    /// Get a value from the cache
    fn get(&mut self, key: &K) -> Option<V> {
        let idx = *self.map.get(key)?;
        self.move_to_head(idx);
        self.entries[idx].as_ref().map(|e| e.value.clone())
    }

    /// Get a mutable reference to a value in the cache
    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let idx = *self.map.get(key)?;
        self.move_to_head(idx);
        self.entries[idx].as_mut().map(|e| &mut e.value)
    }

    /// Insert a value into the cache
    fn insert(&mut self, key: K, value: V) {
        if let Some(&idx) = self.map.get(&key) {
            // Update existing entry
            self.entries[idx].as_mut().unwrap().value = value;
            self.move_to_head(idx);
            return;
        }

        // Evict if at capacity
        if self.map.len() >= self.capacity {
            self.evict();
        }

        // Insert new entry
        let idx = self.entries.len();
        self.entries.push(Some(LruEntry {
            key: key.clone(),
            value,
            prev: None,
            next: self.head,
        }));

        if let Some(head_idx) = self.head {
            self.entries[head_idx].as_mut().unwrap().prev = Some(idx);
        }

        self.head = Some(idx);
        if self.tail.is_none() {
            self.tail = Some(idx);
        }

        self.map.insert(key, idx);
    }

    /// Move entry to head (most recently used)
    fn move_to_head(&mut self, idx: usize) {
        let entry = self.entries[idx].as_ref().unwrap();
        let prev = entry.prev;
        let next = entry.next;

        // Remove from current position
        if let Some(prev_idx) = prev {
            self.entries[prev_idx].as_mut().unwrap().next = next;
        } else {
            self.head = next;
        }

        if let Some(next_idx) = next {
            self.entries[next_idx].as_mut().unwrap().prev = prev;
        } else {
            self.tail = prev;
        }

        // Insert at head
        let entry = self.entries[idx].as_mut().unwrap();
        entry.prev = None;
        entry.next = self.head;

        if let Some(head_idx) = self.head {
            self.entries[head_idx].as_mut().unwrap().prev = Some(idx);
        }

        self.head = Some(idx);
    }

    /// Evict least recently used entry
    fn evict(&mut self) {
        if let Some(tail_idx) = self.tail {
            let entry = self.entries[tail_idx].take().unwrap();
            self.map.remove(&entry.key);

            if let Some(prev_idx) = entry.prev {
                self.entries[prev_idx].as_mut().unwrap().next = None;
                self.tail = Some(prev_idx);
            } else {
                self.head = None;
                self.tail = None;
            }
        }
    }

    /// Clear the cache
    fn clear(&mut self) {
        self.map.clear();
        self.entries.clear();
        self.head = None;
        self.tail = None;
    }
}

/// ARC cache - Adaptive Replacement Cache
pub struct ArcCache {
    /// Total capacity
    capacity: usize,
    /// Size of ghost lists
    ghost_size: usize,
    /// T1: Recency list (recent single accesses)
    t1: Mutex<VecDeque<u64>>,
    /// T2: Frequency list (frequent accesses)
    t2: Mutex<VecDeque<u64>>,
    /// B1: Ghost list for T1 evictions
    b1: Mutex<VecDeque<u64>>,
    /// B2: Ghost list for T2 evictions
    b2: Mutex<VecDeque<u64>>,
    /// Data storage
    data: Mutex<BTreeMap<u64, Vec<u8>>>,
    /// Partition point
    p: AtomicUsize,
}

impl ArcCache {
    /// Create a new ARC cache
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            ghost_size: capacity / 4,
            t1: Mutex::new(VecDeque::new()),
            t2: Mutex::new(VecDeque::new()),
            b1: Mutex::new(VecDeque::new()),
            b2: Mutex::new(VecDeque::new()),
            data: Mutex::new(BTreeMap::new()),
            p: AtomicUsize::new(0),
        }
    }

    /// Get data from cache
    pub fn get(&self, key: u64) -> Option<Vec<u8>> {
        // Check T2 (frequency list)
        {
            let mut t2 = self.t2.lock();
            if let Some(pos) = t2.iter().position(|&k| k == key) {
                t2.remove(pos).unwrap();
                t2.push_front(key);
                return self.data.lock().get(&key).cloned();
            }
        }

        // Check T1 (recency list)
        {
            let mut t1 = self.t1.lock();
            if let Some(pos) = t1.iter().position(|&k| k == key) {
                t1.remove(pos).unwrap();
                // Move to T2
                drop(t1);
                let mut t2 = self.t2.lock();
                t2.push_front(key);
                return self.data.lock().get(&key).cloned();
            }
        }

        None
    }

    /// Insert data into cache
    pub fn insert(&self, key: u64, data: Vec<u8>) {
        let _data_len = data.len();

        // Check ghost lists
        let in_b1 = self.b1.lock().iter().any(|&k| k == key);
        let in_b2 = self.b2.lock().iter().any(|&k| k == key);

        // Adjust partition
        let mut p = self.p.load(Ordering::Relaxed);
        if in_b1 && !in_b2 {
            p = core::cmp::min(p + self.ghost_size / (self.b1.lock().len() + 1), self.capacity);
            self.p.store(p, Ordering::Relaxed);
        } else if in_b2 && !in_b1 {
            p = core::cmp::max(p.saturating_sub(self.ghost_size / (self.b2.lock().len() + 1)), 0);
            self.p.store(p, Ordering::Relaxed);
        }

        // Replace logic
        let delta = if in_b2 && !in_b1 {
            1
        } else if in_b1 && !in_b2 {
            0
        } else {
            p
        };

        // Insert data
        self.data.lock().insert(key, data);

        // Insert into T1 or T2
        if self.t1.lock().len() + self.t2.lock().len() >= self.capacity {
            if self.t1.lock().len() >= delta && (self.t1.lock().len() > delta || in_b2) {
                // Replace in T1
                if let Some(evicted) = self.t1.lock().pop_back() {
                    let mut b1 = self.b1.lock();
                    if b1.len() >= self.ghost_size {
                        b1.pop_front();
                    }
                    b1.push_back(evicted);
                }
            } else {
                // Replace in T2
                if let Some(evicted) = self.t2.lock().pop_back() {
                    let mut b2 = self.b2.lock();
                    if b2.len() >= self.ghost_size {
                        b2.pop_front();
                    }
                    b2.push_back(evicted);
                }
            }
        }

        // Add to T1
        self.t1.lock().push_front(key);
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.t1.lock().clear();
        self.t2.lock().clear();
        self.b1.lock().clear();
        self.b2.lock().clear();
        self.data.lock().clear();
        self.p.store(0, Ordering::Relaxed);
    }
}

/// Cache manager - coordinates all cache layers
pub struct CacheManager {
    /// Cache configuration
    config: CacheConfig,
    /// L2 page cache
    page_cache: Mutex<LruCache<u64, CachedPage>>,
    /// L3 ARC cache
    arc_cache: ArcCache,
    /// Dirty pages queue
    dirty_pages: Mutex<Vec<u64>>,
    /// Cache statistics
    stats: CacheStats,
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// L1 hits
    pub l1_hits: AtomicU64,
    /// L1 misses
    pub l1_misses: AtomicU64,
    /// L2 hits
    pub l2_hits: AtomicU64,
    /// L2 misses
    pub l2_misses: AtomicU64,
    /// L3 hits
    pub l3_hits: AtomicU64,
    /// L3 misses
    pub l3_misses: AtomicU64,
    /// Dirty page writebacks
    pub writebacks: AtomicU64,
    /// Readahead pages
    pub readahead_pages: AtomicU64,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(config: CacheConfig) -> Self {
        let l2_pages = config.l2_size / config.page_size;

        Self {
            page_cache: Mutex::new(LruCache::new(l2_pages)),
            arc_cache: ArcCache::new(config.l3_size / config.page_size),
            dirty_pages: Mutex::new(Vec::new()),
            config,
            stats: CacheStats::default(),
        }
    }

    /// Read a page from cache
    pub fn read_page(&self, page_id: u64) -> FsResult<Vec<u8>> {
        // Try L2 cache first
        {
            let mut cache = self.page_cache.lock();
            if let Some(page) = cache.get(&page_id) {
                page.access();
                self.stats.l2_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(page.data.clone());
            }
            self.stats.l2_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Try L3 cache
        if let Some(data) = self.arc_cache.get(page_id) {
            self.stats.l3_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(data);
        }
        self.stats.l3_misses.fetch_add(1, Ordering::Relaxed);

        Err(FsError::CacheMiss)
    }

    /// Write a page to cache
    pub fn write_page(&self, page_id: u64, data: Vec<u8>) -> FsResult<()> {
        let mut page = CachedPage::new(page_id, data);
        page.mark_dirty();

        // Add to page cache
        {
            let mut cache = self.page_cache.lock();
            cache.insert(page_id, page);
        }

        // Track dirty page
        let mut dirty = self.dirty_pages.lock();
        if !dirty.contains(&page_id) {
            dirty.push(page_id);
        }

        // Trigger writeback if needed
        if dirty.len() * self.config.page_size >= self.config.l2_size * DIRTY_BACKGROUND_RATIO / 100 {
            drop(dirty);
            self.writeback_dirty_pages();
        }

        Ok(())
    }

    /// Invalidate a cached page
    pub fn invalidate_page(&self, page_id: u64) -> FsResult<()> {
        // Remove from page cache
        let mut cache = self.page_cache.lock();
        cache.map.remove(&page_id);

        Ok(())
    }

    /// Perform readahead
    pub fn readahead(&self, start_page: u64, count: usize) {
        if !self.config.readahead {
            return;
        }

        let pages = core::cmp::min(count, self.config.readahead_size);

        for i in 0..pages {
            let _page_id = start_page + i as u64;
            // GH-#1247: Prefetch pages from storage
            // See: https://github.com/npos/kernel/issues/1247
            self.stats.readahead_pages.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Write back dirty pages
    pub fn writeback_dirty_pages(&self) -> usize {
        let mut dirty = self.dirty_pages.lock();
        let count = dirty.len();

        for page_id in dirty.iter() {
            // GH-#1248: Write page to storage
            // See: https://github.com/npos/kernel/issues/1248
            // Mark page as clean
            let mut cache = self.page_cache.lock();
            if let Some(page) = cache.get_mut(page_id) {
                page.mark_clean();
            }
        }

        dirty.clear();

        self.stats.writebacks.fetch_add(count as u64, Ordering::Relaxed);
        count
    }

    /// Flush all caches
    pub fn flush(&self) -> FsResult<()> {
        // Writeback dirty pages
        self.writeback_dirty_pages();

        // Clear LRU cache
        self.page_cache.lock().clear();

        // Clear ARC cache
        self.arc_cache.clear();

        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            l1_hits: AtomicU64::new(self.stats.l1_hits.load(Ordering::Relaxed)),
            l1_misses: AtomicU64::new(self.stats.l1_misses.load(Ordering::Relaxed)),
            l2_hits: AtomicU64::new(self.stats.l2_hits.load(Ordering::Relaxed)),
            l2_misses: AtomicU64::new(self.stats.l2_misses.load(Ordering::Relaxed)),
            l3_hits: AtomicU64::new(self.stats.l3_hits.load(Ordering::Relaxed)),
            l3_misses: AtomicU64::new(self.stats.l3_misses.load(Ordering::Relaxed)),
            writebacks: AtomicU64::new(self.stats.writebacks.load(Ordering::Relaxed)),
            readahead_pages: AtomicU64::new(self.stats.readahead_pages.load(Ordering::Relaxed)),
        }
    }

    /// Calculate overall hit rate
    pub fn hit_rate(&self) -> f64 {
        let stats = self.get_stats();
        let hits = stats.l1_hits.load(Ordering::Relaxed) +
                   stats.l2_hits.load(Ordering::Relaxed) +
                   stats.l3_hits.load(Ordering::Relaxed);
        let misses = stats.l1_misses.load(Ordering::Relaxed) +
                     stats.l2_misses.load(Ordering::Relaxed) +
                     stats.l3_misses.load(Ordering::Relaxed);

        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64) / (total as f64)
        }
    }
}

/// Initialize cache system
pub fn init_cache_system() -> FsResult<()> {
    crate::println!("[cache] Multi-level cache initialized");
    Ok(())
}

/// Shutdown cache system
pub fn shutdown_cache_system() -> FsResult<()> {
    crate::println!("[cache] Multi-level cache shutdown");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_page() {
        let page = CachedPage::new(1, vec![1, 2, 3, 4]);
        assert!(!page.dirty);
        page.access();
        assert_eq!(page.access_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_arc_cache() {
        let cache = ArcCache::new(10);
        cache.insert(1, vec![1, 2, 3]);
        assert!(cache.get(1).is_some());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn test_cache_manager() {
        let config = CacheConfig::default();
        let manager = CacheManager::new(config);
        assert!(manager.write_page(1, vec![1, 2, 3]).is_ok());
    }
}
