//! TimerFd System Calls
//!
//! This module implements the timerfd system calls:
//! - timerfd_create: Create a timer file descriptor
//! - timerfd_settime: Arm or disarm a timer
//! - timerfd_gettime: Get the current setting of a timer
//!
//! These system calls are POSIX-compatible and integrate with epoll.

// Import prelude for common types
use crate::prelude::*;

use crate::subsystems::syscalls::interface::{SyscallHandler, SyscallNumber};
use crate::subsystems::syscalls::common::{SyscallResult, extract_args};
use crate::subsystems::mm::vm::{copyin, copyout};

use crate::subsystems::{sync::{Mutex, lazy::Lazy}, time};

/// TimerFd flags (Linux compatible)
pub mod flags {
    /// Close-on-exec flag
    pub const TFD_CLOEXEC: i32 = 0o2000000;
    /// Non-blocking flag
    pub const TFD_NONBLOCK: i32 = 0o4000;
    /// Absolute time flag
    pub const TFD_TIMER_ABSTIME: i32 = 0x1;
    /// Cancel on clock set flag
    pub const TFD_TIMER_CANCEL_ON_SET: i32 = 0x2;
}

/// TimerFd instance structure
#[derive(Debug)]
pub struct TimerFdInstance {
    /// Clock ID (CLOCK_REALTIME, CLOCK_MONOTONIC, etc.)
    clock_id: i32,
    /// Current timer value
    value: u64,
    /// Timer interval
    interval: u64,
    /// Flags
    flags: i32,
    /// Expiration count (number of times timer has expired)
    expiration_count: u64,
    /// Whether timer is armed
    armed: bool,
}

impl TimerFdInstance {
    /// Create a new timerfd instance
    pub fn new(clock_id: i32, flags: i32) -> Self {
        Self {
            clock_id,
            value: 0,
            interval: 0,
            flags,
            expiration_count: 0,
            armed: false,
        }
    }

    /// Set the timer
    pub fn set_time(&mut self, new_value: u64, new_interval: u64, _flags: i32) -> (u64, u64) {
        let old_value = self.value;
        let old_interval = self.interval;

        self.value = new_value;
        self.interval = new_interval;
        self.armed = true;

        (old_value, old_interval)
    }

    /// Get the current timer setting
    pub fn get_time(&self) -> (u64, u64) {
        (self.value, self.interval)
    }

    /// Check if timer has expired and update expiration count
    pub fn check_expiration(&mut self) -> u64 {
        if !self.armed {
            return 0;
        }

        let current_time = time::timestamp_nanos();
        let mut expirations = 0;

        if current_time >= self.value {
            // Timer has expired
            if self.interval > 0 {
                // Periodic timer - calculate number of expirations
                expirations = (current_time - self.value) / self.interval + 1;
                self.value += expirations * self.interval;
            } else {
                // One-shot timer
                expirations = 1;
                self.armed = false;
            }

            self.expiration_count += expirations;
        }

        expirations
    }

    /// Read expiration count (timerfd semantics)
    pub fn read(&mut self) -> u64 {
        let expirations = self.check_expiration();
        if expirations > 0 {
            self.expiration_count = 0; // Reset after read
        }
        expirations
    }
}

/// Global timerfd instances storage
static TIMERFD_INSTANCES: Mutex<Vec<Option<TimerFdInstance>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Allocate a timerfd instance and return index
fn alloc_timerfd_instance(clock_id: i32, flags: i32) -> Option<usize> {
    let mut instances = TIMERFD_INSTANCES.lock();

    // Find a free slot
    for (idx, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(TimerFdInstance::new(clock_id, flags));
            return Some(idx);
        }
    }

    // No free slot, allocate new one
    let idx = instances.len();
    instances.push(Some(TimerFdInstance::new(clock_id, flags)));
    Some(idx)
}

