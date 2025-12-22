//! Zero-copy network I/O system calls
//!
//! This module provides system call handlers for zero-copy network I/O operations.

#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
use nos_api::Result;
use crate::core::{SyscallHandler, SyscallDispatcher};

/// Zero-copy network manager placeholder
#[cfg(feature = "alloc")]
struct ZeroCopyNetworkManager;

#[cfg(feature = "alloc")]
impl ZeroCopyNetworkManager {
    fn new(_config: ()) -> Result<Self> {
        Ok(Self {})
    }
}

/// Zero-copy configuration
#[derive(Debug, Clone, Default)]
pub struct ZeroCopyConfig;

/// Zero-copy send system call handler
pub struct ZeroCopySendHandler {
    #[cfg(feature = "alloc")]
    manager: Arc<ZeroCopyNetworkManager>,
}

impl ZeroCopySendHandler {
    /// Create a new zero-copy send handler
    pub fn new() -> Self {
        Self::default()
    }
    
    #[cfg(feature = "alloc")]
    pub fn new_with_manager(manager: Arc<ZeroCopyNetworkManager>) -> Self {
        Self { manager }
    }
}

impl Default for ZeroCopySendHandler {
    fn default() -> Self {
        #[cfg(feature = "alloc")]
        {
            Self {
                manager: Arc::new(ZeroCopyNetworkManager::new(()).unwrap()),
            }
        }
        #[cfg(not(feature = "alloc"))]
        {
            Self {}
        }
    }
}

impl SyscallHandler for ZeroCopySendHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_SEND
    }
    
    fn name(&self) -> &str {
        "zero_copy_send"
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments for zero-copy send".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments for zero-copy send".into()));
        }
        
        let _fd = args[0] as i32;
        let _buffer_addr = args[1];
        let _buffer_size = args[2];
        
        // In a real implementation, this would perform zero-copy send
        // For now, just return the buffer size as success
        Ok(_buffer_size as isize)
    }
}

/// Zero-copy receive system call handler
pub struct ZeroCopyRecvHandler {
    #[cfg(feature = "alloc")]
    manager: Arc<ZeroCopyNetworkManager>,
}

impl ZeroCopyRecvHandler {
    /// Create a new zero-copy receive handler
    pub fn new() -> Self {
        Self::default()
    }
    
    #[cfg(feature = "alloc")]
    pub fn new_with_manager(manager: Arc<ZeroCopyNetworkManager>) -> Self {
        Self { manager }
    }
}

impl Default for ZeroCopyRecvHandler {
    fn default() -> Self {
        #[cfg(feature = "alloc")]
        {
            Self {
                manager: Arc::new(ZeroCopyNetworkManager::new(()).unwrap()),
            }
        }
        #[cfg(not(feature = "alloc"))]
        {
            Self {}
        }
    }
}

impl SyscallHandler for ZeroCopyRecvHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_RECV
    }
    
    fn name(&self) -> &str {
        "zero_copy_recv"
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments for zero-copy receive".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments for zero-copy receive".into()));
        }
        
        let _fd = args[0] as i32;
        let _buffer_addr = args[1];
        
        // In a real implementation, this would perform zero-copy receive
        // For now, just return a mock packet size
        Ok(1024 as isize) // Mock packet size
    }
}

/// Register zero-copy network I/O system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Register zero-copy send system call
    dispatcher.register_handler(
        crate::types::SYS_ZERO_COPY_SEND,
        Box::new(ZeroCopySendHandler::new())
    );
    
    // Register zero-copy receive system call
    dispatcher.register_handler(
        crate::types::SYS_ZERO_COPY_RECV,
        Box::new(ZeroCopyRecvHandler::new())
    );
    
    Ok(())
}

/// Register zero-copy network I/O system call handlers (no-alloc version)
#[cfg(not(feature = "alloc"))]
pub fn register_handlers(_dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // In no-alloc environments, handlers would need to be registered differently
    // For now, just return success
    Ok(())
}