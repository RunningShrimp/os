//! Process Management API Interface
//!
//! This module defines trait interfaces for process management.
//! It provides abstractions that separate process management interface
//! from its implementation, helping to break circular dependencies
//! between modules.

use alloc::string::String;
use alloc::vec::Vec;
use crate::types::stubs::{pid_t, uid_t, gid_t};
use crate::posix::mode_t;
use crate::error::unified_framework::{FrameworkError, IntoFrameworkError, FrameworkResult};
use crate::error::unified::UnifiedError;

/// Process manager trait
///
/// This trait defines the interface for process management operations.
/// It provides methods for creating, managing, and querying processes.
pub trait ProcessManager {
    /// Get the current process ID
    ///
    /// # Returns
    /// * `pid_t` - Current process ID
    fn get_current_pid(&self) -> pid_t;

    /// Get the current process
    ///
    /// # Returns
    /// * `Option<&Process>` - Current process if exists
    fn get_current_process(&self) -> Option<&dyn Process>;

    /// Get a process by ID
    ///
    /// # Arguments
    /// * `pid` - Process ID
    ///
    /// # Returns
    /// * `Option<&dyn Process>` - Process if exists
    fn get_process(&self, pid: pid_t) -> Option<&dyn Process>;

    /// Create a new process
    ///
    /// # Arguments
    /// * `config` - Process configuration
    ///
    /// # Returns
    /// * `Ok(pid_t)` - New process ID
    /// * `Err(ProcessError)` - Process creation error
    fn create_process(&self, config: ProcessConfig) -> Result<pid_t, ProcessError>;

    /// Fork the current process
    ///
    /// # Returns
    /// * `Ok(pid_t)` - Child process ID in parent, 0 in child
    /// * `Err(ProcessError)` - Fork error
    fn fork_process(&self) -> Result<pid_t, ProcessError>;

    /// Execute a new program
    ///
    /// # Arguments
    /// * `pid` - Process ID
    /// * `path` - Path to executable
    /// * `args` - Program arguments
    /// * `env` - Environment variables
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(ProcessError)` - Exec error
    fn exec_process(&self, pid: pid_t, path: &str, args: &[&str], env: &[&str]) -> Result<(), ProcessError>;

    /// Exit a process
    ///
    /// # Arguments
    /// * `pid` - Process ID
    /// * `exit_code` - Exit code
    fn exit_process(&self, pid: pid_t, exit_code: i32);

    /// Wait for a process to exit
    ///
    /// # Arguments
    /// * `pid` - Process ID to wait for
    /// * `options` - Wait options
    ///
    /// # Returns
    /// * `Ok(ExitStatus)` - Process exit status
    /// * `Err(ProcessError)` - Wait error
    fn wait_process(&self, pid: pid_t, options: WaitOptions) -> Result<ExitStatus, ProcessError>;

    /// Send a signal to a process
    ///
    /// # Arguments
    /// * `pid` - Process ID
    /// * `signal` - Signal number
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(ProcessError)` - Signal error
    fn signal_process(&self, pid: pid_t, signal: i32) -> Result<(), ProcessError>;

    /// Get all processes
    ///
    /// # Returns
    /// * `Vec<&dyn Process>` - All processes
    fn get_all_processes(&self) -> Vec<&dyn Process>;

    /// Schedule a process
    ///
    /// # Arguments
    /// * `pid` - Process ID
    fn schedule_process(&self, pid: pid_t);

    /// Get process statistics
    ///
    /// # Returns
    /// * `ProcessStats` - Process statistics
    fn get_process_stats(&self) -> ProcessStats;
}

/// Process trait
///
/// This trait defines the interface for a process.
/// It provides methods for querying and modifying process state.
pub trait Process {
    /// Get the process ID
    ///
    /// # Returns
    /// * `pid_t` - Process ID
    fn get_pid(&self) -> pid_t;

    /// Get the parent process ID
    ///
    /// # Returns
    /// * `pid_t` - Parent process ID
    fn get_ppid(&self) -> pid_t;

    /// Get the process state
    ///
    /// # Returns
    /// * `ProcessState` - Process state
    fn get_state(&self) -> ProcessState;

    /// Set the process state
    ///
    /// # Arguments
    /// * `state` - New process state
    fn set_state(&mut self, state: ProcessState);

    /// Get the user ID
    ///
    /// # Returns
    /// * `uid_t` - User ID
    fn get_uid(&self) -> uid_t;

