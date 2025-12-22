//! POSIX File Descriptor System Calls Registration
//!
//! This module registers timerfd, eventfd, and signalfd system calls
//! with the unified dispatcher.

use super::dispatch::unified::{get_unified_dispatcher};
use super::dispatch::unified::FastPathHandler;
use super::common::SyscallError;
use super::timerfd::{sys_timerfd_create, sys_timerfd_settime, sys_timerfd_gettime};
use super::eventfd::{sys_eventfd, sys_eventfd2};
use super::signalfd::{sys_signalfd, sys_signalfd4};

/// System call numbers for POSIX file descriptor syscalls
pub mod syscall_numbers {
    /// eventfd (legacy)
    pub const SYS_EVENTFD: u32 = 0xB002;
    /// eventfd2
    pub const SYS_EVENTFD2: u32 = 0xB003;
    /// timerfd_create
    pub const SYS_TIMERFD_CREATE: u32 = 0xB004;
    /// timerfd_settime
    pub const SYS_TIMERFD_SETTIME: u32 = 0xB005;
    /// timerfd_gettime
    pub const SYS_TIMERFD_GETTIME: u32 = 0xB006;
    /// signalfd (legacy)
    pub const SYS_SIGNALFD: u32 = 0xB007;
    /// signalfd4
    pub const SYS_SIGNALFD4: u32 = 0xB008;
}

/// Register POSIX file descriptor system calls with the unified dispatcher
pub fn register_posix_fd_syscalls() -> Result<(), SyscallError> {
    let dispatcher_mutex = get_unified_dispatcher()
        .ok_or(SyscallError::SystemError)?;
    
    let dispatcher = dispatcher_mutex.lock();
    if let Some(ref d) = *dispatcher {
        use syscall_numbers::*;
        
        // Register fast-path handlers for these syscalls
        // Wrap functions to match FastPathHandler signature: fn(u32, &[u64]) -> Result<u64, SyscallError>
        d.register_fast_path(SYS_EVENTFD, |_num, args| sys_eventfd(args))?;
        d.register_fast_path(SYS_EVENTFD2, |_num, args| sys_eventfd2(args))?;
        d.register_fast_path(SYS_TIMERFD_CREATE, |_num, args| sys_timerfd_create(args))?;
        d.register_fast_path(SYS_TIMERFD_SETTIME, |_num, args| sys_timerfd_settime(args))?;
        d.register_fast_path(SYS_TIMERFD_GETTIME, |_num, args| sys_timerfd_gettime(args))?;
        d.register_fast_path(SYS_SIGNALFD, |_num, args| sys_signalfd(args))?;
        d.register_fast_path(SYS_SIGNALFD4, |_num, args| sys_signalfd4(args))?;
    }
    
    Ok(())
}

