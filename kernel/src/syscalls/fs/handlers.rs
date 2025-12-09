//! File System System Call Handlers
//!
//! This module contains the actual system call handler functions for filesystem operations.
//! These handlers are migrated from the original fs.rs implementation and adapted
//! for the new modular service architecture.

use super::types::*;
use crate::syscalls::common::{SyscallError, SyscallResult};
use crate::error_handling::unified::KernelError;
use alloc::string::ToString;

/// Handle chdir system call - change current working directory
pub fn handle_chdir(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;

    // Get current process
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| KernelError::InvalidArgument)?;

    // Check if root file system is mounted
    if !crate::vfs::is_root_mounted() {
        return Err(KernelError::IoError);
    }

    // Resolve path (handle relative paths)
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, path_str)
        } else {
            format!("/{}", path_str)
        }
    };

    // Normalize path (remove . and .. components)
    let normalized_path = normalize_path(&abs_path);

    // Verify that the path exists and is a directory
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&normalized_path)
        .map_err(|_| KernelError::NotFound)?;

    // Check if it's a directory
    if !attr.mode.is_dir() {
        return Err(KernelError::NotADirectory);
    }

    // Update process's current working directory
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find(pid).ok_or(KernelError::NotFound)?;
    proc.cwd_path = Some(normalized_path);

    Ok(0)
}

/// Handle fchdir system call - change working directory by file descriptor
pub fn handle_fchdir(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let fd = args[0] as i32;

    // Lookup the file descriptor
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(KernelError::BadFileDescriptor),
    };

    // Get the file
    let table = crate::fs::file::FILE_TABLE.lock();
    let file = match table.get(file_idx) {
        Some(f) => f,
        None => return Err(KernelError::BadFileDescriptor),
    };

    // Check if it's a directory (VFS file)
    if file.ftype != crate::fs::file::FileType::Vfs {
        return Err(KernelError::NotADirectory);
    }

    // Update process's current working directory
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find(pid).ok_or(KernelError::NotFound)?;
    proc.cwd = Some(file_idx);

    Ok(0)
}

/// Handle getcwd system call - get current working directory
pub fn handle_getcwd(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let buf_ptr = args[0] as usize;
    let size = args[1] as usize;

    // Validate buffer size
    if size == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Get current process
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Get current working directory path
    let cwd = cwd_path.unwrap_or_else(|| alloc::string::String::from("/"));
    let cwd_bytes = cwd.as_bytes();

    // Check if buffer is large enough (including null terminator)
    if cwd_bytes.len() + 1 > size {
        return Err(KernelError::InvalidArgument); // ERANGE would be more appropriate
    }

    // Copy path to user buffer
    unsafe {
        crate::mm::vm::copyout(pagetable as *mut crate::mm::vm::PageTable, buf_ptr, cwd_bytes.as_ptr(), cwd_bytes.len())
            .map_err(|_| KernelError::BadAddress)?;
        // Null terminate
        crate::mm::vm::copyout(pagetable as *mut crate::mm::vm::PageTable, buf_ptr + cwd_bytes.len(), [0u8].as_ptr(), 1)
            .map_err(|_| KernelError::BadAddress)?;
    }

    Ok(cwd_bytes.len() as u64)
}

/// Handle mkdir system call - create a directory
pub fn handle_mkdir(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;
    let mode = args[1] as u32;

    // Get current process
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| KernelError::InvalidArgument)?;

    // Resolve path (handle relative paths)
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, path_str)
        } else {
            format!("/{}", path_str)
        }
    };

    // Create directory via VFS
    let vfs = crate::vfs::vfs();
    let file_mode = crate::vfs::FileMode::new(mode);
    vfs.mkdir(&abs_path, file_mode)
        .map_err(|e| match e {
            crate::vfs::VfsError::Exists => KernelError::FileExists,
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::NotDirectory => KernelError::InvalidArgument,
            _ => KernelError::IoError,
        })?;

    Ok(0)
}

