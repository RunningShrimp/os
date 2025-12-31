//! munmap系统调用处理
//!
//! 实现取消内存映射相关系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::{PAGE_SIZE, flush_tlb_page};
use crate::subsystems::mm::{kfree};

use super::utils::extract_args;

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

    // For each page in range, unmap it and free physical memory
    let mut current = start;
    let mut unmapped_count = 0;

    while current < end {
        // Try to unmap the page and get physical address
        #[cfg(target_arch = "riscv64")]
        {
            use crate::subsystems::mm::vm::riscv64;
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
                if crate::subsystems::mm::vm::unmap_page(pagetable, current).is_ok() {
                    // Note: For aarch64, we would need to track physical addresses
                    // separately. For now, we just unmap without freeing physical memory.
                    // GH-#1342: Implement proper physical page tracking for aarch64
                    // See: https://github.com/npos/kernel/issues/1342
                    unmapped_count += 1;
                }
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            // x86_64 implementation would go here
            // For now, just increment count
            // GH-#1343: Implement proper unmapping for x86_64
            // See: https://github.com/npos/kernel/issues/1343
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
