//! Job state persistence and recovery
//!
//! This module provides persistence for workflow jobs, including state
//! tracking, history management, and crash recovery capabilities.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::workflow::job::{Job, JobId, JobState};

/// Unique identifier for a history record
pub type HistoryId = u64;

/// Storage backend trait for job persistence
pub trait StorageBackend: Send + Sync {
    /// Save a job to storage
    fn save_job(&mut self, job: &Job) -> Result<(), StorageError>;

    /// Load a job from storage
    fn load_job(&self, id: JobId) -> Result<Option<Job>, StorageError>;

    /// Delete a job from storage
    fn delete_job(&mut self, id: JobId) -> Result<(), StorageError>;

    /// List all job IDs
    fn list_jobs(&self) -> Result<Vec<JobId>, StorageError>;

    /// Save a history record
    fn save_history(&mut self, history: &JobHistory) -> Result<(), StorageError>;

    /// Load history for a job
    fn load_history(&self, job_id: JobId) -> Result<Vec<JobHistory>, StorageError>;

    /// Clear all data
    fn clear(&mut self) -> Result<(), StorageError>;
}

/// In-memory storage backend for testing and development
#[derive(Default)]
pub struct MemoryStorage {
    jobs: BTreeMap<JobId, Job>,
    history: BTreeMap<JobId, Vec<JobHistory>>,
    next_history_id: HistoryId,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StorageBackend for MemoryStorage {
    fn save_job(&mut self, job: &Job) -> Result<(), StorageError> {
        self.jobs.insert(job.id, job.clone());
        Ok(())
    }

    fn load_job(&self, id: JobId) -> Result<Option<Job>, StorageError> {
        Ok(self.jobs.get(&id).cloned())
    }

    fn delete_job(&mut self, id: JobId) -> Result<(), StorageError> {
        self.jobs.remove(&id);
        self.history.remove(&id);
        Ok(())
    }

    fn list_jobs(&self) -> Result<Vec<JobId>, StorageError> {
        Ok(self.jobs.keys().copied().collect())
    }

    fn save_history(&mut self, history: &JobHistory) -> Result<(), StorageError> {
        self.history
            .entry(history.job_id)
            .or_insert_with(Vec::new)
            .push(history.clone());
        Ok(())
    }

    fn load_history(&self, job_id: JobId) -> Result<Vec<JobHistory>, StorageError> {
        Ok(self.history.get(&job_id).cloned().unwrap_or_default())
    }

    fn clear(&mut self) -> Result<(), StorageError> {
        self.jobs.clear();
        self.history.clear();
        Ok(())
    }
}

/// Storage errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// IO error
    Io(String),
    /// Serialization error
    Serialization(String),
    /// Deserialization error
    Deserialization(String),
    /// Job not found
    NotFound(JobId),
    /// Storage full
    StorageFull,
    /// Corrupted data
    Corrupted(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Self::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
            Self::NotFound(id) => write!(f, "Job not found: {}", id),
            Self::StorageFull => write!(f, "Storage full"),
            Self::Corrupted(msg) => write!(f, "Corrupted data: {}", msg),
        }
    }
}

/// Job history record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobHistory {
    /// Unique history ID
    pub id: HistoryId,
    /// Associated job ID
    pub job_id: JobId,
    /// Timestamp of the record
    pub timestamp: u64,
    /// Event type
    pub event: HistoryEvent,
    /// Previous state
    pub previous_state: Option<JobState>,
    /// New state
    pub new_state: JobState,
    /// Optional message
    pub message: Option<String>,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

/// History event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryEvent {
    /// Job was created
    Created,
    /// Job started execution
    Started,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
    /// Job timed out
    TimedOut,
    /// Job was paused
    Paused,
    /// Job was resumed
    Resumed,
    /// Job retry scheduled
    RetryScheduled,
    /// Job recovered from crash
    Recovered,
}

