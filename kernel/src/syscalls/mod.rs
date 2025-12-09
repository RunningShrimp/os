//! System Call Interface Module
//!
//! This module provides the system call dispatch mechanism and system call number constants.
//! It routes system calls to appropriate submodules based on their numeric ranges.
//!
//! # System Call Ranges
//!
//! - `0x1000-0x1FFF`: Process management syscalls
//! - `0x2000-0x2FFF`: File I/O syscalls
//! - `0x3000-0x3FFF`: Memory management syscalls
//! - `0x4000-0x4FFF`: Network syscalls
//! - `0x5000-0x5FFF`: Signal handling syscalls (including advanced signal features)
//! - `0x6000-0x6FFF`: Time-related syscalls
//! - `0x7000-0x7FFF`: Filesystem syscalls
//! - `0x8000-0x8FFF`: Thread management syscalls (including advanced thread features)
//! - `0xF000-0xFFFF`: Security system calls (capabilities, password/group database, user/group ID management)
//! - `0x9000-0x9FFF`: Zero-copy I/O syscalls
//! - `0xA000-0xAFFF`: epoll syscalls
//! - `0xB000-0xBFFF`: GLib compatibility syscalls
//! - `0xC000-0xCFFF`: AIO syscalls
//! - `0xD000-0xDFFF`: Message queue syscalls
//! - `0xE000-0xEFFF`: Real-time scheduling syscalls
//!
//! # Performance Optimizations
//!
//! The module implements fast paths for frequently called system calls:
//! - `getpid`: Direct return without argument conversion
//! - `read/write`: Optimized for small buffers (<=4KB) using stack allocation
//! - `close`: Optimized for common file descriptors (0-7)
//!
//! # Example
//!
//! ```
//! use kernel::syscalls;
//!
//! // Dispatch a getpid system call
//! let pid = syscalls::dispatch(syscalls::SYS_GETPID, &[]);
//!
//! // Dispatch a read system call
//! let args = [0u64, 0x1000u64, 4096u64]; // fd, buf_ptr, count
//! let result = syscalls::dispatch(syscalls::SYS_READ, &args);
//! ```

use crate::syscalls::common::{SyscallError, SyscallResult, syscall_error_to_errno};
use crate::syscalls::enhanced_error_handler::ErrorContext;
use alloc::vec::Vec;

// Import new modular architecture services
use crate::syscalls::services::{
    ServiceRegistry, SyscallDispatcher, Service, ServiceStatus,
    init_service_system, ServiceSystem,
};

// Import service implementations (commenting out non-existent modules)
// use crate::syscalls::process_service::service::ProcessService;
// use crate::syscalls::fs_service::service::FileSystemService;
use crate::syscalls::mm::service::MemoryService;
use crate::syscalls::net::service::NetworkService;

use bincode;
extern crate alloc;

// use alloc::sync::Arc;
// use crate::sync::Mutex;

use alloc::sync::Arc;
use spin::Mutex;

// Import Box from alloc crate
use alloc::boxed::Box;

// Import default service system creation function
use crate::syscalls::services::create_default_service_system;

// Global service system
static GLOBAL_SERVICE_SYSTEM: Mutex<Option<ServiceSystem>> = Mutex::new(None);

/// Initialize the new modular syscall architecture
pub fn initialize_modular_architecture() -> Result<(), SyscallError> {
    use crate::syscalls::services::ServiceMetadata;
    use crate::syscalls::services::ServiceType;
    
    crate::println!("[syscall] Initializing modular architecture");
    
    // Create service system
    let service_system = create_default_service_system();
    
    // Register process service (commented out because module doesn't exist)
    // let process_service = Box::new(ProcessService::new());
    // let process_metadata = ServiceMetadata {
    //     service_type: ServiceType::Process,
    //     priority: 50,
    //     is_syscall_service: true,
    //     tags: vec!["process".to_string(), "syscall".to_string()],
    // };
    // service_system.register_service(process_service, process_metadata)
    //     .map_err(|_| SyscallError::InitializationError)?;
    
    // Register filesystem service (commented out because module doesn't exist)
    // let fs_service = Box::new(FileSystemService::new());
    // let fs_metadata = ServiceMetadata {
    //     service_type: ServiceType::FileSystem,
    //     priority: 60,
    //     is_syscall_service: true,
    //     tags: vec!["filesystem".to_string(), "syscall".to_string()],
    // };
    // service_system.register_service(fs_service, fs_metadata)
    //     .map_err(|_| SyscallError::InitializationError)?;
    
    // Register memory service
    let mem_service = Box::new(MemoryService::new());
    let mem_metadata = ServiceMetadata {
        service_type: ServiceType::Memory,
        priority: 40,
        is_syscall_service: true,
        tags: vec!["memory".to_string(), "syscall".to_string()],
    };
    service_system.register_service(mem_service, mem_metadata)
        .map_err(|_| SyscallError::InitializationError)?;
    
    // Register network service
    let net_service = Box::new(NetworkService::new());
    let net_metadata = ServiceMetadata {
        service_type: ServiceType::Network,
        priority: 70,
        is_syscall_service: true,
        tags: vec!["network".to_string(), "syscall".to_string()],
    };
    service_system.register_service(net_service, net_metadata)
        .map_err(|_| SyscallError::InitializationError)?;
    
    // TODO: Register optimization services when fixed
    // Optimization service registration commented out until trait is fixed
    
    // Register optimization manager service (commented out - trait mismatch)
    // let manager_service = Box::new(OptimizationManagerService::new());
    // let manager_metadata = ServiceMetadata {
    //     service_type: ServiceType::Core,
    //     priority: 95,
    //     is_syscall_service: false,
    //     tags: vec!["manager".to_string(), "optimization".to_string()],
    // };
    // service_system.register_service(manager_service, manager_metadata)
    //     .map_err(|_| SyscallError::InitializationError)?;
    
    // Start all services
    service_system.start_all_services()
        .map_err(|_| SyscallError::InitializationError)?;
    
    // Store in global variable
    let mut service_system_guard = GLOBAL_SERVICE_SYSTEM.lock();
    *service_system_guard = Some(service_system);
    
    crate::println!("[syscall] Modular architecture initialized successfully");
    Ok(())
}

