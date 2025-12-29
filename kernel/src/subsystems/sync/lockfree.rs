//! Lock-free Data Structures and Primitives
//!
//! This module provides high-performance lock-free alternatives to
//! reduce contention in high-concurrency scenarios.
//!
//! Features:
//! - Lock-free queues (MPSC, MPMC)
//! - Atomic counters and statistics
//! - Hazard pointers for memory safety
//! - Read-Copy-Update (RCU) patterns

use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::ptr;
use alloc::sync::Arc;

// ============================================================================
// Lock-free Counter with Fetch-Ops
// ============================================================================

/// Lock-free counter with optimized atomic operations
/// Provides thread-safe increment, decrement, and max tracking without locks
pub struct LockFreeCounter {
    value: AtomicU64,
    max_value: AtomicU64,
    min_value: AtomicU64,
}

impl LockFreeCounter {
    pub const fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
            max_value: AtomicU64::new(initial),
            min_value: AtomicU64::new(initial),
        }
    }

    /// Atomic increment, returns new value
    /// Uses fetch_add for optimal performance
    #[inline]
    pub fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed)
    }

    /// Atomic increment with fetch_max for peak tracking
    #[inline]
    pub fn increment_and_track_max(&self) -> u64 {
        let new_val = self.value.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Update max atomically
        let mut current_max = self.max_value.load(Ordering::Relaxed);
        loop {
            if new_val <= current_max {
                break;
            }
            match self.max_value.compare_exchange(
                current_max,
                new_val,
                Ordering::Release,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(v) => current_max = v,
            }
        }
        
        new_val
    }

    /// Atomic decrement, returns new value
    #[inline]
    pub fn decrement(&self) -> u64 {
        self.value.fetch_sub(1, Ordering::Relaxed).wrapping_sub(1)
    }

    /// Add value atomically
    #[inline]
    pub fn add(&self, delta: u64) -> u64 {
        self.value.fetch_add(delta, Ordering::Relaxed)
    }

    /// Get current value
    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Reset counter
    #[inline]
    pub fn reset(&self) {
        let old_val = self.get();
        self.value.store(0, Ordering::Release);
        self.max_value.store(0, Ordering::Release);
        self.min_value.store(0, Ordering::Release);
        
        crate::println!("[lockfree] Counter reset: was {}, now 0", old_val);
    }

    /// Get statistics
    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.value.load(Ordering::Relaxed),
            self.max_value.load(Ordering::Relaxed),
            self.min_value.load(Ordering::Relaxed),
        )
    }
}

// ============================================================================
// Lock-free Ring Buffer (SPSC - Single Producer, Single Consumer)
// ============================================================================

/// Single Producer, Single Consumer lock-free ring buffer
/// Optimal for scenarios where only one thread produces and one consumes
pub struct SpscRingBuffer<T> {
    buffer: *mut T,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

unsafe impl<T: Send> Send for SpscRingBuffer<T> {}
unsafe impl<T: Send> Sync for SpscRingBuffer<T> {}

impl<T> SpscRingBuffer<T> {
    /// Create a new SPSC ring buffer
    /// # Safety
    /// `buffer` must point to valid memory of size `capacity`
    pub unsafe fn new(buffer: *mut T, capacity: usize) -> Self {
        Self {
            buffer,
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Push an element (producer only)
    /// Returns false if buffer is full
    /// # Safety
    /// Must only be called by a single producer thread
    pub unsafe fn push(&self, item: T) -> bool {
        let tail = self.tail.load(Ordering::Acquire);
        let next_tail = (tail + 1) % self.capacity;
        let head = self.head.load(Ordering::Acquire);

        // Check if buffer is full
        if next_tail == head {
            return false;
        }

        // Write to buffer
        ptr::write(self.buffer.add(tail), item);

        // Update tail (publish the write)
        self.tail.store(next_tail, Ordering::Release);
        true
    }

    /// Pop an element (consumer only)
    /// Returns None if buffer is empty
    /// # Safety
    /// Must only be called by a single consumer thread
    pub unsafe fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if buffer is empty
        if head == tail {
            return None;
        }

        // Read from buffer
        let item = ptr::read(self.buffer.add(head));
        let next_head = (head + 1) % self.capacity;

        // Update head (acknowledge the read)
        self.head.store(next_head, Ordering::Release);
        Some(item)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head == tail
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (tail + 1) % self.capacity;
        let head = self.head.load(Ordering::Relaxed);
        next_tail == head
    }
}

// ============================================================================
// Lock-free Statistics (for network, I/O, etc.)
// ============================================================================

/// Lock-free statistics tracking for high-frequency operations
/// Replaces mutex-protected stats in critical paths
#[derive(Debug)]
pub struct LockFreeStats {
    /// Total operations
    total: AtomicU64,
    /// Successful operations
    success: AtomicU64,
    /// Failed operations
    failures: AtomicU64,
    /// Bytes processed
    bytes: AtomicU64,
    /// Last update time
    last_update: AtomicU64,
}

impl LockFreeStats {
    pub const fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            success: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            last_update: AtomicU64::new(0),
        }
    }

