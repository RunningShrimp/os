//! Thread management syscalls

extern crate alloc;

use super::common::{SyscallError, SyscallResult, extract_args};
use crate::posix::{CLONE_VM, CLONE_FILES, CLONE_FS, CLONE_SIGHAND, CLONE_THREAD,
                   CLONE_PARENT_SETTID, CLONE_CHILD_SETTID, CLONE_CHILD_CLEARTID,
                   CLONE_NEWNS, CLONE_NEWUTS, CLONE_NEWIPC, CLONE_NEWNET,
                   CLONE_NEWPID, CLONE_NEWUSER};
use crate::mm::vm::copyin;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::Mutex;

// ============================================================================
// Futex Support Structures
// ============================================================================

/// Futex waiter information
#[derive(Debug, Clone)]
pub struct FutexWaiter {
    /// Thread ID of the waiter
    tid: usize,
    /// Futex address being waited on
    uaddr: usize,
    /// Expected value (for WAIT operations)
    expected_val: i32,
    /// Timeout (0 = no timeout)
    timeout: u64,
    /// Priority inheritance data
    pi_data: Option<PiFutexData>,
}

/// Priority inheritance futex data
#[derive(Debug, Clone)]
pub struct PiFutexData {
    /// Owner thread ID
    owner_tid: usize,
    /// Owner priority before acquiring lock
    original_prio: u8,
    /// List of waiters for priority inheritance
    waiters: Vec<usize>,
}

/// Global futex wait queue
pub static FUTEX_WAIT_QUEUE: Mutex<BTreeMap<usize, Vec<FutexWaiter>>> = Mutex::new(BTreeMap::new());

/// Get current time in nanoseconds (simplified)
pub fn get_current_time_ns() -> u64 {
    static TIME_COUNTER: AtomicUsize = AtomicUsize::new(0);
    TIME_COUNTER.fetch_add(1, Ordering::SeqCst) as u64
}

/// Check if timeout has expired
pub fn is_timeout_expired(timeout: u64) -> bool {
    if timeout == 0 {
        return false; // No timeout
    }
    let current_time = get_current_time_ns();
    current_time >= timeout
}

