//! Dispatch function for filesystem syscalls
//!
//! Routes syscall numbers to appropriate handler functions

use super::handlers;
use crate::syscalls::common::SyscallError;
use crate::syscalls::common::SyscallResult;
use nos_nos_error_handling::unified::KernelError;

/// Dispatch filesystem syscalls to appropriate handlers
pub fn dispatch(syscall_number: u32, args: &[u64]) -> Result<u64, KernelError> {
    match syscall_number {
        0x7000 => handlers::handle_chdir(args),      // chdir
        0x7001 => handlers::handle_fchdir(args),     // fchdir
        0x7002 => handlers::handle_getcwd(args),     // getcwd
        0x7003 => handlers::handle_mkdir(args),     // mkdir
        0x7004 => handlers::handle_rmdir(args),     // rmdir
        0x7005 => handlers::handle_unlink(args),    // unlink
        0x7006 => handlers::handle_rename(args),    // rename
        0x7007 => handlers::handle_link(args),       // link
        0x7008 => handlers::handle_symlink(args),   // symlink
        0x7009 => handlers::handle_readlink(args),  // readlink
        0x700A => handlers::handle_chmod(args),     // chmod
        0x700B => handlers::handle_fchmod(args),    // fchmod
        0x700C => handlers::handle_chown(args),     // chown
        0x700D => handlers::handle_fchown(args),    // fchown
        0x700E => handlers::handle_lchown(args),    // lchown
        0x700F => handlers::handle_umask(args),     // umask
        0x7010 => handlers::handle_stat(args),      // stat
        0x7011 => handlers::handle_lstat(args),     // lstat
        0x7012 => handlers::handle_access(args),    // access
        0x7013 => handlers::handle_readdir(args),   // readdir/getdents
        _ => Err(KernelError::InvalidSyscall),
    }
}

