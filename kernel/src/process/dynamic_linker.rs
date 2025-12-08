//! Dynamic linker for ELF shared libraries
//!
//! This module implements dynamic linking support for loading and linking
//! shared libraries (.so files) at runtime. It handles symbol resolution,
//! relocation processing, and library dependencies.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::mem::size_of;
use crate::process::elf::{ElfLoader, Elf64Header, Elf64Phdr, ElfError, PAGE_SIZE};

// ============================================================================
// Dynamic Linking Constants
// ============================================================================

/// Dynamic entry tags
pub const DT_NULL: u64 = 0;
pub const DT_NEEDED: u64 = 1;
pub const DT_PLTRELSZ: u64 = 2;
pub const DT_PLTGOT: u64 = 3;
pub const DT_HASH: u64 = 4;
pub const DT_STRTAB: u64 = 5;
pub const DT_SYMTAB: u64 = 6;
pub const DT_RELA: u64 = 7;
pub const DT_RELASZ: u64 = 8;
pub const DT_RELAENT: u64 = 9;
pub const DT_STRSZ: u64 = 10;
pub const DT_SYMENT: u64 = 11;
pub const DT_INIT: u64 = 12;
pub const DT_FINI: u64 = 13;
pub const DT_SONAME: u64 = 14;
pub const DT_RPATH: u64 = 15;
pub const DT_SYMBOLIC: u64 = 16;
pub const DT_REL: u64 = 17;
pub const DT_RELSZ: u64 = 18;
pub const DT_RELENT: u64 = 19;
pub const DT_PLTREL: u64 = 20;
pub const DT_DEBUG: u64 = 21;
pub const DT_TEXTREL: u64 = 22;
pub const DT_JMPREL: u64 = 23;
pub const DT_BIND_NOW: u64 = 24;
pub const DT_INIT_ARRAY: u64 = 25;
pub const DT_FINI_ARRAY: u64 = 26;
pub const DT_INIT_ARRAYSZ: u64 = 27;
pub const DT_FINI_ARRAYSZ: u64 = 28;
pub const DT_RUNPATH: u64 = 29;
pub const DT_FLAGS: u64 = 30;
pub const DT_GNU_HASH: u64 = 0x6ffffef5;

/// Relocation types (x86_64)
#[cfg(target_arch = "x86_64")]
pub mod reloc_types {
    pub const R_X86_64_NONE: u32 = 0;
    pub const R_X86_64_64: u32 = 1;
    pub const R_X86_64_PC32: u32 = 2;
    pub const R_X86_64_GOT32: u32 = 3;
    pub const R_X86_64_PLT32: u32 = 4;
    pub const R_X86_64_COPY: u32 = 5;
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    pub const R_X86_64_RELATIVE: u32 = 8;
    pub const R_X86_64_GOTPCREL: u32 = 9;
    pub const R_X86_64_32: u32 = 10;
    pub const R_X86_64_32S: u32 = 11;
    pub const R_X86_64_16: u32 = 12;
    pub const R_X86_64_PC16: u32 = 13;
    pub const R_X86_64_8: u32 = 14;
    pub const R_X86_64_PC8: u32 = 15;
    pub const R_X86_64_DTPMOD64: u32 = 16;
    pub const R_X86_64_DTPOFF64: u32 = 17;
    pub const R_X86_64_TPOFF64: u32 = 18;
    pub const R_X86_64_TLSGD: u32 = 19;
    pub const R_X86_64_TLSLD: u32 = 20;
    pub const R_X86_64_DTPOFF32: u32 = 21;
    pub const R_X86_64_GOTTPOFF: u32 = 22;
    pub const R_X86_64_TPOFF32: u32 = 23;
}

/// Relocation types (aarch64)
#[cfg(target_arch = "aarch64")]
pub mod reloc_types {
    pub const R_AARCH64_NONE: u32 = 0;
    pub const R_AARCH64_ABS64: u32 = 257;
    pub const R_AARCH64_ABS32: u32 = 258;
    pub const R_AARCH64_ABS16: u32 = 259;
    pub const R_AARCH64_PREL64: u32 = 260;
    pub const R_AARCH64_PREL32: u32 = 261;
    pub const R_AARCH64_PREL16: u32 = 262;
    pub const R_AARCH64_GLOB_DAT: u32 = 1025;
    pub const R_AARCH64_JUMP_SLOT: u32 = 1026;
    pub const R_AARCH64_RELATIVE: u32 = 1027;
    pub const R_AARCH64_TLS_TPREL64: u32 = 1028;
    pub const R_AARCH64_TLS_DTPREL64: u32 = 1029;
    pub const R_AARCH64_TLS_DTPMOD64: u32 = 1030;
    pub const R_AARCH64_TLS_DTPREL32: u32 = 1031;
    pub const R_AARCH64_TLS_DTPMOD32: u32 = 1032;
    pub const R_AARCH64_TLS_TPREL32: u32 = 1033;
    pub const R_AARCH64_TLSDESC: u32 = 1034;
    pub const R_AARCH64_IRELATIVE: u32 = 1038;
}

