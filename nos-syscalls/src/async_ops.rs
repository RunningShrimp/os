//! Asynchronous operations system calls
//!
//! This module provides system calls for asynchronous operations,
//! including async I/O, async file operations, and other async primitives.

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::format;
use nos_api::{Result, Error};
use spin::Mutex;
use crate::core::traits::SyscallHandler;
use crate::core::dispatcher::SyscallDispatcher;;

/// Async operation context
#[derive(Debug, Clone)]
pub struct AsyncContext {
    /// Context ID
    pub id: u64,
    /// Operation type
    pub operation_type: AsyncOperationType,
    /// Operation status
    pub status: AsyncStatus,
    /// Result data
    pub result: Option<isize>,
    /// Error code
    pub error: Option<i32>,
}

/// Async operation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncOperationType {
    Read,
    Write,
    Connect,
    Accept,
    Send,
    Recv,
    Custom(u32),
}

/// Async operation status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Async operation manager
pub struct AsyncManager {
    /// Next context ID
    next_id: Mutex<u64>,
    /// Active operations
    operations: Mutex<BTreeMap<u64, AsyncContext>>,
}

impl AsyncManager {
    /// Create a new async manager
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            operations: Mutex::new(BTreeMap::new()),
        }
    }
    
    /// Create a new async context
    pub fn create_context(&self, operation_type: AsyncOperationType) -> Result<u64> {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        
        let context = AsyncContext {
            id,
            operation_type,
            status: AsyncStatus::Pending,
            result: None,
            error: None,
        };
        
        let mut operations = self.operations.lock();
        operations.insert(id, context);
        
        Ok(id)
    }
    
    /// Get an async context
    pub fn get_context(&self, id: u64) -> Option<AsyncContext> {
        let operations = self.operations.lock();
        operations.get(&id).cloned()
    }
    
    /// Update an async context
    pub fn update_context(&self, id: u64, status: AsyncStatus, result: Option<isize>, error: Option<i32>) -> Result<()> {
        let mut operations = self.operations.lock();
        if let Some(context) = operations.get_mut(&id) {
            context.status = status;
            context.result = result;
            context.error = error;
            Ok(())
        } else {
            #[cfg(feature = "alloc")]
            return Err(Error::NotFound(format!("Async context {} not found", id)));
            #[cfg(not(feature = "alloc"))]
            return Err(Error::NotFound("Async context not found".into()));
        }
    }
    
    /// Complete an async operation
    pub fn complete_operation(&self, id: u64, result: isize) -> Result<()> {
        self.update_context(id, AsyncStatus::Completed, Some(result), None)
    }
    
    /// Fail an async operation
    pub fn fail_operation(&self, id: u64, error: i32) -> Result<()> {
        self.update_context(id, AsyncStatus::Failed, None, Some(error))
    }
    
    /// Cancel an async operation
    pub fn cancel_operation(&self, id: u64) -> Result<()> {
        self.update_context(id, AsyncStatus::Cancelled, None, None)
    }
    
    /// Remove a completed operation
    pub fn remove_operation(&self, id: u64) -> Result<()> {
        let mut operations = self.operations.lock();
        #[cfg(feature = "alloc")]
        {
            operations.remove(&id).ok_or_else(|| Error::NotFound(format!("Async context {} not found", id)))?;
        }
        #[cfg(not(feature = "alloc"))]
        {
            operations.remove(&id).ok_or_else(|| Error::NotFound("Async context not found".into()))?;
        }
        Ok(())
    }
}

/// Async operation system call handler
pub struct AsyncOpHandler {
    /// Async manager
    manager: Arc<AsyncManager>,
}

impl AsyncOpHandler {
    /// Create a new async operation handler
    pub fn new() -> Self {
        Self {
            manager: Arc::new(AsyncManager::new()),
        }
    }
    
    /// Get the async manager
    pub fn manager(&self) -> &Arc<AsyncManager> {
        &self.manager
    }
}

