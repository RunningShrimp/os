// ELF64 Loader
// Parse and load ELF executables into user address space

use core::mem::size_of;

// ============================================================================
// ELF Constants
// ============================================================================

/// ELF Magic number
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF Class
pub const ELFCLASS64: u8 = 2;

/// ELF Data encoding
pub const ELFDATA2LSB: u8 = 1; // Little endian

/// ELF Type
pub const ET_EXEC: u16 = 2;    // Executable
pub const ET_DYN: u16 = 3;     // Shared object (PIE)

/// ELF Machine
pub const EM_RISCV: u16 = 243;
pub const EM_AARCH64: u16 = 183;
pub const EM_X86_64: u16 = 62;

/// Program header types
pub const PT_NULL: u32 = 0;
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;
pub const PT_NOTE: u32 = 4;
pub const PT_PHDR: u32 = 6;
pub const PT_TLS: u32 = 7;

/// Program header flags
pub const PF_X: u32 = 1;       // Executable
pub const PF_W: u32 = 2;       // Writable
pub const PF_R: u32 = 4;       // Readable

// ============================================================================
// ELF Structures
// ============================================================================

/// ELF64 File Header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    /// Magic number and other info
    pub e_ident: [u8; 16],
    /// Object file type
    pub e_type: u16,
    /// Architecture
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u64,
    /// Program header table file offset
    pub e_phoff: u64,
    /// Section header table file offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size
    pub e_ehsize: u16,
    /// Program header table entry size
    pub e_phentsize: u16,
    /// Program header table entry count
    pub e_phnum: u16,
    /// Section header table entry size
    pub e_shentsize: u16,
    /// Section header table entry count
    pub e_shnum: u16,
    /// Section name string table index
    pub e_shstrndx: u16,
}

impl Elf64Header {
    /// Check if this is a valid ELF header
    pub fn is_valid(&self) -> bool {
        self.e_ident[0..4] == ELF_MAGIC
    }
    
    /// Check if this is a 64-bit ELF
    pub fn is_64bit(&self) -> bool {
        self.e_ident[4] == ELFCLASS64
    }
    
    /// Check if this is little-endian
    pub fn is_little_endian(&self) -> bool {
        self.e_ident[5] == ELFDATA2LSB
    }
    
    /// Check if the machine type matches current architecture
    pub fn is_current_arch(&self) -> bool {
        #[cfg(target_arch = "riscv64")]
        { self.e_machine == EM_RISCV }
        
        #[cfg(target_arch = "aarch64")]
        { self.e_machine == EM_AARCH64 }
        
        #[cfg(target_arch = "x86_64")]
        { self.e_machine == EM_X86_64 }
    }
    
    /// Check if this is an executable
    pub fn is_executable(&self) -> bool {
        self.e_type == ET_EXEC || self.e_type == ET_DYN
    }
    
    /// Validate ELF header
    pub fn validate(&self) -> Result<(), ElfError> {
        if !self.is_valid() {
            return Err(ElfError::InvalidMagic);
        }
        if !self.is_64bit() {
            return Err(ElfError::Not64Bit);
        }
        if !self.is_little_endian() {
            return Err(ElfError::NotLittleEndian);
        }
        if !self.is_current_arch() {
            return Err(ElfError::WrongArch);
        }
        if !self.is_executable() {
            return Err(ElfError::NotExecutable);
        }
        if self.e_phentsize as usize != size_of::<Elf64Phdr>() {
            return Err(ElfError::InvalidPhentsize);
        }
        Ok(())
    }
}

/// ELF64 Program Header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Segment file offset
    pub p_offset: u64,
    /// Segment virtual address
    pub p_vaddr: u64,
    /// Segment physical address
    pub p_paddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Segment alignment
    pub p_align: u64,
}

impl Elf64Phdr {
    /// Is this a loadable segment?
    pub fn is_load(&self) -> bool {
        self.p_type == PT_LOAD
    }
    
    /// Is this segment readable?
    pub fn is_readable(&self) -> bool {
        (self.p_flags & PF_R) != 0
    }
    
    /// Is this segment writable?
    pub fn is_writable(&self) -> bool {
        (self.p_flags & PF_W) != 0
    }
    
    /// Is this segment executable?
    pub fn is_executable(&self) -> bool {
        (self.p_flags & PF_X) != 0
    }
}

/// ELF64 Section Header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Shdr {
    /// Section name (string table index)
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Section virtual address
    pub sh_addr: u64,
    /// Section file offset
    pub sh_offset: u64,
    /// Section size
    pub sh_size: u64,
    /// Link to another section
    pub sh_link: u32,
    /// Additional section info
    pub sh_info: u32,
    /// Section alignment
    pub sh_addralign: u64,
    /// Entry size for fixed-size entries
    pub sh_entsize: u64,
}

