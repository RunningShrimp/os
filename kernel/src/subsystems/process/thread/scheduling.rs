
use alloc::vec::Vec;


use super::types::{Thread, Tid, Pid, ThreadState, ThreadType, MAX_THREADS};
use super::table::{ThreadTable, THREAD_TABLE_INIT, THREAD_TABLE};
use super::api::ThreadError;

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
    unsafe {
        CURRENT_THREAD[cpu_id] = tid;
    }
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
    let table = thread_table();
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
        let table = thread_table();
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
                            let child_tid_ptr = thread.child_tid_ptr;
                            let zero_val: i32 = 0;
                            let bytes = unsafe {
                                core::slice::from_raw_parts(&zero_val as *const i32 as *const u8, core::mem::size_of::<i32>())
                            };
                            let _ = crate::subsystems::mm::vm::copyin(
                                child_tid_ptr as *mut u8,
                                bytes,
                                core::mem::size_of::<i32>(),
                            );
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
    let table = thread_table();

    // Find target thread
    let target_thread = table
        .find_thread(target_tid)
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
    let current_thread = table
        .find_thread(current_tid)
        .ok_or(ThreadError::InvalidThreadId)?;

    current_thread.joiner_tid = Some(target_tid);
    current_thread.block();

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
    let table = thread_table();
    let thread = table.find_thread(tid).ok_or(ThreadError::InvalidThreadId)?;

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
        let table = thread_table();
        if let Some(thread) = table.find_thread(tid) {
            if thread.is_running() {
                thread.set_runnable();
            }
        }
    }

    schedule();
}

/// Ensure all processes have their main threads created
///
/// This function checks if any processes exist without threads and creates
/// their main threads if needed. This is called by the scheduler to ensure
/// every process has at least one thread for execution.
fn ensure_main_threads() {
    // Lock the process table and check for processes without threads
    use crate::process::PROC_TABLE;

    // Collect PIDs that need threads first to avoid holding lock across iteration
    let pids_needing_threads: Vec<_> = {
        let process_table = PROC_TABLE.lock();
        let thread_table_ref = thread_table();

        // Iterate through all processes and collect PIDs without threads
        process_table
            .iter()
            .filter_map(|proc| {
                let threads = thread_table_ref.find_threads_by_pid(proc.pid);
                if threads.is_empty() {
                    Some(proc.pid)
                } else {
                    None
                }
            })
            .collect()
    };

    // Now create threads for processes that need them (outside the lock)
    for pid in pids_needing_threads {
        // This process has no threads, we need to create a main thread
        // For now, this is a no-op stub - the actual thread creation
        // would happen during process initialization
        //
        // In a full implementation, we would:
        // 1. Allocate a new thread ID
        // 2. Initialize the thread from process data
        // 3. Add it to the thread table
        // 4. Mark it as runnable

        // Create the main thread for this process
        if let Ok(tid) = create_thread(
            pid,
            ThreadType::Main,
            None,
            core::ptr::null_mut(),
        ) {
            // Get the newly created thread and initialize it from process data
            let table = thread_table();
            if let Some(thread) = table.find_thread(tid) {
                // Re-acquire process table lock for this operation
                // Note: We need mutable access to the process for initialization
                let mut proc_table = crate::process::PROC_TABLE.lock();
                if let Some(proc) = proc_table.find(pid) {
                    init_main_thread_from_process(thread, proc);
                }
            }
        }
    }
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
        use crate::sched::unified::unified_schedule;
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

        let table = thread_table();
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
                // GH-#1334: Add TLS setup for other architectures
                // See: https://github.com/npos/kernel/issues/1334
            }

            // Handle real-time scheduler context switch
            if let Some(rt_scheduler) = crate::subsystems::scheduler::get_rt_scheduler() {
                rt_scheduler.handle_context_switch(tid, current_time);
            }

            // Perform context switch using the new context switch mechanism
            // We need to extract the current thread info before borrowing table again
            let current_thread_info = current_tid.and_then(|tid| {
                let tbl = thread_table();
                tbl.find_thread_ref(tid).map(|t| (t.pid, t.tid))
            });

            if let Some((current_pid, current_tid_val)) = current_thread_info {
                // Check if we're switching between threads of the same process
                let same_process = current_pid == thread.pid;

                // Now we need to get mutable reference to perform the actual switch
                // We create a scope to limit the mutable borrow
                let switch_result = {
                    let tbl = thread_table();
                    let current_thread = tbl.find_thread(current_tid_val);
                    match current_thread {
                        Some(ct) => {
                            // Use fast path if same process, otherwise use full context switch
                            if same_process {
                                unsafe {
                                    crate::subsystems::process::context_switch::fast_context_switch(
                                        &mut ct.context,
                                        &thread.context,
                                        true,
                                    )
                                }
                            } else {
                                unsafe {
                                    crate::subsystems::process::context_switch::context_switch(
                                        &mut ct.context,
                                        &thread.context,
                                    )
                                }
                            }
                        },
                        None => {
                            crate::println!("thread: Switched to thread {} (PID {})", thread.tid, thread.pid);
                            Ok(())
                        },
                    }
                };

                if let Err(e) = switch_result {
                    crate::println!("thread: Context switch failed: {:?}", e);
                    // Fall back to simple logging
                    crate::println!(
                        "thread: Switched to thread {} (PID {})",
                        thread.tid,
                        thread.pid
                    );
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
                cpu.deep_sleep
                    .store(true, core::sync::atomic::Ordering::Relaxed);
                // In a full implementation, this would use architecture-specific
                // deep sleep instructions (e.g., WFI with power management)
                crate::arch::wfi();
                cpu.deep_sleep
                    .store(false, core::sync::atomic::Ordering::Relaxed);
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
    #[cfg(target_arch = "x86_64")]
    let instruction_ptr = proc.context.rip;
    #[cfg(target_arch = "riscv64")]
    let instruction_ptr = proc.context.ra;
    #[cfg(target_arch = "aarch64")]
    let instruction_ptr = proc.context.lr;

    crate::subsystems::process::context_switch::init_context(
        &mut thread.context,
        thread.kstack,
        instruction_ptr, // Use process's instruction pointer
        0,               // No argument for main thread
        false,           // Kernel thread
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