/// Handle rmdir system call - remove a directory
pub fn handle_rmdir(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;

    // Get current process
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| KernelError::InvalidArgument)?;

    // Resolve path (handle relative paths)
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, path_str)
        } else {
            format!("/{}", path_str)
        }
    };

    // Remove directory via VFS
    let vfs = crate::vfs::vfs();
    vfs.rmdir(&abs_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::NotDirectory => KernelError::InvalidArgument,
            crate::vfs::VfsError::NotEmpty => KernelError::DirectoryNotEmpty,
            _ => KernelError::IoError,
        })?;

    Ok(0)
}

/// Handle unlink system call - remove a file
pub fn handle_unlink(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;

    // Get current process
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| KernelError::InvalidArgument)?;

    // Resolve path (handle relative paths)
    let abs_path = if path_str.starts_with('/') {
        path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, path_str)
        } else {
            format!("/{}", path_str)
        }
    };

    // Remove file via VFS
    let vfs = crate::vfs::vfs();
    vfs.unlink(&abs_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::IsDirectory => KernelError::IsADirectory,
            _ => KernelError::IoError,
        })?;

    Ok(0)
}

/// Handle rename system call - rename/move a file or directory
pub fn handle_rename(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let oldpath_ptr = args[0] as usize;
    let newpath_ptr = args[1] as usize;

    // Get current process
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Read old and new pathnames from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut old_path_buf = [0u8; MAX_PATH_LEN];
    let mut new_path_buf = [0u8; MAX_PATH_LEN];

    let old_path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, oldpath_ptr, old_path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    let new_path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, newpath_ptr, new_path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    // Convert to strings
    let old_path_str = core::str::from_utf8(&old_path_buf[..old_path_len])
        .map_err(|_| KernelError::InvalidArgument)?;

    let new_path_str = core::str::from_utf8(&new_path_buf[..new_path_len])
        .map_err(|_| KernelError::InvalidArgument)?;

    // Resolve paths (handle relative paths)
    let abs_old_path = resolve_path(old_path_str, &cwd_path);
    let abs_new_path = resolve_path(new_path_str, &cwd_path);

    // Get old inode for rename operation
    let vfs = crate::vfs::vfs();
    let old_dentry = vfs.lookup_path(&abs_old_path)
        .map_err(|_| KernelError::NotFound)?;
    let old_inode = old_dentry.lock().inode.clone();

    // Split paths
    let (new_parent_path, new_name) = split_path(&abs_new_path)?;
    let (old_parent_path, old_name) = split_path(&abs_old_path)?;

    // Get parent directories
    let new_parent_dentry = vfs.lookup_path(&new_parent_path)
        .map_err(|_| KernelError::NotFound)?;
    let new_parent_inode = new_parent_dentry.lock().inode.clone();

    let old_parent_dentry = vfs.lookup_path(&old_parent_path)
        .map_err(|_| KernelError::NotFound)?;
    let old_parent_inode = old_parent_dentry.lock().inode.clone();

    // Perform rename operation
    old_parent_inode.rename(&old_name, new_parent_inode.as_ref(), &new_name)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::Exists => KernelError::FileExists,
            crate::vfs::VfsError::NotSupported => KernelError::NotSupported,
            _ => KernelError::IoError,
        })?;

    Ok(0)
}

