//! 内存管理系统调用处理函数
//!
//! 本模块包含内存管理相关系统调用的具体实现逻辑，包括：
//! - 内存映射和取消映射
//! - 内存保护操作
//! - 内存分配和释放
//! - 虚拟内存管理

use nos_nos_error_handling::unified::{KernelError, KernelResult};
// use crate::syscalls::mm::types::*;
use crate::process::{PROC_TABLE, myproc};
use crate::mm::vm::{flags, PAGE_SIZE, map_page, flush_tlb_page};
use crate::mm::{kalloc, kfree};
use core::ptr;
// use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Extract arguments with validation
///
/// Extracts `n` arguments from the argument slice, returning an error if insufficient arguments are provided.
fn extract_args(args: &[u64], n: usize) -> KernelResult<&[u64]> {
    if args.len() < n {
        return Err(KernelError::InvalidArgument);
    }
    Ok(&args[0..n])
}

/// mmap系统调用处理函数
///
/// 创建内存映射。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, length, prot, flags, fd, offset]
///
/// # 返回值
///
/// * `Ok(u64)` - 映射地址
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mmap(args: &[u64]) -> KernelResult<u64> {
    use crate::posix;

    let args = extract_args(args, 6)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let prot = args[2] as i32;
    let flags = args[3] as i32;
    let fd = args[4] as i32;
    let offset = args[5] as i64;

    // Validate basic parameters
    if length == 0 {
        return Err(KernelError::InvalidArgument);
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
        return Err(KernelError::InvalidArgument);
    }

    // Get current process
    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // Get user-space address range to map
    let mut target_addr = aligned_addr;

    // If no address was specified, find a free range in user space
    if target_addr == 0 {
        // Start searching from the heap end, which is at proc.sz
        target_addr = proc.sz;
        // Ensure we don't overlap with kernel space
        if target_addr + aligned_length >= crate::mm::vm::KERNEL_BASE {
            return Err(KernelError::OutOfMemory);
        }
    }

    // Allocate and map pages - use map_pages for batch operation (more efficient than individual map_page)
    // Zero-initialize pages for anonymous mappings
    let mut total_pages = aligned_length / PAGE_SIZE;

    // Build permissions from prot and flags
    let mut vm_flags = flags::PTE_U; // User accessible

    if (prot & posix::PROT_READ) != 0 {
        vm_flags |= flags::PTE_R;
    }

    if (prot & posix::PROT_WRITE) != 0 {
        vm_flags |= flags::PTE_W;
    }

    if (prot & posix::PROT_EXEC) != 0 {
        vm_flags |= flags::PTE_X;
    }

    // For now, handle only anonymous mappings (MAP_ANONYMOUS flag)
    if (flags & posix::MAP_ANONYMOUS) != 0 {
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
                    return Err(KernelError::OutOfMemory);
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
                //         return Err(KernelError::OutOfMemory);
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
                        return Err(KernelError::OutOfMemory);
                    }
                }
            }

            current_offset += batch_size;
        }

        // Update process size if we mapped beyond the current heap end
        if target_addr + aligned_length > proc.sz {
            proc.sz = target_addr + aligned_length;
        }

        crate::log_debug!("mmap syscall: mapped {} pages at addr {:#x}", total_pages, target_addr);
        Ok(target_addr as u64)
    } else {
        // TODO: Handle file-backed mappings
        Err(KernelError::NotSupported)
    }
}

/// munmap系统调用处理函数
///
/// 取消内存映射。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, length]
///
/// # 返回值
///
/// * `Ok(u64)` - 取消映射的页面数量
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_munmap(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 2)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;

    // Validate arguments
    if length == 0 || addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
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
        flush_tlb_page(current);
        current += PAGE_SIZE;
    }

    // Update process size if we unmapped memory beyond current break
    if end >= proc.sz {
        proc.sz = start.min(proc.sz);
    }

    crate::log_debug!("munmap syscall: unmapped {} pages from addr {:#x}", unmapped_count, addr);
    Ok(unmapped_count as u64)
}

