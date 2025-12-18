//! ELF64 format support
//!
//! This module provides structures and functions for working with ELF64 files,
//! which is the standard executable format for 64-bit Unix-like systems.

/// ELF64 header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ElfHeader {
    /// Magic number (0x7F 'E' 'L' 'F')
    pub magic: [u8; 4],
    /// File class (1=32-bit, 2=64-bit)
    pub class: u8,
    /// Data encoding (1=little endian, 2=big endian)
    pub data: u8,
    /// ELF version
    pub version: u8,
    /// OS/ABI identification
    pub os_abi: u8,
    /// ABI version
    pub abi_version: u8,
    /// Padding
    pub padding: [u8; 7],
    /// File type
    pub file_type: u16,
    /// Machine architecture
    pub machine: u16,
    /// ELF version
    pub elf_version: u32,
    /// Entry point virtual address
    pub entry_point: u64,
    /// Program header table file offset
    pub program_header_offset: u64,
    /// Section header table file offset
    pub section_header_offset: u64,
    /// Processor-specific flags
    pub flags: u32,
    /// ELF header size
    pub header_size: u16,
    /// Program header entry size
    pub program_header_size: u16,
    /// Program header entry count
    pub program_header_count: u16,
    /// Section header entry size
    pub section_header_size: u16,
    /// Section header entry count
    pub section_header_count: u16,
    /// Section header string table index
    pub section_header_string_index: u16,
}

/// ELF64 program header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProgramHeader {
    /// Segment type
    pub segment_type: u32,
    /// Segment flags
    pub flags: u32,
    /// File offset
    pub file_offset: u64,
    /// Virtual address
    pub virtual_address: u64,
    /// Physical address
    pub physical_address: u64,
    /// Segment size in file
    pub segment_size: u64,
    /// Segment size in memory
    pub mem_size: u64,
    /// Segment alignment
    pub alignment: u64,
}

/// ELF64 section header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SectionHeader {
    /// Section name (index into string table)
    pub name: u32,
    /// Section type
    pub section_type: u32,
    /// Section flags
    pub flags: u64,
    /// Virtual address
    pub virtual_address: u64,
    /// File offset
    pub file_offset: u64,
    /// Section size
    pub size: u64,
    /// Section link
    pub link: u32,
    /// Section info
    pub info: u32,
    /// Section alignment
    pub alignment: u64,
    /// Entry size
    pub entry_size: u64,
}

/// ELF64 symbol table entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Symbol {
    /// Symbol name (index into string table)
    pub name: u32,
    /// Symbol info and type
    pub info: u8,
    /// Symbol visibility
    pub other: u8,
    /// Section index
    pub section_index: u16,
    /// Symbol value
    pub value: u64,
    /// Symbol size
    pub size: u64,
}

/// ELF64 relocation entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Relocation {
    /// Relocation offset
    pub offset: u64,
    /// Relocation info and type
    pub info: u64,
    /// Symbol index
    pub symbol: u32,
    /// Relocation type
    pub rel_type: u32,
    /// Addend
    pub addend: i64,
}

/// ELF64 dynamic entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Dynamic {
    /// Dynamic entry tag
    pub tag: u64,
    /// Value or pointer
    pub value: u64,
}

/// ELF constants
pub mod constants {
    /// ELF magic number
    pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
    
    /// ELF classes
    pub const ELFCLASS32: u8 = 1;
    pub const ELFCLASS64: u8 = 2;
    
    /// ELF data encodings
    pub const ELFDATA2LSB: u8 = 1; // Little endian
    pub const ELFDATA2MSB: u8 = 2; // Big endian
    
    /// ELF versions
    pub const EV_NONE: u8 = 0;
    pub const EV_CURRENT: u8 = 1;
    
    /// ELF file types
    pub const ET_NONE: u16 = 0; // No file type
    pub const ET_REL: u16 = 1; // Relocatable file
    pub const ET_EXEC: u16 = 2; // Executable file
    pub const ET_DYN: u16 = 3; // Shared object file
    pub const ET_CORE: u16 = 4; // Core file
    
