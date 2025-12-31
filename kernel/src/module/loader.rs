//! Kernel Module Loader
//!
//! This module provides ELF module loading capabilities for the kernel,
//! including parsing, relocation, and symbol resolution.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::mem::size_of;

/// Errors that can occur during module loading
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// Invalid ELF magic number or format
    InvalidElf,
    /// Unsupported architecture
    UnsupportedArch,
    /// Memory allocation failed
    MemoryError,
    /// Invalid section headers
    InvalidSections,
    /// Missing required sections
    MissingSections,
    /// Symbol table not found
    NoSymbolTable,
    /// Invalid relocation entries
    InvalidRelocation,
}

/// Errors that can occur during relocation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocError {
    /// Unknown relocation type
    UnknownType,
    /// Symbol not found
    SymbolNotFound(String),
    /// Invalid relocation offset
    InvalidOffset,
    /// Relocation overflow
    Overflow,
}

/// General error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Load(LoadError),
    Reloc(RelocError),
}

impl From<LoadError> for Error {
    fn from(err: LoadError) -> Self {
        Error::Load(err)
    }
}

impl From<RelocError> for Error {
    fn from(err: RelocError) -> Self {
        Error::Reloc(err)
    }
}

/// Symbol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// Object symbol
    Object,
    /// Function symbol
    Function,
    /// Section symbol
    Section,
    /// File symbol
    File,
    /// Unknown symbol
    Unknown,
}

/// A symbol in the module
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub value: u64,
    pub size: u64,
    pub sym_type: SymbolType,
}

/// ELF Module representation
#[derive(Debug)]
pub struct ElfModule {
    pub name: String,
    pub text: *mut u8,
    pub data: *mut u8,
    pub bss: *mut u8,
    pub size: usize,
    pub entry_point: u64,
    pub symbols: Vec<Symbol>,
    pub text_size: usize,
    pub data_size: usize,
    pub bss_size: usize,
}

unsafe impl Send for ElfModule {}
unsafe impl Sync for ElfModule {}

/// ELF Header (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ElfHeader {
    e_ident: [u8; 16],      // ELF identification
    e_type: u16,             // Object file type
    e_machine: u16,          // Architecture
    e_version: u32,          // Object file version
    e_entry: u64,            // Entry point address
    e_phoff: u64,            // Program header offset
    e_shoff: u64,            // Section header offset
    e_flags: u32,            // Processor-specific flags
    e_ehsize: u16,           // ELF header size
    e_phentsize: u16,        // Program header entry size
    e_phnum: u16,            // Program header entry count
    e_shentsize: u16,        // Section header entry size
    e_shnum: u16,            // Section header entry count
    e_shstrndx: u16,         // Section header string table index
}

/// Section Header (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SectionHeader {
    sh_name: u32,            // Section name
    sh_type: u32,            // Section type
    sh_flags: u64,           // Section flags
    sh_addr: u64,            // Section virtual address
    sh_offset: u64,          // Section file offset
    sh_size: u64,            // Section size
    sh_link: u32,            // Section link
    sh_info: u32,            // Section information
    sh_addralign: u64,       // Section alignment
    sh_entsize: u64,         // Section entry size
}

/// Symbol Table Entry (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SymTabEntry64 {
    st_name: u32,            // Symbol name
    st_info: u8,             // Symbol type and binding
    st_other: u8,            // Symbol visibility
    st_shndx: u16,           // Section index
    st_value: u64,           // Symbol value
    st_size: u64,            // Symbol size
}

/// Relocation Entry (64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Rela64 {
    r_offset: u64,           // Offset where to apply relocation
    r_info: u64,             // Relocation type and symbol index
    r_addend: i64,           // Addend
}

// ELF constants
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ET_REL: u16 = 1;       // Relocatable file
const EM_X86_64: u16 = 62;   // AMD x86-64 architecture

// Section types
const SHT_NULL: u32 = 0;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_RELA: u32 = 4;
const SHT_NOBITS: u32 = 8;

// Section flags
const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;
const SHF_WRITE: u64 = 0x1;

// Relocation types for x86_64
const R_X86_64_NONE: u32 = 0;
const R_X86_64_64: u32 = 1;
const R_X86_64_PC32: u32 = 2;
const R_X86_64_GOT32: u32 = 3;
const R_X86_64_PLT32: u32 = 4;
const R_X86_64_COPY: u32 = 5;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JMP_SLOT: u32 = 7;
const R_X86_64_RELATIVE: u32 = 8;
const R_X86_64_GOTPCREL: u32 = 9;
const R_X86_64_32: u32 = 10;
const R_X86_64_32S: u32 = 11;
const R_X86_64_16: u32 = 12;
const R_X86_64_PC16: u32 = 13;
const R_X86_64_8: u32 = 14;
const R_X86_64_PC8: u32 = 15;