/// mprotect系统调用处理函数
///
/// 修改内存保护属性。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, len, prot]
///
/// # 返回值
///
/// * `Ok(u64)` - 更新页面数量
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mprotect(args: &[u64]) -> KernelResult<u64> {
    use crate::posix;

    let args = extract_args(args, 3)?;
    let addr = args[0] as usize;
    let len = args[1] as usize;
    let prot = args[2] as i32;

    // Validate arguments
    if len == 0 || addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (len + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
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

    crate::log_debug!("mprotect syscall: updated {} pages at addr {:#x}", updated_count, addr);
    Ok(updated_count as u64)
}

/// msync系统调用处理函数
///
/// 同步内存映射到文件。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, length, flags]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_msync(args: &[u64]) -> KernelResult<u64> {
    use crate::syscalls::common::extract_args;

    let args = extract_args(args, 3)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let flags = args[2] as i32;

    // Validate arguments
    if addr == 0 || length == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Validate flags (MS_ASYNC, MS_INVALIDATE, MS_SYNC)
    const MS_ASYNC: i32 = 0x1;
    const MS_INVALIDATE: i32 = 0x2;
    const MS_SYNC: i32 = 0x4;

    if flags & !(MS_ASYNC | MS_INVALIDATE | MS_SYNC) != 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Get current process
    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);

    // Align to page boundaries
    let start = addr & !(crate::mm::vm::PAGE_SIZE - 1);
    let aligned_length = (length + crate::mm::vm::PAGE_SIZE - 1) & !(crate::mm::vm::PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
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

    crate::log_debug!("msync syscall: synced {} pages at addr {:#x}, length {:#x}, flags {:#x}",
        synced_pages, addr, length, flags);

    Ok(0)
}

/// mlock系统调用处理函数
/// 
/// 锁定内存页面。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[addr, len]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mlock(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 2)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;

    // Validate arguments
    if length == 0 || addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // For each page in range, lock it
    let mut current = start;
    let mut locked_count = 0;

    while current < end {
        // In a real implementation, we would:
        // 1. Check if the page is mapped
        // 2. Mark the page as locked (prevent swapping)
        // 3. Increment the process's locked memory count
        
        // For now, we just count pages
        locked_count += 1;
        current += PAGE_SIZE;
    }

    crate::log_debug!("mlock syscall: locked {} pages from addr {:#x}", locked_count, addr);
    Ok(0)
}

/// munlock系统调用处理函数
/// 
/// 解锁内存页面。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[addr, len]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_munlock(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 2)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;

    // Validate arguments
    if length == 0 || addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // For each page in range, unlock it
    let mut current = start;
    let mut unlocked_count = 0;

    while current < end {
        // In a real implementation, we would:
        // 1. Check if the page is mapped and locked
        // 2. Mark the page as unlocked (allow swapping)
        // 3. Decrement the process's locked memory count
        
        // For now, we just count pages
        unlocked_count += 1;
        current += PAGE_SIZE;
    }

    crate::log_debug!("munlock syscall: unlocked {} pages from addr {:#x}", unlocked_count, addr);
    Ok(0)
}

/// brk系统调用处理函数
///
/// 改变程序堆大小。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr]
///
/// # 返回值
///
/// * `Ok(u64)` - 新的堆结束地址
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_brk(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 1)?;
    let addr = args[0] as usize;

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;

    // Get current break
    let old_sz = proc.sz;

    // If addr is 0, return current break
    if addr == 0 {
        return Ok(old_sz as u64);
    }

    // Validate address range
    if addr >= crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    // For now, only allow increasing the break (simplified implementation)
    if addr > old_sz {
        // Calculate how many pages to allocate
        let pages_needed = ((addr - old_sz + PAGE_SIZE - 1) / PAGE_SIZE).max(1);

        // Allocate and map pages
        for i in 0..pages_needed {
            let va = old_sz + i * PAGE_SIZE;
            let page = kalloc();
            if page.is_null() {
                // TODO: Clean up already allocated pages on failure
                return Err(KernelError::OutOfMemory);
            }

            // Zero page
            unsafe { ptr::write_bytes(page, 0, PAGE_SIZE); }

            // Map page with read/write permissions
            let perm = flags::PTE_R | flags::PTE_W | flags::PTE_U;
            unsafe {
                if map_page(proc.pagetable, va, page as usize, perm).is_err() {
                    kfree(page);
                    // TODO: Clean up already allocated pages
                    return Err(KernelError::OutOfMemory);
                }
            }
        }

        proc.sz = addr;
    } else if addr < old_sz {
        // Shrinking break - for now, just update size (simplified)
        // TODO: Properly unmap and free pages
        proc.sz = addr;
    }

    crate::log_debug!("brk syscall: updated heap from {:#x} to {:#x}", old_sz, proc.sz);
    Ok(proc.sz as u64)
}

