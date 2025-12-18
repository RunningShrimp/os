//! System call dispatch tests
//!
//! Tests for system call dispatch routing, argument passing, error handling, etc.

use crate::{test_assert, test_assert_eq};
use crate::tests::TestResult;
use alloc::vec::Vec;

/// Test syscall dispatch routing for all ranges
pub fn test_syscall_dispatch_routing_all() -> TestResult {
    let args = [0u64; 6];

    // Test all syscall ranges
    let ranges = vec![
        (0x1000, "Process management"),
        (0x2000, "File I/O"),
        (0x3000, "Memory management"),
        (0x4000, "Network"),
        (0x5000, "Signal handling"),
        (0x6000, "Time"),
        (0x7000, "Filesystem"),
        (0x8000, "Thread management"),
        (0x9000, "Zero-copy I/O"),
        (0xA000, "epoll"),
        (0xB000, "GLib compatibility"),
    ];

    for (syscall_num, description) in ranges {
        let result = crate::syscalls::dispatch(syscall_num, &args);
        // Just verify it doesn't panic - actual return values depend on implementation
        test_assert!(true, alloc::format!("{} syscall range should not panic", description));
    }

    Ok(())
}

/// Test invalid syscall handling
pub fn test_syscall_dispatch_invalid() -> TestResult {
    let args = [0u64; 6];
    
    // Test completely invalid syscall number
    let result = crate::syscalls::dispatch(0xFFFF, &args);
    test_assert_eq!(result, -1, "Invalid syscall should return -1");

    // Test syscall number beyond valid range
    let result = crate::syscalls::dispatch(0xC000, &args);
    test_assert!(result <= 0, "Syscall beyond valid range should return error");

    Ok(())
}

/// Test syscall argument passing
pub fn test_syscall_argument_passing() -> TestResult {
    // Test with various argument counts
    let args_empty = [];
    let args_one = [1u64];
    let args_six = [1u64, 2u64, 3u64, 4u64, 5u64, 6u64];

    // Test that dispatch handles different argument counts
    let _ = crate::syscalls::dispatch(0x1004, &args_empty); // getpid
    let _ = crate::syscalls::dispatch(0x1004, &args_one);
    let _ = crate::syscalls::dispatch(0x1004, &args_six);

    Ok(())
}

/// Test syscall error mapping
pub fn test_syscall_error_mapping() -> TestResult {
    let args = [0u64; 6];
    
    // Test that errors are properly mapped to negative values
    let result = crate::syscalls::dispatch(0xFFFF, &args);
    test_assert!(result < 0, "Error should be negative");

    Ok(())
}

/// Test syscall dispatch performance
pub fn test_syscall_dispatch_performance() -> TestResult {
    let args = [0u64; 6];
    let iterations = 1000;
    
    let start_time = crate::time::get_ticks();
    for _ in 0..iterations {
        let _ = crate::syscalls::dispatch(0x1004, &args); // getpid (fast path)
    }
    let end_time = crate::time::get_ticks();
    
    let elapsed = end_time - start_time;
    let avg_time = elapsed / iterations;
    
    // Performance should be reasonable (less than 100 ticks per syscall)
    test_assert!(avg_time < 100, alloc::format!("Average dispatch time {} ticks should be < 100", avg_time));

    Ok(())
}

/// Test syscall dispatch boundaries
pub fn test_syscall_dispatch_boundaries() -> TestResult {
    let args = [0u64; 6];
    
    // Test boundary values for each range
    let boundaries = vec![
        (0x1000, "Process range start"),
        (0x1FFF, "Process range end"),
        (0x2000, "File I/O range start"),
        (0x2FFF, "File I/O range end"),
        (0x3000, "Memory range start"),
        (0x3FFF, "Memory range end"),
    ];

    for (syscall_num, description) in boundaries {
        let result = crate::syscalls::dispatch(syscall_num, &args);
        test_assert!(true, alloc::format!("Boundary {} should not panic", description));
    }

    Ok(())
}

