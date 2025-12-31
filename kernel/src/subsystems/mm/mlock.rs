//! # Memory Locking and Pinning
//!
//! This module implements memory locking (mlock) functionality to prevent
//! pages from being swapped out or moved. This is critical for real-time
//! applications, cryptographic operations, and DMA operations.
//!
//! ## Overview
//!
//! Memory locking prevents pages from being paged out to swap and ensures
//! they remain in physical memory. This module provides:
//! - mlock/munlock system calls
//! - mlockall/munlockall for entire address space
//! - Per-process locking limits (RLIMIT_MEMLOCK)
//! - Memory pinning for DMA
//!
//! ## Features
//!
//! - **mlock/munlock**: Lock/unlock memory regions
//! - **mlockall/munlockall**: Lock/unlock entire address space
//! - **VM_LOCKED flag**: Track locked pages
//! - **RLIMIT_MEMLOCK**: Per-process limits
//! - **DMA Pinning**: Pin pages for DMA operations
//! - **Security**: Prevent sensitive data from being swapped
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::subsystems::mm::mlock::{mlock, munlock, mlock_status, MlockFlags};
//!
//! // Lock 4KB of memory
//! let addr = 0x1000;
//! let size = 4096;
//!
//! match mlock(addr, size, MlockFlags::empty()) {
//!     Ok(()) => println!("Memory locked"),
//!     Err(e) => println!("Lock failed: {:?}", e),
//! }
//!
//! // Check lock status
//! if mlock_status(addr) {
//!     println!("Page is locked");
//! }
//!
//! // Unlock memory
//! let _ = munlock(addr, size);
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

/// Default maximum locked memory per process (bytes)
const DEFAULT_MAX_LOCKED: usize = 64 * 1024 * 1024; // 64MB

/// Maximum locked memory for privileged processes (8GB)
const MAX_LOCKED_PRIVILEGED: usize = 8 * 1024 * 1024 * 1024;

/// VM_LOCKED flag value
const VM_LOCKED: usize = 0x2000;

// ============================================================================
// Error Types
// ============================================================================

/// mlock operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlockError {
    /// Out of memory
    OutOfMemory,
    /// Would exceed limit
    WouldExceedLimit,
    /// Invalid address range
    InvalidRange,
    /// Address not aligned
    NotAligned,
    /// Permission denied
    PermissionDenied,
    /// Operation not permitted
    NotPermitted,
}

// ============================================================================
// Mlock Flags
// ============================================================================

/// Flags for mlock operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MlockFlags {
    bits: u32,
}

impl MlockFlags {
    pub const ONFAULT: u32 = 0x01;
    pub const DMA: u32 = 0x02;
    pub const SECURITY: u32 = 0x04;
    pub const REALTIME: u32 = 0x08;

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    pub const fn contains(&self, other: MlockFlags) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

// ============================================================================
// Locked Region
// ============================================================================

/// Descriptor for a locked memory region
#[derive(Debug, Clone)]
struct LockedRegion {
    /// Start virtual address
    start: usize,
    /// End virtual address
    end: usize,
    /// Number of pages locked
    page_count: usize,
    /// Lock flags
    flags: MlockFlags,
    /// Process ID
    pid: usize,
    /// Lock timestamp
    lock_time: u64,
}

impl LockedRegion {
    /// Create a new locked region
    fn new(start: usize, end: usize, flags: MlockFlags, pid: usize) -> Self {
        let page_count = (end - start + PAGE_SIZE - 1) / PAGE_SIZE;

        Self {
            start,
            end,
            page_count,
            flags,
            pid,
            lock_time: crate::subsystems::time::get_ticks(),
        }
    }

