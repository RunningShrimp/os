//! POSIX Message Queue System Call Handlers
//!
//! This module implements the system call handlers for POSIX message queues.
//! It provides the interface between user space and the kernel message queue implementation.

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use core::{ptr, slice};

use crate::{
    api::{
        error::{KernelError, Result},
        syscall::{SyscallHandler, SyscallNumber},
    },
    subsystems::syscalls::interface::{SyscallError, SyscallResult},
    posix::mqueue::{MqAttr, MqNotify, Message as PosixMessage},
};

//// Simple message queue storage
static mut MESSAGE_QUEUES: Option<BTreeMap<String, MqAttr>> = None;
static MESSAGE_QUEUE_INIT: spin::Once = spin::Once::new();

/// Message storage for queues
static mut MESSAGE_STORAGE: Option<BTreeMap<String, Vec<PosixMessage>>> = None;

fn get_message_queues() -> &'static mut BTreeMap<String, MqAttr> {
    unsafe {
        if MESSAGE_QUEUES.is_none() {
            MESSAGE_QUEUES = Some(BTreeMap::new());
        }
        MESSAGE_QUEUES.as_mut().unwrap()
    }
}

fn get_message_storage() -> &'static mut BTreeMap<String, Vec<PosixMessage>> {
    unsafe {
        if MESSAGE_STORAGE.is_none() {
            MESSAGE_STORAGE = Some(BTreeMap::new());
        }
        MESSAGE_STORAGE.as_mut().unwrap()
    }
}

/// Initialize message queues subsystem
///
/// This function ensures the message queue storage is initialized.
/// It uses a Once guard to ensure thread-safe one-time initialization.
fn init_message_queues() {
    MESSAGE_QUEUE_INIT.call_once(|| {
        // Initialize the message queues storage if not already done
        let _queues = get_message_queues();
        let _storage = get_message_storage();
        // The get_* functions handle initialization
    });
}

/// Maximum message queue name length
const MQ_NAME_MAX: usize = 255;

/// Message queue open system call handler
pub struct MqOpenHandler;

impl SyscallHandler for MqOpenHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        101 // CommonSyscall::MqOpen
    }

    fn get_name(&self) -> &'static str {
        "mq_open"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let _flags = args[1] as u32;
        let _mode = args[2] as u32;
        let attr_ptr = args[3] as *const MqAttr;

        // Validate name pointer
        if name_ptr.is_null() {
            return SyscallResult::Err(SyscallError::InvalidArgument);
        }

        // Read and validate name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        if name.len() > MQ_NAME_MAX {
            return SyscallResult::Err(SyscallError::NameTooLong);
        }

        // Validate name format (must start with '/')
        if !name.starts_with('/') || name.contains('\0') {
            return SyscallResult::Err(SyscallError::InvalidArgument);
        }

        // Read attributes if provided
        let attr = if !attr_ptr.is_null() {
            Some(unsafe { ptr::read(attr_ptr) })
        } else {
            None
        };

        // Initialize message queues if not already done
        init_message_queues();

        // Create or open message queue
        let queues = get_message_queues();
        let storage = get_message_storage();
        if queues.contains_key(&name) {
            // Queue already exists - in real implementation, would return file descriptor
            return SyscallResult::Ok(());
        } else {
            // Create new queue with default or provided attributes
            let attr = attr.unwrap_or_else(MqAttr::default);
            queues.insert(name.clone(), attr);
            // Initialize empty message storage for this queue
            storage.insert(name, Vec::new());
            return SyscallResult::Ok(());
        }
    }
}

/// Message queue close system call handler
pub struct MqCloseHandler;

impl SyscallHandler for MqCloseHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        102 // CommonSyscall::MqClose
    }

    fn get_name(&self) -> &'static str {
        "mqueue::mq_close"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        // Check if queue exists and remove it
        let queues = get_message_queues();
        let storage = get_message_storage();
        if queues.contains_key(&name) {
            queues.remove(&name);
            storage.remove(&name);
            SyscallResult::Ok(())
        } else {
            SyscallResult::Err(SyscallError::NotFound)
        }
    }
}