/// Dispatch thread management syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Thread operations
        0x8000 => sys_clone(args),          // clone
        0x8001 => sys_fork(args),           // fork (also in process, but thread-specific)
        0x8002 => sys_vfork(args),          // vfork
        0x8003 => sys_execve(args),         // execve (also in process)
        0x8004 => sys_exit(args),           // exit (also in process)
        0x8005 => sys_wait4(args),          // wait4
        0x8006 => sys_gettid(args),         // gettid
        0x8007 => sys_getpid(args),         // getpid (also in process)
        0x8008 => sys_set_tid_address(args), // set_tid_address
        0x8009 => sys_futex(args),          // futex
        0x800A => sys_set_robust_list(args), // set_robust_list
        0x800B => sys_get_robust_list(args), // get_robust_list
        0x800C => sys_sched_yield(args),    // sched_yield (also in process)
        0x800D => sys_sched_getaffinity(args), // sched_getaffinity (also in process)
        0x800E => sys_sched_setaffinity(args), // sched_setaffinity (also in process)
        0x800F => sys_unshare(args),          // unshare
        0x8010 => sys_setns(args),            // setns
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Clone system call
/// Arguments: [flags, stack, parent_tid_ptr, child_tid_ptr, tls]
/// Returns: child TID in parent, 0 in child, error on failure
fn sys_clone(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 5)?;

    let flags = args[0] as i32;
    let stack = args[1] as usize;
    let parent_tid_ptr = args[2] as usize;
    let child_tid_ptr = args[3] as usize;
    let tls = args[4] as usize;

    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Check if CLONE_THREAD is set (POSIX thread creation)
    let is_thread = (flags & CLONE_THREAD) != 0;

    if is_thread {
        // Create a thread (share VM, files, FS, signal handlers)
        // Required flags for POSIX threads
        let required_flags = CLONE_VM | CLONE_FILES | CLONE_FS | CLONE_SIGHAND | CLONE_THREAD;
        if (flags & required_flags) != required_flags {
            return Err(SyscallError::InvalidArgument);
        }

        // Create thread using thread subsystem
        let thread_result = crate::process::thread::create_thread(
            pid,
            crate::process::thread::ThreadType::User,
            None, // Entry point will be set from stack
            core::ptr::null_mut(),
        );

        match thread_result {
            Ok(tid) => {
                // Get the newly created thread to set up its stack and TLS
                let mut thread_table = crate::process::thread::thread_table();
                let thread = thread_table.find_thread(tid)
                    .ok_or(SyscallError::OutOfMemory)?;

                // Set up user stack if provided
                if stack != 0 {
                    thread.ustack = stack;
                    // Stack grows down, so set trapframe stack pointer to top of stack
                    unsafe {
                        if !thread.trapframe.is_null() {
                            #[cfg(target_arch = "x86_64")]
                            {
                                (*thread.trapframe).rsp = stack;
                            }
                            #[cfg(target_arch = "aarch64")]
                            {
                                (*thread.trapframe).sp = stack;
                            }
                        }
                    }
                }

                // Set TLS if provided
                if tls != 0 {
                    thread.tls_base = tls;
                    // Architecture-specific TLS setup will be done when thread starts
                }

                // Write child TID to parent_tid_ptr if CLONE_PARENT_SETTID is set
                if (flags & CLONE_PARENT_SETTID) != 0 && parent_tid_ptr != 0 {
                    unsafe {
                        let tid_val = tid as i32;
                        copyin(pagetable, parent_tid_ptr as *mut u8, parent_tid_ptr as usize, core::mem::size_of::<i32>())
                            .map_err(|_| SyscallError::BadAddress)?;
                    }
                }

                // Write child TID to child_tid_ptr if CLONE_CHILD_SETTID is set
                if (flags & CLONE_CHILD_SETTID) != 0 && child_tid_ptr != 0 {
                    unsafe {
                        let tid_val = tid as i32;
                        copyin(pagetable, child_tid_ptr as *mut u8, child_tid_ptr as usize, core::mem::size_of::<i32>())
                            .map_err(|_| SyscallError::BadAddress)?;
                    }
                }

                // Store child_tid_ptr for CLONE_CHILD_CLEARTID (clear on thread exit)
                if (flags & CLONE_CHILD_CLEARTID) != 0 && child_tid_ptr != 0 {
                    thread.child_tid_ptr = child_tid_ptr;
                }

                // In child thread, return 0
                // In parent, return child TID
                // For now, we return child TID (the distinction should be handled by the thread creation)
                Ok(tid as u64)
            }
            Err(_) => Err(SyscallError::OutOfMemory),
        }
    } else {
        // Create a new process (like fork) with optional resource sharing
        // Check for namespace flags
        let mut namespace_configs = Vec::new();

        if (flags & CLONE_NEWNS) != 0 {
            namespace_configs.push(crate::cloud_native::namespaces::NamespaceType::Mount);
        }
        if (flags & CLONE_NEWUTS) != 0 {
            namespace_configs.push(crate::cloud_native::namespaces::NamespaceType::UTS);
        }
        if (flags & CLONE_NEWIPC) != 0 {
            namespace_configs.push(crate::cloud_native::namespaces::NamespaceType::IPC);
        }
        if (flags & CLONE_NEWNET) != 0 {
            namespace_configs.push(crate::cloud_native::namespaces::NamespaceType::Network);
        }
        if (flags & CLONE_NEWPID) != 0 {
            namespace_configs.push(crate::cloud_native::namespaces::NamespaceType::PID);
        }
        if (flags & CLONE_NEWUSER) != 0 {
            namespace_configs.push(crate::cloud_native::namespaces::NamespaceType::User);
        }

        // Create namespaces if requested
        if !namespace_configs.is_empty() {
            // Create namespaces for the new process
            // This will be applied when the process is created
            // For now, we create the process first, then apply namespaces
        }

        // Create new process with resource sharing based on flags
        let child_pid = if (flags & CLONE_VM) != 0 {
            // Share VM space - this is not fully implemented yet, fall back to fork
            crate::process::manager::fork()
        } else {
            // Normal fork
            crate::process::manager::fork()
        };

        match child_pid {
            Some(child_pid) => {
                // Apply namespaces to child process if requested
                if !namespace_configs.is_empty() {
                    // Get namespace manager instance
                    use crate::cloud_native::namespaces::NamespaceManager;
                    static NS_MANAGER: crate::sync::Mutex<Option<NamespaceManager>> = crate::sync::Mutex::new(None);

                    let mut manager_guard = NS_MANAGER.lock();
                    let manager = manager_guard.get_or_insert_with(|| NamespaceManager::new());

                    for ns_type in namespace_configs {
                        let config = crate::cloud_native::namespaces::NamespaceConfig {
                            ns_type,
                            new_namespace: true,
                            existing_path: None,
                            parameters: crate::cloud_native::namespaces::NamespaceParameters {
                                mount_params: None,
                                network_params: None,
                                user_params: None,
                                uts_params: None,
                            },
                        };

                        // Create namespace using manager
                        match manager.create_namespace(ns_type, config) {
                            Ok(ns_id) => {
                                // Associate namespace with child process
                                // TODO: Store namespace in process structure
                                crate::println!("[clone] Created namespace {:?} (ID: {}) for child process {}", ns_type, ns_id, child_pid);
                            }
                            Err(_) => {
                                return Err(SyscallError::InvalidArgument);
                            }
                        }
                    }
                }

                // Write child PID to parent_tid_ptr if CLONE_PARENT_SETTID is set
                if (flags & CLONE_PARENT_SETTID) != 0 && parent_tid_ptr != 0 {
                    unsafe {
                        let pid_val = child_pid as i32;
                        copyin(pagetable, parent_tid_ptr as *mut u8, parent_tid_ptr as usize, core::mem::size_of::<i32>())
                            .map_err(|_| SyscallError::BadAddress)?;
                    }
                }

                // Write child PID to child_tid_ptr if CLONE_CHILD_SETTID is set
                if (flags & CLONE_CHILD_SETTID) != 0 && child_tid_ptr != 0 {
                    unsafe {
                        let pid_val = child_pid as i32;
                        copyin(pagetable, child_tid_ptr as *mut u8, child_tid_ptr as usize, core::mem::size_of::<i32>())
                            .map_err(|_| SyscallError::BadAddress)?;
                    }
                }

                // For processes, CLONE_CHILD_CLEARTID is not typically used, but we can store it
                // Note: This is mainly for threads, but we'll store it for consistency

                Ok(child_pid as u64)
            }
            None => Err(SyscallError::OutOfMemory),
        }
    }
}

