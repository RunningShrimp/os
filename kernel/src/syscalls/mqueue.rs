//! Message Queue System Calls
//!
//! This module implements system calls for POSIX message queues

use super::common::{SyscallError, SyscallResult, extract_args};
use crate::posix::mqueue::*;
use crate::process::myproc;
use core::ptr;

/// Dispatch message queue syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Message queue operations
        0x4000 => sys_mq_open(args),        // mq_open
        0x4001 => sys_mq_close(args),       // mq_close
        0x4002 => sys_mq_unlink(args),      // mq_unlink
        0x4003 => sys_mq_send(args),        // mq_send
        0x4004 => sys_mq_timedsend(args),   // mq_timedsend
        0x4005 => sys_mq_receive(args),     // mq_receive
        0x4006 => sys_mq_timedreceive(args), // mq_timedreceive
        0x4007 => sys_mq_getattr(args),     // mq_getattr
        0x4008 => sys_mq_setattr(args),     // mq_setattr
        0x4009 => sys_mq_notify(args),      // mq_notify
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Open a message queue (mq_open)
/// 
/// # Arguments
/// * `name` - Queue name pointer
/// * `oflag` - Open flags
/// * `mode` - Permission mode
/// * `attr` - Queue attributes pointer
/// 
/// # Returns
/// * Message queue descriptor on success
/// * Error code on failure
fn sys_mq_open(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 4)?;
    let name_ptr = args[0] as *const i8;
    let oflag = args[1] as i32;
    let mode = args[2] as crate::posix::Mode;
    let attr_ptr = args[3] as *const MqAttr;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate name pointer
    if name_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Convert name from user space
    let name_len = unsafe {
        let mut len = 0;
        while *name_ptr.add(len) != 0 && len < crate::posix::MQ_NAME_MAX {
            len += 1;
        }
        len
    };
    
    let mut name_bytes = vec![0u8; name_len];
    unsafe {
        match crate::mm::vm::copyin(pagetable, name_bytes.as_mut_ptr(), name_ptr as usize, name_len) {
            Ok(_) => {},
            Err(_) => return Err(SyscallError::BadAddress),
        }
    }
    
    let name_str = match core::str::from_utf8(&name_bytes) {
        Ok(s) => s,
        Err(_) => return Err(SyscallError::InvalidArgument),
    };
    
    // Validate attributes pointer
    let attr = if attr_ptr.is_null() {
        core::ptr::null()
    } else {
        // Copy attributes from user space
        let mut attr_bytes = [0u8; core::mem::size_of::<MqAttr>()];
        unsafe {
            match crate::mm::vm::copyin(pagetable, attr_bytes.as_mut_ptr(), attr_ptr as usize, attr_bytes.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }
        attr_ptr as *const MqAttr
    };
    
    // Call POSIX mq_open
    let result = unsafe { mq_open(name_ptr, oflag, mode, attr) };
    
    if result >= 0 {
        Ok(result as u64)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Close a message queue (mq_close)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_close(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let mqd = args[0] as i32;
    
    // Call POSIX mq_close
    let result = unsafe { mq_close(mqd) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Remove a message queue (mq_unlink)
/// 
/// # Arguments
/// * `name` - Queue name pointer
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_unlink(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let name_ptr = args[0] as *const i8;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate name pointer
    if name_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_unlink
    let result = unsafe { mq_unlink(name_ptr) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Send a message to a queue (mq_send)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `msg_ptr` - Message data pointer
/// * `msg_len` - Message length
/// * `msg_prio` - Message priority
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_send(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 4)?;
    let mqd = args[0] as i32;
    let msg_ptr = args[1] as *const core::ffi::c_void;
    let msg_len = args[2] as usize;
    let msg_prio = args[3] as u32;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate message pointer
    if msg_ptr.is_null() || msg_len == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_send
    let result = unsafe { mq_send(mqd, msg_ptr, msg_len, msg_prio) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Send a message to a queue with timeout (mq_timedsend)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `msg_ptr` - Message data pointer
/// * `msg_len` - Message length
/// * `msg_prio` - Message priority
/// * `abs_timeout` - Timeout specification pointer
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_timedsend(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 5)?;
    let mqd = args[0] as i32;
    let msg_ptr = args[1] as *const core::ffi::c_void;
    let msg_len = args[2] as usize;
    let msg_prio = args[3] as u32;
    let timeout_ptr = args[4] as *const crate::posix::Timespec;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate pointers
    if msg_ptr.is_null() || msg_len == 0 || timeout_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Copy timeout from user space
    let mut timeout_bytes = [0u8; core::mem::size_of::<crate::posix::Timespec>()];
    unsafe {
        match crate::mm::vm::copyin(pagetable, timeout_bytes.as_mut_ptr(), timeout_ptr as usize, timeout_bytes.len()) {
            Ok(_) => {},
            Err(_) => return Err(SyscallError::BadAddress),
        }
    }
    
    let timeout = unsafe { core::ptr::read(timeout_bytes.as_ptr() as *const crate::posix::Timespec) };
    
    // For now, implement as non-blocking send with timeout validation
    // TODO: Implement proper timed send with blocking and timeout
    if timeout.tv_sec < 0 || timeout.tv_nsec < 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_send (simplified implementation)
    let result = unsafe { mq_send(mqd, msg_ptr, msg_len, msg_prio) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Receive a message from a queue (mq_receive)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `msg_ptr` - Message buffer pointer
/// * `msg_len` - Buffer size
/// * `msg_prio` - Priority buffer pointer
/// 
/// # Returns
/// * Message length on success
/// * Error code on failure
fn sys_mq_receive(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 4)?;
    let mqd = args[0] as i32;
    let msg_ptr = args[1] as *mut core::ffi::c_void;
    let msg_len = args[2] as usize;
    let prio_ptr = args[3] as *mut u32;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate pointers
    if msg_ptr.is_null() || msg_len == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_receive
    let result = unsafe { mq_receive(mqd, msg_ptr, msg_len, prio_ptr) };
    
    if result >= 0 {
        Ok(result as u64)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Receive a message from a queue with timeout (mq_timedreceive)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `msg_ptr` - Message buffer pointer
/// * `msg_len` - Buffer size
/// * `msg_prio` - Priority buffer pointer
/// * `abs_timeout` - Timeout specification pointer
/// 
/// # Returns
/// * Message length on success
/// * Error code on failure
fn sys_mq_timedreceive(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 5)?;
    let mqd = args[0] as i32;
    let msg_ptr = args[1] as *mut core::ffi::c_void;
    let msg_len = args[2] as usize;
    let prio_ptr = args[3] as *mut u32;
    let timeout_ptr = args[4] as *const crate::posix::Timespec;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate pointers
    if msg_ptr.is_null() || msg_len == 0 || timeout_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Copy timeout from user space
    let mut timeout_bytes = [0u8; core::mem::size_of::<crate::posix::Timespec>()];
    unsafe {
        match crate::mm::vm::copyin(pagetable, timeout_bytes.as_mut_ptr(), timeout_ptr as usize, timeout_bytes.len()) {
            Ok(_) => {},
            Err(_) => return Err(SyscallError::BadAddress),
        }
    }
    
    let timeout = unsafe { core::ptr::read(timeout_bytes.as_ptr() as *const crate::posix::Timespec) };
    
    // For now, implement as non-blocking receive with timeout validation
    // TODO: Implement proper timed receive with blocking and timeout
    if timeout.tv_sec < 0 || timeout.tv_nsec < 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_receive (simplified implementation)
    let result = unsafe { mq_receive(mqd, msg_ptr, msg_len, prio_ptr) };
    
    if result >= 0 {
        Ok(result as u64)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Get message queue attributes (mq_getattr)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `attr` - Attributes buffer pointer
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_getattr(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let mqd = args[0] as i32;
    let attr_ptr = args[1] as *mut MqAttr;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate attributes pointer
    if attr_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_getattr
    let result = unsafe { mq_getattr(mqd, attr_ptr) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Set message queue attributes (mq_setattr)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `attr` - New attributes pointer
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_setattr(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let mqd = args[0] as i32;
    let attr_ptr = args[1] as *const MqAttr;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate attributes pointer
    if attr_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Call POSIX mq_setattr
    let result = unsafe { mq_setattr(mqd, attr_ptr) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Register for asynchronous notification (mq_notify)
/// 
/// # Arguments
/// * `mqd` - Message queue descriptor
/// * `notification` - Notification structure pointer
/// 
/// # Returns
/// * 0 on success
/// * Error code on failure
fn sys_mq_notify(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let mqd = args[0] as i32;
    let notify_ptr = args[1] as *const MqNotify;
    
    // Get current process for memory access
    let pid = myproc().ok_or(SyscallError::InvalidArgument)?;
    let mut table = crate::process::PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate notification pointer
    if notify_ptr.is_null() {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Copy notification from user space
    let mut notify_bytes = [0u8; core::mem::size_of::<MqNotify>()];
    unsafe {
        match crate::mm::vm::copyin(pagetable, notify_bytes.as_mut_ptr(), notify_ptr as usize, notify_bytes.len()) {
            Ok(_) => {},
            Err(_) => return Err(SyscallError::BadAddress),
        }
    }
    
    let notification = unsafe { core::ptr::read(notify_bytes.as_ptr() as *const MqNotify) };
    
    // Call POSIX mq_notify
    let result = unsafe { mq_notify(mqd, &notification) };
    
    if result == 0 {
        Ok(0)
    } else {
        Err(SyscallError::IoError)
    }
}

/// Initialize message queue system calls
pub fn init() -> Result<(), &'static str> {
    // Initialize POSIX message queue subsystem
    match crate::posix::mqueue::init() {
        Ok(()) => {
            crate::println!("[mqueue] Message queue system calls initialized");
            Ok(())
        }
        Err(e) => {
            crate::println!("[mqueue] Failed to initialize message queue subsystem: {}", e);
            Err("Failed to initialize message queue subsystem")
        }
    }
}

/// Cleanup message queue system calls
pub fn cleanup() {
    crate::posix::mqueue::cleanup();
    crate::println!("[mqueue] Message queue system calls cleaned up");
}

/// Get message queue statistics
pub fn get_stats() -> (usize, usize, usize) {
    crate::posix::mqueue::get_stats()
}