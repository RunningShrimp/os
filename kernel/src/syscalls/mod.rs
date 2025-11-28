//! System call dispatcher and common definitions
//! 
//! This module provides the syscall entry point and dispatches to specific
//! syscall implementations organized by category.

extern crate alloc;

use crate::process::TrapFrame;
use crate::errno;
use crate::posix;

mod process;
mod file_io;
mod fs;
mod pipe;
mod signal;
mod time;
mod socket;

use time::*;
use socket::*;

// Re-export all syscall implementations for internal use
use process::*;
use file_io::*;
use fs::*;
use pipe::*;
use signal::*;

// ============================================================================
// System call numbers (xv6-compatible)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum SysNum {
    Fork = 1,
    Exit = 2,
    Wait = 3,
    Pipe = 4,
    Read = 5,
    Kill = 6,
    Exec = 7,
    Fstat = 8,
    Chdir = 9,
    Dup = 10,
    Getpid = 11,
    Sbrk = 12,
    Sleep = 13,
    Uptime = 14,
    Open = 15,
    Write = 16,
    Mknod = 17,
    Unlink = 18,
    Link = 19,
    Mkdir = 20,
    Close = 21,
    // Extended syscalls
    Fcntl = 22,
    Poll = 23,
    Select = 24,
    Lseek = 25,
    Dup2 = 26,
    Getcwd = 27,
    Rmdir = 28,
    // Signal syscalls
    Sigaction = 30,
    Sigprocmask = 31,
    Sigsuspend = 32,
    Sigpending = 33,
    // Execve with environment
    Execve = 44,
    // Memory mapping
    Mmap = 9,
    Munmap = 11,
    // Time
    GetTimeOfDay = 228,
    ClockGetTime = 229,
    
    // Additional file operations (P2-003)
    Ftruncate = 45,
    Fchmod = 90,
    Fchown = 92,
    
    // sendfile and splice syscalls (P2-002)
    Sendfile = 401,
    Splice = 402,
    
    // Symlink syscalls (P2-006)
    Symlink = 83,
    Readlink = 89,
    
    // Process group and session syscalls (P2-008)
    Setpgid = 57,
    Getpgid = 58,
    Setsid = 59,
    Getsid = 64,
    
    
    // Socket syscalls (P2-001)
    Socket = 41,
    Bind = 49,
    Listen = 50,
    Accept = 43,
    Connect = 42,
    Send = 40,
    Recv = 39,
    Sendto = 44,
    Recvfrom = 45,
    Shutdown = 48,
    Setsockopt = 54,
    Getsockopt = 55,
    
    // epoll syscalls (P2-004)
    EpollCreate = 213,
    EpollCtl = 233,
    EpollWait = 232,
}

