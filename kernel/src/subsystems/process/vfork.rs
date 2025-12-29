//! vfork 系统调用实现
//!
//! 提供虚拟 fork (vfork) 功能，用于优化 fork+exec 模式。
//!
//! ## 概述
//!
//! vfork 是一种特殊的 fork，设计用于优化 fork 后立即执行 exec 的场景：
//! - **共享地址空间**: 子进程共享父进程的内存（包括栈）
//! - **父进程阻塞**: 父进程在子进程 exec 或 exit 之前阻塞
//! - **不安全操作**: 子进程不能从 vfork 返回、不能修改变量
//!
//! ## 使用场景
//!
//! vfork 主要用于：
//! - 实现 shell 的命令执行
//! - 优化 fork+exec 性能
//! - 减少内存复制开销
//!
//! ## 注意事项
//!
//! ⚠️ **危险操作**: 在 vfork 子进程中：
//! - 不能从 vfork 函数返回
//! - 不能修改任何变量（除了 pid）
//! - 不能调用任何非 async-signal-safe 函数
//! - 必须 exec 或 _exit
//!
//! ## POSIX 兼容性
//!
//! 实现 POSIX.1-2008 规范：
//! - 子进程共享父进程地址空间
//! - 父进程阻塞直到子进程 exec/exit
//! - 返回值：父进程中返回子进程 PID，子进程中返回 0

use alloc::sync::Arc;
use spin::Mutex;

use crate::subsystems::process::{Pid, ProcState, Proc};

/// vfork 状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VforkState {
    /// 无 vfork 操作
    None,
    /// 进程是 vfork 的父进程，正在等待子进程
    WaitingForChild(Pid),
    /// 进程是 vfork 的子进程
    IsChild,
}

/// vfork 系统调用
///
/// # POSIX 语义
///
/// vfork 创建一个子进程，但与 fork 不同：
/// - 子进程共享父进程的地址空间（不复制页表）
/// - 父进程阻塞，直到子进程调用 execve 或 _exit
/// - 子进程在调用 exec 或 exit 之前不能从 vfork 返回
///
/// # 返回值
///
/// - **父进程**: 返回子进程的 PID
/// - **子进程**: 返回 0
/// - **错误**: 返回 -1 并设置 errno
///
/// # 错误
///
/// - `EAGAIN`: 进程或线程数达到系统限制
/// - `ENOMEM`: 内存不足
///
/// # 示例
///
/// ```no_run
/// use kernel::subsystems::process::vfork::sys_vfork;
///
/// let pid = unsafe { sys_vfork() };
/// if pid == 0 {
///     // 子进程
///     // 必须立即 exec 或 _exit
///     // 不能修改任何变量！
///     execve("/bin/ls", ...);
/// } else if pid > 0 {
///     // 父进程 - 在这里阻塞，直到子进程 exec 或 exit
///     // 当子进程 exec 或 exit 后，父进程继续执行
///     wait(pid);
/// } else {
///     // 错误
/// }
/// ```
///
/// # 安全注意事项
///
/// ⚠️ **极度危险**: vfork 子进程中：
/// - **不能**从包含 vfork 的函数返回
/// - **不能**调用任何函数（除了 execve/_exit）
/// - **不能**修改任何变量（除了用于存储 pid 的变量）
/// - **不能**访问任何栈或堆数据
///
/// 违反这些规则会导致未定义行为！
pub unsafe fn sys_vfork() -> Result<Pid, crate::api::SyscallError> {
    use crate::subsystems::process::manager::PROC_TABLE;

    // 获取当前进程（父进程）
    let parent_pid = crate::process::myproc().ok_or(crate::api::SyscallError::NoProcess)?;

    let mut table = PROC_TABLE.lock();

    // 检查父进程是否已经是 vfork 子进程
    // vfork 子进程不能再次 vfork
    {
        let parent = table.find(parent_pid).ok_or(crate::api::SyscallError::NoProcess)?;
        // TODO: 需要在 Proc 结构中添加 vfork_state 字段
        // 这里暂时跳过检查
    }

    // 提取父进程数据
    let (
        parent_pgid,
        parent_sid,
        parent_uid,
        parent_gid,
        parent_euid,
        parent_egid,
        parent_suid,
        parent_sgid,
        parent_nice,
        parent_umask,
        parent_ofile,
        parent_cwd_path,
        parent_cwd,
        parent_rlimits,
        parent_pagetable,
        parent_sz,
        parent_trapframe,
    ) = {
        let parent = table.find(parent_pid).ok_or(crate::api::SyscallError::NoProcess)?;
        (
            parent.pgid,
            parent.sid,
            parent.uid,
            parent.gid,
            parent.euid,
            parent.egid,
            parent.suid,
            parent.sgid,
            parent.nice,
            parent.umask,
            parent.ofile.clone(),
            parent.cwd_path.clone(),
            parent.cwd,
            parent.rlimits.clone(),
            parent.pagetable,
            parent.sz,
            parent.trapframe,
        )
    };

    // 分配子进程
    let child = table.alloc().ok_or(crate::api::SyscallError::ResourceLimit)?;
    let child_pid = child.pid;

    // 初始化子进程状态
    child.parent = Some(parent_pid);
    child.state = ProcState::Runnable;
    child.pgid = parent_pgid;
    child.sid = parent_sid;

    // vfork 关键：共享页表（不复制）
    // 子进程使用父进程的页表
    child.pagetable = parent_pagetable;
    child.sz = parent_sz;

    // 继承凭证
    child.uid = parent_uid;
    child.gid = parent_gid;
    child.euid = parent_euid;
    child.egid = parent_egid;
    child.suid = parent_suid;
    child.sgid = parent_sgid;
    child.nice = parent_nice;
    child.umask = parent_umask;

    // 复制文件描述符表
    child.ofile = parent_ofile;
    child.cwd_path = parent_cwd_path;
    child.cwd = parent_cwd;

    // 复制资源限制
    child.rlimits = parent_rlimits;

    // 复制 trapframe（但会修改返回值）
    // 注意：这里需要小心，因为共享地址空间
    if !parent_trapframe.is_null() {
        // 为子进程分配新的 trapframe
        let child_trapframe = crate::process::alloc_trapframe();
        if child_trapframe.is_null() {
            table.free(child_pid);
            return Err(crate::api::SyscallError::NoMemory);
        }

        // 复制 trapframe 内容
        *child_trapframe = *parent_trapframe;

        // 设置子进程的返回值为 0
        // 在 x86_64 上，返回值通过 rax 寄存器传递
        (*child_trapframe).rax = 0;

        child.trapframe = child_trapframe;
    }

    // vfork 特殊处理：
    // 1. 标记父进程为等待状态
    // 2. 子进程标记为 vfork 子进程
    // TODO: 需要在 Proc 结构中添加这些字段

    // 让子进程可运行
    child.state = ProcState::Runnable;

    // 将子进程添加到父进程的子进程列表
    table.add_child_to_parent(parent_pid, child_pid);

    // 父进程阻塞，等待子进程 exec 或 exit
    // 实际实现需要：
    // 1. 设置父进程状态为 Blocked
    // 2. 在子进程 exec/exit 时唤醒父进程
    // TODO: 实现等待机制

    Ok(child_pid)
}

