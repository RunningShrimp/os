// Thread management for xv6-rust kernel
//
// Implements threading support that extends the existing process infrastructure.
// Provides kernel threads, POSIX threads (pthreads), and thread scheduling
// integration with the existing process scheduler.

extern crate alloc;

use core::ptr::null_mut;
// use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::subsystems::sync::{Mutex, Once};
use crate::process::{Pid, Context, TrapFrame};
use crate::subsystems::mm::{kalloc, kfree, PAGE_SIZE};
use crate::ipc::signal::SignalState;

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
    /// Thread is currently running
    Running,
    /// Thread is blocked/waiting
    Blocked,
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
        Self {
            priority: 10,
            timeslice: 10,
        }
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
    pub static_prio: u8,  // Static priority
    pub normal_prio: u8, // Normal priority
    pub dyn_prio: u8,    // Dynamic priority

    /// CPU affinity mask
    pub cpus_allowed: u64,

    /// Thread context (registers)
    pub context: Context,
    /// Trap frame for system calls/interrupts
    pub trapframe: *mut TrapFrame,

    /// Stack information
    pub kstack: usize,    // Kernel stack top
    pub ustack: usize,    // User stack top (for user threads)
    pub stack_size: usize,

    /// Thread entry point and arguments
    pub start_routine: Option<unsafe extern "C" fn(*mut u8) -> *mut u8>,
    pub arg: *mut u8,
    pub return_value: *mut u8,

    /// Thread relationships
    pub parent_tid: Option<Tid>,  // Creator thread (for joinable threads)
    pub joiner_tid: Option<Tid>,  // Thread waiting to join this thread

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

            cpus_allowed: u64::MAX, // All CPUs

            context: Context::new(),
            trapframe: null_mut(),

            kstack: 0,
            ustack: 0,
            stack_size: DEFAULT_THREAD_STACK_SIZE,

            start_routine: None,
            arg: null_mut(),
            return_value: null_mut(),

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
            preempt_count: 0,

            wait_channel: 0,
            wake_channel: 0,

            tls_base: 0,

            child_tid_ptr: 0,

            cleanup: None,

            #[cfg(target_arch = "x86_64")]
            fs_base: 0,
            #[cfg(target_arch = "x86_64")]
            gs_base: 0,
        }
    }

    /// Initialize a thread for execution
    pub fn init(&mut self, tid: Tid, pid: Pid, thread_type: ThreadType) -> Result<(), ThreadError> {
        self.tid = tid;
        self.pid = pid;
        self.thread_type = thread_type;
        self.state = ThreadState::Init;

        let pools = get_thread_pools();

        // Allocate kernel stack from pool (reuses freed stacks)
        let kstack_addr = match pools.alloc_stack() {
            Some(addr) => addr,
            None => return Err(ThreadError::OutOfMemory),
        };
        self.kstack = kstack_addr + PAGE_SIZE;

        // Allocate trapframe from pool (reuses freed trapframes)
        let trapframe_addr = match pools.alloc_trapframe() {
            Some(addr) => addr,
            None => {
                // Free the stack if trapframe allocation fails
                pools.free_stack(kstack_addr);
                self.kstack = 0;
                return Err(ThreadError::OutOfMemory);
            }
        };
        self.trapframe = trapframe_addr as *mut TrapFrame;

        // Initialize signal state
        self.signal_state = Some(SignalState::new());

        // Set creation time
        self.create_time = get_current_time();

        // Initialize architecture-specific state
        self.init_arch();

        Ok(())
    }

    /// Initialize architecture-specific thread state
    #[cfg(target_arch = "riscv64")]
    fn init_arch(&mut self) {
        // RISC-V specific initialization
        unsafe {
            if !self.trapframe.is_null() {
                let tf = &mut *self.trapframe;
                *tf = TrapFrame::new();
                tf.kernel_sp = self.kstack;
            }
        }
        
        // Initialize context for this thread
        crate::subsystems::process::context_switch::init_context(
            &mut self.context,
            self.kstack,
            0, // Entry point will be set later
            0, // No argument initially
            self.thread_type == ThreadType::User
        );
    }

    #[cfg(target_arch = "aarch64")]
    fn init_arch(&mut self) {
        // ARM64 specific initialization
        unsafe {
            if !self.trapframe.is_null() {
                let tf = &mut *self.trapframe;
                *tf = TrapFrame::new();
                tf.sp = self.kstack;
            }
        }
        
        // Initialize context for this thread
        crate::subsystems::process::context_switch::init_context(
            &mut self.context,
            self.kstack,
            0, // Entry point will be set later
            0, // No argument initially
            self.thread_type == ThreadType::User
        );
    }

    #[cfg(target_arch = "x86_64")]
    fn init_arch(&mut self) {
        // x86-64 specific initialization
        unsafe {
            if !self.trapframe.is_null() {
                let tf = &mut *self.trapframe;
                *tf = TrapFrame::new();
                tf.rsp = self.kstack;
            }
            // Set up FS/GS bases for TLS
            self.fs_base = 0;
            self.gs_base = 0;
        }
        
        // Initialize context for this thread
        crate::subsystems::process::context_switch::init_context(
            &mut self.context,
            self.kstack,
            0, // Entry point will be set later
            0, // No argument initially
            self.thread_type == ThreadType::User
        );
    }

    /// Check if thread is currently running
    pub fn is_running(&self) -> bool {
        self.state == ThreadState::Running
    }

    /// Check if thread is ready to run
    pub fn is_runnable(&self) -> bool {
        self.state == ThreadState::Runnable
    }

    /// Check if thread can be joined
    pub fn is_joinable(&self) -> bool {
        !self.detached && self.parent_tid.is_some()
    }

    /// Check if thread has terminated
    pub fn is_terminated(&self) -> bool {
        matches!(self.state, ThreadState::Zombie | ThreadState::Detached)
    }

    /// Get effective priority (considering dynamic priority)
    pub fn effective_priority(&self) -> u8 {
        self.dyn_prio
    }

    /// Update thread priority
    pub fn update_priority(&mut self, new_prio: u8) {
        self.dyn_prio = new_prio;
    }

    /// Reset dynamic priority to normal priority
    pub fn reset_priority(&mut self) {
        self.dyn_prio = self.normal_prio;
    }

    /// Check if thread can run on current CPU
    pub fn can_run_on_cpu(&self, cpu_id: usize) -> bool {
        (self.cpus_allowed & (1 << cpu_id)) != 0
    }

    /// Set CPU affinity
    pub fn set_cpu_affinity(&mut self, cpu_mask: u64) {
        self.cpus_allowed = cpu_mask;
    }

    /// Wake up thread
    pub fn wake(&mut self) -> bool {
        if matches!(self.state, ThreadState::Blocked) {
            self.state = ThreadState::Runnable;
            self.wake_channel = 0;
            true
        } else {
            false
        }
    }

    /// Block thread on a channel
    pub fn block(&mut self, channel: usize) {
        self.state = ThreadState::Blocked;
        self.wait_channel = channel;
    }

    /// Set thread to runnable state
    pub fn set_runnable(&mut self) {
        if self.state != ThreadState::Zombie {
            self.state = ThreadState::Runnable;
        }
    }

    /// Set thread to running state
    pub fn set_running(&mut self) {
        self.state = ThreadState::Running;
    }

    /// Clean up thread resources and return them to object pools
    /// 
    /// This function returns kernel stacks and trapframes to the resource pools
    /// for reuse, reducing memory fragmentation and allocation overhead.
    pub fn cleanup(&mut self) {
        let pools = get_thread_pools();
        
        // Return kernel stack to pool for reuse
        if self.kstack != 0 {
            let stack_addr = self.kstack - PAGE_SIZE;
            pools.free_stack(stack_addr);
            self.kstack = 0;
        }

        // Free user stack (not pooled as it's process-specific)
        if self.ustack != 0 && self.stack_size > 0 {
            unsafe {
                let pages = (self.stack_size + PAGE_SIZE - 1) / PAGE_SIZE;
                for i in 0..pages {
                    kfree((self.ustack - i * PAGE_SIZE) as *mut u8);
                }
            }
            self.ustack = 0;
        }

        // Return trapframe to pool for reuse
        if !self.trapframe.is_null() {
            let tf_addr = self.trapframe as usize;
            pools.free_trapframe(tf_addr);
            self.trapframe = null_mut();
        }

        // Call cleanup function if provided
        if let Some(cleanup_fn) = self.cleanup {
            cleanup_fn(self as *mut Thread);
        }

        self.state = ThreadState::Unused;
    }
}

