//! SignalFd System Calls
//!
//! This module implements the signalfd system calls:
//! - signalfd: Create a signal file descriptor (legacy)
//! - signalfd4: Create a signal file descriptor with flags
//!
//! These system calls are POSIX-compatible and integrate with epoll.

use super::common::{SyscallError, SyscallResult, extract_args};
use super::interface::SyscallHandler;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use crate::subsystems::sync::Mutex;

/// SignalFd flags (Linux compatible)
pub mod flags {
    /// Close-on-exec flag
    pub const SFD_CLOEXEC: i32 = 0o2000000;
    /// Non-blocking flag
    pub const SFD_NONBLOCK: i32 = 0o4000;
}

/// SignalFd siginfo structure (Linux compatible)
#[derive(Debug, Clone)]
pub struct SignalfdSiginfo {
    /// Signal number
    pub ssi_signo: u32,
    /// Error number
    pub ssi_errno: i32,
    /// Signal code
    pub ssi_code: i32,
    /// Process ID
    pub ssi_pid: u32,
    /// User ID
    pub ssi_uid: u32,
    /// File descriptor
    pub ssi_fd: i32,
    /// Timer ID
    pub ssi_tid: u32,
    /// Band
    pub ssi_band: u32,
    /// Overrun
    pub ssi_overrun: u32,
    /// Trap number
    pub ssi_trapno: u32,
    /// Status
    pub ssi_status: i32,
    /// Integer
    pub ssi_int: i32,
    /// Pointer
    pub ssi_ptr: u64,
    /// Real-time signal data
    pub ssi_utime: u64,
    pub ssi_stime: u64,
    /// Address
    pub ssi_addr: u64,
}

/// SignalFd instance structure
#[derive(Debug)]
pub struct SignalfdInstance {
    /// Signal mask
    mask: crate::subsystems::ipc::signal::SigSet,
    /// Signal queue
    signal_queue: VecDeque<SignalfdSiginfo>,
    /// Flags from signalfd4
    flags: i32,
}

impl SignalfdInstance {
    /// Create a new signalfd instance
    pub fn new(mask: crate::subsystems::ipc::signal::SigSet, flags: i32) -> Self {
        Self {
            mask,
            signal_queue: VecDeque::new(),
            flags,
        }
    }

    /// Enqueue a signal
    pub fn enqueue_signal(&mut self, sig: crate::subsystems::ipc::signal::Signal, info: crate::subsystems::ipc::signal::SigInfo) -> bool {
        // Check if signal is in mask
        if !self.mask.contains(sig) {
            return false;
        }
        
        // Convert SigInfo to SignalfdSiginfo
        let siginfo = SignalfdSiginfo {
            ssi_signo: sig as u32,
            ssi_errno: 0,
            ssi_code: 0,
            ssi_pid: info.pid,
            ssi_uid: info.uid,
            ssi_fd: 0,
            ssi_tid: 0,
            ssi_band: 0,
            ssi_overrun: 0,
            ssi_trapno: 0,
            ssi_status: 0,
            ssi_int: 0,
            ssi_ptr: 0,
            ssi_utime: 0,
            ssi_stime: 0,
            ssi_addr: 0,
        };
        
        self.signal_queue.push_back(siginfo);
        true
    }

    /// Read a signal (returns siginfo structure)
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, SyscallError> {
        if self.signal_queue.is_empty() {
            if (self.flags & flags::SFD_NONBLOCK) != 0 {
                return Err(SyscallError::WouldBlock);
            }
            // Would block - in a real implementation, we'd wait here
            return Err(SyscallError::WouldBlock);
        }
        
        if let Some(siginfo) = self.signal_queue.pop_front() {
            let siginfo_size = core::mem::size_of::<SignalfdSiginfo>();
            if buf.len() < siginfo_size {
                return Err(SyscallError::InvalidArgument);
            }
            
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &siginfo as *const SignalfdSiginfo as *const u8,
                    buf.as_mut_ptr(),
                    siginfo_size,
                );
            }
            
            Ok(siginfo_size)
        } else {
            Err(SyscallError::WouldBlock)
        }
    }
}

/// Global signalfd instances storage
static SIGNALFD_INSTANCES: Mutex<Vec<Option<SignalfdInstance>>> = Mutex::new(Vec::new());

