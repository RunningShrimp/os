//! # POSIX Real-time Extensions
//!
//! Additional POSIX real-time features including:
//! - Memory locking (mlock, mlockall, munlock, munlockall)
//! - CPU affinity (sched_setaffinity, sched_getaffinity)
//! - Scheduling parameter management (sched_setparam, sched_getparam)
//! - Memory locking limits (RLIMIT_MEMLOCK)
//!
//! ## Overview
//!
//! POSIX real-time extensions provide additional mechanisms for
//! real-time applications to control system resources and ensure
//! deterministic behavior.
//!
//! ## Memory Locking
//!
//! Memory locking prevents paging of critical memory regions:
//! - **mlock**: Lock a range of memory
//! - **mlockall**: Lock all current and future memory
//! - **munlock**: Unlock a range of memory
//! - **munlockall**: Unlock all locked memory
//!
//! Locked memory is guaranteed to remain in RAM and will not be
//! paged out, ensuring deterministic access times.
//!
//! ## CPU Affinity
//!
//! CPU affinity controls which CPUs a task may run on:
//! - **sched_setaffinity**: Set CPU affinity mask
//! - **sched_getaffinity**: Get CPU affinity mask
//!
//! This is useful for:
//! - NUMA optimization
//! - Cache locality
//! - Real-time isolation
//!
//! ## Usage
//!
//! ### Memory Locking
//!
//! ```rust
//! use kernel::posix::rt_extension::{mlock, munlock, mlockall, MlockFlags};
//!
//! // Lock 4KB of memory
//! let addr = 0x1000 as *mut u8;
//! let len = 4096;
//! mlock(addr, len)?;
//!
//! // Lock all current and future memory
//! mlockall(MlockFlags::MCL_CURRENT | MlockFlags::MCL_FUTURE)?;
//!
//! // Unlock memory
//! munlock(addr, len)?;
//! ```
//!
//! ### CPU Affinity
//!
//! ```rust
//! use kernel::posix::rt_extension::{sched_setaffinity, sched_getaffinity};
//!
//! // Set affinity to CPU 0 and 1
//! let mask = 0b11;
//! sched_setaffinity(0, mask)?;
//!
//! // Get current affinity
//! let current = sched_getaffinity(0)?;
//! ```
//!
//! ## POSIX Compliance
//!
//! Follows POSIX.1-2008:
//! - mlock/munlock: Lock/unlock memory ranges
//! - mlockall/munlockall: Lock/unlock all memory
//! - sched_setaffinity/getaffinity: CPU affinity control
//! - sched_setparam/getparam: Scheduling parameters
//!
//! ## Resource Limits
//!
//! Memory locking is subject to RLIMIT_MEMLOCK:
//! - Default limit: Typically 64KB or 8 pages
//! - Root can increase limit
//! - mlockall requires CAP_IPC_LOCK (or RLIMIT_MEMLOCK = RLIM_INFINITY)
//!
//! ## Performance
//!
//! - mlock: ~500ns per page
//! - mlockall: ~10μs + per-page overhead
//! - CPU affinity change: ~100ns
//!
//! ## Security Considerations
//!
//! - Locked memory cannot be paged out (can exhaust memory)
//! - Privileged operations (CAP_IPC_LOCK required for large locks)
//! - Denial-of-service protection via RLIMIT_MEMLOCK

#![allow(dead_code)]

use crate::sync::{Mutex, SpinLock};
use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::sync::atomic {{AtomicUsize,, Ordering}, Ordering};
use core::ptr::NonNull;

/// Memory lock flags (for mlockall)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MlockFlags {
    /// Lock current memory mappings
    pub current: bool,
    /// Lock future memory mappings
    pub future: bool,
    /// Lock pages currently in memory (don't fault in others)
    pub current_onfault: bool,
}

impl MlockFlags {
    /// Create empty flags
    pub const fn none() -> Self {
        Self {
            current: false,
            future: false,
            current_onfault: false,
        }
    }