/// Handle link system call - create a hard link
pub fn handle_link(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let oldpath_ptr = args[0] as usize;
    let newpath_ptr = args[1] as usize;

    // Get current process details
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);

    if pagetable.is_null() {
        return Err(KernelError::BadAddress);
    }

    // Read paths from user space and resolve them
    let abs_old_path = read_and_resolve_path(pagetable as usize, oldpath_ptr, &cwd_path)?;
    let abs_new_path = read_and_resolve_path(pagetable as usize, newpath_ptr, &cwd_path)?;

    // Get old inode
    let vfs = crate::vfs::vfs();
    let old_dentry = vfs.lookup_path(&abs_old_path)
        .map_err(|_| KernelError::NotFound)?;
    let old_inode = old_dentry.lock().inode.clone();

    // Split new path and get parent directory
    let (new_parent_path, new_name) = split_path(&abs_new_path)?;
    let new_parent_dentry = vfs.lookup_path(&new_parent_path)
        .map_err(|_| KernelError::NotFound)?;
    let new_parent_inode = new_parent_dentry.lock().inode.clone();

    // Create hard link
    new_parent_inode.link(&new_name, old_inode)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::Exists => KernelError::FileExists,
            crate::vfs::VfsError::NotSupported => KernelError::NotSupported,
            crate::vfs::VfsError::IsDirectory => KernelError::IsADirectory,
            _ => KernelError::IoError,
        })?;

    Ok(0)
}

/// Handle symlink system call - create a symbolic link
pub fn handle_symlink(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let target_path_ptr = args[0] as usize;
    let link_path_ptr = args[1] as usize;

    // Get process context
    let (pagetable, cwd_path) = get_process_context()?;

    // Read and resolve paths
    let target_path = read_path_from_user(pagetable as usize, target_path_ptr)?;
    let abs_link_path = read_and_resolve_path(pagetable as usize, link_path_ptr, &cwd_path)?;

    // Create symbolic link
    crate::vfs::vfs().symlink(&abs_link_path, &target_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::Exists => KernelError::FileExists,
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::NotSupported => KernelError::NotSupported,
            _ => KernelError::IoError,
        })?;

    Ok(0)
}

/// Handle readlink system call - read symbolic link target
pub fn handle_readlink(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 3 {
        return Err(KernelError::InvalidArgument);
    }

    let path_ptr = args[0] as usize;
    let buf_ptr = args[1] as usize;
    let bufsize = args[2] as usize;

    // Validate buffer size
    if bufsize == 0 {
        return Err(KernelError::InvalidArgument);
    }

    // Get process context
    let (pagetable, cwd_path) = get_process_context()?;

    // Read and resolve path
    let abs_path = read_and_resolve_path(pagetable as usize, path_ptr, &cwd_path)?;

    // Read symbolic link target
    let target = crate::vfs::vfs().readlink(&abs_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => KernelError::NotFound,
            crate::vfs::VfsError::InvalidOperation => KernelError::InvalidArgument,
            _ => KernelError::IoError,
        })?;

    // Copy target to user buffer
    let target_bytes = target.as_bytes();
    let copy_len = target_bytes.len().min(bufsize - 1);

    unsafe {
        crate::mm::vm::copyout(pagetable as *mut crate::mm::vm::PageTable, buf_ptr, target_bytes.as_ptr(), copy_len)
            .map_err(|_| KernelError::BadAddress)?;
        // Null terminate
        crate::mm::vm::copyout(pagetable as *mut crate::mm::vm::PageTable, buf_ptr + copy_len, [0u8].as_ptr(), 1)
            .map_err(|_| KernelError::BadAddress)?;
    }

    Ok(copy_len as u64)
}

/// Handle chmod system call - change file permissions
pub fn handle_chmod(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;
    let mode = args[1] as u32;

    // Get process context and permissions
    let (pagetable, cwd_path) = get_process_context()?;
    let current_uid = crate::process::getuid();
    let is_root = current_uid == 0;

    // Read and resolve path
    let abs_path = read_and_resolve_path(pagetable as usize, pathname_ptr, &cwd_path)?;

    // Check permissions
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&abs_path).map_err(|_| KernelError::NotFound)?;

    // Only owner or root can change file mode
    if !is_root && attr.uid != current_uid {
        return Err(KernelError::PermissionDenied);
    }

    // Update file permissions
    let file_mode = crate::vfs::FileMode::new((attr.mode.0 & 0o170000) | (mode & 0o7777));
    let mut new_attr = attr;
    new_attr.mode = file_mode;

    let vfs_file = vfs.open(&abs_path, crate::posix::O_RDWR as u32)
        .map_err(|_| KernelError::PermissionDenied)?;

    vfs_file.set_attr(&new_attr).map_err(|_| KernelError::PermissionDenied)?;

    Ok(0)
}

