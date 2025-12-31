//! # AI/ML Framework for NOS Kernel
//!
//! A comprehensive artificial intelligence and machine learning framework designed for
//! kernel-level integration. Provides efficient tensor operations, neural network inference,
//! on-device training, model format support, hardware acceleration, and optimization techniques.
//!
//! ## Architecture
//!
//! The AI framework is organized into the following modules:
//!
//! - **tensor**: Multi-dimensional tensor data structures with efficient operations
//! - **neural**: Neural network layers and inference engine
//! - **training**: On-device training with backpropagation and optimizers
//! - **model**: Model format support (ONNX, TFLite, PyTorch)
//! - **accelerator**: Hardware acceleration interfaces (GPU, NPU, TPU)
//! - **optimization**: Model optimization and compression techniques
//!
//! ## Features
//!
//! - **Zero-copy operations**: Efficient tensor views without memory duplication
//! - **SIMD-friendly layouts**: Data layout optimized for vectorized operations
//! - **Edge AI**: Optimized for low-resource environments
//! - **Hardware acceleration**: Support for GPU, NPU, TPU with CPU fallback
//! - **Model optimization**: Quantization, pruning, compression, operator fusion
//! - **On-device training**: Backpropagation with various optimizers
//! - **Multiple formats**: ONNX, TensorFlow Lite, PyTorch support
//!
//! ## Design Goals
//!
//! 1. **Memory Efficiency**: Minimal allocation and copy overhead
//! 2. **Performance**: SIMD-optimized operations, parallel execution
//! 3. **Flexibility**: Support for multiple model architectures and formats
//! 4. **Edge-First**: Optimized for resource-constrained environments
//! 5. **Safety**: Rust's type system ensures memory safety
//!
//! ## Usage Examples
//!
//! ### Basic Tensor Operations
//!
//! ```no_run
//! use kernel::ai::tensor::Tensor;
//!
//! // Create tensors
//! let a = Tensor::<f32>::zeros(&[2, 3]);
//! let b = Tensor::<f32>::ones(&[2, 3]);
//! let c = &a + &b;
//!
//! // Matrix multiplication
//! let m1 = Tensor::<f32>::randn(&[3, 4]);
//! let m2 = Tensor::<f32>::randn(&[4, 5]);
//! let product = m1.matmul(&m2)?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```
//!
//! ### Neural Network Inference
//!
//! ```no_run
//! use kernel::ai::neural::{Dense, Activation, NeuralNetwork};
//!
//! // Create a simple network
//! let mut network = NeuralNetwork::new();
//! network.add_layer(Dense::new(784, 256));
//! network.add_layer(Activation::ReLU);
//! network.add_layer(Dense::new(256, 10));
//! network.add_layer(Activation::Softmax);
//!
//! // Run inference
//! let input = Tensor::randn(&[1, 784]);
//! let output = network.forward(&input)?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```
//!
//! ### Model Training
//!
//! ```no_run
//! use kernel::ai::training::{Trainer, Adam, MSELoss};
//! use kernel::ai::neural::NeuralNetwork;
//!
//! let mut network = NeuralNetwork::new();
//! // ... add layers ...
//!
//! let mut trainer = Trainer::new(network)
//!     .optimizer(Adam::new(0.001))
//!     .loss_function(MSELoss::new());
//!
//! // Train for 100 epochs
//! trainer.train(&train_data, &train_labels, 100)?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```
//!
//! ### Hardware Acceleration
//!
//! ```no_run
//! use kernel::ai::accelerator::{Device, DeviceType};
//!
//! // Select available device
//! let device = Device::select(DeviceType::GPU)?;
//!
//! // Run computation on device
//! let result = device.execute_compute(&kernel, &inputs)?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub mod tensor;
pub mod neural;
pub mod training;
pub mod model;
pub mod accelerator;
pub mod optimization;

pub use tensor::*;
pub use neural::*;
pub use training::*;
pub use model::*;
pub use accelerator::*;
pub use optimization::*;

/// AI framework version
pub const AI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type for AI operations
pub type AiResult<T> = core::result::Result<T, AiError>;

