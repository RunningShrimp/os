//! brk系统调用处理
//!
//! 实现堆内存管理相关系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::{flags, PAGE_SIZE, map_page};
use crate::subsystems::mm::{kalloc, kfree};
use core::ptr;

use super::utils::extract_args;

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
    if addr >= crate::subsystems::mm::vm::KERNEL_BASE {
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
        if new_end >= crate::subsystems::mm::vm::KERNEL_BASE {
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
    let page_size = crate::subsystems::syscalls::mm::types::PageSize::Size4K as u64;

    crate::log_debug!("getpagesize syscall: returning {}", page_size);
    Ok(page_size)
}
