//! System call dispatcher
//!
//! This module provides the core system call dispatch mechanism.

use {
    alloc::{
        boxed::Box,
        collections::BTreeMap,
        format,
    },
    nos_api::Error,
    crate::{SyscallHandler, SyscallStats},
};

/// System call dispatcher
pub struct SyscallDispatcher {
    /// Registered system call handlers
    handlers: BTreeMap<u32, Box<dyn SyscallHandler>>,
    /// System call statistics
    stats: spin::RwLock<SyscallStats>,
}

impl SyscallDispatcher {
    /// Create a new system call dispatcher
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
            stats: spin::RwLock::new(SyscallStats::default()),
        }
    }

    /// Register a system call handler
    pub fn register_handler(&mut self, id: u32, handler: Box<dyn SyscallHandler>) {
        self.handlers.insert(id, handler);
    }

    /// Get a system call handler
    pub fn get_handler(&self, id: u32) -> Option<&Box<dyn SyscallHandler>> {
        self.handlers.get(&id)
    }

    /// Dispatch a system call
    pub fn dispatch(&self, id: u32, args: &[usize]) -> nos_api::Result<isize> {
        let start_time = crate::time::get_timestamp();
        
        // Get the handler
        let handler = self.handlers.get(&id)
            .ok_or_else(|| {
                Error::NotFound(format!("System call {} not found", id))
            })?;
        
        // Execute the handler
        let result = handler.execute(args);
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_calls += 1;
        *stats.calls_by_type.entry(id).or_insert(0) += 1;
        
        let end_time = crate::time::get_timestamp();
        let execution_time = end_time - start_time;
        stats.avg_execution_time = (stats.avg_execution_time * (stats.total_calls - 1) + execution_time) / stats.total_calls;
        
        if result.is_err() {
            stats.error_count += 1;
        }
        
        result
    }

    /// Get system call statistics
    pub fn get_stats(&self) -> SyscallStats {
        self.stats.read().clone()
    }
}





/// Global system call dispatcher
static GLOBAL_DISPATCHER: spin::Mutex<Option<SyscallDispatcher>> = spin::Mutex::new(None);

/// Initialize the global system call dispatcher
pub fn init_dispatcher() -> nos_api::Result<()> {
    let mut dispatcher = GLOBAL_DISPATCHER.lock();
    if dispatcher.is_none() {
        *dispatcher = Some(SyscallDispatcher::new());
    }
    Ok(())
}

/// Get the global system call dispatcher
pub fn get_dispatcher() -> spin::MutexGuard<'static, Option<SyscallDispatcher>> {
    // Ensure dispatcher is initialized before accessing
    init_dispatcher().unwrap();
    GLOBAL_DISPATCHER.lock()
}

/// Get mutable access to the global system call dispatcher
pub fn get_dispatcher_mut() -> spin::MutexGuard<'static, Option<SyscallDispatcher>> {
    // Ensure dispatcher is initialized before accessing
    init_dispatcher().unwrap();
    GLOBAL_DISPATCHER.lock()
}

/// Shutdown the global system call dispatcher
pub fn shutdown_dispatcher() -> nos_api::Result<()> {
    // With MaybeUninit, we can't easily reset it, so we'll just leave it as-is
    Ok(())
}

/// Register all system call handlers
pub fn register_handlers() -> nos_api::Result<()> {
    let mut dispatcher = get_dispatcher_mut();
    let dispatcher = dispatcher.as_mut().unwrap();
    
    // Register core system calls
    crate::fs::register_handlers(dispatcher)?;
    crate::process::register_handlers(dispatcher)?;
    crate::network::register_handlers(dispatcher)?;
    crate::zero_copy_network_impl::register_handlers(dispatcher)?;
    crate::ipc::register_handlers(dispatcher)?;
    crate::signal::register_handlers(dispatcher)?;
    crate::memory::register_handlers(dispatcher)?;
    crate::time::register_handlers(dispatcher)?;
    
    // Register optimized syscall path
    crate::optimized_syscall_path::register_handlers(dispatcher)?;
    
    // Register adaptive scheduler
    crate::adaptive_scheduler::register_handlers(dispatcher)?;
    
    // Register performance monitoring
    crate::performance_monitor::register_handlers(dispatcher)?;
    
    // Register advanced system calls
    #[cfg(feature = "advanced_syscalls")]
    {
        crate::advanced_mmap::register_syscalls(dispatcher)?;
        crate::async_ops::register_syscalls(dispatcher)?;
        crate::epoll::register_syscalls(dispatcher)?;
    }
    
    Ok(())
}

/// Get system call statistics
pub fn get_stats() -> super::super::SyscallStats {
    let dispatcher = get_dispatcher();
    let dispatcher_stats = dispatcher.as_ref().unwrap().get_stats();
    super::super::SyscallStats {
        total_calls: dispatcher_stats.total_calls,
        calls_by_type: dispatcher_stats.calls_by_type,
        avg_execution_time: dispatcher_stats.avg_execution_time,
        error_count: dispatcher_stats.error_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: &'static str,
        result: isize,
    }

    impl SyscallHandler for TestHandler {
        fn execute(&self, _args: &[usize]) -> nos_api::Result<isize> {
            Ok(self.result)
        }
        
        fn name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn test_dispatcher() {
        let mut dispatcher = SyscallDispatcher::new();
        
        // Register a test handler
        let handler = TestHandler {
            name: "test",
            result: 42,
        };
        dispatcher.register_handler(100, Box::new(handler));
        
        // Dispatch the system call
        let result = dispatcher.dispatch(100, &[]);
        assert_eq!(result.unwrap(), 42);
        
        // Check statistics
        let stats = dispatcher.get_stats();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.calls_by_type.get(&100), Some(&1));
        assert_eq!(stats.error_count, 0);
    }
}