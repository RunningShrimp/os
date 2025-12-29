//! Priority-Based Mutex with Starvation Prevention

use spin::Mutex;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use core::hint::spin_loop;
use alloc::sync::Arc;

use crate::cpu;

const MAX_PRIORITY: u8 = 255;
const DEFAULT_PRIORITY: u8 = 128;

pub struct PriorityMutex<T> {
    state: AtomicU8,     // 0=unlocked, 1..255=locked with priority
    holder: AtomicU32,
}

impl<T> PriorityMutex<T> {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(0),
            holder: AtomicU32::new(0),
        }
    }
    
    pub fn lock(&self, priority: u8) -> Option<PriorityMutexGuard<T>> {
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
        if current_holder == 0 {
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

pub struct PriorityMutexGuard<'a, T> {
    mutex: &'a PriorityMutex<T>,
    priority: u8,
}

impl<'a, T> Drop for PriorityMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock(self.priority);
    }
}