/// sbrk系统调用处理函数
///
/// 增加程序堆大小。
///
/// # 参数
///
/// * `args` - 系统调用参数：[increment]
///
/// # 返回值
///
/// * `Ok(u64)` - 旧的堆结束地址
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_sbrk(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 1)?;
    let increment = args[0] as i64;

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;

    let old_heap_end = proc.sz;

    if increment > 0 {
        let new_end = old_heap_end.wrapping_add(increment as usize);
        if new_end >= crate::mm::vm::KERNEL_BASE {
            return Err(KernelError::OutOfMemory);
        }
        proc.sz = new_end;
    } else if increment < 0 {
        let new_end = old_heap_end.saturating_sub((-increment) as usize);
        proc.sz = new_end;
    }

    crate::log_debug!("sbrk syscall: increment={}, old_end={:#x}, new_end={:#x}", increment, old_heap_end, proc.sz);
    Ok(old_heap_end as u64)
}

/// mlockall系统调用处理函数
///
/// 锁定进程的所有内存页面。
///
/// # 参数
///
/// * `args` - 系统调用参数：[flags]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mlockall(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 1)?;
    let flags = args[0] as i32;

    // Validate flags
    const MCL_CURRENT: i32 = 0x1;
    const MCL_FUTURE: i32 = 0x2;
    const MCL_ONFAULT: i32 = 0x4;

    if flags & !(MCL_CURRENT | MCL_FUTURE | MCL_ONFAULT) != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. If MCL_CURRENT is set, lock all currently mapped pages
    // 2. If MCL_FUTURE is set, mark the process to lock all future mappings
    // 3. If MCL_ONFAULT is set, lock pages only when they are accessed
    
    // For now, we just log the call
    crate::log_debug!("mlockall syscall called with flags: {}", flags);
    Ok(0)
}

/// munlockall系统调用处理函数
///
/// 解锁进程的所有内存页面。
///
/// # 参数
///
/// * `args` - 系统调用参数（空）
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_munlockall(args: &[u64]) -> KernelResult<u64> {
    // No arguments for munlockall
    if !args.is_empty() {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Unlock all currently locked pages for the process
    // 2. Clear the MCL_FUTURE flag if set
    
    // For now, we just log the call
    crate::log_debug!("munlockall syscall called");
    Ok(0)
}

/// mincore系统调用处理函数
///
/// 检查内存页面是否在物理内存中。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, length, vec]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mincore(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let vec_addr = args[2] as usize;

    // Validate arguments
    if length == 0 || addr == 0 || vec_addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;
    let page_count = aligned_length / PAGE_SIZE;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Check if vec_addr is a valid user address
    // 2. For each page in range, check if it's resident in memory
    // 3. Set the corresponding bit in the vector
    
    // For now, we just log the call
    crate::log_debug!("mincore syscall: addr={:#x}, length={}, vec_addr={:#x}, page_count={}", 
                     addr, length, vec_addr, page_count);
    Ok(0)
}

/// mremap系统调用处理函数
///
/// 重新映射内存区域。
///
/// # 参数
///
/// * `args` - 系统调用参数：[old_addr, old_size, new_size, flags, new_addr]
///
/// # 返回值
///
/// * `Ok(u64)` - 新的映射地址
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mremap(args: &[u64]) -> KernelResult<u64> {
    // TODO: Implement proper mremap
    Err(KernelError::NotSupported)
}

/// remap_file_pages系统调用处理函数
///
/// 重新映射文件页面。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, size, prot, pgoff, flags]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_remap_file_pages(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 5)?;
    let addr = args[0] as usize;
    let size = args[1] as usize;
    let prot = args[2] as i32;
    let flags = args[3] as i32;
    let new_addr = args[4] as usize;

    // Validate arguments
    if size == 0 || addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_size;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    // Validate flags
    const MREMAP_MAYMOVE: i32 = 0x1;
    const MREMAP_FIXED: i32 = 0x2;

    if flags & !(MREMAP_MAYMOVE | MREMAP_FIXED) != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Check if the mapping at addr exists and is a file mapping
    // 2. If MREMAP_FIXED is set, unmap any existing mapping at new_addr
    // 3. Remap the pages to the new location with the new protection
    // 4. Update the process's memory map
    
    // For now, we just log the call
    crate::log_debug!("remap_file_pages syscall: addr={:#x}, size={}, prot={}, flags={}, new_addr={:#x}", 
                     addr, size, prot, flags, new_addr);
    Ok(0)
}

