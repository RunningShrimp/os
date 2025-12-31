//! # Swap Compression (zswap)
//!
//! Compressed swap cache to reduce swap I/O and improve performance.
//!
//! ## Features
//!
//! - **Compressed Swap Space**: Store swapped out pages in compressed form
//! - **Swap-in/out Strategies**: Intelligent policies for when to swap
//! - **Compression Cache**: LRU cache for frequently swapped pages
//! - **Thrashing Prevention**: Detect and prevent swap thrashing
//! - **Performance Tuning**: Adaptive parameters based on workload
//!
//! ## Architecture
//!
//! ```
//! zSwap Compressed Cache
//!     ├── Swap Operations
//!     │   ├── Swap-in: Decompress and load into memory
//!     │   └── Swap-out: Compress and store in cache
//!     ├── Compression Cache
//!     │   ├── LRU eviction policy
//!     │   ├── Size limits
//!     │   └── Performance monitoring
//!     └── Thrash Detection
//!         ├── Swap rate monitoring
//!         ├── Access pattern analysis
//!         └── Adaptive intervention
//! ```
//!
//! ## Benefits
//!
//! - **Reduced Swap I/O**: Compressed data = faster transfers
//! - **Lower Latency**: Smaller data size = quicker operations
//! - **Extended Memory**: Cache acts as extended RAM
//! - **Better Performance**: Fewer disk accesses

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::mm::compression::{self, CompressionAlgorithm};
use crate::subsystems::sync::Mutex;
use crate::subsystems::time;

// ============================================================================
// Constants
// ============================================================================

/// Default maximum zswap size (in pages)
pub const DEFAULT_MAX_ZSWAP_PAGES: usize = 10000;

/// Maximum age for zswap entry before eviction (milliseconds)
pub const MAX_ZSWAP_AGE_MS: u64 = 60000; // 1 minute

/// Thrash detection threshold (swaps per second)
pub const THRASH_THRESHOLD_SWAPS_PER_SEC: u64 = 100;

/// Thrash recovery cooldown (milliseconds)
pub const THRASH_RECOVERY_COOLDOWN_MS: u64 = 5000;

/// Minimum compression ratio for zswap
pub const MIN_ZSWAP_COMPRESSION_RATIO: f32 = 0.6;

// ============================================================================
// zSwap Entry
// ============================================================================

/// zSwap cache entry metadata
#[derive(Debug)]
pub struct ZswapEntry {
    /// Original virtual address
    pub vaddr: usize,
    /// Original page size
    pub original_size: usize,
    /// Compressed data size
    pub compressed_size: usize,
    /// Compression algorithm used
    pub algorithm: CompressionAlgorithm,
    /// Creation timestamp (ticks)
    pub created: u64,
    /// Last access timestamp (ticks) - using AtomicU64 for interior mutability
    pub last_access: AtomicU64,
    /// Access count
    pub access_count: AtomicUsize,
    /// Whether page is modified
    pub is_dirty: AtomicBool,
    /// Compression ratio achieved
    pub compression_ratio: f32,
}

impl ZswapEntry {
    /// Create new zswap entry
    pub fn new(
        vaddr: usize,
        original_size: usize,
        compressed_size: usize,
        algorithm: CompressionAlgorithm,
    ) -> Self {
        let now = time::get_ticks();

        let compression_ratio = if original_size > 0 {
            compressed_size as f32 / original_size as f32
        } else {
            1.0
        };

        Self {
            vaddr,
            original_size,
            compressed_size,
            algorithm,
            created: now,
            last_access: AtomicU64::new(now),
            access_count: AtomicUsize::new(0),
            is_dirty: AtomicBool::new(false),
            compression_ratio,
        }
    }

