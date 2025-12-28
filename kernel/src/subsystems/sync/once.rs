#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// Once - One-time initialization primitive
//
// This module provides a synchronization primitive for
// one-time initialization, similar to std::sync::Once.

const ONCE_INCOMPLETE: usize = 0;
const ONCE_RUNNING: usize = 1;
const ONCE_COMPLETE: usize = 2;

/// A synchronization primitive for one-time initialization
pub struct Once {
    state: AtomicUsize,
}

impl Once {
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(ONCE_INCOMPLETE),
        }
    }

    /// Executes the given closure exactly once
    /// Multiple calls to call_once() with the same closure will only execute once
    pub fn call_once<F>(&self, f: F) -> F::Output
    where
        F: FnOnce(),
    {
        // Check if already completed
        let mut state = self.state.load(Ordering::Acquire);
        if state == ONCE_COMPLETE {
            panic!("Once::call_once called twice");
        }

        // Try to set to running
        if state == ONCE_INCOMPLETE
            && self.state.compare_exchange_weak(
                ONCE_INCOMPLETE,
                ONCE_RUNNING,
                Ordering::Acquire,
                Ordering::Relaxed,
            ).is_ok()
        {
            // We're the one to run the closure
            f();
            
            // Mark as complete
            self.state.store(ONCE_COMPLETE, Ordering::Release);
        } else {
            // Someone else is running, wait for them to complete
            while self.state.load(Ordering::Acquire) != ONCE_COMPLETE {
                core::hint::spin_loop();
            }
        }
    }

    /// Check if the Once has been called
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == ONCE_COMPLETE
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}