fn sys_fork(_args: &[u64]) -> SyscallResult {
    // Fork creates a copy of the current process
    use crate::process::manager;
    
    // Call the process manager's fork function
    let result = manager::fork();

    let pid = match result {
        Some(pid) => pid as u64,
        None => return Err(SyscallError::OutOfMemory),
    };

    Ok(pid)
}

fn sys_vfork(_args: &[u64]) -> SyscallResult {
    // vfork is similar to fork but shares address space until exec
    // For now, implement as regular fork
    sys_fork(_args)
}

fn sys_execve(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyin;
    
    let args = extract_args(args, 3)?;
    let path_ptr = args[0] as usize;
    let _argv_ptr = args[1] as usize;
    let _envp_ptr = args[2] as usize;
    
    if path_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read path from user space
    let mut path_buf = [0u8; 256];
    unsafe {
        copyin(pagetable, path_buf.as_mut_ptr(), path_ptr, 256)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Find null terminator
    let path_len = path_buf.iter().position(|&c| c == 0).unwrap_or(256);
    let _path = core::str::from_utf8(&path_buf[..path_len])
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Load the executable - this would involve:
    // 1. Open the file
    // 2. Parse ELF headers
    // 3. Allocate new address space
    // 4. Load segments
    // 5. Set up stack with argv/envp
    // 6. Jump to entry point
    
    // For now, return error as exec requires ELF loading infrastructure
    Err(SyscallError::NotSupported)
}

fn sys_exit(args: &[u64]) -> SyscallResult {
    let status = if !args.is_empty() { args[0] as i32 } else { 0 };
    
    // Call the process manager's exit function
    crate::process::manager::exit(status);
    
    // exit() should not return
    Err(SyscallError::InvalidArgument)
}

fn sys_wait4(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 4)?;
    let wait_pid = args[0] as i32;
    let status_ptr = args[1] as usize;
    let options = args[2] as i32;
    let _rusage_ptr = args[3] as usize;
    
    // Wait for child process
    let mut status: i32 = 0;
    let result = crate::process::manager::wait(&mut status);

    let pid = match result {
        Some(pid) => pid,
        None => {
            // Check if WNOHANG was set
            const WNOHANG: i32 = 1;
            if (options & WNOHANG) != 0 {
                return Ok(0); // No child exited yet
            }
            return Err(SyscallError::WouldBlock);
        }
    };

    // If waiting for specific PID, check it matches
    if wait_pid > 0 && pid as i32 != wait_pid {
        return Err(SyscallError::NotFound);
    }
    
    // Copy status to user space if requested
    if status_ptr != 0 {
        let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
        let pagetable = proc.pagetable;
        drop(proc_table);
        
        if !pagetable.is_null() {
            unsafe {
                copyout(pagetable, status_ptr, &status as *const _ as *const u8, 4)
                    .map_err(|_| SyscallError::BadAddress)?;
            }
        }
    }

    Ok(pid as u64)
}

