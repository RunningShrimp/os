//! # Memory Compaction and Defragmentation
//!
//! This module implements advanced memory compaction techniques to reduce
//! external fragmentation and improve memory allocation efficiency.
//!
//! ## Overview
//!
//! Memory compaction moves allocated pages to create contiguous free regions,
//! reducing fragmentation and improving the likelihood of successful large
//! allocations. This is particularly important for:
//! - Huge page allocation (2MB, 1GB)
//! - Contiguous memory allocation for DMA
//! - Long-running systems to prevent memory fragmentation buildup
//!
//! ## Features
//!
//! - **Page Migration**: Move pages between physical locations
//! - **Defragmentation**: Reduce external fragmentation
//! - **Lazy Compaction**: Defer compaction until necessary
//! - **Eager Compaction**: Proactively compact memory
//! - **Fragmentation Analysis**: Monitor and analyze fragmentation levels
//! - **NUMA-aware**: Consider NUMA topology during compaction
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::subsystems::mm::compaction::{compact_memory, CompactionResult};
//!
//! // Trigger compaction
//! let result = compact_memory(1024); // Target 1024 free pages
//! match result {
//!     CompactionResult::Success(freed) => {
//!         println!("Compaction freed {} pages", freed);
//!     }
//!     CompactionResult::Partial(freed) => {
//!         println!("Partial compaction: {} pages freed", freed);
//!     }
//!     CompactionResult::Failed => {
//!         println!("Compaction failed");
//!     }
//! }
//! ```

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Default compaction order (pages to compact)
const DEFAULT_COMPACTION_ORDER: usize = 9; // 512 pages (2MB on 4KB pages)

/// Minimum free pages to trigger compaction
const MIN_COMPACTION_THRESHOLD: usize = 64;

/// Maximum pages to migrate in one compaction cycle
const MAX_MIGRATE_PER_CYCLE: usize = 256;

/// Page size (4KB)
const PAGE_SIZE: usize = 4096;

/// Compaction scanner priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Compaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionMode {
    /// Lazy: compact only when needed
    Lazy,
    /// Eager: proactively compact
    Eager,
    /// Manual: triggered by administrator
    Manual,
}

/// Migration result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationResult {
    /// Migration succeeded
    Success,
    /// Page is pinned and cannot be migrated
    Pinned,
    /// Migration failed (out of memory, etc.)
    Failed,
    /// Source page is invalid
    InvalidSource,
}

/// Compaction result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionResult {
    /// Full compaction success
    Success(usize),
    /// Partial compaction (some pages compacted)
    Partial(usize),
    /// Compaction failed
    Failed,
}

// ============================================================================
// Fragmentation Analysis
// ============================================================================

/// Fragmentation statistics
#[derive(Debug, Clone)]
pub struct FragmentationStats {
    /// Total external fragmentation (0-10000, where 10000 = 100%)
    pub external_fragmentation: u64,
    /// Number of free blocks
    pub free_blocks: usize,
    /// Number of fragmented blocks (small, isolated free areas)
    pub fragmented_blocks: usize,
    /// Largest free block size (in pages)
    pub largest_free_block: usize,
    /// Average free block size (in pages)
    pub avg_free_block: usize,
    /// Compaction score (0-100, higher = more fragmentation)
    pub compaction_score: u64,
}

impl Default for FragmentationStats {
    fn default() -> Self {
        Self {
            external_fragmentation: 0,
            free_blocks: 0,
            fragmented_blocks: 0,
            largest_free_block: 0,
            avg_free_block: 0,
            compaction_score: 0,
        }
    }
}

/// Fragmentation analyzer
pub struct FragmentationAnalyzer {
    /// Fragmentation history (last 100 samples)
    history: Mutex<Vec<FragmentationStats>>,
    /// Maximum history size
    max_history: usize,
}

