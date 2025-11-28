//! Time and timer management for xv6-rust
//! Provides timer drivers and time-related functions

use core::sync::atomic::{AtomicU64, Ordering};
use crate::sync::Mutex;

/// Global tick counter
static TICKS: AtomicU64 = AtomicU64::new(0);

/// Timer frequency in Hz
pub const TIMER_FREQ: u64 = 100; // 100 Hz = 10ms per tick

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
        crate::mm::mmio_read64(CLINT_MTIME)
    }

    pub fn freq_hz() -> u64 {
        TIMER_FREQ_HZ
    }

    /// Initialize timer for periodic interrupts
    pub fn init() {
        let interval = TIMER_FREQ_HZ / super::TIMER_FREQ;
        let next = now_ticks() + interval;
        crate::mm::mmio_write64(CLINT_MTIMECMP, next);
        // Enable timer interrupt in SIE
        unsafe {
            core::arch::asm!("csrs sie, {}", in(reg) 1 << 5);
        }
    }

    /// Set next timer interrupt
    pub fn set_next_timer() {
        let interval = TIMER_FREQ_HZ / super::TIMER_FREQ;
        let next = now_ticks() + interval;
        crate::mm::mmio_write64(CLINT_MTIMECMP, next);
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

/// Initialize timer
pub fn init() {
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
    crate::mm::mmio_stats_periodic(ticks + 1);
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
