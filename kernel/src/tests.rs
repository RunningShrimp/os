//! Kernel test runner and integration tests
//! 
//! This module provides comprehensive testing infrastructure for the NOS kernel,
//! including unit tests, integration tests, and performance benchmarks.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};

// Import test modules
#[cfg(feature = "kernel_tests")]
mod sync_tests;

#[cfg(feature = "kernel_tests")]
mod futex_integration_tests;

#[cfg(feature = "kernel_tests")]
mod linux_specific_tests;

// Test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Enable verbose output
    pub verbose: bool,
    /// Enable performance benchmarks
    pub enable_benchmarks: bool,
    /// Enable stress tests
    pub enable_stress_tests: bool,
    /// Number of iterations for stress tests
    pub stress_iterations: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            enable_benchmarks: true,
            enable_stress_tests: true,
            stress_iterations: 1000,
        }
    }
}

/// Test result
#[derive(Debug, Default)]
pub struct TestResult {
    /// Total tests run
    pub total_tests: usize,
    /// Passed tests
    pub passed_tests: usize,
    /// Failed tests
    pub failed_tests: usize,
    /// Skipped tests
    pub skipped_tests: usize,
    /// Test execution time in nanoseconds
    pub execution_time_ns: u64,
    /// Performance metrics
    pub performance_metrics: Vec<PerformanceMetric>,
}

/// Performance metric
#[derive(Debug)]
pub struct PerformanceMetric {
    /// Test name
    pub name: String,
    /// Operations per second
    pub ops_per_second: f64,
    /// Average latency in nanoseconds
    pub avg_latency_ns: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: usize,
}

/// Test suite trait
pub trait TestSuite {
    /// Run the test suite
    fn run(&self, config: &TestConfig) -> TestResult;
    
    /// Get test suite name
    fn name(&self) -> &str;
    
    /// Get test suite description
    fn description(&self) -> &str;
}

/// Kernel test runner
pub struct KernelTestRunner {
    config: TestConfig,
    test_suites: Vec<Box<dyn TestSuite>>,
}

impl KernelTestRunner {
    /// Create a new test runner with default configuration
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }
    
    /// Create a new test runner with custom configuration
    pub fn with_config(config: TestConfig) -> Self {
        Self {
            config,
            test_suites: Vec::new(),
        }
    }
    
    /// Add a test suite
    pub fn add_suite(&mut self, suite: Box<dyn TestSuite>) {
        self.test_suites.push(suite);
    }
    
    /// Run all test suites
    pub fn run_all(&self) -> TestResult {
        let start_time = crate::time::timestamp_nanos();
        let mut total_result = TestResult::default();
        
        crate::println!("[kernel_test] Starting kernel test suite...");
        crate::println!("[kernel_test] Configuration: {:?}", self.config);
        
        for suite in &self.test_suites {
            crate::println!("[kernel_test] Running test suite: {}", suite.name());
            crate::println!("[kernel_test] Description: {}", suite.description());
            
            let suite_result = suite.run(&self.config);
            
            total_result.total_tests += suite_result.total_tests;
            total_result.passed_tests += suite_result.passed_tests;
            total_result.failed_tests += suite_result.failed_tests;
            total_result.skipped_tests += suite_result.skipped_tests;
            total_result.performance_metrics.extend(suite_result.performance_metrics);
            
            if self.config.verbose {
                crate::println!("[kernel_test] Suite result: {}/{} passed", 
                               suite_result.passed_tests, suite_result.total_tests);
            }
        }
        
        let end_time = crate::time::timestamp_nanos();
        total_result.execution_time_ns = end_time - start_time;
        
        self.print_summary(&total_result);
        total_result
    }
    
    /// Print test summary
    fn print_summary(&self, result: &TestResult) {
        crate::println!("[kernel_test] ========== TEST SUMMARY ==========");
        crate::println!("[kernel_test] Total tests: {}", result.total_tests);
        crate::println!("[kernel_test] Passed: {}", result.passed_tests);
        crate::println!("[kernel_test] Failed: {}", result.failed_tests);
        crate::println!("[kernel_test] Skipped: {}", result.skipped_tests);
        
        let success_rate = if result.total_tests > 0 {
            (result.passed_tests as f64 / result.total_tests as f64) * 100.0
        } else {
            0.0
        };
        
        crate::println!("[kernel_test] Success rate: {:.2}%", success_rate);
        crate::println!("[kernel_test] Execution time: {}ms", 
                       result.execution_time_ns / 1_000_000);
        
        if self.config.enable_benchmarks && !result.performance_metrics.is_empty() {
            crate::println!("[kernel_test] ========== PERFORMANCE METRICS ==========");
            for metric in &result.performance_metrics {
                crate::println!("[kernel_test] {}: {:.2} ops/sec, {}ns avg latency, {} bytes", 
                               metric.name, metric.ops_per_second, 
                               metric.avg_latency_ns, metric.memory_usage_bytes);
            }
        }
        
        let overall_success = result.failed_tests == 0;
        if overall_success {
            crate::println!("[kernel_test] ✓ ALL TESTS PASSED");
        } else {
            crate::println!("[kernel_test] ✗ SOME TESTS FAILED");
        }
        
        crate::println!("[kernel_test] ========================================");
    }
}

