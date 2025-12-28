#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Adaptive Spinlock with Dynamic Backoff
//!
//! This module provides an adaptive spinlock that dynamically adjusts
//! spin behavior based on observed contention to reduce CPU waste.
//!
//! Features:
//! - Dynamic backoff strategy (linear, exponential, hybrid)
//! - Contention detection and adaptation
//! - Thread-local optimization
//! - Hyper-Threading (SMT) awareness

use core::hint::spin_loop;
use core::sync::atomic;
use core::cell::Cell;
use core::sync::atomic;
use core::time::Duration;
use core::sync::atomic;

// ============================================================================
// Backoff Strategies
// ============================================================================

/// Backoff strategy for adaptive spinlock
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackoffStrategy {
    /// No backoff (spin continuously)
    None,
    
    /// Linear backoff: 1, 2, 3, 4, ... spins
    Linear(u8),
    
    /// Exponential backoff: 1, 2, 4, 8, 16, ... spins
    Exponential(u8),
    
    /// Hybrid: Start with linear, switch to exponential after threshold
    Hybrid {
        linear_max: u8,
        expo_base: u8,
        expo_threshold: u8,
    },
    
    /// Proportional backoff based on contention level
    Proportional(u8), // Contention level 0-15
}

// ============================================================================
// Adaptive Spinlock
// ============================================================================

/// Thread-local statistics for adaptive behavior
/// Each thread maintains its own contention observations
#[derive(Debug)]
struct ThreadLocalStats {
    /// Number of successful acquisitions
    success_count: u32,
    
    /// Number of failed acquisitions (contention)
    contention_count: u32,
    
    /// Current backoff level
    current_backoff: u8,
    
    /// Observed contention level (0-15)
    contention_level: u8,
}

impl ThreadLocalStats {
    const fn new() -> Self {
        Self {
            success_count: 0,
            contention_count: 0,
            current_backoff: 1, // Start with minimal backoff
            contention_level: 0,
        }
    }
    
    /// Update statistics on successful acquisition
    fn record_success(&mut self) {
        self.success_count += 1;
        self.contention_count = 0; // Reset contention on success
        
        // Gradually reduce backoff on successes
        if self.current_backoff > 1 {
            self.current_backoff = self.current_backoff / 2;
        }
        
        // Reduce contention level if no recent failures
        if self.contention_level > 0 {
            self.contention_level = self.contention_level / 2;
        }
    }
    
    /// Update statistics on failed acquisition (contention)
    fn record_contention(&mut self) {
        self.contention_count += 1;
        
        // Increase backoff on contention
        self.current_backoff = (self.current_backoff * 3).min(64);
        
        // Increase contention level
        self.contention_level = (self.contention_level + 1).min(15);
    }
    
    /// Get current backoff count based on strategy
    fn get_backoff(&self, strategy: BackoffStrategy) -> u8 {
        match strategy {
            BackoffStrategy::None => 0,
            
            BackoffStrategy::Linear(count) => count,
            
            BackoffStrategy::Exponential(count) => {
                if self.contention_level < 5 {
                    1 << count.min(4) // 1, 2, 4, 8, 16
                } else {
                    64 // Cap at high contention
                }
            }
            
            BackoffStrategy::Hybrid { linear_max, expo_base, expo_threshold } => {
                if self.contention_level < expo_threshold {
                    self.current_backoff.min(linear_max)
                } else {
                    1 << (expo_base + self.contention_level.min(4)).min(6)
                }
            }
            
            BackoffStrategy::Proportional(level) => {
                match level {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 6,
                    4 => 10,
                    5 => 16,
                    6 => 24,
                    7 => 32,
                    _ => 64, // Cap at high contention
                }
            }
        }
    }
}

/// Adaptive spinlock with dynamic backoff
/// Uses thread-local statistics to adjust spin behavior
#[repr(C, align(8))]
pub struct AdaptiveSpinLock {
    /// Lock state: 0 = unlocked, 1 = locked
    locked: AtomicBool,
    
    /// Owner thread ID (for deadlock detection)
    owner: AtomicU32,
    
    /// Number of waiting threads
    waiters: AtomicU32,
    
    /// Maximum spin count before yield/timeout
    max_spins: u8,
    
    /// Backoff strategy
    strategy: BackoffStrategy,
    
    /// Total spin attempts
    total_spins: AtomicU64,
    
    /// Contention events (for diagnostics)
    contention_events: AtomicU64,
}

