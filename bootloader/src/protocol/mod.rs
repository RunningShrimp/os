//! Boot protocol

use arrayvec::ArrayVec;

pub mod multiboot2;
pub mod multiboot2_tags;
pub mod uefi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProtocolType {
    Bios,
    Uefi,
    Multiboot2,
}

#[derive(Debug, Clone)]
pub struct BootInfo {
    pub protocol_type: BootProtocolType,
    pub memory_map: ArrayVec<MemoryMapEntry, 128>,
    pub initrd: Option<KernelImageInfo>,
    pub framebuffer: Option<FramebufferInfo>,
    pub device_tree: Option<u64>,
    pub cmdline: ArrayVec<u8, 512>,
    pub boot_timestamp: u64,
    pub acpi_tables: Option<u64>,
    pub acpi_rsdp: Option<u64>,
}

impl BootInfo {
    pub fn new(protocol_type: BootProtocolType) -> Self {
        Self {
            protocol_type,
            memory_map: ArrayVec::new(),
            initrd: None,
            framebuffer: None,
            device_tree: None,
            cmdline: ArrayVec::new(),
            boot_timestamp: 0,
            acpi_tables: None,
            acpi_rsdp: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    pub base: u64,
    pub size: u64,
    pub entry_type: u32,
}

#[derive(Debug, Clone)]
pub struct KernelImageInfo {
    pub address: u64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct FramebufferInfo {
    pub address: usize,
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub pitch: u32,
}

impl FramebufferInfo {
    /// Create new framebuffer info
    pub fn new(address: usize, width: u32, height: u32, pitch: u32, bpp: u32) -> Self {
        Self {
            address,
            width,
            height,
            pitch,
            bpp,
        }
    }

    /// Calculate buffer size in bytes
    pub fn buffer_size(&self) -> usize {
        (self.height * self.pitch) as usize
    }

    /// Check if coordinates are within bounds
    pub fn in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }
}

#[derive(Debug, Clone)]
pub struct KernelImage {
    pub entry_point: usize,
    pub base_addr: usize,
    pub size: usize,
}