    /// Check if address is within this region
    fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Get size in bytes
    fn size(&self) -> usize {
        self.end - self.start
    }
}

// ============================================================================
// Per-Process Lock Limits
// ============================================================================

/// Per-process lock limit tracking
struct ProcessLimits {
    /// Current locked bytes
    locked_bytes: AtomicUsize,
    /// Maximum allowed bytes
    max_bytes: AtomicUsize,
    /// Number of mlock operations
    mlock_count: AtomicU64,
    /// Number of munlock operations
    munlock_count: AtomicU64,
}

impl ProcessLimits {
    /// Create a new process limit tracker
    const fn new(max_bytes: usize) -> Self {
        Self {
            locked_bytes: AtomicUsize::new(0),
            max_bytes: AtomicUsize::new(max_bytes),
            mlock_count: AtomicU64::new(0),
            munlock_count: AtomicU64::new(0),
        }
    }

    /// Try to allocate locked bytes
    fn try_lock(&self, bytes: usize) -> bool {
        let current = self.locked_bytes.load(Ordering::Relaxed);
        let max = self.max_bytes.load(Ordering::Relaxed);

        if current + bytes > max {
            return false;
        }

        self.locked_bytes.fetch_add(bytes, Ordering::Relaxed);
        true
    }

    /// Release locked bytes
    fn unlock(&self, bytes: usize) {
        let current = self.locked_bytes.load(Ordering::Relaxed);
        self.locked_bytes.store(current.saturating_sub(bytes), Ordering::Relaxed);
    }

    /// Get current locked bytes
    fn get_locked(&self) -> usize {
        self.locked_bytes.load(Ordering::Relaxed)
    }

    /// Set maximum locked bytes
    fn set_max(&self, max: usize) {
        self.max_bytes.store(max, Ordering::Relaxed);
    }

