//! RISC-V synchronization primitives
//!
//! This module provides RISC-V-specific synchronization primitives and
//! memory barrier operations, following the RVWMO (RISC-V Weak Memory Ordering)
//! memory model.
//!
//! # Features
//! - Memory barriers (fence, fence.i)
//! - Atomic operations support
//! - Cache coherency protocols
//! - Lock implementations optimized for RISC-V
//! - RCu (Read-Copy-Update) support
//!
//! # Memory Model
//! RISC-V uses RVWMO (RISC-V Weak Memory Ordering), which allows most
//! memory operations to be reordered. Proper use of memory barriers is
//! essential for correct synchronization.

use core::sync::atomic::{AtomicUsize, Ordering};
use core::arch::asm;
use crate::sync::SpinLock;

/// Fence orders for RISC-V memory barriers
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceOrder {
    /// No ordering (compiler barrier only)
    None = 0,
    /// Write before read (previous writes before subsequent reads)
    WriteRead = 1,
    /// Write before write
    WriteWrite = 2,
    /// Read before write
    ReadWrite = 3,
    /// Read before read
    ReadRead = 4,
    /// All memory operations (full barrier)
    All = 5,
}

/// Memory barrier directions
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceDirection {
    /// Previous operations
    Preceding = 0,
    /// Subsequent operations
    Following = 1,
    /// Both directions
    Both = 2,
}

/// Compiler barrier (prevents compiler reordering)
#[inline]
pub fn compiler_barrier() {
    unsafe {
        // Inline assembly with no actual instructions
        // Prevents compiler from moving accesses across this point
        asm!("", options(nostack, nomem));
    }
}

/// Full memory fence (all memory operations)
#[inline]
pub fn fence() {
    unsafe {
        asm!("fence", options(nostack, nomem));
    }
}

/// Fence with specific ordering
#[inline]
pub fn fence_with_order(order: FenceOrder) {
    match order {
        FenceOrder::None => compiler_barrier(),
        FenceOrder::WriteRead => unsafe {
            asm!("fence w, r", options(nostack, nomem));
        },
        FenceOrder::WriteWrite => unsafe {
            asm!("fence w, w", options(nostack, nomem));
        },
        FenceOrder::ReadWrite => unsafe {
            asm!("fence r, w", options(nostack, nomem));
        },
        FenceOrder::ReadRead => unsafe {
            asm!("fence r, r", options(nostack, nomem));
        },
        FenceOrder::All => fence(),
    }
}

/// Acquire fence (for acquire semantics)
#[inline]
pub fn fence_acquire() {
    unsafe {
        asm!("fence r, rw", options(nostack, nomem));
    }
}

/// Release fence (for release semantics)
#[inline]
pub fn fence_release() {
    unsafe {
        asm!("fence rw, w", options(nostack, nomem));
    }
}

/// Fence for I/O operations (stronger than normal fence)
#[inline]
pub fn fence_io() {
    unsafe {
        asm!("fence iorw, iorw", options(nostack, nomem));
    }
}

/// Instruction synchronization fence
///
/// Ensures that all previous instructions have completed
/// before any subsequent instructions are fetched.
#[inline]
pub fn fence_i() {
    unsafe {
        asm!("fence.i", options(nostack, nomem));
    }
}

/// Memory fence for TLB operations
#[inline]
pub fn fence_tlb() {
    unsafe {
        asm!("sfence.vma", options(nostack, nomem));
    }
}

/// RISC-V atomic operations extension
pub mod atomic {
    use super::*;