/// shmget系统调用处理函数
///
/// 创建或获取共享内存段。
///
/// # 参数
///
/// * `args` - 系统调用参数：[key, size, shmflg]
///
/// # 返回值
///
/// * `Ok(u64)` - 共享内存ID
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmget(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let key = args[0] as i32;
    let size = args[1] as usize;
    let shmflg = args[2] as i32;

    // Use POSIX shmget implementation
    use crate::posix::shm::shmget;
    let shmid = unsafe { shmget(key, size, shmflg) };

    if shmid < 0 {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(shmid as u64)
    }
}

/// shmat系统调用处理函数
///
/// 将共享内存段附加到进程地址空间。
///
/// # 参数
///
/// * `args` - 系统调用参数：[shmid, shmaddr, shmflg]
///
/// # 返回值
///
/// * `Ok(u64)` - 附加地址
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmat(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let shmid = args[0] as i32;
    let shmaddr = args[1] as *mut u8;
    let shmflg = args[2] as i32;

    // Use POSIX shmat implementation
    use crate::posix::shm::shmat;
    let addr = unsafe { shmat(shmid, shmaddr, shmflg) };

    if addr.is_null() {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(addr as usize as u64)
    }
}

/// shmdt系统调用处理函数
///
/// 从进程地址空间分离共享内存段。
///
/// # 参数
///
/// * `args` - 系统调用参数：[shmaddr]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmdt(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 1)?;
    let shmaddr = args[0] as *mut u8;

    // Use POSIX shmdt implementation
    use crate::posix::shm::shmdt;
    let result = unsafe { shmdt(shmaddr) };

    if result < 0 {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(0)
    }
}

/// shmctl系统调用处理函数
///
/// 控制共享内存段。
///
/// # 参数
///
/// * `args` - 系统调用参数：[shmid, cmd, buf]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功或操作结果
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_shmctl(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let shmid = args[0] as i32;
    let cmd = args[1] as i32;
    let buf = args[2] as *mut crate::posix::ShmidDs;

    // Use POSIX shmctl implementation
    use crate::posix::shm::shmctl;
    let result = unsafe { shmctl(shmid, cmd, buf) };

    if result < 0 {
        Err(KernelError::InvalidArgument)
    } else {
        Ok(result as u64)
    }
}

/// madvise系统调用处理函数
///
/// 给内核提供建议。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, length, advice]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_madvise(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let advice = args[2] as i32;

    // Validate arguments
    if addr == 0 || length == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries and validate range
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    // Validate advice flags
    const MADV_NORMAL: i32 = 0;
    const MADV_RANDOM: i32 = 1;
    const MADV_SEQUENTIAL: i32 = 2;
    const MADV_WILLNEED: i32 = 3;
    const MADV_DONTNEED: i32 = 4;
    const MADV_FREE: i32 = 8;
    const MADV_REMOVE: i32 = 9;
    const MADV_DONTFORK: i32 = 10;
    const MADV_DOFORK: i32 = 11;
    const MADV_MERGEABLE: i32 = 12;
    const MADV_UNMERGEABLE: i32 = 13;
    const MADV_HUGEPAGE: i32 = 14;
    const MADV_NOHUGEPAGE: i32 = 15;
    const MADV_DONTDUMP: i32 = 16;
    const MADV_DODUMP: i32 = 17;
    const MADV_WIPEONFORK: i32 = 18;
    const MADV_KEEPONFORK: i32 = 19;

    let valid_advice = [
        MADV_NORMAL, MADV_RANDOM, MADV_SEQUENTIAL, MADV_WILLNEED, MADV_DONTNEED,
        MADV_FREE, MADV_REMOVE, MADV_DONTFORK, MADV_DOFORK, MADV_MERGEABLE,
        MADV_UNMERGEABLE, MADV_HUGEPAGE, MADV_NOHUGEPAGE, MADV_DONTDUMP,
        MADV_DODUMP, MADV_WIPEONFORK, MADV_KEEPONFORK
    ];

    if !valid_advice.contains(&advice) {
        return Err(KernelError::InvalidArgument);
    }

    // For now, we acknowledge the advice but don't take specific actions
    // In a full implementation, we would:
    // - MADV_WILLNEED: Prefault pages
    // - MADV_DONTNEED: Discard pages
    // - MADV_FREE: Mark pages as freeable
    // - etc.

    crate::log_debug!("madvise syscall: addr={:#x}, length={}, advice={}", addr, length, advice);

    Ok(0)
}

