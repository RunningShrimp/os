//! CPU Virtualization Implementation
//!
//! This module provides CPU virtualization functionality including vCPU management,
//! VMX/SVM execution, virtual APIC, and vCPU scheduling.
//!
//! # Features
//! - vCPU creation and management
//! - VMX/SVM execution (VMCS/VMCB)
//! - Virtual APIC (vAPIC)
//! - vCPU scheduling with priority and affinity
//! - CPUID emulation
//! - MSR handling
//! - vCPU hot-plug support
//!
//! # Example
//! ```rust
//! use kernel::virtualization::cpu::{VCpu, VCPU_STATE};
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let vcpu = VCpu::new(1, 0)?;
//! vcpu.set_vcpu_state(VCPU_STATE::VCPU_RUNNING)?;
//! vcpu.run()?;
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]
#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::{Mutex, RwLock};


/// CPU virtualization result type
pub type CpuResult<T> = core::result::Result<T, CpuError>;

/// CPU virtualization errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuError {
    /// Invalid vCPU ID
    InvalidVcpuId,
    /// vCPU not found
    VcpuNotFound,
    /// Invalid vCPU state
    InvalidVcpuState,
    /// vCPU creation failed
    VcpuCreationFailed,
    /// vCPU execution failed
    VcpuExecutionFailed,
    /// Invalid CPUID leaf
    InvalidCpuidLeaf,
    /// MSR access failed
    MsrAccessFailed,
    /// Invalid register value
    InvalidRegisterValue,
    /// vCPU busy
    VcpuBusy,
    /// Hot-plug not supported
    HotplugNotSupported,
    /// Scheduling failed
    SchedulingFailed,
    /// Invalid priority
    InvalidPriority,
}

impl core::fmt::Display for CpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVcpuId => write!(f, "Invalid vCPU ID"),
            Self::VcpuNotFound => write!(f, "vCPU not found"),
            Self::InvalidVcpuState => write!(f, "Invalid vCPU state"),
            Self::VcpuCreationFailed => write!(f, "vCPU creation failed"),
            Self::VcpuExecutionFailed => write!(f, "vCPU execution failed"),
            Self::InvalidCpuidLeaf => write!(f, "Invalid CPUID leaf"),
            Self::MsrAccessFailed => write!(f, "MSR access failed"),
            Self::InvalidRegisterValue => write!(f, "Invalid register value"),
            Self::VcpuBusy => write!(f, "vCPU busy"),
            Self::HotplugNotSupported => write!(f, "Hot-plug not supported"),
            Self::SchedulingFailed => write!(f, "Scheduling failed"),
            Self::InvalidPriority => write!(f, "Invalid priority"),
        }
    }
}

/// vCPU state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VCPU_STATE {
    /// vCPU not created
    VCPU_NOT_CREATED,
    /// vCPU created but not running
    VCPU_CREATED,
    /// vCPU running
    VCPU_RUNNING,
    /// vCPU paused
    VCPU_PAUSED,
    /// vCPU stopped
    VCPU_STOPPED,
    /// vCPU waiting for interrupt
    VCPU_HALT,
    /// vCPU ready to run
    VCPU_READY,
    /// vCPU in error state
    VCPU_ERROR,
}

impl VCPU_STATE {
    /// Check if vCPU is runnable
    pub fn is_runnable(&self) -> bool {
        matches!(self, Self::VCPU_READY | Self::VCPU_RUNNING)
    }

    /// Check if vCPU is stopped
    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::VCPU_NOT_CREATED | Self::VCPU_STOPPED | Self::VCPU_ERROR)
    }
}

/// CPUID leaf information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuidLeaf {
    /// Leaf number
    pub leaf: u32,
    /// Sub-leaf number
    pub subleaf: u32,
    /// EAX value
    pub eax: u32,
    /// EBX value
    pub ebx: u32,
    /// ECX value
    pub ecx: u32,
    /// EDX value
    pub edx: u32,
}

