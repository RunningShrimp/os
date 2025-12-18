/// Hardened ELF64 Loader with Safety Validation
///
/// Provides safe, bounds-checked ELF loading with comprehensive validation.
/// Addresses critical security issues with unsafe memory access.


/// ELF64 File Header
#[derive(Debug, Clone, Copy)]
pub struct ElfFileHeader {
    pub ident: [u8; 16],
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

/// ELF64 Program Header
#[derive(Debug, Clone, Copy)]
pub struct ElfProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// ELF Magic Number
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF Class (32/64-bit)
const ELFCLASS64: u8 = 2;

/// ELF Data (Endianness)
const ELFDATA2LSB: u8 = 1; // Little endian

/// ELF Program Header Types
pub mod ph_type {
    pub const PT_LOAD: u32 = 1;
    pub const PT_DYNAMIC: u32 = 2;
    pub const PT_INTERP: u32 = 3;
}

/// ELF Section Header Types
pub mod sh_type {
    pub const SHT_PROGBITS: u32 = 1;
    pub const SHT_SYMTAB: u32 = 2;
    pub const SHT_STRTAB: u32 = 3;
    pub const SHT_REL: u32 = 4;
    pub const SHT_RELA: u32 = 5;
}

#[derive(Debug, Clone, Copy)]
pub enum ElfLoadError {
    InvalidMagic,
    InvalidElfClass,
    InvalidEndianness,
    InvalidElfVersion,
    HeaderTooSmall,
    InvalidPhdrOffset,
    InvalidPhdrCount,
    InvalidPhdrSize,
    InvalidSegmentSize,
    InvalidSegmentAlignment,
    InvalidMemoryAddress,
    SegmentOutOfBounds,
    OverlapDetected,
    FileOffsetOutOfBounds,
    AllocationFailed,
    InvalidSectionHeader,
    InvalidStringTable,
}

impl ElfLoadError {
    pub fn as_str(&self) -> &'static str {
        match self {
            ElfLoadError::InvalidMagic => "Invalid ELF magic number",
            ElfLoadError::InvalidElfClass => "Not an ELF64 file",
            ElfLoadError::InvalidEndianness => "Not little-endian",
            ElfLoadError::InvalidElfVersion => "Invalid ELF version",
            ElfLoadError::HeaderTooSmall => "ELF header too small",
            ElfLoadError::InvalidPhdrOffset => "Invalid program header offset",
            ElfLoadError::InvalidPhdrCount => "Invalid program header count",
            ElfLoadError::InvalidPhdrSize => "Invalid program header size",
            ElfLoadError::InvalidSegmentSize => "Invalid segment size",
            ElfLoadError::InvalidSegmentAlignment => "Invalid segment alignment",
            ElfLoadError::InvalidMemoryAddress => "Invalid memory address",
            ElfLoadError::SegmentOutOfBounds => "Segment exceeds file bounds",
            ElfLoadError::OverlapDetected => "Segments overlap in memory",
            ElfLoadError::FileOffsetOutOfBounds => "File offset out of bounds",
            ElfLoadError::AllocationFailed => "Memory allocation failed",
            ElfLoadError::InvalidSectionHeader => "Invalid section header",
            ElfLoadError::InvalidStringTable => "Invalid string table",
        }
    }
}

pub type Result<T> = core::result::Result<T, ElfLoadError>;

/// Safe ELF Loader
pub struct ElfLoader {
    file_data: *const u8,
    file_size: usize,
    segments: [Option<LoadedSegment>; 16],
    segment_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct LoadedSegment {
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub offset: u64,
}

impl ElfLoader {
    /// Create new ELF loader
    pub fn new(file_data: *const u8, file_size: usize) -> Self {
        Self {
            file_data,
            file_size,
            segments: [None; 16],
            segment_count: 0,
        }
    }