/// Relocation types (riscv64)
#[cfg(target_arch = "riscv64")]
pub mod reloc_types {
    pub const R_RISCV_NONE: u32 = 0;
    pub const R_RISCV_32: u32 = 1;
    pub const R_RISCV_64: u32 = 2;
    pub const R_RISCV_RELATIVE: u32 = 3;
    pub const R_RISCV_COPY: u32 = 4;
    pub const R_RISCV_JUMP_SLOT: u32 = 5;
    pub const R_RISCV_TLS_DTPMOD32: u32 = 6;
    pub const R_RISCV_TLS_DTPMOD64: u32 = 7;
    pub const R_RISCV_TLS_DTPREL32: u32 = 8;
    pub const R_RISCV_TLS_DTPREL64: u32 = 9;
    pub const R_RISCV_TLS_TPREL32: u32 = 10;
    pub const R_RISCV_TLS_TPREL64: u32 = 11;
}

// ============================================================================
// Dynamic Linking Structures
// ============================================================================

/// Dynamic entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dyn {
    pub d_tag: u64,
    pub d_val: u64,
}

/// Symbol entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Sym {
    pub st_name: u32,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
    pub st_value: u64,
    pub st_size: u64,
}

/// Relocation entry (with addend)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rela {
    pub r_offset: u64,
    pub r_info: u64,
    pub r_addend: i64,
}

/// Relocation entry (without addend)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rel {
    pub r_offset: u64,
    pub r_info: u64,
}

/// Loaded shared library information
pub struct LoadedLibrary {
    /// Base address where library is loaded
    pub base: usize,
    /// Dynamic section address
    pub dynamic: usize,
    /// Symbol table address
    pub symtab: usize,
    /// String table address
    pub strtab: usize,
    /// String table size
    pub strtab_size: usize,
    /// Hash table address
    pub hash: Option<usize>,
    /// Relocation table address
    pub rela: Option<usize>,
    /// Relocation table size
    pub rela_size: usize,
    /// PLT relocation table address
    pub jmprel: Option<usize>,
    /// PLT relocation table size
    pub jmprel_size: usize,
    /// Library name
    pub name: String,
    /// Dependencies (DT_NEEDED entries)
    pub needed: Vec<String>,
    /// Entry point
    pub entry: usize,
}

/// Dynamic linker state
pub struct DynamicLinker {
    /// Loaded libraries (name -> library)
    libraries: BTreeMap<String, LoadedLibrary>,
    /// Global symbol table (symbol name -> (library_name, symbol))
    global_symbols: BTreeMap<String, (String, Sym)>,
    /// Library search paths
    search_paths: Vec<String>,
}

impl DynamicLinker {
    /// Create a new dynamic linker
    pub fn new() -> Self {
        let mut linker = Self {
            libraries: BTreeMap::new(),
            global_symbols: BTreeMap::new(),
            search_paths: Vec::new(),
        };
        
        // Add default search paths
        linker.search_paths.push("/lib".to_string());
        linker.search_paths.push("/usr/lib".to_string());
        linker.search_paths.push("/usr/local/lib".to_string());
        
        linker
    }
    
    /// Add a library search path
    pub fn add_search_path(&mut self, path: String) {
        self.search_paths.push(path);
    }
    
