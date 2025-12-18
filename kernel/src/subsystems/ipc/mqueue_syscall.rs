//! POSIX Message Queue System Call Handlers
//!
//! This module implements the system call handlers for POSIX message queues.
//! It provides the interface between user space and the kernel message queue implementation.

use alloc::string::String;
use alloc::vec::Vec;
use crate::api::syscall::{SyscallHandler, SyscallError, SyscallResult, SyscallNumber, SyscallArgs};
use crate::api::error::{KernelError, Result};
use crate::subsystems::ipc::mqueue;
use crate::subsystems::ipc::mqueue::{MqAttr, MqNotify, MqOpenFlags, MqNotifyType};
use crate::subsystems::process::{get_current_process, get_process_by_pid};
use crate::subsystems::fs::{Path, VfsNode};
use crate::subsystems::time::{get_current_time, Timespec};
use core::ptr;
use core::slice;

/// Maximum message queue name length
const MQ_NAME_MAX: usize = 255;

/// Message queue open system call handler
pub struct MqOpenHandler;

impl SyscallHandler for MqOpenHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let name_ptr = args.arg0 as *const u8;
        let flags = args.arg1 as u32;
        let mode = args.arg2 as u32;
        let attr_ptr = args.arg3 as *const MqAttr;

        // Validate name pointer
        if name_ptr.is_null() {
            return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into()));
        }

        // Read and validate name
        let name = match self.read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into())),
        };

        if name.len() > MQ_NAME_MAX {
            return Ok(SyscallResult::Error(SyscallError::NameTooLong.into()));
        }

        // Validate name format (must start with '/')
        if !name.starts_with('/') || name.contains('\0') {
            return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into()));
        }

        // Read attributes if provided
        let attr = if !attr_ptr.is_null() {
            Some(unsafe { ptr::read(attr_ptr) })
        } else {
            None
        };

        // Convert flags
        let mq_flags = match MqOpenFlags::from_bits(flags) {
            Some(flags) => flags,
            None => return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into())),
        };

        // Open message queue
        match mqueue::mq_open(&name, mq_flags, mode, attr.as_ref()) {
            Ok(mqd) => Ok(SyscallResult::Success(mqd as isize)),
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_open"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 101 // CommonSyscall::MqOpen
    }
}

/// Message queue close system call handler
pub struct MqCloseHandler;

impl SyscallHandler for MqCloseHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;

        match mqueue::mq_close(mqd) {
            Ok(()) => Ok(SyscallResult::Success(0)),
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_close"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 102 // CommonSyscall::MqClose
    }
}

/// Message queue get attributes system call handler
pub struct MqGetattrHandler;

impl SyscallHandler for MqGetattrHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;
        let attr_ptr = args.arg1 as *mut MqAttr;

        // Validate attributes pointer
        if attr_ptr.is_null() {
            return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into()));
        }

        match mqueue::mq_getattr(mqd) {
            Ok(attr) => {
                unsafe { ptr::write(attr_ptr, attr); }
                Ok(SyscallResult::Success(0))
            },
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_getattr"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 103 // CommonSyscall::MqGetattr
    }
}

/// Message queue set attributes system call handler
pub struct MqSetattrHandler;

impl SyscallHandler for MqSetattrHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;
        let new_attr_ptr = args.arg1 as *const MqAttr;
        let old_attr_ptr = args.arg2 as *mut MqAttr;

        // Validate new attributes pointer
        if new_attr_ptr.is_null() {
            return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into()));
        }

        let new_attr = unsafe { ptr::read(new_attr_ptr) };

        match mqueue::mq_setattr(mqd, &new_attr) {
            Ok(old_attr) => {
                if !old_attr_ptr.is_null() {
                    unsafe { ptr::write(old_attr_ptr, old_attr); }
                }
                Ok(SyscallResult::Success(0))
            },
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_setattr"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 104 // CommonSyscall::MqSetattr
    }
}

/// Message queue timed send system call handler
pub struct MqTimedsendHandler;

impl SyscallHandler for MqTimedsendHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;
        let msg_ptr = args.arg1 as *const u8;
        let msg_len = args.arg2 as usize;
        let msg_prio = args.arg3 as u32;
        let timeout_ptr = args.arg4 as *const Timespec;

        // Validate message pointer
        if msg_ptr.is_null() || msg_len == 0 {
            return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into()));
        }

        // Read message
        let msg = unsafe { slice::from_raw_parts(msg_ptr, msg_len) }.to_vec();

        // Read timeout if provided
        let timeout = if !timeout_ptr.is_null() {
            Some(unsafe { ptr::read(timeout_ptr) })
        } else {
            None
        };

        match mqueue::mq_timedsend(mqd, &msg, msg_prio, timeout.as_ref()) {
            Ok(()) => Ok(SyscallResult::Success(0)),
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_timedsend"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 105 // CommonSyscall::MqTimedsend
    }
}

/// Message queue timed receive system call handler
pub struct MqTimedreceiveHandler;

