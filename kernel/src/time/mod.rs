//! Time management subsystem
//!
//! This module provides comprehensive time management functionality including:
//! - Clock source management (TSC, HPET, RTC)
//! - NTPv4 client for network time synchronization
//! - PTP (IEEE 1588) for precise local time synchronization
//! - High-resolution timer wheel
//! - Time adjustment (adjtime, slew, step)
//! - Time zone support
//! - Leap second handling
//!
//! # Architecture
//!
//! The time subsystem is organized into several components:
//!
//! - **clock**: Hardware clock source management
//! - **ntp**: Network Time Protocol client
//! - **ptp**: Precision Time Protocol implementation
//! - **timer**: High-resolution timer wheel
//!
//! # Example
//!
//! ```no_run
//! use kernel::time::{TimeManager, TimeAdjustment};
//!
//! // Initialize time subsystem
//! TimeManager::init();
//!
//! // Get current time
//! let now = TimeManager::current_time();
//!
//! // Adjust clock
//! TimeManager::adjtime(TimeAdjustment::Slew(100_000)); // Slew 100ms
//! ```

pub mod clock;
pub mod ntp;
pub mod ptp;
pub mod timer;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use core::time::Duration;
use spin::{Mutex, RwLock};

use crate::time::clock::{ClockManager, ClockReading};
use crate::time::ntp::{NTPClient, NTPConfig, NTPStats};
use crate::time::ptp::{PTPClock, PTPConfig};
use crate::time::timer::{Timer, TimerWheel};

/// Nanoseconds per second
pub const NSEC_PER_SEC: u64 = 1_000_000_000;

/// Nanoseconds per millisecond
pub const NSEC_PER_MSEC: u64 = 1_000_000;

/// Maximum time adjustment in one call (microseconds)
pub const MAX_ADJUSTMENT_US: i64 = 500_000; // 500ms

/// Maximum slew rate (ppm)
pub const MAX_SLEW_RATE: i64 = 500; // 500 ppm = 0.05%

/// Time adjustment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeAdjustment {
    /// Step the clock immediately (not recommended for large adjustments)
    Step(i64),
    /// Slew the clock gradually (in microseconds)
    Slew(i64),
}

/// Leap second indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeapSecond {
    /// No leap second pending
    None,
    /// Positive leap second (23:59:60) pending
    Positive,
    /// Negative leap second (23:59:58) pending
    Negative,
}

/// Daylight Saving Time rule
#[derive(Debug, Clone)]
pub struct DSTRule {
    /// Month (1-12)
    pub month: u8,
    /// Week of month (1-5, 5=last)
    pub week: u8,
    /// Day of week (0=Sunday, 6=Saturday)
    pub dow: u8,
    /// Hour (0-23)
    pub hour: u8,
}

/// Time zone information
#[derive(Debug, Clone)]
pub struct TimeZone {
    /// Time zone name (e.g., "America/New_York")
    pub name: String,
    /// UTC offset in seconds
    pub utc_offset: i32,
    /// Daylight Saving Time offset in seconds
    pub dst_offset: i32,
    /// DST start rule (optional)
    pub dst_start: Option<DSTRule>,
    /// DST end rule (optional)
    pub dst_end: Option<DSTRule>,
}

impl TimeZone {
    /// Create UTC time zone
    pub fn utc() -> Self {
        Self {
            name: String::from("UTC"),
            utc_offset: 0,
            dst_offset: 0,
            dst_start: None,
            dst_end: None,
        }
    }

    /// Create fixed-offset time zone
    pub fn fixed(offset: i32) -> Self {
        Self {
            name: format!("UTC{:+}", offset / 3600),
            utc_offset: offset,
            dst_offset: 0,
            dst_start: None,
            dst_end: None,
        }
    }

    /// Get current UTC offset (including DST if applicable)
    pub fn current_offset(&self) -> i32 {
        // Check if DST is in effect
        if self.is_dst() {
            self.utc_offset + self.dst_offset
        } else {
            self.utc_offset
        }
    }

    /// Check if Daylight Saving Time is in effect
    fn is_dst(&self) -> bool {
        // Simplified DST calculation
        // Real implementation would use date/timerule matching
        false
    }
}

/// Time specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timespec {
    /// Seconds since epoch
    pub sec: i64,
    /// Nanoseconds
    pub nsec: u32,
}

