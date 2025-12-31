//! Job execution and state management
//!
//! This module provides comprehensive job scheduling, execution, and
//! state management with priority-based scheduling and timeout handling.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

/// Unique identifier for a job
pub type JobId = u64;

/// Atomic job ID counter
static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a new unique job ID
pub fn generate_job_id() -> JobId {
    NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst)
}

/// Job execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// Job is pending execution
    Pending,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
    /// Job timed out
    TimedOut,
    /// Job is paused
    Paused,
    /// Job is waiting for retry
    WaitingRetry,
}

impl JobState {
    /// Check if job is in a terminal state
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut)
    }

    /// Check if job can be executed
    pub fn is_executable(self) -> bool {
        matches!(self, Self::Pending | Self::WaitingRetry)
    }

    /// Check if job is active
    pub fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::TimedOut => write!(f, "TimedOut"),
            Self::Paused => write!(f, "Paused"),
            Self::WaitingRetry => write!(f, "WaitingRetry"),
        }
    }
}

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    /// Low priority
    Low = 0,
    /// Normal priority (default)
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

impl JobPriority {
    /// Convert from integer
    pub fn from_i32(value: i32) -> Self {
        match value {
            x if x <= 0 => Self::Low,
            1 => Self::Normal,
            2 => Self::High,
            _ => Self::Critical,
        }
    }

    /// Convert to integer
    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

impl Default for JobPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Job execution context
pub type JobContext = BTreeMap<String, String>;

/// Result of job execution
pub type JobResult = Result<(), String>;

/// Job execution function type
pub type JobFn = Arc<dyn Fn() -> JobResult + Send + Sync>;

/// Job definition
#[derive(Clone)]
pub struct Job {
    /// Unique job ID
    pub id: JobId,
    /// Job name
    pub name: String,
    /// Job description
    pub description: String,
    /// Job state
    state: JobState,
    /// Job priority
    pub priority: JobPriority,
    /// Execution function
    executor: Option<JobFn>,
    /// Maximum execution time
    pub timeout: Option<Duration>,
    /// Creation time
    pub created_at: u64,
    /// Scheduled execution time
    pub scheduled_at: Option<u64>,
    /// Start time
    pub started_at: Option<u64>,
    /// Completion time
    pub completed_at: Option<u64>,
    /// Number of retries attempted
    pub retry_count: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// Job context
    pub context: JobContext,
    /// Dependencies (job IDs that must complete first)
    pub dependencies: Vec<JobId>,
    /// Error message if failed
    pub error: Option<String>,
}

impl Job {
    /// Create a new job
    pub fn new(name: impl Into<String>) -> Self {
        let id = generate_job_id();
        Self {
            id,
            name: name.into(),
            description: String::new(),
            state: JobState::Pending,
            priority: JobPriority::default(),
            executor: None,
            timeout: None,
            created_at: 0,  // Set by scheduler
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 0,
            context: BTreeMap::new(),
            dependencies: Vec::new(),
            error: None,
        }
    }

    /// Set the job description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the job priority
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the execution function
    pub fn with_executor(mut self, executor: JobFn) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the scheduled execution time
    pub fn with_scheduled_at(mut self, time: u64) -> Self {
        self.scheduled_at = Some(time);
        self
    }

    /// Set the maximum retries
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Add a dependency
    pub fn with_dependency(mut self, job_id: JobId) -> Self {
        self.dependencies.push(job_id);
        self
    }

    /// Add context
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Get the job state
    pub fn state(&self) -> JobState {
        self.state
    }

    /// Set the job state
    pub fn set_state(&mut self, new_state: JobState) {
        self.state = new_state;
    }

    /// Check if job can execute now
    pub fn can_execute(&self, current_time: u64, completed_jobs: &[JobId]) -> bool {
        if !self.state.is_executable() {
            return false;
        }

        // Check if scheduled time has arrived
        if let Some(scheduled) = self.scheduled_at {
            if current_time < scheduled {
                return false;
            }
        }

        // Check if all dependencies are satisfied
        self.dependencies.iter().all(|dep_id| completed_jobs.contains(dep_id))
    }

    /// Execute the job
    pub fn execute(&mut self) -> JobResult {
        if self.state != JobState::Pending && self.state != JobState::WaitingRetry {
            return Err(format!("Cannot execute job in state: {}", self.state));
        }

        self.state = JobState::Running;

        if let Some(executor) = &self.executor {
            executor()
        } else {
            // No executor defined, succeed by default
            Ok(())
        }
    }

    /// Mark job as completed
    pub fn complete(&mut self, time: u64) {
        self.state = JobState::Completed;
        self.completed_at = Some(time);
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: impl Into<String>, time: u64) {
        self.state = JobState::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(time);
    }

