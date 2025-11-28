//! Virtual Memory Management for xv6-rust
//!
//! This module provides virtual memory support including:
//! - Page table management for RISC-V Sv39, AArch64 and x86_64
//! - Kernel address space setup
//! - User address space management

extern crate alloc;

use core::ptr;
use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::platform;
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
    pub const PTE_DEV: usize = 1 << 10; // Device memory (software flag)
    pub const PTE_DEV_STRONG: usize = 1 << 11; // Strongly-ordered device
    pub const PTE_DEV_WC: usize = 1 << 12; // Write-combining device (x86 only)
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
        
        let mut flags = perm | PTE_V;
        
        // Handle Svpbmt for Device memory
        if perm & PTE_DEV != 0 {
             // Svpbmt IO mode: Bit 62=1, Bit 61=0
             // If hardware doesn't support Svpbmt, these bits are reserved (ignored) or cause fault?
             // Usually ignored on non-Svpbmt hardware if 0.
             // We assume Svpbmt support or that it's safe.
             const PTE_PBMT_IO: usize = 1 << 62;
             flags |= PTE_PBMT_IO;
             
             // Also ensure it's not cacheable in standard bits if relevant (RISC-V doesn't have separate C bit in standard Sv39)
        }

        *pte = pa_to_pte(pa) | flags;
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
    pub unsafe fn activate_pt(pagetable: *mut PageTable) {
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
        // Device memory attribute index placeholder (requires MAIR setup): set AttrIndx=0 for normal, 1 for device
        const DESC_ATTR_INDX_SHIFT: usize = 2;
        const DESC_ATTR_DEV: usize = 1 << DESC_ATTR_INDX_SHIFT;
        const DESC_ATTR_DEV_STRONG: usize = 2 << DESC_ATTR_INDX_SHIFT;
        
        if perm & super::flags::PTE_DEV_STRONG != 0 { flags |= DESC_ATTR_DEV_STRONG; }
        else if perm & super::flags::PTE_DEV != 0 { flags |= DESC_ATTR_DEV; }
        
        *pte = (pa & !0xFFF) | flags;
        Ok(())
    }
    
    /// Setup MAIR_EL1 with memory attribute encodings
    /// AttrIdx 0: Normal WB RA WA (0xFF)
    /// AttrIdx 1: Device nGnRE (0x04)
    /// AttrIdx 2: Device nGnRnE (0x00)
    pub unsafe fn setup_mair() {
        let mair: u64 = (0x00u64 << 16) | (0x04u64 << 8) | 0xFFu64;
        core::arch::asm!("msr mair_el1, {}", in(reg) mair);
        core::arch::asm!("isb");
    }

    /// Setup TCR_EL1 for 4KB pages, inner/outer WB WA, inner-shareable, 48-bit VA
    pub unsafe fn setup_tcr() {
        // TCR_EL1 fields for T0: T0SZ=16 (48-bit VA), TG0=00 (4KB), SH0=11 (inner shareable),
        // IRGN0=01 (WB WA), ORGN0=01 (WB WA)
        let tcr: u64 = (16u64) | (0b00u64 << 14) | (0b11u64 << 12) | (0b01u64 << 8) | (0b01u64 << 10);
        core::arch::asm!("msr tcr_el1, {}", in(reg) tcr);
        core::arch::asm!("isb");
    }
    
    pub unsafe fn activate_pt(pagetable: *mut PageTable) {
        let ttbr0 = pagetable as u64;
        core::arch::asm!("msr ttbr0_el1, {}", in(reg) ttbr0);
        core::arch::asm!("isb");
        core::arch::asm!("tlbi vmalle1");
        core::arch::asm!("dsb nsh");
        core::arch::asm!("isb");
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
    const PTE_PWT: usize = 1 << 3;  // Write-Through
    const PTE_PCD: usize = 1 << 4;  // Cache-Disable
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
        
        // Handle Device Memory (PTE_DEV)
        // If PAT is set up as: 0=WB, 1=WC, 2=UC
        // UC: Index 2 -> PAT=0, PCD=1, PWT=0
        // WC: Index 1 -> PAT=0, PCD=0, PWT=1
        if perm & PTE_DEV_WC != 0 {
            flags |= PTE_PWT;
        } else if perm & PTE_DEV != 0 {
            flags |= PTE_PCD;
        }
        
        *pte = (pa & !0xFFF) | flags;
        Ok(())
    }
    
    /// Activate page table
    pub unsafe fn activate_pt(pagetable: *mut PageTable) {
        core::arch::asm!("mov cr3, {}", in(reg) pagetable);
    }

    /// Setup PAT MSR with default WB/WC/UC entries
    pub unsafe fn setup_pat() {
        // IA32_PAT MSR (0x277)
        // PA0: WB (06), PA1: WC (01), PA2: UC (00)
        let pat_val: u64 = (0x00 << 16) | (0x01 << 8) | 0x06;
        let msr = 0x277;
        let low = pat_val as u32;
        let high = (pat_val >> 32) as u32;
        core::arch::asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high);
    }

    pub unsafe fn setup_mtrr_mmio_uc() {
        let mut eax: u32 = 1;
        let mut ebx: u32 = 0;
        let mut ecx: u32 = 0;
        let mut edx: u32 = 0;
        core::arch::asm!(
            "cpuid",
            inlateout("eax") eax => eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
        );
        if (edx & (1<<12)) == 0 { return; }
        let msr_cap: u32 = 0xFE;
        let mut cap_lo: u32 = 0;
        let mut cap_hi: u32 = 0;
        core::arch::asm!("rdmsr", in("ecx") msr_cap, out("eax") cap_lo, out("edx") cap_hi);
        let var_cnt = (cap_lo & 0xFF) as usize;
        let def_msr: u32 = 0x2FF;
        let mut def_lo: u32 = 0;
        let mut def_hi: u32 = 0;
        core::arch::asm!("rdmsr", in("ecx") def_msr, out("eax") def_lo, out("edx") def_hi);
        def_lo = (def_lo & !0xFF) | 0x06;
        def_lo |= 1 << 11;
        core::arch::asm!("wrmsr", in("ecx") def_msr, in("eax") def_lo, in("edx") def_hi);
        let mut idx = 0usize;
        let mut regions: alloc::vec::Vec<(usize, usize, u8)> = alloc::vec::Vec::new();
        for (b, s) in crate::mm::mmio_regions_strong() { regions.push((b, s, 2)); }
        for (b, s) in crate::mm::mmio_regions_wc() { regions.push((b, s, 1)); }
        for (b, s) in crate::mm::mmio_regions() { regions.push((b, s, 0)); }
        regions.sort_by(|a, b| {
            match b.2.cmp(&a.2) {
                core::cmp::Ordering::Equal => b.1.cmp(&a.1),
                o => o,
            }
        });
        let total: u64 = regions.iter().map(|(_, s, _)| s as u64).sum();
        let mut covered: u64 = 0;
        for (base, size, class) in regions.into_iter() {
            let mut cur_base = base as u64;
            let mut cur_size = size as u64;
            while cur_size != 0 && idx < var_cnt {
                let lowbit = cur_base & (!cur_base + 1);
                let mut blk = 1u64 << (63 - cur_size.leading_zeros() as u64);
                if blk > cur_size { blk = cur_size; }
                if lowbit < blk { blk = lowbit; }
                let base_msr = 0x200 + (idx as u32) * 2;
                let mask_msr = base_msr + 1;
                let mtrr_type: u64 = if class == 1 { 0x01 } else { 0x00 }; // WC=1, UC=0
                let physbase = (cur_base & !0xFFF) | mtrr_type;
                let physmask = (! (blk - 1)) & !0xFFF;
                let physmask_lo = (physmask as u32) | (1 << 11);
                let physmask_hi = (physmask >> 32) as u32;
                let physbase_lo = physbase as u32;
                let physbase_hi = (physbase >> 32) as u32;
                core::arch::asm!("wrmsr", in("ecx") base_msr, in("eax") physbase_lo, in("edx") physbase_hi);
                core::arch::asm!("wrmsr", in("ecx") mask_msr, in("eax") physmask_lo, in("edx") physmask_hi);
                cur_base += blk;
                cur_size -= blk;
                idx += 1;
                covered += blk;
            }
            if idx >= var_cnt { break; }
        }
        let left = total.saturating_sub(covered);
        crate::mm::mmio_record_mtrr_usage(idx.min(var_cnt), var_cnt, covered, left);
        let (prev_used, prev_total, prev_cov, prev_left) = crate::mm::mmio_last_usage();
        crate::println!("[mtrr] covered={} bytes, left={} bytes (prev: used={}/{} covered={} left={})", covered, left, prev_used, prev_total, prev_cov, prev_left);
    }

    pub unsafe fn refresh_mtrr_from_stats() {
        let mut eax: u32 = 1;
        let mut ebx: u32 = 0;
        let mut ecx: u32 = 0;
        let mut edx: u32 = 0;
        core::arch::asm!(
            "cpuid",
            inlateout("eax") eax => eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
        );
        if (edx & (1<<12)) == 0 { return; }
        let msr_cap: u32 = 0xFE;
        let mut cap_lo: u32 = 0;
        let mut cap_hi: u32 = 0;
        core::arch::asm!("rdmsr", in("ecx") msr_cap, out("eax") cap_lo, out("edx") cap_hi);
        let var_cnt = (cap_lo & 0xFF) as usize;
        let def_msr: u32 = 0x2FF;
        let mut def_lo: u32 = 0;
        let mut def_hi: u32 = 0;
        core::arch::asm!("rdmsr", in("ecx") def_msr, out("eax") def_lo, out("edx") def_hi);
        def_lo = (def_lo & !0xFF) | 0x06;
        def_lo |= 1 << 11;
        core::arch::asm!("wrmsr", in("ecx") def_msr, in("eax") def_lo, in("edx") def_hi);
        let mut stats = crate::mm::mmio_stats_take();
        stats.sort_by(|a, b| {
            match b.2.cmp(&a.2) {
                core::cmp::Ordering::Equal => b.3.cmp(&a.3),
                o => o,
            }
        });
        let mut idx = 0usize;
        let mut covered: u64 = 0;
        let total: u64 = stats.iter().map(|(_, s, _, _)| s as u64).sum();
        for (base, size, priority, _) in stats.into_iter() {
            let mut cur_base = base as u64;
            let mut cur_size = size as u64;
            while cur_size != 0 && idx < var_cnt {
                let lowbit = cur_base & (!cur_base + 1);
                let mut blk = 1u64 << (63 - cur_size.leading_zeros() as u64);
                if blk > cur_size { blk = cur_size; }
                if lowbit < blk { blk = lowbit; }
                let base_msr = 0x200 + (idx as u32) * 2;
                let mask_msr = base_msr + 1;
                let mtrr_type: u64 = if priority == 1 { 0x01 } else { 0x00 }; // WC if priority==1 (wc list), UC otherwise
                let physbase = (cur_base & !0xFFF) | mtrr_type;
                let physmask = (! (blk - 1)) & !0xFFF;
                let physmask_lo = (physmask as u32) | (1 << 11);
                let physmask_hi = (physmask >> 32) as u32;
                let physbase_lo = physbase as u32;
                let physbase_hi = (physbase >> 32) as u32;
                core::arch::asm!("wrmsr", in("ecx") base_msr, in("eax") physbase_lo, in("edx") physbase_hi);
                core::arch::asm!("wrmsr", in("ecx") mask_msr, in("eax") physmask_lo, in("edx") physmask_hi);
                cur_base += blk;
                cur_size -= blk;
                idx += 1;
                covered += blk;
            }
            if idx >= var_cnt { break; }
        }
        let left = total.saturating_sub(covered);
        let (prev_used, prev_total, prev_cov, prev_left) = crate::mm::mmio_last_usage();
        crate::mm::mmio_record_mtrr_usage(idx.min(var_cnt), var_cnt, covered, left);
        crate::println!("[mtrr] refresh covered={} bytes, left={} bytes (prev: used={}/{} covered={} left={})", covered, left, prev_used, prev_total, prev_cov, prev_left);
        let now = crate::time::get_ticks();
        crate::mm::mmio_cooldown_all(now);
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

    // Probe platform for memory/MMIO via DTB/firmware (stub)
    platform::probe_dtb();
    crate::mm::mmio_stats_init();
    
    #[cfg(target_arch = "aarch64")]
    unsafe { aarch64::setup_mair(); aarch64::setup_tcr(); }

    // Map kernel memory regions - create linear PhysMap for kernel access
    let pt = kernel_pagetable();
    let phys_start = 0usize;
    let phys_end = crate::mm::phys_end();
    let size = phys_end - phys_start;
    #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
    unsafe { let _ = map_pages(pt, phys_start, phys_start, size, flags::PTE_R | flags::PTE_W); }
    #[cfg(target_arch = "x86_64")]
    unsafe { let _ = map_pages(pt, KERNEL_BASE + phys_start, phys_start, size, flags::PTE_R | flags::PTE_W); x86_64::setup_pat(); }

    // Map MMIO device regions (kernel only, no U, no X)
    for (base, size) in crate::mm::mmio_regions() {
        #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
        unsafe { let _ = map_pages(pt, base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV); }
        #[cfg(target_arch = "x86_64")]
        unsafe { let _ = map_pages(pt, KERNEL_BASE + base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV); }
    }
    for (base, size) in crate::mm::mmio_regions_strong() {
        #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
        unsafe { let _ = map_pages(pt, base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV | flags::PTE_DEV_STRONG); }
        #[cfg(target_arch = "x86_64")]
        unsafe { let _ = map_pages(pt, KERNEL_BASE + base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV); }
    }
    #[cfg(target_arch = "x86_64")]
    for (base, size) in crate::mm::mmio_regions_wc() {
        unsafe { let _ = map_pages(pt, KERNEL_BASE + base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV_WC); }
    }
    #[cfg(target_arch = "x86_64")]
    unsafe { x86_64::setup_mtrr_mmio_uc(); }
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
    unsafe { riscv64::activate_pt(pagetable); }
    
    #[cfg(target_arch = "aarch64")]
    unsafe { aarch64::activate_pt(pagetable); }
    
    #[cfg(target_arch = "x86_64")]
    x86_64::activate_pt(pagetable);
}

