//! Virtual Memory Management for xv6-rust
//!
//! This module provides virtual memory support including:
//! - Page table management for RISC-V Sv39, AArch64 and x86_64
//! - Kernel address space setup
//! - User address space management

extern crate alloc;

use core::ptr;
use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::sync::Mutex;

// ============================================================================
// Architecture-specific page table definitions
// ============================================================================

/// Page table entry flags
pub mod flags {
    pub const PTE_V: usize = 1 << 0;  // Valid
    pub const PTE_R: usize = 1 << 1;  // Readable
    pub const PTE_W: usize = 1 << 2;  // Writable
    pub const PTE_X: usize = 1 << 3;  // Executable
    pub const PTE_U: usize = 1 << 4;  // User accessible
    pub const PTE_G: usize = 1 << 5;  // Global
    pub const PTE_A: usize = 1 << 6;  // Accessed
    pub const PTE_D: usize = 1 << 7;  // Dirty
    
    // Software-defined flags (using reserved bits 8-9 for RISC-V)
    pub const PTE_COW: usize = 1 << 8;  // Copy-on-Write (software flag)
    pub const PTE_LAZY: usize = 1 << 9; // Lazy allocation (software flag)
}

/// Number of page table entries per page
pub const PTE_COUNT: usize = PAGE_SIZE / core::mem::size_of::<usize>();

/// Page table structure (architecture-agnostic wrapper)
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [usize; PTE_COUNT],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [0; PTE_COUNT],
        }
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = 0;
        }
    }
}

// Make PageTable available as vm::arch::PageTable for compatibility
pub mod arch {
    pub use super::PageTable;
}

// ============================================================================
// RISC-V Sv39 Virtual Memory
// ============================================================================

#[cfg(target_arch = "riscv64")]
mod riscv64 {
    use super::*;
    use super::flags::*;
    
    /// Virtual address bits for Sv39
    const VA_BITS: usize = 39;
    
    /// Page offset bits
    const PAGE_OFFSET_BITS: usize = 12;
    
    /// VPN bits per level
    const VPN_BITS: usize = 9;
    
    /// Extract VPN from virtual address
    #[inline]
    fn vpn(va: usize, level: usize) -> usize {
        (va >> (PAGE_OFFSET_BITS + level * VPN_BITS)) & 0x1FF
    }
    
    /// Convert physical address to PTE
    #[inline]
    pub fn pa_to_pte(pa: usize) -> usize {
        (pa >> 2) & !0x3FF
    }
    
    /// Convert PTE to physical address
    #[inline]
    pub fn pte_to_pa(pte: usize) -> usize {
        (pte & !0x3FF) << 2
    }
    
    /// Walk page table and return PTE pointer
    /// If alloc is true, allocate intermediate page tables as needed
    pub unsafe fn walk(
        pagetable: *mut PageTable,
        va: usize,
        alloc: bool,
    ) -> Option<*mut usize> {
        if va >= (1 << VA_BITS) {
            return None;
        }
        
        let mut pt = pagetable;
        
        for level in (1..3).rev() {
            let pte = &mut (*pt).entries[vpn(va, level)];
            
            if *pte & PTE_V != 0 {
                pt = pte_to_pa(*pte) as *mut PageTable;
            } else {
                if !alloc {
                    return None;
                }
                
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return None;
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                
                *pte = pa_to_pte(new_pt as usize) | PTE_V;
                pt = new_pt as *mut PageTable;
            }
        }
        
        Some(&mut (*pt).entries[vpn(va, 0)] as *mut usize)
    }
    