    /// Cancel the job
    pub fn cancel(&mut self, time: u64) {
        self.state = JobState::Cancelled;
        self.completed_at = Some(time);
    }

    /// Mark job as timed out
    pub fn timeout(&mut self, time: u64) {
        self.state = JobState::TimedOut;
        self.completed_at = Some(time);
    }

    /// Pause the job
    pub fn pause(&mut self) {
        if self.state == JobState::Running {
            self.state = JobState::Paused;
        }
    }

    /// Resume the job
    pub fn resume(&mut self) {
        if self.state == JobState::Paused {
            self.state = JobState::Running;
        }
    }

    /// Increment retry count and set to waiting
    pub fn schedule_retry(&mut self) -> bool {
        if self.retry_count < self.max_retries {
            self.retry_count += 1;
            self.state = JobState::WaitingRetry;
            true
        } else {
            false
        }
    }

    /// Get execution duration
    pub fn execution_duration(&self) -> Option<Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            Some(Duration::from_millis(completed.saturating_sub(started)))
        } else {
            None
        }
    }

    /// Check if job has timed out
    pub fn has_timed_out(&self, current_time: u64) -> bool {
        if let (Some(timeout), Some(started)) = (self.timeout, self.started_at) {
            let elapsed = Duration::from_millis(current_time.saturating_sub(started));
            elapsed > timeout
        } else {
            false
        }
    }

    /// Get a summary of the job
    pub fn summary(&self) -> JobSummary {
        JobSummary {
            id: self.id,
            name: self.name.clone(),
            state: self.state,
            priority: self.priority,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            retry_count: self.retry_count,
            error: self.error.clone(),
        }
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job(id={}, name={}, state={}, priority={})",
            self.id, self.name, self.state, self.priority as i32)
    }
}

/// Job summary for reporting
#[derive(Debug, Clone)]
pub struct JobSummary {
    pub id: JobId,
    pub name: String,
    pub state: JobState,
    pub priority: JobPriority,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub retry_count: u32,
    pub error: Option<String>,
}

