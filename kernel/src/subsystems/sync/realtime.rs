//! Real-time synchronization primitives
//!
//! Implements real-time locks with priority inheritance to prevent priority inversion.
//! These locks are designed for PREEMPT_RT kernel support.

extern crate alloc;

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use crate::reliability::errno::{EINVAL, EDEADLK};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Real-time mutex with priority inheritance
pub struct RtMutex {
    /// Lock state (true = locked, false = unlocked)
    locked: AtomicBool,
    /// Current owner thread ID (0 = no owner)
    owner: AtomicU8,
    /// Waiting threads queue (ordered by priority)
    waiters: crate::subsystems::sync::Mutex<Vec<Waiter>>,
    /// Priority inheritance: highest priority waiter
    inherited_prio: AtomicU8,
}

/// Waiter information
struct Waiter {
    /// Thread ID waiting for the lock
    tid: u8,
    /// Thread priority
    priority: u8,
}

impl RtMutex {
    /// Create a new real-time mutex
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            owner: AtomicU8::new(0),
            waiters: crate::subsystems::sync::Mutex::new(Vec::new()),
            inherited_prio: AtomicU8::new(0),
        }
    }

    /// Lock the mutex with priority inheritance
    pub fn lock(&self, tid: u8, priority: u8) -> Result<(), i32> {
        // Try to acquire lock immediately
        if self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            self.owner.store(tid, Ordering::Release);
            return Ok(());
        }

        // Lock is held - check for deadlock
        let current_owner = self.owner.load(Ordering::Acquire);
        if current_owner == tid {
            return Err(EDEADLK);
        }

        // Add to waiters queue
        {
            let mut waiters = self.waiters.lock();
            waiters.push(Waiter { tid, priority });
            // Sort by priority (higher priority first)
            waiters.sort_by(|a, b| b.priority.cmp(&a.priority));
            
            // Update inherited priority
            if let Some(highest) = waiters.first() {
                self.inherited_prio.store(highest.priority, Ordering::Release);
                
                // Boost owner's priority if needed
                if current_owner != 0 {
                    self.boost_owner_priority(current_owner, highest.priority);
                }
            }
        }

        // Wait for lock (in real implementation, this would block)
        // For now, spin-wait with priority awareness
        while self.locked.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }

        // Acquire lock
        if self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            self.owner.store(tid, Ordering::Release);
            
            // Remove from waiters
            {
                let mut waiters = self.waiters.lock();
                waiters.retain(|w| w.tid != tid);
                
                // Update inherited priority
                if let Some(highest) = waiters.first() {
                    self.inherited_prio.store(highest.priority, Ordering::Release);
                } else {
                    self.inherited_prio.store(0, Ordering::Release);
                }
            }
            
            Ok(())
        } else {
            Err(EINVAL)
        }
    }

    /// Unlock the mutex
    pub fn unlock(&self, tid: u8) -> Result<(), i32> {
        let current_owner = self.owner.load(Ordering::Acquire);
        if current_owner != tid {
            return Err(EINVAL);
        }

        // Restore original priority if it was boosted
        if self.inherited_prio.load(Ordering::Acquire) > 0 {
            self.restore_owner_priority(tid);
        }

        // Release lock
        self.owner.store(0, Ordering::Release);
        self.locked.store(false, Ordering::Release);

        // Wake up highest priority waiter
        {
            let mut waiters = self.waiters.lock();
            if let Some(waiter) = waiters.pop() {
                // In real implementation, wake up the thread
                // For now, just remove from queue
            }
        }

        Ok(())
    }

    /// Try to lock without blocking
    pub fn try_lock(&self, tid: u8) -> Result<bool, i32> {
        if self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            self.owner.store(tid, Ordering::Release);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Boost owner's priority to match highest waiter
    fn boost_owner_priority(&self, owner_tid: u8, new_priority: u8) {
        // In real implementation, this would update the thread's priority
        // For now, just log
        crate::println!("[rt] Boosting owner {} priority to {}", owner_tid, new_priority);
    }

    /// Restore owner's original priority
    fn restore_owner_priority(&self, owner_tid: u8) {
        // In real implementation, this would restore the thread's original priority
        crate::println!("[rt] Restoring owner {} priority", owner_tid);
    }
}

impl Default for RtMutex {
    fn default() -> Self {
        Self::new()
    }
}

/// Real-time spinlock with priority inheritance
pub struct RtSpinLock {
    /// Lock state
    locked: AtomicBool,
    /// Current owner thread ID
    owner: AtomicU8,
    /// Inherited priority
    inherited_prio: AtomicU8,
}

impl RtSpinLock {
    /// Create a new real-time spinlock
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            owner: AtomicU8::new(0),
            inherited_prio: AtomicU8::new(0),
        }
    }

    /// Lock with priority inheritance
    pub fn lock(&self, tid: u8, priority: u8) {
        // Spin until lock is acquired
        while self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            // Check if we should boost owner's priority
            let current_owner = self.owner.load(Ordering::Acquire);
            if current_owner != 0 && current_owner != tid {
                let current_prio = self.inherited_prio.load(Ordering::Acquire);
                if priority > current_prio {
                    self.inherited_prio.store(priority, Ordering::Release);
                    self.boost_owner_priority(current_owner, priority);
                }
            }
            
            core::hint::spin_loop();
        }
        
        self.owner.store(tid, Ordering::Release);
    }

    /// Unlock
    pub fn unlock(&self, tid: u8) {
        let current_owner = self.owner.load(Ordering::Acquire);
        if current_owner == tid {
            // Restore priority if boosted
            if self.inherited_prio.load(Ordering::Acquire) > 0 {
                self.restore_owner_priority(tid);
                self.inherited_prio.store(0, Ordering::Release);
            }
            
            self.owner.store(0, Ordering::Release);
            self.locked.store(false, Ordering::Release);
        }
    }

    /// Boost owner's priority
    fn boost_owner_priority(&self, owner_tid: u8, new_priority: u8) {
        crate::println!("[rt] Boosting spinlock owner {} priority to {}", owner_tid, new_priority);
    }

    /// Restore owner's priority
    fn restore_owner_priority(&self, owner_tid: u8) {
        crate::println!("[rt] Restoring spinlock owner {} priority", owner_tid);
    }
}

impl Default for RtSpinLock {
    fn default() -> Self {
        Self::new()
    }
}