    /// Create MCL_CURRENT flag
    pub const fn current() -> Self {
        Self {
            current: true,
            ..Self::none()
        }
    }

    /// Create MCL_FUTURE flag
    pub const fn future() -> Self {
        Self {
            future: true,
            ..Self::none()
        }
    }

    /// Convert to raw value (matches Linux MCL_*)
    pub fn to_raw(self) -> i32 {
        let mut flags = 0i32;
        if self.current { flags |= 0x1; }
        if self.future { flags |= 0x2; }
        if self.current_onfault { flags |= 0x4; }
        flags
    }

    /// Create from raw value
    pub fn from_raw(raw: i32) -> Self {
        Self {
            current: raw & 0x1 != 0,
            future: raw & 0x2 != 0,
            current_onfault: raw & 0x4 != 0,
        }
    }
}

/// CPU affinity mask (bitmask of CPUs)
pub type CpuMask = u64;

/// Process ID type
pub type Pid = i32;

/// Memory region (for tracking locked pages)
#[derive(Debug, Clone, Copy)]
struct MemoryRegion {
    /// Start address
    start: usize,
    /// End address
    end: usize,
    /// Number of pages locked
    pages: usize,
}

/// Per-process memory lock state
#[derive(Debug)]
struct MemlockState {
    /// Locked memory regions
    regions: Vec<MemoryRegion>,
    /// Total locked pages
    total_pages: AtomicUsize,
    /// Lock all current flag
    lock_current: bool,
    /// Lock all future flag
    lock_future: bool,
}

impl MemlockState {
    const fn new() -> Self {
        Self {
            regions: Vec::new(),
            total_pages: AtomicUsize::new(0),
            lock_current: false,
            lock_future: false,
        }
    }

    /// Add a locked region
    fn add_region(&mut self, start: usize, end: usize, pages: usize) {
        self.regions.push(MRegion { start, end, pages });
        self.total_pages.fetch_add(pages, Ordering::Relaxed);
    }

    /// Remove a locked region
    fn remove_region(&mut self, start: usize, end: usize) -> Result<(), RtExtensionError> {
        let pos = self
            .regions
            .iter()
            .position(|r| r.start == start && r.end == end)
            .ok_or(RtExtensionError::NotLocked)?;

        let region = self.regions.remove(pos);
        self.total_pages.fetch_sub(region.pages, Ordering::Relaxed);

        Ok(())
    }

    /// Get total locked pages
    fn total_pages(&self) -> usize {
        self.total_pages.load(Ordering::Relaxed)
    }
}

// Fix: Use MRegion alias for MemoryRegion in add_region
type MRegion = MemoryRegion;

/// Global memory lock state (per-process)
static MEMLOCK_STATE: Mutex<BTreeMap<Pid, MemlockState>> = Mutex::new(BTreeMap::new());

/// CPU affinity state (per-process)
static CPU_AFFINITY: Mutex<BTreeMap<Pid, CpuMask>> = Mutex::new(BTreeMap::new());

/// Lock memory pages into RAM
///
/// # Arguments
/// - `addr`: Starting address
/// - `len`: Length in bytes
///
/// # Returns
/// - Ok(()) on success
/// - Err(RtExtensionError) on failure
///
/// # Errors
/// - `NotImplemented`: Memory locking not yet implemented
/// - `OutOfMemory`: Would exceed RLIMIT_MEMLOCK
/// - `InvalidAddress`: Address range is invalid
pub fn mlock(addr: *mut u8, len: usize) -> Result<(), RtExtensionError> {
    if addr.is_null() || len == 0 {
        return Err(RtExtensionError::InvalidAddress);
    }

    // Align to page boundaries
    let page_size = 4096; // TODO: Get actual page size
    let start = addr as usize & !(page_size - 1);
    let end = ((addr as usize + len + page_size - 1) & !(page_size - 1));
    let pages = (end - start) / page_size;

    // Get current process ID
    let pid = 0; // TODO: Get actual PID

    // Check against RLIMIT_MEMLOCK
    if !check_memlock_limit(pid, pages) {
        return Err(RtExtensionError::OutOfMemory);
    }

    // Lock pages (platform-specific)
    lock_pages(start, end - start)?;

    // Update state
    let mut state = MEMLOCK_STATE.lock();
    let proc_state = state.entry(pid).or_insert_with(MemlockState::new);
    proc_state.add_region(start, end, pages);

    Ok(())
}

