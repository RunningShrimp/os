// Boot parameters passed to kernel

use crate::protocol::BootInfo;

#[repr(C)]
pub struct BootParams {
    /// Boot protocol type identifier
    pub boot_protocol: u32,
    
    /// Memory information
    pub memory_lower: u32,
    pub memory_upper: u32,
    
    /// Boot device info
    pub boot_device: u32,
    
    /// Command line address
    pub cmdline: u32,
    
    /// Module count and address
    pub mods_count: u32,
    pub mods_addr: u32,
    
    /// Kernel image info
    pub kernel_entry: u64,
    pub kernel_load_addr: u32,
    pub kernel_size: u32,
    
    /// Memory map
    pub mmap_length: u32,
    pub mmap_addr: u32,
    
    /// Framebuffer info
    pub framebuffer_addr: u64,
    pub framebuffer_pitch: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp: u8,
    
    /// Architecture-specific data
    pub arch_reserved: [u32; 16],
}

impl BootParams {
    pub fn new() -> Self {
        Self {
            boot_protocol: 0,
            memory_lower: 0,
            memory_upper: 0,
            boot_device: 0,
            cmdline: 0,
            mods_count: 0,
            mods_addr: 0,
            kernel_entry: 0,
            kernel_load_addr: 0,
            kernel_size: 0,
            mmap_length: 0,
            mmap_addr: 0,
            framebuffer_addr: 0,
            framebuffer_pitch: 0,
            framebuffer_width: 0,
            framebuffer_height: 0,
            framebuffer_bpp: 0,
            arch_reserved: [0; 16],
        }
    }

    pub fn from_boot_info(info: &BootInfo) -> Self {
        let mut params = Self::new();
        
        // Set basic protocol info
        params.boot_protocol = match info.protocol_type {
            crate::protocol::BootProtocolType::Bios => 1,
            crate::protocol::BootProtocolType::Uefi => 2,
            crate::protocol::BootProtocolType::Multiboot2 => 3,
        };
        
        params
    }
}

impl Default for BootParams {
    fn default() -> Self {
        Self::new()
    }
}
