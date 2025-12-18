/// Boot Information Builder
///
/// Constructs complete boot information structure for kernel.
/// Handles memory maps, modules, command lines, and boot parameters.

use alloc::vec::Vec;
use alloc::string::String;

/// Memory map entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Available,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
}

impl MemoryType {
    pub fn code(&self) -> u32 {
        match self {
            Self::Available => 1,
            Self::Reserved => 2,
            Self::AcpiReclaimable => 3,
            Self::AcpiNvs => 4,
            Self::BadMemory => 5,
        }
    }
}

/// Memory map entry for boot info
#[derive(Debug, Clone, Copy)]
pub struct BootMemoryEntry {
    pub base: u64,
    pub length: u64,
    pub mtype: MemoryType,
}

impl BootMemoryEntry {
    pub fn new(base: u64, length: u64, mtype: MemoryType) -> Self {
        Self { base, length, mtype }
    }

    pub fn is_usable(&self) -> bool {
        self.mtype == MemoryType::Available && self.length > 0
    }

    pub fn end_address(&self) -> u64 {
        self.base.saturating_add(self.length)
    }
}

/// Boot module
#[derive(Debug, Clone)]
pub struct BootModule {
    pub start: u64,
    pub end: u64,
    pub string: String,
}

impl BootModule {
    pub fn new(start: u64, end: u64, string: String) -> Self {
        Self { start, end, string }
    }

    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

/// Boot information builder
pub struct BootInfoBuilder {
    memory_entries: Vec<BootMemoryEntry>,
    modules: Vec<BootModule>,
    command_line: String,
    bootloader_name: &'static str,
    flags: u32,
}

impl BootInfoBuilder {
    /// Create new boot info builder
    pub fn new() -> Self {
        Self {
            memory_entries: Vec::new(),
            modules: Vec::new(),
            command_line: String::new(),
            bootloader_name: "NOS Bootloader",
            flags: 0,
        }
    }

    /// Add memory entry
    pub fn add_memory_entry(&mut self, entry: BootMemoryEntry) -> Result<(), &'static str> {
        if self.memory_entries.len() >= 256 {
            return Err("Memory entries full");
        }

        self.memory_entries.push(entry);
        Ok(())
    }

    /// Add boot module
    pub fn add_module(&mut self, module: BootModule) -> Result<(), &'static str> {
        if self.modules.len() >= 16 {
            return Err("Module list full");
        }

        self.modules.push(module);
        Ok(())
    }

    /// Set command line
    pub fn set_command_line(&mut self, cmdline: &str) {
        self.command_line.clear();
        self.command_line.push_str(cmdline);
    }

    /// Set flag
    pub fn set_flag(&mut self, flag: u32) {
        self.flags |= flag;
    }

    /// Clear flag
    pub fn clear_flag(&mut self, flag: u32) {
        self.flags &= !flag;
    }

    /// Build boot information
    pub fn build(&self) -> BootInfo {
        let total_memory: u64 = self
            .memory_entries
            .iter()
            .filter(|e| e.is_usable())
            .map(|e| e.length)
            .sum();

        let total_module_size: u64 = self.modules.iter().map(|m| m.size()).sum();

        BootInfo {
            memory_entries: self.memory_entries.clone(),
            modules: self.modules.clone(),
            command_line: self.command_line.clone(),
            bootloader_name: self.bootloader_name,
            flags: self.flags,
            total_memory,
            total_module_size,
        }
    }

    /// Get memory entries count
    pub fn memory_count(&self) -> usize {
        self.memory_entries.len()
    }

    /// Get modules count
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get total usable memory
    pub fn total_memory(&self) -> u64 {
        self.memory_entries
            .iter()
            .filter(|e| e.is_usable())
            .map(|e| e.length)
            .sum()
    }
}

/// Complete boot information
#[derive(Debug, Clone)]
pub struct BootInfo {
    pub memory_entries: Vec<BootMemoryEntry>,
    pub modules: Vec<BootModule>,
    pub command_line: String,
    pub bootloader_name: &'static str,
    pub flags: u32,
    pub total_memory: u64,
    pub total_module_size: u64,
}

