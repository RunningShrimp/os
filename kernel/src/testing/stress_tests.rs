//! Stress Testing Module
//! 
//! This module provides comprehensive stress testing capabilities for the NOS kernel,
//! including load testing, resource exhaustion testing, and stability validation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Enable load testing
    pub enable_load_testing: bool,
    /// Enable resource exhaustion testing
    pub enable_resource_exhaustion_testing: bool,
    /// Enable stability testing
    pub enable_stability_testing: bool,
    /// Stress test duration in seconds
    pub test_duration_seconds: u64,
    /// Maximum concurrent operations
    pub max_concurrent_operations: usize,
    /// Output directory for stress reports
    pub output_directory: String,
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            enable_load_testing: true,
            enable_resource_exhaustion_testing: false,
            enable_stability_testing: true,
            test_duration_seconds: 300, // 5 minutes
            max_concurrent_operations: 1000,
            output_directory: String::from("/tmp/stress_tests"),
            enable_detailed_logging: true,
        }
    }
}

/// Stress test result
#[derive(Debug, Clone)]
pub struct StressTestResult {
    /// Test name
    pub name: String,
    /// Test category
    pub category: StressTestCategory,
    /// Test status
    pub status: StressTestStatus,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Operations performed
    pub operations_performed: u64,
    /// Operations per second
    pub operations_per_second: f64,
    /// Peak resource usage
    pub peak_resource_usage: ResourceUsage,
    /// Average resource usage
    pub average_resource_usage: ResourceUsage,
    /// Test metrics
    pub metrics: StressTestMetrics,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Stress test category
#[derive(Debug, Clone, PartialEq)]
pub enum StressTestCategory {
    /// Load testing
    Load,
    /// Resource exhaustion testing
    ResourceExhaustion,
    /// Stability testing
    Stability,
    /// Concurrency testing
    Concurrency,
    /// Memory pressure testing
    MemoryPressure,
    /// CPU stress testing
    CpuStress,
    /// I/O stress testing
    IoStress,
    /// Network stress testing
    NetworkStress,
}

/// Stress test status
#[derive(Debug, Clone, PartialEq)]
pub enum StressTestStatus {
    Passed,
    Failed,
    Timeout,
    Error,
    Interrupted,
}

/// Resource usage metrics
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Memory usage percentage
    pub memory_usage_percent: f64,
    /// Disk I/O operations per second
    pub disk_io_ops_per_sec: f64,
    /// Network I/O operations per second
    pub network_io_ops_per_sec: f64,
    /// Context switches per second
    pub context_switches_per_sec: f64,
    /// Custom metrics
    pub custom_metrics: BTreeMap<String, f64>,
}

/// Stress test metrics
#[derive(Debug, Clone, Default)]
pub struct StressTestMetrics {
    /// Maximum concurrent operations
    pub max_concurrent_operations: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// 95th percentile response time
    pub p95_response_time_ms: f64,
    /// 99th percentile response time
    pub p99_response_time_ms: f64,
    /// Error rate percentage
    pub error_rate_percent: f64,
    /// Throughput
    pub throughput: f64,
    /// Custom metrics
    pub custom_metrics: BTreeMap<String, u64>,
}

/// Stress test suite
#[derive(Debug)]
pub struct StressTestSuite {
    /// Suite name
    pub name: String,
    /// Stress test cases
    pub test_cases: Vec<StressTestCase>,
    /// Setup function
    pub setup_fn: Option<fn() -> Result<(), String>>,
    /// Teardown function
    pub teardown_fn: Option<fn() -> Result<(), String>>,
}

/// Stress test case
#[derive(Debug)]
pub struct StressTestCase {
    /// Test name
    pub name: String,
    /// Test function
    pub test_fn: fn() -> StressTestResult,
    /// Test category
    pub category: StressTestCategory,
    /// Expected duration in seconds
    pub expected_duration_seconds: u64,
    /// Target operations per second
    pub target_ops_per_sec: Option<f64>,
    /// Tags
    pub tags: Vec<String>,
}

/// Stress test system
pub struct StressTestSystem {
    /// Configuration
    config: StressTestConfig,
    /// Stress test suites
    test_suites: Mutex<Vec<StressTestSuite>>,
    /// Global statistics
    global_stats: StressTestStats,
    /// Resource monitor
    resource_monitor: Mutex<ResourceMonitor>,
}

