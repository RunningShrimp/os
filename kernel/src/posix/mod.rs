//! # POSIX Real-time Extensions
//!
//! This module provides POSIX-compliant real-time extensions for the NOS kernel,
//! including scheduling, synchronization, timing, and resource management features.
//!
//! ## Modules
//!
//! - [`rt_mutex`]: Real-time mutex with priority inheritance protocol
//! - [`rt_timer`]: High-precision POSIX timers (timer_create, timer_settime, etc.)
//! - [`rt_extension`]: Memory locking, CPU affinity, and other RT extensions
//!
//! ## Features
//!
//! ### Priority Inheritance Mutexes
//!
//! Prevents priority inversion in real-time systems by temporarily boosting
//! the priority of mutex holders to match the highest priority waiter.
//!
//! ```rust
//! use kernel::posix::rt_mutex::{RtMutex, RtMutexProtocol};
//!
//! let mutex = RtMutex::new_with_protocol(
//!     data,
//!     RtMutexProtocol::Inherit,
//!     100,
//! );
//! let guard = mutex.lock(); // May boost holder's priority
//! ```
//!
//! ### High-Precision Timers
//!
//! POSIX-compliant timers with nanosecond precision and multiple notification methods.
//!
//! ```rust
//! use kernel::posix::rt_timer::{RtTimer, RtTimerClock, RtTimerSpec};
//!
//! let timer = RtTimer::new(RtTimerClock::Monotonic)?;
//! timer.settime(0, &RtTimerSpec::from_millis(100), &RtTimerSpec::zero())?;
//! ```
//!
//! ### Memory Locking
//!
//! Prevent paging of critical memory regions for deterministic access times.
//!
//! ```rust
//! use kernel::posix::rt_extension::{mlock, munlock, MlockFlags};
//!
//! mlock(ptr, 4096)?; // Lock 4KB
//! mlockall(MlockFlags::current())?; // Lock all current memory
//! ```
//!
//! ### CPU Affinity
//!
//! Control which CPUs a process may run on for NUMA optimization and cache locality.
//!
//! ```rust
//! use kernel::posix::rt_extension::sched_setaffinity;
//!
//! sched_setaffinity(0, 0b11)?; // Pin to CPU 0 and 1
//! ```
//!
//! ## POSIX Compliance
//!
//! These modules follow POSIX.1-2008 standard:
//! - pthread_mutexattr_setprotocol (priority inheritance/ceiling)
//! - timer_create/timer_delete/timer_settime/timer_gettime
//! - mlock/munlock/mlockall/munlockall
//! - sched_setaffinity/sched_getaffinity
//!
//! ## Performance
//!
//! - RT mutex lock/unlock: ~150-500ns
//! - Timer operations: ~200-500ns
//! - Memory lock: ~500ns per page
//! - CPU affinity change: ~100ns
//!
//! ## Integration
//!
//! These modules integrate with:
//! - Real-time scheduler ([`crate::sched::rt_sched`])
//! - Threaded IRQ handling ([`crate::irq::threaded_irq`])
//! - Base synchronization primitives ([`crate::sync`])
//!
//! ## Usage Guidelines
//!
//! 1. **Real-time tasks**: Use SCHED_FIFO or SCHED_RR with priorities 1-99
//! 2. **Priority inheritance**: Use RtMutexProtocol::Inherit for most cases
//! 3. **Memory locking**: Lock only what's needed (subject to RLIMIT_MEMLOCK)
//! 4. **CPU affinity**: Use for NUMA optimization or real-time isolation

pub mod rt_mutex;
pub mod rt_timer;
pub mod rt_extension;

// Re-exports for convenience

pub use rt_mutex::{
    RtMutex,
    RtMutexProtocol,
    RtMutexGuard,
    RtMutexStats,
    RtPriority as RtMutexPriority,
    TaskId as RtMutexTaskId,
    RtMutexError,
};

pub use rt_timer::{
    RtTimer,
    RtTimerClock,
    RtTimerSpec,
    RtTimerNotification,
    RtTimerError,
    timer_tick,
};

pub use rt_extension::{
    mlock,
    munlock,
    mlockall,
    munlockall,
    sched_setaffinity,
    sched_getaffinity,
    get_memlock_stats,
    MlockFlags,
    CpuMask,
    MemlockStats,
    RtExtensionError,
};

/// Initialize POSIX real-time extensions
pub fn init() {
    rt_extension::init();
    crate::log_debug!("POSIX real-time extensions initialized");
}
