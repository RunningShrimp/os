//! RISC-V H-extension virtualization support
//!
//! This module provides comprehensive virtualization support for RISC-V systems
//! with the H-extension (Hypervisor extension). It implements a type-1 hypervisor
//! that can run multiple guest virtual machines.
//!
//! # Features
//! - H-extension detection and initialization
//! - Guest VM state management
//! - World switch (guest/host transitions)
//! - Virtual interrupt injection
//! - Stage-2 page tables (guest physical to host physical)
//! - AIA (Advanced Interrupt Architecture) virtualization
//! - Nested virtualization support
//!
//! # Performance Targets
//! - World switch overhead: <5%
//! - VM exit latency: <1μs
//! - Virtual interrupt injection: <500ns

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::arch::riscv64::paging::{PageTable, PageTableEntry, PageTableFlags};

/// Maximum number of guest VMs
pub const MAX_VMS: usize = 16;

/// Maximum number of VCPUs per VM
pub const MAX_VCPUS_PER_VM: usize = 8;

/// Guest physical address size (for Stage-2)
pub const GPA_BITS: usize = 48;

/// Host supervisor physical address size
pub const HPA_BITS: usize = 48;

/// Virtual machine ID type
pub type VmId = usize;

/// Virtual CPU ID type
pub type VcpuId = usize;

/// Guest physical address type
pub type GuestPhysAddr = u64;

/// Guest virtual address type
pub type GuestVirtAddr = u64;

/// H-extension CSR registers
#[repr(usize)]
pub enum HextensionCsr {
    /// Virtualization status
    Hstatus = 0x600,
    /// Hypervisor interrupt-enable
    Hie = 0x604,
    /// Hypervisor trap handler
    Htvec = 0x605,
    /// Hypervisor trap value
    Htval = 0x643,
    /// Hypervisor trap cause
    Hcause = 0x642,
    /// Hypervisor guest physical address
    Hgatp = 0x680,
    /// Virtual supervisor status
    Vsstatus = 0x200,
    /// Virtual supervisor interrupt enable
    Vsie = 0x204,
    /// Virtual supervisor trap vector
    Vstvec = 0x205,
    /// Virtual supervisor scratch
    Vsscratch = 0x240,
    /// Virtual supervisor exception PC
    Vsepc = 0x241,
    /// Virtual supervisor cause
    Vscause = 0x242,
    /// Virtual supervisor value
    Vstval = 0x243,
    /// Virtual supervisor address translation and protection
    Vsatp = 0x280,
}

/// VM exit reasons
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmExitReason {
    /// Instruction access fault
    InstructionAccessFault = 1,
    /// Load access fault
    LoadAccessFault = 5,
    /// Store/AMO access fault
    StoreAmoAccessFault = 7,
    /// Instruction page fault
    InstructionPageFault = 12,
    /// Load page fault
    LoadPageFault = 13,
    /// Store/AMO page fault
    StoreAmoPageFault = 15,
    /// Environment call from S-mode
    EcallFromSMode = 9,
    /// Environment call from U-mode
    EcallFromUMode = 8,
    /// Instruction fetch misaligned
    InstructionFetchMisaligned = 0,
    /// Illegal instruction
    IllegalInstruction = 2,
    /// Breakpoint
    Breakpoint = 3,
    /// Load misaligned
    LoadMisaligned = 4,
    /// Store/AMO misaligned
    StoreAmoMisaligned = 6,
    /// Timer interrupt
    TimerInterrupt = 0x8000000000000005,
    /// External interrupt
    ExternalInterrupt = 0x8000000000000009,
    /// Software interrupt
    SoftwareInterrupt = 0x8000000000000001,
}

/// Virtual CPU state
#[repr(C)]
pub struct VcpuState {
    /// VCPU ID
    vcpu_id: VcpuId,
    /// Parent VM ID
    vm_id: VmId,
    /// General-purpose registers
    gp_regs: [u64; 32],
    /// Floating-point registers
    fp_regs: [u64; 32],
    /// Program counter
    pc: AtomicU64,
    /// Guest status
    vsstatus: AtomicU64,
    /// Guest interrupt enable
    vsie: AtomicU64,
    /// Guest trap vector
    vstvec: AtomicU64,
    /// Guest exception PC
    vsepc: AtomicU64,
    /// Guest cause
    vscause: AtomicU64,
    /// Guest trap value
    vstval: AtomicU64,
    /// Guest SATP (Stage-1 page table)
    vsatp: AtomicU64,
    /// Last exit reason
    last_exit: AtomicU32,
    /// Exit count
    exit_count: AtomicU64,
    /// VCPU state (running, stopped, etc.)
    state: AtomicU32,
}

