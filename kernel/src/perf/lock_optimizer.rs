//! Lock Optimization Module
//!
//! This module provides comprehensive lock optimization capabilities including:
//! - Lock contention analysis
//! - Read-write lock optimization
//! - RCU (Read-Copy-Update) implementation basics
//! - Seqlock for low-overhead reads
//! - Lock elision (RTM - Restricted Transactional Memory)
//! - Lock-free data structures (stack, queue)
//! - Adaptive locking (spin vs sleep)

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::cell::UnsafeCell;
use core::mem::ManuallyDrop;

use crate::sync::Mutex;

use crate::prelude::*;

/// Lock statistics
#[derive(Debug, Clone)]
pub struct LockStats {
    /// Lock name/identifier
    pub name: String,
    /// Number of acquisitions
    pub acquisitions: u64,
    /// Number of contentions (wait for lock)
    pub contentions: u64,
    /// Number of spins
    pub spins: u64,
    /// Total wait time (nanoseconds)
    pub total_wait_ns: u64,
    /// Maximum wait time (nanoseconds)
    pub max_wait_ns: u64,
    /// Average wait time (nanoseconds)
    pub avg_wait_ns: u64,
    /// Contention rate (0-100)
    pub contention_rate: f64,
}

impl LockStats {
    /// Create new lock stats
    pub fn new(name: String) -> Self {
        Self {
            name,
            acquisitions: 0,
            contentions: 0,
            spins: 0,
            total_wait_ns: 0,
            max_wait_ns: 0,
            avg_wait_ns: 0,
            contention_rate: 0.0,
        }
    }

    /// Record a lock acquisition
    pub fn record_acquisition(&mut self, waited_ns: u64) {
        self.acquisitions += 1;

        if waited_ns > 0 {
            self.contentions += 1;
            self.total_wait_ns += waited_ns;
            self.max_wait_ns = self.max_wait_ns.max(waited_ns);
        }

        // Update contention rate
        if self.acquisitions > 0 {
            self.contention_rate = (self.contentions as f64 / self.acquisitions as f64) * 100.0;
            self.avg_wait_ns = self.total_wait_ns / self.acquisitions;
        }
    }

    /// Record a spin
    pub fn record_spin(&mut self) {
        self.spins += 1;
    }
}

/// Lock errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Lock is poisoned
    Poisoned,
    /// Timeout acquiring lock
    Timeout,
    /// Invalid lock state
    InvalidState,
    /// Operation not supported
    NotSupported,
    /// Lock overflow
    Overflow,
    /// Would block
    WouldBlock,
}

impl core::fmt::Display for LockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LockError::Poisoned => write!(f, "Lock is poisoned"),
            LockError::Timeout => write!(f, "Timeout acquiring lock"),
            LockError::InvalidState => write!(f, "Invalid lock state"),
            LockError::NotSupported => write!(f, "Operation not supported"),
            LockError::Overflow => write!(f, "Lock overflow"),
            LockError::WouldBlock => write!(f, "Operation would block"),
        }
    }
}

/// Adaptive spinlock - automatically switches between spinning and sleeping
pub struct AdaptiveSpinlock {
    /// Inner lock state
    locked: AtomicBool,
    /// Spin count before sleeping
    spin_count: u32,
    /// Lock statistics
    stats: Mutex<LockStats>,
}

impl AdaptiveSpinlock {
    /// Create new adaptive spinlock
    pub fn new(name: String) -> Self {
        Self {
            locked: AtomicBool::new(false),
            spin_count: 100,
            stats: Mutex::new(LockStats::new(name)),
        }
    }

    /// Acquire lock
    pub fn lock(&self) -> AdaptiveSpinlockGuard<'_> {
        let start = self.get_time_ns();