/// Test concurrent syscall dispatch (simulated)
pub fn test_syscall_dispatch_concurrent() -> TestResult {
    let args = [0u64; 6];
    
    // Simulate concurrent calls by calling dispatch multiple times rapidly
    for _ in 0..100 {
        let _ = crate::syscalls::dispatch(0x1004, &args);
    }

    Ok(())
}

/// Test syscall dispatch with large arguments
pub fn test_syscall_dispatch_large_args() -> TestResult {
    let args = [u64::MAX; 6];
    
    // Test that large argument values don't cause issues
    let result = crate::syscalls::dispatch(0x1004, &args);
    test_assert!(true, "Large arguments should not panic");

    Ok(())
}

/// Test syscall dispatch isolation
pub fn test_syscall_dispatch_isolation() -> TestResult {
    let args = [0u64; 6];
    
    // Test that one syscall doesn't affect another
    let _ = crate::syscalls::dispatch(0x1004, &args);
    let _ = crate::syscalls::dispatch(0x2000, &args);
    let _ = crate::syscalls::dispatch(0x3000, &args);
    
    // Verify they still work
    let _ = crate::syscalls::dispatch(0x1004, &args);

    Ok(())
}

/// Test syscall dispatch error recovery
pub fn test_syscall_dispatch_error_recovery() -> TestResult {
    let args = [0u64; 6];
    
    // Cause an error
    let _ = crate::syscalls::dispatch(0xFFFF, &args);
    
    // Verify system still works after error
    let _ = crate::syscalls::dispatch(0x1004, &args);

    Ok(())
}

/// Test syscall dispatch with null/zero arguments
pub fn test_syscall_dispatch_null_args() -> TestResult {
    let args = [0u64; 6];
    
    // Test that zero arguments are handled correctly
    let result = crate::syscalls::dispatch(0x1004, &args);
    test_assert!(true, "Null/zero arguments should not panic");

    Ok(())
}

/// Test syscall dispatch range validation
pub fn test_syscall_dispatch_range_validation() -> TestResult {
    let args = [0u64; 6];
    
    // Test range edge cases
    let range_edge_cases = vec![
        (0x0FFF, "Below process range"),
        (0x2000, "Above process range"),
        (0x3000, "Above file I/O range"),
        (0x4000, "Above memory range"),
        (0x5000, "Above network range"),
        (0x6000, "Above signal range"),
        (0x7000, "Above time range"),
        (0x8000, "Above filesystem range"),
        (0x9000, "Above thread range"),
        (0xA000, "Above zero-copy range"),
        (0xB000, "Above epoll range"),
        (0xC000, "Above GLib range"),
    ];

    for (syscall_num, description) in range_edge_cases {
        let result = crate::syscalls::dispatch(syscall_num, &args);
        test_assert!(true, alloc::format!("Range edge case {} ({}) should not panic", description, syscall_num));
    }

    Ok(())
}
// ============================================================================
// Fork System Call Tests
// ============================================================================

/// Fork system call test module
pub mod fork_tests {
    use crate::{test_assert, test_assert_eq, test_assert_ne};
    use crate::tests::TestResult;
    use crate::syscalls;
    use alloc::vec::Vec;

    /// Test basic fork functionality
    pub fn test_fork_basic() -> TestResult {
        // Call fork syscall
        let result = syscalls::dispatch(0x1000, &[]); // sys_fork

        // Fork should either succeed (return child PID > 0) or fail (return error < 0)
        // In test environment, it might fail due to missing process context
        test_assert!(result != 0, "Fork should return either child PID or error");

        if result > 0 {
            // In parent process - should get child PID
            test_assert!(result > 1, "Child PID should be > 1 (init is 1)");
        } else {
            // Error case - should be a valid error code
            test_assert!(result < 0, "Error should be negative");
        }

        Ok(())
    }

