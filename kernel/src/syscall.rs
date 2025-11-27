//! System call dispatcher for xv6-rust
//! Implements xv6-compatible system calls

extern crate alloc;
use crate::process::{self, TrapFrame};
use crate::file::{FILE_TABLE, FileType, file_read, file_write, file_close, file_stat, file_alloc};
use crate::errno;
use crate::posix;

/// System call numbers (xv6 compatible)
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Fcntl = 22,
    Poll = 23,
    Select = 24,
    Lseek = 25,
    Dup2 = 26,
    Getcwd = 27,
    Rmdir = 28,
}

impl TryFrom<usize> for SysNum {
    type Error = ();

    fn try_from(n: usize) -> Result<Self, Self::Error> {
        match n {
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
            _ => Err(()),
        }
    }
}

pub const E_OK: isize = errno::errno_neg(errno::EOK);
pub const E_BADARG: isize = errno::errno_neg(errno::EINVAL);
pub const E_NOENT: isize = errno::errno_neg(errno::ENOENT);
pub const E_BADF: isize = errno::errno_neg(errno::EBADF);
pub const E_NOMEM: isize = errno::errno_neg(errno::ENOMEM);
pub const E_ACCES: isize = errno::errno_neg(errno::EACCES);
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

pub const POLL_WAKE_CHAN: usize = 0x3_0000_0000;

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

/// Dispatch system call based on number
pub fn dispatch(num: usize, args: &[usize]) -> isize {
    let syscall = match SysNum::try_from(num) {
        Ok(s) => s,
        Err(_) => {
            crate::println!("unknown syscall {}", num);
            return E_NOSYS;
        }
    };

    match syscall {
        SysNum::Fork => sys_fork(),
        SysNum::Exit => sys_exit(args[0] as i32),
        SysNum::Wait => sys_wait(args[0] as *mut i32),
        SysNum::Pipe => sys_pipe(args[0] as *mut i32),
        SysNum::Read => sys_read(args[0] as i32, args[1] as *mut u8, args[2]),
        SysNum::Kill => sys_kill(args[0]),
        SysNum::Exec => sys_exec(args[0] as *const u8, args[1] as *const *const u8),
        SysNum::Fstat => sys_fstat(args[0] as i32, args[1] as *mut posix::Stat),
        SysNum::Chdir => sys_chdir(args[0] as *const u8),
        SysNum::Dup => sys_dup(args[0] as i32),
        SysNum::Getpid => sys_getpid(),
        SysNum::Sbrk => sys_sbrk(args[0] as isize),
        SysNum::Sleep => sys_sleep(args[0]),
        SysNum::Uptime => sys_uptime(),
        SysNum::Open => sys_open(args[0] as *const u8, args[1] as i32, args[2] as u32),
        SysNum::Write => sys_write(args[0] as i32, args[1] as *const u8, args[2]),
        SysNum::Mknod => sys_mknod(args[0] as *const u8, args[1] as i16, args[2] as i16),
        SysNum::Unlink => sys_unlink(args[0] as *const u8),
        SysNum::Link => sys_link(args[0] as *const u8, args[1] as *const u8),
        SysNum::Mkdir => sys_mkdir(args[0] as *const u8),
        SysNum::Close => sys_close(args[0] as i32),
        SysNum::Fcntl => sys_fcntl(args[0] as i32, args[1] as i32, args[2] as usize),
        SysNum::Poll => sys_poll(args[0] as *mut crate::posix::PollFd, args[1], args[2] as i32),
        SysNum::Select => sys_select(
            args[0] as i32,
            args[1] as *mut crate::posix::FdSet,
            args[2] as *mut crate::posix::FdSet,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        ),
        SysNum::Lseek => sys_lseek(args[0] as i32, args[1] as i64, args[2] as i32),
        SysNum::Dup2 => sys_dup2(args[0] as i32, args[1] as i32),
        SysNum::Getcwd => sys_getcwd(args[0] as *mut u8, args[1]),
        SysNum::Rmdir => sys_rmdir(args[0] as *const u8),
    }
}

