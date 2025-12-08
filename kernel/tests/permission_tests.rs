//! File permission tests
//!
//! Tests for fchmod, fchown, chmod, chown system calls

use crate::tests::{TestResult, test_assert, test_assert_eq};

/// Test fchmod permission check
pub fn test_fchmod_permission_check() -> TestResult {
    // Test with invalid file descriptor
    let args = [
        999u64, // invalid fd
        0o644u64, // mode
    ];
    
    let result = crate::syscalls::dispatch(0x700B, &args); // fchmod
    // Should return error for invalid FD
    test_assert!(result < 0, "fchmod with invalid FD should return error");
    
    Ok(())
}

/// Test fchown permission check
pub fn test_fchown_permission_check() -> TestResult {
    // Test with invalid file descriptor
    let args = [
        999u64, // invalid fd
        0u64, // uid
        0u64, // gid
    ];
    
    let result = crate::syscalls::dispatch(0x700D, &args); // fchown
    // Should return error for invalid FD
    test_assert!(result < 0, "fchown with invalid FD should return error");
    
    Ok(())
}

/// Test chmod permission check
pub fn test_chmod_permission_check() -> TestResult {
    // Test with null pathname (should return error)
    let args = [
        0u64, // null pathname
        0o644u64, // mode
    ];
    
    let result = crate::syscalls::dispatch(0x700A, &args); // chmod
    // Should return error for null pathname
    test_assert!(result < 0, "chmod with null pathname should return error");
    
    Ok(())
}

/// Test chown permission check
pub fn test_chown_permission_check() -> TestResult {
    // Test with null pathname (should return error)
    let args = [
        0u64, // null pathname
        0u64, // uid
        0u64, // gid
    ];
    
    let result = crate::syscalls::dispatch(0x700C, &args); // chown
    // Should return error for null pathname
    test_assert!(result < 0, "chown with null pathname should return error");
    
    Ok(())
}

/// Test permission syscall error handling
pub fn test_permission_error_handling() -> TestResult {
    // Test various error conditions
    let test_cases = vec![
        (0x700A, vec![0u64, 0o777u64], "chmod with null path"),
        (0x700B, vec![999u64, 0o777u64], "fchmod with invalid FD"),
        (0x700C, vec![0u64, 0u64, 0u64], "chown with null path"),
        (0x700D, vec![999u64, 0u64, 0u64], "fchown with invalid FD"),
    ];
    
    for (syscall_num, args, description) in test_cases {
        let result = crate::syscalls::dispatch(syscall_num, &args);
        test_assert!(result < 0, alloc::format!("{} should return error", description));
    }
    
    Ok(())
}

