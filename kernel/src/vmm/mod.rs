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
//!
//! ## Key Features
//!
//! - VM creation and lifecycle management
//! - vCPU scheduling with priority and affinity
//! - Memory virtualization (EPT/NPT)
//! - I/O virtualization through Virtio
//! - VM exit handling and emulation
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

pub mod hypervisor;
pub mod vcpu_sched;

pub use self::hypervisor::{Hypervisor, VirtualCpu, VirtualMachine, VmConfig, VmExitInfo, VmState};
pub use self::vcpu_sched::{VcpuScheduler, VcpuState, VcpuPriority, VcpuDescriptor};
