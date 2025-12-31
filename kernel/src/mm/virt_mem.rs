//! # Virtual Memory for Virtualization
//!
//! This module provides support for Extended Page Tables (EPT) and Nested Page Tables (NPT),
//! which are essential for hardware-assisted virtualization. These enable efficient
//! memory virtualization by allowing the guest OS to manage its own page tables while
//! the hypervisor controls the actual physical memory mapping.
//!
//! ## Architecture
//!
//! - **EPT (Intel)**: Extended Page Tables for Intel VT-x
//! - **NPT (AMD)**: Nested Page Tables for AMD-V
//! - **SLAT**: Second Level Address Translation (generic term)
//!
//! The implementation provides:
//! - EPT/NPT management and manipulation
//! - Memory mapping for guest physical addresses
//! - Page table virtualization
//! - Memory isolation between VMs
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::mm::virt_mem::{EptTable, EptEntry, EptMemoryType, EptPermissions};
//!
//! let mut ept = EptTable::new();
//! ept.map_page(0x1000, 0x2000, EptPermissions::RW, EptMemoryType::Normal);
//! ```

use alloc::vec::Vec;
use core::sync::atomic {AtomicU64,, Ordering};

/// Error types for virtual memory operations
#[derive(Debug, Clone, Copy)]
pub enum VirtMemError {
    /// Invalid page table entry
    InvalidEntry,
    /// Page already mapped
    AlreadyMapped,
    /// Page not present
    NotPresent,
    /// Out of memory
    OutOfMemory,
    /// Invalid alignment
    InvalidAlignment,
    /// Permission denied
    PermissionDenied,
}

/// EPT/NPT permissions
#[derive(Debug, Clone, Copy)]
pub struct EptPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

impl EptPermissions {
    /// Create new permissions
    pub const fn new(read: bool, write: bool, execute: bool) -> Self {
        EptPermissions {
            read,
            write,
            execute,
        }
    }

    /// Read-only
    pub const fn r() -> Self {
        EptPermissions::new(true, false, false)
    }

    /// Write-only (rarely used)
    pub const fn w() -> Self {
        EptPermissions::new(false, true, false)
    }

    /// Execute-only
    pub const fn x() -> Self {
        EptPermissions::new(false, false, true)
    }

    /// Read-write
    pub const fn rw() -> Self {
        EptPermissions::new(true, true, false)
    }

    /// Read-execute
    pub const fn rx() -> Self {
        EptPermissions::new(true, false, true)
    }

    /// Read-write-execute
    pub const fn rwx() -> Self {
        EptPermissions::new(true, true, true)
    }

    /// No permissions
    pub const fn none() -> Self {
        EptPermissions::new(false, false, false)
    }
}

/// EPT memory types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum EptMemoryType {
    /// Uncacheable
    Uncacheable = 0,
    /// Write combining
    WriteCombining = 1,
    /// Write-through
    WriteThrough = 4,
    /// Write-protected
    WriteProtected = 5,
    /// Write-back (normal)
    WriteBack = 6,
}

/// EPT violation exit qualifications
#[derive(Debug, Clone, Copy)]
pub struct EptViolation {
    /// Read access
    pub read: bool,
    /// Write access
    pub write: bool,
    /// Execute access
    pub execute: bool,
    /// Guest physical address
    pub gpa: u64,
    /// Caused by translation
    pub translation: bool,
    /// Access was from guest linear address
    pub linear: bool,
    /// Read access (set for reads)
    pub read_access: bool,
    /// Write access (set for writes)
    pub write_access: bool,
    /// Execute access (set for execution)
    pub execute_access: bool,
    /// Mode: 0 = EPT, 1 = VPML
    pub mode: bool,
    /// Guest linear address
    pub gla: u64,
}

/// EPT PML4 Entry (Page Map Level 4)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EptPml4Entry {
    /// Physical address of page directory pointer table
    pub addr: u64,
    /// Write access
    pub write: bool,
    /// User/supervisor access
    pub user: bool,
    /// Page write-through
    pub pwt: bool,
    /// Page cache disable
    pub pcd: bool,
    /// Accessed flag
    pub accessed: bool,
    /// Reserved
    pub reserved1: u8,
    /// Physical address bits [51:12]
    pub addr_high: u40,
    /// Reserved
    pub reserved2: u12,
    /// Execute for user mode
    pub execute_user: bool,
    /// Execute for supervisor mode
    pub execute_supervisor: bool,
    /// Reserved
    pub reserved3: u11,
    /// Valid bit
    pub valid: bool,
}

