//! Zero-copy network I/O system calls
//!
//! This module provides system calls for zero-copy network operations,
//! which eliminate data copying between kernel and user space for improved performance.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::format;
use alloc::vec::Vec;
use nos_api::{Result, Error};
use spin::Mutex;
use crate::{SyscallHandler, SyscallDispatcher};

/// Zero-copy buffer flags
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum ZeroCopyFlags {
    Read = 0x1,
    Write = 0x2,
    ReadWrite = 0x3,
    ReadOnly = 0x4,
    WriteOnly = 0x5,
    Persistent = 0x10,
    Ephemeral = 0x20,
    Aligned = 0x40,
}

impl ZeroCopyFlags {
    pub fn from_bits(bits: u32) -> Self {
        match bits & 0x75 {
            0x1 => Self::Read,
            0x2 => Self::Write,
            0x3 => Self::ReadWrite,
            0x4 => Self::ReadOnly,
            0x5 => Self::WriteOnly,
            0x10 => Self::Persistent,
            0x20 => Self::Ephemeral,
            0x40 => Self::Aligned,
            _ => Self::ReadWrite,
        }
    }

    pub fn to_bits(&self) -> u32 {
        *self as u32
    }
}

/// Zero-copy operation type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZeroCopyOpType {
    Send,
    Recv,
    SendMsg,
    RecvMsg,
    Sendfile,
    Splice,
    Tee,
}

/// Zero-copy buffer
#[derive(Debug, Clone)]
pub struct ZeroCopyBuffer {
    /// Buffer ID
    pub id: u64,
    /// Buffer address
    pub addr: usize,
    /// Buffer size
    pub size: usize,
    /// Buffer flags
    pub flags: ZeroCopyFlags,
    /// Reference count
    pub refcount: u32,
    /// Owner socket FD
    pub owner_fd: Option<i32>,
}

/// Zero-copy network socket
#[derive(Debug)]
pub struct ZeroCopySocket {
    /// Socket file descriptor
    pub fd: i32,
    /// Associated buffers
    pub buffers: Mutex<BTreeMap<u64, Arc<ZeroCopyBuffer>>>,
    /// Send queue
    pub send_queue: Mutex<Vec<ZeroCopyBuffer>>,
    /// Recv queue
    pub recv_queue: Mutex<Vec<ZeroCopyBuffer>>,
    /// Maximum buffer size
    pub max_buffer_size: usize,
    /// Enable zero-copy
    pub zero_copy_enabled: bool,
}

impl ZeroCopySocket {
    pub fn new(fd: i32, max_buffer_size: usize) -> Self {
        Self {
            fd,
            buffers: Mutex::new(BTreeMap::new()),
            send_queue: Mutex::new(Vec::new()),
            recv_queue: Mutex::new(Vec::new()),
            max_buffer_size,
            zero_copy_enabled: true,
        }
    }
    
    pub fn register_buffer(&self, buffer: ZeroCopyBuffer) -> Result<()> {
        if buffer.size > self.max_buffer_size {
            return Err(Error::InvalidArgument(format!("Buffer size {} exceeds maximum {}",
                buffer.size, self.max_buffer_size)));
        }

        let mut buffers = self.buffers.lock();
        let buffer_id = buffer.id;
        buffers.insert(buffer_id, Arc::new(buffer));
        sys_trace_with_args!("Registered buffer {} for socket fd {}", buffer_id, self.fd);
        Ok(())
    }
    
    pub fn unregister_buffer(&self, id: u64) -> Result<()> {
        let mut buffers = self.buffers.lock();
        buffers.remove(&id).ok_or_else(|| Error::NotFound(format!("Buffer {} not found", id)))?;
        sys_trace_with_args!("Unregistered buffer {} from socket fd {}", id, self.fd);
        Ok(())
    }
    
    pub fn get_buffer(&self, id: u64) -> Option<Arc<ZeroCopyBuffer>> {
        let buffers = self.buffers.lock();
        buffers.get(&id).cloned()
    }
    
    pub fn queue_send(&self, buffer: ZeroCopyBuffer) -> Result<()> {
        if !self.zero_copy_enabled {
            return Err(Error::InvalidState("Zero-copy is disabled".to_string()));
        }

        let mut send_queue = self.send_queue.lock();
        if send_queue.len() >= 64 {
            return Err(Error::Busy("Send queue is full".to_string()));
        }

        send_queue.push(buffer);
        Ok(())
    }

    pub fn queue_recv(&self, buffer: ZeroCopyBuffer) -> Result<()> {
        if !self.zero_copy_enabled {
            return Err(Error::InvalidState("Zero-copy is disabled".to_string()));
        }

        let mut recv_queue = self.recv_queue.lock();
        if recv_queue.len() >= 64 {
            return Err(Error::Busy("Receive queue is full".to_string()));
        }

        recv_queue.push(buffer);
        Ok(())
    }
    
    pub fn enable_zero_copy(&mut self) {
        self.zero_copy_enabled = true;
    }
    