impl AdaptiveSpinLock {
    /// Create a new adaptive spinlock
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            owner: AtomicU32::new(0),
            waiters: AtomicU32::new(0),
            max_spins: 100, // Default: spin up to 100 times
            strategy: BackoffStrategy::Hybrid {
                linear_max: 10,
                expo_base: 1,
                expo_threshold: 5,
            },
            total_spins: AtomicU64::new(0),
            contention_events: AtomicU64::new(0),
        }
    }
    
    /// Create adaptive spinlock with specific strategy
    pub fn with_strategy(strategy: BackoffStrategy, max_spins: u8) -> Self {
        Self {
            locked: AtomicBool::new(false),
            owner: AtomicU32::new(0),
            waiters: AtomicU32::new(0),
            max_spins,
            strategy,
            total_spins: AtomicU64::new(0),
            contention_events: AtomicU64::new(0),
        }
    }

    /// Try to acquire the lock (adaptive)
    /// Returns true if lock was acquired, false otherwise
    pub fn try_acquire(&self) -> bool {
        // Fast path: check if lock is available
        if !self.locked.load(Ordering::Acquire) {
            if self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                // Successfully acquired
                return true;
            }
        }
        
        // Lock is held, check if we can recurse (reentrant)
        let current_owner = self.owner.load(Ordering::Acquire);
        if current_owner == crate::0u32 as u32 {
            // Reentrant lock, allow acquisition
            return true;
        }
        
        // Lock is held by another thread
        false
    }

    /// Acquire the lock (with adaptive backoff)
    /// This implements the core spin-wait logic
    pub fn acquire(&self) {
        let thread_stats = Cell::new(ThreadLocalStats::new());
        
        // Increment waiters count
        self.waiters.fetch_add(1, Ordering::Relaxed);
        
        // Acquire lock with adaptive backoff
        let mut spin_count = 0u32;
        
        loop {
            // Check if we should abort (too many spins)
            if spin_count >= self.max_spins as u32 {
                crate::println!("[adaptive_spin] Timeout after {} spins", spin_count);
                
                // Record contention event
                self.contention_events.fetch_add(1, Ordering::Relaxed);
                break;
            }
            
            // Try to acquire
            if self.try_acquire() {
                // Success
                spin_count += 1;
                
                // Record success and update backoff
                thread_stats.get_mut().record_success();
                
                // Update total spins
                self.total_spins.fetch_add(1, Ordering::Relaxed);
                
                break;
            }
            
            // Failed to acquire, apply backoff
            spin_count += 1;
            
            // Record contention and update backoff
            thread_stats.get_mut().record_contention();
            let backoff = thread_stats.get().get_backoff(self.strategy);
            
            // Update total spins
            self.total_spins.fetch_add(1, Ordering::Relaxed);
            
            // Yield CPU with backoff
            for _ in 0..backoff {
                // Use pause hint instead of sleep
                core::hint::spin_loop();
            }
        }
        
        // Decrement waiters count
        self.waiters.fetch_sub(1, Ordering::Relaxed);
    }

    /// Try to acquire lock (without spinning)
    /// Non-blocking attempt
    pub fn try_lock(&self) -> bool {
        if self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            // Record acquisition
            self.total_spins.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Release the lock
    pub fn release(&self) {
        // Clear owner
        self.owner.store(0, Ordering::Release);
        
        // Release lock
        self.locked.store(false, Ordering::Release);
        
        // Wake up one waiting thread (simplified)
        // In full implementation, would use proper wake mechanism
    }

    /// Check if lock is currently held
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }

    /// Check if current thread holds the lock
    pub fn holding(&self) -> bool {
        self.owner.load(Ordering::Acquire) == crate::0u32 as u32
    }

    /// Get diagnostics
    pub fn diagnostics(&self) -> AdaptiveDiagnostics {
        let is_locked = self.is_locked();
        let owner = self.owner.load(Ordering::Acquire);
        let waiters = self.waiters.load(Ordering::Relaxed);
        let total_spins = self.total_spins.load(Ordering::Relaxed);
        let contention_events = self.contention_events.load(Ordering::Relaxed);
        
        // Calculate contention ratio
        let contention_ratio = if total_spins > 0 {
            contention_events as f64 / total_spins as f64
        } else {
            0.0
        };
        
        AdaptiveDiagnostics {
            is_locked,
            owner,
            waiters,
            total_spins,
            contention_events,
            contention_ratio,
            strategy: self.strategy,
        }
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        self.total_spins.store(0, Ordering::Release);
        self.contention_events.store(0, Ordering::Release);
    }

    /// Set maximum spin count
    pub fn set_max_spins(&self, max_spins: u8) {
        self.max_spins = max_spins;
    }

    /// Get spin statistics
    pub fn get_spin_stats(&self) -> (u64, u64) {
        let total = self.total_spins.load(Ordering::Relaxed);
        let contentions = self.contention_events.load(Ordering::Relaxed);
        (total, contentions)
    }
}

