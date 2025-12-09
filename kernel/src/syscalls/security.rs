//! POSIX Security System Calls
//!
//! This module implements system calls for POSIX security and permission features:
//! - capget() / capset() - Capability management
//! - getpwnam() / getpwuid() - Password database queries
//! - getgrnam() / getgrgid() - Group database queries
//! - setuid() / setgid() - Set user/group ID
//! - seteuid() / setegid() - Set effective user/group ID
//! - setreuid() / setregid() - Set real/effective user/group ID

use crate::posix::security::*;
use crate::posix::{Uid, Gid, Pid, Mode};
use crate::syscalls::common::{SyscallError, SyscallResult};
use crate::process::myproc;

// Security constants
pub const MAX_NAME_LEN: usize = 256;

/// System call dispatch for security operations
pub fn dispatch(syscall_num: u32, args: &[u64]) -> SyscallResult {
    match syscall_num {
        0xF000 => sys_capget(args),
        0xF001 => sys_capset(args),
        0xF002 => sys_getpwnam(args),
        0xF003 => sys_getpwuid(args),
        0xF004 => sys_getgrnam(args),
        0xF005 => sys_getgrgid(args),
        0xF006 => sys_setuid(args),
        0xF007 => sys_setgid(args),
        0xF008 => sys_seteuid(args),
        0xF009 => sys_setegid(args),
        0xF00A => sys_setreuid(args),
        0xF00B => sys_setregid(args),
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// capget system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: header_ptr - Pointer to cap_user_header_t
/// 2: data_ptr - Pointer to cap_user_data_t
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_capget(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid: Pid = args[0] as Pid;
    let header_ptr = args[1] as usize;
    let data_ptr = args[2] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid {
        return Err(SyscallError::PermissionDenied);
    }

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read header from user space
    let mut header_data = [0u8; core::mem::size_of::<CapHeader>()];
    if header_ptr != 0 {
        unsafe {
            match crate::mm::vm::copyin(pagetable, header_data.as_mut_ptr(), header_ptr, header_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
    }

    // Read data from user space
    let mut data_data = [0u8; core::mem::size_of::<CapData>()];
    if data_ptr != 0 {
        unsafe {
            match crate::mm::vm::copyin(pagetable, data_data.as_mut_ptr(), data_ptr, data_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
    }

    // Get capabilities
    match capget(pid, &mut unsafe { core::mem::transmute::<[u8; 8], CapHeader>(header_data) }, &mut unsafe { core::mem::transmute::<[u8; 12], CapData>(data_data) }) {
        Ok(()) => Ok(0),
        Err(SecurityError::InvalidCapability) => Err(SyscallError::InvalidArgument),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// capset system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: header_ptr - Pointer to cap_user_header_t
/// 2: data_ptr - Pointer to cap_user_data_t
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_capset(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid: Pid = args[0] as Pid;
    let header_ptr = args[1] as usize;
    let data_ptr = args[2] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid {
        return Err(SyscallError::PermissionDenied);
    }

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read header from user space
    let mut header_data = [0u8; core::mem::size_of::<CapHeader>()];
    if header_ptr != 0 {
        unsafe {
            match crate::mm::vm::copyin(pagetable, header_data.as_mut_ptr(), header_ptr, header_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
    }

    // Read data from user space
    let mut data_data = [0u8; core::mem::size_of::<CapData>()];
    if data_ptr != 0 {
        unsafe {
            match crate::mm::vm::copyin(pagetable, data_data.as_mut_ptr(), data_ptr, data_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
    }

    // Set capabilities
    match capset(pid, &unsafe { core::mem::transmute::<[u8; 8], CapHeader>(header_data) }, &unsafe { core::mem::transmute::<[u8; 12], CapData>(data_data) }) {
        Ok(()) => Ok(0),
        Err(SecurityError::InvalidCapability) => Err(SyscallError::InvalidArgument),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// getpwnam system call
/// 
/// Arguments:
/// 0: name_ptr - Pointer to username string
/// 1: pwd_ptr - Pointer to store password entry
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_getpwnam(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let name_ptr = args[0] as usize;
    let pwd_ptr = args[1] as usize;

    // Get current process
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read username from user space
    let mut name_data = [0u8; 256];
    let name_len = if name_ptr != 0 {
        unsafe {
            match crate::mm::vm::copyinstr(pagetable, name_ptr, &mut name_data as *mut u8, MAX_NAME_LEN) {
                Ok(len) => len,
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
    } else {
        return Err(SyscallError::InvalidArgument);
    };

    // Convert to string and look up
    let name = match core::str::from_utf8(&name_data[..name_len]) {
        Ok(name) => name,
        Err(_) => return Err(SyscallError::InvalidArgument),
    };

    // Get password entry
    match getpwnam(&name) {
        Ok(entry) => {
            // Copy entry back to user space
            if pwd_ptr != 0 {
                let entry_data = unsafe { 
                    // Convert PasswdEntry to bytes
                    let name_bytes = entry.pw_name.as_bytes();
                    let name_len = name_bytes.len().min(255);
                    
                    // Create structure layout
                    let mut entry_bytes = [0u8; 512]; // Enough space for the entry
                    let mut offset = 0;
                    
                    // pw_name (256 bytes max)
                    entry_bytes[offset..offset + name_len].copy_from_slice(&name_bytes);
                    offset += name_len;
                    // Pad to 256 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_passwd (1 byte + x for shadow password)
                    offset = 256;
                    entry_bytes[offset] = b'x';
                    offset += 1;
                    for _ in 0..7 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_uid (4 bytes)
                    entry_bytes[offset..offset + 4].copy_from_slice(&entry.pw_uid.to_le_bytes());
                    offset += 4;
                    
                    // pw_gid (4 bytes)
                    entry_bytes[offset..offset + 4].copy_from_slice(&entry.pw_gid.to_le_bytes());
                    offset += 4;
                    
                    // pw_gecos (variable length, up to 255 bytes)
                    let gecos_bytes = entry.pw_gecos.as_bytes();
                    let gecos_len = gecos_bytes.len().min(255);
                    entry_bytes[offset..offset + gecos_len].copy_from_slice(&gecos_bytes);
                    offset += gecos_len;
                    // Pad to 255 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_dir (variable length, up to 255 bytes)
                    let dir_bytes = entry.pw_dir.as_bytes();
                    let dir_len = dir_bytes.len().min(255);
                    entry_bytes[offset..offset + dir_len].copy_from_slice(&dir_bytes);
                    offset += dir_len;
                    // Pad to 255 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_shell (variable length, up to 255 bytes)
                    let shell_bytes = entry.pw_shell.as_bytes();
                    let shell_len = shell_bytes.len().min(255);
                    entry_bytes[offset..offset + shell_len].copy_from_slice(&shell_bytes);
                    // Pad to 256 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    unsafe {
                        match crate::mm::vm::copyout(pagetable, pwd_ptr, entry_bytes.as_ptr(), entry_bytes.len()) {
                            Ok(_) => {},
                            Err(_) => return Err(SyscallError::BadAddress),
                        };
                    }
                };
            }
            
            Ok(0);
        }
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// getpwuid system call
/// 
/// Arguments:
/// 0: uid - User ID
/// 1: pwd_ptr - Pointer to store password entry
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_getpwuid(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let uid = args[0] as Uid;
    let pwd_ptr = args[1] as usize;

    // Get current process
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Get password entry by UID
    match getpwuid(uid) {
        Ok(entry) => {
            // Copy entry back to user space (same as getpwnam)
            if pwd_ptr != 0 {
                let entry_data = unsafe { 
                    // Convert PasswdEntry to bytes
                    let name_bytes = entry.pw_name.as_bytes();
                    let name_len = name_bytes.len().min(255);
                    
                    // Create structure layout
                    let mut entry_bytes = [0u8; 512];
                    let mut offset = 0;
                    
                    // pw_name (256 bytes max)
                    entry_bytes[offset..offset + name_len].copy_from_slice(&name_bytes);
                    offset += name_len;
                    // Pad to 256 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_passwd (1 byte + x for shadow password)
                    offset = 256;
                    entry_bytes[offset] = b'x';
                    offset += 1;
                    for _ in 0..7 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_uid (4 bytes)
                    entry_bytes[offset..offset + 4].copy_from_slice(&entry.pw_uid.to_le_bytes());
                    offset += 4;
                    
                    // pw_gid (4 bytes)
                    entry_bytes[offset..offset + 4].copy_from_slice(&entry.pw_gid.to_le_bytes());
                    offset += 4;
                    
                    // pw_gecos (variable length, up to 255 bytes)
                    let gecos_bytes = entry.pw_gecos.as_bytes();
                    let gecos_len = gecos_bytes.len().min(255);
                    entry_bytes[offset..offset + gecos_len].copy_from_slice(&gecos_bytes);
                    offset += gecos_len;
                    // Pad to 255 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_dir (variable length, up to 255 bytes)
                    let dir_bytes = entry.pw_dir.as_bytes();
                    let dir_len = dir_bytes.len().min(255);
                    entry_bytes[offset..offset + dir_len].copy_from_slice(&dir_bytes);
                    offset += dir_len;
                    // Pad to 255 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // pw_shell (variable length, up to 255 bytes)
                    let shell_bytes = entry.pw_shell.as_bytes();
                    let shell_len = shell_bytes.len().min(255);
                    entry_bytes[offset..offset + shell_len].copy_from_slice(&shell_bytes);
                    // Pad to 256 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    unsafe {
                        match crate::mm::vm::copyout(pagetable, pwd_ptr, entry_bytes.as_ptr(), entry_bytes.len()) {
                            Ok(_) => {},
                            Err(_) => return Err(SyscallError::BadAddress),
                        };
                    }
                };
            }
            
            Ok(0);
        }
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// getgrnam system call
/// 
/// Arguments:
/// 0: name_ptr - Pointer to group name string
/// 1: grp_ptr - Pointer to store group entry
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_getgrnam(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let name_ptr = args[0] as usize;
    let grp_ptr = args[1] as usize;

    // Get current process
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read group name from user space
    let mut name_data = [0u8; 256];
    let name_len = if name_ptr != 0 {
        unsafe {
            match crate::mm::vm::copyinstr(pagetable, name_ptr, &mut name_data as *mut u8, MAX_NAME_LEN) {
                Ok(len) => len,
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
    } else {
        return Err(SyscallError::InvalidArgument);
    };

    // Convert to string and look up
    let name = match core::str::from_utf8(&name_data[..name_len]) {
        Ok(name) => name,
        Err(_) => return Err(SyscallError::InvalidArgument),
    };

    // Get group entry
    match getgrnam(&name) {
        Ok(entry) => {
            // Copy entry back to user space
            if grp_ptr != 0 {
                let entry_data = unsafe { 
                    // Convert GroupEntry to bytes
                    let name_bytes = entry.gr_name.as_bytes();
                    let name_len = name_bytes.len().min(255);
                    
                    // Create structure layout
                    let mut entry_bytes = [0u8; 1024]; // Enough space for the entry
                    let mut offset = 0;
                    
                    // gr_name (256 bytes max)
                    entry_bytes[offset..offset + name_len].copy_from_slice(&name_bytes);
                    offset += name_len;
                    // Pad to 256 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // gr_passwd (1 byte + x for shadow password)
                    offset = 256;
                    entry_bytes[offset] = b'x';
                    offset += 1;
                    for _ in 0..7 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // gr_gid (4 bytes)
                    entry_bytes[offset..offset + 4].copy_from_slice(&entry.gr_gid.to_le_bytes());
                    offset += 4;
                    
                    // gr_mem (variable length, list of member names)
                    let mut mem_offset = offset;
                    for member in &entry.gr_mem {
                        let member_bytes = member.as_bytes();
                        let member_len = member_bytes.len().min(255);
                        
                        if mem_offset + member_len + 1 > 1024 {
                            break; // Buffer full
                        }
                        
                        entry_bytes[mem_offset] = member_len as u8;
                        entry_bytes[mem_offset + 1..mem_offset + 1 + member_len].copy_from_slice(&member_bytes);
                        mem_offset += member_len + 1;
                    }
                    
                    unsafe {
                        match crate::mm::vm::copyout(pagetable, grp_ptr, entry_bytes.as_ptr(), entry_bytes.len()) {
                            Ok(_) => {},
                            Err(_) => return Err(SyscallError::BadAddress),
                        };
                    }
                };
            }
            
            Ok(0);
        }
        Err(SecurityError::GroupNotFound) => Err(SyscallError::NotFound),
    }
}

/// getgrgid system call
/// 
/// Arguments:
/// 0: gid - Group ID
/// 1: grp_ptr - Pointer to store group entry
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_getgrgid(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let gid = args[0] as Gid;
    let grp_ptr = args[1] as usize;

    // Get current process
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Get group entry by GID
    match getgrgid(gid) {
        Ok(entry) => {
            // Copy entry back to user space (same as getgrnam)
            if grp_ptr != 0 {
                let entry_data = unsafe { 
                    // Convert GroupEntry to bytes
                    let name_bytes = entry.gr_name.as_bytes();
                    let name_len = name_bytes.len().min(255);
                    
                    // Create structure layout
                    let mut entry_bytes = [0u8; 1024];
                    let mut offset = 0;
                    
                    // gr_name (256 bytes max)
                    entry_bytes[offset..offset + name_len].copy_from_slice(&name_bytes);
                    offset += name_len;
                    // Pad to 256 bytes
                    for _ in offset..256 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // gr_passwd (1 byte + x for shadow password)
                    offset = 256;
                    entry_bytes[offset] = b'x';
                    offset += 1;
                    for _ in 0..7 {
                        entry_bytes[offset] = 0;
                        offset += 1;
                    }
                    
                    // gr_gid (4 bytes)
                    entry_bytes[offset..offset + 4].copy_from_slice(&entry.gr_gid.to_le_bytes());
                    offset += 4;
                    
                    // gr_mem (variable length, list of member names)
                    let mut mem_offset = offset;
                    for member in &entry.gr_mem {
                        let member_bytes = member.as_bytes();
                        let member_len = member_bytes.len().min(255);
                        
                        if mem_offset + member_len + 1 > 1024 {
                            break; // Buffer full
                        }
                        
                        entry_bytes[mem_offset] = member_len as u8;
                        entry_bytes[mem_offset + 1..mem_offset + 1 + member_len].copy_from_slice(&member_bytes);
                        mem_offset += member_len + 1;
                    }
                    
                    unsafe {
                        match crate::mm::vm::copyout(pagetable, grp_ptr, entry_bytes.as_ptr(), entry_bytes.len()) {
                            Ok(_) => {},
                            Err(_) => return Err(SyscallError::BadAddress),
                        };
                    }
                };
            }
            
            Ok(0);
        }
        Err(SecurityError::GroupNotFound) => Err(SyscallError::NotFound),
    }
}

/// setuid system call
/// 
/// Arguments:
/// 0: uid - User ID to set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_setuid(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let uid = args[0] as Uid;

    // Set user ID
    match setuid(uid, uid) {
        Ok(()) => Ok(0),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// setgid system call
/// 
/// Arguments:
/// 0: gid - Group ID to set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_setgid(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let gid = args[0] as Gid;

    // Set group ID
    match setgid(gid, gid) {
        Ok(()) => Ok(0),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::GroupNotFound) => Err(SyscallError::NotFound),
    }
}

/// seteuid system call
/// 
/// Arguments:
/// 0: euid - Effective user ID to set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_seteuid(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let euid = args[0] as Uid;

    // Set effective user ID
    match seteuid(euid) {
        Ok(()) => Ok(0),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// setegid system call
/// 
/// Arguments:
/// 0: egid - Effective group ID to set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_setegid(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let egid = args[0] as Gid;

    // Set effective group ID
    match setegid(egid) {
        Ok(()) => Ok(0),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::GroupNotFound) => Err(SyscallError::NotFound),
    }
}

/// setreuid system call
/// 
/// Arguments:
/// 0: ruid - Real user ID to set
/// 1: euid - Effective user ID to set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_setreuid(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let ruid = args[0] as Uid;
    let euid = args[1] as Uid;

    // Set real and effective user IDs
    match setreuid(ruid, euid) {
        Ok(()) => Ok(0),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::UserNotFound) => Err(SyscallError::NotFound),
    }
}

/// setregid system call
/// 
/// Arguments:
/// 0: rgid - Real group ID to set
/// 1: egid - Effective group ID to set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_setregid(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let rgid = args[0] as Gid;
    let egid = args[1] as Gid;

    // Set real and effective group IDs
    match setregid(rgid, egid) {
        Ok(()) => Ok(0),
        Err(SecurityError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SecurityError::GroupNotFound) => Err(SyscallError::NotFound),
    }
}

/// Initialize security system calls
pub fn init_security_syscalls() {
    crate::println!("[syscall] Initializing security system calls");
    
    // Initialize security subsystem
    init_security();
    
    crate::println!("[syscall] Security system calls initialized");
    crate::println!("[syscall]   capget / capset - Capability management");
    crate::println!("[syscall]   getpwnam / getpwuid - Password database queries");
    crate::println!("[syscall]   getgrnam / getgrgid - Group database queries");
    crate::println!("[syscall]   setuid / setgid - Set user/group ID");
    crate::println!("[syscall]   seteuid / setegid - Set effective user/group ID");
    crate::println!("[syscall]   setreuid / setregid - Set real/effective user/group ID");
}

/// Get security system call statistics
pub fn get_security_stats() -> SecurityStats {
    let registry = SECURITY_REGISTRY.lock();
    let registry_stats = registry.get_stats();
    
    SecurityStats {
        total_processes: registry_stats.total_processes,
        total_users: registry_stats.total_users,
        total_groups: registry_stats.total_groups,
        next_uid: registry_stats.next_uid,
        next_gid: registry_stats.next_gid,
    }
}

/// Security system call statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    /// Total number of processes with credentials
    pub total_processes: usize,
    /// Total number of users in password database
    pub total_users: usize,
    /// Total number of groups in group database
    pub total_groups: usize,
    /// Next user ID to be allocated
    pub next_uid: Uid,
    /// Next group ID to be allocated
    pub next_gid: Gid,
}