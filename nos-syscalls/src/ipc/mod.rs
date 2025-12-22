//! IPC system calls
//!
//! This module provides inter-process communication system calls.

#[cfg(feature = "alloc")]
use crate::core::SyscallDispatcher;

/// Register IPC system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> nos_api::Result<()> {
    // TODO: Register IPC system calls
    // 使用 dispatcher 参数以避免未使用变量警告
    let _ = dispatcher;
    Ok(())
}