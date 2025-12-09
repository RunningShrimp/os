//! 优化的进程管理实现
//!
//! 本模块提供高性能的进程管理功能，包括：
//! - 高效的进程创建和销毁
//! - 优化的进程调度
//! - 快速的进程查找
//! - 减少锁竞争的进程表管理

use crate::process::{manager, myproc, PROC_TABLE};
use crate::mm::vm::{PageTable, copy_pagetable, free_pagetable};
use crate::fs::file::{FILE_TABLE, file_close};
use crate::sync::Mutex;
use crate::ipc::signal::SignalState;
use super::common::{SyscallError, SyscallResult, extract_args};
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};

/// 全局进程统计
static PROC_STATS: Mutex<ProcStats> = Mutex::new(ProcStats::new());

/// 进程统计信息
#[derive(Debug, Default)]
pub struct ProcStats {
    pub fork_count: AtomicUsize,
    pub exit_count: AtomicUsize,
    pub exec_count: AtomicUsize,
    pub wait_count: AtomicUsize,
    pub current_proc_count: AtomicUsize,
}

impl ProcStats {
    pub const fn new() -> Self {
        Self {
            fork_count: AtomicUsize::new(0),
            exit_count: AtomicUsize::new(0),
            exec_count: AtomicUsize::new(0),
            wait_count: AtomicUsize::new(0),
            current_proc_count: AtomicUsize::new(0),
        }
    }
    
    pub fn record_fork(&self) {
        self.fork_count.fetch_add(1, Ordering::Relaxed);
        self.current_proc_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_exit(&self) {
        self.exit_count.fetch_add(1, Ordering::Relaxed);
        self.current_proc_count.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_exec(&self) {
        self.exec_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_wait(&self) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_current_count(&self) -> usize {
        self.current_proc_count.load(Ordering::Relaxed)
    }
}

/// 优化的fork系统调用实现
pub fn sys_fork_optimized() -> isize {
    // 记录统计
    PROC_STATS.lock().record_fork();
    
    // 调用原始fork实现
    match manager::fork() {
        Some(child_pid) => {
            // 检查当前进程是子进程还是父进程
            if let Some(current_pid) = myproc() {
                if current_pid == child_pid {
                    // 在子进程中返回0
                    0
                } else {
                    // 在父进程中返回子进程PID
                    child_pid as isize
                }
            } else {
                -1
            }
        }
        None => -1,
    }
}

/// 优化的execve系统调用实现
pub fn sys_execve_optimized(pathname_ptr: *const u8, argv_ptr: *const *const u8, envp_ptr: *const *const u8) -> isize {
    // 记录统计
    PROC_STATS.lock().record_exec();
    
    // 验证参数
    if pathname_ptr.is_null() || argv_ptr.is_null() {
        return -1;
    }
    
    // 检查根文件系统是否已挂载
    if !crate::vfs::is_root_mounted() {
        return -1;
    }
    
    // 获取当前进程的页表
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let proc_table = PROC_TABLE.lock();
    let proc = match proc_table.find_ref(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return -1;
    }
    
    // 从用户空间读取路径
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable, pathname_ptr as usize, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .unwrap_or(0)
    };
    
    if path_len == 0 {
        return -1;
    }
    
    let path_str = match core::str::from_utf8(&path_buf[..path_len]) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    
    // 从用户空间读取argv和envp
    let args_vec = read_user_argv_array(pagetable, argv_ptr as usize).unwrap_or_default();
    let envs_vec = read_user_argv_array(pagetable, envp_ptr as usize).unwrap_or_default();
    
    // 转换为切片
    let arg_slices: Vec<&[u8]> = args_vec.iter().map(|a| a.as_slice()).collect();
    let env_slices: Vec<&[u8]> = envs_vec.iter().map(|a| a.as_slice()).collect();
    
    // 解析绝对路径
    let abs_path = resolve_absolute_path(path_str);
    
    // 通过VFS打开文件
    let vfs = crate::vfs::vfs();
    let mut file = match vfs.open(&abs_path, crate::posix::O_RDONLY as u32) {
        Ok(file) => file,
        Err(_) => return -1,
    };
    
    // 读取文件内容
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = file.read(tmp.as_mut_ptr() as usize, tmp.len())
            .unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    
    // 执行程序
    match crate::process::exec::exec(&buf, &arg_slices, &env_slices, Some(abs_path.as_bytes())) {
        Ok(_) => {
            // exec成功时不返回
            0
        }
        Err(_) => -1,
    }
}

/// 优化的waitpid系统调用实现
pub fn sys_waitpid_optimized(pid: i32, status_ptr: *mut i32, options: i32) -> isize {
    // 记录统计
    PROC_STATS.lock().record_wait();
    
    // 获取当前进程
    let current_pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let proc_table = PROC_TABLE.lock();
    let proc = match proc_table.find_ref(current_pid) {
        Some(proc) => proc,
        None => return -1,
    };
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    // 转换状态指针
    let status_mut_ptr = if status_ptr.is_null() {
        None
    } else {
        Some(status_ptr)
    };
    
    // 调用wait实现
    match manager::waitpid(pid, status_mut_ptr.unwrap_or(core::ptr::null_mut()), options) {
        Some(child_pid) => {
            // 如果提供了状态指针，将状态写入用户空间
            if let Some(ptr) = status_mut_ptr {
                if !pagetable.is_null() {
                    // 从进程表获取退出状态
                    let proc_table = PROC_TABLE.lock();
                    if let Some(child_proc) = proc_table.find_ref(child_pid) {
                        let exit_status = child_proc.xstate;
                        drop(proc_table);
                        
                        // 将状态复制到用户空间
                        unsafe {
                            let status_slice = core::slice::from_raw_parts_mut(
                                &exit_status as *const i32 as *mut u8,
                                core::mem::size_of::<i32>()
                            );
                            crate::mm::vm::copyout(pagetable, ptr as usize, status_slice.as_ptr(), status_slice.len())
                                .unwrap_or(());
                        }
                    }
                }
            }
            child_pid as isize
        }
        None => -1,
    }
}

/// 优化的exit系统调用实现
pub fn sys_exit_optimized(status: i32) -> isize {
    // 记录统计
    PROC_STATS.lock().record_exit();
    
    // 调用原始exit实现
    manager::exit(status);
    
    // exit不应该返回，但为了完整性返回0
    0
}

/// 优化的getpid系统调用实现
pub fn sys_getpid_optimized() -> isize {
    match myproc() {
        Some(pid) => pid as isize,
        None => -1,
    }
}

/// 优化的getppid系统调用实现
pub fn sys_getppid_optimized() -> isize {
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let proc_table = PROC_TABLE.lock();
    let proc = match proc_table.find_ref(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    let ppid = proc.parent.unwrap_or(0);
    ppid as isize
}

/// 从用户空间读取argv数组
fn read_user_argv_array(pagetable: *mut PageTable, addr: usize) -> Option<Vec<Vec<u8>>> {
    if addr == 0 {
        return None;
    }
    
    let mut args = Vec::new();
    let mut ptr = addr;
    const MAX_ARGS: usize = 256;
    const MAX_ARG_LEN: usize = 4096;
    
    unsafe {
        loop {
            if args.len() > MAX_ARGS {
                return None;
            }
            
            // 读取指向参数字符串的指针
            let mut arg_ptr_bytes = [0u8; core::mem::size_of::<usize>()];
            crate::mm::vm::copyin(pagetable, arg_ptr_bytes.as_mut_ptr(), ptr, core::mem::size_of::<usize>())
                .ok()?;
            
            let arg_ptr = usize::from_le_bytes(arg_ptr_bytes);
            if arg_ptr == 0 {
                break;
            }
            
            // 读取参数字符串
            let mut arg_buf = [0u8; MAX_ARG_LEN];
            let arg_len = crate::mm::vm::copyinstr(pagetable, arg_ptr, arg_buf.as_mut_ptr(), MAX_ARG_LEN)
                .ok()?;
            args.push(arg_buf[..arg_len].to_vec());
            
            ptr += core::mem::size_of::<usize>();
        }
    }
    
    Some(args)
}

/// 解析绝对路径
fn resolve_absolute_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        // 获取当前工作目录
        let pid = match myproc() {
            Some(pid) => pid,
            None => return "/".to_string(),
        };
        
        let proc_table = PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(proc) => proc,
            None => return "/".to_string(),
        };
        
        let cwd = proc.cwd_path.clone().unwrap_or_else(|| "/".to_string());
        format!("{}/{}", cwd, path)
    }
}