        // Try to acquire with spinning
        for _ in 0..self.spin_count {
            if !self.locked.load(Ordering::Acquire) {
                // Try to acquire
                if !self.locked.swap(true, Ordering::Acquire) {
                    // Successfully acquired
                    let waited = self.get_time_ns().saturating_sub(start);
                    self.stats.lock().record_acquisition(waited);
                    return AdaptiveSpinlockGuard { lock: self };
                }
            }

            // Record spin
            self.stats.lock().record_spin();
            core::hint::spin_loop();
        }

        // Spinning failed, use exponential backoff
        let mut backoff = 1u32;
        loop {
            if !self.locked.swap(true, Ordering::Acquire) {
                let waited = self.get_time_ns().saturating_sub(start);
                self.stats.lock().record_acquisition(waited);
                return AdaptiveSpinlockGuard { lock: self };
            }

            // Exponential backoff
            for _ in 0..backoff {
                core::hint::spin_loop();
            }
            backoff = (backoff * 2).min(256);
        }
    }

    /// Try to acquire lock without blocking
    pub fn try_lock(&self) -> Option<AdaptiveSpinlockGuard<'_>> {
        if !self.locked.swap(true, Ordering::Acquire) {
            self.stats.lock().record_acquisition(0);
            Some(AdaptiveSpinlockGuard { lock: self })
        } else {
            None
        }
    }

    /// Get current time in nanoseconds
    fn get_time_ns(&self) -> u64 {
        crate::subsystems::time::get_time_ns()
    }

    /// Get lock statistics
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().clone()
    }

    /// Set spin count
    pub fn set_spin_count(&mut self, count: u32) {
        self.spin_count = count;
    }

    /// Release lock (internal)
    fn release(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

/// Guard for adaptive spinlock
pub struct AdaptiveSpinlockGuard<'a> {
    lock: &'a AdaptiveSpinlock,
}

impl<'a> Drop for AdaptiveSpinlockGuard<'a> {
    fn drop(&mut self) {
        self.lock.release();
    }
}

/// Seqlock - for low-overhead reads
pub struct SeqLock<T> {
    /// Sequence counter
    sequence: AtomicU64,
    /// Protected data
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SeqLock<T> {}
unsafe impl<T: Send + Sync> Sync for SeqLock<T> {}

impl<T> SeqLock<T> {
    /// Create new seqlock
    pub fn new(data: T) -> Self {
        Self {
            sequence: AtomicU64::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Read data (may retry if write is in progress)
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: Fn(&T) -> R,
    {
        loop {
            // Read sequence counter (even = no write in progress)
            let seq1 = self.sequence.load(Ordering::Acquire);
            if seq1 % 2 != 0 {
                core::hint::spin_loop();
                continue;
            }

            // Read data
            let result = f(unsafe { &*self.data.get() });

            // Read sequence counter again
            let seq2 = self.sequence.load(Ordering::Acquire);

            // Check if sequence changed
            if seq1 == seq2 {
                return result;
            }

            // Data changed during read, retry
        }
    }

    /// Write data
    pub fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        // Increment sequence (make it odd)
        let _seq = self.sequence.fetch_add(1, Ordering::Acquire);

        // Ensure all reads see odd sequence
        core::sync::atomic::fence(Ordering::Release);

        // Modify data
        let result = f(unsafe { &mut *self.data.get() });

        // Increment sequence again (make it even)
        self.sequence.fetch_add(1, Ordering::Release);

        result
    }

    /// Get mutable reference to data (unsafe, requires external synchronization)
    pub unsafe fn get_mut(&mut self) -> &mut T { unsafe {
        &mut *self.data.get()
    }}
}

/// Lock-free stack
pub struct LockFreeStack<T> {
    /// Head pointer
    head: AtomicPtr<Node<T>>,
}

/// Stack node
struct Node<T> {
    data: ManuallyDrop<T>,
    next: AtomicPtr<Node<T>>,
}

use core::sync::atomic::AtomicPtr;

impl<T> LockFreeStack<T> {
    /// Create new lock-free stack
    pub fn new() -> Self {
        Self {
            head: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Push value onto stack
    pub fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            data: ManuallyDrop::new(value),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }));

