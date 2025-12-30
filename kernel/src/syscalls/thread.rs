//! Thread system calls

// Only export futex functions - avoid circular import
pub use crate::subsystems::syscalls::thread_futex::{
    FutexWaiter, PiFutexData, futex_wait_timeout, futex_wake_optimized,
    add_futex_waiter, futex_requeue, futex_lock_pi, futex_trylock_pi,
    futex_unlock_pi, get_current_time_ns, is_timeout_expired, requeue_futex_waiters,
};

// Export other thread functions manually
pub use crate::subsystems::syscalls::thread::{
    ThreadControl, SchedulingParameters, thread_flags,
    create_thread, exit_thread, join_thread, get_current_thread_id,
    get_current_process_id, set_thread_priority, get_thread_priority,
    yield_thread, set_thread_affinity, get_thread_affinity,
    set_thread_name, get_thread_name, set_thread_stack_size,
    get_thread_stack_size, set_thread_guard_size, get_thread_guard_size,
    set_thread_scheduling_policy, get_thread_scheduling_policy,
    set_thread_scheduling_parameters, get_thread_scheduling_parameters,
    THREAD_CONTROL,
};