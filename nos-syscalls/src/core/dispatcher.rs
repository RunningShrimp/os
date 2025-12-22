//! System call dispatcher
//!
//! This module provides the core system call dispatch mechanism.

#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::format;

#[cfg(feature = "alloc")]
use nos_api::Error;
#[cfg(feature = "alloc")]
use spin::Mutex;
#[cfg(feature = "alloc")]
use super::traits::SyscallHandler;

/// System call dispatcher
#[cfg(feature = "alloc")]
pub struct SyscallDispatcher {
    /// Registered system call handlers
    handlers: BTreeMap<u32, Box<dyn SyscallHandler>>,
    /// System call statistics
    stats: Mutex<SyscallStats>,
}

#[cfg(feature = "alloc")]
impl SyscallDispatcher {
    /// Create a new system call dispatcher
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
            stats: Mutex::new(SyscallStats::default()),
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
                #[cfg(feature = "alloc")]
                return Error::NotFound(format!("System call {} not found", id));
                #[cfg(not(feature = "alloc"))]
                return Error::NotFound("System call not found".into());
            })?;
        
        // Execute the handler
        let result = handler.execute(args);
        
        // Update statistics
        let mut stats = self.stats.lock();
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
        self.stats.lock().clone()
    }
}



/// System call statistics
#[derive(Debug, Clone)]
#[cfg(feature = "alloc")]
pub struct SyscallStats {
    /// Total number of system calls
    pub total_calls: u64,
    /// Number of calls by type
    pub calls_by_type: BTreeMap<u32, u64>,
    /// Average execution time (microseconds)
    pub avg_execution_time: u64,
    /// Number of errors
    pub error_count: u64,
}

#[cfg(feature = "alloc")]
impl Default for SyscallStats {
    fn default() -> Self {
        Self {
            total_calls: 0,
            calls_by_type: BTreeMap::new(),
            avg_execution_time: 0,
            error_count: 0,
        }
    }
}

/// /// Global system call dispatcher
#[cfg(feature = "alloc")]
static mut GLOBAL_DISPATCHER: Option<SyscallDispatcher> = None;
static DISPATCHER_INIT: spin::Once = spin::Once::new();

/// Initialize the global system call dispatcher
#[cfg(feature = "alloc")]
pub fn init_dispatcher() -> nos_api::Result<()> {
    DISPATCHER_INIT.call_once(|| {
        unsafe {
            GLOBAL_DISPATCHER = Some(SyscallDispatcher::new());
        }
    });
    Ok(())
}

/// Get the global system call dispatcher
#[cfg(feature = "alloc")]
pub fn get_dispatcher() -> &'static SyscallDispatcher {
    // SAFETY: We've initialized the dispatcher in init_dispatcher
    // and we're only creating a shared reference
    unsafe {
        GLOBAL_DISPATCHER.as_ref().unwrap()
    }
}

/// Get mutable access to the global system call dispatcher
/// 
/// # Safety
/// 
/// This function is unsafe because it allows mutable access to a global static variable.
/// Callers must ensure that there are no concurrent accesses to the dispatcher while using
/// the mutable reference.
#[cfg(feature = "alloc")]
pub unsafe fn get_dispatcher_mut() -> &'static mut SyscallDispatcher {
    // SAFETY: The dispatcher must be initialized before calling this function
    // and callers must ensure no concurrent access
    unsafe { GLOBAL_DISPATCHER.as_mut().unwrap() }
}

/// Shutdown the global system call dispatcher
#[cfg(feature = "alloc")]
pub fn shutdown_dispatcher() -> nos_api::Result<()> {
    // With MaybeUninit, we can't easily reset it, so we'll just leave it as-is
    Ok(())
}

/// Register all system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers() -> nos_api::Result<()> {
    // SAFETY: We're the only ones modifying the dispatcher during initialization
    // and we've already initialized it in init_dispatcher
    let dispatcher = unsafe {
        get_dispatcher_mut()
    };
    
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
        #[cfg(feature = "alloc")]
        crate::advanced_mmap::register_syscalls(dispatcher)?;
        #[cfg(feature = "alloc")]
        crate::async_ops::register_syscalls(dispatcher)?;
        #[cfg(feature = "alloc")]
        crate::epoll::register_syscalls(dispatcher)?;
    }
    
    Ok(())
}

/// Get system call statistics
#[cfg(feature = "alloc")]
pub fn get_stats() -> super::super::SyscallStats {
    let dispatcher_stats = get_dispatcher().get_stats();
    super::super::SyscallStats {
        total_calls: dispatcher_stats.total_calls,
        calls_by_type: dispatcher_stats.calls_by_type,
        avg_execution_time: dispatcher_stats.avg_execution_time,
        error_count: dispatcher_stats.error_count,
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
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