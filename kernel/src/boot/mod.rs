// Boot information handling
//
// This module handles boot information provided by bootloaders,
// supporting both legacy direct boot and modern bootloader interfaces.

extern crate alloc;
use core::ptr;

/// Boot protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProtocolType {
    Unknown = 0,
    Direct = 1,
    UEFI = 2,
    BIOS = 3,
    Multiboot2 = 4,
}

/// Memory types passed from bootloader
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MemoryMapEntry {
    /// Physical base address
    pub base: u64,
    /// Size in bytes
    pub size: u64,
    /// Memory type
    pub mem_type: MemoryType,
    /// Whether this region is available
    pub is_available: bool,
}

/// Memory map from bootloader
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MemoryMap {
    /// Number of entries
    pub entry_count: u32,
    /// Entries array
    pub entries: *const MemoryMapEntry,
}

impl MemoryMap {
    /// Get iterator over entries
    pub fn entries(&self) -> MemoryMapIter {
        MemoryMapIter {
            current: 0,
            count: self.entry_count as usize,
            entries: self.entries,
        }
    }

    /// Get total usable memory size
    pub fn usable_memory(&self) -> u64 {
        self.entries()
            .filter(|entry| entry.is_available && entry.mem_type == MemoryType::Usable)
            .map(|entry| entry.size)
            .sum()
    }
}

/// Iterator over memory map entries
pub struct MemoryMapIter {
    current: usize,
    count: usize,
    entries: *const MemoryMapEntry,
}

impl Iterator for MemoryMapIter {
    type Item = &'static MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.count {
            let entry = unsafe { &*self.entries.add(self.current) };
            self.current += 1;
            Some(entry)
        } else {
            None
        }
    }
}

/// Framebuffer information from bootloader
#[derive(Debug, Clone)]
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

/// Boot parameters passed from bootloader
#[derive(Debug, Clone)]
#[repr(C)]
pub struct BootParameters {
    /// Magic number to identify valid boot parameters
    pub magic: u64,
    /// Version of the boot parameter structure
    pub version: u32,
    /// Architecture type
    pub architecture: u32,
    /// Boot protocol type
    pub boot_protocol: u32,
    /// Memory map information
    pub memory_map: MemoryMap,
    /// Framebuffer information (if available)
    pub framebuffer: Option<FramebufferInfo>,
    /// ACPI RSDP (if available)
    pub acpi_rsdp: Option<u64>,
    /// Device tree blob (if available)
    pub device_tree: Option<u64>,
    /// Command line arguments
    pub command_line: Option<&'static str>,
    /// Boot timestamp (nanoseconds since boot)
    pub timestamp: u64,
    /// Bootloader version
    pub bootloader_version: u32,
    /// Reserved fields
    pub reserved: [u64; 8],
}

impl BootParameters {
    /// Magic number for boot parameters
    pub const MAGIC: u64 = 0x4E4F5342_4F4F5452; // "NOS_BOOT"

    /// Check if boot parameters are valid
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.version >= 1
    }

    /// Get boot protocol type
    pub fn protocol_type(&self) -> BootProtocolType {
        match self.boot_protocol {
            0 => BootProtocolType::Unknown,
            1 => BootProtocolType::Direct,
            2 => BootProtocolType::UEFI,
            3 => BootProtocolType::BIOS,
            4 => BootProtocolType::Multiboot2,
            _ => BootProtocolType::Unknown,
        }
    }

    /// Get architecture name
    pub fn architecture_name(&self) -> &'static str {
        match self.architecture {
            1 => "x86_64",
            2 => "AArch64",
            3 => "RISC-V 64",
            _ => "Unknown",
        }
    }

    /// Check if framebuffer is available
    pub fn has_framebuffer(&self) -> bool {
        self.framebuffer.is_some()
    }

    /// Check if ACPI is available
    pub fn has_acpi(&self) -> bool {
        self.acpi_rsdp.is_some()
    }

    /// Check if device tree is available
    pub fn has_device_tree(&self) -> bool {
        self.device_tree.is_some()
    }

    /// Check if command line is available
    pub fn has_command_line(&self) -> bool {
        self.command_line.is_some()
    }
}

/// Global boot information storage
static mut BOOT_PARAMETERS: Option<BootParameters> = None;
static mut BOOT_INITIALIZED: bool = false;

/// Initialize boot information from bootloader parameters
pub fn init_from_boot_parameters(params: *const BootParameters) {
    unsafe {
        if !BOOT_INITIALIZED {
            BOOT_PARAMETERS = params.as_ref().map(|p| unsafe { core::ptr::read(p as *const BootParameters) });
            BOOT_INITIALIZED = true;
        }
    }
}

/// Check if we were booted by a bootloader
pub fn is_bootloader_boot() -> bool {
    unsafe { BOOT_INITIALIZED }
}

/// Get boot parameters if available
pub fn get_boot_parameters() -> Option<&'static BootParameters> {
    unsafe { BOOT_PARAMETERS.as_ref() }
}

/// Get memory map from boot parameters
pub fn get_memory_map() -> Option<&'static MemoryMap> {
    unsafe { BOOT_PARAMETERS.as_ref().map(|params| &params.memory_map) }
}

/// Get framebuffer information
pub fn get_framebuffer_info() -> Option<&'static FramebufferInfo> {
    unsafe { BOOT_PARAMETERS.as_ref().and_then(|params| params.framebuffer.as_ref()) }
}

