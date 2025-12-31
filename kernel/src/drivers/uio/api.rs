//! Userspace API for UIO devices
//!
//! This module provides the character device interface and ioctl operations
//! for userspace applications to interact with UIO devices.

use crate::drivers::uio::{
    device::UioDevice,
    memory::UioMemRegionType,
    uio::get_uio_driver,
    UioError, UioResult, UIO_MAX_REGIONS,
};
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

/// UIO device file operations
#[derive(Debug)]
pub struct UioDeviceFile {
    /// Device minor number
    minor: u32,
    /// Device reference
    device: UioDevice,
    /// Open flags
    flags: u32,
    /// Event counter for interrupts
    event_count: AtomicU64,
    /// File position
    position: u64,
}

impl UioDeviceFile {
    /// Create a new UIO device file
    pub fn new(device: UioDevice, flags: u32) -> Self {
        Self {
            minor: device.minor(),
            device,
            flags,
            event_count: AtomicU64::new(0),
            position: 0,
        }
    }

    /// Read interrupt events (blocking)
    ///
    /// Returns the number of interrupts that have occurred.
    /// This call blocks until at least one interrupt occurs.
    pub fn read(&self, buf: &mut [u8]) -> UioResult<usize> {
        let event_count = self.event_count.load(Ordering::Acquire);

        // Wait for interrupt if count is zero
        if event_count == 0 {
            self.device.wait_for_interrupt()?;
        }

        // Return event count
        let count = self.event_count.load(Ordering::Acquire);
        let bytes = buf.len().min(core::mem::size_of::<u64>());

        unsafe {
            let ptr = buf.as_ptr() as *mut u64;
            *ptr = count;
        }

        Ok(bytes)
    }

    /// Write to device (typically used to acknowledge interrupts)
    pub fn write(&mut self, _buf: &[u8]) -> UioResult<usize> {
        // Acknowledge interrupt
        self.device.ack_interrupt()?;
        Ok(_buf.len())
    }

    /// Map memory region to userspace
    pub fn mmap(&self, region_index: usize, size: usize) -> UioResult<usize> {
        if region_index >= UIO_MAX_REGIONS {
            return Err(UioError::InvalidRegion);
        }

        let region = self.device.get_region(region_index)?;
        if size > region.size() {
            return Err(UioError::InvalidParam);
        }

        // Return physical address for mapping
        Ok(region.phys_addr() as usize)
    }

    /// Perform ioctl operation
    pub fn ioctl(&mut self, cmd: u32, arg: usize) -> UioResult<u64> {
        match UioIoctl::from_command(cmd) {
            Some(UioIoctl::GetInfo) => self.get_info(arg),
            Some(UioIoctl::MapRegion) => self.map_region_ioctl(arg),
            Some(UioIoctl::UnmapRegion) => self.unmap_region_ioctl(arg),
            Some(UioIoctl::GetRegionInfo) => self.get_region_info(arg),
            Some(UioIoctl::EnableInterrupt) => self.enable_interrupt(arg),
            Some(UioIoctl::DisableInterrupt) => self.disable_interrupt(),
            Some(UioIoctl::GetEventCount) => self.get_event_count(),
            None => Err(UioError::InvalidOperation),
        }
    }

    /// Get device information
    fn get_info(&self, arg: usize) -> UioResult<u64> {
        let info = self.device.info();
        // Copy info to userspace (implementation depends on memory model)
        Ok(info.name.len() as u64)
    }

    /// Map memory region via ioctl
    fn map_region_ioctl(&self, arg: usize) -> UioResult<u64> {
        let region_index = arg;
        let region = self.device.get_region(region_index)?;
        Ok(region.phys_addr() as u64)
    }

    /// Unmap memory region
    fn unmap_region_ioctl(&mut self, arg: usize) -> UioResult<u64> {
        let region_index = arg;
        self.device.unmap_region(region_index)?;
        Ok(0)
    }

    /// Get region information
    fn get_region_info(&self, arg: usize) -> UioResult<u64> {
        let region_index = arg;
        let region = self.device.get_region(region_index)?;
        Ok(region.phys_addr() as u64)
    }

    /// Enable interrupt
    fn enable_interrupt(&mut self, _arg: usize) -> UioResult<u64> {
        self.device.enable_interrupt()?;
        Ok(0)
    }

    /// Disable interrupt
    fn disable_interrupt(&mut self) -> UioResult<u64> {
        self.device.disable_interrupt()?;
        Ok(0)
    }

    /// Get event counter
    fn get_event_count(&self) -> UioResult<u64> {
        Ok(self.event_count.load(Ordering::Acquire))
    }

    /// Increment event counter (called by interrupt handler)
    pub fn increment_event_count(&self) {
        self.event_count.fetch_add(1, Ordering::Release);
    }

    /// Get device minor number
    pub fn minor(&self) -> u32 {
        self.minor
    }

    /// Get device reference
    pub fn device(&self) -> &UioDevice {
        &self.device
    }
}

/// UIO ioctl commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UioIoctl {
    /// Get device information
    GetInfo = 0x0001,

    /// Map memory region
    MapRegion = 0x0002,

    /// Unmap memory region
    UnmapRegion = 0x0003,

    /// Get memory region info
    GetRegionInfo = 0x0004,

    /// Enable interrupt
    EnableInterrupt = 0x0005,

    /// Disable interrupt
    DisableInterrupt = 0x0006,

    /// Get interrupt event count
    GetEventCount = 0x0007,
}

