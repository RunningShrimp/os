//! Multiboot2 Protocol implementation
//!
//! This module implements the Multiboot2 specification for booting
//! kernels on x86_64 systems with BIOS bootloader.

use crate::error::{BootError, Result};
use core::mem;

/// Multiboot2 Magic Number
pub const MULTIBOOT2_MAGIC: u32 = 0x36d76289;

/// Multiboot2 Header Tags
pub const MULTIBOOT2_HEADER_TAG_END: u16 = 0;
pub const MULTIBOOT2_HEADER_TAG_INFORMATION_REQUEST: u16 = 1;
pub const MULTIBOOT2_HEADER_TAG_ADDRESS: u16 = 2;
pub const MULTIBOOT2_HEADER_TAG_ENTRY_ADDRESS: u16 = 3;
pub const MULTIBOOT2_HEADER_TAG_CONSOLE_FLAGS: u16 = 4;
pub const MULTIBOOT2_HEADER_TAG_FRAMEBUFFER: u16 = 5;
pub const MULTIBOOT2_HEADER_TAG_MODULE_ALIGN: u16 = 6;
pub const MULTIBOOT2_HEADER_TAG_EFI_BS: u16 = 7;
pub const MULTIBOOT2_HEADER_TAG_EFI32: u16 = 8;
pub const MULTIBOOT2_HEADER_TAG_EFI64: u16 = 9;
pub const MULTIBOOT2_HEADER_TAG_RELOCATABLE: u16 = 10;

/// Multiboot2 Information Tags
pub const MULTIBOOT2_TAG_TYPE_END: u32 = 0;
pub const MULTIBOOT2_TAG_TYPE_CMDLINE: u32 = 1;
pub const MULTIBOOT2_TAG_TYPE_BOOT_LOADER_NAME: u32 = 2;
pub const MULTIBOOT2_TAG_TYPE_MODULE: u32 = 3;
pub const MULTIBOOT2_TAG_TYPE_BASIC_MEMINFO: u32 = 4;
pub const MULTIBOOT2_TAG_TYPE_BOOTDEV: u32 = 5;
pub const MULTIBOOT2_TAG_TYPE_MMAP: u32 = 6;
pub const MULTIBOOT2_TAG_TYPE_VBE: u32 = 7;
pub const MULTIBOOT2_TAG_TYPE_FRAMEBUFFER: u32 = 8;
pub const MULTIBOOT2_TAG_TYPE_ELF_SECTIONS: u32 = 9;
pub const MULTIBOOT2_TAG_TYPE_APM: u32 = 10;
pub const MULTIBOOT2_TAG_TYPE_EFI32: u32 = 11;
pub const MULTIBOOT2_TAG_TYPE_EFI64: u32 = 12;
pub const MULTIBOOT2_TAG_TYPE_SMBIOS: u32 = 13;
pub const MULTIBOOT2_TAG_TYPE_ACPI_OLD: u32 = 14;
pub const MULTIBOOT2_TAG_TYPE_ACPI_NEW: u32 = 15;
pub const MULTIBOOT2_TAG_TYPE_NETWORK: u32 = 16;
pub const MULTIBOOT2_TAG_TYPE_EFI_MMAP: u32 = 17;
pub const MULTIBOOT2_TAG_TYPE_EFI_BS: u32 = 18;
pub const MULTIBOOT2_TAG_TYPE_EFI32_IH: u32 = 19;
pub const MULTIBOOT2_TAG_TYPE_EFI64_IH: u32 = 20;
pub const MULTIBOOT2_TAG_TYPE_LOAD_BASE_ADDR: u32 = 21;

/// Memory Types for E820 Memory Map
pub const MULTIBOOT2_MEMORY_AVAILABLE: u32 = 1;
pub const MULTIBOOT2_MEMORY_RESERVED: u32 = 2;
pub const MULTIBOOT2_MEMORY_ACPI_RECLAIMABLE: u32 = 3;
pub const MULTIBOOT2_MEMORY_NVS: u32 = 4;
pub const MULTIBOOT2_MEMORY_BADRAM: u32 = 5;

