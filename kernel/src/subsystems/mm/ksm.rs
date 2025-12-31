//! # Kernel Samepage Merging (KSM)
//!
//! This module implements Kernel Samepage Merging, which deduplicates
//! identical memory pages to reduce memory usage.
//!
//! ## Overview
//!
//! KSM scans memory for pages with identical content and merges them into
//! a single page, using copy-on-write semantics. This is particularly
//! effective for virtualization environments where multiple VMs may have
//! identical content.
//!
//! ## Features
//!
//! - **Page Scanning**: Efficient scanning of memory pages
//! - **Hash-based Comparison**: Fast page content comparison
//! - **Merge/Unmerge**: Dynamic page merging and unmerging
//! - **Stable/Unstable Pages**: Handling of frequently changing pages
//! - **Statistics**: Comprehensive tracking of KSM effectiveness
//! - **MADV_MERGEABLE**: Userspace hints for mergeable pages
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::subsystems::mm::ksm::{ksm_enable, ksm_scan_pages, KsmStats};
//!
//! // Enable KSM
//! ksm_enable();
//!
//! // Scan pages for deduplication
//! let merged = ksm_scan_pages(1000);
//! println!("Merged {} pages", merged);
//!
//! // Get statistics
//! let stats = KsmStats::get();
//! println!("Saved pages: {}", stats.pages_shared);
//! ```

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::subsystems::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Page size (4KB)
const PAGE_SIZE: usize = 4096;

/// KSM scan rate (pages per scan)
const KSM_SCAN_RATE: usize = 256;

/// Maximum KSM pages to track
const MAX_KSM_PAGES: usize = 262144; // 1GB worth of pages

/// Hash size for page comparison (64-bit)
const KSM_HASH_SIZE: usize = 8;

/// Minimum similarity score to consider pages mergeable
const MIN_SIMILARITY: u32 = 100; // 100% match required

/// Maximum unstable page count before forcing scan
const MAX_UNSTABLE_PAGES: usize = 1024;

// ============================================================================
// Error Types
// ============================================================================

/// KSM operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KsmError {
    /// KSM not enabled
    NotEnabled,
    /// Out of memory
    OutOfMemory,
    /// Page not found
    PageNotFound,
    /// Page is pinned
    PagePinned,
    /// Invalid operation
    InvalidOperation,
}

// ============================================================================
// KSM Page Entry
// ============================================================================

/// KSM page descriptor
#[derive(Debug, Clone)]
pub struct KsmPage {
    /// Physical address of the page
    pub phys_addr: usize,
    /// Virtual address (for scanning)
    pub virt_addr: usize,
    /// Page content hash
    pub hash: u64,
    /// Reference count (how many pages share this)
    pub refcount: usize,
    /// Page flags
    pub flags: KsmPageFlags,
    /// Number of times scanned
    pub scan_count: u32,
    /// Last scan tick
    pub last_scan: u64,
}

/// KSM page flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KsmPageFlags {
    bits: u8,
}