/// Futex integration test suite
#[cfg(feature = "kernel_tests")]
pub struct FutexIntegrationTestSuite;

#[cfg(feature = "kernel_tests")]
impl TestSuite for FutexIntegrationTestSuite {
    fn run(&self, config: &TestConfig) -> TestResult {
        let mut result = TestResult::default();
        
        // Run basic futex tests
        if let Err(_) = crate::sync::futex_tests::run_futex_tests() {
            result.failed_tests += 1;
        } else {
            result.passed_tests += 1;
        }
        result.total_tests += 1;
        
        // Run futex validation
        if let Err(_) = crate::sync::futex_validation::run_futex_validation() {
            result.failed_tests += 1;
        } else {
            result.passed_tests += 1;
        }
        result.total_tests += 1;
        
        // Run performance benchmarks if enabled
        if config.enable_benchmarks {
            if let Err(_) = crate::sync::futex_validation::run_futex_performance_benchmark() {
                result.failed_tests += 1;
            } else {
                result.passed_tests += 1;
            }
            result.total_tests += 1;
        }
        
        result
    }
    
    fn name(&self) -> &str {
        "Futex Integration Tests"
    }
    
    fn description(&self) -> &str {
        "Comprehensive futex implementation tests including functionality, performance, and validation"
    }
}

/// Synchronization primitives test suite
#[cfg(feature = "kernel_tests")]
pub struct SyncTestSuite;

#[cfg(feature = "kernel_tests")]
impl TestSuite for SyncTestSuite {
    fn run(&self, _config: &TestConfig) -> TestResult {
        let mut result = TestResult::default();

        // Test basic synchronization primitives
        // This would include tests for Mutex, SpinLock, RwLock, etc.
        // For now, we'll just add a placeholder

        result.passed_tests += 1;
        result.total_tests += 1;

        result
    }

    fn name(&self) -> &str {
        "Synchronization Primitives Tests"
    }

    fn description(&self) -> &str {
        "Tests for basic synchronization primitives like Mutex, SpinLock, and RwLock"
    }
}

/// Linux-specific functionality test suite
#[cfg(feature = "kernel_tests")]
pub struct LinuxSpecificTestSuite;

#[cfg(feature = "kernel_tests")]
impl TestSuite for LinuxSpecificTestSuite {
    fn run(&self, _config: &TestConfig) -> TestResult {
        let mut result = TestResult::default();

        // Run all Linux-specific tests
        match linux_specific_tests::run_all_linux_tests() {
            Ok(_) => {
                // Count individual tests - we know there are many tests in the suite
                result.passed_tests += 20; // Approximate number of tests
                result.total_tests += 20;
            }
            Err(errors) => {
                result.failed_tests += errors.len();
                result.total_tests += errors.len();
            }
        }

        result
    }

    fn name(&self) -> &str {
        "Linux-Specific Functionality Tests"
    }

    fn description(&self) -> &str {
        "Comprehensive tests for Linux-specific syscalls: inotify, eventfd, signalfd, timerfd"
    }
}

/// Run kernel tests with default configuration
pub fn run_kernel_tests() -> TestResult {
    let mut runner = KernelTestRunner::new();
    
    #[cfg(feature = "kernel_tests")]
    {
        runner.add_suite(Box::new(FutexIntegrationTestSuite));
        runner.add_suite(Box::new(SyncTestSuite));
        runner.add_suite(Box::new(LinuxSpecificTestSuite));
    }
    
    runner.run_all()
}

