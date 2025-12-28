#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! x86_64 architecture-specific functions

/// Read Time-Stamp Counter
#[inline]
pub unsafe fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "rdtsc",
        out("eax") low,
        out("edx") high,
        options(nostack, nomem)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Check if RDRAND instruction is available
pub fn has_rdrand() -> bool {
    // In a real implementation, this would check CPUID
    // For now, assume it's available on x86_64
    true
}

/// Read random number using RDRAND
#[inline]
pub unsafe fn rdrand64() -> u64 {
    let mut value: u64;
    core::arch::asm!(
        "rdrand {}",
        out(reg) value,
        options(nostack, nomem)
    );
    value
}
