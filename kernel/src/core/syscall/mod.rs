//! Core System Call Implementation
//!
//! This module contains the core implementation of system call handling.
//! It provides the actual implementation of the interfaces defined in the API layer.

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;
use crate::api::syscall::{SyscallDispatcher, SyscallHandler, SyscallError, SyscallResult, get_syscall_category};
use crate::api::context::get_kernel_context;
use crate::api::error::KernelError;

/// Core system call dispatcher
///
/// This struct implements the SyscallDispatcher trait and provides
/// the core functionality for dispatching system calls to their handlers.
pub struct CoreSyscallDispatcher {
    /// System call handlers
    handlers: Mutex<BTreeMap<u32, Box<dyn SyscallHandler>>>,
    /// Fast path handlers for frequently used syscalls
    fast_path_handlers: Mutex<BTreeMap<u32, fn(&[u64]) -> SyscallResult>>,
}

impl CoreSyscallDispatcher {
    /// Create a new core system call dispatcher
    ///
    /// # Returns
    /// * `CoreSyscallDispatcher` - New dispatcher
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(BTreeMap::new()),
            fast_path_handlers: Mutex::new(BTreeMap::new()),
        }
    }

    /// Register a system call handler
    ///
    /// # Arguments
    /// * `handler` - System call handler
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(KernelError)` - Registration error
    pub fn register_handler(&self, handler: Box<dyn SyscallHandler>) -> Result<(), KernelError> {
        let syscall_num = handler.get_syscall_number();
        let mut handlers = self.handlers.lock();
        
        if handlers.contains_key(&syscall_num) {
            return Err(KernelError::AlreadyExists);
        }
        
        handlers.insert(syscall_num, handler);
        Ok(())
    }

    /// Register a fast path handler
    ///
    /// # Arguments
    /// * `syscall_num` - System call number
    /// * `handler` - Fast path handler function
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(KernelError)` - Registration error
    pub fn register_fast_path_handler(&self, syscall_num: u32, handler: fn(&[u64]) -> SyscallResult) -> Result<(), KernelError> {
        let mut fast_handlers = self.fast_path_handlers.lock();
        
        if fast_handlers.contains_key(&syscall_num) {
            return Err(KernelError::AlreadyExists);
        }
        
        fast_handlers.insert(syscall_num, handler);
        Ok(())
    }

    /// Unregister a system call handler
    ///
    /// # Arguments
    /// * `syscall_num` - System call number
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(KernelError)` - Unregistration error
    pub fn unregister_handler(&self, syscall_num: u32) -> Result<(), KernelError> {
        let mut handlers = self.handlers.lock();
        
        if !handlers.contains_key(&syscall_num) {
            return Err(KernelError::NotFound);
        }
        
        handlers.remove(&syscall_num);
        Ok(())
    }

    /// Unregister a fast path handler
    ///
    /// # Arguments
    /// * `syscall_num` - System call number
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(KernelError)` - Unregistration error
    pub fn unregister_fast_path_handler(&self, syscall_num: u32) -> Result<(), KernelError> {
        let mut fast_handlers = self.fast_path_handlers.lock();
        
        if !fast_handlers.contains_key(&syscall_num) {
            return Err(KernelError::NotFound);
        }
        
        fast_handlers.remove(&syscall_num);
        Ok(())
    }

    /// Get all registered system call numbers
    ///
    /// # Returns
    /// * `Vec<u32>` - System call numbers
    pub fn get_registered_syscalls(&self) -> Vec<u32> {
        let handlers = self.handlers.lock();
        handlers.keys().copied().collect()
    }

    /// Get all registered fast path system call numbers
    ///
    /// # Returns
    /// * `Vec<u32>` - Fast path system call numbers
    pub fn get_registered_fast_path_syscalls(&self) -> Vec<u32> {
        let fast_handlers = self.fast_path_handlers.lock();
        fast_handlers.keys().copied().collect()
    }
}

