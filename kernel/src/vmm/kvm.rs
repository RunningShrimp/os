//! # KVM (Kernel-based Virtual Machine) Integration
//!
//! This module provides integration with the Linux KVM API for hardware-assisted
//! virtualization. KVM allows the kernel to run virtual machines with near-native
//! performance by leveraging CPU virtualization extensions (Intel VT-x, AMD-V).
//!
//! ## Features
//!
//! - VM creation and lifecycle management
//! - vCPU creation and execution
//! - Memory slot management
//! - IRQ chip emulation (PIC, IOAPIC)
//! - MSR handling and virtualization
//! - VMX/SVM feature detection
//!
//! ## Architecture
//!
//! The KVM interface uses ioctl() system calls to communicate with the KVM kernel
//! module. Each VM and vCPU are represented as file descriptors, and operations
//! are performed through ioctl commands.
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::kvm::{Kvm, KvmVm, KvmVcpu};
//!
//! let kvm = Kvm::new()?;
//! let vm = kvm.create_vm(2)?;  // 2 vCPUs
//! let vcpu = vm.create_vcpu(0)?;
//! vcpu.run()?;
//! ```

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::usize;
use spin::Mutex;

use crate::error::KernelError;
use crate::memory::{PhysicalAddress, VirtualAddress};
use crate::sync::lock_guard;

// KVM API constants (from Linux kernel headers)
const KVM_GET_API_VERSION: u64 = 0x00;
const KVM_CREATE_VM: u64 = 0x01;
const KVM_GET_MSR_INDEX_LIST: u64 = 0x02;
const KVM_CHECK_EXTENSION: u64 = 0x03;
const KVM_GET_VCPU_MMAP_SIZE: u64 = 0x04;
const KVM_GET_SUPPORTED_CPUID: u64 = 0x05;

const KVM_CREATE_VCPU: u64 = 0x41;
const KVM_GET_DIRTY_LOG: u64 = 0x42;
const KVM_SET_NR_MMU_PAGES: u64 = 0x44;
const KVM_GET_NR_MMU_PAGES: u64 = 0x45;
const KVM_SET_USER_MEMORY_REGION: u64 = 0x46;
const KVM_SET_TSS_ADDR: u64 = 0x47;
const KVM_SET_IDENTITY_MAP_ADDR: u64 = 0x48;
const KVM_SET_BOOT_CPU_ID: u64 = 0x52;

const KVM_RUN: u64 = 0x80;
const KVM_GET_REGS: u64 = 0x81;
const KVM_SET_REGS: u64 = 0x82;
const KVM_GET_SREGS: u64 = 0x83;
const KVM_SET_SREGS: u64 = 0x84;
const KVM_GET_MSRS: u64 = 0x88;
const KVM_SET_MSRS: u64 = 0x89;
const KVM_GET_FPU: u64 = 0x8a;
const KVM_SET_FPU: u64 = 0x8b;
const KVM_GET_LAPIC: u64 = 0x8e;
const KVM_SET_LAPIC: u64 = 0x8f;

const KVM_CREATE_IRQCHIP: u64 = 0x60;
const KVM_IRQ_LINE: u64 = 0x61;
const KVM_GET_IRQCHIP: u64 = 0x62;
const KVM_SET_IRQCHIP: u64 = 0x63;
const KVM_IOEVENTFD: u64 = 0x64;
const KVM_IRQFD: u64 = 0x65;

const KVM_CAP_USER_MEMORY: u32 = 1;
const KVM_CAP_IRQCHIP: u32 = 2;
const KVM_CAP_HLT: u32 = 3;
const KVM_CAP_MMU_SHADOW_CACHE_CONTROL: u32 = 4;
const KVM_CAP_USER_NMI: u32 = 5;
const KVM_CAP_SET_TSS_ADDR: u32 = 6;
const KVM_CAP_VCPU_EVENTS: u32 = 9;
const KVM_CAP_EXT_CPUID: u32 = 10;
const KVM_CAP_IRQCHIP_SPLIT: u32 = 17;
const KVM_CAP_ENABLE_CAP: u32 = 18;
const KVM_CAP_XSAVE: u32 = 24;
const KVM_CAP_XCRS: u32 = 25;
const KVM_CAP_ADJUST_CLOCK: u32 = 27;
const KVM_CAP_TSC_CONTROL: u32 = 29;
const KVM_CAP_VCPU_EVENTS_SMM: u32 = 30;

