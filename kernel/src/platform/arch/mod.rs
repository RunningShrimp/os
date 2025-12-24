// Architecture abstraction layer
// Provides a unified interface for architecture-specific operations

use core::arch::asm;

/// Early hardware initialization (called before any other init)
pub fn early_init() {
    crate::drivers::uart::init();
}

/// Disable interrupts and return previous state
#[inline]
pub fn intr_off() -> bool {
    #[cfg(target_arch = "riscv64")]
    {
        let sstatus: usize;
        unsafe {
            asm!("csrrc {}, sstatus, {}", out(reg) sstatus, const 0x2);
        }
        (sstatus & 0x2) != 0
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let daif: u64;
        unsafe {
            asm!("mrs {}, daif", out(reg) daif);
            asm!("msr daifset, #0xf");
        }
        (daif & 0x3c0) == 0
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        let flags: u64;
        unsafe {
            asm!("pushfq; pop {}; cli", out(reg) flags);
        }
        (flags & 0x200) != 0
    }
}

/// Enable interrupts
#[inline]
pub fn intr_on() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("csrsi sstatus, 0x2");
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("msr daifclr, #0xf");
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("sti");
    }
}

/// Check if interrupts are enabled
#[inline]
pub fn intr_get() -> bool {
    #[cfg(target_arch = "riscv64")]
    {
        let sstatus: usize;
        unsafe {
            asm!("csrr {}, sstatus", out(reg) sstatus);
        }
        (sstatus & 0x2) != 0
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let daif: u64;
        unsafe {
            asm!("mrs {}, daif", out(reg) daif);
        }
        (daif & 0x3c0) == 0
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        let flags: u64;
        unsafe {
            asm!("pushfq; pop {}", out(reg) flags);
        }
        (flags & 0x200) != 0
    }
}

/// Wait for interrupt (low-power idle)
#[inline]
pub fn wfi() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("wfi");
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("wfi");
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("hlt");
    }
}

/// Get current hart/core ID
#[inline]
pub fn cpuid() -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        let hartid: usize;
        unsafe {
            asm!("mv {}, tp", out(reg) hartid);
        }
        hartid
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let mpidr: u64;
        unsafe {
            asm!("mrs {}, mpidr_el1", out(reg) mpidr);
        }
        (mpidr & 0xff) as usize
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // For simplicity, assume single CPU
        0
    }
}

/// Get current CPU ID (alias for cpuid)
#[inline]
pub fn current_cpu_id() -> usize {
    cpuid()
}

/// Raise a security exception
pub fn raise_security_exception(message: &str) {
    // Log the security exception
    crate::println!("Security Exception: {}", message);
    
    // For now, we'll just panic, but in a real system this would trigger
    // appropriate security response mechanisms
    panic!("Security exception raised: {}", message);
}

/// Memory barrier
#[inline]
pub fn fence() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("fence");
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("dsb sy");
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("mfence");
    }
}

/// Instruction barrier
#[inline]
pub fn ifence() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        asm!("fence.i");
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("isb");
    }

    #[cfg(target_arch = "x86_64")]
    {
        // x86 has strong memory model, no explicit ifence needed
    }
}

// ============================================================================
// Spectre V2 Mitigation: Retpoline Compiler Barriers
// ============================================================================

/// Retpoline: LFENCE-based speculation barrier for x86_64
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn retpoline_barrier() {
    asm!(
        "lfence",
        options(nostack, preserves_flags)
    );
}

/// Retpoline thunk for indirect calls (x86_64)
///
/// This replaces indirect calls with a call/ret sequence that controls
/// the return address used by the CPU's return predictor.
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn retpoline_thunk(target: *const u8) -> ! {
    let target_value = target;
    asm!(
        "call 2f",
        "1:",
        "pause",
        "lfence",
        "jmp 1b",
        "2:",
        "mov rax, {0}",
        "jmp rax",
        in(reg) target_value,
        options(nostack, noreturn)
    )
}

/// Retpoline thunk for indirect jumps (x86_64)
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn retpoline_jump_thunk(target: *const u8) -> ! {
    let target_value = target;
    asm!(
        "call 2f",
        "1:",
        "pause",
        "lfence",
        "jmp 1b",
        "2:",
        "mov rax, {0}",
        "jmp rax",
        in(reg) target_value,
        options(nostack, noreturn)
    )
}

/// Speculation barrier using LFENCE (x86_64)
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn speculation_barrier() {
    asm!(
        "lfence",
        options(nostack, preserves_flags)
    );
}

/// Speculation barrier using DSB/ISB (AArch64)
#[inline]
#[cfg(target_arch = "aarch64")]
pub unsafe fn speculation_barrier() {
    asm!(
        "dsb sy",
        "isb",
        options(nostack)
    );
}

/// Speculation barrier using FENCE (RISC-V)
#[inline]
#[cfg(target_arch = "riscv64")]
pub unsafe fn speculation_barrier() {
    asm!(
        "fence",
        options(nostack)
    );
}

/// Flush Return Stack Buffer (RSB) on context switch
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn flush_rsb() {
    const RSB_DEPTH: usize = 16;
    for _ in 0..RSB_DEPTH {
        asm!(
            "call 1f",
            "1:",
            "pause",
            "lfence",
            "ret",
            options(nostack)
        );
    }
}

/// Indirect branch predictor barrier (IBPB) simulation
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn indirect_branch_predictor_barrier() {
    speculation_barrier();
}

/// Control flow integrity barrier for Spectre v2
#[inline]
pub fn cfi_barrier() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        speculation_barrier();
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        speculation_barrier();
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        speculation_barrier();
    }
}