/// Resource monitor for tracking resource usage during stress tests
#[derive(Debug)]
pub struct ResourceMonitor {
    /// Start time
    start_time: u64,
    /// Current resource usage
    current_usage: ResourceUsage,
    /// Peak resource usage
    peak_usage: ResourceUsage,
    /// Resource usage samples
    usage_samples: Vec<ResourceUsage>,
    /// Sample interval in milliseconds
    sample_interval_ms: u64,
}

/// Stress test statistics
#[derive(Debug, Default)]
pub struct StressTestStats {
    /// Total stress tests run
    pub total_tests_run: AtomicU64,
    /// Total tests passed
    pub total_tests_passed: AtomicU64,
    /// Total tests failed
    pub total_tests_failed: AtomicU64,
    /// Total operations performed
    pub total_operations_performed: AtomicU64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: AtomicU64,
    /// Peak CPU usage
    pub peak_cpu_usage: AtomicU64,
    /// Peak memory usage
    pub peak_memory_usage: AtomicU64,
}

impl StressTestSystem {
    /// Create a new stress test system
    pub fn new(config: StressTestConfig) -> Self {
        Self {
            config,
            test_suites: Mutex::new(Vec::new()),
            global_stats: StressTestStats::default(),
            resource_monitor: Mutex::new(ResourceMonitor::new()),
        }
    }

    /// Register a stress test suite
    pub fn register_suite(&self, suite: StressTestSuite) {
        let mut suites = self.test_suites.lock();
        suites.push(suite);
    }

    /// Run all stress tests
    pub fn run_all_stress_tests(&self) -> Result<Vec<StressTestResult>, String> {
        let suites = self.test_suites.lock();
        let mut all_results = Vec::new();

        for suite in suites.iter() {
            let suite_results = self.run_stress_test_suite(suite)?;
            all_results.extend(suite_results);
        }

        Ok(all_results)
    }

    /// Run a specific stress test suite
    pub fn run_stress_test_suite(&self, suite: &StressTestSuite) -> Result<Vec<StressTestResult>, String> {
        let mut results = Vec::new();

        // Run setup function
        if let Some(setup_fn) = suite.setup_fn {
            setup_fn().map_err(|e| format!("Setup failed: {}", e))?;
        }

        // Run all stress test cases
        for test_case in &suite.test_cases {
            let result = self.run_stress_test_case(test_case);
            results.push(result);
        }

        // Run teardown function
        if let Some(teardown_fn) = suite.teardown_fn {
            teardown_fn().map_err(|e| format!("Teardown failed: {}", e))?;
        }

        Ok(results)
    }