impl SyscallHandler for AsyncOpHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ASYNC_OP
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            #[cfg(feature = "alloc")]
            return Err(Error::InvalidArgument("Insufficient arguments for async operation".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(Error::InvalidArgument("Insufficient arguments for async operation".into()));
        }

        let operation_type = match args[0] {
            0 => AsyncOperationType::Read,
            1 => AsyncOperationType::Write,
            2 => AsyncOperationType::Connect,
            3 => AsyncOperationType::Accept,
            4 => AsyncOperationType::Send,
            5 => AsyncOperationType::Recv,
            custom => AsyncOperationType::Custom(custom as u32),
        };

        // Create async context
        let context_id = self.manager.create_context(operation_type)?;
        
        // In a real implementation, we would start the async operation here
        // For now, we'll just return the context ID
        Ok(context_id as isize)
    }
    
    fn name(&self) -> &str {
        "async_op"
    }
}

/// Async wait system call handler
pub struct AsyncWaitHandler {
    /// Async manager
    manager: Arc<AsyncManager>,
}

impl AsyncWaitHandler {
    /// Create a new async wait handler
    pub fn new(manager: Arc<AsyncManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for AsyncWaitHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ASYNC_WAIT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 1 {
            #[cfg(feature = "alloc")]
            return Err(Error::InvalidArgument("Insufficient arguments for async wait".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(Error::InvalidArgument("Insufficient arguments for async wait".into()));
        }

        let context_id = args[0] as u64;
        
        // Get the async context
        #[cfg(feature = "alloc")]
        let context = self.manager.get_context(context_id)
            .ok_or_else(|| Error::NotFound("Async context not found".to_string()))?;
        #[cfg(not(feature = "alloc"))]
        let context = self.manager.get_context(context_id)
            .ok_or_else(|| Error::NotFound("Async context not found".into()))?;
        
        // Check if the operation is completed
        #[cfg(feature = "alloc")]
        match context.status {
            AsyncStatus::Completed => Ok(context.result.unwrap_or(0)),
            AsyncStatus::Failed => Err(Error::InvalidArgument("Operation failed".to_string())),
            AsyncStatus::Cancelled => Err(Error::InvalidArgument("Operation cancelled".to_string())),
            _ => Err(Error::InvalidArgument("Operation still in progress".to_string())),
        }
        #[cfg(not(feature = "alloc"))]
        match context.status {
            AsyncStatus::Completed => Ok(context.result.unwrap_or(0)),
            AsyncStatus::Failed => Err(Error::InvalidArgument("Operation failed".into())),
            AsyncStatus::Cancelled => Err(Error::InvalidArgument("Operation cancelled".into())),
            _ => Err(Error::InvalidArgument("Operation still in progress".into())),
        }
    }
    
    fn name(&self) -> &str {
        "async_wait"
    }
}

/// Register async operation system calls
pub fn register_syscalls(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    #[cfg(feature = "alloc")]
    {
        let async_handler = AsyncOpHandler::new();
        let manager = async_handler.manager().clone();
        
        dispatcher.register_handler(310, Box::new(async_handler));
        dispatcher.register_handler(311, Box::new(AsyncWaitHandler::new(manager)));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_manager() {
        let manager = AsyncManager::new();
        
        // Create a context
        let id = manager.create_context(AsyncOperationType::Read).unwrap();
        assert!(id > 0);
        
        // Get the context
        let context = manager.get_context(id).unwrap();
        assert_eq!(context.id, id);
        assert_eq!(context.operation_type, AsyncOperationType::Read);
        assert_eq!(context.status, AsyncStatus::Pending);
        
        // Complete the operation
        manager.complete_operation(id, 42).unwrap();
        
        // Check the context again
        let context = manager.get_context(id).unwrap();
        assert_eq!(context.status, AsyncStatus::Completed);
        assert_eq!(context.result, Some(42));
    }
    
    #[test]
    fn test_async_handler() {
        let handler = AsyncOpHandler::new();
        assert_eq!(handler.name(), "async_op");
        
        // Test with insufficient arguments
        let result = handler.execute(&[]);
        assert!(result.is_err());
        
        // Test with valid arguments
        let result = handler.execute(&[0, 0]);
        assert!(result.is_ok());
        
        let context_id = result.unwrap() as u64;
        let context = handler.manager().get_context(context_id).unwrap();
        assert_eq!(context.operation_type, AsyncOperationType::Read);
    }
}