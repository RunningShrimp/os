//! System Call Dispatch Layer
//!
//! This module implements the dispatch layer for system calls.
//! It provides routing, validation, and performance monitoring
//! for system call execution.
//!
//! # Architecture
//!
//! The dispatch layer consists of:
//! - Unified dispatcher (recommended, consolidates all features)
//! - Legacy dispatcher implementations (deprecated, will be removed)
//! - Fast path for common syscalls
//! - Performance monitoring
//! - Error handling and recovery
//!
//! # Design Principles
//!
//! - **Fast Path**: Optimize frequently called syscalls
//! - **Validation**: Ensure syscall arguments are valid
//! - **Monitoring**: Track performance and errors
//! - **Resilience**: Graceful error handling

// Unified dispatcher (recommended)
pub mod unified;

// Legacy dispatcher implementations (deprecated, will be removed)
pub mod dispatcher;
pub mod registry;
pub mod traits;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::subsystems::sync::Mutex;

use super::interface::{
    SyscallDispatcher, SyscallHandler, SyscallContext, SyscallError,
    SyscallResult, SyscallCategory, get_syscall_category,
};

// Re-export unified dispatcher as the default
pub use unified::{
    UnifiedSyscallDispatcher, UnifiedDispatcherConfig, DispatchStats,
    init_unified_dispatcher, get_unified_dispatcher, unified_batch_dispatch,
};

/// Dispatcher configuration
///
/// Configuration options for the system call dispatcher.
#[derive(Debug, Clone)]
pub struct DispatcherConfig {
    /// Enable fast path optimization
    pub enable_fast_path: bool,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Enable argument validation
    pub enable_validation: bool,
    /// Maximum number of cached handlers
    pub max_cache_size: usize,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            enable_fast_path: true,
            enable_monitoring: true,
            enable_validation: true,
            max_cache_size: 256,
        }
    }
}

/// Dispatch statistics
///
/// Statistics for system call dispatch performance.
#[derive(Debug, Default)]
pub struct DispatchStats {
    /// Total number of system calls dispatched
    pub total_dispatches: AtomicU64,
    /// Number of successful system calls
    pub successful_dispatches: AtomicU64,
    /// Number of failed system calls
    pub failed_dispatches: AtomicU64,
    /// Number of cache hits
    pub cache_hits: AtomicU64,
    /// Number of cache misses
    pub cache_misses: AtomicU64,
    /// Total time spent in system calls (nanoseconds)
    pub total_time_ns: AtomicU64,
}

impl DispatchStats {
    /// Create new dispatch statistics
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record a successful dispatch
    pub fn record_success(&self, time_ns: u64) {
        self.total_dispatches.fetch_add(1, Ordering::Relaxed);
        self.successful_dispatches.fetch_add(1, Ordering::Relaxed);
        self.total_time_ns.fetch_add(time_ns, Ordering::Relaxed);
    }
    
    /// Record a failed dispatch
    pub fn record_failure(&self) {
        self.total_dispatches.fetch_add(1, Ordering::Relaxed);
        self.failed_dispatches.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.total_dispatches.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let successful = self.successful_dispatches.load(Ordering::Relaxed);
        (successful as f64 / total as f64) * 100.0
    }
    
    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }
        (hits as f64 / total as f64) * 100.0
    }
    
    /// Get average dispatch time
    pub fn avg_time_ns(&self) -> f64 {
        let total = self.total_dispatches.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let time = self.total_time_ns.load(Ordering::Relaxed);
        time as f64 / total as f64
    }
}

/// Cached handler information
///
/// Information about a cached system call handler.
#[derive(Debug)]
pub struct CachedHandlerInfo {
    /// System call number
    pub syscall_number: u32,
    /// Handler implementation
    pub handler: Arc<dyn SyscallHandler>,
    /// Last access timestamp
    pub last_access: AtomicU64,
    /// Access count
    pub access_count: AtomicUsize,
}

