//! RISC-V SMP (Symmetric Multi-Processing) support
//!
//! This module provides comprehensive multi-core support for RISC-V systems,
//! including CPU initialization, hotplug support, and inter-core synchronization.
//!
//! # Features
//! - Multi-core boot and initialization
//! - CPU hotplug support (add/remove CPUs dynamically)
//! - Per-CPU data structures
//! - Inter-processor interrupts (IPI)
//! - CPU affinity and mask management
//! - Hart detection and enumeration
//!
//! # Performance Targets
//! - SMP boot time: <100ms for 8 cores
//! - IPI latency: <5μs
//! - Cache coherency: RVWMO memory model

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Maximum number of supported CPUs (harts)
pub const MAX_CPUS: usize = 8;

/// CPU ID type (hart ID)
pub type CpuId = usize;

/// CPU mask type for affinity operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuMask {
    bits: u64,
}

impl CpuMask {
    /// Create an empty CPU mask
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Create a CPU mask with all CPUs
    pub const fn all() -> Self {
        Self { bits: u64::MAX }
    }

    /// Create a CPU mask from a single CPU
    pub const fn from_cpu(cpu: CpuId) -> Self {
        Self { bits: 1 << cpu }
    }

    /// Add a CPU to the mask
    pub fn add(&mut self, cpu: CpuId) {
        self.bits |= 1 << cpu;
    }

    /// Remove a CPU from the mask
    pub fn remove(&mut self, cpu: CpuId) {
        self.bits &= !(1 << cpu);
    }

    /// Check if CPU is in mask
    pub fn contains(&self, cpu: CpuId) -> bool {
        (self.bits & (1 << cpu)) != 0
    }

    /// Get number of CPUs in mask
    pub fn count(&self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Get first CPU in mask
    pub fn first(&self) -> Option<CpuId> {
        if self.bits == 0 {
            None
        } else {
            Some(self.bits.trailing_zeros() as usize)
        }
    }

    /// Iterate over CPUs in mask
    pub fn iter(&self) -> CpuMaskIter {
        CpuMaskIter { mask: self.bits }
    }
}

/// Iterator over CPUs in a mask
pub struct CpuMaskIter {
    mask: u64,
}

impl Iterator for CpuMaskIter {
    type Item = CpuId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            None
        } else {
            let cpu = self.mask.trailing_zeros() as usize;
            self.mask &= self.mask - 1;
            Some(cpu)
        }
    }
}

/// CPU state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CpuState {
    /// CPU is not present
    Offline = 0,
    /// CPU is booting
    Booting = 1,
    /// CPU is online and operational
    Online = 2,
    /// CPU is being hot-unplugged
    OfflinePending = 3,
}

/// Per-CPU data structure
#[repr(C)]
pub struct PerCpuData {
    /// CPU ID (hart ID)
    cpu_id: CpuId,
    /// Current CPU state
    state: AtomicU32,
    /// CPU frequency in MHz
    frequency_mhz: u32,
    /// Stack pointer for this CPU
    stack_ptr: AtomicUsize,
    /// Boot argument pointer
    boot_arg: AtomicUsize,
    /// Number of times this CPU has been scheduled
    schedule_count: AtomicUsize,
    /// CPU-local timer tick count
    tick_count: AtomicU64,
    /// Reserved for future use
    _reserved: [u8; 64],
}

impl PerCpuData {
    /// Create zero-initialized per-CPU data
    pub const fn zero() -> Self {
        Self {
            cpu_id: 0,
            state: AtomicU32::new(CpuState::Offline as u32),
            frequency_mhz: 0,
            stack_ptr: AtomicUsize::new(0),
            boot_arg: AtomicUsize::new(0),
            schedule_count: AtomicUsize::new(0),
            tick_count: AtomicU64::new(0),
            _reserved: [0; 64],
        }
    }

    /// Get CPU ID
    pub fn cpu_id(&self) -> CpuId {
        self.cpu_id
    }

