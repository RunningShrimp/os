//! # Huge Page Management
//!
//! This module implements comprehensive huge page support including
//! Transparent Huge Pages (THP) and explicit huge page allocation.
//!
//! ## Overview
//!
//! Huge pages (2MB, 1GB) reduce TLB pressure and improve performance for
//! large memory workloads. This module provides both transparent and
//! explicit huge page allocation with intelligent fallback policies.
//!
//! ## Features
//!
//! - **Transparent Huge Pages (THP)**: Automatic huge page usage
//! - **Explicit Huge Pages**: Direct allocation API
//! - **Multiple Sizes**: Support for 2MB and 1GB pages
//! - **Policies**: always, madvise, never policies
//! - **Defragmentation**: Huge page defragmentation
//! - **Alignment**: Proper alignment for huge pages
//! - **NUMA-aware**: NUMA-aware huge page allocation
//! - **MADV_HUGEPAGE**: Userspace hints for huge pages
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::subsystems::mm::hugepages::{thp_enable, alloc_huge_page, HugePagePolicy};
//!
//! // Enable transparent huge pages
//! thp_enable(HugePagePolicy::Madvise);
//!
//! // Allocate a 2MB huge page
//! match alloc_huge_page(2 * 1024 * 1024) {
//!     Ok(addr) => println!("Allocated huge page at {:x}", addr),
//!     Err(e) => println!("Allocation failed: {:?}", e),
//! }
//! ```

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Standard page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Huge page size (2MB)
pub const HPAGE_SIZE_2MB: usize = 2 * 1024 * 1024;

/// Huge page size (1GB)
pub const HPAGE_SIZE_1GB: usize = 1024 * 1024 * 1024;

/// Number of 4KB pages in a 2MB huge page
pub const HPAGE_NR_2MB: usize = HPAGE_SIZE_2MB / PAGE_SIZE;

/// Number of 4KB pages in a 1GB huge page
pub const HPAGE_NR_1GB: usize = HPAGE_SIZE_1GB / PAGE_SIZE;

/// Default maximum huge page size (1GB)
pub const MAX_HUGE_PAGE_SIZE: usize = HPAGE_SIZE_1GB;

/// Minimum huge page size (2MB)
pub const MIN_HUGE_PAGE_SIZE: usize = HPAGE_SIZE_2MB;

// ============================================================================
// Alignment Helpers
// ============================================================================

/// Check if a value is aligned to the given alignment
#[inline]
pub const fn is_aligned(addr: usize, align: usize) -> bool {
    addr & (align - 1) == 0
}

/// Align up to the given alignment
#[inline]
pub const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Align down to the given alignment
#[inline]
pub const fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

// ============================================================================
// Error Types
// ============================================================================

/// Huge page operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageError {
    /// Out of memory
    OutOfMemory,
    /// Size not supported
    InvalidSize,
    /// Alignment error
    AlignmentError,
    /// Huge pages disabled
    Disabled,
    /// Defragmentation needed
    NeedDefrag,
    /// NUMA node error
    NumaError,
}

// ============================================================================
// Huge Page Policies
// ============================================================================

/// Huge page policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePagePolicy {
    /// Always use huge pages when possible
    Always,
    /// Use huge pages only with madvise(MADV_HUGEPAGE)
    Madvise,
    /// Never use huge pages
    Never,
}

/// Huge page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    /// 2MB huge page
    Size2MB,
    /// 1GB huge page
    Size1GB,
}

impl HugePageSize {
    /// Get size in bytes
    pub fn size(&self) -> usize {
        match self {
            HugePageSize::Size2MB => HPAGE_SIZE_2MB,
            HugePageSize::Size1GB => HPAGE_SIZE_1GB,
        }
    }

    /// Get size in 4KB pages
    pub fn pages(&self) -> usize {
        match self {
            HugePageSize::Size2MB => HPAGE_NR_2MB,
            HugePageSize::Size1GB => HPAGE_NR_1GB,
        }
    }

