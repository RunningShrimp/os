//! Workflow orchestration and scheduling
//!
//! This module provides comprehensive workflow orchestration combining DAG execution,
//! cron scheduling, job management, persistence, and retry logic.

mod dag;
mod cron;
mod job;
mod persistence;
mod retry;

pub use dag::{Dag, DagError, DagResult, Node, Edge, DependencyType, NodeId, EdgeId};
pub use cron::{
    CronExpression, CronScheduler, CronError, CronResult,
    CronField, StepBase, validate_cron,
};
pub use job::{
    Job, JobId, JobState, JobPriority, JobContext, JobResult,
    JobScheduler, JobStatistics, JobSummary, generate_job_id,
};
pub use persistence::{
    StorageBackend, MemoryStorage, StorageError,
    JobPersistence, JobHistory, HistoryEvent, JobSnapshot,
    Checkpoint, CacheStats, HistoryQueryBuilder, HistoryId,
};
pub use retry::{
    RetryConfig, RetryStrategy, RetryError, RetryResult,
    RetryExecutor, RetryAttempt, RetryHistory, RetryOutcome,
    AttemptId,
};

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// Workflow execution result
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Workflow errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    /// DAG error
    DagError(DagError),
    /// Cron error
    CronError(CronError),
    /// Job error
    JobError(String),
    /// Persistence error
    PersistenceError(StorageError),
    /// Retry error
    RetryError(RetryError),
    /// Invalid workflow state
    InvalidState(String),
    /// Workflow not found
    NotFound(String),
    /// Dependency error
    DependencyError(String),
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DagError(e) => write!(f, "DAG error: {}", e),
            Self::CronError(e) => write!(f, "Cron error: {}", e),
            Self::JobError(e) => write!(f, "Job error: {}", e),
            Self::PersistenceError(e) => write!(f, "Persistence error: {}", e),
            Self::RetryError(e) => write!(f, "Retry error: {}", e),
            Self::InvalidState(e) => write!(f, "Invalid state: {}", e),
            Self::NotFound(e) => write!(f, "Not found: {}", e),
            Self::DependencyError(e) => write!(f, "Dependency error: {}", e),
        }
    }
}

impl From<DagError> for WorkflowError {
    fn from(err: DagError) -> Self {
        Self::DagError(err)
    }
}

/// Workflow definition
#[derive(Clone)]
pub struct Workflow {
    /// Unique workflow identifier
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: String,
    /// DAG defining task dependencies
    pub dag: Dag,
    /// Nodes to job mappings
    pub jobs: BTreeMap<NodeId, Job>,
    /// Cron schedule (if periodic)
    pub schedule: Option<CronScheduler>,
    /// Workflow state
    pub state: WorkflowState,
    /// Created timestamp
    pub created_at: u64,
    /// Last executed timestamp
    pub last_execution: Option<u64>,
    /// Next execution timestamp
    pub next_execution: Option<u64>,
    /// Execution statistics
    pub statistics: WorkflowStatistics,
    /// Retry configuration
    pub retry_config: Option<RetryConfig>,
}

/// Workflow execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowState {
    /// Workflow is idle
    Idle,
    /// Workflow is running
    Running,
    /// Workflow is paused
    Paused,
    /// Workflow is completed
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow is cancelled
    Cancelled,
}

/// Workflow execution statistics
#[derive(Debug, Clone, Default)]
pub struct WorkflowStatistics {
    /// Total executions
    pub total_executions: u32,
    /// Successful executions
    pub successful_executions: u32,
    /// Failed executions
    pub failed_executions: u32,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Average duration in milliseconds
    pub average_duration_ms: u64,
}

impl Workflow {
    /// Create a new workflow
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            dag: Dag::new(),
            jobs: BTreeMap::new(),
            schedule: None,
            state: WorkflowState::Idle,
            created_at: 0,
            last_execution: None,
            next_execution: None,
            statistics: Default::default(),
            retry_config: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set cron schedule
    pub fn with_schedule(mut self, cron_expr: &str) -> Result<Self, CronError> {
        let scheduler = CronScheduler::new(cron_expr)?;
        self.schedule = Some(scheduler);
        Ok(self)
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }

    /// Add a job to the workflow
    pub fn add_job(&mut self, job: Job) -> WorkflowResult<NodeId> {
        let node_id = self.dag.add_node(job.name.clone())?;
        self.jobs.insert(node_id, job);
        Ok(node_id)
    }

