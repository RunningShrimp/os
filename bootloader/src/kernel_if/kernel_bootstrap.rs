//! Kernel Bootstrap - ELF64 Kernel Environment Setup
//!
//! Handles kernel environment setup including:
//! - ELF64 image parsing and validation
//! - Entry point configuration
//! - Register and memory state preparation
//! - Kernel execution environment initialization

use core::fmt;
use alloc::string::String;
use alloc::format;

/// ELF header magic constant
pub const ELF_MAGIC: u32 = 0x464C457F; // 0x7F 'E' 'L' 'F'

/// Kernel architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelArch {
    X86_64,
    AArch64,
    RiscV64,
    Unknown,
}

impl fmt::Display for KernelArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelArch::X86_64 => write!(f, "x86-64"),
            KernelArch::AArch64 => write!(f, "ARM64"),
            KernelArch::RiscV64 => write!(f, "RISC-V 64"),
            KernelArch::Unknown => write!(f, "Unknown"),
        }
    }
}

/// ELF class (32-bit or 64-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    ELF32,
    ELF64,
    Unknown,
}

impl ElfClass {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => ElfClass::ELF32,
            2 => ElfClass::ELF64,
            _ => ElfClass::Unknown,
        }
    }
}

impl fmt::Display for ElfClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElfClass::ELF32 => write!(f, "ELF32"),
            ElfClass::ELF64 => write!(f, "ELF64"),
            ElfClass::Unknown => write!(f, "Unknown"),
        }
    }
}

/// ELF endianness
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfEndian {
    Little,
    Big,
    Unknown,
}

impl ElfEndian {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => ElfEndian::Little,
            2 => ElfEndian::Big,
            _ => ElfEndian::Unknown,
        }
    }
}

impl fmt::Display for ElfEndian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElfEndian::Little => write!(f, "Little-Endian"),
            ElfEndian::Big => write!(f, "Big-Endian"),
            ElfEndian::Unknown => write!(f, "Unknown"),
        }
    }
}

/// ELF header
#[derive(Debug, Clone)]
pub struct ElfHeader {
    pub magic: u32,
    pub elf_class: ElfClass,
    pub endian: ElfEndian,
    pub version: u8,
    pub os_abi: u8,
    pub abi_version: u8,
    pub e_type: u16,          // File type
    pub e_machine: u16,       // Machine type
    pub e_version: u32,       // Version
    pub e_entry: u64,         // Entry point
    pub e_phoff: u64,         // Program header offset
    pub e_shoff: u64,         // Section header offset
    pub e_flags: u32,         // Flags
    pub e_ehsize: u16,        // ELF header size
    pub e_phentsize: u16,     // Program header size
    pub e_phnum: u16,         // Number of program headers
    pub e_shentsize: u16,     // Section header size
    pub e_shnum: u16,         // Number of section headers
    pub e_shstrndx: u16,      // Section name string table index
}

impl ElfHeader {
    /// Create new ELF header
    pub fn new() -> Self {
        ElfHeader {
            magic: 0,
            elf_class: ElfClass::Unknown,
            endian: ElfEndian::Unknown,
            version: 0,
            os_abi: 0,
            abi_version: 0,
            e_type: 0,
            e_machine: 0,
            e_version: 0,
            e_entry: 0,
            e_phoff: 0,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: 0,
            e_phentsize: 0,
            e_phnum: 0,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        }
    }

    /// Validate ELF header
    pub fn is_valid(&self) -> bool {
        self.magic == ELF_MAGIC && self.elf_class == ElfClass::ELF64
    }

    /// Get architecture
    pub fn get_architecture(&self) -> KernelArch {
        match self.e_machine {
            0x3E => KernelArch::X86_64,    // x86-64
            0xB7 => KernelArch::AArch64,   // ARM64
            0xF3 => KernelArch::RiscV64,   // RISC-V
            _ => KernelArch::Unknown,
        }
    }

    /// Check if executable
    pub fn is_executable(&self) -> bool {
        self.e_type == 2 // ET_EXEC
    }
}

impl fmt::Display for ElfHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ELF({}, {}, Entry: 0x{:x})",
            self.elf_class, self.get_architecture(), self.e_entry
        )
    }
}

/// CPU register state at kernel entry
#[derive(Debug, Clone)]
pub struct RegisterState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rflags: u64,
    pub rip: u64,
    pub cr0: u64,
    pub cr3: u64,
    pub cr4: u64,
}

impl RegisterState {
    /// Create new register state with defaults
    pub fn new() -> Self {
        RegisterState {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rsp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0x202,  // Interrupts enabled
            rip: 0,
            cr0: 0x80000011, // PE + WP
            cr3: 0,
            cr4: 0x20,      // PSE enabled
        }
    }

    /// Set kernel entry point
    pub fn set_entry_point(&mut self, address: u64) {
        self.rip = address;
    }

    /// Set kernel stack
    pub fn set_stack(&mut self, stack_top: u64) {
        self.rsp = stack_top;
    }

    /// Set kernel arguments
    pub fn set_args(&mut self, arg1: u64, arg2: u64, arg3: u64) {
        self.rdi = arg1;
        self.rsi = arg2;
        self.rdx = arg3;
    }