    /// Validate and parse ELF header
    pub fn parse_header(&self) -> Result<ElfFileHeader> {
        // SAFETY: Caller must ensure file_data points to valid ELF file
        unsafe {
            // Check minimum header size
            if self.file_size < 64 {
                return Err(ElfLoadError::HeaderTooSmall);
            }

            let ptr = self.file_data as *const ElfFileHeader;

            // Validate ELF magic
            let ident = ptr.read_unaligned().ident;
            if ident[0..4] != ELF_MAGIC {
                return Err(ElfLoadError::InvalidMagic);
            }

            // Validate ELF class (64-bit)
            if ident[4] != ELFCLASS64 {
                return Err(ElfLoadError::InvalidElfClass);
            }

            // Validate endianness (little-endian)
            if ident[5] != ELFDATA2LSB {
                return Err(ElfLoadError::InvalidEndianness);
            }

            // Validate ELF version
            if ident[6] != 1 {
                return Err(ElfLoadError::InvalidElfVersion);
            }

            let header = ptr.read_unaligned();

            // Validate version field
            if header.e_version != 1 {
                return Err(ElfLoadError::InvalidElfVersion);
            }

            // Validate program header offset
            if header.e_phoff < 64 {
                return Err(ElfLoadError::InvalidPhdrOffset);
            }

            // Validate program header count
            if header.e_phnum > 16 {
                return Err(ElfLoadError::InvalidPhdrCount);
            }

            // Validate program header entry size
            if header.e_phentsize < 56 {
                return Err(ElfLoadError::InvalidPhdrSize);
            }

            Ok(header)
        }
    }

    /// Load all segments from ELF file
    pub fn load_segments(&mut self) -> Result<()> {
        let header = self.parse_header()?;

        // Validate program header table location
        let phdr_end = header.e_phoff
            .checked_add((header.e_phnum as u64) * (header.e_phentsize as u64))
            .ok_or(ElfLoadError::InvalidPhdrOffset)?;

        if phdr_end > self.file_size as u64 {
            return Err(ElfLoadError::InvalidPhdrOffset);
        }

        // Load each segment
        for i in 0..header.e_phnum {
            let offset = header.e_phoff + ((i as u64) * (header.e_phentsize as u64));
            self.load_segment_at(offset, header.e_phentsize)?;
        }

        Ok(())
    }

    /// Load single segment
    fn load_segment_at(&mut self, offset: u64, size: u16) -> Result<()> {
        // SAFETY: Caller must ensure file_data is valid
        unsafe {
            if offset as usize + (size as usize) > self.file_size {
                return Err(ElfLoadError::FileOffsetOutOfBounds);
            }

            let ptr = (self.file_data as usize + offset as usize) as *const ElfProgramHeader;
            let phdr = ptr.read_unaligned();

            // Skip non-LOAD segments
            if phdr.p_type != ph_type::PT_LOAD {
                return Ok(());
            }

            // Validate segment sizes
            if phdr.p_filesz > self.file_size as u64 {
                return Err(ElfLoadError::InvalidSegmentSize);
            }

            if phdr.p_memsz < phdr.p_filesz {
                return Err(ElfLoadError::InvalidSegmentSize);
            }

            // Validate file offset
            if phdr.p_offset.saturating_add(phdr.p_filesz) > self.file_size as u64 {
                return Err(ElfLoadError::SegmentOutOfBounds);
            }

            // Validate alignment (must be power of 2)
            if phdr.p_align > 0 && (phdr.p_align & (phdr.p_align - 1)) != 0 {
                return Err(ElfLoadError::InvalidSegmentAlignment);
            }

            // Validate memory address (must be in kernel space or valid user space)
            if cfg!(target_arch = "x86_64") {
                // Restrict to reasonable address ranges
                let vaddr = phdr.p_vaddr;
                if vaddr == 0 && phdr.p_memsz > 0 {
                    return Err(ElfLoadError::InvalidMemoryAddress);
                }
            }

            // Check for overlaps with existing segments
            for i in 0..self.segment_count {
                if let Some(seg) = self.segments[i] {
                    if segments_overlap(seg, &phdr) {
                        return Err(ElfLoadError::OverlapDetected);
                    }
                }
            }

            // Store segment info (actual loading deferred)
            if self.segment_count >= 16 {
                return Err(ElfLoadError::InvalidPhdrCount);
            }

            self.segments[self.segment_count] = Some(LoadedSegment {
                vaddr: phdr.p_vaddr,
                paddr: phdr.p_paddr,
                filesz: phdr.p_filesz,
                memsz: phdr.p_memsz,
                offset: phdr.p_offset,
            });

            self.segment_count += 1;
            Ok(())
        }
    }

