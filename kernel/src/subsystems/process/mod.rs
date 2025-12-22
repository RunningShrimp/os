pub mod manager;
pub mod thread;
pub mod thread_cancellation;
pub mod exec;
pub mod elf;
pub mod dynamic_linker;
pub mod fd_cache;
pub mod lock_optimized; // Optional: Optimized locking with RW locks and fine-grained locks
pub mod rcu_table;
pub mod context_switch;

#[cfg(feature = "kernel_tests")]
pub mod tests;

pub use manager::*;
pub use fd_cache::*;
pub use lock_optimized::*;
pub use rcu_table::*;

use crate::types::stubs::{UidT as uid_t, GidT as gid_t};

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

/// Initialize process management subsystem
///
/// This function initializes all process management components including:
/// - Process table
/// - Thread subsystem
/// - Context switching
/// - Process manager
pub fn init() -> nos_api::Result<()> {
    // Initialize process manager
    manager::init();
    
    // Initialize thread subsystem
    thread::init();
    
    // Initialize context switching
    context_switch::init();
    
    crate::println!("[process] Process management subsystem initialized");
    Ok(())
}

/// Shutdown process management subsystem
///
/// This function cleans up process management resources.
/// Note: In a production system, this should gracefully terminate all processes.
pub fn shutdown() -> nos_api::Result<()> {
    // TODO: Implement graceful shutdown
    // - Terminate all user processes
    // - Wait for processes to exit
    // - Clean up process table
    // - Release resources
    
    crate::println!("[process] Process management subsystem shutdown");
    Ok(())
}