/// Handle fchmod system call - change file permissions by descriptor
pub fn handle_fchmod(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let mode = args[1] as u32;

    // Validate file descriptor
    if fd < 0 {
        return Err(KernelError::BadFileDescriptor);
    }

    // Get file and change mode
    let file_idx = crate::process::fdlookup(fd).ok_or(KernelError::BadFileDescriptor)?;
    crate::fs::file::file_chmod(file_idx, mode).map_err(|_| KernelError::InvalidArgument)?;

    Ok(0)
}

/// Handle chown system call - change file ownership
pub fn handle_chown(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 3 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;
    let uid = args[1] as u32;
    let gid = args[2] as u32;

    // Only root can change ownership
    let current_uid = crate::process::getuid();
    if current_uid != 0 {
        return Err(KernelError::PermissionDenied);
    }

    let (pagetable, cwd_path) = get_process_context()?;
    let abs_path = read_and_resolve_path(pagetable as usize, pathname_ptr, &cwd_path)?;

    // Get and update attributes
    let vfs = crate::vfs::vfs();
    let mut attr = vfs.stat(&abs_path).map_err(|_| KernelError::NotFound)?;

    if uid != u32::MAX {
        attr.uid = uid;
    }
    if gid != u32::MAX {
        attr.gid = gid;
    }

    let vfs_file = vfs.open(&abs_path, crate::posix::O_RDWR as u32)
        .map_err(|_| KernelError::PermissionDenied)?;

    vfs_file.set_attr(&attr).map_err(|_| KernelError::PermissionDenied)?;

    Ok(0)
}

