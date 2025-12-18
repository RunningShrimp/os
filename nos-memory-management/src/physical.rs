//! Physical memory management module

use nos_api::Result;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;
/// Page shift (log2 of PAGE_SIZE)
pub const PAGE_SHIFT: usize = 12;

/// Align address down to page boundary
#[inline]
pub const fn page_round_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// Align address up to page boundary
#[inline]
pub const fn page_round_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Physical address to page number
#[inline]
pub const fn addr_to_pfn(addr: usize) -> usize {
    addr >> PAGE_SHIFT
}

/// Page number to physical address
#[inline]
pub const fn pfn_to_addr(pfn: usize) -> usize {
    pfn << PAGE_SHIFT
}

/// A physical address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    /// Creates a new physical address from a raw usize value.
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Returns the physical address as a raw usize value.
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Returns the offset within the current page.
    pub const fn page_offset(self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    /// Returns the page number for this physical address.
    pub const fn page_number(self) -> usize {
        self.0 >> PAGE_SHIFT
    }

    /// Checks if the physical address is page-aligned.
    pub const fn is_page_aligned(self) -> bool {
        self.page_offset() == 0
    }

    /// Rounds up the physical address to the next page boundary.
    pub const fn page_round_up(self) -> Self {
        Self(page_round_up(self.0))
    }

    /// Rounds down the physical address to the previous page boundary.
    pub const fn page_round_down(self) -> Self {
        Self(page_round_down(self.0))
    }
}

impl From<usize> for PhysAddr {
    fn from(addr: usize) -> Self {
        Self(addr)
    }
}

impl From<PhysAddr> for usize {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

/// Initialize physical memory management
pub fn initialize() -> Result<()> {
    // Initialize physical memory management
    Ok(())
}

/// Shutdown physical memory management
pub fn shutdown() -> Result<()> {
    // Shutdown physical memory management
    Ok(())
}

/// Get total physical memory
pub fn get_total_memory() -> usize {
    // Return total physical memory size
    1024 * 1024 * 1024 // 1GB placeholder
}

/// Get available physical memory
pub fn get_available_memory() -> usize {
    // Return available physical memory size
    512 * 1024 * 1024 // 512MB placeholder
}
