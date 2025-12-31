//! Architecture abstraction layer
//!
//! This module provides architecture-specific implementations and abstractions
//! for different CPU architectures, allowing to kernel to support multiple
//! architectures with a unified interface.

use nos_api::Result;

// Use memory_layout_stub as memory_layout
pub mod memory_layout_stub;
pub use memory_layout_stub as memory_layout;
pub use memory_layout_stub::*;

// Architecture-specific modules (conditionally compiled)
#[cfg(target_arch = "x86_64")]
pub mod cpuid;
#[cfg(target_arch = "x86_64")]
pub mod kpti;
#[cfg(target_arch = "x86_64")]
pub mod retpoline;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

/// Initialize architecture-specific subsystems
pub fn initialize() -> Result<()> {
    #[cfg(target_arch = "x86_64")]
    {
        // x86_64 specific initialization
        crate::println!("x86_64: Initializing architecture");
        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    {
        // aarch64 specific initialization
        crate::println!("aarch64: Initializing architecture");
        Ok(())
    }

    #[cfg(target_arch = "riscv64")]
    {
        // riscv64 specific initialization
        crate::println!("riscv64: Initializing architecture");
        Ok(())
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
    {
        compile_error!("Unsupported architecture");
        Err(nos_api::Error::NotImplemented("Unsupported architecture".to_string()))
    }
}

/// Shutdown architecture-specific subsystems
pub fn shutdown() -> Result<()> {
    #[cfg(target_arch = "x86_64")]
    {
        // x86_64 specific shutdown
        crate::println!("x86_64: Shutting down architecture");
        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    {
        // aarch64 specific shutdown
        crate::println!("aarch64: Shutting down architecture");
        Ok(())
    }

    #[cfg(target_arch = "riscv64")]
    {
        // riscv64 specific shutdown
        crate::println!("riscv64: Shutting down architecture");
        Ok(())
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
    {
        compile_error!("Unsupported architecture");
        Err(nos_api::Error::NotImplemented("Unsupported architecture".to_string()))
    }
}

/// Wait For Interrupt - CPU idle instruction
///
/// This function puts the CPU into a low-power state until an interrupt occurs.
/// It's commonly used in idle loops to save power.
#[inline]
pub fn wfi() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("hlt");
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("wfi");
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("wfi");
    }
}