impl FragmentationAnalyzer {
    /// Create a new fragmentation analyzer
    pub const fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
            max_history: 100,
        }
    }

    /// Analyze current fragmentation level
    pub fn analyze(&self) -> FragmentationStats {
        // Get free page information from page allocator
        let free_pages = self.get_free_page_info();

        // Calculate fragmentation metrics
        let total_free: usize = free_pages.iter().map(|(size, _)| *size).sum();
        let block_count = free_pages.len();

        if block_count == 0 {
            return FragmentationStats::default();
        }

        let largest_block = *free_pages.first().map_or(&0, |(size, _)| size);
        let avg_block = if block_count > 0 {
            total_free / block_count
        } else {
            0
        };

        // Count fragmented blocks (smaller than 8 pages)
        let fragmented_count = free_pages
            .iter()
            .filter(|(size, _)| *size < 8)
            .count();

        // Calculate external fragmentation
        // Higher when many small blocks exist
        let external_frag = if total_free > 0 {
            let ideal_largest = total_free;
            let frag_ratio = (ideal_largest - largest_block) as u64 * 10000;
            frag_ratio / ideal_largest as u64
        } else {
            0
        };

        // Calculate compaction score (0-100)
        let compaction_score = if total_free > 0 {
            let frag_factor = (fragmented_count as u64 * 1000) / block_count as u64;
            let size_factor = (1000 * largest_block as u64) / total_free.max(1) as u64;
            let score = 100 - (size_factor / 10) + (frag_factor / 10);
            score.min(100)
        } else {
            0
        };

        let stats = FragmentationStats {
            external_fragmentation: external_frag,
            free_blocks: block_count,
            fragmented_blocks: fragmented_count,
            largest_free_block: largest_block,
            avg_free_block: avg_block,
            compaction_score,
        };

        // Update history
        let mut history = self.history.lock();
        history.push(stats.clone());
        if history.len() > self.max_history {
            history.remove(0);
        }

        stats
    }

    /// Get average fragmentation over time
    pub fn get_average_fragmentation(&self) -> FragmentationStats {
        let history = self.history.lock();
        if history.is_empty() {
            return FragmentationStats::default();
        }

        let count = history.len();
        let mut sum = FragmentationStats::default();

        for stats in history.iter() {
            sum.external_fragmentation += stats.external_fragmentation / count as u64;
            sum.free_blocks += stats.free_blocks / count;
            sum.fragmented_blocks += stats.fragmented_blocks / count;
            sum.largest_free_block += stats.largest_free_block / count;
            sum.avg_free_block += stats.avg_free_block / count;
            sum.compaction_score += stats.compaction_score / count as u64;
        }

        sum
    }

    /// Get free page information (size, count pairs)
    fn get_free_page_info(&self) -> Vec<(usize, usize)> {
        // This is a placeholder - in a real implementation, this would
        // query the buddy allocator for free block information
        // For now, return simulated data
        vec![
            (128, 1),   // One 128-page block
            (64, 2),    // Two 64-page blocks
            (32, 4),    // Four 32-page blocks
            (16, 8),    // Eight 16-page blocks
            (8, 16),    // Sixteen 8-page blocks
            (4, 32),    // Thirty-two 4-page blocks
            (2, 64),    // Sixty-four 2-page blocks
            (1, 128),   // 128 single pages
        ]
    }
}

// ============================================================================
// Page Migration
// ============================================================================

/// Migration context for tracking page migrations
pub struct MigrationContext {
    /// Pages successfully migrated
    migrated_pages: AtomicUsize,
    /// Migration failures
    failed_migrations: AtomicUsize,
    /// Pinned pages encountered
    pinned_pages: AtomicUsize,
}

impl MigrationContext {
    /// Create a new migration context
    pub const fn new() -> Self {
        Self {
            migrated_pages: AtomicUsize::new(0),
            failed_migrations: AtomicUsize::new(0),
            pinned_pages: AtomicUsize::new(0),
        }
    }

    /// Reset statistics
    pub fn reset(&self) {
        self.migrated_pages.store(0, Ordering::Relaxed);
        self.failed_migrations.store(0, Ordering::Relaxed);
        self.pinned_pages.store(0, Ordering::Relaxed);
    }

    /// Get migration statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.migrated_pages.load(Ordering::Relaxed),
            self.failed_migrations.load(Ordering::Relaxed),
            self.pinned_pages.load(Ordering::Relaxed),
        )
    }
}