/// Errors in AI/ML operations
#[derive(Debug, Clone, PartialEq)]
pub enum AiError {
    /// Tensor operation error
    TensorError(TensorError),
    /// Neural network error
    NeuralError(NeuralError),
    /// Training error
    TrainingError(TrainingError),
    /// Model format error
    ModelError(ModelError),
    /// Acceleration error
    AcceleratorError(AcceleratorError),
    /// Optimization error
    OptimizationError(OptimizationError),
    /// Out of memory
    OutOfMemory,
    /// Invalid operation
    InvalidOperation(String),
    /// Not implemented
    NotImplemented(String),
    /// IO error during model loading
    IoError(String),
    /// Shape mismatch
    ShapeMismatch {
        expected: Vec<usize>,
        got: Vec<usize>,
    },
    /// Device not available
    DeviceNotAvailable(String),
}

impl core::fmt::Display for AiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AiError::TensorError(e) => write!(f, "Tensor error: {:?}", e),
            AiError::NeuralError(e) => write!(f, "Neural network error: {:?}", e),
            AiError::TrainingError(e) => write!(f, "Training error: {:?}", e),
            AiError::ModelError(e) => write!(f, "Model error: {:?}", e),
            AiError::AcceleratorError(e) => write!(f, "Accelerator error: {:?}", e),
            AiError::OptimizationError(e) => write!(f, "Optimization error: {:?}", e),
            AiError::OutOfMemory => write!(f, "Out of memory"),
            AiError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            AiError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            AiError::IoError(msg) => write!(f, "IO error: {}", msg),
            AiError::ShapeMismatch { expected, got } => {
                write!(f, "Shape mismatch: expected {:?}, got {:?}", expected, got)
            }
            AiError::DeviceNotAvailable(device) => {
                write!(f, "Device not available: {}", device)
            }
        }
    }
}

/// Tensor-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorError {
    /// Index out of bounds
    IndexOutOfBounds,
    /// Invalid shape
    InvalidShape(String),
    /// Type mismatch
    TypeMismatch,
    /// Allocation failed
    AllocationFailed,
    /// Operation not supported for this type
    UnsupportedOperation(String),
}

/// Neural network-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeuralError {
    /// Layer not found
    LayerNotFound(String),
    /// Invalid layer configuration
    InvalidLayerConfig(String),
    /// Forward pass failed
    ForwardPassFailed(String),
    /// Backward pass failed
    BackwardPassFailed(String),
    /// Invalid activation function
    InvalidActivation(String),
}

/// Training-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainingError {
    /// Gradient computation failed
    GradientError(String),
    /// Optimizer error
    OptimizerError(String),
    /// Loss computation failed
    LossError(String),
    /// Checkpoint save/load failed
    CheckpointError(String),
    /// Early stopping triggered
    EarlyStopping,
}

/// Model format-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// Unsupported format
    UnsupportedFormat(String),
    /// Parse error
    ParseError(String),
    /// Invalid version
    InvalidVersion(String),
    /// Missing metadata
    MissingMetadata(String),
    /// Architecture not supported
    ArchitectureNotSupported(String),
}

/// Acceleration-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceleratorError {
    /// Device initialization failed
    InitFailed(String),
    /// Memory transfer failed
    MemoryTransferFailed(String),
    /// Kernel execution failed
    KernelExecutionFailed(String),
    /// Compilation failed
    CompilationFailed(String),
    /// No device available
    NoDeviceAvailable,
    /// Driver not loaded
    DriverNotLoaded(String),
}

/// Optimization-specific errors
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationError {
    /// Quantization failed
    QuantizationError(String),
    /// Pruning failed
    PruningError(String),
    /// Compression failed
    CompressionError(String),
    /// Accuracy degradation too high
    AccuracyDegradation(f32),
    /// Invalid optimization parameters
    InvalidParameters(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AiError::ShapeMismatch {
            expected: vec![2, 3],
            got: vec![3, 2],
        };
        assert!(format!("{}", err).contains("Shape mismatch"));
    }

    #[test]
    fn test_not_implemented() {
        let err = AiError::NotImplemented(String::from("custom_op"));
        assert!(format!("{}", err).contains("custom_op"));
    }

    #[test]
    fn test_device_not_available() {
        let err = AiError::DeviceNotAvailable(String::from("NPU"));
        assert!(format!("{}", err).contains("NPU"));
    }
}