#[cfg(target_arch = "x86_64")]
pub fn refresh_mtrr_hot() {
    unsafe { x86_64::refresh_mtrr_from_stats(); }
}

pub fn idle_maintenance() {
    let t = crate::time::get_ticks();
    crate::mm::mmio_stats_periodic(t);
}

/// Copy data from kernel to user space
pub unsafe fn copyout(
    pagetable: *mut PageTable,
    dst: usize,
    src: *const u8,
    len: usize,
) -> Result<(), ()> {
    if dst == 0 || src.is_null() || len == 0 {
        return Err(());
    }
    // Validate user mapping and permissions
    if dst >= KERNEL_BASE { return Err(()); }
    user_range_check(pagetable, dst, len, true, false)?;
    let mut copied = 0usize;
    while copied < len {
        let va = dst + copied;
        let page_off = va & (PAGE_SIZE - 1);
        let chunk = core::cmp::min(len - copied, PAGE_SIZE - page_off);
        #[cfg(target_arch = "riscv64")]
        let pa = match riscv64::translate(pagetable, va) { Some(p) => p, None => return Err(()) };
        #[cfg(target_arch = "aarch64")]
        let pa = match aarch64::walk(pagetable, va, false) { Some(p) => (*p & !0xFFF) | page_off, None => return Err(()) };
        #[cfg(target_arch = "x86_64")]
        let pa = match x86_64::walk(pagetable, va, false) { Some(p) => (*p & !0xFFF) | page_off, None => return Err(()) };
        let dst_ptr = phys_to_kernel_ptr(pa);
        let src_ptr = unsafe { src.add(copied) };
        ptr::copy_nonoverlapping(src_ptr, dst_ptr, chunk);
        copied += chunk;
    }
    Ok(())
}

