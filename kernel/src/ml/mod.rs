//! # Machine Learning Module
//!
//! This module provides comprehensive machine learning capabilities for the NOS kernel,
//! integrating both the original ML optimization features and the new ML/AI Integration Track.
//!
//! ## Sub-modules
//!
//! - **Original ML**: Prediction, optimization, anomaly detection, adaptive tuning
//! - **New ML/AI Track**: Inference, accelerators, neural networks, optimizers, pipelines
//!
//! ## Architecture
//!
//! The ML module is organized into:
//! - **ML System**: Original optimization and prediction system
//! - **Inference Engine**: Model inference and tensor operations
//! - **Accelerator Support**: GPU, NPU, TPU integration
//! - **Neural Networks**: Layer implementations and training
//! - **Optimizers**: SGD, Adam, learning rate scheduling
//! - **Data Pipeline**: Preprocessing, augmentation, batching
//! - **Framework Facade**: Unified API and model management

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::error::unified::{MlError, UnifiedError};
use crate::sync::{Mutex, RwLock};

// ============================================================================
// Original ML Sub-modules
// ============================================================================

pub mod prediction;
pub mod optimization;
pub mod anomaly;
pub mod adaptive;

// Use the existing inference module structure
pub use inference::{Tensor, TensorDType, TensorShape};

// ============================================================================
// New ML/AI Integration Track
// ============================================================================

/// Model inference engine
pub mod inference;

/// Hardware accelerator support
pub mod accelerator;

/// Neural network primitives
pub mod nn;

/// Optimization algorithms
pub mod optimizer;

/// ML data pipeline
pub mod pipeline;

// Re-export key types from new modules
pub use inference::{
    InferenceEngine, ModelFormat, ModelId, TensorBatch, InferenceStats,
    load_model, unload_model, infer, batch_infer,
};

pub use accelerator::{
    AccelId, AcceleratorInfo, AccelType, DeviceMemory, Kernel, Completion,
    AcceleratorInterface, MemAllocFlags, discover_accelerators, allocate_memory,
};

pub use nn::{
    LayerId, LayerType, LayerConfig, LayerParams, ActivationType,
    create_layer, get_layer, forward, backward, apply_activation, quantize_weights,
};

pub use optimizer::{
    OptimizerId, OptimizerType, OptimizerConfig, LrScheduleType, LrSchedulerConfig,
    create_optimizer, step, set_lr,
};

pub use pipeline::{
    DataSample, DataBatch, PipelineConfig, PipelineStats,
    DataSourceType, DataSourceConfig, PreprocessorConfig, AugmenterConfig, BatcherConfig,
    create_pipeline,
};

// ============================================================================
// ML Framework Facade
// ============================================================================

/// Model registry entry
#[derive(Debug, Clone)]
pub struct ModelEntry {
    /// Model ID
    pub id: ModelId,
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model format
    pub format: ModelFormat,
    /// Is model active
    pub active: bool,
    /// Model metadata
    pub metadata: ModelMetadata,
}

/// Model metadata
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    /// Model author
    pub author: Option<String>,
    /// Model description
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
    /// Model tags
    pub tags: Vec<String>,
    /// Model size in bytes
    pub size_bytes: usize,
    /// Input shapes
    pub input_shapes: Vec<Vec<usize>>,
    /// Output shapes
    pub output_shapes: Vec<Vec<usize>>,
}

/// Model compression type
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    /// No compression
    None,
    /// Quantization to INT8
    QuantizeInt8,
    /// Quantization to INT4
    QuantizeInt4,
    /// Pruning
    Pruning,
    /// Knowledge distillation
    Distillation,
}

/// A/B test configuration
#[derive(Debug, Clone)]
pub struct ABTestConfig {
    /// Test name
    pub name: String,
    /// Control model ID
    pub control_model: ModelId,
    /// Treatment model IDs
    pub treatment_models: Vec<ModelId>,
    /// Traffic split (percentage for control)
    pub traffic_split: f32,
    /// Success metrics
    pub metrics: Vec<String>,
}