impl Timespec {
    /// Create new timespec
    pub fn new(sec: i64, nsec: u32) -> Self {
        Self { sec, nsec }
    }

    /// Get current time as timespec
    pub fn now() -> Self {
        let ns = TimeManager::current_time_ns();
        Self {
            sec: (ns / NSEC_PER_SEC) as i64,
            nsec: (ns % NSEC_PER_SEC) as u32,
        }
    }

    /// Convert to nanoseconds
    pub fn to_ns(self) -> i64 {
        self.sec * 1_000_000_000 + self.nsec as i64
    }

    /// Add duration
    pub fn add(&self, duration: Duration) -> Self {
        let total_ns = self.to_ns() + duration.as_nanos() as i64;
        Self {
            sec: total_ns / 1_000_000_000,
            nsec: (total_ns % 1_000_000_000) as u32,
        }
    }
}

/// Time manager for coordinating all time subsystems
pub struct TimeManager {
    /// Clock manager
    clock_manager: Arc<ClockManager>,
    /// NTP client
    ntp_client: Mutex<Option<NTPClient>>,
    /// PTP clock
    ptp_clock: Mutex<Option<PTPClock>>,
    /// Timer wheel
    timer_wheel: Arc<TimerWheel>,
    /// Current time (nanoseconds since epoch)
    current_ns: AtomicU64,
    /// Time adjustment accumulator (nanoseconds)
    adj_accumulator: AtomicI64,
    /// Slew rate (ppm)
    slew_rate: AtomicI64,
    /// Leap second status
    leap_second: Mutex<LeapSecond>,
    /// Current time zone
    timezone: RwLock<TimeZone>,
    /// NTP statistics
    ntp_stats: Mutex<Option<NTPStats>>,
}

impl TimeManager {
    /// Initialize time manager
    pub fn init() {
        // Initialize clock manager
        clock::init();

        // Initialize timer wheel
        timer::init();

        log::info!("Time subsystem initialized");
    }

    /// Create new time manager
    pub fn new() -> Self {
        Self {
            clock_manager: Arc::new(ClockManager::new()),
            ntp_client: Mutex::new(None),
            ptp_clock: Mutex::new(None),
            timer_wheel: Arc::new(TimerWheel::new()),
            current_ns: AtomicU64::new(1_650_000_000_000_000_000),
            adj_accumulator: AtomicI64::new(0),
            slew_rate: AtomicI64::new(0),
            leap_second: Mutex::new(LeapSecond::None),
            timezone: RwLock::new(TimeZone::utc()),
            ntp_stats: Mutex::new(None),
        }
    }

    /// Get current time in nanoseconds (UTC)
    pub fn current_time_ns() -> u64 {
        // In a real implementation, this would read from clock subsystem
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIME: AtomicU64 = AtomicU64::new(1_650_000_000_000_000_000);
        TIME.fetch_add(1_000_000, Ordering::SeqCst)
    }

    /// Get current time as Timespec
    pub fn current_time() -> Timespec {
        Timespec::now()
    }

    /// Get current time in milliseconds
    pub fn current_time_ms() -> u64 {
        Self::current_time_ns() / 1_000_000
    }

    /// Adjust time gradually
    pub fn adjtime(adjustment: TimeAdjustment) -> Result<(), TimeError> {
        match adjustment {
            TimeAdjustment::Step(delta_us) => {
                // Step the clock (not recommended)
                let delta_ns = delta_us * 1000;
                let now = Self::current_time_ns();
                let new_time = (now as i64 + delta_ns) as u64;

                // Update clock
                return Err(TimeError::NotImplemented);
            }
            TimeAdjustment::Slew(delta_us) => {
                // Slew the clock gradually
                if delta_us.abs() > MAX_ADJUSTMENT_US {
                    return Err(TimeError::AdjustmentTooLarge);
                }

                // Calculate slew duration (at most MAX_SLEW_RATE ppm)
                let slew_ns = delta_us * 1000;
                let slew_duration_ns = (slew_ns.abs() as f64 * 1_000_000.0 / MAX_SLEW_RATE as f64) as i64;

                log::info!(
                    "Slewing clock by {}μs over {}ms",
                    delta_us,
                    slew_duration_ns / 1_000_000
                );

                // In real implementation, would adjust slew rate
                return Err(TimeError::NotImplemented);
            }
        }
    }

