//! # Experiment Tracking
//!
//! Comprehensive experiment tracking system for machine learning experiments.
//! Provides MLflow-compatible logging, hyperparameter tracking, and model versioning.
//!
//! ## Features
//!
//! - **Experiment Logging**: Track parameters, metrics, and artifacts
//! - **Model Versioning**: Automatic version control for models
//! - **Reproducibility**: Capture all experiment context
//! - **Comparison**: Easy comparison between runs
//! - **Storage Backend**: Flexible storage options

use crate::mlops::{ExperimentError, MLOpsError, MLOpsResult};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::time::Duration;
use core::fmt;

/// Experiment run status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Run is scheduled
    Scheduled,
    /// Run is currently running
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed
    Failed,
    /// Run was killed
    Killed,
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunStatus::Scheduled => write!(f, "SCHEDULED"),
            RunStatus::Running => write!(f, "RUNNING"),
            RunStatus::Completed => write!(f, "COMPLETED"),
            RunStatus::Failed => write!(f, "FAILED"),
            RunStatus::Killed => write!(f, "KILLED"),
        }
    }
}

/// Experiment run metadata
#[derive(Debug, Clone)]
pub struct RunMetadata {
    /// Unique run ID
    pub run_id: String,
    /// Experiment name
    pub experiment_name: String,
    /// Run status
    pub status: RunStatus,
    /// Start timestamp (nanoseconds)
    pub start_time: u64,
    /// End timestamp (nanoseconds)
    pub end_time: Option<u64>,
    /// Duration in nanoseconds
    pub duration: Option<Duration>,
    /// User who created the run
    pub user: Option<String>,
    /// Source of the run (e.g., script name)
    pub source_name: Option<String>,
    /// Source version (e.g., git commit)
    pub source_version: Option<String>,
    /// Entry point name
    pub entry_point_name: Option<String>,
}

impl RunMetadata {
    /// Create new run metadata
    pub fn new(run_id: String, experiment_name: String) -> Self {
        Self {
            run_id,
            experiment_name,
            status: RunStatus::Scheduled,
            start_time: 0,
            end_time: None,
            duration: None,
            user: None,
            source_name: None,
            source_version: None,
            entry_point_name: None,
        }
    }

    /// Mark run as started
    pub fn start(&mut self, start_time: u64) {
        self.status = RunStatus::Running;
        self.start_time = start_time;
    }

    /// Mark run as completed
    pub fn complete(&mut self, end_time: u64) {
        self.status = RunStatus::Completed;
        self.end_time = Some(end_time);
        self.duration = Some(Duration::from_nanos(end_time.saturating_sub(self.start_time)));
    }

    /// Mark run as failed
    pub fn fail(&mut self, end_time: u64) {
        self.status = RunStatus::Failed;
        self.end_time = Some(end_time);
        self.duration = Some(Duration::from_nanos(end_time.saturating_sub(self.start_time)));
    }
}

/// Parameter value
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    /// Float parameter
    Float(f64),
    /// Integer parameter
    Integer(i64),
    /// String parameter
    String(String),
    /// Boolean parameter
    Boolean(bool),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::Float(v) => write!(f, "{}", v),
            ParamValue::Integer(v) => write!(f, "{}", v),
            ParamValue::String(v) => write!(f, "{}", v),
            ParamValue::Boolean(v) => write!(f, "{}", v),
        }
    }
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct MetricData {
    /// Metric key
    pub key: String,
    /// Metric value
    pub value: f64,
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Step number
    pub step: Option<i64>,
}

impl MetricData {
    /// Create new metric data
    pub fn new(key: String, value: f64, timestamp: u64) -> Self {
        Self {
            key,
            value,
            timestamp,
            step: None,
        }
    }

    /// Create metric with step
    pub fn with_step(mut self, step: i64) -> Self {
        self.step = Some(step);
        self
    }
}

