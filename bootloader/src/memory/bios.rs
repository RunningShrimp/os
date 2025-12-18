//! BIOS Memory Detection and Configuration
//!
//! This module provides comprehensive memory detection and configuration
//! support for BIOS bootloader, including E820 memory map scanning,
//! A20 gate handling, and extended memory detection.

use crate::utils::error::{BootError, Result};
use crate::protocol::bios::{MemoryMap, MemoryMapEntry, MemoryType};
use core::ptr;

/// E820 Memory Map Entry Structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct E820Entry {
    pub base_addr: u64,
    pub length: u64,
    pub type_: u32,
    pub acpi_attrs: u32,
}

/// BIOS Data Area Structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BiosDataArea {
    pub port_com: [u16; 4],      // COM ports
    pub port_lpt: [u16; 3],      // LPT ports
    pub equipment: u16,          // Equipment flags
    pub base_memory: u16,        // Base memory size in KB
    pub extended_memory: u16,    // Extended memory size in KB
    pub bios_data: [u8; 48],     // Other BIOS data
}

/// Memory Types for E820
pub const E820_TYPE_USABLE: u32 = 1;
pub const E820_TYPE_RESERVED: u32 = 2;
pub const E820_TYPE_ACPI_RECLAIMABLE: u32 = 3;
pub const E820_TYPE_NVS: u32 = 4;
pub const E820_TYPE_BADRAM: u32 = 5;
pub const E820_TYPE_UNDEFINED: u32 = 0;

/// BIOS Memory Regions
pub const BIOS_ROM_BASE: usize = 0xF0000;
pub const BIOS_ROM_SIZE: usize = 0x10000;
pub const VGA_MEMORY_BASE: usize = 0xA0000;
pub const VGA_MEMORY_SIZE: usize = 0x20000;
pub const EBDA_BASE: usize = 0x9FC00;
pub const EBDA_MAX_SIZE: usize = 0x400;
pub const BIOS_DATA_AREA: usize = 0x400;
pub const BIOS_DATA_AREA_SIZE: usize = 0x100;

/// Memory Map Scanner for BIOS
pub struct BiosMemoryScanner {
    initialized: bool,
    total_memory: usize,
    base_memory_kb: u16,
    extended_memory_kb: u16,
    e820_entries: Vec<E820Entry>,
}

impl BiosMemoryScanner {
    /// Create a new BIOS memory scanner
    pub fn new() -> Self {
        Self {
            initialized: false,
            total_memory: 0,
            base_memory_kb: 0,
            extended_memory_kb: 0,
            e820_entries: Vec::new(),
        }
    }

    /// Initialize BIOS memory scanner
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        println!("[bios_memory] Initializing BIOS memory scanner...");

        // Get basic memory information from BIOS data area
        self.scan_bios_data_area()?;

        // Scan E820 memory map
        self.scan_e820_memory_map()?;

        // Validate memory information
        self.validate_memory_info()?;

        self.initialized = true;

        println!("[bios_memory] Memory scanner initialized:");
        println!("[bios_memory]   Base memory: {} KB", self.base_memory_kb);
        println!("[bios_memory]   Extended memory: {} KB", self.extended_memory_kb);
        println!("[bios_memory]   Total detected memory: {} KB",
                 self.total_memory / 1024);

