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
            crate::println!("usertrap: unexpected scause={:#x} sepc={:#x} stval={:#x}",
                scause, sepc, stval);
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
            }
            cause::SUPERVISOR_EXTERNAL => {
                // External interrupt (e.g., UART)
                // TODO: Handle external interrupts
            }
            _ => {
                crate::println!("unexpected interrupt: {:#x}", scause);
            }
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
        pub const SVC64: u32 = 0x15;
        pub const DATA_ABORT_LOWER: u32 = 0x24;
        pub const DATA_ABORT_SAME: u32 = 0x25;
        pub const INST_ABORT_LOWER: u32 = 0x20;
        pub const INST_ABORT_SAME: u32 = 0x21;
    }
    
    /// Handle exception from EL0
    pub fn handle_sync_el0() {
        let esr: u64;
        let elr: u64;
        let far: u64;
        
        unsafe {
            core::arch::asm!("mrs {}, esr_el1", out(reg) esr);
            core::arch::asm!("mrs {}, elr_el1", out(reg) elr);
            core::arch::asm!("mrs {}, far_el1", out(reg) far);
        }
        
        let ec = ((esr >> 26) & 0x3F) as u32;
        
        match ec {
            ec::SVC64 => {
                // System call - handled by assembly with trapframe
            }
            ec::DATA_ABORT_LOWER | ec::INST_ABORT_LOWER => {
                crate::println!("Page fault at {:#x}, esr={:#x}", far, esr);
            }
            _ => {
                crate::println!("Unexpected exception: ec={:#x} esr={:#x} elr={:#x}",
                    ec, esr, elr);
            }
        }
    }
    
    /// Handle IRQ from EL0
    pub fn handle_irq_el0() {
        handle_irq();
    }
    
    /// Handle IRQ from EL1
    pub fn handle_irq_el1() {
        handle_irq();
    }
    
    fn handle_irq() {
        // TODO: Read interrupt controller to determine source
        crate::subsystems::time::timer_interrupt();
    }
}

// ============================================================================
// x86_64 Trap Handling
// ============================================================================

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::*;
    
    /// x86_64 exception vectors
    pub mod vector {
        pub const DIVIDE_ERROR: u8 = 0;
        pub const DEBUG: u8 = 1;
        pub const NMI: u8 = 2;
        pub const BREAKPOINT: u8 = 3;
        pub const OVERFLOW: u8 = 4;
        pub const BOUND_RANGE: u8 = 5;
        pub const INVALID_OPCODE: u8 = 6;
        pub const DEVICE_NOT_AVAILABLE: u8 = 7;
        pub const DOUBLE_FAULT: u8 = 8;
        pub const INVALID_TSS: u8 = 10;
        pub const SEGMENT_NOT_PRESENT: u8 = 11;
        pub const STACK_FAULT: u8 = 12;
        pub const GENERAL_PROTECTION: u8 = 13;
        pub const PAGE_FAULT: u8 = 14;
        pub const X87_FPU_ERROR: u8 = 16;
        pub const ALIGNMENT_CHECK: u8 = 17;
        pub const MACHINE_CHECK: u8 = 18;
        pub const SIMD_ERROR: u8 = 19;
        
        pub const SYSCALL: u8 = 0x80;
        pub const TIMER: u8 = 32;
    }
    
    /// Handle trap/interrupt
    pub fn trap_handler(vector: u8, error_code: usize, rip: usize) {
        match vector {
            vector::SYSCALL => {
                // System call - handled by assembly with trapframe
            }
            vector::PAGE_FAULT => {
                let cr2: usize;
                unsafe {
                    core::arch::asm!("mov {}, cr2", out(reg) cr2);
                }
                crate::println!("Page fault at {:#x}, error={:#x}, rip={:#x}",
                    cr2, error_code, rip);
            }
            vector::TIMER => {
                crate::subsystems::time::timer_interrupt();
            }
            vector::GENERAL_PROTECTION => {
                panic!("General protection fault: error={:#x} rip={:#x}",
                    error_code, rip);
            }
            _ => {
                crate::println!("Unhandled trap: vector={} error={:#x} rip={:#x}",
                    vector, error_code, rip);
            }
        }
    }
}

// ============================================================================
// Trap vector assembly
// ============================================================================

