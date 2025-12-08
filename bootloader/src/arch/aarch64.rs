//! AArch64 architecture-specific implementation

use crate::arch::{CpuFeatures, MemoryLayout};
use crate::error::{BootError, Result};
use core::arch::asm;

/// Early AArch64 initialization
pub fn early_init() {
    // Basic AArch64 setup would go here
}

/// Get AArch64 CPU features
pub fn get_cpu_features() -> CpuFeatures {
    CpuFeatures {
        has_virtualization: true, // AArch64 always has virtualization
        is_64bit: true,
        cache_line_size: 64,
        cpu_count: 1,
        arch_flags: 0,
    }
}

/// Disable interrupts on AArch64
pub fn interrupt_disable() {
    unsafe {
        asm!("msr daifset, #0xf", options(nomem, nostack, preserves_flags));
    }
}

/// Enable interrupts on AArch64
pub fn interrupt_enable() {
    unsafe {
        asm!("msr daifclr, #0xf", options(nomem, nostack, preserves_flags));
    }
}

/// Check if interrupts are enabled on AArch64
pub fn interrupt_enabled() -> bool {
    let daif: u64;
    unsafe {
        asm!("mrs {}, daif", out(reg) daif, options(nomem, nostack, preserves_flags));
    }
    (daif & 0x3c0) == 0
}

/// Wait for interrupt on AArch64
pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

/// Memory barrier for AArch64
pub fn memory_barrier() {
    unsafe {
        asm!("dsb sy", options(nomem, nostack, preserves_flags));
    }
}

/// Get current CPU ID on AArch64
pub fn cpu_id() -> usize {
    let mpidr: u64;
    unsafe {
        asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nomem, nostack, preserves_flags));
    }
    (mpidr & 0xff) as usize
}

/// Jump to kernel entry point on AArch64
pub unsafe fn jump_to_kernel(entry_point: usize, boot_params: &crate::arch::BootParameters) -> ! {
    let stack_top = crate::arch::Architecture::AArch64.default_stack_size() - 8;

    asm!(
        "mov sp, {stack_top}",
        "mov x0, {boot_params}",
        "br {entry_point}",
        stack_top = in(reg) stack_top,
        boot_params = in(reg) boot_params as *const _,
        entry_point = in(reg) entry_point,
        options(nomem, nostack),
    );

    unreachable!();
}

/// Reboot the system on AArch64
pub fn reboot() -> ! {
    loop {
        wait_for_interrupt();
    }
}

/// Shutdown the system on AArch64
pub fn shutdown() -> ! {
    loop {
        wait_for_interrupt();
    }
}