/// Multiboot2 Protocol Executor
///
/// Implements Multiboot2 protocol for kernel boot information passing.
/// Handles boot info structure, tags, and kernel parameters.

use alloc::vec::Vec;

/// Multiboot2 magic
pub const MULTIBOOT2_MAGIC: u32 = 0x36D76289;

/// Multiboot2 tag types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagType {
    End,
    Cmdline,
    BootLoaderName,
    Module,
    BasicMemInfo,
    BootDevice,
    MemoryMap,
    VBEInfo,
    FramebufferInfo,
    ELFSymbols,
    APM,
    EFI32,
    EFI64,
    SMBIOS,
    ACPIOld,
    ACPINew,
    Network,
    EFIBootImageHandle,
    ImageLoadBasePhysAddr,
}

impl TagType {
    pub fn code(&self) -> u32 {
        match self {
            Self::End => 0,
            Self::Cmdline => 1,
            Self::BootLoaderName => 2,
            Self::Module => 3,
            Self::BasicMemInfo => 4,
            Self::BootDevice => 5,
            Self::MemoryMap => 6,
            Self::VBEInfo => 7,
            Self::FramebufferInfo => 8,
            Self::ELFSymbols => 9,
            Self::APM => 10,
            Self::EFI32 => 11,
            Self::EFI64 => 12,
            Self::SMBIOS => 13,
            Self::ACPIOld => 14,
            Self::ACPINew => 15,
            Self::Network => 16,
            Self::EFIBootImageHandle => 17,
            Self::ImageLoadBasePhysAddr => 18,
        }
    }

    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::End),
            1 => Some(Self::Cmdline),
            2 => Some(Self::BootLoaderName),
            3 => Some(Self::Module),
            4 => Some(Self::BasicMemInfo),
            5 => Some(Self::BootDevice),
            6 => Some(Self::MemoryMap),
            _ => None,
        }
    }
}

/// Multiboot2 tag header
#[derive(Debug, Clone, Copy)]
pub struct TagHeader {
    pub tag_type: u32,
    pub size: u32,
}

impl TagHeader {
    pub fn new(tag_type: u32, size: u32) -> Self {
        Self { tag_type, size }
    }

    pub fn aligned_size(&self) -> u32 {
        ((self.size + 7) / 8) * 8
    }
}

/// Multiboot2 boot info header
#[derive(Debug, Clone, Copy)]
pub struct BootInfoHeader {
    pub total_size: u32,
    pub reserved: u32,
}

impl BootInfoHeader {
    pub fn new(total_size: u32) -> Self {
        Self {
            total_size,
            reserved: 0,
        }
    }
}

/// Multiboot2 boot information
pub struct Multiboot2BootInfo {
    header: BootInfoHeader,
    tags: Vec<(TagType, Vec<u8>)>,
}

impl Multiboot2BootInfo {
    /// Create new Multiboot2 boot info
    pub fn new(total_size: u32) -> Self {
        Self {
            header: BootInfoHeader::new(total_size),
            tags: Vec::new(),
        }
    }

    /// Add command line tag
    pub fn add_cmdline(&mut self, cmdline: &str) -> Result<(), &'static str> {
        if self.tags.len() >= 20 {
            return Err("Too many tags");
        }

        let mut data = Vec::new();
        for byte in cmdline.as_bytes() {
            data.push(*byte);
        }
        data.push(0); // Null terminator

        self.tags.push((TagType::Cmdline, data));
        Ok(())
    }

    /// Add boot loader name tag
    pub fn add_bootloader_name(&mut self, name: &str) -> Result<(), &'static str> {
        if self.tags.len() >= 20 {
            return Err("Too many tags");
        }

        let mut data = Vec::new();
        for byte in name.as_bytes() {
            data.push(*byte);
        }
        data.push(0); // Null terminator

        self.tags.push((TagType::BootLoaderName, data));
        Ok(())
    }

    /// Add memory map tag
    pub fn add_memory_map(&mut self, entries: &[(u64, u64, u32)]) -> Result<(), &'static str> {
        if self.tags.len() >= 20 {
            return Err("Too many tags");
        }

        let mut data = Vec::new();

        // Entry size and version (8 bytes)
        data.extend_from_slice(&24u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        // Memory map entries
        for (base, length, mtype) in entries {
            data.extend_from_slice(&base.to_le_bytes());
            data.extend_from_slice(&length.to_le_bytes());
            data.extend_from_slice(&mtype.to_le_bytes());
            data.extend_from_slice(&0u32.to_le_bytes()); // reserved
        }

        self.tags.push((TagType::MemoryMap, data));
        Ok(())
    }

    /// Get total size
    pub fn total_size(&self) -> u32 {
        self.header.total_size
    }

    /// Get tag count
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Get tags
    pub fn tags(&self) -> &[(TagType, Vec<u8>)] {
        &self.tags
    }

    /// Validate boot info
    pub fn is_valid(&self) -> bool {
        self.header.total_size > 0
            && !self.tags.is_empty()
    }
}

/// Multiboot2 protocol executor
pub struct Multiboot2Executor {
    boot_info: Option<Multiboot2BootInfo>,
    magic: u32,
    validated: bool,
}

