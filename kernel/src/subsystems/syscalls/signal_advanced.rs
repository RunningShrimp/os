//! Signal handling syscalls

use super::common::{SyscallError, SyscallResult};

/// Dispatch signal handling syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Signal operations
        0x5000 => sys_kill(args),           // kill
        0x5001 => sys_sigaction(args),      // sigaction
        0x5002 => sys_sigprocmask(args),    // sigprocmask
        0x5003 => sys_sigpending(args),     // sigpending
        0x5004 => sys_sigsuspend(args),     // sigsuspend
        0x5005 => sys_sigaltstack(args),    // sigaltstack
        0x5006 => sys_pause(args),          // pause
        0x5007 => sys_rt_sigaction(args),   // rt_sigaction
        0x5008 => sys_rt_sigprocmask(args), // rt_sigprocmask
        0x5009 => sys_rt_sigpending(args),  // rt_sigpending
        0x500A => sys_rt_sigtimedwait(args), // rt_sigtimedwait
        0x500B => sys_rt_sigqueueinfo(args), // rt_sigqueueinfo
        0x500C => sys_rt_sigsuspend(args),  // rt_sigsuspend
        0x500D => sys_tkill(args),          // tkill
        0x500E => sys_tgkill(args),         // tgkill
        _ => Err(SyscallError::InvalidSyscall),
    }
}

// Signal handling implementations

/// Send a signal to a process
/// Arguments: [pid, sig]
/// Returns: 0 on success, error on failure
fn sys_kill(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::ipc::signal_enhanced::*;
    
    let args = extract_args(args, 2)?;
    let pid = args[0] as i32;
    let sig = args[1] as u32;
    
    // Validate signal number
    if sig < 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process ID for source
    let source_pid = crate::process::myproc().unwrap_or(0) as i32;
    
    // Create signal info
    let info = crate::ipc::signal::SigInfo {
        signo: sig as i32,
        code: crate::ipc::signal::si_code::SI_USER,
        pid: source_pid,
        uid: 0, // Would get from process
        ..Default::default()
    };
    
    // Send signal using enhanced signal system
    send_signal_enhanced(pid as usize, sig, info, source_pid)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Wake up target process if sleeping
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid as crate::process::Pid) {
        if proc.state == crate::process::ProcState::Sleeping {
            // Set process to runnable so scheduler can dispatch it
            proc.state = crate::process::ProcState::Runnable;
        }
    }
    
    Ok(0)
}

