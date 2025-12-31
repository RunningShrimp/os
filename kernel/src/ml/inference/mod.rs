//! # Machine Learning Inference Engine
//!
//! This module provides a comprehensive ML inference framework for kernel-space
//! machine learning operations, including:
//!
//! - Neural network runtime and execution engine
//! - Hardware-accelerated tensor operations
//! - Model quantization and optimization
//! - ONNX model support
//! - Edge inference optimization
//! - Security and privacy protection

pub mod runtime;
pub mod accel;
pub mod quantize;
pub mod onnx;
pub mod edge;
pub mod security;
pub mod tensor;
pub mod graph;
pub mod memory;

pub use runtime::{
    InferenceRuntime, GraphExecutor, TensorPool,
    ExecutionConfig, AsyncExecutor, ComputeGraph,
};
pub use accel::{
    TensorAccelerator, AcceleratorType, SimdBackend,
    GpuBackend, NpuBackend, AcceleratorConfig,
};
pub use quantize::{
    Quantizer, QuantizationType, QuantizationConfig,
    QuantizedTensor, CalibrationData,
};
pub use onnx::{
    OnnxModel, OnnxParser, OnnxOperator,
    StandardOperators, DynamicShapeHandler,
};
pub use edge::{
    EdgeOptimizer, ModelPruner, KnowledgeDistillation,
    PipelinePartitioner, BatchOptimizer,
};
pub use security::{
    MlSecurityFramework, ModelEncryption, TeeIntegration,
    AdversarialDetector, FederatedLearningInterface,
    DifferentialPrivacy,
};

// Re-export common types
pub use tensor::Tensor;
pub use graph::ComputeNode;
pub use memory::MemoryPool;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_init() {
        // Basic initialization test
        let _config = ExecutionConfig::default();
    }
}
