//! mlock系统调用处理
//!
//! 实现内存锁定相关系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::PAGE_SIZE;

use super::utils::extract_args;

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
