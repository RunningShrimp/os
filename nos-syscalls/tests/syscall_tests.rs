//! System call tests

use nos_syscalls::*;

#[test]
fn test_syscall_init() {
    // Test system call initialization
    assert!(init_syscalls().is_ok());
}

#[test]
fn test_syscall_stats() {
    // Test system call statistics
    let stats = get_syscall_stats();
    assert_eq!(stats.total_calls, 0);
    assert_eq!(stats.error_count, 0);
    assert_eq!(stats.avg_execution_time, 0);
}

#[test]
fn test_syscall_result() {
    // Test system call result conversion
    let success = types::SyscallResult::Success(42);
    assert_eq!(success.to_isize(), 42);
    
    let error = types::SyscallResult::Error(2);
    assert_eq!(error.to_isize(), -2);
    
    assert_eq!(types::SyscallResult::from_isize(42), types::SyscallResult::Success(42));
    assert_eq!(types::SyscallResult::from_isize(-2), types::SyscallResult::Error(2));
}