const KVM_EXIT_IO: u32 = 1;
const KVM_EXIT_MMIO: u32 = 2;
const KVM_EXIT_IRQ_WINDOW_OPEN: u32 = 7;
const KVM_EXIT_SHUTDOWN: u32 = 8;
const KVM_EXIT_FAIL_ENTRY: u32 = 9;
const KVM_EXIT_INTR: u32 = 10;
const KVM_EXIT_SET_TPR: u32 = 11;
const KVM_EXIT_TPR_ACCESS: u32 = 12;
const KVM_EXIT_S390_SIEIC: u32 = 13;
const KVM_EXIT_S390_RESET: u32 = 14;
const KVM_EXIT_DCR: u32 = 15;
const KVM_EXIT_NMI: u32 = 16;
const KVM_EXIT_INTERNAL_ERROR: u32 = 17;
const KVM_EXIT_OSI: u32 = 18;
const KVM_EXIT_PAPR_HCALL: u32 = 19;

/// Errors that can occur during KVM operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KvmError {
    /// KVM is not available on this system
    KvmNotAvailable,
    /// Invalid VM file descriptor
    InvalidVmFd,
    /// Invalid vCPU file descriptor
    InvalidVcpuFd(i32),
    /// Failed to create VM
    VmCreationFailed,
    /// Failed to create vCPU
    VcpuCreationFailed,
    /// Memory region setup failed
    MemoryRegionFailed,
    /// Invalid memory slot
    InvalidSlot(u32),
    /// No available memory slots
    NoAvailableSlots,
    /// ioctl operation failed
    IoctlFailed(u64, i32),
    /// vCPU run failed
    VcpuRunFailed,
    /// Invalid exit reason
    InvalidExitReason(u32),
    /// IRQ chip operation failed
    IrqChipFailed,
    /// MSR operation failed
    MsrFailed,
    /// Feature not supported
    FeatureNotSupported(u32),
    /// Invalid parameter
    InvalidParameter(String),
}

impl From<KvmError> for KernelError {
    fn from(err: KvmError) -> Self {
        KernelError::Virtualization(format!("KVM error: {:?}", err))
    }
}

/// KVM system capabilities
#[derive(Debug, Clone, Copy)]
pub struct KvmCapabilities {
    /// User memory regions supported
    pub user_memory: bool,
    /// IRQ chip emulation supported
    pub irqchip: bool,
    /// HLT instruction supported
    pub hlt: bool,
    /// User NMI supported
    pub user_nmi: bool,
    /// TSS address setting supported
    pub set_tss_addr: bool,
    /// vCPU events supported
    pub vcpu_events: bool,
    /// Extended CPUID supported
    pub ext_cpuid: bool,
    /// Split IRQ chip supported
    pub irqchip_split: bool,
    /// XSAVE state supported
    pub xsave: bool,
    /// XCRs supported
    pub xcrs: bool,
    /// Clock adjustment supported
    pub adjust_clock: bool,
    /// TSC control supported
    pub tsc_control: bool,
}

impl Default for KvmCapabilities {
    fn default() -> Self {
        Self {
            user_memory: false,
            irqchip: false,
            hlt: false,
            user_nmi: false,
            set_tss_addr: false,
            vcpu_events: false,
            ext_cpuid: false,
            irqchip_split: false,
            xsave: false,
            xcrs: false,
            adjust_clock: false,
            tsc_control: false,
        }
    }
}

/// Memory slot configuration
#[derive(Debug, Clone)]
pub struct MemorySlot {
    /// Slot number
    pub slot: u32,
    /// Guest physical address
    pub guest_phys_addr: u64,
    /// Size of the memory region
    pub size: u64,
    /// Userspace address
    pub userspace_addr: u64,
    /// Flags for the memory region
    pub flags: u32,
}

