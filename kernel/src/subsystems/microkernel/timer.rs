//! Microkernel timer management
//!
//! Provides high-resolution timer support for the microkernel layer.
//! This includes periodic timers, one-shot timers, and timer callbacks.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use crate::subsystems::sync::Mutex;
use crate::reliability::errno::{EINVAL, ETIMEDOUT, EALREADY, ENOENT};

/// Timer types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    OneShot,    // Fires once and is removed
    Periodic,   // Fires repeatedly
    Deadline,   // Fires at a specific time
}

/// Timer states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Idle,       // Timer not scheduled
    Scheduled,  // Timer is scheduled to fire
    Expired,    // Timer has fired
    Cancelled,  // Timer was cancelled
}

/// Clock sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    Realtime,   // Wall-clock time (may be adjusted)
    Monotonic,  // Monotonically increasing clock
    Boottime,   // Time since system boot
    TAI,        // International Atomic Time
}

/// Timer callback function type
pub type TimerCallback = extern "C" fn(timer_id: u64, data: *mut u8);

/// High-resolution timer
#[derive(Debug)]
pub struct HighResolutionTimer {
    pub id: u64,
    pub timer_type: TimerType,
    pub clock_source: ClockSource,
    pub state: TimerState,
    pub interval_ns: u64,      // Interval for periodic timers
    pub expiry_time: u64,      // Time when timer should fire
    pub callback: TimerCallback,
    pub callback_data: *mut u8,
    pub creation_time: u64,
    pub fire_count: AtomicU64,
    pub next_fire_time: Option<u64>, // For periodic timers
}

impl Clone for HighResolutionTimer {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            timer_type: self.timer_type,
            clock_source: self.clock_source,
            state: self.state,
            interval_ns: self.interval_ns,
            expiry_time: self.expiry_time,
            callback: self.callback,
            callback_data: self.callback_data,
            creation_time: self.creation_time,
            fire_count: AtomicU64::new(self.fire_count.load(Ordering::SeqCst)),
            next_fire_time: self.next_fire_time,
        }
    }
}

impl HighResolutionTimer {
    pub fn new(id: u64, timer_type: TimerType, clock_source: ClockSource,
               callback: TimerCallback, data: *mut u8) -> Self {
        let current_time = get_current_time_ns(clock_source);

        Self {
            id,
            timer_type,
            clock_source,
            state: TimerState::Idle,
            interval_ns: 0,
            expiry_time: 0,
            callback,
            callback_data: data,
            creation_time: current_time,
            fire_count: AtomicU64::new(0),
            next_fire_time: None,
        }
    }

    pub fn set_one_shot(&mut self, delay_ns: u64) -> Result<(), i32> {
        if delay_ns == 0 {
            return Err(EINVAL);
        }

        self.timer_type = TimerType::OneShot;
        self.interval_ns = delay_ns;
        self.expiry_time = get_current_time_ns(self.clock_source) + delay_ns;
        self.next_fire_time = Some(self.expiry_time);
        self.state = TimerState::Scheduled;

        Ok(())
    }

    pub fn set_periodic(&mut self, interval_ns: u64, initial_delay_ns: Option<u64>) -> Result<(), i32> {
        if interval_ns == 0 {
            return Err(EINVAL);
        }

        self.timer_type = TimerType::Periodic;
        self.interval_ns = interval_ns;

        let current_time = get_current_time_ns(self.clock_source);
        let delay = initial_delay_ns.unwrap_or(interval_ns);

        self.expiry_time = current_time + delay;
        self.next_fire_time = Some(self.expiry_time);
        self.state = TimerState::Scheduled;

        Ok(())
    }

    pub fn set_deadline(&mut self, deadline_ns: u64) -> Result<(), i32> {
        let current_time = get_current_time_ns(self.clock_source);

        if deadline_ns <= current_time {
            return Err(EINVAL);
        }

        self.timer_type = TimerType::Deadline;
        self.interval_ns = deadline_ns - current_time;
        self.expiry_time = deadline_ns;
        self.next_fire_time = Some(self.expiry_time);
        self.state = TimerState::Scheduled;

        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), i32> {
        if self.state == TimerState::Idle {
            return Err(ENOENT);
        }

        self.state = TimerState::Cancelled;
        self.next_fire_time = None;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), i32> {
        match self.timer_type {
            TimerType::OneShot => {
                self.expiry_time = get_current_time_ns(self.clock_source) + self.interval_ns;
                self.next_fire_time = Some(self.expiry_time);
                self.state = TimerState::Scheduled;
                Ok(())
            }
            TimerType::Periodic => {
                let current_time = get_current_time_ns(self.clock_source);
                self.expiry_time = current_time + self.interval_ns;
                self.next_fire_time = Some(self.expiry_time);
                self.state = TimerState::Scheduled;
                Ok(())
            }
            TimerType::Deadline => {
                Err(EINVAL) // Deadlines can't be reset
            }
        }
    }