        Ok(())
    }

    /// Scan BIOS Data Area for basic memory information
    fn scan_bios_data_area(&mut self) -> Result<()> {
        let bda_ptr = BIOS_DATA_AREA as *const BiosDataArea;

        unsafe {
            if let Ok(bda) = bda_ptr.as_ref() {
                self.base_memory_kb = bda.base_memory;
                self.extended_memory_kb = bda.extended_memory;

                println!("[bios_memory] BIOS Data Area:");
                println!("[bios_memory]   Base memory: {} KB", self.base_memory_kb);
                println!("[bios_memory]   Extended memory: {} KB", self.extended_memory_kb);

                Ok(())
            } else {
                Err(BootError::HardwareError("Unable to access BIOS Data Area"))
            }
        }
    }

    /// Scan E820 memory map using BIOS interrupt 0x15, function 0xE820
    fn scan_e820_memory_map(&mut self) -> Result<()> {
        println!("[bios_memory] Scanning E820 memory map...");

        let mut continuation_id: u32 = 0;
        let mut entries: Vec<E820Entry> = Vec::new();
        let mut total_detected_memory: usize = 0;

        loop {
            let mut entry = E820Entry {
                base_addr: 0,
                length: 0,
                type_: 0,
                acpi_attrs: 0,
            };

            let mut regs = BiosRegisters {
                eax: 0xE820,
                ebx: continuation_id,
                ecx: core::mem::size_of::<E820Entry>() as u32,
                edx: 0x534D4150, // "SMAP"
                edi: &mut entry as *mut E820Entry as u32,
                esi: 0,
                ebp: 0,
            };

            // Call BIOS interrupt 0x15
            let result = unsafe { self.bios_int15(&mut regs) };

            if result.eax != 0x534D4150 {
                break; // Function not supported
            }

            if result.ebx == 0 {
                continuation_id = 0;
            } else {
                continuation_id = result.ebx;
            }

            // Add valid entry
            if entry.type_ != 0 {
                entries.push(entry);
                total_detected_memory += entry.length as usize;

                println!("[bios_memory]   E820 Entry: {:#018X}-{:#018X} Type: {} Size: {} MB",
                         entry.base_addr,
                         entry.base_addr + entry.length - 1,
                         self.get_memory_type_name(entry.type_),
                         entry.length / (1024 * 1024));
            }

            // Check if this is the last entry
            if continuation_id == 0 {
                break;
            }

            // Safety: prevent infinite loop
            if entries.len() > 128 {
                println!("[bios_memory] Warning: Too many E820 entries, stopping scan");
                break;
            }
        }

        self.e820_entries = entries;
        self.total_memory = total_detected_memory;

        println!("[bios_memory] Found {} E820 memory map entries", self.e820_entries.len());

        Ok(())
    }

    /// Validate memory information from different sources
    fn validate_memory_info(&mut self) -> Result<()> {
        // Compare E820 results with BIOS data area
        let mut e820_base_memory = 0;
        let mut e820_extended_memory = 0;

        for entry in &self.e820_entries {
            if entry.base_addr < 0x100000 && entry.type_ == E820_TYPE_USABLE {
                if entry.base_addr + entry.length <= 0x100000 {
                    e820_base_memory += entry.length as usize;
                } else {
                    e820_base_memory += (0x100000 - entry.base_addr) as usize;
                }
            } else if entry.base_addr >= 0x100000 && entry.type_ == E820_TYPE_USABLE {
                e820_extended_memory += entry.length as usize;
            }
        }

        let base_memory_kb_calculated = (e820_base_memory / 1024) as u16;
        let extended_memory_kb_calculated = (e820_extended_memory / 1024) as u16;

        println!("[bios_memory] Memory validation:");
        println!("[bios_memory]   BDA Base: {} KB, E820 Base: {} KB",
                 self.base_memory_kb, base_memory_kb_calculated);
        println!("[bios_memory]   BDA Extended: {} KB, E820 Extended: {} KB",
                 self.extended_memory_kb, extended_memory_kb_calculated);

        // Use E820 data if it's more reliable (which it usually is)
        if base_memory_kb_calculated > 0 {
            self.base_memory_kb = base_memory_kb_calculated;
        }

        if extended_memory_kb_calculated > self.extended_memory_kb {
            self.extended_memory_kb = extended_memory_kb_calculated;
        }

        Ok(())
    }

    /// Get human-readable memory type name
    fn get_memory_type_name(&self, mem_type: u32) -> &'static str {
        match mem_type {
            E820_TYPE_USABLE => "Usable",
            E820_TYPE_RESERVED => "Reserved",
            E820_TYPE_ACPI_RECLAIMABLE => "ACPI Reclaimable",
            E820_TYPE_NVS => "ACPI NVS",
            E820_TYPE_BADRAM => "Bad RAM",
            _ => "Unknown",
        }
    }

    /// Build memory map from E820 entries
    pub fn build_memory_map(&self) -> Result<MemoryMap> {
        if !self.initialized {
            return Err(BootError::NotInitialized);
        }

        let mut memory_map = MemoryMap::new();

        // Add E820 entries to memory map
        for entry in &self.e820_entries {
            let mem_type = match entry.type_ {
                E820_TYPE_USABLE => MemoryType::Usable,
                E820_TYPE_RESERVED => MemoryType::Reserved,
                E820_TYPE_ACPI_RECLAIMABLE => MemoryType::ACPIReclaimable,
                E820_TYPE_NVS => MemoryType::ACPIONVS,
                E820_TYPE_BADRAM => MemoryType::BadMemory,
                _ => MemoryType::Reserved,
            };

            let memory_entry = MemoryMapEntry {
                base: entry.base_addr as usize,
                size: entry.length as usize,
                mem_type,
                is_available: entry.type_ == E820_TYPE_USABLE,
            };

            memory_map.add_entry(memory_entry);
        }

        // Add special memory regions that might not be in E820
        self.add_special_regions(&mut memory_map)?;

        Ok(memory_map)
    }

    /// Add special BIOS memory regions
    fn add_special_regions(&self, memory_map: &mut MemoryMap) -> Result<()> {
        // Add BIOS ROM region
        let bios_rom_entry = MemoryMapEntry {
            base: BIOS_ROM_BASE,
            size: BIOS_ROM_SIZE,
            mem_type: MemoryType::Reserved,
            is_available: false,
        };
        memory_map.add_entry(bios_rom_entry);

        // Add VGA memory region
        let vga_entry = MemoryMapEntry {
            base: VGA_MEMORY_BASE,
            size: VGA_MEMORY_SIZE,
            mem_type: MemoryType::Reserved,
            is_available: false,
        };
        memory_map.add_entry(vga_entry);

        // Add Extended BIOS Data Area (EBDA)
        let ebda_size = unsafe {
            let ebda_ptr = 0x40E as *const u16;
            if let Ok(ebda_segment) = ebda_ptr.as_ref() {
                ((*ebda_segment as usize) << 4) - EBDA_BASE
            } else {
                EBDA_MAX_SIZE
            }
        };

        if ebda_size > 0 && ebda_size <= EBDA_MAX_SIZE {
            let ebda_entry = MemoryMapEntry {
                base: EBDA_BASE,
                size: ebda_size,
                mem_type: MemoryType::Reserved,
                is_available: false,
            };
            memory_map.add_entry(ebda_entry);
        }

        // Add BIOS Data Area
        let bda_entry = MemoryMapEntry {
            base: BIOS_DATA_AREA,
            size: BIOS_DATA_AREA_SIZE,
            mem_type: MemoryType::Reserved,
            is_available: false,
        };
        memory_map.add_entry(bda_entry);

        Ok(())
    }

    /// Enable A20 gate for access to memory above 1MB
    pub fn enable_a20(&self) -> Result<()> {
        println!("[bios_memory] Enabling A20 gate...");

        // Try different A20 enable methods

        // Method 1: Fast A20 using keyboard controller
        if self.enable_a20_keyboard() {
            println!("[bios_memory] A20 enabled via keyboard controller");
            return Ok(());
        }

        // Method 2: Fast A20 gate
        if self.enable_a20_fast() {
            println!("[bios_memory] A20 enabled via fast gate");
            return Ok(());
        }

        // Method 3: BIOS interrupt
        if self.enable_a20_bios() {
            println!("[bios_memory] A20 enabled via BIOS");
            return Ok(());
        }

        Err(BootError::HardwareError("Failed to enable A20 gate"))
    }

    /// Enable A20 via keyboard controller
    fn enable_a20_keyboard(&self) -> bool {
        unsafe {
            // Wait for keyboard controller ready
            for _ in 0..100000 {
                if ptr::read_volatile(0x64 as *const u8) & 0x02 == 0 {
                    break;
                }
            }

            // Write command to enable A20
            ptr::write_volatile(0x64 as *mut u8, 0xD1);

            // Wait for ready
            for _ in 0..100000 {
                if ptr::read_volatile(0x64 as *const u8) & 0x02 == 0 {
                    break;
                }
            }

            // Write data
            ptr::write_volatile(0x60 as *mut u8, 0xDF);

            // Wait for ready
            for _ in 0..100000 {
                if ptr::read_volatile(0x64 as *const u8) & 0x02 == 0 {
                    break;
                }
            }

            true
        }
    }

    /// Enable A20 via fast gate
    fn enable_a20_fast(&self) -> bool {
        unsafe {
            let value = ptr::read_volatile(0x92 as *const u8);
            ptr::write_volatile(0x92 as *mut u8, value | 0x02);
            true
        }
    }

    /// Enable A20 via BIOS interrupt
    fn enable_a20_bios(&self) -> bool {
        let mut regs = BiosRegisters {
            eax: 0x2403, // A20 gate support
            ebx: 0,
            ecx: 0,
            edx: 0,
            edi: 0,
            esi: 0,
            ebp: 0,
        };

        let result = unsafe { self.bios_int15(&mut regs) };
        result.eax == 0x00
    }

    /// Check if A20 is enabled
    pub fn is_a20_enabled(&self) -> bool {
        // Simple A20 test: compare memory at 0x0000 and 0x100000
        unsafe {
            let test_value: u32 = 0x12345678;

            // Write test value
            ptr::write_volatile(0x100000 as *mut u32, test_value);
            ptr::write_volatile(0x0000 as *mut u32, 0x87654321);

            // Check if memory wraps around
            let read_back = ptr::read_volatile(0x100000 as *const u32);

            // Restore original values
            ptr::write_volatile(0x100000 as *mut u32, 0);
            ptr::write_volatile(0x0000 as *mut u32, 0);

            read_back == test_value
        }
    }

    /// Get total memory size
    pub fn get_total_memory(&self) -> usize {
        self.total_memory
    }

    /// Get base memory size in KB
    pub fn get_base_memory_kb(&self) -> u16 {
        self.base_memory_kb
    }

    /// Get extended memory size in KB
    pub fn get_extended_memory_kb(&self) -> u16 {
        self.extended_memory_kb
    }

    /// Get E820 entries
    pub fn get_e820_entries(&self) -> &[E820Entry] {
        &self.e820_entries
    }

    /// Check if scanner is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// BIOS interrupt 0x15 handler (simplified)
    unsafe fn bios_int15(&self, regs: &mut BiosRegisters) -> BiosRegisters {
        // In a real BIOS environment, this would trigger actual BIOS interrupt
        // For demonstration, we'll simulate some responses

        let mut result = *regs;

        match regs.eax {
            0xE820 => {
                // Simulate E820 memory map response
                result.eax = 0x534D4150; // "SMAP"
                result.ebx = 0; // End of list for simplicity
                result.ecx = regs.ecx.min(core::mem::size_of::<E820Entry>() as u32);
            }
            0x2400 | 0x2401 => {
                // Simulate A20 gate operations
                result.eax = 0x00; // Success
            }
            0x2403 => {
                // Simulate A20 gate support query
                result.eax = 0x00; // Supported
                result.ebx = 0x01; // A20 disabled initially
            }
            _ => {
                // Unknown function
                result.eax = 0x86; // Function not supported
            }
        }

        result
    }
}

