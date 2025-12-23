//! Process management interface

use crate::error::Result;
use crate::core::types::{Pid, Uid, Gid, ProcessState, ThreadState, SchedulingPolicy};
use alloc::vec::Vec;

/// Trait for process manager
pub trait ProcessManager {
    /// Creates a new process
    fn create_process(&mut self, name: &str) -> Result<Pid>;
    
    /// Terminates a process
    fn terminate_process(&mut self, pid: Pid) -> Result<()>;
    
    /// Finds a process by ID
    fn find_process(&self, pid: Pid) -> Option<&dyn Process>;
    
    /// Finds a mutable process by ID
    fn find_process_mut(&mut self, pid: Pid) -> Option<&mut dyn Process>;
    
    /// Lists all processes
    fn list_processes(&self) -> Vec<&dyn Process>;
    
    /// Returns the current process
    fn current_process(&self) -> Option<&dyn Process>;
    
    /// Returns the current process ID
    fn current_pid(&self) -> Option<Pid>;
    
    /// Returns the parent process ID
    fn parent_pid(&self, pid: Pid) -> Option<Pid>;
    
    /// Returns the children of a process
    fn child_processes(&self, pid: Pid) -> Vec<Pid>;
    
    /// Returns the number of processes
    fn process_count(&self) -> usize;
    
    /// Returns the maximum process ID
    fn max_pid(&self) -> Pid;
}

/// Trait for scheduler
pub trait Scheduler {
    /// Schedules the next process
    fn schedule_next(&mut self) -> Option<Pid>;
    
    /// Adds a process to the scheduler
    fn add_process(&mut self, pid: Pid, priority: u32) -> Result<()>;
    
    /// Removes a process from the scheduler
    fn remove_process(&mut self, pid: Pid) -> Result<()>;
    
    /// Changes the priority of a process
    fn set_priority(&mut self, pid: Pid, priority: u32) -> Result<()>;
    
    /// Gets the priority of a process
    fn get_priority(&self, pid: Pid) -> Option<u32>;
    
    /// Yields the current process
    fn yield_process(&mut self) -> Result<()>;
    
    /// Blocks the current process
    fn block_process(&mut self, pid: Pid) -> Result<()>;
    
    /// Unblocks a process
    fn unblock_process(&mut self, pid: Pid) -> Result<()>;
    
    /// Returns the current scheduling policy
    fn scheduling_policy(&self) -> SchedulingPolicy;
    
    /// Sets the scheduling policy
    fn set_scheduling_policy(&mut self, policy: SchedulingPolicy) -> Result<()>;
    
    /// Returns the number of runnable processes
    fn runnable_count(&self) -> usize;
    
    /// Returns the number of blocked processes
    fn blocked_count(&self) -> usize;
}

/// Trait for process
pub trait Process {
    /// Returns the process ID
    fn pid(&self) -> Pid;
    
    /// Returns the parent process ID
    fn ppid(&self) -> Pid;
    
    /// Returns the process name
    fn name(&self) -> &str;
    
    /// Returns the process state
    fn state(&self) -> ProcessState;
    
    /// Sets the process state
    fn set_state(&mut self, state: ProcessState);
    
    /// Returns the user ID
    fn uid(&self) -> Uid;
    
    /// Sets the user ID
    fn set_uid(&mut self, uid: Uid);
    
    /// Returns the group ID
    fn gid(&self) -> Gid;
    
    /// Sets the group ID
    fn set_gid(&mut self, gid: Gid);
    
    /// Returns the process priority
    fn priority(&self) -> u32;
    
    /// Sets the process priority
    fn set_priority(&mut self, priority: u32);
    
    /// Returns the number of threads
    fn thread_count(&self) -> usize;
    
    /// Returns the threads of the process
    fn threads(&self) -> Vec<&dyn Thread>;
    
    /// Returns the mutable threads of the process
    fn threads_mut(&mut self) -> Vec<&mut dyn Thread>;
    
    /// Creates a new thread
    fn create_thread(&mut self, name: &str) -> Result<Pid>;
    
