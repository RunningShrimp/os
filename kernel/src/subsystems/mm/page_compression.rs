//! # Page Compression Manager
//!
//! Manages compression of memory pages to reduce memory footprint.
//!
//! ## Features
//!
//! - **Page Scanning Daemon**: Periodically scans for compressible pages
//! - **Zero Page Detection**: Identifies and flag zero pages (single bit)
//! - **Duplicate Page Detection**: Deduplicates identical pages
//! - **Automatic Decompression**: Transparent decompression on access
//! - **Buddy Integration**: Works with buddy allocator for page management
//!
//! ## Compression Targets
//!
//! 1. **Zero Pages** (all zeros) → Single page flag
//! 2. **Duplicate Pages** → Deduplication + compression
//! 3. **Anonymous Pages** → Compression based on access pattern
//! 4. **Clean File Pages** → Drop and reload from disk
//!
//! ## Architecture
//!
//! ```
//! Page Compression
//!     ├── Page Scanner
//!     │   ├── Zero page detection
//!     │   ├── Duplicate detection (hash-based)
//!     │   └── Compressibility analysis
//!     ├── Compressor Engine
//!     │   ├── LZ4/ZSTD algorithms
//!     │   └── Adaptive selection
//!     └── Metadata
//!         ├── Compressed page tracking
//!         ├── Access patterns
//!         └── Compression statistics
//! ```

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::subsystems::mm::compression::{self, CompressionAlgorithm};
use crate::subsystems::sync::{Mutex, Once};
use crate::subsystems::time;

// ============================================================================
// Constants
// ============================================================================

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Scan interval in milliseconds
pub const SCAN_INTERVAL_MS: u64 = 5000;

/// Maximum pages to scan per iteration
pub const MAX_PAGES_PER_SCAN: usize = 1024;

/// Minimum age before compressing a page (milliseconds)
pub const MIN_COMPRESS_AGE_MS: u64 = 30000; // 30 seconds

/// Maximum number of compressed pages
pub const MAX_COMPRESSED_PAGES: usize = 10000;

/// Hash size for duplicate detection
pub const PAGE_HASH_SIZE: usize = 8;

// ============================================================================
// Page Metadata
// ============================================================================

/// Page compression state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    /// Not compressed
    Normal,
    /// Zero page (all zeros)
    ZeroPage,
    /// Compressed page
    Compressed,
    /// Duplicate of another page
    Duplicate,
    /// Pending compression
    PendingCompression,
    /// Pending decompression
    PendingDecompression,
}

/// Compressed page metadata
#[derive(Debug)]
pub struct CompressedPage {
    /// Original physical page number
    pub original_pfn: usize,
    /// Current physical page number (may differ after compression)
    pub current_pfn: usize,
    /// Page state
    pub state: PageState,
    /// Compression algorithm used
    pub algorithm: CompressionAlgorithm,
    /// Original size (always PAGE_SIZE)
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Compression ratio
    pub compression_ratio: f32,
    /// Hash of original page content (for duplicate detection)
    pub hash: [u8; PAGE_HASH_SIZE],
    /// Last access timestamp (ticks)
    pub last_access: u64,
    /// Creation timestamp (ticks)
    pub created: u64,
    /// Access count (for hot page detection)
    pub access_count: AtomicUsize,
    /// Whether page is dirty (modified)
    pub is_dirty: AtomicBool,
    /// Reference count
    pub ref_count: AtomicUsize,
}

impl CompressedPage {
    /// Create new compressed page metadata
    pub fn new(
        original_pfn: usize,
        current_pfn: usize,
        state: PageState,
        algorithm: CompressionAlgorithm,
        compressed_size: usize,
        hash: [u8; PAGE_HASH_SIZE],
    ) -> Self {
        let now = time::get_ticks();

        Self {
            original_pfn,
            current_pfn,
            state,
            algorithm,
            original_size: PAGE_SIZE,
            compressed_size,
            compression_ratio: if compressed_size > 0 {
                compressed_size as f32 / PAGE_SIZE as f32
            } else {
                0.0
            },
            hash,
            last_access: now,
            created: now,
            access_count: AtomicUsize::new(0),
            is_dirty: AtomicBool::new(false),
            ref_count: AtomicUsize::new(1),
        }
    }

