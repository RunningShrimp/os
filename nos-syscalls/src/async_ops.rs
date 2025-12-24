//! Asynchronous operations system calls
//!
//! This module provides system calls for asynchronous operations,
//! including async I/O, async file operations, and other async primitives.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::format;
use alloc::vec::Vec;
use nos_api::{Result, Error};
use spin::Mutex;
use crate::{SyscallHandler, SyscallDispatcher};

/// Async operation error codes
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum AsyncError {
    Success = 0,
    Invalid = -1,
    NotFound = -2,
    Timeout = -3,
    Cancelled = -4,
    InProgress = -5,
    AlreadyCompleted = -6,
    ResourceBusy = -7,
}

impl AsyncError {
    pub fn to_i32(&self) -> i32 {
        *self as i32
    }
    
    pub fn from_i32(code: i32) -> Self {
        match code {
            0 => Self::Success,
            -1 => Self::Invalid,
            -2 => Self::NotFound,
            -3 => Self::Timeout,
            -4 => Self::Cancelled,
            -5 => Self::InProgress,
            -6 => Self::AlreadyCompleted,
            -7 => Self::ResourceBusy,
            _ => Self::Invalid,
        }
    }
}

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
    pub error: Option<AsyncError>,
    /// Start time (ticks)
    pub start_time: u64,
    /// End time (ticks)
    pub end_time: Option<u64>,
    /// Timeout (ms)
    pub timeout_ms: Option<u32>,
    /// Priority
    pub priority: AsyncPriority,
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
    Fsync,
    Sync,
    Ioctl,
    Poll,
    #[cfg(feature = "advanced_syscalls")]
    Custom(u32),
}

/// Async operation status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncStatus {
    Pending,
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Async operation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AsyncPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

impl AsyncPriority {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Low,
            1 => Self::Normal,
            2 => Self::High,
            3 => Self::Urgent,
            _ => Self::Normal,
        }
    }
}

/// Async operation manager
pub struct AsyncManager {
    /// Next context ID
    next_id: Mutex<u64>,
    /// Active operations
    operations: Mutex<BTreeMap<u64, AsyncContext>>,
    /// Priority queue
    priority_queue: Mutex<Vec<u64>>,
    /// Completed operations count
    completed_count: Mutex<u64>,
    /// Failed operations count
    failed_count: Mutex<u64>,
}

impl Default for AsyncManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncManager {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            operations: Mutex::new(BTreeMap::new()),
            priority_queue: Mutex::new(Vec::new()),
            completed_count: Mutex::new(0),
            failed_count: Mutex::new(0),
        }
    }
    
    pub fn create_context(&self, operation_type: AsyncOperationType, priority: AsyncPriority, timeout_ms: Option<u32>) -> Result<u64> {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        
        let context = AsyncContext {
            id,
            operation_type,
            status: AsyncStatus::Pending,
            result: None,
            error: None,
            start_time: 0,
            end_time: None,
            timeout_ms,
            priority,
        };
        
        let mut operations = self.operations.lock();
        operations.insert(id, context.clone());
        
        let mut priority_queue = self.priority_queue.lock();
        priority_queue.push(id);
        
        sys_trace_with_args!("Created async context: id={}, type={:?}, priority={:?}", 
                   id, operation_type, priority);
        
        Ok(id)
    }
    
    pub fn get_context(&self, id: u64) -> Option<AsyncContext> {
        let operations = self.operations.lock();
        operations.get(&id).cloned()
    }
    
    pub fn update_context(&self, id: u64, status: AsyncStatus, result: Option<isize>, error: Option<AsyncError>) -> Result<()> {
        let mut operations = self.operations.lock();
        if let Some(context) = operations.get_mut(&id) {
            context.status = status;
            context.result = result;
            context.error = error;
            if status == AsyncStatus::Completed || status == AsyncStatus::Failed || status == AsyncStatus::Cancelled || status == AsyncStatus::TimedOut {
                context.end_time = Some(0);
            }
            Ok(())
        } else {
            Err(Error::NotFound(format!("Async context {} not found", id)))
        }
    }
    
    pub fn complete_operation(&self, id: u64, result: isize) -> Result<()> {
        self.update_context(id, AsyncStatus::Completed, Some(result), None)?;
        *self.completed_count.lock() += 1;
        sys_trace_with_args!("Completed async operation: id={}, result={}", id, result);
        Ok(())
    }
    
    pub fn fail_operation(&self, id: u64, error: AsyncError) -> Result<()> {
        self.update_context(id, AsyncStatus::Failed, None, Some(error))?;
        *self.failed_count.lock() += 1;
        sys_trace_with_args!("Failed async operation: id={}, error={:?}", id, error);
        Ok(())
    }
    
    pub fn cancel_operation(&self, id: u64) -> Result<()> {
        let context = self.get_context(id).ok_or_else(|| Error::NotFound("Async context not found".to_string()))?;
        if matches!(context.status, AsyncStatus::Completed | AsyncStatus::Failed | AsyncStatus::Cancelled) {
            return Err(Error::InvalidState("Cannot cancel completed operation".to_string()));
        }
        self.update_context(id, AsyncStatus::Cancelled, None, Some(AsyncError::Cancelled))?;
        Ok(())
    }
    
    pub fn timeout_operation(&self, id: u64) -> Result<()> {
        let context = self.get_context(id).ok_or_else(|| Error::NotFound("Async context not found".to_string()))?;
        if matches!(context.status, AsyncStatus::Completed | AsyncStatus::Failed | AsyncStatus::Cancelled | AsyncStatus::TimedOut) {
            return Err(Error::InvalidState("Operation already terminated".to_string()));
        }
        self.update_context(id, AsyncStatus::TimedOut, None, Some(AsyncError::Timeout))?;
        Ok(())
    }
    
    pub fn remove_operation(&self, id: u64) -> Result<()> {
        let mut operations = self.operations.lock();
        operations.remove(&id).ok_or_else(|| Error::NotFound(format!("Async context {} not found", id)))?;
        Ok(())
    }
    
    pub fn get_stats(&self) -> AsyncStats {
        let completed = *self.completed_count.lock();
        let failed = *self.failed_count.lock();
        let operations = self.operations.lock();
        let active = operations.len() as u64;
        
        AsyncStats {
            total_operations: completed + failed + active,
            completed_operations: completed,
            failed_operations: failed,
            active_operations: active,
        }
    }
}