impl MemorySlot {
    /// Create a new memory slot
    pub fn new(slot: u32, guest_phys_addr: u64, size: u64, userspace_addr: u64) -> Self {
        Self {
            slot,
            guest_phys_addr,
            size,
            userspace_addr,
            flags: 0,
        }
    }

    /// Set flags for the memory region
    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }

    /// Create a read-only memory region
    pub fn readonly(mut self) -> Self {
        self.flags |= 1 << 0;
        self
    }

    /// Create a dirty-log tracked memory region
    pub fn log_dirty(mut self) -> Self {
        self.flags |= 1 << 1;
        self
    }
}

/// KVM exit reason information
#[derive(Debug, Clone)]
pub enum KvmExit {
    /// I/O instruction
    Io {
        direction: u8,
        size: u8,
        port: u16,
        count: u32,
        offset: u64,
    },
    /// MMIO access
    Mmio {
        is_write: bool,
        addr: u64,
        length: u64,
        data: [u8; 8],
    },
    /// IRQ window open
    IrqWindowOpen,
    /// Shutdown
    Shutdown,
    /// Failed entry
    FailEntry {
        hardware_entry_failure_reason: u64,
    },
    /// External interrupt pending
    Intr,
    /// TPR access
    TprAccess,
    /// Internal error
    InternalError {
        suberror: u32,
    },
    /// Unknown exit reason
    Unknown(u32),
}

/// MSR (Model-Specific Register) entry
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct MsrEntry {
    /// MSR index
    pub index: u32,
    /// MSR value
    pub data: u64,
}

/// vCPU general-purpose registers
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct KvmRegs {
    /// RAX register
    pub rax: u64,
    /// RBX register
    pub rbx: u64,
    /// RCX register
    pub rcx: u64,
    /// RDX register
    pub rdx: u64,
    /// RSI register
    pub rsi: u64,
    /// RDI register
    pub rdi: u64,
    /// RSP register
    pub rsp: u64,
    /// RBP register
    pub rbp: u64,
    /// R8 register
    pub r8: u64,
    /// R9 register
    pub r9: u64,
    /// R10 register
    pub r10: u64,
    /// R11 register
    pub r11: u64,
    /// R12 register
    pub r12: u64,
    /// R13 register
    pub r13: u64,
    /// R14 register
    pub r14: u64,
    /// R15 register
    pub r15: u64,
    /// RIP register
    pub rip: u64,
    /// RFLAGS register
    pub rflags: u64,
}

/// vCPU special registers
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct KvmSregs {
    /// CS segment register
    pub cs: KvmSegment,
    /// DS segment register
    pub ds: KvmSegment,
    /// ES segment register
    pub es: KvmSegment,
    /// FS segment register
    pub fs: KvmSegment,
    /// GS segment register
    pub gs: KvmSegment,
    /// SS segment register
    pub ss: KvmSegment,
    /// TR segment register
    pub tr: KvmSegment,
    /// LDT segment register
    pub ldt: KvmSegment,
    /// GDT register
    pub gdt: KvmDtable,
    /// IDT register
    pub idt: KvmDtable,
    /// CR0 control register
    pub cr0: u64,
    /// CR2 control register
    pub cr2: u64,
    /// CR3 control register
    pub cr3: u64,
    /// CR4 control register
    pub cr4: u64,
    /// CR8 control register
    pub cr8: u64,
    /// EFER register
    pub efer: u64,
    /// APIC base
    pub apic_base: u64,
    /// Interrupt bitmap
    pub interrupt_bitmap: [u64; 4],
}

/// Segment register
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct KvmSegment {
    /// Segment base
    pub base: u64,
    /// Segment limit
    pub limit: u32,
    /// Selector
    pub selector: u16,
    /// Type
    pub typ: u8,
    /// Present bit
    pub present: u8,
    /// DPL (Descriptor Privilege Level)
    pub dpl: u8,
    /// DB (Default Operation Size)
    pub db: u8,
    /// S (System/Code or Data)
    pub s: u8,
    /// L (Long mode)
    pub l: u8,
    /// G (Granularity)
    pub g: u8,
    /// AVL (Available)
    pub avl: u8,
    /// L (Unusable in long mode)
    pub unusable: u8,
    /// Padding
    pub padding: u8,
}

