//! Trap handling for xv6-rust
//!
//! This module handles traps (interrupts and exceptions) from both
//! user and kernel mode.

// ============================================================================
// RISC-V Trap Handling
// ============================================================================

#[cfg(target_arch = "riscv64")]
mod riscv64 {
    use super::*;

    /// RISC-V trap causes
    pub mod cause {
        pub const INSTRUCTION_MISALIGNED: usize = 0;
        pub const INSTRUCTION_FAULT: usize = 1;
        pub const ILLEGAL_INSTRUCTION: usize = 2;
        pub const BREAKPOINT: usize = 3;
        pub const LOAD_MISALIGNED: usize = 4;
        pub const LOAD_FAULT: usize = 5;
        pub const STORE_MISALIGNED: usize = 6;
        pub const STORE_FAULT: usize = 7;
        pub const USER_ECALL: usize = 8;
        pub const SUPERVISOR_ECALL: usize = 9;
        pub const INSTRUCTION_PAGE_FAULT: usize = 12;
        pub const LOAD_PAGE_FAULT: usize = 13;
        pub const STORE_PAGE_FAULT: usize = 15;

        pub const SUPERVISOR_SOFTWARE: usize = 0x8000_0000_0000_0001;
        pub const SUPERVISOR_TIMER: usize = 0x8000_0000_0000_0005;
        pub const SUPERVISOR_EXTERNAL: usize = 0x8000_0000_0000_0009;
    }

    /// Handle trap from user mode
    pub fn usertrap() {
        let scause: usize;
        let sepc: usize;
        let stval: usize;

        unsafe {
            core::arch::asm!("csrr {}, scause", out(reg) scause);
            core::arch::asm!("csrr {}, sepc", out(reg) sepc);
            core::arch::asm!("csrr {}, stval", out(reg) stval);
        }

        if scause == cause::USER_ECALL {
            // System call - handled by usertrap assembly which has trapframe
            // This is a placeholder; real implementation passes trapframe
        } else if scause & 0x8000_0000_0000_0000 != 0 {
            // Interrupt
            handle_interrupt(scause);
        } else {
            // Exception
            crate::println!(
                "usertrap: unexpected scause={:#x} sepc={:#x} stval={:#x}",
                scause,
                sepc,
                stval
            );
        }
    }

    /// Handle trap from kernel mode
    pub fn kerneltrap() {
        let scause: usize;
        let sepc: usize;

        unsafe {
            core::arch::asm!("csrr {}, scause", out(reg) scause);
            core::arch::asm!("csrr {}, sepc", out(reg) sepc);
        }

        if scause & 0x8000_0000_0000_0000 != 0 {
            handle_interrupt(scause);
        } else {
            panic!("kerneltrap: scause={:#x} sepc={:#x}", scause, sepc);
        }
    }

    fn handle_interrupt(scause: usize) {
        match scause {
            cause::SUPERVISOR_TIMER => {
                // Timer interrupt - yield CPU
                crate::subsystems::time::timer_interrupt();
            },
            cause::SUPERVISOR_EXTERNAL => {
                // External interrupt (e.g., UART)
                // GH-#1158: Handle external interrupts
                // See: https://github.com/npos/kernel/issues/1158
            },
            _ => {
                crate::println!("unexpected interrupt: {:#x}", scause);
            },
        }
    }
}

// ============================================================================
// AArch64 Trap Handling
// ============================================================================

#[cfg(target_arch = "aarch64")]
mod aarch64 {

    /// Exception Syndrome Register (ESR) exception classes
    pub mod ec {
    }
}

/// Return to user mode
pub fn usertrapret() {
    // Set up trapframe and return to user
    #[cfg(target_arch = "riscv64")]
    unsafe {
        unsafe extern "C" {
            fn uservec();
            fn userret();
        }

        // Set stvec to uservec for user traps
        core::arch::asm!("csrw stvec, {}", in(reg) uservec as usize);

        // GH-#1159: Set up trapframe and call userret
        // See: https://github.com/npos/kernel/issues/1159
    }
}

/// Initialize trap handling
///
/// This function sets up the trap/exception handling for the platform.
/// It must be called early in the boot process.
pub fn init() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        // Set stvec to handle all traps in kernel mode
        extern "C" {
            fn kernelvec();
        }
        core::arch::asm!("csrw stvec, {}", in(reg) kernelvec as usize);
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        // GH-#1160: Set up IDT for x86_64
        // See: https://github.com/npos/kernel/issues/1160
        // For now, this is a placeholder
    }

    #[cfg(target_arch = "aarch64")]
    {
        // GH-#1161: Set up exception vectors for AArch64
        // See: https://github.com/npos/kernel/issues/1161
        // For now, this is a placeholder
    }
}