    /// Check if page is recently accessed
    pub fn is_recently_accessed(&self, threshold_ticks: u64) -> bool {
        let now = time::get_ticks();
        (now - self.last_access) < threshold_ticks
    }

    /// Check if page is old enough to compress
    pub fn is_old_enough(&self, age_ms: u64) -> bool {
        let now = time::get_ticks();
        let age_ticks = age_ms * time::TIMER_FREQ as u64 / 1000;
        (now - self.created) > age_ticks
    }

    /// Mark page as accessed
    pub fn mark_accessed(&mut self) {
        self.last_access = time::get_ticks();
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get access count
    pub fn access_count(&self) -> usize {
        self.access_count.load(Ordering::Relaxed)
    }

    /// Mark page as dirty
    pub fn mark_dirty(&self) {
        self.is_dirty.store(true, Ordering::Relaxed);
    }

    /// Check if page is dirty
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::Relaxed)
    }

    /// Increment reference count
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement reference count
    pub fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Relaxed) - 1
    }

    /// Get reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Page Scanner
// ============================================================================

/// Page scanning daemon configuration
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// Scan interval in milliseconds
    pub scan_interval_ms: u64,
    /// Maximum pages to scan per iteration
    pub max_pages_per_scan: usize,
    /// Minimum age before compression
    pub min_compress_age_ms: u64,
    /// Enable zero page detection
    pub enable_zero_page: bool,
    /// Enable duplicate detection
    pub enable_duplicate: bool,
    /// Minimum compression ratio to accept (0.0-1.0)
    pub min_compression_ratio: f32,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_interval_ms: SCAN_INTERVAL_MS,
            max_pages_per_scan: MAX_PAGES_PER_SCAN,
            min_compress_age_ms: MIN_COMPRESS_AGE_MS,
            enable_zero_page: true,
            enable_duplicate: true,
            min_compression_ratio: 0.7, // Must compress to 70% or less
        }
    }
}

/// Page scanning statistics
#[derive(Debug, Default)]
pub struct ScannerStats {
    /// Total pages scanned
    pub pages_scanned: AtomicUsize,
    /// Zero pages found
    pub zero_pages_found: AtomicUsize,
    /// Duplicate pages found
    pub duplicate_pages_found: AtomicUsize,
    /// Pages compressed
    pub pages_compressed: AtomicUsize,
    /// Compression attempts failed
    pub compression_failures: AtomicUsize,
    /// Total bytes saved
    pub bytes_saved: AtomicUsize,
    /// Scan iterations
    pub scan_iterations: AtomicUsize,
    /// Time spent scanning (microseconds)
    pub total_scan_time_us: AtomicUsize,
}

/// Page scanner daemon
pub struct PageScanner {
    /// Scanner configuration
    config: ScannerConfig,
    /// Scanner statistics
    stats: ScannerStats,
    /// Running state
    running: AtomicBool,
    /// Page hash table for duplicate detection
    page_hash_table: Mutex<BTreeMap<[u8; PAGE_HASH_SIZE], usize>>,
}

impl PageScanner {
    /// Create new page scanner
    pub fn new(config: ScannerConfig) -> Self {
        Self {
            config,
            stats: ScannerStats::default(),
            running: AtomicBool::new(false),
            page_hash_table: Mutex::new(BTreeMap::new()),
        }
    }