impl CpuidLeaf {
    /// Create a new CPUID leaf
    pub const fn new(leaf: u32, subleaf: u32) -> Self {
        Self {
            leaf,
            subleaf,
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        }
    }
}

/// General purpose registers
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GpRegisters {
    /// RAX
    pub rax: u64,
    /// RBX
    pub rbx: u64,
    /// RCX
    pub rcx: u64,
    /// RDX
    pub rdx: u64,
    /// RSI
    pub rsi: u64,
    /// RDI
    pub rdi: u64,
    /// RBP
    pub rbp: u64,
    /// RSP
    pub rsp: u64,
    /// R8-R15
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    /// RIP
    pub rip: u64,
    /// RFLAGS
    pub rflags: u64,
}

impl GpRegisters {
    /// Create zero-initialized registers
    pub const fn new() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0,
        }
    }
}

/// Segment register
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SegmentRegister {
    /// Selector
    pub selector: u16,
    /// Base address
    pub base: u64,
    /// Limit
    pub limit: u32,
    /// Access rights
    pub access_rights: u32,
}

impl SegmentRegister {
    /// Create a null segment
    pub const fn new() -> Self {
        Self {
            selector: 0,
            base: 0,
            limit: 0,
            access_rights: 0,
        }
    }
}

/// Control registers
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ControlRegisters {
    /// CR0
    pub cr0: u64,
    /// CR2
    pub cr2: u64,
    /// CR3
    pub cr3: u64,
    /// CR4
    pub cr4: u64,
    /// CR8 (TPR)
    pub cr8: u64,
}

impl ControlRegisters {
    /// Create default control registers
    pub const fn new() -> Self {
        Self {
            cr0: 0x60000010, // PE, ET
            cr2: 0,
            cr3: 0,
            cr4: 0,
            cr8: 0,
        }
    }
}

/// Descriptor table registers
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DescriptorTableRegister {
    /// Base address
    pub base: u64,
    /// Limit
    pub limit: u16,
}

impl DescriptorTableRegister {
    /// Create a null descriptor table
    pub const fn new() -> Self {
        Self { base: 0, limit: 0 }
    }
}

/// Virtual APIC state
#[derive(Debug, Clone, Copy)]
pub struct VirtualApic {
    /// APIC ID
    pub id: u32,
    /// Version
    pub version: u32,
    /// Task Priority Register
    pub tpr: u32,
    /// Arbitration Priority
    pub apr: u32,
    /// Processor Priority
    pub ppr: u32,
    /// End of Interrupt
    pub eoi: u32,
    /// Logical Destination
    pub ldr: u32,
    /// Destination Format
    pub dfr: u32,
    /// Spurious Interrupt Vector
    pub svr: u32,
    /// In-Service Register
    pub isr: [u32; 8],
    /// Trigger Mode Register
    pub tmr: [u32; 8],
    /// Interrupt Request Register
    pub irr: [u32; 8],
    /// Error Status
    pub esr: u32,
    /// Interrupt Command Low
    pub icr_low: u32,
    /// Interrupt Command High
    pub icr_high: u32,
    /// Timer Local Vector Table
    pub timer_lvt: u32,
    /// Timer Initial Count
    pub timer_initial: u32,
    /// Timer Current Count
    pub timer_current: u32,
    /// Timer Divide Configuration
    pub timer_divide: u32,
}

impl VirtualApic {
    /// Create a new virtual APIC
    pub const fn new() -> Self {
        Self {
            id: 0,
            version: 0x14, // Version 1.4
            tpr: 0,
            apr: 0,
            ppr: 0,
            eoi: 0,
            ldr: 0,
            dfr: 0xFFFFFFFF,
            svr: 0xFF, // Software disabled
            isr: [0; 8],
            tmr: [0; 8],
            irr: [0; 8],
            esr: 0,
            icr_low: 0,
            icr_high: 0,
            timer_lvt: 0x10000, // Masked
            timer_initial: 0,
            timer_current: 0,
            timer_divide: 0,
        }
    }

