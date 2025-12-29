//! System Call Module for API
//!
//! This module provides system call interfaces for the API layer.

pub use crate::subsystems::syscalls::interface::{
    SyscallDispatcher,
    SyscallHandler,
    SyscallContext,
    SyscallResult,
    SyscallNumber,
    SyscallArgs,
    SyscallCategory,
    get_syscall_category,
    SyscallStats,
};

pub use crate::error::{SyscallError, KernelErrorExt};