impl BootInfo {
    /// Check if boot info is valid
    pub fn is_valid(&self) -> bool {
        !self.memory_entries.is_empty() && self.total_memory > 0
    }

    /// Get first usable memory entry
    pub fn first_usable(&self) -> Option<&BootMemoryEntry> {
        self.memory_entries.iter().find(|e| e.is_usable())
    }

    /// Get highest usable address
    pub fn highest_address(&self) -> u64 {
        self.memory_entries
            .iter()
            .filter(|e| e.is_usable())
            .map(|e| e.end_address())
            .max()
            .unwrap_or(0)
    }

    /// Get available memory for kernel
    pub fn available_for_kernel(&self) -> u64 {
        self.total_memory.saturating_sub(self.total_module_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_codes() {
        assert_eq!(MemoryType::Available.code(), 1);
        assert_eq!(MemoryType::Reserved.code(), 2);
    }

    #[test]
    fn test_boot_memory_entry_creation() {
        let entry = BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available);
        assert_eq!(entry.base, 0x1000);
        assert_eq!(entry.length, 0x10000);
    }

    #[test]
    fn test_boot_memory_entry_is_usable() {
        let entry = BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available);
        assert!(entry.is_usable());

        let reserved = BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Reserved);
        assert!(!reserved.is_usable());
    }

    #[test]
    fn test_boot_memory_entry_end_address() {
        let entry = BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available);
        assert_eq!(entry.end_address(), 0x11000);
    }

    #[test]
    fn test_boot_module_creation() {
        let module = BootModule::new(0x100000, 0x110000, "initrd".into());
        assert_eq!(module.start, 0x100000);
        assert_eq!(module.size(), 0x10000);
    }

    #[test]
    fn test_boot_info_builder_creation() {
        let builder = BootInfoBuilder::new();
        assert_eq!(builder.memory_count(), 0);
        assert_eq!(builder.module_count(), 0);
    }

    #[test]
    fn test_boot_info_builder_add_memory() {
        let mut builder = BootInfoBuilder::new();
        let entry = BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available);

        assert!(builder.add_memory_entry(entry).is_ok());
        assert_eq!(builder.memory_count(), 1);
    }

    #[test]
    fn test_boot_info_builder_total_memory() {
        let mut builder = BootInfoBuilder::new();
        builder
            .add_memory_entry(BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available))
            .unwrap();
        builder
            .add_memory_entry(BootMemoryEntry::new(0x20000, 0x20000, MemoryType::Available))
            .unwrap();

        assert_eq!(builder.total_memory(), 0x30000);
    }

    #[test]
    fn test_boot_info_builder_build() {
        let mut builder = BootInfoBuilder::new();
        builder
            .add_memory_entry(BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available))
            .unwrap();

        let info = builder.build();
        assert!(info.is_valid());
        assert_eq!(info.total_memory, 0x10000);
    }

    #[test]
    fn test_boot_info_highest_address() {
        let mut builder = BootInfoBuilder::new();
        builder
            .add_memory_entry(BootMemoryEntry::new(0x1000, 0x10000, MemoryType::Available))
            .unwrap();
        builder
            .add_memory_entry(BootMemoryEntry::new(0x100000, 0x100000, MemoryType::Available))
            .unwrap();

        let info = builder.build();
        assert_eq!(info.highest_address(), 0x200000);
    }

    #[test]
    fn test_boot_info_builder_command_line() {
        let mut builder = BootInfoBuilder::new();
        builder.set_command_line("console=ttyS0 debug");

        let info = builder.build();
        assert!(info.command_line.contains("console"));
    }

    #[test]
    fn test_boot_info_builder_flags() {
        let mut builder = BootInfoBuilder::new();
        builder.set_flag(0x0001);
        builder.set_flag(0x0002);

        let info = builder.build();
        assert_eq!(info.flags, 0x0003);
    }
}
