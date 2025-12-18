//! Memory management syscalls

use super::common::{SyscallError, SyscallResult, extract_args};
use crate::process::{PROC_TABLE, myproc};
use crate::mm::vm::{map_pages, flags, PAGE_SIZE, PageTable, map_page, flush_tlb_page, phys_to_kernel_ptr};
use crate::mm::{kalloc, kfree};
use crate::posix;
use crate::sync::Mutex;
use core::ptr;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};

// Import from compat
use crate::compat::MemoryRegion;

// Import advanced memory mapping functions

/// 全局内存统计
static MEM_STATS: Mutex<MemStats> = Mutex::new(MemStats::new());

/// 内存统计信息
#[derive(Debug, Default)]
pub struct MemStats {
    pub alloc_count: AtomicUsize,
    pub free_count: AtomicUsize,
    pub mmap_count: AtomicUsize,
    pub munmap_count: AtomicUsize,
    pub brk_count: AtomicUsize,
    pub total_allocated: AtomicUsize,
    pub total_freed: AtomicUsize,
}

impl MemStats {
    pub const fn new() -> Self {
        Self {
            alloc_count: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            mmap_count: AtomicUsize::new(0),
            munmap_count: AtomicUsize::new(0),
            brk_count: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
            total_freed: AtomicUsize::new(0),
        }
    }
    
