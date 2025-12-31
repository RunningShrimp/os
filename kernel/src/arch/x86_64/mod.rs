//! x86_64 architecture implementation

pub mod interrupts;
pub mod memory;

/// Initialize x86_64-specific subsystems
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("x86_64: Initializing architecture subsystems");

    // Initialize memory management
    memory::initialize()?;

    // Initialize interrupt handling
    interrupts::initialize()?;

    Ok(())
}

/// Shutdown x86_64-specific subsystems
pub fn shutdown() -> Result<(), &'static str> {
    crate::println!("x86_64: Shutting down architecture subsystems");

    // Shutdown interrupt handling
    interrupts::shutdown()?;

    // Shutdown memory management
    memory::shutdown()?;

    Ok(())
}

/// Read Time-Stamp Counter
#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn rdtsc() -> u64 {
    let (high, low): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("edx") high,
            out("eax") low,
            options(nostack, nomem, pure)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Read Time-Stamp Counter (fallback for non-x86_64)
#[inline]
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn rdtsc() -> u64 {
    // Fallback implementation using a dummy value
    0
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
    unsafe {
        core::arch::asm!(
            "rdrand {}",
            out(reg) value,
            options(nostack, nomem)
        );
    }
    value
}
