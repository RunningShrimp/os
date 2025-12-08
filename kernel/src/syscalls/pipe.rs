//! Pipe-related system calls
//!
//! Implements pipe creation

use crate::process;
use crate::file::FILE_TABLE;
use crate::reliability::errno::{errno_neg, ENOMEM, ENOSPC, EINVAL};

/// Create a pipe
pub fn sys_pipe(pipefd: *mut i32) -> isize {
    if pipefd.is_null() {
        return errno_neg(EINVAL);
    }
    match crate::ipc::pipe::pipe_alloc() {
        Some((ridx, widx)) => {
            let rfd = match process::fdalloc(ridx) {
                Some(fd) => fd,
                None => {
                    let mut t = FILE_TABLE.lock();
                    t.close(ridx);
                    t.close(widx);
                    return errno_neg(ENOMEM);
                }
            };
            let wfd = match process::fdalloc(widx) {
                Some(fd) => fd,
                None => {
                    process::fdclose(rfd);
                    let mut t = FILE_TABLE.lock();
                    t.close(ridx);
                    t.close(widx);
                    return errno_neg(ENOMEM);
                }
            };
            unsafe {
                *pipefd.add(0) = rfd;
                *pipefd.add(1) = wfd;
            }
            0
        }
        None => return errno_neg(ENOSPC),
    }
}