    /// Get highest priority interrupt from IRR
    pub fn get_highest_irr(&self) -> Option<u8> {
        for (i, &word) in self.irr.iter().enumerate() {
            if word != 0 {
                let offset = word.trailing_zeros() as u8;
                return Some((i * 32) as u8 + offset);
            }
        }
        None
    }

    /// Check if interrupt is in-service
    pub fn is_isr_set(&self, vector: u8) -> bool {
        let word = (vector / 32) as usize;
        let bit = vector % 32;
        (self.isr[word] & (1 << bit)) != 0
    }

    /// Set ISR bit
    pub fn set_isr(&mut self, vector: u8) {
        let word = (vector / 32) as usize;
        let bit = vector % 32;
        self.isr[word] |= 1 << bit;
        // Clear corresponding IRR bit
        self.irr[word] &= !(1 << bit);
    }

    /// Trigger interrupt
    pub fn trigger_interrupt(&mut self, vector: u8) -> bool {
        if self.tpr > 0 && (vector as u32) < (self.tpr & 0xF0) {
            return false; // Priority mask
        }
        let word = (vector / 32) as usize;
        let bit = vector % 32;
        self.irr[word] |= 1 << bit;
        true
    }
}

/// Virtual CPU
pub struct VCpu {
    /// Parent VM ID
    vm_id: u64,
    /// vCPU ID
    pub vcpu_id: usize,
    /// vCPU state
    state: RwLock<VCPU_STATE>,
    /// General purpose registers
    gp_regs: Mutex<GpRegisters>,
    /// Segment registers
    cs: Mutex<SegmentRegister>,
    ds: Mutex<SegmentRegister>,
    es: Mutex<SegmentRegister>,
    fs: Mutex<SegmentRegister>,
    gs: Mutex<SegmentRegister>,
    ss: Mutex<SegmentRegister>,
    tr: Mutex<SegmentRegister>,
    ldtr: Mutex<SegmentRegister>,
    /// Control registers
    cr: Mutex<ControlRegisters>,
    /// Descriptor tables
    gdtr: Mutex<DescriptorTableRegister>,
    idtr: Mutex<DescriptorTableRegister>,
    /// Virtual APIC
    apic: Mutex<VirtualApic>,
    /// CPUID leaves
    cpuid_cache: Mutex<BTreeMap<(u32, u32), CpuidLeaf>>,
    /// MSR values
    msr_store: Mutex<BTreeMap<u32, u64>>,
    /// Physical CPU affinity
    cpu_affinity: Mutex<Vec<usize>>,
    /// vCPU priority (0-255, higher = higher priority)
    priority: AtomicUsize,
    /// Number of executions
    exec_count: AtomicU64,
    /// Number of VM exits
    exit_count: AtomicU64,
    /// Hot-plug enabled
    hotplug_enabled: AtomicBool,
}

