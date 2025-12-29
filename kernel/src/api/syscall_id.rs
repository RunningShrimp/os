//! System call ID definitions for the NOS kernel.
//!
//! This module provides system call IDs that can be used by applications
//! and other kernel modules. It re-exports the syscall_id from the
//! syscalls module to make it available at the API layer.

// Re-export the syscall_id module from syscalls
pub use crate::subsystems::syscalls::api::syscall_id::*;