//! Virtual Memory Management for xv6-rust
//!
//! This module provides virtual memory support including:
//! - Page table management for RISC-V Sv39, AArch64 and x86_64
//! - Kernel address space setup
//! - User address space management

extern crate alloc;

use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};
use core::ops::Range;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::subsystems::mm::{kalloc, kfree};

// Re-export PAGE_SIZE for other modules
pub use crate::subsystems::mm::PAGE_SIZE;
use crate::drivers::platform;
use crate::subsystems::sync::Mutex;

// ============================================================================
// VMA 区间管理（mmap 基础骨架）
// ============================================================================

/// 虚拟区间权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmPerm {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub user: bool,
}

impl VmPerm {
    pub const fn rw() -> Self {
        Self {
            read: true,
            write: true,
            exec: false,
            user: true,
        }
    }

    pub const fn r() -> Self {
        Self {
            read: true,
            write: false,
            exec: false,
            user: true,
        }
    }

    pub const fn rx() -> Self {
        Self {
            read: true,
            write: false,
            exec: true,
            user: true,
        }
    }

    pub fn to_pte_flags(&self) -> usize {
        let mut flags = flags::PTE_V;
        if self.read {
            flags |= flags::PTE_R;
        }
        if self.write {
            flags |= flags::PTE_W;
        }
        if self.exec {
            flags |= flags::PTE_X;
        }
        if self.user {
            flags |= flags::PTE_U;
        }
        flags
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    InvalidRange,
    Overlap,
    NotFound,
    NotPinned,
    NoMemory,
    MapFailed,
}

#[derive(Debug, Clone)]
pub struct VmArea {
    pub range: Range<usize>,
    pub perm: VmPerm,
    pub file_backed: bool,
    pub file_offset: usize,
    pub lazy: bool,
    pub cow: bool,
}

impl VmArea {
    pub fn len(&self) -> usize {
        self.range.end.saturating_sub(self.range.start)
    }
}

/// 简单 VMA 树，后续可替换为平衡树/区间树。
#[derive(Default, Debug)]
pub struct VmSpace {
    areas: BTreeMap<usize, VmArea>,
}

impl VmSpace {
    fn overlap(range: &Range<usize>, other: &Range<usize>) -> bool {
        range.start < other.end && other.start < range.end
    }