    /// Mark entry as accessed (cannot mutate, use last_access directly)
    pub fn mark_accessed(&self) {
        self.last_access.store(time::get_ticks(), Ordering::Relaxed);
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get access count
    pub fn access_count(&self) -> usize {
        self.access_count.load(Ordering::Relaxed)
    }

    /// Check if entry is old (should be evicted)
    pub fn is_old(&self, max_age_ms: u64) -> bool {
        let now = time::get_ticks();
        let max_age_ticks = max_age_ms * time::TIMER_FREQ as u64 / 1000;
        let last_access = self.last_access.load(Ordering::Relaxed);
        (now - last_access) > max_age_ticks
    }

    /// Mark entry as dirty
    pub fn mark_dirty(&self) {
        self.is_dirty.store(true, Ordering::Relaxed);
    }

    /// Check if entry is dirty
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Thrash Detection
// ============================================================================

/// Swap thrash detector
#[derive(Debug)]
pub struct ThrashDetector {
    /// Swap operations count
    swap_count: AtomicUsize,
    /// Window start time (ticks)
    window_start: AtomicU64,
    /// Window size (ticks)
    window_ticks: u64,
    /// Thrash threshold (swaps per window)
    threshold: u64,
    /// Currently thrashing
    is_thrashing: AtomicBool,
    /// Last thrash detection time (ticks)
    last_thrash_time: AtomicU64,
    /// Recovery cooldown (ticks)
    recovery_cooldown_ticks: u64,
    /// Total thrash detections
    total_detections: AtomicUsize,
}

impl ThrashDetector {
    /// Create new thrash detector
    pub fn new(window_secs: u64, threshold: u64, cooldown_ms: u64) -> Self {
        let window_ticks = window_secs * time::TIMER_FREQ as u64;
        let cooldown_ticks = cooldown_ms * time::TIMER_FREQ as u64 / 1000;

        Self {
            swap_count: AtomicUsize::new(0),
            window_start: AtomicU64::new(time::get_ticks()),
            window_ticks,
            threshold,
            is_thrashing: AtomicBool::new(false),
            last_thrash_time: AtomicU64::new(0),
            recovery_cooldown_ticks: cooldown_ticks,
            total_detections: AtomicUsize::new(0),
        }
    }

    /// Record a swap operation
    pub fn record_swap(&self) -> bool {
        // Reset window if expired
        let now = time::get_ticks();
        let window_start = self.window_start.load(Ordering::Relaxed);

        if now - window_start >= self.window_ticks {
            // Reset for new window
            self.window_start.store(now, Ordering::Relaxed);
            self.swap_count.store(1, Ordering::Relaxed);
            false
        } else {
            // Increment count
            let count = self.swap_count.fetch_add(1, Ordering::Relaxed) + 1;

            // Check threshold
            if count as u64 >= self.threshold && !self.is_in_recovery() {
                // Trigger thrash detection
                self.is_thrashing.store(true, Ordering::Relaxed);
                self.last_thrash_time.store(now, Ordering::Relaxed);
                self.total_detections.fetch_add(1, Ordering::Relaxed);
                true
            } else {
                false
            }
        }
    }

    /// Check if currently thrashing
    pub fn is_thrashing(&self) -> bool {
        self.is_thrashing.load(Ordering::Relaxed)
    }

    /// Check if in recovery cooldown
    pub fn is_in_recovery(&self) -> bool {
        let now = time::get_ticks();
        let last = self.last_thrash_time.load(Ordering::Relaxed);
        now - last < self.recovery_cooldown_ticks
    }

    /// Clear thrash state
    pub fn clear_thrash(&self) {
        self.is_thrashing.store(false, Ordering::Relaxed);
    }

    /// Get swap rate (swaps per second)
    pub fn swap_rate(&self) -> f32 {
        let now = time::get_ticks();
        let window_start = self.window_start.load(Ordering::Relaxed);
        let elapsed_ticks = now - window_start;
        let elapsed_secs = elapsed_ticks as f32 / time::TIMER_FREQ as f32;

        if elapsed_secs > 0.0 {
            self.swap_count.load(Ordering::Relaxed) as f32 / elapsed_secs
        } else {
            0.0
        }
    }

    /// Get total number of thrash detections
    pub fn total_detections(&self) -> usize {
        self.total_detections.load(Ordering::Relaxed)
    }
}

// ============================================================================
// zSwap Cache
// ============================================================================

/// zSwap cache configuration
#[derive(Debug, Clone)]
pub struct ZswapConfig {
    /// Maximum cache size (in pages)
    pub max_pages: usize,
    /// Maximum entry age (milliseconds)
    pub max_age_ms: u64,
    /// Minimum compression ratio (0.0-1.0)
    pub min_compression_ratio: f32,
    /// Enable thrash detection
    pub enable_thrash_detection: bool,
    /// Thrash detection window (seconds)
    pub thrash_window_secs: u64,
    /// Thrash threshold (swaps per window)
    pub thrash_threshold: u64,
    /// Recovery cooldown (milliseconds)
    pub recovery_cooldown_ms: u64,
}

impl Default for ZswapConfig {
    fn default() -> Self {
        Self {
            max_pages: DEFAULT_MAX_ZSWAP_PAGES,
            max_age_ms: MAX_ZSWAP_AGE_MS,
            min_compression_ratio: MIN_ZSWAP_COMPRESSION_RATIO,
            enable_thrash_detection: true,
            thrash_window_secs: 1,
            thrash_threshold: THRASH_THRESHOLD_SWAPS_PER_SEC,
            recovery_cooldown_ms: THRASH_RECOVERY_COOLDOWN_MS,
        }
    }
}

/// zSwap cache statistics
#[derive(Debug, Default)]
pub struct ZswapStats {
    /// Total swap-in operations
    pub swap_ins: AtomicUsize,
    /// Total swap-out operations
    pub swap_outs: AtomicUsize,
    /// Cache hits
    pub cache_hits: AtomicUsize,
    /// Cache misses
    pub cache_misses: AtomicUsize,
    /// Pages compressed
    pub pages_compressed: AtomicUsize,
    /// Pages decompressed
    pub pages_decompressed: AtomicUsize,
    /// Evictions due to size limit
    pub evictions_size: AtomicUsize,
    /// Evictions due to age
    pub evictions_age: AtomicUsize,
    /// Total bytes saved (vs uncompressed)
    pub bytes_saved: AtomicUsize,
    /// Current cache size (pages)
    pub current_size: AtomicUsize,
    /// Maximum cache size (pages)
    pub max_size: AtomicUsize,
}

/// zSwap compressed cache
pub struct ZswapCache {
    /// Cache configuration
    config: ZswapConfig,
    /// Cache entries (vaddr -> (entry, compressed_data))
    entries: Mutex<Vec<(usize, ZswapEntry, Vec<u8>)>>,
    /// LRU tracking (vaddr in access order)
    lru_list: Mutex<VecDeque<usize>>,
    /// Thrash detector
    thrash_detector: ThrashDetector,
    /// Statistics
    stats: ZswapStats,
    /// Cache enabled
    enabled: AtomicBool,
}

impl ZswapCache {
    /// Create new zswap cache
    pub fn new(config: ZswapConfig) -> Self {
        let max_pages = config.max_pages;

        Self {
            config,
            entries: Mutex::new(Vec::new()),
            lru_list: Mutex::new(VecDeque::new()),
            thrash_detector: ThrashDetector::new(
                1, // 1 second window
                THRASH_THRESHOLD_SWAPS_PER_SEC,
                THRASH_RECOVERY_COOLDOWN_MS,
            ),
            stats: ZswapStats {
                max_size: AtomicUsize::new(max_pages),
                ..Default::default()
            },
            enabled: AtomicBool::new(true),
        }
    }

    /// Enable zswap cache
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable zswap cache
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Swap out a page (compress and store)
    pub fn swap_out(
        &self,
        vaddr: usize,
        data: &[u8],
    ) -> Result<usize, &'static str> {
        if !self.is_enabled() {
            return Err("zswap disabled");
        }

        // Record swap operation for thrash detection
        if self.config.enable_thrash_detection {
            if self.thrash_detector.record_swap() {
                // Thrashing detected, reject swap-out
                self.stats.swap_outs.fetch_add(1, Ordering::Relaxed);
                return Err("Thrashing detected");
            }
        }

        // Compress data
        let compressed = compression::compress(data)?;

        // Check compression ratio
        let ratio = if data.len() > 0 {
            compressed.len() as f32 / data.len() as f32
        } else {
            1.0
        };

        if ratio > self.config.min_compression_ratio {
            return Err("Compression ratio insufficient");
        }

        // Evict old entries if necessary
        self.evict_if_needed();

        // Create entry
        let entry = ZswapEntry::new(
            vaddr,
            data.len(),
            compressed.len(),
            compression::get_algorithm(),
        );

        // Calculate bytes saved for statistics
        let bytes_saved = data.len().saturating_sub(compressed.len());

        // Check for existing entry and update entries
        {
            let mut entries = self.entries.lock();
            if let Some(pos) = entries.iter().position(|(addr, _, _)| *addr == vaddr) {
                // Replace existing entry (can't clone entry due to Atomics)
                entries.remove(pos);
                entries.push((vaddr, entry, compressed.clone()));
            } else {
                // Add new entry
                entries.push((vaddr, entry, compressed.clone()));
                self.stats.current_size.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Update bytes saved statistic (outside the lock)
        self.stats.bytes_saved.fetch_add(bytes_saved, Ordering::Relaxed);

        // Update LRU
        let mut lru = self.lru_list.lock();
        if let Some(pos) = lru.iter().position(|&addr| addr == vaddr) {
            lru.remove(pos);
        }
        lru.push_back(vaddr);

        // Update statistics
        self.stats.swap_outs.fetch_add(1, Ordering::Relaxed);
        self.stats.pages_compressed.fetch_add(1, Ordering::Relaxed);
        let saved = data.len() - compressed.len();
        self.stats.bytes_saved.fetch_add(saved, Ordering::Relaxed);

        Ok(compressed.len())
    }

    /// Swap in a page (decompress and return)
    pub fn swap_in(&self, vaddr: usize) -> Result<Vec<u8>, &'static str> {
        if !self.is_enabled() {
            return Err("zswap disabled");
        }

        // Record swap operation
        if self.config.enable_thrash_detection {
            self.thrash_detector.record_swap();
        }

        // Find entry and clone compressed data
        let (compressed_clone, original_size) = {
            let entries = self.entries.lock();
            let pos = entries
                .iter()
                .position(|(addr, _, _)| *addr == vaddr)
                .ok_or("Entry not found")?;

            let (_vaddr, entry, compressed) = &entries[pos];
            (compressed.clone(), entry.original_size)
        };

        // Update LRU
        {
            let mut lru = self.lru_list.lock();
            if let Some(lru_pos) = lru.iter().position(|&addr| addr == vaddr) {
                lru.remove(lru_pos);
            }
            lru.push_back(vaddr);
        }

        // Decompress after releasing lock
        let decompressed = compression::decompress(&compressed_clone, original_size)?;

        // Update statistics
        self.stats.swap_ins.fetch_add(1, Ordering::Relaxed);
        self.stats.pages_decompressed.fetch_add(1, Ordering::Relaxed);
        self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);

        Ok(decompressed)
    }

    /// Check if page is in cache
    pub fn contains(&self, vaddr: usize) -> bool {
        let entries = self.entries.lock();
        entries.iter().any(|(addr, _, _)| *addr == vaddr)
    }

    /// Invalidate a cache entry
    pub fn invalidate(&self, vaddr: usize) -> bool {
        let mut entries = self.entries.lock();
        if let Some(pos) = entries.iter().position(|(addr, _, _)| *addr == vaddr) {
            entries.remove(pos);
            self.stats.current_size.fetch_sub(1, Ordering::Relaxed);

            // Update LRU
            let mut lru = self.lru_list.lock();
            if let Some(lru_pos) = lru.iter().position(|&addr| addr == vaddr) {
                lru.remove(lru_pos);
            }

            true
        } else {
            false
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        let mut entries = self.entries.lock();
        let count = entries.len();
        entries.clear();

        let mut lru = self.lru_list.lock();
        lru.clear();

        self.stats.current_size.fetch_sub(count, Ordering::Relaxed);
    }

    /// Evict old entries if cache is full
    fn evict_if_needed(&self) {
        let current_size = self.stats.current_size.load(Ordering::Relaxed);
        let max_size = self.stats.max_size.load(Ordering::Relaxed);

        if current_size < max_size {
            return;
        }

        // Evict oldest entries (by LRU)
        let mut lru = self.lru_list.lock();
        let mut entries = self.entries.lock();

        let mut evicted = 0usize;
        while !lru.is_empty() && self.stats.current_size.load(Ordering::Relaxed) >= max_size {
            if let Some(vaddr) = lru.pop_front() {
                if let Some(pos) = entries.iter().position(|(addr, _, _)| *addr == vaddr) {
                    let (_, entry, _) = &entries[pos];

                    // Check age
                    if entry.is_old(self.config.max_age_ms) {
                        entries.remove(pos);
                        self.stats.current_size.fetch_sub(1, Ordering::Relaxed);
                        self.stats.evictions_age.fetch_add(1, Ordering::Relaxed);
                        evicted += 1;
                    } else {
                        // Put back in list if not old enough
                        lru.push_back(vaddr);
                    }
                }
            }

            // Safety check to avoid infinite loop
            if evicted > 10 {
                break;
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> ZswapStats {
        ZswapStats {
            swap_ins: AtomicUsize::new(self.stats.swap_ins.load(Ordering::Relaxed)),
            swap_outs: AtomicUsize::new(self.stats.swap_outs.load(Ordering::Relaxed)),
            cache_hits: AtomicUsize::new(self.stats.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicUsize::new(self.stats.cache_misses.load(Ordering::Relaxed)),
            pages_compressed: AtomicUsize::new(self.stats.pages_compressed.load(Ordering::Relaxed)),
            pages_decompressed: AtomicUsize::new(self.stats.pages_decompressed.load(Ordering::Relaxed)),
            evictions_size: AtomicUsize::new(self.stats.evictions_size.load(Ordering::Relaxed)),
            evictions_age: AtomicUsize::new(self.stats.evictions_age.load(Ordering::Relaxed)),
            bytes_saved: AtomicUsize::new(self.stats.bytes_saved.load(Ordering::Relaxed)),
            current_size: AtomicUsize::new(self.stats.current_size.load(Ordering::Relaxed)),
            max_size: AtomicUsize::new(self.stats.max_size.load(Ordering::Relaxed)),
        }
    }

    /// Check if currently thrashing
    pub fn is_thrashing(&self) -> bool {
        self.thrash_detector.is_thrashing()
    }

    /// Get swap rate
    pub fn swap_rate(&self) -> f32 {
        self.thrash_detector.swap_rate()
    }
}

impl Default for ZswapCache {
    fn default() -> Self {
        Self::new(ZswapConfig::default())
    }
}

// ============================================================================
// Global Instance
// ============================================================================

static GLOBAL_ZSWAP_CACHE: Mutex<ZswapCache> = Mutex::new(ZswapCache {
    config: ZswapConfig {
        max_pages: DEFAULT_MAX_ZSWAP_PAGES,
        min_compression_ratio: MIN_ZSWAP_COMPRESSION_RATIO,
        max_age_ms: MAX_ZSWAP_AGE_MS,
        enable_thrash_detection: true,
        thrash_window_secs: 1,
        thrash_threshold: THRASH_THRESHOLD_SWAPS_PER_SEC,
        recovery_cooldown_ms: THRASH_RECOVERY_COOLDOWN_MS,
    },
    entries: Mutex::new(Vec::new()),
    lru_list: Mutex::new(VecDeque::new()),
    thrash_detector: ThrashDetector {
        swap_count: AtomicUsize::new(0),
        window_start: AtomicU64::new(0),
        window_ticks: time::TIMER_FREQ as u64,
        threshold: THRASH_THRESHOLD_SWAPS_PER_SEC,
        is_thrashing: AtomicBool::new(false),
        last_thrash_time: AtomicU64::new(0),
        recovery_cooldown_ticks: THRASH_RECOVERY_COOLDOWN_MS * time::TIMER_FREQ as u64 / 1000,
        total_detections: AtomicUsize::new(0),
    },
    stats: ZswapStats {
        swap_ins: AtomicUsize::new(0),
        swap_outs: AtomicUsize::new(0),
        cache_hits: AtomicUsize::new(0),
        cache_misses: AtomicUsize::new(0),
        pages_compressed: AtomicUsize::new(0),
        pages_decompressed: AtomicUsize::new(0),
        evictions_size: AtomicUsize::new(0),
        evictions_age: AtomicUsize::new(0),
        bytes_saved: AtomicUsize::new(0),
        current_size: AtomicUsize::new(0),
        max_size: AtomicUsize::new(DEFAULT_MAX_ZSWAP_PAGES),
    },
    enabled: AtomicBool::new(true),
});

/// Initialize zswap cache
pub fn init_zswap(config: ZswapConfig) {
    let cache = ZswapCache::new(config);
    *GLOBAL_ZSWAP_CACHE.lock() = cache;
}

/// Swap out a page
pub fn swap_out(vaddr: usize, data: &[u8]) -> Result<usize, &'static str> {
    GLOBAL_ZSWAP_CACHE.lock().swap_out(vaddr, data)
}

/// Swap in a page
pub fn swap_in(vaddr: usize) -> Result<Vec<u8>, &'static str> {
    GLOBAL_ZSWAP_CACHE.lock().swap_in(vaddr)
}

/// Check if page is in cache
pub fn contains(vaddr: usize) -> bool {
    GLOBAL_ZSWAP_CACHE.lock().contains(vaddr)
}

/// Invalidate cache entry
pub fn invalidate(vaddr: usize) -> bool {
    GLOBAL_ZSWAP_CACHE.lock().invalidate(vaddr)
}

/// Clear cache
pub fn clear_cache() {
    GLOBAL_ZSWAP_CACHE.lock().clear();
}

/// Get zswap statistics
pub fn get_stats() -> ZswapStats {
    GLOBAL_ZSWAP_CACHE.lock().stats()
}

/// Enable zswap
pub fn enable_zswap() {
    GLOBAL_ZSWAP_CACHE.lock().enable();
}

/// Disable zswap
pub fn disable_zswap() {
    GLOBAL_ZSWAP_CACHE.lock().disable();
}

/// Check if thrashing
pub fn is_thrashing() -> bool {
    GLOBAL_ZSWAP_CACHE.lock().is_thrashing()
}

/// Get swap rate
pub fn swap_rate() -> f32 {
    GLOBAL_ZSWAP_CACHE.lock().swap_rate()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_out_in() {
        let cache = ZswapCache::default();
        let vaddr = 0x1000;
        let data = vec![0x42u8; PAGE_SIZE];

        // Swap out
        let compressed_size = cache.swap_out(vaddr, &data).unwrap();
        assert!(compressed_size < data.len());
        assert!(cache.contains(vaddr));

        // Swap in
        let decompressed = cache.swap_in(vaddr).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_thrash_detection() {
        let detector = ThrashDetector::new(1, 10, 1000);

        // Should not trigger initially
        assert!(!detector.is_thrashing());

        // Trigger thrash
        for _ in 0..10 {
            detector.record_swap();
        }

        assert!(detector.is_thrashing());
    }

    #[test]
    fn test_cache_eviction() {
        let mut config = ZswapConfig::default();
        config.max_pages = 2; // Small cache for testing

        let cache = ZswapCache::new(config);
        let data = vec![0x42u8; PAGE_SIZE];

        // Fill cache
        cache.swap_out(0x1000, &data).unwrap();
        cache.swap_out(0x2000, &data).unwrap();
        cache.swap_out(0x3000, &data).unwrap(); // Should trigger eviction

        // Check that old entries were evicted
        assert!(!cache.contains(0x1000));
    }
}