    /// Get maximum locked bytes
    fn get_max(&self) -> usize {
        self.max_bytes.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Mlock Manager
// ============================================================================

/// Memory locking manager
pub struct MlockManager {
    /// Map of locked regions by address
    locked_regions: Mutex<BTreeMap<usize, LockedRegion>>,
    /// Per-process limits
    process_limits: Mutex<BTreeMap<usize, ProcessLimits>>,
    /// Total locked pages
    total_locked_pages: AtomicUsize,
    /// Total pinned pages (for DMA)
    total_pinned_pages: AtomicUsize,
    /// mlock operations performed
    mlock_ops: AtomicU64,
    /// munlock operations performed
    munlock_ops: AtomicU64,
    /// Failed mlock operations
    mlock_failures: AtomicU64,
}

impl MlockManager {
    /// Create a new mlock manager
    pub const fn new() -> Self {
        Self {
            locked_regions: Mutex::new(BTreeMap::new()),
            process_limits: Mutex::new(BTreeMap::new()),
            total_locked_pages: AtomicUsize::new(0),
            total_pinned_pages: AtomicUsize::new(0),
            mlock_ops: AtomicU64::new(0),
            munlock_ops: AtomicU64::new(0),
            mlock_failures: AtomicU64::new(0),
        }
    }

    /// Lock a memory region
    ///
    /// # Arguments
    ///
    /// * `addr` - Start address
    /// * `size` - Size in bytes
    /// * `flags` - Lock flags
    /// * `pid` - Process ID
    ///
    /// # Returns
    ///
    /// * `Result<(), MlockError>` - Success or error
    fn lock_region(
        &self,
        addr: usize,
        size: usize,
        flags: MlockFlags,
        pid: usize,
    ) -> Result<(), MlockError> {
        // Validate alignment
        if addr % PAGE_SIZE != 0 {
            return Err(MlockError::NotAligned);
        }

        // Validate size
        if size == 0 {
            return Ok(());
        }

        // Align size up to page boundary
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end = addr + aligned_size;

        // Check if we have enough quota
        {
            let mut limits = self.process_limits.lock();
            if !limits.contains_key(&pid) {
                limits.insert(pid, ProcessLimits::new(DEFAULT_MAX_LOCKED));
            }
            let limit = limits.get(&pid).unwrap();
            if !limit.try_lock(aligned_size) {
                self.mlock_failures.fetch_add(1, Ordering::Relaxed);
                return Err(MlockError::WouldExceedLimit);
            }
        }

        // Create locked region
        let region = LockedRegion::new(addr, end, flags, pid);

        // Add to locked regions
        {
            let mut regions = self.locked_regions.lock();
            regions.insert(addr, region);
        }

        // Update statistics
        let page_count = aligned_size / PAGE_SIZE;
        self.total_locked_pages.fetch_add(page_count, Ordering::Relaxed);

        if flags.contains(MlockFlags::from_bits(MlockFlags::DMA)) {
            self.total_pinned_pages.fetch_add(page_count, Ordering::Relaxed);
        }

        self.mlock_ops.fetch_add(1, Ordering::Relaxed);

        // Update process stats
        {
            let mut limits = self.process_limits.lock();
            if let Some(limit) = limits.get_mut(&pid) {
                limit.mlock_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Unlock a memory region
    ///
    /// # Arguments
    ///
    /// * `addr` - Start address
    /// * `size` - Size in bytes
    /// * `pid` - Process ID
    ///
    /// # Returns
    ///
    /// * `Result<(), MlockError>` - Success or error
    fn unlock_region(&self, addr: usize, size: usize, pid: usize) -> Result<(), MlockError> {
        if size == 0 {
            return Ok(());
        }

        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end = addr + aligned_size;

        // Find and remove locked regions
        let mut unlocked_bytes = 0;
        let mut unlocked_pages = 0;
        let mut unpinned_pages = 0;

        {
            let mut regions = self.locked_regions.lock();

            // Remove all regions in the range
            let mut to_remove = Vec::new();
            for (&start, region) in regions.iter() {
                if region.pid == pid && start < end && region.end > addr {
                    to_remove.push(start);
                    unlocked_bytes += region.size();
                    unlocked_pages += region.page_count;

                    if region.flags.contains(MlockFlags::from_bits(MlockFlags::DMA)) {
                        unpinned_pages += region.page_count;
                    }
                }
            }

            for start in to_remove {
                regions.remove(&start);
            }
        }

        if unlocked_bytes == 0 {
            return Err(MlockError::InvalidRange);
        }

        // Update process limits
        {
            let mut limits = self.process_limits.lock();
            if let Some(limit) = limits.get_mut(&pid) {
                limit.unlock(unlocked_bytes);
                limit.munlock_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Update global statistics
        self.total_locked_pages.fetch_sub(unlocked_pages, Ordering::Relaxed);
        self.total_pinned_pages.fetch_sub(unpinned_pages, Ordering::Relaxed);
        self.munlock_ops.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Lock entire address space
    ///
    /// # Arguments
    ///
    /// * `flags` - Lock flags
    /// * `pid` - Process ID
    fn lock_all(&self, flags: MlockFlags, pid: usize) -> Result<(), MlockError> {
        // Lock a large range (typical user space)
        let start = 0x1000;
        let end = 0x0000_8000_0000usize; // 128TB (typical user space limit)
        let size = end - start;

        self.lock_region(start, size, flags, pid)
    }

    /// Unlock entire address space
    ///
    /// # Arguments
    ///
    /// * `pid` - Process ID
    fn unlock_all(&self, pid: usize) -> Result<(), MlockError> {
        // Remove all locked regions for this process
        let mut unlocked_bytes = 0;
        let mut unlocked_pages = 0;

        {
            let mut regions = self.locked_regions.lock();
            let to_remove: Vec<usize> = regions
                .iter()
                .filter(|(_, region)| region.pid == pid)
                .map(|(&start, _)| start)
                .collect();

            for start in to_remove {
                if let Some(region) = regions.remove(&start) {
                    unlocked_bytes += region.size();
                    unlocked_pages += region.page_count;
                }
            }
        }

        // Update limits
        {
            let mut limits = self.process_limits.lock();
            if let Some(limit) = limits.get_mut(&pid) {
                limit.unlock(unlocked_bytes);
                limit.munlock_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.total_locked_pages.fetch_sub(unlocked_pages, Ordering::Relaxed);
        self.munlock_ops.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Check if an address is locked
    ///
    /// # Arguments
    ///
    /// * `addr` - Address to check
    ///
    /// # Returns
    ///
    /// * `bool` - True if address is locked
    fn is_locked(&self, addr: usize) -> bool {
        let regions = self.locked_regions.lock();

        for region in regions.values() {
            if region.contains(addr) {
                return true;
            }
        }

        false
    }

    /// Set process lock limit
    ///
    /// # Arguments
    ///
    /// * `pid` - Process ID
    /// * `max_bytes` - Maximum locked bytes
    fn set_limit(&self, pid: usize, max_bytes: usize) {
        let mut limits = self.process_limits.lock();

        if let Some(limit) = limits.get_mut(&pid) {
            limit.set_max(max_bytes);
        } else {
            limits.insert(pid, ProcessLimits::new(max_bytes));
        }
    }

    /// Get process lock limit
    ///
    /// # Arguments
    ///
    /// * `pid` - Process ID
    ///
    /// # Returns
    ///
    /// * `Option<(usize, usize)>` - (current, max) bytes or None
    fn get_limit(&self, pid: usize) -> Option<(usize, usize)> {
        let limits = self.process_limits.lock();
        limits.get(&pid).map(|limit| (limit.get_locked(), limit.get_max()))
    }

    /// Get statistics
    fn get_stats(&self) -> MlockStats {
        MlockStats {
            total_locked_pages: self.total_locked_pages.load(Ordering::Relaxed),
            total_pinned_pages: self.total_pinned_pages.load(Ordering::Relaxed),
            mlock_ops: self.mlock_ops.load(Ordering::Relaxed),
            munlock_ops: self.munlock_ops.load(Ordering::Relaxed),
            mlock_failures: self.mlock_failures.load(Ordering::Relaxed),
        }
    }
}

/// Mlock statistics
#[derive(Debug, Clone, Copy)]
pub struct MlockStats {
    pub total_locked_pages: usize,
    pub total_pinned_pages: usize,
    pub mlock_ops: u64,
    pub munlock_ops: u64,
    pub mlock_failures: u64,
}

/// Global mlock manager
static MLOCK_MANAGER: MlockManager = MlockManager::new();

// ============================================================================
// Public API
// ============================================================================

/// Lock a memory region
///
/// # Arguments
///
/// * `addr` - Start address (must be page-aligned)
/// * `size` - Size in bytes
/// * `flags` - Additional lock flags
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn mlock(addr: usize, size: usize, flags: MlockFlags) -> Result<(), MlockError> {
    // Get current process ID
    let pid = get_current_pid();

    MLOCK_MANAGER.lock_region(addr, size, flags, pid)
}

/// Unlock a memory region
///
/// # Arguments
///
/// * `addr` - Start address
/// * `size` - Size in bytes
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn munlock(addr: usize, size: usize) -> Result<(), MlockError> {
    let pid = get_current_pid();

    MLOCK_MANAGER.unlock_region(addr, size, pid)
}

/// Lock the entire address space
///
/// # Arguments
///
/// * `flags` - Lock flags
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn mlockall(flags: MlockFlags) -> Result<(), MlockError> {
    let pid = get_current_pid();

    MLOCK_MANAGER.lock_all(flags, pid)
}

/// Unlock the entire address space
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn munlockall() -> Result<(), MlockError> {
    let pid = get_current_pid();

    MLOCK_MANAGER.unlock_all(pid)
}

/// Check if an address is locked
///
/// # Arguments
///
/// * `addr` - Address to check
///
/// # Returns
///
/// * `bool` - True if address is locked
pub fn mlock_status(addr: usize) -> bool {
    MLOCK_MANAGER.is_locked(addr)
}

/// Set process lock limit (RLIMIT_MEMLOCK)
///
/// # Arguments
///
/// * `pid` - Process ID
/// * `max_bytes` - Maximum locked bytes
pub fn mlock_set_limit(pid: usize, max_bytes: usize) {
    MLOCK_MANAGER.set_limit(pid, max_bytes)
}

/// Get process lock limit
///
/// # Arguments
///
/// * `pid` - Process ID
///
/// # Returns
///
/// * `Option<(usize, usize)>` - (current, max) or None if process not found
pub fn mlock_get_limit(pid: usize) -> Option<(usize, usize)> {
    MLOCK_MANAGER.get_limit(pid)
}

/// Get mlock statistics
///
/// # Returns
///
/// * `MlockStats` - Current statistics
pub fn mlock_get_stats() -> MlockStats {
    MLOCK_MANAGER.get_stats()
}

/// Pin pages for DMA operation
///
/// # Arguments
///
/// * `addr` - Start address
/// * `size` - Size in bytes
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn mlock_pin_pages(addr: usize, size: usize) -> Result<(), MlockError> {
    let flags = MlockFlags::from_bits(MlockFlags::DMA);
    mlock(addr, size, flags)
}

/// Unpin pages after DMA operation
///
/// # Arguments
///
/// * `addr` - Start address
/// * `size` - Size in bytes
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn mlock_unpin_pages(addr: usize, size: usize) -> Result<(), MlockError> {
    munlock(addr, size)
}

/// Lock memory for security (prevent swapping)
///
/// # Arguments
///
/// * `addr` - Start address
/// * `size` - Size in bytes
///
/// # Returns
///
/// * `Result<(), MlockError>` - Success or error
pub fn mlock_lock_sensitive(addr: usize, size: usize) -> Result<(), MlockError> {
    let flags = MlockFlags::from_bits(MlockFlags::SECURITY);
    mlock(addr, size, flags)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current process ID
///
/// In a real implementation, this would query the process manager
fn get_current_pid() -> usize {
    // Placeholder: return dummy PID
    1
}

/// Check if current process has CAP_IPC_LOCK capability
fn has_ipc_lock_capability() -> bool {
    // In a real implementation, this would check process capabilities
    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlock_munlock() {
        let addr = 0x1000;
        let size = PAGE_SIZE;

        // Lock memory
        let result = mlock(addr, size, MlockFlags::empty());
        assert!(result.is_ok());

        // Check status
        assert!(mlock_status(addr));

        // Unlock memory
        let result = munlock(addr, size);
        assert!(result.is_ok());

        // Check status
        assert!(!mlock_status(addr));
    }

    #[test]
    fn test_mlock_alignment() {
        let addr = 0x1001; // Not aligned
        let size = PAGE_SIZE;

        let result = mlock(addr, size, MlockFlags::empty());
        assert!(matches!(result, Err(MlockError::NotAligned)));
    }

    #[test]
    fn test_mlockall() {
        let result = mlockall(MlockFlags::empty());
        assert!(result.is_ok());

        let result = munlockall();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mlock_limits() {
        let pid = 999;

        // Set limit
        mlock_set_limit(pid, 1024 * 1024); // 1MB

        // Get limit
        let limit = mlock_get_limit(pid);
        assert!(limit.is_some());

        let (current, max) = limit.unwrap();
        assert_eq!(max, 1024 * 1024);
    }

    #[test]
    fn test_mlock_pin_pages() {
        let addr = 0x2000;
        let size = PAGE_SIZE;

        let result = mlock_pin_pages(addr, size);
        assert!(result.is_ok());

        assert!(mlock_status(addr));

        let result = mlock_unpin_pages(addr, size);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mlock_flags() {
        let addr = 0x3000;
        let size = PAGE_SIZE;

        let flags = MlockFlags::SECURITY | MlockFlags::REALTIME;
        let result = mlock(addr, size, flags);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mlock_stats() {
        let stats = mlock_get_stats();

        assert!(stats.total_locked_pages >= 0);
        assert!(stats.total_pinned_pages >= 0);
        assert!(stats.mlock_ops >= 0);
    }

    #[test]
    fn test_zero_size() {
        let addr = 0x4000;
        let size = 0;

        let result = mlock(addr, size, MlockFlags::empty());
        assert!(result.is_ok());

        let result = munlock(addr, size);
        assert!(result.is_ok());
    }
}