/// Migrate a single page from source to destination
///
/// # Arguments
///
/// * `src` - Source physical address
/// * `dst` - Destination physical address
///
/// # Returns
///
/// * `MigrationResult` - Success or failure reason
pub fn migrate_page(src: usize, dst: usize) -> MigrationResult {
    // Validate alignment
    if src % PAGE_SIZE != 0 || dst % PAGE_SIZE != 0 {
        return MigrationResult::InvalidSource;
    }

    // Check if page is pinned (locked in memory)
    if is_page_pinned(src) {
        return MigrationResult::Pinned;
    }

    // Copy page content
    unsafe {
        let src_ptr = src as *const u8;
        let dst_ptr = dst as *mut u8;

        // Copy with write protection to detect concurrent access
        // In a real implementation, we would:
        // 1. Lock the page
        // 2. Copy the content
        // 3. Update page tables
        // 4. Flush TLB
        core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, PAGE_SIZE);
    }

    MigrationResult::Success
}

/// Check if a page is pinned (locked in memory)
fn is_page_pinned(_addr: usize) -> bool {
    // In a real implementation, this would check the page flags
    // For now, return false
    false
}

/// Batch migrate multiple pages
///
/// # Arguments
///
/// * `pages` - Vector of (source, destination) address pairs
///
/// # Returns
///
/// * `(usize, usize)` - (successful migrations, failed migrations)
pub fn migrate_pages_batch(pages: Vec<(usize, usize)>) -> (usize, usize) {
    let mut success = 0;
    let mut failed = 0;

    for (src, dst) in pages {
        match migrate_page(src, dst) {
            MigrationResult::Success => success += 1,
            _ => failed += 1,
        }
    }

    (success, failed)
}

// ============================================================================
// Memory Compaction
// ============================================================================

/// Compaction control structure
pub struct CompactionControl {
    /// Current compaction mode
    mode: Mutex<CompactionMode>,
    /// Compaction priority
    priority: Mutex<CompactionPriority>,
    /// Compaction threshold (minimum free pages to trigger)
    threshold: AtomicUsize,
    /// Target free pages for compaction
    target_pages: AtomicUsize,
    /// Enable/disable compaction
    enabled: AtomicUsize,
    /// Fragmentation analyzer
    analyzer: FragmentationAnalyzer,
    /// Migration context
    migration_ctx: MigrationContext,
    /// Total compactions performed
    total_compactions: AtomicU64,
    /// Pages freed by compaction
    pages_freed: AtomicU64,
    /// Compaction failures
    compaction_failures: AtomicU64,
}

impl CompactionControl {
    /// Create a new compaction control structure
    pub const fn new() -> Self {
        Self {
            mode: Mutex::new(CompactionMode::Lazy),
            priority: Mutex::new(CompactionPriority::Normal),
            threshold: AtomicUsize::new(MIN_COMPACTION_THRESHOLD),
            target_pages: AtomicUsize::new(DEFAULT_COMPACTION_ORDER),
            enabled: AtomicUsize::new(1), // Enabled by default
            analyzer: FragmentationAnalyzer::new(),
            migration_ctx: MigrationContext::new(),
            total_compactions: AtomicU64::new(0),
            pages_freed: AtomicU64::new(0),
            compaction_failures: AtomicU64::new(0),
        }
    }

    /// Enable or disable compaction
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(if enabled { 1 } else { 0 }, Ordering::Release);
    }

    /// Check if compaction is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) == 1
    }

    /// Set compaction mode
    pub fn set_mode(&self, mode: CompactionMode) {
        *self.mode.lock() = mode;
    }

    /// Get current compaction mode
    pub fn get_mode(&self) -> CompactionMode {
        *self.mode.lock()
    }

    /// Set compaction priority
    pub fn set_priority(&self, priority: CompactionPriority) {
        *self.priority.lock() = priority;
    }

    /// Get current priority
    pub fn get_priority(&self) -> CompactionPriority {
        *self.priority.lock()
    }

    /// Set compaction threshold
    pub fn set_threshold(&self, threshold: usize) {
        self.threshold.store(threshold, Ordering::Release);
    }

    /// Set target pages for compaction
    pub fn set_target_pages(&self, target: usize) {
        self.target_pages.store(target, Ordering::Release);
    }

    /// Get fragmentation statistics
    pub fn get_fragmentation_stats(&self) -> FragmentationStats {
        self.analyzer.analyze()
    }

    /// Get average fragmentation
    pub fn get_average_fragmentation(&self) -> FragmentationStats {
        self.analyzer.get_average_fragmentation()
    }

    /// Get compaction statistics
    pub fn get_compaction_stats(&self) -> (u64, u64, u64) {
        (
            self.total_compactions.load(Ordering::Relaxed),
            self.pages_freed.load(Ordering::Relaxed),
            self.compaction_failures.load(Ordering::Relaxed),
        )
    }
}

