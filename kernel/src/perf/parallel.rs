//! Parallel Execution and Workqueue Optimization

use core::fmt;

/// Errors from parallel operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelError {
    QueueFull,
    NoThreads,
    InvalidCpuMask,
    CpuNotAvailable,
    ItemTooLarge,
    AffinityFailed,
    WorkqueueNotFound,
    NotSupported,
    InvalidArgument,
    ThreadCreateFailed,
}

impl fmt::Display for ParallelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParallelError::QueueFull => write!(f, "Workqueue is full"),
            ParallelError::NoThreads => write!(f, "No available threads"),
            ParallelError::InvalidCpuMask => write!(f, "Invalid CPU mask"),
            ParallelError::CpuNotAvailable => write!(f, "CPU not available"),
            ParallelError::ItemTooLarge => write!(f, "Work item too large"),
            ParallelError::AffinityFailed => write!(f, "Failed to set CPU affinity"),
            ParallelError::WorkqueueNotFound => write!(f, "Workqueue not found"),
            ParallelError::NotSupported => write!(f, "Operation not supported"),
            ParallelError::InvalidArgument => write!(f, "Invalid argument"),
            ParallelError::ThreadCreateFailed => write!(f, "Failed to create thread"),
        }
    }
}

pub type ParallelResult<T> = core::result::Result<T, ParallelError>;

/// Work item for workqueue
pub struct WorkItem {
    pub func: unsafe fn(*mut u8),
    pub arg: *mut u8,
}

impl WorkItem {
    pub const fn new(func: unsafe fn(*mut u8), arg: *mut u8) -> Self {
        Self { func, arg }
    }
}
