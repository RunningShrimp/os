//! Syscall implementation handlers

// Note: mm module has been moved to kernel/src/subsystems/syscalls/mm/
// This reduces nesting depth from 5 levels to 3 levels

pub mod fs;
pub mod net;
pub mod types;

pub use types::*;
