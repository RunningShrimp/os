//! User-space library for xv6-rust
//! Provides syscall wrappers, minimal libc functionality, and GLib support

#![no_std]

use core::arch::asm;

// Include GLib modules
pub mod glib;

// Print utilities for GLib use
pub mod print {
    pub use super::puts;
    pub use super::println;
}

// ============================================================================
// System call numbers (must match kernel/src/syscalls.rs)
// ============================================================================

pub const SYS_FORK: usize = 1;
pub const SYS_EXIT: usize = 2;
pub const SYS_WAIT: usize = 3;
pub const SYS_PIPE: usize = 4;
pub const SYS_READ: usize = 5;
pub const SYS_KILL: usize = 6;
pub const SYS_EXEC: usize = 7;
pub const SYS_FSTAT: usize = 8;
pub const SYS_CHDIR: usize = 9;
pub const SYS_DUP: usize = 10;
pub const SYS_GETPID: usize = 11;
pub const SYS_SBRK: usize = 12;
pub const SYS_SLEEP: usize = 13;
pub const SYS_UPTIME: usize = 14;
pub const SYS_OPEN: usize = 15;
pub const SYS_WRITE: usize = 16;
pub const SYS_MKNOD: usize = 17;
pub const SYS_UNLINK: usize = 18;
pub const SYS_LINK: usize = 19;
pub const SYS_MKDIR: usize = 20;
pub const SYS_CLOSE: usize = 21;
pub const SYS_FCNTL: usize = 22;
pub const SYS_POLL: usize = 23;
pub const SYS_SELECT: usize = 24;
pub const SYS_LSEEK: usize = 25;
pub const SYS_DUP2: usize = 26;
pub const SYS_GETCWD: usize = 27;
pub const SYS_RMDIR: usize = 28;
pub const SYS_SIGACTION: usize = 29;
pub const SYS_SIGPROCMASK: usize = 30;
pub const SYS_SIGSUSPEND: usize = 31;
pub const SYS_SIGPENDING: usize = 32;
pub const SYS_EXECVE: usize = 44;

// ============================================================================
// Low-level syscall interface
// ============================================================================

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn syscall0(num: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") num => ret,
            in("a7") num,
        );
    }
    ret
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn syscall1(num: usize, arg0: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a7") num,
        );
    }
    ret
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn syscall2(num: usize, arg0: usize, arg1: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a7") num,
        );
    }
    ret
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn syscall3(num: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") num,
        );
    }
    ret
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
pub fn syscall5(num: usize, a0: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a7") num,
        );
    }
    ret
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn syscall0(num: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") num => ret,
            in("x8") num,
        );
    }
    ret
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn syscall1(num: usize, arg0: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") arg0 => ret,
            in("x8") num,
        );
    }
    ret
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn syscall2(num: usize, arg0: usize, arg1: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") arg0 => ret,
            in("x1") arg1,
            in("x8") num,
        );
    }
    ret
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn syscall3(num: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") arg0 => ret,
            in("x1") arg1,
            in("x2") arg2,
            in("x8") num,
        );
    }
    ret
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn syscall5(num: usize, a0: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "svc #0",
            inlateout("x0") a0 => ret,
            in("x1") a1,
            in("x2") a2,
            in("x3") a3,
            in("x4") a4,
            in("x8") num,
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall0(num: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") num => ret,
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall1(num: usize, arg0: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") num => ret,
            in("rdi") arg0,
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall2(num: usize, arg0: usize, arg1: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") num => ret,
            in("rdi") arg0,
            in("rsi") arg1,
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall3(num: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") num => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
        );
    }
    ret
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall5(num: usize, a0: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") num => ret,
            in("rdi") a0,
            in("rsi") a1,
            in("rdx") a2,
            in("r10") a3,
            in("r8") a4,
        );
    }
    ret
}

// ============================================================================
// System call wrappers
// ============================================================================

/// Fork the current process
pub fn fork() -> isize {
    syscall0(SYS_FORK)
}

/// Exit the current process
pub fn exit(status: i32) -> ! {
    syscall1(SYS_EXIT, status as usize);
    unreachable!()
}

/// Wait for a child to exit
pub fn wait(status: *mut i32) -> isize {
    syscall1(SYS_WAIT, status as usize)
}

/// Create a pipe
pub fn pipe(pipefd: *mut i32) -> isize {
    map_ret(syscall1(SYS_PIPE, pipefd as usize))
}

/// Read from file descriptor
pub fn read(fd: i32, buf: *mut u8, count: usize) -> isize {
    map_ret(syscall3(SYS_READ, fd as usize, buf as usize, count))
}

/// Write to file descriptor
pub fn write(fd: i32, buf: *const u8, count: usize) -> isize {
    map_ret(syscall3(SYS_WRITE, fd as usize, buf as usize, count))
}

/// Kill a process
pub fn kill(pid: i32) -> isize {
    syscall1(SYS_KILL, pid as usize)
}

/// Execute a program
pub fn exec(path: *const u8, argv: *const *const u8) -> isize {
    syscall2(SYS_EXEC, path as usize, argv as usize)
}

pub fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    map_ret(syscall3(SYS_EXECVE, path as usize, argv as usize, envp as usize))
}

