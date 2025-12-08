//! Microkernel core for NOS hybrid architecture
//!
//! This module implements the microkernel layer that provides the most basic
//! system services while maintaining security and stability.

pub mod scheduler;
pub mod memory;
pub mod ipc;
pub mod interrupt;
pub mod timer;
pub mod service_registry;

use crate::reliability::errno::{ENOMEM, EINVAL};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Microkernel initialization state
static MICROKERNEL_INIT: AtomicBool = AtomicBool::new(false);

/// Microkernel statistics
#[derive(Debug)]
pub struct MicrokernelStats {
    pub scheduler_runs: AtomicUsize,
    pub interrupt_count: AtomicUsize,
    pub ipc_messages: AtomicUsize,
    pub memory_allocations: AtomicUsize,
}

impl MicrokernelStats {
    pub const fn new() -> Self {
        Self {
            scheduler_runs: AtomicUsize::new(0),
            interrupt_count: AtomicUsize::new(0),
            ipc_messages: AtomicUsize::new(0),
            memory_allocations: AtomicUsize::new(0),
        }
    }
}

/// Global microkernel statistics
pub static MICROKERNEL_STATS: MicrokernelStats = MicrokernelStats::new();

/// Initialize the microkernel core
///
/// This function must be called early in the boot process to set up
/// all microkernel components in the correct order.
pub fn init_microkernel() -> Result<(), i32> {
    if MICROKERNEL_INIT.load(Ordering::SeqCst) {
        return Ok(()); // Already initialized
    }

    // Initialize in dependency order
    memory::init()?;
    interrupt::init()?;
    timer::init()?;
    scheduler::init()?;
    ipc::init()?;
    service_registry::init()?;

    MICROKERNEL_INIT.store(true, Ordering::SeqCst);
    Ok(())
}

/// Check if microkernel is initialized
pub fn is_initialized() -> bool {
    MICROKERNEL_INIT.load(Ordering::SeqCst)
}

/// Microkernel panic handler
#[cold]
pub fn microkernel_panic(message: &str) -> ! {
    // In a real implementation, this would:
    // 1. Stop all other cores
    // 2. Save crash dump
    // 3. Display error information
    // 4. Halt the system

    crate::println!("MICROKERNEL PANIC: {}", message);

    // Halt the system
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Get microkernel version information
pub fn get_version() -> &'static str {
    "NOS Microkernel v1.0.0"
}

/// Get microkernel build information
pub fn get_build_info() -> &'static str {
    concat!(
        "Build: NOS Kernel v1.0.0"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microkernel_info() {
        assert!(!get_version().is_empty());
        assert!(!get_build_info().is_empty());
    }

    #[test]
    fn test_initialization_state() {
        // Should start uninitialized
        assert!(!is_initialized());
    }
}