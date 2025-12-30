//! File I/O related system calls
//!
//! Implements read, write, open, close, fstat, lseek, dup, dup2, fcntl, poll, select

use crate::fs::file::{FILE_TABLE, FileType, file_close, file_unsubscribe};
use crate::subsystems::sync::Mutex;
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

    pub fn record_close(&self) {
        self.close_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// 获取I/O统计信息
/// Read from a file descriptor

/// Write to a file descriptor


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


