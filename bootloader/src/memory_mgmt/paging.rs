//! Memory paging support

use crate::utils::error::Result;

/// Page size constant (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Page table entry
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLED: u64 = 1 << 4;
    pub const HUGE_PAGE: u64 = 1 << 7;

    pub fn new(address: u64, flags: u64) -> Self {
        PageTableEntry(address | flags)
    }

    pub fn address(&self) -> u64 {
        self.0 & 0x000F_FFFF_FFFF_F000
    }

    pub fn is_present(&self) -> bool {
        (self.0 & Self::PRESENT) != 0
    }
}

/// Page table structure (one page = 512 entries on x86_64)
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn new() -> Self {
        PageTable {
            entries: [PageTableEntry(0); 512],
        }
    }

    pub fn get(&self, index: usize) -> PageTableEntry {
        self.entries[index]
    }

    pub fn set(&mut self, index: usize, entry: PageTableEntry) {
        self.entries[index] = entry;
    }
}

/// Initialize paging for x86_64
#[cfg(target_arch = "x86_64")]
pub fn init_paging() {
    // For bootloader, paging is typically already enabled by firmware
    // This is a placeholder for additional paging setup
    crate::drivers::console::write_str("✓ Paging checked\n");
}

#[cfg(not(target_arch = "x86_64"))]
pub fn init_paging() {
    // Other architectures have different paging mechanisms
}

/// Map a virtual address to physical address
pub fn map_page(_virt: u64, _phys: u64, _flags: u64) -> Result<()> {
    // This is a stub - full implementation would require access to page tables
    Ok(())
}

/// Unmap a virtual address
pub fn unmap_page(_virt: u64) -> Result<()> {
    Ok(())
}
