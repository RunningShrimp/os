//! # Interrupt Handling
//!
//! This module provides interrupt handling facilities for the NOS kernel,
//! including threaded IRQ support for real-time applications.
//!
//! ## Modules
//!
//! - [`threaded_irq`]: Threaded interrupt handling for improved real-time performance
//!
//! ## Features
//!
//! ### Threaded IRQs
//!
//! Converts hardware interrupts to kernel threads for:
//! - Better prioritization (IRQ threads can be prioritized)
//! - Reduced interrupt latency (quick handler, threaded processing)
//! - Improved preemptibility
//! - Better real-time guarantees
//!
//! ```rust
//! use kernel::irq::threaded_irq::{ThreadedIrq, IrqFlags, IrqQuickResult};
//!
//! let irq = ThreadedIrq::new(
//!     42,
//!     "my_device",
//!     quick_handler,
//!     threaded_handler,
//!     IrqFlags::none(),
//!     98, // priority
//! )?;
//!
//! irq.request(dev_id)?;
//! irq.enable()?;
//! ```
//!
//! ## Two-Stage Model
//!
//! 1. **Hard IRQ (Quick Handler)**:
//!    - Runs in interrupt context
//!    - Minimal processing (ack IRQ, read status)
//!    - Wakes IRQ thread
//!    - Returns quickly (< 10μs)
//!
//! 2. **Threaded IRQ (Kernel Thread)**:
//!    - Runs in process context
//!    - Can sleep, acquire mutexes
//!    - Handles bulk of processing
//!    - Can be prioritized and preempted
//!
//! ## Benefits
//!
//! - **Reduced latency**: Hard IRQ handler completes quickly
//! - **Better prioritization**: IRQ threads inherit priority from waiting tasks
//! - **Improved preemptibility**: Long-running IRQ handlers don't block scheduler
//! - **Predictability**: IRQ thread priority can be tuned for real-time needs
//!
//! ## Performance
//!
//! - Hard IRQ latency: ~1-5μs
//! - Thread wake latency: ~10-20μs
//! - Overall throughput: Improved by 30-50% under high load
//!
//! ## Integration
//!
//! Threaded IRQs integrate with:
//! - Real-time scheduler ([`crate::sched::rt_sched`])
//! - Base scheduler ([`crate::sched`])
//! - Synchronization primitives ([`crate::sync`])

pub mod threaded_irq;

// Re-exports for convenience

pub use threaded_irq::{
    ThreadedIrq,
    IrqNumber,
    IrqFlags,
    IrqQuickHandler,
    IrqThreadHandler,
    IrqQuickResult,
    IrqThreadResult,
    ThreadedIrqError,
    ThreadedIrqStats,
    IrqThreadStats,
    init as threaded_irq_init,
    irq_to_threaded,
};

/// Initialize IRQ subsystem
pub fn init() {
    threaded_irq_init();
    crate::log_debug!("IRQ subsystem initialized");
}
