//! # Machine Learning Operations (MLOps) for NOS Kernel
//!
//! This module provides comprehensive MLOps infrastructure for managing the complete
//! machine learning lifecycle within the kernel. It integrates seamlessly with the AI
//! framework to provide production-ready ML operations.
//!
//! ## Architecture
//!
//! The MLOps framework is organized into the following modules:
//!
//! - **experiment**: Experiment tracking, hyperparameter logging, model versioning
//! - **pipeline**: DAG-based pipelines with execution engine
//! - **serving**: Model serving infrastructure with batch/online prediction
//! - **monitoring**: Data drift detection, performance monitoring, alerting
//! - **retraining**: Automated retraining with hyperparameter optimization
//! - **governance**: Model lineage, fairness auditing, explainability, privacy compliance
//!
//! ## Features
//!
//! - **MLflow-Compatible**: Concepts compatible with MLflow for easy migration
//! - **Kernel-Level Integration**: Deep integration with kernel services
//! - **Zero-Copy Operations**: Efficient data handling with minimal overhead
//! - **Reproducibility**: Complete experiment tracking and version control
//! - **Scalability**: Built for production workloads
//! - **Safety**: Rust's type system ensures memory safety
//!
//! ## Usage Examples
//!
//! ### Experiment Tracking
//!
//! ```no_run
//! use kernel::mlops::experiment::{Experiment, ExperimentTracker};
//!
//! let mut tracker = ExperimentTracker::new();
//! let mut experiment = Experiment::new("image_classifier_v1");
//!
//! experiment.log_param("learning_rate", 0.001);
//! experiment.log_param("batch_size", 32);
//! experiment.log_metric("accuracy", 0.95);
//! experiment.log_metric("loss", 0.05);
//!
//! tracker.log_experiment(experiment)?;
//! # Ok::<(), kernel::mlops::MLOpsError>(())
//! ```
//!
//! ### Pipeline Execution
//!
//! ```no_run
//! use kernel::mlops::pipeline::{Pipeline, PipelineStep, PipelineOrchestrator};
//!
//! let mut pipeline = Pipeline::new("training_pipeline");
//!
//! pipeline.add_step(PipelineStep::new("data_load"));
//! pipeline.add_step(PipelineStep::new("preprocess").depends_on("data_load"));
//! pipeline.add_step(PipelineStep::new("train").depends_on("preprocess"));
//! pipeline.add_step(PipelineStep::new("evaluate").depends_on("train"));
//!
//! let orchestrator = PipelineOrchestrator::new();
//! orchestrator.execute_pipeline(&mut pipeline).await?;
//! # Ok::<(), kernel::mlops::MLOpsError>(())
//! ```
//!
//! ### Model Serving
//!
//! ```no_run
//! use kernel::mlops::serving::{ModelServer, PredictionRequest};
//!
//! let mut server = ModelServer::new();
//! server.load_model("model_v1", model_bytes).await?;
//!
//! let request = PredictionRequest::new(model_input);
//! let response = server.predict("model_v1", &request).await?;
//! # Ok::<(), kernel::mlops::MLOpsError>(())
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt;

pub mod experiment;
pub mod pipeline;
pub mod serving;
pub mod monitoring;
pub mod retraining;
pub mod governance;

/// Errors in MLOps operations
#[derive(Debug, Clone, PartialEq)]
pub enum MLOpsError {
    /// Experiment tracking error
    ExperimentError(ExperimentError),
    /// Pipeline execution error
    PipelineError(PipelineError),
    /// Model serving error
    ServingError(ServingError),
    /// Monitoring error
    MonitoringError(MonitoringError),
    /// Retraining error
    RetrainingError(RetrainingError),
    /// Governance error
    GovernanceError(GovernanceError),
    /// Storage error
    StorageError(String),
    /// Serialization error
    SerializationError(String),
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Resource not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
    /// Timeout
    Timeout(String),
    /// Not implemented
    NotImplemented(String),
}

