//! Unified System Call Dispatcher
//!
//! This module provides a unified, high-performance system call dispatcher that
//! consolidates the best features from multiple dispatcher implementations:
//! - Fast-path optimization for common syscalls (256-entry direct jump table)
//! - Per-CPU caching to reduce lock contention
//! - Handler registration mechanism
//! - Performance monitoring and statistics
//! - Batch syscall processing
//! - Adaptive optimization based on call frequency

use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicPtr, Ordering};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::subsystems::sync::Mutex;
use crate::subsystems::sync::rcu;
use crate::cpu;

use crate::subsystems::syscalls::interface::{
    SyscallDispatcher, SyscallHandler, SyscallContext, SyscallArgs, SyscallNumber,
    SyscallResult, InterfaceSyscallError, SyscallStats
};

/// Sendable wrapper for raw pointers to boxed types
///
/// # Safety
/// This wrapper is safe to send across threads because T is Send + Sync
struct SendableBoxPtr<T>(*mut T);

// SAFETY: SendableBoxPtr wraps a raw pointer to a Send type
// It's only used in RCU context with proper synchronization
unsafe impl<T: Send> Send for SendableBoxPtr<T> {}

impl<T> SendableBoxPtr<T> {
    /// Consume this wrapper and drop the boxed value
    ///
    /// # Safety
    /// This must be called exactly once per SendableBoxPtr instance
    unsafe fn drop_box(self) {
        let Self(ptr) = self;
        unsafe {
            drop(Box::from_raw(ptr));
        }
        core::mem::forget(self); // Prevent double-drop
    }
}

/// Batch system call result
#[derive(Debug)]
pub struct SyscallBatchResult {
    /// Individual results for each syscall
    pub results: Vec<Result<u64, InterfaceSyscallError>>,
    /// Total execution time in nanoseconds
    pub total_time_ns: u64,
}

impl SyscallBatchResult {
    /// Create a new batch result
    pub fn new(results: Vec<Result<u64, InterfaceSyscallError>>, total_time_ns: u64) -> Self {
        Self {
            results,
            total_time_ns,
        }
    }

    /// Get the number of results
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Get the number of successful syscalls
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_ok()).count()
    }

    /// Get the number of failed syscalls
    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_err()).count()
    }
}

/// Maximum number of CPUs supported
const MAX_CPUS: usize = 256;

/// Maximum number of fast-path syscalls
const MAX_FAST_PATH_SYSCALLS: usize = 256;

/// Fast-path handler function type
pub type FastPathHandler = fn(u32, &[u64]) -> Result<u64, InterfaceSyscallError>;

/// Per-CPU syscall cache
#[derive(Debug)]
struct PerCpuCache {
    /// Most recent syscalls (for pattern detection)
    recent_syscalls: Vec<u32>,
    /// Syscall frequency table
    frequency_table: BTreeMap<u32, u64>,
    /// Cache hit count
    cache_hits: AtomicUsize,
    /// Cache miss count
    cache_misses: AtomicUsize,
}

impl PerCpuCache {
    /// Create a new per-CPU cache
    fn new() -> Self {
        Self {
            recent_syscalls: Vec::with_capacity(16),
            frequency_table: BTreeMap::new(),
            cache_hits: AtomicUsize::new(0usize),
            cache_misses: AtomicUsize::new(0usize),
        }
    }

    /// Add a syscall to the recent list and update frequency
    fn record_syscall(&mut self, syscall_num: u32) {
        // Add to recent list
        self.recent_syscalls.push(syscall_num);
        if self.recent_syscalls.len() > 16 {
            self.recent_syscalls.remove(0);
        }
        
        // Update frequency table
        *self.frequency_table.entry(syscall_num).or_insert(0) += 1;
    }