/// Copy data from user to kernel space
pub unsafe fn copyin(
    pagetable: *mut PageTable,
    dst: *mut u8,
    src: usize,
    len: usize,
) -> Result<(), ()> {
    if dst.is_null() || src == 0 || len == 0 {
        return Err(());
    }
    if src >= KERNEL_BASE { return Err(()); }
    user_range_check(pagetable, src, len, false, false)?;
    let mut copied = 0usize;
    while copied < len {
        let va = src + copied;
        let page_off = va & (PAGE_SIZE - 1);
        let chunk = core::cmp::min(len - copied, PAGE_SIZE - page_off);
        #[cfg(target_arch = "riscv64")]
        let pa = match riscv64::translate(pagetable, va) { Some(p) => p, None => return Err(()) };
        #[cfg(target_arch = "aarch64")]
        let pa = match aarch64::walk(pagetable, va, false) { Some(p) => (*p & !0xFFF) | page_off, None => return Err(()) };
        #[cfg(target_arch = "x86_64")]
        let pa = match x86_64::walk(pagetable, va, false) { Some(p) => (*p & !0xFFF) | page_off, None => return Err(()) };
        let src_ptr = phys_to_kernel_const_ptr(pa);
        let dst_ptr = unsafe { dst.add(copied) };
        ptr::copy_nonoverlapping(src_ptr, dst_ptr, chunk);
        copied += chunk;
    }
    Ok(())
}