impl SyscallHandler for MqTimedreceiveHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;
        let msg_ptr = args.arg1 as *mut u8;
        let msg_len = args.arg2 as usize;
        let msg_prio_ptr = args.arg3 as *mut u32;
        let timeout_ptr = args.arg4 as *const Timespec;

        // Validate message buffer pointer
        if msg_ptr.is_null() || msg_len == 0 {
            return Ok(SyscallResult::Error(SyscallError::InvalidArgument.into()));
        }

        // Read timeout if provided
        let timeout = if !timeout_ptr.is_null() {
            Some(unsafe { ptr::read(timeout_ptr) })
        } else {
            None
        };

        match mqueue::mq_timedreceive(mqd, msg_len, timeout.as_ref()) {
            Ok((msg, prio)) => {
                // Copy message to user buffer
                let copy_len = core::cmp::min(msg.len(), msg_len);
                unsafe {
                    ptr::copy_nonoverlapping(msg.as_ptr(), msg_ptr, copy_len);
                }
                
                // Set priority if requested
                if !msg_prio_ptr.is_null() {
                    unsafe { ptr::write(msg_prio_ptr, prio); }
                }
                
                Ok(SyscallResult::Success(copy_len as isize))
            },
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_timedreceive"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 106 // CommonSyscall::MqTimedreceive
    }
}

/// Message queue notify system call handler
pub struct MqNotifyHandler;

impl SyscallHandler for MqNotifyHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;
        let notify_ptr = args.arg1 as *const MqNotify;

        // Read notification if provided
        let notify = if !notify_ptr.is_null() {
            Some(unsafe { ptr::read(notify_ptr) })
        } else {
            None
        };

        match mqueue::mq_notify(mqd, notify.as_ref()) {
            Ok(()) => Ok(SyscallResult::Success(0)),
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_notify"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 107 // CommonSyscall::MqNotify
    }
}

/// Message queue get/set attributes system call handler
pub struct MqGetsetattrHandler;

impl SyscallHandler for MqGetsetattrHandler {
    fn handle(&mut self, _number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        let mqd = args.arg0 as i32;
        let new_attr_ptr = args.arg1 as *const MqAttr;
        let old_attr_ptr = args.arg2 as *mut MqAttr;

        // Read new attributes if provided
        let new_attr = if !new_attr_ptr.is_null() {
            Some(unsafe { ptr::read(new_attr_ptr) })
        } else {
            None
        };

        match mqueue::mq_getsetattr(mqd, new_attr.as_ref()) {
            Ok(old_attr) => {
                if !old_attr_ptr.is_null() {
                    unsafe { ptr::write(old_attr_ptr, old_attr); }
                }
                Ok(SyscallResult::Success(0))
            },
            Err(e) => Ok(SyscallResult::Error(e.into())),
        }
    }

    fn name(&self) -> &str {
        "mq_getsetattr"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        number == 108 // CommonSyscall::MqGetsetattr
    }
}

/// Helper trait for reading C-style strings from user space
trait CStringReader {
    /// Read a C-style string from user space
    fn read_cstr(&self, ptr: *const u8) -> Result<String, ()>;
}

impl CStringReader for MqOpenHandler {
    fn read_cstr(&self, ptr: *const u8) -> Result<String, ()> {
        if ptr.is_null() {
            return Err(());
        }

        let mut buf = Vec::new();
        let mut offset = 0;
        
        loop {
            let byte = unsafe { ptr.add(offset).read() };
            
            if byte == 0 {
                break;
            }
            
            buf.push(byte);
            offset += 1;
            
            // Prevent infinite loops
            if offset > MQ_NAME_MAX + 1 {
                return Err(());
            }
        }
        
        String::from_utf8(buf).map_err(|_| ())
    }
}

/// Convert kernel errors to syscall errors
impl From<KernelError> for SyscallError {
    fn from(error: KernelError) -> Self {
        match error {
            KernelError::InvalidArgument => SyscallError::InvalidArgument,
            KernelError::NotFound => SyscallError::NotFound,
            KernelError::PermissionDenied => SyscallError::PermissionDenied,
            KernelError::AlreadyExists => SyscallError::AlreadyExists,
            KernelError::WouldBlock => SyscallError::WouldBlock,
            KernelError::NotConnected => SyscallError::NotConnected,
            KernelError::TimedOut => SyscallError::TimedOut,
            KernelError::NoMemory => SyscallError::NoMemory,
            KernelError::NoSpace => SyscallError::NoSpace,
            KernelError::NotSupported => SyscallError::NotSupported,
            _ => SyscallError::UnknownError,
        }
    }
}

/// Register all message queue system call handlers
pub fn register_handlers(dispatcher: &mut dyn crate::api::syscall::SyscallDispatcher) -> Result<(), KernelError> {
    dispatcher.register_handler(101, Box::new(MqOpenHandler));
    dispatcher.register_handler(102, Box::new(MqCloseHandler));
    dispatcher.register_handler(103, Box::new(MqGetattrHandler));
    dispatcher.register_handler(104, Box::new(MqSetattrHandler));
    dispatcher.register_handler(105, Box::new(MqTimedsendHandler));
    dispatcher.register_handler(106, Box::new(MqTimedreceiveHandler));
    dispatcher.register_handler(107, Box::new(MqNotifyHandler));
    dispatcher.register_handler(108, Box::new(MqGetsetattrHandler));
    
    Ok(())
}