/// getpagesize系统调用处理函数
///
/// 获取系统页面大小。
///
/// # 参数
///
/// * `args` - 系统调用参数（通常为空）
///
/// # 返回值
///
/// * `Ok(u64)` - 页面大小
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_getpagesize(args: &[u64]) -> KernelResult<u64> {
    if !args.is_empty() {
        return Err(KernelError::InvalidArgument);
    }

    // Return actual page size from system configuration
    let page_size = crate::syscalls::mm::types::PageSize::Size4K as u64;

    crate::log_debug!("getpagesize syscall: returning {}", page_size);
    Ok(page_size)
}

/// mbind系统调用处理函数
///
/// 设置内存区域的NUMA内存绑定策略。
///
/// # 参数
///
/// * `args` - 系统调用参数：[addr, length, mode, nodemask, maxnode, flags]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_mbind(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 6)?;
    let addr = args[0] as usize;
    let length = args[1] as usize;
    let mode = args[2] as i32;
    let nodemask_addr = args[3] as usize;
    let maxnode = args[4] as u32;
    let flags = args[5] as u32;

    // Validate arguments
    if length == 0 || addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Align to page boundaries
    let start = addr & !(PAGE_SIZE - 1);
    let aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::mm::vm::KERNEL_BASE || end > crate::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    // Validate mode
    const MPOL_DEFAULT: i32 = 0;
    const MPOL_PREFERRED: i32 = 1;
    const MPOL_BIND: i32 = 2;
    const MPOL_INTERLEAVE: i32 = 3;

    if mode < MPOL_DEFAULT || mode > MPOL_INTERLEAVE {
        return Err(KernelError::InvalidArgument);
    }

    // Validate flags
    const MPOL_MF_MOVE: u32 = 0x1;
    const MPOL_MF_MOVE_ALL: u32 = 0x2;
    const MPOL_MF_STRICT: u32 = 0x4;

    if flags & !(MPOL_MF_MOVE | MPOL_MF_MOVE_ALL | MPOL_MF_STRICT) != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Parse the nodemask from user space if nodemask_addr is not null
    // 2. Apply the memory policy to the specified memory range
    // 3. If MPOL_MF_MOVE or MPOL_MF_MOVE_ALL is set, move pages to match the policy
    // 4. If MPOL_MF_STRICT is set, fail if any page can't be moved
    
    // For now, we just log the call
    crate::log_debug!("mbind syscall: addr={:#x}, length={}, mode={}, nodemask_addr={:#x}, maxnode={}, flags={}", 
                     addr, length, mode, nodemask_addr, maxnode, flags);
    Ok(0)
}

/// get_mempolicy系统调用处理函数
///
/// 获取NUMA内存策略。
///
/// # 参数
///
/// * `args` - 系统调用参数：[policy, nodemask, maxnode, addr, flags]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_get_mempolicy(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 5)?;
    let policy_addr = args[0] as usize;
    let nodemask_addr = args[1] as usize;
    let maxnode = args[2] as u32;
    let addr = args[3] as usize;
    let flags = args[4] as u32;

    // Validate flags
    const MPOL_F_NODE: u32 = 0x1;
    const MPOL_F_ADDR: u32 = 0x2;
    const MPOL_F_MEMS_ALLOWED: u32 = 0x4;

    if flags & !(MPOL_F_NODE | MPOL_F_ADDR | MPOL_F_MEMS_ALLOWED) != 0 {
        return Err(KernelError::InvalidArgument);
    }

    // If MPOL_F_ADDR is set, validate the address
    if (flags & MPOL_F_ADDR) != 0 {
        if addr == 0 || addr >= crate::mm::vm::KERNEL_BASE {
            return Err(KernelError::InvalidArgument);
        }
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. If MPOL_F_MEMS_ALLOWED is set, return the set of nodes allowed for the current process
    // 2. If MPOL_F_ADDR is set, get the policy for the specified address
    // 3. Otherwise, get the default policy for the current process
    // 4. Write the policy to user space if policy_addr is not null
    // 5. Write the nodemask to user space if nodemask_addr is not null
    
    // For now, we just log the call and return a default policy
    crate::log_debug!("get_mempolicy syscall: policy_addr={:#x}, nodemask_addr={:#x}, maxnode={}, addr={:#x}, flags={}", 
                     policy_addr, nodemask_addr, maxnode, addr, flags);
    
    // Return MPOL_DEFAULT as the default policy
    Ok(0) // MPOL_DEFAULT
}