    /// Map a virtual address to a physical address
    pub unsafe fn map_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        perm: usize,
    ) -> Result<(), ()> {
        let pte = walk(pagetable, va, true).ok_or(())?;
        
        if *pte & PTE_V != 0 {
            // Already mapped
            return Err(());
        }
        
        *pte = pa_to_pte(pa) | perm | PTE_V;
        Ok(())
    }
    
    /// Unmap a virtual address
    pub unsafe fn unmap_page(pagetable: *mut PageTable, va: usize) -> Option<usize> {
        let pte = walk(pagetable, va, false)?;
        
        if *pte & PTE_V == 0 {
            return None;
        }
        
        let pa = pte_to_pa(*pte);
        *pte = 0;
        Some(pa)
    }
    
    /// Translate virtual address to physical address
    pub unsafe fn translate(pagetable: *mut PageTable, va: usize) -> Option<usize> {
        let pte = walk(pagetable, va, false)?;
        
        if *pte & PTE_V == 0 {
            return None;
        }
        
        let pa = pte_to_pa(*pte);
        Some(pa | (va & (PAGE_SIZE - 1)))
    }
    
    /// Activate a page table by writing to satp
    pub unsafe fn activate(pagetable: *mut PageTable) {
        let ppn = (pagetable as usize) >> 12;
        let satp = (8 << 60) | ppn; // Sv39 mode
        core::arch::asm!("csrw satp, {}", in(reg) satp);
        core::arch::asm!("sfence.vma");
    }
}

// ============================================================================
// AArch64 Virtual Memory
// ============================================================================

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    use super::*;
    use super::flags::*;
    
    /// AArch64 descriptor flags
    const DESC_VALID: usize = 1 << 0;
    const DESC_TABLE: usize = 1 << 1;
    const DESC_AF: usize = 1 << 10;
    const DESC_AP_RW: usize = 0 << 6;
    const DESC_AP_RO: usize = 2 << 6;
    const DESC_AP_USER: usize = 1 << 6;
    const DESC_UXN: usize = 1 << 54;
    const DESC_PXN: usize = 1 << 53;
    
    /// Extract index from virtual address for a given level
    #[inline]
    fn va_index(va: usize, level: usize) -> usize {
        (va >> (12 + (3 - level) * 9)) & 0x1FF
    }
    
    /// Walk page table
    pub unsafe fn walk(
        pagetable: *mut PageTable,
        va: usize,
        alloc: bool,
    ) -> Option<*mut usize> {
        let mut pt = pagetable;
        
        for level in 0..3 {
            let pte = &mut (*pt).entries[va_index(va, level)];
            
            if *pte & DESC_VALID != 0 {
                pt = (*pte & !0xFFF) as *mut PageTable;
            } else {
                if !alloc {
                    return None;
                }
                
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return None;
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                
                *pte = (new_pt as usize) | DESC_TABLE | DESC_VALID;
                pt = new_pt as *mut PageTable;
            }
        }
        
        Some(&mut (*pt).entries[va_index(va, 3)] as *mut usize)
    }
    
    /// Map a page
    pub unsafe fn map_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        perm: usize,
    ) -> Result<(), ()> {
        let pte = walk(pagetable, va, true).ok_or(())?;
        
        if *pte & DESC_VALID != 0 {
            return Err(());
        }
        
        let mut flags = DESC_VALID | DESC_AF;
        if perm & PTE_W == 0 {
            flags |= DESC_AP_RO;
        }
        if perm & PTE_U != 0 {
            flags |= DESC_AP_USER;
        }
        if perm & PTE_X == 0 {
            flags |= DESC_UXN | DESC_PXN;
        }
        
        *pte = (pa & !0xFFF) | flags;
        Ok(())
    }
    
    /// Activate page table
    pub unsafe fn activate(pagetable: *mut PageTable) {
        unsafe {
            core::arch::asm!("msr ttbr0_el1, {}", in(reg) pagetable);
            core::arch::asm!("isb");
            core::arch::asm!("tlbi vmalle1is");
            core::arch::asm!("dsb ish");
            core::arch::asm!("isb");
        }
    }
}

