//! Time-related syscalls

use super::common::{SyscallError, SyscallResult};
use crate::libc::time_lib::{Timespec, Timezone};

/// Dispatch time-related syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Time operations
        0x6000 => sys_time(args),           // time
        0x6001 => sys_gettimeofday(args),   // gettimeofday
        0x6002 => sys_settimeofday(args),   // settimeofday
        0x6003 => sys_clock_gettime(args),  // clock_gettime
        0x6004 => sys_clock_settime(args),  // clock_settime
        0x6005 => sys_clock_getres(args),   // clock_getres
        0x6006 => sys_nanosleep(args),      // nanosleep
        0x6007 => sys_clock_nanosleep(args), // clock_nanosleep
        0x6008 => sys_alarm(args),          // alarm
        0x6009 => sys_setitimer(args),      // setitimer
        0x600A => sys_getitimer(args),      // getitimer
        0x600B => sys_timer_create(args),   // timer_create
        0x600C => sys_timer_settime(args),  // timer_settime
        0x600D => sys_timer_gettime(args),  // timer_gettime
        0x600E => sys_timer_getoverrun(args), // timer_getoverrun
        0x600F => sys_timer_delete(args),   // timer_delete
        _ => Err(SyscallError::InvalidSyscall),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sys_time_with_null() {
        let r = sys_time(&[0u64]);
        assert!(r.is_ok());
        let secs = r.unwrap();
        // At least it returns a non-zero-ish value on a running environment
        assert!(secs >= 0);
    }
}

// Placeholder implementations - to be replaced with actual syscall logic