    /// Record a cache hit
    fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    fn record_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the most frequent syscalls from the frequency table
    fn get_most_frequent(&self, limit: usize) -> Vec<u32> {
        let mut entries: Vec<(u32, u64)> = self.frequency_table
            .iter()
            .map(|(&syscall, &count)| (syscall, count))
            .collect();

        // Sort by frequency (descending)
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        // Take the top `limit` syscalls
        entries.into_iter()
            .take(limit)
            .map(|(syscall, _)| syscall)
            .collect()
    }
}

/// Dispatch statistics
#[derive(Debug, Default)]
pub struct DispatchStats {
    /// Total number of dispatches
    pub total_dispatches: AtomicU64,
    /// Number of successful dispatches
    pub successful_dispatches: AtomicU64,
    /// Number of failed dispatches
    pub failed_dispatches: AtomicU64,
    /// Number of fast-path dispatches
    pub fast_path_dispatches: AtomicU64,
    /// Number of regular dispatches
    pub regular_dispatches: AtomicU64,
    /// Total execution time (nanoseconds)
    pub total_time_ns: AtomicU64,
}

impl DispatchStats {
    fn record_dispatch(&self, success: bool, fast_path: bool, time_ns: u64) {
        self.total_dispatches.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_dispatches.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_dispatches.fetch_add(1, Ordering::Relaxed);
        }
        if fast_path {
            self.fast_path_dispatches.fetch_add(1, Ordering::Relaxed);
        } else {
            self.regular_dispatches.fetch_add(1, Ordering::Relaxed);
        }
        self.total_time_ns.fetch_add(time_ns, Ordering::Relaxed);
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
    
    /// Get fast-path hit rate
    pub fn fast_path_rate(&self) -> f64 {
        let total = self.total_dispatches.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let fast_path = self.fast_path_dispatches.load(Ordering::Relaxed);
        (fast_path as f64 / total as f64) * 100.0
    }
    
    /// Get average execution time
    pub fn avg_time_ns(&self) -> f64 {
        let total = self.total_dispatches.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let time = self.total_time_ns.load(Ordering::Relaxed);
        time as f64 / total as f64
    }
}

/// Unified system call dispatcher configuration
#[derive(Debug, Clone)]
pub struct UnifiedDispatcherConfig {
    /// Enable fast-path optimization
    pub enable_fast_path: bool,
    /// Enable per-CPU caching
    pub enable_per_cpu_cache: bool,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Enable adaptive optimization
    pub enable_adaptive_optimization: bool,
    /// Fast-path update interval (number of syscalls)
    pub fast_path_update_interval: usize,
}

impl Default for UnifiedDispatcherConfig {
    fn default() -> Self {
        Self {
            enable_fast_path: true,
            enable_per_cpu_cache: true,
            enable_monitoring: true,
            enable_adaptive_optimization: true,
            fast_path_update_interval: 1000,
        }
    }
}

/// Fast-path handlers array (RCU-protected)
type FastPathArray = [Option<FastPathHandler>; MAX_FAST_PATH_SYSCALLS];

/// Handlers map (RCU-protected)
type HandlersMap = BTreeMap<u32, Arc<dyn SyscallHandler>>;

/// Unified system call dispatcher
pub struct UnifiedSyscallDispatcher {
    /// Fast-path handlers: RCU-protected array for lock-free reads
    /// Uses AtomicPtr to atomically update the entire array
    fast_path: AtomicPtr<FastPathArray>,
    /// Regular handlers: RCU-protected map for lock-free reads
    /// Uses AtomicPtr to atomically update the entire map
    handlers: AtomicPtr<HandlersMap>,
    /// Write lock for updating fast_path (only needed during registration)
    fast_path_write_lock: Mutex<()>,
    /// Write lock for updating handlers (only needed during registration)
    handlers_write_lock: Mutex<()>,
    /// Per-CPU caches to reduce lock contention
    per_cpu_caches: [Mutex<PerCpuCache>; MAX_CPUS],
    /// Dispatch statistics
    stats: Arc<DispatchStats>,
    /// Configuration
    config: UnifiedDispatcherConfig,
    /// Syscall count since last fast-path update
    syscall_count_since_update: AtomicUsize,
    /// System call context
    context: Arc<dyn SyscallContext>,
}