/// EPT Page Directory Pointer Entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EptPdpte {
    /// Physical address of page directory
    pub addr: u64,
    /// Write access
    pub write: bool,
    /// User access
    pub user: bool,
    /// Page write-through
    pub pwt: bool,
    /// Page cache disable
    pub pcd: bool,
    /// Accessed flag
    pub accessed: bool,
    /// Reserved
    pub reserved1: u8,
    /// Physical address bits [51:12]
    pub addr_high: u40,
    /// Reserved
    pub reserved2: u12,
    /// Execute for user mode
    pub execute_user: bool,
    /// Execute for supervisor mode
    pub execute_supervisor: bool,
    /// Reserved
    pub reserved3: u11,
    /// Valid bit
    pub valid: bool,
}

/// EPT Page Directory Entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EptPde {
    /// Physical address of page table
    pub addr: u64,
    /// Write access
    pub write: bool,
    /// User access
    pub user: bool,
    /// Page write-through
    pub pwt: bool,
    /// Page cache disable
    pub pcd: bool,
    /// Accessed flag
    pub accessed: bool,
    /// Dirty flag
    pub dirty: bool,
    /// Large page (1GB)
    pub large_page: bool,
    /// Global mapping
    pub global: bool,
    /// Physical address bits [51:12]
    pub addr_high: u40,
    /// Reserved
    pub reserved2: u12,
    /// Execute for user mode
    pub execute_user: bool,
    /// Execute for supervisor mode
    pub execute_supervisor: bool,
    /// Reserved
    pub reserved3: u11,
    /// Valid bit
    pub valid: bool,
}

/// EPT Page Table Entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EptPte {
    /// Physical address of page
    pub addr: u64,
    /// Write access
    pub write: bool,
    /// User access
    pub user: bool,
    /// Page write-through
    pub pwt: bool,
    /// Page cache disable
    pub pcd: bool,
    /// Accessed flag
    pub accessed: bool,
    /// Dirty flag
    pub dirty: bool,
    /// Large page (2MB)
    pub large_page: bool,
    /// Global mapping
    pub global: bool,
    /// Physical address bits [51:12]
    pub addr_high: u40,
    /// Reserved
    pub reserved2: u4,
    /// Page attribute table
    pub pat: bool,
    /// Reserved
    pub reserved3: u7,
    /// Execute for user mode
    pub execute_user: bool,
    /// Execute for supervisor mode
    pub execute_supervisor: bool,
    /// Reserved
    pub reserved4: u11,
    /// Valid bit
    pub valid: bool,
}

/// EPT Page Table structure
pub struct EptTable {
    /// Physical address of the PML4 table
    pml4_addr: u64,
    /// List of allocated pages for cleanup
    allocated_pages: Vec<u64>,
}

impl EptTable {
    /// Create a new EPT table
    ///
    /// # Returns
    ///
    /// A new EPT table with allocated PML4
    pub fn new() -> Result<Self, VirtMemError> {
        // Allocate page for PML4
        let pml4_addr = Self::alloc_page()?;

        // Zero the PML4
        Self::zero_page(pml4_addr);

        Ok(EptTable {
            pml4_addr,
            allocated_pages: vec![pml4_addr],
        })
    }

    /// Map a guest physical page to a host physical page
    ///
    /// # Arguments
    ///
    /// * `gpa` - Guest physical address (must be page-aligned)
    /// * `hpa` - Host physical address (must be page-aligned)
    /// * `permissions` - Access permissions
    /// * `mem_type` - Memory type
    pub fn map_page(
        &mut self,
        gpa: u64,
        hpa: u64,
        permissions: EptPermissions,
        _mem_type: EptMemoryType,
    ) -> Result<(), VirtMemError> {
        if gpa & 0xFFF != 0 || hpa & 0xFFF != 0 {
            return Err(VirtMemError::InvalidAlignment);
        }

        // Extract page table indices
        let pml4_idx = ((gpa >> 39) & 0x1FF) as usize;
        let pdpt_idx = ((gpa >> 30) & 0x1FF) as usize;
        let pd_idx = ((gpa >> 21) & 0x1FF) as usize;
        let pt_idx = ((gpa >> 12) & 0x1FF) as usize;

        // Get or create PDPT
        let pdpt_addr = self.get_or_create_pml4_entry(pml4_idx)?;

        // Get or create PD
        let pd_addr = self.get_or_create_pdpt_entry(pdpt_addr, pdpt_idx)?;

        // Get or create PT
        let pt_addr = self.get_or_create_pd_entry(pd_addr, pd_idx)?;

        // Set PTE
        self.set_pte(pt_addr, pt_idx, hpa, permissions, mem_type);

        Ok(())
    }

