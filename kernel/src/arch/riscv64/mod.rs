//! RISC-V 64 architecture implementation
//!
//! This module provides RISC-V 64-specific implementations for architecture
//! abstractions and operations.

pub mod interrupts;
pub mod memory;

/// Initialize RISC-V 64-specific subsystems
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("riscv64: Initializing architecture subsystems");

    // Initialize memory management
    memory::initialize()?;

    // Initialize interrupt handling
    interrupts::initialize()?;

    Ok(())
}

/// Shutdown RISC-V 64-specific subsystems
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("riscv64: Shutting down architecture subsystems");

    // Shutdown interrupt handling
    interrupts::shutdown()?;

    // Shutdown memory management
    memory::shutdown()?;

    Ok(())
}