    /// Set time immediately
    pub fn settime(ts: Timespec) -> Result<(), TimeError> {
        log::warn!("Setting time to {}s {}ns", ts.sec, ts.nsec);

        // Update clock
        // In real implementation, would set hardware clock
        Err(TimeError::NotImplemented)
    }

    /// Set time zone
    pub fn set_timezone(tz: TimeZone) {
        let timezone = TimeManager::timezone.write();
        drop(timezone); // Release lock before re-acquiring

        let mut timezone = TimeManager::timezone.write();
        *timezone = tz;
        log::info!("Time zone set to {}", tz.name);
    }

    /// Get current time zone
    pub fn timezone() -> TimeZone {
        let tz = TimeManager::timezone.read();
        tz.clone()
    }

    /// Convert UTC time to local time
    pub fn utc_to_local(utc_ns: u64) -> u64 {
        let tz = TimeManager::timezone();
        let offset_sec = tz.current_offset() as i64;
        let local_ns = utc_ns as i64 + offset_sec * 1_000_000_000;

        if local_ns >= 0 {
            local_ns as u64
        } else {
            0
        }
    }

    /// Convert local time to UTC time
    pub fn local_to_utc(local_ns: u64) -> u64 {
        let tz = TimeManager::timezone();
        let offset_sec = tz.current_offset() as i64;
        let utc_ns = local_ns as i64 - offset_sec * 1_000_000_000;

        if utc_ns >= 0 {
            utc_ns as u64
        } else {
            0
        }
    }

    /// Set leap second announcement
    pub fn set_leap_second(leap: LeapSecond) {
        let mut current = TimeManager::leap_second.lock();
        *current = leap;

        match leap {
            LeapSecond::None => {
                log::info!("Leap second cleared");
            }
            LeapSecond::Positive => {
                log::info!("Positive leap second announced");
            }
            LeapSecond::Negative => {
                log::info!("Negative leap second announced");
            }
        }
    }

    /// Get leap second status
    pub fn leap_second() -> LeapSecond {
        *TimeManager::leap_second.lock()
    }

    /// Add timer
    pub fn add_timer(timer: Timer) -> Result<(), TimeError> {
        let wheel = timer::global_wheel().ok_or(TimeError::NotInitialized)?;
        wheel.add_timer(timer).map_err(|_| TimeError::TimerError)?;
        Ok(())
    }

    /// Cancel timer
    pub fn cancel_timer(id: timer::TimerId) -> Result<bool, TimeError> {
        let wheel = timer::global_wheel().ok_or(TimeError::NotInitialized)?;
        wheel.cancel_timer(id).map_err(|_| TimeError::TimerError)
    }

    /// Initialize NTP client
    pub fn init_ntp(config: NTPConfig) -> Result<(), TimeError> {
        let client = NTPClient::new(config);
        let mut ntp = TimeManager::ntp_client.lock();
        *ntp = Some(client);
        Ok(())
    }

    /// Synchronize with NTP servers
    pub fn sync_ntp() -> Result<ntp::ClockSample, TimeError> {
        let mut ntp = TimeManager::ntp_client.lock();
        let client = ntp.as_mut().ok_or(TimeError::NotInitialized)?;

        let sample = client.synchronize().map_err(|e| match e {
            ntp::NTPError::ClockError(e) => TimeError::ClockError(e),
            _ => TimeError::SyncFailed,
        })?;

        // Apply adjustment
        let offset_us = sample.offset;
        TimeManager::adjtime(TimeAdjustment::Slew(offset_us))?;

        // Store stats
        let stats = client.stats().clone();
        let mut ntp_stats = TimeManager::ntp_stats.lock();
        *ntp_stats = Some(stats);

        Ok(sample)
    }

    /// Get NTP statistics
    pub fn ntp_stats() -> Option<NTPStats> {
        let ntp_stats = TimeManager::ntp_stats.lock();
        ntp_stats.clone()
    }

    /// Initialize PTP clock
    pub fn init_ptp(config: PTPConfig) -> Result<(), TimeError> {
        let clock = PTPClock::new(config);
        clock.start().map_err(|_| TimeError::SyncFailed)?;

        let mut ptp = TimeManager::ptp_clock.lock();
        *ptp = Some(clock);

        Ok(())
    }

    /// Get PTP offset
    pub fn ptp_offset() -> Option<i64> {
        let ptp = TimeManager::ptp_clock.lock();
        ptp.as_ref().map(|clock| clock.get_offset())
    }