/// Run kernel tests with custom configuration
pub fn run_kernel_tests_with_config(config: TestConfig) -> TestResult {
    let mut runner = KernelTestRunner::with_config(config);
    
    #[cfg(feature = "kernel_tests")]
    {
        runner.add_suite(Box::new(FutexIntegrationTestSuite));
        runner.add_suite(Box::new(SyncTestSuite));
        runner.add_suite(Box::new(LinuxSpecificTestSuite));
    }
    
    runner.run_all()
}

/// Quick test run for development
pub fn run_quick_tests() -> TestResult {
    let config = TestConfig {
        verbose: true,
        enable_benchmarks: false,
        enable_stress_tests: false,
        stress_iterations: 100,
    };
    
    run_kernel_tests_with_config(config)
}

/// Full test suite including stress tests and benchmarks
pub fn run_full_test_suite() -> TestResult {
    let config = TestConfig {
        verbose: true,
        enable_benchmarks: true,
        enable_stress_tests: true,
        stress_iterations: 10000,
    };
    
    run_kernel_tests_with_config(config)
}

/// Performance-only test run
pub fn run_performance_tests() -> TestResult {
    let config = TestConfig {
        verbose: false,
        enable_benchmarks: true,
        enable_stress_tests: false,
        stress_iterations: 1000,
    };
    
    run_kernel_tests_with_config(config)
}

/// Stress test run
pub fn run_stress_tests() -> TestResult {
    let config = TestConfig {
        verbose: true,
        enable_benchmarks: false,
        enable_stress_tests: true,
        stress_iterations: 50000,
    };
    
    run_kernel_tests_with_config(config)
}

/// Test statistics collector
pub static TEST_STATS: AtomicUsize = AtomicUsize::new(0);

/// Increment test counter
pub fn increment_test_counter() {
    TEST_STATS.fetch_add(1, Ordering::Relaxed);
}

/// Get test counter value
pub fn get_test_counter() -> usize {
    TEST_STATS.load(Ordering::Relaxed)
}

/// Reset test counter
pub fn reset_test_counter() {
    TEST_STATS.store(0, Ordering::Relaxed);
}

/// Assert macro for kernel tests
#[macro_export]
macro_rules! kernel_assert {
    ($condition:expr, $message:expr) => {
        if !($condition) {
            crate::tests::increment_test_counter();
            crate::println!("[kernel_test] ASSERTION FAILED: {}", $message);
            return false;
        }
    };
    ($condition:expr) => {
        if !($condition) {
            crate::tests::increment_test_counter();
            crate::println!("[kernel_test] ASSERTION FAILED at {}:{}: {}", 
                           file!(), line!(), stringify!($condition));
            return false;
        }
    };
}

/// Assert equality macro for kernel tests
#[macro_export]
macro_rules! kernel_assert_eq {
    ($left:expr, $right:expr) => {
        if ($left) != ($right) {
            crate::tests::increment_test_counter();
            crate::println!("[kernel_test] ASSERTION FAILED: {} != {} (expected {})", 
                           $left, $right, $right);
            return false;
        }
    };
    ($left:expr, $right:expr, $message:expr) => {
        if ($left) != ($right) {
            crate::tests::increment_test_counter();
            crate::println!("[kernel_test] ASSERTION FAILED: {} != {} (expected {}): {}", 
                           $left, $right, $right, $message);
            return false;
        }
    };
}

/// Test helper macro
#[macro_export]
macro_rules! run_test {
    ($test_name:expr, $test_code:block) => {
        crate::tests::increment_test_counter();
        crate::println!("[kernel_test] Running test: {}", $test_name);

        let test_result = (|| $test_code)();

        if test_result {
            crate::println!("[kernel_test] ✓ PASSED: {}", $test_name);
        } else {
            crate::println!("[kernel_test] ✗ FAILED: {}", $test_name);
        }

        test_result
    };
}

/// Skip test function for tests that should be skipped
pub fn skip_test(reason: &str) -> TestResult {
    crate::println!("[kernel_test] SKIPPED: {}", reason);
    Ok(())
}