impl TryFrom<usize> for SysNum {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SysNum::Fork),
            2 => Ok(SysNum::Exit),
            3 => Ok(SysNum::Wait),
            4 => Ok(SysNum::Pipe),
            5 => Ok(SysNum::Read),
            6 => Ok(SysNum::Kill),
            7 => Ok(SysNum::Exec),
            8 => Ok(SysNum::Fstat),
            9 => Ok(SysNum::Chdir),
            10 => Ok(SysNum::Dup),
            11 => Ok(SysNum::Getpid),
            12 => Ok(SysNum::Sbrk),
            13 => Ok(SysNum::Sleep),
            14 => Ok(SysNum::Uptime),
            15 => Ok(SysNum::Open),
            16 => Ok(SysNum::Write),
            17 => Ok(SysNum::Mknod),
            18 => Ok(SysNum::Unlink),
            19 => Ok(SysNum::Link),
            20 => Ok(SysNum::Mkdir),
            21 => Ok(SysNum::Close),
            22 => Ok(SysNum::Fcntl),
            23 => Ok(SysNum::Poll),
            24 => Ok(SysNum::Select),
            25 => Ok(SysNum::Lseek),
            26 => Ok(SysNum::Dup2),
            27 => Ok(SysNum::Getcwd),
            28 => Ok(SysNum::Rmdir),
            30 => Ok(SysNum::Sigaction),
            31 => Ok(SysNum::Sigprocmask),
            32 => Ok(SysNum::Sigsuspend),
            33 => Ok(SysNum::Sigpending),
            44 => Ok(SysNum::Execve),
            57 => Ok(SysNum::Setpgid),
            58 => Ok(SysNum::Getpgid),
            59 => Ok(SysNum::Setsid),
            64 => Ok(SysNum::Getsid),
            73 => Ok(SysNum::Getrlimit),
            75 => Ok(SysNum::Setrlimit),
            59 => Ok(SysNum::Setsid),
            64 => Ok(SysNum::Getsid),
            9 => Ok(SysNum::Mmap),
            11 => Ok(SysNum::Munmap),
            228 => Ok(SysNum::GetTimeOfDay),
            229 => Ok(SysNum::ClockGetTime),
            45 => Ok(SysNum::Ftruncate),
            90 => Ok(SysNum::Fchmod),
            92 => Ok(SysNum::Fchown),
            401 => Ok(SysNum::Sendfile),
            402 => Ok(SysNum::Splice),
            41 => Ok(SysNum::Socket),
            49 => Ok(SysNum::Bind),
            50 => Ok(SysNum::Listen),
            43 => Ok(SysNum::Accept),
            42 => Ok(SysNum::Connect),
            40 => Ok(SysNum::Send),
            39 => Ok(SysNum::Recv),
            54 => Ok(SysNum::Setsockopt),
            55 => Ok(SysNum::Getsockopt),
            213 => Ok(SysNum::EpollCreate),
            233 => Ok(SysNum::EpollCtl),
            232 => Ok(SysNum::EpollWait),
            _ => Err(()),
        }
    }
}

// ============================================================================
// Error codes (negated errno values)
// ============================================================================

pub const E_OK: isize = 0;
pub const E_BADARG: isize = errno::errno_neg(errno::EINVAL);
pub const E_NOMEM: isize = errno::errno_neg(errno::ENOMEM);
pub const E_NOENT: isize = errno::errno_neg(errno::ENOENT);
pub const E_BADF: isize = errno::errno_neg(errno::EBADF);
pub const E_EXIST: isize = errno::errno_neg(errno::EEXIST);
pub const E_NOTDIR: isize = errno::errno_neg(errno::ENOTDIR);
pub const E_ISDIR: isize = errno::errno_neg(errno::EISDIR);
pub const E_NOSPC: isize = errno::errno_neg(errno::ENOSPC);
pub const E_IO: isize = errno::errno_neg(errno::EIO);
pub const E_INVAL: isize = errno::errno_neg(errno::EINVAL);
pub const E_NOSYS: isize = errno::errno_neg(errno::ENOSYS);
pub const E_FAULT: isize = errno::errno_neg(errno::EFAULT);
pub const E_MFILE: isize = errno::errno_neg(errno::EMFILE);
pub const E_PIPE: isize = errno::errno_neg(errno::EPIPE);
pub const E_BUSY: isize = errno::errno_neg(errno::EBUSY);
pub const E_PERM: isize = errno::errno_neg(errno::EPERM);
pub const E_NOTEMPTY: isize = errno::errno_neg(errno::ENOTEMPTY);

/// Channel for poll/select wakeup
pub const POLL_WAKE_CHAN: usize = 0x3_0000_0000;

// ============================================================================
// Trap frame argument extraction (architecture-specific)
// ============================================================================

/// Get syscall arguments from trap frame
#[cfg(target_arch = "riscv64")]
pub fn get_args(tf: &TrapFrame) -> (usize, [usize; 6]) {
    (tf.a7, [tf.a0, tf.a1, tf.a2, tf.a3, tf.a4, tf.a5])
}

#[cfg(target_arch = "aarch64")]
pub fn get_args(tf: &TrapFrame) -> (usize, [usize; 6]) {
    (tf.regs[8], [tf.regs[0], tf.regs[1], tf.regs[2], tf.regs[3], tf.regs[4], tf.regs[5]])
}

#[cfg(target_arch = "x86_64")]
pub fn get_args(tf: &TrapFrame) -> (usize, [usize; 6]) {
    (tf.rax, [tf.rdi, tf.rsi, tf.rdx, tf.r10, tf.r8, tf.r9])
}