impl UnifiedSyscallDispatcher {
    /// Create a new unified dispatcher
    pub fn new(config: UnifiedDispatcherConfig) -> Self {
        Self::with_context(config, Arc::new(DefaultContext))
    }
    
    /// Create a new unified dispatcher with a context
    pub fn with_context(config: UnifiedDispatcherConfig, context: Arc<dyn SyscallContext>) -> Self {
        // Initialize per-CPU caches
        let per_cpu_caches = [(); MAX_CPUS].map(|_| Mutex::new(PerCpuCache::new()));

        // Initialize fast-path array (RCU-protected)
        let fast_path_array: Box<FastPathArray> = Box::new([None; MAX_FAST_PATH_SYSCALLS]);

        // Initialize handlers map (RCU-protected)
        let handlers_map: Box<HandlersMap> = Box::new(BTreeMap::new());

        // Store them as raw pointers for AtomicPtr
        let fast_path_ptr = Box::into_raw(fast_path_array);
        let handlers_ptr = Box::into_raw(handlers_map);

        Self {
            fast_path: AtomicPtr::new(fast_path_ptr),
            handlers: AtomicPtr::new(handlers_ptr),
            fast_path_write_lock: Mutex::new(()),
            handlers_write_lock: Mutex::new(()),
            per_cpu_caches,
            stats: Arc::new(DispatchStats::default()),
            config,
            syscall_count_since_update: AtomicUsize::new(0),
            context,
        }
    }
    
    /// Get fast-path array (lock-free read)
    #[inline]
    unsafe fn get_fast_path(&self) -> &FastPathArray {
        // Load atomic pointer (no lock needed for reads)
        let outer_ptr = self.fast_path.load(Ordering::Acquire);
        unsafe { &*outer_ptr }
    }

    /// Get handlers map (lock-free read)
    #[inline]
    unsafe fn get_handlers(&self) -> &HandlersMap {
        // Load atomic pointer (no lock needed for reads)
        let outer_ptr = self.handlers.load(Ordering::Acquire);
        unsafe { &*outer_ptr }
    }

    /// Register a system call handler (RCU update)
    pub fn register_handler(&self, syscall_num: u32, handler: Arc<dyn SyscallHandler>) -> Result<(), InterfaceSyscallError> {
        // Acquire write lock (only needed during updates)
        let _guard = self.handlers_write_lock.lock();

        // Load current map
        let old_outer_ptr = self.handlers.load(Ordering::Acquire);
        let old_map: &HandlersMap = unsafe { &*old_outer_ptr };

        // Create new map with updated handler
        let mut new_map = old_map.clone();
        new_map.insert(syscall_num, handler);

        // Allocate new map
        let new_map_box = Box::new(new_map);
        let new_outer_ptr = Box::into_raw(new_map_box);

        // Atomically update pointer (RCU-style)
        let prev_outer_ptr = self.handlers.swap(new_outer_ptr, Ordering::Release);

        // Memory barrier to ensure all readers see the new pointer
        core::sync::atomic::fence(Ordering::SeqCst);

        // Schedule old map for deletion after grace period
        let sendable = SendableBoxPtr(prev_outer_ptr);
        unsafe {
            rcu::call_rcu(Box::new(move || {
                sendable.drop_box();
            }));
        }

        Ok(())
    }

