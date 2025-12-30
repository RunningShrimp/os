//! mm模块虚拟内存管理公共接口
//!
//! 提供虚拟内存映射和管理功能

use alloc::{collections::BTreeMap, vec::Vec};
use core::ops::Range;

use super::{
    error::VmError,
    types::{MapFlags, MappingType, MemoryMapping, MemoryProtection},
};
use crate::subsystems::{
    mm::vm::{PAGE_SIZE, VmArea, VmPerm, flags, flush_tlb_page, map_pages},
    mm::page_table_isolation::PageTable,
    process::myproc,
    sync::Mutex,
};

/// Virtual memory area manager
///
/// Manages memory mappings for a process
pub struct VmManager {
    /// List of virtual memory areas
    pub vmas: Vec<VmArea>,
    /// Page table for this address space
    pub pagetable: *mut PageTable,
    /// Next available address for anonymous mappings
    pub next_anon_addr: usize,
}

impl VmManager {
    /// Create a new VM manager
    pub fn new(pagetable: *mut PageTable) -> Self {
        Self {
            vmas: Vec::new(),
            pagetable,
            next_anon_addr: 0x400000000, // Start of user address space
        }
    }

    /// Find a VMA that contains the given address
    pub fn find_vma(&self, addr: usize) -> Option<&VmArea> {
        self.vmas.iter().find(|vma| addr >= vma.start && addr < vma.end)
    }

    /// Find a VMA that contains the given address (mutable)
    pub fn find_vma_mut(&mut self, addr: usize) -> Option<&mut VmArea> {
        self.vmas.iter_mut().find(|vma| addr >= vma.start && addr < vma.end)
    }

    /// Find all VMAs that overlap with the given range
    pub fn find_overlapping_vmas(&self, range: &Range<usize>) -> Vec<usize> {
        self.vmas
            .iter()
            .enumerate()
            .filter(|(_, vma)| vma.start < range.end && vma.end > range.start)
            .map(|(i, _)| i)
            .collect()
    }

    /// Add a new VMA
    pub fn add_vma(&mut self, vma: VmArea) -> Result<(), VmError> {
        // Check for overlaps
        let range = Range { start: vma.start, end: vma.end };
        let overlapping = self.find_overlapping_vmas(&range);
        if !overlapping.is_empty() && vma.start < vma.end {
            return Err(VmError::AddressAlreadyMapped);
        }

        self.vmas.push(vma);
        Ok(())
    }