    /// Run a single stress test case
    pub fn run_stress_test_case(&self, test_case: &StressTestCase) -> StressTestResult {
        let start_time = crate::subsystems::time::get_ticks();
        
        // Start resource monitoring
        {
            let mut monitor = self.resource_monitor.lock();
            monitor.start_monitoring();
        }
        
        // Run stress test
        let mut result = (test_case.test_fn)();
        
        // Stop resource monitoring
        let (peak_usage, average_usage) = {
            let mut monitor = self.resource_monitor.lock();
            monitor.stop_monitoring()
        };
        
        let end_time = crate::subsystems::time::get_ticks();
        result.execution_time_ms = end_time - start_time;
        result.peak_resource_usage = peak_usage;
        result.average_resource_usage = average_usage;
        
        // Calculate operations per second
        if result.execution_time_ms > 0 {
            result.operations_per_second = result.operations_performed as f64 / (result.execution_time_ms as f64 / 1000.0);
        }
        
        // Update global statistics
        self.global_stats.total_tests_run.fetch_add(1, Ordering::SeqCst);
        self.global_stats.total_operations_performed.fetch_add(result.operations_performed, Ordering::SeqCst);
        self.global_stats.total_execution_time_ms.fetch_add(result.execution_time_ms, Ordering::SeqCst);
        
        // Update peak resource usage
        let cpu_usage = (result.peak_resource_usage.cpu_usage_percent * 100.0) as u64;
        let current_peak_cpu = self.global_stats.peak_cpu_usage.load(Ordering::SeqCst);
        if cpu_usage > current_peak_cpu {
            self.global_stats.peak_cpu_usage.store(cpu_usage, Ordering::SeqCst);
        }
        
        let memory_usage = result.peak_resource_usage.memory_usage_bytes as u64;
        let current_peak_memory = self.global_stats.peak_memory_usage.load(Ordering::SeqCst);
        if memory_usage > current_peak_memory {
            self.global_stats.peak_memory_usage.store(memory_usage, Ordering::SeqCst);
        }
        
        match result.status {
            StressTestStatus::Passed => {
                self.global_stats.total_tests_passed.fetch_add(1, Ordering::SeqCst);
            }
            _ => {
                self.global_stats.total_tests_failed.fetch_add(1, Ordering::SeqCst);
            }
        }
        
        result
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> &StressTestStats {
        &self.global_stats
    }

    /// Generate stress test report
    pub fn generate_stress_report(&self, results: &[StressTestResult]) -> StressTestReport {
        let mut report = StressTestReport::new();
        
        for result in results {
            report.add_test_result(result.clone());
        }
        
        report.finalize_report();
        report
    }
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new() -> Self {
        Self {
            start_time: 0,
            current_usage: ResourceUsage::default(),
            peak_usage: ResourceUsage::default(),
            usage_samples: Vec::new(),
            sample_interval_ms: 1000, // Sample every second
        }
    }

    /// Start monitoring
    pub fn start_monitoring(&mut self) {
        self.start_time = crate::subsystems::time::get_ticks();
        self.current_usage = self.get_current_usage();
        self.peak_usage = self.current_usage.clone();
        self.usage_samples.clear();
    }

    /// Stop monitoring
    pub fn stop_monitoring(&mut self) -> (ResourceUsage, ResourceUsage) {
        // Take final sample
        self.current_usage = self.get_current_usage();
        self.usage_samples.push(self.current_usage.clone());
        
        // Update peak usage
        self.update_peak_usage();
        
        // Calculate average usage
        let average_usage = self.calculate_average_usage();
        
        (self.peak_usage.clone(), average_usage)
    }

    /// Get current resource usage
    fn get_current_usage(&self) -> ResourceUsage {
        // In a real implementation, this would query system resources
        // For now, return placeholder values
        ResourceUsage {
            cpu_usage_percent: 50.0,
            memory_usage_bytes: 1024 * 1024 * 100, // 100MB
            memory_usage_percent: 25.0,
            disk_io_ops_per_sec: 100.0,
            network_io_ops_per_sec: 50.0,
            context_switches_per_sec: 200.0,
            custom_metrics: BTreeMap::new(),
        }
    }

    /// Update peak usage
    fn update_peak_usage(&mut self) {
        if self.current_usage.cpu_usage_percent > self.peak_usage.cpu_usage_percent {
            self.peak_usage.cpu_usage_percent = self.current_usage.cpu_usage_percent;
        }
        
        if self.current_usage.memory_usage_bytes > self.peak_usage.memory_usage_bytes {
            self.peak_usage.memory_usage_bytes = self.current_usage.memory_usage_bytes;
        }
        
        if self.current_usage.disk_io_ops_per_sec > self.peak_usage.disk_io_ops_per_sec {
            self.peak_usage.disk_io_ops_per_sec = self.current_usage.disk_io_ops_per_sec;
        }
        
        if self.current_usage.network_io_ops_per_sec > self.peak_usage.network_io_ops_per_sec {
            self.peak_usage.network_io_ops_per_sec = self.current_usage.network_io_ops_per_sec;
        }
        
        if self.current_usage.context_switches_per_sec > self.peak_usage.context_switches_per_sec {
            self.peak_usage.context_switches_per_sec = self.current_usage.context_switches_per_sec;
        }
    }

    /// Calculate average usage
    fn calculate_average_usage(&self) -> ResourceUsage {
        if self.usage_samples.is_empty() {
            return ResourceUsage::default();
        }

        let sample_count = self.usage_samples.len() as f64;
        let mut avg_usage = ResourceUsage::default();

        // Calculate averages
        for sample in &self.usage_samples {
            avg_usage.cpu_usage_percent += sample.cpu_usage_percent;
            avg_usage.memory_usage_bytes += sample.memory_usage_bytes;
            avg_usage.memory_usage_percent += sample.memory_usage_percent;
            avg_usage.disk_io_ops_per_sec += sample.disk_io_ops_per_sec;
            avg_usage.network_io_ops_per_sec += sample.network_io_ops_per_sec;
            avg_usage.context_switches_per_sec += sample.context_switches_per_sec;
        }

        avg_usage.cpu_usage_percent /= sample_count;
        avg_usage.memory_usage_bytes /= sample_count as usize;
        avg_usage.memory_usage_percent /= sample_count;
        avg_usage.disk_io_ops_per_sec /= sample_count;
        avg_usage.network_io_ops_per_sec /= sample_count;
        avg_usage.context_switches_per_sec /= sample_count;

        avg_usage
    }
}

/// Stress test report
#[derive(Debug)]
pub struct StressTestReport {
    /// Test results
    pub test_results: Vec<StressTestResult>,
    /// Overall success rate
    pub overall_success_rate: f64,
    /// Total operations performed
    pub total_operations_performed: u64,
    /// Average operations per second
    pub average_ops_per_second: f64,
    /// Peak resource usage
    pub peak_resource_usage: ResourceUsage,
    /// Test assessment
    pub test_assessment: StressTestAssessment,
    /// Recommendations
    pub recommendations: Vec<String>,
}

impl StressTestReport {
    /// Create a new stress test report
    pub fn new() -> Self {
        Self {
            test_results: Vec::new(),
            overall_success_rate: 0.0,
            total_operations_performed: 0,
            average_ops_per_second: 0.0,
            peak_resource_usage: ResourceUsage::default(),
            test_assessment: StressTestAssessment::Unknown,
            recommendations: Vec::new(),
        }
    }