/// Multiboot2 Memory Map Entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2MmapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub type_: u32,
    pub zero: u32,
}

/// Multiboot2 Framebuffer Type
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum Multiboot2FramebufferType {
    Indexed = 0,
    RGB = 1,
    EGAText = 2,
}

/// Multiboot2 Framebuffer Info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2FramebufferInfo {
    pub framebuffer_type: Multiboot2FramebufferType,
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub pitch: u32,
    pub address: u64,
}

/// Multiboot2 VBE Control Info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2VBEInfo {
    pub vbe_mode: u16,
    pub vbe_interface_seg: u16,
    pub vbe_interface_off: u16,
    pub vbe_interface_len: u16,
}

/// Multiboot2 Module Info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2Module {
    pub start: u32,
    pub end: u32,
    pub cmdline: u32,
    pub padding: u32,
}

/// Multiboot2 ELF Section Info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2ELFSection {
    pub num: u32,
    pub size: u32,
    pub entry: u32,
    pub shndx: u32,
}

/// Multiboot2 Boot Loader Name
pub const MULTIBOOT2_BOOT_LOADER_NAME: &str = "NOS BIOS Bootloader v0.1.0";

/// Multiboot2 Header Structure
#[derive(Debug)]
#[repr(C, packed)]
pub struct Multiboot2Header {
    /// Magic number: MULTIBOOT2_MAGIC
    pub magic: u32,
    /// Architecture: 0 for i386, 4 for MIPS32, etc.
    pub architecture: u32,
    /// Header length
    pub header_length: u32,
    /// Checksum
    pub checksum: u32,
    /// Followed by tags
}

/// Generic Multiboot2 Tag Header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagHeader {
    pub type_: u32,
    pub size: u32,
}

/// Multiboot2 End Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagEnd {
    pub header: Multiboot2TagHeader,
}

/// Multiboot2 Command Line Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagCmdline {
    pub header: Multiboot2TagHeader,
    /// Null-terminated string follows
}

/// Multiboot2 Boot Loader Name Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagBootLoaderName {
    pub header: Multiboot2TagHeader,
    /// Null-terminated string follows
}

/// Multiboot2 Basic Memory Info Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagBasicMeminfo {
    pub header: Multiboot2TagHeader,
    pub mem_lower: u32,
    pub mem_upper: u32,
}

/// Multiboot2 Memory Map Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagMmap {
    pub header: Multiboot2TagHeader,
    pub entry_size: u32,
    pub entry_version: u32,
    /// Multiboot2MmapEntry entries follow
}

/// Multiboot2 Framebuffer Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagFramebuffer {
    pub header: Multiboot2TagHeader,
    pub framebuffer: Multiboot2FramebufferInfo,
    /// Additional framebuffer info follows based on type
}

/// Multiboot2 VBE Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagVBE {
    pub header: Multiboot2TagHeader,
    pub vbe_mode: u16,
    pub vbe_interface_seg: u16,
    pub vbe_interface_off: u16,
    pub vbe_interface_len: u16,
    /// VBE control info follows
    /// VBE mode info follows
}

/// Multiboot2 Module Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagModule {
    pub header: Multiboot2TagHeader,
    pub module: Multiboot2Module,
    /// Null-terminated command line string follows
}

/// Multiboot2 ELF Sections Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagELFSections {
    pub header: Multiboot2TagHeader,
    pub num: u32,
    pub entry_size: u32,
    pub section_header_table: Multiboot2ELFSection,
    /// ELF section headers follow
}

/// Multiboot2 ACPI Old Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagACPIOld {
    pub header: Multiboot2TagHeader,
    pub rsdp: [u8; 20],
}

/// Multiboot2 ACPI New Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagACPINew {
    pub header: Multiboot2TagHeader,
    pub rsdp: [u8; 24],
}

/// Multiboot2 Load Base Address Tag
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Multiboot2TagLoadBaseAddr {
    pub header: Multiboot2TagHeader,
    pub load_base_addr: u32,
}