/// Handle fchown system call - change ownership by descriptor
pub fn handle_fchown(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 3 {
        return Err(KernelError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let uid = args[1] as u32;
    let gid = args[2] as u32;

    // Validate descriptor and change ownership
    if fd < 0 {
        return Err(KernelError::BadFileDescriptor);
    }

    let file_idx = crate::process::fdlookup(fd).ok_or(KernelError::BadFileDescriptor)?;
    crate::fs::file::file_chown(file_idx, uid, gid).map_err(|_| KernelError::InvalidArgument)?;

    Ok(0)
}

/// Handle umask system call - set and get file creation mask
pub fn handle_umask(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let new_mask = args[0] as u32 & 0o777;

    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let mut table = crate::process::manager::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(KernelError::NotFound)?;

    let old_mask = proc.umask;
    proc.umask = new_mask;

    Ok(old_mask as u64)
}

/// Handle stat system call - get file status (follows symlinks)
pub fn handle_stat(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;
    let statbuf_ptr = args[1] as *mut crate::posix::stat;

    if statbuf_ptr.is_null() {
        return Err(KernelError::BadAddress);
    }

    let (pagetable, cwd_path) = get_process_context()?;
    let abs_path = read_and_resolve_path(pagetable as usize, pathname_ptr, &cwd_path)?;

    // Get file attributes
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&abs_path).map_err(|_| KernelError::NotFound)?;

    // Convert to POSIX stat and copy out
    let stat_buf = file_attr_to_stat(&attr);
    unsafe {
        crate::mm::vm::copyout(pagetable as *mut crate::mm::vm::PageTable, statbuf_ptr as usize,
            &stat_buf as *const _ as *const u8, core::mem::size_of::<crate::posix::stat>())
            .map_err(|_| KernelError::BadAddress)?;
    }

    Ok(0)
}

/// Handle lstat system call - get file status (doesn't follow symlinks)
pub fn handle_lstat(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;
    let statbuf_ptr = args[1] as *mut crate::posix::stat;

    if statbuf_ptr.is_null() {
        return Err(KernelError::BadAddress);
    }

    let (pagetable, cwd_path) = get_process_context()?;
    let abs_path = read_and_resolve_path(pagetable as usize, pathname_ptr, &cwd_path)?;

    // Get file attributes without following symlinks
    let vfs = crate::vfs::vfs();
    let dentry = vfs.lookup_path(&abs_path).map_err(|_| KernelError::NotFound)?;
    let attr = dentry.lock().inode.getattr().map_err(|_| KernelError::IoError)?;

    // Convert to POSIX stat and copy out
    let stat_buf = file_attr_to_stat(&attr);
    unsafe {
        crate::mm::vm::copyout(pagetable as *mut crate::mm::vm::PageTable, statbuf_ptr as usize,
            &stat_buf as *const _ as *const u8, core::mem::size_of::<crate::posix::stat>())
            .map_err(|_| KernelError::BadAddress)?;
    }

    Ok(0)
}

/// Handle access system call - check file access permissions
pub fn handle_access(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 2 {
        return Err(KernelError::InvalidArgument);
    }

    let pathname_ptr = args[0] as usize;
    let mode = args[1] as i32;

    let (pagetable, cwd_path, uid, gid) = get_process_context_full()?;
    let abs_path = read_and_resolve_path(pagetable as usize, pathname_ptr, &cwd_path)?;

    // F_OK check
    if mode == crate::posix::F_OK {
        return Ok(0);
    }

    // Get file attributes and check permissions
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&abs_path).map_err(|_| KernelError::NotFound)?;

    let file_mode = attr.mode.0;
    let file_uid = attr.uid;
    let file_gid = attr.gid;

    // Determine permission bits to check
    let (r_bit, w_bit, x_bit) = if uid == 0 {
        // Root has access to everything
        if (mode & crate::posix::X_OK) != 0 && (file_mode & 0o111) == 0 {
            return Err(KernelError::PermissionDenied);
        }
        return Ok(0);
    } else if uid == file_uid {
        (0o400, 0o200, 0o100)
    } else if gid == file_gid {
        (0o040, 0o020, 0o010)
    } else {
        (0o004, 0o002, 0o001)
    };

    // Check requested permissions
    if mode & crate::posix::R_OK != 0 && file_mode & r_bit == 0 {
        return Err(KernelError::PermissionDenied);
    }
    if mode & crate::posix::W_OK != 0 && file_mode & w_bit == 0 {
        return Err(KernelError::PermissionDenied);
    }
    if mode & crate::posix::X_OK != 0 && file_mode & x_bit == 0 {
        return Err(KernelError::PermissionDenied);
    }

    Ok(0)
}

// Helper Functions

/// Normalize a path by removing . and .. components
fn normalize_path(path: &str) -> alloc::string::String {
    let mut components = alloc::vec::Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => continue,
            ".." => {
                if !components.is_empty() && components.last() != Some(&"..") {
                    components.pop();
                } else if !path.starts_with('/') {
                    components.push("..");
                }
            }
            _ => components.push(component),
        }
    }

    if path.starts_with('/') {
        format!("/{}", components.join("/"))
    } else if components.is_empty() {
        ".".to_string()
    } else {
        components.join("/")
    }
}

/// Resolve a path to absolute path given current working directory
fn resolve_path(path: &str, cwd: &Option<alloc::string::String>) -> alloc::string::String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        if let Some(ref cwd_path) = cwd {
            format!("{}/{}", cwd_path, path)
        } else {
            format!("/{}", path)
        }
    }
}