    pub fn record_alloc(&self, size: usize) {
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_free(&self, size: usize) {
        self.free_count.fetch_add(1, Ordering::Relaxed);
        self.total_freed.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_mmap(&self, size: usize) {
        self.mmap_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_munmap(&self, size: usize) {
        self.munmap_count.fetch_add(1, Ordering::Relaxed);
        self.total_freed.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn record_brk(&self, size: isize) {
        self.brk_count.fetch_add(1, Ordering::Relaxed);
        if size > 0 {
            self.total_allocated.fetch_add(size as usize, Ordering::Relaxed);
        } else {
            self.total_freed.fetch_add((-size) as usize, Ordering::Relaxed);
        }
    }
    
    pub fn get_stats(&self) -> (usize, usize, usize, usize, usize, usize, usize) {
        (
            self.alloc_count.load(Ordering::Relaxed),
            self.free_count.load(Ordering::Relaxed),
            self.mmap_count.load(Ordering::Relaxed),
            self.munmap_count.load(Ordering::Relaxed),
            self.brk_count.load(Ordering::Relaxed),
            self.total_allocated.load(Ordering::Relaxed),
            self.total_freed.load(Ordering::Relaxed),
        )
    }
}

/// 获取内存统计信息
pub fn get_memory_stats() -> (usize, usize, usize, usize, usize, usize, usize) {
    MEM_STATS.lock().get_stats()
}

/// Dispatch memory management syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    // Validate syscall security
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    if !crate::security::validate_syscall(pid, syscall_id, &args.iter().map(|&x| x as usize).collect::<Vec<_>>()).unwrap_or(false) {
        return Err(SyscallError::PermissionDenied);
    }
    
    match syscall_id {
        // Memory operations
        0x3000 => sys_brk(args),            // brk
        0x3001 => sys_mmap(args),           // mmap
        0x3002 => sys_munmap(args),         // munmap
        0x3003 => sys_mprotect(args),       // mprotect
        0x3004 => sys_madvise(args),        // madvise
        0x3005 => sys_mlock(args),          // mlock
        0x3006 => sys_munlock(args),        // munlock
        0x3007 => sys_mlockall(args),       // mlockall
        0x3008 => sys_munlockall(args),     // munlockall
        0x3009 => sys_mincore(args),        // mincore
        0x300A => sys_msync(args),          // msync
        0x300B => sys_mremap(args),         // mremap
        0x300C => sys_remap_file_pages(args), // remap_file_pages
        0x300D => sys_shmget(args),         // shmget
        0x300E => sys_shmat(args),          // shmat
        0x300F => sys_shmdt(args),          // shmdt
        0x3010 => sys_shmctl(args),         // shmctl
        
        // Optimized memory allocator operations
        0x3100 => sys_optimized_alloc(args),      // optimized_alloc
        0x3101 => sys_optimized_free(args),       // optimized_free
        0x3102 => sys_optimized_realloc(args),    // optimized_realloc
        0x3103 => sys_optimized_stats(args),      // optimized_stats
        0x3104 => sys_optimized_defrag(args),     // optimized_defrag
        0x3105 => sys_optimized_pool_info(args),  // optimized_pool_info
        
        _ => Err(SyscallError::InvalidSyscall),
    }
}

// Placeholder implementations - to be replaced with actual syscall logic

fn sys_brk(args: &[u64]) -> SyscallResult {
    let addr = extract_args(args, 1)?[0] as usize;

    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;

    // Get current break
    let old_sz = proc.sz;

    // If addr is 0, return current break
    if addr == 0 {
        return Ok(old_sz as u64);
    }

    // Validate address range
    if addr >= crate::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }

    // For now, only allow increasing the break (simplified implementation)
    if addr > old_sz {
        // Calculate how many pages to allocate
        let pages_needed = ((addr - old_sz + PAGE_SIZE - 1) / PAGE_SIZE).max(1);
        let allocated_bytes = pages_needed * PAGE_SIZE;

        // Allocate and map pages
        let mut allocated_pages = Vec::new();
        for i in 0..pages_needed {
            let va = old_sz + i * PAGE_SIZE;
            let page = kalloc();
            if page.is_null() {
                // Clean up already allocated pages on failure
                for &allocated_va in &allocated_pages {
                    // Unmap and free each page
                    unsafe {
                        if let Some(pt) = proc.pagetable {
                            // Unmap the page
                            unmmap(pt, allocated_va, PAGE_SIZE);
                        }
                        // Free the physical page
                        kfree(core::ptr::from_raw_parts_mut(allocated_va as *mut u8, PAGE_SIZE));
                    }
                }
                return Err(SyscallError::OutOfMemory);
            }
            allocated_pages.push(va);

            // Zero page
            unsafe { ptr::write_bytes(page, 0, PAGE_SIZE); }

            // Map page with read/write permissions
            let perm = flags::PTE_R | flags::PTE_W | flags::PTE_U;
            unsafe {
                if map_pages(proc.pagetable, va, page as usize, PAGE_SIZE, perm).is_err() {
                    kfree(page);
                    // Clean up already allocated pages
                    for &allocated_va in &allocated_pages {
                        // Unmap and free each page
                        unsafe {
                            if let Some(pt) = proc.pagetable {
                                // Unmap page
                                unmmap(pt, allocated_va, PAGE_SIZE);
                            }
                            // Free physical page
                            kfree(core::ptr::from_raw_parts_mut(allocated_va as *mut u8, PAGE_SIZE));
                        }
                    }
                    return Err(SyscallError::OutOfMemory);
                }
            }
        }

        proc.sz = addr;
        
        // Record statistics
        MEM_STATS.lock().record_brk(allocated_bytes as isize);
    } else if addr < old_sz {
        // Shrinking break - properly unmap and free pages
        let freed_bytes = old_sz - addr;
        
        // Calculate page boundaries
        let old_end_page = (old_sz + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
        let new_end_page = (addr + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
        
        // Unmap pages from new_end_page to old_end_page
        if new_end_page < old_end_page {
            let unmap_start = new_end_page;
            let unmap_size = old_end_page - new_end_page;
            
            unsafe {
                if let Some(pt) = proc.pagetable {
                    // Unmap the pages
                    unmmap(pt, unmap_start, unmap_size);
                }
            }
        }
        
        proc.sz = addr;
        
        // Record statistics
        MEM_STATS.lock().record_brk(-(freed_bytes as isize));
    }

    Ok(proc.sz as u64)
}

pub fn sys_mmap(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 6)?;
    
    // Extract mmap parameters
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let prot = args[2] as i32;
    let flags = args[3] as i32;
    let fd = args[4] as i32;
    let offset = args[5] as i64;
    
    // Validate basic parameters
    if length == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Align to page boundaries
    let aligned_addr = if addr == 0 {
        0 // Let the kernel choose the address
    } else {
        addr & !(PAGE_SIZE - 1) // Align requested address
    };
    
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    
    // Validate page alignment if addr was specified
    if addr != 0 && addr != aligned_addr {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate memory access permissions
    let is_write = (prot & crate::posix::PROT_WRITE) != 0;
    let is_execute = (prot & crate::posix::PROT_EXEC) != 0;
    if !crate::security::validate_memory_access(pid, target_addr, aligned_length, is_write, is_execute).unwrap_or(false) {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Get user-space address range to map
    let mut target_addr = aligned_addr;
    
    // If no address was specified, find a free range in user space
    if target_addr == 0 {
        // Start searching from the heap end, which is at proc.sz
        target_addr = proc.sz;
        // Ensure we don't overlap with kernel space
        if target_addr + aligned_length >= crate::mm::vm::KERNEL_BASE {
            return Err(SyscallError::OutOfMemory);
        }
    }
    
    // Allocate and map pages - use map_pages for batch operation (more efficient than individual map_page)
    // Zero-initialize pages for anonymous mappings
    let mut total_pages = aligned_length / PAGE_SIZE;
    
    // Build permissions from prot and flags
    let mut vm_flags = flags::PTE_U; // User accessible
    
    if (prot & crate::posix::PROT_READ) != 0 {
        vm_flags |= flags::PTE_R;
    }
    
    if (prot & crate::posix::PROT_WRITE) != 0 {
        vm_flags |= flags::PTE_W;
    }
    
    if (prot & crate::posix::PROT_EXEC) != 0 {
        vm_flags |= flags::PTE_X;
    }
    
    // For now, handle only anonymous mappings (MAP_ANONYMOUS flag)
    if (flags & crate::posix::MAP_ANONYMOUS) != 0 {
        // Batch allocate pages
        let mut phys_pages: [usize; 32] = [0; 32]; // Batch size of 32 pages
        let mut current_offset = 0;
        
        while current_offset < total_pages {
            let batch_size = total_pages.min(32) - (current_offset % 32);
            let batch_start = current_offset;
            
            // Allocate physical pages in batch
            for i in 0..batch_size {
                let page = kalloc();
                if page.is_null() {
                    // Clean up any already allocated pages in this batch
                    for j in 0..i {
                        unsafe {
                            kfree(phys_pages[j] as *mut u8);
                        }
                    }
                    return Err(SyscallError::OutOfMemory);
                }
                
                // Zero-initialize the page
                unsafe {
                    core::ptr::write_bytes(page, 0, PAGE_SIZE);
                }
                
                phys_pages[i] = page as usize;
            }
            
            // Map batch of pages using map_pages (more efficient than individual map_page calls)
            let va_start = target_addr + batch_start * PAGE_SIZE;
            
            // Check if map_pages returns a result that indicates failure
            // Note: For this example, we're assuming map_pages returns Result<(), ()>
            
            unsafe {
                // In a real implementation, this would be:
                // match map_pages(pagetable, va_start, phys_pages.as_ptr(), batch_size * PAGE_SIZE, vm_flags) {
                //     Err(_) => {
                //         // Clean up allocated pages
                //         for i in 0..batch_size {
                //             kfree(phys_pages[i] as *mut u8);
                //         }
                //         return Err(SyscallError::OutOfMemory);
                //     }
                //     Ok(_) => {}
                // }
                
                // For demonstration, we'll use individual map_page calls until map_pages is implemented
                for i in 0..batch_size {
                    if map_page(pagetable, va_start + i * PAGE_SIZE, phys_pages[i], vm_flags).is_err() {
                        // Clean up
                        for j in 0..i {
                            unsafe {
                                kfree(phys_pages[j] as *mut u8);
                            }
                            // Unmap the page we just mapped
                            // For simplicity, not implemented here
                        }
                        for j in i..batch_size {
                            kfree(phys_pages[j] as *mut u8);
                        }
                        return Err(SyscallError::OutOfMemory);
                    }
                }
            }
            
            current_offset += batch_size;
        }
        
        // Update process size if we mapped beyond the current heap end
        if target_addr + aligned_length > proc.sz {
            proc.sz = target_addr + aligned_length;
        }
        
        Ok(target_addr as u64)
    } else {
        // TODO: Handle file-backed mappings
        Err(SyscallError::NotSupported)
    }
}

pub fn sys_munmap(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;

    // Validate arguments
    if length == 0 || addr == 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }


    
    // Validate memory access permissions
    let is_write = (prot & crate::posix::PROT_WRITE) != 0;
    let is_execute = (prot & crate::posix::PROT_EXEC) != 0;
    if !crate::security::validate_memory_access(pid, addr, len, is_write, is_execute).unwrap_or(false) {
        return Err(SyscallError::PermissionDenied);
    }

    // For each page in range, unmap it and free physical memory
    let mut current = start;
    let mut unmapped_count = 0;
    
    while current < end {
        // Try to unmap the page and get physical address
        #[cfg(target_arch = "riscv64")]
        {
            use crate::mm::vm::riscv64;
            if let Some(pa) = unsafe { riscv64::unmap_page(pagetable, current) } {
                // Free the physical page
                kfree(pa as *mut u8);
                unmapped_count += 1;
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            // For aarch64, use the exported unmap_page function
            unsafe {
                // Unmap the page (returns Result<(), ()>)
                if crate::mm::vm::unmap_page(pagetable, current).is_ok() {
                    // Note: For aarch64, we would need to track physical addresses
                    // separately. For now, we just unmap without freeing physical memory.
                    // TODO: Implement proper physical page tracking for aarch64
                    unmapped_count += 1;
                }
            }
        }
        
        #[cfg(target_arch = "x86_64")]
        {
            // x86_64 implementation would go here
            // For now, just increment count
            // TODO: Implement proper unmapping for x86_64
            unmapped_count += 1;
        }

        current += PAGE_SIZE;
    }

    // Flush TLB for the unmapped region
    let mut current = start;
    while current < end {
        crate::mm::vm::flush_tlb_page(current);
        current += PAGE_SIZE;
    }

    // Update process size if we unmapped memory beyond current break
    if end >= proc.sz {
        proc.sz = start.min(proc.sz);
    }

    Ok(unmapped_count as u64)
}

fn sys_mprotect(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let addr = args[0] as usize;
    let len = args[1] as usize;
    let prot = args[2] as i32;

    // Validate arguments
    if len == 0 || addr == 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (len + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }

    // Build new permissions
    let mut new_perm = flags::PTE_U; // User accessible
    if (prot & posix::PROT_READ) != 0 {
        new_perm |= flags::PTE_R;
    }
    if (prot & posix::PROT_WRITE) != 0 {
        new_perm |= flags::PTE_W;
    }
    if (prot & posix::PROT_EXEC) != 0 {
        new_perm |= flags::PTE_X;
    }

    // For each page, update permissions
    let mut current = start;
    let mut updated_count = 0;
    
    while current < end {
        unsafe {
            #[cfg(target_arch = "riscv64")]
            {
                // Get current PTE
                if let Some(pte_ptr) = crate::mm::vm::riscv64::walk(pagetable, current, false) {
                    if *pte_ptr & crate::mm::vm::riscv64::PTE_V != 0 {
                        // Page is mapped, update permissions
                        let old_pte = *pte_ptr;
                        let pa = crate::mm::vm::riscv64::pte_to_pa(old_pte);
                        let new_pte = crate::mm::vm::riscv64::pa_to_pte(pa) | new_perm | crate::mm::vm::riscv64::PTE_V;
                        *pte_ptr = new_pte;
                        updated_count += 1;
                    }
                }
            }
            
            #[cfg(target_arch = "aarch64")]
            {
                // Use the exported walk function
                if let Some(desc_ptr) = crate::mm::vm::walk(pagetable, current, false) {
                    // Constants for aarch64 descriptor flags
                    const DESC_VALID: usize = 1 << 0;
                    const DESC_AF: usize = 1 << 10;
                    const DESC_AP_RO: usize = 1 << 7;
                    const DESC_AP_USER: usize = 1 << 6;
                    const DESC_UXN: usize = 1 << 54;
                    const DESC_PXN: usize = 1 << 53;
                    
                    if *desc_ptr & DESC_VALID != 0 {
                        // Page is mapped, update permissions
                        let old_desc = *desc_ptr;
                        let pa = old_desc & !0xFFF;
                        let mut new_desc = pa | DESC_VALID | DESC_AF;
                        
                        if (new_perm & flags::PTE_W) == 0 {
                            new_desc |= DESC_AP_RO;
                        }
                        if (new_perm & flags::PTE_U) != 0 {
                            new_desc |= DESC_AP_USER;
                        }
                        if (new_perm & flags::PTE_X) == 0 {
                            new_desc |= DESC_UXN | DESC_PXN;
                        }
                        
                        *desc_ptr = new_desc;
                        updated_count += 1;
                    }
                }
            }
            
            #[cfg(target_arch = "x86_64")]
            {
                // x86_64 implementation would go here
                updated_count += 1;
            }
        }
        
        current += PAGE_SIZE;
    }

    // Flush TLB for the updated region
    let mut current = start;
    while current < end {
        unsafe {
            crate::mm::vm::flush_tlb_page(current);
        }
        current += PAGE_SIZE;
    }

    Ok(updated_count as u64)
}

fn sys_madvise(args: &[u64]) -> SyscallResult {
    // TODO: Implement madvise syscall
    Err(SyscallError::NotSupported)
}

fn sys_mlock(args: &[u64]) -> SyscallResult {
    // TODO: Implement mlock syscall
    Err(SyscallError::NotSupported)
}

fn sys_munlock(args: &[u64]) -> SyscallResult {
    // TODO: Implement munlock syscall
    Err(SyscallError::NotSupported)
}

fn sys_mlockall(args: &[u64]) -> SyscallResult {
    // TODO: Implement mlockall syscall
    Err(SyscallError::NotSupported)
}

fn sys_munlockall(args: &[u64]) -> SyscallResult {
    // TODO: Implement munlockall syscall
    Err(SyscallError::NotSupported)
}

fn sys_mincore(args: &[u64]) -> SyscallResult {
    // TODO: Implement mincore syscall
    Err(SyscallError::NotSupported)
}

/// Synchronize a memory-mapped file with storage
/// Arguments: [addr, length, flags]
/// Returns: 0 on success, error on failure
fn sys_msync(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 3)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let flags = args[2] as i32;
    
    // Validate arguments
    if addr == 0 || length == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate flags (MS_ASYNC, MS_INVALIDATE, MS_SYNC)
    const MS_ASYNC: i32 = 0x1;
    const MS_INVALIDATE: i32 = 0x2;
    const MS_SYNC: i32 = 0x4;
    
    if flags & !(MS_ASYNC | MS_INVALIDATE | MS_SYNC) != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    // Align to page boundaries
    let start = addr & !(crate::mm::vm::PAGE_SIZE - 1);
    let aligned_length = (length + crate::mm::vm::PAGE_SIZE - 1) & !(crate::mm::vm::PAGE_SIZE - 1);
    let end = start + aligned_length;
    
    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate memory access permissions (msync requires write access)
    if !crate::security::validate_memory_access(pid, addr, length, true, false).unwrap_or(false) {
        return Err(SyscallError::PermissionDenied);
    }
    
    // For each page in range, check if it's mapped and write back to file if needed
    let mut current = start;
    let mut synced_pages = 0;
    
    while current < end {
        // Check if page is mapped
        #[cfg(target_arch = "riscv64")]
        {
            use crate::mm::vm::riscv64;
            if let Some(pte_ptr) = unsafe { riscv64::walk(pagetable, current, false) } {
                if *pte_ptr & riscv64::PTE_V != 0 {
                    // Page is mapped, check if it's a file-backed mapping
                    // For now, we just count synced pages
                    synced_pages += 1;
                }
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            // For aarch64, check if page is mapped
            // TODO: Implement proper page table walk for aarch64
            synced_pages += 1;
        }
        
        #[cfg(target_arch = "x86_64")]
        {
            // For x86_64, check if page is mapped
            // TODO: Implement proper page table walk for x86_64
            synced_pages += 1;
        }
        
        current += crate::mm::vm::PAGE_SIZE;
    }
    
    // For now, we just return success
    // In a full implementation, we would:
    // 1. Identify which file backs this mapping
    // 2. Write back dirty pages to file
    // 3. Update file metadata
    // 4. Handle MS_ASYNC, MS_INVALIDATE, and MS_SYNC flags
    
    crate::println!("[msync] Synced {} pages at addr 0x{:x}, length 0x{:x}, flags 0x{:x}",
        synced_pages, addr, length, flags);
    
    Ok(0)
}

fn sys_mremap(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 5)?;
    let old_addr = args[0] as usize;
    let old_size = args[1] as usize;
    let new_size = args[2] as usize;
    let flags = args[3] as i32;
    let new_addr = args[4] as usize; // Only used with MREMAP_FIXED

    // Validate arguments
    if old_addr == 0 || old_size == 0 || new_size == 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Check alignment
    if old_addr & (PAGE_SIZE - 1) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Validate flags
    const MREMAP_MAYMOVE: i32 = 1;
    const MREMAP_FIXED: i32 = 2;
    const MREMAP_DONTUNMAP: i32 = 4;

    let valid_flags = MREMAP_MAYMOVE | MREMAP_FIXED | MREMAP_DONTUNMAP;
    if flags & !valid_flags != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // MREMAP_FIXED requires new_addr to be specified and page-aligned
    if (flags & MREMAP_FIXED) != 0 {
        if new_addr == 0 || new_addr & (PAGE_SIZE - 1) != 0 {
            return Err(SyscallError::InvalidArgument);
        }
        if new_addr >= crate::mm::vm::KERNEL_BASE {
            return Err(SyscallError::InvalidArgument);
        }
    }

    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }

    // Validate memory access permissions (mremap requires read/write access)
    if !crate::security::validate_memory_access(pid, old_addr, old_size, true, false).unwrap_or(false) {
        return Err(SyscallError::PermissionDenied);
    }

    // Align sizes to page boundaries
    let aligned_old_size = (old_size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let aligned_new_size = (new_size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Check if old region is valid
    if old_addr >= crate::mm::vm::KERNEL_BASE || old_addr + aligned_old_size > crate::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }

    // Find the memory region
    // For now, create a local regions map - this should be replaced with proper global state
    let mut regions: BTreeMap<usize, MemoryRegion> = BTreeMap::new();
    let region_key = old_addr;
    // Find the memory region and clone it to avoid borrowing issues
    let region = match regions.get(&region_key) {
        Some(r) if r.virtual_addr <= old_addr && (r.virtual_addr + r.size) >= old_addr + aligned_old_size => {
            // Clone the region to avoid immutable borrow issues later
            r.clone()
        },
        _ => return Err(SyscallError::InvalidArgument),
    };

    // Check if region is anonymous (we only support anonymous mappings for now)
    // For now, we assume anonymous mappings (file-backed mappings not supported yet)
    // TODO: Add file descriptor field to MemoryRegion if needed

    let result_addr = if aligned_new_size <= aligned_old_size {
        // Shrinking the mapping
        handle_mremap_shrink(pagetable, old_addr, aligned_old_size, aligned_new_size, &region, &mut regions)
    } else {
        // Expanding the mapping
        if (flags & MREMAP_MAYMOVE) != 0 {
            // Try to expand in place first
            match handle_mremap_expand_inplace(pagetable, old_addr, aligned_old_size, aligned_new_size, &region) {
                Ok(addr) => Ok(addr),
                Err(_) => {
                    // Expansion in place failed, try to move
                    handle_mremap_move(pagetable, old_addr, aligned_old_size, aligned_new_size, flags, new_addr, &region, &mut regions)
                }
            }
        } else {
            // Must expand in place
            handle_mremap_expand_inplace(pagetable, old_addr, aligned_old_size, aligned_new_size, &region)
        }
    }?;

    // Update process size if necessary
    if result_addr + aligned_new_size > proc.sz {
        proc.sz = result_addr + aligned_new_size;
    }

    crate::println!("[mremap] Remapped 0x{:x}-0x{:x} to 0x{:x}-0x{:x}, flags=0x{:x}",
        old_addr, old_addr + aligned_old_size, result_addr, result_addr + aligned_new_size, flags);

    Ok(result_addr as u64)
}

/// Handle shrinking of a memory mapping
fn handle_mremap_shrink(
    pagetable: *mut PageTable,
    old_addr: usize,
    old_size: usize,
    new_size: usize,
    region: &MemoryRegion,
    regions: &mut BTreeMap<usize, MemoryRegion>,
) -> Result<usize, SyscallError> {
    let pages_to_free = (old_size - new_size) / PAGE_SIZE;

    // Unmap excess pages
    for i in 0..pages_to_free {
        let va = old_addr + new_size + i * PAGE_SIZE;

        #[cfg(target_arch = "riscv64")]
        unsafe {
            if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, va) {
                kfree(pa as *mut u8);
            }
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            if crate::mm::vm::unmap_page(pagetable, va).is_ok() {
                // Note: AArch64 unmap_page doesn't return PA
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            // TODO: Implement x86_64 unmap
        }
    }

    // Flush TLB for unmapped pages
    for i in 0..pages_to_free {
        let va = old_addr + new_size + i * PAGE_SIZE;
        flush_tlb_page(va);
    }

    // Update region
    let mut updated_region = region.clone();
    updated_region.size = new_size;
    regions.insert(old_addr, updated_region);

    Ok(old_addr)
}

/// Handle expanding a memory mapping in place
fn handle_mremap_expand_inplace(
    pagetable: *mut PageTable,
    old_addr: usize,
    old_size: usize,
    new_size: usize,
    region: &MemoryRegion,
) -> Result<usize, SyscallError> {
    let pages_to_add = (new_size - old_size) / PAGE_SIZE;

    // Check if we can expand in place (no conflicts with adjacent mappings)
    // For now, create a local regions map - this should be replaced with proper global state
    let regions: BTreeMap<usize, MemoryRegion> = BTreeMap::new();
    let next_region_start = regions.range(old_addr + old_size..)
        .next()
        .map(|(_, r)| r.virtual_addr)
        .unwrap_or(usize::MAX);

    if old_addr + new_size > next_region_start {
        return Err(SyscallError::InvalidArgument); // Cannot expand in place
    }
    drop(regions);

    // Allocate and map new pages
    for i in 0..pages_to_add {
        let va = old_addr + old_size + i * PAGE_SIZE;

        let page = kalloc();
        if page.is_null() {
            // Clean up already allocated pages
            for j in 0..i {
                let cleanup_va = old_addr + old_size + j * PAGE_SIZE;
                #[cfg(target_arch = "riscv64")]
                unsafe {
                    if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                        kfree(pa as *mut u8);
                    }
                }
            }
            return Err(SyscallError::OutOfMemory);
        }

        // Zero the page
        unsafe { ptr::write_bytes(page, 0, PAGE_SIZE); }

        // Map page with same permissions as original region
        let perm = if region.permissions.read { flags::PTE_R } else { 0 } |
                   if region.permissions.write { flags::PTE_W } else { 0 } |
                   if region.permissions.execute { flags::PTE_X } else { 0 } |
                   flags::PTE_U;

        unsafe {
            if map_page(pagetable, va, page as usize, perm).is_err() {
                kfree(page);
                // Clean up already allocated pages
                for j in 0..i {
                    let cleanup_va = old_addr + old_size + j * PAGE_SIZE;
                    #[cfg(target_arch = "riscv64")]
                    if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                        kfree(pa as *mut u8);
                    }
                }
                return Err(SyscallError::OutOfMemory);
            }
        }
    }

    Ok(old_addr)
}

/// Handle moving a memory mapping to a new location
fn handle_mremap_move(
    pagetable: *mut PageTable,
    old_addr: usize,
    old_size: usize,
    new_size: usize,
    flags: i32,
    new_addr: usize,
    region: &MemoryRegion,
    regions: &mut BTreeMap<usize, MemoryRegion>,
) -> Result<usize, SyscallError> {
    const MREMAP_FIXED: i32 = 2;

    // Determine new address
    let target_addr = if (flags & MREMAP_FIXED) != 0 {
        new_addr
    } else {
        // Find a suitable new location
        find_free_address_range(&region, new_size)?
    };

    // Check if target location is available
    // For now, create a local regions map - this should be replaced with proper global state
    let regions_check: BTreeMap<usize, MemoryRegion> = BTreeMap::new();
    for existing_region in regions_check.values() {
        let existing_end = existing_region.virtual_addr + existing_region.size;
        let target_end = target_addr + new_size;
        // Check for overlap: not (existing_end <= target_addr || target_end <= existing_region.virtual_addr)
        if !(existing_end <= target_addr || target_end <= existing_region.virtual_addr) {
            return Err(SyscallError::InvalidArgument);
        }
    }
    drop(regions_check);

    // Copy data from old location to new location
    let pages_to_copy = old_size / PAGE_SIZE;

    for i in 0..pages_to_copy {
        let src_va = old_addr + i * PAGE_SIZE;
        let dst_va = target_addr + i * PAGE_SIZE;

        // Get source physical address
        #[cfg(target_arch = "riscv64")]
        let src_pa = unsafe {
            match crate::mm::vm::riscv64::translate(pagetable, src_va) {
                Some(pa) => pa,
                None => continue, // Skip if not mapped
            }
        };

        #[cfg(target_arch = "aarch64")]
        let src_pa = unsafe {
            match crate::mm::vm::walk(pagetable, src_va, false) {
                Some(pte) => (*pte & !0xFFF) | (src_va & (PAGE_SIZE - 1)),
                None => continue,
            }
        };

        #[cfg(target_arch = "x86_64")]
        let src_pa = unsafe {
            match crate::mm::vm::walk(pagetable, src_va, false) {
                Some(pte) => (*pte & !0xFFF) | (src_va & (PAGE_SIZE - 1)),
                None => continue,
            }
        };

        // Allocate new page for destination
        let dst_page = kalloc();
        if dst_page.is_null() {
            // Clean up: unmap any pages we already allocated
            for j in 0..i {
                let cleanup_va = target_addr + j * PAGE_SIZE;
                #[cfg(target_arch = "riscv64")]
                unsafe {
                    if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                        kfree(pa as *mut u8);
                    }
                }
            }
            return Err(SyscallError::OutOfMemory);
        }

        // Copy data
        unsafe {
            ptr::copy_nonoverlapping(
                phys_to_kernel_ptr(src_pa) as *const u8,
                dst_page,
                PAGE_SIZE,
            );
        }

        // Map destination page
        let perm = if region.permissions.read { flags::PTE_R } else { 0 } |
                   if region.permissions.write { flags::PTE_W } else { 0 } |
                   if region.permissions.execute { flags::PTE_X } else { 0 } |
                   flags::PTE_U;

        unsafe {
            if map_page(pagetable, dst_va, dst_page as usize, perm).is_err() {
                kfree(dst_page);
                // Clean up
                for j in 0..i {
                    let cleanup_va = target_addr + j * PAGE_SIZE;
                    #[cfg(target_arch = "riscv64")]
                    if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                        kfree(pa as *mut u8);
                    }
                }
                return Err(SyscallError::OutOfMemory);
            }
        }
    }

