//! OnceLock - One-time initialization with a value.
//!
//! This module provides a synchronization primitive which can be used to run
//! a one-time global initialization. Unlike `Once`, it allows returning a value.

use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU8, Ordering},
};

const ONCE_STATE_UNINIT: u8 = 0;
const ONCE_STATE_INITIALIZING: u8 = 1;
const ONCE_STATE_INITIALIZED: u8 = 2;

/// A synchronization primitive which can be used to run a one-time global
/// initialization. Unlike `Once`, it allows returning a value.
///
/// # Examples
///
/// ```
/// use kernel::subsystems::sync::OnceLock;
///
/// static CELL: OnceLock<usize> = OnceLock::new();
///
/// // Initialize the cell
/// CELL.get_or_init(|| 42);
///
/// assert_eq!(CELL.get(), Some(&42));
/// ```
pub struct OnceLock<T> {
    state: AtomicU8,
    data: UnsafeCell<Option<T>>,
}

// Safety: OnceLock provides synchronized access
unsafe impl<T: Send> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    /// Creates a new `OnceLock`.
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(ONCE_STATE_UNINIT),
            data: UnsafeCell::new(None),
        }
    }

    /// Gets the reference to the underlying value if initialized.
    ///
    /// Returns `None` if the value is not yet initialized.
    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) == ONCE_STATE_INITIALIZED {
            // Safety: We've checked the state
            unsafe { (&*self.data.get()).as_ref() }
        } else {
            None
        }
    }

    /// Gets the mutable reference to the underlying value if initialized.
    ///
    /// Returns `None` if the value is not yet initialized.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.state.load(Ordering::Acquire) == ONCE_STATE_INITIALIZED {
            // Safety: We have &mut self
            unsafe { (&mut *self.data.get()).as_mut() }
        } else {
            None
        }
    }

    /// Sets the value if it's not yet initialized.
    ///
    /// Returns `Ok(())` if the value was set, or `Err(value)` if it was
    /// already initialized.
    pub fn set(&self, value: T) -> Result<(), T> {
        match self.state.compare_exchange(
            ONCE_STATE_UNINIT,
            ONCE_STATE_INITIALIZING,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // Safety: We have exclusive access
                unsafe {
                    *self.data.get() = Some(value);
                }
                self.state.store(ONCE_STATE_INITIALIZED, Ordering::Release);
                Ok(())
            }
            Err(_) => Err(value),
        }
    }

    /// Gets the value, initializing it with `f` if not yet initialized.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get() {
            return value;
        }

        // Try to initialize
        let value = f();
        if self.set(value).is_ok() {
            self.get().unwrap()
        } else {
            // Another thread initialized it, drop our value and return theirs
            self.get().unwrap()
        }
    }

    /// Consumes the `OnceLock`, returning the underlying value if initialized.
    pub fn into_inner(self) -> Option<T> {
        // Safety: We own self
        unsafe { self.data.into_inner() }
    }

    /// Takes the value out of this `OnceLock`, moving it back to an uninitialized state.
    ///
    /// Returns `None` if the `OnceLock` was not initialized.
    pub fn take(&mut self) -> Option<T> {
        if self.state.load(Ordering::Acquire) == ONCE_STATE_INITIALIZED {
            self.state.store(ONCE_STATE_UNINIT, Ordering::Release);
            // Safety: We have &mut self
            unsafe { (&mut *self.data.get()).take() }
        } else {
            None
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OnceLock")
            .field("state", &self.state.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> Self {
        Self::new()
    }
}