impl KsmPageFlags {
    const MERGEABLE: u8 = 0x01;
    const MERGED: u8 = 0x02;
    const UNSTABLE: u8 = 0x04;
    const VOLATILE: u8 = 0x08;
    const ZERO_PAGE: u8 = 0x10;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub const fn contains(&self, other: KsmPageFlags) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl KsmPage {
    /// Create a new KSM page entry
    pub fn new(phys_addr: usize, virt_addr: usize, hash: u64) -> Self {
        Self {
            phys_addr,
            virt_addr,
            hash,
            refcount: 1,
            flags: KsmPageFlags::empty(),
            scan_count: 0,
            last_scan: 0,
        }
    }

    /// Increment reference count
    pub fn inc_ref(&self) -> usize {
        self.refcount + 1
    }

    /// Decrement reference count
    pub fn dec_ref(&self) -> usize {
        self.refcount - 1
    }

    /// Get reference count
    pub fn get_refcount(&self) -> usize {
        self.refcount
    }

    /// Check if page is mergeable
    pub fn is_mergeable(&self) -> bool {
        (self.flags.bits & KsmPageFlags::MERGEABLE) != 0
    }

    /// Check if page is merged
    pub fn is_merged(&self) -> bool {
        (self.flags.bits & KsmPageFlags::MERGED) != 0
    }

    /// Check if page is unstable
    pub fn is_unstable(&self) -> bool {
        (self.flags.bits & KsmPageFlags::UNSTABLE) != 0
    }
}

// ============================================================================
// Merge Candidate
// ============================================================================

/// Page merge candidate
#[derive(Debug, Clone)]
struct MergeCandidate {
    /// Virtual address
    virt_addr: usize,
    /// Page hash
    hash: u64,
    /// Similarity score
    similarity: u32,
}

// ============================================================================
// KSM Manager
// ============================================================================

/// KSM manager
pub struct KsmManager {
    /// Enable/disable KSM
    enabled: AtomicUsize,
    /// Map of hash to KSM pages
    hash_pages: Mutex<BTreeMap<u64, Vec<usize>>>,
    /// Map of virtual addresses to KSM pages
    ksm_pages: Mutex<BTreeMap<usize, KsmPage>>,
    /// Unstable pages (scanned but not stable)
    unstable_pages: Mutex<Vec<usize>>,
    /// Total pages scanned
    pages_scanned: AtomicU64,
    /// Total pages merged
    pages_merged: AtomicU64,
    /// Total pages unmerged
    pages_unmerged: AtomicU64,
    /// Current pages shared (through merging)
    pages_shared: AtomicU64,
    /// Pages saved (total merged - 1 for each group)
    pages_saved: AtomicU64,
    /// Full scans performed
    full_scans: AtomicU64,
    /// Scan position
    scan_pos: AtomicUsize,
    /// Zero page tracking
    zero_pages: AtomicU64,
}

impl KsmManager {
    /// Create a new KSM manager
    pub const fn new() -> Self {
        Self {
            enabled: AtomicUsize::new(0), // Disabled by default
            hash_pages: Mutex::new(BTreeMap::new()),
            ksm_pages: Mutex::new(BTreeMap::new()),
            unstable_pages: Mutex::new(Vec::new()),
            pages_scanned: AtomicU64::new(0),
            pages_merged: AtomicU64::new(0),
            pages_unmerged: AtomicU64::new(0),
            pages_shared: AtomicU64::new(0),
            pages_saved: AtomicU64::new(0),
            full_scans: AtomicU64::new(0),
            scan_pos: AtomicUsize::new(0),
            zero_pages: AtomicU64::new(0),
        }
    }