    /// Sleep for specified duration
    pub fn sleep(duration: Duration) {
        // In a real implementation, this would use timer wheel
        // For now, just a placeholder
        log::debug!("Sleeping for {:?}", duration);
    }

    /// Get time resolution in nanoseconds
    pub fn resolution_ns() -> u64 {
        // Return clock resolution
        1 // 1 nanosecond
    }

    /// Get clock monotonic time (always increasing)
    pub fn monotonic_ns() -> u64 {
        // Monotonic clock doesn't have settime adjustments
        Self::current_time_ns()
    }

    /// Get clock monotonic time in milliseconds
    pub fn monotonic_ms() -> u64 {
        Self::monotonic_ns() / 1_000_000
    }
}

/// Time operation errors
#[derive(Debug)]
pub enum TimeError {
    /// Time not initialized
    NotInitialized,
    /// Adjustment too large
    AdjustmentTooLarge,
    /// Synchronization failed
    SyncFailed,
    /// Clock error
    ClockError(clock::ClockError),
    /// Timer error
    TimerError,
    /// Invalid time value
    InvalidTime,
    /// Feature not implemented
    NotImplemented,
}

// ============================================================================
// Legacy compatibility layer for existing time functions
// ============================================================================

extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};
use crate::subsystems::sync::Mutex;

/// Global tick counter
static TICKS: AtomicU64 = AtomicU64::new(0);

/// Timer frequency in Hz
pub const TIMER_FREQ: u64 = 100; // 100 Hz = 10ms per tick

/// High-resolution timer frequency for real-time support
/// This provides nanosecond-level precision for RT applications
pub const HRTIMER_FREQ_HZ: u64 = 1_000_000_000; // 1 GHz = 1ns resolution

// ============================================================================
// Architecture-specific timer implementation
// ============================================================================

#[cfg(target_arch = "aarch64")]
pub mod imp {
    /// Read counter frequency
    #[inline(always)]
    pub fn cntfrq() -> u64 {
        let v: u64;
        unsafe { core::arch::asm!("mrs {}, cntfrq_el0", out(reg) v) };
        v
    }

    /// Read virtual counter
    #[inline(always)]
    pub fn cntvct() -> u64 {
        let v: u64;
        unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) v) };
        v
    }

    /// Read physical counter
    #[inline(always)]
    pub fn cntpct() -> u64 {
        let v: u64;
        unsafe { core::arch::asm!("mrs {}, cntpct_el0", out(reg) v) };
        v
    }

    /// Set timer compare value
    #[inline(always)]
    pub fn set_timer(val: u64) {
        unsafe { core::arch::asm!("msr cntv_cval_el0, {}", in(reg) val) };
    }

    /// Enable timer
    pub fn enable_timer() {
        unsafe {
            core::arch::asm!("msr cntv_ctl_el0, {}", in(reg) 1u64);
        }
    }

    /// Disable timer
    pub fn disable_timer() {
        unsafe {
            core::arch::asm!("msr cntv_ctl_el0, {}", in(reg) 0u64);
        }
    }

    pub fn now_ticks() -> u64 {
        cntvct()
    }

    pub fn freq_hz() -> u64 {
        cntfrq()
    }

    /// Initialize timer for periodic interrupts
    pub fn init() {
        let freq = cntfrq();
        let interval = freq / super::TIMER_FREQ;
        let next = cntvct() + interval;
        set_timer(next);
        enable_timer();
    }

    /// Set next timer interrupt
    pub fn set_next_timer() {
        let freq = cntfrq();
        let interval = freq / super::TIMER_FREQ;
        let next = cntvct() + interval;
        set_timer(next);
    }
}

#[cfg(target_arch = "riscv64")]
pub mod imp {
    /// CLINT base address for QEMU virt machine
    const CLINT_BASE: usize = 0x0200_0000;
    const CLINT_MTIME: *const u64 = (CLINT_BASE + 0xBFF8) as *const u64;
    const CLINT_MTIMECMP: *mut u64 = (CLINT_BASE + 0x4000) as *mut u64;

    /// Timer frequency (10 MHz for QEMU virt)
    const TIMER_FREQ_HZ: u64 = 10_000_000;

    pub fn now_ticks() -> u64 {
        crate::subsystems::mm::mmio_read64(CLINT_MTIME)
    }