    /// Set the user ID
    ///
    /// # Arguments
    /// * `uid` - New user ID
    fn set_uid(&mut self, uid: uid_t);

    /// Get the effective user ID
    ///
    /// # Returns
    /// * `uid_t` - Effective user ID
    fn get_euid(&self) -> uid_t;

    /// Set the effective user ID
    ///
    /// # Arguments
    /// * `euid` - New effective user ID
    fn set_euid(&mut self, euid: uid_t);

    /// Get the group ID
    ///
    /// # Returns
    /// * `gid_t` - Group ID
    fn get_gid(&self) -> gid_t;

    /// Set the group ID
    ///
    /// # Arguments
    /// * `gid` - New group ID
    fn set_gid(&mut self, gid: gid_t);

    /// Get the effective group ID
    ///
    /// # Returns
    /// * `gid_t` - Effective group ID
    fn get_egid(&self) -> gid_t;

    /// Set the effective group ID
    ///
    /// # Arguments
    /// * `egid` - New effective group ID
    fn set_egid(&mut self, egid: gid_t);

    /// Get the working directory
    ///
    /// # Returns
    /// * `&str` - Working directory path
    fn get_working_directory(&self) -> &str;

    /// Set the working directory
    ///
    /// # Arguments
    /// * `path` - New working directory path
    fn set_working_directory(&mut self, path: &str) -> Result<(), ProcessError>;

    /// Get the command line
    ///
    /// # Returns
    /// * `&str` - Command line
    fn get_command_line(&self) -> &str;

    /// Get the process name
    ///
    /// # Returns
    /// * `&str` - Process name
    fn get_name(&self) -> &str;

    /// Get the process arguments
    ///
    /// # Returns
    /// * `&[&str]` - Process arguments
    fn get_args(&self) -> &[&str];

    /// Get the environment variables
    ///
    /// # Returns
    /// * `&[(&str, &str)]` - Environment variables
    fn get_env(&self) -> &[(&str, &str)];

    /// Get the file descriptor table
    ///
    /// # Returns
    /// * `&FileDescriptorTable` - File descriptor table
    fn get_fd_table(&self) -> &FileDescriptorTable;

    /// Get the memory map
    ///
    /// # Returns
    /// * `&MemoryMap` - Memory map
    fn get_memory_map(&self) -> &MemoryMap;

    /// Get the creation time
    ///
    /// # Returns
    /// * `u64` - Creation time (timestamp)
    fn get_creation_time(&self) -> u64;

    /// Get the CPU time used
    ///
    /// # Returns
    /// * `u64` - CPU time used (in nanoseconds)
    fn get_cpu_time(&self) -> u64;

    /// Get the memory usage
    ///
    /// # Returns
    /// * `usize` - Memory usage (in bytes)
    fn get_memory_usage(&self) -> usize;

    /// Check if the process is alive
    ///
    /// # Returns
    /// * `bool` - True if alive, false otherwise
    fn is_alive(&self) -> bool;

    /// Get the exit code
    ///
    /// # Returns
    /// * `Option<i32>` - Exit code if exited
    fn get_exit_code(&self) -> Option<i32>;

    /// Get the signal that caused the process to exit
    ///
    /// # Returns
    /// * `Option<i32>` - Signal number if killed by signal
    fn get_exit_signal(&self) -> Option<i32>;
}

/// Thread manager trait
///
/// This trait defines the interface for thread management operations.
pub trait ThreadManager {
    /// Create a new thread
    ///
    /// # Arguments
    /// * `config` - Thread configuration
    ///
    /// # Returns
    /// * `Ok(tid_t)` - New thread ID
    /// * `Err(ThreadError)` - Thread creation error
    fn create_thread(&self, config: ThreadConfig) -> Result<u64, ThreadError>;

    /// Get the current thread ID
    ///
    /// # Returns
    /// * `u64` - Current thread ID
    fn get_current_thread_id(&self) -> u64;

    /// Get a thread by ID
    ///
    /// # Arguments
    /// * `tid` - Thread ID
    ///
    /// # Returns
    /// * `Option<&dyn Thread>` - Thread if exists
    fn get_thread(&self, tid: u64) -> Option<&dyn Thread>;

    /// Exit a thread
    ///
    /// # Arguments
    /// * `tid` - Thread ID
    /// * `exit_code` - Exit code
    fn exit_thread(&self, tid: u64, exit_code: i32);