    /// Load a shared library
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .so file
    /// * `base` - Base address to load the library at (0 for ASLR)
    /// * `map_page` - Callback to map pages into memory
    ///
    /// # Returns
    ///
    /// * `Ok(LoadedLibrary)` - Successfully loaded library
    /// * `Err(ElfError)` - Error loading library
    pub fn load_library<F>(
        &mut self,
        path: &str,
        base: usize,
        map_page: F,
    ) -> Result<LoadedLibrary, ElfError>
    where
        F: FnMut(usize, bool, bool, bool) -> Option<*mut u8>,
    {
        // Read library file
        let data = self.read_library_file(path)?;
        
        // Parse ELF
        let loader = ElfLoader::new(&data)?;
        
        // Determine base address (use provided base or calculate)
        let actual_base = if base == 0 {
            // ASLR: choose a random base address (simplified - use fixed address for now)
            0x70000000usize
        } else {
            base
        };
        
        // Create a mutable closure for map_page that adjusts addresses
        let mut map_page_fn = map_page;
        let base_offset = actual_base;
        
        // Load ELF segments with base address adjustment
        let elf_info = loader.load(|vaddr, readable, writable, executable| {
            map_page_fn(base_offset + vaddr, readable, writable, executable)
        })?;
        
        // Parse dynamic section
        let dynamic_phdr = loader.program_headers()
            .find(|ph| ph.p_type == crate::process::elf::PT_DYNAMIC);
        
        if dynamic_phdr.is_none() {
            return Err(ElfError::InvalidMagic); // Not a dynamic library
        }
        
        let dynamic_phdr = dynamic_phdr.unwrap();
        let dynamic_addr = actual_base + dynamic_phdr.p_vaddr as usize;
        
        // Parse dynamic entries
        let (symtab, strtab, strtab_size, hash, rela, rela_size, jmprel, jmprel_size, needed) =
            self.parse_dynamic_section(&data, dynamic_addr, actual_base)?;
        
        // Extract library name from path
        let name = path.split('/').last().unwrap_or(path).to_string();
        
        let library = LoadedLibrary {
            base: actual_base,
            dynamic: dynamic_addr,
            symtab,
            strtab,
            strtab_size,
            hash,
            rela,
            rela_size,
            jmprel,
            jmprel_size,
            name: name.clone(),
            needed,
            entry: elf_info.entry,
        };
        
        // Add to loaded libraries
        self.libraries.insert(name.clone(), library.clone());
        
        // Process relocations
        self.process_relocations(&library, &data)?;
        
        // Add symbols to global symbol table
        self.add_symbols(&library, &data)?;
        
        Ok(library)
    }
    
    /// Resolve a symbol
    ///
    /// Searches for a symbol in loaded libraries and returns its address.
    ///
    /// # Arguments
    ///
    /// * `name` - Symbol name to resolve
    ///
    /// # Returns
    ///
    /// * `Some(address)` - Symbol address if found
    /// * `None` - Symbol not found
    pub fn resolve_symbol(&self, name: &str) -> Option<usize> {
        if let Some((lib_name, sym)) = self.global_symbols.get(name) {
            if let Some(lib) = self.libraries.get(lib_name) {
                return Some(lib.base + sym.st_value as usize);
            }
        }
        None
    }
    
    /// Process relocations for a library
    fn process_relocations(
        &self,
        library: &LoadedLibrary,
        data: &[u8],
    ) -> Result<(), ElfError> {
        // Process RELA relocations
        if let Some(rela_addr) = library.rela {
            let rela_size = library.rela_size;
            let rela_count = rela_size / size_of::<Rela>();
            
            for i in 0..rela_count {
                let offset = rela_addr + i * size_of::<Rela>();
                let rela = unsafe {
                    &*(data.as_ptr().add(offset - library.base) as *const Rela)
                };
                
                self.apply_relocation(library, rela, data)?;
            }
        }
        
        // Process PLT relocations (JUMP_SLOT)
        if let Some(jmprel_addr) = library.jmprel {
            let jmprel_size = library.jmprel_size;
            let jmprel_count = jmprel_size / size_of::<Rela>();
            
            for i in 0..jmprel_count {
                let offset = jmprel_addr + i * size_of::<Rela>();
                let rela = unsafe {
                    &*(data.as_ptr().add(offset - library.base) as *const Rela)
                };
                
                self.apply_relocation(library, rela, data)?;
            }
        }
        
        Ok(())
    }
    
