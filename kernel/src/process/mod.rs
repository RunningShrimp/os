pub mod manager;
pub mod thread;
pub mod exec;
pub mod elf;
pub mod dynamic_linker;
pub mod fd_cache;
pub mod lock_optimized;

#[cfg(feature = "kernel_tests")]
pub mod tests;

pub use manager::*;
pub use fd_cache::*;
pub use lock_optimized::*;

use crate::types::stubs::{uid_t, gid_t};

/// Get current user ID
pub fn getuid() -> uid_t {
    // TODO: Get from current process
    0
}

/// Get current group ID
pub fn getgid() -> gid_t {
    // TODO: Get from current process
    0
}