// Safety: Thread control block is protected by THREAD_TABLE mutex
unsafe impl Send for Thread {}

impl Drop for Thread {
    fn drop(&mut self) {
        self.cleanup();
    }
}

// ============================================================================
// Thread Table Management
// ============================================================================

/// Global thread table
pub struct ThreadTable {
    /// Thread storage
    threads: [Thread; MAX_THREADS],
    /// Next thread ID to allocate
    next_tid: AtomicUsize,
    /// PID to TID mapping for fast lookup
    pid_to_tids: BTreeMap<Pid, Vec<Tid>>,
    /// Free thread slots
    free_slots: Vec<usize>,
    /// Active thread count
    active_count: AtomicUsize,
}

impl ThreadTable {
    /// Create a new thread table with object pool
    pub fn new() -> Self {
        let mut threads: [Thread; MAX_THREADS] = unsafe {
            // SAFETY: Thread can be created as all-zeros for Unused state
            core::mem::MaybeUninit::uninit().assume_init()
        };

        // Initialize each thread
        for i in 0..MAX_THREADS {
            threads[i] = Thread::new();
        }

        // Pre-populate free_slots with all available indices for O(1) allocation
        let mut free_slots = Vec::with_capacity(MAX_THREADS);
        for i in 0..MAX_THREADS {
            free_slots.push(i);
        }

        Self {
            threads,
            next_tid: AtomicUsize::new(1),
            pid_to_tids: BTreeMap::new(),
            free_slots,
            active_count: AtomicUsize::new(0),
        }
    }