/// Descriptor table register
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct KvmDtable {
    /// Base address
    pub base: u64,
    /// Limit
    pub limit: u16,
    /// Padding
    pub padding: [u16; 3],
}

/// KVM instance
#[derive(Debug)]
pub struct Kvm {
    /// KVM file descriptor (simulated)
    fd: Arc<Mutex<i32>>,
    /// KVM capabilities
    capabilities: KvmCapabilities,
    /// Next available VM ID
    next_vm_id: AtomicU32,
    /// Number of active VMs
    active_vms: AtomicU32,
}

impl Kvm {
    /// Create a new KVM instance
    ///
    /// This initializes the KVM subsystem and queries available capabilities.
    pub fn new() -> Result<Self, KvmError> {
        // In a real implementation, this would open /dev/kvm
        // For now, we simulate a successful KVM initialization
        let fd = Arc::new(Mutex::new(3)); // Simulated fd

        let capabilities = KvmCapabilities {
            user_memory: true,
            irqchip: true,
            hlt: true,
            user_nmi: true,
            set_tss_addr: true,
            vcpu_events: true,
            ext_cpuid: true,
            irqchip_split: true,
            xsave: true,
            xcrs: true,
            adjust_clock: true,
            tsc_control: true,
        };

        Ok(Self {
            fd,
            capabilities,
            next_vm_id: AtomicU32::new(1),
            active_vms: AtomicU32::new(0),
        })
    }

    /// Get KVM API version
    pub fn api_version(&self) -> i32 {
        12 // KVM API version 12
    }

    /// Check if a KVM capability is available
    pub fn check_capability(&self, cap: u32) -> bool {
        match cap {
            KVM_CAP_USER_MEMORY => self.capabilities.user_memory,
            KVM_CAP_IRQCHIP => self.capabilities.irqchip,
            KVM_CAP_HLT => self.capabilities.hlt,
            KVM_CAP_USER_NMI => self.capabilities.user_nmi,
            KVM_CAP_SET_TSS_ADDR => self.capabilities.set_tss_addr,
            KVM_CAP_VCPU_EVENTS => self.capabilities.vcpu_events,
            KVM_CAP_EXT_CPUID => self.capabilities.ext_cpuid,
            KVM_CAP_IRQCHIP_SPLIT => self.capabilities.irqchip_split,
            KVM_CAP_XSAVE => self.capabilities.xsave,
            KVM_CAP_XCRS => self.capabilities.xcrs,
            KVM_CAP_ADJUST_CLOCK => self.capabilities.adjust_clock,
            KVM_CAP_TSC_CONTROL => self.capabilities.tsc_control,
            _ => false,
        }
    }

    /// Get KVM vCPU mmap size
    pub fn vcpu_mmap_size(&self) -> usize {
        0x1000 // 4KB run structure
    }

    /// Get recommended number of MMU pages
    pub fn get_nr_mmu_pages(&self) -> u32 {
        256
    }

    /// Create a new virtual machine
    pub fn create_vm(&self, num_vcpus: u32) -> Result<KvmVm, KvmError> {
        if !self.capabilities.user_memory {
            return Err(KvmError::FeatureNotSupported(KVM_CAP_USER_MEMORY));
        }

        let vm_id = self.next_vm_id.fetch_add(1, Ordering::SeqCst);
        self.active_vms.fetch_add(1, Ordering::SeqCst);

        KvmVm::new(vm_id, num_vcpus, self.capabilities)
    }

    /// Get the number of active VMs
    pub fn active_vm_count(&self) -> u32 {
        self.active_vms.load(Ordering::SeqCst)
    }

    /// Get KVM capabilities
    pub fn capabilities(&self) -> KvmCapabilities {
        self.capabilities
    }
}