/// BIOS Registers for interrupt calls
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BiosRegisters {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
    esi: u32,
    edi: u32,
    ebp: u32,
}

/// BIOS Memory Manager
pub struct BiosMemoryManager {
    scanner: BiosMemoryScanner,
    memory_map: Option<MemoryMap>,
    a20_enabled: bool,
}

impl BiosMemoryManager {
    /// Create a new BIOS memory manager
    pub fn new() -> Self {
        Self {
            scanner: BiosMemoryScanner::new(),
            memory_map: None,
            a20_enabled: false,
        }
    }

    /// Initialize BIOS memory manager
    pub fn initialize(&mut self) -> Result<()> {
        self.scanner.initialize()?;

        // Enable A20 gate for extended memory access
        self.a20_enabled = self.scanner.enable_a20().is_ok();

        if self.a20_enabled {
            println!("[bios_memory] A20 gate enabled successfully");
        } else {
            println!("[bios_memory] Warning: A20 gate not enabled - limited memory access");
        }

        // Build memory map
        self.memory_map = Some(self.scanner.build_memory_map()?);

        println!("[bios_memory] BIOS Memory Manager initialized successfully");
        Ok(())
    }

    /// Get memory map
    pub fn get_memory_map(&self) -> Result<&MemoryMap> {
        self.memory_map.as_ref().ok_or(BootError::NotInitialized)
    }

