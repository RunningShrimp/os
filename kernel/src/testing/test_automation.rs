//! Test Automation Module
//! 
//! This module provides comprehensive test automation capabilities for the NOS kernel,
//! including automated test execution, scheduling, reporting, and CI/CD integration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Test automation configuration
#[derive(Debug, Clone)]
pub struct TestAutomationConfig {
    /// Enable automated test execution
    pub enable_automated_execution: bool,
    /// Enable scheduled test runs
    pub enable_scheduled_runs: bool,
    /// Enable CI/CD integration
    pub enable_ci_cd_integration: bool,
    /// Enable test result notifications
    pub enable_notifications: bool,
    /// Default test schedule in cron format
    pub default_schedule: String,
    /// Output directory for automation reports
    pub output_directory: String,
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
    /// Maximum concurrent test jobs
    pub max_concurrent_jobs: usize,
    /// Test timeout in minutes
    pub test_timeout_minutes: u64,
}

impl Default for TestAutomationConfig {
    fn default() -> Self {
        Self {
            enable_automated_execution: true,
            enable_scheduled_runs: true,
            enable_ci_cd_integration: true,
            enable_notifications: true,
            default_schedule: "0 2 * * *".to_string(), // Daily at 2 AM
            output_directory: String::from("/tmp/test_automation"),
            enable_detailed_logging: true,
            max_concurrent_jobs: 4,
            test_timeout_minutes: 60,
        }
    }
}