    pub fn fire(&mut self) -> bool {
        if self.state != TimerState::Scheduled {
            return false;
        }

        let current_time = get_current_time_ns(self.clock_source);
        if current_time < self.expiry_time {
            return false; // Not time yet
        }

        // Fire the timer
        self.fire_count.fetch_add(1, Ordering::SeqCst);

        // Call the callback
        unsafe {
            (self.callback)(self.id, self.callback_data);
        }

        match self.timer_type {
            TimerType::OneShot | TimerType::Deadline => {
                self.state = TimerState::Expired;
                self.next_fire_time = None;
                false // Don't keep in active list
            }
            TimerType::Periodic => {
                // Schedule next fire
                self.expiry_time += self.interval_ns;
                self.next_fire_time = Some(self.expiry_time);
                true // Keep in active list
            }
        }
    }

    pub fn time_until_fire(&self) -> Option<u64> {
        if self.state != TimerState::Scheduled {
            return None;
        }

        let current_time = get_current_time_ns(self.clock_source);
        if current_time >= self.expiry_time {
            Some(0)
        } else {
            Some(self.expiry_time - current_time)
        }
    }

    pub fn get_fire_count(&self) -> u64 {
        self.fire_count.load(Ordering::SeqCst)
    }

    pub fn is_expired(&self) -> bool {
        if self.state != TimerState::Scheduled {
            return false;
        }

        let current_time = get_current_time_ns(self.clock_source);
        current_time >= self.expiry_time
    }
}

unsafe impl Send for HighResolutionTimer {}
unsafe impl Sync for HighResolutionTimer {}

/// Periodic timer with lower overhead
#[derive(Debug)]
pub struct PeriodicTimer {
    pub id: u64,
    pub interval_ticks: u32,   // Interval in system ticks
    pub remaining_ticks: u32,  // Ticks until next fire
    pub callback: TimerCallback,
    pub callback_data: *mut u8,
    pub enabled: bool,
    pub fire_count: AtomicU64,
}

impl Clone for PeriodicTimer {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            interval_ticks: self.interval_ticks,
            remaining_ticks: self.remaining_ticks,
            callback: self.callback,
            callback_data: self.callback_data,
            enabled: self.enabled,
            fire_count: AtomicU64::new(self.fire_count.load(Ordering::SeqCst)),
        }
    }
}