/// Artifact information
#[derive(Debug, Clone)]
pub struct Artifact {
    /// Artifact path
    pub path: String,
    /// Artifact type (e.g., "model", "dataset")
    pub artifact_type: String,
    /// File size in bytes
    pub size: Option<u64>,
    /// Checksum
    pub checksum: Option<String>,
    /// Is directory
    pub is_dir: bool,
}

impl Artifact {
    /// Create new artifact
    pub fn new(path: String, artifact_type: String) -> Self {
        Self {
            path,
            artifact_type,
            size: None,
            checksum: None,
            is_dir: false,
        }
    }
}

/// Model version information
#[derive(Debug, Clone)]
pub struct ModelVersion {
    /// Model name
    pub name: String,
    /// Version string
    pub version: String,
    /// Run ID that created this model
    pub run_id: String,
    /// Source (e.g., path to model file)
    pub source: String,
    /// Creation timestamp
    pub creation_time: u64,
    /// Current stage
    pub stage: ModelStage,
    /// Description
    pub description: Option<String>,
    /// Tags
    pub tags: Vec<(String, String)>,
}

impl ModelVersion {
    /// Create new model version
    pub fn new(name: String, version: String, run_id: String, source: String) -> Self {
        Self {
            name,
            version,
            run_id,
            source,
            creation_time: 0,
            stage: ModelStage::None,
            description: None,
            tags: Vec::new(),
        }
    }

    /// Transition to a new stage
    pub fn transition_stage(&mut self, new_stage: ModelStage) {
        self.stage = new_stage;
    }

    /// Add a tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.push((key, value));
    }
}

/// Model lifecycle stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStage {
    /// No stage
    None,
    /// Development stage
    Development,
    /// Staging stage
    Staging,
    /// Production stage
    Production,
    /// Archived
    Archived,
}

impl fmt::Display for ModelStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelStage::None => write!(f, "None"),
            ModelStage::Development => write!(f, "Development"),
            ModelStage::Staging => write!(f, "Staging"),
            ModelStage::Production => write!(f, "Production"),
            ModelStage::Archived => write!(f, "Archived"),
        }
    }
}

/// Experiment run
#[derive(Debug, Clone)]
pub struct ExperimentRun {
    /// Run metadata
    pub metadata: RunMetadata,
    /// Parameters
    pub params: BTreeMap<String, ParamValue>,
    /// Metrics
    pub metrics: BTreeMap<String, Vec<MetricData>>,
    /// Artifacts
    pub artifacts: Vec<Artifact>,
    /// Tags
    pub tags: Vec<(String, String)>,
}

