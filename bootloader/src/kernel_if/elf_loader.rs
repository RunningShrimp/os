// ELF kernel loader for bootloader


pub const ELF_MAGIC: u32 = 0x464C457F; // "\x7FELF"
pub const ELF_CLASS_64: u8 = 2;
pub const ELF_DATA_LITTLE: u8 = 1;
pub const ELF_TYPE_EXECUTABLE: u16 = 2;

#[repr(C, packed)]
pub struct ElfHeader {
    magic: u32,
    class: u8,
    data: u8,
    version: u8,
    osabi: u8,
    abiversion: u8,
    _padding: [u8; 7],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C, packed)]
pub struct ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

pub const PT_LOAD: u32 = 1;
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

pub struct ElfImage {
    header: *const ElfHeader,
}

impl ElfImage {
    /// Create ELF image from buffer
    pub fn new(buffer: *const u8) -> Option<Self> {
        let header = buffer as *const ElfHeader;
        unsafe {
            if (*header).magic != ELF_MAGIC {
                return None;
            }
            if (*header).class != ELF_CLASS_64 {
                return None;
            }
        }
        Some(Self { header })
    }

    /// Get entry point
    pub fn entry_point(&self) -> u64 {
        unsafe { (*self.header).e_entry }
    }

    /// Get number of program headers
    pub fn num_segments(&self) -> u16 {
        unsafe { (*self.header).e_phnum }
    }

    /// Load all PT_LOAD segments
    pub fn load_segments(&self) -> bool {
        let num = self.num_segments();
        for i in 0..num {
            if !self.load_segment(i) {
                return false;
            }
        }
        true
    }

    /// Load a single segment
    fn load_segment(&self, index: u16) -> bool {
        unsafe {
            let phoff = (*self.header).e_phoff as usize;
            let phentsize = (*self.header).e_phentsize as usize;
            let ph_addr = (self.header as *const _ as usize + phoff
                + (index as usize * phentsize))
                as *const ProgramHeader;

            if (*ph_addr).p_type != PT_LOAD {
                return true; // Skip non-loadable segments
            }

            // Copy segment to memory
            let src = (self.header as *const _ as usize
                + (*ph_addr).p_offset as usize) as *const u8;
            let dst = (*ph_addr).p_paddr as *mut u8;
            let size = (*ph_addr).p_filesz as usize;

            // Safe memory copy for bootloader
            for j in 0..size {
                *dst.add(j) = *src.add(j);
            }

            // Zero BSS section
            let bss_start =
                ((*ph_addr).p_paddr + (*ph_addr).p_filesz) as *mut u8;
            let bss_size =
                ((*ph_addr).p_memsz - (*ph_addr).p_filesz) as usize;
            for j in 0..bss_size {
                *bss_start.add(j) = 0;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_header_size() {
        assert_eq!(mem::size_of::<ElfHeader>(), 64);
    }

    #[test]
    fn test_program_header_size() {
        assert_eq!(mem::size_of::<ProgramHeader>(), 56);
    }
}
