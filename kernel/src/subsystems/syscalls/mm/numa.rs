//! NUMA内存策略系统调用处理
//!
//! 实现NUMA相关内存管理系统调用

use crate::error::unified::KernelResult;
use crate::api::KernelError;
use crate::process::{PROC_TABLE, myproc};
use crate::subsystems::mm::vm::PAGE_SIZE;

use super::utils::extract_args;

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

    if start >= crate::subsystems::mm::vm::KERNEL_BASE || end > crate::subsystems::mm::vm::KERNEL_BASE {
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
        if addr == 0 || addr >= crate::subsystems::mm::vm::KERNEL_BASE {
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
        0x3000 => super::brk::handle_brk(args),         // sys_brk
        0x3001 => super::mmap::handle_mmap(args),        // sys_mmap
        0x3002 => super::munmap::handle_munmap(args),      // sys_munmap
        0x3003 => super::mprotect::handle_mprotect(args),    // sys_mprotect
        0x3004 => handle_madvise(args),     // sys_madvise
        0x3005 => super::mlock::handle_mlock(args),       // sys_mlock
        0x3006 => super::mlock::handle_munlock(args),     // sys_munlock
        0x3007 => super::mlock::handle_mlockall(args),    // sys_mlockall
        0x3008 => super::mlock::handle_munlockall(args),  // sys_munlockall
        0x3009 => handle_mincore(args),     // sys_mincore
        0x300A => super::mprotect::handle_msync(args),       // sys_msync
        0x300B => super::mprotect::handle_mremap(args),      // sys_mremap
        0x300C => super::mprotect::handle_remap_file_pages(args), // sys_remap_file_pages
        0x300D => super::shm::handle_shmget(args),      // sys_shmget
        0x300E => super::shm::handle_shmat(args),       // sys_shmat
        0x300F => super::shm::handle_shmdt(args),       // sys_shmdt
        0x3010 => super::shm::handle_shmctl(args),      // sys_shmctl

        // Linux兼容系统调用号 (x86_64)
        9 => super::mmap::handle_mmap(args),         // linux_mmap
        10 => super::mprotect::handle_mprotect(args),    // linux_mprotect
        11 => super::munmap::handle_munmap(args),      // linux_munmap
        12 => super::brk::handle_brk(args),         // linux_brk
        16 => super::brk::handle_getpagesize(args), // linux_getpagesize
        26 => super::mprotect::handle_msync(args),       // linux_msync
        28 => handle_madvise(args),     // linux_madvise
        149 => super::mlock::handle_mlock(args),      // linux_mlock
        150 => super::mlock::handle_munlock(args),    // linux_munlock

        _ => {
            crate::log_debug!("Unsupported memory syscall: {}", syscall_number);
            Err(KernelError::UnsupportedSyscall)
        },
    }
}