impl CachedHandlerInfo {
    /// Create new cached handler info
    pub fn new(syscall_number: u32, handler: Arc<dyn SyscallHandler>) -> Self {
        Self {
            syscall_number,
            handler,
            last_access: AtomicU64::new(0),
            access_count: AtomicUsize::new(0),
        }
    }
    
    /// Record access to this handler
    pub fn record_access(&self) {
        // In a real implementation, this would get the current time
        // For now, we'll just use a placeholder
        self.last_access.store(0, Ordering::Relaxed);
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get access count
    pub fn get_access_count(&self) -> usize {
        self.access_count.load(Ordering::Relaxed)
    }
}

/// System call dispatcher implementation
///
/// Main implementation of the SyscallDispatcher trait.
pub struct SyscallDispatcherImpl {
    /// Dispatcher configuration
    config: DispatcherConfig,
    /// System call context
    context: Arc<dyn SyscallContext>,
    /// Handler cache
    handler_cache: Mutex<BTreeMap<u32, CachedHandlerInfo>>,
    /// Dispatch statistics
    stats: DispatchStats,
}

impl SyscallDispatcherImpl {
    /// Create a new system call dispatcher
    ///
    /// # Arguments
    /// * `config` - Dispatcher configuration
    /// * `context` - System call context
    ///
    /// # Returns
    /// * `Self` - New dispatcher instance
    pub fn new(config: DispatcherConfig, context: Arc<dyn SyscallContext>) -> Self {
        Self {
            config,
            context,
            handler_cache: Mutex::new(BTreeMap::new()),
            stats: DispatchStats::new(),
        }
    }
    
    /// Create a dispatcher with default configuration
    ///
    /// # Arguments
    /// * `context` - System call context
    ///
    /// # Returns
    /// * `Self` - New dispatcher instance
    pub fn with_default_config(context: Arc<dyn SyscallContext>) -> Self {
        Self::new(DispatcherConfig::default(), context)
    }
    
    /// Register a system call handler
    ///
    /// # Arguments
    /// * `handler` - Handler to register
    ///
    /// # Returns
    /// * `Result<(), SyscallError>` - Registration result
    pub fn register_handler(&self, handler: Arc<dyn SyscallHandler>) -> Result<(), SyscallError> {
        let syscall_number = handler.get_syscall_number();
        
        // Validate handler
        if self.config.enable_validation {
            self.validate_handler(&handler)?;
        }
        
        // Add to cache
        let mut cache = self.handler_cache.lock();
        let cache_size = cache.len();
        
        if cache_size >= self.config.max_cache_size {
            // Evict least recently used handler
            if let Some((&num, _)) = cache.iter()
                .min_by_key(|(_, info)| info.last_access.load(Ordering::Relaxed)) {
                cache.remove(&num);
            }
        }
        
        let info = CachedHandlerInfo::new(syscall_number, Arc::clone(&handler));
        cache.insert(syscall_number, info);
        
        Ok(())
    }
    
    /// Unregister a system call handler
    ///
    /// # Arguments
    /// * `syscall_number` - System call number to unregister
    ///
    /// # Returns
    /// * `Result<(), SyscallError>` - Unregistration result
    pub fn unregister_handler(&self, syscall_number: u32) -> Result<(), SyscallError> {
        let mut cache = self.handler_cache.lock();
        if cache.remove(&syscall_number).is_some() {
            Ok(())
        } else {
            Err(SyscallError::NotFound)
        }
    }
    
    /// Validate a handler
    ///
    /// # Arguments
    /// * `handler` - Handler to validate
    ///
    /// # Returns
    /// * `Result<(), SyscallError>` - Validation result
    fn validate_handler(&self, handler: &Arc<dyn SyscallHandler>) -> Result<(), SyscallError> {
        let syscall_number = handler.get_syscall_number();
        let name = handler.get_name();
        
        // Check if syscall number is valid
        if get_syscall_category(syscall_number).is_none() {
            return Err(SyscallError::InvalidSyscall(syscall_number));
        }
        
        // Check if name is not empty
        if name.is_empty() {
            return Err(SyscallError::InvalidArguments);
        }
        
        Ok(())
    }
    
