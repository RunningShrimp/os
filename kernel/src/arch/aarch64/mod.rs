//! AArch64 architecture implementation
//!
//! This module provides AArch64-specific implementations for architecture
//! abstractions and operations.

pub mod interrupts;
pub mod memory;

/// Initialize AArch64-specific subsystems
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("aarch64: Initializing architecture subsystems");

    // Initialize memory management
    memory::initialize()?;

    // Initialize interrupt handling
    interrupts::initialize()?;

    Ok(())
}

/// Shutdown AArch64-specific subsystems
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("aarch64: Shutting down architecture subsystems");

    // Shutdown interrupt handling
    interrupts::shutdown()?;

    // Shutdown memory management
    memory::shutdown()?;

    Ok(())
}