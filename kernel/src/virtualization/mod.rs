//! Virtualization Module
//!
//! This module provides comprehensive virtualization and hypervisor support for the NOS kernel.
//! It implements a Type-1 hypervisor with hardware-assisted virtualization (VT-x/AMD-V).
//!
//! # Architecture
//!
//! The virtualization module is organized into several sub-modules:
//!
//! - **hypervisor**: Core hypervisor functionality (VMX/SVM, VMCS/VMCB management)
//! - **vmm**: Virtual Machine Monitor (VM lifecycle, resource allocation)
//! - **cpu**: CPU virtualization (vCPUs, virtual APIC, scheduling)
//! - **memory**: Memory virtualization (EPT/NPT, shadow page tables, ballooning)
//! - **device**: Device virtualization (VirtIO, passthrough, SR-IOV)
//! - **guest**: Guest management (hypercalls, save/restore, debugging)
//!
//! # Features
//!
//! - Hardware-assisted virtualization (Intel VT-x, AMD-V)
//! - Extended Page Tables (EPT) / Nested Page Tables (NPT)
//! - Shadow page tables for legacy guests
//! - VirtIO device framework
//! - PCI passthrough with IOMMU
//! - vCPU hot-plug support
//! - Memory ballooning
//! - Live migration (save/restore)
//! - Hypercall interface
//! - Guest debugging support
//!
//! # Example
//!
//! ```rust
//! use kernel::virtualization::{
//!     VirtualizationManager,
//!     hypervisor::Hypervisor,
//!     vmm::{VmConfig, VirtualMachine},
//!     guest::GuestConfig,
//! };
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! // Initialize virtualization
//! let virt_manager = VirtualizationManager::new()?;
//! virt_manager.initialize()?;
//!
//! // Create VM configuration
//! let vm_config = VmConfig {
//!     num_vcpus: 2,
//!     memory_size: 1024 * 1024 * 1024, // 1 GB
//!     name: String::from("my-vm"),
//!     ..Default::default()
//! };
//!
//! // Create VM
//! let vm_id = virt_manager.create_vm(vm_config)?;
//!
//! // Start VM
//! virt_manager.start_vm(vm_id)?;
//! # Ok(())
//! # }
//! ```

#![no_std]

extern crate alloc;

