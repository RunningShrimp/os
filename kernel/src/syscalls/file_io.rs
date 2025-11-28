//! File I/O related system calls
//!
//! Implements read, write, open, close, fstat, lseek, dup, dup2, fcntl, poll, select

use crate::process;
use crate::posix;
use crate::file::{FILE_TABLE, FileType, file_alloc, file_close, file_read, file_write, file_stat, file_poll, file_subscribe, file_unsubscribe, file_truncate, file_chmod, file_chown};
use super::{E_OK, E_BADF, E_MFILE, E_NOENT, E_FAULT, E_INVAL, E_PIPE, E_BADARG, POLL_WAKE_CHAN, copy_path, resolve_with_cwd};

/// Read from a file descriptor
pub fn sys_read(fd: i32, buf: *mut u8, len: usize) -> isize {
    if fd < 0 {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    let user_buf = unsafe {
        core::slice::from_raw_parts_mut(buf, len)
    };
    
    file_read(file_idx, user_buf)
}

/// Write to a file descriptor
pub fn sys_write(fd: i32, buf: *const u8, len: usize) -> isize {
    if fd < 0 {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    let user_buf = unsafe {
        core::slice::from_raw_parts(buf, len)
    };
    
    file_write(file_idx, user_buf)
}

/// Open a file
pub fn sys_open(path: *const u8, flags: i32, mode: u32) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    let abs_path = resolve_with_cwd(&path_str);
    let vfs = crate::vfs::vfs();
    let res = if (flags & posix::O_CREAT) != 0 {
        // Try to open first to check existence
        match vfs.open(&abs_path, flags as u32) {
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
                vfs.create(&abs_path, crate::vfs::FileMode::new(mode))
            }
            Err(e) => Err(e),
        }
    } else {
        vfs.open(&abs_path, flags as u32)
    };

    match res {
        Ok(vfs_file) => {
            let fd = match file_alloc() {
                Some(fd) => fd,
                None => return E_MFILE,
            };
            
            let proc_fd = match process::fdalloc(fd) { Some(fd) => fd, None => { file_close(fd); return E_MFILE; } };
            
            let mut file_table = FILE_TABLE.lock();
            let file = file_table.get_mut(fd).unwrap();
            
            file.ftype = FileType::Vfs;
            match flags & posix::O_ACCMODE {
                x if x == posix::O_RDONLY => { file.readable = true; file.writable = false; }
                x if x == posix::O_WRONLY => { file.readable = false; file.writable = true; }
                x if x == posix::O_RDWR => { file.readable = true; file.writable = true; }
                _ => { file.readable = true; file.writable = false; }
            }
            if (flags & posix::O_NONBLOCK) != 0 { file.status_flags |= posix::O_NONBLOCK; }
            file.vfs_file = Some(vfs_file);
            
            proc_fd as isize
        }
        Err(_) => E_NOENT,
    }
}

/// Close a file descriptor
pub fn sys_close(fd: i32) -> isize {
    if fd < 0 {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    // Unsubscribe before closing
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            match f.ftype {
                FileType::Pipe | FileType::Device => {
                    let base = process::getpid() as usize | 0x4000_0000;
                    let chan_fd = base ^ (fd as usize);
                    drop(table);
                    file_unsubscribe(file_idx, chan_fd);
                }
                _ => {}
            }
        }
    }
    
    file_close(file_idx);
    process::fdclose(fd);
    
    E_OK
}

/// Get file status
pub fn sys_fstat(fd: i32, stat: *mut posix::Stat) -> isize {
    if fd < 0 || stat.is_null() {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    match file_stat(file_idx) {
        Ok(s) => {
            unsafe { *stat = s; }
            E_OK
        },
        Err(_) => E_BADF,
    }
}

/// Seek in a file
pub fn sys_lseek(fd: i32, offset: i64, whence: i32) -> isize {
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

/// Duplicate a file descriptor
pub fn sys_dup(fd: i32) -> isize {
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

/// Truncate a file
pub fn sys_ftruncate(fd: i32, length: i64) -> isize {
    if fd < 0 {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    if let Ok(_) = file_truncate(file_idx, length as u64) {
        E_OK
    } else {
        E_IO
    }
}

/// Change file mode
pub fn sys_fchmod(fd: i32, mode: u32) -> isize {
    if fd < 0 {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    if let Ok(_) = file_chmod(file_idx, mode) {
        E_OK
    } else {
        E_IO
    }
}

/// Change file ownership
pub fn sys_fchown(fd: i32, uid: u32, gid: u32) -> isize {
    if fd < 0 {
        return E_BADF;
    }
    
    let file_idx = match process::fdlookup(fd) { Some(idx) => idx, None => return E_BADF };
    
    if let Ok(_) = file_chown(file_idx, uid, gid) {
        E_OK
    } else {
        E_IO
    }
}


/// Duplicate a file descriptor to a specific number
pub fn sys_dup2(oldfd: i32, newfd: i32) -> isize {
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
        return E_MFILE;
    }
    
    newfd as isize
}

/// File control operations
pub fn sys_fcntl(fd: i32, cmd: i32, arg: usize) -> isize {
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
            E_OK
        }
        _ => E_INVAL,
    }
}

/// Poll for I/O readiness
pub fn sys_poll(fds: *mut crate::posix::PollFd, nfds: usize, _timeout: i32) -> isize {
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
            let ev = file_poll(idx);
            pfd.revents |= ev;
            let chan_fd = base ^ (pfd.fd as usize);
            file_subscribe(idx, pfd.events, chan_fd);
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
                file_unsubscribe(idx, chan_fd);
            }
        }
    }
}

/// Select for I/O readiness
pub fn sys_select(nfds: i32, readfds: *mut crate::posix::FdSet, writefds: *mut crate::posix::FdSet, _exceptfds: *mut crate::posix::FdSet, timeout: *mut crate::posix::Timeval) -> isize {
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
            let ev = file_poll(idx);
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
