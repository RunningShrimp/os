//! EventFd System Calls
//!
//! This module implements the eventfd system calls:
//! - eventfd: Create an event file descriptor (legacy)
//! - eventfd2: Create an event file descriptor with flags
//!
//! These system calls are POSIX-compatible and integrate with epoll.

use alloc::vec::Vec;

use crate::subsystems::syscalls::interface::{SyscallHandler, SyscallNumber, SyscallResult, SyscallError};
use crate::subsystems::syscalls::common::extract_args;
use crate::subsystems::sync::Mutex;

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
        Self { counter: initval as u64, flags }
    }

    /// Read from eventfd
    pub fn read(&mut self) -> Result<u64, SyscallError> {
        if self.counter == 0 {
            if (self.flags & flags::EFD_NONBLOCK) != 0 {
                return Err(SyscallError::IoError);
            }
            // Would block - in a real implementation, we'd wait here
            return Err(SyscallError::IoError);
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
                return Err(SyscallError::IoError);
            }
            // Would block - in a real implementation, we'd wait here
            return Err(SyscallError::IoError);
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
    fn handle(&self, _args: &[u64]) -> SyscallResult<()> {
        // For now, we don't have specific handler logic here
        // Individual syscall functions like sys_eventfd are called directly
        Err(SyscallError::InvalidInterface)
    }

    fn get_syscall_number(&self) -> SyscallNumber {
        // This should be the specific syscall number this handler handles
        // For now, return a placeholder that should be overridden by specific handlers
        0x9005 // Default eventfd syscall number
    }

    fn get_name(&self) -> &'static str {
        "eventfd"
    }
}

/// eventfd system call (legacy, always uses flags=0)
/// Arguments: [initval]
/// Returns: file descriptor on success, error on failure
pub fn sys_eventfd(args: &[u64]) -> SyscallResult<i64> {
    let args = extract_args(args, 0, 1);
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }
    let initval = args[0] as u32;
    sys_eventfd2(&[initval as u64, 0])
}

/// eventfd2 system call
/// Arguments: [initval, flags]
/// Returns: file descriptor on success, error on failure
pub fn sys_eventfd2(args: &[u64]) -> SyscallResult<i64> {
    let args = extract_args(args, 0, 2);
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let initval = args[0] as u32;
    let flags = args[1] as i32;

    // Validate flags
    let valid_flags = flags::EFD_SEMAPHORE | flags::EFD_CLOEXEC | flags::EFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Allocate eventfd instance
    let instance_idx = alloc_eventfd_instance(initval, flags).ok_or(SyscallError::IoError)?;

    // Allocate file from FILE_TABLE
    let file_idx = crate::subsystems::fs::file::file_alloc().ok_or(SyscallError::IoError)?;

    // Configure the file in FILE_TABLE
    {
        let mut file_table = crate::subsystems::fs::file::FILE_TABLE.lock();
        if let Some(file) = file_table.get_mut(file_idx) {
            file.ftype = crate::subsystems::fs::file::FileType::EventFd;
            file.readable = true;
            file.writable = true;
            file.eventfd_instance = Some(instance_idx);

            // Apply flags
            if (flags & flags::EFD_NONBLOCK) != 0 {
                file.status_flags |= crate::posix::O_NONBLOCK;
            }
            // Note: EFD_CLOEXEC would need to be handled separately in exec()
        }
    }

    // Allocate file descriptor in process
    let pid = crate::subsystems::process::manager::myproc().ok_or(SyscallError::InvalidArgument)?;

    let mut proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid) {
        // Find free file descriptor
        for (fd, file_slot) in proc.ofile.iter_mut().enumerate() {
            if file_slot.is_none() {
                *file_slot = Some(file_idx);
                return Ok(fd as i64);
            }
        }
        // No free file descriptor - close the allocated file
        crate::subsystems::fs::file::file_close(file_idx);
        Err(SyscallError::IoError)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}
