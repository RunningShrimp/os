/// Kernel Entry Handler
///
/// Manages kernel entry point detection, validation, and handoff preparation.

use alloc::format;
use alloc::string::String;

/// Kernel entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEntryType {
    Multiboot2,
    ELF64DirectJump,
    LinuxBoot,
    UEFI,
    Unknown,
}

impl KernelEntryType {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Multiboot2 => "Multiboot2 Protocol",
            Self::ELF64DirectJump => "ELF64 Direct Jump",
            Self::LinuxBoot => "Linux Boot Protocol",
            Self::UEFI => "UEFI Boot Services",
            Self::Unknown => "Unknown Protocol",
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// Kernel entry information
#[derive(Debug, Clone, Copy)]
pub struct KernelEntry {
    pub entry_point: u64,
    pub entry_type: KernelEntryType,
    pub bit_width: u32,
    pub page_aligned: bool,
}

impl KernelEntry {
    pub fn new(entry_point: u64, entry_type: KernelEntryType) -> Self {
        Self {
            entry_point,
            entry_type,
            bit_width: 64,
            page_aligned: (entry_point % 0x1000) == 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.entry_point != 0
            && self.entry_type.is_supported()
            && self.page_aligned
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.page_aligned {
            return Err("Entry point not page-aligned");
        }

        if !self.entry_type.is_supported() {
            return Err("Unsupported entry type");
        }

        if self.entry_point == 0 {
            return Err("Invalid entry point");
        }

        Ok(())
    }
}

/// Kernel entry parameters
#[derive(Debug, Clone, Copy)]
pub struct KernelEntryParams {
    pub boot_info_address: u64,
    pub memory_map_address: u64,
    pub magic: u32,
}

impl KernelEntryParams {
    pub fn new(boot_info_address: u64, memory_map_address: u64) -> Self {
        Self {
            boot_info_address,
            memory_map_address,
            magic: 0x36D76289, // Multiboot2 magic
        }
    }

    pub fn is_valid(&self) -> bool {
        self.boot_info_address != 0
            && self.memory_map_address != 0
            && self.boot_info_address != self.memory_map_address
    }
}

/// Kernel entry handler
pub struct KernelEntryHandler {
    entry: Option<KernelEntry>,
    parameters: Option<KernelEntryParams>,
    validation_passed: bool,
}

impl KernelEntryHandler {
    /// Create new kernel entry handler
    pub fn new() -> Self {
        Self {
            entry: None,
            parameters: None,
            validation_passed: false,
        }
    }

    /// Set kernel entry
    pub fn set_entry(&mut self, entry: KernelEntry) -> Result<(), &'static str> {
        entry.validate()?;
        self.entry = Some(entry);
        Ok(())
    }

    /// Set entry parameters
    pub fn set_parameters(&mut self, params: KernelEntryParams) -> Result<(), &'static str> {
        if !params.is_valid() {
            return Err("Invalid entry parameters");
        }

        self.parameters = Some(params);
        Ok(())
    }

    /// Detect kernel entry type
    pub fn detect_entry_type(&mut self, header_signature: u32) -> KernelEntryType {
        match header_signature {
            0xE85250D6 => KernelEntryType::Multiboot2,
            0x464C457F => KernelEntryType::ELF64DirectJump,
            0xAA55 => KernelEntryType::LinuxBoot,
            _ => KernelEntryType::Unknown,
        }
    }

    /// Validate kernel entry
    pub fn validate_entry(&mut self) -> Result<(), &'static str> {
        let entry = self.entry.ok_or("No entry configured")?;
        let _params = self.parameters.ok_or("No parameters configured")?;

        entry.validate()?;

        self.validation_passed = true;
        Ok(())
    }

    /// Get entry point
    pub fn entry_point(&self) -> Option<u64> {
        self.entry.map(|e| e.entry_point)
    }

    /// Get entry type
    pub fn entry_type(&self) -> Option<KernelEntryType> {
        self.entry.map(|e| e.entry_type)
    }

    /// Check if ready for transfer
    pub fn is_ready(&self) -> bool {
        self.validation_passed
            && self.entry.is_some()
            && self.parameters.is_some()
    }

    /// Prepare for kernel transfer
    pub fn prepare_transfer(&self) -> Result<KernelTransferInfo, &'static str> {
        let entry = self.entry.ok_or("No entry point")?;
        let params = self.parameters.ok_or("No parameters")?;

        Ok(KernelTransferInfo {
            entry_point: entry.entry_point,
            entry_type: entry.entry_type,
            boot_info_address: params.boot_info_address,
            memory_map_address: params.memory_map_address,
            magic: params.magic,
        })
    }
}

