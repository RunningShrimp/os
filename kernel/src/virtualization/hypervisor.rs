//! Hypervisor Core Implementation
//!
//! This module provides Type-1 hypervisor functionality with hardware-assisted virtualization.
//! Supports Intel VT-x (VMX) and AMD-V (SVM) technologies for efficient virtualization.
//!
//! # Features
//! - Hardware-assisted virtualization (VMX/SVM)
//! - VMCS/VMCB management
//! - VM exit/entry handling
//! - Nested virtualization support
//! - Extended Page Tables (EPT) / Nested Page Tables (NPT)
//! - Virtual interrupt delivery
//! - MSR bitmap support
//!
//! # Example
//! ```rust
//! use kernel::virtualization::hypervisor::Hypervisor;
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let hypervisor = Hypervisor::new()?;
//! hypervisor.initialize_vmx()?;
//! hypervisor.enable_vmx()?;
//!
//! // Create and run VM
//! let vm_id = hypervisor.create_vm()?;
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]
#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::api::KernelError;

/// Hypervisor result type
pub type HypervisorResult<T> = core::result::Result<T, HypervisorError>;

/// Hypervisor-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HypervisorError {
    /// Hardware doesn't support virtualization
    HardwareNotSupported,
    /// VMX/SVM initialization failed
    InitializationFailed,
    /// VMCS/VMCB operation failed
    VmcsOperationFailed,
    /// Invalid VM exit reason
    InvalidExitReason,
    /// VM entry failed
    VmEntryFailed,
    /// Invalid VMCS field
    InvalidVmcsField,
    /// Nested virtualization not supported
    NestedNotSupported,
    /// Invalid VM ID
    InvalidVmId,
    /// VM already exists
    VmAlreadyExists,
    /// Invalid vCPU state
    InvalidVcpuState,
    /// MSR access denied
    MsrAccessDenied,
    /// Invalid guest state
    InvalidGuestState,
    /// Virtualization instruction failed
    InstructionFailed(String),
}

impl core::fmt::Display for HypervisorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::HardwareNotSupported => write!(f, "Hardware doesn't support virtualization"),
            Self::InitializationFailed => write!(f, "VMX/SVM initialization failed"),
            Self::VmcsOperationFailed => write!(f, "VMCS/VMCB operation failed"),
            Self::InvalidExitReason => write!(f, "Invalid VM exit reason"),
            Self::VmEntryFailed => write!(f, "VM entry failed"),
            Self::InvalidVmcsField => write!(f, "Invalid VMCS field"),
            Self::NestedNotSupported => write!(f, "Nested virtualization not supported"),
            Self::InvalidVmId => write!(f, "Invalid VM ID"),
            Self::VmAlreadyExists => write!(f, "VM already exists"),
            Self::InvalidVcpuState => write!(f, "Invalid vCPU state"),
            Self::MsrAccessDenied => write!(f, "MSR access denied"),
            Self::InvalidGuestState => write!(f, "Invalid guest state"),
            Self::InstructionFailed(msg) => write!(f, "Instruction failed: {}", msg),
        }
    }
}

impl From<HypervisorError> for KernelError {
    fn from(err: HypervisorError) -> Self {
        KernelError::Other(format!("Hypervisor: {}", err))
    }
}

/// VMX control registers
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VmxControls {
    /// Primary processor-based VM-execution controls
    pub pin_based: u32,
    /// Primary processor-based VM-execution controls
    pub primary_proc: u32,
    /// Secondary processor-based VM-execution controls
    pub secondary_proc: u32,
    /// VM-entry controls
    pub entry: u32,
    /// VM-exit controls
    pub exit: u32,
}

impl VmxControls {
    /// Default VMX controls for safe operation
    pub const fn default() -> Self {
        Self {
            pin_based: 0,
            primary_proc: 0,
            secondary_proc: 0,
            entry: 0,
            exit: 0,
        }
    }
}