#[cfg(all(feature = "baremetal", target_arch = "riscv64"))]
core::arch::global_asm!(r#"
.section .text
.globl kernelvec
.align 4
kernelvec:
    # Save all registers
    addi sp, sp, -256
    sd ra, 0(sp)
    sd sp, 8(sp)
    sd gp, 16(sp)
    sd tp, 24(sp)
    sd t0, 32(sp)
    sd t1, 40(sp)
    sd t2, 48(sp)
    sd s0, 56(sp)
    sd s1, 64(sp)
    sd a0, 72(sp)
    sd a1, 80(sp)
    sd a2, 88(sp)
    sd a3, 96(sp)
    sd a4, 104(sp)
    sd a5, 112(sp)
    sd a6, 120(sp)
    sd a7, 128(sp)
    sd s2, 136(sp)
    sd s3, 144(sp)
    sd s4, 152(sp)
    sd s5, 160(sp)
    sd s6, 168(sp)
    sd s7, 176(sp)
    sd s8, 184(sp)
    sd s9, 192(sp)
    sd s10, 200(sp)
    sd s11, 208(sp)
    sd t3, 216(sp)
    sd t4, 224(sp)
    sd t5, 232(sp)
    sd t6, 240(sp)
    
    # Call Rust trap handler
    call kerneltrap_rust
    
    # Restore all registers
    ld ra, 0(sp)
    ld gp, 16(sp)
    ld tp, 24(sp)
    ld t0, 32(sp)
    ld t1, 40(sp)
    ld t2, 48(sp)
    ld s0, 56(sp)
    ld s1, 64(sp)
    ld a0, 72(sp)
    ld a1, 80(sp)
    ld a2, 88(sp)
    ld a3, 96(sp)
    ld a4, 104(sp)
    ld a5, 112(sp)
    ld a6, 120(sp)
    ld a7, 128(sp)
    ld s2, 136(sp)
    ld s3, 144(sp)
    ld s4, 152(sp)
    ld s5, 160(sp)
    ld s6, 168(sp)
    ld s7, 176(sp)
    ld s8, 184(sp)
    ld s9, 192(sp)
    ld s10, 200(sp)
    ld s11, 208(sp)
    ld t3, 216(sp)
    ld t4, 224(sp)
    ld t5, 232(sp)
    ld t6, 240(sp)
    addi sp, sp, 256
    
    sret
"#);

#[cfg(all(feature = "baremetal", target_arch = "riscv64"))]
#[unsafe(no_mangle)]
pub extern "C" fn kerneltrap_rust() {
    riscv64::kerneltrap();
}

#[cfg(all(feature = "baremetal", target_arch = "aarch64"))]
core::arch::global_asm!(r#"
.section .text
.globl exception_vector
.align 11
exception_vector:
    // Current EL with SP0
    .align 7
    b .             // Synchronous
    .align 7
    b .             // IRQ
    .align 7
    b .             // FIQ
    .align 7
    b .             // SError
    
    // Current EL with SPx
    .align 7
    b handle_sync_el1   // Synchronous
    .align 7
    b handle_irq_el1    // IRQ
    .align 7
    b .                 // FIQ
    .align 7
    b .                 // SError
    
    // Lower EL using AArch64
    .align 7
    b handle_sync_el0   // Synchronous
    .align 7
    b handle_irq_el0    // IRQ
    .align 7
    b .                 // FIQ
    .align 7
    b .                 // SError
    
    // Lower EL using AArch32
    .align 7
    b .
    .align 7
    b .
    .align 7
    b .
    .align 7
    b .

handle_sync_el1:
    ret

handle_irq_el1:
    ret

handle_sync_el0:
    ret

handle_irq_el0:
    ret
"#);

// ============================================================================
// Public API
// ============================================================================

/// Initialize trap handling
pub fn init() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        // Set trap handler
        unsafe extern "C" {
            fn kernelvec();
        }
        core::arch::asm!("csrw stvec, {}", in(reg) kernelvec as usize);
        
        // Enable interrupts
        core::arch::asm!("csrsi sstatus, 2"); // Set SIE bit
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        // Set exception vector base
        unsafe extern "C" {
            fn exception_vector();
        }
        core::arch::asm!("msr vbar_el1, {}", in(reg) exception_vector as *const () as usize);
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // IDT setup would go here
    }
}

/// Handle trap (generic interface)
pub fn handle(cause: usize, epc: usize, tval: usize) {
    #[cfg(target_arch = "riscv64")]
    {
        let _ = (cause, epc, tval);
        riscv64::kerneltrap();
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let _ = (cause, epc, tval);
        // Handled by specific handlers
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        let _ = tval;
        x86_64::trap_handler(cause as u8, 0, epc);
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
        
        // TODO: Set up trapframe and call userret
    }
}