    /// Remove VMAs in the given range
    pub fn remove_vmas(&mut self, range: &Range<usize>) -> Vec<VmArea> {
        let mut removed = Vec::new();
        self.vmas.retain(|vma| {
            if vma.start < range.end && vma.end > range.start {
                removed.push(vma.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Find a free address range for anonymous mapping
    pub fn find_free_range(&mut self, size: usize) -> Result<usize, VmError> {
        // Align size to page boundary
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Start from next_anon_addr and find a gap
        let mut addr = self.next_anon_addr;
        let end_addr = 0x800000000; // End of user address space

        // Sort VMAs by start address for easier gap finding
        self.vmas.sort_by_key(|vma| vma.start);

        for vma in &self.vmas {
            if addr + aligned_size <= vma.start {
                // Found a gap
                self.next_anon_addr = addr + aligned_size;
                return Ok(addr);
            }

            // Move to after this VMA
            if addr < vma.end {
                addr = vma.end;
            }
        }

        // Check if we can fit after the last VMA
        if addr + aligned_size <= end_addr {
            self.next_anon_addr = addr + aligned_size;
            return Ok(addr);
        }

        Err(VmError::InvalidAddress)
    }
}

/// Global VM manager for each process
static mut VM_MANAGERS: BTreeMap<usize, Mutex<VmManager>> = BTreeMap::new();
static VM_MANAGERS_INIT: crate::subsystems::sync::Once = crate::subsystems::sync::Once::new();

/// Get VM manager for current process
fn get_current_vm_manager() -> Option<&'static Mutex<VmManager>> {
    unsafe {
        VM_MANAGERS_INIT.call_once(|| {
            // Initialize the map
        });

        if let Some(pid) = myproc() {
            VM_MANAGERS.get(&(pid as usize))
        } else {
            None
        }
    }
}

/// Initialize VM manager for a process
pub fn init_vm_manager(pid: usize, pagetable: *mut PageTable) {
    unsafe {
        VM_MANAGERS_INIT.call_once(|| {
            // Initialize the map
        });

        VM_MANAGERS.insert(pid, Mutex::new(VmManager::new(pagetable)));
    }
}

/// Clean up VM manager for a process
pub fn cleanup_vm_manager(pid: usize) {
    unsafe {
        VM_MANAGERS.remove(&pid);
    }
}

/// Convert MemoryProtection to VmPerm
fn memory_protection_to_vm_perm(prot: &MemoryProtection) -> VmPerm {
    VmPerm {
        read: prot.readable,
        write: prot.writable,
        execute: prot.executable,
    }
}

/// Convert VmPerm to PTE flags
fn vm_perm_to_pte_flags(perm: &VmPerm) -> usize {
    let mut flags = 0;
    if perm.read {
        flags |= flags::READ;
    }
    if perm.write {
        flags |= flags::WRITE;
    }
    if perm.execute {
        flags |= flags::EXEC;
    }
    flags | flags::USER // User mappings are always user accessible
}

/// Convert VmPerm to MemoryProtection
fn vm_perm_to_memory_protection(perm: &VmPerm) -> MemoryProtection {
    MemoryProtection {
        readable: perm.read,
        writable: perm.write,
        executable: perm.execute,
    }
}

/// Map memory region
///
/// # Contract
/// * Address must be page aligned
/// * Size must be page aligned
/// * Validate memory permissions
/// * Update process address space
/// * Handle address conflict
pub fn mmap(
    addr: usize,
    size: usize,
    prot: MemoryProtection,
    flags: MapFlags,
) -> Result<usize, VmError> {
    // Validate size
    if size == 0 {
        return Err(VmError::InvalidSize);
    }

    // Align size to page boundary
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Get current process and VM manager
    let _pid = myproc().ok_or(VmError::InvalidAddress)?;
    let vm_manager = get_current_vm_manager().ok_or(VmError::InvalidAddress)?;
    let mut vm = vm_manager.lock();

    // Determine mapping address
    let map_addr = if flags.fixed {
        // Fixed mapping - use provided address
        if addr & (PAGE_SIZE - 1) != 0 {
            return Err(VmError::InvalidAddress);
        }
        addr
    } else {
        // Find a free address
        if addr != 0 && addr & (PAGE_SIZE - 1) != 0 {
            return Err(VmError::InvalidAddress);
        }
        vm.find_free_range(aligned_size)?
    };

    // Convert permissions to flags
    let vm_perm = memory_protection_to_vm_perm(&prot);
    let pte_flags = vm_perm_to_pte_flags(&vm_perm);

    // Create VMA
    let vma = VmArea {
        start: map_addr,
        end: map_addr + aligned_size,
        flags: pte_flags,
    };

    // Add VMA to VM manager
    vm.add_vma(vma)?;

    // For anonymous mappings, allocate physical pages immediately or lazily
    if flags.anonymous {
        // For simplicity, we'll allocate immediately
        // In a real implementation, we might use lazy allocation
        if map_pages(map_addr, aligned_size, pte_flags).is_err() {
            // Clean up VMA on failure
            vm.remove_vmas(&Range { start: map_addr, end: map_addr + aligned_size });
            return Err(VmError::InvalidAddress);
        }
    }

    // Flush TLB for mapped region
    flush_tlb_page(map_addr);

    Ok(map_addr)
}

/// Unmap memory region
///
/// # Contract
/// * Address and size must match previously mapped region
/// * Cleanup page table entries
/// * Release related resources
/// * Handle partial unmap
pub fn munmap(addr: usize, size: usize) -> Result<(), VmError> {
    // Validate address
    if addr & (PAGE_SIZE - 1) != 0 {
        return Err(VmError::InvalidAddress);
    }

    // Validate size
    if size == 0 {
        return Ok(()); // munmap with size 0 is a no-op
    }

    // Align size to page boundary
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Get current process and VM manager
    let _pid = myproc().ok_or(VmError::InvalidAddress)?;
    let vm_manager = get_current_vm_manager().ok_or(VmError::InvalidAddress)?;
    let mut vm = vm_manager.lock();

    // Define the range to unmap
    let unmap_range = Range { start: addr, end: addr + aligned_size };

    // Find and remove VMAs in the range
    let removed_vmas = vm.remove_vmas(&unmap_range);

    if removed_vmas.is_empty() {
        return Err(VmError::MappingNotFound);
    }

    // Unmap pages from page table
    // Note: In a real implementation, we would need to unmap each page individually
    // For now, we'll just flush the TLB
    flush_tlb_page(addr);

    Ok(())
}

/// Change memory protection
///
/// # Contract
/// * Validate address range validity
/// * Update page table entries
/// * Handle permission conflict
/// * Support partial region protection change
pub fn mprotect(addr: usize, size: usize, prot: MemoryProtection) -> Result<(), VmError> {
    // Validate address
    if addr & (PAGE_SIZE - 1) != 0 {
        return Err(VmError::InvalidAddress);
    }

    // Validate size
    if size == 0 {
        return Ok(()); // mprotect with size 0 is a no-op
    }

    // Align size to page boundary
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Get current process and VM manager
    let _pid = myproc().ok_or(VmError::InvalidAddress)?;
    let vm_manager = get_current_vm_manager().ok_or(VmError::InvalidAddress)?;
    let mut vm = vm_manager.lock();

    // Define the range to protect
    let protect_range = Range { start: addr, end: addr + aligned_size };

    // Find VMAs that overlap with the range
    let overlapping_indices = vm.find_overlapping_vmas(&protect_range);

    if overlapping_indices.is_empty() {
        return Err(VmError::MappingNotFound);
    }

    // Update permissions for overlapping VMAs
    let new_perm = memory_protection_to_vm_perm(&prot);
    let new_flags = vm_perm_to_pte_flags(&new_perm);

    for &idx in &overlapping_indices {
        let vma = &mut vm.vmas[idx];

        // Check if the entire VMA is covered
        if vma.start >= protect_range.start && vma.end <= protect_range.end {
            // Update entire VMA
            vma.flags = new_flags;
        } else {
            // Partial coverage - split VMA (simplified for stub implementation)
            // In a full implementation, we would split the VMA into multiple parts
            // For now, just update the entire VMA's flags
            vma.flags = new_flags;
        }
    }

    // Update page table entries with new permissions
    // Note: In a real implementation, we would need to update each page's PTE
    // For now, we'll just flush the TLB
    flush_tlb_page(addr);

    Ok(())
}

/// Flush memory mapping to device
///
/// # Contract
/// * Write dirty pages to backing store
/// * Wait for I/O to complete
/// * Handle I/O errors
pub fn msync(addr: usize, size: usize) -> Result<(), VmError> {
    // Validate address
    if addr & (PAGE_SIZE - 1) != 0 {
        return Err(VmError::InvalidAddress);
    }

    // Validate size
    if size == 0 {
        return Ok(()); // msync with size 0 is a no-op
    }

    // Align size to page boundary
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Get current process and VM manager
    let _pid = myproc().ok_or(VmError::InvalidAddress)?;
    let vm_manager = get_current_vm_manager().ok_or(VmError::InvalidAddress)?;
    let vm = vm_manager.lock();

    // Define the range to sync
    let sync_range = Range { start: addr, end: addr + aligned_size };

    // Find VMAs that overlap with the range
    let overlapping_indices = vm.find_overlapping_vmas(&sync_range);

    if overlapping_indices.is_empty() {
        return Err(VmError::MappingNotFound);
    }

    // For each overlapping VMA, flush dirty pages
    for &idx in &overlapping_indices {
        let vma = &vm.vmas[idx];

        // Check if VMA is file-backed by looking at flags
        // This is a simplified check - in a real implementation, VmArea would have a file_backed field
        let is_file_backed = (vma.flags & flags::USER) != 0; // Simplified check

        if is_file_backed {
            // For file-backed mappings, write dirty pages to file
            // This is a simplified implementation
            // In a real implementation, we would:
            // 1. Iterate through all pages in the range
            // 2. Check if each page is dirty
            // 3. Write dirty pages to the backing file
            // 4. Wait for I/O to complete

            // For now, we'll just flush the TLB
            flush_tlb_page(vma.start);
        }
        // For anonymous mappings, msync is a no-op
    }

    Ok(())
}

/// Get memory mapping information
///
/// Returns information about memory mappings in the specified range
pub fn get_mapping_info(addr: usize, size: usize) -> Result<Vec<MemoryMapping>, VmError> {
    // Validate address
    if addr & (PAGE_SIZE - 1) != 0 {
        return Err(VmError::InvalidAddress);
    }

    // Validate size
    if size == 0 {
        return Ok(Vec::new());
    }

    // Align size to page boundary
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Get current process and VM manager
    let _pid = myproc().ok_or(VmError::InvalidAddress)?;
    let vm_manager = get_current_vm_manager().ok_or(VmError::InvalidAddress)?;
    let vm = vm_manager.lock();

    // Define the range to query
    let query_range = Range { start: addr, end: addr + aligned_size };

    // Find VMAs that overlap with the range
    let overlapping_indices = vm.find_overlapping_vmas(&query_range);

    let mut mappings = Vec::new();

    for &idx in &overlapping_indices {
        let vma = &vm.vmas[idx];

        // Calculate overlap range
        let overlap_start = core::cmp::max(vma.start, query_range.start);
        let overlap_end = core::cmp::min(vma.end, query_range.end);

        // Convert flags to VmPerm for MemoryProtection conversion
        let vm_perm = VmPerm {
            read: (vma.flags & flags::READ) != 0,
            write: (vma.flags & flags::WRITE) != 0,
            execute: (vma.flags & flags::EXEC) != 0,
        };

        // Create memory mapping
        let mapping = MemoryMapping {
            addr: overlap_start,
            size: overlap_end - overlap_start,
            protection: vm_perm_to_memory_protection(&vm_perm),
            flags: MapFlags {
                private: false,    // Simplified - stub doesn't track COW
                fixed: false,
                anonymous: true,   // Simplified - treat as anonymous
                no_cache: false,
            },
            mapping_type: MappingType::Anonymous, // Simplified
            ref_count: 1, // Simplified
        };

        mappings.push(mapping);
    }

    Ok(mappings)
}