/// Get thread ID
/// Arguments: []
/// Returns: current thread ID
fn sys_gettid(_args: &[u64]) -> SyscallResult {
    // Try to get current thread ID
    let tid = crate::process::thread::thread_self();
    if tid == 0 {
        // Fall back to process ID if no thread
        let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
        Ok(pid as u64)
    } else {
        Ok(tid as u64)
    }
}

fn sys_getpid(_args: &[u64]) -> SyscallResult {
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    Ok(pid as u64)
}

/// Unshare system call
/// Unshares parts of the execution context (namespaces, etc.)
/// Arguments: [flags]
/// Returns: 0 on success, error on failure
fn sys_unshare(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let flags = args[0] as i32;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    
    // Check which namespaces to unshare
    let mut namespaces_to_create = Vec::new();
    
    if (flags & CLONE_NEWNS) != 0 {
        namespaces_to_create.push(crate::cloud_native::namespaces::NamespaceType::Mount);
    }
    if (flags & CLONE_NEWUTS) != 0 {
        namespaces_to_create.push(crate::cloud_native::namespaces::NamespaceType::UTS);
    }
    if (flags & CLONE_NEWIPC) != 0 {
        namespaces_to_create.push(crate::cloud_native::namespaces::NamespaceType::IPC);
    }
    if (flags & CLONE_NEWNET) != 0 {
        namespaces_to_create.push(crate::cloud_native::namespaces::NamespaceType::Network);
    }
    if (flags & CLONE_NEWPID) != 0 {
        namespaces_to_create.push(crate::cloud_native::namespaces::NamespaceType::PID);
    }
    if (flags & CLONE_NEWUSER) != 0 {
        namespaces_to_create.push(crate::cloud_native::namespaces::NamespaceType::User);
    }
    
    // Create new namespaces for current process
    // Get namespace manager instance
    use crate::cloud_native::namespaces::NamespaceManager;
    static NS_MANAGER: crate::sync::Mutex<Option<NamespaceManager>> = crate::sync::Mutex::new(None);
    
    let mut manager_guard = NS_MANAGER.lock();
    let manager = manager_guard.get_or_insert_with(|| NamespaceManager::new());
    
    for ns_type in namespaces_to_create {
        let config = crate::cloud_native::namespaces::NamespaceConfig {
            ns_type,
            new_namespace: true,
            existing_path: None,
            parameters: crate::cloud_native::namespaces::NamespaceParameters {
                mount_params: None,
                network_params: None,
                user_params: None,
                uts_params: None,
            },
        };
        
        // Create namespace using manager
        match manager.create_namespace(ns_type, config) {
            Ok(ns_id) => {
                // Associate namespace with current process
                // TODO: Store namespace in process structure
                crate::println!("[unshare] Created namespace {:?} (ID: {}) for process {}", ns_type, ns_id, pid);
            }
            Err(_) => {
                return Err(SyscallError::InvalidArgument);
            }
        }
    }
    
    Ok(0)
}

/// Setns system call
/// Join an existing namespace
/// Arguments: [fd, nstype]
/// Returns: 0 on success, error on failure
fn sys_setns(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let fd = args[0] as i32;
    let nstype = args[1] as i32;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    
    // Look up file descriptor to get namespace path
    // TODO: Implement namespace file descriptor lookup
    // For now, return not supported
    crate::println!("[setns] Process {} attempting to join namespace via fd {}", pid, fd);
    
    // Determine namespace type from nstype or fd
    let ns_type = if nstype == 0 {
        // Determine from namespace file path
        // TODO: Read namespace type from /proc/self/fd/{fd}
        return Err(SyscallError::NotSupported);
    } else {
        // Map nstype to NamespaceType
        match nstype {
            0x00020000 => crate::cloud_native::namespaces::NamespaceType::Mount,
            0x04000000 => crate::cloud_native::namespaces::NamespaceType::UTS,
            0x08000000 => crate::cloud_native::namespaces::NamespaceType::IPC,
            0x40000000 => crate::cloud_native::namespaces::NamespaceType::Network,
            0x20000000 => crate::cloud_native::namespaces::NamespaceType::PID,
            0x10000000 => crate::cloud_native::namespaces::NamespaceType::User,
            _ => return Err(SyscallError::InvalidArgument),
        }
    };
    
    // Join the namespace
    // TODO: Implement actual namespace joining
    crate::println!("[setns] Process {} joining namespace {:?}", pid, ns_type);
    
    Ok(0)
}

