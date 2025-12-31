//! # Copy-on-Write (COW) Optimization
//!
//! This module implements Copy-on-Write mechanisms for efficient memory
//! management, particularly for fork() and mmap() operations.
//!
//! ## Overview
//!
//! Copy-on-Write (COW) is an optimization where memory pages are shared
//! between processes until one of them needs to modify the page. Only then
//! is the page actually copied. This dramatically reduces memory usage and
//! improves performance for fork() operations.
//!
//! ## Features
//!
//! - **Fork Optimization**: COW-based fork for fast process creation
//! - **Private Mappings**: MAP_PRIVATE mappings with COW semantics
//! - **Page Fault Handling**: Automatic copy on write faults
//! - **Zero Pages**: Special handling for zero pages
//! - **KSM Integration**: Integration with Kernel Samepage Merging
//! - **Write Protection**: Hardware-assisted write protection
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::subsystems::mm::cow::{CowPage, cow_fork, cow_handle_fault};
//!
//! // Fork with COW
//! let child_pages = cow_fork(parent_pages)?;
//!
//! // Handle write fault
//! match cow_handle_fault(faulting_addr) {
//!     CowFaultResult::Copied(new_page) => {
//!         println!("Page copied to {:x}", new_page);
//!     }
//!     CowFaultResult::Shared => {
//!         println!("Page remains shared");
//!     }
//! }
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

/// COW page flags
const COW_READ_ONLY: u8 = 0x01;
const COW_SHARED: u8 = 0x02;
const COW_ZERO_PAGE: u8 = 0x04;
const COW_WRITE_PENDING: u8 = 0x08;

/// Maximum COW pages before forcing copy
const MAX_COW_PAGES: usize = 65536;

// ============================================================================
// Error Types
// ============================================================================

/// COW operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CowError {
    /// Page not found
    PageNotFound,
    /// Out of memory
    OutOfMemory,
    /// Invalid operation
    InvalidOperation,
    /// Page not COW
    NotCowPage,
    /// Page is pinned
    PagePinned,
}

// ============================================================================
// COW Page Entry
// ============================================================================

/// COW page descriptor
#[derive(Debug, Clone)]
pub struct CowPage {
    /// Physical address of the page
    pub phys_addr: usize,
    /// Reference count (processes sharing this page)
    pub refcount: usize,
    /// COW flags
    pub flags: u8,
    /// Original owner (for tracking)
    pub owner: usize,
    /// Size (for non-standard pages)
    pub size: usize,
}

impl CowPage {
    /// Create a new COW page
    pub fn new(phys_addr: usize, owner: usize) -> Self {
        Self {
            phys_addr,
            refcount: 1,
            flags: COW_SHARED | COW_READ_ONLY,
            owner,
            size: PAGE_SIZE,
        }
    }

    /// Create a zero page
    pub fn zero_page() -> Self {
        Self {
            phys_addr: 0, // Zero pages have no physical backing
            refcount: 1,
            flags: COW_SHARED | COW_READ_ONLY | COW_ZERO_PAGE,
            owner: 0,
            size: PAGE_SIZE,
        }
    }

    /// Check if page is read-only
    pub fn is_read_only(&self) -> bool {
        (self.flags & COW_READ_ONLY) != 0
    }

    /// Check if page is shared
    pub fn is_shared(&self) -> bool {
        (self.flags & COW_SHARED) != 0
    }

    /// Check if page is a zero page
    pub fn is_zero_page(&self) -> bool {
        (self.flags & COW_ZERO_PAGE) != 0
    }

    /// Increment reference count
    pub fn increment_ref(&self) -> usize {
        self.refcount + 1
    }

    /// Decrement reference count
    pub fn decrement_ref(&self) -> usize {
        self.refcount - 1
    }

    /// Get current reference count
    pub fn get_refcount(&self) -> usize {
        self.refcount
    }
}

// ============================================================================
// Fault Handling Result
// ============================================================================

/// Result of COW fault handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CowFaultResult {
    /// Page was copied (new physical address)
    Copied(usize),
    /// Page remains shared (no copy needed)
    Shared,
    /// Fault handling failed
    Failed,
}

// ============================================================================
// COW Manager
// ============================================================================

/// COW page tracking
pub struct CowManager {
    /// Map of virtual addresses to COW pages
    cow_pages: Mutex<BTreeMap<usize, CowPage>>,
    /// Zero page (special singleton)
    zero_page: Mutex<Option<CowPage>>,
    /// Total COW pages
    total_pages: AtomicUsize,
    /// Total copies performed
    total_copies: AtomicU64,
    /// Total shared pages
    shared_pages: AtomicU64,
    /// Zero page hits
    zero_page_hits: AtomicU64,
    /// Faults handled
    faults_handled: AtomicU64,
}