    pub fn disable_zero_copy(&mut self) {
        self.zero_copy_enabled = false;
    }
    
    pub fn is_zero_copy_enabled(&self) -> bool {
        self.zero_copy_enabled
    }
}

/// Zero-copy network manager
pub struct ZeroCopyManager {
    /// Next buffer ID
    next_buffer_id: Mutex<u64>,
    /// Registered sockets
    sockets: Mutex<BTreeMap<i32, Arc<ZeroCopySocket>>>,
    /// Next available address
    next_addr: Mutex<usize>,
    /// Total allocated bytes
    total_allocated: Mutex<usize>,
}

impl Default for ZeroCopyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroCopyManager {
    pub fn new() -> Self {
        Self {
            next_buffer_id: Mutex::new(1),
            sockets: Mutex::new(BTreeMap::new()),
            next_addr: Mutex::new(0x80000000000usize),
            total_allocated: Mutex::new(0),
        }
    }
    
    pub fn create_socket(&self, fd: i32, max_buffer_size: usize) -> Result<u64> {
        let socket = Arc::new(ZeroCopySocket::new(fd, max_buffer_size));
        
        let mut sockets = self.sockets.lock();
        sockets.insert(fd, socket);
        
        sys_trace_with_args!("Created zero-copy socket for fd {}", fd);
        
        Ok(fd as u64)
    }
    
    pub fn get_socket(&self, fd: i32) -> Option<Arc<ZeroCopySocket>> {
        let sockets = self.sockets.lock();
        sockets.get(&fd).cloned()
    }
    
    pub fn close_socket(&self, fd: i32) -> Result<()> {
        let mut sockets = self.sockets.lock();
        sockets.remove(&fd).ok_or_else(|| Error::NotFound(format!("Socket fd {} not found", fd)))?;
        sys_trace_with_args!("Closed zero-copy socket fd {}", fd);
        Ok(())
    }
    
    pub fn create_buffer(&self, fd: i32, size: usize, flags: ZeroCopyFlags) -> Result<ZeroCopyBuffer> {
        let socket = self.get_socket(fd)
            .ok_or_else(|| Error::NotFound(format!("Socket fd {} not found", fd)))?;
        
        let mut next_id = self.next_buffer_id.lock();
        let id = *next_id;
        *next_id += 1;
        
        let mut next_addr = self.next_addr.lock();
        let addr = *next_addr;
        *next_addr += size;
        
        let mut total_allocated = self.total_allocated.lock();
        *total_allocated += size;
        
        let buffer = ZeroCopyBuffer {
            id,
            addr,
            size,
            flags,
            refcount: 1,
            owner_fd: Some(fd),
        };
        
        socket.register_buffer(buffer.clone())?;
        
        sys_trace_with_args!("Created zero-copy buffer {} at addr {:#x}, size={}, fd={}", 
                   id, addr, size, fd);
        
        Ok(buffer)
    }
    
    pub fn get_allocated_bytes(&self) -> usize {
        *self.total_allocated.lock()
    }
}

/// Zero-copy send system call handler
pub struct ZeroCopySendHandler {
    manager: Arc<ZeroCopyManager>,
}

impl ZeroCopySendHandler {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(ZeroCopyManager::new()),
        }
    }
    
    pub fn manager(&self) -> &Arc<ZeroCopyManager> {
        &self.manager
    }
}

impl Default for ZeroCopySendHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallHandler for ZeroCopySendHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_SEND
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 4 {
            return Err(Error::InvalidArgument("Insufficient arguments for zero-copy send".to_string()));
        }

        let fd = args[0] as i32;
        let buffer_id = args[1] as u64;
        let offset = args[2];
        let length = args[3];
        
        let socket = self.manager.get_socket(fd)
            .ok_or_else(|| Error::NotFound(format!("Socket fd {} not found", fd)))?;
        
        let buffer = socket.get_buffer(buffer_id)
            .ok_or_else(|| Error::NotFound(format!("Buffer {} not found", buffer_id)))?;
        
        if offset + length > buffer.size {
            return Err(Error::InvalidArgument("Offset + length exceeds buffer size".to_string()));
        }
        
        socket.queue_send((*buffer).clone())?;
        
        sys_trace_with_args!("zero_copy_send: fd={}, buffer_id={}, offset={}, length={}",
                   fd, buffer_id, offset, length);
        
        Ok(length as isize)
    }
    
    fn name(&self) -> &str {
        "zero_copy_send"
    }
}

/// Zero-copy receive system call handler
pub struct ZeroCopyRecvHandler {
    manager: Arc<ZeroCopyManager>,
}