/// Get ACPI RSDP address
pub fn get_acpi_rsdp() -> Option<u64> {
    unsafe { BOOT_PARAMETERS.as_ref().and_then(|params| params.acpi_rsdp) }
}

/// Get device tree blob address
pub fn get_device_tree() -> Option<u64> {
    unsafe { BOOT_PARAMETERS.as_ref().and_then(|params| params.device_tree) }
}

/// Get command line arguments
pub fn get_command_line() -> Option<&'static str> {
    unsafe { BOOT_PARAMETERS.as_ref().and_then(|params| params.command_line) }
}

/// Get boot timestamp
pub fn get_boot_timestamp() -> Option<u64> {
    unsafe { BOOT_PARAMETERS.as_ref().map(|params| params.timestamp) }
}

/// Initialize boot information for direct QEMU boot (legacy mode)
pub fn init_direct_boot() {
    // Create minimal boot parameters for direct QEMU boot
    let params = BootParameters {
        magic: BootParameters::MAGIC,
        version: 1,
        architecture: {
            #[cfg(target_arch = "x86_64")]
            { 1 }
            #[cfg(target_arch = "aarch64")]
            { 2 }
            #[cfg(target_arch = "riscv64")]
            { 3 }
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
            { 0 }
        },
        boot_protocol: BootProtocolType::Direct as u32,
        memory_map: MemoryMap {
            entry_count: 0,
            entries: ptr::null(),
        },
        framebuffer: None,
        acpi_rsdp: None,
        device_tree: None,
        command_line: None,
        timestamp: 0,
        bootloader_version: 0,
        reserved: [0; 8],
    };

    init_from_boot_parameters(&params as *const BootParameters);
}

/// Print boot information
pub fn print_boot_info() {
    if let Some(params) = get_boot_parameters() {
        crate::println!("[boot] Boot parameters:");
        crate::println!("[boot]   Magic: {:#x}", params.magic);
        crate::println!("[boot]   Version: {}", params.version);
        crate::println!("[boot]   Architecture: {}", params.architecture_name());
        crate::println!("[boot]   Protocol: {:?}", params.protocol_type());

        if let Some(memory_map) = get_memory_map() {
            crate::println!("[boot]   Memory map entries: {}", memory_map.entry_count);
            crate::println!("[boot]   Usable memory: {} MB", memory_map.usable_memory() / (1024 * 1024));
        }

        if params.has_framebuffer() {
            let fb = params.framebuffer.as_ref().unwrap();
            crate::println!("[boot]   Framebuffer: {}x{}x{}", fb.width, fb.height, fb.bytes_per_pixel);
        }

        if params.has_acpi() {
            crate::println!("[boot]   ACPI RSDP: {:#x}", params.acpi_rsdp.unwrap());
        }

        if params.has_device_tree() {
            crate::println!("[boot]   Device Tree: {:#x}", params.device_tree.unwrap());
        }

        if params.has_command_line() {
            crate::println!("[boot]   Command line: {}", params.command_line.unwrap());
        }

        if let Some(timestamp) = get_boot_timestamp() {
            crate::println!("[boot]   Boot timestamp: {} ns", timestamp);
        }
    } else {
        crate::println!("[boot] No boot parameters available (legacy mode)");
    }
}

/// Initialize memory management from boot information
pub fn init_memory_from_boot_info() {
    if let Some(params) = get_boot_parameters() {
        // Use params for validation/logging
        let _boot_params = &params; // Use params for validation
        
        // Initialize memory management using bootloader-provided memory map
        if let Some(memory_map) = get_memory_map() {
            crate::println!("[boot] Initializing memory from bootloader memory map");

            // Count usable memory regions
            let usable_regions = memory_map.entries()
                .filter(|entry| entry.is_available && entry.mem_type == MemoryType::Usable)
                .count();

            if usable_regions > 0 {
                crate::println!("[boot] Found {} usable memory regions", usable_regions);
                // In a real implementation, we'd pass this information to the memory manager
            } else {
                crate::println!("[boot] Warning: No usable memory regions found");
            }
        }
    }
}

/// Initialize framebuffer from boot information
pub fn init_framebuffer_from_boot_info() {
    if let Some(fb_info) = get_framebuffer_info() {
        crate::println!("[boot] Initializing framebuffer from bootloader");
        crate::println!("[boot]   Address: {:#x}", fb_info.address);
        crate::println!("[boot]   Resolution: {}x{}", fb_info.width, fb_info.height);
        crate::println!("[boot]   Format: {} BPP, stride: {}", fb_info.bytes_per_pixel, fb_info.stride);

        // In a real implementation, we'd initialize the framebuffer driver here
    }
}

/// Initialize ACPI from boot information
pub fn init_acpi_from_boot_info() {
    if let Some(rsdp) = get_acpi_rsdp() {
        crate::println!("[boot] Initializing ACPI from bootloader");
        crate::println!("[boot]   RSDP at: {:#x}", rsdp);

        // In a real implementation, we'd initialize ACPI subsystem here
    }
}

/// Initialize device tree from boot information
pub fn init_device_tree_from_boot_info() {
    if let Some(dtb) = get_device_tree() {
        crate::println!("[boot] Initializing device tree from bootloader");
        crate::println!("[boot]   DTB at: {:#x}", dtb);

        // In a real implementation, we'd parse the device tree here
    }
}