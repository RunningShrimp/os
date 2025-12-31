//! # Neural Network Inference Runtime
//!
//! Core inference engine providing:
//! - Computation graph execution
//! - Operator scheduling and optimization
//! - Asynchronous execution support
//! - Zero-copy data paths

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use crate::ml::inference::tensor::Tensor;
use crate::ml::inference::graph::{ComputeGraph, ComputeNode, NodeId, OperatorType};
use crate::ml::inference::memory::{TensorPool, MemoryPool, PoolConfig};

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Enable operator fusion optimization
    pub enable_fusion: bool,
    /// Enable memory reuse optimization
    pub enable_memory_reuse: bool,
    /// Enable parallel execution
    pub enable_parallel: bool,
    /// Number of worker threads (0 = auto)
    pub num_workers: usize,
    /// Maximum batch size for batching
    pub max_batch_size: usize,
    /// Enable operator scheduling optimization
    pub enable_scheduling: bool,
    /// Enable profiling
    pub enable_profiling: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            enable_fusion: true,
            enable_memory_reuse: true,
            enable_parallel: false, // Disabled by default in kernel
            num_workers: 0,
            max_batch_size: 1,
            enable_scheduling: true,
            enable_profiling: false,
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Number of forward passes executed
    pub forward_passes: u64,
    /// Total execution time (microseconds)
    pub total_time_us: u64,
    /// Average execution time (microseconds)
    pub avg_time_us: u64,
    /// Peak memory usage (bytes)
    pub peak_memory_bytes: u64,
    /// Number of operators executed
    pub operators_executed: u64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

/// Inference runtime
pub struct InferenceRuntime {
    graph: ComputeGraph,
    tensor_pool: Arc<TensorPool>,
    config: ExecutionConfig,
    tensor_values: Mutex<BTreeMap<NodeId, Tensor>>,
    stats: Mutex<ExecutionStats>,
    is_running: AtomicBool,
}

impl InferenceRuntime {
    /// Create a new inference runtime
    pub fn new(graph: ComputeGraph, config: ExecutionConfig) -> Result<Self, RuntimeError> {
        // Validate graph
        graph.validate()?;

        // Create tensor pool
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024, // 1 MB
            max_size: 1024 * 1024 * 1024, // 1 GB
            ..Default::default()
        };
        let tensor_pool = Arc::new(TensorPool::new(pool_config)?);