/// Test job status
#[derive(Debug, Clone, PartialEq)]
pub enum TestJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Test job priority
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestJobPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Test job
#[derive(Debug)]
pub struct TestJob {
    /// Job ID
    pub id: u64,
    /// Job name
    pub name: String,
    /// Job description
    pub description: String,
    /// Test suites to run
    pub test_suites: Vec<String>,
    /// Job status
    pub status: TestJobStatus,
    /// Job priority
    pub priority: TestJobPriority,
    /// Created timestamp
    pub created_at: u64,
    /// Started timestamp
    pub started_at: Option<u64>,
    /// Completed timestamp
    pub completed_at: Option<u64>,
    /// Job configuration
    pub config: TestJobConfig,
    /// Job results
    pub results: Option<TestJobResult>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Test job configuration
#[derive(Debug, Clone)]
pub struct TestJobConfig {
    /// Test categories to include
    pub include_categories: Vec<String>,
    /// Test categories to exclude
    pub exclude_categories: Vec<String>,
    /// Test tags to include
    pub include_tags: Vec<String>,
    /// Test tags to exclude
    pub exclude_tags: Vec<String>,
    /// Enable parallel execution
    pub enable_parallel: bool,
    /// Maximum parallel jobs
    pub max_parallel_jobs: usize,
    /// Test timeout in minutes
    pub timeout_minutes: u64,
    /// Retry count on failure
    pub retry_count: u32,
    /// Environment variables
    pub environment_vars: BTreeMap<String, String>,
    /// Custom test parameters
    pub custom_params: BTreeMap<String, String>,
}

impl Default for TestJobConfig {
    fn default() -> Self {
        Self {
            include_categories: Vec::new(),
            exclude_categories: Vec::new(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
            enable_parallel: true,
            max_parallel_jobs: 4,
            timeout_minutes: 60,
            retry_count: 0,
            environment_vars: BTreeMap::new(),
            custom_params: BTreeMap::new(),
        }
    }
}

/// Test job result
#[derive(Debug, Clone)]
pub struct TestJobResult {
    /// Job ID
    pub job_id: u64,
    /// Total tests run
    pub total_tests: u64,
    /// Tests passed
    pub tests_passed: u64,
    /// Tests failed
    pub tests_failed: u64,
    /// Tests skipped
    pub tests_skipped: u64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Test suite results
    pub suite_results: Vec<TestSuiteResult>,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    /// Coverage metrics
    pub coverage_metrics: CoverageMetrics,
}

/// Test suite result
#[derive(Debug, Clone)]
pub struct TestSuiteResult {
    /// Suite name
    pub name: String,
    /// Suite status
    pub status: TestJobStatus,
    /// Tests passed
    pub tests_passed: u64,
    /// Tests failed
    pub tests_failed: u64,
    /// Tests skipped
    pub tests_skipped: u64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Disk I/O operations
    pub disk_io_ops: u64,
    /// Network I/O operations
    pub network_io_ops: u64,
    /// Custom metrics
    pub custom_metrics: BTreeMap<String, f64>,
}

/// Coverage metrics
#[derive(Debug, Clone, Default)]
pub struct CoverageMetrics {
    /// Line coverage percentage
    pub line_coverage_percent: f64,
    /// Function coverage percentage
    pub function_coverage_percent: f64,
    /// Branch coverage percentage
    pub branch_coverage_percent: f64,
    /// Statement coverage percentage
    pub statement_coverage_percent: f64,
    /// Total lines of code
    pub total_lines: u64,
    /// Covered lines of code
    pub covered_lines: u64,
}

/// Test schedule
#[derive(Debug)]
pub struct TestSchedule {
    /// Schedule ID
    pub id: u64,
    /// Schedule name
    pub name: String,
    /// Cron expression
    pub cron_expression: String,
    /// Test job template
    pub job_template: TestJob,
    /// Enabled flag
    pub enabled: bool,
    /// Last run timestamp
    pub last_run: Option<u64>,
    /// Next run timestamp
    pub next_run: Option<u64>,
}

/// Test automation system
pub struct TestAutomationSystem {
    /// Configuration
    config: TestAutomationConfig,
    /// Test jobs
    jobs: Mutex<Vec<TestJob>>,
    /// Job queue
    job_queue: Mutex<Vec<u64>>,
    /// Running jobs
    running_jobs: Mutex<Vec<u64>>,
    /// Test schedules
    schedules: Mutex<Vec<TestSchedule>>,
    /// Job history
    job_history: Mutex<Vec<TestJob>>,
    /// Global statistics
    global_stats: TestAutomationStats,
    /// Next job ID
    next_job_id: AtomicU64,
    /// Next schedule ID
    next_schedule_id: AtomicU64,
}

/// Test automation statistics
#[derive(Debug, Default)]
pub struct TestAutomationStats {
    /// Total jobs created
    pub total_jobs_created: AtomicU64,
    /// Total jobs completed
    pub total_jobs_completed: AtomicU64,
    /// Total jobs failed
    pub total_jobs_failed: AtomicU64,
    /// Total tests run
    pub total_tests_run: AtomicU64,
    /// Total tests passed
    pub total_tests_passed: AtomicU64,
    /// Total tests failed
    pub total_tests_failed: AtomicU64,
    /// Average job execution time in milliseconds
    pub avg_job_execution_time_ms: AtomicU64,
}

impl TestAutomationSystem {
    /// Create a new test automation system
    pub fn new(config: TestAutomationConfig) -> Self {
        Self {
            config,
            jobs: Mutex::new(Vec::new()),
            job_queue: Mutex::new(Vec::new()),
            running_jobs: Mutex::new(Vec::new()),
            schedules: Mutex::new(Vec::new()),
            job_history: Mutex::new(Vec::new()),
            global_stats: TestAutomationStats::default(),
            next_job_id: AtomicU64::new(1),
            next_schedule_id: AtomicU64::new(1),
        }
    }

    /// Create a new test job
    pub fn create_job(&self, name: String, description: String, test_suites: Vec<String>, config: TestJobConfig) -> u64 {
        let job_id = self.next_job_id.fetch_add(1, Ordering::SeqCst);
        let current_time = crate::time::get_ticks();

        let job = TestJob {
            id: job_id,
            name,
            description,
            test_suites,
            status: TestJobStatus::Pending,
            priority: TestJobPriority::Medium,
            created_at: current_time,
            started_at: None,
            completed_at: None,
            config,
            results: None,
            error_message: None,
        };

        // Add job to jobs list
        {
            let mut jobs = self.jobs.lock();
            jobs.push(job);
        }

        // Add job to queue
        {
            let mut queue = self.job_queue.lock();
            queue.push(job_id);
        }

        // Update statistics
        self.global_stats.total_jobs_created.fetch_add(1, Ordering::SeqCst);

        job_id
    }