impl PeriodicTimer {
    pub fn new(id: u64, interval_ticks: u32, callback: TimerCallback, data: *mut u8) -> Self {
        Self {
            id,
            interval_ticks,
            remaining_ticks: interval_ticks,
            callback,
            callback_data: data,
            enabled: false,
            fire_count: AtomicU64::new(0),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn tick(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        if self.remaining_ticks > 1 {
            self.remaining_ticks -= 1;
            false
        } else {
            // Time to fire
            self.remaining_ticks = self.interval_ticks;
            self.fire_count.fetch_add(1, Ordering::SeqCst);

            // Call callback
            unsafe {
                (self.callback)(self.id, self.callback_data);
            }

            true
        }
    }

    pub fn get_fire_count(&self) -> u64 {
        self.fire_count.load(Ordering::SeqCst)
    }
}

unsafe impl Send for PeriodicTimer {}
unsafe impl Sync for PeriodicTimer {}

/// Timer manager statistics
#[derive(Debug)]
pub struct TimerStats {
    pub total_timers: AtomicUsize,
    pub active_timers: AtomicUsize,
    pub expired_timers: AtomicUsize,
    pub cancelled_timers: AtomicUsize,
    pub total_fires: AtomicU64,
    pub timer_overruns: AtomicU64,
}

impl TimerStats {
    pub const fn new() -> Self {
        Self {
            total_timers: AtomicUsize::new(0),
            active_timers: AtomicUsize::new(0),
            expired_timers: AtomicUsize::new(0),
            cancelled_timers: AtomicUsize::new(0),
            total_fires: AtomicU64::new(0),
            timer_overruns: AtomicU64::new(0),
        }
    }

    pub fn increment_total(&self) {
        self.total_timers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_active(&self) {
        self.active_timers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_active(&self) {
        self.active_timers.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn increment_expired(&self) {
        self.expired_timers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_cancelled(&self) {
        self.cancelled_timers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn add_fires(&self, count: u64) {
        self.total_fires.fetch_add(count, Ordering::SeqCst);
    }

    pub fn increment_overruns(&self) {
        self.timer_overruns.fetch_add(1, Ordering::SeqCst);
    }
}

/// Microkernel timer manager
pub struct MicroTimerManager {
    pub hrtimers: Mutex<BTreeMap<u64, HighResolutionTimer>>,
    pub periodic_timers: Mutex<BTreeMap<u64, PeriodicTimer>>,
    pub stats: TimerStats,
    pub next_timer_id: AtomicU64,
    pub tick_count: AtomicU64,
    pub last_check_time: AtomicU64,
}

impl MicroTimerManager {
    pub fn new() -> Self {
        Self {
            hrtimers: Mutex::new(BTreeMap::new()),
            periodic_timers: Mutex::new(BTreeMap::new()),
            stats: TimerStats::new(),
            next_timer_id: AtomicU64::new(1),
            tick_count: AtomicU64::new(0),
            last_check_time: AtomicU64::new(0),
        }
    }

    pub fn create_hrtimer(&self, timer_type: TimerType, clock_source: ClockSource,
                         callback: TimerCallback, data: *mut u8) -> Result<u64, i32> {
        let id = self.next_timer_id.fetch_add(1, Ordering::SeqCst);
        let timer = HighResolutionTimer::new(id, timer_type, clock_source, callback, data);

        let mut timers = self.hrtimers.lock();
        timers.insert(id, timer);

        self.stats.increment_total();
        Ok(id)
    }

    pub fn destroy_hrtimer(&self, timer_id: u64) -> Result<(), i32> {
        let mut timers = self.hrtimers.lock();

        if let Some(timer) = timers.remove(&timer_id) {
            if timer.state == TimerState::Scheduled {
                self.stats.decrement_active();
            } else if timer.state == TimerState::Cancelled {
                self.stats.increment_cancelled();
            } else if timer.state == TimerState::Expired {
                self.stats.increment_expired();
            }
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    pub fn start_hrtimer_one_shot(&self, timer_id: u64, delay_ns: u64) -> Result<(), i32> {
        let mut timers = self.hrtimers.lock();

        let timer = timers.get_mut(&timer_id).ok_or(ENOENT)?;

        if timer.state == TimerState::Scheduled {
            return Err(EALREADY);
        }

        timer.set_one_shot(delay_ns)?;
        self.stats.increment_active();
        Ok(())
    }

    pub fn start_hrtimer_periodic(&self, timer_id: u64, interval_ns: u64,
                                 initial_delay_ns: Option<u64>) -> Result<(), i32> {
        let mut timers = self.hrtimers.lock();

        let timer = timers.get_mut(&timer_id).ok_or(ENOENT)?;

        if timer.state == TimerState::Scheduled {
            return Err(EALREADY);
        }

        timer.set_periodic(interval_ns, initial_delay_ns)?;
        self.stats.increment_active();
        Ok(())
    }

    pub fn start_hrtimer_deadline(&self, timer_id: u64, deadline_ns: u64) -> Result<(), i32> {
        let mut timers = self.hrtimers.lock();

        let timer = timers.get_mut(&timer_id).ok_or(ENOENT)?;

        if timer.state == TimerState::Scheduled {
            return Err(EALREADY);
        }

        timer.set_deadline(deadline_ns)?;
        self.stats.increment_active();
        Ok(())
    }

    pub fn stop_hrtimer(&self, timer_id: u64) -> Result<(), i32> {
        let mut timers = self.hrtimers.lock();

        let timer = timers.get_mut(&timer_id).ok_or(ENOENT)?;

        if timer.state == TimerState::Scheduled {
            timer.cancel()?;
            self.stats.decrement_active();
            self.stats.increment_cancelled();
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    pub fn create_periodic_timer(&self, interval_ticks: u32,
                                callback: TimerCallback, data: *mut u8) -> Result<u64, i32> {
        let id = self.next_timer_id.fetch_add(1, Ordering::SeqCst);
        let timer = PeriodicTimer::new(id, interval_ticks, callback, data);

        let mut timers = self.periodic_timers.lock();
        timers.insert(id, timer);

        self.stats.increment_total();
        Ok(id)
    }

    pub fn destroy_periodic_timer(&self, timer_id: u64) -> Result<(), i32> {
        let mut timers = self.periodic_timers.lock();
        timers.remove(&timer_id).ok_or(ENOENT)?;
        Ok(())
    }

    pub fn enable_periodic_timer(&self, timer_id: u64) -> Result<(), i32> {
        let mut timers = self.periodic_timers.lock();

        let timer = timers.get_mut(&timer_id).ok_or(ENOENT)?;
        timer.enable();
        Ok(())
    }

    pub fn disable_periodic_timer(&self, timer_id: u64) -> Result<(), i32> {
        let mut timers = self.periodic_timers.lock();

        let timer = timers.get_mut(&timer_id).ok_or(ENOENT)?;
        timer.disable();
        Ok(())
    }

    pub fn process_hrtimers(&self) -> Vec<u64> {
        let mut timers = self.hrtimers.lock();
        let mut expired_timers = Vec::new();
        let mut total_fires = 0u64;

        // Find expired timers
        for (id, timer) in timers.iter_mut() {
            if timer.is_expired() {
                if timer.fire() {
                    // Timer is still active (periodic)
                    expired_timers.push(*id);
                    total_fires += 1;
                } else {
                    // Timer expired or cancelled
                    self.stats.decrement_active();
                    if timer.state == TimerState::Expired {
                        self.stats.increment_expired();
                    }
                }
            }
        }

        self.stats.add_fires(total_fires);
        expired_timers
    }

    pub fn process_periodic_timers(&self) {
        let mut timers = self.periodic_timers.lock();
        let mut total_fires = 0u64;

        for timer in timers.values_mut() {
            if timer.tick() {
                total_fires += 1;
            }
        }

        self.stats.add_fires(total_fires);
    }

    pub fn tick(&self) {
        self.tick_count.fetch_add(1, Ordering::SeqCst);

        // Process high-resolution timers
        self.process_hrtimers();

        // Process periodic timers
        self.process_periodic_timers();
    }

    pub fn get_next_hrtimer_time(&self) -> Option<u64> {
        let timers = self.hrtimers.lock();

        let mut next_time = None;

        for timer in timers.values() {
            if timer.state == TimerState::Scheduled {
                if let Some(time_until) = timer.time_until_fire() {
                    match next_time {
                        None => next_time = Some(time_until),
                        Some(current) if time_until < current => next_time = Some(time_until),
                        _ => {}
                    }
                }
            }
        }

        next_time
    }

    pub fn get_timer_info(&self, timer_id: u64) -> Option<TimerInfo> {
        // Check high-resolution timers
        {
            let timers = self.hrtimers.lock();
            if let Some(timer) = timers.get(&timer_id) {
                return Some(TimerInfo::HighResolution(timer.clone()));
            }
        }

        // Check periodic timers
        {
            let timers = self.periodic_timers.lock();
            if let Some(timer) = timers.get(&timer_id) {
                return Some(TimerInfo::Periodic(timer.clone()));
            }
        }

        None
    }

    pub fn get_stats(&self) -> TimerStats {
        TimerStats {
            total_timers: AtomicUsize::new(self.stats.total_timers.load(Ordering::SeqCst)),
            active_timers: AtomicUsize::new(self.stats.active_timers.load(Ordering::SeqCst)),
            expired_timers: AtomicUsize::new(self.stats.expired_timers.load(Ordering::SeqCst)),
            cancelled_timers: AtomicUsize::new(self.stats.cancelled_timers.load(Ordering::SeqCst)),
            total_fires: AtomicU64::new(self.stats.total_fires.load(Ordering::SeqCst)),
            timer_overruns: AtomicU64::new(self.stats.timer_overruns.load(Ordering::SeqCst)),
        }
    }

    pub fn get_tick_count(&self) -> u64 {
        self.tick_count.load(Ordering::SeqCst)
    }
}

/// Timer information for queries
#[derive(Debug, Clone)]
pub enum TimerInfo {
    HighResolution(HighResolutionTimer),
    Periodic(PeriodicTimer),
}

/// Global timer manager
static mut GLOBAL_TIMER_MANAGER: Option<MicroTimerManager> = None;
static TIMER_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize timer subsystem
pub fn init() -> Result<(), i32> {
    if TIMER_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    let manager = MicroTimerManager::new();

    unsafe {
        GLOBAL_TIMER_MANAGER = Some(manager);
    }

    TIMER_INIT.store(true, Ordering::SeqCst);
    Ok(())
}

/// Get global timer manager
pub fn get_timer_manager() -> Option<&'static MicroTimerManager> {
    unsafe {
        GLOBAL_TIMER_MANAGER.as_ref()
    }
}

/// Get mutable global timer manager
pub fn get_timer_manager_mut() -> Option<&'static mut MicroTimerManager> {
    unsafe {
        GLOBAL_TIMER_MANAGER.as_mut()
    }
}

/// Timer interrupt handler (called from interrupt system)
extern "C" fn timer_interrupt_handler() {
    if let Some(manager) = get_timer_manager() {
        manager.tick();
    }
}

/// Get current time based on clock source
fn get_current_time_ns(clock_source: ClockSource) -> u64 {
    match clock_source {
        ClockSource::Realtime => crate::subsystems::time::get_time_ns(),
        ClockSource::Monotonic => crate::subsystems::time::get_monotonic_time_ns(),
        ClockSource::Boottime => crate::subsystems::time::get_boot_time_ns(),
        ClockSource::TAI => crate::subsystems::time::get_time_ns(), // Fallback to realtime for now
    }
}

/// Sleep for specified nanoseconds (high-resolution sleep)
pub fn sleep_ns(duration_ns: u64) {
    extern "C" fn sleep_callback(_timer_id: u64, _data: *mut u8) {
        // Timer completion - wake up sleeping thread
    }

    if let Some(manager) = get_timer_manager() {
        // Create a one-shot timer
        let timer_id = manager.create_hrtimer(
            TimerType::OneShot,
            ClockSource::Monotonic,
            sleep_callback,
            core::ptr::null_mut()
        ).unwrap_or(0);

        if timer_id != 0 {
            let _ = manager.start_hrtimer_one_shot(timer_id, duration_ns);

            // Wait for timer to expire
            while let Some(info) = manager.get_timer_info(timer_id) {
                match info {
                    TimerInfo::HighResolution(timer) => {
                        if timer.state != TimerState::Scheduled {
                            break;
                        }
                    }
                    _ => break,
                }
            }

            let _ = manager.destroy_hrtimer(timer_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_resolution_timer() {
        extern "C" fn test_callback(_timer_id: u64, _data: *mut u8) {}

        let mut timer = HighResolutionTimer::new(
            1,
            TimerType::OneShot,
            ClockSource::Monotonic,
            test_callback,
            core::ptr::null_mut()
        );

        assert_eq!(timer.state, TimerState::Idle);
        assert_eq!(timer.get_fire_count(), 0);

        assert_eq!(timer.set_one_shot(1_000_000), Ok(()));
        assert_eq!(timer.state, TimerState::Scheduled);
        assert!(timer.time_until_fire().is_some());

        assert_eq!(timer.cancel(), Ok(()));
        assert_eq!(timer.state, TimerState::Cancelled);
        assert!(timer.time_until_fire().is_none());
    }

    #[test]
    fn test_periodic_timer() {
        extern "C" fn test_callback(_timer_id: u64, _data: *mut u8) {}

        let mut timer = PeriodicTimer::new(
            1,
            10, // 10 ticks
            test_callback,
            core::ptr::null_mut()
        );

        assert!(!timer.enabled);
        assert_eq!(timer.get_fire_count(), 0);

        timer.enable();
        assert!(timer.enabled);

        // Simulate ticks
        for _ in 0..9 {
            assert!(!timer.tick());
        }

        // Should fire on 10th tick
        assert!(timer.tick());
        assert_eq!(timer.get_fire_count(), 1);
        assert_eq!(timer.remaining_ticks, 10);

        timer.disable();
        assert!(!timer.enabled);
    }

    #[test]
    fn test_timer_manager() {
        let manager = MicroTimerManager::new();

        extern "C" fn test_callback(_timer_id: u64, _data: *mut u8) {}

        let timer_id = manager.create_hrtimer(
            TimerType::OneShot,
            ClockSource::Monotonic,
            test_callback,
            core::ptr::null_mut()
        ).unwrap();

        assert!(timer_id > 0);

        let timer_info = manager.get_timer_info(timer_id);
        assert!(timer_info.is_some());

        assert_eq!(manager.start_hrtimer_one_shot(timer_id, 1_000_000), Ok(()));
        assert_eq!(manager.stop_hrtimer(timer_id), Ok(()));

        assert_eq!(manager.destroy_hrtimer(timer_id), Ok(()));
        assert_eq!(manager.get_timer_info(timer_id), None);
    }

    #[test]
    fn test_clock_source() {
        let time_real = get_current_time_ns(ClockSource::Realtime);
        let time_mono = get_current_time_ns(ClockSource::Monotonic);
        let time_boot = get_current_time_ns(ClockSource::Boottime);

        // All should be reasonable values
        assert!(time_real > 0);
        assert!(time_mono > 0);
        assert!(time_boot > 0);
    }
}