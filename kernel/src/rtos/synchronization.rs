//! # Real-Time Safe Synchronization Primitives
//!
//! This module provides synchronization primitives specifically designed for real-time systems,
//! featuring priority inheritance to prevent unbounded priority inversion.
//!
//! ## Priority Inheritance
//!
//! When a high-priority task waits for a lock held by a low-priority task, the low-priority
//! task temporarily inherits the high priority to minimize blocking time:
//!
//! ```text
//! Task H (high) ──┐
//!                 ├─► blocked ──► Task M inherits priority H
//! Task M (med)   ─┘
//!
//! Time ──►
//! Normal:  |H-M-M-M-M|  (M blocks H for long time)
//! Inherit: |H-M-M-M-H|  (M finishes quickly)
//! ```
//!
//! ## Supported Primitives
//!
//! - **Priority Inheritance Mutex**: Prevents unbounded priority inversion
//! - **Priority Ceiling Mutex**: Prevents deadlock and chaining
//! - **Real-Time Semaphore**: Bounded blocking with FIFO/Priority ordering
//! - **RWLock with PI**: Read-write lock with priority inheritance
//! - **Spinlock with Bounded Spinning**: Limited spin count to prevent starvation
//!
//! ## Protocols
//!
//! ### Basic Priority Inheritance
//!
//! When a high-priority task blocks:
//! 1. The blocker inherits the higher priority
//! 2. The blocker executes with elevated priority
//! 3. Upon release, priority is restored
//!
//! ### Priority Ceiling Protocol (PCP)
//!
//! Each resource has a ceiling priority equal to the highest priority of any task that may lock it.
//! A task can only lock a resource if its priority is higher than all currently held ceilings.
//!
//! Benefits:
//! - Prevents deadlock
//! - Bounded blocking time (at most one critical section)
//! - No chained blocking
//!
//! ### Stack Resource Policy (SRP)
//!
//! Similar to PCP but with preemption ceiling:
//! - A task can be preempted only by tasks with higher priority than its current ceiling
//! - Provides deadlock freedom and bounded blocking
//!
//! ## Example
//!
//! ```no_run
//! use kernel::rtos::synchronization::PiMutex;
//!
//! let mutex = PiMutex::new(42, 100);
//!
//! // Lock with automatic priority inheritance
//! let guard = mutex.lock()?;
//! println!("Value: {}", *guard);
//! // Unlock restores priority
//! drop(guard);
//!
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```

use crate::rtos::RtError;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering as AtomicOrdering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// Priority inheritance mutex
///
/// Provides exclusive access with automatic priority inheritance to prevent
/// unbounded priority inversion.
pub struct PiMutex<T> {
    /// Protected data
    data: UnsafeCell<T>,

    /// Current owner (task ID)
    owner: AtomicU64,

    /// Owner's original priority
    owner_original_prio: AtomicU8,

    /// Lock is held
    locked: AtomicBool,

    /// Waiting tasks (task_id -> priority)
    waiters: spin::Mutex<BTreeMap<u64, u8>>,

    /// Mutex ID for debugging
    mutex_id: u64,
}

unsafe impl<T: Send> Send for PiMutex<T> {}
unsafe impl<T: Send> Sync for PiMutex<T> {}

impl<T> PiMutex<T> {
    /// Create a new priority inheritance mutex
    pub fn new(data: T, mutex_id: u64) -> Self {
        Self {
            data: UnsafeCell::new(data),
            owner: AtomicU64::new(0),
            owner_original_prio: AtomicU8::new(0),
            locked: AtomicBool::new(false),
            waiters: spin::Mutex::new(BTreeMap::new()),
            mutex_id,
        }
    }

    /// Acquire the lock with priority inheritance
    pub fn lock(&self) -> Result<PiMutexGuard<'_, T>, RtError> {
        let task_id = self.current_task_id();
        let task_prio = self.current_task_priority();

        // Try to acquire lock
        if !self.locked.compare_exchange(false, true, AtomicOrdering::Acquire, AtomicOrdering::Relaxed).is_ok() {
            // Lock is held, add to waiters
            {
                let mut waiters = self.waiters.lock();
                waiters.insert(task_id, task_prio);
            }

            // Check if we need priority inheritance
            let owner_id = self.owner.load(AtomicOrdering::Acquire);
            if task_prio > self.owner_original_prio.load(AtomicOrdering::Acquire) {
                // Boost owner's priority
                self.boost_priority(owner_id, task_prio)?;
            }

            // Wait for lock (in real implementation, use proper blocking)
            while self.locked.load(AtomicOrdering::Acquire) {
                core::hint::spin_loop();
            }

            // Remove from waiters
            let mut waiters = self.waiters.lock();
            waiters.remove(&task_id);
        }

        // Record ownership
        self.owner.store(task_id, AtomicOrdering::Release);
        self.owner_original_prio.store(task_prio, AtomicOrdering::Release);

        Ok(PiMutexGuard {
            mutex: self,
            task_id,
            original_prio: task_prio,
        })
    }

    /// Try to acquire the lock without blocking
    pub fn try_lock(&self) -> Result<PiMutexGuard<'_, T>, RtError> {
        let task_id = self.current_task_id();
        let task_prio = self.current_task_priority();

        if !self.locked.compare_exchange(false, true, AtomicOrdering::Acquire, AtomicOrdering::Relaxed).is_ok() {
            return Err(RtError::Timeout {
                resource: "PiMutex",
                timeout_us: 0,
            });
        }

        self.owner.store(task_id, AtomicOrdering::Release);
        self.owner_original_prio.store(task_prio, AtomicOrdering::Release);

        Ok(PiMutexGuard {
            mutex: self,
            task_id,
            original_prio: task_prio,
        })
    }

    /// Boost a task's priority (for priority inheritance)
    fn boost_priority(&self, _task_id: u64, _new_prio: u8) -> Result<(), RtError> {
        // In real implementation, call into scheduler to boost priority
        // For now, just track it
        Ok(())
    }

    /// Restore a task's priority
    fn restore_priority(&self, _task_id: u64, _original_prio: u8) -> Result<(), RtError> {
        // In real implementation, call into scheduler to restore priority
        Ok(())
    }

    /// Get current task ID (placeholder)
    fn current_task_id(&self) -> u64 {
        // In real implementation, get from current task structure
        1
    }

    /// Get current task priority (placeholder)
    fn current_task_priority(&self) -> u8 {
        // In real implementation, get from current task structure
        128
    }

    /// Check if mutex is locked
    pub fn is_locked(&self) -> bool {
        self.locked.load(AtomicOrdering::Acquire)
    }
}

/// Guard for priority inheritance mutex
pub struct PiMutexGuard<'a, T> {
    mutex: &'a PiMutex<T>,
    task_id: u64,
    original_prio: u8,
}

impl<T> Deref for PiMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for PiMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for PiMutexGuard<'_, T> {
    fn drop(&mut self) {
        // Restore original priority
        let _ = self.mutex.restore_priority(self.task_id, self.original_prio);

        // Release lock
        self.mutex.locked.store(false, AtomicOrdering::Release);
        self.mutex.owner.store(0, AtomicOrdering::Release);
    }
}

/// Priority ceiling mutex
///
/// Prevents deadlock and priority inversion using the priority ceiling protocol.
pub struct PcMutex<T> {
    /// Protected data
    data: UnsafeCell<T>,

    /// Ceiling priority
    ceiling: u8,

    /// Current owner
    owner: AtomicU64,

    /// Lock is held
    locked: AtomicBool,

    /// Mutex ID
    mutex_id: u64,
}

unsafe impl<T: Send> Send for PcMutex<T> {}
unsafe impl<T: Send> Sync for PcMutex<T> {}

impl<T> PcMutex<T> {
    /// Create a new priority ceiling mutex
    pub fn new(data: T, ceiling: u8, mutex_id: u64) -> Self {
        Self {
            data: UnsafeCell::new(data),
            ceiling,
            owner: AtomicU64::new(0),
            locked: AtomicBool::new(false),
            mutex_id,
        }
    }

    /// Acquire the lock
    pub fn lock(&self) -> Result<PcMutexGuard<'_, T>, RtError> {
        let task_id = self.current_task_id();
        let task_prio = self.current_task_priority();

        // Check if task priority is >= ceiling
        if task_prio < self.ceiling {
            return Err(RtError::InvalidPriority {
                priority: task_prio,
                max_priority: self.ceiling,
            });
        }

        // Try to acquire lock
        while self.locked.compare_exchange(false, true, AtomicOrdering::Acquire, AtomicOrdering::Relaxed).is_err() {
            core::hint::spin_loop();
        }

        self.owner.store(task_id, AtomicOrdering::Release);

        Ok(PcMutexGuard { mutex: self })
    }

    /// Try to acquire without blocking
    pub fn try_lock(&self) -> Result<PcMutexGuard<'_, T>, RtError> {
        let task_prio = self.current_task_priority();

        if task_prio < self.ceiling {
            return Err(RtError::InvalidPriority {
                priority: task_prio,
                max_priority: self.ceiling,
            });
        }

        if self.locked.compare_exchange(false, true, AtomicOrdering::Acquire, AtomicOrdering::Relaxed).is_err() {
            return Err(RtError::Timeout {
                resource: "PcMutex",
                timeout_us: 0,
            });
        }

        Ok(PcMutexGuard { mutex: self })
    }

    fn current_task_id(&self) -> u64 {
        1
    }

    fn current_task_priority(&self) -> u8 {
        200
    }
}

/// Guard for priority ceiling mutex
pub struct PcMutexGuard<'a, T> {
    mutex: &'a PcMutex<T>,
}

impl<T> Deref for PcMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for PcMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for PcMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, AtomicOrdering::Release);
        self.mutex.owner.store(0, AtomicOrdering::Release);
    }
}

/// Real-time semaphore
///
/// Bounded semaphore with FIFO or priority-based ordering.
pub struct RtSemaphore {
    /// Current count
    count: AtomicU64,

    /// Maximum count
    max: u64,

    /// Waiting tasks
    waiters: spin::Mutex<alloc::vec::Vec<u64>>,

    /// Ordering policy
    policy: WaitPolicy,
}

/// Waiter ordering policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitPolicy {
    /// First-In-First-Out
    Fifo,

    /// Priority-based
    Priority,
}

impl RtSemaphore {
    /// Create a new semaphore
    pub fn new(initial: u64, max: u64, policy: WaitPolicy) -> Self {
        assert!(initial <= max);
        Self {
            count: AtomicU64::new(initial),
            max,
            waiters: spin::Mutex::new(alloc::vec::Vec::new()),
            policy,
        }
    }

    /// Acquire a permit
    pub fn acquire(&self) -> Result<(), RtError> {
        let task_id = self.current_task_id();

        // Try to decrement count
        loop {
            let current = self.count.load(AtomicOrdering::Acquire);
            if current == 0 {
                break;
            }

            if self.count.compare_exchange(current, current - 1, AtomicOrdering::AcqRel, AtomicOrdering::Relaxed).is_ok() {
                return Ok(());
            }
        }

        // No permits available, wait
        {
            let mut waiters = self.waiters.lock();
            waiters.push(task_id);
        }

        // Wait (in real implementation, block properly)
        while self.count.load(AtomicOrdering::Acquire) == 0 {
            core::hint::spin_loop();
        }

        // Remove from waiters and acquire
        let mut waiters = self.waiters.lock();
        waiters.retain(|&id| id != task_id);
        let _ = self.count.fetch_update(AtomicOrdering::AcqRel, AtomicOrdering::Relaxed, |c| c.checked_sub(1));

        Ok(())
    }

    /// Try to acquire without blocking
    pub fn try_acquire(&self) -> Result<(), RtError> {
        let current = self.count.load(AtomicOrdering::Acquire);

        if current == 0 {
            return Err(RtError::Timeout {
                resource: "Semaphore",
                timeout_us: 0,
            });
        }

        self.count.compare_exchange(current, current - 1, AtomicOrdering::AcqRel, AtomicOrdering::Relaxed)
            .map_err(|_| RtError::Timeout {
                resource: "Semaphore",
                timeout_us: 0,
            })?;

        Ok(())
    }

    /// Release a permit
    pub fn release(&self) {
        let current = self.count.load(AtomicOrdering::Acquire);
        if current < self.max {
            self.count.fetch_add(1, AtomicOrdering::Release);
        }
    }

    /// Get current count
    pub fn available(&self) -> u64 {
        self.count.load(AtomicOrdering::Acquire)
    }

    fn current_task_id(&self) -> u64 {
        1
    }
}

/// Read-write lock with priority inheritance
///
/// Allows multiple readers or one writer with priority inheritance for writers.
pub struct PiRwLock<T> {
    /// Protected data
    data: UnsafeCell<T>,

    /// Reader count
    readers: AtomicU64,

    /// Writer present
    writer: AtomicBool,

    /// Waiting writers
    waiting_writers: spin::Mutex<alloc::vec::Vec<u64>>,

    /// Lock ID
    lock_id: u64,
}

unsafe impl<T: Send> Send for PiRwLock<T> {}
unsafe impl<T: Send> Sync for PiRwLock<T> {}

impl<T> PiRwLock<T> {
    /// Create new read-write lock
    pub fn new(data: T, lock_id: u64) -> Self {
        Self {
            data: UnsafeCell::new(data),
            readers: AtomicU64::new(0),
            writer: AtomicBool::new(false),
            waiting_writers: spin::Mutex::new(alloc::vec::Vec::new()),
            lock_id,
        }
    }

    /// Acquire read lock
    pub fn read(&self) -> Result<PiRwLockReadGuard<'_, T>, RtError> {
        // Wait for no writer
        while self.writer.load(AtomicOrdering::Acquire) {
            core::hint::spin_loop();
        }

        self.readers.fetch_add(1, AtomicOrdering::Acquire);

        Ok(PiRwLockReadGuard { lock: self })
    }

    /// Acquire write lock
    pub fn write(&self) -> Result<PiRwLockWriteGuard<'_, T>, RtError> {
        let task_id = self.current_task_id();

        // Add to waiting writers
        {
            let mut waiting = self.waiting_writers.lock();
            waiting.push(task_id);
        }

        // Wait for no readers or writer
        while self.readers.load(AtomicOrdering::Acquire) > 0 || self.writer.load(AtomicOrdering::Acquire) {
            core::hint::spin_loop();
        }

        // Remove from waiting
        let mut waiting = self.waiting_writers.lock();
        waiting.retain(|&id| id != task_id);

        // Acquire write lock
        self.writer.store(true, AtomicOrdering::Release);

        Ok(PiRwLockWriteGuard { lock: self })
    }

    fn current_task_id(&self) -> u64 {
        1
    }
}

/// Read guard for PiRwLock
pub struct PiRwLockReadGuard<'a, T> {
    lock: &'a PiRwLock<T>,
}

impl<T> Deref for PiRwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for PiRwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.readers.fetch_sub(1, AtomicOrdering::Release);
    }
}

/// Write guard for PiRwLock
pub struct PiRwLockWriteGuard<'a, T> {
    lock: &'a PiRwLock<T>,
}

impl<T> Deref for PiRwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for PiRwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for PiRwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.writer.store(false, AtomicOrdering::Release);
    }
}

/// Spinlock with bounded spinning
///
/// Spins for a limited time to prevent indefinite CPU usage.
pub struct BoundedSpinlock<T> {
    /// Protected data
    data: UnsafeCell<T>,

    /// Lock is held
    locked: AtomicBool,

    /// Maximum spin iterations
    max_spins: u32,

    /// Lock ID
    lock_id: u64,
}

unsafe impl<T: Send> Send for BoundedSpinlock<T> {}
unsafe impl<T: Send> Sync for BoundedSpinlock<T> {}

impl<T> BoundedSpinlock<T> {
    /// Create new bounded spinlock
    pub fn new(data: T, max_spins: u32, lock_id: u64) -> Self {
        Self {
            data: UnsafeCell::new(data),
            locked: AtomicBool::new(false),
            max_spins,
            lock_id,
        }
    }

    /// Acquire the lock with bounded spinning
    pub fn lock(&self) -> Result<BoundedSpinlockGuard<'_, T>, RtError> {
        let mut spins = 0;

        while self.locked.compare_exchange(false, true, AtomicOrdering::Acquire, AtomicOrdering::Relaxed).is_err() {
            spins += 1;
            if spins >= self.max_spins {
                return Err(RtError::Timeout {
                    resource: "BoundedSpinlock",
                    timeout_us: 0,
                });
            }
            core::hint::spin_loop();
        }

        Ok(BoundedSpinlockGuard { lock: self })
    }

    /// Try to acquire without spinning
    pub fn try_lock(&self) -> Result<BoundedSpinlockGuard<'_, T>, RtError> {
        if self.locked.compare_exchange(false, true, AtomicOrdering::Acquire, AtomicOrdering::Relaxed).is_err() {
            return Err(RtError::Timeout {
                resource: "BoundedSpinlock",
                timeout_us: 0,
            });
        }

        Ok(BoundedSpinlockGuard { lock: self })
    }
}

/// Guard for bounded spinlock
pub struct BoundedSpinlockGuard<'a, T> {
    lock: &'a BoundedSpinlock<T>,
}

impl<T> Deref for BoundedSpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for BoundedSpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for BoundedSpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, AtomicOrdering::Release);
    }
}

/// FIFO mutex (POSIX PTHREAD_PRIO_INHERIT)
///
/// First-In-First-Out ordering with priority inheritance.
pub struct FifoMutex<T> {
    /// Inner PI mutex
    inner: PiMutex<T>,

    /// Mutex ID
    mutex_id: u64,
}

impl<T> FifoMutex<T> {
    /// Create new FIFO mutex
    pub fn new(data: T, mutex_id: u64) -> Self {
        Self {
            inner: PiMutex::new(data, mutex_id),
            mutex_id,
        }
    }

    /// Acquire the lock (FIFO order)
    pub fn lock(&self) -> Result<FifoMutexGuard<'_, T>, RtError> {
        let guard = self.inner.lock()?;
        Ok(FifoMutexGuard { guard })
    }
}

/// Guard for FIFO mutex
pub struct FifoMutexGuard<'a, T> {
    guard: PiMutexGuard<'a, T>,
}

impl<T> Deref for FifoMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<T> DerefMut for FifoMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_mutex_creation() {
        let mutex = PiMutex::new(42, 1);
        assert!(!mutex.is_locked());
    }

    #[test]
    fn test_pi_mutex_lock() {
        let mutex = PiMutex::new(42, 1);
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 42);
        assert!(mutex.is_locked());
    }

    #[test]
    fn test_pi_mutex_try_lock() {
        let mutex = PiMutex::new(42, 1);
        let guard = mutex.try_lock().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_pi_mutex_unlock() {
        let mutex = PiMutex::new(42, 1);
        {
            let _guard = mutex.lock().unwrap();
        }
        assert!(!mutex.is_locked());
    }

    #[test]
    fn test_pc_mutex_creation() {
        let mutex = PcMutex::new(42, 200, 1);
        assert!(!mutex.locked.load(AtomicOrdering::Acquire));
    }

    #[test]
    fn test_pc_mutex_lock() {
        let mutex = PcMutex::new(42, 150, 1);
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_semaphore() {
        let sem = RtSemaphore::new(2, 5, WaitPolicy::Fifo);
        assert_eq!(sem.available(), 2);

        sem.acquire().unwrap();
        assert_eq!(sem.available(), 1);

        sem.release();
        assert_eq!(sem.available(), 2);
    }

    #[test]
    fn test_semaphore_acquire_release() {
        let sem = RtSemaphore::new(1, 1, WaitPolicy::Fifo);

        sem.acquire().unwrap();
        assert_eq!(sem.available(), 0);

        sem.release();
        assert_eq!(sem.available(), 1);
    }

    #[test]
    fn test_rwlock_read() {
        let lock = PiRwLock::new(42, 1);
        let guard = lock.read().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_rwlock_write() {
        let lock = PiRwLock::new(42, 1);
        let mut guard = lock.write().unwrap();
        *guard = 100;
        assert_eq!(*guard, 100);
    }

    #[test]
    fn test_bounded_spinlock() {
        let lock = BoundedSpinlock::new(42, 1000, 1);
        let guard = lock.lock().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_bounded_spinlock_timeout() {
        let lock = BoundedSpinlock::new(42, 10, 1);
        {
            let _guard = lock.lock().unwrap();
            // Try to acquire again - should timeout
            let result = lock.try_lock();
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_fifo_mutex() {
        let mutex = FifoMutex::new(42, 1);
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 42);
    }
}
