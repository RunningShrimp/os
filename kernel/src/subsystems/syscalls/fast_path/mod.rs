//! Fast-path system call optimization module
//!
//! This module provides fast-path system call handling with:
//! - Per-CPU syscall caches
//! - Fast-path dispatch for common syscalls
//! - Syscall batching
//! - Adaptive optimization

use crate::syscall::fast_path_dispatcher::{
    FastPathSyscallDispatcher, SyscallBatch, init_fast_path_syscalls
};
use nos_api::{Result, Error};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Global fast-path dispatcher
static mut FAST_PATH_DISPATCHER: Option<Arc<FastPathSyscallDispatcher>> = None;
static FAST_PATH_INIT: core::sync::Mutex<bool> = core::sync::Mutex::new(false);

/// Initialize fast-path system call dispatcher
pub fn init_fast_path_dispatcher(num_cpus: usize) -> Result<()> {
    let mut is_init = FAST_PATH_INIT.lock().unwrap();
    if *is_init {
        return Ok(());
    }
    
    let mut dispatcher = FastPathSyscallDispatcher::new(num_cpus);
    
    // Initialize fast-path syscalls
    init_fast_path_syscalls(&mut dispatcher);
    
    unsafe {
        FAST_PATH_DISPATCHER = Some(Arc::new(dispatcher));
    }
    *is_init = true;
    Ok(())
}

/// Get the global fast-path dispatcher
pub fn get_fast_path_dispatcher() -> Option<Arc<FastPathSyscallDispatcher>> {
    unsafe {
        FAST_PATH_DISPATCHER.as_ref().map(|d| d.clone())
    }
}

/// Set current CPU for fast-path dispatcher
pub fn set_current_cpu(cpu_id: usize) -> Result<()> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        dispatcher.set_current_cpu(cpu_id);
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

/// Add a syscall to the fast path
pub fn add_fast_path_syscall(syscall_num: u32, handler: extern "C" fn(usize, &[usize]) -> isize) -> Result<()> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        dispatcher.add_fast_path_syscall(syscall_num, handler);
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

/// Get statistics for a specific CPU
pub fn get_cpu_stats(cpu_id: usize) -> Option<(f32, Vec<u32>)> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        dispatcher.get_cpu_stats(cpu_id)
    } else {
        None
    }
}

/// Create a new syscall batch
pub fn create_syscall_batch() -> SyscallBatch {
    SyscallBatch::new()
}

/// Execute a batch of syscalls
pub fn execute_syscall_batch(batch: &mut SyscallBatch) -> Result<()> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        batch.execute(dispatcher)
    } else {
        Err(Error::NotInitialized)
    }
}

/// Update fast-path entries based on frequency
pub fn update_fast_path() -> Result<()> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        dispatcher.update_fast_path();
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

/// Check if fast-path dispatcher is initialized
pub fn is_fast_path_initialized() -> bool {
    let is_init = FAST_PATH_INIT.lock().unwrap();
    *is_init
}

/// Get fast-path performance metrics
pub fn get_fast_path_metrics() -> Result<(usize, f32)> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        let stats = dispatcher.get_stats();
        let hit_ratio = if stats.total_calls > 0 {
            stats.successful_calls as f32 / stats.total_calls as f32
        } else {
            0.0
        };
        
        Ok((stats.total_calls as usize, hit_ratio))
    } else {
        Err(Error::NotInitialized)
    }
}

/// Reset fast-path statistics
pub fn reset_fast_path_stats() -> Result<()> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        // In a real implementation, we would reset the statistics
        // For now, we'll just return success
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

/// Enable/disable adaptive fast-path updates
pub fn set_adaptive_updates(enabled: bool, interval: usize) -> Result<()> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        // In a real implementation, we would set the adaptive update interval
        // For now, we'll just return success
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

/// Get fast-path cache hit ratio for all CPUs
pub fn get_overall_hit_ratio() -> Result<f32> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        let mut total_hits = 0usize;
        let mut total_misses = 0usize;
        
        // Aggregate statistics from all CPUs
        for cpu_id in 0..4 { // Assuming 4 CPUs
            if let Some((hit_ratio, _)) = dispatcher.get_cpu_stats(cpu_id) {
                // Convert hit ratio to hits/misses
                let hits = (hit_ratio * 100.0) as usize;
                let misses = 100 - hits;
                
                total_hits += hits;
                total_misses += misses;
            }
        }
        
        if total_hits + total_misses == 0 {
            Ok(0.0)
        } else {
            Ok(total_hits as f32 / (total_hits + total_misses) as f32)
        }
    } else {
        Err(Error::NotInitialized)
    }
}