    /// Register a fast-path handler (RCU update)
    pub fn register_fast_path(&self, syscall_num: u32, handler: FastPathHandler) -> Result<(), InterfaceSyscallError> {
        if syscall_num as usize >= MAX_FAST_PATH_SYSCALLS {
            return Err(InterfaceSyscallError::InvalidArgument);
        }

        // Acquire write lock (only needed during updates)
        let _guard = self.fast_path_write_lock.lock();

        // Load current array
        let old_outer_ptr = self.fast_path.load(Ordering::Acquire);
        let old_array: &FastPathArray = unsafe { &*old_outer_ptr };

        // Create new array with updated handler
        let mut new_array = old_array.clone();
        new_array[syscall_num as usize] = Some(handler);

        // Allocate new array
        let new_array_box = Box::new(new_array);
        let new_outer_ptr = Box::into_raw(new_array_box);

        // Atomically update pointer (RCU-style)
        let prev_outer_ptr = self.fast_path.swap(new_outer_ptr, Ordering::Release);

        // Memory barrier to ensure all readers see the new pointer
        core::sync::atomic::fence(Ordering::SeqCst);

        // Schedule old array for deletion after grace period
        let sendable = SendableBoxPtr(prev_outer_ptr);
        unsafe {
            rcu::call_rcu(Box::new(move || {
                sendable.drop_box();
            }));
        }

        Ok(())
    }
    
    /// Get current CPU ID
    fn current_cpu_id(&self) -> usize {
        cpu::cpuid() % MAX_CPUS
    }
    
    /// Get per-CPU cache for current CPU
    fn get_per_cpu_cache(&self) -> &Mutex<PerCpuCache> {
        let cpu_id = self.current_cpu_id();
        &self.per_cpu_caches[cpu_id]
    }
    
    /// Update fast-path entries based on frequency
    fn update_fast_path(&self) {
        if !self.config.enable_adaptive_optimization {
            return;
        }

        let cpu_id = self.current_cpu_id();
        let cache = self.per_cpu_caches[cpu_id].lock();
        let _frequent_syscalls = cache.get_most_frequent(MAX_FAST_PATH_SYSCALLS);

        // In a real implementation, we would update the fast_path array here
        // For now, this is a placeholder
        drop(cache);
    }
    
    /// Convert &[u64] to SyscallArgs
    fn args_from_slice(args: &[u64]) -> SyscallArgs {
        [
            args.get(0).copied().unwrap_or(0),
            args.get(1).copied().unwrap_or(0),
            args.get(2).copied().unwrap_or(0),
            args.get(3).copied().unwrap_or(0),
            args.get(4).copied().unwrap_or(0),
            args.get(5).copied().unwrap_or(0),
        ]
    }

    /// Read Time-Stamp Counter (for performance measurement)
    #[inline(always)]
    fn rdtsc() -> u64 {
        #[cfg(target_arch = "x86_64")]
        {
            let mut low: u32;
            let mut high: u32;
            unsafe {
                core::arch::asm!(
                    "rdtsc",
                    out("eax") low,
                    out("edx") high,
                    options(nomem, nostack, preserves_flags)
                );
            }
            ((high as u64) << 32) | (low as u64)
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            let mut val: u64;
            unsafe {
                core::arch::asm!(
                    "mrs {0}, cntvct_el0",
                    out(reg) val,
                    options(nomem, nostack)
                );
            }
            val
        }
        
        #[cfg(target_arch = "riscv64")]
        {
            let mut val: u64;
            unsafe {
                core::arch::asm!(
                    "rdtime {0}",
                    out(reg) val,
                    options(nomem, nostack)
                );
            }
            val
        }
        
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
        {
            0
        }
    }
    
    /// Get dispatch statistics
    pub fn get_stats(&self) -> &DispatchStats {
        &self.stats
    }
}