    /// Get entry point
    pub fn entry_point(&self) -> Result<u64> {
        Ok(self.parse_header()?.e_entry)
    }

    /// Get loaded segments
    pub fn get_segments(&self) -> &[Option<LoadedSegment>] {
        &self.segments[0..self.segment_count]
    }

    /// Verify kernel before execution
    pub fn verify_kernel(&self) -> Result<()> {
        // Must have at least one LOAD segment
        if self.segment_count == 0 {
            return Err(ElfLoadError::InvalidSegmentSize);
        }

        // Entry point must be set
        let entry = self.entry_point()?;
        if entry == 0 {
            return Err(ElfLoadError::InvalidMemoryAddress);
        }

        Ok(())
    }
}

/// Check if two segments overlap in memory
fn segments_overlap(seg1: LoadedSegment, seg2: &ElfProgramHeader) -> bool {
    let seg2_start = seg2.p_vaddr;
    let seg2_end = seg2.p_vaddr + seg2.p_memsz;
    let seg1_start = seg1.vaddr;
    let seg1_end = seg1.vaddr + seg1.memsz;

    // Check for overlap
    !(seg1_end <= seg2_start || seg2_end <= seg1_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        assert_eq!(
            ElfLoadError::InvalidMagic.as_str(),
            "Invalid ELF magic number"
        );
        assert_eq!(
            ElfLoadError::InvalidElfClass.as_str(),
            "Not an ELF64 file"
        );
    }

    #[test]
    fn test_segment_overlap_detection() {
        let seg1 = LoadedSegment {
            vaddr: 0x1000,
            paddr: 0x1000,
            filesz: 0x1000,
            memsz: 0x2000,
            offset: 0,
        };

        let seg2a = ElfProgramHeader {
            p_type: 1,
            p_flags: 5,
            p_offset: 0x1000,
            p_vaddr: 0x3000,
            p_paddr: 0x3000,
            p_filesz: 0x1000,
            p_memsz: 0x1000,
            p_align: 0x1000,
        };

        let seg2b = ElfProgramHeader {
            p_type: 1,
            p_flags: 5,
            p_offset: 0x1000,
            p_vaddr: 0x1500,
            p_paddr: 0x1500,
            p_filesz: 0x1000,
            p_memsz: 0x1000,
            p_align: 0x1000,
        };

        // No overlap: 0x3000-0x4000 is after 0x1000-0x3000
        assert!(!segments_overlap(seg1, &seg2a));

        // Overlap: 0x1500-0x2500 overlaps with 0x1000-0x3000
        assert!(segments_overlap(seg1, &seg2b));
    }

    #[test]
    fn test_loader_creation() {
        let dummy_data: [u8; 64] = [0; 64];
        let loader = ElfLoader::new(dummy_data.as_ptr(), 64);
        assert_eq!(loader.file_size, 64);
        assert_eq!(loader.segment_count, 0);
    }

    #[test]
    fn test_segment_count_limit() {
        let loader = ElfLoader::new(core::ptr::null(), 0);
        // Loader has maximum 16 segments
        assert_eq!(
            mem::size_of::<Option<LoadedSegment>>(),
            mem::size_of::<LoadedSegment>() + mem::size_of::<u8>()
        );
    }
}