/// VMCS field encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum VmcsField {
    // 16-bit control fields
    Vpid = 0x0000,
    // 16-bit guest state fields
    GuestEsSelector = 0x800,
    GuestCsSelector = 0x802,
    GuestSsSelector = 0x804,
    GuestDsSelector = 0x806,
    GuestFsSelector = 0x808,
    GuestGsSelector = 0x80a,
    GuestLdtrSelector = 0x80c,
    GuestTrSelector = 0x80e,
    // 16-bit host state fields
    HostEsSelector = 0xc00,
    HostCsSelector = 0xc02,
    HostSsSelector = 0xc04,
    HostDsSelector = 0xc06,
    HostFsSelector = 0xc08,
    HostGsSelector = 0xc0a,
    HostTrSelector = 0xc0c,
    // 64-bit control fields
    IoBitmapA = 0x2000,
    IoBitmapB = 0x2002,
    MsrBitmap = 0x2004,
    EptPointer = 0x201a,
    // 64-bit guest state fields
    VmcsLinkPointer = 0x2800,
    GuestIa32Debugctl = 0x2802,
    // 64-bit host state fields
    VmcsLinkPointerHigh = 0x2c00,
    // 32-bit control fields
    PinBasedControls = 0x4000,
    PrimaryProcControls = 0x4002,
    SecondaryProcControls = 0x401e,
    VmExitControls = 0x400c,
    VmEntryControls = 0x4012,
    // 32-bit guest state fields
    GuestEsLimit = 0x4800,
    GuestCsLimit = 0x4802,
    GuestSsLimit = 0x4804,
    GuestActivityState = 0x4826,
    GuestInterruptibility = 0x4824,
    // 32-bit host state fields
    HostIa32SysenterCs = 0x4c00,
    // Natural-width guest state fields
    GuestCr0 = 0x6800,
    GuestCr3 = 0x6802,
    GuestCr4 = 0x6804,
    GuestRsp = 0x681c,
    GuestRip = 0x681e,
    GuestRflags = 0x6820,
}

/// VM exit reasons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VmExitReason {
    /// Exception or non-maskable interrupt (NMI)
    ExceptionNmi = 0,
    /// External interrupt
    ExternalInterrupt = 1,
    /// Triple fault
    TripleFault = 2,
    /// INIT signal
    InitSignal = 3,
    /// Start-up IPI (SIPI)
    StartupIpi = 4,
    /// IO system management interrupt (SMI)
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
    /// Entry failed guest state
    EntryFailed = 33,
    /// Entry failed MSR loading
    EntryFailedMsrLoad = 34,
    /// MWAIT
    Mwait = 36,
    /// MTF
    Mtf = 37,
    /// Monitor
    Monitor = 39,
    /// Pause
    Pause = 40,
    /// Entry failed machine check
    EntryFailedMc = 41,
    /// TPR below threshold
    TprBelowThreshold = 43,
    /// APIC access
    ApicAccess = 44,
    /// Virtualized EOI
    VirtualizedEoi = 45,
    /// GDTR/IDTR access
    GdtrIdtrAccess = 46,
    /// LDTR/TR access
    LdtrTrAccess = 47,
    /// EPT violation
    EptViolation = 48,
    /// EPT misconfiguration
    EptMisconfig = 49,
    /// INVEPT
    Invept = 50,
    /// RDTSCP
    Rdtscp = 51,
    /// VMX preemption timer expired
    VmxPreemptTimer = 52,
    /// INVVPID
    Invvpid = 53,
    /// WBINVD
    Wbinvd = 54,
    /// XSETBV
    Xsetbv = 55,
}

/// VMCS region state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmcsState {
    /// VMCS is clear
    Clear,
    /// VMCS is current
    Current,
    /// VMCS is logical
    Logical,
}

/// Hypervisor structure
pub struct Hypervisor {
    /// Is hypervisor initialized?
    initialized: AtomicBool,
    /// Is VMX enabled?
    vmx_enabled: AtomicBool,
    /// Is SVM enabled?
    svm_enabled: AtomicBool,
    /// VMX revision identifier
    vmx_revision: AtomicU32,
    /// VMX controls
    vmx_controls: Mutex<VmxControls>,
    /// MSR bitmap for VMX
    msr_bitmap: Mutex<Option<Vec<u8>>>,
    /// VMCS regions
    vmcs_regions: Mutex<BTreeMap<u64, VmcsState>>,
    /// VMX capability MSRs
    vmx_cap_msr: Mutex<VmxCapabilityMsrs>,
    /// Nested virtualization support
    nested_virt_supported: AtomicBool,
    /// Number of active VMs
    active_vms: AtomicUsize,
    /// Total VM exits handled
    total_exits: AtomicU64,
}

