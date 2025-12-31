//! # Real-Time Operating System (RTOS) Features
//!
//! This module provides comprehensive real-time operating system capabilities for the NOS kernel,
//! including deterministic scheduling, precision timing, and real-time safe synchronization.
//!
//! ## Overview
//!
//! The RTOS subsystem enables hard and soft real-time guarantees with:
//!
//! - **Deterministic Scheduling**: Rate Monotonic, EDF, and Deadline Monotonic algorithms
//! - **Precision Timing**: HPET, TSC, and high-resolution timers with < 100ns accuracy
//! - **Real-Time Safe Sync**: Priority inheritance mutexes and deadlock prevention
//! - **Bounded Memory**: Guaranteed allocation time with memory pools
//! - **Low Interrupt Latency**: < 1μs interrupt response time
//!
//! ## Architecture
//!
//! The RTOS system is organized into several interconnected components:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  RTOS Subsystem                      │
//! ├─────────────────────────────────────────────────────┤
//! │  Scheduler    │  Timing    │  Sync    │  Metrics    │
//! │  - RMS        │  - HPET    │  - PI     │  - Latency  │
//! │  - EDF        │  - TSC     │  - PCP    │  - Jitter   │
//! │  - Analysis   │  - HRTimer │  - FIFO   │  - WCET     │
//! ├─────────────────────────────────────────────────────┤
//! │  Memory        │  Interrupts                        │
//! │  - Pools       │  - Threading                       │
//! │  - Pre-alloc   │  - Priority                        │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! ### Real-Time Tasks
//!
//! Real-time tasks are characterized by:
//! - **WCET**: Worst-Case Execution Time
//! - **Period**: Time between consecutive activations
//! - **Deadline**: Maximum time to complete after activation
//! - **Priority**: Static (RMS) or dynamic (EDF)
//!
//! ### Schedulability Analysis
//!
//! The system provides multiple analysis methods:
//! - **Utilization Bound Test**: Quick check for RMS schedulability
//! - **Response Time Analysis**: Exact analysis for arbitrary task sets
//! - **Liu & Layland**: Theoretical bounds for periodic tasks
//!
//! ### Timing Guarantees
//!
//! Latency targets:
//! - Interrupt latency: < 1μs
//! - Context switch: < 5μs
//! - Timer accuracy: < 100ns
//! - Scheduler overhead: < 1μs
//!
//! ## Usage Examples
//!
//! ### Creating a Real-Time Task
//!
//! ```no_run
//! use kernel::rtos::scheduler::{RealTimeTask, SchedulingPolicy};
//!
//! let task = RealTimeTask::new(
//!     42,                    // task_id
//!     1000,                  // period (μs)
//!     800,                   // deadline (μs)
//!     500,                   // wcet (μs)
//!     SchedulingPolicy::RateMonotonic,
//! );
//! ```
//!
//! ### Priority Inheritance Mutex
//!
//! ```no_run
//! use kernel::rtos::synchronization::PiMutex;
//!
//! let mutex = PiMutex::new(0, 100); // task_id, priority
//! let _guard = mutex.lock()?;
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```
//!
//! ### High-Resolution Timer
//!
//! ```no_run
//! use kernel::rtos::timing::HrTimer;
//! use core::time::Duration;
//!
//! let timer = HrTimer::new()?;
//! timer.arm(Duration::from_micros(100))?;
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```
//!
//! ## Performance Characteristics
//!
//! All operations provide bounded worst-case execution time:
//!
//! | Operation | WCET | Description |
//! |-----------|------|-------------|
//! | Schedule | O(log n) | Task insertion and selection |
//! | Mutex Lock | O(1) | Uncontended case |
//! | Mutex Lock | O(m) | With priority inheritance (m = blocked tasks) |
//! | Timer Arm | O(log n) | Timer insertion |
//! | Memory Alloc | O(1) | Pool-based allocation |
//!
//! ## Configuration
//!
//! The RTOS can be configured via compile-time features:
//!
//! - `rtos_rms`: Enable Rate Monotonic Scheduling
//! - `rtos_edf`: Enable Earliest Deadline First
//! - `rtos_metrics`: Enable performance metrics collection
//! - `rtos_debug`: Enable debug assertions and logging
//!
//! ## Safety and Correctness
//!
//! The RTOS implementation provides:
//!
//! - **Priority Inheritance**: Prevents priority inversion
//! - **Deadlock Prevention**: Priority ceiling protocol
//! - **Bounded Execution**: All operations have known WCET
//! - **Deterministic Behavior**: No dynamic memory allocation in hot paths
//! - **Formal Analysis**: Schedulability guarantees via mathematical proofs
//!
//! ## Error Handling
//!
//! All RTOS operations return `Result<T, RtError>`, where `RtError` includes:
//!
//! - Missed deadlines
//! - Schedulability violations
//! - Timeout errors
//! - Resource exhaustion
//! - Priority inversion detected
//!
//! See [`RtError`] for details.

mod scheduler;
mod timing;
mod synchronization;
mod memory;
mod interrupts;
mod metrics;

pub use scheduler::*;
pub use timing::*;
pub use synchronization::*;
pub use memory::*;
pub use interrupts::*;
pub use metrics::*;

use core::fmt;

/// Real-time operating system errors
///
/// This enum represents all possible error conditions that can occur in the RTOS subsystem.
/// Each error variant provides specific information about the failure condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RtError {
    /// Deadline was missed
    ///
    /// This error occurs when a real-time task fails to complete before its deadline.
    /// Contains the task ID and the amount by which the deadline was missed (in microseconds).
    DeadlineMissed {
        task_id: u64,
        miss_duration_us: u64,
    },

    /// Task set is not schedulable
    ///
    /// This error indicates that the provided task set cannot meet all deadlines
    /// under the selected scheduling policy. Contains the total utilization that
    /// exceeds the schedulability bound.
    NotSchedulable {
        utilization: f64,
        bound: f64,
    },

    /// Invalid timing parameter
    ///
    /// Raised when a timing parameter (period, deadline, WCET) is invalid.
    /// For example, when WCET > deadline or deadline > period.
    InvalidTimingParameter {
        parameter: &'static str,
        value: u64,
    },

    /// Priority inversion detected
    ///
    /// Indicates that priority inheritance mechanisms failed to prevent
    /// unbounded priority inversion.
    PriorityInversion {
        high_prio: u8,
        low_prio: u8,
        duration_us: u64,
    },

    /// Timer operation failed
    ///
    /// Generic error for timer-related failures such as:
    /// - Hardware timer unavailable
    /// - Timer overflow
    /// - Invalid timer configuration
    TimerError {
        operation: &'static str,
        reason: &'static str,
    },

    /// Synchronization timeout
    ///
    /// Raised when a lock acquisition or wait operation exceeds the timeout.
    Timeout {
        resource: &'static str,
        timeout_us: u64,
    },

    /// Memory allocation failed
    ///
    /// Indicates that real-time memory allocation failed due to:
    /// - Pool exhaustion
    /// - Memory reservation failure
    /// - OOM condition
    MemoryAllocationFailed {
        pool_id: usize,
        requested_size: usize,
    },

    /// Interrupt latency too high
    ///
    /// Indicates that interrupt handling exceeded the latency budget.
    InterruptLatencyExceeded {
        irq: u32,
        latency_ns: u64,
        threshold_ns: u64,
    },

    /// Context switch time exceeded
    ///
    /// Context switch took longer than the guaranteed bound.
    ContextSwitchExceeded {
        time_ns: u64,
        threshold_ns: u64,
    },

    /// Invalid priority
    ///
    /// Priority value is outside the valid range [0, 255].
    InvalidPriority {
        priority: u8,
        max_priority: u8,
    },

    /// Resource already exists
    ///
    /// Attempted to create a duplicate resource (task, timer, etc.).
    AlreadyExists {
        resource_type: &'static str,
        id: u64,
    },

    /// Resource not found
    ///
    /// Attempted to access a non-existent resource.
    NotFound {
        resource_type: &'static str,
        id: u64,
    },

    /// Operation not supported
    ///
    /// The requested operation is not supported in the current configuration.
    NotSupported {
        operation: &'static str,
        reason: &'static str,
    },

    /// System state error
    ///
    /// The RTOS subsystem is in an invalid state.
    InvalidState {
        state: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadlineMissed { task_id, miss_duration_us } => {
                write!(f, "Task {} deadline missed by {} μs", task_id, miss_duration_us)
            }
            Self::NotSchedulable { utilization, bound } => {
                write!(f, "Task set not schedulable: utilization {:.3} > bound {:.3}", utilization, bound)
            }
            Self::InvalidTimingParameter { parameter, value } => {
                write!(f, "Invalid timing parameter {}: {}", parameter, value)
            }
            Self::PriorityInversion { high_prio, low_prio, duration_us } => {
                write!(f, "Priority inversion detected: priority {} blocked by {} for {} μs", high_prio, low_prio, duration_us)
            }
            Self::TimerError { operation, reason } => {
                write!(f, "Timer error in {}: {}", operation, reason)
            }
            Self::Timeout { resource, timeout_us } => {
                write!(f, "Timeout waiting for {} after {} μs", resource, timeout_us)
            }
            Self::MemoryAllocationFailed { pool_id, requested_size } => {
                write!(f, "Memory allocation failed in pool {} for {} bytes", pool_id, requested_size)
            }
            Self::InterruptLatencyExceeded { irq, latency_ns, threshold_ns } => {
                write!(f, "IRQ {} latency {} ns exceeds threshold {} ns", irq, latency_ns, threshold_ns)
            }
            Self::ContextSwitchExceeded { time_ns, threshold_ns } => {
                write!(f, "Context switch {} ns exceeds threshold {} ns", time_ns, threshold_ns)
            }
            Self::InvalidPriority { priority, max_priority } => {
                write!(f, "Invalid priority {} (max: {})", priority, max_priority)
            }
            Self::AlreadyExists { resource_type, id } => {
                write!(f, "{} {} already exists", resource_type, id)
            }
            Self::NotFound { resource_type, id } => {
                write!(f, "{} {} not found", resource_type, id)
            }
            Self::NotSupported { operation, reason } => {
                write!(f, "Operation '{}' not supported: {}", operation, reason)
            }
            Self::InvalidState { state, expected } => {
                write!(f, "Invalid state '{}' (expected '{}')", state, expected)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RtError::DeadlineMissed {
            task_id: 42,
            miss_duration_us: 100,
        };
        assert!(err.to_string().contains("42"));
        assert!(err.to_string().contains("100"));
    }

    #[test]
    fn test_error_copy() {
        let err1 = RtError::InvalidTimingParameter {
            parameter: "wcet",
            value: 0,
        };
        let err2 = err1;
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_not_schedulable_error() {
        let err = RtError::NotSchedulable {
            utilization: 1.2,
            bound: 0.693,
        };
        let msg = err.to_string();
        assert!(msg.contains("1.2"));
        assert!(msg.contains("0.693"));
    }
}