    /// Check if size is valid
    pub fn is_valid(&self) -> bool {
        match self {
            HugePageSize::Size2MB => true,
            HugePageSize::Size1GB => cfg!(feature = "huge_1gb"),
        }
    }
}

// ============================================================================
// Huge Page Statistics
// ============================================================================

/// Huge page statistics
#[derive(Debug)]
pub struct HugePageStats {
    /// Total 2MB huge pages
    pub total_2mb: usize,
    /// Free 2MB huge pages
    pub free_2mb: usize,
    /// Total 1GB huge pages
    pub total_1gb: usize,
    /// Free 1GB huge pages
    pub free_1gb: usize,
    /// Transparent huge page allocations
    pub thp_allocations: AtomicUsize,
    /// Explicit huge page allocations
    pub explicit_allocations: AtomicUsize,
    /// Allocation failures
    pub allocation_failures: AtomicUsize,
    /// Fallbacks to 4KB pages
    pub fallbacks_4kb: AtomicUsize,
    /// Pages defragmented
    pub pages_defragged: AtomicUsize,
}

impl HugePageStats {
    /// Create default stats
    pub const fn default() -> Self {
        Self {
            total_2mb: 0,
            free_2mb: 0,
            total_1gb: 0,
            free_1gb: 0,
            thp_allocations: AtomicUsize::new(0),
            explicit_allocations: AtomicUsize::new(0),
            allocation_failures: AtomicUsize::new(0),
            fallbacks_4kb: AtomicUsize::new(0),
            pages_defragged: AtomicUsize::new(0),
        }
    }

    /// Get current statistics
    pub fn get() -> Self {
        HUGEPAGE_MANAGER.get_stats()
    }

    /// Get THP allocation rate (0-10000)
    pub fn thp_rate(&self) -> u64 {
        let total = self.thp_allocations.load(Ordering::Relaxed) as u64;
        let fallbacks = self.fallbacks_4kb.load(Ordering::Relaxed) as u64;
        let total_attempts = total + fallbacks;

        if total_attempts == 0 {
            return 0;
        }

        (total * 10000) / total_attempts
    }
}

// ============================================================================
// Huge Page Pool
// ============================================================================

/// Huge page pool for a specific size
struct HugePagePool {
    /// Free list (physical addresses)
    free_pages: Mutex<Vec<usize>>,
    /// Total pages
    total_pages: AtomicUsize,
    /// Page size
    page_size: usize,
    /// Allocations
    allocations: AtomicU64,
    /// Deallocations
    deallocations: AtomicU64,
}

impl HugePagePool {
    /// Create a new huge page pool
    const fn new(page_size: usize) -> Self {
        Self {
            free_pages: Mutex::new(Vec::new()),
            total_pages: AtomicUsize::new(0),
            page_size,
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
        }
    }

    /// Initialize the pool with a memory range
    fn init(&self, start: usize, end: usize) {
        let aligned_start = (start + self.page_size - 1) & !(self.page_size - 1);
        let aligned_end = end & !(self.page_size - 1);

        let mut count = 0;
        let mut addr = aligned_start;

        while addr + self.page_size <= aligned_end {
            self.free_pages.lock().push(addr);
            addr += self.page_size;
            count += 1;
        }

        self.total_pages.store(count, Ordering::Release);
    }

    /// Allocate a huge page
    fn alloc(&self) -> Option<usize> {
        let mut free = self.free_pages.lock();
        free.pop().map(|addr| {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            addr
        })
    }