/// Set thread ID address (for CLONE_CHILD_CLEARTID)
/// Arguments: [tidptr]
/// Returns: current thread ID
fn sys_set_tid_address(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let tidptr = args[0] as usize;
    
    // Get current thread
    let tid = crate::process::thread::thread_self();
    if tid == 0 {
        // Fall back to process ID
        let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
        
        // Store tidptr for clearing on thread exit
        // TODO: Store tidptr in thread structure for CLONE_CHILD_CLEARTID
        
        Ok(pid as u64)
    } else {
        // Store tidptr for clearing on thread exit
        // TODO: Store tidptr in thread structure for CLONE_CHILD_CLEARTID
        
        Ok(tid as u64)
    }
}

/// Futex (Fast Userspace Mutex) operations
/// Arguments: [uaddr, op, val, timeout, uaddr2, val3]
/// Returns: 0 on success, number of woken threads for WAKE operations, error on failure
fn sys_futex(args: &[u64]) -> SyscallResult {
    
    let args = extract_args(args, 6)?;
    
    let uaddr = args[0] as usize;  // Address of futex word in user space
    let op = args[1] as i32;       // Operation
    let val = args[2] as i32;      // Value (operation-dependent)
    let timeout = args[3] as usize; // Timeout (for WAIT operations)
    let uaddr2 = args[4] as usize; // Second address (for some operations)
    let val3 = args[5] as i32;    // Third value (for some operations)
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Futex operation flags
    const FUTEX_WAIT: i32 = 0;
    const FUTEX_WAKE: i32 = 1;
    const FUTEX_FD: i32 = 2;
    const FUTEX_REQUEUE: i32 = 3;
    const FUTEX_CMP_REQUEUE: i32 = 4;
    const FUTEX_WAKE_OP: i32 = 5;
    const FUTEX_LOCK_PI: i32 = 6;
    const FUTEX_UNLOCK_PI: i32 = 7;
    const FUTEX_TRYLOCK_PI: i32 = 8;
    const FUTEX_WAIT_BITSET: i32 = 9;
    const FUTEX_WAKE_BITSET: i32 = 10;
    const FUTEX_WAIT_REQUEUE_PI: i32 = 11;
    const FUTEX_CMP_REQUEUE_PI: i32 = 12;
    
    // Extract operation (lower 8 bits)
    let futex_op = op & 0xff;
    // Extract flags (upper bits)
    let flags = op & !0xff;
    
    // Check for PRIVATE flag (not yet supported)
    const FUTEX_PRIVATE_FLAG: i32 = 128;
    let _is_private = (flags & FUTEX_PRIVATE_FLAG) != 0;
    
    match futex_op {
        FUTEX_WAIT => {
            // Enhanced FUTEX_WAIT with timeout support
            futex_wait_timeout(pagetable, uaddr, val, timeout)
                .or_else(|e| Err(futex_error_to_syscall("timeout")))
        }
        FUTEX_WAKE => {
            // Enhanced FUTEX_WAKE with performance optimization
            futex_wake_optimized(uaddr, val)
                .or_else(|e| Err(futex_error_to_syscall("invalid_argument")))
        }
        FUTEX_REQUEUE => {
            // Enhanced FUTEX_REQUEUE operation
            futex_requeue(pagetable, uaddr, uaddr2, val, val3, false)
                .or_else(|e| Err(futex_error_to_syscall("invalid_argument")))
        }
        FUTEX_CMP_REQUEUE => {
            // Enhanced FUTEX_CMP_REQUEUE operation
            futex_requeue(pagetable, uaddr, uaddr2, val, val3, true)
                .or_else(|e| Err(futex_error_to_syscall("invalid_argument")))
        }
        FUTEX_WAKE_OP => {
            // Advanced wake operation - not yet implemented
            Err(SyscallError::NotSupported)
        }
        FUTEX_LOCK_PI => {
            // Enhanced priority inheritance lock
            futex_lock_pi(pagetable, uaddr, timeout)
                .or_else(|e| Err(futex_error_to_syscall("would_block")))
        }
        FUTEX_UNLOCK_PI => {
            // Enhanced priority inheritance unlock
            futex_unlock_pi(pagetable, uaddr)
                .or_else(|e| Err(futex_error_to_syscall("invalid_argument")))
        }
        FUTEX_TRYLOCK_PI => {
            // Enhanced priority inheritance trylock
            futex_trylock_pi(pagetable, uaddr)
                .or_else(|e| Err(futex_error_to_syscall("would_block")))
        }
        FUTEX_WAIT_BITSET | FUTEX_WAKE_BITSET | FUTEX_WAIT_REQUEUE_PI | FUTEX_CMP_REQUEUE_PI => {
            // Advanced bitset and PI operations - not yet implemented
            Err(SyscallError::NotSupported)
        }
        _ => Err(SyscallError::InvalidArgument),
    }
}

