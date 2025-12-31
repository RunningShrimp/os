//! RISC-V timer support
//!
//! This module provides per-CPU timer support for RISC-V systems,
//! including both S-mode and M-mode timers, timer interrupts, and
//! time counter management.
//!
//! # Features
//! - Per-CPU timer initialization
//! - Timer interrupt handling
//! - Time counter reading (mtime)
//! - Timer comparison (mtimecmp) management
//! - Support for both S-mode and M-mode timers
//! - High-resolution timekeeping
//!
//! # Performance Targets
//! - Timer interrupt latency: <5μs
//! - Timer read overhead: <100ns
//! - Timer set overhead: <100ns

use core::sync::atomic::{AtomicU64, Ordering};
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Timer frequency (typically 10 MHz on RISC-V systems)
pub const TIMER_FREQ: u64 = 10_000_000;

/// CLINT base address (platform-specific)
const CLINT_BASE: usize = 0x0200_0000;

/// mtime register offset
const MTIME_OFFSET: usize = 0xBFF8;

/// mtimecmp register offset for a hart
const MTIMECMP_OFFSET: usize = 0x4000;

/// Number of timer ticks per microsecond
pub const TICKS_PER_US: u64 = TIMER_FREQ / 1_000_000;

/// Number of timer ticks per millisecond
pub const TICKS_PER_MS: u64 = TIMER_FREQ / 1_000;

/// Number of timer ticks per second
pub const TICKS_PER_SEC: u64 = TIMER_FREQ;

/// Per-CPU timer state
pub struct TimerState {
    /// Hart ID
    hart_id: usize,
    /// Timer compare value
    mtimecmp: AtomicU64,
    /// Timer enabled
    enabled: AtomicU64,
    /// Timer interrupt count
    interrupt_count: AtomicU64,
    /// Next timer deadline
    next_deadline: AtomicU64,
}

impl TimerState {
    /// Create new timer state
    pub const fn new(hart_id: usize) -> Self {
        Self {
            hart_id,
            mtimecmp: AtomicU64::new(u64::MAX),
            enabled: AtomicU64::new(0),
            interrupt_count: AtomicU64::new(0),
            next_deadline: AtomicU64::new(u64::MAX),
        }
    }

    /// Check if timer is enabled
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) != 0
    }

    /// Enable timer
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Release);
    }

    /// Disable timer
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Release);
    }

    /// Get interrupt count
    pub fn interrupt_count(&self) -> u64 {
        self.interrupt_count.load(Ordering::Relaxed)
    }

    /// Increment interrupt count
    pub fn inc_interrupt_count(&self) {
        self.interrupt_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Global timer manager
pub struct TimerManager {
    /// Per-hart timer states
    timers: Vec<TimerState>,
}

impl TimerManager {
    /// Create new timer manager
    pub fn new(num_harts: usize) -> Self {
        let mut timers = Vec::with_capacity(num_harts);
        for hart_id in 0..num_harts {
            timers.push(TimerState::new(hart_id));
        }
        Self { timers }
    }

    /// Get timer state for hart
    pub fn timer(&self, hart_id: usize) -> Option<&TimerState> {
        self.timers.get(hart_id)
    }

    /// Initialize timer for hart
    pub fn init_hart(&self, hart_id: usize) -> Result<(), &'static str> {
        if hart_id >= self.timers.len() {
            return Err("Invalid hart ID");
        }

        let timer = &self.timers[hart_id];

        // Disable timer initially
        timer.disable();

        // Set mtimecmp to maximum (no timer interrupt)
        set_mtimecmp(hart_id, u64::MAX);

        // Enable timer interrupts
        unsafe {
            core::arch::asm!("csrsi sie, 0x20"); // STIE bit
        }

        timer.enable();

        Ok(())
    }

    /// Set timer for hart
    pub fn set_timer(&self, hart_id: usize, deadline: u64) -> Result<(), &'static str> {
        if hart_id >= self.timers.len() {
            return Err("Invalid hart ID");
        }

        let timer = &self.timers[hart_id];

        if !timer.enabled() {
            return Err("Timer not enabled");
        }

        // Set mtimecmp
        set_mtimecmp(hart_id, deadline);

        // Store deadline
        timer.next_deadline.store(deadline, Ordering::Release);

        Ok(())
    }

    /// Handle timer interrupt for hart
    pub fn handle_interrupt(&self, hart_id: usize) -> Result<(), &'static str> {
        if hart_id >= self.timers.len() {
            return Err("Invalid hart ID");
        }

        let timer = &self.timers[hart_id];

        // Increment interrupt count
        timer.inc_interrupt_count();

        // Disable timer by setting mtimecmp to max
        set_mtimecmp(hart_id, u64::MAX);

        Ok(())
    }
}