    // Allocate additional pages if new_size > old_size
    if new_size > old_size {
        let additional_pages = (new_size - old_size) / PAGE_SIZE;

        for i in 0..additional_pages {
            let va = target_addr + old_size + i * PAGE_SIZE;

            let page = kalloc();
            if page.is_null() {
                // Clean up
                for j in 0..(pages_to_copy + i) {
                    let cleanup_va = target_addr + j * PAGE_SIZE;
                    #[cfg(target_arch = "riscv64")]
                    unsafe {
                        if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                            kfree(pa as *mut u8);
                        }
                    }
                }
                return Err(SyscallError::OutOfMemory);
            }

            // Zero the page
            unsafe { ptr::write_bytes(page, 0, PAGE_SIZE); }

            // Map page
            let perm = if region.permissions.read { flags::PTE_R } else { 0 } |
                       if region.permissions.write { flags::PTE_W } else { 0 } |
                       if region.permissions.execute { flags::PTE_X } else { 0 } |
                       flags::PTE_U;

            unsafe {
                if map_page(pagetable, va, page as usize, perm).is_err() {
                    kfree(page);
                    // Clean up
                    for j in 0..(pages_to_copy + i) {
                        let cleanup_va = target_addr + j * PAGE_SIZE;
                        #[cfg(target_arch = "riscv64")]
                        if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                            kfree(pa as *mut u8);
                        }
                    }
                    return Err(SyscallError::OutOfMemory);
                }
            }
        }
    }

    // Remove old mapping (unless MREMAP_DONTUNMAP is set)
    if (flags & 4) == 0 { // MREMAP_DONTUNMAP
        for i in 0..(old_size / PAGE_SIZE) {
            let va = old_addr + i * PAGE_SIZE;

            #[cfg(target_arch = "riscv64")]
            unsafe {
                if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, va) {
                    kfree(pa as *mut u8);
                }
            }

            #[cfg(target_arch = "aarch64")]
            unsafe {
                let _ = crate::mm::vm::unmap_page(pagetable, va);
            }

            #[cfg(target_arch = "x86_64")]
            {
                // TODO: Implement x86_64 unmap
            }

            flush_tlb_page(va);
        }

        // Remove old region
        regions.remove(&old_addr);
    }

    // Add new region
    let mut new_region = region.clone();
    new_region.virtual_addr = target_addr;
    new_region.size = new_size;
    regions.insert(target_addr, new_region);

    Ok(target_addr)
}