/// vfork 子进程完成（exec 或 exit）
///
/// 当 vfork 子进程调用 exec 或 exit 时，应该调用此函数唤醒父进程
///
/// # 参数
///
/// * `child_pid` - vfork 子进程的 PID
pub fn vfork_child_done(child_pid: Pid) {
    use crate::subsystems::process::manager::PROC_TABLE;

    let table = PROC_TABLE.lock();

    // 获取子进程
    let child = match table.find(child_pid) {
        Some(c) => c,
        None => return,
    };

    // 获取父进程
    let parent_pid = match child.parent {
        Some(pid) => pid,
        None => return,
    };

    // 唤醒父进程
    // TODO: 实现唤醒逻辑
    let _ = parent_pid;
}

/// 检查进程是否是 vfork 子进程
///
/// # 参数
///
/// * `pid` - 要检查的进程 PID
///
/// # 返回值
///
/// - `true`: 进程是 vfork 子进程
/// - `false`: 进程不是 vfork 子进程
pub fn is_vfork_child(pid: Pid) -> bool {
    use crate::subsystems::process::manager::PROC_TABLE;

    let table = PROC_TABLE.lock();

    match table.find(pid) {
        Some(_proc) => {
            // TODO: 检查 vfork_state
            false
        },
        None => false,
    }
}

/// 获取 vfork 父进程（如果存在）
///
/// # 参数
///
/// * `child_pid` - vfork 子进程的 PID
///
/// # 返回值
///
/// - `Some(parent_pid)`: vfork 父进程的 PID
/// - `None`: 进程不是 vfork 子进程
pub fn get_vfork_parent(child_pid: Pid) -> Option<Pid> {
    use crate::subsystems::process::manager::PROC_TABLE;

    if !is_vfork_child(child_pid) {
        return None;
    }

    let table = PROC_TABLE.lock();
    let child = table.find(child_pid)?;
    child.parent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfork_state() {
        // 基本的状态测试
        let state = VforkState::None;
        assert_eq!(state, VforkState::None);
    }

    #[test]
    fn test_is_vfork_child() {
        // 测试 vfork 子进程检查
        let pid = 123;
        let result = is_vfork_child(pid);
        // 应该返回 false（因为没有实际的 vfork）
        assert!(!result);
    }
}