pub use experiment::*;
pub use pipeline::*;
pub use serving::*;
pub use monitoring::*;
pub use retraining::*;
pub use governance::*;

/// MLOps framework version
pub const MLOPS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type for MLOps operations
pub type MLOpsResult<T> = core::result::Result<T, MLOpsError>;

impl fmt::Display for MLOpsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MLOpsError::ExperimentError(e) => write!(f, "Experiment error: {}", e),
            MLOpsError::PipelineError(e) => write!(f, "Pipeline error: {}", e),
            MLOpsError::ServingError(e) => write!(f, "Serving error: {}", e),
            MLOpsError::MonitoringError(e) => write!(f, "Monitoring error: {}", e),
            MLOpsError::RetrainingError(e) => write!(f, "Retraining error: {}", e),
            MLOpsError::GovernanceError(e) => write!(f, "Governance error: {}", e),
            MLOpsError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            MLOpsError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            MLOpsError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            MLOpsError::NotFound(msg) => write!(f, "Not found: {}", msg),
            MLOpsError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            MLOpsError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            MLOpsError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
        }
    }
}

/// Experiment tracking errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperimentError {
    /// Experiment not found
    ExperimentNotFound(String),
    /// Invalid experiment ID
    InvalidExperimentId(String),
    /// Logging failed
    LoggingFailed(String),
    /// Version conflict
    VersionConflict(String),
    /// Checkpoint error
    CheckpointError(String),
}

impl fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExperimentError::ExperimentNotFound(id) => write!(f, "Experiment not found: {}", id),
            ExperimentError::InvalidExperimentId(id) => write!(f, "Invalid experiment ID: {}", id),
            ExperimentError::LoggingFailed(msg) => write!(f, "Logging failed: {}", msg),
            ExperimentError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
            ExperimentError::CheckpointError(msg) => write!(f, "Checkpoint error: {}", msg),
        }
    }
}

/// Pipeline execution errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError {
    /// Step not found
    StepNotFound(String),
    /// Circular dependency detected
    CircularDependency(String),
    /// Execution failed
    ExecutionFailed(String),
    /// Invalid DAG
    InvalidDAG(String),
    /// Resource unavailable
    ResourceUnavailable(String),
    /// Timeout
    StepTimeout(String),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::StepNotFound(name) => write!(f, "Step not found: {}", name),
            PipelineError::CircularDependency(msg) => write!(f, "Circular dependency: {}", msg),
            PipelineError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            PipelineError::InvalidDAG(msg) => write!(f, "Invalid DAG: {}", msg),
            PipelineError::ResourceUnavailable(msg) => write!(f, "Resource unavailable: {}", msg),
            PipelineError::StepTimeout(name) => write!(f, "Step timeout: {}", name),
        }
    }
}

/// Model serving errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServingError {
    /// Model not found
    ModelNotFound(String),
    /// Model load failed
    ModelLoadFailed(String),
    /// Prediction failed
    PredictionFailed(String),
    /// Server overload
    ServerOverload,
    /// Invalid request
    InvalidRequest(String),
    /// Model version conflict
    VersionConflict(String),
}

impl fmt::Display for ServingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServingError::ModelNotFound(name) => write!(f, "Model not found: {}", name),
            ServingError::ModelLoadFailed(msg) => write!(f, "Model load failed: {}", msg),
            ServingError::PredictionFailed(msg) => write!(f, "Prediction failed: {}", msg),
            ServingError::ServerOverload => write!(f, "Server overload"),
            ServingError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            ServingError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
        }
    }
}

/// Monitoring errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitoringError {
    /// Metric collection failed
    MetricCollectionFailed(String),
    /// Drift detection failed
    DriftDetectionFailed(String),
    /// Alert trigger failed
    AlertTriggerFailed(String),
    /// Invalid threshold
    InvalidThreshold(String),
    /// Data quality check failed
    DataQualityCheckFailed(String),
}