// ============================================================================
// ELF Loader Errors
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    InvalidMagic,
    Not64Bit,
    NotLittleEndian,
    WrongArch,
    NotExecutable,
    InvalidPhentsize,
    NoLoadSegments,
    SegmentOverlap,
    OutOfMemory,
    InvalidAddress,
    TooLarge,
}

// ============================================================================
// ELF Loader
// ============================================================================

/// Information about a loaded ELF
#[derive(Debug, Clone)]
pub struct ElfInfo {
    /// Entry point address
    pub entry: usize,
    /// Base address (for PIE)
    pub base: usize,
    /// Program break (end of loaded segments)
    pub brk: usize,
    /// Stack pointer suggestion
    pub sp: usize,
    /// Interpreter path (if any)
    pub interp: Option<[u8; 256]>,
}

/// Page size constant
pub const PAGE_SIZE: usize = 4096;

/// Align down to page boundary
#[inline]
pub fn page_align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// Align up to page boundary
#[inline]
pub fn page_align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// ELF Loader structure
pub struct ElfLoader<'a> {
    data: &'a [u8],
    header: &'a Elf64Header,
}

impl<'a> ElfLoader<'a> {
    /// Create a new ELF loader from raw bytes
    pub fn new(data: &'a [u8]) -> Result<Self, ElfError> {
        if data.len() < size_of::<Elf64Header>() {
            return Err(ElfError::TooLarge);
        }
        
        let header = unsafe {
            &*(data.as_ptr() as *const Elf64Header)
        };
        
        header.validate()?;
        
        Ok(Self { data, header })
    }
    
    /// Get the ELF header
    pub fn header(&self) -> &Elf64Header {
        self.header
    }
    
    /// Get entry point
    pub fn entry(&self) -> u64 {
        self.header.e_entry
    }
    
    /// Iterate over program headers
    pub fn program_headers(&self) -> impl Iterator<Item = &Elf64Phdr> + '_ {
        let phoff = self.header.e_phoff as usize;
        let phnum = self.header.e_phnum as usize;
        let phentsize = self.header.e_phentsize as usize;
        let data = self.data;
        
        (0..phnum).filter_map(move |i| {
            let offset = phoff + i * phentsize;
            if offset + phentsize <= data.len() {
                unsafe {
                    Some(&*(data.as_ptr().add(offset) as *const Elf64Phdr))
                }
            } else {
                None
            }
        })
    }
    
    /// Get loadable segments
    pub fn load_segments(&self) -> impl Iterator<Item = &Elf64Phdr> + '_ {
        self.program_headers().filter(|ph| ph.is_load())
    }
    
    /// Calculate total memory size needed
    pub fn total_size(&self) -> usize {
        let mut min_addr = usize::MAX;
        let mut max_addr = 0usize;
        
        for ph in self.load_segments() {
            let start = ph.p_vaddr as usize;
            let end = start + ph.p_memsz as usize;
            min_addr = min_addr.min(start);
            max_addr = max_addr.max(end);
        }
        
        if max_addr > min_addr {
            page_align_up(max_addr) - page_align_down(min_addr)
        } else {
            0
        }
    }
    
    /// Load ELF into memory using provided mapping callback
    /// 
    /// The callback `map_page` should:
    /// - Allocate a physical page
    /// - Map it at the given virtual address with given permissions
    /// - Return the virtual address of the mapped page, or None on failure
    /// 
    /// Returns ElfInfo on success
    pub fn load<F>(&self, mut map_page: F) -> Result<ElfInfo, ElfError>
    where
        F: FnMut(usize, bool, bool, bool) -> Option<*mut u8>,
        // (vaddr, readable, writable, executable) -> mapped_ptr
    {
        let mut brk: usize = 0;
        let mut base = 0usize;
        let mut has_interp = false;
        
        // Load each segment
        for ph in self.load_segments() {
            let vaddr_start = ph.p_vaddr as usize;
            let vaddr_end = vaddr_start + ph.p_memsz as usize;
            let file_start = ph.p_offset as usize;
            let file_size = ph.p_filesz as usize;
            
            // Update program break
            brk = brk.max(vaddr_end);
            
            // Map pages for this segment
            let page_start = page_align_down(vaddr_start);
            let page_end = page_align_up(vaddr_end);
            
            for page_vaddr in (page_start..page_end).step_by(PAGE_SIZE) {
                let readable = ph.is_readable();
                let writable = ph.is_writable();
                let executable = ph.is_executable();
                
                let page_ptr = map_page(page_vaddr + base, readable, writable, executable)
                    .ok_or(ElfError::OutOfMemory)?;
                
                // Zero the page first
                unsafe {
                    core::ptr::write_bytes(page_ptr, 0, PAGE_SIZE);
                }
                
                // Copy data from file if within file bounds
                let page_vaddr_end = page_vaddr + PAGE_SIZE;
                
                // Calculate overlap with file data
                let data_start = vaddr_start.max(page_vaddr);
                let data_end = (vaddr_start + file_size).min(page_vaddr_end);
                
                if data_end > data_start {
                    let file_offset = file_start + (data_start - vaddr_start);
                    let page_offset = (data_start + base) - (page_vaddr + base);
                    let copy_len = data_end - data_start;
                    
                    if file_offset + copy_len <= self.data.len() {
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                self.data.as_ptr().add(file_offset),
                                page_ptr.add(page_offset),
                                copy_len,
                            );
                        }
                    }
                }
            }
        }
        
        // Default stack at high address
        let default_stack = 0x7FFF_FFFF_F000usize;
        
        let mut interp: Option<[u8; 256]> = None;
        for ph in self.program_headers() {
            if ph.p_type == PT_INTERP {
                if let Some(data) = self.segment_data(ph) {
                    let mut buf = [0u8; 256];
                    let n = core::cmp::min(buf.len(), data.len());
                    buf[..n].copy_from_slice(&data[..n]);
                    interp = Some(buf);
                    has_interp = true;
                }
                break;
            }
        }
        if self.header.e_type == ET_DYN || has_interp {
            base = 0x400000;
        }
        Ok(ElfInfo {
            entry: (self.header.e_entry as usize).wrapping_add(base),
            base,
            brk: page_align_up(brk),
            sp: default_stack,
            interp,
        })
    }
    
    /// Get segment data for a program header
    pub fn segment_data(&self, ph: &Elf64Phdr) -> Option<&'a [u8]> {
        let offset = ph.p_offset as usize;
        let size = ph.p_filesz as usize;
        
        if offset + size <= self.data.len() {
            Some(&self.data[offset..offset + size])
        } else {
            None
        }
    }
}