impl ExperimentRun {
    /// Create new experiment run
    pub fn new(run_id: String, experiment_name: String) -> Self {
        Self {
            metadata: RunMetadata::new(run_id, experiment_name),
            params: BTreeMap::new(),
            metrics: BTreeMap::new(),
            artifacts: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Log a parameter
    pub fn log_param(&mut self, key: String, value: ParamValue) {
        self.params.insert(key, value);
    }

    /// Log a float parameter
    pub fn log_param_float(&mut self, key: String, value: f64) {
        self.log_param(key, ParamValue::Float(value));
    }

    /// Log an integer parameter
    pub fn log_param_int(&mut self, key: String, value: i64) {
        self.log_param(key, ParamValue::Integer(value));
    }

    /// Log a string parameter
    pub fn log_param_str(&mut self, key: String, value: String) {
        self.log_param(key, ParamValue::String(value));
    }

    /// Log a boolean parameter
    pub fn log_param_bool(&mut self, key: String, value: bool) {
        self.log_param(key, ParamValue::Boolean(value));
    }

    /// Log a metric
    pub fn log_metric(&mut self, metric: MetricData) {
        self.metrics
            .entry(metric.key.clone())
            .or_insert_with(Vec::new)
            .push(metric);
    }

    /// Log a metric value
    pub fn log_metric_value(&mut self, key: String, value: f64, timestamp: u64) {
        let metric = MetricData::new(key, value, timestamp);
        self.log_metric(metric);
    }

    /// Log a metric with step
    pub fn log_metric_step(&mut self, key: String, value: f64, timestamp: u64, step: i64) {
        let metric = MetricData::new(key, value, timestamp).with_step(step);
        self.log_metric(metric);
    }

    /// Log an artifact
    pub fn log_artifact(&mut self, artifact: Artifact) {
        self.artifacts.push(artifact);
    }

    /// Add a tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.push((key, value));
    }

    /// Get the latest value of a metric
    pub fn get_metric_latest(&self, key: &str) -> Option<f64> {
        self.metrics.get(key).and_then(|values| values.last().map(|m| m.value))
    }

    /// Get all values of a metric
    pub fn get_metric_history(&self, key: &str) -> Option<&[MetricData]> {
        self.metrics.get(key).map(|v| v.as_slice())
    }

    /// Get parameter value
    pub fn get_param(&self, key: &str) -> Option<&ParamValue> {
        self.params.get(key)
    }
}

/// Experiment definition
#[derive(Debug, Clone)]
pub struct Experiment {
    /// Experiment name
    pub name: String,
    /// Experiment ID
    pub experiment_id: String,
    /// Creation timestamp
    pub creation_time: u64,
    /// Last update timestamp
    pub last_update_time: u64,
    /// Lifecycle stage
    pub lifecycle_stage: ExperimentLifecycleStage,
    /// Tags
    pub tags: Vec<(String, String)>,
}

impl Experiment {
    /// Create new experiment
    pub fn new(name: String) -> Self {
        Self {
            name,
            experiment_id: generate_uuid(),
            creation_time: 0,
            last_update_time: 0,
            lifecycle_stage: ExperimentLifecycleStage::Active,
            tags: Vec::new(),
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.push((key, value));
    }
}

/// Experiment lifecycle stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentLifecycleStage {
    /// Active experiment
    Active,
    /// Deleted experiment
    Deleted,
}

/// Experiment tracker for managing experiments
pub struct ExperimentTracker {
    /// Storage backend
    storage: ExperimentStorage,
    /// Active run
    active_run: Option<ExperimentRun>,
}

impl ExperimentTracker {
    /// Create new experiment tracker
    pub fn new() -> Self {
        Self {
            storage: ExperimentStorage::new(),
            active_run: None,
        }
    }

    /// Create a new experiment
    pub fn create_experiment(&mut self, name: String) -> MLOpsResult<Experiment> {
        let experiment = Experiment::new(name);
        self.storage.save_experiment(experiment.clone())?;
        Ok(experiment)
    }

    /// Get an experiment by ID
    pub fn get_experiment(&self, experiment_id: &str) -> MLOpsResult<Experiment> {
        self.storage.get_experiment(experiment_id)
    }

    /// List all experiments
    pub fn list_experiments(&self) -> MLOpsResult<Vec<Experiment>> {
        self.storage.list_experiments()
    }

    /// Delete an experiment
    pub fn delete_experiment(&mut self, experiment_id: &str) -> MLOpsResult<()> {
        self.storage.delete_experiment(experiment_id)
    }

    /// Create a new run
    pub fn create_run(&mut self, experiment_name: String) -> MLOpsResult<String> {
        let run_id = generate_uuid();
        let run = ExperimentRun::new(run_id.clone(), experiment_name);
        self.storage.save_run(run.clone())?;

        self.active_run = Some(run);
        Ok(run_id)
    }

    /// Start a run
    pub fn start_run(&mut self, run_id: &str) -> MLOpsResult<()> {
        let timestamp = get_current_time_ns();
        self.storage.update_run_status(run_id, RunStatus::Running, timestamp)?;
        Ok(())
    }

    /// End a run successfully
    pub fn end_run(&mut self, run_id: &str) -> MLOpsResult<()> {
        let timestamp = get_current_time_ns();
        self.storage.update_run_status(run_id, RunStatus::Completed, timestamp)?;
        self.active_run = None;
        Ok(())
    }

    /// Fail a run
    pub fn fail_run(&mut self, run_id: &str) -> MLOpsResult<()> {
        let timestamp = get_current_time_ns();
        self.storage.update_run_status(run_id, RunStatus::Failed, timestamp)?;
        self.active_run = None;
        Ok(())
    }