/// Kernel transfer information
#[derive(Debug, Clone, Copy)]
pub struct KernelTransferInfo {
    pub entry_point: u64,
    pub entry_type: KernelEntryType,
    pub boot_info_address: u64,
    pub memory_map_address: u64,
    pub magic: u32,
}

impl KernelTransferInfo {
    pub fn summary(&self) -> String {
        format!(
            "Kernel: EP=0x{:x}, Type={}, Info=0x{:x}",
            self.entry_point,
            self.entry_type.description(),
            self.boot_info_address
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_entry_type_description() {
        assert!(KernelEntryType::Multiboot2
            .description()
            .contains("Multiboot2"));
        assert!(KernelEntryType::ELF64DirectJump.is_supported());
    }

    #[test]
    fn test_kernel_entry_creation() {
        let entry = KernelEntry::new(0x100000, KernelEntryType::Multiboot2);
        assert_eq!(entry.entry_point, 0x100000);
        assert!(entry.is_valid());
    }

    #[test]
    fn test_kernel_entry_not_page_aligned() {
        let entry = KernelEntry::new(0x100001, KernelEntryType::Multiboot2);
        assert!(!entry.is_valid());
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_kernel_entry_zero_address() {
        let entry = KernelEntry::new(0, KernelEntryType::Multiboot2);
        assert!(!entry.is_valid());
    }

    #[test]
    fn test_kernel_entry_params_creation() {
        let params = KernelEntryParams::new(0x2000, 0x3000);
        assert!(params.is_valid());
    }

    #[test]
    fn test_kernel_entry_params_same_address() {
        let params = KernelEntryParams::new(0x2000, 0x2000);
        assert!(!params.is_valid());
    }

    #[test]
    fn test_kernel_entry_handler_creation() {
        let handler = KernelEntryHandler::new();
        assert!(!handler.is_ready());
    }

    #[test]
    fn test_kernel_entry_handler_set_entry() {
        let mut handler = KernelEntryHandler::new();
        let entry = KernelEntry::new(0x100000, KernelEntryType::Multiboot2);

        assert!(handler.set_entry(entry).is_ok());
        assert_eq!(handler.entry_point(), Some(0x100000));
    }

    #[test]
    fn test_kernel_entry_handler_set_parameters() {
        let mut handler = KernelEntryHandler::new();
        let params = KernelEntryParams::new(0x2000, 0x3000);

        assert!(handler.set_parameters(params).is_ok());
    }

    #[test]
    fn test_kernel_entry_handler_validate() {
        let mut handler = KernelEntryHandler::new();
        let entry = KernelEntry::new(0x100000, KernelEntryType::Multiboot2);
        let params = KernelEntryParams::new(0x2000, 0x3000);

        handler.set_entry(entry).unwrap();
        handler.set_parameters(params).unwrap();
        assert!(handler.validate_entry().is_ok());
    }

    #[test]
    fn test_kernel_entry_handler_ready() {
        let mut handler = KernelEntryHandler::new();
        let entry = KernelEntry::new(0x100000, KernelEntryType::Multiboot2);
        let params = KernelEntryParams::new(0x2000, 0x3000);

        handler.set_entry(entry).unwrap();
        handler.set_parameters(params).unwrap();
        handler.validate_entry().unwrap();

        assert!(handler.is_ready());
    }

    #[test]
    fn test_kernel_entry_handler_transfer_info() {
        let mut handler = KernelEntryHandler::new();
        let entry = KernelEntry::new(0x100000, KernelEntryType::Multiboot2);
        let params = KernelEntryParams::new(0x2000, 0x3000);

        handler.set_entry(entry).unwrap();
        handler.set_parameters(params).unwrap();

        let info = handler.prepare_transfer().unwrap();
        assert_eq!(info.entry_point, 0x100000);
    }

    #[test]
    fn test_kernel_transfer_info_summary() {
        let info = KernelTransferInfo {
            entry_point: 0x100000,
            entry_type: KernelEntryType::Multiboot2,
            boot_info_address: 0x2000,
            memory_map_address: 0x3000,
            magic: 0x36D76289,
        };

        let summary = info.summary();
        assert!(summary.contains("0x100000"));
        assert!(summary.contains("Multiboot2"));
    }
}