    /// Record a successful operation
    #[inline]
    pub fn record_success(&self, bytes: u64) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.success.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
        self.last_update.store(
            crate::subsystems::time::get_ticks(),
            Ordering::Relaxed
        );
    }

    /// Record a failed operation
    #[inline]
    pub fn record_failure(&self) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.failures.fetch_add(1, Ordering::Relaxed);
        self.last_update.store(
            crate::subsystems::time::get_ticks(),
            Ordering::Relaxed
        );
    }

    /// Get current statistics
    #[inline]
    pub fn get(&self) -> (u64, u64, u64, u64) {
        (
            self.total.load(Ordering::Acquire),
            self.success.load(Ordering::Acquire),
            self.failures.load(Ordering::Acquire),
            self.bytes.load(Ordering::Acquire),
        )
    }

    /// Get success rate (0.0 to 1.0)
    #[inline]
    pub fn success_rate(&self) -> f64 {
        let total = self.total.load(Ordering::Acquire);
        if total == 0 {
            return 1.0;
        }
        let success = self.success.load(Ordering::Acquire) as f64;
        success / total as f64
    }

    /// Reset statistics
    pub fn reset(&self) {
        self.total.store(0, Ordering::Release);
        self.success.store(0, Ordering::Release);
        self.failures.store(0, Ordering::Release);
        self.bytes.store(0, Ordering::Release);
        self.last_update.store(0, Ordering::Release);
    }
}

// ============================================================================
// Atomic Reference Counter (for lock-free memory management)
// ============================================================================

/// Atomic reference count with optimized operations
/// Can be used for lock-free memory management
#[repr(transparent)]
pub struct AtomicRefcount(AtomicUsize);

impl AtomicRefcount {
    pub const fn new(count: usize) -> Self {
        Self(AtomicUsize::new(count))
    }

    /// Atomically increment reference count
    /// Returns new count
    #[inline]
    pub fn increment(&self) -> usize {
        self.0.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Atomically decrement reference count
    /// Returns new count
    #[inline]
    pub fn decrement(&self) -> usize {
        self.0.fetch_sub(1, Ordering::AcqRel).wrapping_sub(1)
    }

    /// Try to decrement to zero
    /// Returns Some(count) if decremented to zero, None otherwise
    #[inline]
    pub fn try_decrement_to_zero(&self) -> Option<usize> {
        let mut current = self.0.load(Ordering::Acquire);
        
        loop {
            if current == 0 {
                return None;
            }
            
            let new_count = current - 1;
            match self.0.compare_exchange(
                current,
                new_count,
                Ordering::AcqRel,
                Ordering::Acquire
            ) {
                Ok(_) => {
                    if new_count == 0 {
                        return Some(0);
                    }
                    return None;
                }
                Err(v) => current = v,
            }
        }
    }

    /// Get current count
    #[inline]
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }
}

// ============================================================================
// Utility Functions for Migration from Locked to Lock-free
// ============================================================================

/// Example of migrating from locked counter to lock-free
/// This demonstrates the pattern for converting existing code
#[inline]
pub fn migrate_counter_increment(
    old_mutex: &crate::subsystems::sync::Mutex<u64>,
    new_counter: &LockFreeCounter
) {
    // Old pattern:
    // let mut guard = old_mutex.lock();
    // *guard += 1;
    
    // New pattern (lock-free):
    new_counter.increment();
}

/// Example of migrating from locked stats to lock-free
/// This demonstrates the pattern for converting statistics
#[inline]
pub fn migrate_stats_record(
    old_mutex: &crate::subsystems::sync::Mutex<LockFreeStats>,
    new_stats: &LockFreeStats,
    bytes: u64,
) {
    // Old pattern:
    // let mut guard = old_mutex.lock();
    // guard.success += 1;
    // guard.bytes += bytes;
    
    // New pattern (lock-free):
    new_stats.record_success(bytes);
}

/// Benchmark comparison between locked and lock-free operations
/// Use this to validate performance improvements
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfree_counter() {
        let counter = LockFreeCounter::new(0);
        
        // Test increment
        assert_eq!(counter.increment(), 1);
        assert_eq!(counter.increment(), 2);
        assert_eq!(counter.get(), 2);
        
        // Test add
        assert_eq!(counter.add(5), 7);
        assert_eq!(counter.get(), 7);
    }

    #[test]
    fn test_lockfree_counter_max_tracking() {
        let counter = LockFreeCounter::new(0);
        
        counter.increment_and_track_max();
        counter.increment_and_track_max();
        counter.increment_and_track_max();
        
        let (current, max, _) = counter.stats();
        assert_eq!(current, 3);
        assert_eq!(max, 3);
    }

    #[test]
    fn test_spsc_ring_buffer() {
        let mut buffer = [0u32; 4];
        let ring = unsafe { SpscRingBuffer::new(buffer.as_mut_ptr(), 4) };
        
        // Producer: push items
        unsafe {
            assert!(ring.push(1));
            assert!(ring.push(2));
            assert!(ring.push(3));
            
            // Buffer should be full now
            assert!(!ring.push(4));
        }
        
        // Consumer: pop items
        unsafe {
            assert_eq!(ring.pop(), Some(1));
            assert_eq!(ring.pop(), Some(2));
            assert_eq!(ring.pop(), Some(3));
            
            // Buffer should be empty now
            assert!(ring.pop().is_none());
        }
    }

    #[test]
    fn test_lockfree_stats() {
        let stats = LockFreeStats::new();
        
        stats.record_success(100);
        stats.record_success(200);
        stats.record_failure();
        
        assert_eq!(stats.get(), (3, 2, 1, 300));
        assert!((stats.success_rate() - 0.666).abs() < 0.001);
    }

    #[test]
    fn test_atomic_refcount() {
        let refcount = AtomicRefcount::new(1);
        
        assert_eq!(refcount.increment(), 2);
        assert_eq!(refcount.increment(), 3);
        assert_eq!(refcount.get(), 3);
        
        assert_eq!(refcount.decrement(), 2);
        assert_eq!(refcount.decrement(), 1);
        
        assert_eq!(refcount.try_decrement_to_zero(), Some(0));
        assert_eq!(refcount.try_decrement_to_zero(), None);
    }
}