    /// Add a test result
    pub fn add_test_result(&mut self, result: StressTestResult) {
        self.test_results.push(result);
    }

    /// Finalize the report
    pub fn finalize_report(&mut self) {
        if self.test_results.is_empty() {
            return;
        }

        // Calculate overall success rate
        let passed_count = self.test_results.iter()
            .filter(|r| r.status == StressTestStatus::Passed)
            .count();
        
        self.overall_success_rate = passed_count as f64 / self.test_results.len() as f64 * 100.0;

        // Calculate total operations
        self.total_operations_performed = self.test_results.iter()
            .map(|r| r.operations_performed)
            .sum();

        // Calculate average operations per second
        let total_time_ms: u64 = self.test_results.iter()
            .map(|r| r.execution_time_ms)
            .sum();
        
        if total_time_ms > 0 {
            self.average_ops_per_second = self.total_operations_performed as f64 / (total_time_ms as f64 / 1000.0);
        }

        // Find peak resource usage
        for result in &self.test_results {
            if result.peak_resource_usage.cpu_usage_percent > self.peak_resource_usage.cpu_usage_percent {
                self.peak_resource_usage.cpu_usage_percent = result.peak_resource_usage.cpu_usage_percent;
            }
            
            if result.peak_resource_usage.memory_usage_bytes > self.peak_resource_usage.memory_usage_bytes {
                self.peak_resource_usage.memory_usage_bytes = result.peak_resource_usage.memory_usage_bytes;
            }
            
            if result.peak_resource_usage.disk_io_ops_per_sec > self.peak_resource_usage.disk_io_ops_per_sec {
                self.peak_resource_usage.disk_io_ops_per_sec = result.peak_resource_usage.disk_io_ops_per_sec;
            }
            
            if result.peak_resource_usage.network_io_ops_per_sec > self.peak_resource_usage.network_io_ops_per_sec {
                self.peak_resource_usage.network_io_ops_per_sec = result.peak_resource_usage.network_io_ops_per_sec;
            }
            
            if result.peak_resource_usage.context_switches_per_sec > self.peak_resource_usage.context_switches_per_sec {
                self.peak_resource_usage.context_switches_per_sec = result.peak_resource_usage.context_switches_per_sec;
            }
        }

        // Determine test assessment
        self.test_assessment = if self.overall_success_rate >= 95.0 {
            StressTestAssessment::Excellent
        } else if self.overall_success_rate >= 85.0 {
            StressTestAssessment::Good
        } else if self.overall_success_rate >= 70.0 {
            StressTestAssessment::Fair
        } else if self.overall_success_rate >= 50.0 {
            StressTestAssessment::Poor
        } else {
            StressTestAssessment::Critical
        };

        // Generate recommendations
        self.generate_recommendations();
    }