    /// Atomic minimum operation
    #[inline]
    pub fn atomic_min(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosmin.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic maximum operation
    #[inline]
    pub fn atomic_max(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosmax.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic unsigned minimum operation
    #[inline]
    pub fn atomic_minu(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosminu.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic unsigned maximum operation
    #[inline]
    pub fn atomic_maxu(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosmaxu.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic AND operation
    #[inline]
    pub fn atomic_and(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosand.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic OR operation
    #[inline]
    pub fn atomic_or(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosor.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic XOR operation
    #[inline]
    pub fn atomic_xor(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amosxor.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// Atomic swap operation
    #[inline]
    pub fn atomic_swap(ptr: *mut u32, value: u32) -> u32 {
        let result: u32;
        unsafe {
            asm!(
                "amoswap.w.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// 64-bit atomic minimum operation
    #[inline]
    pub fn atomic_min_d(ptr: *mut u64, value: u64) -> u64 {
        let result: u64;
        unsafe {
            asm!(
                "amosmin.d.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }

    /// 64-bit atomic maximum operation
    #[inline]
    pub fn atomic_max_d(ptr: *mut u64, value: u64) -> u64 {
        let result: u64;
        unsafe {
            asm!(
                "amosmax.d.aq {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result
    }
}

/// Cache coherency operations
pub mod cache {
    use super::*;

    /// Cache line size (typical for RISC-V systems)
    pub const CACHE_LINE_SIZE: usize = 64;

    /// Clean data cache line
    #[inline]
    pub fn clean_dcache_line(addr: usize) {
        // RISC-V doesn't have explicit cache instructions
        // Cache coherence is maintained by the hardware
        // This is just a compiler barrier
        compiler_barrier();
    }

    /// Invalidate data cache line
    #[inline]
    pub fn invalidate_dcache_line(addr: usize) {
        // RISC-V doesn't have explicit cache instructions
        // Cache coherence is maintained by the hardware
        compiler_barrier();
    }

    /// Clean and invalidate data cache line
    #[inline]
    pub fn clean_invalidate_dcache_line(addr: usize) {
        compiler_barrier();
    }

    /// Clean data cache range
    #[inline]
    pub fn clean_dcache_range(addr: usize, size: usize) {
        let start = addr & !(CACHE_LINE_SIZE - 1);
        let end = ((addr + size - 1) & !(CACHE_LINE_SIZE - 1)) + CACHE_LINE_SIZE;

        for addr in (start..end).step_by(CACHE_LINE_SIZE) {
            clean_dcache_line(addr);
        }
    }

    /// Invalidate data cache range
    #[inline]
    pub fn invalidate_dcache_range(addr: usize, size: usize) {
        let start = addr & !(CACHE_LINE_SIZE - 1);
        let end = ((addr + size - 1) & !(CACHE_LINE_SIZE - 1)) + CACHE_LINE_SIZE;

        for addr in (start..end).step_by(CACHE_LINE_SIZE) {
            invalidate_dcache_line(addr);
        }
    }

    /// Synchronize all caches
    #[inline]
    pub fn sync_caches() {
        fence();
    }
}

/// Test-and-set lock implementation
#[repr(C)]
pub struct TasLock {
    /// Lock flag (0 = unlocked, 1 = locked)
    flag: AtomicUsize,
}

impl TasLock {
    /// Create a new test-and-set lock
    pub const fn new() -> Self {
        Self {
            flag: AtomicUsize::new(0),
        }
    }

    /// Try to acquire the lock
    pub fn try_acquire(&self) -> bool {
        let result = self.flag.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed);
        result.is_ok()
    }

    /// Acquire the lock (spin until acquired)
    pub fn acquire(&self) {
        while !self.try_acquire() {
            // Spin with pause hint
            unsafe {
                asm!("pause");
            }
        }
    }

    /// Release the lock
    pub fn release(&self) {
        self.flag.store(0, Ordering::Release);
    }
}

/// Ticket lock implementation (FIFO fairness)
#[repr(C)]
pub struct TicketLock {
    /// Next ticket to be served
    next_ticket: AtomicUsize,
    /// Next ticket to be issued
    next_serving: AtomicUsize,
}

impl TicketLock {
    /// Create a new ticket lock
    pub const fn new() -> Self {
        Self {
            next_ticket: AtomicUsize::new(0),
            next_serving: AtomicUsize::new(0),
        }
    }

    /// Acquire the lock
    pub fn acquire(&self) {
        // Take a ticket
        let ticket = self.next_ticket.fetch_add(1, Ordering::Acquire);

        // Wait for our ticket to be served
        while self.next_serving.load(Ordering::Acquire) != ticket {
            unsafe {
                asm!("pause");
            }
        }
    }

    /// Release the lock
    pub fn release(&self) {
        // Serve the next ticket
        let next = self.next_serving.load(Ordering::Acquire) + 1;
        self.next_serving.store(next, Ordering::Release);
    }

    /// Try to acquire the lock (non-blocking)
    pub fn try_acquire(&self) -> bool {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Acquire);

        if self.next_serving.load(Ordering::Acquire) == ticket {
            true
        } else {
            // Rollback (not ideal, but works for try_lock)
            self.next_ticket.fetch_sub(1, Ordering::Release);
            false
        }
    }
}

/// Read-copy-update (RCU) support
pub mod rcu {
    use super::*;

    /// RCU read-side critical section
    pub struct RcuReadSection {
        /// Nesting depth
        depth: u32,
    }

    impl RcuReadSection {
        /// Enter an RCU read-side critical section
        pub fn enter() -> Self {
            // Increment RCU nesting depth
            // In production, this would be per-CPU

            Self { depth: 1 }
        }

        /// Exit an RCU read-side critical section
        pub fn exit(self) {
            // Decrement RCU nesting depth
            drop(self);
        }
    }

    /// Wait for all pre-existing RCU read-side critical sections to complete
    pub fn synchronize_rcu() {
        // In production, this would:
        // 1. Increment global grace period counter
        // 2. Wait for all CPUs to pass through a quiescent state
        // 3. Signal completion

        fence();
    }

    /// Register an RCU callback to be called after a grace period
    pub fn call_rcu(_callback: fn()) {
        // In production, this would register the callback
        // to be called after the next grace period
        fence();
    }
}

/// Load-link / store-conditional (LL/SC) operations
pub mod llsc {
    use super::*;

    /// Load-linked (32-bit)
    #[inline]
    pub fn ll(ptr: *const u32) -> Option<u32> {
        let result: u32;
        unsafe {
            asm!(
                "lr.w {result}, ({ptr})",
                result = out(reg) result,
                ptr = in(reg) ptr,
            );
        }
        Some(result)
    }

    /// Store-conditional (32-bit)
    #[inline]
    pub fn sc(ptr: *mut u32, value: u32) -> bool {
        let result: u32;
        unsafe {
            asm!(
                "sc.w {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result == 0
    }

    /// Load-linked (64-bit)
    #[inline]
    pub fn ll_d(ptr: *const u64) -> Option<u64> {
        let result: u64;
        unsafe {
            asm!(
                "lr.d {result}, ({ptr})",
                result = out(reg) result,
                ptr = in(reg) ptr,
            );
        }
        Some(result)
    }

    /// Store-conditional (64-bit)
    #[inline]
    pub fn sc_d(ptr: *mut u64, value: u64) -> bool {
        let result: u64;
        unsafe {
            asm!(
                "sc.d {result}, {value}, ({ptr})",
                result = out(reg) result,
                value = in(reg) value,
                ptr = in(reg) ptr,
            );
        }
        result == 0
    }

    /// Compare-and-swap using LL/SC (32-bit)
    #[inline]
    pub fn cas(ptr: *mut u32, expected: u32, new: u32) -> bool {
        loop {
            let old = ll(ptr)?;
            if old != expected {
                return false;
            }
            if sc(ptr, new) {
                return true;
            }
            // Retry if store-conditional failed
        }
    }

    /// Compare-and-swap using LL/SC (64-bit)
    #[inline]
    pub fn cas_d(ptr: *mut u64, expected: u64, new: u64) -> bool {
        loop {
            let old = ll_d(ptr)?;
            if old != expected {
                return false;
            }
            if sc_d(ptr, new) {
                return true;
            }
            // Retry if store-conditional failed
        }
    }
}

/// Memory ordering and synchronization helpers
pub mod ordering {
    use super::*;

    /// Write barrier (ensure all writes are visible)
    #[inline]
    pub fn write_barrier() {
        fence_release();
    }

    /// Read barrier (ensure all reads are complete)
    #[inline]
    pub fn read_barrier() {
        fence_acquire();
    }

    /// Data dependency barrier
    #[inline]
    pub fn data_dependency_barrier() {
        compiler_barrier();
    }

    /// Serialize instruction execution
    #[inline]
    pub fn serialize() {
        fence();
        fence_i();
    }
}

/// Spin-wait with pause hint
#[inline]
pub fn spin_wait() {
    unsafe {
        asm!("pause");
    }
}

/// Yield the current CPU
#[inline]
pub fn cpu_yield() {
    unsafe {
        asm!("pause");
    }
}

/// Pause hint for spin loops
#[inline]
pub fn pause() {
    unsafe {
        asm!("pause");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fence_functions() {
        fence();
        fence_acquire();
        fence_release();
        fence_io();
        fence_tlb();
        fence_with_order(FenceOrder::All);
    }

    #[test]
    fn test_compiler_barrier() {
        let mut x = 0;
        compiler_barrier();
        x = 1;
        compiler_barrier();
        assert_eq!(x, 1);
    }

    #[test]
    fn test_tas_lock() {
        let lock = TasLock::new();

        assert!(lock.try_acquire());
        assert!(!lock.try_acquire());
        lock.release();
        assert!(lock.try_acquire());
        lock.release();
    }

    #[test]
    fn test_ticket_lock() {
        let lock = TicketLock::new();

        lock.acquire();
        lock.release();

        assert!(lock.try_acquire());
        lock.release();
    }

    #[test]
    fn test_llsc() {
        let mut value = 42u32;
        let ptr = &mut value as *mut u32;

        let old = ll(ptr).unwrap();
        assert_eq!(old, 42);

        assert!(sc(ptr, 100));
        assert_eq!(value, 100);

        assert!(cas(ptr, 100, 200));
        assert_eq!(value, 200);

        assert!(!cas(ptr, 100, 300));
        assert_eq!(value, 200);
    }

    #[test]
    fn test_atomic_ops() {
        let mut value = 10u32;
        let ptr = &mut value as *mut u32;

        let result = atomic_or(ptr, 5);
        assert_eq!(value, 15);

        let result = atomic_and(ptr, 12);
        assert_eq!(value, 12);

        let result = atomic_xor(ptr, 4);
        assert_eq!(value, 8);
    }
}
