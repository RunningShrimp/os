//! Synchronization primitives for NOS operating system
//!
//! This module provides thread-safe synchronization primitives
//! for use in a no_std environment.

pub use spin::Mutex;
pub use spin::RwLock;
pub use core::sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, AtomicUsize, Ordering};