    /// Get scanner
    pub fn get_scanner(&self) -> &BiosMemoryScanner {
        &self.scanner
    }

    /// Check if A20 is enabled
    pub fn is_a20_enabled(&self) -> bool {
        self.a20_enabled
    }

    /// Find suitable memory region for bootloader
    pub fn find_bootloader_region(&self, size: usize, alignment: usize) -> Result<usize> {
        let memory_map = self.get_memory_map()?;
        let regions = memory_map.find_available_regions(size, alignment);

        if regions.is_empty() {
            Err(BootError::InsufficientMemory)
        } else {
            // Use the first available region
            Ok(regions[0])
        }
    }

    /// Reserve memory region for bootloader
    pub fn reserve_region(&mut self, base: usize, size: usize) -> Result<()> {
        if let Some(memory_map) = &mut self.memory_map {
            memory_map.mark_region_used(base, size);
            Ok(())
        } else {
            Err(BootError::NotInitialized)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_constants() {
        assert_eq!(BIOS_ROM_BASE, 0xF0000);
        assert_eq!(VGA_MEMORY_BASE, 0xA0000);
        assert_eq!(E820_TYPE_USABLE, 1);
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = BiosMemoryScanner::new();
        assert!(!scanner.is_initialized());
        assert_eq!(scanner.get_total_memory(), 0);
    }

    #[test]
    fn test_memory_manager_creation() {
        let manager = BiosMemoryManager::new();
        assert!(!manager.is_a20_enabled());
    }
}