impl SyscallDispatcher for UnifiedSyscallDispatcher {
    fn dispatch(&self, number: SyscallNumber, args: &[u64]) -> SyscallResult<()> {
        let start_time = if self.config.enable_monitoring {
            Self::rdtsc()
        } else {
            0
        };

        // Try fast-path first if enabled (lock-free read)
        if self.config.enable_fast_path {
            let fast_path = unsafe { self.get_fast_path() };
            if number < MAX_FAST_PATH_SYSCALLS as u32 {
                if let Some(handler) = fast_path[number as usize] {
                    // No lock needed - direct handler call
                    let result = handler(number, args);

                    // Update per-CPU cache as a fast-path hit
                    if self.config.enable_per_cpu_cache {
                        let mut cache = self.get_per_cpu_cache().lock();
                        cache.record_syscall(number);
                        cache.record_hit();
                    }

                    let end_time = if self.config.enable_monitoring {
                        Self::rdtsc()
                    } else {
                        0
                    };
                    let time_ns = end_time.saturating_sub(start_time);
                    self.stats.record_dispatch(result.is_ok(), true, time_ns);

                    // Convert the result to match the trait
                    return result.map(|_| ());
                }
            }
        }

        // Record syscall in per-CPU cache (regular path)
        if self.config.enable_per_cpu_cache {
            let mut cache = self.get_per_cpu_cache().lock();
            cache.record_syscall(number);
            cache.record_miss();
        }

        // Get handler from regular handlers (lock-free read)
        let handlers = unsafe { self.get_handlers() };
        if let Some(handler) = handlers.get(&number) {
            let _handler_name = handler.get_name();

            // Call the handler with the arguments
            let result = handler.handle(args);

            // Update statistics
            let end_time = if self.config.enable_monitoring {
                Self::rdtsc()
            } else {
                0
            };
            let time_ns = end_time.saturating_sub(start_time);
            self.stats.record_dispatch(result.is_ok(), false, time_ns);

            return result;
        }

        // Handler not found
        let end_time = if self.config.enable_monitoring {
            Self::rdtsc()
        } else {
            0
        };
        let time_ns = end_time.saturating_sub(start_time);
        self.stats.record_dispatch(false, false, time_ns);

        Err(InterfaceSyscallError::InterfaceNotFound)
    }

    fn get_stats(&self) -> SyscallStats {
        SyscallStats {
            total_calls: self.stats.total_dispatches.load(core::sync::atomic::Ordering::Relaxed),
            successful_calls: self.stats.successful_dispatches.load(core::sync::atomic::Ordering::Relaxed),
            failed_calls: self.stats.failed_dispatches.load(core::sync::atomic::Ordering::Relaxed),
            avg_execution_time_ns: self.stats.avg_time_ns(),
        }
    }

    fn list_handlers(&self) -> Vec<(usize, &str)> {
        let handlers = unsafe { self.get_handlers() };
        handlers.iter()
            .map(|(&num, handler)| (num as usize, handler.get_name()))
            .collect::<Vec<_>>()
    }

    fn is_supported(&self, num: SyscallNumber) -> bool {
        let handlers = unsafe { self.get_handlers() };
        handlers.contains_key(&num)
    }

    fn get_name(&self, num: SyscallNumber) -> Option<&'static str> {
        let handlers = unsafe { self.get_handlers() };
        handlers.get(&num).map(|handler| handler.get_name())
    }

    fn get_context(&self) -> &dyn SyscallContext {
        self.context.as_ref()
    }
}

impl UnifiedSyscallDispatcher {
    /// Batch-dispatch system calls with a single entry/exit to the dispatcher.
    ///
    /// This is a lightweight batching mechanism on top of `dispatch` that
    /// preserves existing semantics while reducing per-call overhead when
    /// the caller already has a group of syscalls to execute.
    pub fn batch_dispatch(&mut self, calls: &[(SyscallNumber, &[u64])]) -> SyscallBatchResult {
        let start_time = if self.config.enable_monitoring {
            Self::rdtsc()
        } else {
            0
        };

        let mut results = alloc::vec::Vec::with_capacity(calls.len());
        for (num, args) in calls.iter() {
            let res = self.dispatch(*num, args);
            // Convert SyscallResult (Result<()>) to Result<u64>
            let converted_result = res.map(|_| 0u64);
            results.push(converted_result);
        }

        let end_time = if self.config.enable_monitoring {
            Self::rdtsc()
        } else {
            0
        };
        let total_time = end_time.saturating_sub(start_time);

        SyscallBatchResult::new(results, total_time)
    }
}