impl UioIoctl {
    /// Convert ioctl command number to enum
    pub fn from_command(cmd: u32) -> Option<Self> {
        match cmd {
            0x0001 => Some(Self::GetInfo),
            0x0002 => Some(Self::MapRegion),
            0x0003 => Some(Self::UnmapRegion),
            0x0004 => Some(Self::GetRegionInfo),
            0x0005 => Some(Self::EnableInterrupt),
            0x0006 => Some(Self::DisableInterrupt),
            0x0007 => Some(Self::GetEventCount),
            _ => None,
        }
    }

    /// Get ioctl command number
    pub fn as_command(&self) -> u32 {
        *self as u32
    }
}

/// UIO file operations trait
pub trait UioFileOperations {
    /// Open device file
    fn open(minor: u32, flags: u32) -> UioResult<Box<UioDeviceFile>>;

    /// Close device file
    fn close(file: &mut UioDeviceFile) -> UioResult<()>;

    /// Read from device
    fn read(file: &UioDeviceFile, buf: &mut [u8]) -> UioResult<usize>;

    /// Write to device
    fn write(file: &mut UioDeviceFile, buf: &[u8]) -> UioResult<usize>;

    /// Memory map operation
    fn mmap(file: &UioDeviceFile, region: usize, size: usize) -> UioResult<usize>;

    /// Ioctl operation
    fn ioctl(file: &mut UioDeviceFile, cmd: u32, arg: usize) -> UioResult<u64>;

    /// Poll for events
    fn poll(file: &UioDeviceFile, events: u32) -> UioResult<u32>;
}

/// Default UIO file operations implementation
pub struct DefaultUioFileOps;

impl UioFileOperations for DefaultUioFileOps {
    fn open(minor: u32, flags: u32) -> UioResult<Box<UioDeviceFile>> {
        // Look up device by minor number
        use crate::drivers::uio::uio::get_uio_driver;

        let driver = get_uio_driver().ok_or(UioError::DeviceNotFound)?;

        let device = driver.lookup_device_by_minor(minor)?;

        // Create device file
        let file = Box::new(UioDeviceFile::new(device, flags));

        Ok(file)
    }

    fn close(_file: &mut UioDeviceFile) -> UioResult<()> {
        // Cleanup if needed
        Ok(())
    }

    fn read(file: &UioDeviceFile, buf: &mut [u8]) -> UioResult<usize> {
        file.read(buf)
    }

    fn write(file: &mut UioDeviceFile, buf: &[u8]) -> UioResult<usize> {
        file.write(buf)
    }

    fn mmap(file: &UioDeviceFile, region: usize, size: usize) -> UioResult<usize> {
        file.mmap(region, size)
    }

    fn ioctl(file: &mut UioDeviceFile, cmd: u32, arg: usize) -> UioResult<u64> {
        file.ioctl(cmd, arg)
    }

    fn poll(file: &UioDeviceFile, _events: u32) -> UioResult<u32> {
        // Check if interrupt events are available
        let event_count = file.event_count.load(Ordering::Acquire);
        if event_count > 0 {
            Ok(0x001) // POLLIN
        } else {
            Ok(0)
        }
    }
}

/// UIO device information for userspace
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UioDeviceInfo {
    /// Device name
    pub name: [u8; 32],
    /// Device version
    pub version: [u8; 32],
    /// Number of memory regions
    pub num_regions: u8,
    /// IRQ number
    pub irq: u32,
    /// Device flags
    pub flags: u32,
}

impl UioDeviceInfo {
    /// Create empty device info
    pub fn new() -> Self {
        Self {
            name: [0; 32],
            version: [0; 32],
            num_regions: 0,
            irq: 0,
            flags: 0,
        }
    }
}

impl Default for UioDeviceInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// UIO memory region information for userspace
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UioRegionInfo {
    /// Memory region physical address
    pub phys_addr: u64,
    /// Memory region size
    pub size: u64,
    /// Region type (MMIO, Port I/O, etc.)
    pub region_type: UioMemRegionType,
    /// Region flags (read/write permissions)
    pub flags: u32,
    /// Region name
    pub name: [u8; 32],
}

impl UioRegionInfo {
    /// Create empty region info
    pub fn new() -> Self {
        Self {
            phys_addr: 0,
            size: 0,
            region_type: UioMemRegionType::Mmio,
            flags: 0,
            name: [0; 32],
        }
    }
}

impl Default for UioRegionInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ioctl_from_command() {
        assert_eq!(UioIoctl::from_command(0x0001), Some(UioIoctl::GetInfo));
        assert_eq!(UioIoctl::from_command(0xFFFF), None);
    }

    #[test]
    fn test_ioctl_as_command() {
        assert_eq!(UioIoctl::GetInfo.as_command(), 0x0001);
        assert_eq!(UioIoctl::MapRegion.as_command(), 0x0002);
    }

    #[test]
    fn test_device_info_new() {
        let info = UioDeviceInfo::new();
        assert_eq!(info.name.len(), 32);
        assert_eq!(info.version.len(), 32);
    }

    #[test]
    fn test_region_info_new() {
        let info = UioRegionInfo::new();
        assert_eq!(info.phys_addr, 0);
        assert_eq!(info.size, 0);
    }
}
