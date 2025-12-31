//! # Computation Graph
//!
//! Computation graph representation for neural network models with:
//! - Node-based computation graph
//! - Topological sorting
//! - Dependency analysis
//! - Memory optimization

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::ml::inference::tensor::Tensor;

/// Unique node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

impl NodeId {
    pub fn generate() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst))
    }
}

/// Computation operator types
#[derive(Debug, Clone, PartialEq)]
pub enum OperatorType {
    // Neural network layers
    Conv2D {
        kernel_size: (usize, usize),
        strides: (usize, usize),
        padding: (usize, usize),
        dilation: (usize, usize),
        groups: usize,
    },
    Dense {
        input_dim: usize,
        output_dim: usize,
    },
    MaxPool2D {
        kernel_size: (usize, usize),
        strides: (usize, usize),
        padding: (usize, usize),
    },
    AvgPool2D {
        kernel_size: (usize, usize),
        strides: (usize, usize),
        padding: (usize, usize),
    },
    GlobalAvgPool2D,

    // Activation functions
    Relu,
    Sigmoid,
    Tanh,
    Softmax,
    LeakyRelu { alpha: f32 },
    ELU { alpha: f32 },

    // Normalization
    BatchNorm,
    LayerNorm,
    InstanceNorm,

    // Tensor operations
    Reshape,
    Flatten,
    Transpose,
    Concat,
    Split,
    Slice,
    Gather,
    Scatter,

    // Element-wise operations
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Sqrt,
    Abs,
    Neg,

    // Matrix operations
    MatMul,
    BatchMatMul,

    // Reduction operations
    ReduceSum,
    ReduceMean,
    ReduceMax,
    ReduceMin,

    // Shape manipulation
    Expand,
    Squeeze,
    Unsqueeze,
    Pad,

    // Custom operator
    Custom(String),
}

impl OperatorType {
    pub fn name(&self) -> &str {
        match self {
            OperatorType::Conv2D { .. } => "Conv2D",
            OperatorType::Dense { .. } => "Dense",
            OperatorType::MaxPool2D { .. } => "MaxPool2D",
            OperatorType::AvgPool2D { .. } => "AvgPool2D",
            OperatorType::GlobalAvgPool2D => "GlobalAvgPool2D",
            OperatorType::Relu => "Relu",
            OperatorType::Sigmoid => "Sigmoid",
            OperatorType::Tanh => "Tanh",
            OperatorType::Softmax => "Softmax",
            OperatorType::LeakyRelu { .. } => "LeakyRelu",
            OperatorType::ELU { .. } => "ELU",
            OperatorType::BatchNorm => "BatchNorm",
            OperatorType::LayerNorm => "LayerNorm",
            OperatorType::InstanceNorm => "InstanceNorm",
            OperatorType::Reshape => "Reshape",
            OperatorType::Flatten => "Flatten",
            OperatorType::Transpose => "Transpose",
            OperatorType::Concat => "Concat",
            OperatorType::Split => "Split",
            OperatorType::Slice => "Slice",
            OperatorType::Gather => "Gather",
            OperatorType::Scatter => "Scatter",
            OperatorType::Add => "Add",
            OperatorType::Sub => "Sub",
            OperatorType::Mul => "Mul",
            OperatorType::Div => "Div",
            OperatorType::Pow => "Pow",
            OperatorType::Sqrt => "Sqrt",
            OperatorType::Abs => "Abs",
            OperatorType::Neg => "Neg",
            OperatorType::MatMul => "MatMul",
            OperatorType::BatchMatMul => "BatchMatMul",
            OperatorType::ReduceSum => "ReduceSum",
            OperatorType::ReduceMean => "ReduceMean",
            OperatorType::ReduceMax => "ReduceMax",
            OperatorType::ReduceMin => "ReduceMin",
            OperatorType::Expand => "Expand",
            OperatorType::Squeeze => "Squeeze",
            OperatorType::Unsqueeze => "Unsqueeze",
            OperatorType::Pad => "Pad",
            OperatorType::Custom(name) => name,
        }
    }
}

/// Computation node in the graph
pub struct ComputeNode {
    id: NodeId,
    op_type: OperatorType,
    inputs: Vec<NodeId>,
    outputs: Vec<NodeId>,
    attributes: BTreeMap<String, Attribute>,
    tensor: Option<Tensor>,
}

/// Node attribute value
#[derive(Debug, Clone)]
pub enum Attribute {
    Float(f32),
    Floats(Vec<f32>),
    Int(i64),
    Ints(Vec<i64>),
    String(String),
    Strings(Vec<String>),
    Bool(bool),
    Tensor(Tensor),
}

impl ComputeNode {
    /// Create a new computation node
    pub fn new(op_type: OperatorType) -> Self {
        Self {
            id: NodeId::generate(),
            op_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
            attributes: BTreeMap::new(),
            tensor: None,
        }
    }