impl Default for CoreSyscallDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallDispatcher for CoreSyscallDispatcher {
    fn dispatch(&self, num: u32, args: &[u64]) -> SyscallResult {
        // First try fast path handlers
        {
            let fast_handlers = self.fast_path_handlers.lock();
            if let Some(handler) = fast_handlers.get(&num) {
                return handler(args);
            }
        }
        
        // Then try regular handlers
        {
            let handlers = self.handlers.lock();
            if let Some(handler) = handlers.get(&num) {
                return handler.handle(args);
            }
        }
        
        // If no handler is found, return an error
        Err(SyscallError::InvalidSyscall(num))
    }

    fn is_supported(&self, num: u32) -> bool {
        // Check fast path handlers
        {
            let fast_handlers = self.fast_path_handlers.lock();
            if fast_handlers.contains_key(&num) {
                return true;
            }
        }
        
        // Check regular handlers
        {
            let handlers = self.handlers.lock();
            if handlers.contains_key(&num) {
                return true;
            }
        }
        
        false
    }

    fn get_name(&self, num: u32) -> Option<&'static str> {
        // Check fast path handlers
        {
            let fast_handlers = self.fast_path_handlers.lock();
            if let Some(handler) = fast_handlers.get(&num) {
                // For fast path handlers, we need to look up the name
                return self.get_syscall_name_by_number(num);
            }
        }
        
        // Check regular handlers
        {
            let handlers = self.handlers.lock();
            if let Some(handler) = handlers.get(&num) {
                return Some(handler.get_name());
            }
        }
        
        None
    }
}

impl CoreSyscallDispatcher {
    /// Get system call name by number
    ///
    /// This is a helper method to get the name of a system call
    /// when only the number is available (e.g., for fast path handlers).
    ///
    /// # Arguments
    /// * `num` - System call number
    ///
    /// # Returns
    /// * `Option<&'static str>` - System call name if known
    fn get_syscall_name_by_number(&self, num: u32) -> Option<&'static str> {
        // This is a simplified implementation. In a real system,
        // this would be a comprehensive lookup table.
        match num {
            0x1000 => Some("getpid"),
            0x1001 => Some("fork"),
            0x1002 => Some("execve"),
            0x1003 => Some("exit"),
            0x1004 => Some("wait4"),
            0x1005 => Some("kill"),
            0x2000 => Some("read"),
            0x2001 => Some("write"),
            0x2002 => Some("open"),
            0x2003 => Some("close"),
            0x2004 => Some("stat"),
            0x3000 => Some("mmap"),
            0x3001 => Some("munmap"),
            0x3002 => Some("brk"),
            0x3003 => Some("mprotect"),
            0x4000 => Some("socket"),
            0x4001 => Some("bind"),
            0x4002 => Some("connect"),
            0x4003 => Some("listen"),
            0x4004 => Some("accept"),
            _ => None,
        }
    }
}

/// System call handler registry
///
/// This struct provides a registry for system call handlers.
/// It is used to register and manage system call handlers.
pub struct SyscallHandlerRegistry {
    /// System call handlers
    handlers: Vec<Box<dyn SyscallHandler>>,
}

impl SyscallHandlerRegistry {
    /// Create a new system call handler registry
    ///
    /// # Returns
    /// * `SyscallHandlerRegistry` - New registry
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register a system call handler
    ///
    /// # Arguments
    /// * `handler` - System call handler
    pub fn register(&mut self, handler: Box<dyn SyscallHandler>) {
        self.handlers.push(handler);
    }

    /// Register all handlers to a dispatcher
    ///
    /// # Arguments
    /// * `dispatcher` - System call dispatcher
    ///
    /// # Returns
    /// * `Result<(), KernelError>` - Registration result
    pub fn register_all(&self, dispatcher: &CoreSyscallDispatcher) -> Result<(), KernelError> {
        for handler in &self.handlers {
            // Note: In a real implementation, we would need to clone the handler
            // or use some other mechanism to register it multiple times.
            // For now, we'll just return an error.
            return Err(KernelError::NotSupported);
        }
        Ok(())
    }
}

impl Default for SyscallHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}