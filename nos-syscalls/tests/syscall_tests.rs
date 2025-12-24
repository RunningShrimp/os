//! System call tests

use nos_syscalls::*;
use nos_api::syscall::SyscallResult;

#[test]
fn test_syscall_init() {
    // Test system call initialization
    assert!(init_dispatcher().is_ok());
}

#[test]
fn test_syscall_result() {
    // Test system call result conversion
    let success = SyscallResult::Success(42);
    assert_eq!(success.to_isize(), 42);
    
    let error = SyscallResult::Error(2);
    assert_eq!(error.to_isize(), -2);
    
    assert_eq!(SyscallResult::from_isize(42), SyscallResult::Success(42));
    assert_eq!(SyscallResult::from_isize(-2), SyscallResult::Error(2));
}