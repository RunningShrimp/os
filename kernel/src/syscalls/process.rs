//! Process-related system calls
//!
//! Implements fork, exit, wait, kill, getpid, sbrk, sleep, uptime, exec, execve

use crate::process;
use super::{E_OK, E_NOMEM, E_BADARG, E_INVAL};

/// Fork the current process
pub fn sys_fork() -> isize {
    match process::fork() {
        Some(pid) => pid as isize,
        None => E_NOMEM,
    }
}

/// Exit the current process with status
pub fn sys_exit(status: i32) -> isize {
    process::exit(status);
    0 // Never reached
}

/// Wait for a child process to exit
pub fn sys_wait(status: *mut i32) -> isize {
    match process::wait(status) {
        Some(pid) => pid as isize,
        None => E_BADARG,
    }
}

/// Send signal to a process (currently just kills it)
pub fn sys_kill(pid: usize) -> isize {
    if process::kill(pid) {
        E_OK
    } else {
        E_BADARG
    }
}

/// Get current process ID
pub fn sys_getpid() -> isize {
    process::getpid() as isize
}

/// Adjust process data segment size
pub fn sys_sbrk(increment: isize) -> isize {
    if let Some(pid) = process::myproc() {
        let mut ptable = crate::process::PROC_TABLE.lock();
        if let Some(proc) = ptable.find(pid) {
            let old = proc.sz as isize;
            let new = old + increment;
            if new < 0 { return E_INVAL; }
            proc.sz = new as usize;
            return old;
        }
    }
    E_BADARG
}

/// Sleep for specified number of ticks
pub fn sys_sleep(ticks: usize) -> isize {
    if ticks == 0 {
        return E_OK;
    }
    
    let target = crate::time::get_ticks() + ticks as u64;
    let chan = process::myproc().unwrap_or(0) | 0x1000_0000; // Sleep channel
    
    crate::time::add_sleeper(target, chan);
    process::sleep(chan);
    
    E_OK
}

/// Get system uptime in ticks
pub fn sys_uptime() -> isize {
    crate::time::get_ticks() as isize
}

/// Execute a program (xv6-style)
pub fn sys_exec(path: *const u8, argv: *const *const u8) -> isize {
    crate::exec::sys_exec(path as usize, argv as usize)
}

/// Execute a program with environment (POSIX-style)
pub fn sys_execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    let p = path as usize;
    let a = argv as usize;
    let e = envp as usize;
    crate::exec::sys_execve(p, a, e)
}

/// Memory map - Not fully implemented yet
pub fn sys_mmap(addr: *mut u8, len: usize, prot: u32, flags: u32, fd: i32, offset: u64) -> isize {
    // TODO: Implement proper memory mapping
    // For now, just return error
    E_NOSYS
}

/// Memory unmap - Not fully implemented yet
pub fn sys_munmap(addr: *mut u8, len: usize) -> isize {
    // TODO: Implement proper memory unmapping
    // For now, just return error
    E_NOSYS
}

/// Set process group ID
pub fn sys_setpgid(pid: i32, pgid: i32) -> isize {
    let current_pid = process::myproc().unwrap_or(0) as i32;
    
    // If pid is 0, use current process
    let target_pid = if pid == 0 { current_pid } else { pid };
    
    // If pgid is 0, use pid
    let target_pgid = if pgid == 0 { target_pid } else { pgid };
    
    // Only allow setting pgid of current process or its children
    if target_pid != current_pid {
        // Check if target_pid is a child of current process
        let mut ptable = crate::process::PROC_TABLE.lock();
        if let Some(target_proc) = ptable.find(target_pid as usize) {
            if target_proc.parent != Some(current_pid as usize) {
                return E_INVAL;
            }
        } else {
            return E_INVAL;
        }
    }
    
    // Set pgid
    let mut ptable = crate::process::PROC_TABLE.lock();
    if let Some(target_proc) = ptable.find(target_pid as usize) {
        target_proc.pgid = target_pgid as usize;
        return E_OK;
    }
    
    E_INVAL
}

/// Get process group ID
pub fn sys_getpgid(pid: i32) -> isize {
    let current_pid = process::myproc().unwrap_or(0) as i32;
    let target_pid = if pid == 0 { current_pid } else { pid };
    
    let ptable = crate::process::PROC_TABLE.lock();
    if let Some(target_proc) = ptable.find_ref(target_pid as usize) {
        return target_proc.pgid as isize;
    }
    
    E_INVAL
}

/// Set session ID
pub fn sys_setsid() -> isize {
    let pid = process::myproc().unwrap_or(0);
    
    let mut ptable = crate::process::PROC_TABLE.lock();
    if let Some(proc) = ptable.find(pid) {
        // Check if process is already a session leader
        if proc.pgid == pid {
            return E_INVAL;
        }
        
        // Create new session
        proc.sid = pid;
        proc.pgid = pid;
        return pid as isize;
    }
    
    E_INVAL
}

/// Get session ID
pub fn sys_getsid(pid: i32) -> isize {
    let current_pid = process::myproc().unwrap_or(0) as i32;
    let target_pid = if pid == 0 { current_pid } else { pid };
    
    let ptable = crate::process::PROC_TABLE.lock();
    if let Some(target_proc) = ptable.find_ref(target_pid as usize) {
        return target_proc.sid as isize;
    }
    
    E_INVAL
}


/// Get resource limit
pub fn sys_getrlimit(resource: i32, rlim: *mut crate::posix::Rlimit) -> isize {
    if rlim.is_null() {
        return E_INVAL;
    }
    
    if resource < 0 || resource >= 16 {
        return E_INVAL;
    }
    
    let pid = process::myproc().unwrap_or(0);
    let ptable = crate::process::PROC_TABLE.lock();
    if let Some(proc) = ptable.find_ref(pid) {
        let rlimit = proc.rlimits[resource as usize];
        unsafe { *rlim = rlimit; }
        return E_OK;
    }
    
    E_INVAL
}

/// Set resource limit
pub fn sys_setrlimit(resource: i32, rlim: *const crate::posix::Rlimit) -> isize {
    if rlim.is_null() {
        return E_INVAL;
    }
    
    if resource < 0 || resource >= 16 {
        return E_INVAL;
    }
    
    let new_rlim = unsafe { *rlim };
    if new_rlim.rlim_cur > new_rlim.rlim_max {
        return E_INVAL;
    }
    
    let pid = process::myproc().unwrap_or(0);
    let mut ptable = crate::process::PROC_TABLE.lock();
    if let Some(proc) = ptable.find(pid) {
        proc.rlimits[resource as usize] = new_rlim;
        return E_OK;
    }
    
    E_INVAL
}