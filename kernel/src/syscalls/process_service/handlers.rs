//! 进程系统调用处理函数
//! 
//! 本模块包含进程相关系统调用的具体实现逻辑，包括：
//! - 进程创建和终止
//! - 进程状态查询
//! - 进程调度和控制
//! - 进程间同步操作

use crate::error_handling::unified::KernelError;
use crate::syscalls::process::types::*;
use alloc::vec::Vec;

/// fork系统调用处理函数
/// 
/// 创建当前进程的副本。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数（通常为空）
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 子进程ID（父进程中）或0（子进程中）
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_fork(args: &[u64]) -> Result<u64, KernelError> {
    if !args.is_empty() {
        return Err(KernelError::InvalidArgument);
    }

    // 调用fork函数创建子进程
    match crate::process::fork() {
        Some(child_pid) => {
            crate::log_debug!("fork syscall called, child PID: {}", child_pid);
            Ok(child_pid as u64)
        },
        None => {
            crate::log_debug!("fork syscall failed");
            Err(KernelError::ResourceExhausted)
        }
    }
}

/// execve系统调用处理函数
/// 
/// 执行新的程序替换当前进程映像。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[程序路径指针, 参数数组指针, 环境变量指针]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 成功时不返回
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_execve(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 3 {
        return Err(KernelError::InvalidArgument);
    }

    let path_ptr = args[0];
    let argv_ptr = args[1];
    let envp_ptr = args[2];

    // TODO: 实现execve逻辑
    crate::log_debug!("execve syscall called: path_ptr={:#x}, argv_ptr={:#x}, envp_ptr={:#x}", 
                path_ptr, argv_ptr, envp_ptr);
    
    // 临时返回值
    Ok(0)
}

/// waitpid系统调用处理函数
/// 
/// 等待进程状态改变。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[pid, status_ptr, options]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 子进程ID
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_waitpid(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 3 {
        return Err(KernelError::InvalidArgument);
    }

    let pid = args[0] as i32;
    let status_ptr = args[1];
    let options = args[2] as i32;

    // TODO: 实现waitpid逻辑
    crate::log_debug!("waitpid syscall called: pid={}, status_ptr={:#x}, options={}", 
                pid, status_ptr, options);
    
    // 临时返回值
    Ok(0)
}

/// exit系统调用处理函数
/// 
/// 终止当前进程。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[exit_code]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 成功时不返回
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_exit(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let exit_code = args[0] as i32;

    // 调用进程退出函数
    crate::log_debug!("exit syscall called: exit_code={}", exit_code);
    crate::process::exit(exit_code);
    
    // 不应该返回
    unreachable!("Process should have exited");
}

/// kill系统调用处理函数
/// 
/// 向进程发送信号。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[pid, signal]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_kill(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let pid = args[0] as i32;
    let signal = args[1] as i32;

    // TODO: 实现kill逻辑
    crate::log_debug!("kill syscall called: pid={}, signal={}", pid, signal);
    
    // 临时返回值
    Ok(0)
}

/// getpid系统调用处理函数
/// 
/// 获取当前进程ID。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数（通常为空）
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 当前进程ID
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_getpid(args: &[u64]) -> Result<u64, KernelError> {
    if !args.is_empty() {
        return Err(KernelError::InvalidArgument);
    }

    // 获取当前进程ID
    let pid = crate::process::getpid();
    crate::log_debug!("getpid syscall called, returning PID: {}", pid);
    
    Ok(pid as u64)
}

/// getppid系统调用处理函数
/// 
/// 获取父进程ID。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数（通常为空）
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 父进程ID
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_getppid(args: &[u64]) -> Result<u64, KernelError> {
    if !args.is_empty() {
        return Err(KernelError::InvalidArgument);
    }

    // 获取当前进程ID
    let current_pid = match crate::process::myproc() {
        Some(pid) => pid,
        None => return Ok(0), // 没有当前进程时返回0
    };

    // 获取父进程ID
    let ppid = {
        let table = crate::process::PROC_TABLE.lock();
        table.find_ref(current_pid)
            .and_then(|proc| proc.parent)
            .unwrap_or(0) // init进程没有父进程，返回0
    };

    crate::log_debug!("getppid syscall called, returning PPID: {}", ppid);
    
    Ok(ppid as u64)
}

/// sched_yield系统调用处理函数
/// 
/// 让出CPU给其他进程。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数（通常为空）
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 0表示成功
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_sched_yield(args: &[u64]) -> Result<u64, KernelError> {
    if !args.is_empty() {
        return Err(KernelError::InvalidArgument);
    }

    // TODO: 实现sched_yield逻辑
    crate::log_debug!("sched_yield syscall called");
    
    // 临时返回值
    Ok(0)
}

/// nice系统调用处理函数
/// 
/// 设置进程优先级。
/// 
/// # 参数
/// 
/// * `args` - 系统调用参数：[inc]
/// 
/// # 返回值
/// 
/// * `Ok(u64)` - 新的优先级值
/// * `Err(KernelError)` - 系统调用执行失败
pub fn handle_nice(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let inc = args[0] as i32;

    // TODO: 实现nice逻辑
    crate::log_debug!("nice syscall called: inc={}", inc);
    
    // 临时返回值
    Ok(0)
}

/// 获取进程系统调用号映射
/// 
/// 返回进程模块支持的系统调用号列表。
/// 
/// # 返回值
/// 
/// * `Vec<u32>` - 系统调用号列表
pub fn get_supported_syscalls() -> Vec<u32> {
    vec![
        // Linux系统调用号（x86_64）
        57, // fork
        59, // execve
        60, // exit
        61, // waitpid
        62, // kill
        39, // getpid
        110, // getppid
        24, // sched_yield
        83, // nice
    ]
}

/// 系统调用分发函数
/// 
/// 根据系统调用号分发到相应的处理函数。
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
pub fn dispatch_syscall(syscall_number: u32, args: &[u64]) -> Result<u64, KernelError> {
    match syscall_number {
        57 => handle_fork(args),
        59 => handle_execve(args),
        60 => handle_exit(args),
        61 => handle_waitpid(args),
        62 => handle_kill(args),
        39 => handle_getpid(args),
        110 => handle_getppid(args),
        24 => handle_sched_yield(args),
        83 => handle_nice(args),
        _ => Err(KernelError::UnsupportedSyscall),
    }
}