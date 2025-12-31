//! Userspace I/O (UIO) Driver Framework
//!
//! The UIO framework provides a safe interface for implementing device drivers in userspace.
//! It allows userspace applications to handle device interrupts and access device memory
//! through memory-mapped regions.
//!
//! # Architecture
//!
//! The UIO framework consists of several key components:
//!
//! - **Device Management**: Registration, lifecycle, and enumeration of UIO devices
//! - **Memory Mapping**: Safe mapping of device MMIO regions to userspace
//! - **Interrupt Handling**: Delivery of hardware interrupts to userspace via eventfd
//! - **Sysfs Interface**: Runtime configuration and device information
//! - **File Operations**: Character device interface for userspace interaction
//!
//! # Core Components
//!
//! - [`UioDevice`]: Represents a userspace I/O device with memory regions and interrupts
//! - [`UioDriver`]: Manages UIO device registration and driver operations
//! - [`UioInterrupt`]: Handles interrupt delivery to userspace applications
//! - [`UioMemRegion`]: Describes memory-mappable device regions
//!
//! # Device Operation Flow
//!
//! 1. **Device Registration**: Kernel driver registers UIO device with subsystem
//! 2. **Userspace Open**: Application opens `/dev/uioX` device file
//! 3. **Memory Mapping**: Application mmaps device memory regions via mmap()
//! 4. **Interrupt Wait**: Application blocks on read() or poll() for interrupts
//! 5. **Interrupt Handling**: Kernel delivers interrupt via eventfd mechanism
//! 6. **Device Access**: Application reads/writes device registers directly
//!
//! # Memory Regions
//!
//! UIO devices support multiple memory regions with different types:
//!
//! - **MMIO**: Memory-mapped I/O regions (registers, device memory)
//! - **Port I/O**: Legacy I/O port access (x86 specific)
//! - **Custom**: Application-defined region types
//!
//! Each region has:
//! - Physical address
//! - Size
//! - Access permissions (read/write)
//! - Region type and name
//!
//! # Interrupt Handling
//!
//! UIO provides two interrupt notification mechanisms:
//!
//! 1. **Read Blocking**: `read()` on `/dev/uioX` blocks until interrupt occurs
//! 2. **Eventfd Integration**: Eventfd for notification via poll/select/epoll
//!
//! Interrupt flow:
//!
//! ```text
//! Hardware Interrupt → Kernel Handler → UioInterrupt → Eventfd → Userspace
//! ```
//!
//! # Example Usage
//!
//! ```no_run
//! use kernel::drivers::uio::{UioDevice, UioMemRegion, UioDriver};
//!
//! // Create a UIO device
//! let mut device = UioDevice::new("my_device", 0);
//!
//! // Add memory regions
//! device.add_region(UioMemRegion::mmio(0xF0000000, 0x1000));
//! device.add_region(UioMemRegion::mmio(0xF0001000, 0x2000));
//!
//! // Register interrupt handler
//! device.register_interrupt(42, |irq| {
//!     println!("Interrupt {}", irq);
//!     Ok(())
//! })?;
//!
//! // Register device with subsystem
//! UioDriver::register_device(device)?;
//! ```
//!
//! # Security Considerations
//!
//! - Memory regions are mapped with user-specified permissions
//! - Access control enforced via file permissions on /dev/uioX
//! - Interrupt handlers must validate device state
//! - Privileged operations require appropriate capabilities
//!
//! # Integration
//!
//! The UIO framework integrates with:
//! - Device model: `/sys/class/uio/uioX/`
//! - Character devices: `/dev/uioX`
//! - Device core: `struct device` integration
//! - PCI platform: UIO PCI driver template
//!
//! # References
//!
//! - Linux UIO documentation: Documentation/driver-api/uio-howto.rst
//! - Userspace I/O framework design patterns

pub mod api;
pub mod device;
pub mod examples;
pub mod interrupt;
pub mod memory;
pub mod sysfs;
#[cfg(test)]
mod tests;
pub mod uio;

// Re-exports for convenience
pub use api::{UioDeviceFile, UioDeviceInfo, UioIoctl, UioFileOperations, UioRegionInfo};
pub use device::{UioDevice, UioDeviceRegistry, UioInfo};
pub use examples::{
    CustomConfig, CustomUioDevice, SimpleUioDriver, UioDeviceFactory, UioGpioDriver, UioNetDriver,
    UioPciDriver,
};
pub use interrupt::{UioInterrupt, UioInterruptAffinity, UioInterruptHandler, UioInterruptStats};
pub use memory::{UioMemPermissions, UioMemRegion, UioMemRegionType, UioMemRegionManager};
pub use sysfs::{UioDeviceStats, UioSysfs, UioSysfsAttrs};
pub use uio::{get_uio_driver, init_uio_driver, shutdown_uio_driver, UioDriver, UioDriverInfo};

/// UIO subsystem version
pub const UIO_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of memory regions per device
pub const UIO_MAX_REGIONS: usize = 8;

/// Maximum number of devices
pub const UIO_MAX_DEVICES: usize = 256;

/// UIO device name prefix
pub const UIO_DEVICE_PREFIX: &str = "uio";

/// UIO class name in sysfs
pub const UIO_CLASS_NAME: &str = "uio";

/// UIO device class path
pub const UIO_CLASS_PATH: &str = "/sys/class/uio";

/// UIO error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UioError {
    /// Device not found
    DeviceNotFound,
    /// Invalid device number
    InvalidDevice,
    /// Memory region already mapped
    RegionMapped,
    /// Invalid memory region
    InvalidRegion,
    /// Interrupt registration failed
    InterruptError,
    /// Invalid operation
    InvalidOperation,
    /// Permission denied
    PermissionDenied,
    /// Resource allocation failed
    NoMemory,
    /// Device busy
    DeviceBusy,
    /// Invalid parameter
    InvalidParam,
}

impl UioError {
    /// Convert error code to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeviceNotFound => "UIO device not found",
            Self::InvalidDevice => "Invalid UIO device number",
            Self::RegionMapped => "Memory region already mapped",
            Self::InvalidRegion => "Invalid memory region",
            Self::InterruptError => "Interrupt registration failed",
            Self::InvalidOperation => "Invalid operation",
            Self::PermissionDenied => "Permission denied",
            Self::NoMemory => "Memory allocation failed",
            Self::DeviceBusy => "Device busy",
            Self::InvalidParam => "Invalid parameter",
        }
    }
}

impl core::fmt::Display for UioError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UioError {}

/// Result type for UIO operations
pub type UioResult<T> = Result<T, UioError>;