/// Dispatch syscall using new modular architecture
pub fn dispatch_modular(syscall_num: u32, args: &[u64]) -> SyscallResult {
    use crate::error_handling::unified::KernelError;
    
    let service_system_guard = GLOBAL_SERVICE_SYSTEM.lock();
    let service_system = match service_system_guard.as_ref() {
        Some(system) => system,
        None => return Err(SyscallError::NotSupported),
    };
    
    match service_system.handle_syscall(syscall_num, args) {
        Ok(result) => Ok(result),
        Err(error) => {
            // Convert KernelError to SyscallError
            match error {
                KernelError::SyscallNotSupported => Err(SyscallError::InvalidSyscall),
                KernelError::ServiceNotFound => Err(SyscallError::NotSupported),
                KernelError::BadAddress => Err(SyscallError::BadAddress),
                KernelError::BadFileDescriptor => Err(SyscallError::BadFileDescriptor),
                KernelError::IoError => Err(SyscallError::IoError),
                _ => Err(SyscallError::InternalError),
            }
        }
    }
}

/// Get service status and statistics (stub)
/// TODO: Implement when service modules are ready
pub fn get_service_status() -> Result<alloc::string::String, SyscallError> {
    Ok(alloc::string::String::from("Service status: pending implementation\n"))
}

/// Enhanced service initialization with configuration options (stub)
/// TODO: Implement when service modules are ready
pub fn initialize_modular_architecture_with_config(
    _enable_caching: bool,
    _max_services: Option<usize>,
) -> Result<(), SyscallError> {
    Ok(())
}

/// Get comprehensive service status and health information (stub)
/// TODO: Implement when service modules are ready
pub fn get_comprehensive_service_status() -> Result<alloc::string::String, SyscallError> {
    Ok(alloc::string::String::from("Comprehensive service status: pending implementation\n"))
}

/// Run optimization CLI tool
/// TODO: Implement when optimization modules are refactored
pub fn run_optimization_cli(args: &[String]) -> Result<alloc::string::String, SyscallError> {
    Ok("优化工具正在重构中，暂不可用\n".to_string())
}

/// Run optimization tests and benchmarks
/// TODO: Implement when optimization modules are refactored
pub fn run_optimization_tests() -> Result<alloc::string::String, SyscallError> {
    Ok("优化测试正在重构中，暂不可用\n".to_string())
}

/// Cleanup modular architecture and shutdown services (stub)
/// TODO: Implement when service modules are ready
pub fn shutdown_modular_architecture() -> Result<(), SyscallError> {
    Ok(())
}

/// Get comprehensive optimization report
/// TODO: Implement when optimization services are refactored and fixed
pub fn get_optimization_report() -> Result<alloc::string::String, SyscallError> {
    // Optimization services have been moved to tools/ pending architecture fixes
    Ok("优化报告不可用，优化服务正在重构中\n".to_string())
}

/// Reload service configuration without full restart (stub)
/// TODO: Implement when service modules are ready
pub fn reload_service_configuration() -> Result<(), SyscallError> {
    Ok(())
}

pub mod common;

// New modular architecture - core service modules
pub mod process;
pub mod fs;
pub mod mm;  // Memory management module (renamed from memory for clarity)
pub mod net;  // Network module (renamed from network for consistency)
pub mod ipc;  // Inter-process communication module
pub mod signal_simple;
pub use signal_simple as signal;
// pub mod signal_advanced;  // Temporarily disabled due to borrow checker issues

pub mod realtime_simple;
pub use realtime_simple as realtime;
pub mod sched;

// Service management system
pub mod services;
pub mod core;
pub mod optimizations;

// Legacy modules (maintained for compatibility)
pub mod file_io;
// pub mod file_io_optimized;  // TODO: Fix circular imports with net module
// pub mod process;  // Duplicate - defined earlier as modular service
// pub mod process_optimized;  // TODO: Fix imports
pub mod performance_monitor;
pub mod fast_dispatcher;
pub mod memory;  // Keep for backward compatibility
pub mod network;  // Keep for backward compatibility
pub mod time;
// pub mod posix_tests; // Temporarily disabled due to compilation errors
pub mod posix_integration_test;
pub mod thread;
pub mod zero_copy;
// pub mod zero_copy_optimized;  // TODO: Fix circular imports
pub mod epoll;
pub mod glib;
pub mod batch;
pub mod aio;
pub mod mqueue;
pub mod enhanced_error_handler;
// pub mod advanced_mmap; // Already defined in memory.rs
// pub mod advanced_signal;  // Disabled module
// pub mod realtime;  // Disabled module
// pub mod advanced_thread;  // Disabled module
pub mod cache;
// pub mod security;  // Disabled module
pub mod validation;


