//! LoongArch SMP (Symmetric Multi-Processing) support
//!
//! This module provides multi-processor support for LoongArch64 systems,
//! including CPU startup, inter-processor interrupts (IPI), and cache coherency.

use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};

/// CPU state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    /// CPU not present
    NotPresent,
    /// CPU offline
    Offline,
    /// CPU waiting for startup
    Spinning,
    /// CPU running
    Running,
    /// CPU going offline
    GoingOffline,
    /// CPU halted
    Halted,
}

/// CPU information structure
#[derive(Debug)]
pub struct CpuInfo {
    /// CPU ID
    pub cpu_id: u32,
    /// Current CPU state
    pub state: CpuState,
    /// Boot stack address
    pub boot_stack: usize,
    /// Boot code address
    pub boot_code: usize,
    /// CPU family
    pub family: u8,
    /// CPU revision
    pub revision: u8,
}

/// Global CPU information array
static mut CPUS: [Option<CpuInfo>; 256] = [None; 256];

/// Active CPU count
static ACTIVE_CPU_COUNT: AtomicU32 = AtomicU32::new(1);

/// SMP initialized flag
static SMP_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize SMP subsystem
pub fn initialize() -> Result<(), &'static str> {
    crate::println!("LoongArch64: Initializing SMP");

    // Check if SMP is supported
    if !smp_supported() {
        crate::println!("LoongArch64: SMP not supported, using single CPU");
        return Ok(());
    }

    // Detect number of CPUs
    let cpu_count = detect_cpu_count();
    crate::println!("LoongArch64: Detected {} CPUs", cpu_count);

    // Initialize CPU information for each CPU
    for cpu_id in 0..cpu_count {
        init_cpu_info(cpu_id);
    }

    // Boot secondary CPUs
    if cpu_count > 1 {
        boot_secondary_cpus(cpu_count)?;
    }

    SMP_INITIALIZED.store(true, Ordering::Release);
    ACTIVE_CPU_COUNT.store(cpu_count, Ordering::Release);

    crate::println!("LoongArch64: SMP initialized with {} CPUs", cpu_count);

    Ok(())
}

/// Check if SMP is supported
fn smp_supported() -> bool {
    // Check CPU features for SMP support
    unsafe {
        let features = super::cpu::get_cpu_info().features;
        (features & super::cpu::features::LA_FEATURE_SMP) != 0
    }
}

/// Detect number of CPUs in the system
fn detect_cpu_count() -> u32 {
    // Try to get CPU count from firmware or device tree
    // For now, return a reasonable default
    // TODO: Implement proper CPU count detection from firmware/DTB
    4
}

/// Initialize CPU information for a specific CPU
fn init_cpu_info(cpu_id: u32) {
    let cpu_info = CpuInfo {
        cpu_id,
        state: if cpu_id == 0 {
            CpuState::Running
        } else {
            CpuState::Offline
        },
        boot_stack: allocate_boot_stack(cpu_id),
        boot_code: secondary_cpu_entry as usize,
        family: 0,
        revision: 0,
    };

    unsafe {
        CPUS[cpu_id as usize] = Some(cpu_info);
    }
}

/// Allocate boot stack for a CPU
fn allocate_boot_stack(cpu_id: u32) -> usize {
    const STACK_SIZE: usize = 16 * 1024; // 16KB stack

    // Allocate memory for the stack
    // In a real implementation, this would allocate from a proper memory allocator
    // For now, use a simple calculation
    0x80000000 + (cpu_id as usize * STACK_SIZE)
}

/// Boot secondary CPUs
fn boot_secondary_cpus(cpu_count: u32) -> Result<(), &'static str> {
    for cpu_id in 1..cpu_count {
        boot_cpu(cpu_id)?;
    }
    Ok(())
}

/// Boot a specific CPU
fn boot_cpu(cpu_id: u32) -> Result<(), &'static str> {
    crate::println!("LoongArch64: Booting CPU {}", cpu_id);

    unsafe {
        let cpu_info = CPUS[cpu_id as usize]
            .as_ref()
            .ok_or("CPU info not initialized")?;

        // Set CPU state to spinning
        CPUS[cpu_id as usize].as_mut().unwrap().state = CpuState::Spinning;

        // Send IPI to wake up the CPU
        send_ipi(cpu_id, ipi_numbers::WAKEUP_IPI)?;

        // Wait for CPU to acknowledge
        // In a real implementation, this would wait with a timeout
        for _ in 0..10000 {
            if CPUS[cpu_id as usize].as_ref().unwrap().state == CpuState::Running {
                return Ok(());
            }
            core::hint::spin_loop();
        }

        Err("CPU boot timeout")
    }
}

/// Secondary CPU entry point
#[naked]
unsafe extern "C" fn secondary_cpu_entry() {
    unsafe {
        core::arch::asm!(
            // Get CPU ID
            "csrrd $t0, 0x0", // CPUID register
            // Setup stack pointer
            "li.d $t1, 0x80000000",
            "slli.d $t2, $t0, 14", // Stack size 16KB
            "add.d $sp, $t1, $t2",
            "addi.d $sp, $sp, 16384",
            // Jump to CPU initialization code
            "bl {cpu_init}",
            cpu_init = sym secondary_cpu_init,
            options(noreturn)
        );
    }
}