impl VCpu {
    /// Create a new vCPU
    ///
    /// # Arguments
    /// * `vm_id` - Parent VM ID
    /// * `vcpu_id` - vCPU identifier
    ///
    /// # Returns
    /// * `CpuResult<Self>` - New vCPU instance
    pub fn new(vm_id: u64, vcpu_id: usize) -> CpuResult<Self> {
        // Initialize CPUID cache with standard leaves
        let mut cpuid_cache = BTreeMap::new();

        // CPUID leaf 0: Vendor string
        cpuid_cache.insert((0, 0), CpuidLeaf {
            leaf: 0,
            subleaf: 0,
            eax: 0x0D, // Maximum leaf
            ebx: 0x756E6547, // "Genu"
            ecx: 0x6C65746E, // "ntel"
            edx: 0x49656E69, // "ineI"
        });

        // CPUID leaf 1: Feature info
        cpuid_cache.insert((1, 0), CpuidLeaf {
            leaf: 1,
            subleaf: 0,
            eax: 0x000306C3, // Family 6, Model 60, Stepping 3
            ebx: 0,
            ecx: 0x00013FFD, // Features (SSE4.2, x2APIC, etc.)
            edx: 0x078BFBFF, // Features (FPU, MMX, SSE, etc.)
        });

        Ok(Self {
            vm_id,
            vcpu_id,
            state: RwLock::new(VCPU_STATE::VCPU_CREATED),
            gp_regs: Mutex::new(GpRegisters::new()),
            cs: Mutex::new(SegmentRegister::new()),
            ds: Mutex::new(SegmentRegister::new()),
            es: Mutex::new(SegmentRegister::new()),
            fs: Mutex::new(SegmentRegister::new()),
            gs: Mutex::new(SegmentRegister::new()),
            ss: Mutex::new(SegmentRegister::new()),
            tr: Mutex::new(SegmentRegister::new()),
            ldtr: Mutex::new(SegmentRegister::new()),
            cr: Mutex::new(ControlRegisters::new()),
            gdtr: Mutex::new(DescriptorTableRegister::new()),
            idtr: Mutex::new(DescriptorTableRegister::new()),
            apic: Mutex::new(VirtualApic::new()),
            cpuid_cache: Mutex::new(cpuid_cache),
            msr_store: Mutex::new(BTreeMap::new()),
            cpu_affinity: Mutex::new(Vec::new()),
            priority: AtomicUsize::new(128), // Default priority
            exec_count: AtomicU64::new(0),
            exit_count: AtomicU64::new(0),
            hotplug_enabled: AtomicBool::new(false),
        })
    }

    /// Run vCPU
    pub fn run(&self) -> CpuResult<()> {
        let mut state = self.state.write();
        if !state.is_runnable() {
            return Err(CpuError::InvalidVcpuState);
        }

        *state = VCPU_STATE::VCPU_RUNNING;
        drop(state);

        self.exec_count.fetch_add(1, Ordering::SeqCst);

        // In real implementation, enter VM with VMRESUME/VMLAUNCH
        // This is a simplified placeholder

        Ok(())
    }

    /// Pause vCPU
    pub fn pause(&self) -> CpuResult<()> {
        let mut state = self.state.write();
        if *state != VCPU_STATE::VCPU_RUNNING {
            return Err(CpuError::InvalidVcpuState);
        }

        *state = VCPU_STATE::VCPU_PAUSED;
        Ok(())
    }

    /// Stop vCPU
    pub fn stop(&self) -> CpuResult<()> {
        let mut state = self.state.write();
        *state = VCPU_STATE::VCPU_STOPPED;
        Ok(())
    }

    /// Get vCPU state
    pub fn get_state(&self) -> VCPU_STATE {
        *self.state.read()
    }

    /// Set vCPU state
    pub fn set_state(&self, new_state: VCPU_STATE) -> CpuResult<()> {
        let mut state = self.state.write();

        // Validate state transition
        match (*state, new_state) {
            (VCPU_STATE::VCPU_CREATED, VCPU_STATE::VCPU_READY) => {},
            (VCPU_STATE::VCPU_READY, VCPU_STATE::VCPU_RUNNING) => {},
            (VCPU_STATE::VCPU_RUNNING, VCPU_STATE::VCPU_PAUSED) => {},
            (VCPU_STATE::VCPU_PAUSED, VCPU_STATE::VCPU_RUNNING) => {},
            (_, VCPU_STATE::VCPU_STOPPED) => {},
            _ => return Err(CpuError::InvalidVcpuState),
        }

        *state = new_state;
        Ok(())
    }

    /// Set vCPU state (legacy name for compatibility)
    pub fn set_vcpu_state(&self, state: VCPU_STATE) -> CpuResult<()> {
        self.set_state(state)
    }