#[cfg(feature = "kernel_tests")]
pub mod tests;

/// Wake channel identifier for poll/epoll events
///
/// This constant is used to identify the wake channel for poll/epoll event notifications.
/// When a file descriptor becomes ready, the kernel uses this channel to wake up waiting processes.
pub const POLL_WAKE_CHAN: usize = 0x80000000;

/// System call number for `read`
///
/// Reads data from a file descriptor.
pub const SYS_READ: u32 = 0x2002;

/// System call number for `write`
///
/// Writes data to a file descriptor.
pub const SYS_WRITE: u32 = 0x2003;

/// System call number for `open`
///
/// Opens a file or creates a new file.
pub const SYS_OPEN: u32 = 0x2000;

/// System call number for `close`
///
/// Closes a file descriptor.
pub const SYS_CLOSE: u32 = 0x2001;

/// System call number for `getpid`
///
/// Returns the process ID of the calling process.
pub const SYS_GETPID: u32 = 0x1004;

/// System call number for `fork`
///
/// Creates a new process by duplicating the calling process.
pub const SYS_FORK: u32 = 0x1000;

/// System call number for `exit`
///
/// Terminates the calling process with the specified exit status.
pub const SYS_EXIT: u32 = 0x1003;

/// System call number for `kill`
///
/// Sends a signal to a process or process group.
pub const SYS_KILL: u32 = 0x5000;

/// System call number for `batch`
///
/// Executes multiple system calls in a single batch for improved performance.
pub const SYS_BATCH: u32 = 0x9000;

/// System call number for `aio_read`
///
/// Initiates an asynchronous read operation.
pub const SYS_AIO_READ: u32 = 0xC000;

/// System call number for `aio_write`
///
/// Initiates an asynchronous write operation.
pub const SYS_AIO_WRITE: u32 = 0xC001;

/// System call number for `aio_fsync`
///
/// Initiates an asynchronous file synchronization operation.
pub const SYS_AIO_FSYNC: u32 = 0xC002;

/// System call number for `aio_return`
///
/// Gets the return status of an asynchronous I/O operation.
pub const SYS_AIO_RETURN: u32 = 0xC003;

/// System call number for `aio_error`
///
/// Gets the error status of an asynchronous I/O operation.
pub const SYS_AIO_ERROR: u32 = 0xC004;

/// System call number for `aio_cancel`
///
/// Cancels an asynchronous I/O operation.
pub const SYS_AIO_CANCEL: u32 = 0xC005;

/// System call number for `lio_listio`
///
/// Initiates a list of asynchronous I/O operations.
pub const SYS_LIO_LISTIO: u32 = 0xC006;

/// Message queue system calls (0xD000-0xDFFF)
/// System call number for `mq_open`
///
/// Opens a message queue.
pub const SYS_MQ_OPEN: u32 = 0xD000;

/// System call number for `mq_close`
///
/// Closes a message queue.
pub const SYS_MQ_CLOSE: u32 = 0xD001;

/// System call number for `mq_unlink`
///
/// Removes a message queue.
pub const SYS_MQ_UNLINK: u32 = 0xD002;

/// System call number for `mq_send`
///
/// Sends a message to a message queue.
pub const SYS_MQ_SEND: u32 = 0xD003;

/// System call number for `mq_timedsend`
///
/// Sends a message to a message queue with timeout.
pub const SYS_MQ_TIMEDSEND: u32 = 0xD004;

/// System call number for `mq_receive`
///
/// Receives a message from a message queue.
pub const SYS_MQ_RECEIVE: u32 = 0xD005;

/// System call number for `mq_timedreceive`
///
/// Receives a message from a message queue with timeout.
pub const SYS_MQ_TIMEDRECEIVE: u32 = 0xD006;

/// System call number for `mq_getattr`
///
/// Gets message queue attributes.
pub const SYS_MQ_GETATTR: u32 = 0xD007;

/// System call number for `mq_setattr`
///
/// Sets message queue attributes.
pub const SYS_MQ_SETATTR: u32 = 0xD008;

/// System call number for `mq_notify`
///
/// Registers for asynchronous notification of message arrival.
pub const SYS_MQ_NOTIFY: u32 = 0xD009;

/// Advanced signal system calls (0x5000-0x5FFF)
/// System call number for `sigqueue`
///
/// Queues a signal to a process.
pub const SYS_SIGQUEUE: u32 = 0x5000;

/// System call number for `sigtimedwait`
///
/// Waits for signals with timeout.
pub const SYS_SIGTIMEDWAIT: u32 = 0x5001;

/// System call number for `sigwaitinfo`
///
/// Waits for signals.
pub const SYS_SIGWAITINFO: u32 = 0x5002;

/// System call number for `sigaltstack`
///
/// Sets alternate signal stack.
pub const SYS_SIGALTSTACK: u32 = 0x5003;

/// System call number for `pthread_sigmask`
///
/// Sets thread signal mask.
pub const SYS_PTHREAD_SIGMASK: u32 = 0x5004;

/// Real-time scheduling system calls (0xE000-0xEFFF)
/// System call number for `sched_setscheduler`
///
/// Sets scheduling policy and parameters.
pub const SYS_SCHED_SETSCHEDULER: u32 = 0xE000;