pub unsafe fn copyinstr(
    pagetable: *mut PageTable,
    src: usize,
    dst: *mut u8,
    max: usize,
) -> Result<usize, ()> {
    if dst.is_null() || src == 0 || max == 0 { return Err(()); }
    if src >= KERNEL_BASE { return Err(()); }
    let mut copied = 0usize;
    loop {
        if copied >= max { return Err(()); }
        let va = src + copied;
        let page_off = va & (PAGE_SIZE - 1);
        let chunk = core::cmp::min(max - copied, PAGE_SIZE - page_off);
        #[cfg(target_arch = "riscv64")]
        let pa = match riscv64::translate(pagetable, va) { Some(p) => p, None => return Err(()) };
        #[cfg(target_arch = "aarch64")]
        let pa = match aarch64::walk(pagetable, va, false) { Some(p) => (*p & !0xFFF) | page_off, None => return Err(()) };
        #[cfg(target_arch = "x86_64")]
        let pa = match x86_64::walk(pagetable, va, false) { Some(p) => (*p & !0xFFF) | page_off, None => return Err(()) };
        let src_ptr = phys_to_kernel_const_ptr(pa);
        let mut i = 0usize;
        while i < chunk {
            let b = *src_ptr.add(i);
            *dst.add(copied + i) = b;
            if b == 0 { return Ok(copied + i); }
            i += 1;
        }
        copied += chunk;
    }
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
/// Check user-space mapping and permissions for a range [va, va+len)
fn user_range_check(pagetable: *mut PageTable, va: usize, len: usize, need_write: bool, need_exec: bool) -> Result<(), ()> {
    if len == 0 { return Err(()); }
    let mut cur = va & !(PAGE_SIZE - 1);
    let end = (va + len + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    while cur < end {
        #[cfg(target_arch = "riscv64")]
        unsafe {
            let pte = match riscv64::walk(pagetable, cur, false) { Some(p) => *p, None => return Err(()) };
            let valid = (pte & flags::PTE_V) != 0 && (pte & flags::PTE_U) != 0;
            let w_ok = !need_write || (pte & flags::PTE_W) != 0;
            let x_ok = !need_exec || (pte & flags::PTE_X) != 0;
            if !(valid && w_ok && x_ok) { return Err(()); }
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            const DESC_VALID: usize = 1 << 0;
            const DESC_AP_RO: usize = 2 << 6;
            const DESC_AP_USER: usize = 1 << 6;
            const DESC_UXN: usize = 1 << 54;
            let pte = match aarch64::walk(pagetable, cur, false) { Some(p) => *p, None => return Err(()) };
            let valid = (pte & DESC_VALID) != 0 && (pte & DESC_AP_USER) != 0;
            let w_ok = !need_write || (pte & DESC_AP_RO) == 0;
            let x_ok = !need_exec || (pte & DESC_UXN) == 0;
            if !(valid && w_ok && x_ok) { return Err(()) }
        }
        #[cfg(target_arch = "x86_64")]
        unsafe {
            const PTE_P: usize = 1 << 0;    // Present
            const PTE_RW: usize = 1 << 1;   // Read/Write
            const PTE_US: usize = 1 << 2;   // User/Supervisor
            const PTE_NX: usize = 1 << 63;  // No Execute
            let pte = match x86_64::walk(pagetable, cur, false) { Some(p) => *p, None => return Err(()) };
            let valid = (pte & PTE_P) != 0 && (pte & PTE_US) != 0;
            let w_ok = !need_write || (pte & PTE_RW) != 0;
            let x_ok = !need_exec || (pte & PTE_NX) == 0;
            if !(valid && w_ok && x_ok) { return Err(()) }
        }
        cur += PAGE_SIZE;
    }
    Ok(())
}
#[inline]
fn phys_to_kernel_ptr(pa: usize) -> *mut u8 {
    #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
    { pa as *mut u8 }
    #[cfg(target_arch = "x86_64")]
    { (KERNEL_BASE + pa) as *mut u8 }
}

#[inline]
fn phys_to_kernel_const_ptr(pa: usize) -> *const u8 {
    #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
    { pa as *const u8 }
    #[cfg(target_arch = "x86_64")]
    { (KERNEL_BASE + pa) as *const u8 }
}
/// Convert physical address to kernel virtual address
pub fn phys_to_virt(pa: usize) -> usize {
    #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
    { pa }
    #[cfg(target_arch = "x86_64")]
    { KERNEL_BASE + pa }
}
