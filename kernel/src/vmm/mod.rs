//! # Virtual Machine Monitor (VMM)
//!
//! This module provides the Virtual Machine Monitor functionality for the NOS kernel,
//! enabling hardware-assisted virtualization through Intel VT-x or AMD-V technologies.
//!
//! ## Architecture
//!
//! The VMM is organized into several components:
//!
//! - **hypervisor**: Core VM and vCPU management, VM exit handling
//! - **vcpu_sched**: Virtual CPU scheduling and load balancing
//! - **kvm**: KVM (Kernel-based Virtual Machine) integration
//! - **qemu**: QEMU device model interface and Virtio communication
//! - **migration**: Live migration support with pre-copy and post-copy
//! - **balloon**: Memory ballooning for dynamic memory management
//! - **passthrough**: PCI device passthrough with IOMMU support
//! - **snapshot**: VM snapshot and restore functionality
//!
//! ## Key Features
//!
//! - VM creation and lifecycle management
//! - vCPU scheduling with priority and affinity
//! - Memory virtualization (EPT/NPT)
//! - I/O virtualization through Virtio
//! - VM exit handling and emulation
//! - Live migration with minimal downtime
//! - Dynamic memory ballooning
//! - PCI device passthrough with IOMMU
//! - VM snapshot and restore
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::{Hypervisor, VmConfig};
//!
//! let mut hypervisor = Hypervisor::new();
//! hypervisor.init()?;
//!
//! let config = VmConfig::default();
//! let vm_id = hypervisor.create_vm(&config)?;
//! ```

pub mod balloon;
pub mod hypervisor;
pub mod kvm;
pub mod migration;
pub mod passthrough;
pub mod qemu;
pub mod snapshot;
pub mod vcpu_sched;

// Re-exports from hypervisor module
pub use self::hypervisor::{Hypervisor, VirtualCpu, VirtualMachine, VmConfig, VmExitInfo, VmState};

// Re-exports from vcpu_sched module
pub use self::vcpu_sched::{VcpuScheduler, VcpuState, VcpuPriority, VcpuDescriptor};

// Re-exports from kvm module
pub use self::kvm::{
    Kvm, KvmCapabilities, KvmExit, KvmMsrList, KvmRegs, KvmSregs, KvmVcpu, KvmVm, MemorySlot,
};

// Re-exports from qemu module
pub use self::qemu::{
    HotplugInfo, HotplugOperation, QemuError, QemuInterface, QmpClient, VirtioConfig,
    VirtioDeviceId, VirtioQueueConfig,
};

// Re-exports from migration module
pub use self::migration::{
    DeviceState, DirtyPageBitmap, LiveMigration, MigrationConfig, MigrationError,
    MigrationPhase, MigrationStats, MigrationStrategy,
};

// Re-exports from balloon module
pub use self::balloon::{
    BalloonConfig, BalloonDriver, BalloonError, BalloonOp, BalloonStats, PageRequest,
    PressureEvent, PressureAction, ReclaimStrategy,
};

// Re-exports from passthrough module
pub use self::passthrough::{
    AssignmentFlags, BarInfo, DeviceAssignment, IommuDomain, IommuType, MsiConfig,
    PassthroughError, PassthroughManager, PciDevice, PciId, VirtualFunction,
};

// Re-exports from snapshot module
pub use self::snapshot::{
    CompressionType, DeviceStateEntry, PageEntry, SnapshotConfig, SnapshotData,
    SnapshotError, SnapshotHeader, SnapshotManager, SnapshotMetadata, SnapshotState,
    SnapshotStats, SnapshotType, VcpuState, VmState,
};