/// System call number for `sched_getscheduler`
///
/// Gets scheduling policy.
pub const SYS_SCHED_GETSCHEDULER: u32 = 0xE001;

/// System call number for `sched_setparam`
///
/// Sets scheduling parameters.
pub const SYS_SCHED_SETPARAM: u32 = 0xE002;

/// System call number for `sched_getparam`
///
/// Gets scheduling parameters.
pub const SYS_SCHED_GETPARAM: u32 = 0xE003;

/// System call number for `sched_get_priority_max`
///
/// Gets maximum priority for policy.
pub const SYS_SCHED_GET_PRIORITY_MAX: u32 = 0xE004;

/// System call number for `sched_get_priority_min`
///
/// Gets minimum priority for policy.
pub const SYS_SCHED_GET_PRIORITY_MIN: u32 = 0xE005;

/// System call number for `sched_rr_get_interval`
///
/// Gets round-robin time slice.
pub const SYS_SCHED_RR_GET_INTERVAL: u32 = 0xE006;

/// System call number for `sched_setaffinity`
///
/// Sets CPU affinity.
pub const SYS_SCHED_SETAFFINITY: u32 = 0xE007;

/// System call number for `sched_getaffinity`
///
/// Gets CPU affinity.
pub const SYS_SCHED_GETAFFINITY: u32 = 0xE008;

/// Fast-path sched_yield (O(1) 调度骨架)
pub use sched::SYS_SCHED_YIELD_FAST;
/// 用户态 enqueue hint（tid, prio, cpu_hint）
pub use sched::SYS_SCHED_ENQUEUE_HINT;

/// Advanced thread system calls (0x8000-0x8FFF)
/// System call number for `pthread_attr_setschedpolicy`
///
/// Sets thread scheduling policy attribute.
pub const SYS_PTHREAD_ATTR_SETSCHEDPOLICY: u32 = 0x8000;

/// System call number for `pthread_attr_getschedpolicy`
///
/// Gets thread scheduling policy attribute.
pub const SYS_PTHREAD_ATTR_GETSCHEDPOLICY: u32 = 0x8001;

/// System call number for `pthread_attr_setschedparam`
///
/// Sets thread scheduling parameter attribute.
pub const SYS_PTHREAD_ATTR_SETSCHEDPARAM: u32 = 0x8002;

/// System call number for `pthread_attr_getschedparam`
///
/// Gets thread scheduling parameter attribute.
pub const SYS_PTHREAD_ATTR_GETSCHEDPARAM: u32 = 0x8003;

/// System call number for `pthread_attr_setinheritsched`
///
/// Sets thread scheduling inheritance attribute.
pub const SYS_PTHREAD_ATTR_SETINHERITSCHED: u32 = 0x8004;

/// System call number for `pthread_attr_getinheritsched`
///
/// Gets thread scheduling inheritance attribute.
pub const SYS_PTHREAD_ATTR_GETINHERITSCHED: u32 = 0x8005;

/// System call number for `pthread_setschedparam`
///
/// Sets thread scheduling parameters.
pub const SYS_PTHREAD_SETSCHEDPARAM: u32 = 0x8006;

/// System call number for `pthread_getschedparam`
///
/// Gets thread scheduling parameters.
pub const SYS_PTHREAD_GETSCHEDPARAM: u32 = 0x8007;

/// System call number for `pthread_getcpuclockid`
///
/// Gets thread CPU clock ID.
pub const SYS_PTHREAD_GETCPUCLOCKID: u32 = 0x8008;

/// System call number for `pthread_barrier_init`
///
/// Initializes a barrier.
pub const SYS_PTHREAD_BARRIER_INIT: u32 = 0x8009;

/// System call number for `pthread_barrier_wait`
///
/// Waits at a barrier.
pub const SYS_PTHREAD_BARRIER_WAIT: u32 = 0x800A;

/// System call number for `pthread_barrier_destroy`
///
/// Destroys a barrier.
pub const SYS_PTHREAD_BARRIER_DESTROY: u32 = 0x800B;

/// System call number for `pthread_spin_init`
///
/// Initializes a spinlock.
pub const SYS_PTHREAD_SPIN_INIT: u32 = 0x800C;

/// System call number for `pthread_spin_lock`
///
/// Acquires a spinlock.
pub const SYS_PTHREAD_SPIN_LOCK: u32 = 0x800D;

/// System call number for `pthread_spin_unlock`
///
/// Releases a spinlock.
pub const SYS_PTHREAD_SPIN_UNLOCK: u32 = 0x800E;

/// System call number for `pthread_spin_destroy`
///
/// Destroys a spinlock.
pub const SYS_PTHREAD_SPIN_DESTROY: u32 = 0x800F;

/// Security system calls (0xF000-0xFFFF)
/// System call number for `capget`
///
/// Gets process capabilities.
pub const SYS_CAPGET: u32 = 0xF000;

/// System call number for `capset`
///
/// Sets process capabilities.
pub const SYS_CAPSET: u32 = 0xF001;

/// System call number for `getpwnam`
///
/// Gets password entry by name.
pub const SYS_GETPWNAM: u32 = 0xF002;

/// System call number for `getpwuid`
///
/// Gets password entry by UID.
pub const SYS_GETPWUID: u32 = 0xF003;