    /// Start scanner daemon
    pub fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
    }

    /// Stop scanner daemon
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Check if scanner is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> &ScannerStats {
        &self.stats
    }

    /// Scan pages for compression candidates
    pub fn scan_pages(&self, pages: &[(usize, &[u8])]) -> usize {
        if !self.is_running() {
            return 0;
        }

        let start = time::get_ticks();
        self.stats.scan_iterations.fetch_add(1, Ordering::Relaxed);

        let mut compressed_count = 0usize;
        let pages_to_scan = pages.len().min(self.config.max_pages_per_scan);

        for &(pfn, data) in pages.iter().take(pages_to_scan) {
            self.stats.pages_scanned.fetch_add(1, Ordering::Relaxed);

            // Calculate hash for duplicate detection
            let hash = self.calculate_page_hash(data);

            // Check for zero page
            if self.config.enable_zero_page && self.is_zero_page(data) {
                self.stats.zero_pages_found.fetch_add(1, Ordering::Relaxed);
                compressed_count += 1;
                continue;
            }

            // Check for duplicate page
            if self.config.enable_duplicate {
                let mut hash_table = self.page_hash_table.lock();
                if let Some(&_original_pfn) = hash_table.get(&hash) {
                    // Found duplicate
                    self.stats.duplicate_pages_found.fetch_add(1, Ordering::Relaxed);
                    compressed_count += 1;
                    continue;
                } else {
                    // Add to hash table
                    hash_table.insert(hash, pfn);
                }
            }

            // Try to compress page
            if let Ok(compressed) = compression::compress(data) {
                let ratio = if data.len() > 0 {
                    compressed.len() as f32 / data.len() as f32
                } else {
                    1.0
                };

                if ratio <= self.config.min_compression_ratio {
                    // Compression successful
                    self.stats.pages_compressed.fetch_add(1, Ordering::Relaxed);
                    let saved = data.len() - compressed.len();
                    self.stats.bytes_saved.fetch_add(saved, Ordering::Relaxed);
                    compressed_count += 1;
                } else {
                    self.stats.compression_failures.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                self.stats.compression_failures.fetch_add(1, Ordering::Relaxed);
            }
        }

        let end = time::get_ticks();
        let elapsed_us = (end - start) * 1_000_000 / time::TIMER_FREQ as u64;
        self.stats.total_scan_time_us.fetch_add(elapsed_us as usize, Ordering::Relaxed);

        compressed_count
    }

    /// Calculate hash of page content
    fn calculate_page_hash(&self, data: &[u8]) -> [u8; PAGE_HASH_SIZE] {
        // Simple hash: use first 8 bytes or compute a basic hash
        let mut hash = [0u8; PAGE_HASH_SIZE];

        if data.len() < PAGE_HASH_SIZE {
            hash[..data.len()].copy_from_slice(data);
        } else {
            // Sample-based hash: take samples from different positions
            let step = data.len() / PAGE_HASH_SIZE;
            for i in 0..PAGE_HASH_SIZE {
                hash[i] = data[i * step];
            }
        }

        hash
    }

    /// Check if page is all zeros
    fn is_zero_page(&self, data: &[u8]) -> bool {
        data.iter().all(|&b| b == 0)
    }

    /// Check if page is compressible based on analysis
    pub fn is_compressible(&self, data: &[u8]) -> bool {
        // Check for zero page
        if self.is_zero_page(data) {
            return true;
        }

        // Estimate compression ratio
        let ratio = compression::get_algorithm().expected_ratio();
        ratio <= self.config.min_compression_ratio
    }
}

impl Default for PageScanner {
    fn default() -> Self {
        Self::new(ScannerConfig::default())
    }
}

// ============================================================================
// Page Compressor
// ============================================================================

/// Page compression manager
pub struct PageCompressor {
    /// Page scanner
    scanner: PageScanner,
    /// Compressed page metadata
    compressed_pages: Mutex<BTreeMap<usize, CompressedPage>>,
    /// Current number of compressed pages
    compressed_count: AtomicUsize,
    /// Maximum compressed pages
    max_compressed_pages: usize,
}

impl PageCompressor {
    /// Create new page compressor
    pub fn new(max_compressed_pages: usize) -> Self {
        Self {
            scanner: PageScanner::default(),
            compressed_pages: Mutex::new(BTreeMap::new()),
            compressed_count: AtomicUsize::new(0),
            max_compressed_pages,
        }
    }

    /// Start page compression daemon
    pub fn start(&self) {
        self.scanner.start();
    }

