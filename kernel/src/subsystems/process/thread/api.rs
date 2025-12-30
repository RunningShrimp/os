use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;

use super::types::{Thread, Tid, Pid, ThreadState, ThreadType, SchedPolicy, SchedParam, MAX_THREADS, INVALID_TID};
use super::table::ThreadTable;
use super::scheduling::{thread_table, current_thread, get_current_thread, get_current_thread_mut};

/// Get current thread ID (POSIX compatible)
pub fn thread_self() -> Tid {
    current_thread().unwrap_or(0)
}

/// Set thread-specific data (TLS)
pub fn thread_set_tls(tls_base: usize) {
    if let Some(thread) = get_current_thread_mut() {
        thread.tls_base = tls_base;

        // Architecture-specific TLS setup
        #[cfg(target_arch = "x86_64")]
        {
            unsafe {
                core::arch::asm!("wrfsbase {}", in(reg) tls_base);
                thread.fs_base = tls_base;
            }
        }
    }
}

/// Get thread-specific data
pub fn thread_get_tls() -> usize {
    get_current_thread().map(|t| t.tls_base).unwrap_or(0)
}

/// Set thread CPU affinity
pub fn thread_setaffinity(tid: Tid, cpu_mask: u64) -> Result<(), ThreadError> {
    let table = thread_table();
    let thread = table.find_thread(tid).ok_or(ThreadError::InvalidThreadId)?;

    thread.set_cpu_affinity(cpu_mask);
    Ok(())
}

/// Get thread CPU affinity
pub fn thread_getaffinity(tid: Tid) -> Result<u64, ThreadError> {
    let table = thread_table();
    let thread = table
        .find_thread_ref(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    Ok(thread.cpus_allowed)
}

/// Set thread scheduling policy and parameters
pub fn thread_setschedparam(
    tid: Tid,
    policy: SchedPolicy,
    param: SchedParam,
) -> Result<(), ThreadError> {
    let table = thread_table();
    let thread = table.find_thread(tid).ok_or(ThreadError::InvalidThreadId)?;

    thread.sched_policy = policy;
    thread.sched_param = param;
    thread.static_prio = param.priority;
    thread.normal_prio = param.priority;
    thread.dyn_prio = param.priority;

    // If this is a real-time policy, register with RT scheduler
    if crate::subsystems::scheduler::is_realtime_policy(policy) {
        if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
            let rt_policy = crate::subsystems::scheduler::thread_policy_to_rt(policy);
            let rt_task = crate::subsystems::scheduler::RealtimeTaskParams {
                task_id: tid,
                policy: rt_policy,
                priority: param.priority,
                period_ms: 0, // Default: not periodic
                execution_time_ms: param.timeslice,
                deadline_ms: param.timeslice * 2, // Default: 2x timeslice
                bandwidth_percent: (param.priority as u32 * 10).min(80), /* Scale priority to
                                                   * bandwidth */
                active: true,
                creation_time: crate::subsystems::time::timestamp_nanos(),
                next_activation: crate::subsystems::time::timestamp_nanos(),
                absolute_deadline: 0,
                remaining_time: param.timeslice,
                timeslice_ms: param.timeslice,
                timeslice_remaining: param.timeslice,
            };

            // Check admission control
            if rt_scheduler.check_admission(&rt_task) {
                let _ = rt_scheduler.add_rt_task(rt_task);
            } else {
                return Err(ThreadError::ResourceLimitExceeded);
            }
        }
    }

    Ok(())
}

/// Get thread scheduling parameters
pub fn thread_getschedparam(tid: Tid) -> Result<(SchedPolicy, SchedParam), ThreadError> {
    let table = thread_table();
    let thread = table
        .find_thread_ref(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    Ok((thread.sched_policy, thread.sched_param))
}

/// Activate a real-time task
pub fn activate_rt_task(tid: Tid) -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        let current_time = crate::subsystems::time::timestamp_nanos();
        rt_scheduler
            .activate_task(tid, current_time)
            .map_err(|_| ThreadError::InvalidOperation)
    } else {
        Err(ThreadError::InvalidOperation)
    }
}

/// Deactivate a real-time task
pub fn deactivate_rt_task(tid: Tid) -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        rt_scheduler
            .deactivate_task(tid)
            .map_err(|_| ThreadError::InvalidOperation)
    } else {
        Err(ThreadError::InvalidOperation)
    }
}