    /// Get CPU state
    pub fn state(&self) -> CpuState {
        match self.state.load(Ordering::Acquire) {
            0 => CpuState::Offline,
            1 => CpuState::Booting,
            2 => CpuState::Online,
            3 => CpuState::OfflinePending,
            _ => CpuState::Offline,
        }
    }

    /// Set CPU state
    pub fn set_state(&self, state: CpuState) {
        self.state.store(state as u32, Ordering::Release);
    }

    /// Get CPU frequency
    pub fn frequency_mhz(&self) -> u32 {
        self.frequency_mhz
    }

    /// Set CPU frequency
    pub fn set_frequency_mhz(&mut self, freq: u32) {
        self.frequency_mhz = freq;
    }

    /// Increment schedule count
    pub fn inc_schedule_count(&self) {
        self.schedule_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get schedule count
    pub fn schedule_count(&self) -> usize {
        self.schedule_count.load(Ordering::Relaxed)
    }

    /// Increment tick count
    pub fn inc_tick_count(&self) {
        self.tick_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count.load(Ordering::Relaxed)
    }
}

/// SMP global state
struct SmpState {
    /// Number of online CPUs
    online_count: AtomicUsize,
    /// Number of total detected CPUs
    total_count: AtomicUsize,
    /// Per-CPU data array
    per_cpu: [PerCpuData; MAX_CPUS],
    /// CPU boot lock (for serializing CPU startup)
    boot_lock: SpinLock<()>,
    /// CPU hotplug lock
    hotplug_lock: SpinLock<()>,
    /// Mapping from hart ID to CPU ID
    hart_to_cpu: BTreeMap<usize, CpuId>,
    /// Boot hart (the hart that initialized the system)
    boot_hart: AtomicUsize,
}

impl SmpState {
    /// Create zero-initialized SMP state
    const fn new() -> Self {
        Self {
            online_count: AtomicUsize::new(0),
            total_count: AtomicUsize::new(0),
            per_cpu: [PerCpuData::zero(); MAX_CPUS],
            boot_lock: SpinLock::new(()),
            hotplug_lock: SpinLock::new(()),
            hart_to_cpu: BTreeMap::new(),
            boot_hart: AtomicUsize::new(0),
        }
    }
}

/// Global SMP state
static SMP_STATE: SmpState = SmpState::new();

/// CPU stack size (4 MB per CPU)
const CPU_STACK_SIZE: usize = 4 * 1024 * 1024;

/// Align stacks to 16 bytes
const CPU_STACK_ALIGN: usize = 16;

/// Get current CPU (hart) ID
#[inline]
pub fn get_cpu_id() -> CpuId {
    let mut hart_id: usize;
    unsafe {
        core::arch::asm!("csrr {}, tp", out(reg) hart_id);
    }
    hart_id
}

/// Get per-CPU data for the current CPU
#[inline]
pub fn this_cpu() -> &'static PerCpuData {
    &SMP_STATE.per_cpu[get_cpu_id()]
}

/// Get per-CPU data for a specific CPU
pub fn get_cpu(cpu_id: CpuId) -> Option<&'static PerCpuData> {
    if cpu_id < MAX_CPUS {
        Some(&SMP_STATE.per_cpu[cpu_id])
    } else {
        None
    }
}

/// Get number of online CPUs
pub fn online_cpus() -> usize {
    SMP_STATE.online_count.load(Ordering::Acquire)
}

/// Get total number of detected CPUs
pub fn total_cpus() -> usize {
    SMP_STATE.total_count.load(Ordering::Acquire)
}

/// Detect and enumerate all harts in the system
fn detect_harts() -> Result<Vec<usize>, &'static str> {
    let mut harts = Vec::new();

    // In QEMU, harts are typically sequential starting from 0
    // On real hardware, we'd read from device tree or hardware registers

    // For now, assume up to 8 harts (0-7)
    // In production, this would read from the device tree or ACPI
    for hart in 0..MAX_CPUS {
        // Try to detect if this hart exists
        // This is a simplified check - real implementation would probe hardware
        harts.push(hart);
    }

    Ok(harts)
}