/// Async operation statistics
#[derive(Debug, Clone, Copy)]
pub struct AsyncStats {
    pub total_operations: u64,
    pub completed_operations: u64,
    pub failed_operations: u64,
    pub active_operations: u64,
}

/// Async operation system call handler
pub struct AsyncOpHandler {
    manager: Arc<AsyncManager>,
}

impl AsyncOpHandler {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(AsyncManager::new()),
        }
    }
    
    pub fn manager(&self) -> &Arc<AsyncManager> {
        &self.manager
    }
}

impl Default for AsyncOpHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallHandler for AsyncOpHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ASYNC_OP
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            return Err(Error::InvalidArgument("Insufficient arguments for async operation".to_string()));
        }

        let op_type = match args[0] {
            0 => AsyncOperationType::Read,
            1 => AsyncOperationType::Write,
            2 => AsyncOperationType::Connect,
            3 => AsyncOperationType::Accept,
            4 => AsyncOperationType::Send,
            5 => AsyncOperationType::Recv,
            6 => AsyncOperationType::Fsync,
            7 => AsyncOperationType::Sync,
            8 => AsyncOperationType::Ioctl,
            9 => AsyncOperationType::Poll,
            #[cfg(feature = "advanced_syscalls")]
            custom => AsyncOperationType::Custom(custom as u32),
            _ => AsyncOperationType::Read,
        };
        
        let priority = AsyncPriority::from_u8(args[1] as u8);
        let timeout = if args[2] != 0 { Some(args[2] as u32) } else { None };

        let context_id = self.manager.create_context(op_type, priority, timeout)?;
        
        Ok(context_id as isize)
    }
    
    fn name(&self) -> &str {
        "async_op"
    }
}

/// Async wait system call handler
pub struct AsyncWaitHandler {
    manager: Arc<AsyncManager>,
}

impl AsyncWaitHandler {
    pub fn new(manager: Arc<AsyncManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for AsyncWaitHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ASYNC_WAIT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.is_empty() {
            return Err(Error::InvalidArgument("Insufficient arguments for async wait".to_string()));
        }

        let context_id = args[0] as u64;
        let timeout_ms = if args.len() > 1 { Some(args[1] as u32) } else { None };
        
        let context = self.manager.get_context(context_id)
            .ok_or_else(|| Error::NotFound("Async context not found".to_string()))?;
        
        match context.status {
            AsyncStatus::Completed => Ok(context.result.unwrap_or(0)),
        AsyncStatus::Failed => {
            let error_code = context.error.map(|e| e.to_i32()).unwrap_or(AsyncError::Invalid as i32);
            Err(Error::InvalidArgument(format!("Operation failed with error {}", error_code)))
        },
        AsyncStatus::Cancelled => Err(Error::InvalidArgument("Operation cancelled".to_string())),
        AsyncStatus::TimedOut => Err(nos_api::Error::Timeout),
        AsyncStatus::Pending | AsyncStatus::Queued | AsyncStatus::InProgress => {
            if timeout_ms.is_some() && timeout_ms.unwrap() == 0 {
                Err(nos_api::Error::Timeout)
            } else {
                Err(Error::InvalidState("Operation still in progress".to_string()))
            }
        },
        }
    }
    