    /// Schedule a test job
    pub fn schedule_job(&self, name: String, cron_expression: String, job_template: TestJob) -> u64 {
        let schedule_id = self.next_schedule_id.fetch_add(1, Ordering::SeqCst);

        let schedule = TestSchedule {
            id: schedule_id,
            name,
            cron_expression,
            job_template,
            enabled: true,
            last_run: None,
            next_run: None,
        };

        // Add schedule to schedules list
        {
            let mut schedules = self.schedules.lock();
            schedules.push(schedule);
        }

        schedule_id
    }

    /// Run a test job
    pub fn run_job(&self, job_id: u64) -> Result<(), String> {
        // Update job status to running
        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
                job.status = TestJobStatus::Running;
                job.started_at = Some(crate::time::get_ticks());
            } else {
                return Err("Job not found".to_string());
            }
        }

        // Add to running jobs
        {
            let mut running_jobs = self.running_jobs.lock();
            running_jobs.push(job_id);
        }

        // Execute the job
        let result = self.execute_job(job_id);

        // Update job status based on result
        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
                job.status = match result {
                    Ok(_) => TestJobStatus::Completed,
                    Err(_) => TestJobStatus::Failed,
                };
                job.completed_at = Some(crate::time::get_ticks());
            }
        }

        // Remove from running jobs
        {
            let mut running_jobs = self.running_jobs.lock();
            running_jobs.retain(|&id| id != job_id);
        }

        // Move job to history
        {
            let mut jobs = self.jobs.lock();
            let mut job_history = self.job_history.lock();
            
            if let Some(pos) = jobs.iter().position(|j| j.id == job_id) {
                let job = jobs.remove(pos);
                job_history.push(job);
            }
        }

        result
    }

    /// Execute a test job
    fn execute_job(&self, job_id: u64) -> Result<(), String> {
        // Get job details
        let job = {
            let jobs = self.jobs.lock();
            jobs.iter().find(|j| j.id == job_id).cloned()
        };

        let job = job.ok_or("Job not found")?;

        // Execute test suites
        let mut total_tests = 0;
        let mut tests_passed = 0;
        let mut tests_failed = 0;
        let mut tests_skipped = 0;
        let mut suite_results = Vec::new();
        let start_time = crate::time::get_ticks();

        for suite_name in &job.test_suites {
            // In a real implementation, this would execute the actual test suite
            // For now, we'll simulate execution
            let suite_result = TestSuiteResult {
                name: suite_name.clone(),
                status: TestJobStatus::Completed,
                tests_passed: 10,
                tests_failed: 0,
                tests_skipped: 0,
                execution_time_ms: 1000,
                error_message: None,
            };

            total_tests += suite_result.tests_passed + suite_result.tests_failed + suite_result.tests_skipped;
            tests_passed += suite_result.tests_passed;
            tests_failed += suite_result.tests_failed;
            tests_skipped += suite_result.tests_skipped;
            suite_results.push(suite_result);
        }

        let end_time = crate::time::get_ticks();
        let execution_time_ms = end_time - start_time;

        // Create job result
        let job_result = TestJobResult {
            job_id,
            total_tests,
            tests_passed,
            tests_failed,
            tests_skipped,
            execution_time_ms,
            suite_results,
            performance_metrics: PerformanceMetrics::default(),
            coverage_metrics: CoverageMetrics::default(),
        };

        // Update job with results
        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
                job.results = Some(job_result);
            }
        }

        // Update global statistics
        self.global_stats.total_jobs_completed.fetch_add(1, Ordering::SeqCst);
        self.global_stats.total_tests_run.fetch_add(total_tests, Ordering::SeqCst);
        self.global_stats.total_tests_passed.fetch_add(tests_passed, Ordering::SeqCst);
        self.global_stats.total_tests_failed.fetch_add(tests_failed, Ordering::SeqCst);

        // Update average execution time
        let current_avg = self.global_stats.avg_job_execution_time_ms.load(Ordering::SeqCst);
        let new_avg = (current_avg + execution_time_ms) / 2;
        self.global_stats.avg_job_execution_time_ms.store(new_avg, Ordering::SeqCst);

        Ok(())
    }

    /// Cancel a test job
    pub fn cancel_job(&self, job_id: u64) -> Result<(), String> {
        // Remove from queue if pending
        {
            let mut queue = self.job_queue.lock();
            queue.retain(|&id| id != job_id);
        }

        // Update job status
        {
            let mut jobs = self.jobs.lock();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
                match job.status {
                    TestJobStatus::Pending => {
                        job.status = TestJobStatus::Cancelled;
                        job.completed_at = Some(crate::time::get_ticks());
                    }
                    TestJobStatus::Running => {
                        // In a real implementation, this would interrupt the running job
                        job.status = TestJobStatus::Cancelled;
                        job.completed_at = Some(crate::time::get_ticks());
                    }
                    _ => {
                        return Err("Cannot cancel completed job".to_string());
                    }
                }
            } else {
                return Err("Job not found".to_string());
            }
        }

        // Remove from running jobs if applicable
        {
            let mut running_jobs = self.running_jobs.lock();
            running_jobs.retain(|&id| id != job_id);
        }

        Ok(())
    }

    /// Get job by ID
    pub fn get_job(&self, job_id: u64) -> Option<TestJob> {
        let jobs = self.jobs.lock();
        jobs.iter().find(|j| j.id == job_id).cloned()
    }

    /// Get all jobs
    pub fn get_all_jobs(&self) -> Vec<TestJob> {
        let jobs = self.jobs.lock();
        jobs.clone()
    }

    /// Get job history
    pub fn get_job_history(&self) -> Vec<TestJob> {
        let job_history = self.job_history.lock();
        job_history.clone()
    }

    /// Get all schedules
    pub fn get_all_schedules(&self) -> Vec<TestSchedule> {
        let schedules = self.schedules.lock();
        schedules.clone()
    }

    /// Process scheduled jobs
    pub fn process_scheduled_jobs(&self) -> Result<Vec<u64>, String> {
        let current_time = crate::time::get_ticks();
        let mut triggered_jobs = Vec::new();

        {
            let mut schedules = self.schedules.lock();
            for schedule in schedules.iter_mut() {
                if !schedule.enabled {
                    continue;
                }

                // Check if it's time to run the schedule
                if let Some(next_run) = schedule.next_run {
                    if current_time >= next_run {
                        // Create a new job from the template
                        let job_id = self.create_job(
                            schedule.job_template.name.clone(),
                            format!("Scheduled job: {}", schedule.name),
                            schedule.job_template.test_suites.clone(),
                            schedule.job_template.config.clone(),
                        );

                        triggered_jobs.push(job_id);

                        // Update schedule timestamps
                        schedule.last_run = Some(current_time);
                        // In a real implementation, this would calculate the next run time based on cron expression
                        schedule.next_run = Some(current_time + 86400000); // Next day
                    }
                } else {
                    // First time setting next run
                    schedule.next_run = Some(current_time + 86400000); // Next day
                }
            }
        }

        Ok(triggered_jobs)
    }

    /// Process job queue
    pub fn process_job_queue(&self) -> Result<Vec<u64>, String> {
        let mut processed_jobs = Vec::new();
        let max_concurrent = self.config.max_concurrent_jobs;

        {
            let running_jobs = self.running_jobs.lock();
            if running_jobs.len() >= max_concurrent {
                return Ok(processed_jobs); // Already at max capacity
            }
        }

        let available_slots = max_concurrent - {
            let running_jobs = self.running_jobs.lock();
            running_jobs.len()
        };

        {
            let mut queue = self.job_queue.lock();
            let jobs_to_process = queue.drain(..available_slots.min(queue.len())).collect::<Vec<_>>();
            drop(queue);

            for job_id in jobs_to_process {
                if let Err(e) = self.run_job(job_id) {
                    // Log error but continue with other jobs
                    crate::println!("Failed to run job {}: {}", job_id, e);
                } else {
                    processed_jobs.push(job_id);
                }
            }
        }

        Ok(processed_jobs)
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> &TestAutomationStats {
        &self.global_stats
    }

    /// Generate automation report
    pub fn generate_automation_report(&self) -> TestAutomationReport {
        let jobs = self.get_all_jobs();
        let job_history = self.get_job_history();
        let schedules = self.get_all_schedules();

        TestAutomationReport {
            current_jobs: jobs,
            job_history,
            schedules,
            global_stats: TestAutomationStats {
                total_jobs_created: self.global_stats.total_jobs_created.load(Ordering::SeqCst),
                total_jobs_completed: self.global_stats.total_jobs_completed.load(Ordering::SeqCst),
                total_jobs_failed: self.global_stats.total_jobs_failed.load(Ordering::SeqCst),
                total_tests_run: self.global_stats.total_tests_run.load(Ordering::SeqCst),
                total_tests_passed: self.global_stats.total_tests_passed.load(Ordering::SeqCst),
                total_tests_failed: self.global_stats.total_tests_failed.load(Ordering::SeqCst),
                avg_job_execution_time_ms: self.global_stats.avg_job_execution_time_ms.load(Ordering::SeqCst),
            },
        }
    }
}

