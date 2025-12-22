//! Multiboot2 protocol support

use crate::protocol::{BootInfo, BootProtocolType};

/// Multiboot2 header magic
pub const MULTIBOOT2_HEADER_MAGIC: u32 = 0xE85250D6;

/// Multiboot2 bootloader magic (passed in EAX)
pub const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36D76289;

/// Multiboot2 header tag types
#[repr(u16)]
pub enum TagType {
    EndOfTags = 0,
    InformationRequest = 1,
    Address = 2,
    EntryAddress = 3,
    Flags = 4,
    Console = 5,
    Framebuffer = 6,
    ModuleAlignment = 7,
    RelocatableHeader = 10,
}

/// Multiboot2 header structure
#[repr(C)]
pub struct Header {
    pub magic: u32,
    pub arch: u32,
    pub header_length: u32,
    pub checksum: u32,
}

/// Parse Multiboot2 boot info from bootloader
pub fn parse_multiboot2(magic: u32, _info_ptr: usize) -> Option<BootInfo> {
    // Verify bootloader magic
    if magic != MULTIBOOT2_BOOTLOADER_MAGIC {
        return None;
    }

    log::debug!("Parsing Multiboot2 boot information");
    let boot_info = BootInfo::new(BootProtocolType::Multiboot2);
    
    // Stub: Just return basic info
    // Full parsing would require careful tag processing
    // For now, we just recognize multiboot2 was used
    
    Some(boot_info)
}
