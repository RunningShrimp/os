//! # Hypervisor Interface and VM Management
//!
//! This module provides a comprehensive hypervisor interface for managing virtual machines.
//! It handles VM creation, destruction, vCPU management, and VM exit processing.
//!
//! ## Architecture
//!
//! The hypervisor interface provides:
//! - **VM Lifecycle**: Create, run, pause, and destroy VMs
//! - **vCPU Management**: Create and manage virtual CPUs
//! - **VM Exits**: Handle exits from VM to hypervisor
//! - **Resource Management**: Manage memory, I/O, and other resources
//!
//! ## VM Exit Handling
//!
//! When a VM executes certain sensitive operations or encounters exceptions,
//! it exits to the hypervisor. Common exit reasons include:
//! - I/O instructions (IN, OUT)
//! - MMIO access
//! - System calls
//! - Exceptions and interrupts
//! - EPT violations
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::hypervisor::{Hypervisor, VmConfig};
//!
//! let mut hypervisor = Hypervisor::new();
//! let config = VmConfig::default();
//! let vm = hypervisor.create_vm(&config)?;
//! vm.run();
//! ```

use alloc::vec::Vec;
use core::sync::atomic {AtomicU32, AtomicU64, Ordering, Ordering};

use super::vcpu_sched::VcpuState;
use crate::mm::virt_mem::{EptContext, EptViolation, VirtMemError};

/// Error types for hypervisor operations
#[derive(Debug, Clone, Copy)]
pub enum HypervisorError {
    /// VM creation failed
    VmCreationFailed,
    /// vCPU creation failed
    VcpuCreationFailed,
    /// Invalid VM state
    InvalidVmState,
    /// Invalid vCPU state
    InvalidVcpuState,
    /// Resource allocation failed
    ResourceAllocationFailed,
    /// VM exit handling failed
    ExitHandlingFailed,
    /// Memory setup failed
    MemorySetupFailed,
    /// Invalid configuration
    InvalidConfig,
}

/// VM states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// VM is not running
    Stopped,
    /// VM is running
    Running,
    /// VM is paused
    Paused,
    /// VM is being destroyed
    Destroying,
    /// VM encountered an error
    Error,
}

/// VM configuration
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Number of vCPUs
    pub num_vcpus: u32,
    /// Memory size in bytes
    pub memory_size: u64,
    /// CPU ID to expose to guest
    pub cpu_id: u32,
    /// Enable I/O virtualization
    pub enable_io_virt: bool,
    /// Enable APIC virtualization
    pub enable_apic_virt: bool,
    /// Enable EPT/NPT
    pub_enable_ept: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        VmConfig {
            num_vcpus: 1,
            memory_size: 512 * 1024 * 1024, // 512 MB
            cpu_id: 0,
            enable_io_virt: true,
            enable_apic_virt: true,
            pub_enable_ept: true,
        }
    }
}