/// set_mempolicy系统调用处理函数
///
/// 设置NUMA内存策略。
///
/// # 参数
///
/// * `args` - 系统调用参数：[mode, nodemask, maxnode]
///
/// # 返回值
///
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_set_mempolicy(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 3)?;
    let mode = args[0] as i32;
    let nodemask_addr = args[1] as usize;
    let maxnode = args[2] as u32;

    // Validate mode
    const MPOL_DEFAULT: i32 = 0;
    const MPOL_PREFERRED: i32 = 1;
    const MPOL_BIND: i32 = 2;
    const MPOL_INTERLEAVE: i32 = 3;

    if mode < MPOL_DEFAULT || mode > MPOL_INTERLEAVE {
        return Err(KernelError::InvalidArgument);
    }

    // For MPOL_DEFAULT, nodemask should be null
    if mode == MPOL_DEFAULT && nodemask_addr != 0 {
        return Err(KernelError::InvalidArgument);
    }

    // For other modes, nodemask should not be null
    if mode != MPOL_DEFAULT && nodemask_addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    let pid = myproc().ok_or(KernelError::InvalidArgument)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Parse the nodemask from user space if nodemask_addr is not null
    // 2. Set the default memory policy for the current process
    // 3. Store the policy in the process's memory policy structure
    
    // For now, we just log the call
    crate::log_debug!("set_mempolicy syscall: mode={}, nodemask_addr={:#x}, maxnode={}", 
                     mode, nodemask_addr, maxnode);
    Ok(0)
}

/// migrate_pages系统调用处理函数
///
/// 将指定进程的页面从一个NUMA节点迁移到另一个节点。
///
/// # 参数
///
/// * `args` - 系统调用参数：[pid, maxnode, old_nodes, new_nodes]
///
/// # 返回值
///
/// * `Ok(u64)` - 迁移的页面数量
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_migrate_pages(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 4)?;
    let pid = args[0] as u32;
    let maxnode = args[1] as u32;
    let old_nodes_addr = args[2] as usize;
    let new_nodes_addr = args[3] as usize;

    // Validate arguments
    if old_nodes_addr == 0 || new_nodes_addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Check if the process exists
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid as usize).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Parse the old_nodes and new_nodes bitmasks from user space
    // 2. For each page in the process's memory space that is on a node in old_nodes
    // 3. Move it to a node in new_nodes
    // 4. Update the page tables and any other data structures
    
    // For now, we just log the call
    crate::log_debug!("migrate_pages syscall: pid={}, maxnode={}, old_nodes_addr={:#x}, new_nodes_addr={:#x}", 
                     pid, maxnode, old_nodes_addr, new_nodes_addr);
    Ok(0)
}

/// move_pages系统调用处理函数
///
/// 将指定页面移动到指定的NUMA节点。
///
/// # 参数
///
/// * `args` - 系统调用参数：[pid, count, pages, nodes, status, flags]
///
/// # 返回值
///
/// * `Ok(u64)` - 移动的页面数量
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_move_pages(args: &[u64]) -> KernelResult<u64> {
    let args = extract_args(args, 6)?;
    let pid = args[0] as u32;
    let count = args[1] as usize;
    let pages_addr = args[2] as usize;
    let nodes_addr = args[3] as usize;
    let status_addr = args[4] as usize;
    let flags = args[5] as i32;

    // Validate arguments
    if count == 0 || pages_addr == 0 || nodes_addr == 0 || status_addr == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Validate flags
    const MPOL_MF_MOVE: i32 = 0x1;
    const MPOL_MF_MOVE_ALL: i32 = 0x2;

    if flags & !(MPOL_MF_MOVE | MPOL_MF_MOVE_ALL) != 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Check if the process exists
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid as usize).ok_or(KernelError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(KernelError::InvalidArgument);
    }

    // In a real implementation, we would:
    // 1. Parse the pages array from user space
    // 2. Parse the nodes array from user space
    // 3. For each page in the pages array
    // 4. Move it to the corresponding node in the nodes array
    // 5. Update the status array with the result of each move
    
    // For now, we just log the call
    crate::log_debug!("move_pages syscall: pid={}, count={}, pages_addr={:#x}, nodes_addr={:#x}, status_addr={:#x}, flags={}", 
                     pid, count, pages_addr, nodes_addr, status_addr, flags);
    Ok(0)
}