    /// ELF machine types
    pub const EM_NONE: u16 = 0; // No machine
    pub const EM_386: u16 = 3; // Intel 80386
    pub const EM_X86_64: u16 = 62; // AMD x86-64 architecture
    pub const EM_AARCH64: u16 = 183; // ARM AARCH64
    pub const EM_RISCV: u16 = 243; // RISC-V
    
    /// Program header types
    pub const PT_NULL: u32 = 0; // Unused entry
    pub const PT_LOAD: u32 = 1; // Loadable segment
    pub const PT_DYNAMIC: u32 = 2; // Dynamic linking information
    pub const PT_INTERP: u32 = 3; // Program interpreter
    pub const PT_NOTE: u32 = 4; // Auxiliary information
    pub const PT_SHLIB: u32 = 5; // Reserved
    pub const PT_PHDR: u32 = 6; // Program header table
    pub const PT_TLS: u32 = 7; // Thread-local storage
    pub const PT_GNU_EH_FRAME: u32 = 0x6474e550; // GCC .eh_frame_hdr segment
    pub const PT_GNU_STACK: u32 = 0x6474e551; // Indicates stack executability
    pub const PT_GNU_RELRO: u32 = 0x6474e552; // Read-only after relocation
    
    /// Program header flags
    pub const PF_X: u32 = 0x1; // Executable
    pub const PF_W: u32 = 0x2; // Writable
    pub const PF_R: u32 = 0x4; // Readable
    
    /// Section header types
    pub const SHT_NULL: u32 = 0; // Inactive section
    pub const SHT_PROGBITS: u32 = 1; // Program data
    pub const SHT_SYMTAB: u32 = 2; // Symbol table
    pub const SHT_STRTAB: u32 = 3; // String table
    pub const SHT_RELA: u32 = 4; // Relocation entries with addends
    pub const SHT_HASH: u32 = 5; // Symbol hash table
    pub const SHT_DYNAMIC: u32 = 6; // Dynamic linking information
    pub const SHT_NOTE: u32 = 7; // Notes
    pub const SHT_NOBITS: u32 = 8; // Program space with no data (bss)
    pub const SHT_REL: u32 = 9; // Relocation entries, no addends
    pub const SHT_SHLIB: u32 = 10; // Reserved
    pub const SHT_DYNSYM: u32 = 11; // Dynamic linker symbol table
    pub const SHT_INIT_ARRAY: u32 = 14; // Array of constructors
    pub const SHT_FINI_ARRAY: u32 = 15; // Array of destructors
    pub const SHT_GNU_HASH: u32 = 0x6ffffff6; // GNU-style hash table
    pub const SHT_GNU_VERDEF: u32 = 0x6ffffffd; // Version definition section
    pub const SHT_GNU_VERNEED: u32 = 0x6ffffffe; // Version needs section
    pub const SHT_GNU_VERSYM: u32 = 0x6fffffff; // Version symbol table
    
    /// Section header flags
    pub const SHF_WRITE: u64 = 0x1; // Writable
    pub const SHF_ALLOC: u64 = 0x2; // Occupies memory during execution
    pub const SHF_EXECINSTR: u64 = 0x4; // Executable
    pub const SHF_MERGE: u64 = 0x10; // Might be merged
    pub const SHF_STRINGS: u64 = 0x20; // Contains nul-terminated strings
    pub const SHF_INFO_LINK: u64 = 0x40; // 'sh_info' contains SHT index
    pub const SHF_LINK_ORDER: u64 = 0x80; // Preserve order after combining
    pub const SHF_OS_NONCONFORMING: u64 = 0x100; // Non-standard OS specific handling
    pub const SHF_GROUP: u64 = 0x200; // Section is member of a group
    pub const SHF_TLS: u64 = 0x400; // Section hold thread-local data
    pub const SHF_COMPRESSED: u64 = 0x800; // Section with compressed data
    