    /// Join a thread
    ///
    /// # Arguments
    /// * `tid` - Thread ID
    ///
    /// # Returns
    /// * `Ok(i32)` - Thread exit code
    /// * `Err(ThreadError)` - Join error
    fn join_thread(&self, tid: u64) -> Result<i32, ThreadError>;

    /// Get all threads
    ///
    /// # Returns
    /// * `Vec<&dyn Thread>` - All threads
    fn get_all_threads(&self) -> Vec<&dyn Thread>;
}

/// Thread trait
///
/// This trait defines the interface for a thread.
pub trait Thread {
    /// Get the thread ID
    ///
    /// # Returns
    /// * `u64` - Thread ID
    fn get_id(&self) -> u64;

    /// Get the process ID
    ///
    /// # Returns
    /// * `pid_t` - Process ID
    fn get_process_id(&self) -> pid_t;

    /// Get the thread state
    ///
    /// # Returns
    /// * `ThreadState` - Thread state
    fn get_state(&self) -> ThreadState;

    /// Set the thread state
    ///
    /// # Arguments
    /// * `state` - New thread state
    fn set_state(&mut self, state: ThreadState);

    /// Get the stack pointer
    ///
    /// # Returns
    /// * `usize` - Stack pointer
    fn get_stack_pointer(&self) -> usize;

    /// Get the instruction pointer
    ///
    /// # Returns
    /// * `usize` - Instruction pointer
    fn get_instruction_pointer(&self) -> usize;

    /// Get the CPU time used
    ///
    /// # Returns
    /// * `u64` - CPU time used (in nanoseconds)
    fn get_cpu_time(&self) -> u64;

    /// Check if the thread is alive
    ///
    /// # Returns
    /// * `bool` - True if alive, false otherwise
    fn is_alive(&self) -> bool;

    /// Get the exit code
    ///
    /// # Returns
    /// * `Option<i32>` - Exit code if exited
    fn get_exit_code(&self) -> Option<i32>;
}

/// Process state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessState {
    /// Process is being created
    Creating,
    /// Process is ready to run
    Ready,
    /// Process is currently running
    Running,
    /// Process is waiting for an event
    Waiting,
    /// Process has been suspended
    Suspended,
    /// Process has been stopped
    Stopped,
    /// Process has exited
    Zombie,
    /// Process is dead
    Dead,
}

/// Thread state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadState {
    /// Thread is being created
    Creating,
    /// Thread is ready to run
    Ready,
    /// Thread is currently running
    Running,
    /// Thread is waiting for an event
    Waiting,
    /// Thread has been suspended
    Suspended,
    /// Thread has been stopped
    Stopped,
    /// Thread has exited
    Zombie,
    /// Thread is dead
    Dead,
}

/// Process configuration
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Process name
    pub name: String,
    /// Command line arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Working directory
    pub working_directory: String,
    /// User ID
    pub uid: uid_t,
    /// Group ID
    pub gid: gid_t,
    /// File mode mask
    pub umask: mode_t,
    /// Standard input file descriptor
    pub stdin: Option<i32>,
    /// Standard output file descriptor
    pub stdout: Option<i32>,
    /// Standard error file descriptor
    pub stderr: Option<i32>,
}

/// Thread configuration
#[derive(Debug, Clone)]
pub struct ThreadConfig {
    /// Thread function
    pub function: fn(),
    /// Thread argument
    pub argument: usize,
    /// Stack size
    pub stack_size: usize,
    /// Thread priority
    pub priority: u8,
    /// Thread is detached
    pub detached: bool,
}

/// Wait options
#[derive(Debug, Clone, Copy)]
pub struct WaitOptions {
    /// Don't block if no child has exited
    pub nohang: bool,
    /// Wait for children in the same process group
    pub same_process_group: bool,
    /// Wait for children that have been stopped
    pub stopped: bool,
    /// Wait for children that have continued
    pub continued: bool,
    /// Wait for any child
    pub any_child: bool,
}

/// Exit status
#[derive(Debug, Clone, Copy)]
pub struct ExitStatus {
    /// Process ID
    pub pid: pid_t,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Signal that caused the process to exit
    pub signal: Option<i32>,
    /// Process was stopped by a signal
    pub stopped: bool,
    /// Process was continued
    pub continued: bool,
    /// Process core dumped
    pub core_dumped: bool,
}

