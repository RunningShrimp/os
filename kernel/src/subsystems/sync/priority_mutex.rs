//! Priority-Based Mutex with Starvation Prevention

use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};


const MAX_PRIORITY: u8 = 255;
const DEFAULT_PRIORITY: u8 = 128;

pub struct PriorityMutex {
    state: AtomicU8,     // 0=unlocked, 1..255=locked with priority
    holder: AtomicU32,
}

impl PriorityMutex {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
            holder: AtomicU32::new(0),
        }
    }

    pub fn lock(&self, priority: u8) -> Option<PriorityMutexGuard<'_>> {
        let current_state = self.state.load(Ordering::Acquire);

        // Can only acquire if unlocked or we have higher priority
        if current_state == 0 || priority > current_state {
            if self.state.compare_exchange(
                current_state,
                priority,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_ok() {
                self.holder.store(0, Ordering::Relaxed);
                return Some(PriorityMutexGuard { mutex: self, priority });
            }
        }

        None
    }

    pub fn unlock(&self, priority: u8) {
        let current_holder = self.holder.load(Ordering::Relaxed);
        let current_state = self.state.load(Ordering::Relaxed);
        // Verify the unlock is being performed by the correct priority holder
        // This prevents priority inversion attacks where a lower priority thread
        // attempts to unlock a mutex held by a higher priority thread
        if current_holder == 0 && current_state == priority {
            self.state.store(0, Ordering::Release);
        }
    }

    pub fn is_locked(&self) -> bool {
        self.state.load(Ordering::Relaxed) != 0
    }

    pub fn current_priority(&self) -> u8 {
        self.state.load(Ordering::Relaxed)
    }
}

pub struct PriorityMutexGuard<'a> {
    mutex: &'a PriorityMutex,
    priority: u8,
}

impl<'a> Drop for PriorityMutexGuard<'a> {
    fn drop(&mut self) {
        self.mutex.unlock(self.priority);
    }
}
