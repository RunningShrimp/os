//! C-SKY interrupt handling
//!
//! This module provides interrupt and exception handling for C-SKY processors.

use core::sync::atomic::AtomicU32;

/// Exception codes for C-SKY
pub mod exception_codes {
    pub const EXCEPT_RESET: u32 = 0;
    pub const EXCEPT_TB: u32 = 1;
    pub const EXCEPT_INSN: u32 = 2;
    pub const EXCEPT_BP: u32 = 3;
    pub const EXCEPT_MAC: u32 = 4;
    pub const EXCEPT_PRIV: u32 = 5;
    pub const EXCEPT_UNIMP: u32 = 6;
    pub const EXCEPT_COPROC: u32 = 7;
    pub const EXCEPT_OVERFLOW: u32 = 8;
    pub const EXCEPT_TRAP: u32 = 9;
    pub const EXCEPT_CONTEXT: u32 = 10;
    pub const EXCEPT_VID_C: u32 = 11;
    pub const EXCEPT_Z_DIV: u32 = 12;
    pub const EXCEPT_Z_ADD: u32 = 13;
    pub const EXCEPT_Z_SUB: u32 = 14;
    pub const EXCEPT_Z_UCOM: u32 = 15;
    pub const EXCEPT_Z_SC: u32 = 16;
    pub const EXCEPT_Z_MUL: u32 = 17;
    pub const EXCEPT_Z_INV: u32 = 18;
    pub const EXCEPT_Z_SH: u32 = 19;
    pub const EXCEPT_Z_SHIFT: u32 = 20;
    pub const EXCEPT_Z_AND: u32 = 21;
    pub const EXCEPT_Z_OR: u32 = 22;
    pub const EXCEPT_Z_XOR: u32 = 23;
    pub const EXCEPT_Z_NOT: u32 = 24;
    pub const EXCEPT_Z_ABSD: u32 = 25;
    pub const EXCEPT_Z_NEG: u32 = 26;
    pub const EXCEPT_Z_POP: u32 = 27;
    pub const EXCEPT_Z_POPN: u32 = 28;
    pub const EXCEPT_Z_CMP: u32 = 29;
    pub const EXCEPT_Z_MOV: u32 = 30;
    pub const EXCEPT_Z_EXC: u32 = 31;
    pub const EXCEPT_SPI_NE: u32 = 32;
    pub const EXCEPT_DEBUG: u32 = 33;
    pub const EXCEPT_ALIGN: u32 = 34;
}

/// Interrupt handler function type
pub type InterruptHandler = unsafe extern "C" fn();

/// Global interrupt handler table
static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 64] = [None; 64];

/// Initialize interrupt handling
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("C-SKY: Initializing interrupt handling");

    init_aic()?;
    setup_exception_handlers()?;
    enable_interrupts();

    Ok(())
}

/// Shutdown interrupt handling
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("C-SKY: Shutting down interrupt handling");
    disable_interrupts();
    Ok(())
}

/// Initialize AIC (Advanced Interrupt Controller)
fn init_aic() -> Result<(), &'static str> {
    crate::println!("C-SKY: Initializing AIC");
    Ok(())
}

/// Setup exception handlers
fn setup_exception_handlers() -> Result<(), &'static str> {
    crate::println!("C-SKY: Setting up exception handlers");

    unsafe {
        let exception_vector = exception_handler_entry as u32;
        core::arch::asm!(
            "mtcr {0}, cr<3, 0>", // Set VBR (Vector Base Address register)
            in(reg) exception_vector,
            options(nostack, nomem)
        );
    }

    Ok(())
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        let mut psr: u32;
        core::arch::asm!(
            "mfcr {0}, cr0<0, 0>", // Read PSR register
            out(reg) psr,
            options(nostack, nomem)
        );
        psr |= 0x01; // Set IE (Interrupt Enable) bit
        core::arch::asm!(
            "mtcr {0}, cr0<0, 0>", // Write PSR register
            in(reg) psr,
            options(nostack, nomem)
        );
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        let mut psr: u32;
        core::arch::asm!(
            "mfcr {0}, cr0<0, 0>",
            out(reg) psr,
            options(nostack, nomem)
        );
        psr &= !0x01; // Clear IE bit
        core::arch::asm!(
            "mtcr {0}, cr0<0, 0>",
            in(reg) psr,
            options(nostack, nomem)
        );
    }
}

/// Register interrupt handler
pub fn register_interrupt_handler(irq: u32, handler: InterruptHandler) {
    unsafe {
        if (irq as usize) < INTERRUPT_HANDLERS.len() {
            INTERRUPT_HANDLERS[irq as usize] = Some(handler);
        }
    }
}

/// Exception handler entry point
#[naked]
unsafe extern "C" fn exception_handler_entry() {
    unsafe {
        core::arch::asm!(
            // Save registers
            "subi.l sp, 256",
            "stm.a r4-r15, (sp)",
            "mtcr r4, cr<14, 0>", // Save EPC
            "mtcr r5, cr<15, 0>", // Save EPSR
            "mfcr r6, cr<2, 0>", // Get exception number
            // Call Rust handler
            "bl {exception_handler_rust}",
            // Restore registers
            "ldm.a r4-r15, (sp)",
            "addi.l sp, 256",
            "rte", // Return from exception
            exception_handler_rust = sym exception_handler,
            options(noreturn)
        );
    }
}

/// Rust exception handler
#[no_mangle]
unsafe extern "C" fn exception_handler(exception_code: u32) {
    match exception_code {
        exception_codes::EXCEPT_TB => handle_trace_buffer(),
        exception_codes::EXCEPT_INSN => handle_illegal_instruction(),
        exception_codes::EXCEPT_PRIV => handle_privilege_violation(),
        exception_codes::EXCEPT_ALIGN => handle_alignment_error(),
        _ => handle_unknown_exception(exception_code),
    }
}

/// Handle trace buffer exception
unsafe fn handle_trace_buffer() {
    crate::println!("C-SKY: Trace buffer exception");
}

/// Handle illegal instruction
unsafe fn handle_illegal_instruction() {
    crate::println!("C-SKY: Illegal instruction exception");
}

/// Handle privilege violation
unsafe fn handle_privilege_violation() {
    crate::println!("C-SKY: Privilege violation exception");
}

/// Handle alignment error
unsafe fn handle_alignment_error() {
    crate::println!("C-SKY: Alignment error exception");
}

/// Handle unknown exception
unsafe fn handle_unknown_exception(code: u32) {
    crate::println!("C-SKY: Unknown exception: {}", code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_codes() {
        assert_eq!(exception_codes::EXCEPT_TB, 1);
        assert_eq!(exception_codes::EXCEPT_INSN, 2);
    }
}