/// Initialize SMP system
pub fn smp_setup() -> Result<(), &'static str> {
    crate::println!("riscv64-smp: Initializing SMP system");

    // Detect all available harts
    let harts = detect_harts()?;
    crate::println!("riscv64-smp: Detected {} harts", harts.len());

    // Store boot hart ID
    let boot_hart = get_cpu_id();
    SMP_STATE.boot_hart.store(boot_hart, Ordering::Release);
    SMP_STATE.total_count.store(harts.len(), Ordering::Release);

    // Initialize per-CPU data
    for (idx, &hart_id) in harts.iter().enumerate() {
        let cpu_data = &SMP_STATE.per_cpu[idx];
        cpu_data.cpu_id = idx;
        SMP_STATE.hart_to_cpu.insert(hart_id, idx);

        // Boot hart is already online
        if hart_id == boot_hart {
            cpu_data.set_state(CpuState::Online);
            SMP_STATE.online_count.fetch_add(1, Ordering::Release);
            crate::println!("riscv64-smp: Boot hart {} is CPU {}", hart_id, idx);
        } else {
            cpu_data.set_state(CpuState::Offline);
        }
    }

    // Start secondary CPUs
    for &hart_id in harts.iter() {
        if hart_id != boot_hart {
            start_secondary_cpu(hart_id)?;
        }
    }

    // Wait for all CPUs to come online
    cpu_spinup(harts.len())?;

    crate::println!("riscv64-smp: All {} CPUs online", online_cpus());
    Ok(())
}

/// Start a secondary CPU (hart)
fn start_secondary_cpu(hart_id: usize) -> Result<(), &'static str> {
    let _lock = SMP_STATE.boot_lock.lock();

    let cpu_id = *SMP_STATE.hart_to_cpu
        .get(&hart_id)
        .ok_or("Invalid hart ID")?;

    crate::println!("riscv64-smp: Starting CPU {} (hart {})", cpu_id, hart_id);

    // Allocate stack for secondary CPU
    let stack_bottom = allocate_cpu_stack(cpu_id)?;
    let stack_top = stack_bottom + CPU_STACK_SIZE;

    // Store stack pointer
    SMP_STATE.per_cpu[cpu_id].stack_ptr.store(stack_top, Ordering::Release);

    // Set CPU state to booting
    SMP_STATE.per_cpu[cpu_id].set_state(CpuState::Booting);

    // Memory fence to ensure all state is visible before starting CPU
    core::sync::atomic::fence(Ordering::Release);

    // Start the secondary CPU
    // On RISC-V, this typically involves:
    // 1. Writing the entry point to mhartreset or similar CSR
    // 2. Setting the stack pointer
    // 3. Releasing the CPU from WFI (Wait-For-Interrupt)
    //
    // The exact mechanism depends on the platform (QEMU vs real hardware)

    unsafe {
        // For QEMU, we write to the entry point address
        // This is platform-specific!
        let entry_point = secondary_cpu_entry as usize;

        // Set the hart's stack pointer and entry point
        // In production, this would use platform-specific mechanisms
        core::arch::asm!(
            "mv t0, {stack_top}",
            "mv t1, {entry_point}",
            // Store these in platform-specific registers
            stack_top = in(reg) stack_top,
            entry_point = in(reg) entry_point,
        );

        // Send IPI to wake up the hart
        send_ipi(cpu_id, IpiType::Startup)?;
    }

    Ok(())
}