    /// Apply a single relocation
    fn apply_relocation(
        &self,
        library: &LoadedLibrary,
        rela: &Rela,
        data: &[u8],
    ) -> Result<(), ElfError> {
        let r_type = (rela.r_info & 0xffffffff) as u32;
        let r_sym = (rela.r_info >> 32) as u32;
        
        let target_addr = library.base + rela.r_offset as usize;
        
        #[cfg(target_arch = "x86_64")]
        {
            use reloc_types::*;
            match r_type {
                R_X86_64_RELATIVE => {
                    // Relative relocation: value = base + addend
                    let value = library.base as u64 + rela.r_addend as u64;
                    unsafe {
                        *(target_addr as *mut u64) = value;
                    }
                }
                R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                    // Global data or PLT: resolve symbol
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        unsafe {
                            *(target_addr as *mut u64) = sym_addr as u64;
                        }
                    }
                }
                R_X86_64_64 => {
                    // 64-bit absolute: resolve symbol and add addend
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        let value = sym_addr as u64 + rela.r_addend as u64;
                        unsafe {
                            *(target_addr as *mut u64) = value;
                        }
                    }
                }
                R_X86_64_PC32 => {
                    // 32-bit PC-relative: resolve symbol and calculate offset
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        let value = sym_addr as i64 - target_addr as i64 + rela.r_addend;
                        unsafe {
                            *(target_addr as *mut i32) = value as i32;
                        }
                    }
                }
                _ => {
                    // Other relocation types not yet implemented
                }
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            use reloc_types::*;
            match r_type {
                R_AARCH64_RELATIVE => {
                    let value = library.base as u64 + rela.r_addend as u64;
                    unsafe {
                        *(target_addr as *mut u64) = value;
                    }
                }
                R_AARCH64_GLOB_DAT | R_AARCH64_JUMP_SLOT => {
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        unsafe {
                            *(target_addr as *mut u64) = sym_addr as u64;
                        }
                    }
                }
                R_AARCH64_ABS64 => {
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        let value = sym_addr as u64 + rela.r_addend as u64;
                        unsafe {
                            *(target_addr as *mut u64) = value;
                        }
                    }
                }
                _ => {}
            }
        }
        
        #[cfg(target_arch = "riscv64")]
        {
            use reloc_types::*;
            match r_type {
                R_RISCV_RELATIVE => {
                    let value = library.base as u64 + rela.r_addend as u64;
                    unsafe {
                        *(target_addr as *mut u64) = value;
                    }
                }
                R_RISCV_JUMP_SLOT => {
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        unsafe {
                            *(target_addr as *mut u64) = sym_addr as u64;
                        }
                    }
                }
                R_RISCV_64 => {
                    if let Some(sym_addr) = self.resolve_symbol_by_index(library, r_sym, data) {
                        let value = sym_addr as u64 + rela.r_addend as u64;
                        unsafe {
                            *(target_addr as *mut u64) = value;
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Resolve symbol by index
    fn resolve_symbol_by_index(
        &self,
        library: &LoadedLibrary,
        sym_index: u32,
        data: &[u8],
    ) -> Option<usize> {
        let sym_offset = library.symtab + (sym_index as usize * size_of::<Sym>());
        let sym = unsafe {
            &*(data.as_ptr().add(sym_offset - library.base) as *const Sym)
        };
        
        // Get symbol name from string table
        let name_offset = library.strtab + sym.st_name as usize;
        if name_offset >= library.strtab + library.strtab_size {
            return None;
        }
        
        let name = unsafe {
            let mut len = 0;
            let mut ptr = data.as_ptr().add(name_offset - library.base);
            while *ptr != 0 && len < 256 {
                len += 1;
                ptr = ptr.add(1);
            }
            core::str::from_utf8(core::slice::from_raw_parts(
                data.as_ptr().add(name_offset - library.base),
                len,
            )).ok()?
        };
        
        // Resolve symbol
        self.resolve_symbol(name)
    }
    
    /// Parse dynamic section
    fn parse_dynamic_section(
        &self,
        data: &[u8],
        dynamic_addr: usize,
        base: usize,
    ) -> Result<(usize, usize, usize, Option<usize>, Option<usize>, usize, Option<usize>, usize, Vec<String>), ElfError> {
        let mut symtab = 0;
        let mut strtab = 0;
        let mut strtab_size = 0;
        let mut hash = None;
        let mut rela = None;
        let mut rela_size = 0;
        let mut jmprel = None;
        let mut jmprel_size = 0;
        let mut needed = Vec::new();
        
        let mut offset = dynamic_addr - base;
        loop {
            if offset + size_of::<Dyn>() > data.len() {
                break;
            }
            
            let dyn_entry = unsafe {
                &*(data.as_ptr().add(offset) as *const Dyn)
            };
            
            match dyn_entry.d_tag {
                DT_NULL => break,
                DT_SYMTAB => symtab = base + dyn_entry.d_val as usize,
                DT_STRTAB => strtab = base + dyn_entry.d_val as usize,
                DT_STRSZ => strtab_size = dyn_entry.d_val as usize,
                DT_HASH | DT_GNU_HASH => hash = Some(base + dyn_entry.d_val as usize),
                DT_RELA => rela = Some(base + dyn_entry.d_val as usize),
                DT_RELASZ => rela_size = dyn_entry.d_val as usize,
                DT_JMPREL => jmprel = Some(base + dyn_entry.d_val as usize),
                DT_PLTRELSZ => jmprel_size = dyn_entry.d_val as usize,
                DT_NEEDED => {
                    let name_offset = strtab + dyn_entry.d_val as usize;
                    if name_offset < base + strtab + strtab_size {
                        let name = unsafe {
                            let mut len = 0;
                            let mut ptr = data.as_ptr().add(name_offset - base);
                            while *ptr != 0 && len < 256 {
                                len += 1;
                                ptr = ptr.add(1);
                            }
                            core::str::from_utf8(core::slice::from_raw_parts(
                                data.as_ptr().add(name_offset - base),
                                len,
                            )).ok()
                        };
                        if let Some(name) = name {
                            needed.push(name.to_string());
                        }
                    }
                }
                _ => {}
            }
            
            offset += size_of::<Dyn>();
        }
        
        Ok((symtab, strtab, strtab_size, hash, rela, rela_size, jmprel, jmprel_size, needed))
    }
    
    /// Add symbols from library to global symbol table
    fn add_symbols(
        &mut self,
        library: &LoadedLibrary,
        data: &[u8],
    ) -> Result<(), ElfError> {
        // Iterate through symbol table
        let mut offset = library.symtab - library.base;
        loop {
            if offset + size_of::<Sym>() > data.len() {
                break;
            }
            
            let sym = unsafe {
                &*(data.as_ptr().add(offset) as *const Sym)
            };
            
            // Get symbol name
            let name_offset = library.strtab + sym.st_name as usize;
            if name_offset < library.base + library.strtab + library.strtab_size {
                let name = unsafe {
                    let mut len = 0;
                    let mut ptr = data.as_ptr().add(name_offset - library.base);
                    while *ptr != 0 && len < 256 {
                        len += 1;
                        ptr = ptr.add(1);
                    }
                    core::str::from_utf8(core::slice::from_raw_parts(
                        data.as_ptr().add(name_offset - library.base),
                        len,
                    )).ok()
                };
                
                if let Some(name) = name {
                    // Only add exported symbols (not local)
                    let bind = (sym.st_info >> 4) & 0xf;
                    if bind == 1 || bind == 2 { // STB_GLOBAL or STB_WEAK
                        self.global_symbols.insert(name.to_string(), (library.name.clone(), *sym));
                    }
                }
            }
            
            offset += size_of::<Sym>();
            
            // Stop if we've processed all symbols (heuristic: stop after empty name)
            if sym.st_name == 0 && offset > library.symtab - library.base + size_of::<Sym>() * 10 {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Read library file from filesystem
    fn read_library_file(&self, path: &str) -> Result<Vec<u8>, ElfError> {
        // Try direct path first
        if let Ok(mut file) = crate::vfs::vfs().open(path, crate::posix::O_RDONLY as u32) {
            let mut data = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                match file.read(buffer.as_mut_ptr() as usize, buffer.len()) {
                    Ok(0) => break,
                    Ok(n) => data.extend_from_slice(&buffer[..n]),
                    Err(_) => break,
                }
            }
            return Ok(data);
        }
        
        // Try search paths
        for search_path in &self.search_paths {
            let full_path = format!("{}/{}", search_path, path);
            if let Ok(mut file) = crate::vfs::vfs().open(&full_path, crate::posix::O_RDONLY as u32) {
                let mut data = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    match file.read(buffer.as_mut_ptr() as usize, buffer.len()) {
                        Ok(0) => break,
                        Ok(n) => data.extend_from_slice(&buffer[..n]),
                        Err(_) => break,
                    }
                }
                return Ok(data);
            }
        }
        
        Err(ElfError::InvalidMagic) // File not found
    }
}

impl Clone for LoadedLibrary {
    fn clone(&self) -> Self {
        Self {
            base: self.base,
            dynamic: self.dynamic,
            symtab: self.symtab,
            strtab: self.strtab,
            strtab_size: self.strtab_size,
            hash: self.hash,
            rela: self.rela,
            rela_size: self.rela_size,
            jmprel: self.jmprel,
            jmprel_size: self.jmprel_size,
            name: self.name.clone(),
            needed: self.needed.clone(),
            entry: self.entry,
        }
    }
}