    /// Get node ID
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get operator type
    pub fn op_type(&self) -> &OperatorType {
        &self.op_type
    }

    /// Get input nodes
    pub fn inputs(&self) -> &[NodeId] {
        &self.inputs
    }

    /// Get output nodes
    pub fn outputs(&self) -> &[NodeId] {
        &self.outputs
    }

    /// Add input node
    pub fn add_input(&mut self, input: NodeId) {
        if !self.inputs.contains(&input) {
            self.inputs.push(input);
        }
    }

    /// Add output node
    pub fn add_output(&mut self, output: NodeId) {
        if !self.outputs.contains(&output) {
            self.outputs.push(output);
        }
    }

    /// Set node attribute
    pub fn set_attribute(&mut self, name: String, value: Attribute) {
        self.attributes.insert(name, value);
    }

    /// Get node attribute
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }

    /// Bind tensor to node (for weights/biases)
    pub fn bind_tensor(&mut self, tensor: Tensor) {
        self.tensor = Some(tensor);
    }

    /// Get bound tensor
    pub fn tensor(&self) -> Option<&Tensor> {
        self.tensor.as_ref()
    }

    /// Check if node has inputs
    pub fn has_inputs(&self) -> bool {
        !self.inputs.is_empty()
    }

    /// Check if node has outputs
    pub fn has_outputs(&self) -> bool {
        !self.outputs.is_empty()
    }

    /// Get number of inputs
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    /// Get number of outputs
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }
}

/// Computation graph
pub struct ComputeGraph {
    nodes: BTreeMap<NodeId, ComputeNode>,
    inputs: Vec<NodeId>,
    outputs: Vec<NodeId>,
    name: String,
}

impl ComputeGraph {
    /// Create a new computation graph
    pub fn new(name: String) -> Self {
        Self {
            nodes: BTreeMap::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            name,
        }
    }

    /// Get graph name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add node to graph
    pub fn add_node(&mut self, node: ComputeNode) -> NodeId {
        let id = node.id();
        self.nodes.insert(id, node);
        id
    }

    /// Get node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&ComputeNode> {
        self.nodes.get(&id)
    }

    /// Get mutable node by ID
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut ComputeNode> {
        self.nodes.get_mut(&id)
    }

    /// Remove node from graph
    pub fn remove_node(&mut self, id: NodeId) -> Option<ComputeNode> {
        let node = self.nodes.remove(&id)?;

        // Remove from inputs/outputs
        self.inputs.retain(|&x| x != id);
        self.outputs.retain(|&x| x != id);

        // Update connections
        for node in self.nodes.values_mut() {
            node.inputs.retain(|&x| x != id);
            node.outputs.retain(|&x| x != id);
        }

        Some(node)
    }

    /// Set graph inputs
    pub fn set_inputs(&mut self, inputs: Vec<NodeId>) {
        self.inputs = inputs;
    }

    /// Set graph outputs
    pub fn set_outputs(&mut self, outputs: Vec<NodeId>) {
        self.outputs = outputs;
    }

    /// Get graph inputs
    pub fn inputs(&self) -> &[NodeId] {
        &self.inputs
    }

    /// Get graph outputs
    pub fn outputs(&self) -> &[NodeId] {
        &self.outputs
    }

    /// Connect two nodes
    pub fn connect(&mut self, from: NodeId, to: NodeId) -> bool {
        if let (Some(from_node), Some(to_node)) = (
            self.get_node_mut(from),
            self.get_node_mut(to),
        ) {
            from_node.add_output(to);
            to_node.add_input(from);
            true
        } else {
            false
        }
    }

    /// Get all nodes
    pub fn nodes(&self) -> &BTreeMap<NodeId, ComputeNode> {
        &self.nodes
    }

    /// Get number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Perform topological sort
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, GraphError> {
        let mut sorted = Vec::with_capacity(self.nodes.len());
        let mut visited = alloc::collections::BTreeSet::new();
        let mut temp_visited = alloc::collections::BTreeSet::new();

        for &input in &self.inputs {
            self.visit_node(input, &mut sorted, &mut visited, &mut temp_visited)?;
        }

        // Visit remaining unvisited nodes
        for &id in self.nodes.keys() {
            if !visited.contains(&id) {
                self.visit_node(id, &mut sorted, &mut visited, &mut temp_visited)?;
            }
        }

        Ok(sorted)
    }

    fn visit_node(
        &self,
        node_id: NodeId,
        sorted: &mut Vec<NodeId>,
        visited: &mut alloc::collections::BTreeSet<NodeId>,
        temp_visited: &mut alloc::collections::BTreeSet<NodeId>,
    ) -> Result<(), GraphError> {
        if temp_visited.contains(&node_id) {
            return Err(GraphError::Cycle(format!("Cycle detected at node {:?}", node_id)));
        }

        if visited.contains(&node_id) {
            return Ok(());
        }

        temp_visited.insert(node_id);

        if let Some(node) = self.get_node(node_id) {
            for &input in &node.inputs {
                self.visit_node(input, sorted, visited, temp_visited)?;
            }
        }

        temp_visited.remove(&node_id);
        visited.insert(node_id);
        sorted.push(node_id);

        Ok(())
    }

    /// Validate graph structure
    pub fn validate(&self) -> Result<(), GraphError> {
        // Check that all inputs exist
        for &input in &self.inputs {
            if !self.nodes.contains_key(&input) {
                return Err(GraphError::InvalidInput(format!("Input node {:?} not found", input)));
            }
        }

        // Check that all outputs exist
        for &output in &self.outputs {
            if !self.nodes.contains_key(&output) {
                return Err(GraphError::InvalidOutput(format!("Output node {:?} not found", output)));
            }
        }

        // Check for cycles
        self.topological_sort()?;

        // Validate connections
        for (&id, node) in &self.nodes {
            for &input in &node.inputs {
                if !self.nodes.contains_key(&input) {
                    return Err(GraphError::InvalidConnection(
                        format!("Node {:?} has invalid input {:?}", id, input)
                    ));
                }
            }

            for &output in &node.outputs {
                if !self.nodes.contains_key(&output) {
                    return Err(GraphError::InvalidConnection(
                        format!("Node {:?} has invalid output {:?}", id, output)
                    ));
                }
            }
        }

        Ok(())
    }

    /// Clone graph
    pub fn clone_graph(&self) -> ComputeGraph {
        let mut new_graph = ComputeGraph::new(self.name.clone());

        // Clone nodes
        for (&id, node) in &self.nodes {
            let mut new_node = ComputeNode::new(node.op_type().clone());
            new_node.inputs = node.inputs().to_vec();
            new_node.outputs = node.outputs().to_vec();
            new_node.attributes = node.attributes.clone();
            new_node.tensor = node.tensor().map(|t| t.to_owned());
            new_graph.nodes.insert(id, new_node);
        }

        new_graph.inputs = self.inputs.clone();
        new_graph.outputs = self.outputs.clone();

        new_graph
    }
}