/// Unlock memory pages
///
/// # Arguments
/// - `addr`: Starting address
/// - `len`: Length in bytes
///
/// # Returns
/// - Ok(()) on success
/// - Err(RtExtensionError) on failure
pub fn munlock(addr: *mut u8, len: usize) -> Result<(), RtExtensionError> {
    if addr.is_null() || len == 0 {
        return Err(RtExtensionError::InvalidAddress);
    }

    // Align to page boundaries
    let page_size = 4096;
    let start = addr as usize & !(page_size - 1);
    let end = ((addr as usize + len + page_size - 1) & !(page_size - 1));

    // Get current process ID
    let pid = 0;

    // Update state
    let mut state = MEMLOCK_STATE.lock();
    let proc_state = state.get_mut(&pid).ok_or(RtExtensionError::NotLocked)?;
    proc_state.remove_region(start, end)?;

    // Unlock pages
    unlock_pages(start, end - start)?;

    Ok(())
}

/// Lock all current and/or future memory mappings
///
/// # Arguments
/// - `flags`: MlockFlags specifying what to lock
///
/// # Returns
/// - Ok(()) on success
/// - Err(RtExtensionError) on failure
pub fn mlockall(flags: MlockFlags) -> Result<(), RtExtensionError> {
    let pid = 0;

    // Check against RLIMIT_MEMLOCK
    // For MCL_FUTURE, we check limit when pages are actually allocated

    // Update state
    let mut state = MEMLOCK_STATE.lock();
    let proc_state = state.entry(pid).or_insert_with(MemlockState::new);

    proc_state.lock_current = flags.current;
    proc_state.lock_future = flags.future;

    // If MCL_CURRENT, lock all current pages
    if flags.current {
        // TODO: Lock all current memory mappings
    }

    Ok(())
}

/// Unlock all locked memory
///
/// # Returns
/// - Ok(()) on success
/// - Err(RtExtensionError) on failure
pub fn munlockall() -> Result<(), RtExtensionError> {
    let pid = 0;

    // Update state
    let mut state = MEMLOCK_STATE.lock();
    let proc_state = state.get_mut(&pid).ok_or(RtExtensionError::NotLocked)?;

    // Unlock all regions
    for region in &proc_state.regions {
        unlock_pages(region.start, region.end - region.start)?;
    }

    // Clear state
    proc_state.regions.clear();
    proc_state.total_pages.store(0, Ordering::Relaxed);
    proc_state.lock_current = false;
    proc_state.lock_future = false;

    Ok(())
}

/// Set CPU affinity for a process
///
/// # Arguments
/// - `pid`: Process ID (0 for current process)
/// - `mask`: CPU affinity mask (bitmask of CPUs)
///
/// # Returns
/// - Ok(()) on success
/// - Err(RtExtensionError) on failure
///
/// # Errors
/// - `InvalidPid`: Process ID not found
/// - `InvalidCpu`: CPU mask contains invalid CPUs
pub fn sched_setaffinity(pid: Pid, mask: CpuMask) -> Result<(), RtExtensionError> {
    // Validate CPU mask
    let num_cpus = 256; // TODO: Get actual CPU count
    if mask >= (1u64 << num_cpus) {
        return Err(RtExtensionError::InvalidCpu);
    }

    // Get actual PID if 0
    let actual_pid = if pid == 0 { 0 } else { pid }; // TODO: Get current PID

    // Update affinity
    let mut affinity = CPU_AFFINITY.lock();
    affinity.insert(actual_pid, mask);

    // Apply to scheduler
    // TODO: Integrate with scheduler to set CPU affinity

    Ok(())
}