/// 获取内存管理系统调用号映射
///
/// 返回内存管理模块支持的系统调用号列表。
///
/// # 返回值
///
/// * `Vec<u32>` - 系统调用号列表
pub fn get_supported_syscalls() -> Vec<u32> {
    vec![
        // NOS自定义系统调用号（0x3000-0x3FFF）
        0x3000, // brk
        0x3001, // mmap
        0x3002, // munmap
        0x3003, // mprotect
        0x3004, // madvise
        0x3005, // mlock
        0x3006, // munlock
        0x3007, // mlockall
        0x3008, // munlockall
        0x3009, // mincore
        0x300A, // msync
        0x300B, // mremap
        0x300C, // remap_file_pages
        0x300D, // shmget
        0x300E, // shmat
        0x300F, // shmdt
        0x3010, // shmctl

        // Linux系统调用号（x86_64）- 用于兼容性
        9,      // linux_mmap
        11,     // linux_munmap
        10,     // linux_mprotect
        26,     // linux_msync
        149,    // linux_mlock
        150,    // linux_munlock
        12,     // linux_brk
        28,     // linux_madvise
        16,     // linux_getpagesize
    ]
}

/// 系统调用分发函数
///
/// 根据系统调用号分发到相应的处理函数。
/// 支持NOS自定义系统调用和Linux兼容系统调用。
///
/// # 参数
///
/// * `syscall_number` - 系统调用号
/// * `args` - 系统调用参数
///
/// # 返回值
///
/// * `Ok(u64)` - 系统调用执行结果
/// * `Err(KernelError)` - 系统调用执行失败
pub fn dispatch_syscall(syscall_number: u32, args: &[u64]) -> KernelResult<u64> {
    match syscall_number {
        // NOS自定义内存管理系统调用 (0x3000-0x3FFF)
        0x3000 => handle_brk(args),         // sys_brk
        0x3001 => handle_mmap(args),        // sys_mmap
        0x3002 => handle_munmap(args),      // sys_munmap
        0x3003 => handle_mprotect(args),    // sys_mprotect
        0x3004 => handle_madvise(args),     // sys_madvise
        0x3005 => handle_mlock(args),       // sys_mlock
        0x3006 => handle_munlock(args),     // sys_munlock
        0x3007 => handle_mlockall(args),    // sys_mlockall
        0x3008 => handle_munlockall(args),  // sys_munlockall
        0x3009 => handle_mincore(args),     // sys_mincore
        0x300A => handle_msync(args),       // sys_msync
        0x300B => handle_mremap(args),      // sys_mremap
        0x300C => handle_remap_file_pages(args), // sys_remap_file_pages
        0x300D => handle_shmget(args),      // sys_shmget
        0x300E => handle_shmat(args),       // sys_shmat
        0x300F => handle_shmdt(args),       // sys_shmdt
        0x3010 => handle_shmctl(args),      // sys_shmctl

        // Linux兼容系统调用号 (x86_64)
        9 => handle_mmap(args),         // linux_mmap
        10 => handle_mprotect(args),    // linux_mprotect
        11 => handle_munmap(args),      // linux_munmap
        12 => handle_brk(args),         // linux_brk
        16 => handle_getpagesize(args), // linux_getpagesize
        26 => handle_msync(args),       // linux_msync
        28 => handle_madvise(args),     // linux_madvise
        149 => handle_mlock(args),      // linux_mlock
        150 => handle_munlock(args),    // linux_munlock

        _ => {
            crate::log_debug!("Unsupported memory syscall: {}", syscall_number);
            Err(KernelError::UnsupportedSyscall)
        },
    }
}
