//! RISC-V 64 architecture implementation
//!
//! This module provides RISC-V 64-specific implementations for architecture
//! abstractions and operations, including comprehensive SMP support,
//! virtualization with H-extension, advanced interrupt handling, and
//! synchronization primitives.

pub mod interrupts;
pub mod interrupt;
pub mod memory;
pub mod paging;
pub mod smp;
pub mod sync;
pub mod timer;
pub mod virtualization;

/// Initialize RISC-V 64-specific subsystems
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("riscv64: Initializing architecture subsystems");

    // Initialize synchronization primitives
    crate::println!("riscv64: Initializing synchronization");
    // sync is implicitly initialized

    // Initialize memory management (paging)
    crate::println!("riscv64: Initializing paging");
    // paging is implicitly initialized

    // Initialize interrupt controller
    crate::println!("riscv64: Initializing interrupt controller");
    interrupt::plic_init()?;

    // Initialize timer subsystem
    crate::println!("riscv64: Initializing timers");
    timer::timer_init()?;

    // Initialize SMP (multi-core support)
    crate::println!("riscv64: Initializing SMP");
    smp::smp_setup()?;

    // Initialize virtualization (if H-extension is available)
    if virtualization::has_virtualization() {
        crate::println!("riscv64: H-extension detected, initializing virtualization");
        virtualization::hvf_init()?;
    } else {
        crate::println!("riscv64: No H-extension detected, skipping virtualization");
    }

    Ok(())
}

/// Shutdown RISC-V 64-specific subsystems
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("riscv64: Shutting down architecture subsystems");

    // Shutdown SMP
    crate::println!("riscv64: Shutting down SMP");
    smp::smp_shutdown()?;

    // Shutdown interrupt handling
    crate::println!("riscv64: Shutting down interrupt controller");
    // interrupt shutdown is implicit

    // Shutdown memory management
    crate::println!("riscv64: Shutting down memory management");
    memory::shutdown()?;

    Ok(())
}