/// Global timer manager instance
static TIMER_MANAGER: SpinLock<Option<TimerManager>> = SpinLock::new(None);

/// Initialize timer subsystem
pub fn timer_init() -> Result<(), &'static str> {
    crate::println!("riscv64-timer: Initializing timer subsystem");

    let num_harts = super::smp::total_cpus();
    if num_harts == 0 {
        return Err("No CPUs detected");
    }

    let manager = TimerManager::new(num_harts);

    let mut global = TIMER_MANAGER.lock();
    *global = Some(manager);
    drop(global);

    // Initialize timer for boot hart
    let boot_hart = super::smp::get_cpu_id();
    timer_init_hart(boot_hart)?;

    crate::println!("riscv64-timer: Timer subsystem initialized with {} harts", num_harts);
    Ok(())
}

/// Initialize timer for current hart
pub fn timer_init_hart(hart_id: usize) -> Result<(), &'static str> {
    let global = TIMER_MANAGER.lock();
    let manager = global.as_ref().ok_or("Timer manager not initialized")?;
    manager.init_hart(hart_id)
}

/// Set timer for current hart
pub fn set_next_timer(deadline: u64) -> Result<(), &'static str> {
    let hart_id = super::smp::get_cpu_id();

    let global = TIMER_MANAGER.lock();
    let manager = global.as_ref().ok_or("Timer manager not initialized")?;
    manager.set_timer(hart_id, deadline)
}

/// Set timer with delay from current time
pub fn set_timer_delay_us(delay_us: u64) -> Result<(), &'static str> {
    let current = read_time_counter();
    let deadline = current + (delay_us * TICKS_PER_US);
    set_next_timer(deadline)
}

/// Set timer with delay in milliseconds
pub fn set_timer_delay_ms(delay_ms: u64) -> Result<(), &'static str> {
    let current = read_time_counter();
    let deadline = current + (delay_ms * TICKS_PER_MS);
    set_next_timer(deadline)
}

/// Read current time counter (mtime)
pub fn read_time_counter() -> u64 {
    let addr = CLINT_BASE + MTIME_OFFSET;
    unsafe { core::ptr::read_volatile(addr as *const u64) }
}

/// Set mtimecmp for a hart
pub fn set_mtimecmp(hart_id: usize, value: u64) {
    let addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id * 8);
    unsafe {
        core::ptr::write_volatile(addr as *mut u64, value);
    }
}

/// Read mtimecmp for a hart
pub fn read_mtimecmp(hart_id: usize) -> u64 {
    let addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id * 8);
    unsafe { core::ptr::read_volatile(addr as *const u64) }
}

/// Handle timer interrupt
pub fn handle_timer_interrupt() -> Result<(), &'static str> {
    let hart_id = super::smp::get_cpu_id();

    let global = TIMER_MANAGER.lock();
    let manager = global.as_ref().ok_or("Timer manager not initialized")?;
    manager.handle_interrupt(hart_id)
}

/// Enable timer interrupts for current hart
pub fn enable_timer_interrupts() {
    unsafe {
        core::arch::asm!("csrsi sie, 0x20"); // STIE bit
    }
}

/// Disable timer interrupts for current hart
pub fn disable_timer_interrupts() {
    unsafe {
        core::arch::asm!("csrci sie, 0x20");
    }
}

/// Get current time in microseconds
pub fn get_time_us() -> u64 {
    read_time_counter() / TICKS_PER_US
}

/// Get current time in milliseconds
pub fn get_time_ms() -> u64 {
    read_time_counter() / TICKS_PER_MS
}

/// Get current time in seconds
pub fn get_time_s() -> u64 {
    read_time_counter() / TICKS_PER_SEC
}

/// Delay for specified microseconds
pub fn udelay(us: u64) {
    let start = read_time_counter();
    let target = start + (us * TICKS_PER_US);

    while read_time_counter() < target {
        // Spin wait
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

/// Delay for specified milliseconds
pub fn mdelay(ms: u64) {
    udelay(ms * 1000)
}

/// Delay for specified seconds
pub fn sdelay(s: u64) {
    mdelay(s * 1000)
}

/// Machine mode timer support
#[cfg(feature = "machine_mode")]
pub mod machine {
    use super::*;

    /// Initialize machine mode timer
    pub fn timer_init_m() -> Result<(), &'static str> {
        crate::println!("riscv64-timer: Initializing M-mode timer");

        // Disable timer initially
        set_mtimecmp_m(super::smp::get_cpu_id(), u64::MAX);

        // Enable timer interrupts
        unsafe {
            core::arch::asm!("csrsi mie, 0x80"); // MTIE bit
        }

        Ok(())
    }

    /// Set mtimecmp for machine mode
    pub fn set_mtimecmp_m(hart_id: usize, value: u64) {
        let addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id * 8);
        unsafe {
            core::ptr::write_volatile(addr as *mut u64, value);
        }
    }

    /// Read mtimecmp for machine mode
    pub fn read_mtimecmp_m(hart_id: usize) -> u64 {
        let addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id * 8);
        unsafe { core::ptr::read_volatile(addr as *const u64) }
    }

    /// Enable machine mode timer interrupts
    pub fn enable_timer_interrupts_m() {
        unsafe {
            core::arch::asm!("csrsi mie, 0x80"); // MTIE bit
        }
    }

    /// Disable machine mode timer interrupts
    pub fn disable_timer_interrupts_m() {
        unsafe {
            core::arch::asm!("csrci mie, 0x80");
        }
    }
}