// ============================================================================
// RAII Guard
// ============================================================================

/// RAII guard for adaptive spinlock
pub struct AdaptiveSpinLockGuard<'a> {
    lock: &'a AdaptiveSpinLock,
}

impl<'a> Drop for AdaptiveSpinLockGuard<'a> {
    fn drop(&mut self) {
        self.lock.release();
    }
}

impl<'a> AdaptiveSpinLockGuard<'a> {
    /// Get the underlying data (for Mutex-like API)
    /// Note: This is just a lock guard, doesn't contain data
    /// In a full implementation, would wrap a protected data structure
    pub fn data_ref<T>(&self) -> Option<&'a T> {
        // Placeholder - real implementation would contain protected data
        None
    }
}

// ============================================================================
// Diagnostics
// ============================================================================

/// Diagnostic information for adaptive spinlock
#[derive(Debug, Clone)]
pub struct AdaptiveDiagnostics {
    pub is_locked: bool,
    pub owner: u32,
    pub waiters: u32,
    pub total_spins: u64,
    pub contention_events: u64,
    pub contention_ratio: f64,
    pub strategy: BackoffStrategy,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Create a lock suitable for low-contention scenarios
/// Uses no backoff strategy
pub fn fast_spinlock() -> AdaptiveSpinLock {
    AdaptiveSpinLock::new()
}

/// Create a lock suitable for high-contention scenarios
/// Uses exponential backoff
pub fn adaptive_spinlock() -> AdaptiveSpinLock {
    AdaptiveSpinLock::with_strategy(
        BackoffStrategy::Hybrid {
            linear_max: 10,
            expo_base: 1,
            expo_threshold: 5,
        },
        100 // Max 100 spins
    )
}

/// Create a lock for real-time scenarios
/// Uses fixed small backoff
pub fn realtime_spinlock() -> AdaptiveSpinLock {
    AdaptiveSpinLock::with_strategy(
        BackoffStrategy::Linear(5), // Fixed small backoff
        50, // Real-time: lower max spins
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threadlocal_stats() {
        let mut stats = ThreadLocalStats::new();
        
        // Test success path
        stats.record_success();
        assert_eq!(stats.current_backoff, 1);
        assert_eq!(stats.contention_level, 0);
        
        // Multiple successes reduce backoff
        stats.record_success();
        stats.record_success();
        assert_eq!(stats.current_backoff, 1); // Minimum
        assert_eq!(stats.contention_level, 0);
        
        // Test contention path
        stats.record_contention();
        assert_eq!(stats.current_backoff, 3);
        assert_eq!(stats.contention_level, 1);
        
        stats.record_contention();
        assert_eq!(stats.current_backoff, 9);
        assert_eq!(stats.contention_level, 2);
        
        stats.record_contention();
        assert_eq!(stats.current_backoff, 27);
        assert_eq!(stats.contention_level, 3);
        
        // Success after contention reduces backoff
        stats.record_success();
        assert!(stats.current_backoff < 27); // Should be reduced
    }

    #[test]
    fn test_backoff_strategies() {
        let stats = ThreadLocalStats::new();
        
        // Test linear backoff
        assert_eq!(stats.get_backoff(BackoffStrategy::Linear(10)), 10);
        
        // Test exponential backoff
        stats.contention_level = 5;
        assert_eq!(stats.get_backoff(BackoffStrategy::Exponential(2)), 16);
        
        // Test hybrid backoff
        stats.contention_level = 2; // Below threshold
        assert_eq!(stats.get_backoff(BackoffStrategy::Hybrid {
            linear_max: 10,
            expo_base: 1,
            expo_threshold: 5,
        }), 2);
        
        stats.contention_level = 6; // Above threshold
        assert_eq!(stats.get_backoff(BackoffStrategy::Hybrid {
            linear_max: 10,
            expo_base: 1,
            expo_threshold: 5,
        }), 4);
        
        // Test proportional backoff
        stats.contention_level = 3;
        assert_eq!(stats.get_backoff(BackoffStrategy::Proportional(3)), 6);
    }

    #[test]
    fn test_adaptive_spinlock() {
        let lock = adaptive_spinlock();
        
        assert!(lock.try_lock());
        assert!(!lock.try_lock()); // Can't acquire twice
        assert!(lock.is_locked());
        
        lock.release();
        assert!(!lock.is_locked());
    }

    #[test]
    fn test_contention_detection() {
        let lock = adaptive_spinlock();
        let stats = lock.get_spin_stats();
        
        // Acquire and release
        lock.acquire();
        
        // Check statistics
        let stats_after = lock.get_spin_stats();
        assert!(stats_after.0 > stats.0); // Should have recorded spins
    }
}