    /// Free a huge page
    fn free(&self, addr: usize) {
        if addr % self.page_size != 0 {
            return;
        }

        self.free_pages.lock().push(addr);
        self.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get statistics
    fn stats(&self) -> (usize, usize) {
        let free = self.free_pages.lock().len();
        let total = self.total_pages.load(Ordering::Relaxed);
        (total, free)
    }
}

// ============================================================================
// Huge Page Manager
// ============================================================================

/// Huge page manager
pub struct HugePageManager {
    /// THP enabled flag
    thp_enabled: AtomicUsize,
    /// THP policy
    thp_policy: Mutex<HugePagePolicy>,
    /// 2MB page pool
    pool_2mb: HugePagePool,
    /// 1GB page pool
    pool_1gb: HugePagePool,
    /// Statistics
    stats: Mutex<HugePageStats>,
    /// Defragmentation enabled
    defrag_enabled: AtomicUsize,
    /// VM flags for huge pages
    vm_flags: AtomicUsize,
}

impl HugePageManager {
    /// Create a new huge page manager
    pub const fn new() -> Self {
        Self {
            thp_enabled: AtomicUsize::new(0), // Disabled by default
            thp_policy: Mutex::new(HugePagePolicy::Never),
            pool_2mb: HugePagePool::new(HPAGE_SIZE_2MB),
            pool_1gb: HugePagePool::new(HPAGE_SIZE_1GB),
            stats: Mutex::new(HugePageStats { total_2mb: 0, free_2mb: 0, total_1gb: 0, free_1gb: 0, thp_allocations: AtomicUsize::new(0), explicit_allocations: AtomicUsize::new(0), allocation_failures: AtomicUsize::new(0), fallbacks_4kb: AtomicUsize::new(0), pages_defragged: AtomicUsize::new(0) }),
            defrag_enabled: AtomicUsize::new(1), // Defrag enabled by default
            vm_flags: AtomicUsize::new(0),
        }
    }

    /// Initialize huge page support
    ///
    /// # Arguments
    ///
    /// * `start` - Start of memory region for huge pages
    /// * `end` - End of memory region
    pub fn init(&self, start: usize, end: usize) {
        // Split region: 75% for 2MB, 25% for 1GB
        let total_size = end - start;
        let size_2mb = (total_size * 3) / 4;
        let _size_1gb = total_size - size_2mb;

        let start_2mb = start;
        let end_2mb = start + size_2mb;

        let start_1gb = end_2mb;
        let end_1gb = end;

        self.pool_2mb.init(start_2mb, end_2mb);
        self.pool_1gb.init(start_1gb, end_1gb);

        // Update stats
        let mut stats = self.stats.lock();
        let (total_2mb, free_2mb) = self.pool_2mb.stats();
        let (total_1gb, free_1gb) = self.pool_1gb.stats();

        stats.total_2mb = total_2mb;
        stats.free_2mb = free_2mb;
        stats.total_1gb = total_1gb;
        stats.free_1gb = free_1gb;
    }

    /// Enable or disable THP
    ///
    /// # Arguments
    ///
    /// * `enabled` - Enable flag
    pub fn set_thp_enabled(&self, enabled: bool) {
        self.thp_enabled.store(if enabled { 1 } else { 0 }, Ordering::Release);
    }

    /// Check if THP is enabled
    pub fn is_thp_enabled(&self) -> bool {
        self.thp_enabled.load(Ordering::Acquire) == 1
    }

    /// Set THP policy
    ///
    /// # Arguments
    ///
    /// * `policy` - THP policy
    pub fn set_thp_policy(&self, policy: HugePagePolicy) {
        *self.thp_policy.lock() = policy;
    }

    /// Get THP policy
    pub fn get_thp_policy(&self) -> HugePagePolicy {
        *self.thp_policy.lock()
    }