/// Get timerfd instance by index
pub fn get_timerfd_instance(idx: usize) -> Option<&'static mut TimerFdInstance> {
    let mut instances = TIMERFD_INSTANCES.lock();
    if idx < instances.len() {
        if let Some(ref mut instance) = instances[idx] {
            let ptr = instance as *mut TimerFdInstance;
            unsafe { Some(&mut *ptr) }
        } else {
            None
        }
    } else {
        None
    }
}

/// TimerFd system call handler
pub struct TimerFdHandler;

impl TimerFdHandler {
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for TimerFdHandler {
    fn handle(&self, _args: &[u64]) -> SyscallResult<()> {
        // For now, we don't have specific handler logic here
        // Individual syscall functions like sys_timerfd_create are called directly
        Err(SyscallError::NotSupported)
    }

    fn get_syscall_number(&self) -> SyscallNumber {
        // This should be the specific syscall number this handler handles
        // For now, return a placeholder that should be overridden by specific handlers
        0x9000 // Default timerfd syscall number
    }

    fn get_name(&self) -> &'static str {
        "timerfd"
    }
}

/// timerfd_create system call
/// Arguments: [clockid, flags]
/// Returns: file descriptor on success, error on failure
pub fn sys_timerfd_create(args: &[u64]) -> SyscallResult<i64> {
    let args = extract_args(args, 0, 2);

    let clockid = args[0] as i32;
    let flags = args[1] as i32;

    // Validate clock ID
    if clockid != 0 && clockid != 1 {
        // CLOCK_REALTIME, CLOCK_MONOTONIC
        return Err(SyscallError::InvalidArgument);
    }

    // Validate flags
    let valid_flags = flags::TFD_CLOEXEC | flags::TFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Allocate timerfd instance
    let instance_idx = alloc_timerfd_instance(clockid, flags).ok_or(SyscallError::OutOfMemory)?;

    // Allocate file descriptor
    let pid = crate::subsystems::process::manager::myproc().ok_or(SyscallError::InvalidArgument)?;

    let mut proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid) {
        // Find free file descriptor
        for (fd, file) in proc.ofile.iter_mut().enumerate() {
            if file.is_none() {
                *file = crate::subsystems::fs::file::File {
                    ftype: crate::subsystems::fs::file::FileType::TimerFd,
                    readable: true,
                    writable: false,
                    timerfd_instance: Some(instance_idx),
                    ref_count: 1,
                    status_flags: 0,
                    offset: 0,
                    inode: None,
                    pipe: None,
                    major: 0,
                    minor: 0,
                    vfs_file: None,
                    event_subs: Vec::new(),
                    socket: None,
                    inotify_instance: None,
                    eventfd_instance: None,
                    signalfd_instance: None,
                    memfd_instance: None,
                };

                // Apply flags
                if (flags & flags::TFD_NONBLOCK) != 0 {
                    file.as_mut().unwrap().status_flags |= crate::posix::O_NONBLOCK;
                }
                if (flags & flags::TFD_CLOEXEC) != 0 {
                    file.as_mut().unwrap().status_flags |= crate::posix::O_CLOEXEC;
                }

                return Ok(fd as i64);
            }
        }
        Err(SyscallError::TooManyOpenFiles)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

/// timerfd_settime system call
/// Arguments: [fd, flags, new_value_ptr, old_value_ptr]
/// Returns: 0 on success, error on failure
pub fn sys_timerfd_settime(args: &[u64]) -> SyscallResult<i64> {
    let args = extract_args(args, 0, 4);

    let fd = args[0] as i32;
    let flags = args[1] as i32;
    let new_value_ptr = args[2] as usize;
    let old_value_ptr = args[3] as usize;

    // Validate flags
    let valid_flags = flags::TFD_TIMER_ABSTIME | flags::TFD_TIMER_CANCEL_ON_SET;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get current process
    let pid = crate::subsystems::process::manager::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    let proc = proc_table
        .find_ref(pid)
        .ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Check if it's a timerfd file
    if fd < 0 || fd as usize >= crate::subsystems::process::manager::NOFILE {
        return Err(SyscallError::BadFileDescriptor);
    }

    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid) {
        if let Some(file_idx) = proc.ofile[fd as usize] {
            let file_table = crate::subsystems::fs::file::FILE_TABLE.lock();
            if let Some(file) = file_table.get(file_idx) {
                if file.ftype != crate::subsystems::fs::file::FileType::TimerFd {
                    return Err(SyscallError::InvalidArgument);
                }

                // Get timerfd instance
                let instance_idx = file.timerfd_instance.ok_or(SyscallError::InvalidArgument)?;
                drop(proc_table);

                if let Some(instance) = get_timerfd_instance(instance_idx) {
                    // Read new_value from user space
                    let mut new_value = 0u64;
                    unsafe {
                        crate::subsystems::mm::vm::copyin(
                            pagetable,
                            &mut new_value as *mut u64 as *mut u8,
                            new_value_ptr,
                            core::mem::size_of::<u64>(),
                        )
                        .map_err(|_| SyscallError::BadAddress)?;
                    }

                    // For simplicity, assume interval is 0 (one-shot timer)
                    // In a full implementation, we'd read a timespec structure
                    let new_interval = 0u64;

                    // Get old value before setting
                    let (old_value, old_interval) = instance.get_time();

                    // Set new timer value
                    instance.set_time(new_value, new_interval, flags);

                    // Write old_value to user space if requested
                    if old_value_ptr != 0 {
                        unsafe {
                            crate::subsystems::mm::vm::copyout(
                                pagetable,
                                old_value_ptr,
                                &old_value as *const u64 as *const u8,
                                core::mem::size_of::<u64>(),
                            )
                            .map_err(|_| SyscallError::BadAddress)?;
                        }
                    }

                    Ok(0)
                } else {
                    Err(SyscallError::InvalidArgument)
                }
            } else {
                Err(SyscallError::InvalidArgument)
            }
        } else {
            Err(SyscallError::BadFileDescriptor)
        }
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

/// timerfd_gettime system call
/// Arguments: [fd, curr_value_ptr]
/// Returns: 0 on success, error on failure
pub fn sys_timerfd_gettime(args: &[u64]) -> SyscallResult<i64> {
    let args = extract_args(args, 0, 2);

    let fd = args[0] as i32;
    let curr_value_ptr = args[1] as usize;

    // Get current process
    let pid = crate::subsystems::process::manager::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    let proc = proc_table
        .find_ref(pid)
        .ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Check file descriptor
    if fd < 0 || fd as usize >= crate::subsystems::process::manager::NOFILE {
        return Err(SyscallError::BadFileDescriptor);
    }

    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid) {
        if let Some(file_idx) = proc.ofile[fd as usize] {
            let file_table = crate::subsystems::fs::file::FILE_TABLE.lock();
            if let Some(file) = file_table.get(file_idx) {
                if file.ftype != crate::subsystems::fs::file::FileType::TimerFd {
                    return Err(SyscallError::InvalidArgument);
                }

                // Get timerfd instance
                let instance_idx = file.timerfd_instance.ok_or(SyscallError::InvalidArgument)?;
                drop(proc_table);

                if let Some(instance) = get_timerfd_instance(instance_idx) {
                    let (value, interval) = instance.get_time();

                    // Write current value to user space
                    unsafe {
                        crate::subsystems::mm::vm::copyout(
                            pagetable,
                            curr_value_ptr,
                            &value as *const u64 as *const u8,
                            core::mem::size_of::<u64>(),
                        )
                        .map_err(|_| SyscallError::BadAddress)?;
                    }

                    Ok(0)
                } else {
                    Err(SyscallError::InvalidArgument)
                }
            } else {
                Err(SyscallError::InvalidArgument)
            }
        } else {
            Err(SyscallError::BadFileDescriptor)
        }
    } else {
        Err(SyscallError::InvalidArgument)
    }
}