impl fmt::Display for MonitoringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonitoringError::MetricCollectionFailed(msg) => {
                write!(f, "Metric collection failed: {}", msg)
            }
            MonitoringError::DriftDetectionFailed(msg) => {
                write!(f, "Drift detection failed: {}", msg)
            }
            MonitoringError::AlertTriggerFailed(msg) => write!(f, "Alert trigger failed: {}", msg),
            MonitoringError::InvalidThreshold(msg) => write!(f, "Invalid threshold: {}", msg),
            MonitoringError::DataQualityCheckFailed(msg) => {
                write!(f, "Data quality check failed: {}", msg)
            }
        }
    }
}

/// Retraining errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrainingError {
    /// Training failed
    TrainingFailed(String),
    /// Hyperparameter optimization failed
    OptimizationFailed(String),
    /// Insufficient data
    InsufficientData(String),
    /// Evaluation failed
    EvaluationFailed(String),
    /// Deployment failed
    DeploymentFailed(String),
}

impl fmt::Display for RetrainingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetrainingError::TrainingFailed(msg) => write!(f, "Training failed: {}", msg),
            RetrainingError::OptimizationFailed(msg) => write!(f, "Optimization failed: {}", msg),
            RetrainingError::InsufficientData(msg) => write!(f, "Insufficient data: {}", msg),
            RetrainingError::EvaluationFailed(msg) => write!(f, "Evaluation failed: {}", msg),
            RetrainingError::DeploymentFailed(msg) => write!(f, "Deployment failed: {}", msg),
        }
    }
}

/// Governance errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceError {
    /// Lineage tracking failed
    LineageTrackingFailed(String),
    /// Fairness audit failed
    FairnessAuditFailed(String),
    /// Explainability generation failed
    ExplainabilityFailed(String),
    /// Privacy check failed
    PrivacyCheckFailed(String),
    /// Compliance violation
    ComplianceViolation(String),
}

impl fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GovernanceError::LineageTrackingFailed(msg) => {
                write!(f, "Lineage tracking failed: {}", msg)
            }
            GovernanceError::FairnessAuditFailed(msg) => write!(f, "Fairness audit failed: {}", msg),
            GovernanceError::ExplainabilityFailed(msg) => {
                write!(f, "Explainability failed: {}", msg)
            }
            GovernanceError::PrivacyCheckFailed(msg) => write!(f, "Privacy check failed: {}", msg),
            GovernanceError::ComplianceViolation(msg) => write!(f, "Compliance violation: {}", msg),
        }
    }
}

/// Convert from specific error types to MLOpsError
impl From<ExperimentError> for MLOpsError {
    fn from(err: ExperimentError) -> Self {
        MLOpsError::ExperimentError(err)
    }
}

impl From<PipelineError> for MLOpsError {
    fn from(err: PipelineError) -> Self {
        MLOpsError::PipelineError(err)
    }
}

impl From<ServingError> for MLOpsError {
    fn from(err: ServingError) -> Self {
        MLOpsError::ServingError(err)
    }
}

impl From<MonitoringError> for MLOpsError {
    fn from(err: MonitoringError) -> Self {
        MLOpsError::MonitoringError(err)
    }
}

impl From<RetrainingError> for MLOpsError {
    fn from(err: RetrainingError) -> Self {
        MLOpsError::RetrainingError(err)
    }
}

impl From<GovernanceError> for MLOpsError {
    fn from(err: GovernanceError) -> Self {
        MLOpsError::GovernanceError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MLOpsError::ExperimentError(ExperimentError::ExperimentNotFound(String::from("exp_1")));
        assert!(format!("{}", err).contains("Experiment not found"));
    }

    #[test]
    fn test_not_implemented() {
        let err = MLOpsError::NotImplemented(String::from("feature"));
        assert!(format!("{}", err).contains("Not implemented"));
    }

    #[test]
    fn test_error_conversion() {
        let exp_err = ExperimentError::InvalidExperimentId(String::from("bad_id"));
        let mlops_err: MLOpsError = exp_err.into();
        assert!(matches!(mlops_err, MLOpsError::ExperimentError(_)));
    }
}