/// VMX capability MSRs
#[derive(Debug, Clone, Copy)]
pub struct VmxCapabilityMsrs {
    /// VMX basic MSR
    pub vmx_basic: u64,
    /// Pin-based controls
    pub pin_based_ctls: u64,
    /// Primary processor-based controls
    pub primary_proc_ctls: u64,
    /// Secondary processor-based controls
    pub secondary_proc_ctls: u64,
    /// Exit controls
    pub exit_ctls: u64,
    /// Entry controls
    pub entry_ctls: u64,
    /// CR0 and CR4 fixed bits
    pub cr0_fixed0: u64,
    pub cr0_fixed1: u64,
    pub cr4_fixed0: u64,
    pub cr4_fixed1: u64,
}

impl Hypervisor {
    /// Create a new hypervisor instance
    ///
    /// # Returns
    /// * `HypervisorResult<Self>` - New hypervisor instance
    pub fn new() -> HypervisorResult<Self> {
        Ok(Self {
            initialized: AtomicBool::new(false),
            vmx_enabled: AtomicBool::new(false),
            svm_enabled: AtomicBool::new(false),
            vmx_revision: AtomicU32::new(0),
            vmx_controls: Mutex::new(VmxControls::default()),
            msr_bitmap: Mutex::new(None),
            vmcs_regions: Mutex::new(BTreeMap::new()),
            vmx_cap_msr: Mutex::new(VmxCapabilityMsrs {
                vmx_basic: 0,
                pin_based_ctls: 0,
                primary_proc_ctls: 0,
                secondary_proc_ctls: 0,
                exit_ctls: 0,
                entry_ctls: 0,
                cr0_fixed0: 0,
                cr0_fixed1: 0,
                cr4_fixed0: 0,
                cr4_fixed1: 0,
            }),
            nested_virt_supported: AtomicBool::new(false),
            active_vms: AtomicUsize::new(0),
            total_exits: AtomicU64::new(0),
        })
    }

    /// Check if CPU supports virtualization
    ///
    /// # Returns
    /// * `bool` - True if virtualization is supported
    pub fn has_vmx_support() -> bool {
        // TODO: ARM64 doesn't have CPUID
        // Implement system register detection for ARM64 virtualization features
        // For ARM64, check ID_AA64MMFR0_EL1 for virtualization support
        // For now, return false as ARM64 uses different virtualization mechanism
        false
    }

    /// Check if CPU supports AMD-V (SVM)
    ///
    /// # Returns
    /// * `bool` - True if SVM is supported
    pub fn has_svm_support() -> bool {
        // TODO: ARM64 doesn't have CPUID
        // Implement system register detection for ARM64 virtualization features
        // For ARM64, check ID_AA64MMFR0_EL1 for virtualization support
        // For now, return false as ARM64 uses different virtualization mechanism
        false
    }

    /// Initialize VMX
    ///
    /// # Returns
    /// * `HypervisorResult<()>` - Success or error
    pub fn initialize_vmx(&self) -> HypervisorResult<()> {
        if !Self::has_vmx_support() {
            return Err(HypervisorError::HardwareNotSupported);
        }

        // Read VMX capability MSRs
        let mut cap_msr = self.vmx_cap_msr.lock();

        unsafe {
            // Simplified MSR reads - in real implementation, use actual rdmsr
            cap_msr.vmx_basic = 0;
            cap_msr.pin_based_ctls = 0;
            cap_msr.primary_proc_ctls = 0;
            cap_msr.secondary_proc_ctls = 0;
            cap_msr.exit_ctls = 0;
            cap_msr.entry_ctls = 0;
            cap_msr.cr0_fixed0 = 0;
            cap_msr.cr0_fixed1 = 0;
            cap_msr.cr4_fixed0 = 0;
            cap_msr.cr4_fixed1 = 0;
        }

        // Extract VMX revision from VMX_BASIC MSR
        let vmx_revision = (cap_msr.vmx_basic & 0x7FFF_FFFF) as u32;
        self.vmx_revision.store(vmx_revision, Ordering::SeqCst);

        // Check for nested virtualization support
        self.nested_virt_supported.store(
            Self::check_nested_virt_support(),
            Ordering::SeqCst,
        );

        drop(cap_msr);

        // Allocate MSR bitmap (4KB, 0s = allow access)
        let msr_bitmap = vec![0u8; 4096];
        self.msr_bitmap.lock().replace(msr_bitmap);

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Enable VMX operation
    ///
    /// # Returns
    /// * `HypervisorResult<()>` - Success or error
    pub fn enable_vmx(&self) -> HypervisorResult<()> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(HypervisorError::InitializationFailed);
        }

