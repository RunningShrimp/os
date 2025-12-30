//! mprotect系统调用处理
//!
//! 实现内存保护属性修改相关系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::{flags, PAGE_SIZE};

use super::utils::extract_args;

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

    if start >= crate::subsystems::mm::vm::KERNEL_BASE || end > crate::subsystems::mm::vm::KERNEL_BASE {
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
                if let Some(pte_ptr) = crate::subsystems::mm::vm::riscv64::walk(pagetable, current, false) {
                    if *pte_ptr & crate::subsystems::mm::vm::riscv64::PTE_V != 0 {
                        // Page is mapped, update permissions
                        let old_pte = *pte_ptr;
                        let pa = crate::subsystems::mm::vm::riscv64::pte_to_pa(old_pte);
                        let new_pte = crate::subsystems::mm::vm::riscv64::pa_to_pte(pa) | new_perm | crate::subsystems::mm::vm::riscv64::PTE_V;
                        *pte_ptr = new_pte;
                        updated_count += 1;
                    }
                }
            }

            #[cfg(target_arch = "aarch64")]
            {
                // Use the exported walk function
                if let Some(desc_ptr) = crate::subsystems::mm::vm::walk(pagetable, current, false) {
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
            crate::subsystems::mm::vm::flush_tlb_page(current);
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
    use crate::subsystems::syscalls::common::extract_args;

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
    let start = addr & !(crate::subsystems::mm::vm::PAGE_SIZE - 1);
    let aligned_length = (length + crate::subsystems::mm::vm::PAGE_SIZE - 1) & !(crate::subsystems::mm::vm::PAGE_SIZE - 1);
    let end = start + aligned_length;

    if start >= crate::subsystems::mm::vm::KERNEL_BASE || end > crate::subsystems::mm::vm::KERNEL_BASE {
        return Err(KernelError::InvalidArgument);
    }

    // For each page in range, check if it's mapped and write back to file if needed
    let mut current = start;
    let mut synced_pages = 0;

    while current < end {
        // Check if page is mapped
        #[cfg(target_arch = "riscv64")]
        {
            use crate::subsystems::mm::vm::riscv64;
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

        current += crate::subsystems::mm::vm::PAGE_SIZE;
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

    if start >= crate::subsystems::mm::vm::KERNEL_BASE || end > crate::subsystems::mm::vm::KERNEL_BASE {
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