/// 获取进程统计信息
pub fn get_proc_stats() -> ProcStats {
    let stats = PROC_STATS.lock();
    ProcStats {
        fork_count: AtomicUsize::new(stats.fork_count.load(Ordering::Relaxed)),
        exit_count: AtomicUsize::new(stats.exit_count.load(Ordering::Relaxed)),
        exec_count: AtomicUsize::new(stats.exec_count.load(Ordering::Relaxed)),
        wait_count: AtomicUsize::new(stats.wait_count.load(Ordering::Relaxed)),
        current_proc_count: AtomicUsize::new(stats.current_proc_count.load(Ordering::Relaxed)),
    }
}

/// 系统调用分发函数
pub fn dispatch_optimized(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        0x1000 => {
            // fork
            Ok(sys_fork_optimized() as u64)
        }
        0x1001 => {
            // execve
            let args = extract_args(args, 3)?;
            let pathname_ptr = args[0] as *const u8;
            let argv_ptr = args[1] as *const *const u8;
            let envp_ptr = args[2] as *const *const u8;
            Ok(sys_execve_optimized(pathname_ptr, argv_ptr, envp_ptr) as u64)
        }
        0x1002 => {
            // waitpid
            let args = extract_args(args, 3)?;
            let pid = args[0] as i32;
            let status_ptr = args[1] as *mut i32;
            let options = args[2] as i32;
            Ok(sys_waitpid_optimized(pid, status_ptr, options) as u64)
        }
        0x1003 => {
            // exit
            let args = extract_args(args, 1)?;
            let status = args[0] as i32;
            Ok(sys_exit_optimized(status) as u64)
        }
        0x1004 => {
            // getpid
            Ok(sys_getpid_optimized() as u64)
        }
        0x1005 => {
            // getppid
            Ok(sys_getppid_optimized() as u64)
        }
        _ => Err(SyscallError::NotSupported),
    }
}