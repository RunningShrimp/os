//! Filesystem-related system calls
//!
//! Implements chdir, getcwd, mkdir, rmdir, mknod, link, unlink

extern crate alloc;

use crate::process;
use crate::posix;
use crate::file::{FILE_TABLE, FileType, file_alloc, file_close};
use super::{E_OK, E_BADF, E_MFILE, E_NOENT, E_EXIST, E_NOTDIR, E_ISDIR, E_IO, E_NOSPC, E_NOSYS, E_PERM, E_BUSY, E_NOTEMPTY, E_FAULT, E_NOMEM, E_BADARG, copy_path, join_path, resolve_with_cwd};

/// Change current working directory
pub fn sys_chdir(path: *const u8) -> isize {
    let in_path = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    let abs_path = {
        let mut ptable = crate::process::PROC_TABLE.lock();
        let cur = match crate::process::myproc().and_then(|pid| ptable.find(pid).and_then(|p| p.cwd_path.clone())) {
            Some(s) => s,
            None => alloc::string::String::from("/"),
        };
        join_path(&cur, &in_path)
    };
    
    // Open directory
    match crate::vfs::vfs().open(&abs_path, posix::O_RDONLY as u32) {
        Ok(vfs_file) => {
            // Check if directory
            match vfs_file.stat() {
                Ok(attr) => {
                    if !crate::vfs::FileMode::new(attr.mode.0).file_type().eq(&crate::vfs::FileType::Directory) {
                        return E_NOTDIR;
                    }
                }
                Err(_) => return E_IO,
            }
            
            // Allocate file in FILE_TABLE
            let fd = match file_alloc() {
                Some(fd) => fd,
                None => return E_MFILE,
            };
            
            let mut table = FILE_TABLE.lock();
            let file = table.get_mut(fd).unwrap();
            file.ftype = FileType::Vfs;
            file.readable = true;
            file.vfs_file = Some(vfs_file);
            
            // Update proc.cwd
            if let Some(pid) = process::myproc() {
                let mut ptable = crate::process::PROC_TABLE.lock();
                if let Some(proc) = ptable.find(pid) {
                    let old_cwd = proc.cwd;
                    proc.cwd = Some(fd);
                    proc.cwd_path = Some(abs_path.clone());
                    drop(ptable);
                    drop(table);
                    
                    if let Some(old) = old_cwd {
                        file_close(old);
                    }
                    return E_OK;
                }
            }
            // If failed to find proc, close file
            drop(table);
            file_close(fd);
            E_BADF
        }
        Err(_) => E_NOENT,
    }
}

/// Get current working directory
pub fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    if buf.is_null() || size == 0 { return E_BADARG; }
    let path = {
        let mut ptable = crate::process::PROC_TABLE.lock();
        match crate::process::myproc().and_then(|pid| ptable.find(pid).and_then(|p| p.cwd_path.clone())) {
            Some(s) => s,
            None => alloc::string::String::from("/"),
        }
    };
    let need = path.as_bytes().len() + 1;
    if size < need { return E_BADARG; }
    let mut tmp = alloc::vec::Vec::with_capacity(need);
    tmp.extend_from_slice(path.as_bytes());
    tmp.push(0);
    let mut ptable = crate::process::PROC_TABLE.lock();
    let pagetable = match crate::process::myproc().and_then(|pid| ptable.find(pid).map(|p| p.pagetable)) { Some(pt) => pt, None => return E_BADARG };
    drop(ptable);
    unsafe { match crate::vm::copyout(pagetable, buf as usize, tmp.as_ptr(), tmp.len()) { Ok(()) => E_OK, Err(_) => E_FAULT } }
}

/// Create a directory
pub fn sys_mkdir(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    let abs_path = resolve_with_cwd(&path_str);
    
    // Default mode 0755
    let mode = crate::vfs::FileMode::new(crate::vfs::FileMode::S_IFDIR | 0o755);
    
    match crate::vfs::vfs().mkdir(&abs_path, mode) {
        Ok(_) => E_OK,
        Err(e) => match e {
            crate::vfs::VfsError::Exists => E_EXIST,
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::NoSpace => E_NOSPC,
            _ => E_IO,
        },
    }
}