/// VM exit reasons
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum VmExitReason {
    /// Exception or non-maskable interrupt (NMI)
    ExceptionNmi = 0,
    /// External interrupt
    ExternalInterrupt = 1,
    /// Triple fault (not used)
    TripleFault = 2,
    /// INIT signal
    InitSignal = 3,
    /// Startup IPI (SIPI)
    StartupIpi = 4,
    /// IO SMI
    IoSmi = 5,
    /// Other SMI
    OtherSmi = 6,
    /// Interrupt window
    InterruptWindow = 7,
    /// NMI window
    NmiWindow = 8,
    /// Task switch
    TaskSwitch = 9,
    /// CPUID
    Cpuid = 10,
    /// GETSEC
    Getsec = 11,
    /// HLT
    Hlt = 12,
    /// INVD
    Invd = 13,
    /// INVLPG
    Invlpg = 14,
    /// RDPMC
    Rdpmc = 15,
    /// RDTSC
    Rdtsc = 16,
    /// RSM
    Rsm = 17,
    /// VMCALL
    Vmcall = 18,
    /// VMCLEAR
    Vmclear = 19,
    /// VMLAUNCH
    Vmlaunch = 20,
    /// VMPTRLD
    Vmptrld = 21,
    /// VMPTRST
    Vmptrst = 22,
    /// VMREAD
    Vmread = 23,
    /// VMRESUME
    Vmresume = 24,
    /// VMWRITE
    Vmwrite = 25,
    /// VMXOFF
    Vmxoff = 26,
    /// VMXON
    Vmxon = 27,
    /// CR access
    CrAccess = 28,
    /// DR access
    DrAccess = 29,
    /// IO instruction
    IoInstruction = 30,
    /// RDMSR
    Rdmsr = 31,
    /// WRMSR
    Wrmsr = 32,
    /// Entry to guest failed
    EntryFailure = 33,
    /// VM entry for MWAIT
    VmEntryMwait = 36,
    /// MTF (Monitor Trap Flag)
    Mtf = 37,
    /// VM exit for MWAIT
    VmExitMwait = 39,
    /// TPM
    Tpm = 40,
    /// VM exit for access to GDTR or IDTR
    GdtrIdtr = 41,
    /// VM exit for access to LDTR or TR
    LdtrTr = 42,
    /// EPT violation
    EptViolation = 48,
    /// EPT misconfiguration
    EptMisconfig = 49,
    /// INVEPT
    Invept = 50,
    /// RDTSCP
    Rdtscp = 51,
    /// VMX preemption timer expired
    VmPreemptionTimer = 52,
    /// INVVPID
    Invvpid = 53,
    /// WBINVD
    Wbinvd = 54,
    /// XSETBV
    Xsetbv = 55,
}

/// VM exit information
#[derive(Debug)]
pub struct VmExitInfo {
    /// Exit reason
    pub reason: VmExitReason,
    /// Qualification (extra information about the exit)
    pub qualification: u64,
    /// Instruction length
    pub instruction_length: u64,
    /// Guest physical address (for EPT exits)
    pub guest_physical_address: u64,
    /// Guest linear address
    pub guest_linear_address: u64,
    /// Guest RIP
    pub guest_rip: u64,
}

/// Virtual Machine
pub struct VirtualMachine {
    /// VM ID
    id: u32,
    /// VM state
    state: VmState,
    /// vCPUs
    vcpus: Vec<Option<VirtualCpu>>,
    /// EPT context
    ept_context: Option<EptContext>,
    /// Guest physical memory base
    guest_memory_base: u64,
    /// Guest memory size
    guest_memory_size: u64,
    /// Configuration
    config: VmConfig,
}

impl VirtualMachine {
    /// Create a new VM
    ///
    /// # Arguments
    ///
    /// * `id` - VM identifier
    /// * `config` - VM configuration
    pub fn new(id: u32, config: &VmConfig) -> Result<Self, HypervisorError> {
        // Create EPT context if enabled
        let ept_context = if config.pub_enable_ept {
            Some(EptContext::new().map_err(|_| HypervisorError::MemorySetupFailed)?)
        } else {
            None
        };

        // Allocate guest physical memory
        let guest_memory_base = Self::allocate_guest_memory(config.memory_size)
            .map_err(|_| HypervisorError::MemorySetupFailed)?;
        let guest_memory_size = config.memory_size;

        let mut vm = VirtualMachine {
            id,
            state: VmState::Stopped,
            vcpus: Vec::new(),
            ept_context,
            guest_memory_base,
            guest_memory_size,
            config: config.clone(),
        };

        // Create vCPUs
        for i in 0..config.num_vcpus {
            let vcpu = VirtualCpu::new(i, &vm.config).map_err(|_| HypervisorError::VcpuCreationFailed)?;
            vm.vcpus.push(Some(vcpu));
        }

        Ok(vm)
    }