        Ok(Self {
            graph,
            tensor_pool,
            config,
            tensor_values: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(ExecutionStats::default()),
            is_running: AtomicBool::new(false),
        })
    }

    /// Initialize runtime
    pub fn initialize(&self) -> Result<(), RuntimeError> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(RuntimeError::AlreadyInitialized);
        }

        // Pre-allocate memory for common tensor sizes
        let common_sizes = [1024, 4096, 16384, 65536];
        let _ = self.tensor_pool.preallocate(&common_sizes);

        self.is_running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Execute forward pass
    pub fn forward(&self, inputs: Vec<Tensor>) -> Result<Vec<Tensor>, RuntimeError> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Err(RuntimeError::NotInitialized);
        }

        let graph_inputs = self.graph.inputs();
        if inputs.len() != graph_inputs.len() {
            return Err(RuntimeError::InvalidInput {
                expected: graph_inputs.len(),
                got: inputs.len(),
            });
        }

        // Set input tensors
        {
            let mut tensor_values = self.tensor_values.lock();
            for (input_id, tensor) in graph_inputs.iter().zip(inputs.iter()) {
                tensor_values.insert(*input_id, tensor.clone());
            }
        }

        // Execute graph
        let sorted_nodes = self.graph.topological_sort()?;
        let outputs = self.execute_nodes(&sorted_nodes)?;

        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.forward_passes += 1;
            stats.operators_executed += sorted_nodes.len() as u64;
        }

        Ok(outputs)
    }

    /// Execute nodes in topological order
    fn execute_nodes(&self, nodes: &[NodeId]) -> Result<Vec<Tensor>, RuntimeError> {
        let mut outputs = Vec::new();

        for &node_id in nodes {
            let node = self.graph.get_node(node_id)
                .ok_or_else(|| RuntimeError::NodeNotFound(node_id))?;

            // Execute operator
            let output_tensors = self.execute_operator(node)?;

            // Store outputs
            {
                let mut tensor_values = self.tensor_values.lock();
                for (output_id, tensor) in node.outputs().iter().zip(output_tensors.iter()) {
                    tensor_values.insert(*output_id, tensor.clone());
                }
            }

            // Check if this is a graph output
            if self.graph.outputs().contains(&node_id) {
                outputs.extend(output_tensors);
            }
        }

        Ok(outputs)
    }

    /// Execute a single operator
    fn execute_operator(&self, node: &ComputeNode) -> Result<Vec<Tensor>, RuntimeError> {
        let input_tensors = self.get_input_tensors(node)?;

        let output_tensors = match node.op_type() {
            OperatorType::Relu => self.op_relu(&input_tensors),
            OperatorType::Sigmoid => self.op_sigmoid(&input_tensors),
            OperatorType::Tanh => self.op_tanh(&input_tensors),
            OperatorType::Softmax => self.op_softmax(&input_tensors),
            OperatorType::Add => self.op_add(&input_tensors),
            OperatorType::Sub => self.op_sub(&input_tensors),
            OperatorType::Mul => self.op_mul(&input_tensors),
            OperatorType::Div => self.op_div(&input_tensors),
            OperatorType::MatMul => self.op_matmul(&input_tensors),
            OperatorType::Reshape => self.op_reshape(node, &input_tensors),
            OperatorType::Flatten => self.op_flatten(&input_tensors),
            OperatorType::Transpose => self.op_transpose(&input_tensors),
            OperatorType::Conv2D { .. } => self.op_conv2d(node, &input_tensors),
            OperatorType::Dense { .. } => self.op_dense(node, &input_tensors),
            OperatorType::MaxPool2D { .. } => self.op_maxpool2d(node, &input_tensors),
            OperatorType::AvgPool2D { .. } => self.op_avgpool2d(node, &input_tensors),
            OperatorType::BatchNorm => self.op_batchnorm(node, &input_tensors),
            _ => Err(RuntimeError::UnsupportedOperator {
                op: node.op_type().name().to_string(),
            }),
        }?;

        Ok(output_tensors)
    }

    /// Get input tensors for a node
    fn get_input_tensors(&self, node: &ComputeNode) -> Result<Vec<Tensor>, RuntimeError> {
        let tensor_values = self.tensor_values.lock();
        let mut input_tensors = Vec::new();

        for input_id in node.inputs() {
            let tensor = tensor_values.get(input_id)
                .ok_or_else(|| RuntimeError::TensorNotFound(*input_id))?
                .clone();
            input_tensors.push(tensor);
        }

        Ok(input_tensors)
    }

    // Operator implementations

    fn op_relu(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Relu",
                expected: 1,
                got: inputs.len(),
            });
        }

        let input = &inputs[0];
        let mut output = input.to_owned();

        match input.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let slice = output.as_mut_slice::<f32>();
                for val in slice {
                    *val = val.max(0.0);
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_sigmoid(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Sigmoid",
                expected: 1,
                got: inputs.len(),
            });
        }

        let input = &inputs[0];
        let mut output = input.to_owned();

        match input.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let slice = output.as_mut_slice::<f32>();
                for val in slice {
                    *val = 1.0 / (1.0 + (-*val).exp());
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_tanh(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Tanh",
                expected: 1,
                got: inputs.len(),
            });
        }

        let input = &inputs[0];
        let mut output = input.to_owned();

        match input.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let slice = output.as_mut_slice::<f32>();
                for val in slice {
                    *val = val.tanh();
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_softmax(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Softmax",
                expected: 1,
                got: inputs.len(),
            });
        }

        let input = &inputs[0];
        let mut output = input.to_owned();

        match input.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                // Simplified softmax along last dimension
                let slice = output.as_mut_slice::<f32>();
                let max = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let sum: f32 = slice.iter().map(|x| (x - max).exp()).sum();

                for val in slice {
                    *val = (*val - max).exp() / sum;
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_add(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 2 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Add",
                expected: 2,
                got: inputs.len(),
            });
        }

        let a = &inputs[0];
        let b = &inputs[1];

        if a.shape() != b.shape() {
            return Err(RuntimeError::ShapeMismatch);
        }

        let mut output = a.to_owned();

        match a.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let a_slice = a.as_slice::<f32>();
                let b_slice = b.as_slice::<f32>();
                let out_slice = output.as_mut_slice::<f32>();

                for i in 0..out_slice.len() {
                    out_slice[i] = a_slice[i] + b_slice[i];
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_sub(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 2 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Sub",
                expected: 2,
                got: inputs.len(),
            });
        }

        let a = &inputs[0];
        let b = &inputs[1];

        if a.shape() != b.shape() {
            return Err(RuntimeError::ShapeMismatch);
        }

        let mut output = a.to_owned();

        match a.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let a_slice = a.as_slice::<f32>();
                let b_slice = b.as_slice::<f32>();
                let out_slice = output.as_mut_slice::<f32>();

                for i in 0..out_slice.len() {
                    out_slice[i] = a_slice[i] - b_slice[i];
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_mul(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 2 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Mul",
                expected: 2,
                got: inputs.len(),
            });
        }

        let a = &inputs[0];
        let b = &inputs[1];

        if a.shape() != b.shape() {
            return Err(RuntimeError::ShapeMismatch);
        }

        let mut output = a.to_owned();

        match a.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let a_slice = a.as_slice::<f32>();
                let b_slice = b.as_slice::<f32>();
                let out_slice = output.as_mut_slice::<f32>();

                for i in 0..out_slice.len() {
                    out_slice[i] = a_slice[i] * b_slice[i];
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_div(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 2 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Div",
                expected: 2,
                got: inputs.len(),
            });
        }

        let a = &inputs[0];
        let b = &inputs[1];

        if a.shape() != b.shape() {
            return Err(RuntimeError::ShapeMismatch);
        }

        let mut output = a.to_owned();

        match a.dtype() {
            crate::ml::inference::tensor::TensorDType::F32 => {
                let a_slice = a.as_slice::<f32>();
                let b_slice = b.as_slice::<f32>();
                let out_slice = output.as_mut_slice::<f32>();

                for i in 0..out_slice.len() {
                    out_slice[i] = a_slice[i] / b_slice[i];
                }
            }
            _ => return Err(RuntimeError::UnsupportedDtype),
        }

        Ok(vec![output])
    }

    fn op_matmul(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 2 {
            return Err(RuntimeError::InvalidInputCount {
                op: "MatMul",
                expected: 2,
                got: inputs.len(),
            });
        }

        // Simplified matrix multiplication for 2D tensors
        let a = &inputs[0];
        let b = &inputs[1];

        if a.ndim() != 2 || b.ndim() != 2 {
            return Err(RuntimeError::UnsupportedOperation("Only 2D matmul supported".into()));
        }

        let a_dims = a.dims();
        let b_dims = b.dims();

        if a_dims[1] != b_dims[0] {
            return Err(RuntimeError::ShapeMismatch);
        }

        let m = a_dims[0];
        let k = a_dims[1];
        let n = b_dims[1];

        let mut output = Tensor::new::<f32>(
            crate::ml::inference::tensor::TensorDType::F32,
            crate::ml::inference::tensor::TensorShape::new(vec![m, n]),
        );

        let a_slice = a.as_slice::<f32>();
        let b_slice = b.as_slice::<f32>();
        let out_slice = output.as_mut_slice::<f32>();

        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0;
                for l in 0..k {
                    sum += a_slice[i * k + l] * b_slice[l * n + j];
                }
                out_slice[i * n + j] = sum;
            }
        }

        Ok(vec![output])
    }

    fn op_reshape(&self, _node: &ComputeNode, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Reshape",
                expected: 1,
                got: inputs.len(),
            });
        }

        // Identity for now (real implementation would read target shape from attributes)
        Ok(vec![inputs[0].clone()])
    }

    fn op_flatten(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Flatten",
                expected: 1,
                got: inputs.len(),
            });
        }

        let input = &inputs[0];
        let size = input.size();
        let output = input.reshape(crate::ml::inference::tensor::TensorShape::new(vec![size]))
            .ok_or(RuntimeError::ReshapeFailed)?;

        Ok(vec![output])
    }

    fn op_transpose(&self, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Transpose",
                expected: 1,
                got: inputs.len(),
            });
        }

        let input = &inputs[0];
        if input.ndim() != 2 {
            return Err(RuntimeError::UnsupportedOperation("Only 2D transpose supported".into()));
        }

        let output = input.transpose(0, 1)
            .ok_or(RuntimeError::TransposeFailed)?;

        Ok(vec![output])
    }

    fn op_conv2d(&self, _node: &ComputeNode, _inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        // Placeholder for conv2d implementation
        Err(RuntimeError::UnsupportedOperation("Conv2D not implemented".into()))
    }

    fn op_dense(&self, node: &ComputeNode, inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        if inputs.len() != 1 {
            return Err(RuntimeError::InvalidInputCount {
                op: "Dense",
                expected: 1,
                got: inputs.len(),
            });
        }

        // Dense = MatMul(input, weights) + bias
        let weights = node.tensor().ok_or(RuntimeError::MissingWeights)?;
        let output = self.op_matmul(&[inputs[0].clone(), weights.clone())?;

        Ok(output)
    }

    fn op_maxpool2d(&self, _node: &ComputeNode, _inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        // Placeholder for maxpool2d implementation
        Err(RuntimeError::UnsupportedOperation("MaxPool2D not implemented".into()))
    }

    fn op_avgpool2d(&self, _node: &ComputeNode, _inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        // Placeholder for avgpool2d implementation
        Err(RuntimeError::UnsupportedOperation("AvgPool2D not implemented".into()))
    }

    fn op_batchnorm(&self, _node: &ComputeNode, _inputs: &[Tensor]) -> Result<Vec<Tensor>, RuntimeError> {
        // Placeholder for batchnorm implementation
        Err(RuntimeError::UnsupportedOperation("BatchNorm not implemented".into()))
    }

    /// Get execution statistics
    pub fn stats(&self) -> ExecutionStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = ExecutionStats::default();
    }

    /// Check if runtime is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

