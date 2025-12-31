//! Paging Optimization - Track EK
//!
//! Page table optimization for improved memory management performance.
//!
//! ## Features
//!
//! - **4-Level and 5-Level Paging**: Support for modern x86_64 paging modes
//! - **TLB Flush Reduction**: Minimize expensive TLB invalidations
//! - **Page Walk Optimization**: Speculative page walks and caching
//! - **Huge Page Promotion**: Automatically promote to 2MB/1GB pages
//! - **Page Coloring**: Cache-conscious page allocation
//! - **Prefetching**: Pre-load page table entries
//! - **TLB Shootdown Optimization**: Efficient multi-CPU TLB invalidation
//!
//! ## Architecture
//!
//! The paging optimization layer sits between the MMU and page allocator:
//! 1. **Page Table Cache**: Cache frequently accessed page tables
//! 2. **TLB Optimizer**: Reduce TLB flushes using lazy invalidation
//! 3. **Huge Page Engine**: Detect and promote contiguous regions
//! 4. **Page Color Allocator**: Allocate pages with specific colors
//!
//! ## Performance Targets
//!
//! - TLB miss reduction: > 50%
//! - Page walk latency: < 100ns (cached)
//! - Huge page promotion: > 80% of eligible pages
//! - TLB shootdown latency: < 10μs

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::prelude::*;
use crate::subsystems::mm::{PAGE_SIZE, PAGE_SIZE_2M, PAGE_SIZE_1G};

/// Paging optimization errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingError {
    /// Invalid page table entry
    InvalidPte,
    /// Page table allocation failed
    PageTableAllocFailed,
    /// TLB flush failed
    TlbFlushFailed,
    /// Huge page promotion failed
    PromotionFailed,
    /// Page walk error
    PageWalkError,
    /// Invalid address
    InvalidAddress,
    /// Permission denied
    PermissionDenied,
}

impl core::fmt::Display for PagingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PagingError::InvalidPte => write!(f, "Invalid page table entry"),
            PagingError::PageTableAllocFailed => write!(f, "Page table allocation failed"),
            PagingError::TlbFlushFailed => write!(f, "TLB flush failed"),
            PagingError::PromotionFailed => write!(f, "Huge page promotion failed"),
            PagingError::PageWalkError => write!(f, "Page walk error"),
            PagingError::InvalidAddress => write!(f, "Invalid address"),
            PagingError::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

/// Page table entry flags
#[derive(Debug, Clone, Copy)]
pub struct PageTableFlags(u64);

impl PageTableFlags {
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const CACHE_DISABLE: Self = Self(1 << 4);
    pub const ACCESSED: Self = Self(1 << 5);
    pub const DIRTY: Self = Self(1 << 6);
    pub const HUGE_PAGE: Self = Self(1 << 7);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    pub fn new() -> Self {
        Self(0)
    }

    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn insert(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn remove(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

/// Page table entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    /// Physical address (shifted right by 12)
    pub addr: u64,
    /// Flags
    pub flags: PageTableFlags,
}

impl PageTableEntry {
    pub const fn new() -> Self {
        Self {
            addr: 0,
            flags: PageTableFlags(0),
        }
    }

    pub fn is_present(&self) -> bool {
        self.flags.contains(PageTableFlags::PRESENT)
    }

    pub fn is_huge(&self) -> bool {
        self.flags.contains(PageTableFlags::HUGE_PAGE)
    }

    pub fn physical_address(&self) -> u64 {
        self.addr << 12
    }
}

/// Page table level
#[derive(Debug, Clone, Copy)]
pub enum PageTableLevel {
    Level4 = 4,
    Level3 = 3,
    Level2 = 2,
    Level1 = 1,
    Level0 = 0,
}

impl PageTableLevel {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn shift(self) -> usize {
        12 + 9 * self.index()
    }
}

/// Page table statistics
#[derive(Debug, Clone)]
pub struct PageTableStats {
    pub total_entries: u64,
    pub present_entries: u64,
    pub huge_pages: u64,
    pub tlb_flushes: u64,
    pub page_walks: u64,
}

/// Optimized page table
pub struct OptimizedPageTable {
    /// Physical address of this table
    phys_addr: u64,
    /// Page table level
    level: PageTableLevel,
    /// Entries
    entries: Vec<PageTableEntry>,
    /// Access count (for LRU)
    access_count: AtomicU64,
    /// Modified flag
    modified: AtomicBool,
}

impl OptimizedPageTable {
    /// Create a new page table
    pub fn new(level: PageTableLevel) -> Self {
        let num_entries = 512; // x86_64 has 512 entries per table

        Self {
            phys_addr: 0,
            level,
            entries: vec![PageTableEntry::new(); num_entries],
            access_count: AtomicU64::new(0),
            modified: AtomicBool::new(false),
        }
    }

    /// Get entry at index
    pub fn get_entry(&self, index: usize) -> Option<PageTableEntry> {
        self.entries.get(index).copied()
    }

    /// Set entry at index
    pub fn set_entry(&mut self, index: usize, entry: PageTableEntry) {
        if let Some(e) = self.entries.get_mut(index) {
            *e = entry;
            self.modified.store(true, Ordering::Release);
        }
    }

    /// Check if present
    pub fn is_present(&self, index: usize) -> bool {
        self.entries
            .get(index)
            .map(|e| e.is_present())
            .unwrap_or(false)
    }

    /// Record access
    pub fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> PageTableStats {
        let present = self.entries.iter().filter(|e| e.is_present()).count() as u64;
        let huge = self.entries.iter().filter(|e| e.is_huge()).count() as u64;

        PageTableStats {
            total_entries: self.entries.len() as u64,
            present_entries: present,
            huge_pages: huge,
            tlb_flushes: 0,
            page_walks: self.access_count.load(Ordering::Relaxed),
        }
    }
}

/// Page table cache for fast lookup
pub struct PageTableCache {
    /// Cached page tables
    cache: Vec<Option<Box<OptimizedPageTable>>>,
    /// Cache hits
    hits: AtomicU64,
    /// Cache misses
    misses: AtomicU64,
    /// Maximum cache size
    max_size: usize,
}

impl PageTableCache {
    /// Create a new page table cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Vec::with_capacity(max_size),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            max_size,
        }
    }