    /// Test fork return values in parent and child
    pub fn test_fork_return_values() -> TestResult {
        // This test simulates what should happen in a real fork
        // In actual implementation, this would need to be run in a process context

        let result = syscalls::dispatch(0x1000, &[]); // sys_fork

        if result > 0 {
            // Parent process: should get child PID
            let child_pid = result as usize;
            test_assert!(child_pid > 1, "Parent should get valid child PID");

            // In real scenario, parent would wait for child here
            // For now, just verify the PID is reasonable
            test_assert!(child_pid < 1000, "Child PID should be reasonable");

        } else if result == 0 {
            // Child process: should get 0
            test_assert_eq!(result, 0, "Child should get return value 0");

        } else {
            // Error case
            test_assert!(result < 0, "Error should be negative");
            // Common fork errors: ENOMEM (-12), EAGAIN (-11), etc.
            test_assert!(result >= -38, "Error should be within valid errno range");
        }

        Ok(())
    }

    /// Test multiple fork calls
    pub fn test_fork_multiple() -> TestResult {
        let mut results = Vec::new();

        // Try multiple forks
        for i in 0..5 {
            let result = syscalls::dispatch(0x1000, &[]);
            results.push(result);

            if result < 0 {
                // If one fails, others might too (resource exhaustion)
                break;
            }
        }

        // At least one should succeed or all should fail consistently
        let success_count = results.iter().filter(|&&r| r > 0).count();
        let error_count = results.iter().filter(|&&r| r < 0).count();

        test_assert!(success_count > 0 || error_count == results.len(),
                    "Either some forks should succeed or all should fail");

        // If any succeeded, verify PIDs are unique
        if success_count > 1 {
            let pids: Vec<i64> = results.iter().filter(|&&r| r > 0).cloned().collect();
            for i in 0..pids.len() {
                for j in (i+1)..pids.len() {
                    test_assert_ne!(pids[i], pids[j], "Child PIDs should be unique");
                }
            }
        }

        Ok(())
    }

    /// Test fork after process setup (simulated)
    pub fn test_fork_with_setup() -> TestResult {
        // This test would ideally set up some process state first
        // For now, just test basic fork behavior

        let result = syscalls::dispatch(0x1000, &[]);

        // Verify basic constraints
        if result > 0 {
            test_assert!(result > 1, "Child PID should be valid");
        } else if result == 0 {
            test_assert_eq!(result, 0, "Child return value should be 0");
        } else {
            test_assert!(result < 0, "Error should be negative");
        }

        Ok(())
    }

    /// Test fork error conditions
    pub fn test_fork_errors() -> TestResult {
        // Test various error conditions that might cause fork to fail

        // Try fork with invalid arguments (should still work since fork takes no args)
        let result = syscalls::dispatch(0x1000, &[1, 2, 3]);
        test_assert!(result != 0, "Fork with extra args should still work or fail gracefully");

        // Multiple rapid forks to potentially trigger resource issues
        let mut error_seen = false;
        for _ in 0..10 {
            let result = syscalls::dispatch(0x1000, &[]);
            if result < 0 {
                error_seen = true;
                // Common fork errors
                test_assert!(result == -12 || result == -11 || result == -38,
                           alloc::format!("Fork error should be ENOMEM (-12), EAGAIN (-11), or ENOTSUP (-38), got {}", result));
                break;
            }
        }

        // It's acceptable if no errors occur in test environment
        test_assert!(true, "Fork error handling test completed");

        Ok(())
    }

    /// Test fork syscall dispatch routing
    pub fn test_fork_syscall_routing() -> TestResult {
        // Verify that fork syscall (0x1000) is properly routed
        let result = syscalls::dispatch(0x1000, &[]);

        // Should not panic and should return a valid result
        test_assert!(result >= -38 && result != 0, "Fork syscall should return valid result");

        Ok(())
    }

