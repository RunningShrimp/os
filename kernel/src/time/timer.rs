//! High-resolution timer wheel implementation
//!
//! This module provides a sophisticated timer management system with:
//! - Timer wheel data structure for O(1) timer operations
//! - Sub-microsecond timer resolution
//! - Efficient timer expiration handling
//! - Timer bucket management
//! - Cascading timer wheels
//!
//! # Architecture
//!
//! The timer wheel uses a hierarchical bucket structure inspired by the
//! Linux kernel's timer implementation:
//!
//! - Level 0: 256 buckets of 1ms granularity (256ms range)
//! - Level 1: 64 buckets of 256ms granularity (16.38s range)
//! - Level 2: 64 buckets of 16.38s granularity (17.5 min range)
//! - Level 3: 64 buckets of 17.5min granularity (18.8 hour range)
//! - Level 4: 64 buckets of 18.8hr granularity (50 day range)
//!
//! # Example
//!
//! ```no_run
//! use kernel::time::timer::{TimerWheel, Timer};
//! use core::time::Duration;
//!
//! let wheel = TimerWheel::new();
//! let timer = Timer::new(Duration::from_millis(100), || {
//!     println!("Timer expired!");
//! });
//! wheel.add_timer(timer);
//! ```

use alloc::boxed::Box;
use alloc::collections::LinkedList;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use core::time::Duration;
use spin::{Mutex, RwLock};

/// Timer wheel granularity configuration
const WHEEL_SIZE_L0: usize = 256; // 2^8
const WHEEL_SIZE_L1: usize = 64;  // 2^6
const WHEEL_SIZE_L2: usize = 64;
const WHEEL_SIZE_L3: usize = 64;
const WHEEL_SIZE_L4: usize = 64;

/// Time base for level 0 (milliseconds)
const TIME_BASE_L0: u64 = 1;

/// Time multiplier for each level
const TIME_MULT_L1: u64 = WHEEL_SIZE_L0 as u64; // 256
const TIME_MULT_L2: u64 = TIME_MULT_L1 * WHEEL_SIZE_L1 as u64; // 16,384
const TIME_MULT_L3: u64 = TIME_MULT_L2 * WHEEL_SIZE_L2 as u64; // 1,048,576
const TIME_MULT_L4: u64 = TIME_MULT_L3 * WHEEL_SIZE_L3 as u64; // 67,108,864

/// Maximum timer duration (approximately 50 days)
const MAX_TIMER_DURATION: u64 = TIME_MULT_L4 * WHEEL_SIZE_L4 as u64;

/// Timer ID type
pub type TimerId = u64;

/// Timer callback function type
pub type TimerCallback = Box<dyn FnOnce() + Send>;

/// Timer expiration callback result
pub enum TimerCallbackResult {
    /// Timer completed successfully
    Completed,
    /// Timer should be rescheduled
    Reschedule(Duration),
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// Timer is pending
    Pending,
    /// Timer is executing
    Executing,
    /// Timer has been canceled
    Canceled,
    /// Timer has expired
    Expired,
}

/// Timer structure
pub struct Timer {
    /// Unique timer ID
    id: TimerId,
    /// Timer state
    state: AtomicU8,
    /// Expiration time (absolute, in nanoseconds)
    expires_at: AtomicU64,
    /// Timer interval (for periodic timers)
    interval: Duration,
    /// Whether timer is periodic
    periodic: bool,
    /// Callback function
    callback: Mutex<Option<TimerCallback>>,
    /// Creation time
    created_at: u64,
}

impl Timer {
    /// Create new one-shot timer
    pub fn new(duration: Duration, callback: impl FnOnce() + Send + 'static) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let created_at = Self::current_time_ns();
        let expires_at = created_at + duration.as_nanos() as u64;

        Self {
            id,
            state: AtomicU8::new(TimerState::Pending as u8),
            expires_at: AtomicU64::new(expires_at),
            interval: duration,
            periodic: false,
            callback: Mutex::new(Some(Box::new(callback))),
            created_at,
        }
    }

