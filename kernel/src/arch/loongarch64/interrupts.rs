//! LoongArch interrupt handling
//!
//! This module provides interrupt and exception handling for LoongArch64,
//! including the AIC (Advanced Interrupt Controller) and EIO (Extended I/O) interrupt controller.

use core::sync::atomic::{AtomicU32, Ordering};

/// Exception codes for LoongArch
pub mod exception_codes {
    /// Interrupt
    pub const INT: u32 = 0;
    /// PIL (Platform Interrupt)
    pub const PIL: u32 = 1;
    /// SYS (System Call)
    pub const SYS: u32 = 2;
    /// FPE (Floating Point Exception)
    pub const FPE: u32 = 3;
    /// FPD (Floating Point Disabled)
    pub const FPD: u32 = 4;
    /// FPE (Floating Point Exception, again)
    pub const FPE2: u32 = 5;
    /// ALS (Address Access Error - Load)
    pub const ALS: u32 = 6;
    /// ALE (Address Access Error - Store)
    pub const ALE: u32 = 7;
    /// BCE (Bus Cache Coherency Error)
    pub const BCE: u32 = 8;
    /// SSE (System Service)
    pub const SSE: u32 = 9;
    /// INE (Instruction Non-Existent)
    pub const INE: u32 = 10;
    /// IPE (Instruction Privilege Error)
    pub const IPE: u32 = 11;
    /// FPD (Floating Point Disabled, again)
    pub const FPD2: u32 = 12;
    /// CPUCFG (CPUCFG access error)
    pub const CPUCFG: u32 = 13;
    /// FREG (FP register access error)
    pub const FREG: u32 = 14;
    /// WATCH (Watchpoint)
    pub const WATCH: u32 = 15;
    /// BTV (Binary Translation V)
    pub const BTV: u32 = 16;
    /// BTE (Binary Translation E)
    pub const BTE: u32 = 17;
    /// GSP (Guest State Change)
    pub const GSP: u32 = 18;
    /// HVC (HyperVisor Call)
    pub const HVC: u32 = 19;
    /// GC (Guest Exception)
    pub const GC: u32 = 20;
}

/// IRQ numbers for LoongArch AIC
pub mod irq_numbers {
    /// IPI (Inter-Processor Interrupt)
    pub const IPI_IRQ: u32 = 0;
    /// Timer IRQ
    pub const TIMER_IRQ: u32 = 1;
    /// First external IRQ
    pub const EXT_IRQ_BASE: u32 = 64;
}

/// Interrupt handler function type
pub type InterruptHandler = unsafe extern "C" fn();

/// Global interrupt handler table
static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 256] = [None; 256];

/// Initialize interrupt handling
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Initializing interrupt handling");

    // Initialize AIC (Advanced Interrupt Controller)
    init_aic()?;

    // Initialize EIO (Extended I/O) interrupt controller if present
    if has_eio() {
        init_eio()?;
    }

    // Setup exception handlers
    setup_exception_handlers()?;

    // Enable interrupts
    enable_interrupts();

    Ok(())
}

/// Shutdown interrupt handling
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Shutting down interrupt handling");

    // Disable interrupts
    disable_interrupts();

    Ok(())
}

/// Initialize AIC (Advanced Interrupt Controller)
fn init_aic() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Initializing AIC");

    unsafe {
        // Enable AIC
        // AIC_ENABLE register
        core::arch::asm!(
            "li.w {0}, 0x1",  // AIC base address offset
            "li.w {1}, 0x1",  // Enable bit
            "st.w {1}, {0}, 0x0", // Write to AIC_ENABLE
            out(reg) _,
            out(reg) _,
            options(nostack)
        );

        // Set IRQ priorities
        for irq in 0..64 {
            set_irq_priority(irq, 7); // Default priority
        }
    }

    Ok(())
}

/// Check if EIO (Extended I/O) interrupt controller is present
fn has_eio() -> bool {
    // Check for EIO presence in CPUCFG
    unsafe {
        let cfg = super::read_cpucfg(1);
        (cfg & (1 << 12)) != 0 // EIO present bit
    }
}

/// Initialize EIO (Extended I/O) interrupt controller
fn init_eio() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Initializing EIO");

    unsafe {
        // Enable EIO
        core::arch::asm!(
            "li.d {0}, 0xFE000000", // EIO base address
            "li.w {1}, 0x1", // Enable bit
            "st.w {1}, {0}, 0x0", // Write to EIO_ENABLE
            out(reg) _,
            out(reg) _,
            options(nostack)
        );
    }

    Ok(())
}