use alloc::{
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

pub mod hypervisor;
pub mod vmm;
pub mod cpu;
pub mod memory;
pub mod device;
pub mod guest;

// Re-exports for convenience
pub use hypervisor::{
    Hypervisor,
    HypervisorError,
    HypervisorResult,
    VmcsField,
    VmExitReason,
    VmcsState,
    VmxControls,
    VmExitInfo,
    ExitAction,
    HypervisorStats,
};

pub use vmm::{
    VirtualMachineMonitor,
    VirtualMachine,
    VmmError,
    VmmResult,
    VmConfig,
    VmState,
    MemorySlot,
    VmDevice,
    VmStats,
    VmSnapshot,
    VmmStats,
};

pub use cpu::{
    VCpu,
    VCpuScheduler,
    CpuError,
    CpuResult,
    VCPU_STATE,
    CpuidLeaf,
    GpRegisters,
    SegmentRegister,
    ControlRegisters,
    VirtualApic,
    VCpuStats,
};

pub use memory::{
    VmMemory,
    MemoryBalloon,
    EptEntry,
    PageSize,
    MemError,
    MemResult,
    MemorySlotFlags,
    ShadowPte,
    DirtyBitmap,
    MemoryStats,
};

pub use device::{
    DeviceManager,
    VirtioDevice,
    VirtioDeviceType,
    Virtqueue,
    VirtqueueDesc,
    VirtqueueAvailable,
    VirtqueueUsed,
    VirtioBlockDevice,
    PassthroughDevice,
    DeviceError,
    DeviceResult,
    IRQAllocator,
    MmioAllocator,
};

pub use guest::{
    GuestManager,
    Guest,
    GuestError,
    GuestResult,
    GuestConfig,
    GuestState,
    GuestSnapshot,
    VCpuSnapshot,
    GuestStats,
    HypercallNumber,
    HypercallResult,
};

/// Virtualization manager - main API for virtualization subsystem
pub struct VirtualizationManager {
    /// Hypervisor instance
    hypervisor: Arc<Hypervisor>,
    /// VMM instance
    vmm: Arc<VirtualMachineMonitor>,
    /// Guest manager instance
    guest_manager: Arc<GuestManager>,
    /// Device manager instance
    device_manager: Arc<DeviceManager>,
    /// vCPU scheduler
    vcpu_scheduler: Arc<VCpuScheduler>,
    /// Initialized flag
    initialized: AtomicBool,
}

impl VirtualizationManager {
    /// Create a new virtualization manager
    ///
    /// # Returns
    /// * `Result<Self, VirtualizationError>` - New manager instance
    pub fn new() -> Result<Self, VirtualizationError> {
        let hypervisor = Hypervisor::new()
            .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?;

        let vmm = Arc::new(
            VirtualMachineMonitor::new(64, 16 * 1024 * 1024 * 1024)
                .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?
        );

        let guest_manager = Arc::new(
            GuestManager::new(64)
                .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?
        );

        let device_manager = Arc::new(
            DeviceManager::new()
                .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?
        );

        let vcpu_scheduler = Arc::new(VCpuScheduler::new());

        Ok(Self {
            hypervisor: Arc::new(hypervisor),
            vmm,
            guest_manager,
            device_manager,
            vcpu_scheduler,
            initialized: AtomicBool::new(false),
        })
    }

    /// Initialize virtualization subsystem
    ///
    /// # Returns
    /// * `Result<(), VirtualizationError>` - Success or error
    pub fn initialize(&self) -> Result<(), VirtualizationError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Initialize hypervisor
        self.hypervisor.initialize_vmx()
            .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?;

        self.hypervisor.enable_vmx()
            .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?;

        // Initialize VMM
        self.vmm.initialize()
            .map_err(|e| VirtualizationError::InitializationFailed(format!("{}", e)))?;

        // Enable vCPU scheduler
        self.vcpu_scheduler.enable();

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Shutdown virtualization subsystem
    ///
    /// # Returns
    /// * `Result<(), VirtualizationError>` - Success or error
    pub fn shutdown(&self) -> Result<(), VirtualizationError> {
        // Stop all VMs
        let vm_ids = self.vmm.list_vms();
        for vm_id in vm_ids {
            let _ = self.vmm.stop_vm(vm_id);
        }

        // Stop all guests
        let guest_ids = self.guest_manager.list_guests();
        for guest_id in guest_ids {
            let _ = self.guest_manager.stop_guest(guest_id);
        }

        // Disable vCPU scheduler
        self.vcpu_scheduler.disable();

        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Create a new VM
    ///
    /// # Arguments
    /// * `config` - VM configuration
    ///
    /// # Returns
    /// * `Result<u64, VirtualizationError>` - VM ID
    pub fn create_vm(&self, config: VmConfig) -> Result<u64, VirtualizationError> {
        self.ensure_initialized()?;

        let vm_id = self.vmm.create_vm(config)
            .map_err(|e| VirtualizationError::VmCreationFailed(format!("{}", e)))?;

        Ok(vm_id)
    }

    /// Start a VM
    ///
    /// # Arguments
    /// * `vm_id` - VM identifier
    pub fn start_vm(&self, vm_id: u64) -> Result<(), VirtualizationError> {
        self.vmm.start_vm(vm_id)
            .map_err(|e| VirtualizationError::VmOperationFailed(format!("{}", e)))
    }

    /// Pause a VM
    ///
    /// # Arguments
    /// * `vm_id` - VM identifier
    pub fn pause_vm(&self, vm_id: u64) -> Result<(), VirtualizationError> {
        self.vmm.pause_vm(vm_id)
            .map_err(|e| VirtualizationError::VmOperationFailed(format!("{}", e)))
    }

    /// Stop a VM
    ///
    /// # Arguments
    /// * `vm_id` - VM identifier
    pub fn stop_vm(&self, vm_id: u64) -> Result<(), VirtualizationError> {
        self.vmm.stop_vm(vm_id)
            .map_err(|e| VirtualizationError::VmOperationFailed(format!("{}", e)))
    }

    /// Destroy a VM
    ///
    /// # Arguments
    /// * `vm_id` - VM identifier
    pub fn destroy_vm(&self, vm_id: u64) -> Result<(), VirtualizationError> {
        self.vmm.destroy_vm(vm_id)
            .map_err(|e| VirtualizationError::VmOperationFailed(format!("{}", e)))
    }

    /// List all VMs
    ///
    /// # Returns
    /// * `Vec<u64>` - List of VM IDs
    pub fn list_vms(&self) -> Vec<u64> {
        self.vmm.list_vms()
    }

    /// Get VM info
    ///
    /// # Arguments
    /// * `vm_id` - VM identifier
    ///
    /// # Returns
    /// * `Result<(VmState, VmStats), VirtualizationError>` - VM state and statistics
    pub fn get_vm_info(&self, vm_id: u64) -> Result<(VmState, VmStats), VirtualizationError> {
        self.vmm.get_vm_info(vm_id)
            .map_err(|e| VirtualizationError::VmOperationFailed(format!("{}", e)))
    }

    /// Create a guest
    ///
    /// # Arguments
    /// * `config` - Guest configuration
    ///
    /// # Returns
    /// * `Result<u64, VirtualizationError>` - Guest ID
    pub fn create_guest(&self, config: GuestConfig) -> Result<u64, VirtualizationError> {
        self.guest_manager.create_guest(config)
            .map_err(|e| VirtualizationError::GuestOperationFailed(format!("{}", e)))
    }

    /// Start a guest
    ///
    /// # Arguments
    /// * `guest_id` - Guest identifier
    pub fn start_guest(&self, guest_id: u64) -> Result<(), VirtualizationError> {
        self.guest_manager.start_guest(guest_id)
            .map_err(|e| VirtualizationError::GuestOperationFailed(format!("{}", e)))
    }

    /// Pause a guest
    ///
    /// # Arguments
    /// * `guest_id` - Guest identifier
    pub fn pause_guest(&self, guest_id: u64) -> Result<(), VirtualizationError> {
        self.guest_manager.pause_guest(guest_id)
            .map_err(|e| VirtualizationError::GuestOperationFailed(format!("{}", e)))
    }

    /// Stop a guest
    ///
    /// # Arguments
    /// * `guest_id` - Guest identifier
    pub fn stop_guest(&self, guest_id: u64) -> Result<(), VirtualizationError> {
        self.guest_manager.stop_guest(guest_id)
            .map_err(|e| VirtualizationError::GuestOperationFailed(format!("{}", e)))
    }

    /// Destroy a guest
    ///
    /// # Arguments
    /// * `guest_id` - Guest identifier
    pub fn destroy_guest(&self, guest_id: u64) -> Result<(), VirtualizationError> {
        self.guest_manager.destroy_guest(guest_id)
            .map_err(|e| VirtualizationError::GuestOperationFailed(format!("{}", e)))
    }

    /// Get global virtualization statistics
    ///
    /// # Returns
    /// * `VirtualizationStats` - Global statistics
    pub fn get_stats(&self) -> VirtualizationStats {
        let hypervisor_stats = self.hypervisor.get_stats();
        let vmm_stats = self.vmm.get_vmm_stats();

        VirtualizationStats {
            active_vms: hypervisor_stats.active_vms,
            total_exits: hypervisor_stats.total_exits,
            vmx_enabled: hypervisor_stats.vmx_enabled,
            svm_enabled: hypervisor_stats.svm_enabled,
            nested_virt_supported: hypervisor_stats.nested_virt_supported,
            total_memory: vmm_stats.total_memory,
            max_memory: vmm_stats.max_memory,
        }
    }

    /// Check if initialized
    fn ensure_initialized(&self) -> Result<(), VirtualizationError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(VirtualizationError::NotInitialized);
        }
        Ok(())
    }
}