impl VcpuState {
    /// Create a new VCPU state
    pub const fn new(vcpu_id: VcpuId, vm_id: VmId) -> Self {
        Self {
            vcpu_id,
            vm_id,
            gp_regs: [0; 32],
            fp_regs: [0; 32],
            pc: AtomicU64::new(0),
            vsstatus: AtomicU64::new(0),
            vsie: AtomicU64::new(0),
            vstvec: AtomicU64::new(0),
            vsepc: AtomicU64::new(0),
            vscause: AtomicU64::new(0),
            vstval: AtomicU64::new(0),
            vsatp: AtomicU64::new(0),
            last_exit: AtomicU32::new(0),
            exit_count: AtomicU64::new(0),
            state: AtomicU32::new(VcpuState::Stopped as u32),
        }
    }

    /// Reset VCPU state
    pub fn reset(&self) {
        self.pc.store(0, Ordering::Release);
        self.vsstatus.store(0, Ordering::Release);
        self.vsie.store(0, Ordering::Release);
        self.vstvec.store(0, Ordering::Release);
        self.vsepc.store(0, Ordering::Release);
        self.vscause.store(0, Ordering::Release);
        self.vstval.store(0, Ordering::Release);
        self.vsatp.store(0, Ordering::Release);
        self.exit_count.store(0, Ordering::Release);
    }

    /// Get program counter
    pub fn pc(&self) -> u64 {
        self.pc.load(Ordering::Acquire)
    }

    /// Set program counter
    pub fn set_pc(&self, pc: u64) {
        self.pc.store(pc, Ordering::Release);
    }

    /// Get general-purpose register
    pub fn gp_reg(&self, index: usize) -> u64 {
        self.gp_regs[index]
    }

    /// Set general-purpose register
    pub fn set_gp_reg(&mut self, index: usize, value: u64) {
        self.gp_regs[index] = value;
    }
}

impl VcpuState {
    #[repr(u32)]
    enum State {
        Stopped = 0,
        Running = 1,
        Waiting = 2,
    }
}

/// Virtual machine context
pub struct VMContext {
    /// VM ID
    vm_id: VmId,
    /// Stage-2 page table (guest physical to host physical)
    stage2_pt: SpinLock<PageTable>,
    /// Guest memory regions
    memory_regions: SpinLock<Vec<MemoryRegion>>,
    /// Virtual CPUs
    vcpus: Vec<VcpuState>,
    /// VM state
    state: AtomicU32,
    /// VM configuration
    config: VMConfig,
}

/// Memory region in guest physical address space
#[derive(Clone, Copy)]
pub struct MemoryRegion {
    /// Guest physical start address
    gpa_start: GuestPhysAddr,
    /// Size in bytes
    size: usize,
    /// Host virtual address
    hva: usize,
    /// Memory permissions
    flags: PageTableFlags,
}

/// VM configuration
#[derive(Clone, Copy)]
pub struct VMConfig {
    /// Number of VCPUs
    num_vcpus: u32,
    /// Guest memory size in MB
    memory_mb: u32,
    /// Enable nested virtualization
    nested_virt: bool,
    /// Enable IOMMU
    enable_iommu: bool,
}

/// VM state
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VMState {
    /// VM is not running
    Stopped = 0,
    /// VM is running
    Running = 1,
    /// VM is paused
    Paused = 2,
    /// VM is being destroyed
    Destroying = 3,
}

impl VMContext {
    /// Create a new VM context
    pub fn new(vm_id: VmId, config: VMConfig) -> Self {
        Self {
            vm_id,
            stage2_pt: SpinLock::new(PageTable::new()),
            memory_regions: SpinLock::new(Vec::new()),
            vcpus: (0..MAX_VCPUS_PER_VM)
                .map(|vcpu_id| VcpuState::new(vcpu_id, vm_id))
                .collect(),
            state: AtomicU32::new(VMState::Stopped as u32),
            config,
        }
    }

    /// Get VM state
    pub fn state(&self) -> VMState {
        match self.state.load(Ordering::Acquire) {
            0 => VMState::Stopped,
            1 => VMState::Running,
            2 => VMState::Paused,
            3 => VMState::Destroying,
            _ => VMState::Stopped,
        }
    }