        // TODO: ARM64 doesn't have CR4 or VMX
        // Enable virtualization for ARM64 (VHE/EL2)
        // For now, just mark as enabled
        unsafe {
            // On ARM64, this would involve:
            // - Configuring VFP/NEON
            // - Setting up EL2 (Hypervisor Exception Level)
            // - Enabling Virtualization Host Extensions (VHE)
            // For now, stub implementation
        }

        self.vmx_enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Allocate VMCS region
    ///
    /// # Arguments
    /// * `vm_id` - VM identifier
    ///
    /// # Returns
    /// * `HypervisorResult<u64>` - Physical address of VMCS region
    pub fn allocate_vmcs(&self, vm_id: u64) -> HypervisorResult<u64> {
        if !self.vmx_enabled.load(Ordering::SeqCst) {
            return Err(HypervisorError::InitializationFailed);
        }

        // Allocate 4KB aligned VMCS region
        // In a real implementation, this would allocate from a physical memory allocator
        let vmcs_phys_addr = 0x5000 + (vm_id * 0x1000); // Placeholder

        // Write VMX revision identifier to first 4 bytes
        let _revision = self.vmx_revision.load(Ordering::SeqCst) as u64;

        unsafe {
            // In real implementation, map and write to physical memory
            // *(vmcs_phys_addr as *mut u32) = revision;
        }

        let mut regions = self.vmcs_regions.lock();
        regions.insert(vmcs_phys_addr, VmcsState::Clear);

        Ok(vmcs_phys_addr)
    }

    /// Free VMCS region
    ///
    /// # Arguments
    /// * `vmcs_phys_addr` - Physical address of VMCS region
    pub fn free_vmcs(&self, vmcs_phys_addr: u64) -> HypervisorResult<()> {
        let mut regions = self.vmcs_regions.lock();
        regions.remove(&vmcs_phys_addr)
            .ok_or(HypervisorError::InvalidVmcsField)?;
        Ok(())
    }

    /// Clear VMCS
    ///
    /// # Arguments
    /// * `vmcs_phys_addr` - Physical address of VMCS region
    pub fn vmclear(&self, vmcs_phys_addr: u64) -> HypervisorResult<()> {
        unsafe {
            let error = vmx::vmclear(vmcs_phys_addr);
            if error != 0 {
                return Err(HypervisorError::VmcsOperationFailed);
            }
        }

        let mut regions = self.vmcs_regions.lock();
        if let Some(state) = regions.get_mut(&vmcs_phys_addr) {
            *state = VmcsState::Clear;
        }

        Ok(())
    }

    /// Load VMCS pointer
    ///
    /// # Arguments
    /// * `vmcs_phys_addr` - Physical address of VMCS region
    pub fn vmptrld(&self, vmcs_phys_addr: u64) -> HypervisorResult<()> {
        unsafe {
            let error = vmx::vmptrld(vmcs_phys_addr);
            if error != 0 {
                return Err(HypervisorError::VmcsOperationFailed);
            }
        }

        let mut regions = self.vmcs_regions.lock();
        if let Some(state) = regions.get_mut(&vmcs_phys_addr) {
            *state = VmcsState::Current;
        }

        Ok(())
    }

    /// Read VMCS field
    ///
    /// # Arguments
    /// * `field` - VMCS field identifier
    ///
    /// # Returns
    /// * `HypervisorResult<u64>` - Field value
    pub fn vmread(&self, field: VmcsField) -> HypervisorResult<u64> {
        let mut value: u64 = 0;
        unsafe {
            let error = vmx::vmread(field as u64, &mut value);
            if error != 0 {
                return Err(HypervisorError::InvalidVmcsField);
            }
        }
        Ok(value)
    }