    pub fn map(
        &mut self,
        start: usize,
        length: usize,
        perm: VmPerm,
        file_backed: bool,
        file_offset: usize,
    ) -> Result<usize, VmError> {
        if length == 0 {
            return Err(VmError::InvalidRange);
        }
        let start_aligned = start & !(PAGE_SIZE - 1);
        let end_aligned = (start_aligned + length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        if end_aligned <= start_aligned {
            return Err(VmError::InvalidRange);
        }

        let new_range = start_aligned..end_aligned;
        for area in self.areas.values() {
            if Self::overlap(&new_range, &area.range) {
                return Err(VmError::Overlap);
            }
        }

        self.areas.insert(
            start_aligned,
            VmArea {
                range: new_range.clone(),
                perm,
                file_backed,
                file_offset,
                lazy: false,
                cow: false,
            },
        );
        Ok(start_aligned)
    }

    pub fn mmap_anonymous(&mut self, hint: usize, length: usize, perm: VmPerm) -> Result<usize, VmError> {
        let start = if hint == 0 {
            self.find_free_area(length).ok_or(VmError::InvalidRange)?
        } else {
            hint
        };
        self.map(start, length, perm, false, 0)
    }

    pub fn unmap(&mut self, start: usize, length: usize) -> Result<(), VmError> {
        let start_aligned = start & !(PAGE_SIZE - 1);
        let end_aligned = (start_aligned + length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let target = start_aligned..end_aligned;

        let key = self
            .areas
            .iter()
            .find(|(_, area)| area.range == target)
            .map(|(k, _)| *k)
            .ok_or(VmError::NotFound)?;

        self.areas.remove(&key);
        Ok(())
    }

    pub fn find_free_area(&self, length: usize) -> Option<usize> {
        let length_aligned = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let mut cursor = PAGE_SIZE; // 保留低地址
        for area in self.areas.values() {
            if cursor + length_aligned <= area.range.start {
                return Some(cursor);
            }
            cursor = area.range.end;
        }
        Some(cursor)
    }

    pub fn iter(&self) -> impl Iterator<Item = &VmArea> {
        self.areas.values()
    }

    /// 针对懒分配：调用者提供映射回调
    pub fn fault_in<F>(
        &mut self,
        va: usize,
        map_fn: F,
    ) -> Result<(), VmError>
    where
        F: FnOnce(&VmArea) -> Result<(), VmError>,
    {
        let page_base = va & !(PAGE_SIZE - 1);
        let area = self
            .areas
            .iter_mut()
            .find(|(_, a)| a.range.start <= page_base && page_base < a.range.end)
            .ok_or(VmError::NotFound)?
            .1;
        if !area.lazy {
            return Err(VmError::MapFailed);
        }
        map_fn(area)?;
        area.lazy = false;
        Ok(())
    }

    /// 针对 COW：调用者提供复制+映射回调
    pub fn handle_cow<F>(
        &mut self,
        va: usize,
        copy_map_fn: F,
    ) -> Result<(), VmError>
    where
        F: FnOnce(&VmArea) -> Result<(), VmError>,
    {
        let page_base = va & !(PAGE_SIZE - 1);
        let area = self
            .areas
            .iter_mut()
            .find(|(_, a)| a.range.start <= page_base && page_base < a.range.end)
            .ok_or(VmError::NotFound)?
            .1;
        if !area.cow {
            return Err(VmError::MapFailed);
        }
        copy_map_fn(area)?;
        area.cow = false;
        Ok(())
    }

    /// 将区间标记为懒分配
    pub fn mark_lazy(&mut self, start: usize, length: usize) -> Result<(), VmError> {
        let start_aligned = start & !(PAGE_SIZE - 1);
        let end_aligned = (start_aligned + length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let target = start_aligned..end_aligned;
        let (_, area) = self
            .areas
            .iter_mut()
            .find(|(_, area)| area.range == target)
            .ok_or(VmError::NotFound)?;
        area.lazy = true;
        Ok(())
    }

    /// 标记区间为 COW
    pub fn mark_cow(&mut self, start: usize, length: usize) -> Result<(), VmError> {
        let start_aligned = start & !(PAGE_SIZE - 1);
        let end_aligned = (start_aligned + length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let target = start_aligned..end_aligned;
        let (_, area) = self
            .areas
            .iter_mut()
            .find(|(_, area)| area.range == target)
            .ok_or(VmError::NotFound)?;
        area.cow = true;
        Ok(())
    }
}

// ============================================================================
// Page Reference Counting for COW
// ============================================================================

/// Maximum number of pages to track (configurable based on memory size)
const MAX_TRACKED_PAGES: usize = 65536;

/// Page reference counts (indexed by page frame number)
static PAGE_REFCOUNTS: Mutex<BTreeMap<usize, u32>> = Mutex::new(BTreeMap::new());

/// Increment reference count for a page
pub fn page_ref_inc(pa: usize) {
    let pfn = pa / PAGE_SIZE;
    let mut refcounts = PAGE_REFCOUNTS.lock();
    let count = refcounts.entry(pfn).or_insert(0);
    *count = count.saturating_add(1);
}

/// Decrement reference count for a page and free if zero
/// Returns true if the page was freed
pub fn page_ref_dec(pa: usize) -> bool {
    let pfn = pa / PAGE_SIZE;
    let mut refcounts = PAGE_REFCOUNTS.lock();
    
    if let Some(count) = refcounts.get_mut(&pfn) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            refcounts.remove(&pfn);
            // Free the page
            unsafe { kfree(pa as *mut u8); }
            return true;
        }
    }
    false
}

/// Get reference count for a page
pub fn page_ref_count(pa: usize) -> u32 {
    let pfn = pa / PAGE_SIZE;
    let refcounts = PAGE_REFCOUNTS.lock();
    *refcounts.get(&pfn).unwrap_or(&1)
}

// ============================================================================
// 用户页 pin/unpin（零拷贝基础）
// ============================================================================

/// 固定用户页，返回物理页帧列表（占位实现）
pub fn pin_user_pages(_pagetable: *mut PageTable, addrs: &[usize]) -> Result<Vec<usize>, VmError> {
    // TODO: walk pagetable, increase refcount and mark pinned
    let mut frames = Vec::with_capacity(addrs.len());
    for &va in addrs {
        // 对齐到页
        let pa = va & !(PAGE_SIZE - 1);
        frames.push(pa);
        page_ref_inc(pa);
    }
    Ok(frames)
}

/// 解除固定用户页（占位实现）
pub fn unpin_user_pages(frames: &[usize]) -> Result<(), VmError> {
    for &pa in frames {
        if !page_ref_dec(pa) {
            // 未真正减少到零也视为成功
        }
    }
    Ok(())
}

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
    
    /// Map a huge page (2MB or 1GB) for RISC-V Sv39
    pub unsafe fn map_huge_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        size: usize,
        perm: usize,
    ) -> Result<(), ()> {
        use crate::subsystems::mm::hugepage::{HPAGE_2MB, HPAGE_1GB};
        
        if size == HPAGE_2MB {
            // 2MB huge page: use level 1 PTE (superpage)
            let mut pt = pagetable;
            
            // Walk to level 2
            let vpn2 = (va >> (PAGE_OFFSET_BITS + VPN_BITS)) & 0x1FF;
            let pte2 = &mut (*pt).entries[vpn2];
            
            if *pte2 & PTE_V == 0 {
                // Allocate level 2 page table if needed
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return Err(());
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                *pte2 = pa_to_pte(new_pt as usize) | PTE_V;
                pt = new_pt as *mut PageTable;
            } else {
                pt = pte_to_pa(*pte2) as *mut PageTable;
            }
            
            // Set level 1 PTE for 2MB page
            let vpn1 = (va >> PAGE_OFFSET_BITS) & 0x1FF;
            let pte1 = &mut (*pt).entries[vpn1];
            
            if *pte1 & PTE_V != 0 {
                return Err(()); // Already mapped
            }
            
            let mut flags = perm | PTE_V;
            if perm & PTE_DEV != 0 {
                const PTE_PBMT_IO: usize = 1 << 62;
                flags |= PTE_PBMT_IO;
            }
            
            *pte1 = pa_to_pte(pa) | flags;
            Ok(())
        } else if size == HPAGE_1GB {
            // 1GB huge page: use level 2 PTE directly (megapage)
            let vpn2 = (va >> (PAGE_OFFSET_BITS + VPN_BITS)) & 0x1FF;
            let pte2 = &mut (*pagetable).entries[vpn2];
            
            if *pte2 & PTE_V != 0 {
                return Err(()); // Already mapped
            }
            
            let mut flags = perm | PTE_V;
            if perm & PTE_DEV != 0 {
                const PTE_PBMT_IO: usize = 1 << 62;
                flags |= PTE_PBMT_IO;
            }
            
            *pte2 = pa_to_pte(pa) | flags;
            Ok(())
        } else {
            Err(()) // Unsupported huge page size
        }
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
    
    /// Copy a RISC-V page table level recursively
    pub unsafe fn copy_pagetable_level(old_pt: *mut PageTable, new_pt: *mut PageTable) {
        use super::flags::*;
        
        for i in 0..super::PTE_COUNT {
            let old_pte = (*old_pt).entries[i];
            
            if old_pte & PTE_V == 0 {
                continue; // Skip invalid entries
            }
            
            // Check if this is a leaf PTE (points to a page)
            let leaf = (old_pte & (PTE_R | PTE_W | PTE_X)) != 0;
            
            if leaf {
                // This is a leaf PTE (points to a page)
                // Make a copy of the PTE but with COW flag and without write permission
                let mut new_pte = old_pte;
                
                // Remove write permission and add COW flag if the page is writable
                if (new_pte & PTE_W) != 0 || (new_pte & PTE_COW) != 0 {
                    new_pte &= !PTE_W; // Remove write permission
                    new_pte |= PTE_COW; // Set copy-on-write flag
                }
                
                // Copy the modified PTE to the new page table
                (*new_pt).entries[i] = new_pte;
            } else {
                // This is an intermediate PTE (points to another page table)
                // Allocate a new page table for the new level
                let new_subpt = super::kalloc();
                if new_subpt.is_null() {
                    panic!("copy_pagetable_level: failed to allocate page table");
                }
                core::ptr::write_bytes(new_subpt, 0, super::PAGE_SIZE);
                
                // Copy the intermediate PTE to the new page table
                let pa = pte_to_pa(old_pte);
                
                (*new_pt).entries[i] = pa_to_pte(new_subpt as usize) | (old_pte & 0x3FF);
                
                // Recursively copy the next level
                copy_pagetable_level(
                    pa as *mut PageTable,
                    new_subpt as *mut PageTable
                );
            }
        }
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
    
    /// Map a huge page (2MB or 1GB) for AArch64
    pub unsafe fn map_huge_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        size: usize,
        perm: usize,
    ) -> Result<(), ()> {
        use crate::subsystems::mm::hugepage::{HPAGE_2MB, HPAGE_1GB};
        
        if size == HPAGE_2MB {
            // 2MB huge page: use level 2 descriptor
            let mut pt = pagetable;
            
            // Walk to level 1
            let idx1 = va_index(va, 1);
            let pte1 = &mut (*pt).entries[idx1];
            
            if *pte1 & DESC_VALID == 0 {
                // Allocate level 1 page table if needed
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return Err(());
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                *pte1 = (new_pt as usize) | DESC_TABLE | DESC_VALID;
                pt = new_pt as *mut PageTable;
            } else {
                pt = ((*pte1) & !0xFFF) as *mut PageTable;
            }
            
            // Set level 2 descriptor for 2MB page
            let idx2 = va_index(va, 2);
            let pte2 = &mut (*pt).entries[idx2];
            
            if *pte2 & DESC_VALID != 0 {
                return Err(()); // Already mapped
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
            
            *pte2 = (pa & !0x1FFFFF) | flags; // 2MB alignment
            Ok(())
        } else if size == HPAGE_1GB {
            // 1GB huge page: use level 1 descriptor directly
            let idx1 = va_index(va, 1);
            let pte1 = &mut (*pagetable).entries[idx1];
            
            if *pte1 & DESC_VALID != 0 {
                return Err(()); // Already mapped
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
            
            *pte1 = (pa & !0x3FFFFFFF) | flags; // 1GB alignment
            Ok(())
        } else {
            Err(()) // Unsupported huge page size
        }
    }
    
    /// Unmap a page
    pub unsafe fn unmap_page(pagetable: *mut PageTable, va: usize) -> Result<(), ()> {
        let pte = walk(pagetable, va, false).ok_or(())?;

        if *pte & DESC_VALID == 0 {
            return Err(());
        }

        *pte = 0;
        flush_tlb_page(va);
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
    
    /// Map a huge page (2MB or 1GB) for AArch64
    pub unsafe fn map_huge_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        size: usize,
        perm: usize,
    ) -> Result<(), ()> {
        use crate::subsystems::mm::hugepage::{HPAGE_2MB, HPAGE_1GB};
        
        if size == HPAGE_2MB {
            // 2MB huge page: use level 2 descriptor
            let mut pt = pagetable;
            
            // Walk to level 1
            let idx1 = va_index(va, 1);
            let pte1 = &mut (*pt).entries[idx1];
            
            if *pte1 & DESC_VALID == 0 {
                // Allocate level 1 page table if needed
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return Err(());
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                *pte1 = (new_pt as usize) | DESC_TABLE | DESC_VALID;
                pt = new_pt as *mut PageTable;
            } else {
                pt = ((*pte1) & !0xFFF) as *mut PageTable;
            }
            
            // Set level 2 descriptor for 2MB page
            let idx2 = va_index(va, 2);
            let pte2 = &mut (*pt).entries[idx2];
            
            if *pte2 & DESC_VALID != 0 {
                return Err(()); // Already mapped
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
            
            *pte2 = (pa & !0x1FFFFF) | flags; // 2MB alignment
            Ok(())
        } else if size == HPAGE_1GB {
            // 1GB huge page: use level 1 descriptor directly
            let idx1 = va_index(va, 1);
            let pte1 = &mut (*pagetable).entries[idx1];
            
            if *pte1 & DESC_VALID != 0 {
                return Err(()); // Already mapped
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
            
            *pte1 = (pa & !0x3FFFFFFF) | flags; // 1GB alignment
            Ok(())
        } else {
            Err(()) // Unsupported huge page size
        }
    }
    
    pub unsafe fn activate_pt(pagetable: *mut PageTable) {
        let ttbr0 = pagetable as u64;
        core::arch::asm!("msr ttbr0_el1, {}", in(reg) ttbr0);
        core::arch::asm!("isb");
        core::arch::asm!("tlbi vmalle1");
        core::arch::asm!("dsb nsh");
        core::arch::asm!("isb");
    }
    
    /// Copy an AArch64 page table level recursively
    pub unsafe fn copy_pagetable_level(old_pt: *mut PageTable, new_pt: *mut PageTable) {
        use super::flags::*;
        
        for i in 0..super::PTE_COUNT {
            let old_pte = (*old_pt).entries[i];
            
            // Skip invalid entries
            if old_pte & DESC_TABLE == 0 && old_pte & PTE_V == 0 {
                continue;
            }
            
            // Check if this is a leaf PTE (doesn't have table flag set)
            let leaf = (old_pte & DESC_TABLE) == 0;
            
            if leaf {
                // This is a leaf PTE (points to a page)
                // Make a copy of the PTE but with COW flag and without write permission
                let mut new_pte = old_pte;
                
                // For AArch64, need to adjust permissions
                let mut perm = new_pte & 0x3FF;
                
                // Remove write permission if needed
                if (perm & (3 << 6)) == 0 { // write permission set
                    perm |= 2 << 6; // set to read-only
                    new_pte = (new_pte & !0x3FF) | perm | PTE_COW;
                }
                
                // Copy the modified PTE to the new page table
                (*new_pt).entries[i] = new_pte;
            } else {
                // This is an intermediate PTE (points to another page table)
                // Allocate a new page table for the new level
                let new_subpt = super::kalloc();
                if new_subpt.is_null() {
                    panic!("copy_pagetable_level: failed to allocate page table");
                }
                core::ptr::write_bytes(new_subpt, 0, super::PAGE_SIZE);
                
                // Copy the intermediate PTE to the new page table
                let pa = (old_pte & !0xFFF);
                
                (*new_pt).entries[i] = (new_subpt as usize) | DESC_TABLE | (old_pte & 0xFFF);
                
                // Recursively copy the next level
                copy_pagetable_level(
                    pa as *mut PageTable,
                    new_subpt as *mut PageTable
                );
            }
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
    
    /// Map a huge page (2MB or 1GB) for x86_64
    pub unsafe fn map_huge_page(
        pagetable: *mut PageTable,
        va: usize,
        pa: usize,
        size: usize,
        perm: usize,
    ) -> Result<(), ()> {
        use crate::subsystems::mm::hugepage::{HPAGE_2MB, HPAGE_1GB};
        
        if size == HPAGE_2MB {
            // 2MB huge page: use level 1 PTE (PDE with PS bit set)
            let mut pt = pagetable;
            
            // Walk to level 2
            let idx2 = va_index(va, 2);
            let pte2 = &mut (*pt).entries[idx2];
            
            if *pte2 & PTE_P == 0 {
                // Allocate level 2 page table if needed
                let new_pt = kalloc();
                if new_pt.is_null() {
                    return Err(());
                }
                ptr::write_bytes(new_pt, 0, PAGE_SIZE);
                *pte2 = (new_pt as usize) | PTE_P | PTE_RW | PTE_US;
                pt = new_pt as *mut PageTable;
            } else {
                pt = ((*pte2) & !0xFFF) as *mut PageTable;
            }
            
            // Set level 1 PTE for 2MB page (with PS bit = 1)
            let idx1 = va_index(va, 1);
            let pte1 = &mut (*pt).entries[idx1];
            
            if *pte1 & PTE_P != 0 {
                return Err(()); // Already mapped
            }
            
            let mut flags = PTE_P | (1 << 7); // PS bit for 2MB page
            if perm & PTE_W != 0 {
                flags |= PTE_RW;
            }
            if perm & PTE_U != 0 {
                flags |= PTE_US;
            }
            if perm & PTE_X == 0 {
                flags |= PTE_NX;
            }
            
            *pte1 = (pa & !0x1FFFFF) | flags; // 2MB alignment
            Ok(())
        } else if size == HPAGE_1GB {
            // 1GB huge page: use level 2 PTE (PDPTE with PS bit set)
            let idx2 = va_index(va, 2);
            let pte2 = &mut (*pagetable).entries[idx2];
            
            if *pte2 & PTE_P != 0 {
                return Err(()); // Already mapped
            }
            
            let mut flags = PTE_P | (1 << 7); // PS bit for 1GB page
            if perm & PTE_W != 0 {
                flags |= PTE_RW;
            }
            if perm & PTE_U != 0 {
                flags |= PTE_US;
            }
            if perm & PTE_X == 0 {
                flags |= PTE_NX;
            }
            
            *pte2 = (pa & !0x3FFFFFFF) | flags; // 1GB alignment
            Ok(())
        } else {
            Err(()) // Unsupported huge page size
        }
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
        for (b, s) in crate::subsystems::mm::mmio_regions_strong() { regions.push((b, s, 2)); }
        for (b, s) in crate::subsystems::mm::mmio_regions_wc() { regions.push((b, s, 1)); }
        for (b, s) in crate::subsystems::mm::mmio_regions() { regions.push((b, s, 0)); }
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
        crate::subsystems::mm::mmio_record_mtrr_usage(idx.min(var_cnt), var_cnt, covered, left);
        let (prev_used, prev_total, prev_cov, prev_left) = crate::subsystems::mm::mmio_last_usage();
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
        let mut stats = crate::subsystems::mm::mmio_stats_take();
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
        let (prev_used, prev_total, prev_cov, prev_left) = crate::subsystems::mm::mmio_last_usage();
        crate::subsystems::mm::mmio_record_mtrr_usage(idx.min(var_cnt), var_cnt, covered, left);
        crate::println!("[mtrr] refresh covered={} bytes, left={} bytes (prev: used={}/{} covered={} left={})", covered, left, prev_used, prev_total, prev_cov, prev_left);
        let now = crate::subsystems::time::get_ticks();
        crate::subsystems::mm::mmio_cooldown_all(now);
    }
    
    /// Copy an x86_64 page table level recursively
    pub unsafe fn copy_pagetable_level(old_pt: *mut PageTable, new_pt: *mut PageTable) {
        use super::flags::*;
        
        for i in 0..super::PTE_COUNT {
            let old_pte = (*old_pt).entries[i];
            
            if old_pte & PTE_P == 0 {
                continue; // Skip invalid entries
            }
            
            // Check if this is a leaf PTE (PS flag set or last level)
            let leaf = (old_pte & (1 << 7)) == 0;
            
            if leaf {
                // This is a leaf PTE (points to a page)
                // Make a copy of the PTE but with COW flag and without write permission
                let mut new_pte = old_pte;
                
                // Remove write permission and add COW flag if the page is writable
                if (new_pte & (1 << 1)) != 0 { // write permission set
                    new_pte &= !(1 << 1); // remove write permission
                    new_pte |= PTE_COW; // set copy-on-write flag
                }
                
                // Copy the modified PTE to the new page table
                (*new_pt).entries[i] = new_pte;
            } else {
                // This is an intermediate PTE (points to another page table)
                // Allocate a new page table for the new level
                let new_subpt = kalloc();
                if new_subpt.is_null() {
                    panic!("copy_pagetable_level: failed to allocate page table");
                }
                core::ptr::write_bytes(new_subpt, 0, PAGE_SIZE);
                
                // Copy the intermediate PTE to the new page table
                let pa = (old_pte & !0xFFF);
                
                (*new_pt).entries[i] = (new_subpt as usize) | (old_pte & 0xFFF);
                
                // Recursively copy the next level
                copy_pagetable_level(
                    pa as *mut PageTable,
                    new_subpt as *mut PageTable
                );
            }
        }
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
    crate::subsystems::mm::mmio_stats_init();
    
    #[cfg(target_arch = "aarch64")]
    unsafe { aarch64::setup_mair(); aarch64::setup_tcr(); }

    // Get ASLR offset from boot parameters
    let aslr_offset = crate::platform::boot::get_aslr_offset();
    let aslr_enabled = crate::platform::boot::is_aslr_enabled();
    
    if aslr_enabled {
        crate::println!("[vm] ASLR enabled with offset: {:#x}", aslr_offset);
    }

    // Map kernel memory regions - create linear PhysMap for kernel access
    // Use separate physical mapping region to avoid conflicts with kernel code
    let pt = kernel_pagetable();
    let phys_start = 0usize;
    let phys_end = crate::subsystems::mm::phys_end();
    let size = phys_end - phys_start;
    
    // Get physical mapping base from memory layout (separate from kernel code)
    // Apply ASLR offset if enabled
    let base_phys_map = crate::arch::memory_layout::MemoryLayout::current()
        .phys_map_base
        .expect("Physical memory mapping not supported on this architecture");
    
    let phys_map_base = if aslr_enabled {
        // Use layout module's apply_aslr_offset function
        use nos_memory_management::layout::AddressSpaceLayout;
        AddressSpaceLayout::current().apply_aslr_offset(base_phys_map, aslr_offset)
    } else {
        base_phys_map
    };
    
    // Verify memory layout doesn't have conflicts
    let layout = crate::arch::memory_layout::MemoryLayout::current();
    if let Err(e) = layout.verify_memory_layout() {
        panic!("vm::init: Memory layout verification failed: {}", e);
    }
    
    // Verify ASLR offset doesn't cause conflicts
    if aslr_enabled {
        // Check that ASLR-adjusted addresses don't overlap with kernel regions
        let layout = crate::arch::memory_layout::MemoryLayout::current();
        if phys_map_base >= layout.kernel_code_base && 
           phys_map_base < layout.kernel_code_base + 0x1000_0000 {
            panic!("vm::init: ASLR-adjusted phys_map_base overlaps with kernel code region");
        }
    }
    
    #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
    unsafe { 
        // For RISC-V and AArch64, map at phys_map_base
        let _ = map_pages(pt, phys_map_base + phys_start, phys_start, size, flags::PTE_R | flags::PTE_W); 
    }
    #[cfg(target_arch = "x86_64")]
    unsafe { 
        // For x86_64, use phys_map_base instead of KERNEL_BASE to avoid conflicts
        let _ = map_pages(pt, phys_map_base + phys_start, phys_start, size, flags::PTE_R | flags::PTE_W); 
        x86_64::setup_pat(); 
    }

    // Map MMIO device regions (kernel only, no U, no X)
    for (base, size) in crate::subsystems::mm::mmio_regions() {
        #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
        unsafe { let _ = map_pages(pt, base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV); }
        #[cfg(target_arch = "x86_64")]
        unsafe { let _ = map_pages(pt, KERNEL_BASE + base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV); }
    }
    for (base, size) in crate::subsystems::mm::mmio_regions_strong() {
        #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
        unsafe { let _ = map_pages(pt, base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV | flags::PTE_DEV_STRONG); }
        #[cfg(target_arch = "x86_64")]
        unsafe { let _ = map_pages(pt, KERNEL_BASE + base, base, size, flags::PTE_R | flags::PTE_W | flags::PTE_DEV); }
    }
    #[cfg(target_arch = "x86_64")]
    for (base, size) in crate::subsystems::mm::mmio_regions_wc() {
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
    
    // Recursively free page table pages
    let pt = &*pagetable;
    
    // Iterate through top-level page table entries
    for i in 0..512 {
        let pte = pt.entries[i];
        if pte != 0 && (pte & flags::PTE_V) != 0 {
            // Check if this is a leaf page or points to another page table
            // On RISC-V, if it's not a leaf (R/W/X bits are not set), it's a page table
            #[cfg(target_arch = "riscv64")]
            let is_page_table = (pte & (flags::PTE_R | flags::PTE_W | flags::PTE_X)) == 0;
            
            #[cfg(target_arch = "aarch64")]
            let is_page_table = (pte & 0x3) == 0x3; // Table descriptor
            
            #[cfg(target_arch = "x86_64")]
            let is_page_table = (pte & (1 << 7)) == 0; // Not a huge page
            
            #[cfg(not(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64")))]
            let is_page_table = false;
            
            if is_page_table {
                // Extract physical address of next level page table
                let next_pt = ((pte >> 10) << 12) as *mut PageTable;
                if !next_pt.is_null() {
                    // Recursively free
                    free_pagetable(next_pt);
                }
            }
        }
    }
    
    // Free this page table page
    kfree(pagetable as *mut u8);
}

/// Map pages in a page table
pub unsafe fn map_pages(
    pagetable: *mut PageTable,
    va: usize,
    pa: usize,
    size: usize,
    perm: usize,
) -> Result<(), ()> {
    // Check if we can use huge pages for this mapping
    use crate::subsystems::mm::hugepage::{HPAGE_2MB, HPAGE_1GB};
    
    let mut va = va & !(PAGE_SIZE - 1);
    let mut pa = pa & !(PAGE_SIZE - 1);
    let end = (va + size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    
    // Try to use huge pages for large mappings
    while va < end {
        let remaining = end - va;
        
        // Try 1GB huge page first (if size and alignment allow)
        if remaining >= HPAGE_1GB 
            && (va & (HPAGE_1GB - 1)) == 0 
            && (pa & (HPAGE_1GB - 1)) == 0 {
            #[cfg(target_arch = "riscv64")]
            if riscv64::map_huge_page(pagetable, va, pa, HPAGE_1GB, perm).is_ok() {
                va += HPAGE_1GB;
                pa += HPAGE_1GB;
                continue;
            }
            
            #[cfg(target_arch = "aarch64")]
            if aarch64::map_huge_page(pagetable, va, pa, HPAGE_1GB, perm).is_ok() {
                va += HPAGE_1GB;
                pa += HPAGE_1GB;
                continue;
            }
            
            #[cfg(target_arch = "x86_64")]
            if x86_64::map_huge_page(pagetable, va, pa, HPAGE_1GB, perm).is_ok() {
                va += HPAGE_1GB;
                pa += HPAGE_1GB;
                continue;
            }
        }
        
        // Try 2MB huge page (if size and alignment allow)
        if remaining >= HPAGE_2MB 
            && (va & (HPAGE_2MB - 1)) == 0 
            && (pa & (HPAGE_2MB - 1)) == 0 {
            #[cfg(target_arch = "riscv64")]
            if riscv64::map_huge_page(pagetable, va, pa, HPAGE_2MB, perm).is_ok() {
                va += HPAGE_2MB;
                pa += HPAGE_2MB;
                continue;
            }
            
            #[cfg(target_arch = "aarch64")]
            if aarch64::map_huge_page(pagetable, va, pa, HPAGE_2MB, perm).is_ok() {
                va += HPAGE_2MB;
                pa += HPAGE_2MB;
                continue;
            }
            
            #[cfg(target_arch = "x86_64")]
            if x86_64::map_huge_page(pagetable, va, pa, HPAGE_2MB, perm).is_ok() {
                va += HPAGE_2MB;
                pa += HPAGE_2MB;
                continue;
            }
        }
        
        // Fall back to regular 4KB pages
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
    let t = crate::subsystems::time::get_ticks();
    crate::subsystems::mm::mmio_stats_periodic(t);
}

/// Map a page (module-level wrapper)
pub unsafe fn map_page(
    pagetable: *mut PageTable,
    va: usize,
    pa: usize,
    perm: usize,
) -> Result<(), ()> {
    #[cfg(target_arch = "riscv64")]
    return riscv64::map_page(pagetable, va, pa, perm);
    
    #[cfg(target_arch = "aarch64")]
    return aarch64::map_page(pagetable, va, pa, perm);
    
    #[cfg(target_arch = "x86_64")]
    return x86_64::map_page(pagetable, va, pa, perm);
    
    #[cfg(not(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64")))]
    Err(())
}

/// Copy an entire page table with copy-on-write semantics
pub unsafe fn copy_pagetable(pagetable: *mut PageTable) -> Option<*mut PageTable> {
    // Create a new page table for the child
    let new_pagetable = create_pagetable()?;
    
    // Recursively copy all page table entries
    #[cfg(target_arch = "riscv64")]
    {
        // RISC-V Sv39 uses 3 levels (1,2,3)
        unsafe { riscv64::copy_pagetable_level(pagetable, new_pagetable); }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // AArch64 uses 4 levels (0,1,2,3)
        unsafe { aarch64::copy_pagetable_level(pagetable, new_pagetable); }
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // x86_64 uses 4 levels (1,2,3,4)
        unsafe { x86_64::copy_pagetable_level(pagetable, new_pagetable); }
    }
    
    Some(new_pagetable)
}

/// Helper function to check if a PTE is a leaf node (points to a page)
#[inline(always)]
pub unsafe fn is_leaf_pte(old_pte: usize) -> bool {
    #[cfg(target_arch = "riscv64")]
    {
        // RISC-V: leaf PTE has R/W/X flag set
        (old_pte & (flags::PTE_R | flags::PTE_W | flags::PTE_X)) != 0
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // AArch64: leaf PTE doesn't have table flag set
        const DESC_TABLE: usize = 1 << 1;
        (old_pte & DESC_TABLE) == 0
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // x86_64: leaf PTE doesn't have PS flag set for last level
        (old_pte & (1 << 7)) == 0
    }
}

/// Helper function to get physical address from PTE
#[inline(always)]
unsafe fn pte_to_pa(pte: usize) -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        riscv64::pte_to_pa(pte)
    }
    
    #[cfg(not(target_arch = "riscv64"))]
    {
        pte & !0xFFF
    }
}

/// Helper function to convert PA to PTE
#[inline(always)]
unsafe fn pa_to_pte(pa: usize) -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        riscv64::pa_to_pte(pa)
    }
    
    #[cfg(not(target_arch = "riscv64"))]
    {
        pa & !0xFFF
    }
}

/// Find a free virtual address range
pub fn find_free_range(_size: usize) -> Option<usize> {
    // Placeholder: In a real implementation, this would search the process's virtual address space
    // For now, return a fixed address range
    Some(0x40000000) // Typical user space start address
}

/// Get page mapping for a process
pub fn get_page_mapping(_process: *const crate::process::Proc, _vaddr: usize) -> Option<usize> {
    // Placeholder: In a real implementation, this would look up the page table entry
    None
}

/// Page structure (for compatibility)
#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub addr: usize,
    pub size: usize,
    pub flags: u8,
}

/// Allocate a page
pub fn alloc_page() -> Option<usize> {
    // Placeholder: In a real implementation, this would allocate a physical page
    Some(0)
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
    if is_kernel_address(dst) { return Err(()); }
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
    if is_kernel_address(src) { return Err(()); }
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

/// Optimized string copy from user space using bulk page copying
/// Uses cache-line aligned bulk copies for better performance
pub unsafe fn copyinstr(
    pagetable: *mut PageTable,
    src: usize,
    dst: *mut u8,
    max: usize,
) -> Result<usize, ()> {
    if dst.is_null() || src == 0 || max == 0 { return Err(()); }
    if is_kernel_address(src) { return Err(()); }
    
    // Cache line size (typically 64 bytes)
    const CACHE_LINE_SIZE: usize = 64;
    
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
        let dst_ptr = dst.add(copied);
        
        // Use bulk copy for aligned chunks when possible
        if chunk >= CACHE_LINE_SIZE && (copied % CACHE_LINE_SIZE == 0) {
            // Try to copy in cache-line aligned chunks
            let aligned_chunk = (chunk / CACHE_LINE_SIZE) * CACHE_LINE_SIZE;
            
            // Copy aligned portion using bulk copy
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, aligned_chunk);
            
            // Check for null terminator in aligned portion
            let mut found_null = false;
            let mut null_pos = 0;
            for i in 0..aligned_chunk {
                if *dst_ptr.add(i) == 0 {
                    found_null = true;
                    null_pos = i;
                    break;
                }
            }
            
            if found_null {
                return Ok(copied + null_pos);
            }
            
            copied += aligned_chunk;
            
            // Handle remaining unaligned bytes
            let remaining = chunk - aligned_chunk;
            if remaining > 0 {
                let remaining_src = src_ptr.add(aligned_chunk);
                let remaining_dst = dst_ptr.add(aligned_chunk);
                
                for i in 0..remaining {
                    let b = *remaining_src.add(i);
                    *remaining_dst.add(i) = b;
                    if b == 0 {
                        return Ok(copied + i);
                    }
                }
                copied += remaining;
            }
        } else {
            // For small chunks or unaligned data, use optimized byte-by-byte copy
            // but still check for null terminator efficiently
            let mut i = 0usize;
            while i < chunk {
                let b = *src_ptr.add(i);
                *dst_ptr.add(i) = b;
                if b == 0 {
                    return Ok(copied + i);
                }
                i += 1;
            }
            copied += chunk;
        }
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
    if is_user && is_kernel_address(va) {
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
    
    // Check reference count - if this is the only reference, just make it writable
    if page_ref_count(old_pa) == 1 {
        // Only one reference, make page writable without copying
        let new_pte = (old_pte & !flags::PTE_COW) | flags::PTE_W;
        unsafe { *pte_ptr = new_pte; }
        flush_tlb_page(va);
        return PageFaultResult::Handled;
    }
    
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
    
    // Decrement reference count on old page (may free it if count reaches 0)
    page_ref_dec(old_pa);
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

// Re-export memory layout constants from architecture abstraction layer
pub use crate::arch::memory_layout::{
    kernel_base as KERNEL_BASE,
    user_base as USER_BASE,
    user_stack_top as USER_STACK_TOP,
    user_max as USER_MAX,
    is_kernel_address,
    is_user_address,
};
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
            if !(valid && w_ok && x_ok) { return Err(()); }
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
            if !(valid && w_ok && x_ok) { return Err(()); }
        }
        cur += PAGE_SIZE;
    }
    Ok(())
}
#[inline]
pub fn phys_to_kernel_ptr(pa: usize) -> *mut u8 {
    crate::arch::memory_layout::phys_to_virt(pa)
        .map(|virt| virt as *mut u8)
        .unwrap_or_else(|| {
            // Fallback for architectures without direct mapping
            // Use phys_map_base instead of KERNEL_BASE to avoid conflicts
            let phys_map_base = crate::arch::memory_layout::MemoryLayout::current()
                .phys_map_base
                .unwrap_or(0);
            #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
            { (phys_map_base + pa) as *mut u8 }
            #[cfg(target_arch = "x86_64")]
            { (phys_map_base + pa) as *mut u8 }
            #[cfg(not(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64")))]
            { pa as *mut u8 }
        })
}

#[inline]
fn phys_to_kernel_const_ptr(pa: usize) -> *const u8 {
    crate::arch::memory_layout::phys_to_virt(pa)
        .map(|virt| virt as *const u8)
        .unwrap_or_else(|| {
            // Fallback for architectures without direct mapping
            // Use phys_map_base instead of KERNEL_BASE to avoid conflicts
            let phys_map_base = crate::arch::memory_layout::MemoryLayout::current()
                .phys_map_base
                .unwrap_or(0);
            #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
            { (phys_map_base + pa) as *const u8 }
            #[cfg(target_arch = "x86_64")]
            { (phys_map_base + pa) as *const u8 }
            #[cfg(not(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64")))]
            { pa as *const u8 }
        })
}
/// Convert physical address to kernel virtual address
pub fn phys_to_virt(pa: usize) -> usize {
    crate::arch::memory_layout::phys_to_virt(pa)
        .unwrap_or_else(|| {
            // Fallback for architectures without direct mapping
            // Use phys_map_base instead of KERNEL_BASE to avoid conflicts
            let phys_map_base = crate::arch::memory_layout::MemoryLayout::current()
                .phys_map_base
                .unwrap_or(0);
            #[cfg(any(target_arch = "riscv64", target_arch = "aarch64"))]
            { phys_map_base + pa }
            #[cfg(target_arch = "x86_64")]
            { phys_map_base + pa }
            #[cfg(not(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64")))]
            { pa }
        })
}

// Re-export architecture-specific functions
#[cfg(target_arch = "aarch64")]
pub use aarch64::{walk, unmap_page};

#[cfg(target_arch = "riscv64")]
pub use riscv64::walk;
#[cfg(target_arch = "x86_64")]
pub use x86_64::walk;

// Prefetch integration
pub fn vm_prefetch_page(addr: usize) {
    // Call the adaptive prefetch module
    crate::subsystems::mm::prefetch::process_memory_access(addr, crate::subsystems::mm::PAGE_SIZE, crate::subsystems::mm::prefetch::AccessType::Read);
}