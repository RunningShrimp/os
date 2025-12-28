// Mutex implementation with RAII guard
//
// This module provides a mutual exclusion primitive protecting
// data of type T, with RAII guard for automatic
// lock release.

use spin::Mutex;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

// Import RawSpinLock from spinlock module
use super::spinlock::RawSpinLock;

/// A mutual exclusion primitive protecting data of type T
pub struct Mutex<T: ?Sized> {
    lock: RawSpinLock,
    data: UnsafeCell<T>,
}

// Safety: Mutex provides synchronized access
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex protecting of given data
    pub const fn new(data: T) -> Self {
        Self {
            lock: RawSpinLock::new(),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes the mutex and returns the inner data
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires the mutex, blocking until available
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.lock.lock();
        MutexGuard { mutex: self }
    }

    /// Attempts to acquire the mutex without blocking
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.lock.try_lock() {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying data
    /// This is safe because we have &mut self
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// Check if the mutex is currently locked
    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }

    /// Force unlock - unsafe, only use in panic handlers
    /// # Safety
    /// Caller must ensure no other code is using the lock
    pub unsafe fn force_unlock(&self) {
        self.lock.unlock();
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

/// RAII guard for Mutex
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: We hold the lock
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: We hold the lock exclusively
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.lock.unlock();
    }
}

// ============================================================================
// MutexIrq<T> - Mutex that disables interrupts
// ============================================================================

// Import SpinLockIrq from spinlock module
use super::spinlock::SpinLockIrq;

/// A mutex that disables interrupts while held
/// Use this when protected data might be accessed from interrupt handlers
pub struct MutexIrq<T: ?Sized> {
    lock: SpinLockIrq,
    data: UnsafeCell<T>,
}

// Safety: MutexIrq provides synchronized access
unsafe impl<T: ?Sized + Send> Sync for MutexIrq<T> {}
unsafe impl<T: ?Sized + Send> Send for MutexIrq<T> {}

impl<T> MutexIrq<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: SpinLockIrq::new(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> MutexIrq<T> {
    /// Acquires the lock with interrupts disabled
    pub fn lock(&self) -> MutexIrqGuard<'_, T> {
        let guard = self.lock.lock();
        MutexIrqGuard {
            mutex: self,
            _guard: guard,
        }
    }

    pub fn try_lock(&self) -> Option<MutexIrqGuard<'_, T>> {
        self.lock.try_lock().map(|guard| MutexIrqGuard {
            mutex: self,
            _guard: guard,
        })
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }
}

impl<T: ?Sized + Default> Default for MutexIrq<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

/// RAII guard for MutexIrq
pub struct MutexIrqGuard<'a, T: ?Sized> {
    mutex: &'a MutexIrq<T>,
    _guard: <SpinLockIrq as super::spinlock::SpinLockIrqGuard<'a>>,
}

impl<T: ?Sized> Deref for MutexIrqGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.mutex.data.get()
    }
}

impl<T: ?Sized> DerefMut for MutexIrqGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.mutex.data.get()
    }
}

impl<T: ?Sized> Drop for MutexIrqGuard<'_, T> {
    fn drop(&mut self) {
        // SpinLockIrqGuard will automatically restore interrupts on drop
    }
}
