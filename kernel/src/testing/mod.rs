//! Testing Module
//! 
//! This module provides comprehensive testing capabilities for NOS kernel,
//! including unit tests, integration tests, performance benchmarks,
//! security testing, and test automation.

pub mod framework;
pub mod benchmarks;
pub mod security_tests;
pub mod integration_tests;
pub mod stress_tests;
pub mod test_automation;
pub mod test_runner;

// Re-export commonly used types
pub use framework::{
    TestFramework, TestFrameworkConfig, TestSuite, TestCase, TestResult,
    TestStatus, TestCategory, TestPriority, TestMetrics,
    TestFrameworkReport, TestSuiteReport, TestRegistry,
    init_test_framework, get_test_framework,
};

pub use benchmarks::{
    BenchmarkSuite, BenchmarkResult, BenchmarkConfig,
    run_benchmarks, register_benchmark,
};

pub use security_tests::{
    SecurityTestSuite, SecurityTestResult, SecurityTestConfig,
    run_security_tests, register_security_test,
};

pub use integration_tests::{
    IntegrationTestSuite, IntegrationTestResult, IntegrationTestConfig,
    run_integration_tests, register_integration_test,
};

pub use stress_tests::{
    StressTestSuite, StressTestResult, StressTestConfig,
    run_stress_tests, register_stress_test,
};

pub use test_automation::{
    TestAutomationSystem, TestAutomationConfig, TestJob, TestJobConfig, TestJobResult,
    TestJobStatus, TestJobPriority, TestSchedule, TestAutomationReport,
    init_test_automation_system, get_test_automation_system,
    create_test_job, create_test_job_with_config, schedule_test_job,
    run_test_job, cancel_test_job, process_scheduled_jobs, process_job_queue,
    generate_automation_report,
};

pub use test_runner::{
    TestRunner, TestRunnerConfig, TestRunnerResult, TestSummary, TestRunnerStats,
    ComprehensiveTestReport,
    init_test_runner, get_test_runner,
    run_all_tests as run_all_comprehensive_tests,
    generate_comprehensive_report,
};

/// Initialize all testing subsystems
pub fn init_testing_subsystems(config: TestFrameworkConfig) -> Result<(), &'static str> {
    // Initialize main test framework
    init_test_framework(config);
    
    // Initialize benchmark subsystem
    benchmarks::init_benchmark_system(benchmarks::BenchmarkConfig::default())?;
    
    // Initialize security test subsystem
    security_tests::init_security_test_system(security_tests::SecurityTestConfig::default())?;
    
    // Initialize integration test subsystem
    integration_tests::init_integration_test_system(integration_tests::IntegrationTestConfig::default())?;
    
    // Initialize stress test subsystem
    stress_tests::init_stress_test_system(stress_tests::StressTestConfig::default())?;
    
    // Initialize test automation subsystem
    test_automation::init_test_automation_system(test_automation::TestAutomationConfig::default())
        .map_err(|_| "Failed to initialize test automation system")?;
    
    // Initialize test runner
    test_runner::init_test_runner(test_runner::TestRunnerConfig::default())
        .map_err(|_| "Failed to initialize test runner")?;
    
    Ok(())
}

/// Run all tests across all subsystems
pub fn run_all_tests() -> Result<TestFrameworkReport, &'static str> {
    let framework = get_test_framework().ok_or("Test framework not initialized")?;
    
    // Run all framework tests
    let mut report = framework.run_all_tests();
    
    // Run benchmarks
    if let Ok(benchmark_results) = benchmarks::run_all_benchmarks() {
        // Add benchmark results to report
        // This would need to be implemented in report structure
    }
    
    // Run security tests
    if let Ok(security_results) = security_tests::run_all_security_tests() {
        // Add security test results to report
        // This would need to be implemented in report structure
    }
    
    // Run integration tests
    if let Ok(integration_results) = integration_tests::run_all_integration_tests() {
        // Add integration test results to report
        // This would need to be implemented in report structure
    }
    
    // Run stress tests
    if let Ok(stress_results) = stress_tests::run_all_stress_tests() {
        // Add stress test results to report
        // This would need to be implemented in report structure
    }
    
    // Process scheduled test automation jobs
    if let Ok(scheduled_jobs) = test_automation::process_scheduled_jobs() {
        if !scheduled_jobs.is_empty() {
            crate::println!("Processed {} scheduled test jobs", scheduled_jobs.len());
        }
    }
    
    // Process test automation job queue
    if let Ok(processed_jobs) = test_automation::process_job_queue() {
        if !processed_jobs.is_empty() {
            crate::println!("Processed {} test jobs from queue", processed_jobs.len());
        }
    }
    
    Ok(report)
}