/// Setup exception handlers
fn setup_exception_handlers() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Setting up exception handlers");

    // Set exception vector base address
    unsafe {
        let exception_vector = exception_handler_entry as u64;

        core::arch::asm!(
            "csrwr {0}, 0x6", // Set EBASE (Exception Base Address register)
            in(reg) exception_vector,
            options(nostack, nomem)
        );
    }

    Ok(())
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        let mut crmd: u64;
        core::arch::asm!(
            "csrrd {0}, 0x0", // Read CRMD
            out(reg) crmd,
            options(nostack, nomem)
        );

        // Enable interrupts by setting IE (Interrupt Enable) bit
        crmd |= (1 << 2);

        core::arch::asm!(
            "csrwr {0}, 0x0", // Write CRMD
            in(reg) crmd,
            options(nostack, nomem)
        );
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        let mut crmd: u64;
        core::arch::asm!(
            "csrrd {0}, 0x0", // Read CRMD
            out(reg) crmd,
            options(nostack, nomem)
        );

        // Disable interrupts by clearing IE bit
        crmd &= !(1 << 2);

        core::arch::asm!(
            "csrwr {0}, 0x0", // Write CRMD
            in(reg) crmd,
            options(nostack, nomem)
        );
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    unsafe {
        let crmd: u64;
        core::arch::asm!(
            "csrrd {0}, 0x0", // Read CRMD
            out(reg) crmd,
            options(nostack, nomem)
        );

        (crmd & (1 << 2)) != 0
    }
}

/// Set IRQ priority
///
/// # Arguments
///
/// * `irq` - IRQ number (0-255)
/// * `priority` - Priority level (0-15, where 15 is highest)
pub fn set_irq_priority(irq: u32, priority: u8) {
    unsafe {
        core::arch::asm!(
            "li.d {0}, 0xFE004000", // AIC base address
            "andi.w {1}, {2}, 0xFF",
            "st.b {1}, {0}, {3}", // Write to AIC_IRQ_PRI[irq]
            out(reg) _,
            out(reg) _,
            in(reg) priority,
            in(reg) irq,
            options(nostack)
        );
    }
}

/// Enable specific IRQ
///
/// # Arguments
///
/// * `irq` - IRQ number to enable
pub fn enable_irq(irq: u32) {
    unsafe {
        core::arch::asm!(
            "li.d {0}, 0xFE004000", // AIC base address
            "li.w {1}, 1",
            "st.b {1}, {0}, {2}", // Write to AIC_IRQ_EN[irq]
            out(reg) _,
            out(reg) _,
            in(reg) (irq + 0x400), // AIC_IRQ_EN offset
            options(nostack)
        );
    }
}

/// Disable specific IRQ
///
/// # Arguments
///
/// * `irq` - IRQ number to disable
pub fn disable_irq(irq: u32) {
    unsafe {
        core::arch::asm!(
            "li.d {0}, 0xFE004000", // AIC base address
            "li.w {1}, 0",
            "st.b {1}, {0}, {2}", // Write to AIC_IRQ_EN[irq]
            out(reg) _,
            out(reg) _,
            in(reg) (irq + 0x400), // AIC_IRQ_EN offset
            options(nostack)
        );
    }
}

/// Register interrupt handler
///
/// # Arguments
///
/// * `irq` - IRQ number
/// * `handler` - Handler function
pub fn register_interrupt_handler(irq: u32, handler: InterruptHandler) {
    unsafe {
        if (irq as usize) < INTERRUPT_HANDLERS.len() {
            INTERRUPT_HANDLERS[irq as usize] = Some(handler);
        }
    }
}

/// Unregister interrupt handler
///
/// # Arguments
///
/// * `irq` - IRQ number
pub fn unregister_interrupt_handler(irq: u32) {
    unsafe {
        if (irq as usize) < INTERRUPT_HANDLERS.len() {
            INTERRUPT_HANDLERS[irq as usize] = None;
        }
    }
}