/// Secondary CPU entry point
extern "C" fn secondary_cpu_entry() -> ! {
    let cpu_id = get_cpu_id();
    crate::println!("riscv64-smp: CPU {} (hart {}) is booting", cpu_id, cpu_id);

    // Initialize CPU-local state
    SMP_STATE.per_cpu[cpu_id].set_state(CpuState::Online);
    SMP_STATE.online_count.fetch_add(1, Ordering::Release);

    // Enable interrupts
    unsafe {
        core::arch::asm!("csrsi sstatus, 0x2"); // Enable S-mode interrupts
    }

    crate::println!("riscv64-smp: CPU {} online", cpu_id);

    // Enter idle loop
    loop {
        unsafe {
            core::arch::asm!("wfi"); // Wait for interrupt
        }
    }
}

/// Allocate stack for a CPU
fn allocate_cpu_stack(cpu_id: CpuId) -> Result<usize, &'static str> {
    // In a real implementation, this would allocate from physical memory
    // For now, use a simple calculation
    let stack_base = 0x8000_0000 + (cpu_id * CPU_STACK_SIZE);
    Ok(stack_base)
}

/// Wait for all CPUs to come online
fn cpu_spinup(expected_count: usize) -> Result<(), &'static str> {
    const TIMEOUT_MS: u64 = 1000;
    const CHECK_INTERVAL_US: u64 = 100;

    let start = crate::time::read_time_counter();
    let mut elapsed = 0;

    while elapsed < TIMEOUT_MS * 1000 {
        let online = SMP_STATE.online_count.load(Ordering::Acquire);

        if online >= expected_count {
            return Ok(());
        }

        // Small delay
        crate::time::microdelay(CHECK_INTERVAL_US);

        let now = crate::time::read_time_counter();
        elapsed = now - start;
    }

    Err("CPU spinup timeout")
}

/// Inter-processor interrupt types
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpiType {
    /// Start up a CPU
    Startup = 0,
    /// Request reschedule
    Reschedule = 1,
    /// Stop CPU (hot-unplug)
    Stop = 2,
    /// TLB shootdown
    TlbFlush = 3,
}

/// Send an inter-processor interrupt
pub fn send_ipi(target_cpu: CpuId, ipi_type: IpiType) -> Result<(), &'static str> {
    if target_cpu >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    // Memory fence before sending IPI
    core::sync::atomic::fence(Ordering::Release);

    // In RISC-V, IPIs are typically implemented using MSIP (machine software
    // interrupt pending) registers in the CLINT (Core-Local Interrupt Controller)
    // or using SSIP (supervisor software interrupt pending) in newer designs

    unsafe {
        // Set the software interrupt pending bit for the target hart
        // This is platform-specific - real implementation would write to MSIP/SSIP

        // For CLINT, write to the hart's MSIP register:
        // *(volatile uint32_t*)(clint_base + hart_id * 4 + MSIP_OFFSET) = 1;

        // Pass the IPI type through memory or registers
        SMP_STATE.per_cpu[target_cpu].boot_arg.store(ipi_type as usize, Ordering::Release);
    }

    Ok(())
}

/// Broadcast an IPI to all CPUs
pub fn send_ipi_all(ipi_type: IpiType) {
    for cpu in 0..online_cpus() {
        let _ = send_ipi(cpu, ipi_type);
    }
}

/// Broadcast an IPI to all CPUs except the current one
pub fn send_ipi_others(ipi_type: IpiType) {
    let current = get_cpu_id();
    for cpu in 0..online_cpus() {
        if cpu != current {
            let _ = send_ipi(cpu, ipi_type);
        }
    }
}

/// Handle an incoming IPI
pub fn handle_ipi() {
    let cpu_id = get_cpu_id();

    // Read IPI type from memory
    let ipi_type = SMP_STATE.per_cpu[cpu_id].boot_arg.load(Ordering::Acquire);

    match ipi_type {
        0 => { /* Startup - handled in entry point */ }
        1 => {
            // Reschedule
            SMP_STATE.per_cpu[cpu_id].inc_schedule_count();
            // Trigger scheduler
        }
        2 => {
            // Stop - hot unplug
            SMP_STATE.per_cpu[cpu_id].set_state(CpuState::OfflinePending);
            SMP_STATE.online_count.fetch_sub(1, Ordering::Release);
            unsafe {
                core::arch::asm!("wfi"); // Wait for interrupt indefinitely
            }
        }
        3 => {
            // TLB flush
            unsafe {
                core::arch::asm!("sfence.vma");
            }
        }
        _ => {
            crate::println!("riscv64-smp: Unknown IPI type {} on CPU {}", ipi_type, cpu_id);
        }
    }

    // Clear the IPI
    // In production, this would write to MSIP/SSIP register to clear
}

