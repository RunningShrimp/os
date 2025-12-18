//! Test Runner Module
//! 
//! This module provides a comprehensive test runner that integrates all testing components
//! for the NOS kernel, including unit tests, integration tests, benchmarks,
//! security tests, stress tests, and automated test execution.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Test runner configuration
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    /// Enable unit tests
    pub enable_unit_tests: bool,
    /// Enable integration tests
    pub enable_integration_tests: bool,
    /// Enable benchmarks
    pub enable_benchmarks: bool,
    /// Enable security tests
    pub enable_security_tests: bool,
    /// Enable stress tests
    pub enable_stress_tests: bool,
    /// Enable test automation
    pub enable_test_automation: bool,
    /// Test categories to run
    pub test_categories: Vec<String>,
    /// Test tags to include
    pub include_tags: Vec<String>,
    /// Test tags to exclude
    pub exclude_tags: Vec<String>,
    /// Output directory for test reports
    pub output_directory: String,
    /// Enable detailed reporting
    pub enable_detailed_reporting: bool,
    /// Enable parallel execution
    pub enable_parallel_execution: bool,
    /// Maximum parallel test jobs
    pub max_parallel_jobs: usize,
    /// Test timeout in minutes
    pub test_timeout_minutes: u64,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            enable_unit_tests: true,
            enable_integration_tests: true,
            enable_benchmarks: true,
            enable_security_tests: true,
            enable_stress_tests: false,
            enable_test_automation: true,
            test_categories: Vec::new(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            output_directory: String::from("/tmp/test_runner"),
            enable_detailed_reporting: true,
            enable_parallel_execution: true,
            max_parallel_jobs: 4,
            test_timeout_minutes: 60,
        }
    }
}

/// Test runner result
#[derive(Debug, Clone)]
pub struct TestRunnerResult {
    /// Execution timestamp
    pub execution_timestamp: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Unit test results
    pub unit_test_results: Option<crate::testing::framework::TestFrameworkReport>,
    /// Integration test results
    pub integration_test_results: Option<crate::testing::integration_tests::IntegrationTestReport>,
    /// Benchmark results
    pub benchmark_results: Option<Vec<crate::testing::benchmarks::BenchmarkResult>>,
    /// Security test results
    pub security_test_results: Option<crate::testing::security_tests::SecurityReport>,
    /// Stress test results
    pub stress_test_results: Option<crate::testing::stress_tests::StressTestReport>,
    /// Test automation results
    pub test_automation_results: Option<crate::testing::test_automation::TestAutomationReport>,
    /// Overall success status
    pub overall_success: bool,
    /// Test summary
    pub test_summary: TestSummary,
}

/// Test summary
#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    /// Total tests run
    pub total_tests_run: u64,
    /// Total tests passed
    pub total_tests_passed: u64,
    /// Total tests failed
    pub total_tests_failed: u64,
    /// Total tests skipped
    pub total_tests_skipped: u64,
    /// Total benchmarks run
    pub total_benchmarks_run: u64,
    /// Total vulnerabilities found
    pub total_vulnerabilities_found: u64,
    /// Total operations performed
    pub total_operations_performed: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Success rate percentage
    pub success_rate_percent: f64,
}

/// Test runner
pub struct TestRunner {
    /// Configuration
    config: TestRunnerConfig,
    /// Global statistics
    global_stats: TestRunnerStats,
}

/// Test runner statistics
#[derive(Debug, Default)]
pub struct TestRunnerStats {
    /// Total test runs
    pub total_test_runs: AtomicU64,
    /// Total successful test runs
    pub total_successful_runs: AtomicU64,
    /// Total failed test runs
    pub total_failed_runs: AtomicU64,
    /// Total tests executed
    pub total_tests_executed: AtomicU64,
    /// Total tests passed
    pub total_tests_passed: AtomicU64,
    /// Total tests failed
    pub total_tests_failed: AtomicU64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: AtomicU64,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(config: TestRunnerConfig) -> Self {
        Self {
            config,
            global_stats: TestRunnerStats::default(),
        }
    }

