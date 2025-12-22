//! Process management types

use crate::core::types::{Pid, Size};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use alloc::string::String;


/// Process creation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessFlags {
    /// No special flags
    None = 0x0,
    /// Fork to create new process
    Fork = 0x1,
    /// Create new thread
    Thread = 0x2,
    /// Share virtual memory
    VmShare = 0x4,
    /// Share file descriptors
    FdShare = 0x8,
    /// Share filesystem
    FsShare = 0x10,
    /// Share signal handlers
    SignalShare = 0x20,
    /// Share system V IPC
    IpcShare = 0x40,
    /// Clear signals
    ClearSignals = 0x80,
    /// Set child to leader
    ChildLeader = 0x100,
    /// Set parent to death signal
    ParentDeathSignal = 0x200,
    /// Set tracing
    Trace = 0x400,
    /// Set untraced
    Untrace = 0x800,
    /// Set stop on start
    StopOnStart = 0x1000,
    /// Set stop on exit
    StopOnExit = 0x2000,
}

/// Thread creation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadFlags {
    /// No special flags
    None = 0x0,
    /// Create detached thread
    Detached = 0x1,
    /// Create joinable thread
    Joinable = 0x2,
    /// Create real-time thread
    RealTime = 0x4,
    /// Create thread with specific stack size
    CustomStack = 0x8,
    /// Create thread with specific priority
    CustomPriority = 0x10,
    /// Create thread with specific affinity
    CustomAffinity = 0x20,
    /// Create thread with specific scheduling policy
    CustomScheduling = 0x40,
}

/// Process exit codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Successful exit
    Success = 0,
    /// General error
    Error = 1,
    /// Usage error
    Usage = 2,
    /// Permission error
    Permission = 3,
    /// Not found error
    NotFound = 4,
    /// System error
    System = 5,
    /// I/O error
    Io = 6,
    /// Protocol error
    Protocol = 7,
    /// Software error
    Software = 8,
    /// Signal exit
    Signal = 9,
    /// Terminated
    Terminated = 10,
    /// Killed
    Killed = 11,
    /// Aborted
    Aborted = 12,
    /// Bus error
    Bus = 13,
    /// Floating point error
    FloatingPoint = 14,
    /// Segmentation fault
    SegmentationFault = 15,
    /// Pipe error
    Pipe = 16,
    /// Alarm
    Alarm = 17,
    /// User defined
    User = 18,
    /// Child exit
    Child = 19,
    /// Continued
    Continued = 20,
    /// Stopped
    Stopped = 21,
    /// Stopped (signal)
    StoppedSignal = 22,
    /// Stopped (tty input)
    StoppedTtyInput = 23,
    /// Stopped (tty output)
    StoppedTtyOutput = 24,
}

/// Signal information
#[derive(Debug, Clone)]
pub struct SignalInfo {
    /// Signal number
    pub signal: i32,
    /// Signal code
    pub code: i32,
    /// Signal value
    pub value: i32,
    /// Sending process ID
    pub pid: Pid,
    /// Sending user ID
    pub uid: u32,
    /// Signal address
    pub address: usize,
    /// Signal band
    pub band: u32,
    /// Signal status
    pub status: i32,
    /// Signal timer ID
    pub timer_id: i32,
    /// Signal overrun count
    pub overrun_count: i32,
    /// Signal queue ID
    pub queue_id: i32,
    /// Signal value pointer
    pub value_ptr: *mut i32,
}

/// Process context
#[derive(Debug, Clone)]
pub struct ProcessContext {
    /// Instruction pointer
    pub instruction_pointer: usize,
    /// Stack pointer
    pub stack_pointer: usize,
    /// Frame pointer
    pub frame_pointer: usize,
    /// General purpose registers
    pub general_registers: [usize; 16],
    /// Floating point registers
    pub floating_point_registers: [u64; 16],
    /// Control registers
    pub control_registers: [usize; 8],
    /// Status flags
    pub status_flags: u32,
    /// FPU status word
    pub fpu_status_word: u32,
    /// FPU control word
    pub fpu_control_word: u32,
    /// MXCSR register
    pub mxcsr: u32,
    /// FS segment register
    pub fs: u16,
    /// GS segment register
    pub gs: u16,
    /// Kernel stack pointer
    pub kernel_stack_pointer: usize,
    /// User stack pointer
    pub user_stack_pointer: usize,
    /// Signal mask
    pub signal_mask: u64,
}