/// System call number for `getgrnam`
///
/// Gets group entry by name.
pub const SYS_GETGRNAM: u32 = 0xF004;

/// System call number for `getgrgid`
///
/// Gets group entry by GID.
pub const SYS_GETGRGID: u32 = 0xF005;

/// System call number for `setuid`
///
/// Sets user ID.
pub const SYS_SETUID: u32 = 0xF006;

/// System call number for `setgid`
///
/// Sets group ID.
pub const SYS_SETGID: u32 = 0xF007;

/// System call number for `seteuid`
///
/// Sets effective user ID.
pub const SYS_SETEUID: u32 = 0xF008;

/// System call number for `setegid`
///
/// Sets effective group ID.
pub const SYS_SETEGID: u32 = 0xF009;

/// System call number for `setreuid`
///
/// Sets real and effective user ID.
pub const SYS_SETREUID: u32 = 0xF00A;

/// System call number for `setregid`
///
/// Sets real and effective group ID.
pub const SYS_SETREGID: u32 = 0xF00B;

/// Optimized argument conversion helper
/// Converts usize slice to u64 array without unnecessary bounds checks
#[inline(always)]
fn convert_args_fast(args: &[usize]) -> ([u64; 6], usize) {
    const MAX_ARGS: usize = 6;
    let len = args.len().min(MAX_ARGS);
    let mut result = [0u64; MAX_ARGS];
    
    // Unroll loop for better performance on small argument counts
    match len {
        0 => {},
        1 => result[0] = args[0] as u64,
        2 => {
            result[0] = args[0] as u64;
            result[1] = args[1] as u64;
        },
        3 => {
            result[0] = args[0] as u64;
            result[1] = args[1] as u64;
            result[2] = args[2] as u64;
        },
        4 => {
            result[0] = args[0] as u64;
            result[1] = args[1] as u64;
            result[2] = args[2] as u64;
            result[3] = args[3] as u64;
        },
        5 => {
            result[0] = args[0] as u64;
            result[1] = args[1] as u64;
            result[2] = args[2] as u64;
            result[3] = args[3] as u64;
            result[4] = args[4] as u64;
        },
        _ => {
            result[0] = args[0] as u64;
            result[1] = args[1] as u64;
            result[2] = args[2] as u64;
            result[3] = args[3] as u64;
            result[4] = args[4] as u64;
            result[5] = args[5] as u64;
        },
    }
    
    (result, len)
}

/// Fast path for common system calls
/// These are the most frequently called syscalls and benefit from direct handling
#[inline(always)]
fn fast_path_dispatch(syscall_num: usize, args: &[usize]) -> Option<SyscallResult> {
    match syscall_num {
        // Fast path: getpid (very common, no arguments)
        val if val == SYS_GETPID as usize => {
            use crate::process::getpid;
            return Some(Ok(getpid() as u64));
        },
        // Fast path: gettid (common, no arguments)
        0x8006 => {
            use crate::process::thread::current_thread;
            return Some(Ok(current_thread().unwrap_or(0) as u64));
        },
        // Fast path: read (high frequency, optimize common case)
        val if val == SYS_READ as usize => {
            return fast_path_read(args);
        },
        // Fast path: write (high frequency, optimize common case)
        val if val == SYS_WRITE as usize => {
            return fast_path_write(args);
        },
        // Fast path: close (common, simple operation)
        val if val == SYS_CLOSE as usize => {
            return fast_path_close(args);
        },
        // Fast path: batch (performance optimization)
        val if val == SYS_BATCH as usize => {
            return fast_path_batch(args);
        },
        _ => {},
    }
    None
}

/// Fast path for read system call
/// Optimized for small reads (<4KB) from cached file descriptors
/// 
/// Performance optimizations:
/// - Stack-allocated buffer (no heap allocation)
/// - Minimal lock holding time (only for FD lookup)
/// - Early validation to avoid unnecessary work
/// - Target latency: <300ns for small reads
#[inline(always)]
fn fast_path_read(args: &[usize]) -> Option<SyscallResult> {
    // Quick validation: must have 3 arguments
    if args.len() < 3 {
        return None;
    }
    
    let fd = args[0] as i32;
    let buf_ptr = args[1] as usize;
    let count = args[2] as usize;
    
    // Fast path conditions:
    // 1. Valid file descriptor (0-7 for cached FDs)
    // 2. Valid buffer pointer
    // 3. Small read size (<=4KB for stack buffer)
    // 4. Count > 0
    if fd < 0 || fd >= 8 || buf_ptr == 0 || count == 0 || count > 4096 {
        return None; // Fall back to normal path
    }
    
    // Get current process (quick check, no lock needed)
    let pid = match crate::process::myproc() {
        Some(p) => p,
        None => return None,
    };
    
    // Minimize lock holding time: only lock to get file index and pagetable
    let (file_idx, pagetable) = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return None,
        };
        
        // Get file index (cached for first 8 FDs, O(1) lookup)
        let file_idx = match proc.ofile[fd as usize] {
            Some(idx) => idx,
            None => return None, // Invalid FD, fall back to normal path
        };
        
        let pagetable = proc.pagetable;
        (file_idx, pagetable)
    }; // Lock released here
    
    if pagetable.is_null() {
        return None;
    }
    
    // Use stack-allocated buffer for small reads (no heap allocation)
    // This avoids allocator overhead and reduces latency
    let mut kernel_buf = [0u8; 4096];
    let read_buf = &mut kernel_buf[..count];
    
    // Read from file (optimized path, no lock held)
    let bytes_read = crate::fs::file::file_read(file_idx, read_buf);
    
    if bytes_read < 0 {
        return Some(Err(crate::syscalls::common::SyscallError::IoError));
    }
    
    let bytes_read = bytes_read as usize;
    
    // Copy data to user space (no lock held)
    if bytes_read > 0 {
        unsafe {
            match crate::mm::vm::copyout(pagetable, buf_ptr, read_buf.as_ptr(), bytes_read) {
                Ok(_) => {},
                Err(_) => return Some(Err(crate::syscalls::common::SyscallError::BadAddress)),
            }
        }
    }
    
    Some(Ok(bytes_read as u64))
}