    /// Set page table
    pub fn set_page_table(&mut self, pml4: u64) {
        self.cr3 = pml4;
    }

    /// Enable SSE
    pub fn enable_sse(&mut self) {
        self.cr4 |= 0x2;
    }

    /// Enable AVX
    pub fn enable_avx(&mut self) {
        self.cr4 |= 0x40000;
    }
}

impl fmt::Display for RegisterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RegisterState {{ RIP: 0x{:x}, RSP: 0x{:x}, CR3: 0x{:x} }}",
            self.rip, self.rsp, self.cr3
        )
    }
}

/// Boot arguments for kernel
#[derive(Debug, Clone)]
pub struct KernelBootArgs {
    pub boot_loader_name: u32,     // Pointer to boot loader name
    pub mmap_addr: u32,            // Memory map address
    pub mmap_length: u32,          // Memory map length
    pub drives_addr: u32,          // Drives address
    pub drives_length: u32,        // Drives length
    pub config_table: u32,         // Config table address
    pub boot_loader_version: u32,  // Bootloader version
    pub symbol_table: u32,         // Symbol table address
    pub reserved1: u32,
}

impl KernelBootArgs {
    /// Create new boot arguments
    pub fn new() -> Self {
        KernelBootArgs {
            boot_loader_name: 0,
            mmap_addr: 0,
            mmap_length: 0,
            drives_addr: 0,
            drives_length: 0,
            config_table: 0,
            boot_loader_version: 0x00010000, // Version 1.0
            symbol_table: 0,
            reserved1: 0,
        }
    }

    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.boot_loader_version > 0
    }
}

impl fmt::Display for KernelBootArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KernelBootArgs {{ loader_version: 0x{:x}, mmap: 0x{:x}+{} }}",
            self.boot_loader_version, self.mmap_addr, self.mmap_length
        )
    }
}

/// Kernel Bootstrap Manager
pub struct KernelBootstrap {
    elf_header: Option<ElfHeader>,
    register_state: RegisterState,
    boot_args: KernelBootArgs,
    entry_point: u64,
    is_prepared: bool,
    kernel_arch: KernelArch,
}

impl KernelBootstrap {
    /// Create new kernel bootstrap manager
    pub fn new() -> Self {
        KernelBootstrap {
            elf_header: None,
            register_state: RegisterState::new(),
            boot_args: KernelBootArgs::new(),
            entry_point: 0,
            is_prepared: false,
            kernel_arch: KernelArch::Unknown,
        }
    }

    /// Load ELF header
    pub fn load_elf_header(&mut self, header: ElfHeader) -> bool {
        if !header.is_valid() {
            return false;
        }

        self.kernel_arch = header.get_architecture();
        self.entry_point = header.e_entry;
        self.elf_header = Some(header);
        true
    }

    /// Get ELF header
    pub fn get_elf_header(&self) -> Option<&ElfHeader> {
        self.elf_header.as_ref()
    }

    /// Get kernel architecture
    pub fn get_kernel_architecture(&self) -> KernelArch {
        self.kernel_arch
    }

    /// Set kernel entry point
    pub fn set_entry_point(&mut self, address: u64) {
        self.entry_point = address;
        self.register_state.set_entry_point(address);
    }

    /// Set kernel stack
    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.register_state.set_stack(stack_top);
    }

    /// Set kernel arguments
    pub fn set_kernel_args(&mut self, arg1: u64, arg2: u64, arg3: u64) {
        self.register_state.set_args(arg1, arg2, arg3);
    }

    /// Set page table
    pub fn set_page_table(&mut self, pml4: u64) {
        self.register_state.set_page_table(pml4);
    }

    /// Prepare CPU features
    pub fn prepare_cpu_features(&mut self, enable_sse: bool, enable_avx: bool) {
        if enable_sse {
            self.register_state.enable_sse();
        }
        if enable_avx {
            self.register_state.enable_avx();
        }
    }

    /// Set boot arguments
    pub fn set_boot_arguments(&mut self, args: KernelBootArgs) {
        self.boot_args = args;
    }

    /// Get register state
    pub fn get_register_state(&self) -> &RegisterState {
        &self.register_state
    }

    /// Get boot arguments
    pub fn get_boot_arguments(&self) -> &KernelBootArgs {
        &self.boot_args
    }

    /// Prepare kernel environment
    pub fn prepare_kernel_environment(&mut self) -> bool {
        if self.entry_point == 0 {
            return false;
        }

        if self.register_state.rsp == 0 {
            return false;
        }

        if !self.boot_args.is_valid() {
            return false;
        }

        self.is_prepared = true;
        true
    }

    /// Check if kernel is prepared
    pub fn is_kernel_prepared(&self) -> bool {
        self.is_prepared
    }

    /// Get entry point
    pub fn get_entry_point(&self) -> u64 {
        self.entry_point
    }

    /// Get detailed bootstrap report
    pub fn bootstrap_report(&self) -> String {
        let mut report = String::from("=== Kernel Bootstrap Report ===\n");

        if let Some(header) = &self.elf_header {
            report.push_str(&format!("ELF Header: {}\n", header));
            report.push_str(&format!("Architecture: {}\n", self.kernel_arch));
            report.push_str(&format!("Class: {}\n", header.elf_class));
            report.push_str(&format!("Endian: {}\n", header.endian));
            report.push_str(&format!("Program Headers: {}\n", header.e_phnum));
        }

        report.push_str(&format!("\nEntry Point: 0x{:x}\n", self.entry_point));
        report.push_str(&format!("{}\n", self.register_state));
        report.push_str(&format!("\n{}\n", self.boot_args));
        report.push_str(&format!("Kernel Prepared: {}\n", self.is_prepared));

        report
    }
}