/// Thread context
#[derive(Debug, Clone)]
pub struct ThreadContext {
    /// Thread ID
    pub tid: Pid,
    /// Process ID
    pub pid: Pid,
    /// Instruction pointer
    pub instruction_pointer: usize,
    /// Stack pointer
    pub stack_pointer: usize,
    /// Frame pointer
    pub frame_pointer: usize,
    /// General purpose registers
    pub general_registers: [usize; 16],
    /// Floating point registers
    pub floating_point_registers: [u64; 16],
    /// Control registers
    pub control_registers: [usize; 8],
    /// Status flags
    pub status_flags: u32,
    /// FPU status word
    pub fpu_status_word: u32,
    /// FPU control word
    pub fpu_control_word: u32,
    /// MXCSR register
    pub mxcsr: u32,
    /// FS segment register
    pub fs: u16,
    /// GS segment register
    pub gs: u16,
    /// Thread local storage base
    pub tls_base: usize,
    /// Thread local storage limit
    pub tls_limit: Size,
    /// Signal mask
    pub signal_mask: u64,
    /// Thread-specific data
    pub thread_specific_data: usize,
}

/// Process attributes (alloc version)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ProcessAttributes {
    /// Process name
    pub name: String,
    /// Process arguments
    pub arguments: Vec<String>,
    /// Environment variables
    pub environment: Vec<String>,
    /// Working directory
    pub working_directory: String,
    /// Root directory
    pub root_directory: String,
    /// Standard input
    pub stdin: i32,
    /// Standard output
    pub stdout: i32,
    /// Standard error
    pub stderr: i32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Supplementary group IDs
    pub supplementary_gids: Vec<u32>,
    /// Process priority
    pub priority: u32,
    /// Scheduling policy
    pub scheduling_policy: u32,
    /// CPU affinity
    pub cpu_affinity: u32,
    /// Memory limit
    pub memory_limit: Size,
    /// Stack size
    pub stack_size: Size,
    /// Nice value
    pub nice_value: i32,
    /// OOM adjustment
    pub oom_adjustment: i32,
}

/// Process attributes (no-alloc version)
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ProcessAttributes {
    /// Process name
    pub name: &'static str,
    /// Process arguments (static slice)
    pub arguments: &'static [&'static str],
    /// Environment variables (static slice)
    pub environment: &'static [&'static str],
    /// Working directory
    pub working_directory: &'static str,
    /// Root directory
    pub root_directory: &'static str,
    /// Standard input
    pub stdin: i32,
    /// Standard output
    pub stdout: i32,
    /// Standard error
    pub stderr: i32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Supplementary group IDs (static slice)
    pub supplementary_gids: &'static [u32],
    /// Process priority
    pub priority: u32,
    /// Scheduling policy
    pub scheduling_policy: u32,
    /// CPU affinity
    pub cpu_affinity: u32,
    /// Memory limit
    pub memory_limit: Size,
    /// Stack size
    pub stack_size: Size,
    /// Nice value
    pub nice_value: i32,
    /// OOM adjustment
    pub oom_adjustment: i32,
}

/// Thread attributes (alloc version)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ThreadAttributes {
    /// Thread name
    pub name: String,
    /// Thread priority
    pub priority: u32,
    /// Scheduling policy
    pub scheduling_policy: u32,
    /// CPU affinity
    pub cpu_affinity: u32,
    /// Stack size
    pub stack_size: Size,
    /// Stack address
    pub stack_address: usize,
    /// Guard size
    pub guard_size: Size,
    /// Detached state
    pub detached: bool,
    /// Joinable state
    pub joinable: bool,
    /// Real-time priority
    pub real_time_priority: u32,
    /// Thread-specific data
    pub thread_specific_data: usize,
}

/// Thread attributes (no-alloc version)
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ThreadAttributes {
    /// Thread name
    pub name: &'static str,
    /// Thread priority
    pub priority: u32,
    /// Scheduling policy
    pub scheduling_policy: u32,
    /// CPU affinity
    pub cpu_affinity: u32,
    /// Stack size
    pub stack_size: Size,
    /// Stack address
    pub stack_address: usize,
    /// Guard size
    pub guard_size: Size,
    /// Detached state
    pub detached: bool,
    /// Joinable state
    pub joinable: bool,
    /// Real-time priority
    pub real_time_priority: u32,
    /// Thread-specific data
    pub thread_specific_data: usize,
}