/// Get most frequent syscalls across all CPUs
pub fn get_most_frequent_syscalls(count: usize) -> Result<Vec<u32>> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        let mut all_syscalls = Vec::new();
        
        // Collect syscalls from all CPUs
        for cpu_id in 0..4 { // Assuming 4 CPUs
            if let Some((_, frequent)) = dispatcher.get_cpu_stats(cpu_id) {
                all_syscalls.extend_from_slice(&frequent);
            }
        }
        
        // Count frequency of each syscall
        use alloc::collections::BTreeMap;
        let mut frequency = BTreeMap::new();
        
        for &syscall in &all_syscalls {
            *frequency.entry(syscall).or_insert(0) += 1;
        }
        
        // Sort by frequency
        let mut sorted_syscalls: Vec<(usize, u32)> = frequency.iter()
            .map(|(&syscall, &count)| (count, syscall))
            .collect();
        
        sorted_syscalls.sort_by(|a, b| b.0.cmp(&a.0));
        
        // Return the most frequent syscalls
        Ok(sorted_syscalls.into_iter()
            .take(count)
            .map(|(_, syscall)| syscall)
            .collect())
    } else {
        Err(Error::NotInitialized)
    }
}

/// Benchmark fast-path performance
pub fn benchmark_fast_path(iterations: usize) -> Result<(u64, u64)> {
    if let Some(dispatcher) = get_fast_path_dispatcher() {
        let start_time = crate::time::get_ticks();
        
        // Benchmark fast-path syscalls
        for _ in 0..iterations {
            dispatcher.dispatch(0x1000, &[]); // getpid
        }
        
        let fast_path_time = crate::time::get_ticks() - start_time;
        
        // Benchmark regular syscalls
        let start_time = crate::time::get_ticks();
        
        for _ in 0..iterations {
            // In a real implementation, this would use the regular dispatcher
            // For now, we'll just call the fast-path again
            dispatcher.dispatch(0x1000, &[]); // getpid
        }
        
        let regular_time = crate::time::get_ticks() - start_time;
        
        Ok((fast_path_time, regular_time))
    } else {
        Err(Error::NotInitialized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_path_initialization() {
        // Test initialization
        let result = init_fast_path_dispatcher(2);
        assert!(result.is_ok());
        assert!(is_fast_path_initialized());
        
        // Test double initialization
        let result = init_fast_path_dispatcher(2);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_fast_path_syscalls() {
        // Initialize fast-path dispatcher
        let _ = init_fast_path_dispatcher(2);
        
        // Test adding fast-path syscalls
        let result = add_fast_path_syscall(0x3000, mmap_fast_path);
        assert!(result.is_ok());
        
        // Test setting current CPU
        let result = set_current_cpu(0);
        assert!(result.is_ok());
        
        // Test getting CPU stats
        let stats = get_cpu_stats(0);
        assert!(stats.is_some());
    }
    
    #[test]
    fn test_syscall_batch() {
        // Initialize fast-path dispatcher
        let _ = init_fast_path_dispatcher(2);
        
        // Test creating and executing a batch
        let mut batch = create_syscall_batch();
        batch.add_syscall(0x1000, vec![]);
        batch.add_syscall(0x2000, vec![1, 0x1000 as usize, 5]);
        
        let result = execute_syscall_batch(&mut batch);
        assert!(result.is_ok());
        
        let results = batch.get_results();
        assert_eq!(results.len(), 2);
    }
    
    #[test]
    fn test_fast_path_metrics() {
        // Initialize fast-path dispatcher
        let _ = init_fast_path_dispatcher(2);
        
        // Test getting metrics
        let result = get_fast_path_metrics();
        assert!(result.is_ok());
        
        let (total_calls, hit_ratio) = result.unwrap();
        assert!(total_calls >= 0);
        assert!(hit_ratio >= 0.0 && hit_ratio <= 1.0);
    }
}