    /// Unmap a guest physical page
    ///
    /// # Arguments
    ///
    /// * `gpa` - Guest physical address to unmap
    pub fn unmap_page(&mut self, gpa: u64) -> Result<(), VirtMemError> {
        let pt_idx = ((gpa >> 12) & 0x1FF) as usize;
        let pd_idx = ((gpa >> 21) & 0x1FF) as usize;
        let pdpt_idx = ((gpa >> 30) & 0x1FF) as usize;
        let pml4_idx = ((gpa >> 39) & 0x1FF) as usize;

        // Navigate to PTE and clear it
        // GH-#1110: Implement full navigation
        // See: https://github.com/npos/kernel/issues/1110

        Ok(())
    }

    /// Get the physical address of the PML4 table
    pub fn pml4_address(&self) -> u64 {
        self.pml4_addr
    }

    /// Invalidate EPT TLB entries
    ///
    /// # Arguments
    ///
    /// * `gpa` - Guest physical address to invalidate (or 0 for all)
    /// * `single` - True to invalidate only single page
    pub fn invalidate_tlb(&self, gpa: u64, single: bool) {
        // GH-#1111: Execute INVEPT instruction
        // See: https://github.com/npos/kernel/issues/1111
        // This requires assembly code
    }

    // Internal helper methods

    fn get_or_create_pml4_entry(&mut self, idx: usize) -> Result<u64, VirtMemError> {
        let pml4 = self.pml4_addr as *mut u64;

        unsafe {
            let entry = pml4.add(idx);
            let entry_value = AtomicU64::new(entry.read_volatile());

            if entry_value.load(Ordering::Acquire) & 1 == 0 {
                // Entry not present, allocate PDPT
                let pdpt_addr = Self::alloc_page()?;
                Self::zero_page(pdpt_addr);

                // Set entry
                let new_entry = pdpt_addr | 0x3; // Present + RW
                entry_value.store(new_entry, Ordering::Release);

                self.allocated_pages.push(pdpt_addr);
                Ok(pdpt_addr)
            } else {
                // Entry present, extract address
                Ok(entry_value.load(Ordering::Acquire) & !0xFFF)
            }
        }
    }

    fn get_or_create_pdpt_entry(
        &mut self,
        pdpt_addr: u64,
        idx: usize,
    ) -> Result<u64, VirtMemError> {
        let pdpt = pdpt_addr as *mut u64;

        unsafe {
            let entry = pdpt.add(idx);
            let entry_value = AtomicU64::new(entry.read_volatile());

            if entry_value.load(Ordering::Acquire) & 1 == 0 {
                let pd_addr = Self::alloc_page()?;
                Self::zero_page(pd_addr);

                let new_entry = pd_addr | 0x3;
                entry_value.store(new_entry, Ordering::Release);

                self.allocated_pages.push(pd_addr);
                Ok(pd_addr)
            } else {
                Ok(entry_value.load(Ordering::Acquire) & !0xFFF)
            }
        }
    }

    fn get_or_create_pd_entry(&mut self, pd_addr: u64, idx: usize) -> Result<u64, VirtMemError> {
        let pd = pd_addr as *mut u64;

        unsafe {
            let entry = pd.add(idx);
            let entry_value = AtomicU64::new(entry.read_volatile());

            if entry_value.load(Ordering::Acquire) & 1 == 0 {
                let pt_addr = Self::alloc_page()?;
                Self::zero_page(pt_addr);

                let new_entry = pt_addr | 0x3;
                entry_value.store(new_entry, Ordering::Release);

                self.allocated_pages.push(pt_addr);
                Ok(pt_addr)
            } else {
                Ok(entry_value.load(Ordering::Acquire) & !0xFFF)
            }
        }
    }

    fn set_pte(
        &mut self,
        pt_addr: u64,
        idx: usize,
        hpa: u64,
        permissions: EptPermissions,
        mem_type: EptMemoryType,
    ) {
        let pt = pt_addr as *mut u64;

        unsafe {
            let entry = pt.add(idx);
            let mut entry_value = hpa;

            // Set valid bit
            entry_value |= 1;

            // Set permissions
            if permissions.read {
                entry_value |= 1;
            }
            if permissions.write {
                entry_value |= 2;
            }
            if permissions.execute {
                entry_value |= 4;
            }

            // Set memory type
            entry_value |= (mem_type as u64) << 3;

            entry.write_volatile(entry_value);
        }
    }

