// SMP (Symmetric Multi-Processing) Support
// Per-CPU data structures and multi-core management

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::cell::UnsafeCell;
use crate::process::{Context, Pid};

/// Maximum number of CPUs supported
pub const NCPU: usize = 8;

/// Per-CPU load statistics
#[derive(Default)]
pub struct CpuLoadStats {
    /// Total time spent running (in ticks)
    pub run_time: u64,
    /// Total time spent idle (in ticks)
    pub idle_time: u64,
    /// Last update tick
    pub last_update: u64,
    /// Number of context switches
    pub context_switches: u64,
}

impl CpuLoadStats {
    pub const fn new() -> Self {
        Self {
            run_time: 0,
            idle_time: 0,
            last_update: 0,
            context_switches: 0,
        }
    }
}

/// Per-CPU state
#[repr(C)]
pub struct CpuInfo {
    /// CPU ID (hart ID for RISC-V, core ID for others)
    pub id: usize,
    
    /// Is this CPU started?
    pub started: AtomicBool,
    
    /// Current running process PID (None if idle)
    pub proc: Option<Pid>,
    
    /// Scheduler context for this CPU
    pub context: Context,
    
    /// Interrupt disable nesting depth
    pub noff: i32,
    
    /// Were interrupts enabled before push_off?
    pub intena: bool,
    
    /// Load statistics for this CPU
    pub load_stats: CpuLoadStats,
    
    /// Is CPU in deep sleep mode?
    pub deep_sleep: AtomicBool,
}

impl CpuInfo {
    pub const fn new() -> Self {
        Self {
            id: 0,
            started: AtomicBool::new(false),
            proc: None,
            context: Context::new(),
            noff: 0,
            intena: false,
            load_stats: CpuLoadStats::new(),
            deep_sleep: AtomicBool::new(false),
        }
    }
    
    /// Update load statistics
    pub fn update_load_stats(&mut self, is_idle: bool) {
        let current_tick = crate::time::get_ticks();
        let elapsed = current_tick.saturating_sub(self.load_stats.last_update);
        
        if is_idle {
            self.load_stats.idle_time += elapsed;
        } else {
            self.load_stats.run_time += elapsed;
        }
        
        self.load_stats.last_update = current_tick;
    }
    
    /// Get CPU utilization percentage (0-100)
    pub fn utilization(&self) -> u32 {
        let total = self.load_stats.run_time + self.load_stats.idle_time;
        if total == 0 {
            return 0;
        }
        ((self.load_stats.run_time * 100) / total) as u32
    }
    
    /// Check if CPU should enter deep sleep
    pub fn should_deep_sleep(&self) -> bool {
        // Enter deep sleep if utilization is very low (< 5%) and CPU has been idle
        self.utilization() < 5 && self.proc.is_none()
    }
}

/// Per-CPU data array
/// Using UnsafeCell because each CPU only accesses its own data
struct PerCpu {
    cpus: [UnsafeCell<CpuInfo>; NCPU],
}

unsafe impl Sync for PerCpu {}

static CPUS: PerCpu = PerCpu {
    cpus: [
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
        UnsafeCell::new(CpuInfo::new()),
    ],
};

/// Number of CPUs that have started
static NCPUS_STARTED: AtomicUsize = AtomicUsize::new(0);

/// Boot CPU has finished initialization
static BOOT_COMPLETE: AtomicBool = AtomicBool::new(false);

// ============================================================================
// CPU ID Detection
// ============================================================================

/// Get current CPU ID
#[inline]
pub fn cpuid() -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        let hartid: usize;
        unsafe {
            core::arch::asm!("mv {}, tp", out(reg) hartid);
        }
        hartid
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let mpidr: u64;
        unsafe {
            core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr);
        }
        (mpidr & 0xff) as usize
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // Read APIC ID from CPUID
        // CPUID with EAX=1 returns initial APIC ID in EBX[31:24]
        let apic_id: u32;
        unsafe {
            let ebx: u32;
            core::arch::asm!(
                "push rbx",
                "mov eax, 1",
                "cpuid",
                "mov {0:e}, ebx",
                "pop rbx",
                out(reg) ebx,
                out("eax") _,
                out("ecx") _,
                out("edx") _,
            );
            apic_id = ebx >> 24;
        }
        apic_id as usize
    }
}

// ============================================================================
// Per-CPU Data Access
// ============================================================================

/// Get mutable reference to current CPU's data
/// # Safety
/// Must be called with interrupts disabled
#[inline]
pub fn mycpu() -> &'static mut CpuInfo {
    let id = cpuid();
    debug_assert!(id < NCPU, "CPU ID out of range");
    unsafe { &mut *CPUS.cpus[id].get() }
}

/// Get reference to a specific CPU's data
pub fn cpu(id: usize) -> &'static CpuInfo {
    debug_assert!(id < NCPU, "CPU ID out of range");
    unsafe { &*CPUS.cpus[id].get() }
}

/// Get mutable reference to a specific CPU's data
/// # Safety  
/// Caller must ensure exclusive access
pub unsafe fn cpu_mut(id: usize) -> &'static mut CpuInfo {
    debug_assert!(id < NCPU, "CPU ID out of range");
    &mut *CPUS.cpus[id].get()
}

// ============================================================================
// Interrupt Control with Nesting
// ============================================================================

