//! Boot protocol abstraction layer
//!
//! This module provides a unified interface for different boot protocols
//! (UEFI, BIOS/Multiboot2, etc.) allowing the bootloader to work with
//! various firmware implementations.

use crate::error::{BootError, Result};
use crate::memory::BootMemoryManager;
use core::ptr;

#[cfg(feature = "uefi_support")]
pub mod uefi;
#[cfg(feature = "bios_support")]
pub mod bios;
#[cfg(feature = "multiboot2_support")]
pub mod multiboot2;

/// Boot protocol type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    Unknown,
    UEFI,
    BIOS,
    Multiboot2,
}

impl ProtocolType {
    /// Get the name of the protocol type
    pub fn name(self) -> &'static str {
        match self {
            ProtocolType::Unknown => "Unknown",
            ProtocolType::UEFI => "UEFI",
            ProtocolType::BIOS => "BIOS",
            ProtocolType::Multiboot2 => "Multiboot2",
        }
    }

    /// Check if this protocol type is supported by the current build
    pub fn is_supported(self) -> bool {
        match self {
            ProtocolType::UEFI => cfg!(feature = "uefi_support"),
            ProtocolType::BIOS => cfg!(feature = "bios_support"),
            ProtocolType::Multiboot2 => cfg!(feature = "multiboot2_support"),
            ProtocolType::Unknown => false,
        }
    }
}

/// Boot protocol trait - all boot protocols must implement this
pub trait BootProtocol {
    /// Get the protocol type
    fn protocol_type(&self) -> ProtocolType;

    /// Check if this protocol is available on the current system
    fn detect(&self) -> bool;

    /// Initialize the protocol
    fn initialize(&mut self) -> Result<()>;

    /// Get memory map from the firmware
    fn get_memory_map(&self) -> Result<MemoryMap>;

    /// Get boot information from the firmware
    fn get_boot_info(&self) -> Result<BootInfo>;

    /// Load a kernel image
    fn load_kernel(&mut self, path: &str) -> Result<KernelImage>;

    /// Exit boot services (prepare for kernel boot)
    fn exit_boot_services(&mut self) -> Result<()>;

    /// Get framebuffer information (if available)
    fn get_framebuffer_info(&self) -> Result<Option<FramebufferInfo>> {
        Ok(None)
    }

    /// Get ACPI RSDP address (if available)
    fn get_acpi_rsdp(&self) -> Result<Option<usize>> {
        Ok(None)
    }

    /// Get device tree blob address (if available)
    fn get_device_tree(&self) -> Result<Option<usize>> {
        Ok(None)
    }

    /// Get command line arguments
    fn get_command_line(&self) -> Result<Option<String>> {
        Ok(None)
    }

    /// Reboot the system
    fn reboot(&self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("Reboot"))
    }

    /// Shutdown the system
    fn shutdown(&self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("Shutdown"))
    }
}

/// Memory map entry
#[derive(Debug, Clone)]
pub struct MemoryMapEntry {
    /// Physical base address
    pub base: usize,
    /// Size in bytes
    pub size: usize,
    /// Memory type
    pub mem_type: MemoryType,
    /// Whether this region is available for use
    pub is_available: bool,
}

/// Memory types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Reserved,
    Usable,
    ACPIReclaimable,
    ACPIONVS,
    BadMemory,
    BootloaderCode,
    BootloaderData,
    RuntimeCode,
    RuntimeData,
    ConventionMemory,
    UnconventionalMemory,
}

/// Memory map
#[derive(Debug, Clone)]
pub struct MemoryMap {
    /// Entries in the memory map
    pub entries: Vec<MemoryMapEntry>,
    /// Total memory size
    pub total_memory: usize,
    /// Available memory size
    pub available_memory: usize,
}

impl MemoryMap {
    /// Create a new empty memory map
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            total_memory: 0,
            available_memory: 0,
        }
    }

    /// Add an entry to the memory map
    pub fn add_entry(&mut self, entry: MemoryMapEntry) {
        self.total_memory += entry.size;
        if entry.is_available {
            self.available_memory += entry.size;
        }
        self.entries.push(entry);
    }

    /// Find available memory regions that can fit the requested size
    pub fn find_available_regions(&self, size: usize, alignment: usize) -> Vec<usize> {
        let mut regions = Vec::new();

        for entry in &self.entries {
            if entry.is_available && entry.size >= size {
                let aligned_base = (entry.base + alignment - 1) & !(alignment - 1);
                let aligned_end = aligned_base + size;

                if aligned_end <= entry.base + entry.size {
                    regions.push(aligned_base);
                }
            }
        }

        regions
    }

    /// Mark a region as used
    pub fn mark_region_used(&mut self, base: usize, size: usize) {
        for entry in &mut self.entries {
            if entry.is_available && entry.base <= base && (entry.base + entry.size) >= base + size {
                entry.is_available = false;
                self.available_memory -= size;
                break;
            }
        }
    }
}

/// Boot information passed to the kernel
#[derive(Debug, Clone)]
pub struct BootInfo {
    /// Boot protocol type
    pub protocol_type: ProtocolType,
    /// Memory map
    pub memory_map: MemoryMap,
    /// Framebuffer information (if available)
    pub framebuffer: Option<FramebufferInfo>,
    /// ACPI RSDP address (if available)
    pub acpi_rsdp: Option<usize>,
    /// Device tree blob address (if available)
    pub device_tree: Option<usize>,
    /// Command line arguments
    pub command_line: Option<String>,
    /// Boot timestamp (in nanoseconds)
    pub boot_timestamp: u64,
    /// Boot loader version
    pub bootloader_version: &'static str,
}