/// Get file status
pub fn fstat(fd: i32, stat: *mut u8) -> isize {
    map_ret(syscall2(SYS_FSTAT, fd as usize, stat as usize))
}

/// Change current directory
pub fn chdir(path: *const u8) -> isize {
    map_ret(syscall1(SYS_CHDIR, path as usize))
}

/// Duplicate file descriptor
pub fn dup(fd: i32) -> isize {
    map_ret(syscall1(SYS_DUP, fd as usize))
}

/// Get process ID
pub fn getpid() -> isize {
    syscall0(SYS_GETPID)
}

/// Change program break
pub fn sbrk(increment: isize) -> isize {
    map_ret(syscall1(SYS_SBRK, increment as usize))
}

/// Sleep for ticks
pub fn sleep(ticks: usize) -> isize {
    map_ret(syscall1(SYS_SLEEP, ticks))
}

/// Get uptime in ticks
pub fn uptime() -> isize {
    syscall0(SYS_UPTIME)
}

/// Open a file
pub fn open(path: *const u8, flags: i32) -> isize {
    map_ret(syscall2(SYS_OPEN, path as usize, flags as usize))
}

/// Create a device node
pub fn mknod(path: *const u8, major: i16, minor: i16) -> isize {
    map_ret(syscall3(SYS_MKNOD, path as usize, major as usize, minor as usize))
}

/// Unlink a file
pub fn unlink(path: *const u8) -> isize {
    map_ret(syscall1(SYS_UNLINK, path as usize))
}

/// Create a hard link
pub fn link(old: *const u8, new: *const u8) -> isize {
    map_ret(syscall2(SYS_LINK, old as usize, new as usize))
}

/// Create a directory
pub fn mkdir(path: *const u8) -> isize {
    map_ret(syscall1(SYS_MKDIR, path as usize))
}

/// Close a file descriptor
pub fn close(fd: i32) -> isize {
    map_ret(syscall1(SYS_CLOSE, fd as usize))
}

// Signals (minimal)
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SigSet { pub bits: u64 }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SigAction {
    pub handler: usize,
    pub flags: u32,
    pub mask: SigSet,
    pub restorer: usize,
}

pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;
pub const SIGINT: u32 = 2;

#[inline]
pub fn sigemptyset(set: &mut SigSet) { set.bits = 0; }
#[inline]
pub fn sigaddset(set: &mut SigSet, sig: u32) { if sig>0 { set.bits |= 1u64 << (sig - 1); } }

pub fn sigprocmask(how: i32, set: *const SigSet, old: *mut SigSet) -> isize {
    map_ret(syscall3(SYS_SIGPROCMASK, how as usize, set as usize, old as usize))
}

pub fn sigpending(set: *mut SigSet) -> isize {
    map_ret(syscall1(SYS_SIGPENDING, set as usize))
}

pub fn sigsuspend(mask: *const SigSet) -> isize {
    syscall1(SYS_SIGSUSPEND, mask as usize)
}

pub fn sigaction(sig: u32, act: *const SigAction, old: *mut SigAction) -> isize {
    map_ret(syscall3(SYS_SIGACTION, sig as usize, act as usize, old as usize))
}

// ============================================================================
// File open flags (must match kernel)
// ============================================================================

pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREATE: i32 = 0x200;
pub const O_TRUNC: i32 = 0x400;

// ============================================================================
// Standard file descriptors
// ============================================================================

pub const STDIN: i32 = 0;
pub const STDOUT: i32 = 1;
pub const STDERR: i32 = 2;

// ============================================================================
// Helper functions
// ============================================================================

/// Write a string to stdout
pub fn puts(s: &str) {
    write(STDOUT, s.as_ptr(), s.len());
}

/// Write a string followed by newline to stdout
pub fn println(s: &str) {
    puts(s);
    puts("\n");
}

/// Write an integer to stdout
pub fn print_int(mut n: isize) {
    if n < 0 {
        puts("-");
        n = -n;
    }
    
    let mut buf = [0u8; 20];
    let mut i = 0;
    
    if n == 0 {
        puts("0");
        return;
    }
    
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    
    // Reverse
    while i > 0 {
        i -= 1;
        let s = core::str::from_utf8(&buf[i..i+1]).unwrap();
        puts(s);
    }
}

/// Get string length
pub fn strlen(s: *const u8) -> usize {
    let mut len = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

/// Compare strings
pub fn strcmp(s1: *const u8, s2: *const u8) -> i32 {
    let mut i = 0;
    unsafe {
        loop {
            let c1 = *s1.add(i);
            let c2 = *s2.add(i);
            if c1 != c2 {
                return (c1 as i32) - (c2 as i32);
            }
            if c1 == 0 {
                return 0;
            }
            i += 1;
        }
    }
}

// ============================================================================
// Panic handler for user space
// ============================================================================

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    puts("user panic!\n");
    exit(1);
}
pub static mut ERRNO: i32 = 0;

