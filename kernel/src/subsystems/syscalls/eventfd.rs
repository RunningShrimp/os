//! EventFd System Calls
//!
//! This module implements the eventfd system calls:
//! - eventfd: Create an event file descriptor (legacy)
//! - eventfd2: Create an event file descriptor with flags
//!
//! These system calls are POSIX-compatible and integrate with epoll.

use super::common::{SyscallError, SyscallResult, extract_args};
use super::interface::SyscallHandler;
use alloc::sync::Arc;
use crate::subsystems::sync::Mutex;
use alloc::vec::Vec;

/// EventFd flags (Linux compatible)
pub mod flags {
    /// Semaphore mode flag
    pub const EFD_SEMAPHORE: i32 = 0x1;
    /// Close-on-exec flag
    pub const EFD_CLOEXEC: i32 = 0o2000000;
    /// Non-blocking flag
    pub const EFD_NONBLOCK: i32 = 0o4000;
}

/// EventFd instance structure
#[derive(Debug)]
pub struct EventFdInstance {
    /// Current counter value
    counter: u64,
    /// Flags from eventfd2
    flags: i32,
}

impl EventFdInstance {
    /// Create a new eventfd instance
    pub fn new(initval: u32, flags: i32) -> Self {
        Self {
            counter: initval as u64,
            flags,
        }
    }

    /// Read from eventfd
    pub fn read(&mut self) -> Result<u64, SyscallError> {
        if self.counter == 0 {
            if (self.flags & flags::EFD_NONBLOCK) != 0 {
                return Err(SyscallError::WouldBlock);
            }
            // Would block - in a real implementation, we'd wait here
            return Err(SyscallError::WouldBlock);
        }
        
        let value_to_read = if (self.flags & flags::EFD_SEMAPHORE) != 0 {
            // Semaphore mode: read 1
            1
        } else {
            // Counter mode: read entire counter
            self.counter
        };
        
        self.counter -= value_to_read;
        Ok(value_to_read)
    }

    /// Write to eventfd
    pub fn write(&mut self, value: u64) -> Result<(), SyscallError> {
        if value == 0xfffffffffffffffe {
            return Err(SyscallError::InvalidArgument);
        }
        
        if self.counter > 0xfffffffffffffffe - value {
            if (self.flags & flags::EFD_NONBLOCK) != 0 {
                return Err(SyscallError::WouldBlock);
            }
            // Would block - in a real implementation, we'd wait here
            return Err(SyscallError::WouldBlock);
        }
        
        self.counter += value;
        Ok(())
    }
}

/// Global eventfd instances storage
static EVENTFD_INSTANCES: Mutex<Vec<Option<EventFdInstance>>> = Mutex::new(Vec::new());

/// Allocate an eventfd instance and return index
fn alloc_eventfd_instance(initval: u32, flags: i32) -> Option<usize> {
    let mut instances = EVENTFD_INSTANCES.lock();
    
    // Find a free slot
    for (idx, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(EventFdInstance::new(initval, flags));
            return Some(idx);
        }
    }
    
    // No free slot, allocate new one
    let idx = instances.len();
    instances.push(Some(EventFdInstance::new(initval, flags)));
    Some(idx)
}

/// Get eventfd instance by index
pub fn get_eventfd_instance(idx: usize) -> Option<&'static mut EventFdInstance> {
    let mut instances = EVENTFD_INSTANCES.lock();
    if idx < instances.len() {
        if let Some(ref mut instance) = instances[idx] {
            let ptr = instance as *mut EventFdInstance;
            unsafe { Some(&mut *ptr) }
        } else {
            None
        }
    } else {
        None
    }
}

/// EventFd system call handler
pub struct EventFdHandler;

impl EventFdHandler {
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for EventFdHandler {
    fn handle(&self, args: &[u64]) -> Result<u64, SyscallError> {
        // This is a placeholder - actual dispatch is done via syscall numbers
        Err(SyscallError::InvalidSyscall(0))
    }

    fn get_syscall_number(&self) -> u32 {
        0 // Will be set during registration
    }

    fn get_name(&self) -> &'static str {
        "eventfd"
    }
}

/// eventfd system call (legacy, always uses flags=0)
/// Arguments: [initval]
/// Returns: file descriptor on success, error on failure
pub fn sys_eventfd(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let initval = args[0] as u32;
    sys_eventfd2(&[initval as u64, 0])
}

/// eventfd2 system call
/// Arguments: [initval, flags]
/// Returns: file descriptor on success, error on failure
pub fn sys_eventfd2(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    
    let initval = args[0] as u32;
    let flags = args[1] as i32;
    
    // Validate flags
    let valid_flags = flags::EFD_SEMAPHORE | flags::EFD_CLOEXEC | flags::EFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Allocate eventfd instance
    let instance_idx = alloc_eventfd_instance(initval, flags)
        .ok_or(SyscallError::OutOfMemory)?;
    
    // Allocate file descriptor
    let pid = crate::subsystems::process::manager::myproc()
        .ok_or(SyscallError::InvalidArgument)?;
    
    let mut proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid) {
        // Find free file descriptor
        for (fd, file) in proc.ofile.iter_mut().enumerate() {
            if file.is_none() {
                *file = Some(crate::subsystems::fs::file::File {
                    ftype: crate::subsystems::fs::file::FileType::EventFd,
                    readable: true,
                    writable: true,
                    eventfd_instance: Some(instance_idx),
                    ..Default::default()
                });
                
                // Apply flags
                if (flags & flags::EFD_NONBLOCK) != 0 {
                    file.as_mut().unwrap().nonblock = true;
                }
                if (flags & flags::EFD_CLOEXEC) != 0 {
                    file.as_mut().unwrap().close_on_exec = true;
                }
                
                return Ok(fd as u64);
            }
        }
        Err(SyscallError::TooManyFiles)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