/// Message queue get attributes system call handler
pub struct MqGetattrHandler;

impl SyscallHandler for MqGetattrHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        103 // CommonSyscall::MqGetattr
    }

    fn get_name(&self) -> &'static str {
        "mqueue::mq_getattr"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let attr_ptr = args[1] as *mut MqAttr;

        // Validate attributes pointer
        if attr_ptr.is_null() {
            return SyscallResult::Err(SyscallError::InvalidArgument);
        }

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        let queues = get_message_queues();
        match queues.get(&name) {
            Some(attr) => {
                unsafe {
                    ptr::write(attr_ptr, *attr);
                }
                SyscallResult::Ok(())
            },
            None => SyscallResult::Err(SyscallError::NotFound),
        }
    }
}

/// Message queue set attributes system call handler
pub struct MqSetattrHandler;

impl SyscallHandler for MqSetattrHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        104 // CommonSyscall::MqSetattr
    }

    fn get_name(&self) -> &'static str {
        "mqueue::mq_setattr"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let new_attr_ptr = args[1] as *const MqAttr;
        let old_attr_ptr = args[2] as *mut MqAttr;

        // Validate new attributes pointer
        if new_attr_ptr.is_null() {
            return SyscallResult::Err(SyscallError::InvalidArgument);
        }

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        let new_attr = unsafe { ptr::read(new_attr_ptr) };

        // Get current attributes for old_attr if requested
        let old_attr = if !old_attr_ptr.is_null() {
            if let Some(attr) = get_message_queues().get(&name) {
                Some(*attr)
            } else {
                return SyscallResult::Err(SyscallError::NotFound);
            }
        } else {
            None
        };

        // Update the queue attributes
        get_message_queues().insert(name, new_attr);

        // Write old attributes if requested
        if !old_attr_ptr.is_null() {
            unsafe {
                ptr::write(old_attr_ptr, old_attr.unwrap());
            }
        }

        SyscallResult::Ok(())
    }
}

/// Message queue timed send system call handler
pub struct MqTimedsendHandler;

impl SyscallHandler for MqTimedsendHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        105 // CommonSyscall::MqTimedsend
    }

    fn get_name(&self) -> &'static str {
        "mq_timedsend"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let msg_ptr = args[1] as *const u8;
        let msg_len = args[2] as usize;
        let msg_prio = args[3] as u32;
        let timeout_ms = args[4] as u32;

        // Validate message pointer
        if msg_ptr.is_null() || msg_len == 0 {
            return SyscallResult::Err(SyscallError::InvalidArgument);
        }

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        // Read message
        let msg_data = unsafe { slice::from_raw_parts(msg_ptr, msg_len) }.to_vec();

        // Create message
        let msg = PosixMessage {
            priority: msg_prio,
            data: msg_data,
            timestamp: 0,
        };

        let storage = get_message_storage();
        match storage.get_mut(&name) {
            Some(messages) => {
                // Check if queue is full (simple implementation: max 100 messages)
                if messages.len() >= 100 {
                    return SyscallResult::Err(SyscallError::WouldBlock);
                }

                // Check timeout (simple implementation: if timeout_ms is 0, no wait)
                if timeout_ms == 0 {
                    return SyscallResult::Err(SyscallError::WouldBlock);
                }

                // Add message to queue
                messages.push(msg);
                SyscallResult::Ok(())
            },
            None => SyscallResult::Err(SyscallError::NotFound),
        }
    }
}

/// Message queue timed receive system call handler
pub struct MqTimedreceiveHandler;

