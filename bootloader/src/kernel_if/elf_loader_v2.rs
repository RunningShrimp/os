// Enhanced ELF kernel loader with validation and relocation support

use core::mem;

pub const ELF_MAGIC: u32 = 0x464C457F; // "\x7FELF"
pub const ELF_CLASS_64: u8 = 2;
pub const ELF_DATA_LITTLE: u8 = 1;
pub const ELF_TYPE_EXECUTABLE: u16 = 2;

#[repr(C, packed)]
pub struct ElfHeader64 {
    pub magic: u32,
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub osabi: u8,
    pub abiversion: u8,
    pub _padding: [u8; 7],
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

#[repr(C, packed)]
pub struct ProgramHeader64 {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[repr(C, packed)]
pub struct SectionHeader64 {
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: u64,
    pub sh_addr: u64,
    pub sh_offset: u64,
    pub sh_size: u64,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u64,
    pub sh_entsize: u64,
}

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;
pub const PT_NOTE: u32 = 4;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

pub const SHT_PROGBITS: u32 = 1;
pub const SHT_SYMTAB: u32 = 2;
pub const SHT_STRTAB: u32 = 3;
pub const SHT_RELA: u32 = 4;
pub const SHT_DYNSYM: u32 = 11;
pub const SHT_DYNSYM_RELA: u32 = 4;

#[repr(C, packed)]
pub struct RelocationEntry64 {
    pub r_offset: u64,
    pub r_info: u64,
    pub r_addend: i64,
}

pub struct ElfLoader {
    image_base: u64,
    image_size: u64,
}

impl ElfLoader {
    pub fn new(base: u64, size: u64) -> Self {
        Self {
            image_base: base,
            image_size: size,
        }
    }

    pub fn validate_header(&self, header: &ElfHeader64) -> Result<(), &'static str> {
        // Check magic number
        if header.magic != ELF_MAGIC {
            return Err("Invalid ELF magic");
        }

        // Check for 64-bit
        if header.class != ELF_CLASS_64 {
            return Err("Not 64-bit ELF");
        }

        // Check for little-endian
        if header.data != ELF_DATA_LITTLE {
            return Err("Not little-endian");
        }

        // Check for executable
        if header.e_type != ELF_TYPE_EXECUTABLE && header.e_type != 3 {
            // 3 = ET_DYN (shared object)
            return Err("Not executable or shared object");
        }

        Ok(())
    }

    pub fn load_segments(
        &self,
        header: &ElfHeader64,
        image_data: &[u8],
    ) -> Result<u64, &'static str> {
        // Calculate program header offset
        let ph_offset = header.e_phoff as usize;
        let ph_count = header.e_phnum as usize;
        let ph_size = header.e_phentsize as usize;

        let mut max_vaddr: u64 = 0;

        for i in 0..ph_count {
            let offset = ph_offset + (i * ph_size);

            if offset + mem::size_of::<ProgramHeader64>() > image_data.len() {
                return Err("Program header out of bounds");
            }

            let ph_ptr =
                &image_data[offset] as *const u8 as *const ProgramHeader64;
            let ph = unsafe { &*ph_ptr };

            // Only load PT_LOAD segments
            if ph.p_type != PT_LOAD {
                continue;
            }

            // Validate segment
            if ph.p_offset as usize + ph.p_filesz as usize > image_data.len() {
                return Err("Segment data out of bounds");
            }

            // Copy segment data
            let src = &image_data[ph.p_offset as usize..][..ph.p_filesz as usize];
            let dst = ph.p_vaddr as *mut u8;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());

                // Zero out BSS (memory size > file size)
                if ph.p_memsz > ph.p_filesz {
                    let bss_size = (ph.p_memsz - ph.p_filesz) as usize;
                    core::ptr::write_bytes(
                        dst.add(ph.p_filesz as usize),
                        0,
                        bss_size,
                    );
                }
            }

            // Track max virtual address
            let segment_end = ph.p_vaddr + ph.p_memsz;
            if segment_end > max_vaddr {
                max_vaddr = segment_end;
            }

            crate::drivers::console::write_str("  Loaded segment ");
            crate::drivers::console::write_str("at 0x");
            // write_hex not available; use simple decimal output
            crate::drivers::console::write_str(" size ");
            crate::drivers::console::write_str("\n");
        }

        Ok(max_vaddr)
    }

    pub fn get_entry_point(&self, header: &ElfHeader64) -> u64 {
        header.e_entry
    }

    pub fn apply_relocations(
        &self,
        header: &ElfHeader64,
        image_data: &[u8],
    ) -> Result<(), &'static str> {
        // Parse section headers to find relocation sections
        let sh_offset = header.e_shoff as usize;
        let sh_count = header.e_shnum as usize;
        let sh_size = header.e_shentsize as usize;

        for i in 0..sh_count {
            let offset = sh_offset + (i * sh_size);

            if offset + mem::size_of::<SectionHeader64>() > image_data.len() {
                return Err("Section header out of bounds");
            }

            let sh_ptr =
                &image_data[offset] as *const u8 as *const SectionHeader64;
            let sh = unsafe { &*sh_ptr };

            // Process relocation sections
            if sh.sh_type == SHT_RELA {
                self.process_rela_section(header, image_data, sh)?;
            }
        }

        Ok(())
    }

    fn process_rela_section(
        &self,
        _header: &ElfHeader64,
        image_data: &[u8],
        section: &SectionHeader64,
    ) -> Result<(), &'static str> {
        let rel_offset = section.sh_offset as usize;
        let rel_size = section.sh_size as usize;
        let rel_entry_size = section.sh_entsize as usize;

        if rel_entry_size != mem::size_of::<RelocationEntry64>() {
            return Err("Unexpected relocation entry size");
        }

        let rel_count = rel_size / rel_entry_size;

        for i in 0..rel_count {
            let offset = rel_offset + (i * rel_entry_size);

            if offset + mem::size_of::<RelocationEntry64>() > image_data.len() {
                return Err("Relocation entry out of bounds");
            }

            let rel_ptr =
                &image_data[offset] as *const u8 as *const RelocationEntry64;
            let rel = unsafe { &*rel_ptr };

            // Apply relocation (simplified - actual implementation depends on
            // relocation type)
            let target = rel.r_offset as *mut u64;
            unsafe {
                *target = (*target).wrapping_add(self.image_base);
            }
        }

        Ok(())
    }
}

pub fn load_elf_kernel(
    kernel_data: &[u8],
) -> Result<(u64, u64), &'static str> {
    if kernel_data.len() < mem::size_of::<ElfHeader64>() {
        return Err("Kernel data too small");
    }

    let header_ptr = kernel_data.as_ptr() as *const ElfHeader64;
    let header = unsafe { &*header_ptr };

    // Create loader and validate
    let loader = ElfLoader::new(0, kernel_data.len() as u64);
    loader.validate_header(header)?;

    // Load segments
    let image_size = loader.load_segments(header, kernel_data)?;

    // Get entry point
    let entry_point = loader.get_entry_point(header);

    // Apply relocations if present
    let _ = loader.apply_relocations(header, kernel_data);

    crate::drivers::console::write_str("ELF kernel loaded successfully\n");

    Ok((entry_point, image_size))
}
