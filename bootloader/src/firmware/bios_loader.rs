// BIOS/Multiboot2 bootloader support

pub struct BiosBootInfo {
    pub magic: u32,
    pub multiboot_info_addr: u32,
    pub boot_device: u32,
    pub memory_lower: u32,
    pub memory_upper: u32,
}

impl BiosBootInfo {
    pub fn new(magic: u32, info_addr: u32) -> Self {
        Self {
            magic,
            multiboot_info_addr: info_addr,
            boot_device: 0,
            memory_lower: 0,
            memory_upper: 0,
        }
    }

    pub fn is_multiboot2(&self) -> bool {
        self.magic == 0x36D76289 // Multiboot2 magic
    }

    pub fn is_multiboot1(&self) -> bool {
        self.magic == 0x2BADB002 // Multiboot1 magic
    }

    pub fn is_valid(&self) -> bool {
        self.is_multiboot1() || self.is_multiboot2()
    }
}

/// Detect boot mode (BIOS vs UEFI)
pub fn detect_boot_mode() -> BootMode {
    // Check EFI system table pointer location
    // For now, assume BIOS/Multiboot2
    BootMode::Bios
}

pub enum BootMode {
    Bios,
    Uefi,
    Unknown,
}

/// Parse BIOS/Multiboot boot information
pub fn parse_bios_boot_info(
    magic: u32,
    info_addr: u32,
) -> Option<BiosBootInfo> {
    let info = BiosBootInfo::new(magic, info_addr);

    if !info.is_valid() {
        crate::drivers::console::write_str("Invalid BIOS boot magic\n");
        return None;
    }

    if info.is_multiboot2() {
        crate::drivers::console::write_str("Multiboot2 detected\n");
    } else if info.is_multiboot1() {
        crate::drivers::console::write_str("Multiboot1 detected\n");
    }

    Some(info)
}

/// Setup for BIOS boot continuation
pub fn prepare_bios_boot() -> bool {
    crate::drivers::console::write_str("BIOS boot mode initialized\n");
    true
}
