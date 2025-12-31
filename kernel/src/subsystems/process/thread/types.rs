// Thread management for xv6-rust kernel
//
// Implements threading support that extends the existing process infrastructure.
// Provides kernel threads, POSIX threads (pthreads), and thread scheduling
// integration with the existing process scheduler.

extern crate alloc;

// use alloc::sync::Arc;
use alloc::collections::BTreeMap;

use crate::{
    ipc::signal::SignalState,
    process::{Context, TrapFrame},
    subsystems::sync::{Mutex, Once},
};

// Import ThreadError from the api module
use super::api::ThreadError;

// Re-export Pid from the process module
pub use crate::process::Pid;

// ============================================================================
// Constants and Types
// ============================================================================

/// Thread ID type - for POSIX compatibility
pub type Tid = usize;

/// Invalid thread ID
pub const INVALID_TID: Tid = 0;

/// Maximum number of threads per process
pub const MAX_THREADS_PER_PROCESS: usize = 64;

/// Maximum number of threads system-wide
pub const MAX_THREADS: usize = 1024;

/// Default thread stack size
pub const DEFAULT_THREAD_STACK_SIZE: usize = 8192;

/// Thread states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Thread is unused
    Unused,
    /// Thread is allocated but not yet initialized
    Init,
    /// Thread is ready to run
    Runnable,
    /// Thread is ready to run (alias for Runnable)
    Ready,
    /// Thread is currently running
    Running,
    /// Thread is blocked/waiting
    Blocked,
    /// Thread is sleeping
    Sleeping,
    /// Thread has terminated
    Zombie,
    /// Thread is detached
    Detached,
}

/// Thread types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadType {
    /// Kernel thread (runs in kernel space)
    Kernel,
    /// User thread (POSIX thread)
    User,
    /// Main thread of a process
    Main,
}

/// Thread scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicy {
    /// Normal round-robin scheduling
    Normal,
    /// Real-time FIFO
    Fifo,
    /// Real-time round-robin
    RoundRobin,
    /// Batch scheduling
    Batch,
    /// Idle scheduling
    Idle,
}

/// Thread scheduling parameters
#[derive(Debug, Clone, Copy)]
pub struct SchedParam {
    /// Scheduling priority (1-99 for RT policies)
    pub priority: u8,
    /// Time slice in milliseconds
    pub timeslice: u32,
}

impl Default for SchedParam {
    fn default() -> Self {
        Self { priority: 10, timeslice: 10 }
    }
}

/// Thread control block
#[repr(C)]
pub struct Thread {
    /// Thread ID
    pub tid: Tid,
    /// Process ID this thread belongs to
    pub pid: Pid,
    /// Thread state
    pub state: ThreadState,
    /// Thread type
    pub thread_type: ThreadType,

    /// Scheduling information
    pub sched_policy: SchedPolicy,
    pub sched_param: SchedParam,
    pub static_prio: u8, // Static priority
    pub normal_prio: u8, // Normal priority
    pub dyn_prio: u8,    // Dynamic priority

    /// CPU affinity mask
    pub cpus_allowed: u64,

    /// Thread context (registers)
    pub context: Context,
    /// Trap frame for system calls/interrupts
    pub trapframe: *mut TrapFrame,

    /// Stack information
    pub kstack: usize, // Kernel stack top
    pub ustack: usize, // User stack top (for user threads)
    pub stack_size: usize,

    /// Thread entry point and arguments
    pub start_routine: Option<unsafe extern "C" fn(*mut u8) -> *mut u8>,
    pub arg: *mut u8,
    pub return_value: *mut u8,

    /// Thread relationships
    pub parent_tid: Option<Tid>, // Creator thread (for joinable threads)
    pub joiner_tid: Option<Tid>, // Thread waiting to join this thread

    /// Thread flags and attributes
    pub detached: bool,
    pub cancelled: bool,
    pub cancel_state: CancelState,
    pub cancel_type: CancelType,

    /// Signal handling
    pub signal_mask: u64,
    pub pending_signals: u64,
    pub signal_state: Option<SignalState>,

    /// Statistics and debugging
    pub create_time: u64,
    pub run_time: u64,
    pub preempt_count: u32,

    /// Sleep/wait information
    pub wait_channel: usize,
    pub wake_channel: usize,
    pub lock_depth: usize,
    pub flags: usize,

    /// Thread-local storage (simplified)
    pub tls_base: usize,

    /// Child TID pointer for CLONE_CHILD_CLEARTID
    pub child_tid_ptr: usize,

    /// Cleanup function
    pub cleanup: Option<fn(*mut Thread)>,

    /// Architecture-specific data
    #[cfg(target_arch = "x86_64")]
    pub fs_base: usize,
    #[cfg(target_arch = "x86_64")]
    pub gs_base: usize,