/// Update real-time task execution time
pub fn update_rt_task_execution(tid: Tid, elapsed_ms: u32) -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        let current_time = crate::subsystems::time::timestamp_nanos();
        rt_scheduler.update_task_execution(tid, elapsed_ms, current_time);
        Ok(())
    } else {
        Err(ThreadError::InvalidOperation)
    }
}

/// Get real-time scheduling statistics
pub fn get_rt_scheduling_stats() -> Option<crate::subsystems::scheduler::RealtimeSchedulingStats> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        Some(rt_scheduler.get_stats())
    } else {
        None
    }
}

/// Reset real-time scheduling statistics
pub fn reset_rt_scheduling_stats() -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        rt_scheduler.reset_stats();
        Ok(())
    } else {
        Err(ThreadError::InvalidOperation)
    }
}

/// Set maximum CPU bandwidth for real-time tasks
pub fn set_rt_max_bandwidth(max_percent: u32) -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        rt_scheduler.set_max_bandwidth(max_percent);
        Ok(())
    } else {
        Err(ThreadError::InvalidOperation)
    }
}

/// Get current CPU bandwidth allocation for real-time tasks
pub fn get_rt_allocated_bandwidth() -> Option<usize> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        Some(rt_scheduler.get_allocated_bandwidth())
    } else {
        None
    }
}

/// Get maximum allowed CPU bandwidth for real-time tasks
pub fn get_rt_max_bandwidth() -> Option<usize> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        Some(rt_scheduler.get_max_bandwidth())
    } else {
        None
    }
}

// ============================================================================
// Thread Statistics and Diagnostics
// ============================================================================

/// Get thread statistics
pub fn get_thread_stats() -> ThreadStats {
    let table = thread_table();
    let mut stats = ThreadStats::default();

    for thread in table.iter() {
        stats.total_threads += 1;

        match thread.state {
            ThreadState::Running => stats.running_threads += 1,
            ThreadState::Runnable => stats.runnable_threads += 1,
            ThreadState::Blocked => stats.blocked_threads += 1,
            ThreadState::Zombie => stats.zombie_threads += 1,
            _ => {},
        }

        if thread.thread_type == ThreadType::Kernel {
            stats.kernel_threads += 1;
        } else {
            stats.user_threads += 1;
        }
    }

    stats
}

/// Print thread information for debugging
pub fn print_thread_info() {
    let table = thread_table();
    crate::println!("=== Thread Information ===");
    crate::println!("Active threads: {}", table.active_count());

    for thread in table.iter() {
        crate::println!(
            "Thread {}: PID={}, State={:?}, Type={:?}, CPU={:016X}",
            thread.tid,
            thread.pid,
            thread.state,
            thread.thread_type,
            thread.cpus_allowed
        );
    }
    crate::println!("========================");
}

// ============================================================================
// Error Types and Structures
// ============================================================================

/// Thread errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadError {
    /// Invalid thread ID
    InvalidThreadId,
    /// No available thread slots
    NoSlotsAvailable,
    /// Out of memory
    OutOfMemory,
    /// Operation not permitted
    OperationNotPermitted,
    /// Permission denied
    PermissionDenied,
    /// Invalid operation
    InvalidOperation,
    /// Thread was killed
    ThreadKilled,
    /// Thread already detached
    AlreadyDetached,
    /// Thread not joinable
    NotJoinable,
    /// Resource limit exceeded
    ResourceLimitExceeded,
}

/// Thread statistics
#[derive(Debug, Clone, Default)]
pub struct ThreadStats {
    /// Total number of threads
    pub total_threads: usize,
    /// Currently running threads
    pub running_threads: usize,
    /// Runnable threads
    pub runnable_threads: usize,
    /// Blocked threads
    pub blocked_threads: usize,
    /// Zombie threads
    pub zombie_threads: usize,
    /// Kernel threads
    pub kernel_threads: usize,
    /// User threads
    pub user_threads: usize,
}

/// Get current time (simplified implementation)
fn get_current_time() -> u64 {
    static TIMER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    TIMER.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

// ============================================================================
// Module Exports
// ============================================================================

// All types are already available as they're defined in this module
// No need to re-export them

// Ensure module is properly closed
