//! Type stubs for missing modules
//!
//! This module provides placeholder type definitions for modules that
//! haven't been fully implemented yet, allowing compilation to proceed.

extern crate alloc;
use alloc::vec::Vec;
use heapless::String as HeaplessString;

// Microkernel IPC types - using real implementations
pub use crate::microkernel::service_registry::{
    ServiceId
};
pub use crate::subsystems::microkernel::ipc::IpcMessage as Message;

/// Message type for IPC communication
/// Maps to the message_type field in IpcMessage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageType(pub u32);

impl MessageType {
    pub const REQUEST: MessageType = MessageType(0);
    pub const RESPONSE: MessageType = MessageType(1);
    pub const EVENT: MessageType = MessageType(2);
    pub const NOTIFICATION: MessageType = MessageType(3);
    
    pub fn new(msg_type: u32) -> Self {
        MessageType(msg_type)
    }
    
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Message {
    /// Create a new message with the given type and data
    pub fn new_with_type(message_type: MessageType, data: Vec<u8>) -> Self {
        // Use a default sender/receiver ID (0 means system)
        crate::subsystems::microkernel::ipc::IpcMessage::new(0, 0, message_type.as_u32(), data)
    }
    
    /// Create a new request message
    pub fn new_request(data: Vec<u8>) -> Self {
        Self::new_with_type(MessageType::REQUEST, data)
    }
    
    /// Create a new response message
    pub fn new_response(data: Vec<u8>) -> Self {
        Self::new_with_type(MessageType::RESPONSE, data)
    }
    
    /// Get the message type
    pub fn message_type(&self) -> MessageType {
        MessageType(self.message_type)
    }
}

// IPC function implementations using real IPC system
pub fn send_message(service_id: ServiceId, message: Message) -> Result<(), ()> {
    use crate::subsystems::microkernel::ipc;
    
    // Get the IPC manager instance
    let manager = match ipc::get_ipc_manager() {
        Some(m) => m,
        None => {
            // Initialize IPC if not already done
            let _ = ipc::init();
            ipc::get_ipc_manager().ok_or(())?
        }
    };
    
    // Find the message queue for the service
    // In a real implementation, we'd look up the queue_id from service_registry
    // For now, use service_id as queue_id (simplified)
    match manager.send_message(service_id, message) {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

pub fn receive_message() -> Result<Message, ()> {
    use crate::subsystems::microkernel::ipc;
    
    // Get the IPC manager instance
    let manager = match ipc::get_ipc_manager() {
        Some(m) => m,
        None => {
            // Initialize IPC if not already done
            let _ = ipc::init();
            ipc::get_ipc_manager().ok_or(())?
        }
    };
    
    // Receive from default queue (queue_id 0)
    // In a real implementation, we'd get the queue_id from the current service context
    match manager.receive_message(0, 0) {
        Ok(msg) => Ok(msg),
        Err(_) => Err(()),
    }
}

// POSIX type stubs - These should be moved to posix module
// For now, re-export from posix module if available, otherwise keep as stubs
#[allow(unused_imports)]
use crate::posix::{Pid, Uid, Gid};

// Re-export POSIX types (use posix module types if available)
pub type PidT = crate::posix::Pid;
pub type UidT = crate::posix::Uid;
pub type GidT = crate::posix::Gid;
pub type AfUnix = i32;

pub const AF_UNIX_CONST: AfUnix = 1;

// Service registry - using real implementation
// TODO: Re-enable when service registry is fully implemented
// pub use crate::subsystems::microkernel::service_registry::{ServiceRegistry, get_service_registry};

// Process stubs - Use real Process type from process module when possible
// For compatibility, keep a minimal stub but prefer using crate::process::Proc
#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: HeaplessString<64>,
}

impl Process {
    pub fn new(pid: u32, name: &str) -> Self {
        Self {
            pid,
            name: HeaplessString::try_from(name).unwrap_or_else(|_| HeaplessString::new()),
        }
    }

    pub fn pid(&self) -> u64 {
        self.pid as u64
    }
}

// TODO: Replace Process stub with crate::process::Proc when all usages are updated

// Memory address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    pub fn new(addr: usize) -> Self {
        VirtAddr(addr)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

// RNG stub
pub struct RNG;

impl RNG {
    pub fn get_random(&self) -> usize {
        // Simple pseudo-random generator based on a static seed
        use core::sync::atomic::{AtomicUsize, Ordering};
        static SEED: AtomicUsize = AtomicUsize::new(12345);
        let seed = SEED.fetch_add(1103515245, Ordering::SeqCst);
        seed.wrapping_mul(1103515245).wrapping_add(12345)
    }
}

pub const RNG_INSTANCE: RNG = RNG;
// ServiceStatus and InterfaceVersion are already re-exported above

// Error handling stubs
pub mod errno {
    pub const EPERM: i32 = 1;
    pub const EACCES: i32 = 13;
    pub const ENOENT: i32 = 2;
}

pub struct VfsNode;
pub struct FileMode;

// ServiceCategory constants are available from the real implementation
// Use ServiceCategory::System, ServiceCategory::Network, etc.

// IPC manager stubs
pub struct IpcManager;
pub struct IpcMessage;

impl IpcManager {
    pub fn get() -> &'static IpcManager {
        static INSTANCE: IpcManager = IpcManager;
        &INSTANCE
    }
}

// Memory manager stubs
pub mod memory {
    pub struct MicroMemoryManager;

    impl MicroMemoryManager {
        pub fn get() -> &'static MicroMemoryManager {
            static INSTANCE: MicroMemoryManager = MicroMemoryManager;
            &INSTANCE
        }
    }
}


// ServiceInfo::new and InterfaceVersion::new are available from the real implementation

// MessageQueue stub
pub struct MessageQueue;

impl MessageQueue {
    pub fn new(_service_id: ServiceId, _capacity: usize) -> Result<Self, &'static str> {
        Ok(MessageQueue)
    }
}

// POSIX socket constants
pub const AF_UNIX: i32 = 1;
pub const AF_INET: i32 = 2;
pub const AF_INET6: i32 = 10;
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;
pub const SOCK_RAW: i32 = 3;

// Additional type stubs for re-exporting core atomic types
// TODO: Re-enable when atomic types are needed
// pub use core::sync::atomic::{AtomicU32, AtomicU64};

// Device driver trait stubs
pub trait BlockDevice {
    fn read(&self, sector: usize, buf: &mut [u8]);
    fn write(&self, sector: usize, buf: &[u8]);
}

// Debug stubs
pub fn log_info(_msg: &str) {
    // Placeholder implementation
}

// Additional function stubs needed by security modules
pub fn get_timestamp() -> u64 {
    use core::sync::atomic::{AtomicU64, Ordering};
    static TIMESTAMP: AtomicU64 = AtomicU64::new(1000000);
    TIMESTAMP.fetch_add(1, Ordering::Relaxed)
}

pub fn kill_process(_pid: u64, _signal: i32) {
    // Stub implementation
}