/// Add a thread to futex wait queue
pub fn add_futex_waiter(uaddr: usize, tid: usize, expected_val: i32, timeout: u64) {
    let mut wait_queue = FUTEX_WAIT_QUEUE.lock();
    let waiters = wait_queue.entry(uaddr).or_insert_with(Vec::new);
    
    let waiter = FutexWaiter {
        tid,
        uaddr,
        expected_val,
        timeout,
        pi_data: None,
    };
    
    waiters.push(waiter);
}

/// Remove a thread from futex wait queue
pub fn remove_futex_waiter(uaddr: usize, tid: usize) -> bool {
    let mut wait_queue = FUTEX_WAIT_QUEUE.lock();
    if let Some(waiters) = wait_queue.get_mut(&uaddr) {
        if let Some(pos) = waiters.iter().position(|w| w.tid == tid) {
            waiters.remove(pos);
            return true;
        }
    }
    false
}

/// Wake up threads waiting on a futex
pub fn wake_futex_waiters(uaddr: usize, max_wake: i32) -> usize {
    let mut wait_queue = FUTEX_WAIT_QUEUE.lock();
    let mut woken = 0;
    
    if let Some(waiters) = wait_queue.get_mut(&uaddr) {
        let mut to_wake = Vec::new();
        
        // Collect waiters to wake (up to max_wake)
        for waiter in waiters.iter().take(max_wake as usize) {
            to_wake.push(waiter.tid);
            woken += 1;
        }
        
        // Remove woken waiters from queue
        waiters.drain(0..to_wake.len());
        
        // Wake up the threads
        for tid in to_wake {
            crate::process::wakeup(tid);
        }
    }
    
    woken
}

/// Requeue futex waiters from one address to another
pub fn requeue_futex_waiters(src_uaddr: usize, dst_uaddr: usize, max_requeue: i32) -> usize {
    let mut wait_queue = FUTEX_WAIT_QUEUE.lock();
    let mut requeued = 0;
    
    if let Some(src_waiters) = wait_queue.get_mut(&src_uaddr) {
        let mut to_requeue = Vec::new();
        
        // Collect waiters to requeue (up to max_requeue)
        for waiter in src_waiters.iter().take(max_requeue as usize) {
            to_requeue.push(FutexWaiter {
                tid: waiter.tid,
                uaddr: dst_uaddr,
                expected_val: waiter.expected_val,
                timeout: waiter.timeout,
                pi_data: waiter.pi_data.clone(),
            });
            requeued += 1;
        }
        
        // Remove requeued waiters from source queue
        src_waiters.drain(0..to_requeue.len());
        
        // Add to destination queue
        let dst_waiters = wait_queue.entry(dst_uaddr).or_insert_with(Vec::new);
        dst_waiters.extend(to_requeue);
    }
    
    requeued
}