/// Multiboot2 Information Structure Builder
pub struct Multiboot2InfoBuilder {
    buffer: *mut u8,
    offset: usize,
    total_size: usize,
}

impl Multiboot2InfoBuilder {
    /// Create a new Multiboot2 info structure builder
    pub fn new(buffer: *mut u8, max_size: usize) -> Self {
        Self {
            buffer,
            offset: 0,
            total_size: max_size,
        }
    }

    /// Add a tag to the structure
    pub fn add_tag<T>(&mut self, tag: &T) -> Result<*const T> {
        let tag_size = mem::size_of::<T>();

        if self.offset + tag_size > self.total_size {
            return Err(BootError::InsufficientMemory);
        }

        let tag_ptr = unsafe {
            self.buffer.add(self.offset) as *mut T
        };

        unsafe {
            tag_ptr.write(*tag);
        }

        let added_ptr = unsafe { &*tag_ptr };

        // Align to 8-byte boundary
        self.offset = (self.offset + tag_size + 7) & !7;

        Ok(added_ptr)
    }

    /// Add a string tag
    pub fn add_string_tag(&mut self, tag_header: Multiboot2TagHeader, string: &str) -> Result<()> {
        let header_size = mem::size_of::<Multiboot2TagHeader>();
        let string_size = string.len() + 1; // Include null terminator
        let total_size = (header_size + string_size + 7) & !7; // 8-byte align

        if self.offset + total_size > self.total_size {
            return Err(BootError::InsufficientMemory);
        }

        // Write tag header
        let header_ptr = unsafe {
            self.buffer.add(self.offset) as *mut Multiboot2TagHeader
        };

        unsafe {
            header_ptr.write(tag_header);

            // Write string
            let string_ptr = self.buffer.add(self.offset + header_size);
            core::ptr::copy_nonoverlapping(string.as_ptr(), string_ptr, string.len());
            *string_ptr.add(string.len() - 1) = 0; // Add null terminator
        }

        self.offset += total_size;
        Ok(())
    }

    /// Add memory map entries
    pub fn add_mmap_entries(&mut self, entries: &[Multiboot2MmapEntry]) -> Result<()> {
        let header_size = mem::size_of::<Multiboot2TagMmap>();
        let entry_size = mem::size_of::<Multiboot2MmapEntry>();
        let total_entries_size = entries.len() * entry_size;
        let total_size = (header_size + total_entries_size + 7) & !7;

        if self.offset + total_size > self.total_size {
            return Err(BootError::InsufficientMemory);
        }

        // Write mmap tag header
        let mmap_tag = Multiboot2TagMmap {
            header: Multiboot2TagHeader {
                type_: MULTIBOOT2_TAG_TYPE_MMAP,
                size: (total_size as u32),
            },
            entry_size: (entry_size as u32),
            entry_version: 0,
        };

        let tag_ptr = unsafe {
            self.buffer.add(self.offset) as *mut Multiboot2TagMmap
        };

        unsafe {
            tag_ptr.write(mmap_tag);

            // Write entries
            let entries_ptr = self.buffer.add(self.offset + header_size) as *mut Multiboot2MmapEntry;
            for (i, entry) in entries.iter().enumerate() {
                entries_ptr.add(i).write(*entry);
            }
        }

        self.offset += total_size;
        Ok(())
    }

    /// Finish building and add end tag
    pub fn finish(&mut self) -> Result<u32> {
        // Add end tag
        let end_tag = Multiboot2TagEnd {
            header: Multiboot2TagHeader {
                type_: MULTIBOOT2_TAG_TYPE_END,
                size: (mem::size_of::<Multiboot2TagEnd>() as u32),
            },
        };

        self.add_tag(&end_tag)?;
        Ok((self.offset as u32))
    }
}

/// Multiboot2 Protocol implementation
#[cfg(feature = "multiboot2_support")]
pub struct Multiboot2Protocol {
    initialized: bool,
    info_buffer: *mut u8,
    info_size: usize,
}