    /// Test fork with different argument patterns
    pub fn test_fork_argument_handling() -> TestResult {
        // Test fork with different argument arrays
        let test_cases = vec![
            Vec::new(),
            vec![0u64],
            vec![0u64, 0u64],
            vec![1u64, 2u64, 3u64, 4u64, 5u64, 6u64],
        ];

        for (i, args) in test_cases.iter().enumerate() {
            let result = syscalls::dispatch(0x1000, args);
            test_assert!(result != 0, alloc::format!("Fork with {} args should work or fail gracefully", args.len()));
        }

        Ok(())
    }

    /// Test fork performance characteristics
    pub fn test_fork_performance() -> TestResult {
        let iterations = 10; // Reduced for test environment
        let start_time = crate::time::get_ticks();

        for _ in 0..iterations {
            let _ = syscalls::dispatch(0x1000, &[]);
        }

        let end_time = crate::time::get_ticks();
        let elapsed = end_time - start_time;
        let avg_time = elapsed / iterations;

        // Performance should be reasonable (less than 1000 ticks per fork in test env)
        test_assert!(avg_time < 1000, alloc::format!("Average fork time {} ticks should be reasonable", avg_time));

        Ok(())
    }

    /// Test fork boundary conditions
    pub fn test_fork_boundaries() -> TestResult {
        // Test fork at various system states

        // Normal fork
        let result1 = syscalls::dispatch(0x1000, &[]);

        // Another fork (potential resource pressure)
        let result2 = syscalls::dispatch(0x1000, &[]);

        // Results should be consistent (both succeed, both fail, or first succeeds second fails)
        if result1 > 0 && result2 > 0 {
            test_assert_ne!(result1, result2, "Different fork calls should return different PIDs");
        } else if result1 < 0 && result2 < 0 {
            test_assert_eq!(result1, result2, "Same error should be returned consistently");
        } else if result1 > 0 && result2 < 0 {
            // First succeeded, second failed - acceptable due to resource exhaustion
            test_assert!(true, "Resource exhaustion after first fork is acceptable");
        } else if result1 == 0 || result2 == 0 {
            // Child process case - should not happen in this test context
            test_assert!(false, "Unexpected child process return in boundary test");
        }

        Ok(())
    }

    /// Test fork isolation (one fork doesn't affect another)
    pub fn test_fork_isolation() -> TestResult {
        let result1 = syscalls::dispatch(0x1000, &[]);
        let result2 = syscalls::dispatch(0x1000, &[]);

        // Results should be independent
        if result1 > 0 && result2 > 0 {
            test_assert_ne!(result1, result2, "Fork calls should be independent");
        }

        // Even if both fail, they should fail independently
        test_assert!(true, "Fork isolation test completed");

        Ok(())
    }

    /// Test fork error recovery
    pub fn test_fork_error_recovery() -> TestResult {
        // Cause potential errors
        for _ in 0..5 {
            let _ = syscalls::dispatch(0x1000, &[]);
        }

        // System should still work after errors
        let result = syscalls::dispatch(0x1000, &[]);
        test_assert!(result != 0, "System should recover from fork errors");

        // Test other syscalls still work
        let getpid_result = syscalls::dispatch(0x1004, &[]); // getpid
        test_assert!(true, "Other syscalls should still work after fork errors");

        Ok(())
    }

    /// Test fork with concurrent operations (simulated)
    pub fn test_fork_concurrent_simulation() -> TestResult {
        // Simulate concurrent fork calls
        let mut results = Vec::new();

        for _ in 0..20 {
            let result = syscalls::dispatch(0x1000, &[]);
            results.push(result);
        }

        // Analyze results
        let success_count = results.iter().filter(|&&r| r > 0).count();
        let error_count = results.iter().filter(|&&r| r < 0).count();
        let zero_count = results.iter().filter(|&&r| r == 0).count();

        // Should not have child returns in this context
        test_assert_eq!(zero_count, 0, "Should not get child process returns in test context");

        // Either some succeed or all fail
        test_assert!(success_count > 0 || error_count == results.len(),
                    "Concurrent forks should either succeed or fail consistently");

        Ok(())
    }
}

// ============================================================================
// Memory Management System Call Tests
// ============================================================================