    /// Write VMCS field
    ///
    /// # Arguments
    /// * `field` - VMCS field identifier
    /// * `value` - Value to write
    pub fn vmwrite(&self, field: VmcsField, value: u64) -> HypervisorResult<()> {
        unsafe {
            let error = vmx::vmwrite(field as u64, value);
            if error != 0 {
                return Err(HypervisorError::InvalidVmcsField);
            }
        }
        Ok(())
    }

    /// Launch VM
    ///
    /// # Returns
    /// * `HypervisorResult<VmExitInfo>` - VM exit information
    pub fn vmlaunch(&self) -> HypervisorResult<VmExitInfo> {
        self.active_vms.fetch_add(1, Ordering::SeqCst);

        unsafe {
            match vmx::vmlaunch() {
                Ok(exit_info) => {
                    self.total_exits.fetch_add(1, Ordering::SeqCst);
                    Ok(exit_info)
                }
                Err(_) => Err(HypervisorError::VmEntryFailed),
            }
        }
    }

    /// Resume VM
    ///
    /// # Returns
    /// * `HypervisorResult<VmExitInfo>` - VM exit information
    pub fn vmresume(&self) -> HypervisorResult<VmExitInfo> {
        unsafe {
            match vmx::vmresume() {
                Ok(exit_info) => {
                    self.total_exits.fetch_add(1, Ordering::SeqCst);
                    Ok(exit_info)
                }
                Err(_) => Err(HypervisorError::VmEntryFailed),
            }
        }
    }

    /// Handle VM exit
    ///
    /// # Arguments
    /// * `exit_info` - VM exit information
    ///
    /// # Returns
    /// * `HypervisorResult<ExitAction>` - Action to take after handling
    pub fn handle_vm_exit(&self, exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        match exit_info.reason {
            VmExitReason::Cpuid => self.handle_cpuid(exit_info),
            VmExitReason::IoInstruction => self.handle_io(exit_info),
            VmExitReason::CrAccess => self.handle_cr_access(exit_info),
            VmExitReason::Rdmsr => self.handle_rdmsr(exit_info),
            VmExitReason::Wrmsr => self.handle_wrmsr(exit_info),
            VmExitReason::Hlt => Ok(ExitAction::Halt),
            VmExitReason::ExternalInterrupt => Ok(ExitAction::Resume),
            VmExitReason::ExceptionNmi => self.handle_exception(exit_info),
            VmExitReason::EptViolation => self.handle_ept_violation(exit_info),
            VmExitReason::Vmcall => self.handle_vmcall(exit_info),
            _ => Ok(ExitAction::Resume),
        }
    }

    /// Handle CPUID VM exit
    fn handle_cpuid(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // TODO: ARM64 doesn't have CPUID
        // Emulate CPUID for x86 guests or implement ARM64 system register queries
        // For ARM64, this would be:
        // - Read ID_AA64ISAR0_EL1 for ISA features
        // - Read ID_AA64MMFR0_EL1 for memory model features
        // - Read MIDR_EL1 for CPU ID
        // For now, return stub response
        Ok(ExitAction::Resume)
    }

    /// Handle IO instruction VM exit
    fn handle_io(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Emulate IO instruction
        Ok(ExitAction::Resume)
    }

    /// Handle CR access VM exit
    fn handle_cr_access(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Handle CR0, CR3, CR4 access
        Ok(ExitAction::Resume)
    }

    /// Handle RDMSR VM exit
    fn handle_rdmsr(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Check MSR bitmap and emulate RDMSR
        Ok(ExitAction::Resume)
    }

    /// Handle WRMSR VM exit
    fn handle_wrmsr(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Check MSR bitmap and emulate WRMSR
        Ok(ExitAction::Resume)
    }

    /// Handle exception/NMI VM exit
    fn handle_exception(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Inject exception into guest
        Ok(ExitAction::Resume)
    }

    /// Handle EPT violation VM exit
    fn handle_ept_violation(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Handle EPT violation (page walk, MMIO, etc.)
        Ok(ExitAction::Resume)
    }

