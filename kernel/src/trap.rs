//! Kernel trap/interrupt handling
//!
//! This module provides the top-level trap and interrupt handling interface
//! for the kernel. It delegates to architecture-specific implementations.

/// Initialize trap handling
///
/// This function sets up the kernel's trap/interrupt handling subsystem.
/// It must be called early in the boot process, after basic architecture
/// initialization but before other subsystems that depend on interrupts.
pub fn initialize() {
    // Delegate to platform-specific trap initialization
    crate::platform::trap::init();

    crate::println!("[trap] kernel trap handling initialized");
}

/// Shutdown trap handling
///
/// This function performs a controlled shutdown of the trap/interrupt
/// handling subsystem. It should be called during kernel shutdown.
pub fn shutdown() {
    crate::println!("[trap] shutting down trap handling");

    // Disable interrupts globally
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli");
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        // Clear interrupt enable bits
        core::arch::asm!("csrw sie, 0");
    }

    #[cfg(target_arch = "aarch64")]
    {
        // GH-#1227: Disable interrupts for AArch64
        // See: https://github.com/npos/kernel/issues/1227
    }

    crate::println!("[trap] trap handling shutdown complete");
}
