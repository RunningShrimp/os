//! Fast-Path System Call Dispatcher
//!
//! This module implements an optimized system call dispatcher with:
//! - Fast-path for common syscalls
//! - Per-CPU syscall caches
//! - Inline assembly optimization for x86_64
//! - Syscall batching support
//! - Adaptive dispatch based on frequency

use core::arch::asm;
use core::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::sync::SpinLock;
use nos_api::{Result, Error};
use super::syscall_interface::{SyscallDispatcher, SyscallStats};
use super::api::syscall::{get_syscall_category, SyscallCategory};

/// Maximum number of fast-path syscalls
pub const MAX_FAST_PATH_SYSCALLS: usize = 64;

/// Syscall handler function type
pub type SyscallHandler = unsafe extern "C" fn(usize, &[usize]) -> isize;

/// Fast-path syscall entry
#[derive(Debug, Clone, Copy)]
pub struct FastPathEntry {
    /// System call number
    pub syscall_num: u32,
    /// Handler function
    pub handler: SyscallHandler,
    /// Call count (for statistics)
    pub call_count: AtomicU64,
    /// Total execution time (nanoseconds)
    pub total_time_ns: AtomicU64,
}

/// Per-CPU syscall cache
#[derive(Debug)]
pub struct PerCpuSyscallCache {
    /// Most recent syscalls (for pattern detection)
    recent_syscalls: Vec<u32>,
    /// Syscall frequency table
    frequency_table: BTreeMap<u32, u64>,
    /// Fast-path entries
    fast_path: Vec<FastPathEntry>,
    /// Cache hit count
    cache_hits: AtomicUsize,
    /// Cache miss count
    cache_misses: AtomicUsize,
}

impl PerCpuSyscallCache {
    /// Create a new per-CPU syscall cache
    pub fn new() -> Self {
        Self {
            recent_syscalls: Vec::with_capacity(16),
            frequency_table: BTreeMap::new(),
            fast_path: Vec::with_capacity(MAX_FAST_PATH_SYSCALLS),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }
    
    /// Add a syscall to the recent list
    pub fn add_recent_syscall(&mut self, syscall_num: u32) {
        // Add to recent list
        self.recent_syscalls.push(syscall_num);
        
        // Keep only the most recent 16 syscalls
        if self.recent_syscalls.len() > 16 {
            self.recent_syscalls.remove(0);
        }
        
        // Update frequency table
        let count = self.frequency_table.entry(syscall_num).or_insert(0);
        *count += 1;
    }
    
    /// Get the most frequent syscalls
    pub fn get_most_frequent_syscalls(&self, count: usize) -> Vec<u32> {
        let mut syscalls: Vec<(u64, u32)> = self.frequency_table.iter()
            .map(|(&num, &freq)| (freq, num))
            .collect();
        
        // Sort by frequency (descending)
        syscalls.sort_by(|a, b| b.0.cmp(&a.0));
        
        // Return the top 'count' syscalls
        syscalls.into_iter()
            .take(count)
            .map(|(_, num)| num)
            .collect()
    }
    
    /// Check if a syscall is in the fast path
    pub fn is_fast_path(&self, syscall_num: u32) -> bool {
        self.fast_path.iter().any(|entry| entry.syscall_num == syscall_num)
    }
    
    /// Get fast-path handler for a syscall
    pub fn get_fast_path_handler(&self, syscall_num: u32) -> Option<SyscallHandler> {
        self.fast_path.iter()
            .find(|entry| entry.syscall_num == syscall_num)
            .map(|entry| entry.handler)
    }
    
    /// Add a syscall to the fast path
    pub fn add_fast_path(&mut self, syscall_num: u32, handler: SyscallHandler) {
        // Check if we have space
        if self.fast_path.len() >= MAX_FAST_PATH_SYSCALLS {
            // Replace the least frequently used entry
            let mut min_count = u64::MAX;
            let mut min_index = 0;
            
            for (i, entry) in self.fast_path.iter().enumerate() {
                let count = entry.call_count.load(Ordering::Relaxed);
                if count < min_count {
                    min_count = count;
                    min_index = i;
                }
            }
            
            self.fast_path[min_index] = FastPathEntry {
                syscall_num,
                handler,
                call_count: AtomicU64::new(0),
                total_time_ns: AtomicU64::new(0),
            };
        } else {
            self.fast_path.push(FastPathEntry {
                syscall_num,
                handler,
                call_count: AtomicU64::new(0),
                total_time_ns: AtomicU64::new(0),
            });
        }
    }
    
    /// Record a cache hit
    pub fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a cache miss
    pub fn record_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get cache hit ratio
    pub fn hit_ratio(&self) -> f32 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        
        if hits + misses == 0 {
            return 0.0;
        }
        
        hits as f32 / (hits + misses) as f32
    }
    
