//! madvise系统调用处理
//!
//! 实现内存建议相关系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::PAGE_SIZE;

use super::utils::extract_args;

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

    if start >= crate::subsystems::mm::vm::KERNEL_BASE || end > crate::subsystems::mm::vm::KERNEL_BASE {
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

    // In a real implementation, we would:
    // 1. Check if vec_addr is a valid user address
    // 2. For each page in range, check if it's resident in memory
    // 3. Set the corresponding bit in the vector

    // For now, we just log the call
    crate::log_debug!("mincore syscall: addr={:#x}, length={}, vec_addr={:#x}, page_count={}",
                     addr, length, vec_addr, page_count);
    Ok(0)
}