    /// Allocate a new thread using object pool
    /// 
    /// This function provides O(1) allocation by reusing freed thread slots
    /// from the free_slots list, reducing memory fragmentation and allocation overhead.
    /// 
    /// # Arguments
    /// 
    /// * `pid` - Process ID that owns this thread
    /// * `thread_type` - Type of thread (Kernel, User, or Main)
    /// 
    /// # Returns
    /// 
    /// * `Ok(&mut Thread)` if allocation succeeds
    /// * `Err(ThreadError)` if no slots are available
    pub fn alloc_thread(&mut self, pid: Pid, thread_type: ThreadType) -> Result<&mut Thread, ThreadError> {
        // Get free slot from object pool (O(1))
        let slot_idx = match self.free_slots.pop() {
            Some(idx) => idx,
            None => {
                // Fallback to linear search if free_slots is empty (should rarely happen)
                let mut found_slot = None;
                for (i, thread) in self.threads.iter().enumerate() {
                    if thread.state == ThreadState::Unused {
                        found_slot = Some(i);
                        break;
                    }
                }
                match found_slot {
                    Some(idx) => idx,
                    None => return Err(ThreadError::NoSlotsAvailable),
                }
            }
        };

        let tid = self.next_tid.fetch_add(1, Ordering::SeqCst);
        if tid == INVALID_TID {
            return Err(ThreadError::InvalidThreadId);
        }

        let thread = &mut self.threads[slot_idx];
        thread.init(tid, pid, thread_type)?;

        // Add to PID mapping
        self.pid_to_tids.entry(pid).or_insert_with(Vec::new).push(tid);

        // Update active count
        self.active_count.fetch_add(1, Ordering::SeqCst);

        Ok(thread)
    }

    /// Find thread by TID
    pub fn find_thread(&mut self, tid: Tid) -> Option<&mut Thread> {
        if tid == 0 || tid >= MAX_THREADS {
            return None;
        }

        let thread = &mut self.threads[tid];
        if thread.state != ThreadState::Unused && thread.tid == tid {
            Some(thread)
        } else {
            None
        }
    }

