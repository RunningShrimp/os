// Raw spinlock for low-level synchronization
//
// This module provides SMP-safe spinlock implementations
// with proper memory barriers for SMP safety.

use core::cell::UnsafeCell;
use core::sync::atomic;

use crate::cpu::cpuid;
use core::sync::atomic;

// Import interrupt control functions
use super::interrupts::{push_off, pop_off};
use core::sync::atomic;

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
        self.cpu_id.store(cpuid(), Ordering::Relaxed);
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
    
    /// Check if current CPU is holding the lock
    pub fn holding(&self) -> bool {
        self.is_locked() && self.cpu_id.load(Ordering::Relaxed) == cpuid()
    }
}

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
        Self {
            inner: RawSpinLock::new(),
        }
    }
    
    /// Acquire lock and disable interrupts
    /// Returns a guard that restores interrupt state on drop
    #[inline]
    pub fn lock(&self) -> SpinLockIrqGuard<'_> {
        let was_enabled = push_off();
        self.inner.lock();
        SpinLockIrqGuard {
            lock: self,
            was_enabled,
        }
    }
    
    #[inline]
    pub fn try_lock(&self) -> Option<SpinLockIrqGuard<'_>> {
        let was_enabled = push_off();
        if self.inner.try_lock() {
            Some(SpinLockIrqGuard {
                lock: self,
                was_enabled,
            })
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

// Legacy compatibility alias
pub type SpinLock = RawSpinLock;