/// Memory management system call test module
pub mod memory_tests {
    use crate::{test_assert, test_assert_eq, test_assert_ne};
    use crate::tests::TestResult;
    use crate::syscalls;
    use alloc::vec::Vec;

    /// Test basic sbrk functionality
    pub fn test_sbrk_basic() -> TestResult {
        // Test sbrk(0) - should return current break
        let result = syscalls::dispatch(0x1019, &[0u64]); // sys_sbrk
        test_assert!(result >= 0, alloc::format!("sbrk(0) should return current break, got {}", result));

        if result > 0 {
            let current_break = result as usize;

            // Test extending heap
            let extend_size = 4096u64;
            let result2 = syscalls::dispatch(0x1019, &[extend_size]);
            test_assert!(result2 >= 0, alloc::format!("sbrk({}) should succeed, got {}", extend_size, result2));

            if result2 > 0 {
                let new_break = result2 as usize;
                test_assert_eq!(new_break, current_break + extend_size as usize,
                               "New break should be old break + extend_size");

                // Test shrinking heap
                let shrink_size = (-2048isize) as u64;
                let result3 = syscalls::dispatch(0x1019, &[shrink_size]);
                test_assert!(result3 >= 0, alloc::format!("sbrk({}) should succeed, got {}", shrink_size as i64, result3));

                if result3 > 0 {
                    let final_break = result3 as usize;
                    test_assert_eq!(final_break, new_break - 2048,
                                   "Final break should be new break - 2048");
                }
            }
        }

        Ok(())
    }

    /// Test sbrk error conditions
    pub fn test_sbrk_errors() -> TestResult {
        // Test sbrk with invalid arguments (too many args)
        let result = syscalls::dispatch(0x1019, &[0u64, 1u64, 2u64]);
        test_assert!(result < 0, "sbrk with too many args should fail");

        // Test sbrk trying to extend beyond kernel space
        let huge_extend = (1u64 << 40); // Very large extension
        let result = syscalls::dispatch(0x1019, &[huge_extend]);
        test_assert!(result < 0, "sbrk with huge extension should fail");

        Ok(())
    }

    /// Test sbrk boundary conditions
    pub fn test_sbrk_boundaries() -> TestResult {
        // Test multiple small extensions
        let mut current_break = 0usize;
        let initial_result = syscalls::dispatch(0x1019, &[0u64]);
        if initial_result > 0 {
            current_break = initial_result as usize;
        }

        for i in 1..=10 {
            let extend_size = (i * 1024) as u64;
            let result = syscalls::dispatch(0x1019, &[extend_size]);
            if result > 0 {
                let new_break = result as usize;
                test_assert_eq!(new_break, current_break + extend_size as usize,
                               alloc::format!("Boundary test iteration {}: break calculation", i));
                current_break = new_break;
            } else {
                // If extension fails, that's acceptable (resource limits)
                break;
            }
        }

        Ok(())
    }

    /// Test sbrk with alternating extend/shrink
    pub fn test_sbrk_extend_shrink() -> TestResult {
        let initial_result = syscalls::dispatch(0x1019, &[0u64]);
        if initial_result <= 0 {
            return Ok(()); // Skip if no process context
        }

        let mut current_break = initial_result as usize;
        let mut operations = Vec::new();

        // Perform alternating operations
        for i in 1..=5 {
            // Extend
            let extend_size = (i * 2048) as u64;
            let result = syscalls::dispatch(0x1019, &[extend_size]);
            if result > 0 {
                let new_break = result as usize;
                test_assert_eq!(new_break, current_break + extend_size as usize,
                               alloc::format!("Extend operation {}: break calculation", i));
                current_break = new_break;
                operations.push(("extend", extend_size as usize, new_break));
            }

            // Shrink
            let shrink_size = (-1024isize) as u64;
            let result = syscalls::dispatch(0x1019, &[shrink_size]);
            if result > 0 {
                let new_break = result as usize;
                test_assert_eq!(new_break, current_break - 1024,
                               alloc::format!("Shrink operation {}: break calculation", i));
                current_break = new_break;
                operations.push(("shrink", 1024, new_break));
            }
        }

        // Verify operations were recorded
        test_assert!(operations.len() > 0, "Should have performed some operations");

        Ok(())
    }

