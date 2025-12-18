//! Fast-path system call optimization module
//!
//! This module provides optimized system call handling with:
//! - Fast-path dispatch for common syscalls
//! - Per-CPU syscall caches
//! - Syscall batching
//! - Adaptive optimization

pub mod fast_path_dispatcher;

pub use fast_path_dispatcher::*;