    /// Log parameters for the active run
    pub fn log_params(&mut self, params: Vec<(String, ParamValue)>) -> MLOpsResult<()> {
        if let Some(ref mut run) = self.active_run {
            for (key, value) in params {
                run.log_param(key, value);
            }
            self.storage.update_run(run.clone())?;
            Ok(())
        } else {
            Err(ExperimentError::LoggingFailed(String::from(
                "No active run",
            ))
            .into())
        }
    }

    /// Log a single parameter
    pub fn log_param(&mut self, key: String, value: ParamValue) -> MLOpsResult<()> {
        if let Some(ref mut run) = self.active_run {
            run.log_param(key, value);
            self.storage.update_run(run.clone())?;
            Ok(())
        } else {
            Err(ExperimentError::LoggingFailed(String::from(
                "No active run",
            ))
            .into())
        }
    }

    /// Log metrics for the active run
    pub fn log_metrics(&mut self, metrics: Vec<MetricData>) -> MLOpsResult<()> {
        if let Some(ref mut run) = self.active_run {
            for metric in metrics {
                run.log_metric(metric);
            }
            self.storage.update_run(run.clone())?;
            Ok(())
        } else {
            Err(ExperimentError::LoggingFailed(String::from(
                "No active run",
            ))
            .into())
        }
    }

    /// Log a single metric
    pub fn log_metric(&mut self, key: String, value: f64, timestamp: u64) -> MLOpsResult<()> {
        if let Some(ref mut run) = self.active_run {
            run.log_metric_value(key, value, timestamp);
            self.storage.update_run(run.clone())?;
            Ok(())
        } else {
            Err(ExperimentError::LoggingFailed(String::from(
                "No active run",
            ))
            .into())
        }
    }

    /// Log an artifact
    pub fn log_artifact(&mut self, artifact: Artifact) -> MLOpsResult<()> {
        if let Some(ref mut run) = self.active_run {
            run.log_artifact(artifact);
            self.storage.update_run(run.clone())?;
            Ok(())
        } else {
            Err(ExperimentError::LoggingFailed(String::from(
                "No active run",
            ))
            .into())
        }
    }

    /// Register a model version
    pub fn log_model(&mut self, model: ModelVersion) -> MLOpsResult<()> {
        self.storage.save_model_version(model)
    }

    /// Get a model version
    pub fn get_model(&self, name: &str, version: &str) -> MLOpsResult<ModelVersion> {
        self.storage.get_model_version(name, version)
    }

    /// Transition model stage
    pub fn transition_model_stage(
        &mut self,
        name: &str,
        version: &str,
        new_stage: ModelStage,
    ) -> MLOpsResult<()> {
        self.storage.transition_model_stage(name, version, new_stage)
    }

    /// Get run by ID
    pub fn get_run(&self, run_id: &str) -> MLOpsResult<ExperimentRun> {
        self.storage.get_run(run_id)
    }

    /// List runs for an experiment
    pub fn list_runs(&self, experiment_name: &str) -> MLOpsResult<Vec<ExperimentRun>> {
        self.storage.list_runs(experiment_name)
    }

    /// Search runs
    pub fn search_runs(&self, filter: RunFilter) -> MLOpsResult<Vec<ExperimentRun>> {
        self.storage.search_runs(filter)
    }