/// Disable interrupts and track nesting level
/// Called at the start of critical sections
pub fn push_off() {
    let old = intr_get();
    intr_off();
    
    let cpu = mycpu();
    if cpu.noff == 0 {
        cpu.intena = old;
    }
    cpu.noff += 1;
}

/// Re-enable interrupts if we've popped all push_off calls
pub fn pop_off() {
    let cpu = mycpu();
    
    debug_assert!(!intr_get(), "pop_off: interrupts enabled");
    debug_assert!(cpu.noff >= 1, "pop_off: noff < 1");
    
    cpu.noff -= 1;
    if cpu.noff == 0 && cpu.intena {
        intr_on();
    }
}

// ============================================================================
// Low-level Interrupt Control
// ============================================================================

#[inline]
fn intr_off() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("csrc sstatus, {}", in(reg) 1usize << 1);
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr daifset, #0xf");
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli");
    }
}

#[inline]
fn intr_on() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("csrs sstatus, {}", in(reg) 1usize << 1);
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr daifclr, #0xf");
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sti");
    }
}

#[inline]
fn intr_get() -> bool {
    #[cfg(target_arch = "riscv64")]
    {
        let sstatus: usize;
        unsafe {
            core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);
        }
        (sstatus & (1 << 1)) != 0
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let daif: u64;
        unsafe {
            core::arch::asm!("mrs {}, daif", out(reg) daif);
        }
        (daif & 0x3c0) == 0
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        let flags: u64;
        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) flags);
        }
        (flags & (1 << 9)) != 0
    }
}

// ============================================================================
// SMP Initialization
// ============================================================================

/// Initialize the boot CPU
pub fn init_boot_cpu() {
    let id = cpuid();
    
    let cpu = unsafe { cpu_mut(id) };
    cpu.id = id;
    cpu.started.store(true, Ordering::SeqCst);
    cpu.noff = 0;
    cpu.intena = false;
    
    NCPUS_STARTED.fetch_add(1, Ordering::SeqCst);
    
    crate::println!("cpu: boot CPU {} initialized", id);
}

/// Mark boot complete, allowing APs to start
pub fn boot_complete() {
    BOOT_COMPLETE.store(true, Ordering::SeqCst);
    core::sync::atomic::fence(Ordering::SeqCst);
}

/// Check if boot is complete
pub fn is_boot_complete() -> bool {
    BOOT_COMPLETE.load(Ordering::SeqCst)
}

/// Initialize an application processor (AP)
pub fn init_ap() {
    // Wait for boot CPU to finish
    while !is_boot_complete() {
        core::hint::spin_loop();
    }
    
    let id = cpuid();
    
    let cpu = unsafe { cpu_mut(id) };
    cpu.id = id;
    cpu.started.store(true, Ordering::SeqCst);
    cpu.noff = 0;
    cpu.intena = false;
    
    let n = NCPUS_STARTED.fetch_add(1, Ordering::SeqCst);
    crate::println!("cpu: AP {} started (total: {})", id, n + 1);
}

/// Get number of started CPUs
pub fn ncpus() -> usize {
    NCPUS_STARTED.load(Ordering::SeqCst)
}

// ============================================================================
// Inter-Processor Interrupts (IPI)
// ============================================================================

/// Send IPI to a specific CPU
#[allow(unused)]
pub fn send_ipi(target_cpu: usize) {
    #[cfg(target_arch = "riscv64")]
    {
        // Use SBI to send IPI
        // sbi_send_ipi(hart_mask)
        let hart_mask: usize = 1 << target_cpu;
        unsafe {
            core::arch::asm!(
                "li a7, 0x735049", // sbi_send_ipi extension
                "li a6, 0",
                "mv a0, {mask}",
                "li a1, 0",
                "ecall",
                mask = in(reg) hart_mask,
                out("a0") _,
                out("a1") _,
                out("a6") _,
                out("a7") _,
            );
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // Use GIC SGI (Software Generated Interrupt)
        // This is simplified; real implementation needs GIC driver
        let _ = target_cpu;
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // Use APIC to send IPI
        // This is simplified; real implementation needs APIC driver
        let _ = target_cpu;
    }
}

/// Broadcast IPI to all other CPUs
#[allow(unused)]
pub fn broadcast_ipi() {
    let my_id = cpuid();
    for i in 0..NCPU {
        if i != my_id && cpu(i).started.load(Ordering::SeqCst) {
            send_ipi(i);
        }
    }
}

// ============================================================================
// AP Startup (Architecture-specific)
// ============================================================================

/// Start all application processors
#[cfg(target_arch = "riscv64")]
pub fn start_aps() {
    // On RISC-V with OpenSBI, other harts are typically started by
    // the bootloader. We just need to wait for them.
    crate::println!("cpu: waiting for other harts to start...");
    
    // In QEMU virt machine, all harts start running
    // They spin waiting for BOOT_COMPLETE
}

#[cfg(target_arch = "aarch64")]
pub fn start_aps() {
    // On AArch64, we need to use PSCI to bring up secondary cores
    // This is simplified
    crate::println!("cpu: starting secondary cores via PSCI...");
    
    // PSCI CPU_ON call would go here
}

#[cfg(target_arch = "x86_64")]
pub fn start_aps() {
    // On x86_64, we need to:
    // 1. Set up AP boot code at a known address (e.g., 0x8000)
    // 2. Send INIT IPI
    // 3. Send STARTUP IPI
    crate::println!("cpu: starting APs via INIT/SIPI...");
    
    // APIC initialization would go here
}
