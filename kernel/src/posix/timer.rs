//! POSIX Timer Implementation
//!
//! Implements POSIX timers for per-process and per-thread timing
//! with various clock sources and notification methods.

extern crate alloc;

use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;
use crate::reliability::errno::{EOK, EINVAL, ENOENT, EPERM, EAGAIN};
use crate::posix::{TimerT, ClockId, SigEvent, Itimerspec, Timespec, Pid};

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq)]
enum TimerState {
    Disarmed,
    Armed,
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_remaining_positive() {
        let mut timer = Timer::new(1, crate::posix::CLOCK_MONOTONIC, SigEvent::default(), 1, None);

        // Set expiry a short time in the future
        let now_ns = crate::time::timestamp_nanos();
        let future_ns = now_ns + 2_000_000_000; // 2 seconds
        timer.expiry_time = Timespec { tv_sec: (future_ns / 1_000_000_000) as i64, tv_nsec: (future_ns % 1_000_000_000) as i64 };
        timer.state = TimerState::Armed;

        let rem = timer.get_remaining();
        // remaining should be > 0 and <= 2
        assert!(rem.tv_sec >= 0 && rem.tv_sec <= 2);
    }
}

/// Timer information
#[derive(Debug)]
struct Timer {
    /// Timer ID
    id: usize,
    /// Clock source
    clock_id: ClockId,
    /// Timer state
    state: TimerState,
    /// Timer expiration time
    expiry_time: Timespec,
    /// Timer interval (for periodic timers)
    interval: Timespec,
    /// Overrun count
    overrun_count: u32,
    /// Notification settings
    sigevent: SigEvent,
    /// Owner process
    owner_pid: Pid,
    /// Owner thread (for thread-specific timers)
    owner_tid: Option<usize>,
}

impl Timer {
    fn new(id: usize, clock_id: ClockId, sigevent: SigEvent, owner_pid: Pid, owner_tid: Option<usize>) -> Self {
        Self {
            id,
            clock_id,
            state: TimerState::Disarmed,
            expiry_time: Timespec::zero(),
            interval: Timespec::zero(),
            overrun_count: 0,
            sigevent,
            owner_pid,
            owner_tid,
        }
    }

    fn arm(&mut self, expiry: Timespec, interval: Timespec) -> Result<(), i32> {
        // Validate times
        if expiry.tv_sec < 0 || expiry.tv_nsec < 0 || expiry.tv_nsec >= 1_000_000_000 {
            return Err(EINVAL);
        }
        if interval.tv_sec < 0 || interval.tv_nsec < 0 || interval.tv_nsec >= 1_000_000_000 {
            return Err(EINVAL);
        }

        self.expiry_time = expiry;
        self.interval = interval;
        self.state = TimerState::Armed;
        self.overrun_count = 0;

        Ok(())
    }

    fn disarm(&mut self) {
        self.state = TimerState::Disarmed;
        self.overrun_count = 0;
    }

    fn get_remaining(&self) -> Timespec {
        if self.state != TimerState::Armed {
            return Timespec::zero();
        }

        // Calculate remaining time (expiry_time - now)
        // Use the system time source (ns) and convert to Timespec
        let now_ns = crate::time::timestamp_nanos();
        let now = Timespec {
            tv_sec: (now_ns / 1_000_000_000) as i64,
            tv_nsec: (now_ns % 1_000_000_000) as i64,
        };

        // Convert expiry_time and now into nanoseconds (i128 to avoid overflow)
        let expiry_ns = (self.expiry_time.tv_sec as i128) * 1_000_000_000i128 + (self.expiry_time.tv_nsec as i128);
        let now_ns_i = (now.tv_sec as i128) * 1_000_000_000i128 + (now.tv_nsec as i128);

        if now_ns_i >= expiry_ns {
            // Already expired
            Timespec::zero()
        } else {
            let rem_ns = (expiry_ns - now_ns_i) as u128; // safely positive
            Timespec {
                tv_sec: (rem_ns / 1_000_000_000u128) as i64,
                tv_nsec: (rem_ns % 1_000_000_000u128) as i64,
            }
        }
    }

    fn get_interval(&self) -> Timespec {
        self.interval
    }

    fn check_expiry(&mut self, current_time: Timespec) -> bool {
        if self.state != TimerState::Armed {
            return false;
        }

        // Check if timer has expired
        if current_time.tv_sec > self.expiry_time.tv_sec ||
           (current_time.tv_sec == self.expiry_time.tv_sec && current_time.tv_nsec >= self.expiry_time.tv_nsec) {

            // Handle expiration
            self.state = TimerState::Expired;
            self.overrun_count += 1;

            // Send notification
            self.send_notification();

            // Rearm for periodic timers
            if self.interval.tv_sec > 0 || self.interval.tv_nsec > 0 {
                // Calculate next expiry time
                self.expiry_time.tv_sec += self.interval.tv_sec;
                self.expiry_time.tv_nsec += self.interval.tv_nsec;

                // Handle nanosecond overflow
                if self.expiry_time.tv_nsec >= 1_000_000_000 {
                    self.expiry_time.tv_sec += 1;
                    self.expiry_time.tv_nsec -= 1_000_000_000;
                }

                self.state = TimerState::Armed;
            }

            true
        } else {
            false
        }
    }

