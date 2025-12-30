// Lazy<T, F> - Lazily initialized value
//
// This module provides a value which is initialized on
// first access, similar to std::sync::Lazy.

use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;

/// A value which is initialized on first access
pub struct Lazy<T, F = fn() -> T> {
    once: super::Once,
    init: UnsafeCell<Option<F>>,
    value: UnsafeCell<Option<T>>,
}

// Safety: Lazy uses Once for synchronization
unsafe impl<T, F> Send for Lazy<T, F> {}
unsafe impl<T, F> Sync for Lazy<T, F> {}

impl<T, F> Lazy<T, F>
where
    F: FnOnce() -> T,
{
    /// Create a new lazy value with the given initialization function
    pub const fn new(f: F) -> Self {
        Self {
            once: super::Once::new(),
            init: UnsafeCell::new(Some(f)),
            value: UnsafeCell::new(None),
        }
    }
}

impl<T, F> Deref for Lazy<T, F>
where
    F: FnOnce() -> T,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Fast path: already initialized
        unsafe {
            if let Some(ref value) = *self.value.get() {
                return value;
            }
        }

        // Slow path: need to initialize
        self.once.call_once(|| {
            // Safety: We're inside call_once, so only one thread runs this
            let init = unsafe { (*self.init.get()).take().expect("Lazy::deref called with uninitialized init") };
            let value = init();
            unsafe { *self.value.get() = Some(value) };
        });

        // Safety: After call_once, value is initialized
        unsafe { (*self.value.get()).as_ref().expect("Lazy::deref failed after initialization") }
    }
}

impl<T: DerefMut, F> DerefMut for Lazy<T, F>
where
    F: FnOnce() -> T,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Fast path: already initialized
        // SAFETY: We have &mut self, which guarantees exclusive access to the Lazy instance.
        // This means we can safely get a mutable reference to the contents of the UnsafeCell.
        unsafe {
            if let Some(ref mut value) = *self.value.get() {
                return value;
            }
        }

        // Slow path: need to initialize
        self.once.call_once(|| {
            // Safety: We're inside call_once, so only one thread runs this
            let init = unsafe { (*self.init.get()).take().expect("Lazy::deref_mut called with uninitialized init") };
            let value = init();
            unsafe { *self.value.get() = Some(value) };
        });

        // SAFETY: After call_once, value is initialized. We have &mut self which guarantees
        // exclusive access, so we can safely return a mutable reference.
        unsafe {
            if let Some(ref mut value) = *self.value.get() {
                return value;
            }
        }

        // SAFETY: We just stored Some value above, so we know it's Some
        unsafe { &mut *self.value.get() }.as_mut().expect("Lazy::deref_mut failed after initialization");
        unreachable!()
    }
}