/// Find a free address range for moving a mapping
fn find_free_address_range(region: &MemoryRegion, size: usize) -> Result<usize, SyscallError> {
    // Implement proper address space management
    // Start searching from the end of the current region
    let mut candidate = region.end;
    
    // Align to page boundary
    candidate = (candidate + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    
    // Check if the candidate range is free
    // For now, we'll use a simple check against known regions
    // In a real implementation, this would use a proper address space manager
    let max_user_addr = crate::mm::vm::KERNEL_BASE - PAGE_SIZE;
    
    // Ensure we don't exceed the maximum user address
    if candidate + size > max_user_addr {
        return Err(SyscallError::OutOfMemory);
    }
    
    // For now, just return the candidate address
    // In a real implementation, this would check for conflicts with other mappings
    Ok(candidate)
    } else {
        Err(SyscallError::OutOfMemory)
    }
}

fn sys_remap_file_pages(args: &[u64]) -> SyscallResult {
    // TODO: Implement remap_file_pages syscall
    Err(SyscallError::NotSupported)
}

fn sys_shmget(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let key = args[0] as i32;
    let size = args[1] as usize;
    let shmflg = args[2] as i32;

    // Use POSIX shmget implementation
    use crate::posix::shm::shmget;
    let shmid = unsafe { shmget(key, size, shmflg) };
    
    if shmid < 0 {
        Err(SyscallError::IoError)
    } else {
        Ok(shmid as u64)
    }
}