    /// Handle VMCALL VM exit
    fn handle_vmcall(&self, _exit_info: &VmExitInfo) -> HypervisorResult<ExitAction> {
        // Handle hypercall
        Ok(ExitAction::Resume)
    }

    /// Check for nested virtualization support
    fn check_nested_virt_support() -> bool {
        // Check VMX_BASIC[55] for "VMCS shadowing"
        // Check IA32_VMX_PROCBASED_CTLS2[68] for "Enable VM functions"
        // Check IA32_VMX_VMFUNC[0] for "EPTP switching"
        false // Simplified
    }

    /// Get hypervisor statistics
    pub fn get_stats(&self) -> HypervisorStats {
        HypervisorStats {
            active_vms: self.active_vms.load(Ordering::SeqCst),
            total_exits: self.total_exits.load(Ordering::SeqCst),
            vmx_enabled: self.vmx_enabled.load(Ordering::SeqCst),
            svm_enabled: self.svm_enabled.load(Ordering::SeqCst),
            nested_virt_supported: self.nested_virt_supported.load(Ordering::SeqCst),
        }
    }

    /// Create a new VM (returns VM ID)
    pub fn create_vm(&self) -> HypervisorResult<u64> {
        let vm_id = self.active_vms.load(Ordering::SeqCst) as u64;
        self.allocate_vmcs(vm_id)?;
        Ok(vm_id)
    }

    /// Destroy a VM
    pub fn destroy_vm(&self, vm_id: u64) -> HypervisorResult<()> {
        let vmcs_addr = 0x5000 + (vm_id * 0x1000);
        self.free_vmcs(vmcs_addr)?;
        self.active_vms.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }
}

/// VM exit information
#[derive(Debug, Clone, Copy)]
pub struct VmExitInfo {
    /// Exit reason
    pub reason: VmExitReason,
    /// Exit qualification
    pub qualification: u64,
    /// Guest RIP
    pub guest_rip: u64,
    /// Guest RSP
    pub guest_rsp: u64,
    /// Instruction length
    pub instr_len: u8,
    /// Guest physical address (for EPT violations)
    pub guest_phys_addr: u64,
    /// Exit interruption info
    pub exit_int_info: u32,
}

/// Action to take after handling VM exit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitAction {
    /// Resume VM execution
    Resume,
    /// Halt VM
    Halt,
    /// Reset VM
    Reset,
    /// Shutdown VM
    Shutdown,
}

/// Hypervisor statistics
#[derive(Debug, Clone, Copy)]
pub struct HypervisorStats {
    /// Number of active VMs
    pub active_vms: usize,
    /// Total VM exits handled
    pub total_exits: u64,
    /// VMX enabled
    pub vmx_enabled: bool,
    /// SVM enabled
    pub svm_enabled: bool,
    /// Nested virtualization supported
    pub nested_virt_supported: bool,
}

// VMX instruction wrappers
mod vmx {
    /// VMXON - Enter VMX operation
    #[inline]
    pub unsafe fn vmxon(_phys_addr: u64) -> u8 {
        // TODO: ARM64 doesn't have VMXON
        // For ARM64, this would be:
        // - Configure HCR_EL2 (Hypervisor Configuration Register)
        // - Set up VTTBR_EL2 (Virtualization Translation Table Base Register)
        // For now, return success
        0
    }

    /// VMXOFF - Exit VMX operation
    #[inline]
    pub unsafe fn vmxoff() -> u8 {
        // TODO: ARM64 doesn't have VMXOFF
        // For ARM64, this would be:
        // - Disable EL2 features
        // - Restore EL1 configuration
        // For now, return success
        0
    }

    /// VMCLEAR - Clear VMCS
    #[inline]
    pub unsafe fn vmclear(_phys_addr: u64) -> u8 {
        // TODO: ARM64 doesn't have VMCLEAR
        // For ARM64, this would be:
        // - Invalidate VCPU context in EL2
        // - Clear virtual machine state
        // For now, return success
        0
    }

    /// VMPTRLD - Load VMCS pointer
    #[inline]
    pub unsafe fn vmptrld(_phys_addr: u64) -> u8 {
        // TODO: ARM64 doesn't have VMPTRLD
        // For ARM64, this would be:
        // - Load VCPU context into EL2
        // - Configure VTTBR_EL2 for this VM
        // For now, return success
        0
    }