#[cfg(feature = "multiboot2_support")]
impl Multiboot2Protocol {
    /// Create a new Multiboot2 protocol instance
    pub fn new() -> Self {
        Self {
            initialized: false,
            info_buffer: core::ptr::null_mut(),
            info_size: 0,
        }
    }

    /// Initialize Multiboot2 protocol
    pub fn initialize(&mut self, buffer: *mut u8, max_size: usize) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        self.info_buffer = buffer;
        self.info_size = max_size;
        self.initialized = true;

        println!("[multiboot2] Initialized with buffer at {:#x}, size: {} bytes",
                 buffer as usize, max_size);

        Ok(())
    }

    /// Build Multiboot2 information structure
    pub fn build_info(&self,
                     cmdline: Option<&str>,
                     mem_lower: u32,
                     mem_upper: u32,
                     mmap_entries: &[Multiboot2MmapEntry],
                     framebuffer_info: Option<Multiboot2FramebufferInfo>,
                     modules: &[Multiboot2Module]) -> Result<u32> {
        if !self.initialized {
            return Err(BootError::NotInitialized);
        }

        let mut builder = Multiboot2InfoBuilder::new(self.info_buffer, self.info_size);

        // Add command line tag
        if let Some(cmdline) = cmdline {
            let tag_header = Multiboot2TagHeader {
                type_: MULTIBOOT2_TAG_TYPE_CMDLINE,
                size: (mem::size_of::<Multiboot2TagCmdline>() as u32) + (cmdline.len() as u32) + 1,
            };
            builder.add_string_tag(tag_header, cmdline)?;
        }

        // Add boot loader name tag
        let bootloader_tag = Multiboot2TagHeader {
            type_: MULTIBOOT2_TAG_TYPE_BOOT_LOADER_NAME,
            size: (mem::size_of::<Multiboot2TagBootLoaderName>() as u32) +
                  (MULTIBOOT2_BOOT_LOADER_NAME.len() as u32) + 1,
        };
        builder.add_string_tag(bootloader_tag, MULTIBOOT2_BOOT_LOADER_NAME)?;

        // Add basic memory info tag
        let meminfo_tag = Multiboot2TagBasicMeminfo {
            header: Multiboot2TagHeader {
                type_: MULTIBOOT2_TAG_TYPE_BASIC_MEMINFO,
                size: (mem::size_of::<Multiboot2TagBasicMeminfo>() as u32),
            },
            mem_lower,
            mem_upper,
        };
        builder.add_tag(&meminfo_tag)?;

        // Add memory map entries
        if !mmap_entries.is_empty() {
            builder.add_mmap_entries(mmap_entries)?;
        }

        // Add framebuffer info
        if let Some(fb_info) = framebuffer_info {
            let framebuffer_tag = Multiboot2TagFramebuffer {
                header: Multiboot2TagHeader {
                    type_: MULTIBOOT2_TAG_TYPE_FRAMEBUFFER,
                    size: (mem::size_of::<Multiboot2TagFramebuffer>() as u32),
                },
                framebuffer: fb_info,
            };
            builder.add_tag(&framebuffer_tag)?;
        }

        // Add modules
        for module in modules {
            let module_tag = Multiboot2TagModule {
                header: Multiboot2TagHeader {
                    type_: MULTIBOOT2_TAG_TYPE_MODULE,
                    size: (mem::size_of::<Multiboot2TagModule>() as u32),
                },
                module: *module,
            };
            builder.add_tag(&module_tag)?;
        }

        // Finish with end tag
        let total_size = builder.finish()?;

        println!("[multiboot2] Built info structure: {} bytes", total_size);
        Ok(total_size)
    }

    /// Validate Multiboot2 header
    pub fn validate_header(&self, header_addr: u64) -> Result<bool> {
        if header_addr == 0 {
            return Err(BootError::InvalidParameter("Header address is null"));
        }

        let header = unsafe { &*(header_addr as *const Multiboot2Header) };

        let magic_valid = header.magic == MULTIBOOT2_MAGIC;
        let checksum_valid = {
            let sum = header.magic.wrapping_add(header.architecture)
                .wrapping_add(header.header_length).wrapping_add(header.checksum);
            sum == 0
        };

        Ok(magic_valid && checksum_valid)
    }

    /// Get the info buffer address
    pub fn get_info_address(&self) -> u64 {
        self.info_buffer as u64
    }

    /// Check if protocol is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

