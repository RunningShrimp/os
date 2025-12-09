//! Real-time Scheduling System Calls
//!
//! This module implements system calls for POSIX real-time scheduling features:
//! - sched_setscheduler() - Set scheduling policy
//! - sched_getscheduler() - Get scheduling policy
//! - sched_setparam() - Set scheduling parameters
//! - sched_getparam() - Get scheduling parameters
//! - sched_get_priority_max() - Get maximum priority
//! - sched_get_priority_min() - Get minimum priority
//! - sched_rr_get_interval() - Get round-robin time slice
//! - sched_setaffinity() - Set CPU affinity
//! - sched_getaffinity() - Get CPU affinity

use crate::posix::realtime::*;
use crate::syscalls::common::{SyscallError, SyscallResult};
use crate::process::myproc;

/// System call dispatch for real-time scheduling operations
pub fn dispatch(syscall_num: u32, args: &[u64]) -> SyscallResult {
    match syscall_num {
        0xE000 => sys_sched_setscheduler(args),
        0xE001 => sys_sched_getscheduler(args),
        0xE002 => sys_sched_setparam(args),
        0xE003 => sys_sched_getparam(args),
        0xE004 => sys_sched_get_priority_max(args),
        0xE005 => sys_sched_get_priority_min(args),
        0xE006 => sys_sched_rr_get_interval(args),
        0xE007 => sys_sched_setaffinity(args),
        0xE008 => sys_sched_getaffinity(args),
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// sched_setscheduler system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: policy - Scheduling policy
/// 2: param_ptr - Pointer to sched_param structure
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sched_setscheduler(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;
    let policy = args[1] as i32;
    let param_ptr = args[2] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions (simplified - in real implementation would check capabilities)
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Read sched_param from user space
    let param = if param_ptr != 0 {
        let mut param_data = [0u8; core::mem::size_of::<SchedParam>()];
        
        let pagetable = {
            let proc_table = crate::process::manager::PROC_TABLE.lock();
            let proc = match proc_table.find_ref(current_pid) {
                Some(p) => p,
                None => return Err(SyscallError::NotFound),
            };
            proc.pagetable
        };

        if pagetable.is_null() {
            return Err(SyscallError::BadAddress);
        }

        unsafe {
            match crate::mm::vm::copyin(pagetable, param_data.as_mut_ptr(), param_ptr, param_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        unsafe { core::mem::transmute::<[u8; 4], SchedParam>(param_data) }
    } else {
        SchedParam::default()
    };

    // Set scheduling policy and parameters
    match sched_setscheduler(pid, policy, param) {
        Ok(()) => Ok(0),
        Err(SchedError::InvalidPolicy) => Err(SyscallError::InvalidArgument),
        Err(SchedError::InvalidPriority) => Err(SyscallError::InvalidArgument),
        Err(SchedError::InvalidAffinity) => Err(SyscallError::InvalidArgument),
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
        Err(SchedError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SchedError::NotSupported) => Err(SyscallError::NotSupported),
    }
}

/// sched_getscheduler system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 
/// Returns: scheduling policy on success, negative errno on failure
fn sys_sched_getscheduler(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Get scheduling policy
    match sched_getscheduler(pid) {
        Ok(policy) => Ok(policy as u64),
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
    }
}

/// sched_setparam system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: param_ptr - Pointer to sched_param structure
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sched_setparam(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;
    let param_ptr = args[1] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Read sched_param from user space
    let param = if param_ptr != 0 {
        let mut param_data = [0u8; core::mem::size_of::<SchedParam>()];
        
        let pagetable = {
            let proc_table = crate::process::manager::PROC_TABLE.lock();
            let proc = match proc_table.find_ref(current_pid) {
                Some(p) => p,
                None => return Err(SyscallError::NotFound),
            };
            proc.pagetable
        };

        if pagetable.is_null() {
            return Err(SyscallError::BadAddress);
        }

        unsafe {
            match crate::mm::vm::copyin(pagetable, param_data.as_mut_ptr(), param_ptr, param_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        unsafe { core::mem::transmute::<[u8; 4], SchedParam>(param_data) }
    } else {
        SchedParam::default()
    };

    // Set scheduling parameters
    match sched_setparam(pid, param) {
        Ok(()) => Ok(0),
        Err(SchedError::InvalidPriority) => Err(SyscallError::InvalidArgument),
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
        Err(SchedError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SchedError::InvalidAffinity) => Err(SyscallError::InvalidArgument),
        Err(SchedError::InvalidPolicy) => Err(SyscallError::InvalidArgument),
        Err(SchedError::NotSupported) => Err(SyscallError::NotSupported),
    }
}

/// sched_getparam system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: param_ptr - Pointer to store sched_param structure
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sched_getparam(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;
    let param_ptr = args[1] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Get scheduling parameters
    match sched_getparam(pid) {
        Ok(param) => {
            // Copy parameters back to user space
            if param_ptr != 0 {
                let param_data = unsafe { core::mem::transmute::<SchedParam, [u8; 4]>(param) };
                
                let pagetable = {
                    let proc_table = crate::process::manager::PROC_TABLE.lock();
                    let proc = match proc_table.find_ref(current_pid) {
                        Some(p) => p,
                        None => return Err(SyscallError::NotFound),
                    };
                    proc.pagetable
                };

                if pagetable.is_null() {
                    return Err(SyscallError::BadAddress);
                }

                unsafe {
                    match crate::mm::vm::copyout(pagetable, param_ptr, param_data.as_ptr(), param_data.len()) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
    }
}

/// sched_get_priority_max system call
/// 
/// Arguments:
/// 0: policy - Scheduling policy
/// 
/// Returns: maximum priority on success, negative errno on failure
fn sys_sched_get_priority_max(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let policy = args[0] as i32;

    match sched_get_priority_max(policy) {
        Ok(max_prio) => Ok(max_prio as u64),
        Err(SchedError::InvalidPolicy) => Err(SyscallError::InvalidArgument),
        _ => Err(SyscallError::InvalidArgument),
    }
}

/// sched_get_priority_min system call
/// 
/// Arguments:
/// 0: policy - Scheduling policy
/// 
/// Returns: minimum priority on success, negative errno on failure
fn sys_sched_get_priority_min(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let policy = args[0] as i32;

    match sched_get_priority_min(policy) {
        Ok(min_prio) => Ok(min_prio as u64),
        Err(SchedError::InvalidPolicy) => Err(SyscallError::InvalidArgument),
        _ => Err(SyscallError::InvalidArgument),
    }
}

/// sched_rr_get_interval system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: ts_ptr - Pointer to store timespec structure
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sched_rr_get_interval(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;
    let ts_ptr = args[1] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Get round-robin interval
    match sched_rr_get_interval(pid) {
        Ok(interval_ns) => {
            // Convert to timespec and copy back to user space
            if ts_ptr != 0 {
                let ts = crate::posix::Timespec {
                    tv_sec: (interval_ns / 1_000_000_000) as i64,
                    tv_nsec: (interval_ns % 1_000_000_000) as i64,
                };
                
                let ts_data = unsafe { core::mem::transmute::<crate::posix::Timespec, [u8; 16]>(ts) };
                
                let pagetable = {
                    let proc_table = crate::process::manager::PROC_TABLE.lock();
                    let proc = match proc_table.find_ref(current_pid) {
                        Some(p) => p,
                        None => return Err(SyscallError::NotFound),
                    };
                    proc.pagetable
                };

                if pagetable.is_null() {
                    return Err(SyscallError::BadAddress);
                }

                unsafe {
                    match crate::mm::vm::copyout(pagetable, ts_ptr, ts_data.as_ptr(), ts_data.len()) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
        Err(SchedError::InvalidPolicy) => Err(SyscallError::InvalidArgument),
    }
}

/// sched_setaffinity system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: cpusetsize - Size of CPU set
/// 2: mask_ptr - Pointer to CPU set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sched_setaffinity(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;
    let cpusetsize = args[1] as usize;
    let mask_ptr = args[2] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Read CPU set from user space
    let affinity = if mask_ptr != 0 {
        let mut mask_data = vec![0u8; cpusetsize];
        
        let pagetable = {
            let proc_table = crate::process::manager::PROC_TABLE.lock();
            let proc = match proc_table.find_ref(current_pid) {
                Some(p) => p,
                None => return Err(SyscallError::NotFound),
            };
            proc.pagetable
        };

        if pagetable.is_null() {
            return Err(SyscallError::BadAddress);
        }

        unsafe {
            match crate::mm::vm::copyin(pagetable, mask_data.as_mut_ptr(), mask_ptr, cpusetsize) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        CpuSet::from_bytes(&mask_data)
    } else {
        return Err(SyscallError::InvalidArgument);
    };

    // Set CPU affinity
    match sched_setaffinity(pid, cpusetsize, &affinity) {
        Ok(()) => Ok(0),
        Err(SchedError::InvalidAffinity) => Err(SyscallError::InvalidArgument),
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
        Err(SchedError::PermissionDenied) => Err(SyscallError::PermissionDenied),
        Err(SchedError::InvalidPolicy) => Err(SyscallError::InvalidArgument),
        Err(SchedError::NotSupported) => Err(SyscallError::NotSupported),
    }
}

/// sched_getaffinity system call
/// 
/// Arguments:
/// 0: pid - Process ID
/// 1: cpusetsize - Size of CPU set
/// 2: mask_ptr - Pointer to store CPU set
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sched_getaffinity(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as usize;
    let cpusetsize = args[1] as usize;
    let mask_ptr = args[2] as usize;

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions
    if pid != current_pid && current_pid != 0 {
        return Err(SyscallError::PermissionDenied);
    }

    // Get CPU affinity
    match sched_getaffinity(pid, cpusetsize) {
        Ok(affinity) => {
            // Copy CPU set back to user space
            if mask_ptr != 0 {
                let mask_data = affinity.to_bytes();
                let copy_size = core::cmp::min(cpusetsize, mask_data.len());
                
                let pagetable = {
                    let proc_table = crate::process::manager::PROC_TABLE.lock();
                    let proc = match proc_table.find_ref(current_pid) {
                        Some(p) => p,
                        None => return Err(SyscallError::NotFound),
                    };
                    proc.pagetable
                };

                if pagetable.is_null() {
                    return Err(SyscallError::BadAddress);
                }

                unsafe {
                    match crate::mm::vm::copyout(pagetable, mask_ptr, mask_data.as_ptr(), copy_size) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SchedError::ProcessNotFound) => Err(SyscallError::NotFound),
    }
}

/// Initialize real-time scheduling system calls
pub fn init_realtime_syscalls() {
    crate::println!("[syscall] Initializing real-time scheduling system calls");
    
    // Initialize real-time scheduling subsystem
    init_realtime();
    
    crate::println!("[syscall] Real-time scheduling system calls initialized");
    crate::println!("[syscall]   sched_setscheduler - Set scheduling policy");
    crate::println!("[syscall]   sched_getscheduler - Get scheduling policy");
    crate::println!("[syscall]   sched_setparam - Set scheduling parameters");
    crate::println!("[syscall]   sched_getparam - Get scheduling parameters");
    crate::println!("[syscall]   sched_get_priority_max - Get max priority");
    crate::println!("[syscall]   sched_get_priority_min - Get min priority");
    crate::println!("[syscall]   sched_rr_get_interval - Get RR time slice");
    crate::println!("[syscall]   sched_setaffinity - Set CPU affinity");
    crate::println!("[syscall]   sched_getaffinity - Get CPU affinity");
}

/// Get real-time scheduling system call statistics
pub fn get_realtime_stats() -> RealtimeStats {
    let registry = SCHED_REGISTRY.lock();
    let sched_stats = registry.get_stats();
    
    RealtimeStats {
        total_processes: sched_stats.total_processes,
        realtime_processes: sched_stats.realtime_processes,
        policy_counts: sched_stats.policy_counts,
        total_cpu_time_ms: sched_stats.total_cpu_time_ms,
        cpu_count: sched_stats.cpu_count,
        default_rr_timeslice_ms: sched_stats.default_rr_timeslice_ms,
    }
}

/// Real-time scheduling system call statistics
#[derive(Debug, Clone)]
pub struct RealtimeStats {
    /// Total number of processes
    pub total_processes: usize,
    /// Number of real-time processes
    pub realtime_processes: usize,
    /// Count of processes by policy
    pub policy_counts: [u32; 7],
    /// Total CPU time used by all processes (ms)
    pub total_cpu_time_ms: u64,
    /// Number of CPUs in the system
    pub cpu_count: usize,
    /// Default round-robin time slice (ms)
    pub default_rr_timeslice_ms: u64,
}