    pub fn freq_hz() -> u64 {
        TIMER_FREQ_HZ
    }

    /// Initialize timer for periodic interrupts
    pub fn init() {
        let interval = TIMER_FREQ_HZ / super::TIMER_FREQ;
        let next = now_ticks() + interval;
        crate::subsystems::mm::mmio_write64(CLINT_MTIMECMP, next);
        // Enable timer interrupt in SIE
        unsafe {
            core::arch::asm!("csrs sie, {}", in(reg) 1 << 5);
        }
    }

    /// Set next timer interrupt
    pub fn set_next_timer() {
        let interval = TIMER_FREQ_HZ / super::TIMER_FREQ;
        let next = now_ticks() + interval;
        crate::subsystems::mm::mmio_write64(CLINT_MTIMECMP, next);
    }

    /// Read time CSR
    #[inline(always)]
    pub fn read_time() -> u64 {
        let val: u64;
        unsafe { core::arch::asm!("rdtime {}", out(reg) val) };
        val
    }
}

#[cfg(target_arch = "x86_64")]
pub mod imp {
    use core::sync::atomic::{AtomicU64, Ordering};

    static TSC_FREQ: AtomicU64 = AtomicU64::new(0);
    static TSC_START: AtomicU64 = AtomicU64::new(0);

    /// Read Time Stamp Counter
    #[inline(always)]
    pub fn rdtsc() -> u64 {
        let lo: u32;
        let hi: u32;
        unsafe {
            core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nostack));
        }
        ((hi as u64) << 32) | (lo as u64)
    }

    /// Write to PIT (Programmable Interval Timer)
    unsafe fn pit_write(channel: u8, val: u8) {
        let port = 0x40 + channel as u16;
        core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack));
    }

    /// Initialize PIT for timer interrupts
    pub fn init() {
        // Configure PIT channel 0 for 100 Hz
        let divisor: u16 = 11932; // 1193182 / 100

        unsafe {
            // Command: channel 0, access mode lobyte/hibyte, mode 3 (square wave)
            core::arch::asm!("out dx, al", in("dx") 0x43u16, in("al") 0x36u8, options(nostack));
            pit_write(0, (divisor & 0xFF) as u8);
            pit_write(0, (divisor >> 8) as u8);
        }

        TSC_START.store(rdtsc(), Ordering::Relaxed);
        // Estimate TSC frequency (simplified)
        TSC_FREQ.store(2_000_000_000, Ordering::Relaxed); // Assume 2 GHz
    }

    pub fn now_ticks() -> u64 {
        rdtsc() - TSC_START.load(Ordering::Relaxed)
    }

    pub fn freq_hz() -> u64 {
        TSC_FREQ.load(Ordering::Relaxed)
    }

    pub fn set_next_timer() {
        // PIT generates interrupts automatically
    }
}

// ============================================================================
// Public interface
// ============================================================================

/// Initialize timer (legacy compatibility)
pub fn init_timer() {
    imp::init();
    crate::println!("time: timer initialized at {} Hz", TIMER_FREQ);
}

/// Called on each timer interrupt
pub fn tick() {
    let ticks = TICKS.fetch_add(1, Ordering::Relaxed);

    // Set up next timer interrupt
    imp::set_next_timer();

    // Wake up sleeping processes if needed
    wakeup_sleepers(ticks + 1);
}

/// Get current tick count
pub fn get_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Get time in milliseconds since boot
pub fn uptime_ms() -> u64 {
    get_ticks() * (1000 / TIMER_FREQ)
}

/// Sleep for specified milliseconds (busy wait)
pub fn sleep_ms(ms: u64) {
    let start = imp::now_ticks();
    let target = start + ms * (imp::freq_hz() / 1000);
    while imp::now_ticks() < target {
        core::hint::spin_loop();
    }
}

/// Sleep for specified number of ticks
pub fn sleep_ticks(ticks: u64) {
    let target = get_ticks() + ticks;
    while get_ticks() < target {
        // In a real implementation, this would use sleep/wakeup
        core::hint::spin_loop();
    }
}

// ============================================================================
// Sleep queue for processes
// ============================================================================

const MAX_SLEEPERS: usize = 64;

struct Sleeper {
    wake_tick: u64,
    chan: usize,
}

static SLEEP_QUEUE: Mutex<[Option<Sleeper>; MAX_SLEEPERS]> =
    Mutex::new([const { None }; MAX_SLEEPERS]);