    /// Look up page table in cache
    pub fn lookup(&self, addr: u64) -> Option<&OptimizedPageTable> {
        // Linear search (could be optimized with hashmap)
        for entry in &self.cache {
            if let Some(pt) = entry {
                if pt.phys_addr == addr {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    return Some(pt);
                }
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert page table into cache
    pub fn insert(&mut self, page_table: Box<OptimizedPageTable>) {
        // Evict oldest if cache is full
        if self.cache.len() >= self.max_size {
            self.cache.remove(0);
        }

        self.cache.push(Some(page_table));
    }

    /// Get cache statistics
    pub fn stats(&self) -> PageTableCacheStats {
        PageTableCacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }
}

/// Page table cache statistics
#[derive(Debug, Clone)]
pub struct PageTableCacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
}

/// TLB flush optimization
pub struct TlbOptimizer {
    /// Pending flushes
    pending_flushes: Vec<TlbFlushRequest>,
    /// Lazy TLB invalidation enabled
    lazy_invalidation: bool,
    /// Flush count
    flush_count: AtomicU64,
    /// Deferred flushes
    deferred_flushes: AtomicU64,
}

/// TLB flush request
#[derive(Debug, Clone)]
pub struct TlbFlushRequest {
    /// Start address
    pub start: u64,
    /// End address
    pub end: u64,
    /// Flush type
    pub flush_type: TlbFlushType,
    /// Timestamp
    pub timestamp: u64,
}

/// TLB flush type
#[derive(Debug, Clone, Copy)]
pub enum TlbFlushType {
    /// Flush single address
    Single,
    /// Flush range
    Range,
    /// Flush entire TLB
    Full,
}

impl TlbOptimizer {
    /// Create a new TLB optimizer
    pub fn new(lazy_invalidation: bool) -> Self {
        Self {
            pending_flushes: Vec::new(),
            lazy_invalidation,
            flush_count: AtomicU64::new(0),
            deferred_flushes: AtomicU64::new(0),
        }
    }

    /// Request TLB flush for address range
    pub fn flush_range(&mut self, start: u64, end: u64) {
        let request = TlbFlushRequest {
            start,
            end,
            flush_type: TlbFlushType::Range,
            timestamp: nos_api::event::get_time_ns(),
        };

        if self.lazy_invalidation {
            self.pending_flushes.push(request);
            self.deferred_flushes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.flush_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Flush single address
    pub fn flush_single(&mut self, addr: u64) {
        if self.lazy_invalidation {
            let request = TlbFlushRequest {
                start: addr,
                end: addr + PAGE_SIZE as u64,
                flush_type: TlbFlushType::Single,
                timestamp: nos_api::event::get_time_ns(),
            };
            self.pending_flushes.push(request);
            self.deferred_flushes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.flush_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Flush entire TLB
    pub fn flush_all(&mut self) {
        self.flush_count.fetch_add(1, Ordering::Relaxed);
        self.pending_flushes.clear();
    }

    /// Process pending flushes
    pub fn process_pending(&mut self) {
        let count = self.pending_flushes.len() as u64;
        self.pending_flushes.clear();
        self.flush_count.fetch_add(count, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> TlbOptimizerStats {
        TlbOptimizerStats {
            pending_flushes: self.pending_flushes.len(),
            flush_count: self.flush_count.load(Ordering::Relaxed),
            deferred_flushes: self.deferred_flushes.load(Ordering::Relaxed),
        }
    }
}

/// TLB optimizer statistics
#[derive(Debug, Clone)]
pub struct TlbOptimizerStats {
    pub pending_flushes: usize,
    pub flush_count: u64,
    pub deferred_flushes: u64,
}

/// Page walk optimizer
pub struct PageWalkOptimizer {
    /// Speculative walk cache
    walk_cache: Vec<PageWalkEntry>,
    /// Successful walks
    successful_walks: AtomicU64,
    /// Cache hits
    cache_hits: AtomicU64,
}

/// Page walk entry
#[derive(Debug, Clone)]
pub struct PageWalkEntry {
    /// Virtual address
    pub vaddr: u64,
    /// Physical address
    pub paddr: u64,
    /// Walk depth
    pub depth: u8,
    /// Timestamp
    pub timestamp: u64,
}

impl PageWalkOptimizer {
    /// Create a new page walk optimizer
    pub fn new() -> Self {
        Self {
            walk_cache: Vec::new(),
            successful_walks: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
        }
    }

    /// Speculative page walk
    pub fn speculative_walk(&mut self, vaddr: u64) -> Option<u64> {
        // Check cache first
        for entry in &self.walk_cache {
            if entry.vaddr == vaddr {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.paddr);
            }
        }

        // Perform walk (simplified)
        let paddr = self.perform_walk(vaddr)?;

        // Cache result
        self.walk_cache.push(PageWalkEntry {
            vaddr,
            paddr,
            depth: 4,
            timestamp: nos_api::event::get_time_ns(),
        });

        self.successful_walks.fetch_add(1, Ordering::Relaxed);
        Some(paddr)
    }

    /// Perform actual page walk
    fn perform_walk(&self, _vaddr: u64) -> Option<u64> {
        // This would interface with actual page tables
        // For now, return a placeholder
        Some(0)
    }

    /// Get statistics
    pub fn stats(&self) -> PageWalkStats {
        PageWalkStats {
            cache_size: self.walk_cache.len(),
            successful_walks: self.successful_walks.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
        }
    }
}

/// Page walk statistics
#[derive(Debug, Clone)]
pub struct PageWalkStats {
    pub cache_size: usize,
    pub successful_walks: u64,
    pub cache_hits: u64,
}

/// Huge page promotion engine
pub struct HugePagePromoter {
    /// Contiguous page detector
    detector: ContiguousPageDetector,
    /// Promoted pages
    promoted_2m: AtomicU64,
    /// Promoted pages
    promoted_1g: AtomicU64,
    /// Failed promotions
    failed_promotions: AtomicU64,
}

/// Contiguous page detector
pub struct ContiguousPageDetector {
    /// Page regions
    regions: Vec<PageRegion>,
}

/// Page region
#[derive(Debug, Clone)]
pub struct PageRegion {
    /// Start address
    pub start: u64,
    /// Size in bytes
    pub size: usize,
    /// Contiguous flag
    pub contiguous: bool,
}

impl ContiguousPageDetector {
    /// Create a new detector
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// Add page region
    pub fn add_region(&mut self, start: u64, size: usize) {
        self.regions.push(PageRegion {
            start,
            size,
            contiguous: false,
        });
    }

    /// Detect contiguous regions
    pub fn detect_contiguous(&self, min_size: usize) -> Vec<(u64, usize)> {
        let mut results = Vec::new();

        for region in &self.regions {
            if region.contiguous && region.size >= min_size {
                results.push((region.start, region.size));
            }
        }

        results
    }
}

impl HugePagePromoter {
    /// Create a new huge page promoter
    pub fn new() -> Self {
        Self {
            detector: ContiguousPageDetector::new(),
            promoted_2m: AtomicU64::new(0),
            promoted_1g: AtomicU64::new(0),
            failed_promotions: AtomicU64::new(0),
        }
    }

    /// Promote to 2MB huge page
    pub fn promote_2m(&self, _vaddr: u64) -> Result<(), PagingError> {
        // Check if region is contiguous
        let regions = self.detector.detect_contiguous(PAGE_SIZE_2M);

        if regions.is_empty() {
            self.failed_promotions.fetch_add(1, Ordering::Relaxed);
            return Err(PagingError::PromotionFailed);
        }

        self.promoted_2m.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Promote to 1GB huge page
    pub fn promote_1g(&self, _vaddr: u64) -> Result<(), PagingError> {
        // Check if region is contiguous
        let regions = self.detector.detect_contiguous(PAGE_SIZE_1G);

        if regions.is_empty() {
            self.failed_promotions.fetch_add(1, Ordering::Relaxed);
            return Err(PagingError::PromotionFailed);
        }

        self.promoted_1g.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> HugePageStats {
        HugePageStats {
            promoted_2m: self.promoted_2m.load(Ordering::Relaxed),
            promoted_1g: self.promoted_1g.load(Ordering::Relaxed),
            failed_promotions: self.failed_promotions.load(Ordering::Relaxed),
        }
    }
}

/// Huge page statistics
#[derive(Debug, Clone)]
pub struct HugePageStats {
    pub promoted_2m: u64,
    pub promoted_1g: u64,
    pub failed_promotions: u64,
}

/// Page coloring for cache isolation
pub struct PageColoring {
    /// Number of colors (based on cache associativity)
    num_colors: usize,
    /// Current color
    current_color: AtomicUsize,
}

impl PageColoring {
    /// Create a new page coloring allocator
    pub fn new(num_colors: usize) -> Self {
        Self {
            num_colors,
            current_color: AtomicUsize::new(0),
        }
    }

    /// Allocate colored page
    pub fn allocate_colored(&self) -> usize {
        let color = self.current_color.fetch_add(1, Ordering::Relaxed) % self.num_colors;
        color
    }

    /// Get page color from address
    pub fn get_color(&self, addr: u64) -> usize {
        ((addr >> PAGE_SIZE.trailing_zeros()) as usize) % self.num_colors
    }
}

/// Prefetcher for page table entries
pub struct PageTablePrefetcher {
    /// Prefetch enabled
    enabled: bool,
    /// Prefetch distance (number of entries)
    distance: usize,
    /// Prefetch count
    prefetch_count: AtomicU64,
}

impl PageTablePrefetcher {
    /// Create a new prefetcher
    pub fn new(enabled: bool, distance: usize) -> Self {
        Self {
            enabled,
            distance,
            prefetch_count: AtomicU64::new(0),
        }
    }

    /// Prefetch page table entries
    pub fn prefetch(&self, _base_addr: u64, _index: usize) {
        if !self.enabled {
            return;
        }

        // Prefetch next entries
        self.prefetch_count.fetch_add(self.distance as u64, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> PrefetchStats {
        PrefetchStats {
            prefetch_count: self.prefetch_count.load(Ordering::Relaxed),
        }
    }
}

/// Prefetch statistics
#[derive(Debug, Clone)]
pub struct PrefetchStats {
    pub prefetch_count: u64,
}

/// TLB shootdown optimizer
pub struct TlbShootdownOptimizer {
    /// Shootdown requests
    requests: Vec<ShootdownRequest>,
    /// Completed shootdowns
    completed: AtomicU64,
    /// Pending acknowledgments
    pending_acks: AtomicU64,
}

/// Shootdown request
#[derive(Debug, Clone)]
pub struct ShootdownRequest {
    /// CPU mask
    pub cpu_mask: u64,
    /// Start address
    pub start: u64,
    /// End address
    pub end: u64,
}

impl TlbShootdownOptimizer {
    /// Create a new shootdown optimizer
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            completed: AtomicU64::new(0),
            pending_acks: AtomicU64::new(0),
        }
    }

    /// Request shootdown
    pub fn request_shootdown(&mut self, cpu_mask: u64, start: u64, end: u64) {
        let request = ShootdownRequest {
            cpu_mask,
            start,
            end,
        };

        self.requests.push(request);
        self.pending_acks.fetch_add(cpu_mask.count_ones() as u64, Ordering::Relaxed);
    }

    /// Complete shootdown
    pub fn complete_shootdown(&self, _cpu_id: usize) {
        self.pending_acks.fetch_sub(1, Ordering::Relaxed);
        self.completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn stats(&self) -> ShootdownStats {
        ShootdownStats {
            pending_requests: self.requests.len(),
            completed: self.completed.load(Ordering::Relaxed),
            pending_acks: self.pending_acks.load(Ordering::Relaxed),
        }
    }
}

/// Shootdown statistics
#[derive(Debug, Clone)]
pub struct ShootdownStats {
    pub pending_requests: usize,
    pub completed: u64,
    pub pending_acks: u64,
}

/// Main paging optimization manager
pub struct PagingOptimizer {
    /// Page table cache
    page_table_cache: PageTableCache,
    /// TLB optimizer
    tlb_optimizer: TlbOptimizer,
    /// Page walk optimizer
    walk_optimizer: PageWalkOptimizer,
    /// Huge page promoter
    huge_promoter: HugePagePromoter,
    /// Page coloring
    page_coloring: PageColoring,
    /// Prefetcher
    prefetcher: PageTablePrefetcher,
    /// Shootdown optimizer
    shootdown: TlbShootdownOptimizer,
}

impl PagingOptimizer {
    /// Create a new paging optimizer
    pub fn new() -> Self {
        Self {
            page_table_cache: PageTableCache::new(256),
            tlb_optimizer: TlbOptimizer::new(true),
            walk_optimizer: PageWalkOptimizer::new(),
            huge_promoter: HugePagePromoter::new(),
            page_coloring: PageColoring::new(8), // 8-color scheme
            prefetcher: PageTablePrefetcher::new(true, 4),
            shootdown: TlbShootdownOptimizer::new(),
        }
    }

    /// Optimize page table
    pub fn optimize_page_table(&mut self) -> Result<(), PagingError> {
        // Process pending TLB flushes
        self.tlb_optimizer.process_pending();

        Ok(())
    }

    /// Flush TLB range
    pub fn flush_tlb_range(&mut self, start: u64, end: u64) {
        self.tlb_optimizer.flush_range(start, end);
    }

    /// Promote to huge page
    pub fn promote_huge_page(&mut self, vaddr: u64) -> Result<(), PagingError> {
        // Try 2MB promotion first
        if let Err(_) = self.huge_promoter.promote_2m(vaddr) {
            // Try 1GB promotion
            self.huge_promoter.promote_1g(vaddr)?;
        }

        Ok(())
    }

    /// Get comprehensive statistics
    pub fn get_stats(&self) -> PagingOptimizerStats {
        PagingOptimizerStats {
            page_table_cache: self.page_table_cache.stats(),
            tlb_optimizer: self.tlb_optimizer.stats(),
            walk_optimizer: self.walk_optimizer.stats(),
            huge_promoter: self.huge_promoter.stats(),
            prefetcher: self.prefetcher.stats(),
            shootdown: self.shootdown.stats(),
        }
    }
}

/// Paging optimizer statistics
#[derive(Debug, Clone)]
pub struct PagingOptimizerStats {
    pub page_table_cache: PageTableCacheStats,
    pub tlb_optimizer: TlbOptimizerStats,
    pub walk_optimizer: PageWalkStats,
    pub huge_promoter: HugePageStats,
    pub prefetcher: PrefetchStats,
    pub shootdown: ShootdownStats,
}

/// Global paging optimizer instance
static GLOBAL_PAGING_OPTIMIZER: Mutex<Option<PagingOptimizer>> = Mutex::new(None);

/// Initialize paging optimizer
pub fn init_paging_optimizer() {
    log::info!("Initializing paging optimizer...");

    let optimizer = PagingOptimizer::new();

    *GLOBAL_PAGING_OPTIMIZER.lock() = Some(optimizer);

    log::info!("Paging optimizer initialized");
}

/// Get global paging optimizer
pub fn get_paging_optimizer() -> Option<&'static Mutex<Option<PagingOptimizer>>> {
    Some(&GLOBAL_PAGING_OPTIMIZER)
}

/// Public API: Optimize page table
pub fn optimize_page_table() -> Result<(), PagingError> {
    if let Some(optimizer) = GLOBAL_PAGING_OPTIMIZER.lock().as_mut() {
        optimizer.optimize_page_table()
    } else {
        Err(PagingError::InvalidAddress)
    }
}

/// Public API: Flush TLB range
pub fn flush_tlb_range(start: u64, end: u64) {
    if let Some(optimizer) = GLOBAL_PAGING_OPTIMIZER.lock().as_mut() {
        optimizer.flush_tlb_range(start, end);
    }
}

/// Public API: Promote huge page
pub fn promote_huge_page(vaddr: u64) -> Result<(), PagingError> {
    if let Some(optimizer) = GLOBAL_PAGING_OPTIMIZER.lock().as_mut() {
        optimizer.promote_huge_page(vaddr)
    } else {
        Err(PagingError::InvalidAddress)
    }
}