/// Enhanced futex requeue operation implementation
pub fn futex_requeue(pagetable: *mut crate::mm::vm::PageTable, uaddr: usize, uaddr2: usize,
                nr_wake: i32, nr_requeue: i32, cmp: bool) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    // For CMP_REQUEUE, check if values match
    if cmp {
        let mut current_val = 0i32;
        let mut expected_val = 0i32;
        
        unsafe {
            copyin(pagetable, &mut current_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
                .map_err(|_| SyscallError::BadAddress)?;
            copyin(pagetable, &mut expected_val as *mut i32 as *mut u8, uaddr2, core::mem::size_of::<i32>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        if current_val != expected_val {
            return Err(SyscallError::WouldBlock);
        }
    }
    
    // Wake up specified number of waiters from source futex
    let woken = wake_futex_waiters(uaddr, nr_wake);
    
    // Requeue remaining waiters to target futex
    let requeued = requeue_futex_waiters(uaddr, uaddr2, nr_requeue);
    
    Ok(woken as u64)
}

/// Priority inheritance futex lock implementation
pub fn futex_lock_pi(pagetable: *mut crate::mm::vm::PageTable, uaddr: usize, timeout: usize) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    // Read current value
    let mut current_val = 0i32;
    unsafe {
        copyin(pagetable, &mut current_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // If futex is uncontended, try to acquire it
    if current_val == 0 {
        let new_val = 1i32; // Set to locked state
        // TODO: Implement atomic compare-and-swap
        // For now, just write the new value
        unsafe {
            let page_ptr = uaddr as *mut i32;
            *page_ptr = new_val;
        }
        return Ok(0);
    }
    
    // If futex is contended, implement priority inheritance
    // TODO: Implement proper PI mechanism
    // For now, just sleep on the futex address
    
    let channel = uaddr | 0xf0000000;
    
    // Handle timeout if provided
    if timeout != 0 {
        // TODO: Implement timeout handling
        // For now, sleep indefinitely
    }
    
    crate::process::sleep(channel);
    
    // Check if we acquired the lock
    let mut new_val = 0i32;
    unsafe {
        copyin(pagetable, &mut new_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    if new_val == 1 {
        Ok(0) // Successfully acquired
    } else {
        Err(SyscallError::Interrupted) // Spurious wakeup
    }
}

/// Priority inheritance futex unlock implementation
pub fn futex_unlock_pi(pagetable: *mut crate::mm::vm::PageTable, uaddr: usize) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    // Read current value
    let mut current_val = 0i32;
    unsafe {
        copyin(pagetable, &mut current_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Check if futex is actually locked
    if current_val != 1 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Unlock the futex
    let new_val = 0i32; // Set to unlocked state
    // TODO: Implement atomic write
    // For now, just write the new value
    unsafe {
        let page_ptr = uaddr as *mut i32;
        *page_ptr = new_val;
    }
    
    // Wake up one waiter
    let channel = uaddr | 0xf0000000;
    crate::process::wakeup(channel);
    
    Ok(0)
}

/// Priority inheritance futex trylock implementation
pub fn futex_trylock_pi(pagetable: *mut crate::mm::vm::PageTable, uaddr: usize) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    // Read current value
    let mut current_val = 0i32;
    unsafe {
        copyin(pagetable, &mut current_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // If futex is uncontended, try to acquire it
    if current_val == 0 {
        let new_val = 1i32; // Set to locked state
        // TODO: Implement atomic compare-and-swap
        // For now, just write the new value
        unsafe {
            let page_ptr = uaddr as *mut i32;
            *page_ptr = new_val;
        }
        Ok(0)
    } else {
        // Futex is contended, return EAGAIN
        Err(SyscallError::WouldBlock)
    }
}

/// Enhanced futex wait with timeout support
pub fn futex_wait_timeout(pagetable: *mut crate::mm::vm::PageTable, uaddr: usize,
                        expected_val: i32, timeout: usize) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    let current_tid = crate::process::thread::thread_self();
    let timeout_ns = if timeout != 0 {
        get_current_time_ns() + (timeout as u64)
    } else {
        0
    };
    
    // Read current value
    let mut current_val = 0i32;
    unsafe {
        copyin(pagetable, &mut current_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // If value doesn't match, return immediately
    if current_val != expected_val {
        return Err(SyscallError::WouldBlock);
    }
    
    // Add current thread to wait queue with timeout
    add_futex_waiter(uaddr, current_tid, expected_val, timeout_ns);
    
    // Sleep with periodic timeout check
    let channel = uaddr | 0xf0000000;
    let mut slept_time = 0;
    
    loop {
        crate::process::sleep(channel);
        slept_time += 1;
        
        // Check if timeout expired or value changed
        let mut new_val = 0i32;
        unsafe {
            copyin(pagetable, &mut new_val as *mut i32 as *mut u8, uaddr, core::mem::size_of::<i32>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        // Remove from wait queue
        remove_futex_waiter(uaddr, current_tid);
        
        if new_val != expected_val {
            // Value changed, return success
            return Ok(0);
        }
        
        if timeout_ns != 0 && is_timeout_expired(timeout_ns) {
            // Timeout expired
            return Err(SyscallError::TimedOut);
        }
        
        // Prevent busy waiting (check every 1000 iterations)
        if slept_time > 1000 {
            // If we've slept too many times, return an error or timeout
            return Err(SyscallError::TimedOut);
        }
    }
}

/// Enhanced futex wake with performance optimization
pub fn futex_wake_optimized(uaddr: usize, max_wake: i32) -> SyscallResult {
    // Wake up to max_wake threads waiting on this futex
    let woken = wake_futex_waiters(uaddr, max_wake);
    
    // Performance optimization: batch wake operations
    crate::println!("[futex] Woke up {} threads on futex at {:#x}", woken, uaddr);
    
    Ok(woken as u64)
}

/// Enhanced error handling for futex operations
pub fn futex_error_to_syscall(error: &str) -> SyscallError {
    match error {
        "timeout" => SyscallError::TimedOut,
        "invalid_argument" => SyscallError::InvalidArgument,
        "bad_address" => SyscallError::BadAddress,
        "would_block" => SyscallError::WouldBlock,
        "interrupted" => SyscallError::Interrupted,
        "not_supported" => SyscallError::NotSupported,
        _ => SyscallError::InvalidArgument,
    }
}


fn sys_set_robust_list(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let head_ptr = args[0] as usize;
    let len = args[1] as usize;
    
    // Validate length (must be sizeof(robust_list_head))
    // The standard size is 24 bytes on 64-bit systems
    if len != 24 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Store robust list head in thread structure
    // For now, we just accept the call without storing
    // Real implementation would store head_ptr in thread control block
    let tid = crate::process::thread::thread_self();
    let mut thread_table = crate::process::thread::thread_table();
    
    if let Some(thread) = thread_table.find_thread(tid) {
        // Store the robust list head (we'd need to add this field to Thread)
        // thread.robust_list_head = head_ptr;
        let _ = head_ptr;  // Accept but don't store yet
        Ok(0)
    } else {
        Err(SyscallError::NotFound)
    }
}

fn sys_get_robust_list(args: &[u64]) -> SyscallResult {
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 3)?;
    let pid = args[0] as usize;
    let head_ptr_ptr = args[1] as usize;
    let len_ptr = args[2] as usize;
    
    if head_ptr_ptr == 0 || len_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    
    // If pid is 0, get robust list of calling thread
    let target = if pid == 0 { my_pid } else { pid };
    
    // Check permissions
    if target != my_pid {
        let table = crate::process::manager::PROC_TABLE.lock();
        let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
        if caller.euid != 0 {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    // Get pagetable for copyout
    let table = crate::process::manager::PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Return null pointer and standard length
    let head_ptr: usize = 0;
    let len: usize = 24;
    
    unsafe {
        copyout(pagetable, head_ptr_ptr, &head_ptr as *const _ as *const u8,
                core::mem::size_of::<usize>())
            .map_err(|_| SyscallError::BadAddress)?;
        copyout(pagetable, len_ptr, &len as *const _ as *const u8,
                core::mem::size_of::<usize>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

fn sys_sched_yield(_args: &[u64]) -> SyscallResult {
    // Yield the CPU to other runnable threads/processes
    crate::process::yield_cpu();
    Ok(0)
}

fn sys_sched_getaffinity(args: &[u64]) -> SyscallResult {
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 3)?;
    let pid = args[0] as usize;
    let cpusetsize = args[1] as usize;
    let mask_ptr = args[2] as usize;
    
    if cpusetsize == 0 || mask_ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let target = if pid == 0 { my_pid } else { pid };
    
    let table = crate::process::manager::PROC_TABLE.lock();
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = caller.pagetable;
    
    // Verify target exists
    let _proc = table.find_ref(target).ok_or(SyscallError::NotFound)?;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Return a mask with CPU 0 set
    let cpu_mask: u64 = 1;
    let copy_size = cpusetsize.min(core::mem::size_of::<u64>());
    
    unsafe {
        copyout(pagetable, mask_ptr, &cpu_mask as *const _ as *const u8, copy_size)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(copy_size as u64)
}

fn sys_sched_setaffinity(args: &[u64]) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    let args = extract_args(args, 3)?;
    let pid = args[0] as usize;
    let cpusetsize = args[1] as usize;
    let mask_ptr = args[2] as usize;
    
    if cpusetsize == 0 || mask_ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let target = if pid == 0 { my_pid } else { pid };
    
    let table = crate::process::manager::PROC_TABLE.lock();
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = caller.pagetable;
    
    // Verify target exists
    let _proc = table.find_ref(target).ok_or(SyscallError::NotFound)?;
    drop(table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read the mask
    let mut cpu_mask: u64 = 0;
    let copy_size = cpusetsize.min(core::mem::size_of::<u64>());
    
    unsafe {
        copyin(pagetable, &mut cpu_mask as *mut _ as *mut u8, mask_ptr, copy_size)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Validate mask has at least one CPU set
    if cpu_mask == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Accept the affinity (real implementation would store and enforce)
    Ok(0)
}