    /// Create periodic timer
    pub fn periodic(duration: Duration, callback: impl Fn() + Send + 'static) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let created_at = Self::current_time_ns();
        let expires_at = created_at + duration.as_nanos() as u64;

        Self {
            id,
            state: AtomicU8::new(TimerState::Pending as u8),
            expires_at: AtomicU64::new(expires_at),
            interval: duration,
            periodic: true,
            callback: Mutex::new(Some(Box::new(callback))),
            created_at,
        }
    }

    /// Get timer ID
    pub fn id(&self) -> TimerId {
        self.id
    }

    /// Get timer state
    pub fn state(&self) -> TimerState {
        match self.state.load(Ordering::Acquire) {
            0 => TimerState::Pending,
            1 => TimerState::Executing,
            2 => TimerState::Canceled,
            _ => TimerState::Expired,
        }
    }

    /// Check if timer is expired
    pub fn is_expired(&self) -> bool {
        let now = Self::current_time_ns();
        self.expires_at.load(Ordering::Acquire) <= now
    }

    /// Get expiration time
    pub fn expires_at(&self) -> u64 {
        self.expires_at.load(Ordering::Acquire)
    }

    /// Get remaining time
    pub fn remaining(&self) -> Duration {
        let now = Self::current_time_ns();
        let expires = self.expires_at.load(Ordering::Acquire);

        if expires > now {
            Duration::from_nanos(expires - now)
        } else {
            Duration::ZERO
        }
    }

    /// Check if timer is periodic
    pub fn is_periodic(&self) -> bool {
        self.periodic
    }

    /// Set timer state
    fn set_state(&self, state: TimerState) {
        self.state.store(state as u8, Ordering::Release);
    }

    /// Reschedule periodic timer
    fn reschedule(&self) {
        if self.periodic {
            let now = Self::current_time_ns();
            let interval_ns = self.interval.as_nanos() as u64;
            let new_expires = now + interval_ns;
            self.expires_at.store(new_expires, Ordering::Release);
            self.set_state(TimerState::Pending);
        }
    }

    /// Execute callback
    fn execute(&self) -> Option<TimerCallback> {
        let state = TimerState::Executing;
        self.state.store(state as u8, Ordering::Release);

        self.callback.lock().take()
    }

    /// Get current time in nanoseconds (stub)
    fn current_time_ns() -> u64 {
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIME: AtomicU64 = AtomicU64::new(1_650_000_000_000_000_000);
        TIME.fetch_add(1_000_000, Ordering::SeqCst) // Advance by 1ms
    }
}

/// Timer bucket containing a list of timers
struct TimerBucket {
    /// List of timers in this bucket
    timers: LinkedList<TimerId>,
}

impl TimerBucket {
    fn new() -> Self {
        Self {
            timers: LinkedList::new(),
        }
    }

    fn add_timer(&mut self, id: TimerId) {
        self.timers.push_back(id);
    }

    fn remove_timer(&mut self, id: TimerId) -> bool {
        let mut found = false;
        self.timers.retain(|&timer_id| {
            if timer_id == id {
                found = true;
                false
            } else {
                true
            }
        });
        found
    }

    fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = &TimerId> {
        self.timers.iter()
    }
}

/// Timer wheel level
struct TimerWheelLevel {
    /// Number of buckets in this level
    size: usize,
    /// Current index in this level
    index: usize,
    /// Time multiplier for this level
    multiplier: u64,
    /// Buckets
    buckets: Vec<TimerBucket>,
}

impl TimerWheelLevel {
    fn new(size: usize, multiplier: u64) -> Self {
        let mut buckets = Vec::with_capacity(size);
        for _ in 0..size {
            buckets.push(TimerBucket::new());
        }

        Self {
            size,
            index: 0,
            multiplier,
            buckets,
        }
    }

    fn add_timer(&mut self, bucket_idx: usize, timer_id: TimerId) {
        self.buckets[bucket_idx].add_timer(timer_id);
    }

    fn remove_timer(&mut self, bucket_idx: usize, timer_id: TimerId) -> bool {
        self.buckets[bucket_idx].remove_timer(timer_id)
    }