/// Fast path for write system call
/// Optimized for small writes (<4KB) to cached file descriptors
/// 
/// Performance optimizations:
/// - Stack-allocated buffer (no heap allocation)
/// - Minimal lock holding time (only for FD lookup)
/// - Early validation to avoid unnecessary work
/// - Target latency: <400ns for small writes
#[inline(always)]
fn fast_path_write(args: &[usize]) -> Option<SyscallResult> {
    // Quick validation: must have 3 arguments
    if args.len() < 3 {
        return None;
    }
    
    let fd = args[0] as i32;
    let buf_ptr = args[1] as usize;
    let count = args[2] as usize;
    
    // Fast path conditions:
    // 1. Valid file descriptor (0-7 for cached FDs)
    // 2. Valid buffer pointer
    // 3. Small write size (<=4KB for stack buffer)
    // 4. Count > 0
    if fd < 0 || fd >= 8 || buf_ptr == 0 || count == 0 || count > 4096 {
        return None; // Fall back to normal path
    }
    
    // Get current process (quick check, no lock needed)
    let pid = match crate::process::myproc() {
        Some(p) => p,
        None => return None,
    };
    
    // Minimize lock holding time: only lock to get file index and pagetable
    let (file_idx, pagetable) = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return None,
        };
        
        // Get file index (cached for first 8 FDs, O(1) lookup)
        let file_idx = match proc.ofile[fd as usize] {
            Some(idx) => idx,
            None => return None, // Invalid FD, fall back to normal path
        };
        
        let pagetable = proc.pagetable;
        (file_idx, pagetable)
    }; // Lock released here
    
    if pagetable.is_null() {
        return None;
    }
    
    // Use stack-allocated buffer for small writes (no heap allocation)
    // This avoids allocator overhead and reduces latency
    let mut kernel_buf = [0u8; 4096];
    let write_buf = &mut kernel_buf[..count];
    
    // Copy data from user space (no lock held)
    unsafe {
        match crate::mm::vm::copyin(pagetable, write_buf.as_mut_ptr(), buf_ptr, count) {
            Ok(_) => {},
            Err(_) => return Some(Err(crate::syscalls::common::SyscallError::BadAddress)),
        }
    }
    
    // Write to file (optimized path, no lock held)
    let bytes_written = crate::fs::file::file_write(file_idx, write_buf);
    
    if bytes_written < 0 {
        return Some(Err(crate::syscalls::common::SyscallError::IoError));
    }
    
    Some(Ok(bytes_written as u64))
}

/// Fast path for close system call
/// Optimized for simple close operation
#[inline(always)]
fn fast_path_close(args: &[usize]) -> Option<SyscallResult> {
    // Quick validation: must have 1 argument
    if args.len() < 1 {
        return None;
    }
    
    let fd = args[0] as i32;
    
    // Fast path conditions:
    // 1. Valid file descriptor (0-7 for cached FDs)
    if fd < 0 || fd >= 8 {
        return None; // Fall back to normal path
    }
    
    // Get current process (quick check)
    let pid = match crate::process::myproc() {
        Some(p) => p,
        None => return None,
    };
    
    // Try to get file descriptor quickly
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = match proc_table.find(pid) {
        Some(p) => p,
        None => {
            drop(proc_table);
            return None;
        }
    };
    
    // Get file index
    let file_idx = match proc.ofile[fd as usize] {
        Some(idx) => idx,
        None => {
            drop(proc_table);
            return Some(Err(crate::syscalls::common::SyscallError::BadFileDescriptor));
        }
    };
    
    // Clear process file descriptor
    proc.ofile[fd as usize] = None;
    drop(proc_table);
    
    // Close file in global file table
    crate::fs::file::file_close(file_idx);
    
    Some(Ok(0))
}

