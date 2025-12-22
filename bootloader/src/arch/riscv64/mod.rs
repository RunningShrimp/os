//! RISC-V 64-bit architecture support

pub use self::cpu::*;

mod cpu {
    pub fn early_init() {
        // RISC-V early initialization
    }

    pub fn get_cpu_features() -> u64 {
        0
    }

    pub fn interrupt_disable() {
        unsafe { core::arch::asm!("csrc mstatus, {}", in(reg) 0x8); }
    }

    pub fn interrupt_enable() {
        unsafe { core::arch::asm!("csrs mstatus, {}", in(reg) 0x8); }
    }

    pub fn interrupt_enabled() -> bool {
        false
    }

    pub fn wait_for_interrupt() {
        // RISC-V wfi instruction
        unsafe { core::arch::asm!("wfi"); }
    }

    pub fn memory_barrier() {
        unsafe { core::arch::asm!("fence"); }
    }

    pub fn cache_invalidate() {
        // No cache invalidation in standard RISC-V
    }

    pub fn tlb_invalidate() {
        unsafe { core::arch::asm!("sfence.vma"); }
    }

    pub fn get_hartid() -> u64 {
        0
    }

    pub fn get_mstatus() -> u64 {
        0
    }
}