/// Set syscall return value in trap frame
#[cfg(target_arch = "riscv64")]
pub fn set_return(tf: &mut TrapFrame, val: isize) {
    tf.a0 = val as usize;
}

#[cfg(target_arch = "aarch64")]
pub fn set_return(tf: &mut TrapFrame, val: isize) {
    tf.regs[0] = val as usize;
}

#[cfg(target_arch = "x86_64")]
pub fn set_return(tf: &mut TrapFrame, val: isize) {
    tf.rax = val as usize;
}

// ============================================================================
// Syscall dispatcher
// ============================================================================

/// Dispatch system call based on number
pub fn dispatch(num: usize, args: &[usize]) -> isize {
    let syscall_num = match SysNum::try_from(num) {
        Ok(s) => s,
        Err(_) => {
            crate::println!("unknown syscall {}", num);
            return E_NOSYS;
        }
    };

    match syscall_num {
        // Process management
        SysNum::Fork => sys_fork(),
        SysNum::Exit => sys_exit(args[0] as i32),
        SysNum::Wait => sys_wait(args[0] as *mut i32),
        SysNum::Kill => sys_kill(args[0]),
        SysNum::Getpid => sys_getpid(),
        SysNum::Sbrk => sys_sbrk(args[0] as isize),
        SysNum::Sleep => sys_sleep(args[0]),
        SysNum::Uptime => sys_uptime(),
        
        // Process group and session syscalls (P2-008)
        SysNum::Setpgid => sys_setpgid(
            args[0] as i32,
            args[1] as i32
        ),
        SysNum::Getpgid => sys_getpgid(
            args[0] as i32
        ),
        SysNum::Setsid => sys_setsid(),
        SysNum::Getsid => sys_getsid(
            args[0] as i32
        ),
        
        // Resource limit syscalls (P2-008)
        SysNum::Getrlimit => sys_getrlimit(
            args[0] as i32,
            args[1] as *mut crate::posix::Rlimit
        ),
        SysNum::Setrlimit => sys_setrlimit(
            args[0] as i32,
            args[1] as *const crate::posix::Rlimit
        ),
        
        SysNum::Exec => sys_exec(args[0] as *const u8, args[1] as *const *const u8),
        SysNum::Execve => sys_execve(args[0] as *const u8, args[1] as *const *const u8, args[2] as *const *const u8),
        
        // File I/O
        SysNum::Read => sys_read(args[0] as i32, args[1] as *mut u8, args[2]),
        SysNum::Write => sys_write(args[0] as i32, args[1] as *const u8, args[2]),
        SysNum::Open => sys_open(args[0] as *const u8, args[1] as i32, args[2] as u32),
        SysNum::Close => sys_close(args[0] as i32),
        SysNum::Fstat => sys_fstat(args[0] as i32, args[1] as *mut posix::Stat),
        SysNum::Lseek => sys_lseek(args[0] as i32, args[1] as i64, args[2] as i32),
        SysNum::Dup => sys_dup(args[0] as i32),
        SysNum::Dup2 => sys_dup2(args[0] as i32, args[1] as i32),
        SysNum::Fcntl => sys_fcntl(args[0] as i32, args[1] as i32, args[2] as usize),
        SysNum::Poll => sys_poll(args[0] as *mut crate::posix::PollFd, args[1], args[2] as i32),
        SysNum::Select => sys_select(
            args[0] as i32,
            args[1] as *mut crate::posix::FdSet,
            args[2] as *mut crate::posix::FdSet,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        ),
        
        // Pipe
        SysNum::Pipe => sys_pipe(args[0] as *mut i32),
        
        // Filesystem
        SysNum::Chdir => sys_chdir(args[0] as *const u8),
        SysNum::Getcwd => sys_getcwd(args[0] as *mut u8, args[1]),
        SysNum::Mkdir => sys_mkdir(args[0] as *const u8),
        SysNum::Rmdir => sys_rmdir(args[0] as *const u8),
        SysNum::Mknod => sys_mknod(args[0] as *const u8, args[1] as i16, args[2] as i16),
        SysNum::Link => sys_link(args[0] as *const u8, args[1] as *const u8),
        SysNum::Unlink => sys_unlink(args[0] as *const u8),
        
        // Signal
        SysNum::Sigaction => sys_sigaction(
            args[0] as i32,
            args[1] as *const crate::signal::SigAction,
            args[2] as *mut crate::signal::SigAction,
        ),
        SysNum::Sigprocmask => sys_sigprocmask(
            args[0] as i32,
            args[1] as *const crate::signal::SigSet,
            args[2] as *mut crate::signal::SigSet,
        ),
        SysNum::Sigsuspend => sys_sigsuspend(args[0] as *const crate::signal::SigSet),
        SysNum::Sigpending => sys_sigpending(args[0] as *mut crate::signal::SigSet),
        // Memory mapping
        SysNum::Mmap => sys_mmap(
            args[0] as *mut u8,
            args[1],
            args[2] as u32,
            args[3] as u32,
            args[4] as i32,
            args[5] as u64
        ),
        SysNum::Munmap => sys_munmap(
            args[0] as *mut u8,
            args[1]
        ),
        // Time
        SysNum::GetTimeOfDay => sys_gettimeofday(
            args[0] as *mut crate::posix::Timeval,
            args[1] as *mut u8
        ),
        SysNum::ClockGetTime => sys_clock_gettime(
            args[0] as crate::posix::ClockId,
            args[1] as *mut crate::posix::Timespec
        ),
        
        // Additional file operations
        SysNum::Ftruncate => sys_ftruncate(
            args[0] as i32,
            args[1] as i64
        ),
        SysNum::Fchmod => sys_fchmod(
            args[0] as i32,
            args[1] as u32
        ),
        SysNum::Fchown => sys_fchown(
            args[0] as i32,
            args[1] as u32,
            args[2] as u32
        ),
        
        // Symlink syscalls (P2-006)
        SysNum::Symlink => sys_symlink(
            args[0] as *const u8,
            args[1] as *const u8
        ),
        SysNum::Readlink => sys_readlink(
            args[0] as *const u8,
            args[1] as *mut u8,
            args[2]
        ),
        
        // Socket syscalls
        SysNum::Socket => sys_socket(
            args[0] as i32,
            args[1] as i32,
            args[2] as i32
        ),
        SysNum::Bind => sys_bind(
            args[0] as i32,
            args[1] as *const crate::posix::Sockaddr,
            args[2]
        ),
        SysNum::Listen => sys_listen(
            args[0] as i32,
            args[1] as i32
        ),
        SysNum::Accept => sys_accept(
            args[0] as i32,
            args[1] as *mut crate::posix::Sockaddr,
            args[2] as *mut usize
        ),
        SysNum::Connect => sys_connect(
            args[0] as i32,
            args[1] as *const crate::posix::Sockaddr,
            args[2]
        ),
        SysNum::Send => sys_send(
            args[0] as i32,
            args[1] as *const u8,
            args[2],
            args[3] as i32
        ),
        SysNum::Recv => sys_recv(
            args[0] as i32,
            args[1] as *mut u8,
            args[2],
            args[3] as i32
        ),
        SysNum::Sendto => sys_sendto(
            args[0] as i32,
            args[1] as *const u8,
            args[2],
            args[3] as i32,
            args[4] as *const crate::posix::Sockaddr,
            args[5]
        ),
        SysNum::Recvfrom => sys_recvfrom(
            args[0] as i32,
            args[1] as *mut u8,
            args[2],
            args[3] as i32,
            args[4] as *mut crate::posix::Sockaddr,
            args[5] as *mut usize
        ),
        SysNum::Shutdown => sys_shutdown(
            args[0] as i32,
            args[1] as i32
        ),
        SysNum::Setsockopt => sys_setsockopt(
            args[0] as i32,
            args[1] as i32,
            args[2] as i32,
            args[3] as *const u8,
            args[4]
        ),
        SysNum::Getsockopt => sys_getsockopt(
            args[0] as i32,
            args[1] as i32,
            args[2] as i32,
            args[3] as *mut u8,
            args[4] as *mut usize
        ),
        
        // sendfile and splice syscalls (P2-002)
        SysNum::Sendfile => sys_sendfile(
            args[0] as i32,
            args[1] as i32,
            args[2] as *mut i64,
            args[3]
        ),
        SysNum::Splice => sys_splice(
            args[0] as i32,
            args[1] as *mut i64,
            args[2] as i32,
            args[3] as *mut i64,
            args[4],
            args[5] as i32
        ),
        
        // epoll syscalls (P2-004)
        SysNum::EpollCreate => sys_epoll_create(
            args[0] as i32
        ),
        SysNum::EpollCtl => sys_epoll_ctl(
            args[0] as i32,
            args[1] as i32,
            args[2] as i32,
            args[3] as *mut u8
        ),
        SysNum::EpollWait => sys_epoll_wait(
            args[0] as i32,
            args[1] as *mut u8,
            args[2] as i32,
            args[3] as i32
        ),
    }
}