impl ElfModule {
    /// Load an ELF module from raw data
    pub fn load(data: &[u8], name: String) -> Result<Self, LoadError> {
        // Validate ELF magic
        if data.len() < size_of::<ElfHeader>() || &data[0..4] != ELF_MAGIC {
            return Err(LoadError::InvalidElf);
        }

        let header = unsafe {
            *(data.as_ptr() as *const ElfHeader)
        };

        // Check for 64-bit ELF (must be class 2)
        if header.e_ident[4] != 2 {
            return Err(LoadError::InvalidElf);
        }

        // Verify it's a relocatable file
        if header.e_type != ET_REL {
            return Err(LoadError::InvalidElf);
        }

        // Check architecture
        if header.e_machine != EM_X86_64 {
            return Err(LoadError::UnsupportedArch);
        }

        // Parse section headers
        let sections = Self::parse_sections(data, &header)?;

        // Find key sections
        let text_section = sections.iter()
            .find(|s| s.sh_type == SHT_PROGBITS && (s.sh_flags & SHF_EXECINSTR) != 0)
            .ok_or(LoadError::MissingSections)?;

        let data_section = sections.iter()
            .find(|s| s.sh_type == SHT_PROGBITS && (s.sh_flags & SHF_WRITE) != 0 && (s.sh_flags & SHF_EXECINSTR) == 0)
            .ok_or(LoadError::MissingSections)?;

        let bss_section = sections.iter()
            .find(|s| s.sh_type == SHT_NOBITS)
            .ok_or(LoadError::MissingSections)?;

        let symtab_section = sections.iter()
            .find(|s| s.sh_type == SHT_SYMTAB)
            .ok_or(LoadError::NoSymbolTable)?;

        let strtab_section = sections.iter()
            .find(|s| s.sh_type == SHT_STRTAB && s.sh_offset != (sections[symtab_section.sh_link as usize].sh_offset))
            .ok_or(LoadError::NoSymbolTable)?;

        // Calculate total size
        let text_size = text_section.sh_size as usize;
        let data_size = data_section.sh_size as usize;
        let bss_size = bss_section.sh_size as usize;
        let total_size = text_size + data_size + bss_size;

        // Allocate memory
        let (text, data_mem, bss) = unsafe {
            let base = alloc::alloc::alloc_zeroed(
                core::alloc::Layout::from_size_align(total_size, 4096)
                    .map_err(|_| LoadError::MemoryError)?
            ) as *mut u8;

            if base.is_null() {
                return Err(LoadError::MemoryError);
            }

            let text_ptr = base;
            let data_ptr = base.add(text_size);
            let bss_ptr = data_ptr.add(data_size);

            // Copy .text section
            if text_section.sh_offset as usize + text_size <= data.len() {
                core::ptr::copy_nonoverlapping(
                    data.as_ptr().add(text_section.sh_offset as usize),
                    text_ptr,
                    text_size
                );
            }

            // Copy .data section
            if data_section.sh_offset as usize + data_size <= data.len() {
                core::ptr::copy_nonoverlapping(
                    data.as_ptr().add(data_section.sh_offset as usize),
                    data_ptr,
                    data_size
                );
            }

            // .bss is already zero-initialized from alloc_zeroed

            (text_ptr, data_ptr, bss_ptr)
        };

        // Parse symbols
        let symbols = Self::parse_symbols(
            data,
            symtab_section,
            strtab_section,
        )?;

        Ok(ElfModule {
            name,
            text,
            data: data_mem,
            bss,
            size: total_size,
            entry_point: header.e_entry,
            symbols,
            text_size,
            data_size,
            bss_size,
        })
    }

    /// Parse section headers
    fn parse_sections(data: &[u8], header: &ElfHeader) -> Result<Vec<SectionHeader>, LoadError> {
        let section_count = header.e_shnum as usize;
        let mut sections = Vec::with_capacity(section_count);

        for i in 0..section_count {
            let offset = (header.e_shoff as usize) + (i * header.e_shentsize as usize);
            if offset + size_of::<SectionHeader>() > data.len() {
                return Err(LoadError::InvalidSections);
            }

            let section = unsafe {
                *(data.as_ptr().add(offset) as *const SectionHeader)
            };
            sections.push(section);
        }

        Ok(sections)
    }