    /// Get general purpose registers
    pub fn get_gp_regs(&self) -> GpRegisters {
        *self.gp_regs.lock()
    }

    /// Set general purpose registers
    pub fn set_gp_regs(&self, regs: GpRegisters) -> CpuResult<()> {
        *self.gp_regs.lock() = regs;
        Ok(())
    }

    /// Emulate CPUID
    ///
    /// # Arguments
    /// * `leaf` - CPUID leaf
    /// * `subleaf` - CPUID sub-leaf
    ///
    /// # Returns
    /// * `CpuResult<CpuidLeaf>` - CPUID result
    pub fn emulate_cpuid(&self, leaf: u32, subleaf: u32) -> CpuResult<CpuidLeaf> {
        let cache = self.cpuid_cache.lock();
        if let Some(&result) = cache.get(&(leaf, subleaf)) {
            return Ok(result);
        }

        // Default: execute real CPUID
        let mut result = CpuidLeaf::new(leaf, subleaf);

        // ARM64 doesn't have CPUID instruction
        // TODO: Implement MIDR_EL1 read for CPU info on ARM64
        // For now, return cached values or zeros
        unsafe {
            let mut eax = leaf;
            let mut ebx: u32 = 0;
            let mut ecx = subleaf;
            let mut edx: u32 = 0;

            // ARM64 stub: Read MIDR_EL1 for CPU identification
            // mrs x0, MIDR_EL1
            // For now, return default values
            match leaf {
                0 => {
                    // Vendor string and max leaf
                    eax = 0x0D;
                    ebx = 0x756E6547; // "Genu"
                    ecx = 0x6C65746E; // "ntel"
                    edx = 0x49656E69; // "ineI"
                }
                1 => {
                    // Feature info
                    eax = 0x000306C3;
                    ebx = 0;
                    ecx = 0x00013FFD;
                    edx = 0x078BFBFF;
                }
                _ => {
                    eax = 0;
                    ebx = 0;
                    ecx = 0;
                    edx = 0;
                }
            }

            result.eax = eax;
            result.ebx = ebx;
            result.ecx = ecx;
            result.edx = edx;
        }

        Ok(result)
    }

    /// Read MSR
    ///
    /// # Arguments
    /// * `msr` - MSR address
    ///
    /// # Returns
    /// * `CpuResult<u64>` - MSR value
    pub fn read_msr(&self, msr: u32) -> CpuResult<u64> {
        let store = self.msr_store.lock();
        if let Some(&value) = store.get(&msr) {
            return Ok(value);
        }

        // ARM64 uses different system registers (MRS/MSR instructions)
        // TODO: Implement MRS for ARM64 system registers
        // For now, return stub values for common MSRs
        unsafe {
            let value = match msr {
                0x830 => 0x00070406, // IA32_CR_PAT
                0xC0000080 => 0x0000, // IA32_EFER
                0xC0000081 => 0x0000, // IA32_STAR
                0xC0000082 => 0x0000, // IA32_LSTAR
                0xC0000083 => 0x0000, // IA32_CSTAR
                0xC0000084 => 0x0000, // IA32_FMASK
                0x174 => 0x0000, // IA32_SYSENTER_CS
                0x175 => 0x0000, // IA32_SYSENTER_ESP
                0x176 => 0x0000, // IA32_SYSENTER_EIP
                _ => 0,
            };
            Ok(value)
        }
    }

    /// Write MSR
    ///
    /// # Arguments
    /// * `msr` - MSR address
    /// * `value` - Value to write
    pub fn write_msr(&self, msr: u32, value: u64) -> CpuResult<()> {
        let mut store = self.msr_store.lock();

        // Check if MSR is writable
        match msr {
            0xC0000080 | 0xC0000081 | 0xC0000082 | 0xC0000083 | 0xC0000084 => {
                // Extended MSRs
            }
            0x174 | 0x175 | 0x176 | 0x177 | 0x178 => {
                // SYSENTER MSRs
            }
            _ => return Err(CpuError::MsrAccessFailed),
        }

        store.insert(msr, value);
        Ok(())
    }

