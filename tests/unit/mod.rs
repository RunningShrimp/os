//! Unit test module
//!
//! This module contains unit tests for NOS components.

pub mod common;
pub mod comprehensive_core_tests;
pub mod error_handling_tests;
pub mod fast_path_tests;
pub mod fs_syscall_tests;
pub mod memory_pool_tests;
pub mod network_socket_tests;
pub mod permission_tests;
pub mod process_tests;
pub mod service_lifecycle_tests;
pub mod stress_stability_tests;
pub mod syscall_tests;
pub mod thread_syscall_tests;

/// Run all unit tests
///
/// # Returns
/// * `TestResult` - Unit test result
pub fn run_unit_tests() -> crate::TestResult {

    
    let mut result = crate::TestResult::new();
    
    // Run each test suite
    let suites = vec![
        ("comprehensive_core_tests", comprehensive_core_tests::run_tests as fn() -> crate::common::TestResult),
        ("error_handling_tests", error_handling_tests::run_tests as fn() -> crate::common::TestResult),
        ("fast_path_tests", fast_path_tests::run_tests as fn() -> crate::common::TestResult),
        ("fs_syscall_tests", fs_syscall_tests::run_tests as fn() -> crate::common::TestResult),
        ("memory_pool_tests", memory_pool_tests::run_tests as fn() -> crate::common::TestResult),
        ("network_socket_tests", network_socket_tests::run_tests as fn() -> crate::common::TestResult),
        ("permission_tests", permission_tests::run_tests as fn() -> crate::common::TestResult),
        ("process_tests", process_tests::run_tests as fn() -> crate::common::TestResult),
        ("service_lifecycle_tests", service_lifecycle_tests::run_tests as fn() -> crate::common::TestResult),
        ("stress_stability_tests", stress_stability_tests::run_tests as fn() -> crate::common::TestResult),
        ("syscall_tests", syscall_tests::run_tests as fn() -> crate::common::TestResult),
        ("thread_syscall_tests", thread_syscall_tests::run_tests as fn() -> crate::common::TestResult),
    ];
    
    for (name, run_suite) in suites {
        println!("  Running {}...", name);
        let suite_result = run_suite();
        result.unit_total += suite_result.total;
        result.unit_passed += suite_result.passed;
    }
    
    result
}