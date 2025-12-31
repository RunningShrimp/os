//! Device drivers framework
//!
//! This module provides comprehensive device driver support including:
//! - UIO (Userspace I/O) driver framework for simple devices
//! - VFIO (Virtual Function I/O) for secure device assignment
//!
//! ## UIO Framework
//!
//! The UIO framework enables safe and efficient userspace device drivers:
//! - Direct memory mapping to userspace
//! - Interrupt delivery via eventfd
//! - Character device interface
//! - Sysfs integration
//!
//! ## VFIO Framework
//!
//! The VFIO framework provides secure device assignment:
//! - Full device access to userspace
//! - DMA support
//! - IOMMU integration
//! - Live migration support
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::drivers::uio::{UioDevice, UioMemRegion, UioDriver};
//!
//! // Create a UIO device
//! let mut device = UioDevice::new("my_device", 0);
//!
//! // Add memory regions
//! device.add_region(UioMemRegion::mmio(0xF0000000, 0x1000));
//!
//! // Register with subsystem
//! let mut driver = UioDriver::new();
//! driver.init()?;
//! driver.register_device(device)?;
//! # Ok::<(), kernel::drivers::uio::UioError>(())
//! ```

pub mod uio;
pub mod vfio;

// Re-export commonly used types
pub use uio::{
    UioDevice, UioDriver, UioError, UioInfo, UioMemRegion, UioMemRegionType,
    UioInterrupt, UioInterruptHandler, UioResult,
};

// Re-export platform drivers for backward compatibility
pub use crate::platform::drivers::*;