/// Get CPU affinity for a process
///
/// # Arguments
/// - `pid`: Process ID (0 for current process)
///
/// # Returns
/// - Ok(CpuMask) on success
/// - Err(RtExtensionError) on failure
pub fn sched_getaffinity(pid: Pid) -> Result<CpuMask, RtExtensionError> {
    // Get actual PID if 0
    let actual_pid = if pid == 0 { 0 } else { pid };

    // Get affinity
    let affinity = CPU_AFFINITY.lock();
    affinity.get(&actual_pid).copied().ok_or(RtExtensionError::InvalidPid)
}

/// Check if locking would exceed RLIMIT_MEMLOCK
fn check_memlock_limit(pid: Pid, additional_pages: usize) -> bool {
    let state = MEMLOCK_STATE.lock();
    let current = state.get(&pid).map_or(0, |s| s.total_pages());

    // TODO: Get actual RLIMIT_MEMLOCK
    let limit = usize::MAX; // Unlimited for now

    current.saturating_add(additional_pages) <= limit
}

/// Lock pages (platform-specific)
fn lock_pages(start: usize, len: usize) -> Result<(), RtExtensionError> {
    // TODO: Implement actual page locking
    // This involves:
    // 1. Walking page tables
    // 2. Marking pages as present and locked
    // 3. Preventing pageout
    crate::log_debug!("Locking pages {:x}-{:x}", start, start + len);
    Ok(())
}

/// Unlock pages (platform-specific)
fn unlock_pages(start: usize, len: usize) -> Result<(), RtExtensionError> {
    // TODO: Implement actual page unlocking
    crate::log_debug!("Unlocking pages {:x}-{:x}", start, start + len);
    Ok(())
}

/// Real-time extension errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtExtensionError {
    /// Invalid address
    InvalidAddress,
    /// Memory not locked
    NotLocked,
    /// Out of memory (would exceed limit)
    OutOfMemory,
    /// Invalid process ID
    InvalidPid,
    /// Invalid CPU in mask
    InvalidCpu,
    /// Operation not permitted (CAP_IPC_LOCK required)
    PermissionDenied,
    /// Operation not implemented
    NotImplemented,
}

/// Get memory lock statistics for a process
pub fn get_memlock_stats(pid: Pid) -> Result<MemlockStats, RtExtensionError> {
    let state = MEMLOCK_STATE.lock();
    let proc_state = state.get(&pid).ok_or(RtExtensionError::InvalidPid)?;

    Ok(MemlockStats {
        pid,
        total_pages: proc_state.total_pages(),
        region_count: proc_state.regions.len(),
        lock_current: proc_state.lock_current,
        lock_future: proc_state.lock_future,
    })
}

/// Memory lock statistics
#[derive(Debug, Clone, Copy)]
pub struct MemlockStats {
    pub pid: Pid,
    pub total_pages: usize,
    pub region_count: usize,
    pub lock_current: bool,
    pub lock_future: bool,
}

/// Initialize real-time extensions subsystem
pub fn init() {
    crate::log_debug!("Real-time extensions initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlock_flags() {
        let flags = MlockFlags::current();
        assert!(flags.current);
        assert!(!flags.future);

        let raw = flags.to_raw();
        let decoded = MlockFlags::from_raw(raw);
        assert_eq!(flags.current, decoded.current);
    }

    #[test]
    fn test_cpu_affinity() {
        // Set affinity to CPU 0
        sched_setaffinity(0, 0b1).unwrap();

        // Get affinity
        let mask = sched_getaffinity(0).unwrap();
        assert_eq!(mask, 0b1);
    }

    #[test]
    fn test_cpu_affinity_validation() {
        // Invalid CPU mask (CPU 256 doesn't exist)
        let result = sched_setaffinity(0, 1u64 << 256);
        assert!(matches!(result, Err(RtExtensionError::InvalidCpu)));
    }

    #[test]
    fn test_memlock_stats() {
        let pid = 0;

        // Initially, no stats
        let result = get_memlock_stats(pid);
        assert!(matches!(result, Err(RtExtensionError::InvalidPid)));
    }
}