    /// Compare runs
    pub fn compare_runs(&self, run_ids: Vec<String>) -> MLOpsResult<RunComparison> {
        let mut runs = Vec::new();
        for run_id in &run_ids {
            runs.push(self.get_run(run_id)?);
        }
        Ok(RunComparison::new(runs))
    }
}

/// Run filter for searching
#[derive(Debug, Clone, Default)]
pub struct RunFilter {
    /// Experiment name
    pub experiment_name: Option<String>,
    /// Run status
    pub status: Option<RunStatus>,
    /// Parameter filters
    pub params: Vec<(String, ParamValue)>,
    /// Metric filters (key, min_value, max_value)
    pub metrics: Vec<(String, Option<f64>, Option<f64>)>,
}

/// Comparison between runs
#[derive(Debug, Clone)]
pub struct RunComparison {
    /// Runs being compared
    pub runs: Vec<ExperimentRun>,
    /// Parameter differences
    pub param_diffs: Vec<(String, Vec<ParamValue>)>,
    /// Metric differences
    pub metric_diffs: Vec<(String, Vec<f64>)>,
}

impl RunComparison {
    /// Create new run comparison
    pub fn new(runs: Vec<ExperimentRun>) -> Self {
        // Collect all unique parameter keys
        let mut all_param_keys: Vec<String> = Vec::new();
        for run in &runs {
            for key in run.params.keys() {
                if !all_param_keys.contains(key) {
                    all_param_keys.push(key.clone());
                }
            }
        }

        // Collect parameter values for each key
        let mut param_diffs = Vec::new();
        for key in all_param_keys {
            let values = runs
                .iter()
                .filter_map(|run| run.params.get(&key).cloned())
                .collect();
            param_diffs.push((key, values));
        }

        // Collect all unique metric keys
        let mut all_metric_keys: Vec<String> = Vec::new();
        for run in &runs {
            for key in run.metrics.keys() {
                if !all_metric_keys.contains(key) {
                    all_metric_keys.push(key.clone());
                }
            }
        }

        // Collect latest metric values for each key
        let mut metric_diffs = Vec::new();
        for key in all_metric_keys {
            let values = runs
                .iter()
                .filter_map(|run| run.get_metric_latest(&key))
                .collect();
            metric_diffs.push((key, values));
        }

        Self {
            runs,
            param_diffs,
            metric_diffs,
        }
    }

    /// Get best run by metric
    pub fn get_best_run(&self, metric_key: &str, maximize: bool) -> Option<&ExperimentRun> {
        let mut best_run = None;
        let mut best_value = None;

        for run in &self.runs {
            if let Some(value) = run.get_metric_latest(metric_key) {
                if best_value.is_none()
                    || (maximize && value > best_value.unwrap())
                    || (!maximize && value < best_value.unwrap())
                {
                    best_value = Some(value);
                    best_run = Some(run);
                }
            }
        }

        best_run
    }
}

/// Experiment storage backend
#[derive(Debug, Clone)]
struct ExperimentStorage {
    /// Experiments indexed by ID
    experiments: Vec<Experiment>,
    /// Runs indexed by ID
    runs: Vec<ExperimentRun>,
    /// Model versions indexed by (name, version)
    models: Vec<(String, String, ModelVersion)>,
}

impl ExperimentStorage {
    /// Create new storage backend
    fn new() -> Self {
        Self {
            experiments: Vec::new(),
            runs: Vec::new(),
            models: Vec::new(),
        }
    }

    /// Save an experiment
    fn save_experiment(&mut self, experiment: Experiment) -> MLOpsResult<()> {
        // Check for duplicates
        if self
            .experiments
            .iter()
            .any(|e| e.experiment_id == experiment.experiment_id)
        {
            return Err(ExperimentError::VersionConflict(String::from(
                "Experiment already exists",
            ))
            .into());
        }

        self.experiments.push(experiment);
        Ok(())
    }

    /// Get an experiment
    fn get_experiment(&self, experiment_id: &str) -> MLOpsResult<Experiment> {
        self.experiments
            .iter()
            .find(|e| e.experiment_id == experiment_id)
            .cloned()
            .ok_or_else(|| {
                ExperimentError::ExperimentNotFound(experiment_id.to_string()).into()
            })
    }

    /// List all experiments
    fn list_experiments(&self) -> MLOpsResult<Vec<Experiment>> {
        Ok(self.experiments.clone())
    }

    /// Delete an experiment
    fn delete_experiment(&mut self, experiment_id: &str) -> MLOpsResult<()> {
        let pos = self
            .experiments
            .iter()
            .position(|e| e.experiment_id == experiment_id)
            .ok_or_else(|| ExperimentError::ExperimentNotFound(experiment_id.to_string()))?;

        self.experiments.remove(pos);
        Ok(())
    }

