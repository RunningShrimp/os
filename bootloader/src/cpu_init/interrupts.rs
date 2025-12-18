//! Interrupt and exception handling

/// Initialize interrupt handling for the platform
pub fn init_interrupts() {
    #[cfg(target_arch = "x86_64")]
    init_x86_64_interrupts();

    #[cfg(target_arch = "aarch64")]
    init_aarch64_interrupts();

    #[cfg(target_arch = "riscv64")]
    init_riscv64_interrupts();
}

#[cfg(target_arch = "x86_64")]
fn init_x86_64_interrupts() {
    // Set up IDT (Interrupt Descriptor Table)
    // For bootloader, we typically just set up basic handlers
    
    unsafe {
        // Disable interrupts during setup
        core::arch::asm!("cli");
        
        // Load IDT (simplified - in real bootloader would need full IDT)
        // For now, just leave interrupts disabled
        core::arch::asm!("cli");
    }
}

#[cfg(target_arch = "aarch64")]
fn init_aarch64_interrupts() {
    // Set up exception vectors for AArch64
    unsafe {
        // Disable all interrupts (DAIF)
        core::arch::asm!("msr daifset, #0xf");
    }
}

#[cfg(target_arch = "riscv64")]
fn init_riscv64_interrupts() {
    // Set up RISC-V interrupt handling
    unsafe {
        // Disable machine interrupts
        core::arch::asm!("csrci mie, 0");
    }
}

/// Exception handler trait
pub trait ExceptionHandler {
    fn handle_exception(&mut self);
}

/// Panic on any exception (minimal bootloader behavior)
pub struct DefaultExceptionHandler;

impl ExceptionHandler for DefaultExceptionHandler {
    fn handle_exception(&mut self) {
        crate::drivers::console::write_str("EXCEPTION: Halting\n");
        loop {}
    }
}