/// KVM virtual machine
#[derive(Debug)]
pub struct KvmVm {
    /// VM ID
    vm_id: u32,
    /// VM file descriptor (simulated)
    fd: Arc<Mutex<i32>>,
    /// Number of vCPUs
    num_vcpus: u32,
    /// vCPUs in this VM
    vcpus: Arc<Mutex<BTreeMap<u32, Arc<Mutex<KvmVcpu>>>>>,
    /// Memory slots
    memory_slots: Arc<Mutex<BTreeMap<u32, MemorySlot>>>,
    /// Next available slot number
    next_slot: AtomicU32,
    /// IRQ chip initialized
    irqchip_initialized: Arc<Mutex<bool>>,
    /// KVM capabilities
    capabilities: KvmCapabilities,
}

impl KvmVm {
    /// Create a new KVM VM
    fn new(vm_id: u32, num_vcpus: u32, capabilities: KvmCapabilities) -> Result<Self, KvmError> {
        let fd = Arc::new(Mutex::new(4)); // Simulated fd

        Ok(Self {
            vm_id,
            fd,
            num_vcpus,
            vcpus: Arc::new(Mutex::new(BTreeMap::new())),
            memory_slots: Arc::new(Mutex::new(BTreeMap::new())),
            next_slot: AtomicU32::new(0),
            irqchip_initialized: Arc::new(Mutex::new(false)),
            capabilities,
        })
    }

    /// Get VM ID
    pub fn vm_id(&self) -> u32 {
        self.vm_id
    }

    /// Get number of vCPUs
    pub fn num_vcpus(&self) -> u32 {
        self.num_vcpus
    }

    /// Create a new vCPU
    pub fn create_vcpu(&self, vcpu_id: u32) -> Result<Arc<Mutex<KvmVcpu>>, KvmError> {
        if vcpu_id >= self.num_vcpus {
            return Err(KvmError::InvalidParameter(format!(
                "vCPU ID {} exceeds number of vCPUs {}",
                vcpu_id, self.num_vcpus
            )));
        }

        let vcpu = Arc::new(Mutex::new(KvmVcpu::new(vcpu_id, self.vm_id)?));
        let mut vcpus = self.vcpus.lock();
        vcpus.insert(vcpu_id, vcpu.clone());

        Ok(vcpu)
    }

    /// Get a vCPU by ID
    pub fn get_vcpu(&self, vcpu_id: u32) -> Option<Arc<Mutex<KvmVcpu>>> {
        let vcpus = self.vcpus.lock();
        vcpus.get(&vcpu_id).cloned()
    }

    /// Set up a memory region
    pub fn set_user_memory_region(&self, slot: MemorySlot) -> Result<(), KvmError> {
        if !self.capabilities.user_memory {
            return Err(KvmError::FeatureNotSupported(KVM_CAP_USER_MEMORY));
        }

        let mut slots = self.memory_slots.lock();
        slots.insert(slot.slot, slot);

        Ok(())
    }

    /// Add a memory region (auto-assigns slot)
    pub fn add_memory_region(
        &self,
        guest_phys_addr: u64,
        size: u64,
        userspace_addr: u64,
    ) -> Result<u32, KvmError> {
        let slot = self.next_slot.fetch_add(1, Ordering::SeqCst);
        let mem_slot = MemorySlot::new(slot, guest_phys_addr, size, userspace_addr);

        self.set_user_memory_region(mem_slot)?;
        Ok(slot)
    }

    /// Get a memory slot
    pub fn get_memory_slot(&self, slot: u32) -> Option<MemorySlot> {
        let slots = self.memory_slots.lock();
        slots.get(&slot).cloned()
    }

    /// Initialize the IRQ chip
    pub fn create_irq_chip(&self) -> Result<(), KvmError> {
        if !self.capabilities.irqchip {
            return Err(KvmError::FeatureNotSupported(KVM_CAP_IRQCHIP));
        }

        let mut initialized = self.irqchip_initialized.lock();
        *initialized = true;

        Ok(())
    }

    /// Check if IRQ chip is initialized
    pub fn irqchip_initialized(&self) -> bool {
        *self.irqchip_initialized.lock()
    }