/// Default context implementation (placeholder)
struct DefaultContext;

impl SyscallContext for DefaultContext {
    fn get_pid(&self) -> u32 {
        0
    }

    fn get_uid(&self) -> u32 {
        0
    }

    fn get_gid(&self) -> u32 {
        0
    }

    fn has_permission(&self, _operation: &str) -> bool {
        true
    }

    fn get_cwd(&self) -> &str {
        "/"
    }
}

/// Global unified dispatcher instance
static GLOBAL_UNIFIED_DISPATCHER: Mutex<Option<UnifiedSyscallDispatcher>> = Mutex::new(None);

/// Initialize the global unified dispatcher
pub fn init_unified_dispatcher(config: UnifiedDispatcherConfig) {
    let mut dispatcher_guard = GLOBAL_UNIFIED_DISPATCHER.lock();
    *dispatcher_guard = Some(UnifiedSyscallDispatcher::new(config));
}

/// Get the global unified dispatcher
pub fn get_unified_dispatcher() -> Option<&'static Mutex<Option<UnifiedSyscallDispatcher>>> {
    Some(&GLOBAL_UNIFIED_DISPATCHER)
}

/// Convenience helper for batch-dispatch using the global dispatcher.
pub fn unified_batch_dispatch(calls: &[(SyscallNumber, &[u64])]) -> Option<SyscallBatchResult> {
    get_unified_dispatcher().and_then(|mutex| {
        let mut guard = mutex.lock();
        guard.as_mut().map(|disp| disp.batch_dispatch(calls))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: &'static str,
    }

    impl SyscallHandler for TestHandler {
        fn handle(&self, _args: &[u64]) -> SyscallResult<()> {
            Ok(())
        }

        fn get_name(&self) -> &'static str {
            self.name
        }

        fn get_syscall_number(&self) -> SyscallNumber {
            0x1000
        }
    }

    #[test]
    fn test_unified_dispatcher_creation() {
        let config = UnifiedDispatcherConfig::default();
        let dispatcher = UnifiedSyscallDispatcher::new(config);
        // Test that dispatcher was created successfully
        assert!(!dispatcher.is_supported(0x1000));
    }

    #[test]
    fn test_handler_registration() {
        let config = UnifiedDispatcherConfig::default();
        let dispatcher = UnifiedSyscallDispatcher::new(config);
        let handler = Arc::new(TestHandler {
            name: "test",
        });

        // Test registering a handler
        let result = dispatcher.register_handler(0x1000, handler);
        assert!(result.is_ok());
        // Note: is_supported is part of the nos-api SyscallDispatcher trait
        assert!(dispatcher.is_supported(0x1000));
    }

    #[test]
    fn test_dispatch() {
        let config = UnifiedDispatcherConfig::default();
        let dispatcher = UnifiedSyscallDispatcher::new(config);
        let handler = Arc::new(TestHandler {
            name: "test",
        });

        dispatcher.register_handler(0x1000, handler);
        let result = dispatcher.dispatch(0x1000, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fast_path_registration() {
        let config = UnifiedDispatcherConfig::default();
        let dispatcher = UnifiedSyscallDispatcher::new(config);

        // Test registering a fast-path handler
        let fast_handler: FastPathHandler = |_num, _args| Ok(42);
        let result = dispatcher.register_fast_path(0x100, fast_handler);
        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_dispatch() {
        let config = UnifiedDispatcherConfig::default();
        let mut dispatcher = UnifiedSyscallDispatcher::new(config);
        let handler = Arc::new(TestHandler {
            name: "test",
        });

        dispatcher.register_handler(0x1000, handler.clone());
        dispatcher.register_handler(0x1001, handler);

        let calls = [(0x1000, &[][..]), (0x1001, &[][..])];
        let result = dispatcher.batch_dispatch(&calls);

        assert_eq!(result.len(), 2);
        assert_eq!(result.success_count(), 2);
        assert_eq!(result.failure_count(), 0);
    }
}