    /// Handle fast path system calls
    ///
    /// # Arguments
    /// * `syscall_number` - System call number
    /// * `args` - System call arguments
    ///
    /// # Returns
    /// * `Option<SyscallResult>` - Result if fast path handled
    fn handle_fast_path(&self, syscall_number: u32, args: &[u64]) -> Option<SyscallResult> {
        if !self.config.enable_fast_path {
            return None;
        }
        
        // Use fast-path registry for hot syscalls
        use crate::subsystems::syscalls::fast_path::hot_syscalls;
        hot_syscalls::init_fast_path_registry();
        
        if let Some(result) = hot_syscalls::dispatch_fast_path(syscall_number, args) {
            self.stats.record_success(10); // Fast path is very fast
            return Some(result);
        }
        
        // Fallback to inline fast paths for specific syscalls
        match syscall_number {
            // Fast path for getpid (fallback if not in registry)
            0x1004 => {
                self.stats.record_success(10);
                Some(Ok(self.context.get_pid() as u64))
            }
            _ => None,
        }
    }
    
    /// Get cached handler
    ///
    /// # Arguments
    /// * `syscall_number` - System call number
    ///
    /// # Returns
    /// * `Option<Arc<dyn SyscallHandler>>` - Cached handler if found
    fn get_cached_handler(&self, syscall_number: u32) -> Option<Arc<dyn SyscallHandler>> {
        let cache = self.handler_cache.lock();
        if let Some(info) = cache.get(&syscall_number) {
            self.stats.record_cache_hit();
            info.record_access();
            Some(Arc::clone(&info.handler))
        } else {
            self.stats.record_cache_miss();
            None
        }
    }
}

impl SyscallDispatcher for SyscallDispatcherImpl {
    fn dispatch(&self, num: u32, args: &[u64]) -> SyscallResult {
        let start_time = if self.config.enable_monitoring {
            Some(0) // Placeholder for current time
        } else {
            None
        };
        
        // Try fast path first
        if let Some(result) = self.handle_fast_path(num, args) {
            return result;
        }
        
        // Get handler from cache
        let handler = self.get_cached_handler(num)
            .ok_or(SyscallError::InvalidSyscall(num))?;
        
        // Validate arguments if enabled
        if self.config.enable_validation {
            // Add argument validation here
        }
        
        // Call handler
        let result = handler.handle(args);
        
        // Record statistics
        if let Some(start_time) = start_time {
            let elapsed = 0; // Placeholder for elapsed time
            match &result {
                Ok(_) => self.stats.record_success(elapsed),
                Err(_) => self.stats.record_failure(),
            }
        }
        
        result
    }
    
    fn is_supported(&self, num: u32) -> bool {
        // Check fast path
        if self.config.enable_fast_path {
            match num {
                0x1004 => return true, // getpid
                _ => {}
            }
        }
        
        // Check cache
        let cache = self.handler_cache.lock();
        cache.contains_key(&num)
    }
    
    fn get_name(&self, num: u32) -> Option<&'static str> {
        // Check fast path
        if self.config.enable_fast_path {
            match num {
                0x1004 => return Some("getpid"),
                _ => {}
            }
        }
        
        // Check cache
        let cache = self.handler_cache.lock();
        if let Some(info) = cache.get(&num) {
            Some(info.handler.get_name())
        } else {
            None
        }
    }
    
    fn get_context(&self) -> &dyn SyscallContext {
        self.context.as_ref()
    }
}

/// Get dispatch statistics
///
/// # Arguments
/// * `dispatcher` - Dispatcher to get statistics from
///
/// # Returns
/// * `DispatchStats` - Dispatch statistics
pub fn get_dispatch_stats(dispatcher: &dyn SyscallDispatcher) -> DispatchStats {
    // This is a workaround since we can't access stats directly from trait
    // In a real implementation, we would add a get_stats method to the trait
    DispatchStats::default()
}