    /// Update fast-path statistics
    pub fn update_fast_path_stats(&self, syscall_num: u32, execution_time_ns: u64) {
        if let Some(entry) = self.fast_path.iter_mut().find(|e| e.syscall_num == syscall_num) {
            entry.call_count.fetch_add(1, Ordering::Relaxed);
            entry.total_time_ns.fetch_add(execution_time_ns, Ordering::Relaxed);
        }
    }
}

/// Syscall batch for batching multiple syscalls
#[derive(Debug)]
pub struct SyscallBatch {
    /// Syscall numbers
    pub syscall_nums: Vec<u32>,
    /// Syscall arguments
    pub syscall_args: Vec<Vec<usize>>,
    /// Syscall results
    pub syscall_results: Vec<isize>,
    /// Execution times (nanoseconds)
    pub execution_times: Vec<u64>,
}

impl SyscallBatch {
    /// Create a new syscall batch
    pub fn new() -> Self {
        Self {
            syscall_nums: Vec::new(),
            syscall_args: Vec::new(),
            syscall_results: Vec::new(),
            execution_times: Vec::new(),
        }
    }
    
    /// Add a syscall to the batch
    pub fn add_syscall(&mut self, syscall_num: u32, args: Vec<usize>) {
        self.syscall_nums.push(syscall_num);
        self.syscall_args.push(args);
    }
    
    /// Execute all syscalls in the batch
    pub fn execute(&mut self, dispatcher: &dyn SyscallDispatcher) -> Result<()> {
        for (i, &syscall_num) in self.syscall_nums.iter().enumerate() {
            let args = &self.syscall_args[i];
            let start_time = rdtsc();
            
            let result = dispatcher.dispatch(syscall_num as usize, args);
            let end_time = rdtsc();
            
            let execution_time = end_time - start_time;
            
            self.syscall_results.push(result);
            self.execution_times.push(execution_time);
        }
        
        Ok(())
    }
    
    /// Get the results of the batch
    pub fn get_results(&self) -> &[isize] {
        &self.syscall_results
    }
    
    /// Get the execution times of the batch
    pub fn get_execution_times(&self) -> &[u64] {
        &self.execution_times
    }
}

/// Fast-path system call dispatcher
pub struct FastPathSyscallDispatcher {
    /// Per-CPU syscall caches
    per_cpu_caches: Vec<SpinLock<PerCpuSyscallCache>>,
    /// Current CPU ID
    current_cpu: AtomicUsize,
    /// Fallback dispatcher for non-fast-path syscalls
    fallback_dispatcher: Option<Box<dyn SyscallDispatcher>>,
    /// Global statistics
    global_stats: SyscallStats,
    /// Adaptive fast-path update interval
    update_interval: AtomicUsize,
    /// Syscall count since last update
    syscall_count_since_update: AtomicUsize,
}

impl FastPathSyscallDispatcher {
    /// Create a new fast-path syscall dispatcher
    pub fn new(num_cpus: usize) -> Self {
        let mut per_cpu_caches = Vec::with_capacity(num_cpus);
        for _ in 0..num_cpus {
            per_cpu_caches.push(Spin_lock!(PerCpuSyscallCache::new()));
        }
        
        Self {
            per_cpu_caches,
            current_cpu: AtomicUsize::new(0),
            fallback_dispatcher: None,
            global_stats: SyscallStats::default(),
            update_interval: AtomicUsize::new(1000), // Update every 1000 syscalls
            syscall_count_since_update: AtomicUsize::new(0),
        }
    }
    
    /// Set the fallback dispatcher
    pub fn set_fallback_dispatcher(&mut self, dispatcher: Box<dyn SyscallDispatcher>) {
        self.fallback_dispatcher = Some(dispatcher);
    }
    
    /// Set the current CPU ID
    pub fn set_current_cpu(&self, cpu_id: usize) {
        self.current_cpu.store(cpu_id, Ordering::Relaxed);
    }
    
    /// Add a syscall to the fast path
    pub fn add_fast_path_syscall(&self, syscall_num: u32, handler: SyscallHandler) {
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        let mut cache = self.per_cpu_caches[cpu_id].lock();
        cache.add_fast_path(syscall_num, handler);
    }
    
    /// Update fast-path entries based on frequency
    pub fn update_fast_path(&self) {
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        let mut cache = self.per_cpu_caches[cpu_id].lock();
        
        // Get the most frequent syscalls
        let frequent_syscalls = cache.get_most_frequent_syscalls(MAX_FAST_PATH_SYSCALLS);
        
        // Update the fast path with the most frequent syscalls
        // In a real implementation, we would have actual handlers
        // For now, we'll just update the statistics
        for &syscall_num in &frequent_syscalls {
            // This is where we would add the actual handler
            // cache.add_fast_path(syscall_num, handler);
        }
    }
    