    /// Set TSS address
    pub fn set_tss_addr(&self, addr: u64) -> Result<(), KvmError> {
        if !self.capabilities.set_tss_addr {
            return Err(KvmError::FeatureNotSupported(KVM_CAP_SET_TSS_ADDR));
        }

        // In a real implementation, this would call ioctl(KVM_SET_TSS_ADDR)
        Ok(())
    }

    /// Get dirty log for a memory slot
    pub fn get_dirty_log(&self, slot: u32, bitmap: &mut [u8]) -> Result<(), KvmError> {
        let slots = self.memory_slots.lock();
        if !slots.contains_key(&slot) {
            return Err(KvmError::InvalidSlot(slot));
        }

        // In a real implementation, this would call ioctl(KVM_GET_DIRTY_LOG)
        // For now, we simulate no dirty pages
        bitmap.fill(0);

        Ok(())
    }

    /// Set number of MMU pages
    pub fn set_nr_mmu_pages(&self, nr_mmu_pages: u32) -> Result<(), KvmError> {
        // In a real implementation, this would call ioctl(KVM_SET_NR_MMU_PAGES)
        Ok(())
    }

    /// Get number of MMU pages
    pub fn get_nr_mmu_pages(&self) -> Result<u32, KvmError> {
        // In a real implementation, this would call ioctl(KVM_GET_NR_MMU_PAGES)
        Ok(256)
    }

    /// Get all vCPUs
    pub fn vcpus(&self) -> Vec<Arc<Mutex<KvmVcpu>>> {
        let vcpus = self.vcpus.lock();
        vcpus.values().cloned().collect()
    }
}

impl Drop for KvmVm {
    fn drop(&mut self) {
        // Cleanup VM resources
        let vcpus = self.vcpus.lock();
        // vCPUs will be dropped when their Arc references go away
    }
}

/// KVM virtual CPU
#[derive(Debug)]
pub struct KvmVcpu {
    /// vCPU ID
    vcpu_id: u32,
    /// VM ID
    vm_id: u32,
    /// vCPU file descriptor (simulated)
    fd: Arc<Mutex<i32>>,
    /// vCPU state
    state: Arc<Mutex<KvmVcpuState>>,
    /// Number of times this vCPU has run
    run_count: AtomicU64,
}

#[derive(Debug, Default)]
struct KvmVcpuState {
    regs: KvmRegs,
    sregs: KvmSregs,
    msrs: Vec<MsrEntry>,
}

impl KvmVcpu {
    /// Create a new KVM vCPU
    fn new(vcpu_id: u32, vm_id: u32) -> Result<Self, KvmError> {
        let fd = Arc::new(Mutex::new(5)); // Simulated fd

        Ok(Self {
            vcpu_id,
            vm_id,
            fd,
            state: Arc::new(Mutex::new(KvmVcpuState::default())),
            run_count: AtomicU64::new(0),
        })
    }

    /// Get vCPU ID
    pub fn vcpu_id(&self) -> u32 {
        self.vcpu_id
    }

    /// Get VM ID
    pub fn vm_id(&self) -> u32 {
        self.vm_id
    }

    /// Run the vCPU
    ///
    /// This executes the vCPU until an exit condition is encountered.
    pub fn run(&self) -> Result<KvmExit, KvmError> {
        self.run_count.fetch_add(1, Ordering::SeqCst);

        // In a real implementation, this would call ioctl(KVM_RUN)
        // For now, we simulate an exit condition
        Ok(KvmExit::Intr)
    }

    /// Get vCPU registers
    pub fn get_regs(&self) -> Result<KvmRegs, KvmError> {
        let state = self.state.lock();
        Ok(state.regs)
    }

    /// Set vCPU registers
    pub fn set_regs(&self, regs: KvmRegs) -> Result<(), KvmError> {
        let mut state = self.state.lock();
        state.regs = regs;
        Ok(())
    }

    /// Get vCPU special registers
    pub fn get_sregs(&self) -> Result<KvmSregs, KvmError> {
        let state = self.state.lock();
        Ok(state.sregs)
    }

    /// Set vCPU special registers
    pub fn set_sregs(&self, sregs: KvmSregs) -> Result<(), KvmError> {
        let mut state = self.state.lock();
        state.sregs = sregs;
        Ok(())
    }