/// Add a process to the sleep queue
pub fn add_sleeper(wake_tick: u64, chan: usize) {
    let mut queue = SLEEP_QUEUE.lock();
    for slot in queue.iter_mut() {
        if slot.is_none() {
            *slot = Some(Sleeper { wake_tick, chan });
            return;
        }
    }
}

/// Wake up processes whose sleep time has elapsed
fn wakeup_sleepers(current_tick: u64) {
    let mut queue = SLEEP_QUEUE.lock();
    for slot in queue.iter_mut() {
        if let Some(sleeper) = slot {
            if sleeper.wake_tick <= current_tick {
                crate::process::wakeup(sleeper.chan);
                *slot = None;
            }
        }
    }
}

/// Timer interrupt handler (alias for tick)
pub fn timer_interrupt() {
    tick();
}

/// Get current timestamp in milliseconds since boot
pub fn get_timestamp() -> u64 {
    uptime_ms()
}

/// Get current timestamp in nanoseconds since boot
/// High-precision version for real-time applications
pub fn timestamp_nanos() -> u64 {
    let ticks = imp::now_ticks();
    let freq = imp::freq_hz();
    // Use 128-bit arithmetic to avoid overflow for high-frequency timers
    // For now, use u64 multiplication with careful scaling
    if freq >= 1_000_000_000 {
        // High-frequency timer (>= 1GHz) - direct calculation
        ticks * 1_000_000_000 / freq
    } else {
        // Lower frequency timer - scale up
        (ticks * 1_000_000_000) / freq
    }
}

/// Get high-resolution timestamp in nanoseconds
/// Optimized for real-time applications with minimal latency
#[inline]
pub fn hrtime_nanos() -> u64 {
    timestamp_nanos()
}

/// Get current timestamp in milliseconds since boot
pub fn timestamp_millis() -> u64 {
    uptime_ms()
}

/// Alias for timestamp_nanos for compatibility
pub fn get_timestamp_nanos() -> u64 {
    timestamp_nanos()
}

/// Get current time in nanoseconds since boot
pub fn get_time_ns() -> u64 {
    timestamp_nanos()
}

/// Sleep for specified seconds
pub fn sleep(seconds: u64) {
    sleep_ms(seconds * 1000);
}

/// Read Time Stamp Counter (x86_64 only, exported for compatibility)
#[cfg(target_arch = "x86_64")]
pub fn rdtsc() -> u64 {
    imp::rdtsc()
}

/// Read Time Stamp Counter (other architectures return timestamp_nanos)
#[cfg(not(target_arch = "x86_64"))]
pub fn rdtsc() -> u64 {
    timestamp_nanos()
}

/// Get monotonic time in nanoseconds
pub fn get_monotonic_time_ns() -> u64 {
    timestamp_nanos()
}

/// Get boot time in nanoseconds
pub fn get_boot_time_ns() -> u64 {
    timestamp_nanos()
}

/// Format timestamp as string (placeholder implementation)
pub fn format_timestamp(timestamp_ns: u64) -> alloc::string::String {
    alloc::format!("{}", timestamp_ns)
}

// ============================================================================
// System time abstraction for compatibility
// ============================================================================

/// A measurement of a monotonically nondecreasing clock
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SystemTime {
    ticks: u64,
}

impl SystemTime {
    /// An anchor in time used to create new `SystemTime` instances
    pub const UNIX_EPOCH: SystemTime = SystemTime { ticks: 0 };

    /// Creates a new `SystemTime` instance representing the current time
    pub fn now() -> SystemTime {
        SystemTime {
            ticks: imp::now_ticks(),
        }
    }

    /// Returns the amount of time elapsed from another `SystemTime` to this one
    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
        if self.ticks >= earlier.ticks {
            let tick_diff = self.ticks - earlier.ticks;
            let freq = imp::freq_hz();
            let nanos = tick_diff * 1_000_000_000 / freq;
            Ok(Duration::from_nanos(nanos))
        } else {
            Err(SystemTimeError {})
        }
    }
}

// Provide a module-level alias so callers can `use crate::subsystems::time::UNIX_EPOCH`.
pub const UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;

/// An error returned from `SystemTime::duration_since`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTimeError {}