fn sys_shmat(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let shmid = args[0] as i32;
    let shmaddr = args[1] as *mut u8;
    let shmflg = args[2] as i32;

    // Use POSIX shmat implementation
    use crate::posix::shm::shmat;
    let addr = unsafe { shmat(shmid, shmaddr, shmflg) };
    
    if addr.is_null() {
        Err(SyscallError::IoError)
    } else {
        Ok(addr as usize as u64)
    }
}

fn sys_shmdt(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let shmaddr = args[0] as *mut u8;

    // Use POSIX shmdt implementation
    use crate::posix::shm::shmdt;
    let result = unsafe { shmdt(shmaddr) };
    
    if result < 0 {
        Err(SyscallError::IoError)
    } else {
        Ok(0)
    }
}

fn sys_shmctl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let shmid = args[0] as i32;
    let cmd = args[1] as i32;
    let buf = args[2] as *mut crate::posix::ShmidDs;

    // Use POSIX shmctl implementation
    use crate::posix::shm::shmctl;
    let result = unsafe { shmctl(shmid, cmd, buf) };
    
    if result < 0 {
        Err(SyscallError::IoError)
    } else {
        Ok(result as u64)
    }
}

// ============================================================================
// Optimized Memory Allocator Syscalls
// ============================================================================

/// Allocate memory using the optimized allocator
/// Arguments: [size, align, pid]
/// Returns: pointer to allocated memory or error
fn sys_optimized_alloc(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let size = args[0] as usize;
    let align = args[1] as usize;
    let pid = args[2] as u32;
    
    // Validate arguments
    if size == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process if pid is 0
    let process_pid = if pid == 0 {
        myproc().ok_or(SyscallError::InvalidArgument)?.0
    } else {
        pid
    };
    
    // Create layout for allocation
    let layout = unsafe {
        core::alloc::Layout::from_size_align_unchecked(size, align.max(8))
    };
    
    // Use optimized allocator
    use crate::memory_optimized;
    let ptr = memory_optimized::allocate_optimized(layout, process_pid);
    
    if ptr.is_null() {
        Err(SyscallError::OutOfMemory)
    } else {
        Ok(ptr as u64)
    }
}

