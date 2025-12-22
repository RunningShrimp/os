//! File I/O related system calls
//!
//! Implements read, write, open, close, fstat, lseek, dup, dup2, fcntl, poll, select

use crate::fs::file::{FILE_TABLE, FileType, file_alloc, file_close, file_read, file_write, file_stat, file_lseek, file_unsubscribe};
use crate::syscalls::common::{SyscallError, SyscallResult, extract_args};
use crate::subsystems::sync::Mutex;
use alloc::string::ToString;
use core::sync::atomic::{AtomicUsize, Ordering};

/// 全局文件I/O统计
static IO_STATS: Mutex<IoStats> = Mutex::new(IoStats::new());

/// I/O统计信息
#[derive(Debug, Default)]
pub struct IoStats {
    pub read_count: AtomicUsize,
    pub write_count: AtomicUsize,
    pub open_count: AtomicUsize,
    pub close_count: AtomicUsize,
    pub bytes_read: AtomicUsize,
    pub bytes_written: AtomicUsize,
}

impl IoStats {
    pub const fn new() -> Self {
        Self {
            read_count: AtomicUsize::new(0),
            write_count: AtomicUsize::new(0),
            open_count: AtomicUsize::new(0),
            close_count: AtomicUsize::new(0),
            bytes_read: AtomicUsize::new(0),
            bytes_written: AtomicUsize::new(0),
        }
    }
    
    pub fn record_read(&self, bytes: usize) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn record_write(&self, bytes: usize) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn record_open(&self) {
        self.open_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_close(&self) {
        self.close_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> (usize, usize, usize, usize, usize, usize) {
        (
            self.read_count.load(Ordering::Relaxed),
            self.write_count.load(Ordering::Relaxed),
            self.open_count.load(Ordering::Relaxed),
            self.close_count.load(Ordering::Relaxed),
            self.bytes_read.load(Ordering::Relaxed),
            self.bytes_written.load(Ordering::Relaxed),
        )
    }
}

/// 获取I/O统计信息
pub fn get_io_stats() -> (usize, usize, usize, usize, usize, usize) {
    IO_STATS.lock().get_stats()
}

/// Read from a file descriptor
pub fn sys_read(fd: i32, buf: *mut u8, len: usize) -> isize {
    if fd < 0 {
        return crate::reliability::errno::errno_neg(crate::reliability::errno::EBADF);
    }
    
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EBADF),
    };
    
    let user_buf = unsafe {
        core::slice::from_raw_parts_mut(buf, len)
    };
    
    let result = file_read(file_idx, user_buf);
    
    // Record statistics
    if result >= 0 {
        IO_STATS.lock().record_read(result as usize);
    }
    
    result
}

/// Write to a file descriptor
pub fn sys_write(fd: i32, buf: *const u8, len: usize) -> isize {
    if fd < 0 {
        return crate::reliability::errno::errno_neg(crate::reliability::errno::EBADF);
    }
    
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EBADF),
    };
    
    let user_buf = unsafe {
        core::slice::from_raw_parts(buf, len)
    };
    
    let result = file_write(file_idx, user_buf);
    
    // Record statistics
    if result >= 0 {
        IO_STATS.lock().record_write(result as usize);
    }
    
    result
}