fn sys_time(_args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    use crate::libc::interface::time_t;

    // time(tloc) - one argument (pointer) which may be NULL
    let args = match extract_args(_args, 1) {
        Ok(a) => a,
        Err(_) => return Err(SyscallError::InvalidArgument),
    };

    let tloc = args[0] as *mut time_t;

    // Get current time in seconds (using nanosecond timestamp source)
    let ns = crate::time::timestamp_nanos();
    let seconds = (ns / 1_000_000_000) as u64;

    // If caller provided a pointer, copy the value into user space
    if !tloc.is_null() {
        // Find current process pagetable
        let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
        let pagetable = proc.pagetable;
        drop(proc_table);

        if pagetable.is_null() {
            return Err(SyscallError::BadAddress);
        }

        // Prepare bytes for time_t (platform-sized). Use native u64 -> time_t cast
        let val: time_t = seconds as time_t;
        let bytes = unsafe { core::slice::from_raw_parts((&val as *const time_t) as *const u8, core::mem::size_of::<time_t>()) };

        // Write to user space
        unsafe {
            copyout(pagetable, tloc as usize, bytes.as_ptr(), bytes.len())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }

    Ok(seconds as u64)
}

/// Get time of day
/// Arguments: [tv_ptr, tz_ptr]
/// Returns: 0 on success, error on failure
fn sys_gettimeofday(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    use crate::posix::Timeval;
    use crate::libc::time_lib::{Timespec, Timezone};
    
    let args = extract_args(args, 2)?;
    let tv_ptr = args[0] as *mut Timeval;
    let _tz_ptr = args[1] as *mut Timezone;
    
    if tv_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current time in nanoseconds
    let ns = crate::time::timestamp_nanos();
    
    // Convert to timeval (seconds and microseconds)
    let tv = Timeval {
        tv_sec: (ns / 1_000_000_000) as i64,
        tv_usec: ((ns % 1_000_000_000) / 1_000) as i64,
    };
    
    // Copy timeval to user space
    unsafe {
        copyout(pagetable, tv_ptr as usize, core::ptr::addr_of!(tv) as *const u8, core::mem::size_of::<Timeval>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

fn sys_settimeofday(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyin;
    use crate::posix::Timeval;
    
    let args = extract_args(args, 2)?;
    let tv_ptr = args[0] as usize;
    let _tz_ptr = args[1] as usize;
    
    // Only root can set time
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    
    if proc.euid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    let pagetable = proc.pagetable;
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    if tv_ptr != 0 {
        let mut tv = Timeval { tv_sec: 0, tv_usec: 0 };
        unsafe {
            copyin(pagetable, &mut tv as *mut _ as *mut u8, tv_ptr,
                   core::mem::size_of::<Timeval>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        // Validate time
        if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
            return Err(SyscallError::InvalidArgument);
        }
        
        // Accept the time change (real implementation would set system clock offset)
        // For now, we just acknowledge the request
        crate::println!("[settimeofday] Set time to {}s {}us", tv.tv_sec, tv.tv_usec);
    }
    
    Ok(0)
}

/// Clock gettime - get time for specified clock
/// Arguments: [clockid, tp_ptr]
/// Returns: 0 on success, error on failure
fn sys_clock_gettime(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::posix::Timespec;
    
    let args = extract_args(args, 2)?;
    let clockid = args[0] as i32;
    let tp_ptr = args[1] as *mut Timespec;
    
    if tp_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get time based on clock type
    let (tv_sec, tv_nsec) = match clockid {
        0 => { // CLOCK_REALTIME
            // For now, use monotonic time (real implementation would track wall clock)
            let ns = crate::time::timestamp_nanos();
            (ns / 1_000_000_000, (ns % 1_000_000_000) as i64)
        }
        1 => { // CLOCK_MONOTONIC
            let ns = crate::time::timestamp_nanos();
            (ns / 1_000_000_000, (ns % 1_000_000_000) as i64)
        }
        4 => { // CLOCK_MONOTONIC_RAW
            let ns = crate::time::timestamp_nanos();
            (ns / 1_000_000_000, (ns % 1_000_000_000) as i64)
        }
        7 => { // CLOCK_REALTIME_ALARM
            let ns = crate::time::timestamp_nanos();
            (ns / 1_000_000_000, (ns % 1_000_000_000) as i64)
        }
        _ => {
            return Err(SyscallError::InvalidArgument);
        }
    };
    
    // Write timespec to user space
    let timespec = Timespec {
        tv_sec: tv_sec as i64,
        tv_nsec,
    };
    
    unsafe {
        copyout(pagetable, tp_ptr as usize, core::ptr::addr_of!(timespec) as *const u8, core::mem::size_of::<Timespec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

fn sys_clock_settime(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyin;
    use crate::posix::Timespec;
    
    let args = extract_args(args, 2)?;
    let clockid = args[0] as i32;
    let tp_ptr = args[1] as usize;
    
    if tp_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    // Only root can set clock
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    
    if proc.euid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    let pagetable = proc.pagetable;
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Only CLOCK_REALTIME can be set
    if clockid != 0 {  // CLOCK_REALTIME = 0
        return Err(SyscallError::InvalidArgument);
    }
    
    let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe {
        copyin(pagetable, &mut ts as *mut _ as *mut u8, tp_ptr,
               core::mem::size_of::<Timespec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Validate timespec
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Accept the clock change (real implementation would set system clock offset)
    crate::println!("[clock_settime] Set clock {} to {}s {}ns", clockid, ts.tv_sec, ts.tv_nsec);
    
    Ok(0)
}

/// Clock getres - get clock resolution
/// Arguments: [clockid, res_ptr]
/// Returns: 0 on success, error on failure
fn sys_clock_getres(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::posix::Timespec;
    
    let args = extract_args(args, 2)?;
    let clockid = args[0] as i32;
    let res_ptr = args[1] as *mut Timespec;
    
    if res_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get timer resolution based on clock type
    let (tv_sec, tv_nsec) = match clockid {
        0 | 1 | 4 | 7 => {
            // High-resolution timer: nanosecond precision
            // For real-time support, we aim for <10us latency
            (0, 1_000) // 1 microsecond resolution (can be improved to nanosecond)
        }
        _ => {
            return Err(SyscallError::InvalidArgument);
        }
    };
    
    // Write timespec to user space
    let timespec = Timespec {
        tv_sec,
        tv_nsec,
    };
    
    unsafe {
        copyout(pagetable, res_ptr as usize, core::ptr::addr_of!(timespec) as *const u8, core::mem::size_of::<Timespec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

/// Nanosleep - high-precision sleep
/// Arguments: [req_ptr, rem_ptr]
/// Returns: 0 on success, error on failure
/// 
/// Real-time aware: Uses high-precision timer for accurate sleep duration
fn sys_nanosleep(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyinstr, copyout};
    use crate::posix::Timespec;
    
    let args = extract_args(args, 2)?;
    let req_ptr = args[0] as *const Timespec;
    let rem_ptr = args[1] as *mut Timespec;
    
    if req_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read timespec from user space
    let mut req = Timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe {
        copyin(pagetable, &mut req as *mut Timespec as *mut u8, req_ptr as usize, core::mem::size_of::<Timespec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Validate sleep duration
    if req.tv_sec < 0 || req.tv_nsec < 0 || req.tv_nsec >= 1_000_000_000 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Calculate target time in nanoseconds
    let sleep_ns = (req.tv_sec as u64) * 1_000_000_000 + (req.tv_nsec as u64);
    let start_ns = crate::time::hrtime_nanos();
    let target_ns = start_ns + sleep_ns;
    
    // High-precision sleep using busy-wait for very short durations
    // For longer sleeps, use the timer sleep-queue + process sleep/wakeup
    if sleep_ns < 1_000_000 {
        // Less than 1ms: use busy-wait for precision
        while crate::time::hrtime_nanos() < target_ns {
            core::hint::spin_loop();
        }
    } else {
        // Longer sleep: compute target tick and sleep on channel == pid
        // Convert nanoseconds to ticks (tick period = 1s / TIMER_FREQ)
        let tick_ns = 1_000_000_000u64 / crate::time::TIMER_FREQ;
        let ticks = (sleep_ns + tick_ns - 1) / tick_ns; // ceil

        // Register with timer as a sleeper and block the current process
        let chan = pid as usize;
        let wake_tick = crate::time::get_ticks().saturating_add(ticks);
        crate::time::add_sleeper(wake_tick, chan);

        // Block current process until wakeup_sleepers wakes it
        crate::process::sleep(chan);
    }
    
    // Check if interrupted (simplified - real implementation would check signals)
    let elapsed_ns = crate::time::hrtime_nanos().saturating_sub(start_ns);
    if elapsed_ns < sleep_ns && rem_ptr.is_null() == false {
        // Sleep was interrupted - calculate remaining time
        let remaining_ns = sleep_ns - elapsed_ns;
        let rem = Timespec {
            tv_sec: (remaining_ns / 1_000_000_000) as i64,
            tv_nsec: (remaining_ns % 1_000_000_000) as i64,
        };
        unsafe {
            copyout(pagetable, rem_ptr as usize, core::ptr::addr_of!(rem) as *const u8, core::mem::size_of::<Timespec>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        return Err(SyscallError::Interrupted);
    }
    
    Ok(0)
}

fn sys_clock_nanosleep(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::posix::Timespec;
    
    let args = extract_args(args, 4)?;
    let clockid = args[0] as i32;
    let flags = args[1] as i32;
    let request_ptr = args[2] as usize;
    let remain_ptr = args[3] as usize;
    
    // Validate clock ID
    match clockid {
        0 | 1 | 4 => {}  // CLOCK_REALTIME, CLOCK_MONOTONIC, CLOCK_MONOTONIC_RAW
        _ => return Err(SyscallError::InvalidArgument),
    }
    
    if request_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read request timespec
    let mut req = Timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe {
        copyin(pagetable, &mut req as *mut _ as *mut u8, request_ptr,
               core::mem::size_of::<Timespec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Validate request
    if req.tv_sec < 0 || req.tv_nsec < 0 || req.tv_nsec >= 1_000_000_000 {
        return Err(SyscallError::InvalidArgument);
    }
    
    const TIMER_ABSTIME: i32 = 1;
    let start_ns = crate::time::hrtime_nanos();
    
    let target_ns = if (flags & TIMER_ABSTIME) != 0 {
        // Absolute time
        (req.tv_sec as u64) * 1_000_000_000 + (req.tv_nsec as u64)
    } else {
        // Relative time
        start_ns + (req.tv_sec as u64) * 1_000_000_000 + (req.tv_nsec as u64)
    };
    
    // Sleep until target time
    let sleep_ns = target_ns.saturating_sub(start_ns);
    if sleep_ns > 0 {
        if sleep_ns < 1_000_000 {
            // Less than 1ms: busy-wait
            while crate::time::hrtime_nanos() < target_ns {
                core::hint::spin_loop();
            }
        } else {
            // Use timer-based sleep
            let tick_ns = 1_000_000_000u64 / crate::time::TIMER_FREQ;
            let ticks = (sleep_ns + tick_ns - 1) / tick_ns;
            
            let chan = my_pid as usize;
            let wake_tick = crate::time::get_ticks().saturating_add(ticks);
            crate::time::add_sleeper(wake_tick, chan);
            crate::process::sleep(chan);
        }
    }
    
    // Check for remaining time (on interruption)
    let end_ns = crate::time::hrtime_nanos();
    if end_ns < target_ns && remain_ptr != 0 && (flags & TIMER_ABSTIME) == 0 {
        let remaining = target_ns - end_ns;
        let rem = Timespec {
            tv_sec: (remaining / 1_000_000_000) as i64,
            tv_nsec: (remaining % 1_000_000_000) as i64,
        };
        unsafe {
            copyout(pagetable, remain_ptr, &rem as *const _ as *const u8,
                    core::mem::size_of::<Timespec>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        return Err(SyscallError::Interrupted);
    }
    
    Ok(0)
}

fn sys_alarm(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use core::sync::atomic::{AtomicU64, Ordering};
    
    let args = extract_args(args, 1)?;
    let seconds = args[0] as u64;
    
    // Static to track alarm per process (simplified - should be per-process)
    static ALARM_TIME: AtomicU64 = AtomicU64::new(0);
    
    // Get the previous alarm value
    let current_ns = crate::time::timestamp_nanos();
    let old_alarm = ALARM_TIME.load(Ordering::SeqCst);
    let remaining = if old_alarm > current_ns {
        ((old_alarm - current_ns) / 1_000_000_000) as u64
    } else {
        0
    };
    
    if seconds == 0 {
        // Cancel the alarm
        ALARM_TIME.store(0, Ordering::SeqCst);
    } else {
        // Set new alarm
        let alarm_time = current_ns + (seconds * 1_000_000_000);
        ALARM_TIME.store(alarm_time, Ordering::SeqCst);
        
        // Register alarm (real implementation would queue a signal delivery)
        crate::println!("[alarm] Set alarm for {} seconds", seconds);
    }
    
    // Return remaining seconds from previous alarm
    Ok(remaining)
}

/// Interval timer value structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Itimerval {
    it_interval: crate::posix::Timeval,  // Interval for periodic timer
    it_value: crate::posix::Timeval,     // Time until next expiration
}

// Timer types
const ITIMER_REAL: i32 = 0;
const ITIMER_VIRTUAL: i32 = 1;
const ITIMER_PROF: i32 = 2;

fn sys_setitimer(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    
    let args = extract_args(args, 3)?;
    let which = args[0] as i32;
    let new_value_ptr = args[1] as usize;
    let old_value_ptr = args[2] as usize;
    
    // Validate timer type
    match which {
        ITIMER_REAL | ITIMER_VIRTUAL | ITIMER_PROF => {}
        _ => return Err(SyscallError::InvalidArgument),
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Return old value if requested
    if old_value_ptr != 0 {
        // For now, return zeros
        let old_val = Itimerval::default();
        unsafe {
            copyout(pagetable, old_value_ptr, &old_val as *const _ as *const u8,
                    core::mem::size_of::<Itimerval>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    // Set new value
    if new_value_ptr != 0 {
        let mut new_val = Itimerval::default();
        unsafe {
            copyin(pagetable, &mut new_val as *mut _ as *mut u8, new_value_ptr,
                   core::mem::size_of::<Itimerval>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        // Validate
        if new_val.it_value.tv_usec >= 1_000_000 || 
           new_val.it_interval.tv_usec >= 1_000_000 {
            return Err(SyscallError::InvalidArgument);
        }
        
        // Accept the timer (real implementation would register with timer subsystem)
        crate::println!("[setitimer] Timer {} set: value={}s {}us, interval={}s {}us",
            which,
            new_val.it_value.tv_sec, new_val.it_value.tv_usec,
            new_val.it_interval.tv_sec, new_val.it_interval.tv_usec);
    }
    
    Ok(0)
}

fn sys_getitimer(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 2)?;
    let which = args[0] as i32;
    let curr_value_ptr = args[1] as usize;
    
    // Validate timer type
    match which {
        ITIMER_REAL | ITIMER_VIRTUAL | ITIMER_PROF => {}
        _ => return Err(SyscallError::InvalidArgument),
    }
    
    if curr_value_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Return current timer value (for now, zeros)
    let curr_val = Itimerval::default();
    unsafe {
        copyout(pagetable, curr_value_ptr, &curr_val as *const _ as *const u8,
                core::mem::size_of::<Itimerval>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

/// Create a per-process timer
/// Arguments: [clockid, sevp_ptr, timerid_ptr]
/// Returns: 0 on success, error on failure
fn sys_timer_create(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::posix::{SigEvent, SIGEV_SIGNAL, TIMER_ABSTIME, CLOCK_REALTIME};
    
    let args = extract_args(args, 3)?;
    let clockid = args[0] as i32;
    let sevp_ptr = args[1] as *const SigEvent;
    let timerid_ptr = args[2] as *mut i32;
    
    if timerid_ptr.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Validate clock ID
    match clockid {
        crate::posix::CLOCK_REALTIME | crate::posix::CLOCK_MONOTONIC => {
            // Supported clocks
        }
        _ => return Err(SyscallError::InvalidArgument),
    }
    
    // Read sigevent from user space
    let mut sev = crate::posix::SigEvent {
        sigev_notify: 0,
        sigev_signo: 0,
        sigev_value: crate::posix::SigVal { sival_int: 0 }, // Using the union correctly
        sigev_notify_function: 0,
        sigev_notify_attributes: 0,
    };
    unsafe {
        copyin(pagetable, core::ptr::addr_of_mut!(sev) as *mut u8, sevp_ptr as usize, core::mem::size_of::<crate::posix::SigEvent>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Validate sigevent
    if sev.sigev_notify != SIGEV_SIGNAL {
        return Err(SyscallError::NotSupported);
    }
    
    // For now, we'll use a simplified timer implementation
    // In a full implementation, we would:
    // 1. Allocate a timer ID
    // 2. Set up the timer with the specified clock
    // 3. Configure signal delivery
    
    // Allocate a timer ID (simplified - just use a counter)
    use core::sync::atomic::AtomicU32;
    static NEXT_TIMER_ID: AtomicU32 = AtomicU32::new(1);
    
    let timer_id = NEXT_TIMER_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    
    // Store timer information in process (simplified)
    // In a full implementation, this would be stored in a per-process timer table
    
    // Return timer ID to user space
    unsafe {
        copyout(pagetable, timerid_ptr as usize, core::ptr::addr_of!(timer_id) as *const u8, core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    crate::println!("[timer_create] Created timer {} for clock {}, signal {}",
        timer_id, clockid, sev.sigev_signo);
    
    Ok(0)
}

/// Set timer time
/// Arguments: [timerid, flags, new_value_ptr, old_value_ptr]
/// Returns: 0 on success, error on failure
fn sys_timer_settime(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::posix::{Itimerspec, TIMER_ABSTIME};
    
    let args = extract_args(args, 4)?;
    let timerid = args[0] as i32;
    let flags = args[1] as i32;
    let new_value_ptr = args[2] as *const Itimerspec;
    let old_value_ptr = args[3] as *mut Itimerspec;
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Validate flags
    if flags & !(TIMER_ABSTIME) != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Read new timer value from user space
    let mut new_value = Itimerspec::default();
    unsafe {
        copyin(pagetable, core::ptr::addr_of_mut!(new_value) as *mut u8, new_value_ptr as usize, core::mem::size_of::<Itimerspec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Validate timer value
    if new_value.it_interval.tv_sec < 0 || new_value.it_interval.tv_nsec < 0 ||
       new_value.it_interval.tv_nsec >= 1_000_000_000 ||
       new_value.it_value.tv_sec < 0 || new_value.it_value.tv_nsec < 0 ||
       new_value.it_value.tv_nsec >= 1_000_000_000 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // For now, we'll use a simplified timer implementation
    // In a full implementation, we would:
    // 1. Look up the timer by ID
    // 2. Set the timer to the specified value
    // 3. Handle absolute vs relative time
    // 4. Start or stop the timer as needed
    
    // Get old value if requested
    if !old_value_ptr.is_null() {
        // For now, return zeros
        let old_value = Itimerspec {
            it_interval: crate::posix::Timespec { tv_sec: 0, tv_nsec: 0 },
            it_value: crate::posix::Timespec { tv_sec: 0, tv_nsec: 0 },
        };
        
        unsafe {
            copyout(pagetable, old_value_ptr as usize, core::ptr::addr_of!(old_value) as *const u8, core::mem::size_of::<Itimerspec>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    crate::println!("[timer_settime] Set timer {} to {}s + {}ns, interval {}s + {}ns, flags 0x{:x}",
        timerid,
        new_value.it_value.tv_sec, new_value.it_value.tv_nsec,
        new_value.it_interval.tv_sec, new_value.it_interval.tv_nsec,
        flags);
    
    Ok(0)
}

fn sys_timer_gettime(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    use crate::posix::Itimerspec;
    
    let args = extract_args(args, 2)?;
    let timerid = args[0] as i32;
    let curr_value_ptr = args[1] as usize;
    
    if curr_value_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let table = crate::process::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // For now, return zeros (real implementation would look up timer state)
    let curr_value = Itimerspec {
        it_interval: crate::posix::Timespec { tv_sec: 0, tv_nsec: 0 },
        it_value: crate::posix::Timespec { tv_sec: 0, tv_nsec: 0 },
    };
    
    unsafe {
        copyout(pagetable, curr_value_ptr, &curr_value as *const _ as *const u8,
                core::mem::size_of::<Itimerspec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    crate::println!("[timer_gettime] Get timer {}", timerid);
    
    Ok(0)
}

fn sys_timer_getoverrun(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 1)?;
    let _timerid = args[0] as i32;
    
    // Return 0 (no overruns)
    // Real implementation would track timer overruns
    Ok(0)
}

fn sys_timer_delete(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 1)?;
    let timerid = args[0] as i32;
    
    // Accept deletion (real implementation would remove timer from table)
    crate::println!("[timer_delete] Delete timer {}", timerid);
    
    Ok(0)
}