    /// Run all tests according to configuration
    pub fn run_all_tests(&mut self) -> TestRunnerResult {
        let start_time = crate::time::get_ticks();
        let mut result = TestRunnerResult {
            execution_timestamp: start_time,
            total_execution_time_ms: 0,
            unit_test_results: None,
            integration_test_results: None,
            benchmark_results: None,
            security_test_results: None,
            stress_test_results: None,
            test_automation_results: None,
            overall_success: true,
            test_summary: TestSummary::default(),
        };

        // Update statistics
        self.global_stats.total_test_runs.fetch_add(1, Ordering::SeqCst);

        // Run unit tests
        if self.config.enable_unit_tests {
            crate::println!("Running unit tests...");
            if let Some(framework) = crate::testing::framework::get_test_framework() {
                let unit_test_results = framework.run_all_tests();
                result.unit_test_results = Some(unit_test_results);
            }
        }

        // Run integration tests
        if self.config.enable_integration_tests {
            crate::println!("Running integration tests...");
            if let Ok(integration_results) = crate::testing::integration_tests::run_all_integration_tests() {
                let integration_report = crate::testing::integration_tests::get_integration_test_system()
                    .unwrap()
                    .generate_integration_report(&integration_results);
                result.integration_test_results = Some(integration_report);
            }
        }

        // Run benchmarks
        if self.config.enable_benchmarks {
            crate::println!("Running benchmarks...");
            if let Ok(benchmark_results) = crate::testing::benchmarks::run_all_benchmarks() {
                result.benchmark_results = Some(benchmark_results);
            }
        }

        // Run security tests
        if self.config.enable_security_tests {
            crate::println!("Running security tests...");
            if let Ok(security_results) = crate::testing::security_tests::run_all_security_tests() {
                let security_report = crate::testing::security_tests::get_security_test_system()
                    .unwrap()
                    .generate_security_report(&security_results);
                result.security_test_results = Some(security_report);
            }
        }

        // Run stress tests
        if self.config.enable_stress_tests {
            crate::println!("Running stress tests...");
            if let Ok(stress_results) = crate::testing::stress_tests::run_all_stress_tests() {
                let stress_report = crate::testing::stress_tests::get_stress_test_system()
                    .unwrap()
                    .generate_stress_report(&stress_results);
                result.stress_test_results = Some(stress_report);
            }
        }

        // Process test automation
        if self.config.enable_test_automation {
            crate::println!("Processing test automation...");
            
            // Process scheduled jobs
            if let Ok(scheduled_jobs) = crate::testing::test_automation::process_scheduled_jobs() {
                if !scheduled_jobs.is_empty() {
                    crate::println!("Processed {} scheduled test jobs", scheduled_jobs.len());
                }
            }
            
            // Process job queue
            if let Ok(processed_jobs) = crate::testing::test_automation::process_job_queue() {
                if !processed_jobs.is_empty() {
                    crate::println!("Processed {} test jobs from queue", processed_jobs.len());
                }
            }
            
            // Generate automation report
            let automation_report = crate::testing::test_automation::generate_automation_report();
            result.test_automation_results = Some(automation_report);
        }

        let end_time = crate::time::get_ticks();
        result.total_execution_time_ms = end_time - start_time;

        // Calculate test summary
        self.calculate_test_summary(&mut result);

        // Update global statistics
        self.update_global_stats(&result);

        result
    }

    /// Calculate test summary from results
    fn calculate_test_summary(&self, result: &mut TestRunnerResult) {
        let mut summary = TestSummary::default();

        // Process unit test results
        if let Some(ref unit_results) = result.unit_test_results {
            summary.total_tests_run += unit_results.total_tests as u64;
            summary.total_tests_passed += unit_results.total_passed as u64;
            summary.total_tests_failed += unit_results.total_failed as u64;
            summary.total_tests_skipped += unit_results.total_skipped as u64;
        }

        // Process integration test results
        if let Some(ref integration_results) = result.integration_test_results {
            for test_result in &integration_results.test_results {
                summary.total_tests_run += 1;
                match test_result.status {
                    crate::testing::integration_tests::IntegrationTestStatus::Passed => {
                        summary.total_tests_passed += 1;
                    }
                    crate::testing::integration_tests::IntegrationTestStatus::Failed => {
                        summary.total_tests_failed += 1;
                    }
                    crate::testing::integration_tests::IntegrationTestStatus::Skipped => {
                        summary.total_tests_skipped += 1;
                    }
                    _ => {}
                }
            }
        }

        // Process benchmark results
        if let Some(ref benchmark_results) = result.benchmark_results {
            summary.total_benchmarks_run += benchmark_results.len() as u64;
        }

        // Process security test results
        if let Some(ref security_results) = result.security_test_results {
            summary.total_vulnerabilities_found += security_results.vulnerabilities.len() as u64;
            
            for test_result in &security_results.test_results {
                summary.total_tests_run += 1;
                match test_result.status {
                    crate::testing::security_tests::SecurityTestStatus::Passed => {
                        summary.total_tests_passed += 1;
                    }
                    crate::testing::security_tests::SecurityTestStatus::Failed => {
                        summary.total_tests_failed += 1;
                    }
                    crate::testing::security_tests::SecurityTestStatus::Skipped => {
                        summary.total_tests_skipped += 1;
                    }
                    _ => {}
                }
            }
        }

        // Process stress test results
        if let Some(ref stress_results) = result.stress_test_results {
            summary.total_operations_performed += stress_results.total_operations_performed;
            
            for test_result in &stress_results.test_results {
                summary.total_tests_run += 1;
                match test_result.status {
                    crate::testing::stress_tests::StressTestStatus::Passed => {
                        summary.total_tests_passed += 1;
                    }
                    crate::testing::stress_tests::StressTestStatus::Failed => {
                        summary.total_tests_failed += 1;
                    }
                    _ => {}
                }
            }
        }

        // Calculate derived metrics
        if summary.total_tests_run > 0 {
            summary.success_rate_percent = (summary.total_tests_passed as f64 / summary.total_tests_run as f64) * 100.0;
        }

        summary.avg_execution_time_ms = result.total_execution_time_ms as f64;

        // Determine overall success
        result.overall_success = summary.total_tests_failed == 0;

        result.test_summary = summary;
    }