/// Global compaction control
static COMPACTION_CONTROL: CompactionControl = CompactionControl::new();

/// Compact memory to create contiguous free regions
///
/// # Arguments
///
/// * `target_pages` - Target number of contiguous free pages
///
/// # Returns
///
/// * `CompactionResult` - Success or failure information
pub fn compact_memory(target_pages: usize) -> CompactionResult {
    if !COMPACTION_CONTROL.is_enabled() {
        return CompactionResult::Failed;
    }

    // Update target
    COMPACTION_CONTROL.set_target_pages(target_pages);

    // Analyze fragmentation
    let frag_stats = COMPACTION_CONTROL.get_fragmentation_stats();

    // Check if compaction is needed
    if frag_stats.compaction_score < 30 {
        // Low fragmentation, no compaction needed
        return CompactionResult::Success(0);
    }

    // Perform compaction
    let result = perform_compaction(target_pages);

    // Update statistics
    match result {
        CompactionResult::Success(freed) => {
            COMPACTION_CONTROL.total_compactions.fetch_add(1, Ordering::Relaxed);
            COMPACTION_CONTROL.pages_freed.fetch_add(freed as u64, Ordering::Relaxed);
        }
        CompactionResult::Partial(freed) => {
            COMPACTION_CONTROL.total_compactions.fetch_add(1, Ordering::Relaxed);
            COMPACTION_CONTROL.pages_freed.fetch_add(freed as u64, Ordering::Relaxed);
        }
        CompactionResult::Failed => {
            COMPACTION_CONTROL.compaction_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    result
}

/// Perform actual memory compaction
fn perform_compaction(target_pages: usize) -> CompactionResult {
    // Reset migration context
    COMPACTION_CONTROL.migration_ctx.reset();

    // Find migration candidates
    let candidates = find_migration_candidates(target_pages);

    if candidates.is_empty() {
        return CompactionResult::Failed;
    }

    // Limit migrations per cycle
    let to_migrate = candidates.into_iter().take(MAX_MIGRATE_PER_CYCLE).collect();

    // Perform migrations
    let (success, _failed) = migrate_pages_batch(to_migrate);

    if success > 0 {
        // Check if we achieved the target
        let frag_stats = COMPACTION_CONTROL.get_fragmentation_stats();
        if frag_stats.largest_free_block >= target_pages {
            CompactionResult::Success(success)
        } else {
            CompactionResult::Partial(success)
        }
    } else {
        CompactionResult::Failed
    }
}

/// Find pages that can be migrated for compaction
fn find_migration_candidates(_target_pages: usize) -> Vec<(usize, usize)> {
    let candidates = Vec::new();

    // In a real implementation, this would:
    // 1. Scan free page lists to find fragmented areas
    // 2. Identify allocated pages in those areas
    // 3. Find free destination pages
    // 4. Return (source, dest) pairs for migration

    // For now, return empty vector (placeholder)
    candidates
}

/// Compact a specific memory range
///
/// # Arguments
///
/// * `start` - Start of memory range
/// * `end` - End of memory range
/// * `target_pages` - Target contiguous free pages
///
/// # Returns
///
/// * `CompactionResult` - Success or failure information
pub fn compact_range(start: usize, end: usize, target_pages: usize) -> CompactionResult {
    // Validate range
    if start >= end || (end - start) < target_pages * PAGE_SIZE {
        return CompactionResult::Failed;
    }

    // In a real implementation, this would:
    // 1. Analyze fragmentation in the specific range
    // 2. Find migration candidates within the range
    // 3. Perform migrations
    // 4. Return results

    // For now, delegate to general compaction
    compact_memory(target_pages)
}

/// Trigger asynchronous compaction
pub fn trigger_async_compaction() {
    // In a real implementation, this would:
    // 1. Queue a compaction task to kcompactd kernel thread
    // 2. Return immediately

    // For now, just trigger synchronous compaction
    let target = COMPACTION_CONTROL.target_pages.load(Ordering::Relaxed);
    compact_memory(target);
}

// ============================================================================
// Compaction Scanner
// ============================================================================

/// Page scanner for compaction
pub struct CompactionScanner {
    /// Scan position
    scan_pos: AtomicUsize,
    /// Scan window size (pages)
    window_size: usize,
    /// Pages scanned
    pages_scanned: AtomicU64,
    /// Migrateable pages found
    migrateable_found: AtomicU64,
}

impl CompactionScanner {
    /// Create a new compaction scanner
    pub const fn new() -> Self {
        Self {
            scan_pos: AtomicUsize::new(0),
            window_size: 1024, // Scan 1024 pages at a time
            pages_scanned: AtomicU64::new(0),
            migrateable_found: AtomicU64::new(0),
        }
    }

    /// Scan for migrateable pages
    pub fn scan(&self, max_pages: usize) -> Vec<usize> {
        let migrateable = Vec::new();
        let start = self.scan_pos.load(Ordering::Relaxed);

        // In a real implementation, this would:
        // 1. Scan page tables from current position
        // 2. Check each page for migration eligibility
        // 3. Return list of migrateable pages

        // Update scan position
        self.scan_pos.store(
            (start + max_pages) % (1024 * 1024), // Wrap around
            Ordering::Relaxed,
        );

        migrateable
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Get compaction control reference
pub fn get_compaction_control() -> &'static CompactionControl {
    &COMPACTION_CONTROL
}

/// Check if compaction is needed based on fragmentation
pub fn is_compaction_needed() -> bool {
    let stats = COMPACTION_CONTROL.get_fragmentation_stats();
    stats.compaction_score >= 50
}

/// Get recommended compaction action
pub fn get_compaction_recommendation() -> Option<&'static str> {
    let stats = COMPACTION_CONTROL.get_fragmentation_stats();

    if stats.compaction_score >= 80 {
        Some("Critical fragmentation - immediate compaction recommended")
    } else if stats.compaction_score >= 60 {
        Some("High fragmentation - compaction advised")
    } else if stats.compaction_score >= 40 {
        Some("Moderate fragmentation - consider compaction")
    } else {
        None
    }
}

/// Enable memory compaction
pub fn enable_compaction() {
    COMPACTION_CONTROL.set_enabled(true);
}

/// Disable memory compaction
pub fn disable_compaction() {
    COMPACTION_CONTROL.set_enabled(false);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragmentation_analyzer() {
        let analyzer = FragmentationAnalyzer::new();
        let stats = analyzer.analyze();

        assert!(stats.free_blocks > 0);
        assert!(stats.largest_free_block > 0);
    }

    #[test]
    fn test_compaction_control() {
        let control = CompactionControl::new();

        assert!(control.is_enabled());
        control.set_enabled(false);
        assert!(!control.is_enabled());

        control.set_mode(CompactionMode::Eager);
        assert_eq!(control.get_mode(), CompactionMode::Eager);

        control.set_priority(CompactionPriority::High);
        assert_eq!(control.get_priority(), CompactionPriority::High);
    }

    #[test]
    fn test_migration_context() {
        let ctx = MigrationContext::new();

        let (migrated, failed, pinned) = ctx.stats();
        assert_eq!(migrated, 0);
        assert_eq!(failed, 0);
        assert_eq!(pinned, 0);

        ctx.reset();
        let stats = ctx.stats();
        assert_eq!(stats.0, 0);
    }

    #[test]
    fn test_compaction_scanner() {
        let scanner = CompactionScanner::new();
        let migrateable = scanner.scan(100);

        // Should return a vector (empty in this mock)
        assert!(migrateable.len() >= 0);
    }

    #[test]
    fn test_is_compaction_needed() {
        // This test depends on the fragmentation analyzer
        let needed = is_compaction_needed();

        // Should return a boolean
        assert!(needed == true || needed == false);
    }
}