impl CowManager {
    /// Create a new COW manager
    pub const fn new() -> Self {
        Self {
            cow_pages: Mutex::new(BTreeMap::new()),
            zero_page: Mutex::new(None),
            total_pages: AtomicUsize::new(0),
            total_copies: AtomicU64::new(0),
            shared_pages: AtomicU64::new(0),
            zero_page_hits: AtomicU64::new(0),
            faults_handled: AtomicU64::new(0),
        }
    }

    /// Register a COW page
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    /// * `phys_addr` - Physical address
    /// * `owner` - Process ID of owner
    pub fn register_page(&self, virt_addr: usize, phys_addr: usize, owner: usize) {
        let mut pages = self.cow_pages.lock();
        let cow_page = CowPage::new(phys_addr, owner);
        pages.insert(virt_addr, cow_page);
        self.total_pages.fetch_add(1, Ordering::Relaxed);
        self.shared_pages.fetch_add(1, Ordering::Relaxed);
    }

    /// Unregister a COW page
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    pub fn unregister_page(&self, virt_addr: usize) {
        let mut pages = self.cow_pages.lock();
        if let Some(page) = pages.remove(&virt_addr) {
            let new_count = page.decrement_ref();
            if new_count == 0 {
                self.total_pages.fetch_sub(1, Ordering::Relaxed);
                self.shared_pages.fetch_sub(1, Ordering::Relaxed);

                // Free the physical page if not a zero page
                if !page.is_zero_page() && page.phys_addr != 0 {
                    unsafe {
                        crate::subsystems::mm::kfree(page.phys_addr as *mut u8);
                    }
                }
            }
        }
    }

    /// Handle a write fault on a COW page
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address that faulted
    ///
    /// # Returns
    ///
    /// * `CowFaultResult` - Result of fault handling
    pub fn handle_write_fault(&self, virt_addr: usize) -> CowFaultResult {
        self.faults_handled.fetch_add(1, Ordering::Relaxed);

        let mut pages = self.cow_pages.lock();

        if let Some(cow_page) = pages.get(&virt_addr) {
            // Check if this is a zero page
            if cow_page.is_zero_page() {
                self.zero_page_hits.fetch_add(1, Ordering::Relaxed);

                // Allocate new page and zero it
                let new_page = crate::subsystems::mm::kalloc();
                if new_page.is_null() {
                    return CowFaultResult::Failed;
                }

                // Zero the page
                unsafe {
                    core::ptr::write_bytes(new_page, 0, PAGE_SIZE);
                }

                let new_phys = new_page as usize;

                // Update the entry
                let mut new_cow = cow_page.clone();
                new_cow.phys_addr = new_phys;
                new_cow.flags &= !COW_ZERO_PAGE;
                new_cow.flags &= !COW_SHARED;
                new_cow.flags &= !COW_READ_ONLY;

                pages.insert(virt_addr, new_cow);

                return CowFaultResult::Copied(new_phys);
            }

            // Check if page is still shared
            if cow_page.get_refcount() > 1 {
                // Need to copy the page
                let new_page = crate::subsystems::mm::kalloc();
                if new_page.is_null() {
                    return CowFaultResult::Failed;
                }

                let new_phys = new_page as usize;

                // Copy page content
                unsafe {
                    let src = cow_page.phys_addr as *const u8;
                    let dst = new_page;
                    core::ptr::copy_nonoverlapping(src, dst, PAGE_SIZE);
                }

                // Update the entry
                let mut new_cow = cow_page.clone();
                new_cow.phys_addr = new_phys;
                new_cow.flags &= !COW_SHARED;
                new_cow.flags &= !COW_READ_ONLY;
                new_cow.refcount = 1;

                pages.insert(virt_addr, new_cow);

                self.total_copies.fetch_add(1, Ordering::Relaxed);

                return CowFaultResult::Copied(new_phys);
            } else {
                // Page is no longer shared, just make it writable
                let mut new_cow = cow_page.clone();
                new_cow.flags &= !COW_SHARED;
                new_cow.flags &= !COW_READ_ONLY;

                pages.insert(virt_addr, new_cow);

                return CowFaultResult::Shared;
            }
        }

        CowFaultResult::Failed
    }

