//! C-SKY architecture implementation
//!
//! This module provides support for C-SKY processors, a 32-bit RISC architecture
//! developed by Hangzhou C-SKY Microsystems for embedded systems.
//!
//! ## Overview
//!
//! C-SKY is a 32-bit reduced instruction set computer (RISC) architecture
//! designed for embedded applications with emphasis on:
//! - Low power consumption
//! - High code density
//! - DSP (Digital Signal Processing) extensions
//! - Optional floating point unit
//!
//! ### Key Features
//!
//! - **32-bit Architecture**: Full 32-bit addressing and data
//! - **16-bit/32-bit Instructions**: Mixed instruction encoding for code density
//! - **DSP Extensions**: Optional DSP instructions for signal processing
//! - **Floating Point**: Optional FPU with IEEE 754 support
//! - **Memory Management**: MMU with page table support
//! - **Interrupts**: Advanced interrupt controller (AIC)
//!
//! ## Components
//!
//! - [`cpu`]: CPU features and initialization
//! - [`memory`]: Memory management and page tables
//! - [`interrupts`]: Interrupt handling and exceptions
//! - [`syscall`]: System call interface
//!
//! ## Supported Processors
//!
//! - CK801, CK802, CK803 (entry-level)
//! - CK805, CK807, CK810, CK810V (mid-range)
//! - CK860, CK862, CK870, CK872 (high-performance)
//! - Future C-SKY processors

pub mod cpu;
pub mod memory;
pub mod interrupts;
pub mod syscall;

use core::sync::atomic::{AtomicU32, Ordering};

/// Global CPU count
static CPU_COUNT: AtomicU32 = AtomicU32::new(1);

/// Boot CPU ID
const BOOT_CPU_ID: u32 = 0;

/// Initialize C-SKY-specific subsystems
///
/// This function initializes all architecture-specific components:
/// 1. CPU feature detection
/// 2. Memory management setup
/// 3. Interrupt controller initialization
///
/// # Returns
///
/// Returns `Ok(())` if initialization succeeds, or an error string if it fails.
///
/// # Example
///
/// ```no_run
/// use kernel::arch::csky::initialize;
///
/// match initialize() {
///     Ok(()) => println!("C-SKY initialized"),
///     Err(e) => println!("Initialization failed: {}", e),
/// }
/// ```
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("C-SKY: Initializing architecture subsystems");

    // Initialize CPU features
    crate::println!("C-SKY: Detecting CPU features");
    cpu::init_cpu();
    let cpu_info = cpu::get_cpu_info();
    crate::println!(
        "C-SKY: CPU: {}, CPID: {:#08x}, Features: {:#x}",
        cpu_info.processor_name,
        cpu_info.cpid,
        cpu_info.features
    );

    // Initialize memory management
    memory::initialize()?;

    // Initialize interrupt handling
    interrupts::initialize()?;

    Ok(())
}

/// Shutdown C-SKY-specific subsystems
///
/// Performs a graceful shutdown of all architecture-specific components.
///
/// # Returns
///
/// Returns `Ok(())` if shutdown succeeds, or an error string if it fails.
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("C-SKY: Shutting down architecture subsystems");

    // Shutdown interrupt handling
    interrupts::shutdown()?;

    // Shutdown memory management
    memory::shutdown()?;

    Ok(())
}

/// Get the number of active CPUs
///
/// # Returns
///
/// The number of active CPUs in the system
pub fn get_cpu_count() -> u32 {
    CPU_COUNT.load(Ordering::Relaxed)
}

/// Get the boot CPU ID
///
/// # Returns
///
/// The ID of the boot CPU (always 0 for C-SKY)
pub fn get_boot_cpu_id() -> u32 {
    BOOT_CPU_ID
}

/// Get current CPU ID
///
/// # Returns
///
/// The ID of the current CPU
#[inline]
pub fn get_cpuid() -> u32 {
    let cpuid: u32;
    unsafe {
        core::arch::asm!(
            "mfcr {0}, cr13", // CPUID register
            out(reg) cpuid,
            options(nostack, nomem)
        );
    }
    cpuid
}

/// Halt the current CPU
///
/// This halts execution on the current CPU core.
#[inline]
pub fn halt() -> ! {
    unsafe {
        core::arch::asm!(
            "idle", // Wait for interrupt
            options(nostack, nomem)
        );
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Memory barrier
///
/// Ensures all memory operations before this point complete
/// before any memory operations after this point.
#[inline]
pub fn memory_barrier() {
    unsafe {
        core::arch::asm!(
            "sync", // Memory synchronization barrier
            options(nostack, nomem, preserves_flags)
        );
    }
}

/// Sync barrier (stronger than normal memory barrier)
#[inline]
pub fn sync_barrier() {
    unsafe {
        core::arch::asm!(
            "sync", // Full synchronization
            options(nostack, nomem, preserves_flags)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpuid() {
        let id = get_cpuid();
        assert!(id < 256, "CPU ID should be less than 256");
    }

    #[test]
    fn test_cpu_count() {
        let count = get_cpu_count();
        assert!(count >= 1, "Should have at least 1 CPU");
    }

    #[test]
    fn test_boot_cpu_id() {
        assert_eq!(get_boot_cpu_id(), 0, "Boot CPU ID should be 0");
    }

    #[test]
    fn test_memory_barrier() {
        // Should not panic
        memory_barrier();
        sync_barrier();
    }
}