    /// Generate recommendations
    fn generate_recommendations(&mut self) {
        // Count failures by category
        let mut failures_by_category = BTreeMap::new();
        for result in &self.test_results {
            if result.status != StressTestStatus::Passed {
                let count = failures_by_category.entry(result.category.clone()).or_insert(0);
                *count += 1;
            }
        }

        // Generate recommendations based on failure categories
        for (category, count) in failures_by_category {
            let recommendation = match category {
                StressTestCategory::Load => {
                    format!("Found {} load test failures. Review system capacity and scaling mechanisms.", count)
                }
                StressTestCategory::ResourceExhaustion => {
                    format!("Found {} resource exhaustion test failures. Review resource management and limits.", count)
                }
                StressTestCategory::Stability => {
                    format!("Found {} stability test failures. Review error handling and recovery mechanisms.", count)
                }
                StressTestCategory::Concurrency => {
                    format!("Found {} concurrency test failures. Review synchronization mechanisms and race conditions.", count)
                }
                StressTestCategory::MemoryPressure => {
                    format!("Found {} memory pressure test failures. Review memory management and allocation strategies.", count)
                }
                StressTestCategory::CpuStress => {
                    format!("Found {} CPU stress test failures. Review CPU-intensive operations and optimization.", count)
                }
                StressTestCategory::IoStress => {
                    format!("Found {} I/O stress test failures. Review I/O handling and buffering strategies.", count)
                }
                StressTestCategory::NetworkStress => {
                    format!("Found {} network stress test failures. Review network handling and congestion control.", count)
                }
            };
            self.recommendations.push(recommendation);
        }

        // Add resource usage recommendations
        if self.peak_resource_usage.cpu_usage_percent > 90.0 {
            self.recommendations.push("High CPU usage detected during stress tests. Consider CPU optimization or scaling.".to_string());
        }

        if self.peak_resource_usage.memory_usage_percent > 90.0 {
            self.recommendations.push("High memory usage detected during stress tests. Consider memory optimization or增加内存.".to_string());
        }
    }

    /// Print detailed report
    pub fn print_detailed_report(&self) {
        crate::println!();
        crate::println!("==== Stress Test Report ====");
        crate::println!("Overall success rate: {:.1}%", self.overall_success_rate);
        crate::println!("Test assessment: {:?}", self.test_assessment);
        crate::println!("Total tests run: {}", self.test_results.len());
        crate::println!("Total operations performed: {}", self.total_operations_performed);
        crate::println!("Average operations per second: {:.2}", self.average_ops_per_second);
        crate::println!();

        // Print test results by category
        let mut results_by_category = BTreeMap::new();
        for result in &self.test_results {
            let category_results = results_by_category.entry(result.category.clone()).or_insert_with(Vec::new);
            category_results.push(result);
        }

        for (category, results) in results_by_category {
            crate::println!("==== {:?} ====", category);
            for result in results {
                let status_str = match result.status {
                    StressTestStatus::Passed => "\x1b[32mPASS\x1b[0m",
                    StressTestStatus::Failed => "\x1b[31mFAIL\x1b[0m",
                    StressTestStatus::Timeout => "\x1b[35mTIME\x1b[0m",
                    StressTestStatus::Error => "\x1b[31mERROR\x1b[0m",
                    StressTestStatus::Interrupted => "\x1b[33mINTR\x1b[0m",
                };

                crate::println!("  {} {} ({} ops, {:.2} ops/sec, {}ms)",
                    status_str,
                    result.name,
                    result.operations_performed,
                    result.operations_per_second,
                    result.execution_time_ms
                );

                crate::println!("    Peak CPU: {:.1}%, Peak Memory: {}MB",
                    result.peak_resource_usage.cpu_usage_percent,
                    result.peak_resource_usage.memory_usage_bytes / (1024 * 1024)
                );

                if let Some(ref error) = result.error_message {
                    crate::println!("    Error: {}", error);
                }
            }
            crate::println!();
        }

        // Print peak resource usage
        crate::println!("==== Peak Resource Usage ====");
        crate::println!("CPU: {:.1}%", self.peak_resource_usage.cpu_usage_percent);
        crate::println!("Memory: {}MB ({:.1}%)",
            self.peak_resource_usage.memory_usage_bytes / (1024 * 1024),
            self.peak_resource_usage.memory_usage_percent
        );
        crate::println!("Disk I/O: {:.1} ops/sec", self.peak_resource_usage.disk_io_ops_per_sec);
        crate::println!("Network I/O: {:.1} ops/sec", self.peak_resource_usage.network_io_ops_per_sec);
        crate::println!("Context Switches: {:.1}/sec", self.peak_resource_usage.context_switches_per_sec);
        crate::println!();

        // Print recommendations
        if !self.recommendations.is_empty() {
            crate::println!("==== Recommendations ====");
            for (i, recommendation) in self.recommendations.iter().enumerate() {
                crate::println!("{}. {}", i + 1, recommendation);
            }
            crate::println!();
        }
    }
}

/// Stress test assessment
#[derive(Debug, Clone, PartialEq)]
pub enum StressTestAssessment {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
    Unknown,
}

/// Global stress test system instance
static mut STRESS_TEST_SYSTEM: Option<StressTestSystem> = None;
static STRESS_TEST_SYSTEM_INIT: spin::Once = spin::Once::new();

/// Initialize the global stress test system
pub fn init_stress_test_system(config: StressTestConfig) -> Result<(), String> {
    STRESS_TEST_SYSTEM_INIT.call_once(|| {
        let system = StressTestSystem::new(config);
        unsafe {
            STRESS_TEST_SYSTEM = Some(system);
        }
    });
    Ok(())
}

/// Get the global stress test system
pub fn get_stress_test_system() -> Option<&'static StressTestSystem> {
    unsafe {
        STRESS_TEST_SYSTEM.as_ref()
    }
}