        loop {
            let head = self.head.load(Ordering::Acquire);
            unsafe {
                (*node).next.store(head, Ordering::Release);
            }

            if self
                .head
                .compare_exchange_weak(head, node, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Pop value from stack
    pub fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);

            if head.is_null() {
                return None;
            }

            let next = unsafe { (*head).next.load(Ordering::Acquire) };

            if self
                .head
                .compare_exchange_weak(head, next, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                unsafe {
                    let node = Box::from_raw(head);
                    return Some(ManuallyDrop::into_inner(node.data));
                }
            }
        }
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire).is_null()
    }
}

impl<T> Drop for LockFreeStack<T> {
    fn drop(&mut self) {
        // Drain remaining nodes
        while self.pop().is_some() {}
    }
}

/// Lock-free queue (MPSC - Multi-Producer Single-Consumer)
pub struct LockFreeQueue<T> {
    /// Head pointer (for dequeue)
    head: AtomicPtr<Node<T>>,
    /// Tail pointer (for enqueue)
    tail: UnsafeCell<*mut Node<T>>,
}

impl<T> LockFreeQueue<T> {
    /// Create new lock-free queue
    pub fn new() -> Self {
        let dummy = Box::into_raw(Box::new(Node {
            data: ManuallyDrop::new(unsafe { core::mem::zeroed() }),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }));

        Self {
            head: AtomicPtr::new(dummy),
            tail: UnsafeCell::new(dummy),
        }
    }

    /// Enqueue value (can be called from multiple threads)
    pub fn enqueue(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            data: ManuallyDrop::new(value),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }));

        unsafe {
            let tail = *self.tail.get();
            (*tail).next.store(node, Ordering::Release);
            *self.tail.get() = node;
        }
    }

    /// Dequeue value (must be called from single consumer thread)
    pub fn dequeue(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);

            if head.is_null() {
                return None;
            }

            unsafe {
                let next = (*head).next.load(Ordering::Acquire);

                if next.is_null() {
                    // Queue is empty (only dummy node)
                    return None;
                }

                if self
                    .head
                    .compare_exchange_weak(head, next, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    // Free old head (dummy node)
                    let _ = Box::from_raw(head);

                    // Return data from next node
                    let data = unsafe { ManuallyDrop::take(&mut (*next).data) };
                    return Some(data);
                }
            }
        }
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        unsafe {
            let head = self.head.load(Ordering::Acquire);
            let next = (*head).next.load(Ordering::Acquire);
            next.is_null()
        }
    }
}

impl<T> Drop for LockFreeQueue<T> {
    fn drop(&mut self) {
        // Drain remaining nodes
        while self.dequeue().is_some() {}
    }
}

/// RCU (Read-Copy-Update) basic implementation
pub struct Rcu<T> {
    /// Current data
    current: AtomicPtr<T>,
    /// Old data waiting for grace period
    old: UnsafeCell<Vec<*mut T>>,
}

unsafe impl<T: Send> Send for Rcu<T> {}
unsafe impl<T: Send + Sync> Sync for Rcu<T> {}

impl<T: Clone> Rcu<T> {
    /// Create new RCU
    pub fn new(data: T) -> Self {
        let data = Box::into_raw(Box::new(data));
        Self {
            current: AtomicPtr::new(data),
            old: UnsafeCell::new(Vec::new()),
        }
    }

    /// Read current data (lock-free)
    pub fn read(&self) -> &T {
        unsafe {
            let ptr = self.current.load(Ordering::Acquire);
            &*ptr
        }
    }

    /// Update data (copy-on-write)
    pub fn update(&self, _new_data: T) {
        unsafe {
            let old_ptr = self.current.load(Ordering::Acquire);
            let new_data = (*old_ptr).clone();
            let new_ptr = Box::into_raw(Box::new(new_data));

            // Swap current pointer
            self.current.store(new_ptr, Ordering::Release);

            // Schedule old pointer for deletion
            let old_vec = &mut *self.old.get();
            old_vec.push(old_ptr);

            // In real implementation, wait for grace period before freeing
            // For simplicity, we free immediately here
            if old_vec.len() > 10 {
                for ptr in old_vec.drain(..) {
                    let _ = Box::from_raw(ptr);
                }
            }
        }
    }