    /// Set CPU affinity
    ///
    /// # Arguments
    /// * `cpus` - List of physical CPU IDs
    pub fn set_affinity(&self, cpus: Vec<usize>) -> CpuResult<()> {
        *self.cpu_affinity.lock() = cpus;
        Ok(())
    }

    /// Get CPU affinity
    pub fn get_affinity(&self) -> Vec<usize> {
        self.cpu_affinity.lock().clone()
    }

    /// Set vCPU priority
    ///
    /// # Arguments
    /// * `priority` - Priority (0-255, higher = higher priority)
    pub fn set_priority(&self, priority: usize) -> CpuResult<()> {
        if priority > 255 {
            return Err(CpuError::InvalidPriority);
        }
        self.priority.store(priority, Ordering::SeqCst);
        Ok(())
    }

    /// Get vCPU priority
    pub fn get_priority(&self) -> usize {
        self.priority.load(Ordering::SeqCst)
    }

    /// Get vCPU statistics
    pub fn get_stats(&self) -> VCpuStats {
        VCpuStats {
            exec_count: self.exec_count.load(Ordering::SeqCst),
            exit_count: self.exit_count.load(Ordering::SeqCst),
            priority: self.priority.load(Ordering::SeqCst),
        }
    }

    /// Inject interrupt into vCPU
    ///
    /// # Arguments
    /// * `vector` - Interrupt vector
    pub fn inject_interrupt(&self, vector: u8) -> CpuResult<()> {
        let mut apic = self.apic.lock();
        if apic.trigger_interrupt(vector) {
            Ok(())
        } else {
            Err(CpuError::InvalidVcpuState)
        }
    }

    /// Enable hot-plug
    pub fn enable_hotplug(&self) -> CpuResult<()> {
        self.hotplug_enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Check if hot-plug is enabled
    pub fn is_hotplug_enabled(&self) -> bool {
        self.hotplug_enabled.load(Ordering::SeqCst)
    }
}

/// vCPU statistics
#[derive(Debug, Clone, Copy)]
pub struct VCpuStats {
    /// Number of executions
    pub exec_count: u64,
    /// Number of VM exits
    pub exit_count: u64,
    /// vCPU priority
    pub priority: usize,
}

/// vCPU scheduler
pub struct VCpuScheduler {
    /// Run queue per priority level
    run_queues: [Mutex<Vec<Arc<VCpu>>>; 256],
    /// Current scheduling epoch
    epoch: AtomicU64,
    /// Scheduler enabled
    enabled: AtomicBool,
}

impl VCpuScheduler {
    /// Create a new vCPU scheduler
    pub fn new() -> Self {
        const INIT: Mutex<Vec<Arc<VCpu>>> = Mutex::new(Vec::new());
        Self {
            run_queues: [INIT; 256],
            epoch: AtomicU64::new(0),
            enabled: AtomicBool::new(false),
        }
    }

    /// Enable scheduler
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable scheduler
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Add vCPU to run queue
    ///
    /// # Arguments
    /// * `vcpu` - vCPU to schedule
    pub fn add_vcpu(&self, vcpu: Arc<VCpu>) -> CpuResult<()> {
        let priority = vcpu.get_priority();
        self.run_queues[priority].lock().push(vcpu);
        Ok(())
    }

    /// Remove vCPU from run queue
    ///
    /// # Arguments
    /// * `vcpu_id` - vCPU ID
    pub fn remove_vcpu(&self, vcpu_id: usize) -> CpuResult<()> {
        for queue in &self.run_queues {
            let mut q = queue.lock();
            if let Some(pos) = q.iter().position(|v| v.vcpu_id == vcpu_id) {
                q.remove(pos);
                return Ok(());
            }
        }
        Err(CpuError::VcpuNotFound)
    }