    /// Add a dependency between jobs
    pub fn add_dependency(
        &mut self,
        from: NodeId,
        to: NodeId,
    ) -> WorkflowResult<()> {
        self.dag.add_edge(from, to, DependencyType::Sequential)?;
        Ok(())
    }

    /// Validate the workflow
    pub fn validate(&self) -> WorkflowResult<()> {
        // Check DAG has no cycles
        if let Some(cycle) = self.dag.detect_cycle() {
            return Err(WorkflowError::DagError(DagError::CycleDetected(cycle)));
        }

        // Check all nodes have jobs
        for node_id in self.dag.node_ids() {
            if !self.jobs.contains_key(&node_id) {
                return Err(WorkflowError::JobError(
                    format!("No job for node: {}", node_id)
                ));
            }
        }

        Ok(())
    }

    /// Get ready jobs for execution
    pub fn ready_jobs(&self) -> Vec<NodeId> {
        let completed = self.jobs.iter()
            .filter(|(_, job)| job.state().is_terminal())
            .map(|(id, _)| *id)
            .collect::<BTreeSet<_>>();

        self.dag.ready_nodes(&completed).unwrap_or_default()
    }
}

/// Workflow orchestrator
pub struct WorkflowOrchestrator {
    /// All workflows
    workflows: BTreeMap<String, Workflow>,
    /// Job scheduler
    job_scheduler: JobScheduler,
    /// Persistence backend
    persistence: Option<JobPersistence<MemoryStorage>>,
    /// Current timestamp (for testing)
    current_time: u64,
}

impl Default for WorkflowOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowOrchestrator {
    /// Create a new orchestrator
    pub fn new() -> Self {
        Self {
            workflows: BTreeMap::new(),
            job_scheduler: JobScheduler::new(),
            persistence: None,
            current_time: 0,
        }
    }

    /// Enable persistence
    pub fn with_persistence(mut self) -> Self {
        self.persistence = Some(JobPersistence::new(MemoryStorage::new()));
        self
    }

    /// Set current time (for testing)
    pub fn with_time(mut self, time: u64) -> Self {
        self.current_time = time;
        self
    }

    /// Register a workflow
    pub fn register_workflow(&mut self, mut workflow: Workflow) -> WorkflowResult<()> {
        workflow.validate()?;
        workflow.created_at = self.current_time;
        self.workflows.insert(workflow.id.clone(), workflow);
        Ok(())
    }

    /// Get a workflow by ID
    pub fn get_workflow(&self, id: &str) -> Option<&Workflow> {
        self.workflows.get(id)
    }

    /// Get a mutable workflow
    pub fn get_workflow_mut(&mut self, id: &str) -> Option<&mut Workflow> {
        self.workflows.get_mut(id)
    }

    /// Unregister a workflow
    pub fn unregister_workflow(&mut self, id: &str) -> WorkflowResult<()> {
        self.workflows.remove(id)
            .ok_or_else(|| WorkflowError::NotFound(id.to_string()))?;
        Ok(())
    }

    /// Execute a workflow
    pub fn execute_workflow(&mut self, id: &str) -> WorkflowResult<()> {
        let workflow = self.workflows.get_mut(id)
            .ok_or_else(|| WorkflowError::NotFound(id.to_string()))?;

        if workflow.state == WorkflowState::Running {
            return Err(WorkflowError::InvalidState(
                "Workflow already running".to_string()
            ));
        }

        workflow.state = WorkflowState::Running;
        workflow.last_execution = Some(self.current_time);

        // Submit all jobs to the scheduler
        let execution_order = workflow.dag.topological_sort()?;

        for node_id in execution_order {
            if let Some(job) = workflow.jobs.get(&node_id) {
                let job_id = self.job_scheduler.submit(job.clone(), self.current_time);

                // Persist if enabled
                if let Some(persistence) = &mut self.persistence {
                    let _ = persistence.save_job(job);
                    let _ = persistence.record_event(
                        job_id,
                        self.current_time,
                        HistoryEvent::Started,
                        job.state(),
                        None,
                        Some(format!("Started as part of workflow {}", id)),
                    );
                }
            }
        }

        Ok(())
    }

    /// Process workflow execution
    pub fn process_workflows(&mut self) -> WorkflowResult<()> {
        // Check for scheduled workflows
        let mut ready_workflows = Vec::new();

        for (id, workflow) in &self.workflows {
            if let Some(_scheduler) = &workflow.schedule {
                if let Some(next_time) = workflow.next_execution {
                    if self.current_time >= next_time {
                        ready_workflows.push(id.clone());
                    }
                } else {
                    // No next execution set, calculate it
                    ready_workflows.push(id.clone());
                }
            }
        }

        // Execute ready workflows
        for id in ready_workflows {
            self.execute_workflow(&id)?;
        }

        // Process jobs
        self.process_jobs()?;

        Ok(())
    }