/// Process statistics
#[derive(Debug, Clone)]
pub struct ProcessStats {
    /// Total number of processes
    pub total_processes: usize,
    /// Number of running processes
    pub running_processes: usize,
    /// Number of sleeping processes
    pub sleeping_processes: usize,
    /// Number of stopped processes
    pub stopped_processes: usize,
    /// Number of zombie processes
    pub zombie_processes: usize,
    /// Total memory usage
    pub total_memory_usage: usize,
    /// Total CPU usage
    pub total_cpu_usage: u64,
}

/// File descriptor table
#[derive(Debug)]
pub struct FileDescriptorTable {
    /// File descriptors
    pub descriptors: Vec<Option<FileDescriptor>>,
}

/// File descriptor
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// File descriptor number
    pub fd: i32,
    /// File path
    pub path: String,
    /// Open flags
    pub flags: i32,
    /// File mode
    pub mode: mode_t,
    /// Current offset
    pub offset: u64,
}

/// Memory map
#[derive(Debug)]
pub struct MemoryMap {
    /// Memory regions
    pub regions: Vec<MemoryRegion>,
}

/// Memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Start address
    pub start: usize,
    /// End address
    pub end: usize,
    /// Memory permissions
    pub permissions: MemoryPermissions,
    /// Memory type
    pub region_type: MemoryRegionType,
    /// Memory region name
    pub name: String,
}

/// Memory permissions
#[derive(Debug, Clone, Copy)]
pub struct MemoryPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

/// Memory region type
#[derive(Debug, Clone, Copy)]
pub enum MemoryRegionType {
    /// Anonymous memory
    Anonymous,
    /// File-backed memory
    File,
    /// Stack memory
    Stack,
    /// Heap memory
    Heap,
    /// Code memory
    Code,
    /// Data memory
    Data,
    /// Shared memory
    Shared,
}

/// Process error
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessError {
    /// Invalid process ID
    InvalidPid,
    /// Process not found
    ProcessNotFound,
    /// Permission denied
    PermissionDenied,
    /// Resource not available
    ResourceUnavailable,
    /// Invalid arguments
    InvalidArguments,
    /// Out of memory
    OutOfMemory,
    /// Operation not supported
    NotSupported,
    /// Process already exists
    ProcessExists,
    /// Process is not a child
    NotChild,
    /// Process is already running
    AlreadyRunning,
    /// Process is not running
    NotRunning,
    /// Process is not stopped
    NotStopped,
    /// Process is not zombie
    NotZombie,
    /// Unknown error
    Unknown,
}

/// Thread error
#[derive(Debug, Clone, PartialEq)]
pub enum ThreadError {
    /// Invalid thread ID
    InvalidTid,
    /// Thread not found
    ThreadNotFound,
    /// Permission denied
    PermissionDenied,
    /// Resource not available
    ResourceUnavailable,
    /// Invalid arguments
    InvalidArguments,
    /// Out of memory
    OutOfMemory,
    /// Operation not supported
    NotSupported,
    /// Thread already exists
    ThreadExists,
    /// Thread is not a child
    NotChild,
    /// Thread is already running
    AlreadyRunning,
    /// Thread is not running
    NotRunning,
    /// Thread is not stopped
    NotStopped,
    /// Thread is not zombie
    NotZombie,
    /// Unknown error
    Unknown,
}

