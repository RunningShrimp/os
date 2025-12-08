// Filesystem operations syscalls

extern crate alloc;

use super::common::{SyscallError, SyscallResult};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Dispatch filesystem operations syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Filesystem operations
        0x7000 => sys_chdir(args),          // chdir
        0x7001 => sys_fchdir(args),         // fchdir
        0x7002 => sys_getcwd(args),         // getcwd
        0x7003 => sys_mkdir(args),          // mkdir
        0x7004 => sys_rmdir(args),          // rmdir
        0x7005 => sys_unlink(args),         // unlink
        0x7006 => sys_rename(args),         // rename
        0x7007 => sys_link(args),           // link
        0x7008 => sys_symlink(args),        // symlink
        0x7009 => sys_readlink(args),       // readlink
        0x700A => sys_chmod(args),          // chmod
        0x700B => sys_fchmod(args),         // fchmod
        0x700C => sys_chown(args),          // chown
        0x700D => sys_fchown(args),         // fchown
        0x700E => sys_lchown(args),         // lchown
        0x700F => sys_umask(args),          // umask
        0x7010 => sys_stat(args),           // stat
        0x7011 => sys_lstat(args),          // lstat
        0x7012 => sys_access(args),         // access
        0x7013 => sys_mount(args),          // mount
        0x7014 => sys_umount(args),         // umount
        0x7015 => sys_pivot_root(args),     // pivot_root
        _ => Err(SyscallError::InvalidSyscall),
    }
}

// Placeholder implementations - to be replaced with actual syscall logic

/// Change current working directory
/// Arguments: [pathname_ptr]
/// Returns: 0 on success, error on failure
fn sys_chdir(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 1)?;
    let pathname_ptr = args[0] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Check if root file system is mounted
    if !crate::vfs::is_root_mounted() {
        return Err(SyscallError::IoError);
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
        .map_err(|_| SyscallError::NotFound)?;
    
    // Check if it's a directory
    if !attr.mode.is_dir() {
        return Err(SyscallError::NotADirectory);
    }
    
    // Update process's current working directory
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find(pid).ok_or(SyscallError::NotFound)?;
    proc.cwd_path = Some(normalized_path.clone());
    // TODO: Open directory and store file descriptor in proc.cwd
    // For now, we just store the path
    
    Ok(0)
}

/// Change current working directory by file descriptor
/// Arguments: [fd]
/// Returns: 0 on success, error on failure
fn sys_fchdir(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 1)?;
    let fd = args[0] as i32;
    
    // Lookup the file descriptor
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::BadFileDescriptor),
    };
    
    // Get the file
    let table = crate::fs::file::FILE_TABLE.lock();
    let file = match table.get(file_idx) {
        Some(f) => f,
        None => return Err(SyscallError::BadFileDescriptor),
    };
    
    // Check if it's a directory (VFS file)
    if file.ftype != crate::fs::file::FileType::Vfs {
        return Err(SyscallError::NotADirectory);
    }
    
    // Get file attributes to verify it's a directory
    if let Some(ref vfs_file) = file.vfs_file {
        let attr = vfs_file.stat().map_err(|_| SyscallError::InvalidArgument)?;
        if !attr.mode.is_dir() {
            return Err(SyscallError::NotADirectory);
        }
        
        // Get the path from the VFS file (if available)
        // For now, we'll need to track the path separately
        // TODO: Store path in file structure or lookup from inode
    }
    
    // Update process's current working directory
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find(pid).ok_or(SyscallError::NotFound)?;
    proc.cwd = Some(file_idx);
    // TODO: Update cwd_path from file path
    
    Ok(0)
}

