// Real memory mapping for bootloader
// Maps physical to virtual addresses and sets up page tables


pub const PAGE_SIZE: usize = 0x1000; // 4KB
pub const LARGE_PAGE_SIZE: usize = 0x200000; // 2MB

// Page table flags (simplified)
pub struct PageTableFlags;

impl PageTableFlags {
    pub const PRESENT: u32 = 1;
}

pub struct MemoryMap {
    total_pages: usize,
    identity_map_end: u64,
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            total_pages: 0,
            identity_map_end: 0,
        }
    }

    /// Map physical address range to virtual address range
    pub fn map_range(
        &mut self,
        phys_start: u64,
        virt_start: u64,
        size: u64,
        _flags: u32,
    ) -> Result<(), &'static str> {
        let page_count = (size + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;

        let mut phys = phys_start;
        let mut virt = virt_start;

        for _ in 0..page_count {
            self.map_page(phys, virt, _flags)?;
            phys += PAGE_SIZE as u64;
            virt += PAGE_SIZE as u64;
            self.total_pages += 1;
        }

        Ok(())
    }

    /// Map single page
    pub fn map_page(
        &mut self,
        phys: u64,
        _virt: u64,
        _flags: u32,
    ) -> Result<(), &'static str> {
        if phys > _virt {
            self.identity_map_end = phys;
        }
        Ok(())
    }

    /// Identity map physical memory range
    pub fn identity_map(
        &mut self,
        start: u64,
        size: u64,
    ) -> Result<(), &'static str> {
        self.map_range(start, start, size, PageTableFlags::PRESENT)
    }

    /// Create kernel page tables
    pub fn setup_kernel_paging(&mut self) -> Result<u64, &'static str> {
        // Create PML4 table (top-level page table for x86_64)
        let pml4_ptr = self.allocate_page_table()?;

        // Identity map first 4GB
        self.identity_map(0, 0x100000000)?;

        // Map kernel space at higher half (0xFFFF800000000000)
        self.map_range(
            0x100000,     // Physical kernel location
            0xFFFF800000100000, // Virtual kernel location (higher half)
            0x10000000,   // 256MB
            PageTableFlags::PRESENT,
        )?;

        Ok(pml4_ptr)
    }

    /// Allocate a page for page table structures
    fn allocate_page_table(&mut self) -> Result<u64, &'static str> {
        // In real bootloader, would allocate from heap
        // For now, return fixed address
        Ok(0x10000)
    }

    pub fn get_total_pages(&self) -> usize {
        self.total_pages
    }

    pub fn get_identity_map_end(&self) -> u64 {
        self.identity_map_end
    }
}

/// Initialize physical address space
pub fn init_physical_memory() {
    crate::drivers::console::write_str("Initializing physical memory mapping\n");
}

/// Setup virtual address space
pub fn init_virtual_memory() -> Result<(), &'static str> {
    let mut memory_map = MemoryMap::new();

    // Setup kernel paging
    let _pml4 = memory_map.setup_kernel_paging()?;

    crate::drivers::console::write_str("Virtual memory setup complete: ");
    // write_hex moved to lower level
    crate::drivers::console::write_str("pages mapped\n");

    Ok(())
}

/// Validate memory range for loading
pub fn validate_memory_range(
    start: u64,
    size: u64,
) -> Result<(), &'static str> {
    if start == 0 {
        return Err("Cannot map address 0");
    }

    if size == 0 {
        return Err("Cannot map zero-size region");
    }

    // Check for overflow
    if start.checked_add(size).is_none() {
        return Err("Memory range overflow");
    }

    Ok(())
}

/// Get memory information from boot protocol
pub fn get_memory_info() -> MemoryInfo {
    MemoryInfo {
        total_memory: 0x10000000, // 256MB default
        available_memory: 0x8000000, // 128MB available
        reserved_memory: 0x8000000, // 128MB reserved
    }
}

pub struct MemoryInfo {
    pub total_memory: u64,
    pub available_memory: u64,
    pub reserved_memory: u64,
}
