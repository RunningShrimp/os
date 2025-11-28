//! Time-related system calls
//! Implements gettimeofday, clock_gettime, etc.

use crate::posix::{Timeval, Timespec, ClockId, CLOCK_REALTIME, CLOCK_MONOTONIC};
use super::E_OK;

/// Get the current time of day
pub fn sys_gettimeofday(tv: *mut Timeval, tz: *mut u8) -> isize {
    // TZ parameter is ignored as we don't support time zones yet
    let _ = tz;
    
    if tv.is_null() {
        return E_OK;
    }
    
    // Get current ticks and convert to timeval
    let ticks = crate::time::get_ticks();
    let sec = ticks / crate::time::TIMER_FREQ;
    let usec = (ticks % crate::time::TIMER_FREQ) * (1_000_000 / crate::time::TIMER_FREQ as u64);
    
    unsafe {
        *tv = Timeval {
            tv_sec: sec as i64,
            tv_usec: usec as i64,
        };
    }
    
    E_OK
}

/// Get the current time for a specific clock
pub fn sys_clock_gettime(clockid: ClockId, tp: *mut Timespec) -> isize {
    if tp.is_null() {
        return E_OK;
    }
    
    // Only support CLOCK_REALTIME and CLOCK_MONOTONIC for now
    if clockid != CLOCK_REALTIME && clockid != CLOCK_MONOTONIC {
        return super::E_INVAL;
    }
    
    // Get current ticks and convert to timespec
    let ticks = crate::time::get_ticks();
    let sec = ticks / crate::time::TIMER_FREQ;
    let nsec = (ticks % crate::time::TIMER_FREQ) * (1_000_000_000 / crate::time::TIMER_FREQ as u64);
    
    unsafe {
        *tp = Timespec {
            tv_sec: sec as i64,
            tv_nsec: nsec as i64,
        };
    }
    
    E_OK
}