    /// Save a run
    fn save_run(&mut self, run: ExperimentRun) -> MLOpsResult<()> {
        // Check for duplicates
        if self.runs.iter().any(|r| r.metadata.run_id == run.metadata.run_id) {
            return Err(ExperimentError::VersionConflict(String::from(
                "Run already exists",
            ))
            .into());
        }

        self.runs.push(run);
        Ok(())
    }

    /// Update a run
    fn update_run(&mut self, run: ExperimentRun) -> MLOpsResult<()> {
        let pos = self
            .runs
            .iter()
            .position(|r| r.metadata.run_id == run.metadata.run_id)
            .ok_or_else(|| {
                MLOpsError::ExperimentError(ExperimentError::ExperimentNotFound(run.metadata.run_id.clone()))
            })?;

        self.runs[pos] = run;
        Ok(())
    }

    /// Update run status
    fn update_run_status(
        &mut self,
        run_id: &str,
        status: RunStatus,
        timestamp: u64,
    ) -> MLOpsResult<()> {
        let pos = self
            .runs
            .iter()
            .position(|r| r.metadata.run_id == run_id)
            .ok_or_else(|| MLOpsError::ExperimentError(ExperimentError::ExperimentNotFound(run_id.to_string())))?;

        match status {
            RunStatus::Running => {
                self.runs[pos].metadata.start(timestamp);
            }
            RunStatus::Completed => {
                self.runs[pos].metadata.complete(timestamp);
            }
            RunStatus::Failed => {
                self.runs[pos].metadata.fail(timestamp);
            }
            _ => {
                self.runs[pos].metadata.status = status;
            }
        }

        Ok(())
    }

    /// Get a run
    fn get_run(&self, run_id: &str) -> MLOpsResult<ExperimentRun> {
        self.runs
            .iter()
            .find(|r| r.metadata.run_id == run_id)
            .cloned()
            .ok_or_else(|| ExperimentError::ExperimentNotFound(run_id.to_string()).into())
    }

    /// List runs for an experiment
    fn list_runs(&self, experiment_name: &str) -> MLOpsResult<Vec<ExperimentRun>> {
        Ok(self
            .runs
            .iter()
            .filter(|r| r.metadata.experiment_name == experiment_name)
            .cloned()
            .collect())
    }