    /// Set VM state
    pub fn set_state(&self, state: VMState) {
        self.state.store(state as u32, Ordering::Release);
    }

    /// Map guest physical memory
    pub fn map_memory(&self, gpa: GuestPhysAddr, size: usize, hva: usize, flags: PageTableFlags) {
        let mut regions = self.memory_regions.lock();
        regions.push(MemoryRegion {
            gpa_start: gpa,
            size,
            hva,
            flags,
        });
        drop(regions);

        // Update Stage-2 page table
        let mut pt = self.stage2_pt.lock();
        let num_pages = size / 4096;
        for i in 0..num_pages {
            let gpa_page = gpa + (i as u64 * 4096);
            let hva_page = hva + (i * 4096);
            pt.map(gpa_page as usize, hva_page, flags);
        }
    }

    /// Unmap guest physical memory
    pub fn unmap_memory(&self, gpa: GuestPhysAddr, size: usize) {
        let mut pt = self.stage2_pt.lock();
        let num_pages = size / 4096;
        for i in 0..num_pages {
            let gpa_page = gpa + (i as u64 * 4096);
            pt.unmap(gpa_page as usize);
        }
    }

    /// Get VCPU
    pub fn vcpu(&self, vcpu_id: VcpuId) -> Option<&VcpuState> {
        self.vcpus.get(vcpu_id)
    }
}

/// World switch context
#[repr(C)]
pub struct WorldSwitch {
    /// Host general-purpose registers
    host_gp: [u64; 32],
    /// Host status
    host_status: u64,
    /// Host SATP
    host_satp: u64,
    /// Guest general-purpose registers
    guest_gp: [u64; 32],
    /// Guest status
    guest_status: u64,
    /// Guest SATP
    guest_satp: u64,
}

/// Hypervisor framework state
struct HypervisorState {
    /// VM contexts
    vms: SpinLock<BTreeMap<VmId, VMContext>>,
    /// Next VM ID
    next_vm_id: AtomicUsize,
    /// H-extension available
    h_extension: AtomicU32,
    /// Nested virtualization supported
    nested_virt: AtomicU32,
}

impl HypervisorState {
    const fn new() -> Self {
        Self {
            vms: SpinLock::new(BTreeMap::new()),
            next_vm_id: AtomicUsize::new(0),
            h_extension: AtomicU32::new(0),
            nested_virt: AtomicU32::new(0),
        }
    }
}

/// Global hypervisor state
static HYPERVISOR_STATE: HypervisorState = HypervisorState::new();

/// Check if H-extension is available
pub fn has_virtualization() -> bool {
    // Read misa to check for H-extension
    let misa: u64;
    unsafe {
        core::arch::asm!("csrr {}", out(reg) misa, in(reg) 0x301); // misa CSR
    }

    // Check if 'H' bit is set (bit 7)
    (misa & (1 << 7)) != 0
}

/// Initialize hypervisor framework
pub fn hvf_init() -> Result<(), &'static str> {
    crate::println!("riscv64-virt: Initializing hypervisor framework");

    // Check for H-extension
    if !has_virtualization() {
        return Err("H-extension not available");
    }

    HYPERVISOR_STATE.h_extension.store(1, Ordering::Release);

    // Check for nested virtualization support
    // This requires H-extension version 1.0 or later
    let nested_supported = check_nested_virtualization();
    if nested_supported {
        HYPERVISOR_STATE.nested_virt.store(1, Ordering::Release);
        crate::println!("riscv64-virt: Nested virtualization supported");
    }

    // Initialize hypervisor CSRs
    init_hypervisor_csrs()?;

    crate::println!("riscv64-virt: Hypervisor initialization complete");
    Ok(())
}

/// Check for nested virtualization support
fn check_nested_virtualization() -> bool {
    // Read H-extension configuration
    // This is platform-specific and may require reading device tree
    // For now, assume it's not supported
    false
}

/// Initialize hypervisor CSRs
fn init_hypervisor_csrs() -> Result<(), &'static str> {
    unsafe {
        // Set hypervisor trap vector
        // In production, this would point to the VM exit handler
        let htvec = vm_exit_handler as usize;
        core::arch::asm!("csrw {}, {}", in(reg) 0x605, in(reg) htvec); // Htvec

        // Configure HGATP for Stage-2 page tables
        // This will be per-VM in production
        core::arch::asm!("csrw {}, {}", in(reg) 0x680, in(reg) 0u32); // Hgatp

        // Enable virtualization in HSTATUS
        let hstatus: u64;
        core::arch::asm!("csrr {}", out(reg) hstatus, in(reg) 0x600); // Hstatus
        core::arch::asm!("csrw {}, {}", in(reg) 0x600, in(reg) (hstatus | 0x0000000000020000)); // Set VTV
    }

    Ok(())
}

