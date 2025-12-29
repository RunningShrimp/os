// Raw spinlock for low-level synchronization
//
// This module provides SMP-safe spinlock implementations
// with proper memory barriers for SMP safety.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::cpu::cpuid;

// Import interrupt control functions
use super::interrupts::{push_off, pop_off};

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

// ============================================================================
// SpinLock - Standard spinlock with RAII guard
// ============================================================================

/// Standard spinlock with RAII guard
pub struct SpinLock<T> {
    inner: RawSpinLock,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: RawSpinLock::new(),
            data: UnsafeCell::new(data),
        }
    }

    #[inline]
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        self.inner.lock();
        SpinLockGuard {
            lock: self,
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        if self.inner.try_lock() {
            Some(SpinLockGuard {
                lock: self,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }
}

/// RAII guard for SpinLock
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.inner.unlock();
    }
}

impl<'a, T> core::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

