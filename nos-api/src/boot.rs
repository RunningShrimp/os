//! Boot parameters and structures
//!
//! This module defines the unified boot parameter structures used for
//! communication between the bootloader and the kernel.

#![allow(dead_code)]

/// Boot protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootProtocolType {
    Unknown = 0,
    Direct = 1,
    UEFI = 2,
    BIOS = 3,
    Multiboot2 = 4,
    Multiboot3 = 5,
}

/// Memory types passed from bootloader
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryType {
    Reserved = 0,
    Usable = 1,
    ACPIReclaimable = 2,
    ACPIONVS = 3,
    BadMemory = 4,
    BootloaderCode = 5,
    BootloaderData = 6,
    RuntimeCode = 7,
    RuntimeData = 8,
    ConventionMemory = 9,
    UnconventionalMemory = 10,
}

/// Memory map entry from bootloader
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryMapEntry {
    /// Physical base address
    pub base: u64,
    /// Size in bytes
    pub size: u64,
    /// Memory type
    pub mem_type: u32, // Using u32 for C compatibility
    /// Whether this region is available
    pub is_available: u32, // Using u32 for C compatibility (0=false, 1=true)
}

/// Memory map from bootloader
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryMap {
    /// Number of entries
    pub entry_count: u32,
    /// Pointer to entries array
    pub entries: u64, // Pointer as u64 for cross-architecture compatibility
}

/// Framebuffer information from bootloader
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FramebufferInfo {
    /// Physical base address
    pub address: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bytes per pixel
    pub bytes_per_pixel: u32,
    /// Stride (bytes per row)
    pub stride: u32,
    /// Pixel format (0=RGB, 1=BGR, etc.)
    pub pixel_format: u32,
}

/// Unified boot parameters structure
///
/// This structure is used for communication between the bootloader and kernel.
/// It must be #[repr(C)] to ensure proper memory layout across the boundary.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootParameters {
    /// Magic number to identify valid boot parameters
    /// Must be BOOT_PARAMETERS_MAGIC
    pub magic: u64,
    
    /// Version of the boot parameter structure
    /// Increment when structure layout changes
    pub version: u32,
    
    /// Architecture type (0=x86_64, 1=aarch64, 2=riscv64)
    pub architecture: u32,
    
    /// Boot protocol type (see BootProtocolType)
    pub boot_protocol: u32,
    
    /// Memory map information
    pub memory_map: MemoryMap,
    
    /// Framebuffer information (if available)
    /// Address 0 means no framebuffer
    pub framebuffer: FramebufferInfo,
    
    /// ACPI RSDP address (if available)
    /// Address 0 means no ACPI
    pub acpi_rsdp: u64,
    
    /// Device tree blob address (if available)
    /// Address 0 means no device tree
    pub device_tree: u64,
    
    /// Command line string address (if available)
    /// Address 0 means no command line
    pub command_line: u64,
    
    /// Boot timestamp (nanoseconds since boot)
    pub timestamp: u64,
    
    /// Bootloader version
    pub bootloader_version: u32,
    
    /// ASLR offset for address space layout randomization
    /// This is a page-aligned random offset applied to base addresses
    /// Set to 0 if ASLR is disabled
    pub aslr_offset: u64,
    
    /// Reserved fields for future extensions
    pub reserved: [u64; 7], // Reduced from 8 to make room for aslr_offset
}

impl BootParameters {
    /// Magic number for boot parameters
    /// "NOS_BOOT" in ASCII
    pub const MAGIC: u64 = 0x4E4F5342_4F4F5452;
    
    /// Current version of the boot parameter structure
    pub const VERSION: u32 = 1;
    
    /// Create a new empty boot parameters structure
    pub const fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            architecture: 0,
            boot_protocol: 0,
            memory_map: MemoryMap {
                entry_count: 0,
                entries: 0,
            },
            framebuffer: FramebufferInfo {
                address: 0,
                width: 0,
                height: 0,
                bytes_per_pixel: 0,
                stride: 0,
                pixel_format: 0,
            },
            acpi_rsdp: 0,
            device_tree: 0,
            command_line: 0,
            timestamp: 0,
            bootloader_version: 0,
            aslr_offset: 0,
            reserved: [0; 7],
        }
    }
    
    /// Check if boot parameters are valid
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.version >= 1
    }
    
    /// Get boot protocol type as enum
    pub fn protocol_type(&self) -> BootProtocolType {
        match self.boot_protocol {
            1 => BootProtocolType::Direct,
            2 => BootProtocolType::UEFI,
            3 => BootProtocolType::BIOS,
            4 => BootProtocolType::Multiboot2,
            5 => BootProtocolType::Multiboot3,
            _ => BootProtocolType::Unknown,
        }
    }
    
    /// Check if framebuffer is available
    pub fn has_framebuffer(&self) -> bool {
        self.framebuffer.address != 0
    }
    
    /// Check if ACPI is available
    pub fn has_acpi(&self) -> bool {
        self.acpi_rsdp != 0
    }
    
    /// Check if device tree is available
    pub fn has_device_tree(&self) -> bool {
        self.device_tree != 0
    }
    
    /// Check if command line is available
    pub fn has_command_line(&self) -> bool {
        self.command_line != 0
    }
    
    /// Check version compatibility
    /// Returns true if the version is compatible with the current kernel
    pub fn is_version_compatible(&self) -> bool {
        // Current kernel supports version 1
        // Future versions should check for backward compatibility
        self.version == Self::VERSION || self.version == 1
    }
    
    /// Get architecture name as string
    pub fn architecture_name(&self) -> &'static str {
        match self.architecture {
            0 => "x86_64",
            1 => "aarch64",
            2 => "riscv64",
            _ => "unknown",
        }
    }
    
    /// Validate architecture matches current build
    pub fn validate_architecture(&self) -> bool {
        #[cfg(target_arch = "x86_64")]
        return self.architecture == 0;
        
        #[cfg(target_arch = "aarch64")]
        return self.architecture == 1;
        
        #[cfg(target_arch = "riscv64")]
        return self.architecture == 2;
        
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
        return false;
    }
    
    /// Check if ASLR is enabled (aslr_offset != 0)
    pub fn has_aslr(&self) -> bool {
        self.aslr_offset != 0
    }
    
    /// Get ASLR offset as usize
    pub fn aslr_offset_usize(&self) -> usize {
        self.aslr_offset as usize
    }
}

impl Default for BootParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate boot parameters
///
/// # Arguments
/// * `params` - Pointer to boot parameters structure
///
/// # Returns
/// * `true` if parameters are valid, `false` otherwise
///
/// # Safety
/// * `params` must point to valid memory containing a BootParameters structure
pub unsafe fn validate_boot_parameters(params: *const BootParameters) -> bool {
    if params.is_null() {
        return false;
    }
    
    let params = &*params;
    params.is_valid()
}