/// Set signal action
/// Arguments: [sig, act_ptr, oldact_ptr]
/// Returns: 0 on success, error on failure
fn sys_sigaction(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::ipc::signal::{SigAction, SIG_DFL, SIG_IGN};
    
    let args = extract_args(args, 3)?;
    let sig = args[0] as u32;
    let act_ptr = args[1] as *const SigAction;
    let oldact_ptr = args[2] as *mut SigAction;
    
    // Validate signal number
    if sig == 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
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
    
    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;
    
    // Get old action if requested
    if !oldact_ptr.is_null() {
        let old_action = signals.get_action(sig);
        unsafe {
            copyout(pagetable, oldact_ptr as usize, core::ptr::addr_of!(old_action) as *const u8, core::mem::size_of::<SigAction>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    // Set new action if provided
    if !act_ptr.is_null() {
        let mut new_action = SigAction::default();
        unsafe {
            copyin(pagetable, core::ptr::addr_of_mut!(new_action) as *mut u8, act_ptr as usize, core::mem::size_of::<SigAction>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        // Validate action
        let handler = new_action.handler;
        if handler != SIG_DFL && handler != SIG_IGN &&
           (handler as usize) < crate::mm::vm::USER_BASE {
            return Err(SyscallError::InvalidArgument);
        }
        
        // Set new action
        let old_action = signals.set_action(sig, new_action)
            .map_err(|_| SyscallError::InvalidArgument)?;
        
        // Return old action if oldact_ptr was provided
        if !oldact_ptr.is_null() {
            unsafe {
                copyout(pagetable, oldact_ptr as usize, core::ptr::addr_of!(old_action) as *const u8, core::mem::size_of::<SigAction>())
                    .map_err(|_| SyscallError::BadAddress)?;
            }
        }
    }
    
    Ok(0)
}

/// Simple signal handling (BSD compatibility)
/// Arguments: [sig, handler]
/// Returns: previous handler on success, error on failure
fn sys_signal(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::ipc::signal::{SigAction, SigActionFlags, SIG_DFL, SIG_IGN};
    
    let args = extract_args(args, 2)?;
    let sig = args[0] as u32;
    let handler = args[1] as usize;
    
    // Validate signal number
    if sig == 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
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
    
    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;
    
    // Create new action based on handler
    let new_action = if handler == SIG_DFL as usize {
        SigAction::default() // Default action
    } else if handler == SIG_IGN as usize {
        SigAction {
            handler: SIG_IGN,
            flags: SigActionFlags::default(),
            mask: crate::ipc::signal::SigSet::empty(),
            restorer: 0,
        }
    } else if handler < crate::mm::vm::USER_BASE {
        SigAction {
            handler,
            flags: SigActionFlags::default(),
            mask: crate::ipc::signal::SigSet::empty(),
            restorer: 0,
        }
    } else {
        return Err(SyscallError::InvalidArgument);
    };
    
    // Set new action and get old action
    let old_action = signals.set_action(sig, new_action)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    // Return old handler
    Ok(old_action.handler as u64)
}

/// Change signal mask
/// Arguments: [how, set_ptr, oldset_ptr]
/// Returns: 0 on success, error on failure
fn sys_sigprocmask(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::ipc::signal::{SigSet, SIG_BLOCK, SIG_UNBLOCK, SIG_SETMASK};
    
    let args = extract_args(args, 3)?;
    let how = args[0] as i32;
    let set_ptr = args[1] as *const SigSet;
    let oldset_ptr = args[2] as *mut SigSet;
    
    // Validate how parameter
    if how != SIG_BLOCK && how != SIG_UNBLOCK && how != SIG_SETMASK {
        return Err(SyscallError::InvalidArgument);
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
    
    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;
    
    // Get old mask if requested
    if !oldset_ptr.is_null() {
        let old_mask = signals.get_mask();
        unsafe {
            copyout(pagetable, oldset_ptr as usize, core::ptr::addr_of!(old_mask) as *const u8, core::mem::size_of::<SigSet>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    // Read new set if provided
    let new_set = if !set_ptr.is_null() {
        let mut set = SigSet::empty();
        unsafe {
            copyin(pagetable, core::ptr::addr_of_mut!(set) as *mut u8, set_ptr as usize, core::mem::size_of::<SigSet>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        Some(set)
    } else {
        None
    };
    
    // Update mask based on how parameter
    let old_mask = if let Some(set) = new_set {
        match how {
            SIG_BLOCK => signals.block(set),
            SIG_UNBLOCK => signals.unblock(set),
            SIG_SETMASK => signals.set_mask(set),
            _ => return Err(SyscallError::InvalidArgument),
        }
    } else {
        signals.get_mask()
    };
    
    // Return old mask if oldset_ptr was provided
    if !oldset_ptr.is_null() {
        unsafe {
            copyout(pagetable, oldset_ptr as usize, core::ptr::addr_of!(old_mask) as *const u8, core::mem::size_of::<SigSet>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    Ok(0)
}

/// Check for pending signals
/// Arguments: [set_ptr]
/// Returns: 0 on success, error on failure
fn sys_sigpending(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    use crate::ipc::signal::SigSet;
    
    let args = extract_args(args, 1)?;
    let set_ptr = args[0] as *mut SigSet;
    
    if set_ptr.is_null() {
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
    
    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;
    
    // Get pending signals
    let pending = signals.pending_signals();
    
    // Copy to user space
    unsafe {
        copyout(pagetable, set_ptr as usize, core::ptr::addr_of!(pending) as *const u8, core::mem::size_of::<SigSet>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

fn sys_sigsuspend(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyin;
    use crate::ipc::signal::SigSet;

    let args = extract_args(args, 1)?;
    let mask_ptr = args[0] as *const SigSet;

    if mask_ptr.is_null() {
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

    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;

    // Read the new mask from user space
    let mut new_mask = SigSet::empty();
    unsafe {
        copyin(pagetable, core::ptr::addr_of_mut!(new_mask) as *mut u8, mask_ptr as usize, core::mem::size_of::<SigSet>())
            .map_err(|_| SyscallError::BadAddress)?;
    }

    // Save current mask and set new mask
    signals.suspend(new_mask);

    // Block until a signal is delivered
    // Put the process to sleep waiting for signals
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    {
        let mut proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find(pid) {
            // Use a special channel for signal waiting
            proc.chan = pid; // Use PID as signal wait channel
            proc.state = crate::process::ProcState::Sleeping;
        }
    }
    
    // Yield to let other processes run
    crate::process::manager::yield_cpu();
    
    // When we wake up, a signal was delivered
    signals.restore_mask();

    // sigsuspend always returns -1 with EINTR
    Err(SyscallError::Interrupted)
}

fn sys_sigaltstack(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};

    // Define stack_t structure for signal alternate stack
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct StackT {
        ss_sp: usize,    // Stack base address
        ss_flags: i32,   // Stack flags
        ss_size: usize,  // Stack size
    }
    
    // Sigaltstack flags
    const SS_ONSTACK: i32 = 1;
    const SS_DISABLE: i32 = 2;

    let args = extract_args(args, 2)?;
    let ss_ptr = args[0] as *const StackT;
    let old_ss_ptr = args[1] as *mut StackT;

    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Get old stack info if requested
    if !old_ss_ptr.is_null() {
        // Return default stack info (no alternate stack configured)
        let old_stack = StackT {
            ss_sp: 0,
            ss_flags: SS_DISABLE,
            ss_size: 0,
        };
        unsafe {
            copyout(pagetable, old_ss_ptr as usize, core::ptr::addr_of!(old_stack) as *const u8, core::mem::size_of::<StackT>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }

    // Set new stack if provided
    if !ss_ptr.is_null() {
        let mut new_stack = StackT::default();
        unsafe {
            copyin(pagetable, core::ptr::addr_of_mut!(new_stack) as *mut u8, ss_ptr as usize, core::mem::size_of::<StackT>())
                .map_err(|_| SyscallError::BadAddress)?;
        }

        // Validate the new stack
        if new_stack.ss_flags != 0 && new_stack.ss_flags != SS_DISABLE && new_stack.ss_flags != SS_ONSTACK {
            return Err(SyscallError::InvalidArgument);
        }
        
        // Minimum stack size check (typically MINSIGSTKSZ = 2048)
        if new_stack.ss_flags != SS_DISABLE && new_stack.ss_size < 2048 {
            return Err(SyscallError::InvalidArgument);
        }
        
        // Store the alternate signal stack info (would be stored in process)
        // For now, just accept the configuration
    }

    Ok(0)
}

fn sys_pause(_args: &[u64]) -> SyscallResult {
    // pause() suspends execution until a signal is delivered
    // It always returns -1 with EINTR

    // Get current process and put it to sleep waiting for signals
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    {
        let mut proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find(pid) {
            // Put process to sleep
            proc.chan = pid; // Use PID as signal wait channel
            proc.state = crate::process::ProcState::Sleeping;
        }
    }
    
    // Yield to let other processes run
    crate::process::manager::yield_cpu();
    
    // When we wake up, a signal was delivered
    Err(SyscallError::Interrupted)
}

fn sys_rt_sigaction(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::ipc::signal::{SigAction, SigActionFlags, SIG_DFL, SIG_IGN};

    let args = extract_args(args, 4)?;
    let sig = args[0] as u32;
    let act_ptr = args[1] as *const SigAction;
    let oldact_ptr = args[2] as *mut SigAction;
    let sigsetsize = args[3] as usize;

    // Validate sigsetsize (should be size of sigset_t, typically 8 or 128)
    if sigsetsize != core::mem::size_of::<crate::ipc::signal::SigSet>() {
        return Err(SyscallError::InvalidArgument);
    }

    // Validate signal number
    if sig == 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
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

    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;

    // Get old action if requested
    if !oldact_ptr.is_null() {
        let old_action = signals.get_action(sig);
        unsafe {
            copyout(pagetable, oldact_ptr as usize, core::ptr::addr_of!(old_action) as *const u8, core::mem::size_of::<SigAction>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }

    // Set new action if provided
    if !act_ptr.is_null() {
        let mut new_action = SigAction::default();
        unsafe {
            copyin(pagetable, core::ptr::addr_of_mut!(new_action) as *mut u8, act_ptr as usize, core::mem::size_of::<SigAction>())
                .map_err(|_| SyscallError::BadAddress)?;
        }

        // Validate action
        let handler = new_action.handler;
        if handler != SIG_DFL && handler != SIG_IGN &&
           (handler as usize) < crate::mm::vm::USER_BASE {
            return Err(SyscallError::InvalidArgument);
        }

        // Set new action
        let old_action = signals.set_action(sig, new_action)
            .map_err(|_| SyscallError::InvalidArgument)?;

        // Return old action if oldact_ptr was provided
        if !oldact_ptr.is_null() {
            unsafe {
                copyout(pagetable, oldact_ptr as usize, core::ptr::addr_of!(old_action) as *const u8, core::mem::size_of::<SigAction>())
                    .map_err(|_| SyscallError::BadAddress)?;
            }
        }
    }

    Ok(0)
}

fn sys_rt_sigprocmask(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::ipc::signal::{SigSet, SIG_BLOCK, SIG_UNBLOCK, SIG_SETMASK};

    let args = extract_args(args, 4)?;
    let how = args[0] as i32;
    let set_ptr = args[1] as *const SigSet;
    let oldset_ptr = args[2] as *mut SigSet;
    let sigsetsize = args[3] as usize;

    // Validate sigsetsize
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return Err(SyscallError::InvalidArgument);
    }

    // Validate how parameter
    if how != SIG_BLOCK && how != SIG_UNBLOCK && how != SIG_SETMASK {
        return Err(SyscallError::InvalidArgument);
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

    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;

    // Get old mask if requested
    if !oldset_ptr.is_null() {
        let old_mask = signals.get_mask();
        unsafe {
            copyout(pagetable, oldset_ptr as usize, core::ptr::addr_of!(old_mask) as *const u8, core::mem::size_of::<SigSet>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }

    // Read new set if provided
    let new_set = if !set_ptr.is_null() {
        let mut set = SigSet::empty();
        unsafe {
            copyin(pagetable, core::ptr::addr_of_mut!(set) as *mut u8, set_ptr as usize, core::mem::size_of::<SigSet>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        Some(set)
    } else {
        None
    };

    // Update mask based on how parameter
    let old_mask = if let Some(set) = new_set {
        match how {
            SIG_BLOCK => signals.block(set),
            SIG_UNBLOCK => signals.unblock(set),
            SIG_SETMASK => signals.set_mask(set),
            _ => return Err(SyscallError::InvalidArgument),
        }
    } else {
        signals.get_mask()
    };

    // Return old mask if oldset_ptr was provided
    if !oldset_ptr.is_null() {
        unsafe {
            copyout(pagetable, oldset_ptr as usize, core::ptr::addr_of!(old_mask) as *const u8, core::mem::size_of::<SigSet>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }

    Ok(0)
}

fn sys_rt_sigpending(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyout;
    use crate::ipc::signal::SigSet;

    let args = extract_args(args, 2)?;
    let set_ptr = args[0] as *mut SigSet;
    let sigsetsize = args[1] as usize;

    // Validate sigsetsize
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return Err(SyscallError::InvalidArgument);
    }

    if set_ptr.is_null() {
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

    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;

    // Get pending signals
    let pending = signals.pending_signals();

    // Copy to user space
    unsafe {
        copyout(pagetable, set_ptr as usize, core::ptr::addr_of!(pending) as *const u8, core::mem::size_of::<SigSet>())
            .map_err(|_| SyscallError::BadAddress)?;
    }

    Ok(0)
}

fn sys_rt_sigtimedwait(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::{copyin, copyout};
    use crate::ipc::signal::{SigSet, SigInfo};

    let args = extract_args(args, 4)?;
    let set_ptr = args[0] as *const SigSet;
    let info_ptr = args[1] as *mut SigInfo;
    let timeout_ptr = args[2] as *const crate::posix::Timespec;
    let sigsetsize = args[3] as usize;

    // Validate sigsetsize
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return Err(SyscallError::InvalidArgument);
    }

    if set_ptr.is_null() {
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

    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;

    // Read the signal set to wait for
    let mut wait_set = SigSet::empty();
    unsafe {
        copyin(pagetable, core::ptr::addr_of_mut!(wait_set) as *mut u8, set_ptr as usize, core::mem::size_of::<SigSet>())
            .map_err(|_| SyscallError::BadAddress)?;
    }

    // Check if any requested signal is already pending
    let pending = signals.pending_signals();
    let deliverable = pending.intersect(&wait_set);

    if let Some(sig) = deliverable.first_signal() {
        // Signal is already pending, dequeue it
        let (signal, info) = signals.dequeue_signal().unwrap(); // We know it's there

        // Copy signal info to user space if requested
        if !info_ptr.is_null() {
            unsafe {
                copyout(pagetable, info_ptr as usize, core::ptr::addr_of!(info) as *const u8, core::mem::size_of::<SigInfo>())
                    .map_err(|_| SyscallError::BadAddress)?;
            }
        }

        return Ok(signal as u64);
    }

    // Calculate deadline if timeout is specified
    let deadline = if !timeout_ptr.is_null() {
        let mut timeout = crate::posix::Timespec { tv_sec: 0, tv_nsec: 0 };
        unsafe {
            copyin(pagetable, core::ptr::addr_of_mut!(timeout) as *mut u8, timeout_ptr as usize, core::mem::size_of::<crate::posix::Timespec>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        let now = crate::time::timestamp_nanos();
        let timeout_ns = timeout.tv_sec as u64 * 1_000_000_000 + timeout.tv_nsec as u64;
        Some(now + timeout_ns)
    } else {
        None // No timeout, block indefinitely
    };
    
    // Block waiting for signal
    loop {
        // Check for timeout
        if let Some(dl) = deadline {
            if crate::time::timestamp_nanos() >= dl {
                return Err(SyscallError::TimedOut);
            }
        }
        
        // Check for pending signal
        let pending = signals.pending_signals();
        let deliverable = pending.intersect(&wait_set);
        
        if let Some(sig) = deliverable.first_signal() {
            // Signal arrived, dequeue it
            let (signal, info) = signals.dequeue_signal().unwrap();
            
            // Copy signal info to user space if requested
            if !info_ptr.is_null() {
                unsafe {
                    copyout(pagetable, info_ptr as usize, core::ptr::addr_of!(info) as *const u8, core::mem::size_of::<SigInfo>())
                        .map_err(|_| SyscallError::BadAddress)?;
                }
            }
            
            return Ok(signal as u64);
        }
        
        // Put process to sleep briefly
        {
            let mut proc_table = crate::process::manager::PROC_TABLE.lock();
            if let Some(proc) = proc_table.find(pid) {
                proc.chan = pid;
                proc.state = crate::process::ProcState::Sleeping;
            }
        }
        crate::process::manager::yield_cpu();
    }
}

fn sys_rt_sigqueueinfo(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyin;
    use crate::ipc::signal::SigInfo;

    let args = extract_args(args, 3)?;
    let pid = args[0] as i32;
    let sig = args[1] as u32;
    let info_ptr = args[2] as *const SigInfo;

    // Validate signal number
    if sig < 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
    }

    // Find target process
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid as crate::process::Pid) {
        if let Some(ref signals) = proc.signals {
            // Read signal info from user space
            let mut info = SigInfo::default();
            if !info_ptr.is_null() {
                let pagetable = proc.pagetable;
                if !pagetable.is_null() {
                    unsafe {
                        copyin(pagetable, core::ptr::addr_of_mut!(info) as *mut u8, info_ptr as usize, core::mem::size_of::<SigInfo>())
                            .map_err(|_| SyscallError::BadAddress)?;
                    }
                }
                // Set signal number in info
                info.signo = sig as i32;
            } else {
                // Create default info
                info.signo = sig as i32;
                info.code = crate::ipc::signal::si_code::SI_QUEUE;
            }

            // Send signal with info to process
            signals.send_signal_info(sig, info)
                .map_err(|_| SyscallError::InvalidArgument)?;

            // Wake up process if it's sleeping
            if proc.state == crate::process::ProcState::Sleeping {
                proc.state = crate::process::ProcState::Runnable;
            }

            Ok(0)
        } else {
            Err(SyscallError::NotFound)
        }
    } else {
        Err(SyscallError::NotFound)
    }
}

fn sys_rt_sigsuspend(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::mm::vm::copyin;
    use crate::ipc::signal::SigSet;

    let args = extract_args(args, 2)?;
    let mask_ptr = args[0] as *const SigSet;
    let sigsetsize = args[1] as usize;

    // Validate sigsetsize
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return Err(SyscallError::InvalidArgument);
    }

    if mask_ptr.is_null() {
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

    // Get signal state
    let signals = proc.signals.as_ref().ok_or(SyscallError::NotFound)?;

    // Read the new mask from user space
    let mut new_mask = SigSet::empty();
    unsafe {
        copyin(pagetable, core::ptr::addr_of_mut!(new_mask) as *mut u8, mask_ptr as usize, core::mem::size_of::<SigSet>())
            .map_err(|_| SyscallError::BadAddress)?;
    }

    // Save current mask and set new mask
    signals.suspend(new_mask);

    // Block until a signal is delivered
    // Put the process to sleep waiting for signals
    {
        let mut proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find(pid) {
            // Use a special channel for signal waiting
            proc.chan = pid; // Use PID as signal wait channel
            proc.state = crate::process::ProcState::Sleeping;
        }
    }
    
    // Yield to let other processes run
    crate::process::manager::yield_cpu();
    
    // When we wake up, a signal was delivered
    signals.restore_mask();

    // rt_sigsuspend always returns -1 with EINTR
    Err(SyscallError::Interrupted)
}

fn sys_tkill(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;

    let args = extract_args(args, 2)?;
    let tid = args[0] as i32;
    let sig = args[1] as u32;

    // Validate signal number
    if sig < 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
    }

    // For now, treat tid as pid (single-threaded processes)
    // Treat tid as pid (single-threaded processes for now)
    let pid = tid;

    // Find target process
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid as crate::process::Pid) {
        if let Some(ref signals) = proc.signals {
            // Send signal to process
            signals.send_signal(sig)
                .map_err(|_| SyscallError::InvalidArgument)?;

            // Wake up process if it's sleeping
            if proc.state == crate::process::ProcState::Sleeping {
                proc.state = crate::process::ProcState::Runnable;
            }

            Ok(0)
        } else {
            Err(SyscallError::NotFound)
        }
    } else {
        Err(SyscallError::NotFound)
    }
}

fn sys_tgkill(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;

    let args = extract_args(args, 3)?;
    let tgid = args[0] as i32;
    let tid = args[1] as i32;
    let sig = args[2] as u32;

    // Validate signal number
    if sig < 0 || sig >= crate::ipc::signal::NSIG as u32 {
        return Err(SyscallError::InvalidArgument);
    }

    // Treat tgid and tid as the same (single-threaded processes for now)
    let pid = if tgid != 0 { tgid } else { tid };

    // Find target process
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid as crate::process::Pid) {
        if let Some(ref signals) = proc.signals {
            // Send signal to process
            signals.send_signal(sig)
                .map_err(|_| SyscallError::InvalidArgument)?;

            // Wake up process if it's sleeping
            if proc.state == crate::process::ProcState::Sleeping {
                proc.state = crate::process::ProcState::Runnable;
            }

            Ok(0)
        } else {
            Err(SyscallError::NotFound)
        }
    } else {
        Err(SyscallError::NotFound)
    }
}

/// Kill a process (exported function for compatibility)
pub fn kill_process(pid: u64, signal: i32) -> Result<(), i32> {
    // Validate signal number
    if signal < 0 || signal >= crate::ipc::signal::NSIG as i32 {
        return Err(crate::reliability::errno::EINVAL);
    }
    
    // Find target process
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid as crate::process::Pid) {
        if let Some(ref signals) = proc.signals {
            // Send signal to process
            signals.send_signal(signal as u32)
                .map_err(|_| crate::reliability::errno::EINVAL)?;
            
            // Wake up process if it's sleeping
            if proc.state == crate::process::ProcState::Sleeping {
                proc.state = crate::process::ProcState::Runnable;
            }
            
            Ok(())
        } else {
            Err(crate::reliability::errno::ESRCH)
        }
    } else {
        Err(crate::reliability::errno::ESRCH)
    }
}
