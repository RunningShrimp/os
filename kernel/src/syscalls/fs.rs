//! Filesystem-related system calls
//!
//! 文件系统相关系统调用

extern crate alloc;

use crate::error::KernelError;

pub struct Stub;
impl Stub {
    pub fn new() -> Self { Stub }
}

pub fn stub_function() -> Result<(), KernelError> { Ok(()) }

/// Mount a filesystem
///
/// 挂载文件系统
pub fn mount(
    _fs_type: &str,
    _target: &str,
    _source: Option<&str>,
    _flags: u32,
) -> Result<(), i32> {
    crate::println!(
        "[syscalls::fs] mount: fs_type={}, target={}, source={:?}, flags={}",
        _fs_type, _target, _source, _flags
    );
    // Stub implementation - always returns success
    Ok(())
}

/// Unmount a filesystem
///
/// 卸载文件系统
pub fn unmount(_target: &str) -> Result<(), i32> {
    crate::println!("[syscalls::fs] unmount: target={}", _target);
    // Stub implementation - always returns success
    Ok(())
}