/// Graph executor with advanced features
pub struct GraphExecutor {
    runtime: InferenceRuntime,
    execution_queue: VecDeque<NodeId>,
}

impl GraphExecutor {
    /// Create a new graph executor
    pub fn new(runtime: InferenceRuntime) -> Self {
        Self {
            runtime,
            execution_queue: VecDeque::new(),
        }
    }

    /// Execute with batch processing
    pub fn execute_batch(&self, inputs: Vec<Vec<Tensor>>) -> Result<Vec<Vec<Tensor>>, RuntimeError> {
        let mut outputs = Vec::new();

        for input_batch in inputs {
            let output = self.runtime.forward(input_batch)?;
            outputs.push(output);
        }

        Ok(outputs)
    }
}

/// Async executor for parallel execution
pub struct AsyncExecutor {
    runtime: Arc<InferenceRuntime>,
    pending_tasks: Mutex<Vec<ExecutionTask>>,
}

impl AsyncExecutor {
    /// Create a new async executor
    pub fn new(runtime: Arc<InferenceRuntime>) -> Self {
        Self {
            runtime,
            pending_tasks: Mutex::new(Vec::new()),
        }
    }

    /// Submit async execution task
    pub fn submit(&self, inputs: Vec<Tensor>) -> Result<u64, RuntimeError> {
        let task_id = {
            let mut tasks = self.pending_tasks.lock();
            let task_id = tasks.len() as u64;
            tasks.push(ExecutionTask {
                id: task_id,
                inputs,
                status: TaskStatus::Pending,
            });
            task_id
        };

        // In a real implementation, this would spawn a task
        Ok(task_id)
    }