    /// Run the VM
    pub fn run(&mut self) -> Result<(), HypervisorError> {
        if self.state != VmState::Stopped && self.state != VmState::Paused {
            return Err(HypervisorError::InvalidVmState);
        }

        self.state = VmState::Running;

        // Run all vCPUs
        for vcpu in &mut self.vcpus {
            if let Some(v) = vcpu {
                v.run()?;
            }
        }

        Ok(())
    }

    /// Pause the VM
    pub fn pause(&mut self) -> Result<(), HypervisorError> {
        if self.state != VmState::Running {
            return Err(HypervisorError::InvalidVmState);
        }

        self.state = VmState::Paused;

        // Pause all vCPUs
        for vcpu in &mut self.vcpus {
            if let Some(v) = vcpu {
                v.pause()?;
            }
        }

        Ok(())
    }

    /// Stop the VM
    pub fn stop(&mut self) -> Result<(), HypervisorError> {
        if self.state != VmState::Running && self.state != VmState::Paused {
            return Err(HypervisorError::InvalidVmState);
        }

        self.state = VmState::Stopped;

        // Stop all vCPUs
        for vcpu in &mut self.vcpus {
            if let Some(v) = vcpu {
                v.stop()?;
            }
        }

        Ok(())
    }

    /// Get vCPU by index
    pub fn get_vcpu(&mut self, index: usize) -> Option<&mut VirtualCpu> {
        self.vcpus.get_mut(index).and_then(|v| v.as_mut())
    }

    /// Get EPT context
    pub fn ept_context(&mut self) -> Option<&mut EptContext> {
        self.ept_context.as_mut()
    }

    /// Get guest physical memory base
    pub fn guest_memory_base(&self) -> u64 {
        self.guest_memory_base
    }

    /// Handle VM exit
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU that caused the exit
    /// * `exit_info` - Exit information
    pub fn handle_vm_exit(
        &mut self,
        vcpu_id: usize,
        exit_info: &VmExitInfo,
    ) -> Result<(), HypervisorError> {
        match exit_info.reason {
            VmExitReason::IoInstruction => {
                self.handle_io_exit(vcpu_id, exit_info)?;
            }
            VmExitReason::EptViolation => {
                self.handle_ept_violation(vcpu_id, exit_info)?;
            }
            VmExitReason::Cpuid => {
                self.handle_cpuid(vcpu_id, exit_info)?;
            }
            VmExitReason::Hlt => {
                self.handle_hlt(vcpu_id)?;
            }
            VmExitReason::CrAccess => {
                self.handle_cr_access(vcpu_id, exit_info)?;
            }
            VmExitReason::Vmcall => {
                self.handle_vmcall(vcpu_id, exit_info)?;
            }
            _ => {
                log::warn!("Unhandled VM exit: {:?}", exit_info.reason);
            }
        }

        Ok(())
    }

    // Exit handlers

    fn handle_io_exit(
        &mut self,
        vcpu_id: usize,
        exit_info: &VmExitInfo,
    ) -> Result<(), HypervisorError> {
        // Parse I/O instruction from qualification
        // TODO: Implement I/O emulation
        log::debug!("I/O exit on vCPU {}", vcpu_id);
        Ok(())
    }

    fn handle_ept_violation(
        &mut self,
        vcpu_id: usize,
        exit_info: &VmExitInfo,
    ) -> Result<(), HypervisorError> {
        log::debug!(
            "EPT violation on vCPU {}: GPA={:#x}",
            vcpu_id,
            exit_info.guest_physical_address
        );

        // TODO: Handle EPT violations (e.g., map memory on demand)
        Ok(())
    }

    fn handle_cpuid(
        &mut self,
        vcpu_id: usize,
        exit_info: &VmExitInfo,
    ) -> Result<(), HypervisorError> {
        // TODO: Emulate CPUID
        log::debug!("CPUID on vCPU {}", vcpu_id);
        Ok(())
    }