    fn name(&self) -> &str {
        "async_wait"
    }
}

/// Async cancel system call handler
pub struct AsyncCancelHandler {
    manager: Arc<AsyncManager>,
}

impl AsyncCancelHandler {
    pub fn new(manager: Arc<AsyncManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for AsyncCancelHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ASYNC_CANCEL
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.is_empty() {
            return Err(Error::InvalidArgument("Insufficient arguments for async cancel".to_string()));
        }

        let context_id = args[0] as u64;
        self.manager.cancel_operation(context_id)?;
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "async_cancel"
    }
}

/// Async stats system call handler
pub struct AsyncStatsHandler {
    manager: Arc<AsyncManager>,
}

impl AsyncStatsHandler {
    pub fn new(manager: Arc<AsyncManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for AsyncStatsHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ASYNC_STATS
    }
    
    fn execute(&self, _args: &[usize]) -> Result<isize> {
        let stats = self.manager.get_stats();
        sys_trace_with_args!("Async stats: total={}, completed={}, failed={}, active={}", 
                   stats.total_operations, stats.completed_operations, stats.failed_operations, stats.active_operations);
        Ok(stats.completed_operations as isize)
    }
    
    fn name(&self) -> &str {
        "async_stats"
    }
}

/// Register async operation system calls
pub fn register_syscalls(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    let async_handler = AsyncOpHandler::new();
    let manager = async_handler.manager().clone();
    
    dispatcher.register_handler(1004, Box::new(async_handler));
    dispatcher.register_handler(1005, Box::new(AsyncWaitHandler::new(manager.clone())));
    dispatcher.register_handler(1010, Box::new(AsyncCancelHandler::new(manager.clone())));
    dispatcher.register_handler(1011, Box::new(AsyncStatsHandler::new(manager)));
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_error() {
        assert_eq!(AsyncError::from_i32(0), AsyncError::Success);
        assert_eq!(AsyncError::from_i32(-3), AsyncError::Timeout);
        assert_eq!(AsyncError::from_i32(-4), AsyncError::Cancelled);
        assert_eq!(AsyncError::Timeout.to_i32(), -3);
    }

    #[test]
    fn test_async_priority() {
        assert_eq!(AsyncPriority::from_u8(0), AsyncPriority::Low);
        assert_eq!(AsyncPriority::from_u8(1), AsyncPriority::Normal);
        assert_eq!(AsyncPriority::from_u8(3), AsyncPriority::Urgent);
        assert_eq!(AsyncPriority::from_u8(5), AsyncPriority::Normal);
    }

    #[test]
    fn test_async_manager() {
        let manager = AsyncManager::new();
        
        let id = manager.create_context(AsyncOperationType::Read, AsyncPriority::Normal, Some(1000)).unwrap();
        assert!(id > 0);
        
        let context = manager.get_context(id).unwrap();
        assert_eq!(context.id, id);
        assert_eq!(context.operation_type, AsyncOperationType::Read);
        assert_eq!(context.priority, AsyncPriority::Normal);
        
        manager.complete_operation(id, 42).unwrap();
        
        let context = manager.get_context(id).unwrap();
        assert_eq!(context.status, AsyncStatus::Completed);
        assert_eq!(context.result, Some(42));
        
        let stats = manager.get_stats();
        assert_eq!(stats.completed_operations, 1);
    }
    
    #[test]
    fn test_async_handler() {
        let handler = AsyncOpHandler::new();
        assert_eq!(handler.name(), "async_op");
        
        let result = handler.execute(&[]);
        assert!(result.is_err());
        
        let result = handler.execute(&[0, 1, 0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_async_cancel() {
        let manager = AsyncManager::new();
        let handler = AsyncCancelHandler::new(Arc::new(manager.clone()));
        
        let id = manager.create_context(AsyncOperationType::Write, AsyncPriority::Normal, None).unwrap();
        
        let result = handler.execute(&[id as usize]);
        assert!(result.is_ok());
        
        let context = manager.get_context(id).unwrap();
        assert_eq!(context.status, AsyncStatus::Cancelled);
    }
}