impl BootInfo {
    /// Create new boot information
    pub fn new(protocol_type: ProtocolType) -> Self {
        Self {
            protocol_type,
            memory_map: MemoryMap::new(),
            framebuffer: None,
            acpi_rsdp: None,
            device_tree: None,
            command_line: None,
            boot_timestamp: 0, // Will be filled in by bootloader
            bootloader_version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Framebuffer information
#[derive(Debug, Clone)]
pub struct FramebufferInfo {
    /// Physical base address
    pub address: usize,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bytes per pixel
    pub bytes_per_pixel: u32,
    /// Stride (bytes per row)
    pub stride: u32,
    /// Pixel format
    pub pixel_format: PixelFormat,
}

/// Pixel formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    RGB,
    BGR,
    /// 8-bit red, 8-bit green, 8-bit blue, 8-bit reserved
    RGBReserved,
    /// 8-bit blue, 8-bit green, 8-bit red, 8-bit reserved
    BGRReserved,
}

/// Kernel image information
#[derive(Debug)]
pub struct KernelImage {
    /// Physical load address
    pub load_address: usize,
    /// Entry point address
    pub entry_point: usize,
    /// Size in bytes
    pub size: usize,
    /// Kernel image data
    pub data: Vec<u8>,
}

impl KernelImage {
    /// Create a new kernel image
    pub fn new(load_address: usize, entry_point: usize, data: Vec<u8>) -> Self {
        Self {
            load_address,
            entry_point,
            size: data.len(),
            data,
        }
    }

    /// Validate the kernel image
    pub fn validate(&self) -> Result<()> {
        if self.data.is_empty() {
            return Err(BootError::KernelNotFound);
        }

        if self.entry_point < self.load_address {
            return Err(BootError::InvalidKernelFormat);
        }

        if self.entry_point >= self.load_address + self.size {
            return Err(BootError::InvalidKernelFormat);
        }

        Ok(())
    }
}

/// Boot protocol manager
pub struct ProtocolManager {
    protocols: Vec<Box<dyn BootProtocol>>,
    active_protocol: Option<Box<dyn BootProtocol>>,
}

impl ProtocolManager {
    /// Create a new protocol manager
    pub fn new() -> Self {
        let mut protocols: Vec<Box<dyn BootProtocol>> = Vec::new();

        // Add supported protocols
        #[cfg(feature = "uefi_support")]
        protocols.push(Box::new(uefi::UefiProtocol::new()));

        #[cfg(feature = "bios_support")]
        protocols.push(Box::new(bios::BiosProtocol::new()));

        #[cfg(feature = "multiboot2_support")]
        protocols.push(Box::new(multiboot2::Multiboot2Protocol::new()));

        Self {
            protocols,
            active_protocol: None,
        }
    }

    /// Detect and initialize the appropriate boot protocol
    pub fn detect_and_initialize(&mut self) -> Result<ProtocolType> {
        for protocol in &mut self.protocols {
            if protocol.detect() {
                protocol.initialize()?;
                let protocol_type = protocol.protocol_type();

                println!("Detected and initialized boot protocol: {}", protocol_type.name());

                // Note: In a real implementation, we'd need to handle the move properly
                // For now, we'll just remember the protocol type
                return Ok(protocol_type);
            }
        }

        Err(BootError::ProtocolDetectionFailed)
    }

    /// Get the currently active protocol
    pub fn get_active_protocol(&self) -> Result<&dyn BootProtocol> {
        self.active_protocol.as_ref().map(|p| p.as_ref()).ok_or(BootError::NotInitialized)
    }

    /// Get the currently active protocol (mutable)
    pub fn get_active_protocol_mut(&mut self) -> Result<&mut dyn BootProtocol> {
        self.active_protocol.as_mut().map(|p| p.as_mut()).ok_or(BootError::NotInitialized)
    }

    /// Get memory map from active protocol
    pub fn get_memory_map(&self) -> Result<MemoryMap> {
        let protocol = self.get_active_protocol()?;
        protocol.get_memory_map()
    }

    /// Get boot information from active protocol
    pub fn get_boot_info(&self) -> Result<BootInfo> {
        let protocol = self.get_active_protocol()?;

        let mut boot_info = BootInfo::new(protocol.protocol_type());
        boot_info.memory_map = protocol.get_memory_map()?;
        boot_info.framebuffer = protocol.get_framebuffer_info()?;
        boot_info.acpi_rsdp = protocol.get_acpi_rsdp()?;
        boot_info.device_tree = protocol.get_device_tree()?;
        boot_info.command_line = protocol.get_command_line()?;

        Ok(boot_info)
    }

    /// Load kernel using active protocol
    pub fn load_kernel(&mut self, path: &str) -> Result<KernelImage> {
        let protocol = self.get_active_protocol_mut()?;
        protocol.load_kernel(path)
    }

    /// Exit boot services using active protocol
    pub fn exit_boot_services(&mut self) -> Result<()> {
        let protocol = self.get_active_protocol_mut()?;
        protocol.exit_boot_services()
    }
}

/// Simple timestamp function (would be replaced with proper timer)
pub fn get_timestamp() -> u64 {
    // This is a placeholder - in a real implementation we'd get this from hardware
    static mut TIMESTAMP: u64 = 0;
    unsafe {
        TIMESTAMP += 1;
        TIMESTAMP
    }
}

/// Alignment helper
pub fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

/// Alignment helper
pub fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

/// Check if a value is aligned
pub fn is_aligned(value: usize, alignment: usize) -> bool {
    (value & (alignment - 1)) == 0
}