    /// Get MSRs
    pub fn get_msrs(&self, msr_list: &mut [MsrEntry]) -> Result<usize, KvmError> {
        let state = self.state.lock();
        let n = core::cmp::min(msr_list.len(), state.msrs.len());
        msr_list[..n].copy_from_slice(&state.msrs[..n]);
        Ok(n)
    }

    /// Set MSRs
    pub fn set_msrs(&self, msr_list: &[MsrEntry]) -> Result<usize, KvmError> {
        let mut state = self.state.lock();
        for msr in msr_list {
            // Update or add MSR
            if let Some(entry) = state.msrs.iter_mut().find(|e| e.index == msr.index) {
                entry.data = msr.data;
            } else {
                state.msrs.push(*msr);
            }
        }
        Ok(msr_list.len())
    }

    /// Read an MSR
    pub fn read_msr(&self, index: u32) -> Result<u64, KvmError> {
        let state = self.state.lock();
        Ok(state
            .msrs
            .iter()
            .find(|e| e.index == index)
            .map(|e| e.data)
            .unwrap_or(0))
    }

    /// Write an MSR
    pub fn write_msr(&self, index: u32, data: u64) -> Result<(), KvmError> {
        let mut state = self.state.lock();
        if let Some(entry) = state.msrs.iter_mut().find(|e| e.index == index) {
            entry.data = data;
        } else {
            state.msrs.push(MsrEntry { index, data });
        }
        Ok(())
    }

    /// Get FPU state
    pub fn get_fpu(&self) -> Result<Vec<u8>, KvmError> {
        // Return simulated FPU state
        Ok(vec![0u8; 512])
    }

    /// Set FPU state
    pub fn set_fpu(&self, _fpu: &[u8]) -> Result<(), KvmError> {
        Ok(())
    }

    /// Get LAPIC state
    pub fn get_lapic(&self) -> Result<Vec<u8>, KvmError> {
        // Return simulated LAPIC state
        Ok(vec![0u8; 1024])
    }

    /// Set LAPIC state
    pub fn set_lapic(&self, _lapic: &[u8]) -> Result<(), KvmError> {
        Ok(())
    }

    /// Get run count
    pub fn run_count(&self) -> u64 {
        self.run_count.load(Ordering::SeqCst)
    }

    /// Reset vCPU state
    pub fn reset(&self) -> Result<(), KvmError> {
        let mut state = self.state.lock();
        state.regs = KvmRegs::default();
        state.sregs = KvmSregs::default();
        state.msrs.clear();
        Ok(())
    }

    /// Get vCPU state as a tuple
    pub fn get_state(&self) -> Result<(KvmRegs, KvmSregs), KvmError> {
        let state = self.state.lock();
        Ok((state.regs, state.sregs))
    }

    /// Set vCPU state from a tuple
    pub fn set_state(&self, regs: KvmRegs, sregs: KvmSregs) -> Result<(), KvmError> {
        let mut state = self.state.lock();
        state.regs = regs;
        state.sregs = sregs;
        Ok(())
    }
}

/// KVM MSR list
#[derive(Debug)]
pub struct KvmMsrList {
    /// MSR indices
    indices: Vec<u32>,
}

impl KvmMsrList {
    /// Create a new MSR list
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
        }
    }

    /// Add an MSR index
    pub fn add(&mut self, index: u32) {
        self.indices.push(index);
    }

    /// Get the list of MSR indices
    pub fn as_slice(&self) -> &[u32] {
        &self.indices
    }

    /// Get the number of MSRs
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

impl Default for KvmMsrList {
    fn default() -> Self {
        Self::new()
    }
}

/// KVM CPUID entry
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct KvmCpuidEntry {
    /// CPUID function
    pub function: u32,
    /// CPUID index
    pub index: u32,
    /// EAX register
    pub eax: u32,
    /// EBX register
    pub ebx: u32,
    /// ECX register
    pub ecx: u32,
    /// EDX register
    pub edx: u32,
    /// Padding
    pub padding: [u32; 3],
}