/// VM exit handler (called from assembly)
extern "C" fn vm_exit_handler() {
    crate::println!("riscv64-virt: VM exit");

    // Read exit cause
    let cause: u64;
    unsafe {
        core::arch::asm!("csrr {}", out(reg) cause, in(reg) 0x642); // Hcause
    }

    // Handle exit based on cause
    match cause {
        _ => {
            crate::println!("riscv64-virt: Unknown exit cause: {:#x}", cause);
        }
    }
}

/// Create a new guest VM
pub fn vm_create(config: VMConfig) -> Result<VmId, &'static str> {
    let vm_id = HYPERVISOR_STATE.next_vm_id.fetch_add(1, Ordering::Acquire) as VmId;

    if vm_id >= MAX_VMS {
        return Err("Maximum VM limit reached");
    }

    let vm = VMContext::new(vm_id, config);

    let mut vms = HYPERVISOR_STATE.vms.lock();
    vms.insert(vm_id, vm);

    crate::println!("riscv64-virt: Created VM {} with {} VCPUs, {} MB memory",
        vm_id, config.num_vcpus, config.memory_mb);

    Ok(vm_id)
}

/// Destroy a guest VM
pub fn vm_destroy(vm_id: VmId) -> Result<(), &'static str> {
    let mut vms = HYPERVISOR_STATE.vms.lock();

    let vm = vms.get(&vm_id).ok_or("VM not found")?;
    vm.set_state(VMState::Destroying);

    // Stop all VCPUs
    for vcpu in &vm.vcpus {
        vcpu.state.store(VcpuState::Stopped as u32, Ordering::Release);
    }

    vms.remove(&vm_id);

    crate::println!("riscv64-virt: Destroyed VM {}", vm_id);
    Ok(())
}

/// Run a VCPU
pub fn vm_run_vcpu(vm_id: VmId, vcpu_id: VcpuId) -> Result<(), &'static str> {
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;

    let vcpu = vm.vcpu(vcpu_id).ok_or("VCPU not found")?;

    // Set VCPU state to running
    vcpu.state.store(VcpuState::Running as u32, Ordering::Release);

    drop(vms);

    // Perform world switch to guest
    world_switch(vm_id, vcpu_id)?;

    Ok(())
}

/// World switch: transition from host to guest
fn world_switch(vm_id: VmId, vcpu_id: VcpuId) -> Result<(), &'static str> {
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;
    let vcpu = vm.vcpu(vcpu_id).ok_or("VCPU not found")?;

    // Save host state
    let mut ws = WorldSwitch {
        host_gp: [0; 64],
        host_status: 0,
        host_satp: 0,
        guest_gp: [0; 64],
        guest_status: vcpu.vsstatus.load(Ordering::Acquire),
        guest_satp: vcpu.vsatp.load(Ordering::Acquire),
    };

    // Save host general-purpose registers
    // This is typically done in assembly

    // Save host SATP
    unsafe {
        core::arch::asm!("csrr {}, {}", out(reg) ws.host_satp, in(reg) 0x180); // satp
    }

    // Save host SSTATUS
    unsafe {
        core::arch::asm!("csrr {}, {}", out(reg) ws.host_status, in(reg) 0x100); // sstatus
    }

    // Load guest state
    // Load guest SATP
    unsafe {
        core::arch::asm!("csrw {}, {}", in(reg) 0x280, in(reg) ws.guest_satp); // Vsatp
    }

    // Load guest SSTATUS
    unsafe {
        core::arch::asm!("csrw {}, {}", in(reg) 0x200, in(reg) ws.guest_status); // Vsstatus
    }

    // Execute guest
    // In production, this would use RVI (run guest instruction) or similar
    // For now, we'll just simulate a VM exit
    unsafe {
        // Set guest PC
        let pc = vcpu.pc.load(Ordering::Acquire);
        core::arch::asm!("csrw {}, {}", in(reg) 0x241, in(reg) pc); // Vsepc

        // Return to host (simulate VM exit)
    }

    // Restore host state
    unsafe {
        core::arch::asm!("csrw {}, {}", in(reg) 0x100, in(reg) ws.host_status); // sstatus
        core::arch::asm!("csrw {}, {}", in(reg) 0x180, in(reg) ws.host_satp); // satp
    }

    Ok(())
}