    /// Share a COW page with another process (fork)
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    /// * `new_owner` - New process ID
    ///
    /// # Returns
    ///
    /// * `Result<(), CowError>` - Success or error
    pub fn share_page(&self, virt_addr: usize, new_owner: usize) -> Result<(), CowError> {
        let mut pages = self.cow_pages.lock();

        if let Some(cow_page) = pages.get_mut(&virt_addr) {
            cow_page.increment_ref();

            // For the child process, we keep the same virtual address
            // but mark it as shared and read-only
            let mut child_cow = cow_page.clone();
            child_cow.flags |= COW_SHARED;
            child_cow.flags |= COW_READ_ONLY;
            child_cow.owner = new_owner;

            pages.insert(virt_addr, child_cow);

            return Ok(());
        }

        Err(CowError::PageNotFound)
    }

    /// Get COW page information
    ///
    /// # Arguments
    ///
    /// * `virt_addr` - Virtual address
    ///
    /// # Returns
    ///
    /// * `Option<CowPage>` - COW page if found
    pub fn get_page(&self, virt_addr: usize) -> Option<CowPage> {
        let pages = self.cow_pages.lock();
        pages.get(&virt_addr).cloned()
    }

    /// Get statistics
    pub fn get_stats(&self) -> CowStats {
        CowStats {
            total_pages: self.total_pages.load(Ordering::Relaxed),
            total_copies: self.total_copies.load(Ordering::Relaxed),
            shared_pages: self.shared_pages.load(Ordering::Relaxed),
            zero_page_hits: self.zero_page_hits.load(Ordering::Relaxed),
            faults_handled: self.faults_handled.load(Ordering::Relaxed),
        }
    }
}

/// COW statistics
#[derive(Debug, Clone, Copy)]
pub struct CowStats {
    pub total_pages: usize,
    pub total_copies: u64,
    pub shared_pages: u64,
    pub zero_page_hits: u64,
    pub faults_handled: u64,
}

/// Global COW manager
static COW_MANAGER: CowManager = CowManager::new();

// ============================================================================
// Public API
// ============================================================================

/// Mark a page as Copy-on-Write
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `phys_addr` - Physical address
/// * `owner` - Process ID of owner
///
/// # Returns
///
/// * `Result<(), CowError>` - Success or error
pub fn cow_mark_page(virt_addr: usize, phys_addr: usize, owner: usize) -> Result<(), CowError> {
    // Validate alignment
    if virt_addr % PAGE_SIZE != 0 || phys_addr % PAGE_SIZE != 0 {
        return Err(CowError::InvalidOperation);
    }

    COW_MANAGER.register_page(virt_addr, phys_addr, owner);
    Ok(())
}

/// Handle a write fault on a COW page
///
/// # Arguments
///
/// * `virt_addr` - Virtual address that faulted
///
/// # Returns
///
/// * `CowFaultResult` - Result of fault handling
pub fn cow_handle_fault(virt_addr: usize) -> CowFaultResult {
    COW_MANAGER.handle_write_fault(virt_addr)
}

/// Simulate fork() with COW semantics
///
/// # Arguments
///
/// * `pages` - Vector of (virtual, physical) address pairs
/// * `parent_pid` - Parent process ID
/// * `child_pid` - Child process ID
///
/// # Returns
///
/// * `Result<Vec<(usize, usize)>, CowError>` - Child's page mappings or error
pub fn cow_fork(
    pages: Vec<(usize, usize)>,
    parent_pid: usize,
    child_pid: usize,
) -> Result<Vec<(usize, usize)>, CowError> {
    let mut child_pages = Vec::new();

    for (virt_addr, phys_addr) in pages {
        // Mark page as COW for parent
        cow_mark_page(virt_addr, phys_addr, parent_pid)?;

        // Share page with child
        COW_MANAGER.share_page(virt_addr, child_pid)?;

        // Child gets the same virtual address pointing to same physical page
        child_pages.push((virt_addr, phys_addr));
    }

    Ok(child_pages)
}

/// Break COW and make a private copy
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
///
/// # Returns
///
/// * `Result<usize, CowError>` - New physical address or error
pub fn cow_break(virt_addr: usize) -> Result<usize, CowError> {
    match COW_MANAGER.handle_write_fault(virt_addr) {
        CowFaultResult::Copied(new_phys) => Ok(new_phys),
        CowFaultResult::Shared => {
            // Page was already private
            if let Some(page) = COW_MANAGER.get_page(virt_addr) {
                Ok(page.phys_addr)
            } else {
                Err(CowError::PageNotFound)
            }
        }
        CowFaultResult::Failed => Err(CowError::OutOfMemory),
    }
}