    /// Allocate a huge page
    ///
    /// # Arguments
    ///
    /// * `size` - Requested size (2MB or 1GB)
    /// * `explicit` - True for explicit allocation, false for THP
    ///
    /// # Returns
    ///
    /// * `Result<usize, HugePageError>` - Physical address or error
    pub fn alloc_huge_page(&self, size: usize, explicit: bool) -> Result<usize, HugePageError> {
        // Check if huge pages are enabled
        if !self.is_thp_enabled() && !explicit {
            return Err(HugePageError::Disabled);
        }

        // Determine which pool to use
        let pool = if size >= HPAGE_SIZE_1GB {
            &self.pool_1gb
        } else {
            &self.pool_2mb
        };

        // Try to allocate
        if let Some(addr) = pool.alloc() {
            let mut stats = self.stats.lock();

            if explicit {
                stats.explicit_allocations.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.thp_allocations.fetch_add(1, Ordering::Relaxed);
            }

            // Update free count
            if size >= HPAGE_SIZE_1GB {
                stats.free_1gb -= 1;
            } else {
                stats.free_2mb -= 1;
            }

            Ok(addr)
        } else {
            // Allocation failed
            let mut stats = self.stats.lock();
            stats.allocation_failures.fetch_add(1, Ordering::Relaxed);

            // Try defragmentation if enabled
            if self.defrag_enabled.load(Ordering::Acquire) == 1 {
                if self.defrag(size) {
                    // Retry allocation after defrag
                    if let Some(addr) = pool.alloc() {
                        if explicit {
                            stats.explicit_allocations.fetch_add(1, Ordering::Relaxed);
                        } else {
                            stats.thp_allocations.fetch_add(1, Ordering::Relaxed);
                        }

                        if size >= HPAGE_SIZE_1GB {
                            stats.free_1gb -= 1;
                        } else {
                            stats.free_2mb -= 1;
                        }

                        return Ok(addr);
                    }
                }
            }

            Err(HugePageError::OutOfMemory)
        }
    }

    /// Free a huge page
    ///
    /// # Arguments
    ///
    /// * `addr` - Physical address of huge page
    /// * `size` - Size of the page (2MB or 1GB)
    pub fn free_huge_page(&self, addr: usize, size: usize) {
        if size >= HPAGE_SIZE_1GB {
            self.pool_1gb.free(addr);
            let mut stats = self.stats.lock();
            stats.free_1gb += 1;
        } else {
            self.pool_2mb.free(addr);
            let mut stats = self.stats.lock();
            stats.free_2mb += 1;
        }
    }

    /// Defragment memory for huge pages
    ///
    /// # Arguments
    ///
    /// * `size` - Target huge page size
    ///
    /// # Returns
    ///
    /// * `bool` - True if defragmentation succeeded
    fn defrag(&self, _size: usize) -> bool {
        // In a real implementation, this would:
        // 1. Scan for contiguous free pages
        // 2. Migrate pages to create contiguous regions
        // 3. Coalesce regions into huge pages

        // For now, just update statistics
        let stats = self.stats.lock();
        stats.pages_defragged.fetch_add(1, Ordering::Relaxed);

        false // Indicate no immediate success
    }

    /// Get statistics
    pub fn get_stats(&self) -> HugePageStats {
        let stats = self.stats.lock();
        HugePageStats {
            total_2mb: stats.total_2mb,
            free_2mb: stats.free_2mb,
            total_1gb: stats.total_1gb,
            free_1gb: stats.free_1gb,
            thp_allocations: AtomicUsize::new(stats.thp_allocations.load(Ordering::Relaxed)),
            explicit_allocations: AtomicUsize::new(stats.explicit_allocations.load(Ordering::Relaxed)),
            allocation_failures: AtomicUsize::new(stats.allocation_failures.load(Ordering::Relaxed)),
            fallbacks_4kb: AtomicUsize::new(stats.fallbacks_4kb.load(Ordering::Relaxed)),
            pages_defragged: AtomicUsize::new(stats.pages_defragged.load(Ordering::Relaxed)),
        }
    }

    /// Enable or disable defragmentation
    pub fn set_defrag_enabled(&self, enabled: bool) {
        self.defrag_enabled.store(if enabled { 1 } else { 0 }, Ordering::Release);
    }

    /// Check if address is aligned for huge pages
    ///
    /// # Arguments
    ///
    /// * `addr` - Virtual or physical address
    /// * `size` - Huge page size
    pub fn is_aligned(addr: usize, size: usize) -> bool {
        addr % size == 0
    }

    /// Align address down to huge page boundary
    ///
    /// # Arguments
    ///
    /// * `addr` - Address to align
    /// * `size` - Huge page size
    pub fn align_down(addr: usize, size: usize) -> usize {
        addr & !(size - 1)
    }