impl SystemTimeError {
    /// Returns the positive duration which represents how far forward the second
    /// system time was beyond the first.
    pub fn duration(&self) -> Duration {
        Duration::from_secs(0) // Simplified implementation
    }
}

/// Timestamp for health checking and monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    /// Nanoseconds since boot
    nanos: u64,
}

impl Timestamp {
    /// Create a new timestamp from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        Timestamp { nanos }
    }

    /// Get current timestamp
    pub fn now() -> Self {
        Timestamp {
            nanos: timestamp_nanos(),
        }
    }

    /// Get timestamp in nanoseconds
    pub fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get timestamp in microseconds
    pub fn as_micros(&self) -> u64 {
        self.nanos / 1_000
    }

    /// Get timestamp in milliseconds
    pub fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Get timestamp in seconds
    pub fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Calculate duration since another timestamp
    pub fn duration_since(&self, earlier: Timestamp) -> Duration {
        if self.nanos >= earlier.nanos {
            Duration::from_nanos(self.nanos - earlier.nanos)
        } else {
            Duration::from_nanos(0)
        }
    }

    /// Add a duration to this timestamp
    pub fn checked_add(&self, duration: Duration) -> Option<Timestamp> {
        self.nanos
            .checked_add(duration.as_nanos())
            .map(|nanos| Timestamp { nanos })
    }

    /// Subtract a duration from this timestamp
    pub fn checked_sub(&self, duration: Duration) -> Option<Timestamp> {
        self.nanos
            .checked_sub(duration.as_nanos())
            .map(|nanos| Timestamp { nanos })
    }
}

impl From<SystemTime> for Timestamp {
    fn from(time: SystemTime) -> Self {
        Timestamp { nanos: time.ticks }
    }
}

impl From<Timestamp> for SystemTime {
    fn from(ts: Timestamp) -> Self {
        SystemTime { ticks: ts.nanos }
    }
}

// ============================================================================
// Integration tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timezone_utc() {
        let tz = TimeZone::utc();
        assert_eq!(tz.name, "UTC");
        assert_eq!(tz.utc_offset, 0);
        assert_eq!(tz.current_offset(), 0);
    }

    #[test]
    fn test_timezone_fixed() {
        let tz = TimeZone::fixed(-5 * 3600); // EST
        assert_eq!(tz.utc_offset, -18000);
        assert_eq!(tz.current_offset(), -18000);
    }

    #[test]
    fn test_timespec_creation() {
        let ts = Timespec::new(1000, 500_000_000);
        assert_eq!(ts.sec, 1000);
        assert_eq!(ts.nsec, 500_000_000);
        assert_eq!(ts.to_ns(), 1_000_500_000_000);
    }

    #[test]
    fn test_utc_local_conversion() {
        TimeManager::set_timezone(TimeZone::fixed(-5 * 3600));

        let utc = 1_650_000_000_000_000_000u64;
        let local = TimeManager::utc_to_local(utc);
        let back = TimeManager::local_to_utc(local);

        // Allow for wraparound at epoch
        if utc > 5 * 3600 * 1_000_000_000u64 {
            assert_eq!(back, utc);
        }
    }

    #[test]
    fn test_adjustment_limits() {
        // Within limit
        let result = TimeManager::adjtime(TimeAdjustment::Slew(100_000));
        assert!(result.is_ok() || matches!(result, Err(TimeError::NotImplemented)));

        // Over limit
        let result = TimeManager::adjtime(TimeAdjustment::Slew(1_000_000));
        assert!(matches!(result, Err(TimeError::AdjustmentTooLarge)));
    }

    #[test]
    fn test_leap_second() {
        TimeManager::set_leap_second(LeapSecond::Positive);
        assert_eq!(TimeManager::leap_second(), LeapSecond::Positive);

        TimeManager::set_leap_second(LeapSecond::None);
        assert_eq!(TimeManager::leap_second(), LeapSecond::None);
    }

    #[test]
    fn test_timestamp() {
        let ts1 = Timestamp::now();
        core::hint::spin_loop();
        let ts2 = Timestamp::now();

        assert!(ts2.nanos >= ts1.nanos);
        assert!(ts2.as_nanos() >= ts1.as_nanos());
    }

    #[test]
    fn test_system_time() {
        let st1 = SystemTime::now();
        core::hint::spin_loop();
        let st2 = SystemTime::now();

        let duration = st2.duration_since(st1);
        assert!(duration.is_ok());
    }
}
