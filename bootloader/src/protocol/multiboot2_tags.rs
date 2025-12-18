// Multiboot2 tag structures and parsing

#[repr(C, packed)]
pub struct MultibootTag {
    pub tag_type: u32,
    pub size: u32,
}

pub const MULTIBOOT2_MAGIC: u32 = 0xE85250D6;
pub const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36D76289;

pub const MULTIBOOT_TAG_TYPE_END: u32 = 0;
pub const MULTIBOOT_TAG_TYPE_CMDLINE: u32 = 1;
pub const MULTIBOOT_TAG_TYPE_BOOTLOADER_NAME: u32 = 2;
pub const MULTIBOOT_TAG_TYPE_MODULE: u32 = 3;
pub const MULTIBOOT_TAG_TYPE_BASIC_MEMINFO: u32 = 4;
pub const MULTIBOOT_TAG_TYPE_BOOTDEV: u32 = 5;
pub const MULTIBOOT_TAG_TYPE_MMAP: u32 = 6;
pub const MULTIBOOT_TAG_TYPE_VBE: u32 = 7;
pub const MULTIBOOT_TAG_TYPE_FRAMEBUFFER: u32 = 8;
pub const MULTIBOOT_TAG_TYPE_ELF_SECTIONS: u32 = 9;
pub const MULTIBOOT_TAG_TYPE_APM: u32 = 10;
pub const MULTIBOOT_TAG_TYPE_EFI32: u32 = 11;
pub const MULTIBOOT_TAG_TYPE_EFI64: u32 = 12;
pub const MULTIBOOT_TAG_TYPE_SMBIOS: u32 = 13;
pub const MULTIBOOT_TAG_TYPE_ACPI_OLD: u32 = 14;
pub const MULTIBOOT_TAG_TYPE_ACPI_NEW: u32 = 15;
pub const MULTIBOOT_TAG_TYPE_NETWORK: u32 = 16;
pub const MULTIBOOT_TAG_TYPE_EFI_MMAP: u32 = 17;
pub const MULTIBOOT_TAG_TYPE_EFI_BS: u32 = 18;
pub const MULTIBOOT_TAG_TYPE_EFI32_IH: u32 = 19;
pub const MULTIBOOT_TAG_TYPE_EFI64_IH: u32 = 20;
pub const MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR: u32 = 21;

#[repr(C, packed)]
pub struct MultibootMemoryMap {
    pub size: u32,
    pub entry_size: u32,
    pub version: u32,
    pub first_entry: [u8; 0],
}

#[repr(C, packed)]
pub struct MultibootMemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub mem_type: u32,
    pub reserved: u32,
}

pub const MULTIBOOT_MEMORY_AVAILABLE: u32 = 1;
pub const MULTIBOOT_MEMORY_RESERVED: u32 = 2;
pub const MULTIBOOT_MEMORY_ACPI_RECLAIMABLE: u32 = 3;
pub const MULTIBOOT_MEMORY_NVS: u32 = 4;
pub const MULTIBOOT_MEMORY_BADRAM: u32 = 5;

#[repr(C, packed)]
pub struct MultibootFramebufferInfo {
    pub framebuffer_addr: u64,
    pub framebuffer_pitch: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp: u8,
    pub framebuffer_type: u8,
    pub reserved: u8,
    pub color_info_union: [u8; 0],
}

pub fn parse_tags(info_addr: usize) -> Option<TagIterator> {
    // Validate that we can read tag info
    if info_addr == 0 {
        return None;
    }
    
    // Read total size from the Multiboot2 information structure
    // The structure starts with: u32 total_size; u32 reserved;
    let total_size = unsafe {
        *(info_addr as *const u32)
    };
    
    Some(TagIterator {
        current: (info_addr + 8) as *const MultibootTag,
        end_addr: info_addr + total_size as usize,
    })
}

pub struct TagIterator {
    current: *const MultibootTag,
    end_addr: usize,
}

impl Iterator for TagIterator {
    type Item = *const MultibootTag;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            // Check if we've reached the end tag or exceeded the memory boundary
            if (*self.current).tag_type == MULTIBOOT_TAG_TYPE_END {
                return None;
            }
            
            // Get current tag address as usize for boundary checking
            let current_addr = self.current as usize;
            if current_addr >= self.end_addr {
                return None;
            }

            let tag = self.current;
            let size = (*self.current).size as usize;
            let aligned_size = (size + 7) & !7;
            self.current = (current_addr + aligned_size)
                as *const MultibootTag;

            Some(tag)
        }
    }
}
