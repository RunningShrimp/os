//! File system system calls
//!
//! This module provides file system related system calls.

use alloc::string::ToString;
use alloc::boxed::Box;
use nos_api::Result;
use crate::SyscallHandler;
use crate::SyscallDispatcher;

/// Register file system system call handlers
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Register read system call
    dispatcher.register_handler(
        crate::types::SYS_READ,
        Box::new(ReadHandler)
    );
    
    // Register write system call
    dispatcher.register_handler(
        crate::types::SYS_WRITE,
        Box::new(WriteHandler)
    );
    
    // Register open system call
    dispatcher.register_handler(
        crate::types::SYS_OPEN,
        Box::new(OpenHandler)
    );
    
    // Register close system call
    dispatcher.register_handler(
        crate::types::SYS_CLOSE,
        Box::new(CloseHandler)
    );
    
    Ok(())
}

/// Read system call handler
#[allow(clippy::should_implement_trait)]
pub struct ReadHandler;

impl ReadHandler {
    /// Create a new read handler
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for ReadHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_READ
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
        }
        
        let fd = args[0] as i32;
        let buf = args[1] as *mut u8;
        let count = args[2];
        
        // TODO: Implement actual read logic using parameters:
        // fd: File descriptor to read from
        // buf: Buffer to read data into
        // count: Maximum number of bytes to read
        sys_trace_with_args!("read called with: fd={}, buf={:?}, count={}", fd, buf, count);
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "read"
    }
}

/// Write system call handler
#[allow(clippy::should_implement_trait)]
pub struct WriteHandler;

impl WriteHandler {
    /// Create a new write handler
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for WriteHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_WRITE
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
        }
        
        let fd = args[0] as i32;
        let buf = args[1] as *const u8;
        let count = args[2];
        
        // TODO: Implement actual write logic using parameters:
        // fd: File descriptor to write to
        // buf: Buffer containing data to write
        // count: Number of bytes to write
        sys_trace_with_args!("write called with: fd={}, buf={:?}, count={}", fd, buf, count);
        
        Ok(count as isize)
    }
    
    fn name(&self) -> &str {
        "write"
    }
}

/// Open system call handler
#[allow(clippy::should_implement_trait)]
pub struct OpenHandler;

impl OpenHandler {
    /// Create a new open handler
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for OpenHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_OPEN
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
        }
        
        let pathname = args[0] as *const u8;
        let flags = args[1] as i32;
        let mode = args[2] as u32;
        
        // TODO: Implement actual open logic using parameters:
        // pathname: Path to the file to open
        // flags: Open flags (O_RDONLY, O_WRONLY, O_CREAT, etc.)
        // mode: File permissions (only used when O_CREAT is set)
        sys_trace_with_args!("open called with: pathname={:?}, flags={}, mode={}", pathname, flags, mode);
        
        Ok(3) // Return a dummy file descriptor
    }
    
    fn name(&self) -> &str {
        "open"
    }
}

/// Close system call handler
#[allow(clippy::should_implement_trait)]
pub struct CloseHandler;

impl CloseHandler {
    /// Create a new close handler
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for CloseHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_CLOSE
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.is_empty() {
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
        }
        
        let fd = args[0] as i32;

        // TODO: Implement actual close logic using parameter:
        // fd: File descriptor to close
        sys_trace_with_args!("close called with: fd={}", fd);

        Ok(0)
    }
    
    fn name(&self) -> &str {
        "close"
    }
}