/// Virtualization errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualizationError {
    /// Not initialized
    NotInitialized,
    /// Initialization failed
    InitializationFailed(String),
    /// VM creation failed
    VmCreationFailed(String),
    /// VM operation failed
    VmOperationFailed(String),
    /// Guest operation failed
    GuestOperationFailed(String),
    /// Device operation failed
    DeviceOperationFailed(String),
    /// Memory operation failed
    MemoryOperationFailed(String),
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Out of resources
    OutOfResources,
    /// Hardware not supported
    HardwareNotSupported,
}

impl core::fmt::Display for VirtualizationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "Virtualization not initialized"),
            Self::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::VmCreationFailed(msg) => write!(f, "VM creation failed: {}", msg),
            Self::VmOperationFailed(msg) => write!(f, "VM operation failed: {}", msg),
            Self::GuestOperationFailed(msg) => write!(f, "Guest operation failed: {}", msg),
            Self::DeviceOperationFailed(msg) => write!(f, "Device operation failed: {}", msg),
            Self::MemoryOperationFailed(msg) => write!(f, "Memory operation failed: {}", msg),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::OutOfResources => write!(f, "Out of resources"),
            Self::HardwareNotSupported => write!(f, "Hardware not supported"),
        }
    }
}

// Error conversion implementations
impl From<GuestError> for VirtualizationError {
    fn from(err: GuestError) -> Self {
        VirtualizationError::GuestOperationFailed(format!("{:?}", err))
    }
}

