// Memory management and paging setup for bootloader


/// PML4 (Page Map Level 4) entry for x86_64
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Pml4Entry(u64);

impl Pml4Entry {
    pub const fn new(addr: u64, flags: u64) -> Self {
        Pml4Entry(addr | flags)
    }

    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER_ACCESSIBLE: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLED: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;

    pub fn address(&self) -> u64 {
        self.0 & 0x000FFFFFFFFFF000
    }

    pub fn is_present(&self) -> bool {
        self.0 & Self::PRESENT != 0
    }
}

/// Page Directory Pointer Table (PDPT) entry
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PdptEntry(u64);

impl PdptEntry {
    pub const fn new(addr: u64, flags: u64) -> Self {
        PdptEntry(addr | flags)
    }

    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER_ACCESSIBLE: u64 = 1 << 2;
}

/// Page Directory (PD) entry
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PdEntry(u64);

impl PdEntry {
    pub const fn new(addr: u64, flags: u64) -> Self {
        PdEntry(addr | flags)
    }

    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER_ACCESSIBLE: u64 = 1 << 2;
    pub const HUGE_PAGE: u64 = 1 << 7; // 2MB page
}

/// Page Table (PT) entry
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PtEntry(u64);

impl PtEntry {
    pub const fn new(addr: u64, flags: u64) -> Self {
        PtEntry(addr | flags)
    }

    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER_ACCESSIBLE: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLED: u64 = 1 << 4;
}

pub const PAGE_SIZE: u64 = 0x1000;
pub const LARGE_PAGE_SIZE: u64 = 0x200000; // 2MB
pub const HUGE_PAGE_SIZE: u64 = 0x40000000; // 1GB

/// Page table structure (512 entries * 8 bytes = 4096 bytes = 1 page)
#[repr(align(4096))]
pub struct PageTable {
    pub entries: [u64; 512],
}

impl PageTable {
    pub const fn new() -> Self {
        PageTable { entries: [0; 512] }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }
}

/// Paging manager
pub struct PagingManager {
    pml4: &'static mut PageTable,
}

impl PagingManager {
    pub fn new(pml4_addr: u64) -> Self {
        let pml4 = unsafe { &mut *(pml4_addr as *mut PageTable) };
        pml4.clear();

        Self { pml4 }
    }

    /// Setup identity mapping (physical = virtual)
    pub fn identity_map_region(
        &mut self,
        start: u64,
        size: u64,
    ) -> Result<(), &'static str> {
        let mut current = (start / PAGE_SIZE) * PAGE_SIZE;
        let end = ((start + size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;

        while current < end {
            self.map_page(current, current)?;
            current += PAGE_SIZE;
        }

        crate::drivers::console::write_str("Identity mapped\n");
        Ok(())
    }

    /// Map a single page
    pub fn map_page(&mut self, phys: u64, virt: u64) -> Result<(), &'static str> {
        let pml4_idx = (virt >> 39) & 0x1FF;
        let pdpt_idx = (virt >> 30) & 0x1FF;
        let pd_idx = (virt >> 21) & 0x1FF;
        let pt_idx = (virt >> 12) & 0x1FF;

        // PML4
        if self.pml4.entries[pml4_idx as usize] == 0 {
            // Allocate PDPT
            let pdpt_addr = self.allocate_table();
            self.pml4.entries[pml4_idx as usize] =
                pdpt_addr | Pml4Entry::PRESENT | Pml4Entry::WRITABLE;
        }

        let pdpt_addr =
            self.pml4.entries[pml4_idx as usize] & 0x000FFFFFFFFFF000;
        let pdpt = unsafe { &mut *(pdpt_addr as *mut PageTable) };

        // PDPT
        if pdpt.entries[pdpt_idx as usize] == 0 {
            let pd_addr = self.allocate_table();
            pdpt.entries[pdpt_idx as usize] =
                pd_addr | PdptEntry::PRESENT | PdptEntry::WRITABLE;
        }

        let pd_addr = pdpt.entries[pdpt_idx as usize] & 0x000FFFFFFFFFF000;
        let pd = unsafe { &mut *(pd_addr as *mut PageTable) };

        // PD
        if pd.entries[pd_idx as usize] == 0 {
            let pt_addr = self.allocate_table();
            pd.entries[pd_idx as usize] =
                pt_addr | PdEntry::PRESENT | PdEntry::WRITABLE;
        }

        let pt_addr = pd.entries[pd_idx as usize] & 0x000FFFFFFFFFF000;
        let pt = unsafe { &mut *(pt_addr as *mut PageTable) };

        // PT
        pt.entries[pt_idx as usize] = phys | PtEntry::PRESENT | PtEntry::WRITABLE;

        Ok(())
    }

    /// Allocate a page table structure
    fn allocate_table(&self) -> u64 {
        // In real implementation, would use allocator
        // For now, return fixed addresses
        static mut TABLE_COUNTER: u64 = 0x11000;
        unsafe {
            let addr = TABLE_COUNTER;
            TABLE_COUNTER += 0x1000;
            addr
        }
    }
}

/// Enable paging on x86_64
#[cfg(target_arch = "x86_64")]
pub fn enable_paging(pml4_addr: u64) -> Result<(), &'static str> {
    crate::drivers::console::write_str("Enabling paging\n");

    unsafe {
        // Load CR3 with PML4 address
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) pml4_addr,
            options(nomem, nostack)
        );

        // Enable PAE and PGE if not already enabled
        let mut cr4: u64;
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
            options(nomem, nostack)
        );

        // Set PAE (Physical Address Extension) bit
        cr4 |= 1 << 5; // PAE
        cr4 |= 1 << 7; // PGE (Page Global Enable)

        core::arch::asm!(
            "mov cr4, {}",
            in(reg) cr4,
            options(nomem, nostack)
        );

        // Enable paging in CR0
        let mut cr0: u64;
        core::arch::asm!(
            "mov {}, cr0",
            out(reg) cr0,
            options(nomem, nostack)
        );

        cr0 |= 1 << 31; // PG (Paging) bit

        core::arch::asm!(
            "mov cr0, {}",
            in(reg) cr0,
            options(nomem, nostack)
        );
    }

    crate::drivers::console::write_str("Paging enabled\n");
    Ok(())
}

/// Initialize paging system
pub fn initialize_paging() -> Result<u64, &'static str> {
    crate::drivers::console::write_str("Initializing paging system\n");

    let pml4_addr = 0x1000u64;
    let mut manager = PagingManager::new(pml4_addr);

    // Identity map first 4GB
    manager.identity_map_region(0, 0x100000000)?;

    // Enable paging
    #[cfg(target_arch = "x86_64")]
    enable_paging(pml4_addr)?;

    Ok(pml4_addr)
}
