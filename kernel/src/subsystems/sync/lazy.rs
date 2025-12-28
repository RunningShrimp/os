#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// Lazy<T, F> - Lazily initialized value
//
// This module provides a value which is initialized on
// first access, similar to std::sync::Lazy.

use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;

/// A value which is initialized on first access
pub struct Lazy<T, F = fn() -> T> {
    once: crate::subsystems::sync::primitives::Once,
    init: UnsafeCell<Option<F>>,
    value: UnsafeCell<Option<T>>,
}

// Safety: Lazy uses Once for synchronization
unsafe impl<T, F> Send for Lazy<T, F> {}
unsafe impl<T, F> Sync for Lazy<T, F> {}

impl<T, F> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Fast path: already initialized
        unsafe {
            if let Some(ref value) = *self.value.get() {
                return value;
            }
        }
        
        // Slow path: need to initialize
        let init = unsafe { &*self.init.get() };
        
        // Call initialization function
        let f: F = unsafe {
            // SAFETY: We know init is Some and F returns T
            init.expect("Lazy::deref called with uninitialized init")()
        };
        
        // Store the initialized value
        *self.value.get() = Some(f());
        
        // Return reference to the value
        unsafe {
            if let Some(ref value) = *self.value.get() {
                return value;
            }
        }
        
        // SAFETY: We just stored Some value above
        self.value.get().expect("Lazy::deref failed after initialization")
    }
}

impl<T: DerefMut, F> DerefMut for Lazy<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Fast path: already initialized
        unsafe {
            if let Some(ref value) = *self.value.get() {
                // SAFETY: We have &mut self, so we can convert &T to &mut T
                return &mut *(value as *const T as *mut T);
            }
        }
        
        // Slow path: need to initialize
        let init = unsafe { &*self.init.get() };
        
        // Call initialization function
        let f: F = unsafe {
            // SAFETY: We know init is Some and F returns T
            init.expect("Lazy::deref_mut called with uninitialized init")()
        };
        
        // Store the initialized value
        *self.value.get() = Some(f());
        
        // Return mutable reference to the value
        unsafe {
            if let Some(ref value) = *self.value.get() {
                // SAFETY: We have &mut self, so we can convert &T to &mut T
                return &mut *(value as *const T as *mut T);
            }
        }
        
        // SAFETY: We just stored Some value above
        self.value.get().expect("Lazy::deref_mut failed after initialization")
    }
}