impl fmt::Display for KernelBootstrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KernelBootstrap {{ arch: {}, entry: 0x{:x}, prepared: {} }}",
            self.kernel_arch, self.entry_point, self.is_prepared
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_class_from_u8() {
        assert_eq!(ElfClass::from_u8(1), ElfClass::ELF32);
        assert_eq!(ElfClass::from_u8(2), ElfClass::ELF64);
        assert_eq!(ElfClass::from_u8(99), ElfClass::Unknown);
    }

    #[test]
    fn test_elf_endian_from_u8() {
        assert_eq!(ElfEndian::from_u8(1), ElfEndian::Little);
        assert_eq!(ElfEndian::from_u8(2), ElfEndian::Big);
        assert_eq!(ElfEndian::from_u8(99), ElfEndian::Unknown);
    }

    #[test]
    fn test_elf_header_creation() {
        let header = ElfHeader::new();
        assert_eq!(header.magic, 0);
        assert!(!header.is_valid());
    }

    #[test]
    fn test_elf_header_validity() {
        let mut header = ElfHeader::new();
        header.magic = ELF_MAGIC;
        header.elf_class = ElfClass::ELF64;
        assert!(header.is_valid());
    }

    #[test]
    fn test_elf_header_architecture() {
        let mut header = ElfHeader::new();
        header.e_machine = 0x3E; // x86-64
        assert_eq!(header.get_architecture(), KernelArch::X86_64);
    }

    #[test]
    fn test_elf_header_executable() {
        let mut header = ElfHeader::new();
        header.e_type = 2; // ET_EXEC
        assert!(header.is_executable());
    }

    #[test]
    fn test_register_state_creation() {
        let reg = RegisterState::new();
        assert_eq!(reg.rflags, 0x202);
        assert!(reg.cr0 > 0);
    }

    #[test]
    fn test_register_state_entry_point() {
        let mut reg = RegisterState::new();
        reg.set_entry_point(0x400000);
        assert_eq!(reg.rip, 0x400000);
    }

    #[test]
    fn test_register_state_stack() {
        let mut reg = RegisterState::new();
        reg.set_stack(0x80000000);
        assert_eq!(reg.rsp, 0x80000000);
    }

    #[test]
    fn test_register_state_args() {
        let mut reg = RegisterState::new();
        reg.set_args(1, 2, 3);
        assert_eq!(reg.rdi, 1);
        assert_eq!(reg.rsi, 2);
        assert_eq!(reg.rdx, 3);
    }

    #[test]
    fn test_register_state_page_table() {
        let mut reg = RegisterState::new();
        reg.set_page_table(0x100000);
        assert_eq!(reg.cr3, 0x100000);
    }

    #[test]
    fn test_kernel_boot_args_creation() {
        let args = KernelBootArgs::new();
        assert!(args.is_valid());
    }

    #[test]
    fn test_kernel_bootstrap_creation() {
        let bootstrap = KernelBootstrap::new();
        assert!(!bootstrap.is_kernel_prepared());
    }

    #[test]
    fn test_kernel_bootstrap_load_elf() {
        let mut bootstrap = KernelBootstrap::new();
        let mut header = ElfHeader::new();
        header.magic = ELF_MAGIC;
        header.elf_class = ElfClass::ELF64;
        header.e_entry = 0x400000;

        assert!(bootstrap.load_elf_header(header));
        assert_eq!(bootstrap.get_entry_point(), 0x400000);
    }

    #[test]
    fn test_kernel_bootstrap_prepare_environment() {
        let mut bootstrap = KernelBootstrap::new();
        bootstrap.set_entry_point(0x400000);
        bootstrap.set_kernel_stack(0x80000000);

        assert!(bootstrap.prepare_kernel_environment());
        assert!(bootstrap.is_kernel_prepared());
    }

    #[test]
    fn test_kernel_bootstrap_cpu_features() {
        let mut bootstrap = KernelBootstrap::new();
        bootstrap.prepare_cpu_features(true, true);

        let reg = bootstrap.get_register_state();
        assert!(reg.cr4 & 0x2 != 0); // SSE
        assert!(reg.cr4 & 0x40000 != 0); // AVX
    }

    #[test]
    fn test_kernel_bootstrap_report() {
        let bootstrap = KernelBootstrap::new();
        let report = bootstrap.bootstrap_report();
        assert!(report.contains("Kernel Bootstrap Report"));
    }
}