/// Process statistics (alloc version)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ProcessStats {
    /// Process ID
    pub pid: Pid,
    /// Parent process ID
    pub ppid: Pid,
    /// Process name
    pub name: String,
    /// Process state
    pub state: String,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Process priority
    pub priority: u32,
    /// Nice value
    pub nice_value: i32,
    /// Virtual memory size
    pub virtual_size: Size,
    /// Resident set size
    pub resident_size: Size,
    /// Shared memory size
    pub shared_size: Size,
    /// Text segment size
    pub text_size: Size,
    /// Data segment size
    pub data_size: Size,
    /// Stack segment size
    pub stack_size: Size,
    /// Start time
    pub start_time: u64,
    /// User time
    pub user_time: u64,
    /// System time
    pub system_time: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage percentage
    pub memory_percent: f32,
    /// Number of threads
    pub thread_count: usize,
    /// Number of page faults
    pub page_faults: u64,
    /// Number of major page faults
    pub major_page_faults: u64,
    /// Number of minor page faults
    pub minor_page_faults: u64,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of voluntary context switches
    pub voluntary_context_switches: u64,
    /// Number of involuntary context switches
    pub involuntary_context_switches: u64,
    /// Number of bytes read
    pub bytes_read: u64,
    /// Number of bytes written
    pub bytes_written: u64,
    /// Number of read operations
    pub read_operations: u64,
    /// Number of write operations
    pub write_operations: u64,
}

/// Process statistics (no-alloc version)
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ProcessStats {
    /// Process ID
    pub pid: Pid,
    /// Parent process ID
    pub ppid: Pid,
    /// Process name
    pub name: &'static str,
    /// Process state
    pub state: &'static str,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Process priority
    pub priority: u32,
    /// Nice value
    pub nice_value: i32,
    /// Virtual memory size
    pub virtual_size: Size,
    /// Resident set size
    pub resident_size: Size,
    /// Shared memory size
    pub shared_size: Size,
    /// Text segment size
    pub text_size: Size,
    /// Data segment size
    pub data_size: Size,
    /// Stack segment size
    pub stack_size: Size,
    /// Start time
    pub start_time: u64,
    /// User time
    pub user_time: u64,
    /// System time
    pub system_time: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage percentage
    pub memory_percent: f32,
    /// Number of threads
    pub thread_count: usize,
    /// Number of page faults
    pub page_faults: u64,
    /// Number of major page faults
    pub major_page_faults: u64,
    /// Number of minor page faults
    pub minor_page_faults: u64,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of voluntary context switches
    pub voluntary_context_switches: u64,
    /// Number of involuntary context switches
    pub involuntary_context_switches: u64,
    /// Number of bytes read
    pub bytes_read: u64,
    /// Number of bytes written
    pub bytes_written: u64,
    /// Number of read operations
    pub read_operations: u64,
    /// Number of write operations
    pub write_operations: u64,
}

/// Thread statistics (alloc version)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ThreadStats {
    /// Thread ID
    pub tid: Pid,
    /// Process ID
    pub pid: Pid,
    /// Thread name
    pub name: String,
    /// Thread state
    pub state: String,
    /// Thread priority
    pub priority: u32,
    /// Nice value
    pub nice_value: i32,
    /// CPU affinity
    pub cpu_affinity: u32,
    /// Stack size
    pub stack_size: Size,
    /// Stack usage
    pub stack_usage: Size,
    /// Start time
    pub start_time: u64,
    /// User time
    pub user_time: u64,
    /// System time
    pub system_time: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of voluntary context switches
    pub voluntary_context_switches: u64,
    /// Number of involuntary context switches
    pub involuntary_context_switches: u64,
}

/// Thread statistics (no-alloc version)
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone)]
pub struct ThreadStats {
    /// Thread ID
    pub tid: Pid,
    /// Process ID
    pub pid: Pid,
    /// Thread name
    pub name: &'static str,
    /// Thread state
    pub state: &'static str,
    /// Thread priority
    pub priority: u32,
    /// Nice value
    pub nice_value: i32,
    /// CPU affinity
    pub cpu_affinity: u32,
    /// Stack size
    pub stack_size: Size,
    /// Stack usage
    pub stack_usage: Size,
    /// Start time
    pub start_time: u64,
    /// User time
    pub user_time: u64,
    /// System time
    pub system_time: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of voluntary context switches
    pub voluntary_context_switches: u64,
    /// Number of involuntary context switches
    pub involuntary_context_switches: u64,
}