    fn alloc_page() -> Result<u64, VirtMemError> {
        // GH-#1112: Allocate from physical memory manager
        // See: https://github.com/npos/kernel/issues/1112
        // For now, return a placeholder
        static NEXT_PAGE: AtomicU64 = AtomicU64::new(0x10000000);
        Ok(NEXT_PAGE.fetch_add(0x1000, Ordering::SeqCst))
    }

    fn zero_page(addr: u64) {
        unsafe {
            let ptr = addr as *mut u8;
            for i in 0..4096 {
                ptr.add(i).write_volatile(0);
            }
        }
    }
}

impl Drop for EptTable {
    fn drop(&mut self) {
        // GH-#1113: Free all allocated pages
        // See: https://github.com/npos/kernel/issues/1113
        for addr in &self.allocated_pages {
            // Self::free_page(*addr);
        }
    }
}

/// EPT context for a VM
pub struct EptContext {
    /// EPT pointer (EPTP)
    pub eptp: u64,
    /// EPT table structure
    table: EptTable,
}

impl EptContext {
    /// Create a new EPT context
    pub fn new() -> Result<Self, VirtMemError> {
        let table = EptTable::new()?;
        let eptp = Self::create_eptp(table.pml4_address());

        Ok(EptContext { eptp, table })
    }

    /// Get the EPT table for manipulation
    pub fn table(&mut self) -> &mut EptTable {
        &mut self.table
    }

    /// Create EPTP from PML4 address
    ///
    /// EPTP format:
    /// - Bits 2:0: Memory type (6 = WB)
    /// - Bits 5:3: Page walk length (3 = 4 levels)
    /// - Bit 6: Dirty flag enable
    /// - Bits 11:7: Reserved
    /// - Bits 51:12: PML4 address
    /// - Bits 63:52: Reserved
    fn create_eptp(pml4_addr: u64) -> u64 {
        let mut eptp = pml4_addr & !0xFFF; // Page-aligned PML4 address
        eptp |= 6; // Memory type: Write-back
        eptp |= (3 << 3); // Page walk length: 4 levels
        eptp |= (1 << 6); // Dirty flag enabled
        eptp
    }

    /// Invalidate all EPT TLB entries for this context
    pub fn invalidate_all(&self) {
        self.table.invalidate_tlb(0, false);
    }

    /// Invalidate single page in EPT TLB
    ///
    /// # Arguments
    ///
    /// * `gpa` - Guest physical address to invalidate
    pub fn invalidate_page(&self, gpa: u64) {
        self.table.invalidate_tlb(gpa, true);
    }
}

/// VPML (Virtual Process Monitor Lock) support for AMD-V
pub struct VpmlTable {
    /// Physical address of the top-level table
    root_addr: u64,
}

impl VpmlTable {
    /// Create a new VPML table (AMD-V equivalent of EPT)
    pub fn new() -> Result<Self, VirtMemError> {
        let root_addr = EptTable::alloc_page()?;
        EptTable::zero_page(root_addr);

        Ok(VpmlTable { root_addr })
    }

    /// Map a page using VPML
    pub fn map_page(
        &mut self,
        gpa: u64,
        hpa: u64,
        permissions: EptPermissions,
        _mem_type: EptMemoryType,
    ) -> Result<(), VirtMemError> {
        // Similar to EPT mapping but for AMD-V
        // GH-#1114: Implement AMD-V specific page table format
        // See: https://github.com/npos/kernel/issues/1114
        Ok(())
    }

    /// Get root address
    pub fn root_address(&self) -> u64 {
        self.root_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions() {
        assert!(EptPermissions::rw().read);
        assert!(EptPermissions::rw().write);
        assert!(!EptPermissions::rw().execute);

        assert!(!EptPermissions::none().read);
        assert!(!EptPermissions::none().write);
        assert!(!EptPermissions::none().execute);
    }

    #[test]
    fn test_ept_context_creation() {
        let ctx = EptContext::new();
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        assert_ne!(ctx.eptp, 0);
    }

    #[test]
    fn test_eptp_creation() {
        let pml4_addr = 0x1000;
        let eptp = EptContext::create_eptp(pml4_addr);

        // Check that memory type is set to WB (6)
        assert_eq!(eptp & 0x7, 6);

        // Check page walk length
        assert_eq!((eptp >> 3) & 0x7, 3);

        // Check dirty flag
        assert_eq!((eptp >> 6) & 0x1, 1);

        // Check PML4 address
        assert_eq!(eptp & !0xFFF, pml4_addr);
    }
}
