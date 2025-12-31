//! POSIX File Descriptor System Calls Registration
//!
//! This module registers timerfd, eventfd, and signalfd system calls
//! with the unified dispatcher.

use super::{
    dispatch::unified::get_unified_dispatcher,
    eventfd::{sys_eventfd, sys_eventfd2},
    signalfd::{sys_signalfd, sys_signalfd4},
    timerfd::{sys_timerfd_create, sys_timerfd_gettime, sys_timerfd_settime},
};
use crate::subsystems::syscalls::interface::InterfaceSyscallError;

/// Wrapper functions to convert syscall results to FastPathHandler expected type
/// The FastPathHandler type is: fn(u32, &[u64]) -> Result<u64, InterfaceSyscallError>
fn wrap_eventfd(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_eventfd(args).map(|v| v as u64)
}

fn wrap_eventfd2(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_eventfd2(args).map(|v| v as u64)
}

fn wrap_timerfd_create(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_timerfd_create(args).map(|v| v as u64)
}

fn wrap_timerfd_settime(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_timerfd_settime(args).map(|v| v as u64)
}

fn wrap_timerfd_gettime(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_timerfd_gettime(args).map(|v| v as u64)
}

fn wrap_signalfd(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_signalfd(args).map(|v| v as u64)
}

fn wrap_signalfd4(_num: u32, args: &[u64]) -> Result<u64, InterfaceSyscallError> {
    sys_signalfd4(args).map(|v| v as u64)
}

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
pub fn register_posix_fd_syscalls() -> Result<(), crate::subsystems::syscalls::interface::InterfaceSyscallError> {
    let dispatcher_mutex = get_unified_dispatcher()
        .ok_or(crate::subsystems::syscalls::interface::InterfaceSyscallError::InterfaceNotFound)?;

    let dispatcher = dispatcher_mutex.lock();
    if let Some(ref d) = *dispatcher {
        use syscall_numbers::*;

        // Register fast-path handlers for these syscalls
        // The wrapper functions already match FastPathHandler signature: fn(u32, &[u64]) -> Result<u64, InterfaceSyscallError>
        d.register_fast_path(SYS_EVENTFD, wrap_eventfd)?;
        d.register_fast_path(SYS_EVENTFD2, wrap_eventfd2)?;
        d.register_fast_path(SYS_TIMERFD_CREATE, wrap_timerfd_create)?;
        d.register_fast_path(SYS_TIMERFD_SETTIME, wrap_timerfd_settime)?;
        d.register_fast_path(SYS_TIMERFD_GETTIME, wrap_timerfd_gettime)?;
        d.register_fast_path(SYS_SIGNALFD, wrap_signalfd)?;
        d.register_fast_path(SYS_SIGNALFD4, wrap_signalfd4)?;
    }

    Ok(())
}