    /// Schedule next vCPU
    ///
    /// # Returns
    /// * `Option<Arc<VCpu>>` - Next vCPU to run, or None
    pub fn schedule(&self) -> Option<Arc<VCpu>> {
        if !self.enabled.load(Ordering::SeqCst) {
            return None;
        }

        // Find highest priority non-empty queue
        for queue in (0..256).rev() {
            let mut q = self.run_queues[queue].lock();
            if let Some(_vcpu) = q.first() {
                // Rotate queue
                let vcpu = q.remove(0);
                q.push(vcpu.clone());
                return Some(vcpu);
            }
        }

        None
    }

    /// Increment epoch
    pub fn next_epoch(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcpu_creation() {
        let vcpu = VCpu::new(1, 0).unwrap();
        assert_eq!(vcpu.get_state(), VCPU_STATE::VCPU_CREATED);
        assert_eq!(vcpu.vcpu_id, 0);
        assert_eq!(vcpu.vm_id, 1);
    }

    #[test]
    fn test_vcpu_state_transitions() {
        let vcpu = VCpu::new(1, 0).unwrap();

        // Created -> Ready
        vcpu.set_state(VCPU_STATE::VCPU_READY).unwrap();
        assert_eq!(vcpu.get_state(), VCPU_STATE::VCPU_READY);

        // Ready -> Running
        vcpu.set_state(VCPU_STATE::VCPU_RUNNING).unwrap();
        assert_eq!(vcpu.get_state(), VCPU_STATE::VCPU_RUNNING);

        // Running -> Paused
        vcpu.set_state(VCPU_STATE::VCPU_PAUSED).unwrap();
        assert_eq!(vcpu.get_state(), VCPU_STATE::VCPU_PAUSED);
    }

    #[test]
    fn test_gp_registers() {
        let vcpu = VCpu::new(1, 0).unwrap();
        let mut regs = GpRegisters::new();
        regs.rax = 0xDEADBEEF;
        regs.rbx = 0xCAFEBABE;

        vcpu.set_gp_regs(regs).unwrap();
        let retrieved = vcpu.get_gp_regs();
        assert_eq!(retrieved.rax, 0xDEADBEEF);
        assert_eq!(retrieved.rbx, 0xCAFEBABE);
    }

    #[test]
    fn test_cpuid_emulation() {
        let vcpu = VCpu::new(1, 0).unwrap();
        let result = vcpu.emulate_cpuid(0, 0).unwrap();
        assert_eq!(result.leaf, 0);
    }

    #[test]
    fn test_priority() {
        let vcpu = VCpu::new(1, 0).unwrap();
        assert_eq!(vcpu.get_priority(), 128); // Default

        vcpu.set_priority(200).unwrap();
        assert_eq!(vcpu.get_priority(), 200);

        let result = vcpu.set_priority(300);
        assert!(result.is_err()); // Invalid
    }

    #[test]
    fn test_virtual_apic() {
        let mut apic = VirtualApic::new();
        assert!(!apic.is_isr_set(32));

        apic.trigger_interrupt(32);
        assert!(apic.get_highest_irr().is_some());

        apic.set_isr(32);
        assert!(apic.is_isr_set(32));
    }

    #[test]
    fn test_scheduler() {
        let scheduler = VCpuScheduler::new();
        scheduler.enable();

        let vcpu1 = Arc::new(VCpu::new(1, 0).unwrap());
        vcpu1.set_priority(200).unwrap();

        let vcpu2 = Arc::new(VCpu::new(1, 1).unwrap());
        vcpu2.set_priority(100).unwrap();

        scheduler.add_vcpu(vcpu1.clone()).unwrap();
        scheduler.add_vcpu(vcpu2.clone()).unwrap();

        let next = scheduler.schedule();
        assert!(next.is_some());
    }
}