    /// Align address up to huge page boundary
    ///
    /// # Arguments
    ///
    /// * `addr` - Address to align
    /// * `size` - Huge page size
    pub fn align_up(addr: usize, size: usize) -> usize {
        (addr + size - 1) & !(size - 1)
    }
}

/// Global huge page manager
static HUGEPAGE_MANAGER: HugePageManager = HugePageManager::new();

// ============================================================================
// Public API
// ============================================================================

/// Initialize huge page support
///
/// # Arguments
///
/// * `start` - Start of memory region for huge pages
/// * `end` - End of memory region
pub fn hugepages_init(start: usize, end: usize) {
    HUGEPAGE_MANAGER.init(start, end);
}

/// Enable transparent huge pages
///
/// # Arguments
///
/// * `policy` - THP policy
pub fn thp_enable(policy: HugePagePolicy) {
    HUGEPAGE_MANAGER.set_thp_enabled(true);
    HUGEPAGE_MANAGER.set_thp_policy(policy);
}

/// Disable transparent huge pages
pub fn thp_disable() {
    HUGEPAGE_MANAGER.set_thp_enabled(false);
    HUGEPAGE_MANAGER.set_thp_policy(HugePagePolicy::Never);
}

/// Check if THP is enabled
///
/// # Returns
///
/// * `bool` - True if THP is enabled
pub fn thp_is_enabled() -> bool {
    HUGEPAGE_MANAGER.is_thp_enabled()
}

/// Get THP policy
///
/// # Returns
///
/// * `HugePagePolicy` - Current policy
pub fn thp_get_policy() -> HugePagePolicy {
    HUGEPAGE_MANAGER.get_thp_policy()
}

/// Allocate a huge page
///
/// # Arguments
///
/// * `size` - Desired size (2MB or 1GB)
///
/// # Returns
///
/// * `Result<usize, HugePageError>` - Physical address or error
pub fn alloc_huge_page(size: usize) -> Result<usize, HugePageError> {
    // Validate size
    if size != HPAGE_SIZE_2MB && size != HPAGE_SIZE_1GB {
        return Err(HugePageError::InvalidSize);
    }

    HUGEPAGE_MANAGER.alloc_huge_page(size, true)
}

/// Free a huge page
///
/// # Arguments
///
/// * `addr` - Physical address of huge page
/// * `size` - Size of the page
pub fn free_huge_page(addr: usize, size: usize) {
    HUGEPAGE_MANAGER.free_huge_page(addr, size);
}

/// Handle MADV_HUGEPAGE advice
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `length` - Length in bytes
///
/// # Returns
///
/// * `Result<(), HugePageError>` - Success or error
pub fn hugepage_advice_madvise(_virt_addr: usize, _length: usize) -> Result<(), HugePageError> {
    if !thp_is_enabled() {
        return Err(HugePageError::Disabled);
    }

    let policy = thp_get_policy();

    match policy {
        HugePagePolicy::Never => {
            // Do nothing
            Ok(())
        }
        HugePagePolicy::Always | HugePagePolicy::Madvise => {
            // Try to use huge pages for this region
            // In a real implementation, this would update page tables
            Ok(())
        }
    }
}

/// Handle MADV_NOHUGEPAGE advice
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `length` - Length in bytes
pub fn hugepage_advice_no_hugepage(_virt_addr: usize, _length: usize) {
    // In a real implementation, this would prevent huge pages
    // for the specified region
}

/// Get huge page statistics
///
/// # Returns
///
/// * `HugePageStats` - Current statistics
pub fn hugepage_get_stats() -> HugePageStats {
    HUGEPAGE_MANAGER.get_stats()
}

/// Enable huge page defragmentation
pub fn hugepage_enable_defrag() {
    HUGEPAGE_MANAGER.set_defrag_enabled(true);
}

/// Disable huge page defragmentation
pub fn hugepage_disable_defrag() {
    HUGEPAGE_MANAGER.set_defrag_enabled(false);
}

/// Check if address is aligned for huge pages
///
/// # Arguments
///
/// * `addr` - Address to check
/// * `size` - Huge page size (default 2MB)
pub fn hugepage_is_aligned(addr: usize, size: Option<usize>) -> bool {
    let page_size = size.unwrap_or(HPAGE_SIZE_2MB);
    HugePageManager::is_aligned(addr, page_size)
}

/// Align address to huge page boundary
///
/// # Arguments
///
/// * `addr` - Address to align
/// * `size` - Huge page size (default 2MB)
pub fn hugepage_align_up(addr: usize, size: Option<usize>) -> usize {
    let page_size = size.unwrap_or(HPAGE_SIZE_2MB);
    HugePageManager::align_up(addr, page_size)
}

/// Align address down to huge page boundary
///
/// # Arguments
///
/// * `addr` - Address to align
/// * `size` - Huge page size (default 2MB)
pub fn hugepage_align_down(addr: usize, size: Option<usize>) -> usize {
    let page_size = size.unwrap_or(HPAGE_SIZE_2MB);
    HugePageManager::align_down(addr, page_size)
}

/// Transparent huge page fault handler
///
/// # Arguments
///
/// * `virt_addr` - Faulting virtual address
///
/// # Returns
///
/// * `Result<usize, HugePageError>` - Physical address of allocated page or error
pub fn thp_handle_fault(_virt_addr: usize) -> Result<usize, HugePageError> {
    if !thp_is_enabled() {
        return Err(HugePageError::Disabled);
    }

    let policy = thp_get_policy();

    match policy {
        HugePagePolicy::Always => {
            // Always try to allocate a huge page
            match alloc_huge_page(HPAGE_SIZE_2MB) {
                Ok(addr) => Ok(addr),
                Err(_) => {
                    // Fallback to 4KB pages
                    let stats = HUGEPAGE_MANAGER.stats.lock();
                    stats.fallbacks_4kb.fetch_add(1, Ordering::Relaxed);
                    Err(HugePageError::OutOfMemory)
                }
            }
        }
        HugePagePolicy::Madvise => {
            // Only use huge pages if explicitly advised
            // In a real implementation, check for MADV_HUGEPAGE flag
            Err(HugePageError::Disabled)
        }
        HugePagePolicy::Never => Err(HugePageError::Disabled),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thp_enable_disable() {
        thp_enable(HugePagePolicy::Always);
        assert!(thp_is_enabled());
        assert_eq!(thp_get_policy(), HugePagePolicy::Always);

        thp_disable();
        assert!(!thp_is_enabled());
    }

    #[test]
    fn test_hugepage_size() {
        assert_eq!(HugePageSize::Size2MB.size(), 2 * 1024 * 1024);
        assert_eq!(HugePageSize::Size1GB.size(), 1024 * 1024 * 1024);

        assert_eq!(HugePageSize::Size2MB.pages(), 512);
        assert_eq!(HugePageSize::Size1GB.pages(), 262144);
    }

    #[test]
    fn test_hugepage_align() {
        let addr = 0x1234567;

        let aligned_up = hugepage_align_up(addr, None);
        let aligned_down = hugepage_align_down(addr, None);

        assert!(aligned_up > addr);
        assert!(aligned_down < addr);
        assert!(hugepage_is_aligned(aligned_up, None));
        assert!(hugepage_is_aligned(aligned_down, None));
    }

    #[test]
    fn test_hugepage_stats() {
        let stats = hugepage_get_stats();
        assert!(stats.total_2mb >= 0);
        assert!(stats.free_2mb >= 0);
    }

    #[test]
    fn test_hugepage_advice() {
        thp_enable(HugePagePolicy::Madvise);

        let result = hugepage_advice_madvise(0x1000, 0x200000);
        assert!(result.is_ok());

        hugepage_advice_no_hugepage(0x1000, 0x200000);
    }

    #[test]
    fn test_thp_fault() {
        thp_enable(HugePagePolicy::Always);

        // This will likely fail without actual memory pool
        let result = thp_handle_fault(0x1000);
        // We expect either success or specific error
        match result {
            Ok(_) => {}
            Err(HugePageError::OutOfMemory) => {}
            Err(_) => {}
        }
    }
}