/// CPU hotplug: Add a CPU
pub fn cpu_add(hart_id: usize) -> Result<CpuId, &'static str> {
    let _lock = SMP_STATE.hotplug_lock.lock();

    let online = SMP_STATE.online_count.load(Ordering::Acquire);
    if online >= MAX_CPUS {
        return Err("Maximum CPU limit reached");
    }

    // Find next available CPU ID
    let cpu_id = SMP_STATE.hart_to_cpu.len();

    SMP_STATE.hart_to_cpu.insert(hart_id, cpu_id);
    SMP_STATE.total_count.fetch_add(1, Ordering::Release);

    start_secondary_cpu(hart_id)?;
    cpu_spinup(online + 1)?;

    Ok(cpu_id)
}

/// CPU hotplug: Remove a CPU
pub fn cpu_remove(cpu_id: CpuId) -> Result<(), &'static str> {
    let _lock = SMP_STATE.hotplug_lock.lock();

    if cpu_id >= MAX_CPUS {
        return Err("Invalid CPU ID");
    }

    let state = SMP_STATE.per_cpu[cpu_id].state();
    if state != CpuState::Online {
        return Err("CPU is not online");
    }

    // Send stop IPI
    send_ipi(cpu_id, IpiType::Stop)?;

    // Wait for CPU to go offline
    const TIMEOUT_MS: u64 = 500;
    let start = crate::time::read_time_counter();

    while crate::time::read_time_counter() - start < TIMEOUT_MS * 1000 {
        if SMP_STATE.per_cpu[cpu_id].state() == CpuState::OfflinePending {
            SMP_STATE.per_cpu[cpu_id].set_state(CpuState::Offline);
            return Ok(());
        }
        crate::time::microdelay(100);
    }

    Err("CPU removal timeout")
}

/// Get CPU load balancing mask
pub fn get_load_balance_mask() -> CpuMask {
    // Simple round-robin load balancing
    // More sophisticated implementations would consider:
    // - CPU utilization
    // - Cache topology
    // - NUMA affinity
    let online = online_cpus();
    let mut mask = CpuMask::empty();
    for cpu in 0..online {
        mask.add(cpu);
    }
    mask
}

/// Shutdown SMP system
pub fn smp_shutdown() -> Result<(), &'static str> {
    crate::println!("riscv64-smp: Shutting down SMP system");

    // Send stop IPIs to all secondary CPUs
    for cpu in 1..online_cpus() {
        let _ = cpu_remove(cpu);
    }

    crate::println!("riscv64-smp: All secondary CPUs stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_mask() {
        let mut mask = CpuMask::empty();
        assert_eq!(mask.count(), 0);
        assert!(!mask.contains(0));

        mask.add(0);
        mask.add(2);
        mask.add(4);
        assert_eq!(mask.count(), 3);
        assert!(mask.contains(0));
        assert!(mask.contains(2));
        assert!(!mask.contains(1));

        mask.remove(2);
        assert_eq!(mask.count(), 2);
        assert!(!mask.contains(2));

        let first = mask.first().unwrap();
        assert_eq!(first, 0);
    }

    #[test]
    fn test_cpu_mask_iter() {
        let mut mask = CpuMask::empty();
        mask.add(1);
        mask.add(3);
        mask.add(5);

        let cpus: Vec<_> = mask.iter().collect();
        assert_eq!(cpus, vec![1, 3, 5]);
    }
}