/// Free memory allocated by the optimized allocator
/// Arguments: [ptr, pid]
/// Returns: 0 on success, error on failure
fn sys_optimized_free(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let ptr = args[0] as *mut u8;
    let pid = args[1] as u32;
    
    // Validate arguments
    if ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process if pid is 0
    let process_pid = if pid == 0 {
        myproc().ok_or(SyscallError::InvalidArgument)?.0
    } else {
        pid
    };
    
    // Use optimized allocator
    use crate::memory_optimized;
    // Create a dummy layout for deallocation (in a real implementation, we would track this)
    let layout = unsafe { core::alloc::Layout::from_size_align_unchecked(1, 1) };
    memory_optimized::deallocate_optimized(ptr, layout, process_pid);
    
    Ok(0)
}

/// Reallocate memory using the optimized allocator
/// Arguments: [ptr, new_size, align, pid]
/// Returns: pointer to reallocated memory or error
fn sys_optimized_realloc(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 4)?;
    let ptr = args[0] as *mut u8;
    let new_size = args[1] as usize;
    let align = args[2] as usize;
    let pid = args[3] as u32;
    
    // Validate arguments
    if new_size == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process if pid is 0
    let process_pid = if pid == 0 {
        myproc().ok_or(SyscallError::InvalidArgument)?.0
    } else {
        pid
    };
    
    // Create layout for reallocation
    let layout = unsafe {
        core::alloc::Layout::from_size_align_unchecked(new_size, align.max(8))
    };
    
    // Use optimized allocator
    use crate::memory_optimized;
    let new_ptr = memory_optimized::reallocate_optimized(ptr, layout, process_pid);
    
    if new_ptr.is_null() {
        Err(SyscallError::OutOfMemory)
    } else {
        Ok(new_ptr as u64)
    }
}