    fn send_notification(&self) {
        match self.sigevent.sigev_notify {
            crate::posix::SIGEV_SIGNAL => {
                // Send signal to process
                unsafe {
                    let _ = crate::ipc::signal::kill(self.owner_pid as usize, self.sigevent.sigev_signo as u32);
                }
            }
            crate::posix::SIGEV_THREAD => {
                // Lightweight thread notification: in full implementation we would
                // create a proper kernel thread to run the provided notification
                // function. For now we log the desired behavior so the system
                // remains safe and observable.
                crate::println!("[timer] thread notification requested (SIGEV_THREAD) for timer={} owner_pid={} owner_tid={:?}", self.id, self.owner_pid, self.owner_tid);
            }
            _ => {
                // No notification
            }
        }
    }
}

/// Global timer registry
static TIMER_REGISTRY: Mutex<BTreeMap<usize, Arc<Mutex<Timer>>>> =
    Mutex::new(BTreeMap::new());

/// Next timer ID
static NEXT_TIMER_ID: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(1);

/// Maximum number of timers per process
const TIMER_MAX: usize = 32;

// ============================================================================
// Timer Functions
// ============================================================================

/// Create a new timer
///
/// # Arguments
/// * `clock_id` - Clock source
/// * `sevp` - Notification settings (null for default)
/// * `timer_id` - Pointer to store timer ID
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn timer_create(
    clock_id: ClockId,
    sevp: *const SigEvent,
    timer_id: *mut TimerT,
) -> i32 {
    if timer_id.is_null() {
        return EINVAL;
    }

    // Validate clock ID
    match clock_id {
        crate::posix::CLOCK_REALTIME |
        crate::posix::CLOCK_MONOTONIC |
        crate::posix::CLOCK_PROCESS_CPUTIME_ID |
        crate::posix::CLOCK_THREAD_CPUTIME_ID => {}
        _ => return EINVAL,
    }

    // Use default notification if none provided
    let sigevent = if sevp.is_null() {
        SigEvent {
            sigev_notify: crate::posix::SIGEV_SIGNAL,
            sigev_signo: crate::posix::SIGALRM,
            sigev_value: crate::posix::SigVal { sival_int: 0 },
            sigev_notify_function: 0,
            sigev_notify_attributes: 0,
        }
    } else {
        *sevp
    };

    // Get current process
    let current_pid = crate::process::getpid() as i32;
    let current_tid = if clock_id == crate::posix::CLOCK_THREAD_CPUTIME_ID {
        crate::process::thread::current_thread()
    } else {
        None
    };

    // Generate timer ID
    let id = NEXT_TIMER_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);

    // Create timer
    let timer = Timer::new(id, clock_id, sigevent, current_pid, current_tid);
    let timer = Arc::new(Mutex::new(timer));

    // Register timer
    let mut registry = TIMER_REGISTRY.lock();

    // Check timer limit per process
    let timer_count = registry.values()
        .filter(|t| t.lock().owner_pid == current_pid)
        .count();

    if timer_count >= TIMER_MAX {
        return EAGAIN;
    }

    registry.insert(id, timer);

    // Return timer ID as opaque pointer
    *timer_id = id as TimerT;
    EOK
}

/// Delete a timer
///
/// # Arguments
/// * `timer_id` - Timer to delete
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn timer_delete(timer_id: TimerT) -> i32 {
    if timer_id.is_null() {
        return EINVAL;
    }

    let id = timer_id as usize;
    let mut registry = TIMER_REGISTRY.lock();

    match registry.remove(&id) {
        Some(_) => EOK,
        None => ENOENT,
    }
}

/// Arm or disarm a timer
///
/// # Arguments
/// * `timer_id` - Timer to modify
/// * `flags` - Timer flags
/// * `new_value` - New timer settings
/// * `old_value` - Buffer to store old settings
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn timer_settime(
    timer_id: TimerT,
    flags: i32,
    new_value: *const Itimerspec,
    old_value: *mut Itimerspec,
) -> i32 {
    if timer_id.is_null() || new_value.is_null() {
        return EINVAL;
    }

    let id = timer_id as usize;
    let registry = TIMER_REGISTRY.lock();

    let timer = match registry.get(&id) {
        Some(t) => t.clone(),
        None => return ENOENT,
    };

    drop(registry);

    let mut timer_guard = timer.lock();

    // Store old value if requested
    if !old_value.is_null() {
        *old_value = Itimerspec {
            it_interval: timer_guard.get_interval(),
            it_value: timer_guard.get_remaining(),
        };
    }

    let new_spec = &*new_value;

    // Handle TIMER_ABSTIME flag
    let expiry = if (flags & crate::posix::TIMER_ABSTIME) != 0 {
        new_spec.it_value
    } else {
        // Convert relative time to absolute time
        // TODO: Add current time to relative time
        new_spec.it_value
    };

    // Set timer
    match timer_guard.arm(expiry, new_spec.it_interval) {
        Ok(()) => EOK,
        Err(e) => e,
    }
}