impl SyscallHandler for MqTimedreceiveHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        106 // CommonSyscall::MqTimedreceive
    }

    fn get_name(&self) -> &'static str {
        "mq_timedreceive"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let msg_ptr = args[1] as *mut u8;
        let msg_len = args[2] as usize;
        let msg_prio_ptr = args[3] as *mut u32;
        let timeout_ms = args[4] as u32;

        // Validate message buffer pointer
        if msg_ptr.is_null() || msg_len == 0 {
            return SyscallResult::Err(SyscallError::InvalidArgument);
        }

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        let storage = get_message_storage();
        match storage.get_mut(&name) {
            Some(messages) => {
                // Check if queue is empty (simple implementation)
                if messages.is_empty() {
                    return SyscallResult::Err(SyscallError::WouldBlock);
                }

                // Check timeout (simple implementation: if timeout_ms is 0, no wait)
                if timeout_ms == 0 {
                    return SyscallResult::Err(SyscallError::WouldBlock);
                }

                // Get the first message (FIFO)
                let msg = messages.remove(0);

                // Copy message data to user buffer
                let copy_len = core::cmp::min(msg.data.len(), msg_len);
                unsafe {
                    ptr::copy_nonoverlapping(msg.data.as_ptr(), msg_ptr, copy_len);
                }

                // Set priority if requested
                if !msg_prio_ptr.is_null() {
                    unsafe {
                        ptr::write(msg_prio_ptr, msg.priority);
                    }
                }

                SyscallResult::Ok(())
            },
            None => SyscallResult::Err(SyscallError::NotFound),
        }
    }
}

/// Message queue notify system call handler
pub struct MqNotifyHandler;

impl SyscallHandler for MqNotifyHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        107 // CommonSyscall::MqNotify
    }

    fn get_name(&self) -> &'static str {
        "mqueue::mq_notify"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let notify_ptr = args[1] as *const MqNotify;

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        // GH-#1315: Implement full notification registration
        // See: https://github.com/npos/kernel/issues/1315
        // The notify parameter specifies how the process should be notified when a message arrives:
        // - If notify_ptr is null, cancel any existing notification
        // - If notify_ptr is non-null, register the notification (signal, eventfd, etc.)
        // Full implementation needs to:
        // 1. Parse the notification structure from notify_ptr
        // 2. Store it in a per-queue notification registry
        // 3. Trigger the notification when mq_timedsend adds a message to an empty queue
        // For now, we just validate the queue exists but don't read notify (avoid unused var)
        if !notify_ptr.is_null() {
            // Notify structure is provided but not yet implemented
            // unsafe { let _notify = ptr::read(notify_ptr); }
        }

        let queues = get_message_queues();
        if queues.contains_key(&name) {
            // In a real implementation, this would set up notification callbacks
            SyscallResult::Ok(())
        } else {
            SyscallResult::Err(SyscallError::NotFound)
        }
    }
}

/// Message queue get/set attributes system call handler
pub struct MqGetsetattrHandler;

impl SyscallHandler for MqGetsetattrHandler {
    fn get_syscall_number(&self) -> SyscallNumber {
        108 // CommonSyscall::MqGetsetattr
    }

    fn get_name(&self) -> &'static str {
        "mq_getsetattr"
    }

    fn handle(&self, args: &[u64]) -> SyscallResult<()> {
        let name_ptr = args[0] as *const u8;
        let new_attr_ptr = args[1] as *const MqAttr;
        let old_attr_ptr = args[2] as *mut MqAttr;

        // Read queue name
        let name = match read_cstr(name_ptr) {
            Ok(name) => name,
            Err(_) => return SyscallResult::Err(SyscallError::InvalidArgument),
        };

        let queues = get_message_queues();

        // Get current attributes for old_attr if requested
        if !old_attr_ptr.is_null() {
            match queues.get(&name) {
                Some(attr) => {
                    unsafe {
                        ptr::write(old_attr_ptr, *attr);
                    }
                },
                None => return SyscallResult::Err(SyscallError::NotFound),
            }
        }

        // Set new attributes if provided
        if !new_attr_ptr.is_null() {
            let new_attr = unsafe { ptr::read(new_attr_ptr) };
            queues.insert(name, new_attr);
        }

        SyscallResult::Ok(())
    }
}

/// Helper function for reading C-style strings from user space
fn read_cstr(ptr: *const u8) -> Result<String> {
    if ptr.is_null() {
        return Err(KernelError::InvalidArgument);
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
            return Err(KernelError::InvalidArgument);
        }
    }

    String::from_utf8(buf).map_err(|_| KernelError::InvalidArgument)
}