    fn handle_hlt(&mut self, vcpu_id: usize) -> Result<(), HypervisorError> {
        // HLT usually means the CPU is idle
        // TODO: Implement idle handling
        log::debug!("HLT on vCPU {}", vcpu_id);
        Ok(())
    }

    fn handle_cr_access(
        &mut self,
        vcpu_id: usize,
        exit_info: &VmExitInfo,
    ) -> Result<(), HypervisorError> {
        // TODO: Handle CR access
        log::debug!("CR access on vCPU {}", vcpu_id);
        Ok(())
    }

    fn handle_vmcall(
        &mut self,
        vcpu_id: usize,
        exit_info: &VmExitInfo,
    ) -> Result<(), HypervisorError> {
        // VMCALL is a hypercall interface
        // TODO: Implement hypercall handling
        log::debug!("VMCALL on vCPU {}", vcpu_id);
        Ok(())
    }

    fn allocate_guest_memory(size: u64) -> Result<u64, VirtMemError> {
        // TODO: Allocate from physical memory manager
        static NEXT_GUEST_MEM: AtomicU64 = AtomicU64::new(0x40000000);
        Ok(NEXT_GUEST_MEM.fetch_add(size, Ordering::SeqCst))
    }
}

impl Drop for VirtualMachine {
    fn drop(&mut self) {
        self.state = VmState::Destroying;

        // Clean up vCPUs
        self.vcpus.clear();

        // TODO: Free guest memory
    }
}

/// Virtual CPU
pub struct VirtualCpu {
    /// vCPU ID
    id: u32,
    /// vCPU state
    state: VcpuState,
    /// VMCS (Virtual Machine Control Structure) address
    vmcs_addr: u64,
    /// Host state saved on VM exit
    host_state: VcpuHostState,
    /// Enable
    enabled: bool,
}

/// Host state saved on VM exit
#[derive(Debug, Clone, Copy)]
pub struct VcpuHostState {
    /// Host RSP
    pub host_rsp: u64,
    /// Host RIP
    pub host_rip: u64,
    /// Host CR0
    pub host_cr0: u64,
    /// Host CR3
    pub host_cr3: u64,
    /// Host CR4
    pub host_cr4: u64,
}

impl VirtualCpu {
    /// Create a new vCPU
    ///
    /// # Arguments
    ///
    /// * `id` - vCPU identifier
    /// * `config` - VM configuration
    pub fn new(id: u32, config: &VmConfig) -> Result<Self, HypervisorError> {
        // Allocate VMCS
        let vmcs_addr = Self::allocate_vmcs().map_err(|_| HypervisorError::ResourceAllocationFailed)?;

        Ok(VirtualCpu {
            id,
            state: VcpuState::Stopped,
            vmcs_addr,
            host_state: VcpuHostState {
                host_rsp: 0,
                host_rip: 0,
                host_cr0: 0,
                host_cr3: 0,
                host_cr4: 0,
            },
            enabled: false,
        })
    }

    /// Run the vCPU
    pub fn run(&mut self) -> Result<(), HypervisorError> {
        if self.state != VcpuState::Stopped && self.state != VcpuState::Blocked {
            return Err(HypervisorError::InvalidVcpuState);
        }

        self.state = VcpuState::Running;
        self.enabled = true;

        // TODO: Execute VM entry
        // This requires assembly code to execute VMLAUNCH/VMRESUME

        Ok(())
    }

    /// Pause the vCPU
    pub fn pause(&mut self) -> Result<(), HypervisorError> {
        if self.state != VcpuState::Running {
            return Err(HypervisorError::InvalidVcpuState);
        }

        self.state = VcpuState::Blocked;
        self.enabled = false;

        Ok(())
    }

    /// Stop the vCPU
    pub fn stop(&mut self) -> Result<(), HypervisorError> {
        self.state = VcpuState::Stopped;
        self.enabled = false;

        Ok(())
    }