    /// Generic architecture fields (non-x86_64)
    #[cfg(not(target_arch = "x86_64"))]
    pub fs_base: usize,
    #[cfg(not(target_arch = "x86_64"))]
    pub gs_base: usize,
}

/// Thread cancellation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelState {
    /// Cancellation is disabled
    Disabled,
    /// Cancellation is enabled (default)
    Enabled,
    /// Cancellation is enabled and asynchronous
    Asynchronous,
}

/// Thread cancellation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelType {
    /// Deferred cancellation (at cancellation points)
    Deferred,
    /// Asynchronous cancellation
    Asynchronous,
}

// ============================================================================
// Thread Method Implementations
// ============================================================================

impl Thread {
    /// Create a new thread control block
    pub const fn new() -> Self {
        Self {
            tid: 0,
            pid: 0,
            state: ThreadState::Unused,
            thread_type: ThreadType::User,

            sched_policy: SchedPolicy::Normal,
            sched_param: SchedParam { priority: 10, timeslice: 10 },
            static_prio: 10,
            normal_prio: 10,
            dyn_prio: 10,

            cpus_allowed: u64::MAX,

            context: Context::new(),
            trapframe: core::ptr::null_mut(),

            kstack: 0,
            ustack: 0,
            stack_size: DEFAULT_THREAD_STACK_SIZE,

            start_routine: None,
            arg: core::ptr::null_mut(),
            return_value: core::ptr::null_mut(),

            parent_tid: None,
            joiner_tid: None,

            detached: false,
            cancelled: false,
            cancel_state: CancelState::Enabled,
            cancel_type: CancelType::Deferred,

            signal_mask: 0,
            pending_signals: 0,
            signal_state: None,

            create_time: 0,
            run_time: 0,
            preempt_count: 0u32,

            wait_channel: 0,
            wake_channel: 0,
            lock_depth: 0,
            flags: 0,

            tls_base: 0,
            child_tid_ptr: 0,
            cleanup: None,

            #[cfg(target_arch = "x86_64")]
            fs_base: 0,
            #[cfg(target_arch = "x86_64")]
            gs_base: 0,
            #[cfg(not(target_arch = "x86_64"))]
            fs_base: 0,
            #[cfg(not(target_arch = "x86_64"))]
            gs_base: 0,
        }
    }

    /// Initialize a thread slot
    pub fn init(&mut self, tid: Tid, pid: Pid, thread_type: ThreadType) -> Result<(), ThreadError> {
        self.tid = tid;
        self.pid = pid;
        self.state = ThreadState::Ready;
        self.thread_type = thread_type;
        Ok(())
    }

    /// Clean up thread resources
    pub fn cleanup(&mut self) {
        self.state = ThreadState::Unused;
        self.tid = 0;
        self.pid = 0;
        self.detached = false;
        self.cancelled = false;
        self.joiner_tid = None;
        self.parent_tid = None;
    }

    /// Wake up a sleeping/blocked thread
    pub fn wake(&mut self) {
        if self.state == ThreadState::Blocked || self.state == ThreadState::Sleeping {
            self.state = ThreadState::Ready;
        }
    }

    /// Block the current thread
    pub fn block(&mut self) {
        self.state = ThreadState::Blocked;
    }

    /// Check if thread is running
    pub fn is_running(&self) -> bool {
        self.state == ThreadState::Running
    }

    /// Check if thread is ready to run
    pub fn is_ready(&self) -> bool {
        self.state == ThreadState::Ready
    }

    /// Check if thread is blocked
    pub fn is_blocked(&self) -> bool {
        self.state == ThreadState::Blocked
    }

    /// Get thread ID
    pub fn tid(&self) -> Tid {
        self.tid
    }

    /// Get process ID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Set thread state to runnable
    pub fn set_runnable(&mut self) {
        self.state = ThreadState::Runnable;
    }

    /// Check if thread is runnable
    pub fn is_runnable(&self) -> bool {
        self.state == ThreadState::Runnable || self.state == ThreadState::Ready
    }

    /// Set thread state to running
    pub fn set_running(&mut self) {
        self.state = ThreadState::Running;
    }

    /// Check if thread can run on a specific CPU
    pub fn can_run_on_cpu(&self, cpu_id: usize) -> bool {
        (self.cpus_allowed & (1 << cpu_id)) != 0
    }

    /// Set CPU affinity mask
    pub fn set_cpu_affinity(&mut self, cpu_mask: u64) {
        self.cpus_allowed = cpu_mask;
    }

    /// Get CPU affinity mask
    pub fn cpu_affinity(&self) -> u64 {
        self.cpus_allowed
    }
}

