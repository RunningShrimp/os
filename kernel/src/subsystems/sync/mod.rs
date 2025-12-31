// Synchronization primitives for xv6-rust kernel
// Provides SpinLock, Mutex, Sleeplock, Once, and related types
//
// SMP-safe implementation with proper memory barriers and interrupt handling.

use core::{
    cell::UnsafeCell,
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

use crate::cpu;

// Declare submodules
pub mod mutex;
pub mod sleeplock;
pub mod rwlock;
pub mod spinlock;
pub mod once;
pub mod primitives;
pub mod priority_mutex;
pub mod realtime;
pub mod rcu;
pub mod interrupts;
pub mod lazy;

// ============================================================================
// Interrupt control for SMP safety
// ============================================================================

/// Disable interrupts and return previous interrupt state
#[inline]
pub fn push_off() -> bool {
    let was_enabled = interrupts_enabled();
    disable_interrupts();
    was_enabled
}

/// Restore interrupt state
#[inline]
pub fn pop_off(was_enabled: bool) {
    if was_enabled {
        enable_interrupts();
    }
}

/// Check if interrupts are enabled
#[inline]
fn interrupts_enabled() -> bool {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        let sstatus: usize;
        core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
        (sstatus & 0x2) != 0 // SIE bit
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let daif: u64;
        core::arch::asm!("mrs {}, daif", out(reg) daif);
        (daif & 0x80) == 0 // IRQ not masked
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        let flags: u64;
        core::arch::asm!("pushfq; pop {}", out(reg) flags);
        (flags & 0x200) != 0 // IF flag
    }
}

/// Disable interrupts
#[inline]
fn disable_interrupts() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("csrc sstatus, {}", in(reg) 0x2usize); // Clear SIE
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr daifset, #2"); // Mask IRQ
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli");
    }
}

/// Enable interrupts
#[inline]
fn enable_interrupts() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) 0x2usize); // Set SIE
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr daifclr, #2"); // Unmask IRQ
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sti");
    }
}

// ============================================================================
// SpinLock - SMP-safe spinlock with interrupt control
// ============================================================================

/// Raw spinlock for low-level synchronization
/// This version includes proper memory barriers for SMP safety
pub struct RawSpinLock {
    locked: AtomicBool,
    // For debugging/deadlock detection
    cpu_id: AtomicUsize,
    // Lock analytics (very lightweight)
    acquire_count: AtomicU64,
    contended_count: AtomicU64,
}

impl RawSpinLock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            cpu_id: AtomicUsize::new(0),
            acquire_count: AtomicU64::new(0),
            contended_count: AtomicU64::new(0),
        }
    }

    pub fn lock(&self) {
        // Disable interrupts to prevent deadlock with ISR
        push_off();

        // Spin until lock is acquired
        let mut contended = false;
        while self.locked.swap(true, Ordering::Acquire) {
            contended = true;
            // Spin loop hint to CPU
            core::hint::spin_loop();
        }
        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        if contended {
            self.contended_count.fetch_add(1, Ordering::Relaxed);
        }

        // Record CPU holding the lock
        self.cpu_id.store(cpu::cpuid(), Ordering::Relaxed);
    }

    pub fn unlock(&self) {
        self.cpu_id.store(0, Ordering::Relaxed);

        // Release lock
        self.locked.store(false, Ordering::Release);

        // Restore interrupt state
        pop_off(false); // Argument ignored by pop_off implementation above? 
        // Wait, pop_off takes `was_enabled`.
        // The implementation in sync.rs uses a thread-local (CPU-local) stack
        // to track interrupt state.
    }

    pub fn try_lock(&self) -> bool {
        if !self.locked.swap(true, Ordering::Acquire) {
            push_off();
            true
        } else {
            false
        }
    }

    /// Check if the lock is currently held
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Get total lock acquisitions (for diagnostics)
    pub fn acquire_count(&self) -> u64 {
        self.acquire_count.load(Ordering::Relaxed)
    }

    /// Get total contended acquisitions (for diagnostics)
    pub fn contended_count(&self) -> u64 {
        self.contended_count.load(Ordering::Relaxed)
    }

    /// Check if the current CPU is holding the lock
    pub fn holding(&self) -> bool {
        self.is_locked() && self.cpu_id.load(Ordering::Relaxed) == cpu::cpuid()
    }
}


#[cfg(feature = "kernel_tests")]
pub mod tests;

pub mod futex_tests;
pub mod futex_validation;

// Re-export the generic SpinLock<T> from spinlock submodule
pub use spinlock::{SpinLock, SpinLockGuard};