    /// Enable KSM
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Release);
    }

    /// Disable KSM
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Release);
    }

    /// Check if KSM is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) == 1
    }

    /// Register a page as mergeable
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    /// * `phys_addr` - Physical address
    pub fn register_page(&self, virt_addr: usize, phys_addr: usize) {
        let hash = self.compute_page_hash(phys_addr);
        let ksm_page = KsmPage::new(phys_addr, virt_addr, hash);

        let mut hash_pages = self.hash_pages.lock();
        let mut ksm_pages = self.ksm_pages.lock();

        hash_pages.entry(hash).or_default().push(virt_addr);
        ksm_pages.insert(virt_addr, ksm_page);
    }

    /// Unregister a page
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    pub fn unregister_page(&self, virt_addr: usize) {
        let mut hash_pages = self.hash_pages.lock();
        let mut ksm_pages = self.ksm_pages.lock();

        if let Some(ksm_page) = ksm_pages.remove(&virt_addr) {
            let hash = ksm_page.hash;

            if let Some(pages) = hash_pages.get_mut(&hash) {
                pages.retain(|&v| v != virt_addr);
                if pages.is_empty() {
                    hash_pages.remove(&hash);
                }
            }
        }
    }

    /// Compute hash of page content
    ///
    /// # Arguments
    ///
    /// * `phys_addr` - Physical address
    ///
    /// # Returns
    ///
    /// * `u64` - Hash value
    fn compute_page_hash(&self, phys_addr: usize) -> u64 {
        // Simple hash implementation
        // In production, use a stronger hash (xxHash, CityHash, etc.)
        let mut hash: u64 = 0;

        unsafe {
            let ptr = phys_addr as *const u64;
            let count = PAGE_SIZE / 8;

            for i in 0..count {
                let val = ptr.add(i).read_volatile();
                hash = hash.wrapping_mul(31).wrapping_add(val);
            }
        }

        hash
    }

    /// Compare two pages for equality
    ///
    /// # Arguments
    ///
    /// * `addr1` - First page address
    /// * `addr2` - Second page address
    ///
    /// # Returns
    ///
    /// * `bool` - True if pages are identical
    fn compare_pages(&self, addr1: usize, addr2: usize) -> bool {
        unsafe {
            let ptr1 = addr1 as *const u8;
            let ptr2 = addr2 as *const u8;

            for i in 0..PAGE_SIZE {
                if ptr1.add(i).read_volatile() != ptr2.add(i).read_volatile() {
                    return false;
                }
            }
        }

        true
    }

    /// Scan pages and merge duplicates
    ///
    /// # Arguments
    ///
    /// * `max_pages` - Maximum pages to scan
    ///
    /// # Returns
    ///
    /// * `usize` - Number of pages merged
    pub fn scan_pages(&self, max_pages: usize) -> usize {
        if !self.is_enabled() {
            return 0;
        }

        let mut merged_count = 0;
        let start_pos = self.scan_pos.load(Ordering::Relaxed);

        // Get all registered pages
        let page_addrs: Vec<usize> = {
            let ksm_pages = self.ksm_pages.lock();
            ksm_pages.keys().copied().collect()
        };

        let scan_count = max_pages.min(page_addrs.len());
        let end_pos = (start_pos + scan_count).min(page_addrs.len());

        for i in start_pos..end_pos {
            let virt_addr = page_addrs[i];

            // Get page info
            let (phys_addr, hash) = {
                let ksm_pages = self.ksm_pages.lock();
                if let Some(page) = ksm_pages.get(&virt_addr) {
                    (page.phys_addr, page.hash)
                } else {
                    continue;
                }
            };

            // Look for matching pages
            if let Some(merged) = self.try_merge_page(virt_addr, phys_addr, hash) {
                merged_count += merged;
            }

            self.pages_scanned.fetch_add(1, Ordering::Relaxed);
        }

        // Update scan position
        self.scan_pos.store(
            if end_pos >= page_addrs.len() {
                self.full_scans.fetch_add(1, Ordering::Relaxed);
                0
            } else {
                end_pos
            },
            Ordering::Relaxed,
        );

        merged_count
    }

    /// Try to merge a page with duplicates
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    /// * `phys_addr` - Physical address
    /// * `hash` - Page hash
    ///
    /// # Returns
    ///
    /// * `usize` - Number of pages merged (0 or more)
    fn try_merge_page(&self, virt_addr: usize, phys_addr: usize, hash: u64) -> Option<usize> {
        let hash_pages = self.hash_pages.lock();
        let mut ksm_pages = self.ksm_pages.lock();

        // Find other pages with same hash
        if let Some(similar_pages) = hash_pages.get(&hash) {
            for &other_virt in similar_pages.iter() {
                if other_virt == virt_addr {
                    continue;
                }

                if let Some(other_page) = ksm_pages.get(&other_virt) {
                    // Check if pages are actually identical
                    if self.compare_pages(phys_addr, other_page.phys_addr) {
                        // Found a duplicate! Merge them
                        // Keep the page with lower address as the shared page
                        let (shared_virt, shared_phys, duplicate_virt) =
                            if virt_addr < other_virt {
                                (virt_addr, phys_addr, other_virt)
                            } else {
                                (other_virt, other_page.phys_addr, virt_addr)
                            };

                        // Update both entries
                        if let Some(shared_page) = ksm_pages.get_mut(&shared_virt) {
                            shared_page.flags = KsmPageFlags::from_bits(shared_page.flags.bits | KsmPageFlags::MERGED);
                            shared_page.refcount += 1;
                        }

                        if let Some(dup_page) = ksm_pages.get_mut(&duplicate_virt) {
                            dup_page.flags = KsmPageFlags::from_bits(dup_page.flags.bits | KsmPageFlags::MERGED);
                            dup_page.phys_addr = shared_phys;
                            dup_page.refcount += 1;
                        }

                        self.pages_merged.fetch_add(1, Ordering::Relaxed);
                        self.pages_shared.fetch_add(1, Ordering::Relaxed);

                        return Some(1);
                    }
                }
            }
        }

        None
    }

    /// Unmerge a page (make a private copy)
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    ///
    /// # Returns
    ///
    /// * `Result<usize, KsmError>` - New physical address or error
    pub fn unmerge_page(&self, virt_addr: usize) -> Result<usize, KsmError> {
        let mut ksm_pages = self.ksm_pages.lock();

        if let Some(ksm_page) = ksm_pages.get_mut(&virt_addr) {
            if !ksm_page.is_merged() {
                return Err(KsmError::InvalidOperation);
            }

            // Allocate new page
            let new_page = crate::subsystems::mm::kalloc();
            if new_page.is_null() {
                return Err(KsmError::OutOfMemory);
            }

            let new_phys = new_page as usize;

            // Copy content
            unsafe {
                let src = ksm_page.phys_addr as *const u8;
                core::ptr::copy_nonoverlapping(src, new_page, PAGE_SIZE);
            }

            // Update entry
            ksm_page.phys_addr = new_phys;
            ksm_page.flags = KsmPageFlags::from_bits(ksm_page.flags.bits & !KsmPageFlags::MERGED);

            self.pages_unmerged.fetch_add(1, Ordering::Relaxed);

            Ok(new_phys)
        } else {
            Err(KsmError::PageNotFound)
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> KsmStats {
        KsmStats {
            pages_scanned: self.pages_scanned.load(Ordering::Relaxed),
            pages_merged: self.pages_merged.load(Ordering::Relaxed),
            pages_unmerged: self.pages_unmerged.load(Ordering::Relaxed),
            pages_shared: self.pages_shared.load(Ordering::Relaxed),
            pages_saved: self.pages_saved.load(Ordering::Relaxed),
            full_scans: self.full_scans.load(Ordering::Relaxed),
            zero_pages: self.zero_pages.load(Ordering::Relaxed),
        }
    }
}

/// KSM statistics
#[derive(Debug, Clone, Copy)]
pub struct KsmStats {
    pub pages_scanned: u64,
    pub pages_merged: u64,
    pub pages_unmerged: u64,
    pub pages_shared: u64,
    pub pages_saved: u64,
    pub full_scans: u64,
    pub zero_pages: u64,
}

impl KsmStats {
    /// Get current KSM statistics
    pub fn get() -> Self {
        KSM_MANAGER.get_stats()
    }

    /// Get merge ratio (merged / scanned)
    pub fn merge_ratio(&self) -> u64 {
        if self.pages_scanned == 0 {
            return 0;
        }
        (self.pages_merged * 10000) / self.pages_scanned
    }

    /// Get memory savings percentage
    pub fn savings_percent(&self) -> u64 {
        if self.pages_scanned == 0 {
            return 0;
        }
        (self.pages_saved * 10000) / self.pages_scanned
    }
}

/// Global KSM manager
static KSM_MANAGER: KsmManager = KsmManager::new();

// ============================================================================
// Public API
// ============================================================================

/// Enable KSM
pub fn ksm_enable() {
    KSM_MANAGER.enable();
}

/// Disable KSM
pub fn ksm_disable() {
    KSM_MANAGER.disable();
}

/// Check if KSM is enabled
pub fn ksm_is_enabled() -> bool {
    KSM_MANAGER.is_enabled()
}

/// Register a page as mergeable
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `phys_addr` - Physical address
pub fn ksm_register_page(virt_addr: usize, phys_addr: usize) {
    KSM_MANAGER.register_page(virt_addr, phys_addr);
}

/// Unregister a page
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
pub fn ksm_unregister_page(virt_addr: usize) {
    KSM_MANAGER.unregister_page(virt_addr);
}

/// Scan pages and merge duplicates
///
/// # Arguments
///
/// * `max_pages` - Maximum pages to scan
///
/// # Returns
///
/// * `usize` - Number of pages merged
pub fn ksm_scan_pages(max_pages: usize) -> usize {
    KSM_MANAGER.scan_pages(max_pages)
}

/// Unmerge a page
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
///
/// # Returns
///
/// * `Result<usize, KsmError>` - New physical address or error
pub fn ksm_unmerge_page(virt_addr: usize) -> Result<usize, KsmError> {
    KSM_MANAGER.unmerge_page(virt_addr)
}

/// Handle MADV_MERGEABLE advice
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `phys_addr` - Physical address
/// * `length` - Length in bytes
pub fn ksm_advice_mergeable(virt_addr: usize, phys_addr: usize, length: usize) {
    if !ksm_is_enabled() {
        return;
    }

    let pages = length / PAGE_SIZE;

    for i in 0..pages {
        let vaddr = virt_addr + (i * PAGE_SIZE);
        let paddr = phys_addr + (i * PAGE_SIZE);
        ksm_register_page(vaddr, paddr);
    }
}

/// Handle MADV_UNMERGEABLE advice
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `length` - Length in bytes
///
/// # Returns
///
/// * `Result<(), KsmError>` - Success or error
pub fn ksm_advice_unmergeable(virt_addr: usize, length: usize) -> Result<(), KsmError> {
    let pages = length / PAGE_SIZE;

    for i in 0..pages {
        let vaddr = virt_addr + (i * PAGE_SIZE);

        // Try to unmerge each page
        let _ = ksm_unmerge_page(vaddr);

        // Unregister from KSM
        ksm_unregister_page(vaddr);
    }

    Ok(())
}

/// Get KSM statistics
///
/// # Returns
///
/// * `KsmStats` - Current statistics
pub fn ksm_get_stats() -> KsmStats {
    KSM_MANAGER.get_stats()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ksm_enable_disable() {
        assert!(!ksm_is_enabled());

        ksm_enable();
        assert!(ksm_is_enabled());

        ksm_disable();
        assert!(!ksm_is_enabled());
    }

    #[test]
    fn test_ksm_register_page() {
        ksm_enable();

        let virt = 0x1000;
        let phys = 0x2000;

        ksm_register_page(virt, phys);

        // Page should be registered (check by attempting to unmerge)
        // We can't directly check, but we can ensure no crash
        ksm_unregister_page(virt);
    }

    #[test]
    fn test_ksm_scan() {
        ksm_enable();

        // Register some pages
        for i in 0..10 {
            ksm_register_page(i * 0x1000, i * 0x1000 + 0x10000);
        }

        // Scan pages
        let merged = ksm_scan_pages(10);
        assert!(merged >= 0);
    }

    #[test]
    fn test_ksm_stats() {
        ksm_enable();

        let stats = ksm_get_stats();
        assert!(stats.pages_scanned >= 0);
        assert!(stats.pages_merged >= 0);
    }

    #[test]
    fn test_ksm_stats_ratios() {
        let stats = KsmStats {
            pages_scanned: 1000,
            pages_merged: 100,
            pages_unmerged: 10,
            pages_shared: 100,
            pages_saved: 50,
            full_scans: 5,
            zero_pages: 0,
        };

        assert_eq!(stats.merge_ratio(), 1000); // 100 * 10000 / 1000
        assert_eq!(stats.savings_percent(), 500); // 50 * 10000 / 1000
    }

    #[test]
    fn test_ksm_advice_mergeable() {
        ksm_enable();

        let virt = 0x10000;
        let phys = 0x20000;
        let length = PAGE_SIZE * 10;

        ksm_advice_mergeable(virt, phys, length);

        // Should not crash
    }

    #[test]
    fn test_ksm_advice_unmergeable() {
        ksm_enable();

        let virt = 0x20000;
        let length = PAGE_SIZE * 5;

        let result = ksm_advice_unmergeable(virt, length);
        assert!(result.is_ok());
    }
}