/// Get current working directory
/// Arguments: [buf_ptr, size]
/// Returns: number of bytes written on success, error on failure
fn sys_getcwd(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 2)?;
    let buf_ptr = args[0] as usize;
    let size = args[1] as usize;
    
    // Validate buffer size
    if size == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current working directory path
    let cwd = cwd_path.unwrap_or_else(|| String::from("/"));
    let cwd_bytes = cwd.as_bytes();
    
    // Check if buffer is large enough (including null terminator)
    if cwd_bytes.len() + 1 > size {
        return Err(SyscallError::InvalidArgument); // ERANGE would be more appropriate
    }
    
    // Copy path to user buffer
    unsafe {
        copyout(pagetable, buf_ptr, cwd_bytes.as_ptr(), cwd_bytes.len())
            .map_err(|_| SyscallError::BadAddress)?;
        // Null terminate
        copyout(pagetable, buf_ptr + cwd_bytes.len(), [0u8].as_ptr(), 1)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(cwd_bytes.len() as u64)
}

/// Normalize a path by removing . and .. components
fn normalize_path(path: &str) -> String {
    let mut components = Vec::new();
    
    for component in path.split('/') {
        match component {
            "" | "." => continue,
            ".." => {
                if !components.is_empty() && components.last() != Some(&"..") {
                    components.pop();
                } else if !path.starts_with('/') {
                    // Relative path, keep ..
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

/// Create a directory
/// Arguments: [pathname_ptr, mode]
/// Returns: 0 on success, error on failure
fn sys_mkdir(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 2)?;
    let pathname_ptr = args[0] as usize;
    let mode = args[1] as u32;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
            crate::vfs::VfsError::Exists => SyscallError::FileExists,
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::NotDirectory => SyscallError::InvalidArgument,
            _ => SyscallError::IoError,
        })?;
    
    Ok(0)
}

/// Remove a directory
/// Arguments: [pathname_ptr]
/// Returns: 0 on success, error on failure
fn sys_rmdir(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 1)?;
    let pathname_ptr = args[0] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::NotDirectory => SyscallError::InvalidArgument,
            crate::vfs::VfsError::NotEmpty => SyscallError::DirectoryNotEmpty,
            _ => SyscallError::IoError,
        })?;
    
    Ok(0)
}

/// Remove a file (unlink)
/// Arguments: [pathname_ptr]
/// Returns: 0 on success, error on failure
fn sys_unlink(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 1)?;
    let pathname_ptr = args[0] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::IsDirectory => SyscallError::IsADirectory,
            _ => SyscallError::IoError,
        })?;
    
    Ok(0)
}