    /// Terminates a thread
    fn terminate_thread(&mut self, tid: Pid) -> Result<()>;
    
    /// Returns the process memory usage
    fn memory_usage(&self) -> ProcessMemoryUsage;
    
    /// Returns the process CPU usage
    fn cpu_usage(&self) -> ProcessCpuUsage;
    
    /// Returns the process I/O usage
    fn io_usage(&self) -> ProcessIoUsage;
    
    /// Returns the process creation time
    fn creation_time(&self) -> u64;
    
    /// Returns the process execution time
    fn execution_time(&self) -> u64;
    
    /// Returns the process user time
    fn user_time(&self) -> u64;
    
    /// Returns the process system time
    fn system_time(&self) -> u64;
}

/// Trait for thread
pub trait Thread {
    /// Returns the thread ID
    fn tid(&self) -> Pid;
    
    /// Returns the parent process ID
    fn pid(&self) -> Pid;
    
    /// Returns the thread name
    fn name(&self) -> &str;
    
    /// Returns the thread state
    fn state(&self) -> ThreadState;
    
    /// Sets the thread state
    fn set_state(&mut self, state: ThreadState);
    
    /// Returns the thread priority
    fn priority(&self) -> u32;
    
    /// Sets the thread priority
    fn set_priority(&mut self, priority: u32);
    
    /// Returns the thread stack pointer
    fn stack_pointer(&self) -> usize;
    
    /// Returns the thread instruction pointer
    fn instruction_pointer(&self) -> usize;
    
    /// Returns the thread CPU affinity
    fn cpu_affinity(&self) -> u32;
    
    /// Sets the thread CPU affinity
    fn set_cpu_affinity(&mut self, affinity: u32);
    
    /// Yields the thread
    fn yield_thread(&mut self) -> Result<()>;
    
    /// Joins the thread
    fn join(&mut self) -> Result<()>;
    
    /// Detaches the thread
    fn detach(&mut self) -> Result<()>;
    
    /// Returns the thread creation time
    fn creation_time(&self) -> u64;
    
    /// Returns the thread execution time
    fn execution_time(&self) -> u64;
    
    /// Returns the thread user time
    fn user_time(&self) -> u64;
    
    /// Returns the thread system time
    fn system_time(&self) -> u64;
}

/// Process memory usage
#[derive(Debug, Clone)]
pub struct ProcessMemoryUsage {
    /// Virtual memory size
    pub virtual_size: usize,
    /// Resident set size
    pub resident_size: usize,
    /// Shared memory size
    pub shared_size: usize,
    /// Text segment size
    pub text_size: usize,
    /// Data segment size
    pub data_size: usize,
    /// Stack segment size
    pub stack_size: usize,
    /// Number of page faults
    pub page_faults: u64,
    /// Number of major page faults
    pub major_page_faults: u64,
    /// Number of minor page faults
    pub minor_page_faults: u64,
}

/// Process CPU usage
#[derive(Debug, Clone)]
pub struct ProcessCpuUsage {
    /// User time in nanoseconds
    pub user_time: u64,
    /// System time in nanoseconds
    pub system_time: u64,
    /// Total time in nanoseconds
    pub total_time: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of voluntary context switches
    pub voluntary_context_switches: u64,
    /// Number of involuntary context switches
    pub involuntary_context_switches: u64,
}

/// Process I/O usage
#[derive(Debug, Clone)]
pub struct ProcessIoUsage {
    /// Number of bytes read
    pub bytes_read: u64,
    /// Number of bytes written
    pub bytes_written: u64,
    /// Number of read operations
    pub read_operations: u64,
    /// Number of write operations
    pub write_operations: u64,
    /// Number of bytes read from storage
    pub storage_bytes_read: u64,
    /// Number of bytes written to storage
    pub storage_bytes_written: u64,
    /// Number of storage read operations
    pub storage_read_operations: u64,
    /// Number of storage write operations
    pub storage_write_operations: u64,
}