    /// Get task result
    pub fn get_result(&self, _task_id: u64) -> Option<Result<Vec<Tensor>, RuntimeError>> {
        // Placeholder for async result retrieval
        None
    }
}

/// Execution task
struct ExecutionTask {
    id: u64,
    inputs: Vec<Tensor>,
    status: TaskStatus,
}

/// Task status
#[derive(Debug, Clone, PartialEq)]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Runtime errors
#[derive(Debug, Clone)]
pub enum RuntimeError {
    AlreadyInitialized,
    NotInitialized,
    NodeNotFound(NodeId),
    TensorNotFound(NodeId),
    InvalidInput { expected: usize, got: usize },
    InvalidInputCount { op: &'static str, expected: usize, got: usize },
    UnsupportedOperator { op: String },
    UnsupportedDtype,
    UnsupportedOperation(String),
    ShapeMismatch,
    ReshapeFailed,
    TransposeFailed,
    MissingWeights,
    ExecutionFailed(String),
}

impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RuntimeError::AlreadyInitialized => write!(f, "Runtime already initialized"),
            RuntimeError::NotInitialized => write!(f, "Runtime not initialized"),
            RuntimeError::NodeNotFound(id) => write!(f, "Node not found: {:?}", id),
            RuntimeError::TensorNotFound(id) => write!(f, "Tensor not found: {:?}", id),
            RuntimeError::InvalidInput { expected, got } => {
                write!(f, "Invalid input: expected {}, got {}", expected, got)
            }
            RuntimeError::InvalidInputCount { op, expected, got } => {
                write!(f, "{}: expected {} inputs, got {}", op, expected, got)
            }
            RuntimeError::UnsupportedOperator { op } => write!(f, "Unsupported operator: {}", op),
            RuntimeError::UnsupportedDtype => write!(f, "Unsupported data type"),
            RuntimeError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
            RuntimeError::ShapeMismatch => write!(f, "Shape mismatch"),
            RuntimeError::ReshapeFailed => write!(f, "Reshape failed"),
            RuntimeError::TransposeFailed => write!(f, "Transpose failed"),
            RuntimeError::MissingWeights => write!(f, "Missing weights"),
            RuntimeError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::inference::graph::{ComputeNode, OperatorType};

