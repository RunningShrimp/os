//! 进程管理相关系统调用

use crate::process;

/// fork 系统调用
pub fn sys_fork() -> isize {
    // TODO: 实现 fork
    -1
}

/// exit 系统调用
pub fn sys_exit(status: i32) -> isize {
    // TODO: 实现 exit
    -1
}

/// wait 系统调用
pub fn sys_wait(status: *mut i32) -> isize {
    // TODO: 实现 wait
    -1
}

/// exec 系统调用
pub fn sys_exec(path: *const u8, argv: *const *const u8) -> isize {
    // TODO: 实现 exec
    -1
}

/// execve 系统调用
pub fn sys_execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    // TODO: 实现 execve
    -1
}

/// kill 系统调用
pub fn sys_kill(pid: usize) -> isize {
    // TODO: 实现 kill
    -1
}

/// getpid 系统调用
pub fn sys_getpid() -> isize {
    // TODO: 实现 getpid
    -1
}

/// sbrk 系统调用
pub fn sys_sbrk(increment: isize) -> isize {
    // TODO: 实现 sbrk
    -1
}

/// sleep 系统调用
pub fn sys_sleep(ticks: usize) -> isize {
    // TODO: 实现 sleep
    -1
}

/// uptime 系统调用
pub fn sys_uptime() -> isize {
    // TODO: 实现 uptime
    -1
}
