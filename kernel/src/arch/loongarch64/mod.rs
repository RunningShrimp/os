//! LoongArch 64-bit architecture implementation
//!
//! This module provides support for LoongArch 64-bit processors, specifically
//! the Loongson (龙芯) series of processors.
//!
//! ## Overview
//!
//! LoongArch is a reduced instruction set computer (RISC) instruction set
//! architecture developed by the Loongson Technology Corporation.
//!
//! ### Key Features
//!
//! - **RISC Architecture**: Simple, efficient instruction set
//! - **64-bit**: Full 64-bit addressing and data handling
//! - **Vector Extensions**: LSX (128-bit SIMD) and LASX (256-bit SIMD)
//! - **Virtualization**: Hardware virtualization support
//! - **Memory Management**: MMU with page table support
//!
//! ## Components
//!
//! - [`cpu`]: CPU features and initialization
//! - [`memory`]: Memory management and page tables
//! - [`interrupts`]: Interrupt handling and exceptions
//! - [`smp`]: Multi-processor support
//! - [`syscall`]: System call interface
//!
//! ## Supported Processors
//!
//! - Loongson 3A5000 series (LA464)
//! - Loongson 3A6000 series (LA664)
//! - Future Loongson processors

pub mod cpu;
pub mod memory;
pub mod interrupts;
pub mod smp;
pub mod syscall;

use core::sync::atomic::{AtomicU32, Ordering};

/// Global CPU count
static CPU_COUNT: AtomicU32 = AtomicU32::new(1);

/// Boot CPU ID
const BOOT_CPU_ID: u32 = 0;

/// Initialize LoongArch64-specific subsystems
///
/// This function initializes all architecture-specific components:
/// 1. CPU feature detection
/// 2. Memory management setup
/// 3. Interrupt controller initialization
/// 4. Multi-processor initialization (if applicable)
///
/// # Returns
///
/// Returns `Ok(())` if initialization succeeds, or an error string if it fails.
///
/// # Example
///
/// ```no_run
/// use kernel::arch::loongarch64::initialize;
///
/// match initialize() {
///     Ok(()) => println!("LoongArch initialized"),
///     Err(e) => println!("Initialization failed: {}", e),
/// }
/// ```
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Initializing architecture subsystems");

    // Initialize CPU features
    crate::println!("LoongArch64: Detecting CPU features");
    cpu::init_cpu();
    let cpu_info = cpu::get_cpu_info();
    crate::println!(
        "LoongArch64: CPU: {}, PRID: {:#08x}, Features: {:#x}",
        cpu_info.processor_name,
        cpu_info.prid,
        cpu_info.features
    );

    // Initialize memory management
    memory::initialize()?;

    // Initialize interrupt handling
    interrupts::initialize()?;

    // Initialize SMP if available
    if cpu_info.features & cpu::LA_FEATURE_SMP != 0 {
        smp::initialize()?;
    }

    Ok(())
}

/// Shutdown LoongArch64-specific subsystems
///
/// Performs a graceful shutdown of all architecture-specific components.
///
/// # Returns
///
/// Returns `Ok(())` if shutdown succeeds, or an error string if it fails.
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Shutting down architecture subsystems");

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
/// The ID of the boot CPU (always 0 for LoongArch)
pub fn get_boot_cpu_id() -> u32 {
    BOOT_CPU_ID
}

/// Read CPUCFG register
///
/// LoongArch provides CPUCFG registers for CPU feature detection.
///
/// # Arguments
///
/// * `reg` - CPUCFG register number (0-63)
///
/// # Returns
///
/// The value of the specified CPUCFG register
#[inline]
pub unsafe fn read_cpucfg(reg: u32) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "cpucfg {0}, {1}",
            out(reg) value,
            in(reg) reg,
            options(nostack, nomem, preserves_flags)
        );
    }
    value
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
            "csrrd {0}, 0x0", // CPUID register
            out(reg) cpuid,
            options(nostack, nomem, preserves_flags)
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
            "idle 0",
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
            "dbar 0",
            options(nostack, nomem, preserves_flags)
        );
    }
}

/// Acquire barrier
///
/// Ensures all memory reads before this point complete
/// before any subsequent operations.
#[inline]
pub fn acquire_barrier() {
    unsafe {
        core::arch::asm!(
            "dbar 0",
            options(nostack, nomem, preserves_flags)
        );
    }
}

/// Release barrier
///
/// Ensures all memory operations before this point complete
/// before any subsequent writes.
#[inline]
pub fn release_barrier() {
    unsafe {
        core::arch::asm!(
            "dbar 0",
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
        acquire_barrier();
        release_barrier();
    }
}