    /// Symbol types
    pub const STT_NOTYPE: u8 = 0; // Symbol type is unspecified
    pub const STT_OBJECT: u8 = 1; // Symbol is a data object
    pub const STT_FUNC: u8 = 2; // Symbol is a code object
    pub const STT_SECTION: u8 = 3; // Symbol associated with a section
    pub const STT_FILE: u8 = 4; // Symbol's name is file name
    pub const STT_COMMON: u8 = 5; // Symbol is a common data object
    pub const STT_TLS: u8 = 6; // Symbol is thread-local data object
    pub const STT_GNU_IFUNC: u8 = 10; // Symbol is indirect code object
    
    /// Symbol bindings
    pub const STB_LOCAL: u8 = 0; // Local symbol
    pub const STB_GLOBAL: u8 = 1; // Global symbol
    pub const STB_WEAK: u8 = 2; // Weak symbol
    pub const STB_GNU_UNIQUE: u8 = 10; // Unique symbol
    
    /// Symbol visibility
    pub const STV_DEFAULT: u8 = 0; // Default symbol visibility rules
    pub const STV_INTERNAL: u8 = 1; // Processor specific hidden class
    pub const STV_HIDDEN: u8 = 2; // Sym unavailable in other modules
    pub const STV_PROTECTED: u8 = 3; // Not preemptible, not exported
    
    /// Dynamic tags
    pub const DT_NULL: u64 = 0; // Marks end of dynamic section
    pub const DT_NEEDED: u64 = 1; // Name of needed library
    pub const DT_PLTRELSZ: u64 = 2; // Size in bytes of PLT relocs
    pub const DT_PLTGOT: u64 = 3; // Processor defined value
    pub const DT_HASH: u64 = 4; // Address of symbol hash table
    pub const DT_STRTAB: u64 = 5; // Address of string table
    pub const DT_SYMTAB: u64 = 6; // Address of symbol table
    pub const DT_RELA: u64 = 7; // Address of Rela relocs
    pub const DT_RELASZ: u64 = 8; // Total size of Rela relocs
    pub const DT_RELAENT: u64 = 9; // Size of one Rela reloc
    pub const DT_STRSZ: u64 = 10; // Size of string table
    pub const DT_SYMENT: u64 = 11; // Size of one symbol table entry
    pub const DT_INIT: u64 = 12; // Address of init function
    pub const DT_FINI: u64 = 13; // Address of termination function
    pub const DT_SONAME: u64 = 14; // Name of shared object
    pub const DT_RPATH: u64 = 15; // Library search path (deprecated)
    pub const DT_SYMBOLIC: u64 = 16; // Start symbol search here
    pub const DT_REL: u64 = 17; // Address of Rel relocs
    pub const DT_RELSZ: u64 = 18; // Total size of Rel relocs
    pub const DT_RELENT: u64 = 19; // Size of one Rel reloc
    pub const DT_PLTREL: u64 = 20; // Type of reloc in PLT
    pub const DT_DEBUG: u64 = 21; // For debugging; unspecified
    pub const DT_TEXTREL: u64 = 22; // Reloc might modify .text
    pub const DT_JMPREL: u64 = 23; // Address of PLT relocs
    pub const DT_BIND_NOW: u64 = 24; // Process relocations of object
    pub const DT_INIT_ARRAY: u64 = 25; // Array with addresses of init fct
    pub const DT_FINI_ARRAY: u64 = 26; // Array with addresses of fini fct
    pub const DT_INIT_ARRAYSZ: u64 = 27; // Size in bytes of DT_INIT_ARRAY
    pub const DT_FINI_ARRAYSZ: u64 = 28; // Size in bytes of DT_FINI_ARRAY
    pub const DT_RUNPATH: u64 = 29; // Library search path
    pub const DT_FLAGS: u64 = 30; // Flags for the object being loaded
}