impl IntoFrameworkError for ProcessError {
    fn into_framework_error(self) -> FrameworkError {
        match self {
            ProcessError::InvalidPid => UnifiedError::InvalidArgument.into_framework_error(),
            ProcessError::ProcessNotFound => UnifiedError::NotFound.into_framework_error(),
            ProcessError::PermissionDenied => UnifiedError::PermissionDenied.into_framework_error(),
            ProcessError::ResourceUnavailable => UnifiedError::ResourceUnavailable.into_framework_error(),
            ProcessError::InvalidArguments => UnifiedError::InvalidArgument.into_framework_error(),
            ProcessError::OutOfMemory => UnifiedError::OutOfMemory.into_framework_error(),
            ProcessError::NotSupported => UnifiedError::NotSupported.into_framework_error(),
            ProcessError::ProcessExists => UnifiedError::AlreadyExists.into_framework_error(),
            ProcessError::NotChild => UnifiedError::InvalidArgument.into_framework_error(),
            ProcessError::AlreadyRunning => UnifiedError::InvalidState.into_framework_error(),
            ProcessError::NotRunning => UnifiedError::InvalidState.into_framework_error(),
            ProcessError::NotStopped => UnifiedError::InvalidState.into_framework_error(),
            ProcessError::NotZombie => UnifiedError::InvalidState.into_framework_error(),
            ProcessError::Unknown => UnifiedError::Unknown.into_framework_error(),
        }
    }
    
    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        match self {
            ProcessError::InvalidPid => UnifiedError::InvalidArgument.with_context(context, location),
            ProcessError::ProcessNotFound => UnifiedError::NotFound.with_context(context, location),
            ProcessError::PermissionDenied => UnifiedError::PermissionDenied.with_context(context, location),
            ProcessError::ResourceUnavailable => UnifiedError::ResourceUnavailable.with_context(context, location),
            ProcessError::InvalidArguments => UnifiedError::InvalidArgument.with_context(context, location),
            ProcessError::OutOfMemory => UnifiedError::OutOfMemory.with_context(context, location),
            ProcessError::NotSupported => UnifiedError::NotSupported.with_context(context, location),
            ProcessError::ProcessExists => UnifiedError::AlreadyExists.with_context(context, location),
            ProcessError::NotChild => UnifiedError::InvalidArgument.with_context(context, location),
            ProcessError::AlreadyRunning => UnifiedError::InvalidState.with_context(context, location),
            ProcessError::NotRunning => UnifiedError::InvalidState.with_context(context, location),
            ProcessError::NotStopped => UnifiedError::InvalidState.with_context(context, location),
            ProcessError::NotZombie => UnifiedError::InvalidState.with_context(context, location),
            ProcessError::Unknown => UnifiedError::Unknown.with_context(context, location),
        }
    }
}

impl IntoFrameworkError for ThreadError {
    fn into_framework_error(self) -> FrameworkError {
        match self {
            ThreadError::InvalidTid => UnifiedError::InvalidArgument.into_framework_error(),
            ThreadError::ThreadNotFound => UnifiedError::NotFound.into_framework_error(),
            ThreadError::PermissionDenied => UnifiedError::PermissionDenied.into_framework_error(),
            ThreadError::ResourceUnavailable => UnifiedError::ResourceUnavailable.into_framework_error(),
            ThreadError::InvalidArguments => UnifiedError::InvalidArgument.into_framework_error(),
            ThreadError::OutOfMemory => UnifiedError::OutOfMemory.into_framework_error(),
            ThreadError::NotSupported => UnifiedError::NotSupported.into_framework_error(),
            ThreadError::ThreadExists => UnifiedError::AlreadyExists.into_framework_error(),
            ThreadError::NotChild => UnifiedError::InvalidArgument.into_framework_error(),
            ThreadError::AlreadyRunning => UnifiedError::InvalidState.into_framework_error(),
            ThreadError::NotRunning => UnifiedError::InvalidState.into_framework_error(),
            ThreadError::NotStopped => UnifiedError::InvalidState.into_framework_error(),
            ThreadError::NotZombie => UnifiedError::InvalidState.into_framework_error(),
            ThreadError::Unknown => UnifiedError::Unknown.into_framework_error(),
        }
    }
    
    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        match self {
            ThreadError::InvalidTid => UnifiedError::InvalidArgument.with_context(context, location),
            ThreadError::ThreadNotFound => UnifiedError::NotFound.with_context(context, location),
            ThreadError::PermissionDenied => UnifiedError::PermissionDenied.with_context(context, location),
            ThreadError::ResourceUnavailable => UnifiedError::ResourceUnavailable.with_context(context, location),
            ThreadError::InvalidArguments => UnifiedError::InvalidArgument.with_context(context, location),
            ThreadError::OutOfMemory => UnifiedError::OutOfMemory.with_context(context, location),
            ThreadError::NotSupported => UnifiedError::NotSupported.with_context(context, location),
            ThreadError::ThreadExists => UnifiedError::AlreadyExists.with_context(context, location),
            ThreadError::NotChild => UnifiedError::InvalidArgument.with_context(context, location),
            ThreadError::AlreadyRunning => UnifiedError::InvalidState.with_context(context, location),
            ThreadError::NotRunning => UnifiedError::InvalidState.with_context(context, location),
            ThreadError::NotStopped => UnifiedError::InvalidState.with_context(context, location),
            ThreadError::NotZombie => UnifiedError::InvalidState.with_context(context, location),
            ThreadError::Unknown => UnifiedError::Unknown.with_context(context, location),
        }
    }
}