/// Graph errors
#[derive(Debug, Clone)]
pub enum GraphError {
    Cycle(String),
    InvalidInput(String),
    InvalidOutput(String),
    InvalidConnection(String),
    EmptyGraph,
    NoOutputs,
}

impl core::fmt::Display for GraphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GraphError::Cycle(msg) => write!(f, "Graph cycle: {}", msg),
            GraphError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            GraphError::InvalidOutput(msg) => write!(f, "Invalid output: {}", msg),
            GraphError::InvalidConnection(msg) => write!(f, "Invalid connection: {}", msg),
            GraphError::EmptyGraph => write!(f, "Empty graph"),
            GraphError::NoOutputs => write!(f, "Graph has no outputs"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = ComputeGraph::new("test".to_string());
        assert_eq!(graph.name(), "test");
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = ComputeGraph::new("test".to_string());
        let node = ComputeNode::new(OperatorType::Relu);
        let id = graph.add_node(node);
        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node(id).is_some());
    }

    #[test]
    fn test_connect_nodes() {
        let mut graph = ComputeGraph::new("test".to_string());
        let node1 = ComputeNode::new(OperatorType::Relu);
        let node2 = ComputeNode::new(OperatorType::Sigmoid);

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        assert!(graph.connect(id1, id2));

        let node1 = graph.get_node(id1).unwrap();
        assert_eq!(node1.outputs(), &[id2]);

        let node2 = graph.get_node(id2).unwrap();
        assert_eq!(node2.inputs(), &[id1]);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ComputeGraph::new("test".to_string());

        let input = ComputeNode::new(OperatorType::Relu);
        let hidden = ComputeNode::new(OperatorType::Dense {
            input_dim: 10,
            output_dim: 5,
        });
        let output = ComputeNode::new(OperatorType::Softmax);

        let id_input = graph.add_node(input);
        let id_hidden = graph.add_node(hidden);
        let id_output = graph.add_node(output);

        graph.connect(id_input, id_hidden);
        graph.connect(id_hidden, id_output);

        graph.set_inputs(vec![id_input]);
        graph.set_outputs(vec![id_output]);

        let sorted = graph.topological_sort();
        assert!(sorted.is_ok());
        let sorted = sorted.unwrap();
        assert_eq!(sorted, vec![id_input, id_hidden, id_output]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = ComputeGraph::new("test".to_string());

        let node1 = ComputeNode::new(OperatorType::Relu);
        let node2 = ComputeNode::new(OperatorType::Sigmoid);

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        graph.connect(id1, id2);
        graph.connect(id2, id1); // Create cycle

        let result = graph.topological_sort();
        assert!(result.is_err());
    }
}