/// High-resolution time counter support
pub mod hrtc {
    use super::*;

    /// Get high-resolution time counter
    #[inline]
    pub fn get_hrtc() -> u64 {
        super::read_time_counter()
    }

    /// Convert time counter to nanoseconds
    pub fn to_ns(ticks: u64) -> u64 {
        const NS_PER_SEC: u64 = 1_000_000_000;
        ((ticks as u128) * (NS_PER_SEC as u128) / (TIMER_FREQ as u128)) as u64
    }

    /// Convert nanoseconds to time counter ticks
    pub fn from_ns(ns: u64) -> u64 {
        const NS_PER_SEC: u64 = 1_000_000_000;
        ((ns as u128) * (TIMER_FREQ as u128) / (NS_PER_SEC as u128)) as u64
    }

    /// Get high-resolution time in nanoseconds
    pub fn get_time_ns() -> u64 {
        to_ns(get_hrtc())
    }
}

/// Timer calibration support
pub mod calibration {
    use super::*;

    /// Calibrate timer frequency
    pub fn calibrate_timer_frequency() -> u64 {
        // Use a known external time source to calibrate
        // For now, assume 10 MHz (common on RISC-V systems)

        // In production, this would:
        // 1. Use a known external time source (e.g., RTC)
        // 2. Measure timer ticks over a known period
        // 3. Calculate the actual frequency

        TIMER_FREQ
    }

    /// Measure timer overhead
    pub fn measure_timer_overhead() -> u64 {
        const ITERATIONS: u64 = 1000;
        let mut total = 0u64;

        for _ in 0..ITERATIONS {
            let start = read_time_counter();
            unsafe {
                core::arch::asm!("nop");
            }
            let end = read_time_counter();
            total += end - start;
        }

        total / ITERATIONS
    }
}

/// Statistics and profiling
pub mod stats {
    use super::*;

    /// Timer statistics per hart
    #[derive(Default)]
    pub struct TimerStats {
        /// Total timer interrupts
        pub total_interrupts: AtomicU64,
        /// Timer overflows
        pub overflows: AtomicU64,
        /// Average timer interval
        pub avg_interval: AtomicU64,
        /// Maximum timer interval
        pub max_interval: AtomicU64,
        /// Minimum timer interval
        pub min_interval: AtomicU64,
    }

    /// Get timer statistics for a hart
    pub fn get_timer_stats(hart_id: usize) -> Option<&'static TimerStats> {
        // In production, this would return actual statistics
        // For now, return None
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_counter() {
        let time1 = read_time_counter();
        udelay(100); // Delay 100μs
        let time2 = read_time_counter();

        let elapsed = time2 - time1;
        let expected = 100 * TICKS_PER_US;

        // Allow 10% error
        assert!(elapsed >= expected - (expected / 10));
        assert!(elapsed <= expected + (expected / 10));
    }

    #[test]
    fn test_mtimecmp() {
        let hart_id = super::super::smp::get_cpu_id();

        set_mtimecmp(hart_id, 0x1234_5678_9ABC_DEF0);
        let value = read_mtimecmp(hart_id);

        assert_eq!(value, 0x1234_5678_9ABC_DEF0);
    }

    #[test]
    fn test_timer_functions() {
        // Test that timer functions don't crash
        let _us = get_time_us();
        let _ms = get_time_ms();
        let _s = get_time_s();

        udelay(10);
        mdelay(1);
    }

    #[test]
    fn test_hrtc() {
        use hrtc::*;

        let ticks = get_hrtc();
        let ns = to_ns(ticks);

        assert!(ns > 0);

        let ns_back = from_ns(ns);
        // Should be approximately equal
        let diff = if ticks > ns_back { ticks - ns_back } else { ns_back - ticks };
        assert!(diff < 1000); // Within 1000 ticks
    }
}
