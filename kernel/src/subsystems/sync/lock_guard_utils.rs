//! # Lock Guard Types
//!
//! RAII-style guard types for various lock primitives.
//! Guards automatically release locks when dropped.
//!
//! ## Overview
//!
//! Lock guards provide scoped locking:
//! - Acquire lock on creation
//! - Automatically release on drop
//! - Provide dereference access to protected data
//!
//! ## Guard Types
//!
//! - [`SpinLockGuard`]: Guard for spinlocks
//! - [`MutexGuard`]: Guard for mutexes
//! - [`RwLockReadGuard`]: Guard for read locks
//! - [`RwLockWriteGuard`]: Guard for write locks
//! - [`SleepLockGuard`]: Guard for sleep locks

use core::{
    ops::{Deref, DerefMut},
};

// Re-export guards from subsystems::sync for convenience
pub use crate::subsystems::sync::{
    MutexGuard as MutexGuardIrq,
    MutexIrqGuard,
    SleeplockGuard as SleepLockGuard,
    SpinLockIrqGuard,
    RwLockReadGuard,
    RwLockWriteGuard,
};

/// Basic spinlock guard (when you don't need interrupt control)
pub struct SpinLockGuard<'a, T> {
    lock: &'a crate::subsystems::sync::RawSpinLock,
    data: core::cell::UnsafeCell<T>,
}

impl<'a, T> SpinLockGuard<'a, T> {
    /// Create a new spinlock guard
    ///
    /// # Safety
    /// Caller must ensure the lock is held
    pub unsafe fn new(
        lock: &'a crate::subsystems::sync::RawSpinLock,
        data: core::cell::UnsafeCell<T>,
    ) -> Self {
        Self { lock, data }
    }

    /// Get a reference to the underlying lock
    pub fn lock(&self) -> &'a crate::subsystems::sync::RawSpinLock {
        self.lock
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

/// Type alias for convenience
pub type LockGuard<T> = SpinLockGuard<'static, T>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystems::sync::RawSpinLock;

    #[test]
    fn test_spinlock_guard_deref() {
        let lock = RawSpinLock::new();
        let data = core::cell::UnsafeCell::new(42);

        lock.lock();
        let guard = unsafe { SpinLockGuard::new(&lock, data) };

        assert_eq!(*guard, 42);
        *guard += 1;
        assert_eq!(*guard, 43);

        drop(guard);
        assert!(!lock.is_locked());
    }
}