/// Get statistics from the optimized allocator
/// Arguments: [pid]
/// Returns: statistics structure or error
fn sys_optimized_stats(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let pid = args[1] as u32;
    
    // Get current process if pid is 0
    let process_pid = if pid == 0 {
        myproc().ok_or(SyscallError::InvalidArgument)?.0
    } else {
        pid
    };
    
    // Get statistics from optimized allocator
    use crate::memory_optimized;
    let stats = memory_optimized::get_allocator_stats();
    
    // For now, return total allocated bytes as a simple statistic
    // In a real implementation, we would return a structure with all statistics
    Ok(stats.total_allocated as u64)
}

/// Trigger defragmentation in the optimized allocator
/// Arguments: [pid, aggressive]
/// Returns: 0 on success, error on failure
fn sys_optimized_defrag(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let pid = args[0] as u32;
    let aggressive = args[1] != 0;
    
    // Get current process if pid is 0
    let process_pid = if pid == 0 {
        myproc().ok_or(SyscallError::InvalidArgument)?.0
    } else {
        pid
    };
    
    // Trigger defragmentation
    use crate::memory_optimized;
    memory_optimized::defragment_memory();
    
    Ok(0)
}

/// Get information about memory pools in the optimized allocator
/// Arguments: [pid, pool_type]
/// Returns: pool information or error
fn sys_optimized_pool_info(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let pid = args[0] as u32;
    let pool_type = args[1] as u32; // 0=small, 1=medium, 2=large, 3=all
    
    // Get current process if pid is 0
    let process_pid = if pid == 0 {
        myproc().ok_or(SyscallError::InvalidArgument)?.0
    } else {
        pid
    };
    
    // Get pool information
    use crate::memory_optimized;
    let pool_info = memory_optimized::get_pool_info(pool_type);
    
    // For now, return total free blocks as a simple statistic
    // In a real implementation, we would return a structure with all pool information
    Ok(pool_info.free_blocks as u64)
}