// ============================================================================
// x86_64 Virtual Memory
// ============================================================================

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use super::*;
    use super::flags::*;
    
    /// x86_64 page table entry flags
    const PTE_P: usize = 1 << 0;    // Present
    const PTE_RW: usize = 1 << 1;   // Read/Write
    const PTE_US: usize = 1 << 2;   // User/Supervisor
    const PTE_NX: usize = 1 << 63;  // No Execute
    
    /// Extract index from virtual address
    #[inline]
    fn va_index(va: usize, level: usize) -> usize {
        (va >> (12 + level * 9)) & 0x1FF
    }
    
    /// Walk page table
    pub unsafe fn walk(
        pagetable: *mut PageTable,
        va: usize,
        alloc: bool,
    ) -> Option<*mut usize> {
        let mut pt = pagetable;
        
        for level in (1..4).rev() {
            let pte = &mut (*pt).entries[va_index(va, level)];
            
            if *pte & PTE_P != 0 {
                pt = (*pte & !0xFFF) as *mut PageTable;
            } else {
                if !alloc {
                    return None;
                }
                
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return None;
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                
                *pte = (new_pt as usize) | PTE_P | PTE_RW | PTE_US;
                pt = new_pt as *mut PageTable;
            }
        }
        
        Some(&mut (*pt).entries[va_index(va, 0)] as *mut usize)
    }
    
    /// Map a page
    pub unsafe fn map_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        perm: usize,
    ) -> Result<(), ()> {
        let pte = walk(pagetable, va, true).ok_or(())?;
        
        if *pte & PTE_P != 0 {
            return Err(());
        }
        
        let mut flags = PTE_P;
        if perm & PTE_W != 0 {
            flags |= PTE_RW;
        }
        if perm & PTE_U != 0 {
            flags |= PTE_US;
        }
        if perm & PTE_X == 0 {
            flags |= PTE_NX;
        }
        
        *pte = (pa & !0xFFF) | flags;
        Ok(())
    }
    
    /// Activate page table
    pub unsafe fn activate(pagetable: *mut PageTable) {
        core::arch::asm!("mov cr3, {}", in(reg) pagetable);
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Wrapper for page table pointer to implement Send
struct PageTablePtr(*mut PageTable);
unsafe impl Send for PageTablePtr {}

/// Kernel page table
static KERNEL_PAGETABLE: Mutex<PageTablePtr> = Mutex::new(PageTablePtr(ptr::null_mut()));

/// Initialize virtual memory system
pub fn init() {
    // Allocate kernel page table
    let pt = kalloc();
    if pt.is_null() {
        panic!("vm::init: failed to allocate kernel page table");
    }
    
    unsafe {
        ptr::write_bytes(pt, 0, PAGE_SIZE);
    }
    
    *KERNEL_PAGETABLE.lock() = PageTablePtr(pt as *mut PageTable);
    
    // TODO: Map kernel memory regions
}

/// Get kernel page table
pub fn kernel_pagetable() -> *mut PageTable {
    KERNEL_PAGETABLE.lock().0
}

/// Create a new user page table
pub fn create_pagetable() -> Option<*mut PageTable> {
    let pt = kalloc();
    if pt.is_null() {
        return None;
    }
    
    unsafe {
        ptr::write_bytes(pt, 0, PAGE_SIZE);
    }
    
    Some(pt as *mut PageTable)
}

/// Free a page table and all its pages
pub unsafe fn free_pagetable(pagetable: *mut PageTable) {
    if pagetable.is_null() {
        return;
    }
    
    // TODO: Recursively free page table pages
    unsafe { kfree(pagetable as *mut u8); }
}

/// Map pages in a page table
pub unsafe fn map_pages(
    pagetable: *mut PageTable,
    va: usize,
    pa: usize,
    size: usize,
    perm: usize,
) -> Result<(), ()> {
    let mut va = va & !(PAGE_SIZE - 1);
    let mut pa = pa & !(PAGE_SIZE - 1);
    let end = (va + size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    
    while va < end {
        #[cfg(target_arch = "riscv64")]
        unsafe { riscv64::map_page(pagetable, va, pa, perm)?; }
        
        #[cfg(target_arch = "aarch64")]
        unsafe { aarch64::map_page(pagetable, va, pa, perm)?; }
        
        #[cfg(target_arch = "x86_64")]
        unsafe { x86_64::map_page(pagetable, va, pa, perm)?; }
        
        va += PAGE_SIZE;
        pa += PAGE_SIZE;
    }
    
    Ok(())
}

/// Activate a page table
pub unsafe fn activate(pagetable: *mut PageTable) {
    #[cfg(target_arch = "riscv64")]
    unsafe { riscv64::activate(pagetable); }
    
    #[cfg(target_arch = "aarch64")]
    unsafe { aarch64::activate(pagetable); }
    
    #[cfg(target_arch = "x86_64")]
    x86_64::activate(pagetable);
}

/// Copy data from kernel to user space
pub unsafe fn copyout(
    _pagetable: *mut PageTable,
    dst: usize,
    src: *const u8,
    len: usize,
) -> Result<(), ()> {
    // TODO: Implement proper page table walk and copy
    unsafe { ptr::copy_nonoverlapping(src, dst as *mut u8, len); }
    Ok(())
}

/// Copy data from user to kernel space
pub unsafe fn copyin(
    _pagetable: *mut PageTable,
    dst: *mut u8,
    src: usize,
    len: usize,
) -> Result<(), ()> {
    // TODO: Implement proper page table walk and copy
    unsafe { ptr::copy_nonoverlapping(src as *const u8, dst, len); }
    Ok(())
}

// ============================================================================
// Page Fault Handling
// ============================================================================

/// Page fault error codes
#[derive(Debug, Clone, Copy)]
pub enum PageFaultType {
    /// Page not present
    NotPresent,
    /// Permission violation (write to read-only)
    WriteProtection,
    /// User accessing kernel page
    UserKernel,
    /// Instruction fetch from non-executable page
    ExecuteProtection,
}

/// Page fault result
#[derive(Debug, Clone, Copy)]
pub enum PageFaultResult {
    /// Fault handled successfully
    Handled,
    /// Segmentation fault - terminate process
    SegFault,
    /// Out of memory
    OutOfMemory,
}

/// Handle a page fault
/// 
/// # Arguments
/// * `pagetable` - The page table that faulted
/// * `fault_addr` - The virtual address that caused the fault
/// * `is_write` - True if this was a write access
/// * `is_user` - True if fault occurred in user mode
/// * `is_exec` - True if fault was instruction fetch
pub unsafe fn handle_page_fault(
    pagetable: *mut PageTable,
    fault_addr: usize,
    is_write: bool,
    is_user: bool,
    _is_exec: bool,
) -> PageFaultResult {
    // Align to page boundary
    let va = fault_addr & !(PAGE_SIZE - 1);
    
    // Check if address is in valid range for user
    if is_user && va >= KERNEL_BASE {
        crate::println!("Page fault: user access to kernel space at {:#x}", fault_addr);
        return PageFaultResult::SegFault;
    }
    
    // Try to look up the PTE
    #[cfg(target_arch = "riscv64")]
    let pte_result = unsafe { riscv64::walk(pagetable, va, false) };
    
    #[cfg(target_arch = "aarch64")]
    let pte_result = unsafe { aarch64::walk(pagetable, va, false) };
    
    #[cfg(target_arch = "x86_64")]
    let pte_result = unsafe { x86_64::walk(pagetable, va, false) };
    
    match pte_result {
        Some(pte_ptr) => {
            let pte = unsafe { *pte_ptr };
            
            // Check for Copy-on-Write
            if is_write && (pte & flags::PTE_COW) != 0 {
                unsafe { return handle_cow_fault(pagetable, va, pte_ptr); }
            }
            
            // Check for lazy allocation (valid VMA but not yet allocated)
            // For now, treat unmapped pages in user space as potential lazy alloc
            if (pte & flags::PTE_V) == 0 && is_user {
                unsafe { return handle_lazy_alloc(pagetable, va, true, true, false); }
            }
            
            // Permission violation
            crate::println!("Page fault: permission violation at {:#x}, pte={:#x}", 
                fault_addr, pte);
            PageFaultResult::SegFault
        }
        None => {
            // Page table structure doesn't exist
            // This could be lazy allocation for user pages
            if is_user && va < USER_STACK_TOP {
                unsafe { return handle_lazy_alloc(pagetable, va, true, true, false); }
            }
            
            crate::println!("Page fault: unmapped address {:#x}", fault_addr);
            PageFaultResult::SegFault
        }
    }
}

/// Handle Copy-on-Write fault
unsafe fn handle_cow_fault(
    pagetable: *mut PageTable,
    va: usize,
    pte_ptr: *mut usize,
) -> PageFaultResult {
    let old_pte = unsafe { *pte_ptr };
    
    // Get the physical address of the old page
    #[cfg(target_arch = "riscv64")]
    let old_pa = riscv64::pte_to_pa(old_pte);
    
    #[cfg(not(target_arch = "riscv64"))]
    let old_pa = (old_pte & !0xFFF) as usize;
    
    // Allocate a new page
    let new_page = kalloc();
    if new_page.is_null() {
        return PageFaultResult::OutOfMemory;
    }
    
    // Copy the old page contents
    unsafe {
        ptr::copy_nonoverlapping(
            old_pa as *const u8,
            new_page,
            PAGE_SIZE,
        );
    }
    
    // Update PTE: remove COW flag, add write permission
    let new_pte = (old_pte & !flags::PTE_COW) | flags::PTE_W;
    
    #[cfg(target_arch = "riscv64")]
    {
        unsafe { *pte_ptr = riscv64::pa_to_pte(new_page as usize) | (new_pte & 0x3FF); }
    }
    
    #[cfg(not(target_arch = "riscv64"))]
    {
        unsafe { *pte_ptr = (new_page as usize & !0xFFF) | (new_pte & 0xFFF); }
    }
    
    // Flush TLB for this address
    flush_tlb_page(va);
    
    // TODO: Decrement reference count on old page and free if zero
    let _ = pagetable;
    
    PageFaultResult::Handled
}

/// Handle lazy allocation (demand paging)
unsafe fn handle_lazy_alloc(
    pagetable: *mut PageTable,
    va: usize,
    readable: bool,
    writable: bool,
    executable: bool,
) -> PageFaultResult {
    // Allocate a new page
    let page = kalloc();
    if page.is_null() {
        return PageFaultResult::OutOfMemory;
    }
    
    // Zero the page
    unsafe { ptr::write_bytes(page, 0, PAGE_SIZE); }
    
    // Build permissions
    let mut perm = flags::PTE_V | flags::PTE_U;
    if readable {
        perm |= flags::PTE_R;
    }
    if writable {
        perm |= flags::PTE_W;
    }
    if executable {
        perm |= flags::PTE_X;
    }
    
    // Map the page
    #[cfg(target_arch = "riscv64")]
    unsafe {
        if riscv64::map_page(pagetable, va, page as usize, perm).is_err() {
            kfree(page);
            return PageFaultResult::OutOfMemory;
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        if aarch64::map_page(pagetable, va, page as usize, perm).is_err() {
            kfree(page);
            return PageFaultResult::OutOfMemory;
        }
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if x86_64::map_page(pagetable, va, page as usize, perm).is_err() {
            kfree(page);
            return PageFaultResult::OutOfMemory;
        }
    }
    
    PageFaultResult::Handled
}

/// Flush TLB for a specific virtual address
#[inline]
pub fn flush_tlb_page(va: usize) {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("sfence.vma {}, zero", in(reg) va);
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vale1is, {}",
            "dsb ish",
            "isb",
            in(reg) va >> 12,
        );
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) va);
    }
}

/// Flush entire TLB
#[inline]
pub fn flush_tlb_all() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("sfence.vma");
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
        );
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // Reload CR3 to flush TLB
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        core::arch::asm!("mov cr3, {}", in(reg) cr3);
    }
}

/// Kernel base address
pub const KERNEL_BASE: usize = 0xFFFF_FFFF_8000_0000;

/// User stack top address
pub const USER_STACK_TOP: usize = 0x0000_7FFF_FFFF_F000;
