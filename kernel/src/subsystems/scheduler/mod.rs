//! Scheduler Subsystem
//! 
//! This module provides comprehensive scheduling capabilities for the NOS kernel,
//! including real-time scheduling, priority-based scheduling, and CPU affinity.

pub mod realtime;

pub use realtime::{
    RealtimeScheduler, RealtimePolicy, RealtimeTaskParams, RealtimeSchedulingStats,
    init_rt_scheduler, get_rt_scheduler,
    thread_policy_to_rt, rt_policy_to_thread,
    is_realtime_policy, is_valid_rt_policy,
};