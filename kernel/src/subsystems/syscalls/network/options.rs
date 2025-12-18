//! Socket options syscalls

use crate::syscalls::common::{SyscallError, SyscallResult};

/// Set socket options
pub fn sys_setsockopt(args: &[u64]) -> SyscallResult {
    if args.len() < 5 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let _level = args[1] as i32;
    let _optname = args[2] as i32;
    let _optval = args[3] as *const u8;
    let _optlen = args[4] as usize;

    // Validate file descriptor
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::NotFound),
    };

    // Check if it's a socket
    let ft = crate::fs::file::FILE_TABLE.lock();
    let file = ft.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    if file.ftype != crate::fs::file::FileType::Socket {
        return Err(SyscallError::InvalidArgument);
    }

    Ok(0)
}

/// Get socket options
pub fn sys_getsockopt(args: &[u64]) -> SyscallResult {
    if args.len() < 5 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let _level = args[1] as i32;
    let _optname = args[2] as i32;
    let _optval = args[3] as *mut u8;
    let _optlen = args[4] as *mut usize;

    // Validate file descriptor
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::NotFound),
    };

    // Check if it's a socket
    let ft = crate::fs::file::FILE_TABLE.lock();
    let file = ft.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    if file.ftype != crate::fs::file::FileType::Socket {
        return Err(SyscallError::InvalidArgument);
    }

    Ok(0)
}

/// Get socket name
pub fn sys_getsockname(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getsockname syscall
    Err(SyscallError::NotSupported)
}

/// Get peer name
pub fn sys_getpeername(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getpeername syscall
    Err(SyscallError::NotSupported)
}