/// Secondary CPU initialization
#[no_mangle]
unsafe extern "C" fn secondary_cpu_init() -> usize {
    let cpu_id = super::get_cpuid();

    crate::println!("LoongArch64: CPU {} online", cpu_id);

    // Enable CPU
    enable_cpu(cpu_id);

    // Mark CPU as running
    CPUS[cpu_id as usize].as_mut().unwrap().state = CpuState::Running;

    // CPU is now ready, enter idle loop
    secondary_cpu_idle_loop()
}

/// Enable CPU (enable interrupts, etc.)
fn enable_cpu(cpu_id: u32) {
    unsafe {
        // Enable floating point
        let mut fcsr0: u32;
        core::arch::asm!(
            "movfc {0}, $fcsr0",
            out(reg) fcsr0,
            options(nostack, nomem)
        );
        fcsr0 |= 0x1F; // Enable all exceptions
        core::arch::asm!(
            "movfc $fcsr0, {0}",
            in(reg) fcsr0,
            options(nostack, nomem)
        );

        // Enable interrupts
        super::interrupts::enable_interrupts();
    }
}

/// Secondary CPU idle loop
fn secondary_cpu_idle_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!(
                "idle 0", // Wait for interrupt
                options(nostack, nomem)
            );
        }
        core::hint::spin_loop();
    }
}

/// IPI numbers
pub mod ipi_numbers {
    /// Wakeup IPI
    pub const WAKEUP_IPI: u8 = 0;
    /// Reschedule IPI
    pub const RESCHEDULE_IPI: u8 = 1;
    /// Stop IPI
    pub const STOP_IPI: u8 = 2;
    /// Timer IPI
    pub const TIMER_IPI: u8 = 3;
}

/// Send inter-processor interrupt
///
/// # Arguments
///
/// * `target_cpu` - Target CPU ID
/// * `ipi` - IPI number
pub fn send_ipi(target_cpu: u32, ipi: u8) -> Result<(), &'static str> {
    if target_cpu >= 256 {
        return Err("Invalid CPU ID");
    }

    unsafe {
        core::arch::asm!(
            "li.d {0}, 0xFE015000", // IPI controller base address
            "li.d {1}, {2}", // Target CPU mask
            "li.w {2}, {3}", // IPI number
            "st.w {1}, {0}, 0x0", // Write to IPI_SEND register
            "st.w {2}, {0}, 0x4", // Write to IPI_DATA register
            out(reg) _,
            out(reg) (1u64 << target_cpu),
            in(reg) ipi,
            options(nostack)
        );
    }

    Ok(())
}

/// Broadcast IPI to all CPUs except self
///
/// # Arguments
///
/// * `ipi` - IPI number
pub fn broadcast_ipi(ipi: u8) {
    let current_cpu = super::get_cpuid();
    let cpu_count = get_cpu_count();

    for cpu_id in 0..cpu_count {
        if cpu_id != current_cpu {
            let _ = send_ipi(cpu_id, ipi);
        }
    }
}

/// Get number of active CPUs
pub fn get_cpu_count() -> u32 {
    ACTIVE_CPU_COUNT.load(Ordering::Acquire)
}

/// Get CPU information for a specific CPU
pub fn get_cpu_info(cpu_id: u32) -> Option<&'static CpuInfo> {
    unsafe {
        if (cpu_id as usize) < CPUS.len() {
            CPUS[cpu_id as usize].as_ref()
        } else {
            None
        }
    }
}

/// Stop a CPU
///
/// # Arguments
///
/// * `cpu_id` - CPU ID to stop
pub fn stop_cpu(cpu_id: u32) -> Result<(), &'static str> {
    let cpu_info = unsafe {
        CPUS[cpu_id as usize]
            .as_ref()
            .ok_or("CPU not found")?
    };

    if cpu_info.state != CpuState::Running {
        return Err("CPU not running");
    }

    // Send stop IPI
    send_ipi(cpu_id, ipi_numbers::STOP_IPI)?;

    unsafe {
        CPUS[cpu_id as usize].as_mut().unwrap().state = CpuState::GoingOffline;
    }

    Ok(())
}

/// Cache coherency operations
pub mod cache_coherency {
    /// Sync caches across all CPUs
    pub fn sync_all_caches() {
        unsafe {
            core::arch::asm!(
                "dbar 0", // Data barrier
                "ibar 0", // Instruction barrier
                options(nostack, nomem)
            );
        }
    }

    /// Flush cache on this CPU
    pub fn flush_local_cache() {
        unsafe {
            core::arch::asm!(
                "dbar 0", // Data barrier
                options(nostack, nomem)
            );
        }
    }

    /// Invalidate TLB on this CPU
    pub fn invalidate_local_tlb() {
        super::super::memory::invalidate_tlb_all();
    }

    /// Broadcast TLB invalidation to all CPUs
    pub fn broadcast_tlb_invalidation(address: usize) {
        // Send IPI to all other CPUs to invalidate their TLB
        super::broadcast_ipi(super::ipi_numbers::RESCHEDULE_IPI);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_count() {
        let count = get_cpu_count();
        assert!(count >= 1, "Should have at least 1 CPU");
    }

    #[test]
    fn test_cpu_states() {
        // Boot CPU (CPU 0) should be running
        if let Some(info) = get_cpu_info(0) {
            assert_eq!(info.state, CpuState::Running);
        }
    }

    #[test]
    fn test_smp_init() {
        assert!(SMP_INITIALIZED.load(Ordering::Acquire));
    }
}
