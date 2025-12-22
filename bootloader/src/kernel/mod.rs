//! Kernel loading and execution (ELF parsing and loading)

use arrayvec::ArrayVec;

/// ELF file header for 64-bit
#[repr(C)]
pub struct Elf64_Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF program header for 64-bit
#[repr(C)]
pub struct Elf64_Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// ELF segment types
pub const PT_LOAD: u32 = 1;

/// ELF magic number
pub const ELFMAG: [u8; 4] = [0x7F, 0x45, 0x4C, 0x46]; // '\x7F' 'E' 'L' 'F'

/// ELF file classes
pub const ELFCLASS64: u8 = 2; // 64-bit ELF

/// ELF data encoding
pub const ELFDATA2LSB: u8 = 1; // Little-endian

/// ELF file types
pub const ET_EXEC: u16 = 2; // Executable file

/// ELF machine types
pub const EM_X86_64: u16 = 62; // x86-64 architecture

/// Kernel image structure
#[derive(Debug)]
pub struct KernelImage {
	pub entry_point: usize,
	pub memory_size: usize,
}

impl KernelImage {
    pub fn new(entry_point: usize, memory_size: usize) -> Self {
        Self {
            entry_point,
            memory_size,
        }
    }
}

/// Kernel loader
pub struct KernelLoader;

impl KernelLoader {
    pub fn load(data: &[u8]) -> crate::Result<KernelImage> {
        // Check if the data is long enough for an ELF header
        if data.len() < core::mem::size_of::<Elf64_Ehdr>() {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Parse the ELF header
        let ehdr = unsafe { &*(data.as_ptr() as *const Elf64_Ehdr) };

        // Validate ELF magic number
        if ehdr.e_ident[0..4] != ELFMAG {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Validate ELF class (64-bit)
        if ehdr.e_ident[4] != ELFCLASS64 {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Validate data encoding (little-endian)
        if ehdr.e_ident[5] != ELFDATA2LSB {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Validate file type (executable)
        if ehdr.e_type != ET_EXEC {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Validate machine type (x86-64)
        if ehdr.e_machine != EM_X86_64 {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Calculate total memory required for kernel
        let mut total_size = 0;

        // Iterate through program headers
        for i in 0..ehdr.e_phnum as usize {
            let phdr_offset = ehdr.e_phoff as usize + (i * ehdr.e_phentsize as usize);
            if phdr_offset + core::mem::size_of::<Elf64_Phdr>() > data.len() {
                return Err(crate::BootError::InvalidKernelFormat);
            }

            let phdr = unsafe { &*(data.as_ptr().add(phdr_offset) as *const Elf64_Phdr) };

            // Only process load segments
            if phdr.p_type != PT_LOAD {
                continue;
            }

            // Calculate the end address of this segment
            let seg_end = (phdr.p_vaddr + phdr.p_memsz) as usize;
            if seg_end > total_size {
                total_size = seg_end;
            }
        }

        if total_size == 0 {
            return Err(crate::BootError::InvalidKernelFormat);
        }

        // Create kernel image
        Ok(KernelImage::new(
            ehdr.e_entry as usize,
            total_size,
        ))
    }

    pub fn execute(kernel: KernelImage) -> crate::Result<()> {
        log::info!("Executing kernel image");
        log::debug!("Kernel entry point: {:#x}", kernel.entry_point);
        log::debug!("Kernel total size: {} bytes", kernel.total_size);
        
        // Verify kernel image validity
        if kernel.entry_point == 0 {
            log::error!("Invalid kernel entry point");
            return Err(crate::Error::InvalidKernel);
        }
        
        if kernel.total_size == 0 {
            log::error!("Invalid kernel size");
            return Err(crate::Error::InvalidKernel);
        }
        
        log::info!("Kernel image validated successfully");
        Ok(())
    }
}