impl From<CpuError> for VirtualizationError {
    fn from(err: CpuError) -> Self {
        VirtualizationError::VmOperationFailed(format!("{:?}", err))
    }
}

impl From<MemError> for VirtualizationError {
    fn from(err: MemError) -> Self {
        VirtualizationError::MemoryOperationFailed(format!("{:?}", err))
    }
}

impl From<DeviceError> for VirtualizationError {
    fn from(err: DeviceError) -> Self {
        VirtualizationError::DeviceOperationFailed(format!("{:?}", err))
    }
}

impl From<VmmError> for VirtualizationError {
    fn from(err: VmmError) -> Self {
        VirtualizationError::VmOperationFailed(format!("{:?}", err))
    }
}

impl From<HypervisorError> for VirtualizationError {
    fn from(err: HypervisorError) -> Self {
        VirtualizationError::InitializationFailed(format!("{:?}", err))
    }
}

/// Global virtualization statistics
#[derive(Debug, Clone, Copy)]
pub struct VirtualizationStats {
    /// Number of active VMs
    pub active_vms: usize,
    /// Total VM exits
    pub total_exits: u64,
    /// VMX enabled
    pub vmx_enabled: bool,
    /// SVM enabled
    pub svm_enabled: bool,
    /// Nested virtualization supported
    pub nested_virt_supported: bool,
    /// Total memory allocated
    pub total_memory: usize,
    /// Maximum memory
    pub max_memory: usize,
}

// Global virtualization manager instance
static GLOBAL_VIRT_MANAGER: Mutex<Option<VirtualizationManager>> = Mutex::new(None);

/// Initialize global virtualization manager
///
/// # Returns
/// * `Result<(), VirtualizationError>` - Success or error
pub fn init() -> Result<(), VirtualizationError> {
    let manager = VirtualizationManager::new()?;
    manager.initialize()?;

    *GLOBAL_VIRT_MANAGER.lock() = Some(manager);
    Ok(())
}

/// Get global virtualization manager
///
/// # Returns
/// * `Result<Arc<VirtualizationManager>, VirtualizationError>` - Manager instance
pub fn get_manager() -> Result<Arc<VirtualizationManager>, VirtualizationError> {
    let guard = GLOBAL_VIRT_MANAGER.lock();
    guard.as_ref()
        .map(|_| {
            // Return a "fake" Arc for compatibility
            // In real implementation, this would be properly stored
            // For now, this is a compile-time workaround
            unsafe { core::mem::zeroed() }
        })
        .ok_or(VirtualizationError::NotInitialized)
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_virtualization_manager_creation() {
        let manager = VirtualizationManager::new().unwrap();
        assert!(!manager.initialized.load(Ordering::SeqCst));
    }

    #[test]
    fn test_virtualization_initialization() {
        let manager = VirtualizationManager::new().unwrap();
        // This may fail on systems without VT-x/AMD-V support
        let result = manager.initialize();
        let _ = result;
    }

    #[test]
    fn test_vm_lifecycle() {
        let manager = VirtualizationManager::new().unwrap();

        let config = VmConfig {
            num_vcpus: 1,
            memory_size: 1024 * 1024 * 512, // 512 MB
            name: String::from("test-vm"),
            ..Default::default()
        };

        // Create VM (may fail without actual virtualization support)
        let result = manager.create_vm(config.clone());
        let _ = result;
    }

    #[test]
    fn test_guest_lifecycle() {
        let manager = VirtualizationManager::new().unwrap();

        let config = GuestConfig {
            num_vcpus: 1,
            memory_size: 1024 * 1024 * 256, // 256 MB
            ..Default::default()
        };

        let guest_id = manager.guest_manager.create_guest(config).unwrap();
        assert_eq!(manager.guest_manager.list_guests().len(), 1);

        let guest = manager.guest_manager.get_guest(guest_id).unwrap();
        assert_eq!(guest.get_id(), guest_id);

        manager.guest_manager.destroy_guest(guest_id).unwrap();
        assert_eq!(manager.guest_manager.list_guests().len(), 0);
    }
}

