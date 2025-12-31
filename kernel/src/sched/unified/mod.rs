//! Unified Scheduler Module
//!
//! Provides unified scheduling interface

use nos_api::Result;
use crate::subsystems::process::thread::Tid;

/// Unified scheduler
pub struct UnifiedScheduler {
    quantum: u64,
}

impl UnifiedScheduler {
    pub fn new() -> Self {
        Self { quantum: 10000 }
    }

    /// Const constructor for static initialization
    pub const fn const_new() -> Self {
        Self { quantum: 10000 }
    }
}

/// Initialize unified scheduler
pub fn init_unified_scheduler() -> Result<()> {
    let _scheduler = UnifiedScheduler::new();
    Ok(())
}

/// Get unified scheduler
pub fn get_unified_scheduler() -> Option<&'static UnifiedScheduler> {
    static SCHEDULER: UnifiedScheduler = UnifiedScheduler::const_new();
    Some(&SCHEDULER)
}

/// Unified scheduling function
/// Returns the next thread ID to schedule, or None if no thread is available
pub fn unified_schedule() -> Option<Tid> {
    use crate::sched::O1Scheduler;
    O1Scheduler::schedule_next().map(|tid| tid as Tid)
}