/// Register a stress test suite
pub fn register_stress_test_suite(suite: StressTestSuite) {
    if let Some(system) = get_stress_test_system() {
        system.register_suite(suite);
    }
}

/// Run all stress tests
pub fn run_all_stress_tests() -> Result<Vec<StressTestResult>, String> {
    let system = get_stress_test_system().ok_or("Stress test system not initialized")?;
    system.run_all_stress_tests()
}

/// Macro to create a stress test case
#[macro_export]
macro_rules! stress_test_case {
    ($name:expr, $fn:expr, $category:expr, $duration:expr) => {
        $crate::testing::stress_tests::StressTestCase {
            name: $name.to_string(),
            test_fn: $fn,
            category: $category,
            expected_duration_seconds: $duration,
            target_ops_per_sec: None,
            tags: Vec::new(),
        }
    };
    ($name:expr, $fn:expr, $category:expr, $duration:expr, target_ops => $ops:expr) => {
        $crate::testing::stress_tests::StressTestCase {
            name: $name.to_string(),
            test_fn: $fn,
            category: $category,
            expected_duration_seconds: $duration,
            target_ops_per_sec: Some($ops),
            tags: Vec::new(),
        }
    };
    ($name:expr, $fn:expr, $category:expr, $duration:expr, target_ops => $ops:expr, tags => [$($tag:expr),*]) => {
        $crate::testing::stress_tests::StressTestCase {
            name: $name.to_string(),
            test_fn: $fn,
            category: $category,
            expected_duration_seconds: $duration,
            target_ops_per_sec: Some($ops),
            tags: vec![$($tag.to_string()),*],
        }
    };
}

/// Macro to create a stress test suite
#[macro_export]
macro_rules! stress_test_suite {
    ($name:expr, [$($stress_test_case:expr),*]) => {
        $crate::testing::stress_tests::StressTestSuite {
            name: $name.to_string(),
            test_cases: vec![$($stress_test_case),*],
            setup_fn: None,
            teardown_fn: None,
        }
    };
    ($name:expr, [$($stress_test_case:expr),*], setup => $setup:expr) => {
        $crate::testing::stress_tests::StressTestSuite {
            name: $name.to_string(),
            test_cases: vec![$($stress_test_case),*],
            setup_fn: Some($setup),
            teardown_fn: None,
        }
    };
    ($name:expr, [$($stress_test_case:expr),*], setup => $setup:expr, teardown => $teardown:expr) => {
        $crate::testing::stress_tests::StressTestSuite {
            name: $name.to_string(),
            test_cases: vec![$($stress_test_case),*],
            setup_fn: Some($setup),
            teardown_fn: Some($teardown),
        }
    };
}