#[inline]
fn map_ret(ret: isize) -> isize {
    if ret < 0 {
        unsafe { ERRNO = (-ret) as i32; }
        -1
    } else {
        ret
    }
}

pub fn errno() -> i32 { unsafe { ERRNO } }

pub fn strerror(e: i32) -> &'static str {
    match e {
        0 => "EOK",
        1 => "EPERM",
        2 => "ENOENT",
        3 => "ESRCH",
        4 => "EINTR",
        5 => "EIO",
        6 => "ENXIO",
        7 => "E2BIG",
        8 => "ENOEXEC",
        9 => "EBADF",
        10 => "ECHILD",
        11 => "EAGAIN",
        12 => "ENOMEM",
        13 => "EACCES",
        14 => "EFAULT",
        20 => "ENOTDIR",
        21 => "EISDIR",
        22 => "EINVAL",
        23 => "ENFILE",
        24 => "EMFILE",
        25 => "ENOTTY",
        26 => "ETXTBSY",
        27 => "EFBIG",
        28 => "ENOSPC",
        29 => "ESPIPE",
        30 => "EROFS",
        31 => "EMLINK",
        32 => "EPIPE",
        36 => "ENAMETOOLONG",
        38 => "ENOSYS",
        _ => "UNKNOWN",
    }
}

pub fn errno_name() -> &'static str { strerror(errno()) }

pub fn perror_r(msg: &str, out: &mut [u8]) -> usize {
    let mut n = 0;
    let name = errno_name();
    let m = msg.as_bytes();
    let sep = b": ";
    let nl = b"\n";
    let parts = [m, sep, name.as_bytes(), nl];
    for p in parts.iter() {
        let k = core::cmp::min(out.len().saturating_sub(n), p.len());
        if k == 0 { break; }
        out[n..n+k].copy_from_slice(&p[..k]);
        n += k;
    }
    n
}

pub fn errno_name_num(e: i32, buf: &mut [u8]) -> usize {
    let name = strerror(e);
    if name != "UNKNOWN" {
        let n = name.as_bytes().len().min(buf.len());
        buf[..n].copy_from_slice(&name.as_bytes()[..n]);
        return n;
    }
    let prefix = b"E";
    let mut i = 0;
    if i < buf.len() { buf[i] = prefix[0]; i += 1; }
    let mut val = e;
    let mut tmp = [0u8; 12];
    let mut t = 0;
    if val == 0 { tmp[t] = b'0'; t += 1; }
    if val < 0 { val = -val; }
    while val > 0 && t < tmp.len() {
        tmp[t] = b'0' + (val % 10) as u8;
        val /= 10;
        t += 1;
    }
    while t > 0 && i < buf.len() {
        t -= 1;
        buf[i] = tmp[t];
        i += 1;
    }
    i
}

pub fn perror(msg: &str) {
    let e = errno();
    write(STDERR, msg.as_ptr(), msg.len());
    write(STDERR, ": ".as_ptr(), 2);
    let s = strerror(e);
    write(STDERR, s.as_ptr(), s.len());
    write(STDERR, "\n".as_ptr(), 1);
}

pub fn fcntl(fd: i32, cmd: i32, arg: usize) -> isize {
    map_ret(syscall3(SYS_FCNTL, fd as usize, cmd as usize, arg))
}

pub fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> isize {
    map_ret(syscall3(SYS_POLL, fds as usize, nfds, timeout as usize))
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FdSet { pub bits: [u64; (FD_SETSIZE + 63) / 64] }

pub const FD_SETSIZE: usize = 1024;

#[inline] pub fn fd_zero(set: &mut FdSet) { for b in set.bits.iter_mut() { *b = 0; } }
#[inline] pub fn fd_set(set: &mut FdSet, fd: i32) { if fd>=0 && (fd as usize)<FD_SETSIZE { let i=(fd as usize)/64; let o=(fd as usize)%64; set.bits[i]|=1u64<<o; } }
#[inline] pub fn fd_clr(set: &mut FdSet, fd: i32) { if fd>=0 && (fd as usize)<FD_SETSIZE { let i=(fd as usize)/64; let o=(fd as usize)%64; set.bits[i]&=!(1u64<<o); } }
#[inline] pub fn fd_isset(set: &FdSet, fd: i32) -> bool { if fd>=0 && (fd as usize)<FD_SETSIZE { let i=(fd as usize)/64; let o=(fd as usize)%64; (set.bits[i] & (1u64<<o))!=0 } else { false } }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Timeval { pub tv_sec: i64, pub tv_usec: i64 }

pub fn select(nfds: i32, readfds: *mut FdSet, writefds: *mut FdSet, exceptfds: *mut FdSet, timeout: *mut Timeval) -> isize {
    map_ret(syscall5(SYS_SELECT, nfds as usize, readfds as usize, writefds as usize, exceptfds as usize, timeout as usize))
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

pub const POLLIN: i16 = 0x0001;
pub const POLLOUT: i16 = 0x0004;
pub const POLLERR: i16 = 0x0008;
pub const POLLHUP: i16 = 0x0010;