/// Main system call dispatch function
/// 
/// This function routes system calls to appropriate handlers based on their number.
/// It supports both legacy and fast architectures.
/// 
/// # Arguments
/// 
/// * `syscall_num` - System call number
/// * `args` - System call arguments
/// 
/// # Returns
/// 
/// * `isize` - Return value (negative for errors)
pub fn dispatch(syscall_num: usize, args: &[usize]) -> isize {
    // 初始化快速系统调用架构
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    static mut FAST_ARCHITECTURE_ENABLED: bool = false;

    INIT_ONCE.call_once(|| {
        #[cfg(feature = "fast_syscall")]
        unsafe {
            fast_dispatcher::initialize_fast_syscall_architecture();
            performance_optimized::initialize_global_performance_optimizer();
            FAST_ARCHITECTURE_ENABLED = true;
            crate::println!("[syscall] Fast architecture enabled");
        }
        #[cfg(feature = "sched_opt")]
        unsafe {
            scheduler_optimized::initialize_global_optimized_scheduler(4);
        }
        #[cfg(feature = "zero_copy")]
        unsafe {
            zero_copy_optimized::initialize_global_zero_copy_manager();
        }
    });

    // 转换参数为u64
    let (args_u64, args_len) = convert_args_fast(args);
    
    // 使用性能优化分发器
    let start_ns = crate::time::get_time_ns();
    let result = unsafe {
        if FAST_ARCHITECTURE_ENABLED {
            // 首先尝试性能优化分发器
            match performance_optimized::dispatch_with_optimization(syscall_num as u32, &args_u64[..args_len]) {
                Ok(result) => Ok(result),
                Err(_) => {
                    // 如果性能优化分发器失败，回退到快速分发器
                    fast_dispatcher::fast_dispatch(syscall_num as u32, &args_u64[..args_len])
                }
            }
        } else {
            // 回退到原始实现
            dispatch_legacy(syscall_num as u32, &args_u64[..args_len])
        }
    };
    let end_ns = crate::time::get_time_ns();
    if end_ns >= start_ns {
        let dur_us = ((end_ns - start_ns) / 1000) as u64;
        crate::syscalls::performance_monitor::record_syscall_performance(dur_us);
    }
    
    match result {
        Ok(value) => value as isize,
        Err(error) => -(syscall_error_to_errno(error) as isize),
    }
}