/// Get the list of supported MSRs
pub fn get_msr_index_list() -> KvmMsrList {
    let mut list = KvmMsrList::new();

    // Add common x86_64 MSRs
    list.add(0x1b); // IA32_APIC_BASE
    list.add(0x10a); // IA32_FEATURE_CONTROL
    list.add(0x174); // IA32_SYSENTER_CS
    list.add(0x175); // IA32_SYSENTER_ESP
    list.add(0x176); // IA32_SYSENTER_EIP
    list.add(0x19c); // IA32_TSC_AUX
    list.add(0x1a0); // IA32_MISC_ENABLE
    list.add(0xc000_0080); // IA32_EFER
    list.add(0xc000_0081); // IA32_STAR
    list.add(0xc000_0082); // IA32_LSTAR
    list.add(0xc000_0083); // IA32_CSTAR
    list.add(0xc000_0084); // IA32_FMASK
    list.add(0xc000_0100); // IA32_FS_BASE
    list.add(0xc000_0101); // IA32_GS_BASE
    list.add(0xc000_0102); // IA32_KERNEL_GS_BASE

    list
}

/// Get the default CPUID entries for a vCPU
pub fn get_default_cpuid() -> Vec<KvmCpuidEntry> {
    let mut entries = Vec::new();

    // Basic CPUID information
    entries.push(KvmCpuidEntry {
        function: 0x00,
        eax: 0x0d, // Maximum standard level
        ebx: 0x756e_6547, // "Genu"
        ecx: 0x6c65_746e, // "ntel"
        edx: 0x4965_6e69, // "ineI"
        ..Default::default()
    });

    // Model and feature information
    entries.push(KvmCpuidEntry {
        function: 0x01,
        eax: 0x0003_06f2, // Family 6, Model 60, Stepping 2
        ebx: 0x0100_8000,
        ecx: 0x7f8e_fbff, // Feature flags ECX
        edx: 0x178bfbff, // Feature flags EDX
        ..Default::default()
    });

    // Extended feature flags
    entries.push(KvmCpuidEntry {
        function: 0x07,
        eax: 0x00,
        ebx: 0x2083_2d09, // Feature flags EBX
        ecx: 0x0000_0400,
        edx: 0x0000_0000,
        ..Default::default()
    });

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvm_creation() {
        let kvm = Kvm::new().unwrap();
        assert_eq!(kvm.api_version(), 12);
        assert!(kvm.check_capability(KVM_CAP_USER_MEMORY));
    }

    #[test]
    fn test_vm_creation() {
        let kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm(2).unwrap();
        assert_eq!(vm.num_vcpus(), 2);
    }

    #[test]
    fn test_vcpu_creation() {
        let kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm(2).unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let vcpu_guard = vcpu.lock();
        assert_eq!(vcpu_guard.vcpu_id(), 0);
    }

    #[test]
    fn test_memory_slot() {
        let kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm(2).unwrap();

        let slot_id = vm
            .add_memory_region(0x1000, 0x1000, 0x7fff_0000_0000)
            .unwrap();

        let slot = vm.get_memory_slot(slot_id).unwrap();
        assert_eq!(slot.slot, slot_id);
        assert_eq!(slot.guest_phys_addr, 0x1000);
        assert_eq!(slot.size, 0x1000);
    }

    #[test]
    fn test_registers() {
        let kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm(2).unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let vcpu_guard = vcpu.lock();

        let mut regs = KvmRegs::default();
        regs.rax = 0x1234_5678_9abc_def0;
        vcpu_guard.set_regs(regs).unwrap();

        let retrieved = vcpu_guard.get_regs().unwrap();
        assert_eq!(retrieved.rax, 0x1234_5678_9abc_def0);
    }

    #[test]
    fn test_msr_operations() {
        let kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm(2).unwrap();
        let vcpu = vm.create_vcpu(0).unwrap();
        let vcpu_guard = vcpu.lock();

        vcpu_guard.write_msr(0x1b, 0xfee0_0900).unwrap();
        let value = vcpu_guard.read_msr(0x1b).unwrap();
        assert_eq!(value, 0xfee0_0900);
    }
}