/// Get timer settings
///
/// # Arguments
/// * `timer_id` - Timer to query
/// * `curr_value` - Buffer to store current settings
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn timer_gettime(timer_id: TimerT, curr_value: *mut Itimerspec) -> i32 {
    if timer_id.is_null() || curr_value.is_null() {
        return EINVAL;
    }

    let id = timer_id as usize;
    let registry = TIMER_REGISTRY.lock();

    let timer = match registry.get(&id) {
        Some(t) => t.clone(),
        None => return ENOENT,
    };

    drop(registry);

    let timer_guard = timer.lock();

    *curr_value = Itimerspec {
        it_interval: timer_guard.get_interval(),
        it_value: timer_guard.get_remaining(),
    };

    EOK
}

/// Get timer overrun count
///
/// # Arguments
/// * `timer_id` - Timer to query
///
/// # Returns
/// * Overrun count on success, -1 on failure
pub unsafe extern "C" fn timer_getoverrun(timer_id: TimerT) -> i32 {
    if timer_id.is_null() {
        return -1;
    }

    let id = timer_id as usize;
    let registry = TIMER_REGISTRY.lock();

    let timer = match registry.get(&id) {
        Some(t) => t.clone(),
        None => return -1,
    };

    drop(registry);

    let timer_guard = timer.lock();
    timer_guard.overrun_count as i32
}

/// Get clock time
///
/// # Arguments
/// * `clock_id` - Clock to query
/// * `tp` - Buffer to store time
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn clock_gettime(clock_id: ClockId, tp: *mut Timespec) -> i32 {
    if tp.is_null() {
        return EINVAL;
    }

    let current_time = match clock_id {
        crate::posix::CLOCK_REALTIME => {
            // TODO: Get real-time clock
            Timespec::new(0, 0)
        }
        crate::posix::CLOCK_MONOTONIC => {
            // TODO: Get monotonic clock
            Timespec::new(0, 0)
        }
        crate::posix::CLOCK_PROCESS_CPUTIME_ID => {
            // TODO: Get process CPU time
            Timespec::new(0, 0)
        }
        crate::posix::CLOCK_THREAD_CPUTIME_ID => {
            // TODO: Get thread CPU time
            Timespec::new(0, 0)
        }
        _ => return EINVAL,
    };

    *tp = current_time;
    EOK
}

/// Set clock time (only for certain clocks)
///
/// # Arguments
/// * `clock_id` - Clock to set
/// * `tp` - New time
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn clock_settime(clock_id: ClockId, tp: *const Timespec) -> i32 {
    if tp.is_null() {
        return EINVAL;
    }

    if clock_id != crate::posix::CLOCK_REALTIME {
        return EPERM; // Only real-time clock can be set
    }

    // TODO: Implement setting real-time clock
    EPERM
}

/// Get clock resolution
///
/// # Arguments
/// * `clock_id` - Clock to query
/// * `res` - Buffer to store resolution
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn clock_getres(clock_id: ClockId, res: *mut Timespec) -> i32 {
    if res.is_null() {
        return EINVAL;
    }

    let resolution = match clock_id {
        crate::posix::CLOCK_REALTIME | crate::posix::CLOCK_MONOTONIC => {
            // Typically 1ms or better
            Timespec::new(0, 1_000_000) // 1ms
        }
        crate::posix::CLOCK_PROCESS_CPUTIME_ID | crate::posix::CLOCK_THREAD_CPUTIME_ID => {
            // CPU time clocks usually have nanosecond resolution
            Timespec::new(0, 1) // 1ns
        }
        _ => return EINVAL,
    };

    *res = resolution;
    EOK
}

/// Sleep until specified time
///
/// # Arguments
/// * `clock_id` - Clock to use
/// * `flags` - Sleep flags
/// * `request` - Wake time
/// * `remain` - Remaining time if interrupted
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn clock_nanosleep(
    clock_id: ClockId,
    flags: i32,
    request: *const Timespec,
    remain: *mut Timespec,
) -> i32 {
    if request.is_null() {
        return EINVAL;
    }

    let wake_time = &*request;

    // Validate time
    if wake_time.tv_sec < 0 || wake_time.tv_nsec < 0 || wake_time.tv_nsec >= 1_000_000_000 {
        return EINVAL;
    }

    // TODO: Implement actual sleep logic
    // This would involve checking the clock and sleeping until the specified time

    if (flags & crate::posix::TIMER_ABSTIME) != 0 {
        // Absolute time - sleep until wake_time
    } else {
        // Relative time - sleep for duration specified in wake_time
    }

    EOK
}

/// Process expired timers
///
/// This function should be called periodically to check for timer expirations
/// and deliver notifications.
pub fn process_timers() {
    let registry = TIMER_REGISTRY.lock();

    // Get current time for all clocks
    let current_time = Timespec::new(0, 0); // TODO: Get actual current time

    // Check all timers
    for timer in registry.values() {
        let mut timer_guard = timer.lock();
        timer_guard.check_expiry(current_time);
    }
}