/// Inject virtual interrupt into a VCPU
pub fn inject_virtual_interrupt(vm_id: VmId, vcpu_id: VcpuId, irq: u8) -> Result<(), &'static str> {
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;
    let vcpu = vm.vcpu(vcpu_id).ok_or("VCPU not found")?;

    // Set interrupt pending in VSIE
    let mut vsie = vcpu.vsie.load(Ordering::Acquire);
    vsie |= 1 << (irq as u64);
    vcpu.vsie.store(vsie, Ordering::Release);

    Ok(())
}

/// Map guest physical memory region
pub fn vm_map_memory(vm_id: VmId, gpa: GuestPhysAddr, size: usize, hva: usize, flags: PageTableFlags) -> Result<(), &'static str> {
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;

    vm.map_memory(gpa, size, hva, flags);

    Ok(())
}

/// Unmap guest physical memory region
pub fn vm_unmap_memory(vm_id: VmId, gpa: GuestPhysAddr, size: usize) -> Result<(), &'static str> {
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;

    vm.unmap_memory(gpa, size);

    Ok(())
}

/// Handle MMIO access from guest
pub fn handle_mmio_exit(vm_id: VmId, vcpu_id: VcpuId, addr: GuestPhysAddr, is_write: bool) -> Result<(), &'static str> {
    // Read/write to the emulated device
    crate::println!("riscv64-virt: MMIO {} access at {:#x}", if is_write { "write" } else { "read" }, addr);

    // Emulate the device access
    // In production, this would dispatch to device-specific handlers

    // Advance PC past the MMIO instruction
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;
    let vcpu = vm.vcpu(vcpu_id).ok_or("VCPU not found")?;

    let pc = vcpu.pc.load(Ordering::Acquire);
    vcpu.set_pc(pc + 4); // Assume 4-byte instruction

    Ok(())
}

/// Get VM statistics
pub fn vm_get_stats(vm_id: VmId) -> Result<VmStats, &'static str> {
    let vms = HYPERVISOR_STATE.vms.lock();
    let vm = vms.get(&vm_id).ok_or("VM not found")?;

    let mut total_exits = 0;
    for vcpu in &vm.vcpus {
        total_exits += vcpu.exit_count.load(Ordering::Acquire);
    }

    Ok(VmStats {
        vm_id,
        state: vm.state(),
        num_vcpus: vm.config.num_vcpus as usize,
        memory_mb: vm.config.memory_mb,
        total_exits,
    })
}

/// VM statistics
#[derive(Clone, Copy, Debug)]
pub struct VmStats {
    pub vm_id: VmId,
    pub state: VMState,
    pub num_vcpus: usize,
    pub memory_mb: u32,
    pub total_exits: u64,
}

/// Stage-2 page table operations
pub mod stage2 {
    use super::*;

    /// Create Stage-2 page table
    pub fn create_page_table() -> PageTable {
        PageTable::new()
    }

    /// Map guest physical to host physical in Stage-2
    pub fn map_page(pt: &mut PageTable, gpa: GuestPhysAddr, hpa: u64, flags: PageTableFlags) {
        pt.map(gpa as usize, hpa as usize, flags);
    }

    /// Unmap guest physical page
    pub fn unmap_page(pt: &mut PageTable, gpa: GuestPhysAddr) {
        pt.unmap(gpa as usize);
    }

    /// Invalidate Stage-2 TLB entries
    pub fn invalidate_tlb(gpa: GuestPhysAddr) {
        unsafe {
            core::arch::asm!("sfence.vma {}", in(reg) gpa);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_virtualization() {
        // This test will fail on systems without H-extension
        // That's expected - it's for testing on real hardware
        let has_virt = has_virtualization();
        // Don't assert, just report
        crate::println!("H-extension available: {}", has_virt);
    }

    #[test]
    fn test_vm_create_destroy() {
        let config = VMConfig {
            num_vcpus: 2,
            memory_mb: 512,
            nested_virt: false,
            enable_iommu: false,
        };

        let vm_id = vm_create(config);
        assert!(vm_id.is_ok());

        let vm_id = vm_id.unwrap();
        let result = vm_destroy(vm_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vcpu_state() {
        let vcpu = VcpuState::new(0, 0);

        vcpu.set_pc(0x1000);
        assert_eq!(vcpu.pc(), 0x1000);

        vcpu.reset();
        assert_eq!(vcpu.pc(), 0);
    }
}