/// Handle syscall from trap
pub fn syscall(tf: &mut TrapFrame) {
    let (num, args) = get_args(tf);
    let ret = dispatch(num, &args);
    set_return(tf, ret);
}

// ============================================================================
// System call implementations
// ============================================================================

fn sys_fork() -> isize {
    match process::fork() {
        Some(pid) => pid as isize,
        None => E_NOMEM,
    }
}

fn sys_exit(status: i32) -> isize {
    process::exit(status);
    0 // Never reached
}

fn sys_wait(status: *mut i32) -> isize {
    match process::wait(status) {
        Some(pid) => pid as isize,
        None => E_BADARG,
    }
}

fn sys_pipe(pipefd: *mut i32) -> isize {
    if pipefd.is_null() {
        return E_BADARG;
    }
    match crate::pipe::pipe_alloc() {
        Some((ridx, widx)) => {
            let rfd = match process::fdalloc(ridx) {
                Some(fd) => fd,
                None => {
                    let mut t = crate::file::FILE_TABLE.lock();
                    t.close(ridx);
                    t.close(widx);
                    return E_NOMEM;
                }
            };
            let wfd = match process::fdalloc(widx) {
                Some(fd) => fd,
                None => {
                    process::fdclose(rfd);
                    let mut t = crate::file::FILE_TABLE.lock();
                    t.close(ridx);
                    t.close(widx);
                    return E_NOMEM;
                }
            };
            unsafe {
                *pipefd.add(0) = rfd;
                *pipefd.add(1) = wfd;
            }
            E_OK
        }
        None => E_NOSPC,
    }
}

fn sys_read(fd: i32, buf: *mut u8, len: usize) -> isize {
    // 检查文件描述符是否有效
    if fd < 0 {
        return E_BADF;
    }
    
    // 获取当前进程
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    // 创建用户空间缓冲区切片
    let user_buf = unsafe {
        core::slice::from_raw_parts_mut(buf, len)
    };
    
    file_read(file_idx, user_buf)
}

fn sys_kill(pid: usize) -> isize {
    if process::kill(pid) {
        E_OK
    } else {
        E_BADARG
    }
}

fn sys_exec(_path: *const u8, _argv: *const *const u8) -> isize {
    crate::exec::sys_exec(_path as usize, _argv as usize)
}

/// Get file status system call
fn sys_fstat(fd: i32, stat: *mut posix::Stat) -> isize {
    // 检查文件描述符是否有效
    if fd < 0 || stat.is_null() {
        return E_BADF;
    }
    
    // 获取当前进程
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    // 获取文件状态
    match file_stat(file_idx) {
        Ok(s) => {
            // 复制到用户空间
            unsafe { *stat = s; }
            0
        },
        Err(_) => E_BADF,
    }
}

fn sys_chdir(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    // Open directory
    match crate::vfs::vfs().open(&path_str, posix::O_RDONLY as u32) {
        Ok(vfs_file) => {
            // Check if directory
            match vfs_file.stat() {
                Ok(attr) => {
                    if !crate::vfs::FileMode::new(attr.mode.0).file_type().eq(&crate::vfs::FileType::Directory) {
                        return E_NOTDIR;
                    }
                }
                Err(_) => return E_IO,
            }
            
            // Allocate file in FILE_TABLE
            let fd = match file_alloc() {
                Some(fd) => fd,
                None => return E_MFILE,
            };
            
            let mut table = FILE_TABLE.lock();
            let file = table.get_mut(fd).unwrap();
            file.ftype = FileType::Vfs;
            file.readable = true;
            file.vfs_file = Some(vfs_file);
            
            // Update proc.cwd
            if let Some(pid) = process::myproc() {
                let mut ptable = crate::process::PROC_TABLE.lock();
                if let Some(proc) = ptable.find(pid) {
                    let old_cwd = proc.cwd;
                    proc.cwd = Some(fd);
                    drop(ptable);
                    drop(table); // Drop FILE_TABLE lock
                    
                    if let Some(old) = old_cwd {
                        file_close(old);
                    }
                    return 0;
                }
            }
            // If failed to find proc, close file
            drop(table);
            file_close(fd);
            E_BADF
        }
        Err(_) => E_NOENT,
    }
}