/// Remove a directory
pub fn sys_rmdir(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    let abs_path = resolve_with_cwd(&path_str);
    
    match crate::vfs::vfs().rmdir(&abs_path) {
        Ok(_) => E_OK,
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::NotDirectory => E_NOTDIR,
            crate::vfs::VfsError::NotEmpty => E_NOTEMPTY,
            crate::vfs::VfsError::Busy => E_BUSY,
            _ => E_IO,
        },
    }
}

/// Create a special file (device node)
pub fn sys_mknod(path: *const u8, major: i16, minor: i16) -> isize {
    let path_str = match unsafe { copy_path(path) } { Ok(s) => s, Err(_) => return E_FAULT };
    let abs_path = resolve_with_cwd(&path_str);
    let mode = crate::vfs::FileMode::new(crate::vfs::FileMode::S_IFCHR | 0o600);
    let vfs = crate::vfs::vfs();
    match vfs.create(&abs_path, mode) {
        Ok(vf) => {
            if let Ok(mut attr) = vf.stat() {
                attr.rdev = (((major as u32) << 16) | (minor as u32)) as u64;
                let _ = vf.set_attr(&attr);
            }
            E_OK
        }
        Err(e) => match e {
            crate::vfs::VfsError::Exists => E_EXIST,
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::NotDirectory => E_NOTDIR,
            crate::vfs::VfsError::NoSpace => E_NOMEM,
            crate::vfs::VfsError::NotSupported => E_NOSYS,
            _ => E_IO,
        },
    }
}

/// Create a hard link
pub fn sys_link(old_path: *const u8, new_path: *const u8) -> isize {
    let old_path_str = match unsafe { copy_path(old_path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    let new_path_str = match unsafe { copy_path(new_path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    let old_abs = resolve_with_cwd(&old_path_str);
    let new_abs = resolve_with_cwd(&new_path_str);
    
    match crate::vfs::vfs().link(&old_abs, &new_abs) {
        Ok(_) => E_OK,
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::Exists => E_EXIST,
            crate::vfs::VfsError::IsDirectory => E_PERM,
            crate::vfs::VfsError::NotSupported => E_NOSYS,
            _ => E_IO,
        },
    }
}

/// Remove a file
pub fn sys_unlink(path: *const u8) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    let abs_path = resolve_with_cwd(&path_str);
    
    match crate::vfs::vfs().unlink(&abs_path) {
        Ok(_) => E_OK,
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::IsDirectory => E_ISDIR,
            crate::vfs::VfsError::Busy => E_BUSY,
            _ => E_IO,
        },
    }
}

/// Create a symbolic link
pub fn sys_symlink(oldpath: *const u8, newpath: *const u8) -> isize {
    let old_path = match unsafe { copy_path(oldpath) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    let new_path = match unsafe { copy_path(newpath) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    let abs_path = resolve_with_cwd(&new_path);
    
    match crate::vfs::vfs().symlink(&abs_path, &old_path) {
        Ok(_) => E_OK,
        Err(e) => match e {
            crate::vfs::VfsError::Exists => E_EXIST,
            crate::vfs::VfsError::NotFound => E_NOENT,
            _ => E_IO,
        },
    }
}

/// Read a symbolic link
pub fn sys_readlink(path: *const u8, buf: *mut u8, size: usize) -> isize {
    let path_str = match unsafe { copy_path(path) } {
        Ok(s) => s,
        Err(_) => return E_FAULT,
    };
    
    let abs_path = resolve_with_cwd(&path_str);
    
    match crate::vfs::vfs().readlink(&abs_path) {
        Ok(target) => {
            let target_len = target.len();
            let copy_len = if target_len > size { size } else { target_len };
            
            let mut ptable = crate::process::PROC_TABLE.lock();
            let pagetable = match crate::process::myproc().and_then(|pid| ptable.find(pid).map(|p| p.pagetable)) {
                Some(pt) => pt,
                None => return E_BADF,
            };
            drop(ptable);
            
            let result = unsafe {
                crate::vm::copyout(pagetable, buf as usize, target.as_bytes() as *const u8, copy_len)
            };
            
            match result {
                Ok(_) => copy_len as isize,
                Err(_) => E_FAULT,
            }
        },
        Err(e) => match e {
            crate::vfs::VfsError::NotFound => E_NOENT,
            crate::vfs::VfsError::InvalidOperation => E_BADF,
            _ => E_IO,
        },
    }
}