/// Allocate a signalfd instance and return index
fn alloc_signalfd_instance(mask: crate::subsystems::ipc::signal::SigSet, flags: i32) -> Option<usize> {
    let mut instances = SIGNALFD_INSTANCES.lock();
    
    // Find a free slot
    for (idx, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(SignalfdInstance::new(mask, flags));
            return Some(idx);
        }
    }
    
    // No free slot, allocate new one
    let idx = instances.len();
    instances.push(Some(SignalfdInstance::new(mask, flags)));
    Some(idx)
}

/// Get signalfd instance by index
pub fn get_signalfd_instance(idx: usize) -> Option<&'static mut SignalfdInstance> {
    let mut instances = SIGNALFD_INSTANCES.lock();
    if idx < instances.len() {
        if let Some(ref mut instance) = instances[idx] {
            let ptr = instance as *mut SignalfdInstance;
            unsafe { Some(&mut *ptr) }
        } else {
            None
        }
    } else {
        None
    }
}

/// SignalFd system call handler
pub struct SignalFdHandler;

impl SignalFdHandler {
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for SignalFdHandler {
    fn handle(&self, args: &[u64]) -> Result<u64, SyscallError> {
        // This is a placeholder - actual dispatch is done via syscall numbers
        Err(SyscallError::InvalidSyscall(0))
    }

    fn get_syscall_number(&self) -> u32 {
        0 // Will be set during registration
    }

    fn get_name(&self) -> &'static str {
        "signalfd"
    }
}

/// signalfd system call (legacy, always uses flags=0)
/// Arguments: [fd, mask_ptr]
/// Returns: file descriptor on success, error on failure
pub fn sys_signalfd(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let fd = args[0] as i32;
    let mask_ptr = args[1] as usize;
    let flags = 0; // signalfd doesn't take flags, always 0
    
    // Call signalfd4 with flags = 0
    sys_signalfd4(&[fd as u64, mask_ptr as u64, flags as u64])
}

/// signalfd4 system call
/// Arguments: [fd, mask_ptr, flags]
/// Returns: file descriptor on success, error on failure
pub fn sys_signalfd4(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    
    let fd = args[0] as i32;
    let mask_ptr = args[1] as usize;
    let flags = args[2] as i32;
    
    // Validate flags
    let valid_flags = flags::SFD_CLOEXEC | flags::SFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process
    let pid = crate::subsystems::process::manager::myproc()
        .ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read signal mask from user space
    let mask = unsafe {
        crate::subsystems::mm::vm::copyin(
            pagetable,
            mask_ptr as *mut u8,
            mask_ptr,
            core::mem::size_of::<u64>(),
        ).map_err(|_| SyscallError::BadAddress)?;
        crate::subsystems::ipc::signal::SigSet::from_bits(*(mask_ptr as *const u64) as u64)
            .ok_or(SyscallError::InvalidArgument)?
    };
    
    if fd == -1 {
        // Create new signalfd
        let instance_idx = alloc_signalfd_instance(mask, flags)
            .ok_or(SyscallError::OutOfMemory)?;
        
        // Allocate file descriptor
        let mut proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find(pid) {
            // Find free file descriptor
            for (fd, file) in proc.ofile.iter_mut().enumerate() {
                if file.is_none() {
                    *file = Some(crate::subsystems::fs::file::File {
                        ftype: crate::subsystems::fs::file::FileType::Signalfd,
                        readable: true,
                        writable: false,
                        signalfd_instance: Some(instance_idx),
                        ..Default::default()
                    });
                    
                    // Apply flags
                    if (flags & flags::SFD_NONBLOCK) != 0 {
                        file.as_mut().unwrap().nonblock = true;
                    }
                    if (flags & flags::SFD_CLOEXEC) != 0 {
                        file.as_mut().unwrap().close_on_exec = true;
                    }
                    
                    return Ok(fd as u64);
                }
            }
            Err(SyscallError::TooManyFiles)
        } else {
            Err(SyscallError::InvalidArgument)
        }
    } else {
        // Modify existing signalfd
        let mut proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find(pid) {
            if fd < 0 || fd as usize >= crate::subsystems::process::manager::NOFILE {
                return Err(SyscallError::BadFileDescriptor);
            }
            
            if let Some(ref file) = proc.ofile[fd as usize] {
                // Check if it's a signalfd file
                if file.ftype != crate::subsystems::fs::file::FileType::Signalfd {
                    return Err(SyscallError::InvalidArgument);
                }
                
                // Update the mask
                if let Some(instance_idx) = file.signalfd_instance {
                    drop(proc_table);
                    if let Some(instance) = get_signalfd_instance(instance_idx) {
                        instance.mask = mask;
                        Ok(fd as u64)
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
}