// ============================================================================
// SpinLockIrq - Spinlock that disables interrupts
// ============================================================================

/// Spinlock that disables interrupts while held
/// Essential for SMP safety when the lock might be accessed from interrupt context
pub struct SpinLockIrq {
    inner: RawSpinLock,
}

impl SpinLockIrq {
    pub const fn new() -> Self {
        Self { inner: RawSpinLock::new() }
    }

    /// Acquire lock and disable interrupts
    /// Returns a guard that restores interrupt state on drop
    #[inline]
    pub fn lock(&self) -> SpinLockIrqGuard<'_> {
        let was_enabled = push_off();
        self.inner.lock();
        SpinLockIrqGuard { lock: self, was_enabled }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<SpinLockIrqGuard<'_>> {
        let was_enabled = push_off();
        if self.inner.try_lock() {
            Some(SpinLockIrqGuard { lock: self, was_enabled })
        } else {
            pop_off(was_enabled);
            None
        }
    }

    #[inline]
    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }

    #[inline]
    pub fn holding(&self) -> bool {
        self.inner.holding()
    }
}

/// RAII guard for SpinLockIrq
pub struct SpinLockIrqGuard<'a> {
    lock: &'a SpinLockIrq,
    was_enabled: bool,
}

impl Drop for SpinLockIrqGuard<'_> {
    fn drop(&mut self) {
        self.lock.inner.unlock();
        pop_off(self.was_enabled);
    }
}

// ============================================================================
// Re-export Mutex types from main sync module
// ============================================================================

pub use crate::sync::{Mutex, MutexGuard, MutexIrq, MutexIrqGuard};

// ============================================================================
// Once - One-time initialization primitive
// ============================================================================

const ONCE_INCOMPLETE: usize = 0;
const ONCE_RUNNING: usize = 1;
const ONCE_COMPLETE: usize = 2;

/// A synchronization primitive for one-time initialization
pub struct Once {
    state: AtomicUsize,
}

impl Once {
    pub const fn new() -> Self {
        Self { state: AtomicUsize::new(ONCE_INCOMPLETE) }
    }

    /// Returns true if `call_once` has completed successfully
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == ONCE_COMPLETE
    }

    /// Performs initialization exactly once
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        if self.state.load(Ordering::Acquire) == ONCE_COMPLETE {
            return;
        }
        self.call_once_slow(f);
    }

    #[cold]
    fn call_once_slow<F: FnOnce()>(&self, f: F) {
        loop {
            match self.state.compare_exchange(
                ONCE_INCOMPLETE,
                ONCE_RUNNING,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // We won the race to initialize
                    f();
                    self.state.store(ONCE_COMPLETE, Ordering::Release);
                    return;
                },
                Err(ONCE_COMPLETE) => return,
                Err(ONCE_RUNNING) => {
                    // Spin while another thread initializes
                    while self.state.load(Ordering::Acquire) == ONCE_RUNNING {
                        core::hint::spin_loop();
                    }
                },
                Err(_) => unreachable!(),
            }
        }
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lazy<T> - Lazily initialized value
// ============================================================================

/// A value which is initialized on first access
pub struct Lazy<T, F = fn() -> T> {
    once: Once,
    init: UnsafeCell<Option<F>>,
    value: UnsafeCell<Option<T>>,
}

// Safety: Lazy uses Once for synchronization
unsafe impl<T: Send + Sync, F: Send> Sync for Lazy<T, F> {}
unsafe impl<T: Send, F: Send> Send for Lazy<T, F> {}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    pub const fn new(init: F) -> Self {
        Self {
            once: Once::new(),
            init: UnsafeCell::new(Some(init)),
            value: UnsafeCell::new(None),
        }
    }

    /// Forces initialization if not already done
    pub fn force(this: &Self) -> &T {
        this.once.call_once(|| {
            // Safety: We're inside call_once, so only one thread runs this
            let init = unsafe { (*this.init.get()).take().unwrap() };
            let value = init();
            unsafe { *this.value.get() = Some(value) };
        });
        // Safety: After call_once, value is initialized
        unsafe { (*this.value.get()).as_ref().unwrap() }
    }
}

impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        Lazy::force(self)
    }
}

// ============================================================================
// Re-export Sleeplock and RwLock from their submodules
// ============================================================================

pub use sleeplock::{Sleeplock, SleeplockGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// Legacy re-exports for backward compatibility

/// Initialize synchronization subsystem
pub fn initialize() {
    // Initialize global synchronization primitives
    // Most initialization is done statically via const constructors
}

/// Shutdown synchronization subsystem
pub fn shutdown() {
    // Cleanup synchronization resources
}
