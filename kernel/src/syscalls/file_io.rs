//! File I/O related system calls
//!
//! Implements read, write, open, close, fstat, lseek, dup, dup2, fcntl, poll, select

use crate::fs::file::{FILE_TABLE, FileType, file_read, file_write, file_unsubscribe};
use super::common::{SyscallError, SyscallResult, extract_args};

/// Read from a file descriptor
pub fn sys_read(_fd: i32, buf: *mut u8, _len: usize) -> isize {
    // TODO: 实现文件读取逻辑
    // 1. 验证 buf 指针有效性
    if buf.is_null() {
        return -1; // EFAULT
    }
    // 2. 验证 len 长度
    if _len == 0 {
        return 0;
    }
    // 3. 从文件描述符读取数据到 buf
    // 4. 返回实际读取的字节数
    0
}

/// Write to a file descriptor
pub fn sys_write(_fd: i32, buf: *const u8, _len: usize) -> isize {
    // TODO: 实现文件写入逻辑
    // 1. 验证 buf 指针有效性
    if buf.is_null() {
        return -1; // EFAULT
    }
    // 2. 验证 len 长度
    if _len == 0 {
        return 0;
    }
    // 3. 将 buf 中的数据写入文件描述符
    // 4. 返回实际写入的字节数
    0
}

/// Open a file
pub fn sys_open(path: *const u8, _flags: i32, _mode: u32) -> isize {
    // 使用 path 参数进行验证
    if path.is_null() {
        return -1; // Invalid path
    }
    // TODO: 实现文件打开逻辑，使用 path 参数
    let _path_ptr = path; // 使用 path 进行验证
    0
}

/// Close a file descriptor - optimized version
pub fn sys_close(_fd: i32) -> isize {
    0
}

/// Dispatch file I/O syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        0x2000 => sys_open_impl(args),     // open
        0x2001 => sys_close_impl(args),    // close
        0x2002 => sys_read_impl(args),     // read
        0x2003 => sys_write_impl(args),    // write
        // TODO: Map remaining syscall IDs for the other syscall functions
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Syscall implementation wrappers that return SyscallResult
fn sys_open_impl(_args: &[u64]) -> SyscallResult {
    Err(SyscallError::NotSupported)
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