    /// Search runs with filters
    fn search_runs(&self, filter: RunFilter) -> MLOpsResult<Vec<ExperimentRun>> {
        let mut results = self.runs.clone();

        // Filter by experiment name
        if let Some(ref exp_name) = filter.experiment_name {
            results.retain(|r| r.metadata.experiment_name == *exp_name);
        }

        // Filter by status
        if let Some(status) = filter.status {
            results.retain(|r| r.metadata.status == status);
        }

        // Filter by parameters
        for (key, value) in &filter.params {
            results.retain(|r| r.params.get(key).map_or(false, |v| v == value));
        }

        // Filter by metrics
        for (key, min_val, max_val) in &filter.metrics {
            results.retain(|r| {
                if let Some(val) = r.get_metric_latest(key) {
                    if let Some(min) = min_val {
                        if val < *min {
                            return false;
                        }
                    }
                    if let Some(max) = max_val {
                        if val > *max {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            });
        }

        Ok(results)
    }

    /// Save a model version
    fn save_model_version(&mut self, model: ModelVersion) -> MLOpsResult<()> {
        // Remove existing version if present
        self.models
            .retain(|(n, v, _)| n != &model.name || v != &model.version);

        self.models.push((model.name.clone(), model.version.clone(), model));
        Ok(())
    }

    /// Get a model version
    fn get_model_version(&self, name: &str, version: &str) -> MLOpsResult<ModelVersion> {
        self.models
            .iter()
            .find(|(n, v, _)| n == &name && v == &version)
            .map(|(_, _, m)| m.clone())
            .ok_or_else(|| {
                ExperimentError::ExperimentNotFound(format!("{}:{}", name, version)).into()
            })
    }

    /// Transition model stage
    fn transition_model_stage(
        &mut self,
        name: &str,
        version: &str,
        new_stage: ModelStage,
    ) -> MLOpsResult<()> {
        let pos = self
            .models
            .iter()
            .position(|(n, v, _)| n == &name && v == &version)
            .ok_or_else(|| {
                MLOpsError::ExperimentError(ExperimentError::ExperimentNotFound(format!("{}:{}", name, version)))
            })?;

        self.models[pos].2.transition_stage(new_stage);
        Ok(())
    }
}

/// Generate a UUID (simplified implementation)
fn generate_uuid() -> String {
    use core::fmt::Write;
    let mut uuid = String::with_capacity(36);
    for i in 0..32 {
        if i == 8 || i == 12 || i == 16 || i == 20 {
            uuid.push('-');
        }
        // Simplified: use counter-based ID
        write!(&mut uuid, "{:02x}", i % 256).unwrap();
    }
    uuid
}

/// Get current time in nanoseconds (simplified)
fn get_current_time_ns() -> u64 {
    // In production, use actual time source
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_creation() {
        let exp = Experiment::new(String::from("test_exp"));
        assert_eq!(exp.name, "test_exp");
        assert!(!exp.experiment_id.is_empty());
    }

    #[test]
    fn test_run_creation() {
        let run = ExperimentRun::new(String::from("run_1"), String::from("test_exp"));
        assert_eq!(run.metadata.experiment_name, "test_exp");
        assert_eq!(run.metadata.run_id, "run_1");
    }

    #[test]
    fn test_log_params() {
        let mut run = ExperimentRun::new(String::from("run_1"), String::from("test_exp"));
        run.log_param_float(String::from("lr"), 0.001);
        run.log_param_int(String::from("batch_size"), 32);

        assert_eq!(
            run.get_param(&"lr"),
            Some(&ParamValue::Float(0.001))
        );
        assert_eq!(
            run.get_param(&"batch_size"),
            Some(&ParamValue::Integer(32))
        );
    }

    #[test]
    fn test_log_metrics() {
        let mut run = ExperimentRun::new(String::from("run_1"), String::from("test_exp"));
        run.log_metric_value(String::from("accuracy"), 0.95, 1000);
        run.log_metric_value(String::from("loss"), 0.05, 1000);

        assert_eq!(run.get_metric_latest("accuracy"), Some(0.95));
        assert_eq!(run.get_metric_latest("loss"), Some(0.05));
    }

    #[test]
    fn test_tracker() {
        let mut tracker = ExperimentTracker::new();
        let exp = tracker.create_experiment(String::from("test_exp")).unwrap();
        assert_eq!(exp.name, "test_exp");

        let run_id = tracker.create_run(String::from("test_exp")).unwrap();
        tracker
            .log_param(String::from("lr"), ParamValue::Float(0.001))
            .unwrap();

        let run = tracker.get_run(&run_id).unwrap();
        assert_eq!(run.metadata.experiment_name, "test_exp");
    }

    #[test]
    fn test_run_comparison() {
        let mut run1 = ExperimentRun::new(String::from("run_1"), String::from("test_exp"));
        run1.log_metric_value(String::from("accuracy"), 0.90, 1000);

        let mut run2 = ExperimentRun::new(String::from("run_2"), String::from("test_exp"));
        run2.log_metric_value(String::from("accuracy"), 0.95, 1000);

        let comparison = RunComparison::new(vec![run1.clone(), run2.clone()]);
        let best = comparison.get_best_run("accuracy", true).unwrap();

        assert_eq!(best.metadata.run_id, "run_2");
    }

    #[test]
    fn test_model_version() {
        let mut model = ModelVersion::new(
            String::from("test_model"),
            String::from("v1.0"),
            String::from("run_1"),
            String::from("/path/to/model"),
        );

        model.transition_stage(ModelStage::Production);
        assert_eq!(model.stage, ModelStage::Production);

        model.add_tag(String::from("architecture"), String::from("CNN"));
        assert_eq!(model.tags.len(), 1);
    }
}