/// Job scheduler with priority-based fair scheduling
pub struct JobScheduler {
    /// Pending jobs organized by priority
    pending: BTreeMap<JobPriority, Vec<JobId>>,
    /// All jobs by ID
    jobs: BTreeMap<JobId, Job>,
    /// Currently executing jobs
    running: BTreeMap<JobId, u64>,
    /// Completed job IDs
    completed: Vec<JobId>,
    /// Maximum concurrent jobs
    max_concurrent: usize,
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl JobScheduler {
    /// Create a new job scheduler
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            jobs: BTreeMap::new(),
            running: BTreeMap::new(),
            completed: Vec::new(),
            max_concurrent: 4,
        }
    }

    /// Create a scheduler with a specific max concurrency
    pub fn with_concurrency(max_concurrent: usize) -> Self {
        Self {
            pending: BTreeMap::new(),
            jobs: BTreeMap::new(),
            running: BTreeMap::new(),
            completed: Vec::new(),
            max_concurrent,
        }
    }

    /// Submit a job for execution
    pub fn submit(&mut self, mut job: Job, current_time: u64) -> JobId {
        job.created_at = current_time;
        let id = job.id;
        let priority = job.priority;

        self.jobs.insert(id, job);
        self.pending.entry(priority)
            .or_insert_with(Vec::new)
            .push(id);

        id
    }

    /// Get a job by ID
    pub fn get_job(&self, id: JobId) -> Option<&Job> {
        self.jobs.get(&id)
    }

    /// Get a mutable job by ID
    pub fn get_job_mut(&mut self, id: JobId) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    /// Get next job to execute
    pub fn next_job(&mut self, current_time: u64) -> Option<JobId> {
        if self.running.len() >= self.max_concurrent {
            return None;
        }

        // Check priorities from highest to lowest
        for priority in [JobPriority::Critical, JobPriority::High,
                         JobPriority::Normal, JobPriority::Low] {
            if let Some(jobs) = self.pending.get_mut(&priority) {
                while let Some(job_id) = jobs.pop() {
                    if let Some(job) = self.jobs.get(&job_id) {
                        if job.can_execute(current_time, &self.completed) {
                            return Some(job_id);
                        } else {
                            // Not ready, put back
                            jobs.insert(0, job_id);
                            break;
                        }
                    }
                }
            }
        }

        None
    }

    /// Start a job execution
    pub fn start_job(&mut self, id: JobId, current_time: u64) -> Result<(), String> {
        let job = self.jobs.get_mut(&id)
            .ok_or_else(|| format!("Job not found: {}", id))?;

        if !job.can_execute(current_time, &self.completed) {
            return Err(format!("Job cannot execute: {}", id));
        }

        job.state = JobState::Running;
        job.started_at = Some(current_time);
        self.running.insert(id, current_time);

        Ok(())
    }

    /// Complete a job execution
    pub fn complete_job(&mut self, id: JobId, result: JobResult, current_time: u64) {
        if let Some(job) = self.jobs.get_mut(&id) {
            match result {
                Ok(()) => {
                    job.complete(current_time);
                }
                Err(error) => {
                    if job.schedule_retry() {
                        // Schedule for retry
                        job.started_at = None;
                        self.pending.entry(job.priority)
                            .or_insert_with(Vec::new)
                            .push(id);
                    } else {
                        job.fail(error, current_time);
                    }
                }
            }

            self.running.remove(&id);
            if job.state.is_terminal() {
                self.completed.push(id);
            }
        }
    }

    /// Cancel a job
    pub fn cancel_job(&mut self, id: JobId, current_time: u64) -> Result<(), String> {
        let job = self.jobs.get_mut(&id)
            .ok_or_else(|| format!("Job not found: {}", id))?;

        if job.state.is_terminal() {
            return Err(format!("Job already completed: {}", id));
        }

        job.cancel(current_time);

        // Remove from pending or running
        self.running.remove(&id);
        if let Some(jobs) = self.pending.get_mut(&job.priority) {
            jobs.retain(|j| *j != id);
        }

        self.completed.push(id);
        Ok(())
    }

    /// Check for timed out jobs
    pub fn check_timeouts(&mut self, current_time: u64) -> Vec<JobId> {
        let mut timed_out = Vec::new();

        for (&id, &_started) in self.running.iter() {
            if let Some(job) = self.jobs.get(&id) {
                if job.has_timed_out(current_time) {
                    timed_out.push(id);
                }
            }
        }

        for id in timed_out.clone() {
            if let Some(job) = self.jobs.get_mut(&id) {
                job.timeout(current_time);
            }
            self.running.remove(&id);
            self.completed.push(id);
        }

        timed_out
    }

    /// Get all jobs
    pub fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values()
    }

    /// Get pending job count
    pub fn pending_count(&self) -> usize {
        self.pending.values().map(|v| v.len()).sum()
    }

    /// Get running job count
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Get completed job count
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Get job statistics
    pub fn statistics(&self) -> JobStatistics {
        let mut stats = JobStatistics::default();
        stats.total = self.jobs.len();
        stats.pending = self.pending_count();
        stats.running = self.running_count();
        stats.completed = self.completed_count();

        for job in self.jobs.values() {
            match job.state {
                JobState::Completed => stats.succeeded += 1,
                JobState::Failed | JobState::TimedOut => stats.failed += 1,
                JobState::Cancelled => stats.cancelled += 1,
                _ => {}
            }
        }

        stats
    }

    /// Clear completed jobs
    pub fn clear_completed(&mut self) {
        for id in &self.completed {
            self.jobs.remove(id);
        }
        self.completed.clear();
    }

    /// Clear all jobs
    pub fn clear_all(&mut self) {
        self.pending.clear();
        self.jobs.clear();
        self.running.clear();
        self.completed.clear();
    }
}

/// Job execution statistics
#[derive(Debug, Clone, Default)]
pub struct JobStatistics {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new("test_job");
        assert_eq!(job.state(), JobState::Pending);
        assert_eq!(job.priority, JobPriority::Normal);
    }

    #[test]
    fn test_job_state_transitions() {
        let mut job = Job::new("test");
        assert_eq!(job.state(), JobState::Pending);

        job.complete(100);
        assert_eq!(job.state(), JobState::Completed);
        assert!(job.state().is_terminal());
    }

    #[test]
    fn test_job_retry() {
        let mut job = Job::new("test").with_max_retries(3);
        assert!(job.schedule_retry());
        assert_eq!(job.retry_count, 1);
        assert_eq!(job.state(), JobState::WaitingRetry);
    }

    #[test]
    fn test_job_timeout() {
        let job = Job::new("test")
            .with_timeout(Duration::from_secs(1));
        job.started_at = Some(100);
        assert!(!job.has_timed_out(1500));
        assert!(job.has_timed_out(2500));
    }

    #[test]
    fn test_scheduler_submit() {
        let mut scheduler = JobScheduler::new();
        let job = Job::new("test");
        let id = scheduler.submit(job, 0);
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn test_scheduler_statistics() {
        let mut scheduler = JobScheduler::new();
        let job = Job::new("test");
        scheduler.submit(job, 0);

        let stats = scheduler.statistics();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(JobPriority::Critical > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Normal);
        assert!(JobPriority::Normal > JobPriority::Low);
    }
}