impl JobHistory {
    /// Create a new history record
    pub fn new(
        job_id: JobId,
        timestamp: u64,
        event: HistoryEvent,
        new_state: JobState,
    ) -> Self {
        Self {
            id: 0,  // Set by persistence layer
            job_id,
            timestamp,
            event,
            previous_state: None,
            new_state,
            message: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Set previous state
    pub fn with_previous_state(mut self, state: JobState) -> Self {
        self.previous_state = Some(state);
        self
    }

    /// Set message
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Job persistence manager
pub struct JobPersistence<B: StorageBackend> {
    /// Storage backend
    backend: B,
    /// In-memory cache of jobs
    cache: BTreeMap<JobId, Job>,
    /// Next history ID
    next_history_id: HistoryId,
    /// Maximum history records per job
    max_history: usize,
    /// Whether to cache all jobs
    cache_enabled: bool,
}

impl<B: StorageBackend> JobPersistence<B> {
    /// Create a new persistence manager
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            cache: BTreeMap::new(),
            next_history_id: 1,
            max_history: 100,
            cache_enabled: true,
        }
    }

    /// Set maximum history records per job
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Enable or disable caching
    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Save a job
    pub fn save_job(&mut self, job: &Job) -> Result<(), StorageError> {
        self.backend.save_job(job)?;

        if self.cache_enabled {
            self.cache.insert(job.id, job.clone());
        }

        Ok(())
    }

    /// Load a job
    pub fn load_job(&mut self, id: JobId) -> Result<Option<Job>, StorageError> {
        // Check cache first
        if self.cache_enabled {
            if let Some(job) = self.cache.get(&id) {
                return Ok(Some(job.clone()));
            }
        }

        // Load from backend
        if let Some(job) = self.backend.load_job(id)? {
            if self.cache_enabled {
                self.cache.insert(id, job.clone());
            }
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    /// Delete a job
    pub fn delete_job(&mut self, id: JobId) -> Result<(), StorageError> {
        self.backend.delete_job(id)?;
        self.cache.remove(&id);
        Ok(())
    }

    /// List all jobs
    pub fn list_jobs(&self) -> Result<Vec<JobId>, StorageError> {
        self.backend.list_jobs()
    }

    /// Record a history event
    pub fn record_event(
        &mut self,
        job_id: JobId,
        timestamp: u64,
        event: HistoryEvent,
        new_state: JobState,
        previous_state: Option<JobState>,
        message: Option<String>,
    ) -> Result<(), StorageError> {
        let mut history = JobHistory::new(job_id, timestamp, event, new_state);
        history.id = self.next_history_id;
        self.next_history_id += 1;

        if let Some(prev) = previous_state {
            history = history.with_previous_state(prev);
        }

        if let Some(msg) = message {
            history = history.with_message(msg);
        }

        self.backend.save_history(&history)?;

        // Prune old history if needed
        let all_history = self.backend.load_history(job_id)?;
        if all_history.len() > self.max_history {
            // Keep only the most recent records
            let _to_keep: Vec<_> = all_history.iter()
                .rev()
                .take(self.max_history)
                .cloned()
                .collect();

            // This is simplified - real implementation would delete old records
        }

        Ok(())
    }

    /// Load history for a job
    pub fn load_history(&self, job_id: JobId) -> Result<Vec<JobHistory>, StorageError> {
        self.backend.load_history(job_id)
    }

    /// Get job state snapshot
    pub fn get_snapshot(&self, id: JobId) -> Result<Option<JobSnapshot>, StorageError> {
        if let Some(job) = self.cache.get(&id) {
            Ok(Some(JobSnapshot::from_job(job)))
        } else if let Some(job) = self.backend.load_job(id)? {
            Ok(Some(JobSnapshot::from_job(&job)))
        } else {
            Ok(None)
        }
    }

    /// Recover jobs after a crash
    pub fn recover_jobs(&mut self) -> Result<Vec<Job>, StorageError> {
        let job_ids = self.backend.list_jobs()?;
        let mut recovered = Vec::new();

        for id in job_ids {
            if let Some(mut job) = self.backend.load_job(id)? {
                // Check job state and determine recovery action
                match job.state() {
                    JobState::Running => {
                        // Job was running when crash occurred
                        // Mark as failed or ready for retry
                        if job.schedule_retry() {
                            let msg = format!("Job recovered from crash, retry {}/{}",
                                job.retry_count, job.max_retries);
                            self.record_event(
                                id,
                                0,  // Current time would be passed in
                                HistoryEvent::Recovered,
                                job.state(),
                                Some(JobState::Running),
                                Some(msg),
                            )?;
                        }
                    }
                    JobState::Paused => {
                        // Resume paused jobs
                        job.resume();
                        self.record_event(
                            id,
                            0,
                            HistoryEvent::Resumed,
                            job.state(),
                            Some(JobState::Paused),
                            Some("Job resumed after crash".to_string()),
                        )?;
                    }
                    _ => {
                        // Leave other states as is
                    }
                }

                if self.cache_enabled {
                    self.cache.insert(id, job.clone());
                }

                recovered.push(job);
            }
        }

        Ok(recovered)
    }

    /// Create a checkpoint of all jobs
    pub fn create_checkpoint(&mut self) -> Result<Checkpoint, StorageError> {
        let job_ids = self.backend.list_jobs()?;
        let mut jobs = Vec::new();

        for id in job_ids {
            if let Some(job) = self.load_job(id)? {
                jobs.push(job);
            }
        }

        let checkpoint = Checkpoint {
            id: self.next_history_id,
            timestamp: 0,  // Would be set to current time
            job_count: jobs.len(),
            jobs: jobs.into_iter().map(|j| j.id).collect(),
        };

        Ok(checkpoint)
    }

    /// Restore from a checkpoint
    pub fn restore_checkpoint(&mut self, _checkpoint: &Checkpoint) -> Result<(), StorageError> {
        // Load all jobs
        let job_ids = self.backend.list_jobs()?;

        for id in job_ids {
            if let Some(job) = self.backend.load_job(id)? {
                if self.cache_enabled {
                    self.cache.insert(id, job);
                }
            }
        }

        Ok(())
    }

    /// Clear all persisted data
    pub fn clear(&mut self) -> Result<(), StorageError> {
        self.backend.clear()?;
        self.cache.clear();
        Ok(())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            cached_jobs: self.cache.len(),
            cache_enabled: self.cache_enabled,
        }
    }
}

/// Snapshot of a job's state
#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub id: JobId,
    pub name: String,
    pub state: JobState,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub retry_count: u32,
}

impl JobSnapshot {
    fn from_job(job: &Job) -> Self {
        Self {
            id: job.id,
            name: job.name.clone(),
            state: job.state(),
            created_at: job.created_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            retry_count: job.retry_count,
        }
    }
}

/// Checkpoint data
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub id: HistoryId,
    pub timestamp: u64,
    pub job_count: usize,
    pub jobs: Vec<JobId>,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cached_jobs: usize,
    pub cache_enabled: bool,
}