    /// Get statistics for a specific CPU
    pub fn get_cpu_stats(&self, cpu_id: usize) -> Option<(f32, Vec<u32>)> {
        if cpu_id >= self.per_cpu_caches.len() {
            return None;
        }
        
        let cache = self.per_cpu_caches[cpu_id].lock();
        let hit_ratio = cache.hit_ratio();
        let frequent_syscalls = cache.get_most_frequent_syscalls(10);
        
        Some((hit_ratio, frequent_syscalls))
    }
}

impl SyscallDispatcher for FastPathSyscallDispatcher {
    fn dispatch(&self, syscall_num: usize, args: &[usize]) -> isize {
        let syscall_num = syscall_num as u32;
        
        // Get current CPU cache
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        let mut cache = self.per_cpu_caches[cpu_id].lock();
        
        // Add to recent syscalls
        cache.add_recent_syscall(syscall_num);
        
        // Check if it's a fast-path syscall
        if let Some(handler) = cache.get_fast_path_handler(syscall_num) {
            // Fast-path execution
            cache.record_hit();
            
            let start_time = rdtsc();
            let result = unsafe { handler(syscall_num as usize, args) };
            let end_time = rdtsc();
            
            let execution_time = end_time - start_time;
            cache.update_fast_path_stats(syscall_num, execution_time);
            
            return result;
        }
        
        // Not in fast path, use fallback
        cache.record_miss();
        
        // Update fast-path if needed
        let count = self.syscall_count_since_update.fetch_add(1, Ordering::Relaxed) + 1;
        let interval = self.update_interval.load(Ordering::Relaxed);
        
        if count >= interval {
            self.syscall_count_since_update.store(0, Ordering::Relaxed);
            drop(cache); // Release lock before updating
            
            // Update fast-path entries
            self.update_fast_path();
            
            // Re-acquire lock for fallback
            cache = self.per_cpu_caches[cpu_id].lock();
        }
        
        // Use fallback dispatcher
        if let Some(ref dispatcher) = self.fallback_dispatcher {
            dispatcher.dispatch(syscall_num as usize, args)
        } else {
            // No fallback dispatcher, return error
            -1
        }
    }
    
    fn get_stats(&self) -> SyscallStats {
        // Aggregate statistics from all CPUs
        let mut total_calls = 0u64;
        let mut successful_calls = 0u64;
        let mut failed_calls = 0u64;
        let mut total_execution_time_ns = 0u64;
        
        for cache in &self.per_cpu_caches {
            let cache = cache.lock();
            
            // Calculate statistics from fast-path entries
            for entry in &cache.fast_path {
                let calls = entry.call_count.load(Ordering::Relaxed);
                let time = entry.total_time_ns.load(Ordering::Relaxed);
                
                total_calls += calls;
                successful_calls += calls; // Assume all fast-path calls are successful
                total_execution_time_ns += time;
            }
            
            // Add cache misses to failed calls
            let misses = cache.cache_misses.load(Ordering::Relaxed);
            failed_calls += misses as u64;
        }
        
        // Calculate average execution time
        let avg_execution_time_ns = if total_calls > 0 {
            total_execution_time_ns / total_calls
        } else {
            0
        };
        
        SyscallStats {
            total_calls,
            successful_calls,
            failed_calls,
            avg_execution_time_ns,
        }
    }
}

/// Read Time-Stamp Counter
#[inline(always)]
fn rdtsc() -> u64 {
    let mut low: u32;
    let mut high: u32;
    
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
        
        #[cfg(target_arch = "aarch64")]
        asm!(
            "mrs {0}, cntvct_el0",
            out(reg) low,
            options(nomem, nostack)
        );
        
        #[cfg(target_arch = "riscv64")]
        asm!(
            "rdtime {0}",
            out(reg) low,
            options(nomem, nostack)
        );
    }
    
    ((high as u64) << 32) | (low as u64)
}

/// Initialize fast-path syscalls
pub fn init_fast_path_syscalls(dispatcher: &mut FastPathSyscallDispatcher) {
    // Add common syscalls to fast path
    // In a real implementation, we would add actual handlers
    
    // Process management syscalls
    dispatcher.add_fast_path_syscall(0x1000, getpid_fast_path);
    dispatcher.add_fast_path_syscall(0x1001, fork_fast_path);
    
    // File I/O syscalls
    dispatcher.add_fast_path_syscall(0x2000, read_fast_path);
    dispatcher.add_fast_path_syscall(0x2001, write_fast_path);
    dispatcher.add_fast_path_syscall(0x2002, open_fast_path);
    dispatcher.add_fast_path_syscall(0x2003, close_fast_path);
    
    // Memory management syscalls
    dispatcher.add_fast_path_syscall(0x3000, mmap_fast_path);
    dispatcher.add_fast_path_syscall(0x3001, munmap_fast_path);
}