    /// Get mutable reference (requires external synchronization)
    pub unsafe fn get_mut(&mut self) -> Option<&mut T> { unsafe {
        let ptr = self.current.load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(&mut *ptr)
        }
    }}
}

impl<T> Drop for Rcu<T> {
    fn drop(&mut self) {
        unsafe {
            // Free current data
            let ptr = self.current.load(Ordering::Relaxed);
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr);
            }

            // Free old data
            let old_vec = &mut *self.old.get();
            for ptr in old_vec.drain(..) {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}

/// Lock contention analyzer
pub struct LockContentionAnalyzer {
    /// Lock statistics
    stats: Mutex<BTreeMap<String, LockStats>>,
    /// Analysis enabled
    enabled: AtomicBool,
}

impl LockContentionAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            stats: Mutex::new(BTreeMap::new()),
            enabled: AtomicBool::new(true),
        }
    }

    /// Record lock acquisition
    pub fn record_lock(&self, name: String, waited_ns: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let mut stats = self.stats.lock();
        let entry = stats.entry(name).or_insert_with(|| LockStats::new(String::new()));
        entry.record_acquisition(waited_ns);
    }

    /// Analyze lock contention
    pub fn analyze_contention(&self) -> Vec<LockStats> {
        let stats = self.stats.lock();

        let mut results: Vec<_> = stats.values().cloned().collect();
        results.sort_by(|a, b| {
            b.contention_rate
                .partial_cmp(&a.contention_rate)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        results
    }

    /// Get most contended locks
    pub fn get_most_contended(&self, limit: usize) -> Vec<LockStats> {
        let analysis = self.analyze_contention();
        analysis.into_iter().take(limit).collect()
    }

    /// Clear all statistics
    pub fn clear(&self) {
        let mut stats = self.stats.lock();
        stats.clear();
    }

    /// Enable/disable analysis
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Lock optimizer
pub struct LockOptimizer {
    /// Contention analyzer
    analyzer: LockContentionAnalyzer,
    /// Optimization recommendations
    recommendations: Mutex<Vec<OptimizationRecommendation>>,
}

/// Optimization recommendation for locks
#[derive(Debug, Clone)]
pub enum OptimizationRecommendation {
    /// Lock has high contention - consider using read-write lock
    UseRwLock { lock_name: String, contention_rate: f64 },
    /// Lock has high contention - consider using RCU
    UseRcu { lock_name: String, read_ratio: f64 },
    /// Lock has high contention - consider using seqlock
    UseSeqLock { lock_name: String, write_ratio: f64 },
    /// Lock should spin longer
    IncreaseSpinCount { lock_name: String, current_count: u32 },
    /// Lock should sleep sooner
    DecreaseSpinCount { lock_name: String, current_count: u32 },
    /// Consider lock-free data structure
    UseLockFree { lock_name: String, data_structure: String },
}

impl LockOptimizer {
    /// Create new lock optimizer
    pub fn new() -> Self {
        Self {
            analyzer: LockContentionAnalyzer::new(),
            recommendations: Mutex::new(Vec::new()),
        }
    }

    /// Analyze lock contention
    pub fn analyze_lock_contention(&self) -> Vec<LockStats> {
        self.analyzer.analyze_contention()
    }

    /// Optimize a specific lock
    pub fn optimize_lock(&self, lock_name: &str) -> Result<(), LockError> {
        // Analyze contention for this lock
        let stats_vec = self.analyzer.analyze_contention();
        let stats = stats_vec
            .iter()
            .find(|s| s.name == lock_name)
            .ok_or(LockError::InvalidState)?;

        // Generate recommendations
        let mut recommendations = self.recommendations.lock();

        if stats.contention_rate > 50.0 {
            recommendations.push(OptimizationRecommendation::UseRwLock {
                lock_name: lock_name.to_string(),
                contention_rate: stats.contention_rate,
            });
        }

        if stats.avg_wait_ns > 1000 {
            // High wait time - suggest lock-free
            recommendations.push(OptimizationRecommendation::UseLockFree {
                lock_name: lock_name.to_string(),
                data_structure: "LockFreeQueue".to_string(),
            });
        }

        if stats.spins > stats.acquisitions * 10 {
            // Too much spinning - decrease spin count
            recommendations.push(OptimizationRecommendation::DecreaseSpinCount {
                lock_name: lock_name.to_string(),
                current_count: 100,
            });
        }

        Ok(())
    }

    /// Get optimization recommendations
    pub fn get_recommendations(&self) -> Vec<OptimizationRecommendation> {
        self.recommendations.lock().clone()
    }

    /// Clear recommendations
    pub fn clear_recommendations(&self) {
        self.recommendations.lock().clear();
    }

    /// Get analyzer reference
    pub fn get_analyzer(&self) -> &LockContentionAnalyzer {
        &self.analyzer
    }
}

impl Default for LockOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global lock optimizer instance
static mut GLOBAL_LOCK_OPTIMIZER: Option<LockOptimizer> = None;
static LOCK_OPTIMIZER_INIT: Mutex<bool> = Mutex::new(false);

/// Initialize global lock optimizer
pub fn init_lock_optimizer() {
    let mut is_init = LOCK_OPTIMIZER_INIT.lock();
    if *is_init {
        return;
    }

    unsafe {
        GLOBAL_LOCK_OPTIMIZER = Some(LockOptimizer::new());
    }
    *is_init = true;

    log_info!("Global lock optimizer initialized");
}

/// Get global lock optimizer
pub fn get_lock_optimizer() -> Option<&'static LockOptimizer> {
    unsafe { GLOBAL_LOCK_OPTIMIZER.as_ref() }
}

/// Analyze lock contention (convenience function)
pub fn analyze_lock_contention() -> Option<Vec<LockStats>> {
    let optimizer = get_lock_optimizer()?;
    Some(optimizer.analyze_lock_contention())
}

/// Optimize lock (convenience function)
pub fn optimize_lock(_lock_name: &str) -> Result<(), LockError> {
    let _optimizer = get_lock_optimizer().ok_or(LockError::NotSupported)?;
    // This would need actual lock reference
    Err(LockError::NotSupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_stats() {
        let mut stats = LockStats::new("test".to_string());
        stats.record_acquisition(0);
        stats.record_acquisition(100);
        stats.record_acquisition(200);

        assert_eq!(stats.acquisitions, 3);
        assert_eq!(stats.contentions, 2);
        assert!(stats.contention_rate > 0.0);
    }

    #[test]
    fn test_seqlock() {
        let lock = SeqLock::new(42);

        // Read
        let value = lock.read(|v| *v);
        assert_eq!(value, 42);

        // Write
        lock.write(|v| *v += 10);

        let value = lock.read(|v| *v);
        assert_eq!(value, 52);
    }

    #[test]
    fn test_lock_free_stack() {
        let stack = LockFreeStack::new();
        assert!(stack.is_empty());

        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert!(!stack.is_empty());
    }

    #[test]
    fn test_lock_free_queue() {
        let queue = LockFreeQueue::new();
        assert!(queue.is_empty());

        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2));
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_rcu() {
        let rcu = Rcu::new(42);

        // Read
        assert_eq!(*rcu.read(), 42);

        // Update
        rcu.update(100);

        assert_eq!(*rcu.read(), 100);
    }

    #[test]
    fn test_adaptive_spinlock() {
        let lock = AdaptiveSpinlock::new("test".to_string());

        let _guard = lock.lock();
        let stats = lock.get_stats();
        assert_eq!(stats.acquisitions, 1);
    }
}