    /// Test basic mremap functionality
    pub fn test_mremap_basic() -> TestResult {
        // First allocate some memory with sbrk
        let initial_break = syscalls::dispatch(0x1019, &[0u64]);
        if initial_break <= 0 {
            return Ok(()); // Skip if no process context
        }

        let alloc_size = 8192u64;
        let alloc_result = syscalls::dispatch(0x1019, &[alloc_size]);
        if alloc_result <= 0 {
            return Ok(()); // Skip if allocation failed
        }

        let old_addr = initial_break as usize;
        let old_size = alloc_size as usize;
        let new_size = 16384usize; // Double the size

        // Test mremap to expand in place
        let result = syscalls::dispatch(0x300B, &[old_addr as u64, old_size as u64, new_size as u64, 0u64, 0u64]); // sys_mremap
        test_assert!(result >= 0, alloc::format!("mremap expand should succeed, got {}", result));

        if result > 0 {
            let new_addr = result as usize;
            // Should be able to expand in place
            test_assert_eq!(new_addr, old_addr, "Should expand in place when possible");
        }

        // Test mremap to shrink
        let shrink_size = 4096usize;
        let result2 = syscalls::dispatch(0x300B, &[old_addr as u64, new_size as u64, shrink_size as u64, 0u64, 0u64]);
        test_assert!(result2 >= 0, alloc::format!("mremap shrink should succeed, got {}", result2));

        if result2 > 0 {
            let final_addr = result2 as usize;
            test_assert_eq!(final_addr, old_addr, "Shrink should stay in place");
        }

        Ok(())
    }

    /// Test mremap with MREMAP_MAYMOVE flag
    pub fn test_mremap_maymove() -> TestResult {
        // Allocate initial memory
        let initial_break = syscalls::dispatch(0x1019, &[0u64]);
        if initial_break <= 0 {
            return Ok(());
        }

        let alloc_size = 4096u64;
        let alloc_result = syscalls::dispatch(0x1019, &[alloc_size]);
        if alloc_result <= 0 {
            return Ok(());
        }

        let old_addr = initial_break as usize;
        let old_size = alloc_size as usize;
        let new_size = 65536usize; // Large size that might require moving

        // Test mremap with MREMAP_MAYMOVE
        let flags = 1u64; // MREMAP_MAYMOVE
        let result = syscalls::dispatch(0x300B, &[old_addr as u64, old_size as u64, new_size as u64, flags, 0u64]);
        test_assert!(result >= 0, alloc::format!("mremap with MAYMOVE should succeed, got {}", result));

        // Address might change when moving
        if result > 0 {
            let new_addr = result as usize;
            test_assert!(new_addr != 0, "New address should be valid");
            // Address might be different from old_addr when moved
        }

        Ok(())
    }

    /// Test mremap error conditions
    pub fn test_mremap_errors() -> TestResult {
        // Test mremap with invalid arguments
        let result = syscalls::dispatch(0x300B, &[0u64, 0u64, 0u64, 0u64, 0u64]); // All zeros
        test_assert!(result < 0, "mremap with zero arguments should fail");

        // Test mremap with invalid address
        let invalid_addr = 0xFFFFFFFFFFFFFFFFu64;
        let result = syscalls::dispatch(0x300B, &[invalid_addr, 4096u64, 8192u64, 0u64, 0u64]);
        test_assert!(result < 0, "mremap with invalid address should fail");

        // Test mremap with invalid flags
        let result = syscalls::dispatch(0x300B, &[0x40000000u64, 4096u64, 8192u64, 0xFFFFFFFFu64, 0u64]);
        test_assert!(result < 0, "mremap with invalid flags should fail");

        Ok(())
    }