// Fast-path syscall handlers
// In a real implementation, these would be optimized versions of the syscalls

extern "C" fn getpid_fast_path(_syscall_num: usize, _args: &[usize]) -> isize {
    // Fast-path implementation of getpid
    // In a real implementation, this would directly access the current process structure
    42 // Example PID
}

extern "C" fn fork_fast_path(_syscall_num: usize, _args: &[usize]) -> isize {
    // Fast-path implementation of fork
    // In a real implementation, this would use optimized process creation
    0 // Example child PID
}

extern "C" fn read_fast_path(_syscall_num: usize, args: &[usize]) -> isize {
    // Fast-path implementation of read
    // In a real implementation, this would use optimized I/O paths
    if args.len() >= 3 {
        let fd = args[0];
        let buf = args[1] as *mut u8;
        let count = args[2];
        
        // Example implementation
        if fd == 0 && !buf.is_null() && count > 0 {
            // Read from stdin
            unsafe {
                // In a real implementation, this would read from the actual stdin
                *buf = b'H';
                return 1;
            }
        }
    }
    
    -1 // Error
}

extern "C" fn write_fast_path(_syscall_num: usize, args: &[usize]) -> isize {
    // Fast-path implementation of write
    // In a real implementation, this would use optimized I/O paths
    if args.len() >= 3 {
        let fd = args[0];
        let buf = args[1] as *const u8;
        let count = args[2];
        
        // Example implementation
        if fd == 1 && !buf.is_null() && count > 0 {
            // Write to stdout
            unsafe {
                // In a real implementation, this would write to the actual stdout
                let slice = core::slice::from_raw_parts(buf, count);
                crate::println!("{}", core::str::from_utf8_unchecked(slice));
                return count as isize;
            }
        }
    }
    
    -1 // Error
}

extern "C" fn open_fast_path(_syscall_num: usize, _args: &[usize]) -> isize {
    // Fast-path implementation of open
    // In a real implementation, this would use optimized file opening
    3 // Example file descriptor
}

extern "C" fn close_fast_path(_syscall_num: usize, _args: &[usize]) -> isize {
    // Fast-path implementation of close
    // In a real implementation, this would use optimized file closing
    0 // Success
}

extern "C" fn mmap_fast_path(_syscall_num: usize, _args: &[usize]) -> isize {
    // Fast-path implementation of mmap
    // In a real implementation, this would use optimized memory mapping
    0x10000000 as isize // Example address
}

extern "C" fn munmap_fast_path(_syscall_num: usize, _args: &[usize]) -> isize {
    // Fast-path implementation of munmap
    // In a real implementation, this would use optimized memory unmapping
    0 // Success
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_per_cpu_syscall_cache() {
        let mut cache = PerCpuSyscallCache::new();
        
        // Test adding recent syscalls
        cache.add_recent_syscall(0x1000);
        cache.add_recent_syscall(0x2000);
        cache.add_recent_syscall(0x1000);
        
        // Test frequency table
        let frequent = cache.get_most_frequent_syscalls(2);
        assert_eq!(frequent.len(), 2);
        assert_eq!(frequent[0], 0x1000); // Most frequent
        
        // Test fast path
        assert!(!cache.is_fast_path(0x3000));
        
        cache.add_fast_path(0x3000, mmap_fast_path);
        assert!(cache.is_fast_path(0x3000));
        
        let handler = cache.get_fast_path_handler(0x3000);
        assert!(handler.is_some());
    }
    
    #[test]
    fn test_syscall_batch() {
        let mut batch = SyscallBatch::new();
        
        batch.add_syscall(0x1000, vec![]);
        batch.add_syscall(0x2000, vec![1, 0x1000 as usize, 5]);
        
        assert_eq!(batch.syscall_nums.len(), 2);
        assert_eq!(batch.syscall_args.len(), 2);
    }
    
    #[test]
    fn test_fast_path_dispatcher() {
        let mut dispatcher = FastPathSyscallDispatcher::new(2);
        
        // Set current CPU
        dispatcher.set_current_cpu(0);
        
        // Add fast-path syscalls
        init_fast_path_syscalls(&mut dispatcher);
        
        // Test fast-path dispatch
        let result = dispatcher.dispatch(0x1000 as usize, &[]);
        assert_eq!(result, 42); // getpid_fast_path returns 42
        
        // Test fallback (non-fast-path syscall)
        let result = dispatcher.dispatch(0x9999 as usize, &[]);
        assert_eq!(result, -1); // No fallback dispatcher, returns error
    }
}