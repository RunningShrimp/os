// Interrupt control functions for SMP safety
//
// This module provides interrupt control functions for SMP-safe
// synchronization, including disabling/enabling interrupts and
// tracking interrupt state.

use core::arch::asm;

/// Disable interrupts and return previous interrupt state
#[inline]
pub fn push_off() -> bool {
    let was_enabled = interrupts_enabled();
    disable_interrupts();
    was_enabled
}

/// Restore interrupt state
#[inline]
pub fn pop_off(was_enabled: bool) {
    if was_enabled {
        enable_interrupts();
    }
}

/// Check if interrupts are enabled
#[inline]
pub fn interrupts_enabled() -> bool {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        let sstatus: usize;
        asm!("csrr {}, sstatus", out(reg) sstatus);
        (sstatus & 0x2) != 0 // SIE bit
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let daif: u64;
        asm!("mrs {}, daif", out(reg) daif);
        (daif & 0x80) == 0 // IRQ not masked
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let flags: u64;
        asm!("pushfq; pop {}", out(reg) flags);
        (flags & 0x200) != 0 // IF flag
    }
}

/// Disable interrupts
#[inline]
fn disable_interrupts() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("csrc sstatus, {}", in(reg) 0x2usize); // Clear SIE
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("msr daifset, #2"); // Mask IRQ
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("cli");
    }
}

/// Enable interrupts
#[inline]
fn enable_interrupts() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("csrs sstatus, {}", in(reg) 0x2usize); // Set SIE
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("msr daifclr, #2"); // Unmask IRQ
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("sti");
    }
}