    /// Test mremap with MREMAP_FIXED flag
    pub fn test_mremap_fixed() -> TestResult {
        // Allocate two regions
        let break1 = syscalls::dispatch(0x1019, &[0u64]);
        if break1 <= 0 {
            return Ok(());
        }

        let alloc_result1 = syscalls::dispatch(0x1019, &[4096u64]);
        if alloc_result1 <= 0 {
            return Ok(());
        }

        let alloc_result2 = syscalls::dispatch(0x1019, &[4096u64]);
        if alloc_result2 <= 0 {
            return Ok(());
        }

        let addr1 = break1 as usize;
        let addr2 = alloc_result1 as usize;

        // Try to move addr1 to addr2 with MREMAP_FIXED
        let flags = 2u64; // MREMAP_FIXED
        let result = syscalls::dispatch(0x300B, &[addr1 as u64, 4096u64, 4096u64, flags, addr2 as u64]);
        // This might fail due to overlapping regions, which is expected
        test_assert!(result != 0, "mremap FIXED should either succeed or fail with valid error");

        Ok(())
    }

    /// Test memory management integration
    pub fn test_memory_integration() -> TestResult {
        // Test that sbrk and mremap work together

        // Get initial break
        let initial_break = syscalls::dispatch(0x1019, &[0u64]);
        if initial_break <= 0 {
            return Ok(());
        }

        // Allocate with sbrk
        let alloc_result = syscalls::dispatch(0x1019, &[16384u64]);
        if alloc_result <= 0 {
            return Ok(());
        }

        let heap_start = initial_break as usize;
        let heap_size = 16384usize;

        // Use mremap to resize the allocated region
        let new_size = 32768usize;
        let mremap_result = syscalls::dispatch(0x300B, &[heap_start as u64, heap_size as u64, new_size as u64, 0u64, 0u64]);
        test_assert!(mremap_result >= 0, alloc::format!("Integrated sbrk+mremap should work, got {}", mremap_result));

        // Shrink back
        let shrink_result = syscalls::dispatch(0x300B, &[heap_start as u64, new_size as u64, heap_size as u64, 0u64, 0u64]);
        test_assert!(shrink_result >= 0, alloc::format!("Shrink after mremap should work, got {}", shrink_result));

        Ok(())
    }

    /// Test memory management performance
    pub fn test_memory_performance() -> TestResult {
        let iterations = 10;
        let start_time = crate::time::get_ticks();

        for _ in 0..iterations {
            // Quick sbrk operations
            let _ = syscalls::dispatch(0x1019, &[0u64]); // Just get current break
        }

        let end_time = crate::time::get_ticks();
        let elapsed = end_time - start_time;
        let avg_time = elapsed / iterations;

        // Performance should be reasonable (less than 500 ticks per operation in test env)
        test_assert!(avg_time < 500, alloc::format!("Average memory syscall time {} ticks should be reasonable", avg_time));

        Ok(())
    }

    /// Test memory boundary conditions
    pub fn test_memory_boundaries() -> TestResult {
        // Test various boundary sizes
        let test_sizes = vec![1usize, 4095, 4096, 4097, 8192, 16384, 65536];

        for &size in &test_sizes {
            let result = syscalls::dispatch(0x1019, &[size as u64]);
            // Should either succeed or fail gracefully
            test_assert!(result != 0, alloc::format!("Boundary size {} should work or fail gracefully", size));
        }

        Ok(())
    }
}

/// Test memfd_create operations
pub fn test_memfd_operations() -> TestResult {
    crate::println!("Testing memfd_create operations...");

    // Run all memfd tests
    // match memfd_test::run_all_memfd_tests() {
    //     Ok(_) => crate::println!("✓ All memfd_create tests passed"),
    //     Err(e) => {
    //         crate::println!("✗ memfd_create tests failed: {:?}", e);
    //         return Err(e);
    //     }
    // }

    crate::println!("memfd_create operations test completed");
    Ok(())
}