fn sys_dup(fd: i32) -> isize {
    if fd < 0 { return E_BADF; }
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    // Increment refcount in global table
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            f.ref_count += 1;
        } else {
            return E_BADF;
        }
    }
    // Allocate new process-level fd
    match process::fdalloc(file_idx) {
        Some(newfd) => newfd as isize,
        None => E_MFILE,
    }
}

fn sys_getpid() -> isize {
    process::getpid() as isize
}

fn sys_sbrk(increment: isize) -> isize {
    // TODO: Implement sbrk (grow/shrink process heap)
    if increment == 0 {
        return 0;
    }
    E_NOSYS
}

fn sys_sleep(ticks: usize) -> isize {
    if ticks == 0 {
        return E_OK;
    }
    
    let target = crate::time::get_ticks() + ticks as u64;
    let chan = process::myproc().unwrap_or(0) | 0x1000_0000; // Sleep channel
    
    crate::time::add_sleeper(target, chan);
    process::sleep(chan);
    
    E_OK
}

fn sys_uptime() -> isize {
    crate::time::get_ticks() as isize
}

/// Copy a string from user space to kernel space
unsafe fn copy_path(ptr: *const u8) -> Result<alloc::string::String, ()> {
    let mut buf = [0u8; 256];  // Maximum path length
    let mut i = 0;
    
    while i < buf.len() - 1 {
        // Copy one byte at a time (inefficient but simple)
        let result = crate::vm::copyin(
            core::ptr::null_mut(),
            buf.as_mut_ptr().add(i),
            ptr.add(i) as usize,
            1,
        );
        
        if result.is_err() {
            return Err(());
        }
        
        if buf[i] == 0 {
            break;
        }
        
        i += 1;
    }
    
    // Ensure null termination
    buf[i] = 0;
    
    // Convert to string
    match alloc::string::String::from_utf8(buf[..i].to_vec()) {
        Ok(s) => Ok(s),
        Err(_) => Err(()),
    }
}
fn sys_open(path: *const u8, flags: i32, mode: u32) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    if path_str == "/dev/console" {
        let fd_idx = match file_alloc() { Some(fd) => fd, None => return E_MFILE };
        let proc_fd = match process::fdalloc(fd_idx) { Some(fd) => fd, None => { file_close(fd_idx); return E_MFILE; } };
        let mut table = FILE_TABLE.lock();
        let f = table.get_mut(fd_idx).unwrap();
        f.ftype = FileType::Device;
        f.readable = true;
        f.writable = true;
        f.major = 1;
        f.minor = 1;
        return proc_fd as isize;
    }
    
    let vfs = crate::vfs::vfs();
    let res = if (flags & posix::O_CREAT) != 0 {
        // Try to open first to check existence
        match vfs.open(&path_str, flags as u32) {
            Ok(f) => {
                if (flags & posix::O_EXCL) != 0 {
                    Err(crate::vfs::VfsError::Exists)
                } else {
                    if (flags & posix::O_TRUNC) != 0 {
                        let _ = f.truncate(0);
                    }
                    Ok(f)
                }
            }
            Err(crate::vfs::VfsError::NotFound) => {
                vfs.create(&path_str, crate::vfs::FileMode::new(mode))
            }
            Err(e) => Err(e),
        }
    } else {
        vfs.open(&path_str, flags as u32)
    };

    // 调用VFS层的open函数
    match res {
        Ok(vfs_file) => {
            // 分配一个文件描述符
            let fd = match file_alloc() {
                Some(fd) => fd,
                None => return E_MFILE,
            };
            
            // 获取当前进程
            let proc_fd = match process::fdalloc(fd) { Some(fd) => fd, None => { file_close(fd); return E_MFILE; } };
            
            // 获取全局文件表中的文件结构
            let mut file_table = FILE_TABLE.lock();
            let file = file_table.get_mut(fd).unwrap();
            
            // 设置文件类型为VFS
            file.ftype = FileType::Vfs;
            // 根据 flags 设置读写权限
            match flags & posix::O_ACCMODE {
                x if x == posix::O_RDONLY => { file.readable = true; file.writable = false; }
                x if x == posix::O_WRONLY => { file.readable = false; file.writable = true; }
                x if x == posix::O_RDWR => { file.readable = true; file.writable = true; }
                _ => { file.readable = true; file.writable = false; }
            }
            if (flags & posix::O_NONBLOCK) != 0 { file.status_flags |= posix::O_NONBLOCK; }
            file.vfs_file = Some(vfs_file);
            
            // 返回进程级文件描述符
            proc_fd as isize
        }
        Err(_) => E_NOENT,
    }
}