    /// Get vCPU ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get vCPU state
    pub fn state(&self) -> VcpuState {
        self.state
    }

    fn allocate_vmcs() -> Result<u64, VirtMemError> {
        // VMCS must be 4KB aligned
        // TODO: Allocate from physical memory manager
        static NEXT_VMCS: AtomicU64 = AtomicU64::new(0x20000000);
        let addr = NEXT_VMCS.fetch_add(0x1000, Ordering::SeqCst);
        assert!(addr & 0xFFF == 0, "VMCS must be 4KB aligned");
        Ok(addr)
    }
}

/// Hypervisor manager
pub struct Hypervisor {
    /// List of VMs
    vms: Vec<Option<VirtualMachine>>,
    /// Next VM ID
    next_vm_id: AtomicU32,
    /// Enabled flag
    enabled: bool,
}

impl Hypervisor {
    /// Create a new hypervisor manager
    pub fn new() -> Self {
        Hypervisor {
            vms: Vec::new(),
            next_vm_id: AtomicU32::new(0),
            enabled: false,
        }
    }

    /// Initialize the hypervisor
    ///
    /// This must be called before creating any VMs
    pub fn init(&mut self) -> Result<(), HypervisorError> {
        // TODO: Enable hardware virtualization (VMX or SVM)
        // This involves:
        // - Enabling VMX in CR4
        // - Executing VMXON
        // - Setting up VMX region

        self.enabled = true;
        Ok(())
    }

    /// Create a new VM
    ///
    /// # Arguments
    ///
    /// * `config` - VM configuration
    ///
    /// # Returns
    ///
    /// VM ID
    pub fn create_vm(&mut self, config: &VmConfig) -> Result<u32, HypervisorError> {
        if !self.enabled {
            return Err(HypervisorError::InvalidVmState);
        }

        let vm_id = self.next_vm_id.fetch_add(1, Ordering::SeqCst);
        let vm = VirtualMachine::new(vm_id, config)?;

        // Find empty slot
        let slot = self.vms.iter().position(|v| v.is_none()).unwrap_or(self.vms.len());

        if slot == self.vms.len() {
            self.vms.push(Some(vm));
        } else {
            self.vms[slot] = Some(vm);
        }

        Ok(vm_id)
    }

    /// Destroy a VM
    ///
    /// # Arguments
    ///
    /// * `vm_id` - VM identifier
    pub fn destroy_vm(&mut self, vm_id: u32) -> Result<(), HypervisorError> {
        let slot = self
            .vms
            .iter()
            .position(|v| v.as_ref().map(|vm| vm.id) == Some(vm_id))
            .ok_or(HypervisorError::InvalidVmState)?;

        self.vms[slot] = None;
        Ok(())
    }

    /// Get VM by ID
    pub fn get_vm(&mut self, vm_id: u32) -> Option<&mut VirtualMachine> {
        self.vms
            .iter_mut()
            .find_map(|v| v.as_mut().filter(|vm| vm.id == vm_id))
    }

    /// Check if hypervisor is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for Hypervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VmConfig::default();
        assert_eq!(config.num_vcpus, 1);
        assert_eq!(config.memory_size, 512 * 1024 * 1024);
        assert!(config.enable_io_virt);
        assert!(config.enable_apic_virt);
    }

    #[test]
    fn test_hypervisor_creation() {
        let hypervisor = Hypervisor::new();
        assert!(!hypervisor.is_enabled());
        assert_eq!(hypervisor.next_vm_id.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_vm_state_transitions() {
        // Test state transition logic
        let mut state = VmState::Stopped;
        assert_eq!(state, VmState::Stopped);

        state = VmState::Running;
        assert_eq!(state, VmState::Running);

        state = VmState::Paused;
        assert_eq!(state, VmState::Paused);
    }
}