/// Query builder for job history
pub struct HistoryQueryBuilder {
    job_id: Option<JobId>,
    event_type: Option<HistoryEvent>,
    start_time: Option<u64>,
    end_time: Option<u64>,
    limit: Option<usize>,
}

impl Default for HistoryQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryQueryBuilder {
    pub fn new() -> Self {
        Self {
            job_id: None,
            event_type: None,
            start_time: None,
            end_time: None,
            limit: None,
        }
    }

    pub fn with_job_id(mut self, id: JobId) -> Self {
        self.job_id = Some(id);
        self
    }

    pub fn with_event_type(mut self, event: HistoryEvent) -> Self {
        self.event_type = Some(event);
        self
    }

    pub fn with_time_range(mut self, start: u64, end: u64) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn execute<B: StorageBackend>(
        &self,
        persistence: &JobPersistence<B>,
    ) -> Result<Vec<JobHistory>, StorageError> {
        let mut results = Vec::new();

        // Get all history or specific job history
        let histories = if let Some(id) = self.job_id {
            persistence.load_history(id)?
        } else {
            // Would need to load all histories in real implementation
            return Ok(Vec::new());
        };

        for history in histories {
            // Apply filters
            if let Some(event) = self.event_type {
                if history.event != event {
                    continue;
                }
            }

            if let Some(start) = self.start_time {
                if history.timestamp < start {
                    continue;
                }
            }

            if let Some(end) = self.end_time {
                if history.timestamp > end {
                    continue;
                }
            }

            results.push(history);

            if let Some(limit) = self.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::job::{Job, JobState};

    #[test]
    fn test_memory_storage() {
        let mut storage = MemoryStorage::new();
        let job = Job::new("test");

        storage.save_job(&job).unwrap();
        let loaded = storage.load_job(job.id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "test");
    }

    #[test]
    fn test_history_record() {
        let history = JobHistory::new(1, 100, HistoryEvent::Created, JobState::Pending)
            .with_message("Job created")
            .with_metadata("key", "value");

        assert_eq!(history.job_id, 1);
        assert_eq!(history.event, HistoryEvent::Created);
        assert!(history.message.is_some());
    }

    #[test]
    fn test_persistence_save_load() {
        let backend = MemoryStorage::new();
        let mut persistence = JobPersistence::new(backend);

        let job = Job::new("test");
        persistence.save_job(&job).unwrap();

        let loaded = persistence.load_job(job.id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "test");
    }

    #[test]
    fn test_record_event() {
        let backend = MemoryStorage::new();
        let mut persistence = JobPersistence::new(backend);

        persistence.record_event(
            1,
            100,
            HistoryEvent::Created,
            JobState::Pending,
            None,
            Some("Created".to_string()),
        ).unwrap();

        let history = persistence.load_history(1).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_snapshot() {
        let job = Job::new("test");
        let snapshot = JobSnapshot::from_job(&job);

        assert_eq!(snapshot.id, job.id);
        assert_eq!(snapshot.name, job.name);
        assert_eq!(snapshot.state, job.state());
    }

    #[test]
    fn test_history_query_builder() {
        let query = HistoryQueryBuilder::new()
            .with_job_id(1)
            .with_event_type(HistoryEvent::Created)
            .with_limit(10);

        assert_eq!(query.job_id, Some(1));
        assert_eq!(query.event_type, Some(HistoryEvent::Created));
        assert_eq!(query.limit, Some(10));
    }
}