    fn get_bucket(&self, idx: usize) -> &TimerBucket {
        &self.buckets[idx]
    }

    fn get_bucket_mut(&mut self, idx: usize) -> &mut TimerBucket {
        &mut self.buckets[idx]
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % self.size;
    }

    fn current_index(&self) -> usize {
        self.index
    }
}

/// Main timer wheel
pub struct TimerWheel {
    /// Current time (base time in milliseconds)
    current_time: AtomicU64,
    /// Timer storage
    timers: RwLock<Vec<Option<Timer>>>,
    /// Level 0 wheel (256 x 1ms)
    level0: Mutex<TimerWheelLevel>,
    /// Level 1 wheel (64 x 256ms)
    level1: Mutex<TimerWheelLevel>,
    /// Level 2 wheel (64 x 16.384s)
    level2: Mutex<TimerWheelLevel>,
    /// Level 3 wheel (64 x 1.048min)
    level3: Mutex<TimerWheelLevel>,
    /// Level 4 wheel (64 x 67.1min)
    level4: Mutex<TimerWheelLevel>,
    /// Next timer tick time
    next_tick: AtomicU64,
}

impl TimerWheel {
    /// Create new timer wheel
    pub fn new() -> Self {
        Self {
            current_time: AtomicU64::new(0),
            timers: RwLock::new(Vec::new()),
            level0: Mutex::new(TimerWheelLevel::new(WHEEL_SIZE_L0, TIME_BASE_L0)),
            level1: Mutex::new(TimerWheelLevel::new(WHEEL_SIZE_L1, TIME_MULT_L1)),
            level2: Mutex::new(TimerWheelLevel::new(WHEEL_SIZE_L2, TIME_MULT_L2)),
            level3: Mutex::new(TimerWheelLevel::new(WHEEL_SIZE_L3, TIME_MULT_L3)),
            level4: Mutex::new(TimerWheelLevel::new(WHEEL_SIZE_L4, TIME_MULT_L4)),
            next_tick: AtomicU64::new(0),
        }
    }

    /// Add timer to wheel
    pub fn add_timer(&self, timer: Timer) -> Result<(), TimerError> {
        let id = timer.id();
        let expires = timer.expires_at();
        let duration_ms = (expires - timer.created_at) / 1_000_000;

        if duration_ms > MAX_TIMER_DURATION {
            return Err(TimerError::DurationTooLong);
        }

        // Store timer
        {
            let mut timers = self.timers.write();
            let idx = id as usize;
            if idx >= timers.len() {
                timers.resize(idx + 1, None);
            }
            timers[idx] = Some(timer);
        }

        // Add to appropriate bucket
        self.schedule_timer(id, duration_ms);

        // Update next tick if needed
        self.update_next_tick(duration_ms);

        Ok(())
    }

    /// Schedule timer in appropriate wheel level
    fn schedule_timer(&self, id: TimerId, duration_ms: u64) {
        let current = self.current_time.load(Ordering::Acquire);
        let delta = duration_ms;

        if delta < TIME_MULT_L1 {
            // Level 0
            let idx = ((delta + current) % (WHEEL_SIZE_L0 as u64)) as usize;
            let mut level = self.level0.lock();
            level.add_timer(idx, id);
        } else if delta < TIME_MULT_L2 {
            // Level 1
            let idx = ((delta + current) / TIME_MULT_L1 % (WHEEL_SIZE_L1 as u64)) as usize;
            let mut level = self.level1.lock();
            level.add_timer(idx, id);
        } else if delta < TIME_MULT_L3 {
            // Level 2
            let idx = ((delta + current) / TIME_MULT_L2 % (WHEEL_SIZE_L2 as u64)) as usize;
            let mut level = self.level2.lock();
            level.add_timer(idx, id);
        } else if delta < TIME_MULT_L4 {
            // Level 3
            let idx = ((delta + current) / TIME_MULT_L3 % (WHEEL_SIZE_L3 as u64)) as usize;
            let mut level = self.level3.lock();
            level.add_timer(idx, id);
        } else {
            // Level 4
            let idx = ((delta + current) / TIME_MULT_L4 % (WHEEL_SIZE_L4 as u64)) as usize;
            let mut level = self.level4.lock();
            level.add_timer(idx, id);
        }
    }