/// Exception handler entry point
///
/// This is called by the hardware when an exception occurs
#[naked]
unsafe extern "C" fn exception_handler_entry() {
    unsafe {
        core::arch::asm!(
            // Save general-purpose registers
            "addi.d $sp, $sp, -256",
            "st.d $ra, $sp, 0",
            "st.d $tp, $sp, 8",
            "st.d $a0, $sp, 16",
            "st.d $a1, $sp, 24",
            "st.d $a2, $sp, 32",
            "st.d $a3, $sp, 40",
            "st.d $a4, $sp, 48",
            "st.d $a5, $sp, 56",
            "st.d $a6, $sp, 64",
            "st.d $a7, $sp, 72",
            "st.d $t0, $sp, 80",
            "st.d $t1, $sp, 88",
            "st.d $t2, $sp, 96",
            "st.d $t3, $sp, 104",
            "st.d $t4, $sp, 112",
            "st.d $t5, $sp, 120",
            "st.d $t6, $sp, 128",
            "st.d $t7, $sp, 136",
            "st.d $t8, $sp, 144",
            // Get exception code
            "csrrd $a0, 0x6", // PRMD
            "andi.w $a1, $a0, 0x1F", // Extract exception code
            // Call Rust exception handler
            "bl {exception_handler_rust}",
            // Restore general-purpose registers
            "ld.d $ra, $sp, 0",
            "ld.d $tp, $sp, 8",
            "ld.d $a0, $sp, 16",
            "ld.d $a1, $sp, 24",
            "ld.d $a2, $sp, 32",
            "ld.d $a3, $sp, 40",
            "ld.d $a4, $sp, 48",
            "ld.d $a5, $sp, 56",
            "ld.d $a6, $sp, 64",
            "ld.d $a7, $sp, 72",
            "ld.d $t0, $sp, 80",
            "ld.d $t1, $sp, 88",
            "ld.d $t2, $sp, 96",
            "ld.d $t3, $sp, 104",
            "ld.d $t4, $sp, 112",
            "ld.d $t5, $sp, 120",
            "ld.d $t6, $sp, 128",
            "ld.d $t7, $sp, 136",
            "ld.d $t8, $sp, 144",
            "addi.d $sp, $sp, 256",
            // Return from exception
            "ertn",
            exception_handler_rust = sym exception_handler,
            options(noreturn)
        );
    }
}

/// Rust exception handler
///
/// # Arguments
///
/// * `exception_code` - Exception code
#[no_mangle]
unsafe extern "C" fn exception_handler(exception_code: u32) {
    match exception_code {
        exception_codes::INT => {
            handle_interrupt();
        }
        exception_codes::SYS => {
            handle_syscall();
        }
        exception_codes::FPE | exception_codes::FPE2 => {
            handle_fpe_exception();
        }
        exception_codes::ALS | exception_codes::ALE => {
            handle_memory_access_error();
        }
        _ => {
            handle_unknown_exception(exception_code);
        }
    }
}

/// Handle interrupt
unsafe fn handle_interrupt() {
    // Read IRQ number from AIC
    let irq: u32;
    core::arch::asm!(
        "li.d {0}, 0xFE004000", // AIC base address
        "ld.w {1}, {0}, 0x200", // Read AIC_IRQ_STATUS
        out(reg) _,
        out(reg) irq,
        options(nostack, nomem)
    );

    // Call registered handler if exists
    unsafe {
        if (irq as usize) < INTERRUPT_HANDLERS.len() {
            if let Some(handler) = INTERRUPT_HANDLERS[irq as usize] {
                handler();
            }
        }
    }

    // End of interrupt
    core::arch::asm!(
        "li.d {0}, 0xFE004000", // AIC base address
        "li.w {1}, 1",
        "st.w {1}, {0}, 0x200", // Write to AIC_IRQ_EOI
        out(reg) _,
        out(reg) _,
        options(nostack)
    );
}

/// Handle system call
unsafe fn handle_syscall() {
    // System call handling would go here
    crate::println!("LoongArch64: System call");
}

/// Handle floating point exception
unsafe fn handle_fpe_exception() {
    crate::println!("LoongArch64: Floating point exception");
}

/// Handle memory access error
unsafe fn handle_memory_access_error() {
    crate::println!("LoongArch64: Memory access error");
}

/// Handle unknown exception
unsafe fn handle_unknown_exception(code: u32) {
    crate::println!("LoongArch64: Unknown exception: {}", code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_enable_disable() {
        disable_interrupts();
        assert!(!interrupts_enabled());
        enable_interrupts();
        assert!(interrupts_enabled());
    }

    #[test]
    fn test_irq_handlers() {
        // Test handler registration
        extern "C" fn test_handler() {}
        register_interrupt_handler(10, test_handler);
        // Handler should be registered
        unregister_interrupt_handler(10);
    }

    #[test]
    fn test_exception_codes() {
        assert_eq!(exception_codes::INT, 0);
        assert_eq!(exception_codes::SYS, 2);
        assert_eq!(exception_codes::FPE, 3);
    }
}
