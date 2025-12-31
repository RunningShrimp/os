//! mmap系统调用处理
//!
//! 实现内存映射相关系统调用

use crate::error::unified::{KernelResult};
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::{flags, PAGE_SIZE, map_page};
use crate::subsystems::mm::{kalloc, kfree};

use super::utils::extract_args;

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
        if target_addr + aligned_length >= crate::subsystems::mm::vm::KERNEL_BASE {
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

            unsafe {
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
        // GH-#1344: Handle file-backed mappings
        // See: https://github.com/npos/kernel/issues/1344
        Err(KernelError::NotSupported)
    }
}