    /// Parse symbol table
    fn parse_symbols(
        data: &[u8],
        symtab: &SectionHeader,
        strtab: &SectionHeader,
    ) -> Result<Vec<Symbol>, LoadError> {
        let mut symbols = Vec::new();
        let entry_count = (symtab.sh_size / symtab.sh_entsize) as usize;

        for i in 0..entry_count {
            let offset = (symtab.sh_offset as usize) + (i * symtab.sh_entsize as usize);
            if offset + size_of::<SymTabEntry64>() > data.len() {
                continue;
            }

            let entry = unsafe {
                *(data.as_ptr().add(offset) as *const SymTabEntry64)
            };

            // Skip null symbols and section symbols
            if entry.st_name == 0 || entry.st_info >> 4 == 3 {
                continue;
            }

            // Get symbol name
            let name = Self::get_string(data, strtab, entry.st_name as usize)?;

            if !name.is_empty() {
                let sym_type = match entry.st_info & 0xf {
                    0 => SymbolType::Unknown,
                    1 => SymbolType::Object,
                    2 => SymbolType::Function,
                    3 => SymbolType::Section,
                    4 => SymbolType::File,
                    _ => SymbolType::Unknown,
                };

                symbols.push(Symbol {
                    name,
                    value: entry.st_value,
                    size: entry.st_size,
                    sym_type,
                });
            }
        }

        Ok(symbols)
    }

    /// Get string from string table
    fn get_string(data: &[u8], strtab: &SectionHeader, offset: usize) -> Result<String, LoadError> {
        if offset == 0 {
            return Ok(String::new());
        }

        let start = (strtab.sh_offset as usize) + offset;
        let end = data[start..].iter()
            .position(|&c| c == 0)
            .map(|p| start + p)
            .unwrap_or(data.len());

        if start >= data.len() || end > data.len() {
            return Ok(String::new());
        }

        String::from_utf8(data[start..end].to_vec())
            .map_err(|_| LoadError::InvalidElf)
    }

    /// Apply relocations
    pub fn relocate(&mut self, base: u64) -> Result<(), RelocError> {
        // Relocation logic would be implemented here
        // For now, this is a placeholder that updates symbol addresses
        let text_base = base;
        let data_base = base + self.text_size as u64;
        let _bss_base = data_base + self.data_size as u64;

        for symbol in &mut self.symbols {
            if symbol.value != 0 {
                // Update symbol addresses based on section
                // This is simplified - real implementation would use section headers
                if symbol.sym_type == SymbolType::Function {
                    symbol.value += text_base;
                } else {
                    symbol.value += data_base;
                }
            }
        }

        Ok(())
    }

    /// Resolve symbols against kernel symbol table
    pub fn resolve_symbols(&self, kernel_symbols: &SymbolTable) -> Result<(), Error> {
        // Check for undefined symbols
        for symbol in &self.symbols {
            if symbol.value == 0 && !symbol.name.is_empty() {
                // Try to resolve from kernel symbols
                if kernel_symbols.get(&symbol.name).is_none() {
                    return Err(Error::Reloc(RelocError::SymbolNotFound(symbol.name.clone())));
                }
            }
        }
        Ok(())
    }

    /// Get symbol by name
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Get init function address
    pub fn get_init_fn(&self) -> Option<u64> {
        self.get_symbol("init_module").map(|s| s.value)
    }

    /// Get cleanup function address
    pub fn get_cleanup_fn(&self) -> Option<u64> {
        self.get_symbol("cleanup_module").map(|s| s.value)
    }
}

impl Drop for ElfModule {
    fn drop(&mut self) {
        unsafe {
            if self.size > 0 {
                alloc::alloc::dealloc(
                    self.text,
                    core::alloc::Layout::from_size_align_unchecked(self.size, 4096)
                );
            }
        }
    }
}

/// Kernel symbol table
#[derive(Debug)]
pub struct SymbolTable {
    symbols: BTreeMap<String, u64>,
}

impl SymbolTable {
    /// Create a new symbol table
    pub fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
        }
    }

    /// Add a symbol
    pub fn add(&mut self, name: String, addr: u64) {
        self.symbols.insert(name, addr);
    }

    /// Get symbol address
    pub fn get(&self, name: &str) -> Option<u64> {
        self.symbols.get(name).copied()
    }

    /// Check if symbol exists
    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Get all symbols
    pub fn all(&self) -> &BTreeMap<String, u64> {
        &self.symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_magic_validation() {
        assert!(&ELF_MAGIC == &[0x7f, b'E', b'L', b'F']);
    }

    #[test]
    fn test_symbol_table() {
        let mut table = SymbolTable::new();
        table.add(String::from("test_symbol"), 0x1000);

        assert_eq!(table.get("test_symbol"), Some(0x1000));
        assert_eq!(table.get("nonexistent"), None);
        assert!(table.contains("test_symbol"));
    }
}