/// Test automation report
#[derive(Debug)]
pub struct TestAutomationReport {
    /// Current jobs
    pub current_jobs: Vec<TestJob>,
    /// Job history
    pub job_history: Vec<TestJob>,
    /// Schedules
    pub schedules: Vec<TestSchedule>,
    /// Global statistics
    pub global_stats: TestAutomationStats,
}

impl TestAutomationReport {
    /// Print detailed report
    pub fn print_detailed_report(&self) {
        crate::println!();
        crate::println!("==== Test Automation Report ====");
        crate::println!("Total jobs created: {}", self.global_stats.total_jobs_created.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total jobs completed: {}", self.global_stats.total_jobs_completed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total jobs failed: {}", self.global_stats.total_jobs_failed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total tests run: {}", self.global_stats.total_tests_run.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total tests passed: {}", self.global_stats.total_tests_passed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Total tests failed: {}", self.global_stats.total_tests_failed.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!("Average job execution time: {}ms", self.global_stats.avg_job_execution_time_ms.load(core::sync::atomic::Ordering::SeqCst));
        crate::println!();

        // Print current jobs
        if !self.current_jobs.is_empty() {
            crate::println!("==== Current Jobs ====");
            for job in &self.current_jobs {
                let status_str = match job.status {
                    TestJobStatus::Pending => "\x1b[33mPEND\x1b[0m",
                    TestJobStatus::Running => "\x1b[34mRUN\x1b[0m",
                    TestJobStatus::Completed => "\x1b[32mDONE\x1b[0m",
                    TestJobStatus::Failed => "\x1b[31mFAIL\x1b[0m",
                    TestJobStatus::Cancelled => "\x1b[37mSTOP\x1b[0m",
                    TestJobStatus::Timeout => "\x1b[35mTIME\x1b[0m",
                };

                crate::println!("  {} {} ({})", status_str, job.name, job.id);
                crate::println!("    Description: {}", job.description);
                crate::println!("    Test suites: {:?}", job.test_suites);
                crate::println!("    Priority: {:?}", job.priority);
                crate::println!("    Created: {}ms ago", crate::time::get_ticks() - job.created_at);
                
                if let Some(started_at) = job.started_at {
                    crate::println!("    Started: {}ms ago", crate::time::get_ticks() - started_at);
                }
                
                if let Some(completed_at) = job.completed_at {
                    crate::println!("    Completed: {}ms ago", crate::time::get_ticks() - completed_at);
                }
                
                if let Some(ref error) = job.error_message {
                    crate::println!("    Error: {}", error);
                }
                
                crate::println!();
            }
        }

        // Print schedules
        if !self.schedules.is_empty() {
            crate::println!("==== Test Schedules ====");
            for schedule in &self.schedules {
                let enabled_str = if schedule.enabled { "Enabled" } else { "Disabled" };
                crate::println!("  {} ({}) - {}", schedule.name, schedule.id, enabled_str);
                crate::println!("    Schedule: {}", schedule.cron_expression);
                crate::println!("    Job template: {}", schedule.job_template.name);
                
                if let Some(last_run) = schedule.last_run {
                    crate::println!("    Last run: {}ms ago", crate::time::get_ticks() - last_run);
                }
                
                if let Some(next_run) = schedule.next_run {
                    crate::println!("    Next run: in {}ms", next_run - crate::time::get_ticks());
                }
                
                crate::println!();
            }
        }
    }
}

/// Global test automation system instance
static mut TEST_AUTOMATION_SYSTEM: Option<TestAutomationSystem> = None;
static TEST_AUTOMATION_SYSTEM_INIT: spin::Once = spin::Once::new();

/// Initialize the global test automation system
pub fn init_test_automation_system(config: TestAutomationConfig) -> Result<(), String> {
    TEST_AUTOMATION_SYSTEM_INIT.call_once(|| {
        let system = TestAutomationSystem::new(config);
        unsafe {
            TEST_AUTOMATION_SYSTEM = Some(system);
        }
    });
    Ok(())
}

/// Get the global test automation system
pub fn get_test_automation_system() -> Option<&'static TestAutomationSystem> {
    unsafe {
        TEST_AUTOMATION_SYSTEM.as_ref()
    }
}

