//! RISC-V 64 architecture-specific implementation

use crate::arch::{CpuFeatures, MemoryLayout};
use crate::error::{BootError, Result};
use core::arch::asm;

/// Early RISC-V initialization
pub fn early_init() {
    // Basic RISC-V setup would go here
}

/// Get RISC-V CPU features
pub fn get_cpu_features() -> CpuFeatures {
    CpuFeatures {
        has_virtualization: false,
        is_64bit: true,
        cache_line_size: 64,
        cpu_count: 1,
        arch_flags: 0,
    }
}

/// Disable interrupts on RISC-V
pub fn interrupt_disable() {
    unsafe {
        asm!("csrsi sstatus, 1", options(nomem, nostack, preserves_flags));
    }
}

/// Enable interrupts on RISC-V
pub fn interrupt_enable() {
    unsafe {
        asm!("csrsi sstatus, 2", options(nomem, nostack, preserves_flags));
    }
}

/// Check if interrupts are enabled on RISC-V
pub fn interrupt_enabled() -> bool {
    let sstatus: usize;
    unsafe {
        asm!("csrr {}, sstatus", out(reg) sstatus, options(nomem, nostack, preserves_flags));
    }
    (sstatus & 2) != 0
}

/// Wait for interrupt on RISC-V
pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

/// Memory barrier for RISC-V
pub fn memory_barrier() {
    unsafe {
        asm!("fence", options(nomem, nostack, preserves_flags));
    }
}

/// Get current CPU ID on RISC-V
pub fn cpu_id() -> usize {
    let hartid: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) hartid, options(nomem, nostack, preserves_flags));
    }
    hartid
}

/// Jump to kernel entry point on RISC-V
pub unsafe fn jump_to_kernel(entry_point: usize, boot_params: &crate::arch::BootParameters) -> ! {
    let stack_top = crate::arch::Architecture::RiscV64.default_stack_size() - 8;

    asm!(
        "mv sp, {stack_top}",
        "mv a0, {boot_params}",
        "jr {entry_point}",
        stack_top = in(reg) stack_top,
        boot_params = in(reg) boot_params as *const _,
        entry_point = in(reg) entry_point,
        options(nomem, nostack),
    );

    unreachable!();
}

/// Reboot the system on RISC-V
pub fn reboot() -> ! {
    loop {
        wait_for_interrupt();
    }
}

/// Shutdown the system on RISC-V
pub fn shutdown() -> ! {
    loop {
        wait_for_interrupt();
    }
}