// ============================================================================
// Auxiliary Vector for Program Initialization
// ============================================================================

/// Auxiliary vector types (for passing info to user program)
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum AuxType {
    Null = 0,
    Ignore = 1,
    ExecFd = 2,
    Phdr = 3,
    Phent = 4,
    Phnum = 5,
    Pagesz = 6,
    Base = 7,
    Flags = 8,
    Entry = 9,
    NotElf = 10,
    Uid = 11,
    Euid = 12,
    Gid = 13,
    Egid = 14,
    Platform = 15,
    Hwcap = 16,
    Clktck = 17,
    Random = 25,
    Execfn = 31,
}

/// Auxiliary vector entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AuxEntry {
    pub a_type: usize,
    pub a_val: usize,
}

impl AuxEntry {
    pub const fn new(a_type: AuxType, a_val: usize) -> Self {
        Self {
            a_type: a_type as usize,
            a_val,
        }
    }
    
    pub const fn null() -> Self {
        Self { a_type: 0, a_val: 0 }
    }
}

// ============================================================================
// Stack Setup for New Process
// ============================================================================

/// Set up the initial stack for a new process
/// Returns the new stack pointer
/// 
/// Stack layout (growing down):
/// ```
/// [high address]
/// null terminator for strings
/// environment strings
/// argument strings  
/// padding for alignment
/// null (end of auxv)
/// auxv entries
/// null (end of envp)
/// envp pointers
/// null (end of argv)
/// argv pointers
/// argc
/// [low address] <- SP
/// ```
pub fn setup_stack(
    stack_top: usize,
    argv: &[&[u8]],
    envp: &[&[u8]],
    auxv: &[AuxEntry],
) -> Result<(usize, usize, usize), ElfError> {
    // This is a simplified version - in real implementation,
    // we'd write directly to user stack memory
    
    let argc = argv.len();
    
    // Calculate space needed
    let mut string_size = 0;
    for arg in argv {
        string_size += arg.len() + 1; // +1 for null terminator
    }
    for env in envp {
        string_size += env.len() + 1;
    }
    
    let ptr_size = core::mem::size_of::<usize>();
    let ptrs_size = (argc + 1 + envp.len() + 1 + auxv.len() * 2 + 2 + 1) * ptr_size;
    
    let total_size = string_size + ptrs_size + 16; // +16 for alignment
    
    let sp = (stack_top - total_size) & !0xF; // 16-byte aligned
    
    // Return: (sp, argc, argv_ptr)
    // In practice, we'd write the actual data to memory
    Ok((sp, argc, sp + ptr_size))
}