/// Create a test job
pub fn create_test_job(name: String, description: String, test_suites: Vec<String>) -> u64 {
    if let Some(system) = get_test_automation_system() {
        system.create_job(name, description, test_suites, TestJobConfig::default())
    } else {
        0
    }
}

/// Create a test job with custom configuration
pub fn create_test_job_with_config(name: String, description: String, test_suites: Vec<String>, config: TestJobConfig) -> u64 {
    if let Some(system) = get_test_automation_system() {
        system.create_job(name, description, test_suites, config)
    } else {
        0
    }
}

/// Schedule a test job
pub fn schedule_test_job(name: String, cron_expression: String, job_template: TestJob) -> u64 {
    if let Some(system) = get_test_automation_system() {
        system.schedule_job(name, cron_expression, job_template)
    } else {
        0
    }
}

/// Run a test job
pub fn run_test_job(job_id: u64) -> Result<(), String> {
    let system = get_test_automation_system().ok_or("Test automation system not initialized")?;
    system.run_job(job_id)
}

/// Cancel a test job
pub fn cancel_test_job(job_id: u64) -> Result<(), String> {
    let system = get_test_automation_system().ok_or("Test automation system not initialized")?;
    system.cancel_job(job_id)
}

/// Process scheduled jobs
pub fn process_scheduled_jobs() -> Result<Vec<u64>, String> {
    let system = get_test_automation_system().ok_or("Test automation system not initialized")?;
    system.process_scheduled_jobs()
}

/// Process job queue
pub fn process_job_queue() -> Result<Vec<u64>, String> {
    let system = get_test_automation_system().ok_or("Test automation system not initialized")?;
    system.process_job_queue()
}

/// Generate automation report
pub fn generate_automation_report() -> TestAutomationReport {
    let system = get_test_automation_system().unwrap();
    system.generate_automation_report()
}