/// 遗留系统调用分发函数
/// 
/// 当快速架构不可用时，使用此函数作为回退
fn dispatch_legacy(syscall_num: u32, args: &[u64]) -> SyscallResult {
    // 使用原始的分发逻辑
    match syscall_num {
        // Process management syscalls (0x1000-0x1FFF)
        n if (n & 0xF000) == 0x1000 && n <= 0x1FFF => {
            // Use original implementation for process syscalls
            process::dispatch(syscall_num, args)
        },

        // File I/O syscalls (0x2000-0x2FFF)
        n if (n & 0xF000) == 0x2000 && n <= 0x2FFF => {
            // Use original implementation for file I/O syscalls
            file_io::dispatch(syscall_num, args)
        },

        // Memory management syscalls (0x3000-0x3FFF)
        n if (n & 0xF000) == 0x3000 && n <= 0x3FFF => {
            // Use original implementation for memory syscalls
            mm::handlers::dispatch_syscall(syscall_num, args)
        },

        // Network syscalls (0x4000-0x4FFF)
        n if (n & 0xF000) == 0x4000 && n <= 0x4FFF => {
            network::dispatch(syscall_num, args)
        },

        // Signal handling syscalls (0x5000-0x5FFF)
        n if (n & 0xF000) == 0x5000 && n <= 0x5FFF => {
            // Use original implementation for signal syscalls
            signal_simple::dispatch(syscall_num, args)
        },
        
        // Time-related syscalls (0x6000-0x6FFF)
        n if (n & 0xF000) == 0x6000 && n <= 0x6FFF => {
            time::dispatch(syscall_num, args)
        },
        
        // Filesystem syscalls (0x7000-0x7FFF)
        n if (n & 0xF000) == 0x7000 && n <= 0x7FFF => {
            fs::dispatch(syscall_num, args)
        },
        
        // Thread management syscalls (0x8000-0x8FFF)
        n if (n & 0xF000) == 0x8000 && n <= 0x8FFF => {
            thread::dispatch(syscall_num, args)
        },
        
        // Zero-copy I/O syscalls (0x9000-0x9FFF)
        n if (n & 0xF000) == 0x9000 && n <= 0x9FFF => {
            zero_copy::dispatch(syscall_num, args)
        },
        
        // AIO syscalls (0xC000-0xCFFF)
        n if (n & 0xF000) == 0xC000 && n <= 0xCFFF => {
            aio::dispatch(syscall_num, args)
        },
        
        // epoll syscalls (0xA000-0xAFFF)
        n if (n & 0xF000) == 0xA000 && n <= 0xAFFF => {
            epoll::dispatch(syscall_num, args)
        },
        
        // GLib compatibility syscalls (0xB000-0xBFFF)
        n if (n & 0xF000) == 0xB000 && n <= 0xBFFF => {
            glib::dispatch(syscall_num, args)
        },
        
        // Message queue syscalls (0xD000-0xDFFF)
        n if (n & 0xF000) == 0xD000 && n <= 0xDFFF => {
            mqueue::dispatch(syscall_num, args)
        },
        
        // Real-time scheduling syscalls (0xE000-0xEFFF)
        n if (n & 0xF000) == 0xE000 && n <= 0xEFFF => {
            sched::dispatch(syscall_num, args)
        },
        
        // Invalid syscall number
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Dispatch Linux system call by translating it to NOS syscall
/// This function handles Linux x86_64 system call numbers (0-360+)
#[inline]
fn dispatch_linux_syscall(syscall_num: usize, args: &[usize]) -> isize {
    use crate::compat::{TargetPlatform};
    use crate::compat::syscall_translator::{SyscallTranslator, ForeignSyscall, TranslationFlags};
    use crate::compat::CompatibilityError;
    
    // Get or create syscall translator (lazy initialization)
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    static TRANSLATOR: crate::sync::Mutex<Option<SyscallTranslator>> = crate::sync::Mutex::new(None);
    
    // Initialize translator on first use
    INIT_ONCE.call_once(|| {
        match SyscallTranslator::new() {
            Ok(t) => {
                *TRANSLATOR.lock() = Some(t);
            }
            Err(_) => {
                crate::println!("[syscall] Failed to initialize syscall translator");
            }
        }
    });
    
    let translator_guard = TRANSLATOR.lock();
    let translator = match translator_guard.as_ref() {
        Some(t) => t,
        None => {
            // Translator initialization failed
            use crate::reliability::errno::ENOSYS;
            return -(ENOSYS as isize);
        }
    };
    
    // Create foreign syscall representation
    let mut syscall_args = [0usize; 6];
    for (i, &arg) in args.iter().take(6).enumerate() {
        syscall_args[i] = arg;
    }
    
    let foreign_syscall = ForeignSyscall {
        platform: TargetPlatform::Linux,
        number: syscall_num as u32,
        args: syscall_args,
        name: None,
        flags: TranslationFlags {
            hot_path: syscall_num < 100, // First 100 syscalls are hot path
            batchable: false,
            pure: false,
            special: false,
        },
    };
    
    // Translate and execute (translator is already locked)
    match translator.translate_syscall(foreign_syscall) {
        Ok(result) => {
            result.return_value
        }
        Err(e) => {
            // Map compatibility errors to errno
            use crate::reliability::errno::*;
            let errno = match e {
                CompatibilityError::UnsupportedApi => ENOSYS,
                CompatibilityError::UnsupportedArchitecture => ENOSYS,
                CompatibilityError::InvalidArguments => EINVAL,
                CompatibilityError::SyscallTranslationFailed => ENOSYS,
                CompatibilityError::InvalidBinaryFormat => EINVAL,
                CompatibilityError::MemoryError => ENOMEM,
                CompatibilityError::CompilationError => ENOSYS,
                CompatibilityError::SecurityViolation => EPERM,
                CompatibilityError::NotFound => ENOENT,
                CompatibilityError::PermissionDenied => EPERM,
                CompatibilityError::IoError => EIO,
            };
            -(errno as isize)
        }
    }
}

/// Fast path for batch system call
/// Optimized for executing multiple system calls in a single operation
///
/// Performance optimizations:
/// - Reduced context switching overhead
/// - Batch validation and execution
/// - Optimized error handling for batch operations
/// - Target latency: <50ns per syscall in batch (vs ~200ns individually)
#[inline(always)]
fn fast_path_batch(args: &[usize]) -> Option<SyscallResult> {
    use crate::syscalls::batch::{BatchProcessor, BatchRequest, BatchConfig};
    
    // Quick validation: must have at least 1 argument (batch request pointer)
    if args.len() < 1 {
        return None;
    }
    
    let batch_req_ptr = args[0] as usize;
    
    // Validate pointer
    if batch_req_ptr == 0 {
        return Some(Err(crate::syscalls::common::SyscallError::BadAddress));
    }
    
    // Get current process
    let pid = match crate::process::myproc() {
        Some(p) => p,
        None => return None,
    };
    
    // Get process pagetable
    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return None,
        };
        proc.pagetable
    };
    
    if pagetable.is_null() {
        return None;
    }
    
    // Get batch processor (lazy initialization)
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    static BATCH_PROCESSOR: crate::sync::Mutex<Option<BatchProcessor>> = crate::sync::Mutex::new(None);
    
    INIT_ONCE.call_once(|| {
        let config = BatchConfig {
            max_batch_size: 32,
            enable_auto_batching: false,
            enable_atomic_batches: true,
            enable_stats: true,
            default_timeout_ms: 1000,
        };
        
        let processor = BatchProcessor::new(config);
        *BATCH_PROCESSOR.lock() = Some(processor);
    });
    
    let processor_guard = BATCH_PROCESSOR.lock();
    let processor = match processor_guard.as_ref() {
        Some(p) => p,
        None => {
            return Some(Err(crate::syscalls::common::SyscallError::IoError));
        }
    };
    
    // Read batch request from user space
    let mut batch_req_data = [0u8; core::mem::size_of::<BatchRequest>()];
    unsafe {
        match crate::mm::vm::copyin(pagetable, batch_req_data.as_mut_ptr(), batch_req_ptr, batch_req_data.len()) {
            Ok(_) => {},
            Err(_) => return Some(Err(crate::syscalls::common::SyscallError::BadAddress)),
        }
    }
    
    // For now, return empty batch response (simplified implementation)
    // TODO: Implement proper batch processing without serde
    let response_data = Vec::new();
    
    // If we have a second argument, it's the response buffer pointer
    if args.len() >= 2 {
        let resp_ptr = args[1] as usize;
        let resp_max_len = args[2] as usize;
        
        // Copy response data back to user space, truncating if necessary
        let copy_len = response_data.len().min(resp_max_len);
        
        unsafe {
            match crate::mm::vm::copyout(pagetable, resp_ptr, response_data.as_ptr(), copy_len) {
                Ok(_) => {},
                Err(_) => return Some(Err(crate::syscalls::common::SyscallError::BadAddress)),
            }
        }
    }
    
    Some(Ok(0)) // TODO: Implement proper batch processing
}
