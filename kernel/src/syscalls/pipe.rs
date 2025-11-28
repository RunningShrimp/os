//! Pipe-related system calls
//!
//! Implements pipe creation

use crate::process;
use crate::file::FILE_TABLE;
use super::{E_OK, E_NOMEM, E_NOSPC, E_BADARG};

/// Create a pipe
pub fn sys_pipe(pipefd: *mut i32) -> isize {
    if pipefd.is_null() {
        return E_BADARG;
    }
    match crate::pipe::pipe_alloc() {
        Some((ridx, widx)) => {
            let rfd = match process::fdalloc(ridx) {
                Some(fd) => fd,
                None => {
                    let mut t = FILE_TABLE.lock();
                    t.close(ridx);
                    t.close(widx);
                    return E_NOMEM;
                }
            };
            let wfd = match process::fdalloc(widx) {
                Some(fd) => fd,
                None => {
                    process::fdclose(rfd);
                    let mut t = FILE_TABLE.lock();
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