/// sendfile system call - zero-copy I/O
pub fn sys_sendfile(out_fd: i32, in_fd: i32, offset: *mut i64, count: usize) -> isize {
    // TODO: Implement zero-copy sendfile
    E_NOSYS
}

/// splice system call - zero-copy data transfer
pub fn sys_splice(fd_in: i32, off_in: *mut i64, fd_out: i32, off_out: *mut i64,
                  len: usize, flags: i32) -> isize {
    // TODO: Implement splice
    E_NOSYS
}

/// epoll_create system call - create epoll instance
pub fn sys_epoll_create(size: i32) -> isize {
    // TODO: Implement epoll_create
    E_NOSYS
}

/// epoll_ctl system call - control epoll instance
pub fn sys_epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut u8) -> isize {
    // TODO: Implement epoll_ctl
    E_NOSYS
}

/// epoll_wait system call - wait for events
pub fn sys_epoll_wait(epfd: i32, events: *mut u8, maxevents: i32, timeout: i32) -> isize {
    // TODO: Implement epoll_wait
    E_NOSYS
}

/// Handle syscall from trap
pub fn syscall(tf: &mut TrapFrame) {
    let (num, args) = get_args(tf);
    let ret = dispatch(num, &args);
    set_return(tf, ret);
}

// ============================================================================
// Utility functions (shared across syscall modules)
// ============================================================================