/// Split a path into parent directory and file name
fn split_path(path: &str) -> Result<(alloc::string::String, alloc::string::String), KernelError> {
    if path.is_empty() || path == "/" {
        return Err(KernelError::InvalidArgument);
    }

    let path = path.trim_end_matches('/');
    if let Some(pos) = path.rfind('/') {
        let parent = if pos == 0 { "/" } else { &path[..pos] };
        let name = &path[pos + 1..];
        Ok((parent.to_string(), name.to_string()))
    } else {
        Ok(("/".to_string(), path.to_string()))
    }
}

/// Get current process context (pagetable and cwd)
fn get_process_context() -> Result<(usize, Option<alloc::string::String>), KernelError> {
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable as usize;
    let cwd_path = proc.cwd_path.clone();

    if pagetable == 0 {
        return Err(KernelError::BadAddress);
    }

    Ok((pagetable, cwd_path))
}

/// Get current process context with UID/GID
fn get_process_context_full() -> Result<(usize, Option<alloc::string::String>, u32, u32), KernelError> {
    let pid = crate::process::myproc().ok_or(KernelError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(KernelError::NotFound)?;
    let pagetable = proc.pagetable as usize;
    let cwd_path = proc.cwd_path.clone();
    let uid = proc.uid;
    let gid = proc.gid;

    if pagetable == 0 {
        return Err(KernelError::BadAddress);
    }

    Ok((pagetable, cwd_path, uid, gid))
}

/// Read a path string from user space
fn read_path_from_user(pagetable: usize, ptr: usize) -> Result<alloc::string::String, KernelError> {
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];

    let path_len = unsafe {
        crate::mm::vm::copyinstr(pagetable as *mut crate::mm::vm::PageTable, ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| KernelError::BadAddress)?
    };

    core::str::from_utf8(&path_buf[..path_len])
        .map(|s| s.to_string())
        .map_err(|_| KernelError::InvalidArgument)
}

/// Read and resolve a path from user space
fn read_and_resolve_path(pagetable: usize, ptr: usize, cwd: &Option<alloc::string::String>) -> Result<alloc::string::String, KernelError> {
    let path_str = read_path_from_user(pagetable, ptr)?;
    Ok(resolve_path(&path_str, cwd))
}

/// Convert VFS FileAttr to POSIX stat structure
fn file_attr_to_stat(attr: &crate::vfs::types::FileAttr) -> crate::posix::stat {
    crate::posix::stat {
        st_dev: 0,  // Device ID (not implemented)
        st_ino: attr.ino,
        st_mode: attr.mode.0,
        st_nlink: attr.nlink as u64,
        st_uid: attr.uid,
        st_gid: attr.gid,
        st_rdev: attr.rdev,
        st_size: attr.size as i64,
        st_blksize: attr.blksize as i64,
        st_blocks: attr.blocks as i64,
        st_atime: (attr.atime / 1_000_000_000) as i64,  // Convert nanoseconds to seconds
        st_atime_nsec: (attr.atime % 1_000_000_000) as i64,
        st_mtime: (attr.mtime / 1_000_000_000) as i64,
        st_mtime_nsec: (attr.mtime % 1_000_000_000) as i64,
        st_ctime: (attr.ctime / 1_000_000_000) as i64,
        st_ctime_nsec: (attr.ctime % 1_000_000_000) as i64,
    }
}

/// Get list of filesystem syscalls supported by this module
pub fn get_supported_syscalls() -> alloc::vec::Vec<u32> {
    alloc::vec![
        0x7000, // chdir
        0x7001, // fchdir
        0x7002, // getcwd
        0x7003, // mkdir
        0x7004, // rmdir
        0x7005, // unlink
        0x7006, // rename
        0x7007, // link
        0x7008, // symlink
        0x7009, // readlink
        0x700A, // chmod
        0x700B, // fchmod
        0x700C, // chown
        0x700D, // fchown
        0x700E, // lchown
        0x700F, // umask
        0x7010, // stat
        0x7011, // lstat
        0x7012, // access
    ]
}