    /// Find thread by TID (immutable)
    pub fn find_thread_ref(&self, tid: Tid) -> Option<&Thread> {
        if tid == 0 || tid >= MAX_THREADS {
            return None;
        }

        let thread = &self.threads[tid];
        if thread.state != ThreadState::Unused && thread.tid == tid {
            Some(thread)
        } else {
            None
        }
    }

    /// Find all threads for a process
    pub fn find_threads_by_pid(&self, pid: Pid) -> Vec<&Thread> {
        if let Some(tids) = self.pid_to_tids.get(&pid) {
            tids.iter()
                .filter_map(|&tid| self.find_thread_ref(tid))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Free a thread and return it to the object pool
    /// 
    /// This function cleans up thread resources and returns the thread slot
    /// to the free_slots pool for reuse, reducing memory fragmentation.
    /// 
    /// # Arguments
    /// 
    /// * `tid` - Thread ID to free
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` if the thread was successfully freed
    /// * `Err(ThreadError)` if the thread ID is invalid
    pub fn free_thread(&mut self, tid: Tid) -> Result<(), ThreadError> {
        if tid == 0 || tid >= MAX_THREADS {
            return Err(ThreadError::InvalidThreadId);
        }

        let thread = &mut self.threads[tid];
        if thread.state == ThreadState::Unused || thread.tid != tid {
            return Err(ThreadError::InvalidThreadId);
        }

        let pid = thread.pid;

        // Clean up thread resources
        thread.cleanup();

        // Remove from PID mapping
        if let Some(tids) = self.pid_to_tids.get_mut(&pid) {
            tids.retain(|&t| t != tid);
            if tids.is_empty() {
                self.pid_to_tids.remove(&pid);
            }
        }

        // Return slot to object pool for reuse (O(1))
        // Only add back if free_slots is not full (prevent unbounded growth)
        if self.free_slots.len() < MAX_THREADS {
            self.free_slots.push(tid);
        }

        // Update active count
        self.active_count.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    /// Get number of active threads
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Get maximum number of threads
    pub fn max_threads(&self) -> usize {
        MAX_THREADS
    }

    /// Get iterator over all threads
    pub fn iter(&self) -> impl Iterator<Item = &Thread> {
        self.threads.iter()
            .filter(|t| t.state != ThreadState::Unused)
    }

    /// Get mutable iterator over all threads
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Thread> {
        self.threads.iter_mut()
            .filter(|t| t.state != ThreadState::Unused)
    }
}

/// Thread resource pools for efficient allocation
/// Reuses freed kernel stacks and trapframes to reduce memory fragmentation
struct ThreadResourcePools {
    stack_pool: Mutex<alloc::vec::Vec<usize>>,      // Reusable kernel stack addresses
    trapframe_pool: Mutex<alloc::vec::Vec<usize>>,  // Reusable trapframe addresses
}

impl ThreadResourcePools {
    fn new() -> Self {
        Self {
            stack_pool: Mutex::new(alloc::vec::Vec::new()),
            trapframe_pool: Mutex::new(alloc::vec::Vec::new()),
        }
    }

    /// Get a kernel stack from pool or allocate new one
    fn alloc_stack(&self) -> Option<usize> {
        let mut pool = self.stack_pool.lock();
        pool.pop().or_else(|| {
            let stack = kalloc();
            if stack.is_null() { None } else { Some(stack as usize) }
        })
    }

    /// Return kernel stack to pool for reuse
    fn free_stack(&self, stack_addr: usize) {
        if stack_addr != 0 {
            let mut pool = self.stack_pool.lock();
            // Limit pool size to prevent unbounded growth
            if pool.len() < MAX_THREADS {
                pool.push(stack_addr);
            } else {
                // Pool is full, free the memory
                drop(pool);
                unsafe { kfree(stack_addr as *mut u8); }
            }
        }
    }

    /// Get a trapframe from pool or allocate new one
    fn alloc_trapframe(&self) -> Option<usize> {
        let mut pool = self.trapframe_pool.lock();
        pool.pop().or_else(|| {
            let tf = kalloc() as *mut TrapFrame;
            if tf.is_null() { None } else { Some(tf as usize) }
        })
    }

    /// Return trapframe to pool for reuse
    fn free_trapframe(&self, tf_addr: usize) {
        if tf_addr != 0 {
            let mut pool = self.trapframe_pool.lock();
            // Limit pool size to prevent unbounded growth
            if pool.len() < MAX_THREADS {
                pool.push(tf_addr);
            } else {
                // Pool is full, free the memory
                drop(pool);
                unsafe { kfree(tf_addr as *mut u8); }
            }
        }
    }
}

/// Global thread resource pools
static THREAD_RESOURCE_POOLS: Once = Once::new();
static mut THREAD_POOLS: Option<ThreadResourcePools> = None;

fn get_thread_pools() -> &'static ThreadResourcePools {
    unsafe {
        THREAD_RESOURCE_POOLS.call_once(|| {
            THREAD_POOLS = Some(ThreadResourcePools::new());
        });
        THREAD_POOLS.as_ref().unwrap()
    }
}

/// Global thread table instance
static mut THREAD_TABLE: Option<ThreadTable> = None;
static THREAD_TABLE_INIT: Once = Once::new();

/// Get the global thread table
pub fn thread_table() -> &'static mut ThreadTable {
    unsafe {
        THREAD_TABLE_INIT.call_once(|| {
            THREAD_TABLE = Some(ThreadTable::new());
        });
        THREAD_TABLE.as_mut().unwrap()
    }
}

// ============================================================================
// Thread Management API
// ============================================================================

/// Current thread ID for each CPU (indexed by CPU ID)
static mut CURRENT_THREAD: [Option<Tid>; 8] = [None; 8];

/// Initialize thread subsystem
pub fn init() {
    let table = thread_table();
    crate::println!("thread: Thread subsystem initialized (max_threads={})", table.max_threads());
    
    // Initialize context switch subsystem
    crate::subsystems::process::context_switch::init();
    
    // Initialize real-time scheduler
    crate::subsystems::scheduler::init_rt_scheduler();
    crate::println!("thread: Real-time scheduler initialized");
}

/// Get current thread ID
pub fn current_thread() -> Option<Tid> {
    let cpu_id = crate::cpu::cpuid();
    unsafe { CURRENT_THREAD[cpu_id] }
}

/// Set current thread ID
pub fn set_current_thread(tid: Option<Tid>) {
    let cpu_id = crate::cpu::cpuid();
    unsafe { CURRENT_THREAD[cpu_id] = tid; }
}

/// Get current thread
pub fn get_current_thread() -> Option<&'static Thread> {
    current_thread().and_then(|tid| thread_table().find_thread_ref(tid))
}

/// Get current thread (mutable)
pub fn get_current_thread_mut() -> Option<&'static mut Thread> {
    current_thread().and_then(|tid| thread_table().find_thread(tid))
}

/// Create a new thread
pub fn create_thread(
    pid: Pid,
    thread_type: ThreadType,
    start_routine: Option<unsafe extern "C" fn(*mut u8) -> *mut u8>,
    arg: *mut u8,
) -> Result<Tid, ThreadError> {
    let mut table = thread_table();
    let thread = table.alloc_thread(pid, thread_type)?;

    // Set thread entry point
    thread.start_routine = start_routine;
    thread.arg = arg;
    thread.state = ThreadState::Runnable;

    crate::println!("thread: Created thread {} for process {}", thread.tid, pid);

    Ok(thread.tid)
}

/// Exit current thread
pub fn thread_exit(retval: *mut u8) -> ! {
    if let Some(tid) = current_thread() {
        let mut table = thread_table();
        if let Some(thread) = table.find_thread(tid) {
            thread.return_value = retval;

            // Handle CLONE_CHILD_CLEARTID: clear the TID pointer on exit
            if thread.child_tid_ptr != 0 {
                // Get current process for memory access
                if let Some(pid) = crate::process::myproc() {
                    let proc_table = crate::process::manager::PROC_TABLE.lock();
                    if let Some(proc) = proc_table.find_ref(pid) {
                        let pagetable = proc.pagetable;
                        if !pagetable.is_null() {
                            // Clear the child TID pointer (set to 0)
                            unsafe {
                                let zero_val = 0i32;
                                let _ = crate::subsystems::mm::vm::copyin(pagetable, thread.child_tid_ptr as *mut u8, thread.child_tid_ptr, core::mem::size_of::<i32>());
                            }
                        }
                    }
                }
            }

            if thread.detached {
                // Detached thread - clean up immediately
                let _ = table.free_thread(tid);
            } else {
                // Joinable thread - become zombie
                thread.state = ThreadState::Zombie;

                // Wake up any thread waiting to join
                if let Some(joiner_tid) = thread.joiner_tid {
                    if let Some(joiner) = table.find_thread(joiner_tid) {
                        joiner.wake();
                    }
                }
            }
        }
    }

    // Schedule next thread
    schedule();

    // Should never reach here
    unreachable!();
}

/// Join with a thread
pub fn thread_join(target_tid: Tid) -> Result<*mut u8, ThreadError> {
    let current_tid = current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let mut table = thread_table();

    // Find target thread
    let target_thread = table.find_thread(target_tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    // Check if thread is joinable
    if target_thread.detached {
        return Err(ThreadError::InvalidOperation);
    }

    // Check if caller is allowed to join
    if target_thread.parent_tid != Some(current_tid) {
        return Err(ThreadError::PermissionDenied);
    }

    // If thread already terminated, collect return value
    if target_thread.state == ThreadState::Zombie {
        let retval = target_thread.return_value;
        table.free_thread(target_tid)?;
        return Ok(retval);
    }

    // Wait for thread to terminate
    let current_thread = table.find_thread(current_tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    current_thread.joiner_tid = Some(target_tid);
    current_thread.block(target_tid as usize);

    // Schedule and wait
    schedule();

    // When we wake up, thread should be terminated
    if let Some(target_thread) = table.find_thread_ref(target_tid) {
        if target_thread.state == ThreadState::Zombie {
            let retval = target_thread.return_value;
            table.free_thread(target_tid)?;
            Ok(retval)
        } else {
            Err(ThreadError::ThreadKilled)
        }
    } else {
        Err(ThreadError::InvalidThreadId)
    }
}

/// Detach a thread
pub fn thread_detach(tid: Tid) -> Result<(), ThreadError> {
    let mut table = thread_table();
    let thread = table.find_thread(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    if thread.detached {
        return Ok(()); // Already detached
    }

    thread.detached = true;
    thread.parent_tid = None;

    // If thread already terminated, clean it up
    if thread.state == ThreadState::Zombie {
        table.free_thread(tid)?;
    }

    Ok(())
}

/// Yield CPU to another thread
pub fn thread_yield() {
    if let Some(tid) = current_thread() {
        let mut table = thread_table();
        if let Some(thread) = table.find_thread(tid) {
            if thread.is_running() {
                thread.set_runnable();
            }
        }
    }

    schedule();
}

/// Thread scheduler - integrated with process scheduling
/// Real-time aware: prioritizes RT threads (FIFO/RR) over normal threads
pub fn schedule() {
    let current_tid = current_thread();

    // First, create main threads for any processes that don't have threads yet
    ensure_main_threads();

    // Check real-time scheduler first
    let current_time = crate::subsystems::time::timestamp_nanos();
    let mut next_tid = None;
    
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        next_tid = rt_scheduler.pick_next_rt_task(current_time);
    }
    
    // If no RT thread found, fall back to unified scheduler
    if next_tid.is_none() {
        // Use unified scheduler with priority queues (O(log n) instead of O(n))
        use crate::subsystems::scheduler::unified::unified_schedule;
        if let Some(tid) = unified_schedule() {
            next_tid = Some(tid);
        } else {
            // Fallback to old linear search if unified scheduler not initialized
            let mut start_idx = current_tid.unwrap_or(0) + 1;
            let table = thread_table();
            for _ in 0..MAX_THREADS {
                if start_idx >= MAX_THREADS {
                    start_idx = 1; // Skip TID 0 (invalid)
                }

                if let Some(thread) = table.find_thread_ref(start_idx) {
                    if thread.is_runnable() && thread.can_run_on_cpu(crate::cpu::cpuid()) {
                        next_tid = Some(start_idx);
                        break;
                    }
                }

                start_idx += 1;
            }
        }
    }

    // Switch to next thread
    if let Some(tid) = next_tid {
        // Update CPU load statistics (switching from idle to running)
        let cpu = crate::cpu::mycpu();
        cpu.update_load_stats(false);
        cpu.load_stats.context_switches += 1;
        
        let mut table = thread_table();
        if let Some(thread) = table.find_thread(tid) {
            thread.set_running();
            set_current_thread(Some(thread.tid));

            // Update the corresponding process state
            update_process_state(thread.pid, crate::process::ProcState::Running);

            // Update CPU's current process
            cpu.proc = Some(thread.pid);

            // Set up TLS for the new thread
            if thread.tls_base != 0 {
                #[cfg(target_arch = "x86_64")]
                {
                    unsafe {
                        core::arch::asm!("wrfsbase {}", in(reg) thread.tls_base);
                        thread.fs_base = thread.tls_base;
                    }
                }
                // TODO: Add TLS setup for other architectures
            }

            // Handle real-time scheduler context switch
            if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
                rt_scheduler.handle_context_switch(tid, current_time);
            }

            // Perform context switch using the new context switch mechanism
            if let Some(current_thread) = current_tid.and_then(|tid| table.find_thread(tid)) {
                // Check if we're switching between threads of the same process
                let same_process = current_thread.pid == thread.pid;
                
                // Use fast path if same process, otherwise use full context switch
                let result = if same_process {
                    unsafe { 
                        crate::subsystems::process::context_switch::fast_context_switch(
                            &mut current_thread.context, 
                            &thread.context, 
                            true
                        )
                    }
                } else {
                    unsafe { 
                        crate::subsystems::process::context_switch::context_switch(
                            &mut current_thread.context, 
                            &thread.context
                        )
                    }
                };
                
                if let Err(e) = result {
                    crate::println!("thread: Context switch failed: {:?}", e);
                    // Fall back to simple logging
                    crate::println!("thread: Switched to thread {} (PID {})", thread.tid, thread.pid);
                }
            } else {
                // No current thread, just set up the new thread
                crate::println!("thread: Switched to thread {} (PID {})", thread.tid, thread.pid);
            }
        }
    } else {
        // No runnable threads - optimize idle behavior
        set_current_thread(None);
        
        // Update CPU load statistics
        let cpu_id = crate::cpu::cpuid();
        let cpu = crate::cpu::mycpu();
        cpu.update_load_stats(true);
        
        // Check if we should enter deep sleep
        if cpu.should_deep_sleep() {
            // Enter deep sleep mode (only for non-boot CPUs)
            if cpu_id > 0 {
                cpu.deep_sleep.store(true, core::sync::atomic::Ordering::Relaxed);
                // In a full implementation, this would use architecture-specific
                // deep sleep instructions (e.g., WFI with power management)
                crate::arch::wfi();
                cpu.deep_sleep.store(false, core::sync::atomic::Ordering::Relaxed);
            } else {
                // Boot CPU should use regular WFI
                crate::arch::wfi();
            }
        } else {
            // Regular idle - use WFI
            crate::arch::wfi();
        }
    }
}

/// Find highest priority real-time thread
/// Returns the TID of the highest priority RT thread, or None if none found
fn find_realtime_thread(current_tid: Option<Tid>) -> Option<Tid> {
    let table = thread_table();
    let mut highest_prio_rt: Option<(Tid, u8)> = None;
    
    // Search for RT threads (FIFO or RoundRobin policy)
    for tid in 1..MAX_THREADS {
        if let Some(thread) = table.find_thread_ref(tid) {
            // Check if thread is runnable and RT
            if thread.is_runnable() && thread.can_run_on_cpu(crate::cpu::cpuid()) {
                match thread.sched_policy {
                    SchedPolicy::Fifo | SchedPolicy::RoundRobin => {
                        let priority = thread.sched_param.priority;
                        // RT priorities are 1-99, higher = more important
                        if let Some((_, current_prio)) = highest_prio_rt {
                            if priority > current_prio {
                                highest_prio_rt = Some((tid, priority));
                            }
                        } else {
                            highest_prio_rt = Some((tid, priority));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    highest_prio_rt.map(|(tid, _)| tid)
}

/// Ensure each process has at least one main thread
fn ensure_main_threads() {
    // Collect process info first to avoid borrowing conflicts
    let process_pids = {
        let process_table = crate::process::PROC_TABLE.lock();
        let thread_table = thread_table();

        process_table.iter()
            .filter(|proc| proc.state == crate::process::ProcState::Runnable ||
                          proc.state == crate::process::ProcState::Running)
            .filter_map(|proc| {
                // Check if process already has threads
                let has_threads = thread_table.find_threads_by_pid(proc.pid).iter().any(|t| {
                    t.thread_type == ThreadType::Main && t.state != ThreadState::Unused
                });

                if !has_threads {
                    Some(proc.pid)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    // Create main threads for processes that need them
    for pid in process_pids {
        if let Ok(tid) = create_thread(pid, ThreadType::Main, None, null_mut()) {
            let mut table = thread_table();
            if let Some(thread) = table.find_thread(tid) {
                // Initialize main thread context from process
                let mut process_table = crate::process::PROC_TABLE.lock();
                if let Some(proc) = process_table.find(pid) {
                    init_main_thread_from_process(thread, proc);
                }
            }
        }
    }
}

/// Initialize main thread from process data
fn init_main_thread_from_process(thread: &mut Thread, proc: &crate::process::Proc) {
    // Copy process context to thread
    thread.context = proc.context;

    // Set up trapframe
    if !proc.trapframe.is_null() {
        unsafe {
            *thread.trapframe = *proc.trapframe;
        }
    }

    // Initialize context for main thread
    crate::subsystems::process::context_switch::init_context(
        &mut thread.context,
        thread.kstack,
        proc.context.rip, // Use process's instruction pointer
        0, // No argument for main thread
        false // Kernel thread
    );

    // Main threads inherit the process state
    thread.set_runnable();
}

/// Update process state based on thread state
fn update_process_state(pid: crate::process::Pid, new_state: crate::process::ProcState) {
    let mut process_table = crate::process::PROC_TABLE.lock();
    if let Some(proc) = process_table.find(pid) {
        proc.state = new_state;
    }
}

/// Send cancel request to thread
pub fn thread_cancel(tid: Tid) -> Result<(), ThreadError> {
    // Use enhanced cancellation mechanism
    crate::subsystems::process::thread_cancellation::cancel_thread(tid)
}

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
    get_current_thread()
        .map(|t| t.tls_base)
        .unwrap_or(0)
}

/// Set thread CPU affinity
pub fn thread_setaffinity(tid: Tid, cpu_mask: u64) -> Result<(), ThreadError> {
    let mut table = thread_table();
    let thread = table.find_thread(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    thread.set_cpu_affinity(cpu_mask);
    Ok(())
}

/// Get thread CPU affinity
pub fn thread_getaffinity(tid: Tid) -> Result<u64, ThreadError> {
    let table = thread_table();
    let thread = table.find_thread_ref(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    Ok(thread.cpus_allowed)
}

/// Set thread scheduling policy and parameters
pub fn thread_setschedparam(
    tid: Tid,
    policy: SchedPolicy,
    param: SchedParam,
) -> Result<(), ThreadError> {
    let mut table = thread_table();
    let thread = table.find_thread(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

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
                bandwidth_percent: (param.priority as u32 * 10).min(80), // Scale priority to bandwidth
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
    let thread = table.find_thread_ref(tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    Ok((thread.sched_policy, thread.sched_param))
}

/// Activate a real-time task
pub fn activate_rt_task(tid: Tid) -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        let current_time = crate::subsystems::time::timestamp_nanos();
        rt_scheduler.activate_task(tid, current_time)
            .map_err(|_| ThreadError::InvalidOperation)
    } else {
        Err(ThreadError::InvalidOperation)
    }
}

/// Deactivate a real-time task
pub fn deactivate_rt_task(tid: Tid) -> Result<(), ThreadError> {
    if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
        rt_scheduler.deactivate_task(tid)
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
            _ => {}
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