/// A/B test status
#[derive(Debug, Clone)]
pub struct ABTestStatus {
    /// Test name
    pub name: String,
    /// Is test active
    pub active: bool,
    /// Requests to control
    pub control_requests: u64,
    /// Requests to treatments
    pub treatment_requests: Vec<u64>,
    /// Control performance metrics
    pub control_metrics: BTreeMap<String, f64>,
    /// Treatment performance metrics
    pub treatment_metrics: Vec<BTreeMap<String, f64>>,
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of loaded models
    pub max_models: usize,
    /// Maximum memory for ML operations (bytes)
    pub max_memory: usize,
    /// Default device for inference
    pub default_device: Option<AccelId>,
    /// Enable profiling
    pub enable_profiling: bool,
    /// Enable model caching
    pub enable_caching: bool,
    /// Cache size (bytes)
    pub cache_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_models: 100,
            max_memory: 8 * 1024 * 1024 * 1024, // 8GB
            default_device: None,
            enable_profiling: false,
            enable_caching: true,
            cache_size: 1 * 1024 * 1024 * 1024, // 1GB
        }
    }
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total inference count
    pub total_inferences: u64,
    /// Total inference time (nanoseconds)
    pub total_inference_time: u64,
    /// Average inference time (microseconds)
    pub avg_inference_time_us: f64,
    /// P95 inference time (microseconds)
    pub p95_inference_time_us: f64,
    /// P99 inference time (microseconds)
    pub p99_inference_time_us: f64,
    /// Throughput (inferences per second)
    pub throughput: f64,
    /// Memory usage (bytes)
    pub memory_usage: usize,
    /// GPU utilization (percentage)
    pub gpu_utilization: f32,
    /// CPU utilization (percentage)
    pub cpu_utilization: f32,
}

/// Model registry
pub struct ModelRegistry {
    /// Registered models
    models: Arc<RwLock<BTreeMap<ModelId, ModelEntry>>>,
    /// Name to model ID mapping
    name_index: Arc<RwLock<BTreeMap<String, ModelId>>>,
    /// Next model ID
    next_id: Arc<AtomicU64>,
    /// Statistics
    stats: Arc<Mutex<RegistryStats>>,
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
struct RegistryStats {
    total_models: usize,
    active_models: usize,
    total_size_bytes: usize,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(BTreeMap::new())),
            name_index: Arc::new(RwLock::new(BTreeMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            stats: Arc::new(Mutex::new(RegistryStats::default())),
        }
    }

    /// Register a model
    pub fn register_model(&self, entry: ModelEntry) -> Result<(), MlError> {
        let id = entry.id;
        let name = entry.name.clone();
        let size = entry.metadata.size_bytes;

        // Insert into registry
        {
            let mut models = self.models.write();
            models.insert(id, entry.clone());
        }

        // Update name index
        {
            let mut name_index = self.name_index.write();
            name_index.insert(name, id);
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_models += 1;
            stats.total_size_bytes += size;
        }

        Ok(())
    }

    /// Unregister a model
    pub fn unregister_model(&self, id: ModelId) -> Result<(), MlError> {
        let entry = {
            let mut models = self.models.write();
            models.remove(&id).ok_or(MlError::ModelNotFound)?
        };

        // Remove from name index
        let mut name_index = self.name_index.write();
        name_index.remove(&entry.name);

        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_models -= 1;
        stats.total_size_bytes -= entry.metadata.size_bytes;

        Ok(())
    }

    /// Get a model by ID
    pub fn get_model(&self, id: ModelId) -> Result<ModelEntry, MlError> {
        let models = self.models.read();
        models.get(&id).cloned().ok_or(MlError::ModelNotFound)
    }

    /// Get a model by name
    pub fn get_model_by_name(&self, name: &str) -> Result<ModelEntry, MlError> {
        let name_index = self.name_index.read();
        let id = name_index.get(name).ok_or(MlError::ModelNotFound)?;
        self.get_model(*id)
    }

