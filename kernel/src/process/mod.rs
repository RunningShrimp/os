pub mod manager;
pub mod thread;
pub mod exec;
pub mod elf;
pub mod dynamic_linker;
pub mod fd_cache;
pub mod lock_optimized;
pub mod rcu_table;

#[cfg(feature = "kernel_tests")]
pub mod tests;

pub use manager::*;
pub use fd_cache::*;
pub use lock_optimized::*;
pub use rcu_table::*;

use crate::types::stubs::{uid_t, gid_t};

/// Get current real user ID
pub fn getuid() -> uid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.uid;
        }
    }
    0
}

/// Get current real group ID
pub fn getgid() -> gid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.gid;
        }
    }
    0
}

/// Get current effective user ID
pub fn geteuid() -> uid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.euid;
        }
    }
    0
}

/// Get current effective group ID
pub fn getegid() -> gid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.egid;
        }
    }
    0
}