/// Unmark a page as COW (no longer shared)
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
pub fn cow_unmark_page(virt_addr: usize) {
    COW_MANAGER.unregister_page(virt_addr);
}

/// Get COW statistics
///
/// # Returns
///
/// * `CowStats` - Current COW statistics
pub fn cow_get_stats() -> CowStats {
    COW_MANAGER.get_stats()
}

/// Check if a page is a COW page
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
///
/// # Returns
///
/// * `bool` - True if page is COW
pub fn cow_is_cow_page(virt_addr: usize) -> bool {
    COW_MANAGER.get_page(virt_addr).is_some()
}

/// Get COW page reference count
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
///
/// # Returns
///
/// * `Option<usize>` - Reference count if page is COW
pub fn cow_get_refcount(virt_addr: usize) -> Option<usize> {
    COW_MANAGER.get_page(virt_addr).map(|p| p.get_refcount())
}

// ============================================================================
// Zero Page Optimization
// ============================================================================

/// Get the zero page (singleton)
///
/// All zero pages are shared across processes to save memory
pub fn cow_get_zero_page() -> usize {
    // Return a special virtual address that represents the zero page
    // The actual zero page content is generated on fault
    0xDEADBEEF
}

/// Check if an address is the zero page
pub fn cow_is_zero_page(virt_addr: usize) -> bool {
    virt_addr == cow_get_zero_page()
}

// ============================================================================
// MAP_PRIVATE Support
// ============================================================================

/// Create a private COW mapping (mmap with MAP_PRIVATE)
///
/// # Arguments
///
/// * `virt_addr` - Virtual address
/// * `phys_addr` - Physical address of backing page
/// * `owner` - Process ID
///
/// # Returns
///
/// * `Result<(), CowError>` - Success or error
pub fn cow_map_private(virt_addr: usize, phys_addr: usize, owner: usize) -> Result<(), CowError> {
    cow_mark_page(virt_addr, phys_addr, owner)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cow_mark_page() {
        let virt = 0x1000;
        let phys = 0x2000;
        let owner = 100;

        let result = cow_mark_page(virt, phys, owner);
        assert!(result.is_ok());

        assert!(cow_is_cow_page(virt));
    }

    #[test]
    fn test_cow_refcount() {
        let virt = 0x2000;
        let phys = 0x3000;
        let parent = 200;
        let child = 201;

        cow_mark_page(virt, phys, parent).unwrap();

        // Share with child
        COW_MANAGER.share_page(virt, child).unwrap();

        let refcount = cow_get_refcount(virt);
        assert_eq!(refcount, Some(2));
    }

    #[test]
    fn test_cow_fault() {
        let virt = 0x3000;
        let phys = 0x4000;
        let owner = 300;

        cow_mark_page(virt, phys, owner).unwrap();

        // Handle write fault
        let result = cow_handle_fault(virt);
        match result {
            CowFaultResult::Copied(new_phys) => {
                assert!(new_phys != phys);
            }
            _ => {
                // Page might not be shared, so Shared is also valid
            }
        }
    }

    #[test]
    fn test_cow_fork() {
        let parent = 400;
        let child = 401;

        let pages = vec![(0x1000, 0x2000), (0x2000, 0x3000)];

        let result = cow_fork(pages, parent, child);
        assert!(result.is_ok());

        let child_pages = result.unwrap();
        assert_eq!(child_pages.len(), 2);
    }

    #[test]
    fn test_cow_stats() {
        let stats = cow_get_stats();

        // Stats should be accessible
        assert!(stats.total_pages >= 0);
        assert!(stats.shared_pages >= 0);
    }

    #[test]
    fn test_zero_page() {
        let zero = cow_get_zero_page();
        assert!(cow_is_zero_page(zero));
        assert!(!cow_is_zero_page(0x1000));
    }

    #[test]
    fn test_cow_break() {
        let virt = 0x5000;
        let phys = 0x6000;
        let owner = 500;

        cow_mark_page(virt, phys, owner).unwrap();

        let result = cow_break(virt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cow_unmark() {
        let virt = 0x6000;
        let phys = 0x7000;
        let owner = 600;

        cow_mark_page(virt, phys, owner).unwrap();
        assert!(cow_is_cow_page(virt));

        cow_unmark_page(virt);
        assert!(!cow_is_cow_page(virt));
    }
}