    /// List all models
    pub fn list_models(&self) -> Vec<ModelEntry> {
        let models = self.models.read();
        models.values().cloned().collect()
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> RegistryStats {
        self.stats.lock().clone()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// ML system statistics
#[derive(Debug, Clone)]
pub struct MlStatistics {
    /// Total number of models
    pub total_models: usize,
    /// Number of active models
    pub active_models: usize,
    /// Total memory used (bytes)
    pub total_memory_bytes: usize,
    /// Total inferences performed
    pub total_inferences: u64,
    /// Average inference time (microseconds)
    pub avg_inference_time_us: f64,
    /// Current throughput (inferences/second)
    pub throughput: f64,
}

/// Global ML framework
static GLOBAL_FRAMEWORK: Mutex<Option<Arc<MlFramework>>> = Mutex::new(None);

/// ML framework facade
pub struct MlFramework {
    /// Runtime configuration
    config: Arc<RwLock<RuntimeConfig>>,
    /// Model registry
    registry: Arc<ModelRegistry>,
    /// Is initialized
    initialized: Arc<AtomicBool>,
}

impl MlFramework {
    /// Create a new ML framework
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(RuntimeConfig::default())),
            registry: Arc::new(ModelRegistry::new()),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the ML framework
    pub fn init(&self) -> Result<(), MlError> {
        // Initialize sub-modules
        inference::init();
        accelerator::init();
        nn::init();
        optimizer::init();
        pipeline::init();

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Get model registry
    pub fn get_registry(&self) -> Arc<ModelRegistry> {
        self.registry.clone()
    }

    /// Export statistics
    pub fn export_statistics(&self) -> MlStatistics {
        let registry_stats = self.registry.get_stats();

        MlStatistics {
            total_models: registry_stats.total_models,
            active_models: registry_stats.active_models,
            total_memory_bytes: registry_stats.total_size_bytes,
            total_inferences: 0,
            avg_inference_time_us: 0.0,
            throughput: 0.0,
        }
    }
}

impl Default for MlFramework {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the global ML framework
pub fn init_framework() -> Result<(), MlError> {
    let framework = Arc::new(MlFramework::new());
    framework.init()?;
    *GLOBAL_FRAMEWORK.lock() = Some(framework);
    Ok(())
}

/// Get the global ML framework
pub fn get_framework() -> Result<Arc<MlFramework>, MlError> {
    GLOBAL_FRAMEWORK
        .lock()
        .as_ref()
        .cloned()
        .ok_or(MlError::InferenceError("Framework not initialized".to_string()))
}

// ============================================================================
// Original ML System (Preserved for backward compatibility)
// ============================================================================

/// Machine learning system for kernel optimization
pub struct MLSystem {
    prediction_engine: Option<prediction::PredictionEngine>,
    optimization_engine: Option<optimization::OptimizationEngine>,
    anomaly_detector: Option<anomaly::AnomalyDetector>,
    adaptive_tuner: Option<adaptive::AdaptiveTuner>,
    stats: Mutex<MLStats>,
    active: Mutex<bool>,
}

impl MLSystem {
    /// Create a new machine learning system
    pub fn new() -> Result<Self, UnifiedError> {
        Ok(Self {
            prediction_engine: None,
            optimization_engine: None,
            anomaly_detector: None,
            adaptive_tuner: None,
            stats: Mutex::new(MLStats::default()),
            active: Mutex::new(false),
        })
    }

    /// Initialize the machine learning system
    pub fn initialize(&self) -> Result<(), UnifiedError> {
        let mut active = self.active.lock();
        if *active {
            return Ok(()); // Already initialized
        }
        *active = true;
        Ok(())
    }

    /// Shutdown the machine learning system
    pub fn shutdown(&self) -> Result<(), UnifiedError> {
        let mut active = self.active.lock();
        *active = false;
        Ok(())
    }

    /// Check if ML system is active
    pub fn is_active(&self) -> bool {
        *self.active.lock()
    }

    /// Get ML system statistics
    pub fn get_stats(&self) -> MLStats {
        self.stats.lock().clone()
    }
}

/// ML system statistics
#[derive(Debug, Clone, Default)]
pub struct MLStats {
    /// Models trained
    pub models_trained: u64,
    /// Predictions made
    pub predictions_made: u64,
    /// Optimizations performed
    pub optimizations_performed: u64,
    /// Anomaly checks
    pub anomaly_checks: u64,
    /// Adaptive tunings
    pub adaptive_tunings: u64,
}

/// Global ML system instance (original)
static mut ML_SYSTEM: Option<MLSystem> = None;
static ML_INIT: spin::Once = spin::Once::new();

/// Initialize global ML system (original)
pub fn init_ml() -> Result<(), UnifiedError> {
    ML_INIT.call_once(|| {
        match MLSystem::new() {
            Ok(system) => {
                if let Err(e) = system.initialize() {
                    log::error!("Failed to initialize ML system: {}", e);
                    return;
                }
                unsafe {
                    ML_SYSTEM = Some(system);
                }
                log::info!("Global ML system initialized");
            }
            Err(e) => {
                log::error!("Failed to create ML system: {}", e);
            }
        }
    });
    Ok(())
}

/// Get global ML system (original)
pub fn get_ml_system() -> Option<&'static MLSystem> {
    unsafe { ML_SYSTEM.as_ref() }
}

/// Check if ML features are available
pub fn is_ml_available() -> bool {
    if let Some(system) = get_ml_system() {
        system.is_active()
    } else {
        false
    }
}

/// Initialize ML subsystem (unified)
pub fn init() -> Result<(), UnifiedError> {
    log::info!("Initializing ML subsystem");

    // Initialize original ML system
    init_ml()?;

    // Initialize new ML/AI framework
    if let Err(e) = init_framework() {
        log::warn!("Failed to initialize ML framework: {:?}", e);
    }

    log::info!("ML subsystem initialized");
    Ok(())
}

/// Shutdown ML subsystem
pub fn shutdown() -> Result<(), UnifiedError> {
    log::info!("Shutting down ML subsystem");
    log::info!("ML subsystem shutdown complete");
    Ok(())
}