/// Open a file
pub fn sys_open(path: *const u8, flags: i32, mode: u32) -> isize {
    // Read path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    
    let pagetable = match crate::process::myproc() {
        Some(pid) => {
            let mut table = crate::process::manager::PROC_TABLE.lock();
            match table.find(pid) {
                Some(proc) => proc.pagetable,
                None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EFAULT),
            }
        }
        None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EFAULT),
    };

    let path_len = unsafe {
        match crate::subsystems::mm::vm::copyinstr(pagetable,
                                     path as usize,
                                     path_buf.as_mut_ptr(),
                                     MAX_PATH_LEN) {
            Ok(len) => len,
            Err(_) => return crate::reliability::errno::errno_neg(crate::reliability::errno::EFAULT),
        }
    };
    
    let path_str = match core::str::from_utf8(&path_buf[..path_len]) {
        Ok(s) => s,
        Err(_) => return crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL),
    };
    
    // Resolve absolute path
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        // Get current working directory from process
        let pid = match crate::process::myproc() {
            Some(pid) => pid,
            None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EPERM),
        };
        
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(proc) => proc,
            None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EPERM),
        };
        
        let cwd = proc.cwd_path.clone().unwrap_or_else(|| "/".to_string());
        format!("{}/{}", cwd, path_str)
    };
    
    // Check if we need to create the file
    let vfs_file = if (flags & (crate::posix::O_CREAT as i32)) != 0 {
        // Create or open the file
        let file_mode = crate::vfs::FileMode::new(mode);
        match crate::vfs::vfs().create(&abs_path, file_mode) {
            Ok(file) => file,
            Err(_) => return crate::reliability::errno::errno_neg(crate::reliability::errno::ENOENT),
        }
    } else {
        // Open existing file
        match crate::vfs::vfs().open(&abs_path, flags as u32) {
            Ok(file) => file,
            Err(_) => return crate::reliability::errno::errno_neg(crate::reliability::errno::ENOENT),
        }
    };
    
    // Allocate a file entry in FILE_TABLE
    let file_idx = match file_alloc() {
        Some(idx) => idx,
        None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EMFILE),
    };
    
    // Initialize the file entry
    {
        let mut table = FILE_TABLE.lock();
        let file = match table.get_mut(file_idx) {
            Some(f) => f,
            None => {
                file_close(file_idx);
                return crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL);
            }
        };
        
        file.ftype = FileType::Vfs;
        file.readable = (flags & (crate::posix::O_RDONLY | crate::posix::O_RDWR) as i32) != 0;
        file.writable = (flags & (crate::posix::O_WRONLY | crate::posix::O_RDWR) as i32) != 0;
        file.status_flags = flags;
        file.vfs_file = Some(vfs_file);
    }
    
    // Allocate a file descriptor for the process
let result = match crate::process::fdalloc(file_idx) {
    Some(fd) => fd as isize,
    None => {
        file_close(file_idx);
        crate::reliability::errno::errno_neg(crate::reliability::errno::EMFILE)
    }
};

// Record statistics
if result >= 0 {
    IO_STATS.lock().record_open();
}

result
}

/// Close a file descriptor - optimized version
pub fn sys_close(fd: i32) -> isize {
    if fd < 0 {
        return crate::reliability::errno::errno_neg(crate::reliability::errno::EBADF);
    }
    
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return crate::reliability::errno::errno_neg(crate::reliability::errno::EBADF),
    };
    
    // Unsubscribe before closing if needed
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            match f.ftype {
                FileType::Pipe | FileType::Device => {
                    let base = crate::process::getpid() as usize | 0x4000_0000;
                    let chan_fd = base ^ (fd as usize);
                    drop(table);
                    file_unsubscribe(file_idx, chan_fd);
                }
                _ => {}
            }
        }
    }
    
    file_close(file_idx);
    crate::process::fdclose(fd);
    
    // Record statistics
    IO_STATS.lock().record_close();
    
    0
}