fn sys_write(fd: i32, buf: *const u8, len: usize) -> isize {
    // 检查文件描述符是否有效
    if fd < 0 {
        return E_BADF;
    }
    
    // 获取当前进程
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    // 创建用户空间缓冲区切片
    let user_buf = unsafe {
        core::slice::from_raw_parts(buf, len)
    };
    
    // 调用文件写入函数
    file_write(file_idx, user_buf)
}

fn sys_mknod(_path: *const u8, _major: i16, _minor: i16) -> isize {
    let path_str = match unsafe { copy_path(_path) } { Ok(s) => s, Err(_) => return E_FAULT };
    if path_str == "/dev/console" { return 0; }
    E_NOSYS
}

fn sys_unlink(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    match crate::vfs::vfs().unlink(&path_str) {
        Ok(_) => 0,
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::IsDirectory => E_ISDIR,
            crate::vfs::VfsError::Busy => E_BUSY,
            _ => E_IO,
        },
    }
}

fn sys_link(old_path: *const u8, new_path: *const u8) -> isize {
    let old_path_str = match unsafe { copy_path(old_path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    let new_path_str = match unsafe { copy_path(new_path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    match crate::vfs::vfs().link(&old_path_str, &new_path_str) {
        Ok(_) => 0,
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::Exists => E_EXIST,
            crate::vfs::VfsError::IsDirectory => E_PERM,
            crate::vfs::VfsError::NotSupported => E_NOSYS,
            _ => E_IO,
        },
    }
}

fn sys_mkdir(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    // Default mode 0755
    let mode = crate::vfs::FileMode::new(crate::vfs::FileMode::S_IFDIR | 0o755);
    
    match crate::vfs::vfs().mkdir(&path_str, mode) {
        Ok(_) => 0,
        Err(e) => match e {
            crate::vfs::VfsError::Exists => E_EXIST,
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::NoSpace => E_NOSPC,
            _ => E_IO,
        },
    }
}

/// Close file system call
fn sys_close(fd: i32) -> isize {
    // 检查文件描述符是否有效
    if fd < 0 {
        return E_BADF;
    }
    
    // 获取当前进程
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    // 关闭前取消订阅
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            match f.ftype {
                FileType::Pipe | FileType::Device => {
                    let base = process::getpid() as usize | 0x4000_0000;
                    let chan_fd = base ^ (fd as usize);
                    drop(table);
                    crate::file::file_unsubscribe(file_idx, chan_fd);
                }
                _ => {}
            }
        }
    }
    // 关闭文件
    file_close(file_idx);
    
    // 清除进程中的文件描述符
    process::fdclose(fd);
    
    0
}

fn sys_fcntl(fd: i32, cmd: i32, arg: usize) -> isize {
    if fd < 0 { return E_BADF; }
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    let mut table = FILE_TABLE.lock();
    let file = match table.get_mut(file_idx) { Some(f) => f, None => return E_BADF };
    match cmd {
        x if x == crate::posix::F_GETFL => file.status_flags as isize,
        x if x == crate::posix::F_SETFL => {
            let mut flags = file.status_flags;
            let nonblock = (arg as i32) & crate::posix::O_NONBLOCK;
            flags = (flags & !crate::posix::O_NONBLOCK) | nonblock;
            file.status_flags = flags;
            0
        }
        _ => E_INVAL,
    }
}

fn sys_poll(fds: *mut crate::posix::PollFd, nfds: usize, _timeout: i32) -> isize {
    if fds.is_null() { return E_BADARG; }
    let pfds = unsafe { core::slice::from_raw_parts_mut(fds, nfds) };
    let mut ready;
    let start = crate::time::get_ticks();
    let base = process::myproc().unwrap_or(0) as usize | 0x4000_0000;
    loop {
        ready = 0;
        for pfd in pfds.iter_mut() {
            pfd.revents = 0;
            if pfd.fd < 0 { pfd.revents |= crate::posix::POLLNVAL; continue; }
            let idx = match process::fdlookup(pfd.fd) { Some(i) => i, None => { pfd.revents |= crate::posix::POLLNVAL; continue; } };
            let mut table = FILE_TABLE.lock();
            let _file = match table.get_mut(idx) { Some(f) => f, None => { pfd.revents |= crate::posix::POLLNVAL; continue; } };
            let ev = crate::file::file_poll(idx);
            pfd.revents |= ev;
            let chan_fd = base ^ (pfd.fd as usize);
            crate::file::file_subscribe(idx, pfd.events, chan_fd);
            if (pfd.revents & pfd.events) != 0 { ready += 1; }
        }
        if ready > 0 { return ready as isize; }
        if _timeout == 0 { return 0; }
        if _timeout > 0 {
            let elapsed = (crate::time::get_ticks() - start) as i32;
            if elapsed >= _timeout as i32 { return 0; }
        }
        let target = crate::time::get_ticks() + 1;
        crate::time::add_sleeper(target, POLL_WAKE_CHAN);
        process::sleep(POLL_WAKE_CHAN);
        for pfd in pfds.iter_mut() {
            if pfd.fd < 0 { continue; }
            if let Some(idx) = process::fdlookup(pfd.fd) {
                let chan_fd = base ^ (pfd.fd as usize);
                crate::file::file_unsubscribe(idx, chan_fd);
            }
        }
    }
}

fn sys_select(nfds: i32, readfds: *mut crate::posix::FdSet, writefds: *mut crate::posix::FdSet, _exceptfds: *mut crate::posix::FdSet, timeout: *mut crate::posix::Timeval) -> isize {
    if nfds < 0 { return E_BADARG; }
    let mut ready;
    let start = crate::time::get_ticks();
    let mut deadline: Option<u64> = None;
    if !timeout.is_null() {
        let tv = unsafe { *timeout };
        let total_us = tv.tv_sec as i64 * 1_000_000 + tv.tv_usec;
        if total_us <= 0 { return 0; }
        let tick_us = (1_000_000u64 / crate::time::TIMER_FREQ) as i64;
        let ticks = ((total_us + tick_us - 1) / tick_us) as u64;
        deadline = Some(start + ticks);
    }
    loop {
        ready = 0;
        for fd in 0..(nfds as usize) {
            let mut want_read = false;
            let mut want_write = false;
            if !readfds.is_null() {
                let set = unsafe { &*readfds };
                want_read = crate::posix::fd_isset(set, fd as i32);
            }
            if !writefds.is_null() {
                let set = unsafe { &*writefds };
                want_write = crate::posix::fd_isset(set, fd as i32);
            }
            if !want_read && !want_write { continue; }
            let idx = match process::fdlookup(fd as i32) { Some(i) => i, None => { continue; } };
            let mut table = FILE_TABLE.lock();
            let _file = match table.get_mut(idx) { Some(f) => f, None => { continue; } };
            let ev = crate::file::file_poll(idx);
            let r_ok = want_read && ((ev & crate::posix::POLLIN) != 0);
            let w_ok = want_write && ((ev & crate::posix::POLLOUT) != 0);
            let x_ok = (ev & crate::posix::POLLPRI) != 0 || (ev & crate::posix::POLLERR) != 0;
            if !readfds.is_null() {
                let set = unsafe { &mut *readfds };
                if want_read && !r_ok { crate::posix::fd_clr(set, fd as i32); } else if r_ok { ready += 1; }
            }
            if !writefds.is_null() {
                let set = unsafe { &mut *writefds };
                if want_write && !w_ok { crate::posix::fd_clr(set, fd as i32); } else if w_ok { ready += 1; }
            }
            if !_exceptfds.is_null() && x_ok {
                let set = unsafe { &mut *_exceptfds };
                crate::posix::fd_set(set, fd as i32);
            }
        }
        if ready > 0 { return ready as isize; }
        if let Some(dl) = deadline { if crate::time::get_ticks() >= dl { return 0; } }
        let target = crate::time::get_ticks() + 1;
        crate::time::add_sleeper(target, POLL_WAKE_CHAN);
        process::sleep(POLL_WAKE_CHAN);
    }
}

fn sys_lseek(fd: i32, offset: i64, whence: i32) -> isize {
    if fd < 0 { return E_BADF; }
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    let mut table = FILE_TABLE.lock();
    let file = match table.get_mut(file_idx) { Some(f) => f, None => return E_BADF };
    
    if file.ftype == FileType::Pipe {
        return E_PIPE;
    }
    
    let current_size = match file.ftype {
        FileType::Vfs => {
            if let Some(ref vfs_file) = file.vfs_file {
                match vfs_file.stat() {
                    Ok(attr) => attr.size as i64,
                    Err(_) => 0,
                }
            } else { 0 }
        },
        _ => 0,
    };
    
    let new_offset = match whence {
        crate::posix::SEEK_SET => offset,
        crate::posix::SEEK_CUR => file.offset as i64 + offset,
        crate::posix::SEEK_END => current_size + offset,
        _ => return E_INVAL,
    };
    
    if new_offset < 0 {
        return E_INVAL;
    }
    
    file.offset = new_offset as usize;
    new_offset as isize
}

fn sys_dup2(oldfd: i32, newfd: i32) -> isize {
    if oldfd < 0 || newfd < 0 || newfd >= crate::file::NOFILE as i32 {
        return E_BADF;
    }
    
    if oldfd == newfd {
        // Check if oldfd is valid
        if process::fdlookup(oldfd).is_none() {
            return E_BADF;
        }
        return newfd as isize;
    }
    
    let file_idx = match process::fdlookup(oldfd) { Some(idx) => idx, None => return E_BADF };
    
    // Close newfd if open
    if process::fdlookup(newfd).is_some() {
        sys_close(newfd);
    }
    
    // Increment refcount
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            f.ref_count += 1;
        } else {
            return E_BADF;
        }
    }
    
    // Install into newfd
    if process::fdinstall(newfd, file_idx).is_err() {
        // Should not happen if we checked range and closed it
        return E_MFILE;
    }
    
    newfd as isize
}

fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    if buf.is_null() || size == 0 {
        return E_BADARG;
    }
    
    // TODO: Implement getcwd properly
    // For now, return root "/"
    if size < 2 {
        return E_BADARG; // ERANGE
    }
    
    unsafe {
        *buf = b'/';
        *buf.add(1) = 0;
    }
    
    // Return pointer to buf (but syscall returns isize)
    // In Linux getcwd returns length or pointer?
    // man getcwd: returns pointer to buf on success.
    // But here we return isize.
    // xv6 returns string in buf.
    // Let's return address of buf as isize.
    buf as isize
}

fn sys_rmdir(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    match crate::vfs::vfs().rmdir(&path_str) {
        Ok(_) => 0,
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::NotDirectory => E_NOTDIR,
            crate::vfs::VfsError::NotEmpty => E_NOTEMPTY,
            crate::vfs::VfsError::Busy => E_BUSY,
            _ => E_IO,
        },
    }
}