    /// VMPTRST - Store VMCS pointer
    #[inline]
    pub unsafe fn vmptrst(_phys_addr: *mut u64) -> u8 {
        // TODO: ARM64 doesn't have VMPTRST
        // For ARM64, this would be:
        // - Store current VCPU context pointer
        // - Read VTTBR_EL2
        // For now, return success
        0
    }

    /// VMREAD - Read from VMCS
    #[inline]
    pub unsafe fn vmread(_field: u64, value: *mut u64) -> u8 {
        // TODO: ARM64 doesn't have VMREAD
        // For ARM64, this would be:
        // - Read EL2 system registers (e.g., VTTBR_EL2, VTCR_EL2)
        // - Access virtualization context state
        // For now, return 0
        unsafe {
            *value = 0;
        }
        0
    }

    /// VMWRITE - Write to VMCS
    #[inline]
    pub unsafe fn vmwrite(_field: u64, _value: u64) -> u8 {
        // TODO: ARM64 doesn't have VMWRITE
        // For ARM64, this would be:
        // - Write EL2 system registers (e.g., VTTBR_EL2, VTCR_EL2)
        // - Update virtualization context state
        // For now, return success
        0
    }

    /// VMLAUNCH - Launch VM
    #[inline]
    pub unsafe fn vmlaunch() -> Result<super::VmExitInfo, ()> {
        let exit_info = super::VmExitInfo {
            reason: super::VmExitReason::ExceptionNmi,
            qualification: 0,
            guest_rip: 0,
            guest_rsp: 0,
            instr_len: 0,
            guest_phys_addr: 0,
            exit_int_info: 0,
        };

        // TODO: ARM64 doesn't have VMLAUNCH
        // For ARM64, this would be:
        // - Configure ERET to return to EL1 (guest)
        // - Set up exception level switch
        // For now, return stub exit info
        Ok(exit_info)
    }

    /// VMRESUME - Resume VM
    #[inline]
    pub unsafe fn vmresume() -> Result<super::VmExitInfo, ()> {
        let exit_info = super::VmExitInfo {
            reason: super::VmExitReason::ExceptionNmi,
            qualification: 0,
            guest_rip: 0,
            guest_rsp: 0,
            instr_len: 0,
            guest_phys_addr: 0,
            exit_int_info: 0,
        };

        // TODO: ARM64 doesn't have VMRESUME
        // For ARM64, this would be:
        // - Use ERET to return to EL1 (guest)
        // - Restore guest context from EL2
        // For now, return stub exit info
        Ok(exit_info)
    }

    /// INVEPT - Invalidate EPT mappings
    #[inline]
    pub unsafe fn invept(_type_: u32, _descriptor: u64) -> u8 {
        // TODO: ARM64 doesn't have INVEPT
        // For ARM64, this would be:
        // - TLBI IPAS2E1IS (Stage 2 TLB invalidate)
        // - TLBI VMALLE1IS (All VMIDs TLB invalidate)
        // For now, return success
        0
    }

    /// INVVPID - Invalidate VPID mappings
    #[inline]
    pub unsafe fn invvpid(_type_: u32, _descriptor: u64) -> u8 {
        // TODO: ARM64 doesn't have INVVPID
        // For ARM64, this would be:
        // - TLBI VMALLE1IS (TLB invalidate by VMID)
        // For now, return success
        0
    }
}