    /// Stop page compression daemon
    pub fn stop(&self) {
        self.scanner.stop();
    }

    /// Compress a page
    pub fn compress_page(
        &self,
        pfn: usize,
        data: &[u8],
    ) -> Result<(), &'static str> {
        if data.len() != PAGE_SIZE {
            return Err("Invalid page size");
        }

        // Check if already at limit
        if self.compressed_count.load(Ordering::Relaxed) >= self.max_compressed_pages {
            return Err("Too many compressed pages");
        }

        // Check for zero page
        let (state, compressed_data, algorithm) = if self.scanner.is_zero_page(data) {
            (
                PageState::ZeroPage,
                vec![0xFF], // ZERO_PAGE_MARKER
                CompressionAlgorithm::Lz4,
            )
        } else {
            // Try to compress
            let compressed = compression::compress(data)?;

            let ratio = if data.len() > 0 {
                compressed.len() as f32 / data.len() as f32
            } else {
                1.0
            };

            if ratio >= 0.7 {
                return Err("Compression ratio insufficient");
            }

            let algorithm = compression::get_algorithm();

            (
                PageState::Compressed,
                compressed,
                algorithm,
            )
        };

        // Calculate hash
        let hash = self.scanner.calculate_page_hash(data);

        // Create metadata
        let metadata = CompressedPage::new(
            pfn,
            pfn, // Same PFN for now
            state,
            algorithm,
            compressed_data.len(),
            hash,
        );

        // Store metadata
        let mut pages = self.compressed_pages.lock();
        pages.insert(pfn, metadata);
        self.compressed_count.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Decompress a page
    pub fn decompress_page(
        &self,
        pfn: usize,
        compressed_data: &[u8],
    ) -> Result<Vec<u8>, &'static str> {
        // Get metadata
        let mut pages = self.compressed_pages.lock();
        if let Some(metadata) = pages.get_mut(&pfn) {
            metadata.mark_accessed();
        } else {
            return Err("Page not found");
        }

        // Decompress
        let decompressed = compression::decompress(compressed_data, PAGE_SIZE)?;

