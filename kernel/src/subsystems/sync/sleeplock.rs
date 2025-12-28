#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// Sleeplock - Lock that allows sleeping
//
// This module provides a lock that can be held during
// sleeping operations (for I/O operations).
//
// Unlike spinlocks, sleeplocks yield the CPU while waiting.

use core::cell::UnsafeCell;
use core::sync::atomic;

use crate::process::thread::current_thread;
use core::sync::atomic;
use crate::process::sleep;
use core::sync::atomic;

/// A lock that can be held during sleeping operations
/// Unlike spinlocks, sleeplocks yield the CPU while waiting
pub struct Sleeplock<T: ?Sized> {
    locked: AtomicBool,
    // Process ID of lock holder (0 if unlocked)
    holder: AtomicUsize,
    data: UnsafeCell<T>,
}

// Safety: Sleeplock provides synchronized access
unsafe impl<T: ?Sized + Send> Sync for Sleeplock<T> {}
unsafe impl<T: ?Sized + Send> Send for Sleeplock<T> {}

impl<T> Sleeplock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            holder: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T> Sleeplock<T> {
    /// Acquire the sleeplock
    /// In a full implementation, this would sleep instead of spin
    /// For now, use a simple spin with yield to reduce CPU usage
    pub fn lock(&self) -> SleeplockGuard<'_, T> {
        // TODO: Implement proper sleep/wakeup when scheduler is ready
        // For now, use a simple spin with yield to reduce CPU usage
        let mut spin_count = 0;
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // In real implementation: yield CPU and sleep
            // For now: simple spin loop hint
            core::hint::spin_loop();
            spin_count += 1;

            // After many spins, yield to reduce CPU contention
            if spin_count > 1000 {
                // TODO: Call scheduler yield when available
                spin_count = 0;
            }
        }
        SleeplockGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<SleeplockGuard<'_, T>> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SleeplockGuard { lock: self })
        } else {
            None
        }
    }

    pub fn holding(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }
}

/// RAII guard for Sleeplock
pub struct SleeplockGuard<'a, T: ?Sized> {
    lock: &'a Sleeplock<T>,
}

impl<T: ?Sized> Deref for SleeplockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SleeplockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for SleeplockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        // TODO: Wakeup waiting processes when scheduler is ready
        // This would involve calling the scheduler to wakeup processes waiting on this lock
        crate::println!("[sync] SleepLock released - would wakeup waiting processes");
    }
}