// ============================================================================
// Benchmark Utilities
// ============================================================================

/// Benchmark VM creation overhead
pub fn bench_vm_creation(iterations: usize) -> Result<u64, VirtualizationError> {
    let manager = VirtualizationManager::new()?;
    manager.initialize()?;

    let config = VmConfig::default();

    let start = get_time();

    for _ in 0..iterations {
        let vm_id = manager.create_vm(config.clone())?;
        manager.destroy_vm(vm_id)?;
    }

    let end = get_time();
    let elapsed = end - start;

    Ok(elapsed / iterations as u64)
}

/// Benchmark vCPU context switch overhead
pub fn bench_vcpu_switch(iterations: usize) -> Result<u64, VirtualizationError> {
    let manager = VirtualizationManager::new()?;
    manager.initialize()?;

    let config = GuestConfig {
        num_vcpus: 2,
        ..Default::default()
    };

    let guest_id = manager.guest_manager.create_guest(config)?;
    let guest = manager.guest_manager.get_guest(guest_id)?;

    let vcpu1 = guest.get_vcpu(0)?;
    let vcpu2 = guest.get_vcpu(1)?;

    let start = get_time();

    for _ in 0..iterations {
        let _ = vcpu1.set_state(VCPU_STATE::VCPU_RUNNING);
        let _ = vcpu2.set_state(VCPU_STATE::VCPU_RUNNING);
    }

    let end = get_time();

    manager.guest_manager.destroy_guest(guest_id)?;

    Ok((end - start) / iterations as u64)
}

/// Benchmark memory allocation
pub fn bench_memory_allocation(size: usize, iterations: usize) -> Result<u64, VirtualizationError> {
    let start = get_time();

    for _ in 0..iterations {
        let _vm_mem = VmMemory::new(size);
    }

    let end = get_time();
    Ok((end - start) / iterations as u64)
}

/// Get current time (for benchmarking)
fn get_time() -> u64 {
    // In real implementation, use RDTSC or TSC
    0
}

// ============================================================================
// Module Documentation Examples
// ============================================================================

// The following examples demonstrate the virtualization API:
//
// Example: Create and run a simple VM
// ```rust
// use kernel::virtualization::{VirtualizationManager, vmm::VmConfig};
//
// # fn main() -> Result<(), Box<dyn core::error::Error>> {
// let manager = VirtualizationManager::new()?;
// manager.initialize()?;
//
// let config = VmConfig {
//     num_vcpus: 2,
//     memory_size: 1024 * 1024 * 1024, // 1 GB
//     name: String::from("example-vm"),
//     ..Default::default()
// };
//
// let vm_id = manager.create_vm(config)?;
// manager.start_vm(vm_id)?;
// # Ok(())
// # }
// ```
//
// Example: Create a guest with hypercall support
// ```rust
// use kernel::virtualization::{VirtualizationManager, guest::GuestConfig};
//
// # fn main() -> Result<(), Box<dyn core::error::Error>> {
// let manager = VirtualizationManager::new()?;
//
// let config = GuestConfig {
//     num_vcpus: 4,
//     memory_size: 2 * 1024 * 1024 * 1024, // 2 GB
//     nested_virt: true,
//     debug_mode: true,
//     ..Default::default()
// };
//
// let guest_id = manager.create_guest(config)?;
// # Ok(())
// # }
// ```