/// Rename a file or directory
/// Arguments: [oldpath_ptr, newpath_ptr]
/// Returns: 0 on success, error on failure
fn sys_rename(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 2)?;
    let oldpath_ptr = args[0] as usize;
    let newpath_ptr = args[1] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read old pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut old_path_buf = [0u8; MAX_PATH_LEN];
    let old_path_len = unsafe {
        copyinstr(pagetable, oldpath_ptr, old_path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Read new pathname from user space
    let mut new_path_buf = [0u8; MAX_PATH_LEN];
    let new_path_len = unsafe {
        copyinstr(pagetable, newpath_ptr, new_path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to strings
    let old_path_str = core::str::from_utf8(&old_path_buf[..old_path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    let new_path_str = core::str::from_utf8(&new_path_buf[..new_path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Resolve paths (handle relative paths)
    let abs_old_path = if old_path_str.starts_with('/') {
        old_path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, old_path_str)
        } else {
            format!("/{}", old_path_str)
        }
    };
    
    let abs_new_path = if new_path_str.starts_with('/') {
        new_path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, new_path_str)
        } else {
            format!("/{}", new_path_str)
        }
    };
    
    // Rename via VFS
    // We need to use the VFS rename functionality
    // Since VFS doesn't expose a public rename method, we'll need to implement it
    // using the inode operations directly
    let vfs = crate::vfs::vfs();
    
    // Get old inode
    let old_dentry = vfs.lookup_path(&abs_old_path)
        .map_err(|_| SyscallError::NotFound)?;
    let old_inode = old_dentry.lock().inode.clone();
    
    // Split paths manually (since split_path is private)
    let split_path_helper = |path: &str| -> Result<(String, String), SyscallError> {
        if path.is_empty() || path == "/" {
            return Err(SyscallError::InvalidArgument);
        }
        let path = path.trim_end_matches('/');
        if let Some(pos) = path.rfind('/') {
            let parent = if pos == 0 { "/" } else { &path[..pos] };
            let name = &path[pos + 1..];
            Ok((parent.to_string(), name.to_string()))
        } else {
            Ok(("/".to_string(), path.to_string()))
        }
    };
    
    // Split new path into parent and name
    let (new_parent_path, new_name) = split_path_helper(&abs_new_path)?;
    
    // Get new parent directory
    let new_parent_dentry = vfs.lookup_path(&new_parent_path)
        .map_err(|_| SyscallError::NotFound)?;
    let new_parent_inode = new_parent_dentry.lock().inode.clone();
    
    // Get old parent directory
    let (old_parent_path, old_name) = split_path_helper(&abs_old_path)?;
    let old_parent_dentry = vfs.lookup_path(&old_parent_path)
        .map_err(|_| SyscallError::NotFound)?;
    let old_parent_inode = old_parent_dentry.lock().inode.clone();
    
    // Perform rename operation
    old_parent_inode.rename(&old_name, new_parent_inode.as_ref(), &new_name)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::Exists => SyscallError::FileExists,
            crate::vfs::VfsError::NotSupported => SyscallError::NotSupported,
            _ => SyscallError::IoError,
        })?;
    
    // Note: Dentry cache will be updated automatically by VFS layer
    
    Ok(0)
}

/// Create a hard link
/// Arguments: [oldpath_ptr, newpath_ptr]
/// Returns: 0 on success, error on failure
fn sys_link(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 2)?;
    let oldpath_ptr = args[0] as usize;
    let newpath_ptr = args[1] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read old pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut old_path_buf = [0u8; MAX_PATH_LEN];
    let old_path_len = unsafe {
        copyinstr(pagetable, oldpath_ptr, old_path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Read new pathname from user space
    let mut new_path_buf = [0u8; MAX_PATH_LEN];
    let new_path_len = unsafe {
        copyinstr(pagetable, newpath_ptr, new_path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to strings
    let old_path_str = core::str::from_utf8(&old_path_buf[..old_path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    let new_path_str = core::str::from_utf8(&new_path_buf[..new_path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Resolve paths (handle relative paths)
    let abs_old_path = if old_path_str.starts_with('/') {
        old_path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, old_path_str)
        } else {
            format!("/{}", old_path_str)
        }
    };
    
    let abs_new_path = if new_path_str.starts_with('/') {
        new_path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, new_path_str)
        } else {
            format!("/{}", new_path_str)
        }
    };
    
    // Create hard link via VFS
    let vfs = crate::vfs::vfs();
    
    // Get old inode
    let old_dentry = vfs.lookup_path(&abs_old_path)
        .map_err(|_| SyscallError::NotFound)?;
    let old_inode = old_dentry.lock().inode.clone();
    
    // Split new path into parent and name
    let split_path_helper = |path: &str| -> Result<(String, String), SyscallError> {
        if path.is_empty() || path == "/" {
            return Err(SyscallError::InvalidArgument);
        }
        let path = path.trim_end_matches('/');
        if let Some(pos) = path.rfind('/') {
            let parent = if pos == 0 { "/" } else { &path[..pos] };
            let name = &path[pos + 1..];
            Ok((parent.to_string(), name.to_string()))
        } else {
            Ok(("/".to_string(), path.to_string()))
        }
    };
    
    let (new_parent_path, new_name) = split_path_helper(&abs_new_path)?;
    
    // Get new parent directory
    let new_parent_dentry = vfs.lookup_path(&new_parent_path)
        .map_err(|_| SyscallError::NotFound)?;
    let new_parent_inode = new_parent_dentry.lock().inode.clone();
    
    // Create hard link
    new_parent_inode.link(&new_name, old_inode)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::Exists => SyscallError::FileExists,
            crate::vfs::VfsError::NotSupported => SyscallError::NotSupported,
            crate::vfs::VfsError::IsDirectory => SyscallError::IsADirectory,
            _ => SyscallError::IoError,
        })?;
    
    Ok(0)
}

/// Create a symbolic link
/// Arguments: [target_path_ptr, link_path_ptr]
fn sys_symlink(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 2)?;
    let target_path_ptr = args[0] as usize;
    let link_path_ptr = args[1] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read target path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut target_buf = [0u8; MAX_PATH_LEN];
    let target_len = unsafe {
        copyinstr(pagetable, target_path_ptr, target_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Read link path from user space
    let mut link_buf = [0u8; MAX_PATH_LEN];
    let link_len = unsafe {
        copyinstr(pagetable, link_path_ptr, link_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to strings
    let target_path = core::str::from_utf8(&target_buf[..target_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    let link_path_str = core::str::from_utf8(&link_buf[..link_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Resolve link path (handle relative paths)
    let abs_link_path = if link_path_str.starts_with('/') {
        link_path_str.to_string()
    } else {
        if let Some(ref cwd) = cwd_path {
            format!("{}/{}", cwd, link_path_str)
        } else {
            format!("/{}", link_path_str)
        }
    };
    
    // Use VFS to create symbolic link
    // Note: target_path is stored as-is (can be relative or absolute)
    crate::vfs::vfs().symlink(&abs_link_path, target_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::Exists => SyscallError::FileExists,
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::NotSupported => SyscallError::NotSupported,
            _ => SyscallError::IoError,
        })?;
    
    Ok(0)
}

/// Read symbolic link target
/// Arguments: [path_ptr, buf_ptr, bufsize]
fn sys_readlink(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyinstr, copyout};
    
    let args = extract_args(args, 3)?;
    let path_ptr = args[0] as usize;
    let buf_ptr = args[1] as usize;
    let bufsize = args[2] as usize;
    
    // Validate buffer size
    if bufsize == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, path_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
    
    // Read symbolic link target using VFS
    let target = crate::vfs::vfs().readlink(&abs_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            crate::vfs::VfsError::InvalidOperation => SyscallError::InvalidArgument,
            _ => SyscallError::IoError,
        })?;
    
    // Copy target to user buffer
            let target_bytes = target.as_bytes();
    let copy_len = target_bytes.len().min(bufsize - 1); // Leave room for null terminator
            
            unsafe {
        copyout(pagetable, buf_ptr, target_bytes.as_ptr(), copy_len)
            .map_err(|_| SyscallError::BadAddress)?;
        // Null terminate
        copyout(pagetable, buf_ptr + copy_len, [0u8].as_ptr(), 1)
            .map_err(|_| SyscallError::BadAddress)?;
            }
            
            Ok(copy_len as u64)
        }

/// Change file mode (permissions)
/// Arguments: [pathname_ptr, mode]
/// Returns: 0 on success, error on failure
fn sys_chmod(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyinstr, copyin};
    
    let args = extract_args(args, 2)?;
    let pathname_ptr = args[0] as usize;
    let mode = args[1] as u32;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
    
    // Get current process UID
    let current_uid = crate::process::getuid();
    let is_root = current_uid == 0;
    
    // Get file attributes via VFS
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&abs_path)
        .map_err(|_| SyscallError::NotFound)?;
    
    // Check permissions: only owner or root can change file mode
    if !is_root && attr.uid != current_uid {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Preserve file type bits, only change permission bits
    // Mode format: S_IFMT (file type) | permissions (0o777)
    let file_type_mask = attr.mode.0 & crate::vfs::FileMode::S_IFMT;
    let new_mode = file_type_mask | (mode & 0o7777);
    let mut new_attr = attr;
    new_attr.mode = crate::vfs::FileMode::new(new_mode);
    
    // Update file attributes
    // We need to get the file handle to update attributes
    // For now, we'll use a workaround: open the file, update, then close
    let vfs_file = vfs.open(&abs_path, crate::posix::O_RDWR as u32)
        .map_err(|_| SyscallError::PermissionDenied)?;
    
    vfs_file.set_attr(&new_attr)
        .map_err(|_| SyscallError::PermissionDenied)?;
    
    Ok(0)
}

/// Change file mode (permissions) by file descriptor
/// Arguments: [fd, mode]
/// Returns: 0 on success, error on failure
fn sys_fchmod(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 2)?;
    let fd = args[0] as i32;
    let mode = args[1] as u32;
    
    // Validate file descriptor
    if fd < 0 {
        return Err(SyscallError::BadFileDescriptor);
    }
    
    // Get file descriptor index
    let file_idx = crate::process::fdlookup(fd)
        .ok_or(SyscallError::BadFileDescriptor)?;
    
    // Change file mode using helper function
    crate::fs::file::file_chmod(file_idx, mode)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    Ok(0)
}

/// Change file owner and group
/// Arguments: [pathname_ptr, uid, gid]
/// Returns: 0 on success, error on failure
/// Note: uid or gid of -1 means don't change that value
fn sys_chown(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 3)?;
    let pathname_ptr = args[0] as usize;
    let uid = args[1] as u32;
    let gid = args[2] as u32;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
    
    // Get current process UID
    let current_uid = crate::process::getuid();
    let is_root = current_uid == 0;
    
    // Get file attributes via VFS
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&abs_path)
        .map_err(|_| SyscallError::NotFound)?;
    
    // Check permissions: only root can change file ownership
    if !is_root {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Update uid and gid (if not -1, which means don't change)
    // In POSIX, -1 means don't change, but we use u32, so we check for u32::MAX
    let mut new_attr = attr;
    if uid != u32::MAX {
        new_attr.uid = uid;
    }
    if gid != u32::MAX {
        new_attr.gid = gid;
    }
    
    // Update file attributes
    let vfs_file = vfs.open(&abs_path, crate::posix::O_RDWR as u32)
        .map_err(|_| SyscallError::PermissionDenied)?;
    
    vfs_file.set_attr(&new_attr)
        .map_err(|_| SyscallError::PermissionDenied)?;
    
    Ok(0)
}

/// Change file owner and group by file descriptor
/// Arguments: [fd, uid, gid]
/// Returns: 0 on success, error on failure
/// Note: uid or gid of -1 means don't change that value
fn sys_fchown(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 3)?;
    let fd = args[0] as i32;
    let uid = args[1] as u32;
    let gid = args[2] as u32;
    
    // Validate file descriptor
    if fd < 0 {
        return Err(SyscallError::BadFileDescriptor);
    }
    
    // Get file descriptor index
    let file_idx = crate::process::fdlookup(fd)
        .ok_or(SyscallError::BadFileDescriptor)?;
    
    // Change file owner using helper function
    // Note: file_chown expects both uid and gid, so we need to handle -1 case
    // For now, we'll get current values if -1 is passed
    let current_uid = if uid == u32::MAX {
        // Get current uid from file
        let file_table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = file_table.get(file_idx) {
            if let Some(ref vfs_file) = file.vfs_file {
                if let Ok(attr) = vfs_file.stat() {
                    attr.uid
                } else {
                    return Err(SyscallError::InvalidArgument);
                }
            } else {
                return Err(SyscallError::InvalidArgument);
            }
        } else {
            return Err(SyscallError::BadFileDescriptor);
        }
    } else {
        uid
    };
    
    let current_gid = if gid == u32::MAX {
        // Get current gid from file
        let file_table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = file_table.get(file_idx) {
            if let Some(ref vfs_file) = file.vfs_file {
                if let Ok(attr) = vfs_file.stat() {
                    attr.gid
                } else {
                    return Err(SyscallError::InvalidArgument);
                }
            } else {
                return Err(SyscallError::InvalidArgument);
            }
        } else {
            return Err(SyscallError::BadFileDescriptor);
        }
    } else {
        gid
    };
    
    crate::fs::file::file_chown(file_idx, current_uid, current_gid)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    Ok(0)
}

/// Change file owner and group (don't follow symbolic links)
/// Arguments: [pathname_ptr, uid, gid]
/// Returns: 0 on success, error on failure
/// Note: uid or gid of -1 means don't change that value
/// This is the same as chown but doesn't follow symlinks
fn sys_lchown(args: &[u64]) -> SyscallResult {
    // For now, lchown is the same as chown
    // In a full implementation, we would check if the path is a symlink
    // and operate on the symlink itself rather than the target
    sys_chown(args)
}

fn sys_umask(_args: &[u64]) -> SyscallResult {
    // TODO: Implement umask syscall
    Err(SyscallError::NotSupported)
}

/// Get file status (follows symbolic links)
/// Arguments: [pathname_ptr, statbuf_ptr]
/// Returns: 0 on success, error on failure
fn sys_stat(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyinstr, copyout};
    
    let args = extract_args(args, 2)?;
    let pathname_ptr = args[0] as usize;
    let statbuf_ptr = args[1] as *mut crate::posix::stat;
    
    if statbuf_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
    
    // Get file attributes via VFS (follows symbolic links)
    let vfs = crate::vfs::vfs();
    let attr = vfs.stat(&abs_path)
        .map_err(|e| match e {
            crate::vfs::VfsError::NotFound => SyscallError::NotFound,
            _ => SyscallError::IoError,
        })?;
    
    // Convert FileAttr to POSIX stat structure
    let stat_buf = file_attr_to_stat(&attr);
    
    // Copy stat structure to user space
    unsafe {
        copyout(pagetable, statbuf_ptr as usize, &stat_buf as *const _ as *const u8, core::mem::size_of::<crate::posix::stat>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

/// Get file status (does not follow symbolic links)
/// Arguments: [pathname_ptr, statbuf_ptr]
/// Returns: 0 on success, error on failure
fn sys_lstat(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyinstr, copyout};
    
    let args = extract_args(args, 2)?;
    let pathname_ptr = args[0] as usize;
    let statbuf_ptr = args[1] as *mut crate::posix::stat;
    
    if statbuf_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    let cwd_path = proc.cwd_path.clone();
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let path_str = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
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
    
    // Get file attributes via VFS (does not follow symbolic links)
    // For now, we use the same lookup as stat, but in a full implementation
    // we would check if the path is a symlink and return its attributes directly
    let vfs = crate::vfs::vfs();
    
    // Try to lookup the path without following symlinks
    // Since VFS doesn't expose a direct lstat, we'll use lookup_path
    // which should give us the dentry without following symlinks
    let dentry = vfs.lookup_path(&abs_path)
        .map_err(|_| SyscallError::NotFound)?;
    
    let attr = dentry.lock().inode.getattr()
        .map_err(|_| SyscallError::IoError)?;
    
    // Convert FileAttr to POSIX stat structure
    let stat_buf = file_attr_to_stat(&attr);
    
    // Copy stat structure to user space
    unsafe {
        copyout(pagetable, statbuf_ptr as usize, &stat_buf as *const _ as *const u8, core::mem::size_of::<crate::posix::stat>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
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

fn sys_access(_args: &[u64]) -> SyscallResult {
    // TODO: Implement access syscall
    Err(SyscallError::NotSupported)
}

/// Mount a filesystem
/// Arguments: [source_ptr, target_ptr, fstype_ptr, flags, data_ptr]
/// Returns: 0 on success, error on failure
fn sys_mount(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 5)?;
    let source_ptr = args[0] as usize;
    let target_ptr = args[1] as usize;
    let fstype_ptr = args[2] as usize;
    let flags = args[3] as u32;
    let _data_ptr = args[4] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Only root can mount filesystems
    let current_uid = crate::process::getuid();
    if current_uid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Read target path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut target_buf = [0u8; MAX_PATH_LEN];
    let target_len = unsafe {
        copyinstr(pagetable, target_ptr, target_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let target_str = core::str::from_utf8(&target_buf[..target_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Read filesystem type from user space
    let mut fstype_buf = [0u8; 32];
    let fstype_len = unsafe {
        copyinstr(pagetable, fstype_ptr, fstype_buf.as_mut_ptr(), 32)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    let fstype_str = if fstype_len > 0 {
        Some(core::str::from_utf8(&fstype_buf[..fstype_len])
            .map_err(|_| SyscallError::InvalidArgument)?)
    } else {
        None
    };
    
    // Resolve target path (handle relative paths)
    let abs_target = if target_str.starts_with('/') {
        target_str.to_string()
    } else {
        let cwd_path = proc.cwd_path.as_ref().map(|s| s.as_str()).unwrap_or("/");
        if cwd_path == "/" {
            format!("/{}", target_str)
        } else {
            format!("{}/{}", cwd_path, target_str)
        }
    };
    
    // For now, we only support mounting a simple filesystem at a target
    // In a full implementation, this would:
    // 1. Parse source device
    // 2. Identify filesystem type
    // 3. Mount the filesystem at the target
    // 4. Update VFS mount table
    
    // Simplified implementation: just create the target directory if it doesn't exist
    let vfs = crate::vfs::vfs();
    
    // Check if target already exists
    match vfs.stat(&abs_target) {
        Ok(_) => return Err(SyscallError::FileExists),
        Err(crate::vfs::VfsError::NotFound) => {
            // Target doesn't exist, create it as a directory
            let mode = crate::vfs::FileMode::new(0o755); // rwxr-xr-x
            vfs.mkdir(&abs_target, mode)
                .map_err(|e| match e {
                    crate::vfs::VfsError::Exists => SyscallError::FileExists,
                    crate::vfs::VfsError::NotFound => SyscallError::NotFound,
                    crate::vfs::VfsError::NotDirectory => SyscallError::InvalidArgument,
                    _ => SyscallError::IoError,
                })?;
        }
        Err(_) => return Err(SyscallError::IoError),
    }
    
    // Log the mount operation
    crate::println!("[mount] Mounted {:?} at {} with flags 0x{:x}",
        fstype_str, abs_target, flags);
    
    Ok(0)
}

/// Unmount a filesystem
/// Arguments: [target_ptr]
/// Returns: 0 on success, error on failure
fn sys_umount(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyinstr;
    
    let args = extract_args(args, 1)?;
    let target_ptr = args[0] as usize;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Only root can unmount filesystems
    let current_uid = crate::process::getuid();
    if current_uid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Read target path from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut target_buf = [0u8; MAX_PATH_LEN];
    let target_len = unsafe {
        copyinstr(pagetable, target_ptr, target_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    
    // Convert to string
    let target_str = core::str::from_utf8(&target_buf[..target_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Resolve target path (handle relative paths)
    let abs_target = if target_str.starts_with('/') {
        target_str.to_string()
    } else {
        let cwd_path = proc.cwd_path.as_ref().map(|s| s.as_str()).unwrap_or("/");
        if cwd_path == "/" {
            format!("/{}", target_str)
        } else {
            format!("{}/{}", cwd_path, target_str)
        }
    };
    
    // For now, we just check if the target exists and is a directory
    // In a full implementation, this would:
    // 1. Check if target is a mount point
    // 2. Unmount the filesystem
    // 3. Update VFS mount table
    // 4. Clean up any resources
    
    let vfs = crate::vfs::vfs();
    
    // Check if target exists and is a directory
    match vfs.stat(&abs_target) {
        Ok(attr) => {
            if !attr.mode.is_dir() {
                return Err(SyscallError::NotADirectory);
            }
            
            // For now, just log the unmount operation
            // In a real implementation, we would unmount the filesystem
            crate::println!("[umount] Unmounted {}", abs_target);
            Ok(0)
        }
        Err(crate::vfs::VfsError::NotFound) => {
            Err(SyscallError::NotFound)
        }
        Err(_) => {
            Err(SyscallError::IoError)
        }
    }
}

fn sys_pivot_root(_args: &[u64]) -> SyscallResult {
    // TODO: Implement pivot_root syscall
    Err(SyscallError::NotSupported)
}

/// Mount a filesystem (exported function)
pub fn mount(_fs_type: &str, _mount_point: &str, _device: Option<&str>, _flags: u32) -> Result<(), i32> {
    // TODO: Implement mount using VFS
    Err(crate::reliability::errno::ENOSYS)
}