impl Multiboot2Executor {
    /// Create new Multiboot2 executor
    pub fn new() -> Self {
        Self {
            boot_info: None,
            magic: MULTIBOOT2_MAGIC,
            validated: false,
        }
    }

    /// Set boot info
    pub fn set_boot_info(&mut self, boot_info: Multiboot2BootInfo) {
        self.boot_info = Some(boot_info);
    }

    /// Validate Multiboot2 boot info
    pub fn validate(&mut self) -> Result<(), &'static str> {
        let boot_info = self.boot_info.as_ref().ok_or("No boot info")?;

        if !boot_info.is_valid() {
            return Err("Invalid boot info");
        }

        self.validated = true;
        Ok(())
    }

    /// Get magic number
    pub fn magic(&self) -> u32 {
        self.magic
    }

    /// Check if validated
    pub fn is_validated(&self) -> bool {
        self.validated
    }

    /// Get boot info address
    pub fn get_boot_info_address(&self) -> Option<u64> {
        if self.validated {
            Some(0x2000) // Standard address
        } else {
            None
        }
    }

    /// Get tag count
    pub fn tag_count(&self) -> usize {
        self.boot_info
            .as_ref()
            .map(|info| info.tag_count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiboot2_magic_constant() {
        assert_eq!(MULTIBOOT2_MAGIC, 0x36D76289);
    }

    #[test]
    fn test_tag_type_codes() {
        assert_eq!(TagType::End.code(), 0);
        assert_eq!(TagType::Cmdline.code(), 1);
        assert_eq!(TagType::MemoryMap.code(), 6);
    }

    #[test]
    fn test_tag_type_from_code() {
        assert_eq!(TagType::from_code(0), Some(TagType::End));
        assert_eq!(TagType::from_code(1), Some(TagType::Cmdline));
    }

    #[test]
    fn test_tag_header_creation() {
        let header = TagHeader::new(1, 32);
        assert_eq!(header.tag_type, 1);
        assert_eq!(header.size, 32);
    }

    #[test]
    fn test_tag_header_aligned_size() {
        let header = TagHeader::new(1, 25);
        assert_eq!(header.aligned_size(), 32);
    }

    #[test]
    fn test_boot_info_header_creation() {
        let header = BootInfoHeader::new(1024);
        assert_eq!(header.total_size, 1024);
    }

    #[test]
    fn test_multiboot2_boot_info_creation() {
        let boot_info = Multiboot2BootInfo::new(1024);
        assert_eq!(boot_info.total_size(), 1024);
        assert_eq!(boot_info.tag_count(), 0);
    }

    #[test]
    fn test_multiboot2_boot_info_add_cmdline() {
        let mut boot_info = Multiboot2BootInfo::new(1024);
        assert!(boot_info.add_cmdline("console=ttyS0").is_ok());
        assert_eq!(boot_info.tag_count(), 1);
    }

    #[test]
    fn test_multiboot2_boot_info_add_bootloader() {
        let mut boot_info = Multiboot2BootInfo::new(1024);
        assert!(boot_info.add_bootloader_name("NOS Bootloader").is_ok());
        assert_eq!(boot_info.tag_count(), 1);
    }

    #[test]
    fn test_multiboot2_boot_info_add_memory_map() {
        let mut boot_info = Multiboot2BootInfo::new(1024);
        let entries = [(0x0, 0x100000, 1), (0x100000, 0x700000, 1)];
        assert!(boot_info.add_memory_map(&entries).is_ok());
        assert_eq!(boot_info.tag_count(), 1);
    }

    #[test]
    fn test_multiboot2_boot_info_is_valid() {
        let mut boot_info = Multiboot2BootInfo::new(1024);
        assert!(!boot_info.is_valid());

        boot_info.add_cmdline("test").unwrap();
        assert!(boot_info.is_valid());
    }

    #[test]
    fn test_multiboot2_executor_creation() {
        let executor = Multiboot2Executor::new();
        assert_eq!(executor.magic(), MULTIBOOT2_MAGIC);
        assert!(!executor.is_validated());
    }

    #[test]
    fn test_multiboot2_executor_set_boot_info() {
        let mut executor = Multiboot2Executor::new();
        let mut boot_info = Multiboot2BootInfo::new(1024);
        boot_info.add_cmdline("test").unwrap();

        executor.set_boot_info(boot_info);
        assert_eq!(executor.tag_count(), 1);
    }

    #[test]
    fn test_multiboot2_executor_validate() {
        let mut executor = Multiboot2Executor::new();
        let mut boot_info = Multiboot2BootInfo::new(1024);
        boot_info.add_cmdline("test").unwrap();

        executor.set_boot_info(boot_info);
        assert!(executor.validate().is_ok());
        assert!(executor.is_validated());
    }

    #[test]
    fn test_multiboot2_executor_boot_info_address() {
        let mut executor = Multiboot2Executor::new();
        let mut boot_info = Multiboot2BootInfo::new(1024);
        boot_info.add_cmdline("test").unwrap();

        executor.set_boot_info(boot_info);
        executor.validate().unwrap();

        assert_eq!(executor.get_boot_info_address(), Some(0x2000));
    }
}