/// Copy a string from user space to kernel space
pub(crate) unsafe fn copy_path(ptr: *const u8) -> Result<alloc::string::String, ()> {
    let mut table = crate::process::PROC_TABLE.lock();
    let pagetable = match crate::process::myproc().and_then(|pid| table.find(pid).map(|p| p.pagetable)) {
        Some(pt) => pt,
        None => return Err(()),
    };
    drop(table);
    let mut buf = [0u8; 256];
    let n = match crate::vm::copyinstr(pagetable, ptr as usize, buf.as_mut_ptr(), buf.len()) {
        Ok(len) => len,
        Err(_) => return Err(()),
    };
    match alloc::string::String::from_utf8(buf[..n].to_vec()) {
        Ok(s) => Ok(s),
        Err(_) => Err(()),
    }
}

/// Join a base path with a relative path, handling . and ..
pub(crate) fn join_path(base: &str, rel: &str) -> alloc::string::String {
    let mut out: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();
    let is_abs = rel.starts_with('/');
    if !is_abs {
        for p in base.split('/').filter(|s| !s.is_empty()) {
            out.push(alloc::string::String::from(p));
        }
    }
    for p in rel.split('/').filter(|s| !s.is_empty()) {
        if p == "." { continue; }
        if p == ".." { if !out.is_empty() { out.pop(); } continue; }
        out.push(alloc::string::String::from(p));
    }
    let mut s = alloc::string::String::from("/");
    for (i, seg) in out.iter().enumerate() {
        if i > 0 { s.push('/'); }
        s.push_str(seg);
    }
    s
}

/// Resolve a path relative to current working directory
pub(crate) fn resolve_with_cwd(in_path: &str) -> alloc::string::String {
    if in_path.starts_with('/') { return alloc::string::String::from(in_path); }
    let mut ptable = crate::process::PROC_TABLE.lock();
    let cur = match crate::process::myproc().and_then(|pid| ptable.find(pid).and_then(|p| p.cwd_path.clone())) {
        Some(s) => s,
        None => alloc::string::String::from("/"),
    };
    join_path(&cur, in_path)
}