impl ZeroCopyRecvHandler {
    pub fn new(manager: Arc<ZeroCopyManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for ZeroCopyRecvHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_RECV
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 4 {
            return Err(Error::InvalidArgument("Insufficient arguments for zero-copy recv".to_string()));
        }

        let fd = args[0] as i32;
        let buffer_id = args[1] as u64;
        let offset = args[2];
        let length = args[3];
        
        let socket = self.manager.get_socket(fd)
            .ok_or_else(|| Error::NotFound(format!("Socket fd {} not found", fd)))?;
        
        let buffer = socket.get_buffer(buffer_id)
            .ok_or_else(|| Error::NotFound(format!("Buffer {} not found", buffer_id)))?;
        
        if offset + length > buffer.size {
            return Err(Error::InvalidArgument("Offset + length exceeds buffer size".to_string()));
        }
        
        socket.queue_recv((*buffer).clone())?;
        
        sys_trace_with_args!("zero_copy_recv: fd={}, buffer_id={}, offset={}, length={}",
                   fd, buffer_id, offset, length);
        
        Ok(length as isize)
    }
    
    fn name(&self) -> &str {
        "zero_copy_recv"
    }
}

/// Zero-copy buffer create system call handler
pub struct ZeroCopyCreateBufferHandler {
    manager: Arc<ZeroCopyManager>,
}

impl ZeroCopyCreateBufferHandler {
    pub fn new(manager: Arc<ZeroCopyManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for ZeroCopyCreateBufferHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_CREATE_BUFFER
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            return Err(Error::InvalidArgument("Insufficient arguments for create buffer".to_string()));
        }

        let fd = args[0] as i32;
        let size = args[1];
        let flags = ZeroCopyFlags::from_bits(args[2] as u32);
        
        let buffer = self.manager.create_buffer(fd, size, flags)?;
        
        Ok(buffer.id as isize)
    }
    
    fn name(&self) -> &str {
        "zero_copy_create_buffer"
    }
}

/// Zero-copy buffer destroy system call handler
pub struct ZeroCopyDestroyBufferHandler {
    manager: Arc<ZeroCopyManager>,
}

impl ZeroCopyDestroyBufferHandler {
    pub fn new(manager: Arc<ZeroCopyManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for ZeroCopyDestroyBufferHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_DESTROY_BUFFER
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            return Err(Error::InvalidArgument("Insufficient arguments for destroy buffer".to_string()));
        }

        let fd = args[0] as i32;
        let buffer_id = args[1] as u64;
        
        let socket = self.manager.get_socket(fd)
            .ok_or_else(|| Error::NotFound(format!("Socket fd {} not found", fd)))?;
        
        socket.unregister_buffer(buffer_id)?;
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "zero_copy_destroy_buffer"
    }
}

/// Register zero-copy network system calls
pub fn register_syscalls(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    let handler = ZeroCopySendHandler::new();
    let manager = handler.manager().clone();
    
    dispatcher.register_handler(2000, Box::new(handler));
    dispatcher.register_handler(2001, Box::new(ZeroCopyRecvHandler::new(manager.clone())));
    dispatcher.register_handler(2002, Box::new(ZeroCopyCreateBufferHandler::new(manager.clone())));
    dispatcher.register_handler(2003, Box::new(ZeroCopyDestroyBufferHandler::new(manager)));
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_flags() {
        assert_eq!(ZeroCopyFlags::from_bits(0x1), ZeroCopyFlags::Read);
        assert_eq!(ZeroCopyFlags::from_bits(0x2), ZeroCopyFlags::Write);
        assert_eq!(ZeroCopyFlags::from_bits(0x3), ZeroCopyFlags::ReadWrite);
        assert_eq!(ZeroCopyFlags::ReadWrite.to_bits(), 0x3);
    }

    #[test]
    fn test_zero_copy_socket() {
        let socket = ZeroCopySocket::new(3, 4096);
        
        let buffer = ZeroCopyBuffer {
            id: 1,
            addr: 0x1000,
            size: 1024,
            flags: ZeroCopyFlags::ReadWrite,
            refcount: 1,
            owner_fd: Some(3),
        };
        
        socket.register_buffer(buffer.clone()).unwrap();
        assert!(socket.get_buffer(1).is_some());
        
        socket.unregister_buffer(1).unwrap();
        assert!(socket.get_buffer(1).is_none());
        
        assert_eq!(socket.is_zero_copy_enabled(), true);
    }

    #[test]
    fn test_zero_copy_manager() {
        let manager = ZeroCopyManager::new();
        
        let _ = manager.create_socket(3, 4096).unwrap();
        assert!(manager.get_socket(3).is_some());
        
        let buffer = manager.create_buffer(3, 1024, ZeroCopyFlags::ReadWrite).unwrap();
        assert_eq!(buffer.owner_fd, Some(3));
        
        assert_eq!(manager.get_allocated_bytes(), 1024);
        
        manager.close_socket(3).unwrap();
        assert!(manager.get_socket(3).is_none());
    }

    #[test]
    fn test_zero_copy_handler() {
        let handler = ZeroCopySendHandler::new();
        assert_eq!(handler.name(), "zero_copy_send");
        
        let result = handler.execute(&[]);
        assert!(result.is_err());
    }
}
