//! Advanced Signal System Calls
//!
//! This module implements system calls for advanced POSIX signal handling features:
//! - sigqueue() - Queue a signal to a process
//! - sigtimedwait() - Wait for signals with timeout
//! - sigwaitinfo() - Wait for signals
//! - sigaltstack() - Set alternate signal stack
//! - pthread_sigmask() - Set thread signal mask

use crate::posix::advanced_signal::*;
use crate::posix::{SigSet, SigVal, Timespec, StackT, SigInfoT};
use crate::syscalls::common::{SyscallError, SyscallResult};
use crate::process::myproc;

/// System call dispatch for advanced signal operations
pub fn dispatch(syscall_num: u32, args: &[u64]) -> SyscallResult {
    match syscall_num {
        0x5000 => sys_sigqueue(args),
        0x5001 => sys_sigtimedwait(args),
        0x5002 => sys_sigwaitinfo(args),
        0x5003 => sys_sigaltstack(args),
        0x5004 => sys_pthread_sigmask(args),
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// sigqueue system call
/// 
/// Arguments:
/// 0: pid - Target process ID
/// 1: sig - Signal number
/// 2: value_ptr - Pointer to sigval union
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sigqueue(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = args[0] as i32;
    let sig = args[1] as i32;
    let value_ptr = args[2] as usize;

    // Validate signal number
    if sig <= 0 || sig > 64 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get current process for permission check
    let current_pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    // Check permissions (simplified - in real implementation would check UID/GID)
    if pid as usize != current_pid && pid != 0 {
        // Only allow sending to self or init process (PID 0) for now
        return Err(SyscallError::PermissionDenied);
    }

    // Read sigval from user space
    let value = if value_ptr != 0 {
        let mut value_data = [0u8; core::mem::size_of::<SigVal>()];
        
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
            match crate::mm::vm::copyin(pagetable, value_data.as_mut_ptr(), value_ptr, value_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        unsafe { core::mem::transmute::<[u8; core::mem::size_of::<SigVal>()], SigVal>(value_data) }
    } else {
        SigVal { sival_int: 0 }
    };

    // Queue the signal
    match sigqueue(pid.try_into().unwrap(), sig, value) {
        Ok(()) => Ok(0),
        Err(SignalQueueError::QueueFull) => Err(SyscallError::WouldBlock),
        Err(SignalQueueError::SignalBlocked) => Err(SyscallError::WouldBlock),
        Err(SignalQueueError::InvalidSignal) => Err(SyscallError::InvalidArgument),
        Err(SignalQueueError::ProcessNotFound) => Err(SyscallError::NotFound),
    }
}

/// sigtimedwait system call
/// 
/// Arguments:
/// 0: sigmask_ptr - Pointer to signal mask
/// 1: info_ptr - Pointer to store signal info
/// 2: timeout_ptr - Pointer to timeout specification
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sigtimedwait(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let sigmask_ptr = args[0] as usize;
    let info_ptr = args[1] as usize;
    let timeout_ptr = args[2] as usize;

    // Get current process
    let pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read signal mask from user space
    let sigmask = if sigmask_ptr != 0 {
        let mut mask_data = [0u8; core::mem::size_of::<SigSet>()];
        
        unsafe {
            match crate::mm::vm::copyin(pagetable, mask_data.as_mut_ptr(), sigmask_ptr, mask_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        unsafe { core::mem::transmute::<[u8; core::mem::size_of::<SigSet>()], SigSet>(mask_data) }
    } else {
        SigSet::empty()
    };

    // Read timeout from user space
    let timeout = if timeout_ptr != 0 {
        let mut timeout_data = [0u8; core::mem::size_of::<Timespec>()];
        
        unsafe {
            match crate::mm::vm::copyin(pagetable, timeout_data.as_mut_ptr(), timeout_ptr, timeout_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        let timeout: Timespec = unsafe { core::mem::transmute::<[u8; core::mem::size_of::<Timespec>()], Timespec>(timeout_data) };
        
        // Validate timeout
        if timeout.tv_sec < 0 || timeout.tv_nsec < 0 || timeout.tv_nsec >= 1_000_000_000 {
            return Err(SyscallError::InvalidArgument);
        }
        
        Some(timeout)
    } else {
        None
    };

    // Wait for signal
    match sigtimedwait(&sigmask, timeout.as_ref()) {
        Ok(info) => {
            // Copy signal info back to user space
            if info_ptr != 0 {
                let info_data = unsafe { core::mem::transmute::<SigInfoT, [u8; core::mem::size_of::<SigInfoT>()]>(info) };
                
                unsafe {
                    match crate::mm::vm::copyout(pagetable, info_ptr, info_data.as_ptr(), info_data.len()) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SignalWaitError::Timeout) => Err(SyscallError::TimedOut),
        Err(SignalWaitError::InvalidTimeout) => Err(SyscallError::InvalidArgument),
        Err(SignalWaitError::ProcessNotFound) => Err(SyscallError::NotFound),
        Err(SignalWaitError::Interrupted) => Err(SyscallError::Interrupted),
        Err(SignalWaitError::InvalidMask) => Err(SyscallError::InvalidArgument),
        _ => Err(SyscallError::InvalidArgument), // Handle any other variants
    }
}

/// sigwaitinfo system call
/// 
/// Arguments:
/// 0: sigmask_ptr - Pointer to signal mask
/// 1: info_ptr - Pointer to store signal info
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sigwaitinfo(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let sigmask_ptr = args[0] as usize;
    let info_ptr = args[1] as usize;

    // Get current process
    let pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read signal mask from user space
    let sigmask = if sigmask_ptr != 0 {
        let mut mask_data = [0u8; core::mem::size_of::<SigSet>()];
        
        unsafe {
            match crate::mm::vm::copyin(pagetable, mask_data.as_mut_ptr(), sigmask_ptr, mask_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        unsafe { core::mem::transmute::<[u8; 8], SigSet>(mask_data) }
    } else {
        SigSet::empty()
    };

    // Wait for signal
    match sigwaitinfo(&sigmask) {
        Ok(info) => {
            // Copy signal info back to user space
            if info_ptr != 0 {
                let info_data = unsafe { core::mem::transmute::<SigInfoT, [u8; 128]>(info) };
                
                unsafe {
                    match crate::mm::vm::copyout(pagetable, info_ptr, info_data.as_ptr(), info_data.len()) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SignalWaitError::ProcessNotFound) => Err(SyscallError::NotFound),
        Err(SignalWaitError::Interrupted) => Err(SyscallError::Interrupted),
        Err(SignalWaitError::InvalidMask) => Err(SyscallError::InvalidArgument),
        Err(SignalWaitError::Timeout) => Err(SyscallError::TimedOut),
        Err(SignalWaitError::InvalidTimeout) => Err(SyscallError::InvalidArgument),
    }
}

/// sigaltstack system call
/// 
/// Arguments:
/// 0: new_stack_ptr - Pointer to new stack specification
/// 1: old_stack_ptr - Pointer to store old stack specification
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_sigaltstack(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let new_stack_ptr = args[0] as usize;
    let old_stack_ptr = args[1] as usize;

    // Get current process
    let pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read new stack from user space
    let new_stack = if new_stack_ptr != 0 {
        let mut stack_data = [0u8; core::mem::size_of::<StackT>()];
        
        unsafe {
            match crate::mm::vm::copyin(pagetable, stack_data.as_mut_ptr(), new_stack_ptr, stack_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        let stack: StackT = unsafe { core::mem::transmute::<[u8; core::mem::size_of::<StackT>()], StackT>(stack_data) };
        
        // Validate stack
        if stack.ss_flags & crate::posix::SS_ONSTACK != 0 {
            return Err(SyscallError::InvalidArgument);
        }
        
        if stack.ss_size < crate::posix::MINSIGSTKSZ && (stack.ss_flags & crate::posix::SS_DISABLE) == 0 {
            return Err(SyscallError::InvalidArgument);
        }
        
        Some(stack)
    } else {
        None
    };

    // Call sigaltstack
    let mut old_stack = StackT::default();
    let result = sigaltstack(
        new_stack.as_ref(),
        Some(&mut old_stack),
    );

    match result {
        Ok(()) => {
            // Copy old stack back to user space
            if old_stack_ptr != 0 {
                let old_stack_data = unsafe { core::mem::transmute::<StackT, [u8; 24]>(old_stack) };
                
                unsafe {
                    match crate::mm::vm::copyout(pagetable, old_stack_ptr, old_stack_data.as_ptr(), old_stack_data.len()) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SignalStackError::StackTooSmall) => Err(SyscallError::InvalidArgument),
        Err(SignalStackError::InvalidSize) => Err(SyscallError::InvalidArgument),
        Err(SignalStackError::AllocationFailed) => Err(SyscallError::OutOfMemory),
        Err(SignalStackError::StackInUse) => Err(SyscallError::InvalidArgument),
        Err(SignalStackError::NoAlternateStack) => Err(SyscallError::BadAddress),
    }
}

/// pthread_sigmask system call
/// 
/// Arguments:
/// 0: how - How to change the mask
/// 1: new_mask_ptr - Pointer to new signal mask
/// 2: old_mask_ptr - Pointer to store old signal mask
/// 
/// Returns: 0 on success, negative errno on failure
fn sys_pthread_sigmask(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let how = args[0] as i32;
    let new_mask_ptr = args[1] as usize;
    let old_mask_ptr = args[2] as usize;

    // Validate how parameter
    match how {
        crate::posix::SIG_BLOCK | crate::posix::SIG_UNBLOCK | crate::posix::SIG_SETMASK => {},
        _ => return Err(SyscallError::InvalidArgument),
    }

    // Get current process
    let pid = match myproc() {
        Some(p) => p,
        None => return Err(SyscallError::NotFound),
    };

    let pagetable = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(pid) {
            Some(p) => p,
            None => return Err(SyscallError::NotFound),
        };
        proc.pagetable
    };

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read new mask from user space
    let new_mask = if new_mask_ptr != 0 {
        let mut mask_data = [0u8; core::mem::size_of::<SigSet>()];
        
        unsafe {
            match crate::mm::vm::copyin(pagetable, mask_data.as_mut_ptr(), new_mask_ptr, mask_data.len()) {
                Ok(_) => {},
                Err(_) => return Err(SyscallError::BadAddress),
            }
        }

        unsafe { core::mem::transmute::<[u8; 8], SigSet>(mask_data) }
    } else {
        SigSet::empty()
    };

    // Call pthread_sigmask
    let mut old_mask = SigSet::empty();
    let result = pthread_sigmask(how, &new_mask, Some(&mut old_mask));

    match result {
        Ok(()) => {
            // Copy old mask back to user space
            if old_mask_ptr != 0 {
                let old_mask_data = unsafe { core::mem::transmute::<SigSet, [u8; 8]>(old_mask) };
                
                unsafe {
                    match crate::mm::vm::copyout(pagetable, old_mask_ptr, old_mask_data.as_ptr(), old_mask_data.len()) {
                        Ok(_) => {},
                        Err(_) => return Err(SyscallError::BadAddress),
                    }
                }
            }
            
            Ok(0)
        }
        Err(SignalMaskError::InvalidHow) => Err(SyscallError::InvalidArgument),
        Err(SignalMaskError::TooManyPending) => Err(SyscallError::WouldBlock),
        Err(SignalMaskError::InvalidSignal) => Err(SyscallError::InvalidArgument),
    }
}

/// Initialize advanced signal system calls
pub fn init_advanced_signal_syscalls() {
    crate::println!("[syscall] Initializing advanced signal system calls");
    
    // Initialize advanced signal handling subsystem
    init_advanced_signal();
    
    crate::println!("[syscall] Advanced signal system calls initialized");
    crate::println!("[syscall]   sigqueue - Queue signals to processes");
    crate::println!("[syscall]   sigtimedwait - Wait for signals with timeout");
    crate::println!("[syscall]   sigwaitinfo - Wait for signals");
    crate::println!("[syscall]   sigaltstack - Set alternate signal stack");
    crate::println!("[syscall]   pthread_sigmask - Set thread signal mask");
}

/// Get advanced signal system call statistics
pub fn get_advanced_signal_stats() -> AdvancedSignalStats {
    let registry = SIGNAL_QUEUE_REGISTRY.lock();
    let registry_stats = registry.get_stats();
    
    AdvancedSignalStats {
        total_processes: registry_stats.total_processes,
        total_pending_signals: registry_stats.total_pending_signals,
        total_real_time_signals: registry_stats.total_real_time_signals,
        total_standard_signals: registry_stats.total_standard_signals,
        real_time_signal_range: get_real_time_signal_range(),
    }
}

/// Advanced signal system call statistics
#[derive(Debug, Clone, Copy)]
pub struct AdvancedSignalStats {
    /// Total number of processes with signal queues
    pub total_processes: usize,
    /// Total number of pending signals
    pub total_pending_signals: usize,
    /// Total number of pending real-time signals
    pub total_real_time_signals: usize,
    /// Total number of pending standard signals
    pub total_standard_signals: usize,
    /// Real-time signal range (min, max)
    pub real_time_signal_range: (i32, i32),
}