/// Stub implementations when feature is disabled
#[cfg(not(feature = "multiboot2_support"))]
pub struct Multiboot2Protocol;

#[cfg(not(feature = "multiboot2_support"))]
impl Multiboot2Protocol {
    pub fn new() -> Self {
        Self
    }

    pub fn initialize(&mut self, _buffer: *mut u8, _max_size: usize) -> Result<()> {
        Err(BootError::FeatureNotEnabled("Multiboot2 support"))
    }

    pub fn build_info(&self,
                     _cmdline: Option<&str>,
                     _mem_lower: u32,
                     _mem_upper: u32,
                     _mmap_entries: &[Multiboot2MmapEntry],
                     _framebuffer_info: Option<Multiboot2FramebufferInfo>,
                     _modules: &[Multiboot2Module]) -> Result<u32> {
        Err(BootError::FeatureNotEnabled("Multiboot2 support"))
    }

    pub fn validate_header(&self, _header_addr: u64) -> Result<bool> {
        Err(BootError::FeatureNotEnabled("Multiboot2 support"))
    }

    pub fn get_info_address(&self) -> u64 {
        0
    }

    pub fn is_initialized(&self) -> bool {
        false
    }
}

/// Helper function to create E820 entry for Multiboot2
pub fn create_e820_entry(base_addr: u64, length: u64, type_: u32) -> Multiboot2MmapEntry {
    Multiboot2MmapEntry {
        base_addr,
        length,
        type_,
        zero: 0,
    }
}

/// Helper function to create module entry for Multiboot2
pub fn create_module_entry(start: u32, end: u32, cmdline: u32) -> Multiboot2Module {
    Multiboot2Module {
        start,
        end,
        cmdline,
        padding: 0,
    }
}

/// Helper function to create framebuffer info for Multiboot2
pub fn create_framebuffer_info(
    width: u32,
    height: u32,
    bpp: u32,
    pitch: u32,
    address: u64,
    fb_type: Multiboot2FramebufferType,
) -> Multiboot2FramebufferInfo {
    Multiboot2FramebufferInfo {
        framebuffer_type: fb_type,
        width,
        height,
        bpp,
        pitch,
        address,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiboot2_constants() {
        assert_eq!(MULTIBOOT2_MAGIC, 0x36d76289);
        assert_eq!(MULTIBOOT2_MEMORY_AVAILABLE, 1);
        assert_eq!(MULTIBOOT2_MEMORY_RESERVED, 2);
    }

    #[test]
    fn test_create_helpers() {
        let e820_entry = create_e820_entry(0x1000, 0x1000, MULTIBOOT2_MEMORY_AVAILABLE);
        assert_eq!(e820_entry.base_addr, 0x1000);
        assert_eq!(e820_entry.length, 0x1000);
        assert_eq!(e820_entry.type_, MULTIBOOT2_MEMORY_AVAILABLE);

        let module_entry = create_module_entry(0x100000, 0x200000, 0x300000);
        assert_eq!(module_entry.start, 0x100000);
        assert_eq!(module_entry.end, 0x200000);
        assert_eq!(module_entry.cmdline, 0x300000);

extern crate alloc;
        let fb_info = create_framebuffer_info(
            1024, 768, 32, 4096, 0xE0000000, Multiboot2FramebufferType::RGB
        );
        assert_eq!(fb_info.width, 1024);
        assert_eq!(fb_info.height, 768);
        assert_eq!(fb_info.bpp, 32);
    }
}