/// Dispatch file I/O syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        0x2000 => sys_open_impl(args),     // open
        0x2001 => sys_close_impl(args),    // close
        0x2002 => sys_read_impl(args),     // read
        0x2003 => sys_write_impl(args),    // write
        0x2004 => sys_lseek_impl(args),    // lseek
        0x2005 => sys_fstat_impl(args),    // fstat
        0x2006 => sys_stat_impl(args),    // stat
        0x2007 => Err(SyscallError::NotSupported), // lstat - TODO: implement later
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Syscall implementation wrappers that return SyscallResult
fn sys_open_impl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let path_ptr = args[0] as *const u8;
    let flags = args[1] as i32;
    let mode = args[2] as u32;
    
    // Read path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];

    let pagetable = match crate::process::myproc() {
        Some(pid) => {
            let mut table = crate::process::manager::PROC_TABLE.lock();
            match table.find(pid) {
                Some(proc) => proc.pagetable,
                None => return Err(SyscallError::BadAddress),
            }
        }
        None => return Err(SyscallError::BadAddress),
    };

    let path_len = unsafe {
        crate::subsystems::mm::vm::copyinstr(pagetable,
                                path_ptr as usize,
                                path_buf.as_mut_ptr(),
                                MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Resolve absolute path
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        // Get current working directory from process
        let pid = crate::process::myproc().ok_or(SyscallError::IoError)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SyscallError::IoError)?;
        let cwd = proc.cwd_path.clone().unwrap_or_else(|| "/".to_string());
        format!("{}/{}", cwd, path_str)
    };
    
    // Check if we need to create the file
    let vfs_file = if (flags & (crate::posix::O_CREAT as i32)) != 0 {
        // Create or open the file
        let file_mode = crate::vfs::FileMode::new(mode);
        crate::vfs::vfs().create(&abs_path, file_mode)
    } else {
        // Open existing file
        crate::vfs::vfs().open(&abs_path, flags as u32)
    }
    .map_err(|_| SyscallError::NotFound)?;
    
    // Allocate a file entry in FILE_TABLE
    let file_idx = file_alloc().ok_or(SyscallError::IoError)?;
    
    // Initialize the file entry
    let mut table = FILE_TABLE.lock();
    let file = table.get_mut(file_idx).ok_or(SyscallError::IoError)?;
    file.ftype = FileType::Vfs;
    file.readable = (flags & (crate::posix::O_RDONLY | crate::posix::O_RDWR) as i32) != 0;
    file.writable = (flags & (crate::posix::O_WRONLY | crate::posix::O_RDWR) as i32) != 0;
    file.status_flags = flags;
    file.vfs_file = Some(vfs_file);
    
    // Allocate a file descriptor for the process
    let fd = crate::process::fdalloc(file_idx).ok_or(SyscallError::IoError)?;
    
    Ok(fd as u64)
}

fn sys_read_impl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let fd = args[0] as i32;
    let buf_ptr = args[1] as *mut u8;
    let count = args[2] as usize;
    
    if fd < 0 {
        return Err(SyscallError::BadFileDescriptor);
    }
    
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::BadFileDescriptor),
    };
    
    let user_buf = unsafe {
        core::slice::from_raw_parts_mut(buf_ptr, count)
    };
    
    let result = file_read(file_idx, user_buf);
    if result < 0 {
        Err(SyscallError::IoError)
    } else {
        Ok(result as u64)
    }
}

fn sys_write_impl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let fd = args[0] as i32;
    let buf_ptr = args[1] as *const u8;
    let count = args[2] as usize;
    
    if fd < 0 {
        return Err(SyscallError::BadFileDescriptor);
    }
    
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::BadFileDescriptor),
    };
    
    let user_buf = unsafe {
        core::slice::from_raw_parts(buf_ptr, count)
    };
    
    let result = file_write(file_idx, user_buf);
    if result < 0 {
        Err(SyscallError::IoError)
    } else {
        Ok(result as u64)
    }
}

fn sys_close_impl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let fd = args[0] as i32;
    
    if fd < 0 {
        return Err(SyscallError::BadFileDescriptor);
    }
    
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::BadFileDescriptor),
    };
    
    // Unsubscribe before closing
    {
        let mut table = FILE_TABLE.lock();
        if let Some(f) = table.get_mut(file_idx) {
            match f.ftype {
                FileType::Pipe | FileType::Device => {
                    let base = crate::process::getpid() as usize | 0x4000_0000;
                    let chan_fd = base ^ (fd as usize);
                    drop(table);
                    file_unsubscribe(file_idx, chan_fd);
                }
                _ => {}
            }
        }
    }
    
    crate::fs::file::file_close(file_idx);
    crate::process::fdclose(fd);
    
    Ok(0)
}

fn sys_lseek_impl(args: &[u64]) -> SyscallResult {
   let args = extract_args(args, 3)?;
   let fd = args[0] as i32;
   let offset = args[1] as i64;
   let whence = args[2] as i32;
   
   if fd < 0 {
       return Err(SyscallError::BadFileDescriptor);
   }
   
   let file_idx = match crate::process::fdlookup(fd) {
       Some(idx) => idx,
       None => return Err(SyscallError::BadFileDescriptor),
   };
   
   let result = file_lseek(file_idx, offset, whence);
   if result < 0 {
       Err(SyscallError::InvalidArgument)
   } else {
       Ok(result as u64)
   }
}