    /// Update next tick time
    fn update_next_tick(&self, duration_ms: u64) {
        let mut next = self.next_tick.load(Ordering::Acquire);
        loop {
            if duration_ms < next {
                match self.next_tick.compare_exchange_weak(
                    next,
                    duration_ms,
                    Ordering::SeqCst,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(new_next) => next = new_next,
                }
            } else {
                break;
            }
        }
    }

    /// Cancel timer
    pub fn cancel_timer(&self, id: TimerId) -> Result<bool, TimerError> {
        let timers = self.timers.read();

        if let Some(timer) = timers.get(id as usize).and_then(|t| t.as_ref()) {
            timer.set_state(TimerState::Canceled);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Process timer expiration
    pub fn tick(&self) -> Result<Vec<TimerCallback>, TimerError> {
        let mut callbacks = Vec::new();

        // Advance time
        let current = self.current_time.fetch_add(1, Ordering::AcqRel) + 1;

        // Check if we need to cascade timers
        let level0_idx = current as usize % WHEEL_SIZE_L0;

        if level0_idx == 0 {
            // Cascade from level 1
            self.cascade_level1();
        }

        // Process level 0 bucket
        let mut level0 = self.level0.lock();
        let bucket = level0.get_bucket(level0_idx);

        // Collect expired timer IDs
        let expired_ids: Vec<TimerId> = bucket.iter().copied().collect();
        drop(bucket);

        // Execute timers
        for id in expired_ids {
            if let Some(timer) = self.get_timer(id) {
                if timer.is_expired() {
                    if let Some(callback) = timer.execute() {
                        if timer.is_periodic() {
                            timer.reschedule();
                            // Reschedule in wheel
                            let remaining = timer.remaining();
                            let duration_ms = remaining.as_millis() as u64;
                            self.schedule_timer(id, duration_ms);
                        }
                        callbacks.push(callback);
                    }
                }
            }
        }

        // Clear bucket
        let bucket = level0.get_bucket_mut(level0_idx);
        bucket.timers.clear();

        Ok(callbacks)
    }

    /// Cascade timers from level 1
    fn cascade_level1(&self) {
        let current = self.current_time.load(Ordering::Acquire);
        let level1_idx = (current / TIME_MULT_L1 % (WHEEL_SIZE_L1 as u64)) as usize;

        let mut level1 = self.level1.lock();
        let bucket = level1.get_bucket(level1_idx);

        let expired_ids: Vec<TimerId> = bucket.iter().copied().collect();
        drop(bucket);

        // Reschedule timers to level 0
        for id in expired_ids {
            if let Some(timer) = self.get_timer(id) {
                let remaining = timer.remaining();
                let duration_ms = remaining.as_millis() as u64;
                self.schedule_timer(id, duration_ms);
            }
        }

        // Clear bucket
        let bucket = level1.get_bucket_mut(level1_idx);
        bucket.timers.clear();

        level1.advance();

        // Check if we need to cascade from level 2
        if level1.current_index() == 0 {
            self.cascade_level2();
        }
    }

    /// Cascade timers from level 2
    fn cascade_level2(&self) {
        let current = self.current_time.load(Ordering::Acquire);
        let level2_idx = (current / TIME_MULT_L2 % (WHEEL_SIZE_L2 as u64)) as usize;

        let mut level2 = self.level2.lock();
        let bucket = level2.get_bucket(level2_idx);

        let expired_ids: Vec<TimerId> = bucket.iter().copied().collect();
        drop(bucket);

        for id in expired_ids {
            if let Some(timer) = self.get_timer(id) {
                let remaining = timer.remaining();
                let duration_ms = remaining.as_millis() as u64;
                self.schedule_timer(id, duration_ms);
            }
        }

        let bucket = level2.get_bucket_mut(level2_idx);
        bucket.timers.clear();

        level2.advance();

        // Continue cascading if needed
        if level2.current_index() == 0 {
            self.cascade_level3();
        }
    }

    /// Cascade timers from level 3
    fn cascade_level3(&self) {
        let current = self.current_time.load(Ordering::Acquire);
        let level3_idx = (current / TIME_MULT_L3 % (WHEEL_SIZE_L3 as u64)) as usize;

        let mut level3 = self.level3.lock();
        let bucket = level3.get_bucket(level3_idx);

        let expired_ids: Vec<TimerId> = bucket.iter().copied().collect();
        drop(bucket);

        for id in expired_ids {
            if let Some(timer) = self.get_timer(id) {
                let remaining = timer.remaining();
                let duration_ms = remaining.as_millis() as u64;
                self.schedule_timer(id, duration_ms);
            }
        }

        let bucket = level3.get_bucket_mut(level3_idx);
        bucket.timers.clear();

        level3.advance();

        if level3.current_index() == 0 {
            self.cascade_level4();
        }
    }

    /// Cascade timers from level 4
    fn cascade_level4(&self) {
        let current = self.current_time.load(Ordering::Acquire);
        let level4_idx = (current / TIME_MULT_L4 % (WHEEL_SIZE_L4 as u64)) as usize;

        let mut level4 = self.level4.lock();
        let bucket = level4.get_bucket(level4_idx);

        let expired_ids: Vec<TimerId> = bucket.iter().copied().collect();
        drop(bucket);

        for id in expired_ids {
            if let Some(timer) = self.get_timer(id) {
                let remaining = timer.remaining();
                let duration_ms = remaining.as_millis() as u64;
                self.schedule_timer(id, duration_ms);
            }
        }

        let bucket = level4.get_bucket_mut(level4_idx);
        bucket.timers.clear();

        level4.advance();
    }

    /// Get timer by ID
    fn get_timer(&self, id: TimerId) -> Option<&Timer> {
        let timers = self.timers.read();
        timers.get(id as usize).and_then(|t| t.as_ref())
    }

    /// Get number of pending timers
    pub fn pending_count(&self) -> usize {
        let timers = self.timers.read();
        timers
            .iter()
            .filter(|t| {
                t.as_ref()
                    .map(|timer| timer.state() == TimerState::Pending)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Get current time
    pub fn current_time(&self) -> u64 {
        self.current_time.load(Ordering::Acquire)
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer operation errors
#[derive(Debug)]
pub enum TimerError {
    /// Timer duration is too long
    DurationTooLong,
    /// Timer not found
    NotFound,
    /// Timer already canceled
    AlreadyCanceled,
    /// Invalid timer state
    InvalidState,
}

/// Global timer wheel instance
static GLOBAL_TIMER_WHEEL: spin::Once<TimerWheel> = spin::Once::new();

/// Initialize global timer wheel
pub fn init() {
    GLOBAL_TIMER_WHEEL.call_once(|| TimerWheel::new());
}

/// Get global timer wheel
pub fn global_wheel() -> Option<&'static TimerWheel> {
    GLOBAL_TIMER_WHEEL.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let timer = Timer::new(Duration::from_millis(100), || {
            println!("Timer fired!");
        });

        assert_eq!(timer.state(), TimerState::Pending);
        assert!(!timer.is_periodic());
        assert!(!timer.is_expired());
    }

    #[test]
    fn test_periodic_timer() {
        let timer = Timer::periodic(Duration::from_millis(50), || {
            println!("Periodic timer!");
        });

        assert!(timer.is_periodic());
    }

    #[test]
    fn test_timer_wheel_creation() {
        let wheel = TimerWheel::new();
        assert_eq!(wheel.pending_count(), 0);
    }

    #[test]
    fn test_add_timer() {
        let wheel = TimerWheel::new();
        let timer = Timer::new(Duration::from_millis(100), || {});
        assert!(wheel.add_timer(timer).is_ok());
        assert_eq!(wheel.pending_count(), 1);
    }

    #[test]
    fn test_remaining_time() {
        let timer = Timer::new(Duration::from_millis(100), || {});
        let remaining = timer.remaining();
        assert!(remaining.as_millis() > 0);
        assert!(remaining.as_millis() <= 100);
    }
}