    /// Process job execution
    fn process_jobs(&mut self) -> WorkflowResult<()> {
        // Start new jobs
        while let Some(job_id) = self.job_scheduler.next_job(self.current_time) {
            if let Err(_e) = self.job_scheduler.start_job(job_id, self.current_time) {
                // Log error (would use proper logging in production)
            }

            // Simulate job execution (in real implementation, would run async)
            self.job_scheduler.complete_job(job_id, Ok(()), self.current_time);
        }

        // Check timeouts
        let timed_out = self.job_scheduler.check_timeouts(self.current_time);
        for _job_id in timed_out {
            // Log timeout (would use proper logging in production)
        }

        Ok(())
    }

    /// Cancel a workflow
    pub fn cancel_workflow(&mut self, id: &str) -> WorkflowResult<()> {
        let workflow = self.workflows.get_mut(id)
            .ok_or_else(|| WorkflowError::NotFound(id.to_string()))?;

        workflow.state = WorkflowState::Cancelled;

        // Cancel all associated jobs
        for (_node_id, job) in &workflow.jobs {
            let _ = self.job_scheduler.cancel_job(job.id, self.current_time);
        }

        Ok(())
    }

    /// Pause a workflow
    pub fn pause_workflow(&mut self, id: &str) -> WorkflowResult<()> {
        let workflow = self.workflows.get_mut(id)
            .ok_or_else(|| WorkflowError::NotFound(id.to_string()))?;

        if workflow.state != WorkflowState::Running {
            return Err(WorkflowError::InvalidState(
                "Can only pause running workflows".to_string()
            ));
        }

        workflow.state = WorkflowState::Paused;
        Ok(())
    }

    /// Resume a paused workflow
    pub fn resume_workflow(&mut self, id: &str) -> WorkflowResult<()> {
        let workflow = self.workflows.get_mut(id)
            .ok_or_else(|| WorkflowError::NotFound(id.to_string()))?;

        if workflow.state != WorkflowState::Paused {
            return Err(WorkflowError::InvalidState(
                "Can only resume paused workflows".to_string()
            ));
        }

        workflow.state = WorkflowState::Running;
        Ok(())
    }

    /// Get statistics for all workflows
    pub fn get_statistics(&self) -> OrchestratorStatistics {
        let mut stats = OrchestratorStatistics::default();

        for workflow in self.workflows.values() {
            stats.total_workflows += 1;
            match workflow.state {
                WorkflowState::Running => stats.running_workflows += 1,
                WorkflowState::Paused => stats.paused_workflows += 1,
                WorkflowState::Completed => stats.completed_workflows += 1,
                WorkflowState::Failed => stats.failed_workflows += 1,
                _ => {}
            }
        }

        stats.job_stats = self.job_scheduler.statistics();
        stats
    }

    /// Update time and process scheduled workflows
    pub fn tick(&mut self) -> WorkflowResult<()> {
        self.current_time += 1;
        self.process_workflows()
    }
}

/// Orchestrator-wide statistics
#[derive(Debug, Clone, Default)]
pub struct OrchestratorStatistics {
    pub total_workflows: usize,
    pub running_workflows: usize,
    pub paused_workflows: usize,
    pub completed_workflows: usize,
    pub failed_workflows: usize,
    pub job_stats: JobStatistics,
}

