//! # Virtio Device Support
//!
//! This module provides support for Virtio devices, which are the standard
//! I/O virtualization framework used in virtual machines. Virtio uses a set
//! of virtual I/O queues (virtqueues) for efficient communication between
//! the guest OS and the hypervisor.
//!
//! ## Modules
//!
//! - **virtio**: Core Virtio framework and device abstractions
//! - **mmio**: MMIO transport layer for Virtio devices

pub mod mmio;
pub mod virtio;

pub use self::virtio::{
    VirtioDevice, VirtioDeviceId, VirtioError, Virtqueue,
};
pub use self::mmio::{VirtioMmioDevice, VirtioMmioTransport};