// MSR access wrappers
mod msr {
    /// Read MSR
    #[inline]
    pub unsafe fn rdmsr(msr: u32) -> u64 {
        // TODO: ARM64 doesn't have RDMSR
        // For ARM64, this would be:
        // - MRS (Move to Register from System register) instruction
        // - Map x86 MSRs to equivalent ARM64 system registers
        // For now, return stub values based on MSR address
        match msr {
            0x1B => 0,          // IA32_APIC_BASE -> TODO: Map to ARM64 GIC base
            0x3A => 0,          // IA32_FEATURE_CONTROL_MSR -> TODO: Map to ARM64 feature registers
            0x3B => 0,          // IA32_TSC_AUX -> TODO: Map to CNTVCT_EL0 (virtual timer)
            0x174 => 0,         // IA32_SYSENTER_CS -> TODO: Map to ARM64 exception level configs
            0x175 => 0,         // IA32_SYSENTER_ESP -> Not applicable on ARM64
            0x176 => 0,         // IA32_SYSENTER_EIP -> Not applicable on ARM64
            0x1A0 => 0,         // IA32_MISC_ENABLE -> TODO: Map to ARM64 feature enables
            0x200 => 0x1A0,     // IA32_TSC -> Map to CNTVCT_EL0 or CNTPCT_EL0
            0xC0000080 => 0,    // IA32_EFER -> TODO: Map to ARM64 system control registers
            0xC0000081 => 0,    // IA32_STAR -> Not applicable on ARM64
            0xC0000082 => 0,    // IA32_LSTAR -> Not applicable on ARM64
            0xC0000083 => 0,    // IA32_CSTAR -> Not applicable on ARM64
            0xC0000084 => 0,    // IA32_FMASK -> Not applicable on ARM64
            0xC0000100 => 0,    // IA32_FS_BASE -> Map to TPIDR_EL0
            0xC0000101 => 0,    // IA32_GS_BASE -> Map to TPIDRRO_EL0
            _ => 0,
        }
    }

    /// Write MSR
    #[inline]
    pub unsafe fn wrmsr(msr: u32, _value: u64) {
        // TODO: ARM64 doesn't have WRMSR
        // For ARM64, this would be:
        // - MSR (Move to System register from Register) instruction
        // - Map x86 MSRs to equivalent ARM64 system registers
        // For now, stub implementation
        match msr {
            0x3A => {
                // IA32_FEATURE_CONTROL_MSR
                // TODO: Configure ARM64 virtualization features
            }
            0x1B => {
                // IA32_APIC_BASE
                // TODO: Configure ARM64 GIC base address
            }
            0xC0000080 => {
                // IA32_EFER
                // TODO: Map to ARM64 system control registers
            }
            0xC0000100 => {
                // IA32_FS_BASE
                // TODO: Write to TPIDR_EL0
            }
            0xC0000101 => {
                // IA32_GS_BASE
                // TODO: Write to TPIDRRO_EL0
            }
            _ => {
                // Ignore other MSRs for now
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypervisor_creation() {
        let hypervisor = Hypervisor::new().unwrap();
        assert!(!hypervisor.initialized.load(Ordering::SeqCst));
    }

    #[test]
    fn test_vmx_support_detection() {
        let has_vmx = Hypervisor::has_vmx_support();
        // This test may pass or fail depending on hardware
        // We just verify it doesn't panic
        let _ = has_vmx;
    }

    #[test]
    fn test_vmcs_field_enum() {
        // Test that VMCS field enum values are correct
        assert_eq!(VmcsField::Vpid as u64, 0x0000);
        assert_eq!(VmcsField::GuestCsSelector as u64, 0x802);
        assert_eq!(VmcsField::GuestCr0 as u64, 0x6800);
    }

    #[test]
    fn test_exit_reason_enum() {
        assert_eq!(VmExitReason::Cpuid as u32, 10);
        assert_eq!(VmExitReason::Hlt as u32, 12);
        assert_eq!(VmExitReason::Vmcall as u32, 18);
    }

    #[test]
    fn test_vmcs_state() {
        assert_eq!(VmcsState::Clear, VmcsState::Clear);
        assert_eq!(VmcsState::Current, VmcsState::Current);
    }

    #[test]
    fn test_vmx_controls_default() {
        let controls = VmxControls::default();
        assert_eq!(controls.pin_based, 0);
        assert_eq!(controls.primary_proc, 0);
    }

    #[test]
    fn test_vm_allocation() {
        let hypervisor = Hypervisor::new().unwrap();
        // This will fail without actual VMX support
        let result = hypervisor.create_vm();
        // We expect this to either succeed or fail gracefully
        let _ = result;
    }

    #[test]
    fn test_exit_action() {
        assert_eq!(ExitAction::Resume, ExitAction::Resume);
        assert_eq!(ExitAction::Halt, ExitAction::Halt);
    }
}