/// Builder for creating workflows
pub struct WorkflowBuilder {
    workflow: Workflow,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            workflow: Workflow::new(id, name),
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.workflow = self.workflow.with_description(desc);
        self
    }

    /// Set cron schedule
    pub fn schedule(mut self, cron_expr: &str) -> Result<Self, CronError> {
        self.workflow = self.workflow.with_schedule(cron_expr)?;
        Ok(self)
    }

    /// Set retry configuration
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.workflow = self.workflow.with_retry_config(config);
        self
    }

    /// Add a job
    pub fn job(mut self, job: Job) -> Result<Self, WorkflowError> {
        let _node_id = self.workflow.add_job(job)?;
        Ok(self)
    }

    /// Add a dependency
    pub fn dependency(mut self, from: NodeId, to: NodeId) -> Result<Self, WorkflowError> {
        self.workflow.add_dependency(from, to)?;
        Ok(self)
    }

    /// Build the workflow
    pub fn build(self) -> WorkflowResult<Workflow> {
        self.workflow.validate()?;
        Ok(self.workflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("test-id", "Test Workflow");
        assert_eq!(workflow.id, "test-id");
        assert_eq!(workflow.name, "Test Workflow");
    }

    #[test]
    fn test_workflow_validation() {
        let mut workflow = Workflow::new("test", "Test");

        // Add nodes
        let node1 = workflow.dag.add_node("job1").unwrap();
        let node2 = workflow.dag.add_node("job2").unwrap();

        // Add jobs
        workflow.jobs.insert(node1, Job::new("job1"));
        workflow.jobs.insert(node2, Job::new("job2"));

        // Add edge
        workflow.dag.add_edge(node1, node2, DependencyType::Sequential).unwrap();

        assert!(workflow.validate().is_ok());
    }

    #[test]
    fn test_workflow_cycle_detection() {
        let mut workflow = Workflow::new("test", "Test");

        let node1 = workflow.dag.add_node("job1").unwrap();
        let node2 = workflow.dag.add_node("job2").unwrap();

        workflow.dag.add_edge(node1, node2, DependencyType::Sequential).unwrap();

        // Create cycle
        let result = workflow.dag.add_edge(node2, node1, DependencyType::Sequential);
        assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }

    #[test]
    fn test_orchestrator_registration() {
        let mut orchestrator = WorkflowOrchestrator::new();
        let workflow = Workflow::new("test", "Test");

        orchestrator.register_workflow(workflow).unwrap();
        assert!(orchestrator.get_workflow("test").is_some());
    }

    #[test]
    fn test_orchestrator_execution() {
        let mut orchestrator = WorkflowOrchestrator::new().with_time(100);

        let mut workflow = Workflow::new("test", "Test");
        let node1 = workflow.dag.add_node("job1").unwrap();
        workflow.jobs.insert(node1, Job::new("job1"));

        orchestrator.register_workflow(workflow).unwrap();
        orchestrator.execute_workflow("test").unwrap();

        let stats = orchestrator.get_statistics();
        assert_eq!(stats.job_stats.total, 1);
    }

    #[test]
    fn test_workflow_builder() {
        let job = Job::new("test-job");
        let workflow = WorkflowBuilder::new("test-id", "Test Workflow")
            .description("A test workflow")
            .job(job)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(workflow.id, "test-id");
        assert_eq!(workflow.description, "A test workflow");
    }

    #[test]
    fn test_ready_jobs() {
        let mut workflow = Workflow::new("test", "Test");

        let node1 = workflow.dag.add_node("job1").unwrap();
        let node2 = workflow.dag.add_node("job2").unwrap();

        let mut job1 = Job::new("job1");
        job1.complete(100);
        workflow.jobs.insert(node1, job1.clone());

        let job2 = Job::new("job2");
        workflow.jobs.insert(node2, job2);

        workflow.dag.add_edge(node1, node2, DependencyType::Sequential).unwrap();

        let ready = workflow.ready_jobs();
        assert!(ready.contains(&node2));
    }

    #[test]
    fn test_statistics() {
        let mut orchestrator = WorkflowOrchestrator::new();

        let workflow = Workflow::new("test", "Test");
        orchestrator.register_workflow(workflow).unwrap();

        let stats = orchestrator.get_statistics();
        assert_eq!(stats.total_workflows, 1);
    }

    #[test]
    fn test_cron_scheduler() {
        let scheduler = CronScheduler::new("0 * * * * *").unwrap();
        assert!(scheduler.is_scheduled(0, 0, 0, 1, 1, 0));
        assert!(!scheduler.is_scheduled(1, 0, 0, 1, 1, 0));
    }

    #[test]
    fn test_retry_config() {
        let config = RetryConfig::exponential(3, Duration::from_millis(100));
        assert_eq!(config.max_attempts, 3);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_job_priority() {
        let high = JobPriority::High;
        let low = JobPriority::Low;
        assert!(high > low);
    }

    #[test]
    fn test_dag_performance() {
        let mut dag = Dag::new();

        // Create a chain of 100 nodes
        let mut prev = dag.add_node("node0").unwrap();
        for i in 1..100 {
            let curr = dag.add_node(format!("node{}", i)).unwrap();
            dag.add_edge(prev, curr, DependencyType::Sequential).unwrap();
            prev = curr;
        }

        // Verify topological sort works
        let result = dag.topological_sort();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 100);
    }
}