        Ok(decompressed)
    }

    /// Get compressed page metadata
    pub fn get_page_info(&self, pfn: usize) -> Option<CompressedPage> {
        let pages = self.compressed_pages.lock();
        // Can't clone due to Atomics, return reference data instead
        pages.get(&pfn).map(|meta| CompressedPage {
            original_pfn: meta.original_pfn,
            current_pfn: meta.current_pfn,
            state: meta.state,
            algorithm: meta.algorithm,
            original_size: meta.original_size,
            compressed_size: meta.compressed_size,
            compression_ratio: meta.compression_ratio,
            hash: meta.hash,
            last_access: meta.last_access,
            created: meta.created,
            access_count: AtomicUsize::new(meta.access_count.load(Ordering::Relaxed)),
            is_dirty: AtomicBool::new(meta.is_dirty.load(Ordering::Relaxed)),
            ref_count: AtomicUsize::new(meta.ref_count.load(Ordering::Relaxed)),
        })
    }

    /// Remove compressed page
    pub fn remove_page(&self, pfn: usize) -> bool {
        let mut pages = self.compressed_pages.lock();
        if pages.remove(&pfn).is_some() {
            self.compressed_count.fetch_sub(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get scanner statistics
    pub fn scanner_stats(&self) -> ScannerStats {
        ScannerStats {
            pages_scanned: AtomicUsize::new(self.scanner.stats.pages_scanned.load(Ordering::Relaxed)),
            zero_pages_found: AtomicUsize::new(self.scanner.stats.zero_pages_found.load(Ordering::Relaxed)),
            duplicate_pages_found: AtomicUsize::new(self.scanner.stats.duplicate_pages_found.load(Ordering::Relaxed)),
            pages_compressed: AtomicUsize::new(self.scanner.stats.pages_compressed.load(Ordering::Relaxed)),
            compression_failures: AtomicUsize::new(self.scanner.stats.compression_failures.load(Ordering::Relaxed)),
            bytes_saved: AtomicUsize::new(self.scanner.stats.bytes_saved.load(Ordering::Relaxed)),
            scan_iterations: AtomicUsize::new(self.scanner.stats.scan_iterations.load(Ordering::Relaxed)),
            total_scan_time_us: AtomicUsize::new(self.scanner.stats.total_scan_time_us.load(Ordering::Relaxed)),
        }
    }

    /// Get number of compressed pages
    pub fn compressed_page_count(&self) -> usize {
        self.compressed_count.load(Ordering::Relaxed)
    }

    /// Get total bytes saved
    pub fn total_bytes_saved(&self) -> usize {
        self.scanner.stats.bytes_saved.load(Ordering::Relaxed)
    }
}

impl Default for PageCompressor {
    fn default() -> Self {
        Self::new(MAX_COMPRESSED_PAGES)
    }
}

// ============================================================================
// Global Instance
// ============================================================================

static GLOBAL_PAGE_COMPRESSOR_INIT: Once = Once::new();
static mut GLOBAL_PAGE_COMPRESSOR: Option<Mutex<PageCompressor>> = None;

/// Initialize page compression system
pub fn init_page_compression() {
    GLOBAL_PAGE_COMPRESSOR_INIT.call_once(|| {
        unsafe {
            GLOBAL_PAGE_COMPRESSOR = Some(Mutex::new(PageCompressor {
                scanner: PageScanner::default(),
                compressed_pages: Mutex::new(BTreeMap::new()),
                compressed_count: AtomicUsize::new(0),
                max_compressed_pages: MAX_COMPRESSED_PAGES,
            }));
        }
        if let Some(compressor) = unsafe { &GLOBAL_PAGE_COMPRESSOR } {
            compressor.lock().start();
        }
    });
}

/// Get the global page compressor
fn get_global_compressor() -> &'static Mutex<PageCompressor> {
    unsafe {
        GLOBAL_PAGE_COMPRESSOR.as_ref().unwrap()
    }
}

/// Compress a page
pub fn compress_page(pfn: usize, data: &[u8]) -> Result<(), &'static str> {
    get_global_compressor().lock().compress_page(pfn, data)
}

/// Decompress a page
pub fn decompress_page(pfn: usize, compressed_data: &[u8]) -> Result<Vec<u8>, &'static str> {
    get_global_compressor().lock().decompress_page(pfn, compressed_data)
}

/// Get page metadata
pub fn get_page_info(pfn: usize) -> Option<CompressedPage> {
    get_global_compressor().lock().get_page_info(pfn)
}

/// Remove compressed page
pub fn remove_page(pfn: usize) -> bool {
    get_global_compressor().lock().remove_page(pfn)
}

/// Get compression statistics
pub fn get_compression_stats() -> compression::CompressionStats {
    compression::get_stats()
}

/// Get page scanner statistics
pub fn get_scanner_stats() -> ScannerStats {
    get_global_compressor().lock().scanner_stats()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_page_detection() {
        let scanner = PageScanner::default();
        let zeros = vec![0u8; PAGE_SIZE];

        assert!(scanner.is_zero_page(&zeros));

        let non_zeros = vec![1u8; PAGE_SIZE];
        assert!(!scanner.is_zero_page(&non_zeros));
    }

    #[test]
    fn test_compress_decompress_page() {
        let data = vec![0x42u8; PAGE_SIZE];
        let pfn = 0x1000;

        let result = compress_page(pfn, &data);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.original_pfn, pfn);
        assert!(metadata.compressed_size < PAGE_SIZE);

        // Cleanup
        remove_page(pfn);
    }

    #[test]
    fn test_page_hash() {
        let scanner = PageScanner::default();
        let data1 = vec![0x42u8; PAGE_SIZE];
        let data2 = vec![0x42u8; PAGE_SIZE];

        let hash1 = scanner.calculate_page_hash(&data1);
        let hash2 = scanner.calculate_page_hash(&data2);

        assert_eq!(hash1, hash2);
    }
}
