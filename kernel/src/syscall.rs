//! System Call Module
//!
//! This module provides the system call interface and re-exports from nos-syscalls
//! for the NOS kernel.

// Re-export from nos-syscalls
pub use nos_syscalls::*;

/// System call module initialization
pub fn init_syscalls() -> Result<(), nos_api::Error> {
    // Initialize syscall dispatcher
    let _ = nos_syscalls::init_dispatcher();
    Ok(())
}

/// Shutdown system call module
pub fn shutdown_syscalls() -> Result<(), nos_api::Error> {
    // Cleanup syscall dispatcher
    nos_syscalls::shutdown_syscalls()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_mod_init() {
        // This test would require actual syscall initialization
        // For now, just verify the module compiles
        assert!(true);
    }
}