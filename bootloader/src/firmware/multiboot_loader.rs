// Multiboot2 bootloader information handler

use crate::protocol::BootInfo;
use crate::protocol::multiboot2_tags;

pub struct MultibootLoader {
    magic: u32,
    info_addr: usize,
}

impl MultibootLoader {
    pub fn new(magic: u32, info_addr: usize) -> Self {
        Self { magic, info_addr }
    }

    /// Check if valid Multiboot2 info
    pub fn is_valid(&self) -> bool {
        self.magic == multiboot2_tags::MULTIBOOT2_BOOTLOADER_MAGIC
            && self.info_addr != 0
    }

    /// Parse Multiboot2 tags and build BootInfo
    pub fn parse(&self) -> Option<BootInfo> {
        if !self.is_valid() {
            return None;
        }

        let mut boot_info =
            BootInfo::new(crate::protocol::BootProtocolType::Multiboot2);

        // Parse tags
        if let Some(mut iter) =
            multiboot2_tags::parse_tags(self.info_addr)
        {
            while let Some(tag) = iter.next() {
                unsafe {
                    self.parse_tag(tag, &mut boot_info);
                }
            }
        }

        Some(boot_info)
    }

    unsafe fn parse_tag(
        &self,
        tag: *const multiboot2_tags::MultibootTag,
        _boot_info: &mut BootInfo,
    ) {
        let tag_type = (*tag).tag_type;

        match tag_type {
            multiboot2_tags::MULTIBOOT_TAG_TYPE_CMDLINE => {
                // Command line tag at offset 8 bytes
                let cmdline_ptr =
                    (tag as usize + 8) as *const u8;
                crate::drivers::console::write_str("Kernel command: ");
                let mut i = 0;
                while i < 128 {
                    let byte = *cmdline_ptr.add(i);
                    if byte == 0 {
                        break;
                    }
                    crate::drivers::console::write_byte(byte);
                    i += 1;
                }
                crate::drivers::console::write_str("\n");
            }

            multiboot2_tags::MULTIBOOT_TAG_TYPE_BOOTLOADER_NAME => {
                let name_ptr = (tag as usize + 8) as *const u8;
                crate::drivers::console::write_str("Bootloader: ");
                let mut i = 0;
                while i < 64 {
                    let byte = *name_ptr.add(i);
                    if byte == 0 {
                        break;
                    }
                    crate::drivers::console::write_byte(byte);
                    i += 1;
                }
                crate::drivers::console::write_str("\n");
            }

            multiboot2_tags::MULTIBOOT_TAG_TYPE_BASIC_MEMINFO => {
                let meminfo_ptr =
                    tag as *const multiboot2_tags::MultibootTag;
                // Memory info at offset +8 (lower mem in KB)
                let mem_lower =
                    *(meminfo_ptr as usize as *const u32).add(2);
                crate::drivers::console::write_str("Memory: ");
                crate::drivers::console::write_str(
                    if mem_lower > 0 { "OK" } else { "?" },
                );
                crate::drivers::console::write_str("\n");
            }

            multiboot2_tags::MULTIBOOT_TAG_TYPE_FRAMEBUFFER => {
                crate::drivers::console::write_str("Framebuffer: detected\n");
            }

            multiboot2_tags::MULTIBOOT_TAG_TYPE_MMAP => {
                crate::drivers::console::write_str("Memory map: present\n");
            }

            _ => {
                // Unknown tag, skip
            }
        }
    }
}

/// Parse Multiboot2 boot information
pub fn load_from_multiboot2(
    magic: u32,
    info_addr: usize,
) -> Option<BootInfo> {
    let loader = MultibootLoader::new(magic, info_addr);
    loader.parse()
}