    /// Update global statistics
    fn update_global_stats(&self, result: &TestRunnerResult) {
        self.global_stats.total_tests_executed.fetch_add(result.test_summary.total_tests_run, Ordering::SeqCst);
        self.global_stats.total_tests_passed.fetch_add(result.test_summary.total_tests_passed, Ordering::SeqCst);
        self.global_stats.total_tests_failed.fetch_add(result.test_summary.total_tests_failed, Ordering::SeqCst);
        self.global_stats.total_execution_time_ms.fetch_add(result.total_execution_time_ms, Ordering::SeqCst);

        if result.overall_success {
            self.global_stats.total_successful_runs.fetch_add(1, Ordering::SeqCst);
        } else {
            self.global_stats.total_failed_runs.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> &TestRunnerStats {
        &self.global_stats
    }

    /// Generate comprehensive test report
    pub fn generate_comprehensive_report(&self, result: &TestRunnerResult) -> ComprehensiveTestReport {
        ComprehensiveTestReport {
            runner_result: result.clone(),
            global_stats: TestRunnerStats {
                total_test_runs: self.global_stats.total_test_runs.load(Ordering::SeqCst),
                total_successful_runs: self.global_stats.total_successful_runs.load(Ordering::SeqCst),
                total_failed_runs: self.global_stats.total_failed_runs.load(Ordering::SeqCst),
                total_tests_executed: self.global_stats.total_tests_executed.load(Ordering::SeqCst),
                total_tests_passed: self.global_stats.total_tests_passed.load(Ordering::SeqCst),
                total_tests_failed: self.global_stats.total_tests_failed.load(Ordering::SeqCst),
                total_execution_time_ms: self.global_stats.total_execution_time_ms.load(Ordering::SeqCst),
            },
        }
    }
}

/// Comprehensive test report
#[derive(Debug)]
pub struct ComprehensiveTestReport {
    /// Test runner result
    pub runner_result: TestRunnerResult,
    /// Global statistics
    pub global_stats: TestRunnerStats,
}

impl ComprehensiveTestReport {
    /// Print detailed report
    pub fn print_detailed_report(&self) {
        crate::println!();
        crate::println!("==== Comprehensive Test Report ====");
        crate::println!("Execution timestamp: {}", self.runner_result.execution_timestamp);
        crate::println!("Total execution time: {}ms", self.runner_result.total_execution_time_ms);
        crate::println!("Overall success: {}", self.runner_result.overall_success);
        crate::println!();

        // Print test summary
        crate::println!("==== Test Summary ====");
        crate::println!("Total tests run: {}", self.runner_result.test_summary.total_tests_run);
        crate::println!("Total tests passed: {}", self.runner_result.test_summary.total_tests_passed);
        crate::println!("Total tests failed: {}", self.runner_result.test_summary.total_tests_failed);
        crate::println!("Total tests skipped: {}", self.runner_result.test_summary.total_tests_skipped);
        crate::println!("Total benchmarks run: {}", self.runner_result.test_summary.total_benchmarks_run);
        crate::println!("Total vulnerabilities found: {}", self.runner_result.test_summary.total_vulnerabilities_found);
        crate::println!("Total operations performed: {}", self.runner_result.test_summary.total_operations_performed);
        crate::println!("Average execution time: {:.2}ms", self.runner_result.test_summary.avg_execution_time_ms);
        crate::println!("Success rate: {:.1}%", self.runner_result.test_summary.success_rate_percent);
        crate::println!();

        // Print unit test results
        if let Some(ref unit_results) = self.runner_result.unit_test_results {
            crate::println!("==== Unit Test Results ====");
            crate::println!("Total tests: {}", unit_results.total_tests);
            crate::println!("Passed: {}", unit_results.total_passed);
            crate::println!("Failed: {}", unit_results.total_failed);
            crate::println!("Skipped: {}", unit_results.total_skipped);
            crate::println!("Success rate: {:.1}%", unit_results.success_rate);
            crate::println!();
        }

        // Print integration test results
        if let Some(ref integration_results) = self.runner_result.integration_test_results {
            crate::println!("==== Integration Test Results ====");
            crate::println!("Total tests: {}", integration_results.test_results.len());
            crate::println!("Components tested: {}", integration_results.components_tested.len());
            crate::println!("Overall success rate: {:.1}%", integration_results.overall_success_rate);
            crate::println!("Test assessment: {:?}", integration_results.test_assessment);
            crate::println!();
        }

        // Print benchmark results
        if let Some(ref benchmark_results) = self.runner_result.benchmark_results {
            crate::println!("==== Benchmark Results ====");
            crate::println!("Total benchmarks: {}", benchmark_results.len());
            
            let mut regressions = 0;
            let mut improvements = 0;
            
            for result in benchmark_results {
                if result.regression_detected {
                    regressions += 1;
                }
                if result.improvement_detected {
                    improvements += 1;
                }
            }
            
            crate::println!("Regressions detected: {}", regressions);
            crate::println!("Improvements detected: {}", improvements);
            crate::println!();
        }

        // Print security test results
        if let Some(ref security_results) = self.runner_result.security_test_results {
            crate::println!("==== Security Test Results ====");
            crate::println!("Total tests: {}", security_results.test_results.len());
            crate::println!("Vulnerabilities found: {}", security_results.vulnerabilities.len());
            crate::println!("Overall security score: {}/100", security_results.overall_security_score);
            crate::println!("Security assessment: {:?}", security_results.security_assessment);
            crate::println!();
        }

        // Print stress test results
        if let Some(ref stress_results) = self.runner_result.stress_test_results {
            crate::println!("==== Stress Test Results ====");
            crate::println!("Total tests: {}", stress_results.test_results.len());
            crate::println!("Total operations performed: {}", stress_results.total_operations_performed);
            crate::println!("Average operations per second: {:.2}", stress_results.average_ops_per_second);
            crate::println!("Overall success rate: {:.1}%", stress_results.overall_success_rate);
            crate::println!("Test assessment: {:?}", stress_results.test_assessment);
            crate::println!();
        }

        // Print test automation results
        if let Some(ref automation_results) = self.runner_result.test_automation_results {
            crate::println!("==== Test Automation Results ====");
            crate::println!("Current jobs: {}", automation_results.current_jobs.len());
            crate::println!("Job history: {}", automation_results.job_history.len());
            crate::println!("Schedules: {}", automation_results.schedules.len());
            crate::println!("Total jobs created: {}", automation_results.global_stats.total_jobs_created.load(core::sync::atomic::Ordering::SeqCst));
            crate::println!("Total jobs completed: {}", automation_results.global_stats.total_jobs_completed.load(core::sync::atomic::Ordering::SeqCst));
            crate::println!("Total jobs failed: {}", automation_results.global_stats.total_jobs_failed.load(core::sync::atomic::Ordering::SeqCst));
            crate::println!();
        }

        // Print global statistics
        crate::println!("==== Global Statistics ====");
        crate::println!("Total test runs: {}", self.global_stats.total_test_runs.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total successful runs: {}", self.global_stats.total_successful_runs.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total failed runs: {}", self.global_stats.total_failed_runs.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total tests executed: {}", self.global_stats.total_tests_executed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total tests passed: {}", self.global_stats.total_tests_passed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total tests failed: {}", self.global_stats.total_tests_failed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total execution time: {}ms", self.global_stats.total_execution_time_ms.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!();
    }
}

/// Global test runner instance
static mut TEST_RUNNER: Option<TestRunner> = None;
static TEST_RUNNER_INIT: spin::Once = spin::Once::new();

/// Initialize the global test runner
pub fn init_test_runner(config: TestRunnerConfig) -> Result<(), String> {
    TEST_RUNNER_INIT.call_once(|| {
        let runner = TestRunner::new(config);
        unsafe {
            TEST_RUNNER = Some(runner);
        }
    });
    Ok(())
}

/// Get the global test runner
pub fn get_test_runner() -> Option<&'static mut TestRunner> {
    unsafe {
        TEST_RUNNER.as_mut()
    }
}

/// Run all tests using the global test runner
pub fn run_all_tests() -> Result<TestRunnerResult, String> {
    let runner = get_test_runner().ok_or("Test runner not initialized")?;
    Ok(runner.run_all_tests())
}

/// Generate comprehensive test report
pub fn generate_comprehensive_report(result: &TestRunnerResult) -> ComprehensiveTestReport {
    let runner = get_test_runner().unwrap();
    runner.generate_comprehensive_report(result)
}