fn sys_fstat_impl(args: &[u64]) -> SyscallResult {
   let args = extract_args(args, 2)?;
   let fd = args[0] as i32;
   let statbuf_ptr = args[1] as *mut crate::posix::stat;
   
   if fd < 0 {
       return Err(SyscallError::BadFileDescriptor);
   }
   
   let file_idx = match crate::process::fdlookup(fd) {
       Some(idx) => idx,
       None => return Err(SyscallError::BadFileDescriptor),
   };
   match file_stat(file_idx) {
       Ok(rust_stat) => {
           // Convert from Rust Stat struct to C stat struct
           let c_stat = crate::posix::stat {
               st_dev: rust_stat.st_dev,
               st_ino: rust_stat.st_ino,
               st_mode: rust_stat.st_mode,
               st_nlink: rust_stat.st_nlink,
               st_uid: rust_stat.st_uid,
               st_gid: rust_stat.st_gid,
               st_rdev: rust_stat.st_rdev,
               st_size: rust_stat.st_size,
               st_blksize: rust_stat.st_blksize,
               st_blocks: rust_stat.st_blocks,
               st_atime: rust_stat.st_atime,
               st_atime_nsec: rust_stat.st_atime_nsec,
               st_mtime: rust_stat.st_mtime,
               st_mtime_nsec: rust_stat.st_mtime_nsec,
               st_ctime: rust_stat.st_ctime,
               st_ctime_nsec: rust_stat.st_ctime_nsec,
           };
           
           unsafe {
               // Copy the C stat structure to user space
               *statbuf_ptr = c_stat;
           }
           Ok(0)
       }
       Err(_) => Err(SyscallError::IoError),
   }
}

/// Implementation of syscall 0x2006: stat
fn sys_stat_impl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let path_ptr = args[0] as *const u8;
    let statbuf_ptr = args[1] as *mut crate::posix::stat;

    // Read path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];

    let pagetable = match crate::process::myproc() {
        Some(pid) => {
            let mut table = crate::process::manager::PROC_TABLE.lock();
            match table.find(pid) {
                Some(proc) => proc.pagetable,
                None => return Err(SyscallError::BadAddress),
            }
        }
        None => return Err(SyscallError::BadAddress),
    };

    let path_len = unsafe {
        crate::subsystems::mm::vm::copyinstr(pagetable,
                                path_ptr as usize,
                                path_buf.as_mut_ptr(),
                                MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Resolve absolute path
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        // Get current working directory from process
        let pid = crate::process::myproc().ok_or(SyscallError::IoError)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SyscallError::IoError)?;
        let cwd = proc.cwd_path.clone().unwrap_or_else(|| "/".to_string());
        format!("{}/{}", cwd, path_str)
    };
    
    // Call VFS stat
    match crate::vfs::stat(&abs_path) {
        Ok(file_attr) => {
            // Convert VFS FileAttr directly to C posix::stat struct
            let c_stat = crate::posix::stat {
                st_dev: 0, // device not implemented in VFS
                st_ino: file_attr.ino as _,
                st_mode: file_attr.mode.0 as _,
                st_nlink: file_attr.nlink as _,
                st_uid: file_attr.uid as _,
                st_gid: file_attr.gid as _,
                st_rdev: file_attr.rdev as _,
                st_size: file_attr.size as _,
                st_blksize: file_attr.blksize as _,
                st_blocks: file_attr.blocks as _,
                st_atime: file_attr.atime as _,
                st_atime_nsec: 0,
                st_mtime: file_attr.mtime as _,
                st_mtime_nsec: 0,
                st_ctime: file_attr.ctime as _,
                st_ctime_nsec: 0,
            };
            
            unsafe {
                // Copy the C stat structure to user space
                *statbuf_ptr = c_stat;
            }
            Ok(0)
        }
        Err(_) => Err(SyscallError::NotFound),
    }
}