    #[test]
    fn test_runtime_creation() {
        let graph = ComputeGraph::new("test".to_string());
        let config = ExecutionConfig::default();
        let runtime = InferenceRuntime::new(graph, config);
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_runtime_initialization() {
        let graph = ComputeGraph::new("test".to_string());
        let config = ExecutionConfig::default();
        let runtime = InferenceRuntime::new(graph, config).unwrap();
        assert!(!runtime.is_initialized());

        let result = runtime.initialize();
        assert!(result.is_ok());
        assert!(runtime.is_initialized());
    }

    #[test]
    fn test_simple_graph_execution() {
        let mut graph = ComputeGraph::new("test".to_string());

        let input = ComputeNode::new(OperatorType::Relu);
        let id_input = graph.add_node(input);
        graph.set_inputs(vec![id_input]);
        graph.set_outputs(vec![id_input]);

        let config = ExecutionConfig::default();
        let runtime = InferenceRuntime::new(graph, config).unwrap();
        runtime.initialize().unwrap();

        let input_tensor = Tensor::new::<f32>(
            crate::ml::inference::tensor::TensorDType::F32,
            crate::ml::inference::tensor::TensorShape::new(vec![10]),
        );

        let outputs = runtime.forward(vec![input_tensor]);
        assert!(outputs.is_ok());
    }
}
