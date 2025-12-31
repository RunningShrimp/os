//! DAG (Directed Acyclic Graph) representation for workflow execution
//!
//! This module provides a comprehensive DAG implementation for managing
//! task dependencies and execution ordering in workflows.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// Unique identifier for a node in the DAG
pub type NodeId = u64;

/// Unique identifier for an edge in the DAG
pub type EdgeId = u64;

/// A node in the DAG representing a task or operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Unique identifier
    pub id: NodeId,
    /// Human-readable name
    pub name: String,
    /// Task type identifier
    pub task_type: String,
    /// Optional metadata
    pub metadata: BTreeMap<String, String>,
    /// Priority for execution (higher = more important)
    pub priority: i32,
}

impl Node {
    /// Create a new node
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            task_type: "default".to_string(),
            metadata: BTreeMap::new(),
            priority: 0,
        }
    }

    /// Set the task type
    pub fn with_task_type(mut self, task_type: impl Into<String>) -> Self {
        self.task_type = task_type.into();
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node(id={}, name={}, type={}, priority={})",
            self.id, self.name, self.task_type, self.priority)
    }
}

/// An edge in the DAG representing a dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// Unique identifier
    pub id: EdgeId,
    /// Source node ID
    pub from: NodeId,
    /// Target node ID
    pub to: NodeId,
    /// Dependency type
    pub dependency_type: DependencyType,
    /// Optional condition
    pub condition: Option<String>,
}

/// Type of dependency between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Standard dependency (execute after)
    Sequential,
    /// Weak dependency (prefer to execute after)
    Weak,
    /// Conditional dependency (execute after if condition met)
    Conditional,
}

impl Edge {
    /// Create a new edge
    pub fn new(id: EdgeId, from: NodeId, to: NodeId) -> Self {
        Self {
            id,
            from,
            to,
            dependency_type: DependencyType::Sequential,
            condition: None,
        }
    }

    /// Set the dependency type
    pub fn with_type(mut self, dep_type: DependencyType) -> Self {
        self.dependency_type = dep_type;
        self
    }

    /// Set a condition
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }
}

/// Errors that can occur during DAG operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagError {
    /// Node not found
    NodeNotFound(NodeId),
    /// Edge not found
    EdgeNotFound(EdgeId),
    /// Cycle detected in the graph
    CycleDetected(Vec<NodeId>),
    /// Invalid edge (self-loop, etc.)
    InvalidEdge(String),
    /// Dependency not satisfied
    DependencyNotSatisfied(NodeId),
    /// Graph is empty
    EmptyGraph,
}

impl fmt::Display for DagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::EdgeNotFound(id) => write!(f, "Edge not found: {}", id),
            Self::CycleDetected(cycle) => write!(f, "Cycle detected: {:?}", cycle),
            Self::InvalidEdge(msg) => write!(f, "Invalid edge: {}", msg),
            Self::DependencyNotSatisfied(id) => write!(f, "Dependency not satisfied: {}", id),
            Self::EmptyGraph => write!(f, "Graph is empty"),
        }
    }
}

/// Result type for DAG operations
pub type DagResult<T> = Result<T, DagError>;

/// Directed Acyclic Graph for workflow execution
#[derive(Clone)]
pub struct Dag {
    /// All nodes in the graph
    nodes: BTreeMap<NodeId, Node>,
    /// All edges in the graph
    edges: BTreeMap<EdgeId, Edge>,
    /// Adjacency list: node -> outgoing edges
    adjacency: BTreeMap<NodeId, BTreeSet<EdgeId>>,
    /// Reverse adjacency: node -> incoming edges
    reverse_adjacency: BTreeMap<NodeId, BTreeSet<EdgeId>>,
    /// Next node ID
    next_node_id: NodeId,
    /// Next edge ID
    next_edge_id: EdgeId,
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

impl Dag {
    /// Create a new empty DAG
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            reverse_adjacency: BTreeMap::new(),
            next_node_id: 1,
            next_edge_id: 1,
        }
    }

    /// Create a DAG with capacity pre-allocated
    pub fn with_capacity(_nodes: usize, _edges: usize) -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            adjacency: BTreeMap::new(),
            reverse_adjacency: BTreeMap::new(),
            next_node_id: 1,
            next_edge_id: 1,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, name: impl Into<String>) -> DagResult<NodeId> {
        let node = Node::new(self.next_node_id, name);
        let id = node.id;
        self.nodes.insert(id, node);
        self.adjacency.insert(id, BTreeSet::new());
        self.reverse_adjacency.insert(id, BTreeSet::new());
        self.next_node_id += 1;
        Ok(id)
    }

    /// Add a pre-configured node to the graph
    pub fn add_node_with(&mut self, node: Node) -> DagResult<NodeId> {
        let id = node.id;
        if self.nodes.contains_key(&id) {
            return Err(DagError::InvalidEdge(format!("Node {} already exists", id)));
        }
        self.nodes.insert(id, node);
        self.adjacency.insert(id, BTreeSet::new());
        self.reverse_adjacency.insert(id, BTreeSet::new());
        if id >= self.next_node_id {
            self.next_node_id = id + 1;
        }
        Ok(id)
    }

    /// Remove a node from the graph
    pub fn remove_node(&mut self, id: NodeId) -> DagResult<()> {
        if !self.nodes.contains_key(&id) {
            return Err(DagError::NodeNotFound(id));
        }

        // Remove all edges connected to this node
        let outgoing = self.adjacency.get(&id).cloned().unwrap_or_default();
        let incoming = self.reverse_adjacency.get(&id).cloned().unwrap_or_default();

        for edge_id in outgoing {
            self.remove_edge(edge_id)?;
        }
        for edge_id in incoming {
            self.remove_edge(edge_id)?;
        }

        self.nodes.remove(&id);
        self.adjacency.remove(&id);
        self.reverse_adjacency.remove(&id);

        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> DagResult<&Node> {
        self.nodes.get(&id).ok_or(DagError::NodeNotFound(id))
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, id: NodeId) -> DagResult<&mut Node> {
        self.nodes.get_mut(&id).ok_or(DagError::NodeNotFound(id))
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Get all node IDs
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Add an edge between two nodes
    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        dep_type: DependencyType,
    ) -> DagResult<EdgeId> {
        // Validate nodes exist
        if !self.nodes.contains_key(&from) {
            return Err(DagError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(DagError::NodeNotFound(to));
        }

        // Prevent self-loops
        if from == to {
            return Err(DagError::InvalidEdge("Self-loops are not allowed".to_string()));
        }

        let edge = Edge::new(self.next_edge_id, from, to)
            .with_type(dep_type);
        let edge_id = edge.id;

        self.edges.insert(edge_id, edge);
        self.adjacency.get_mut(&from).unwrap().insert(edge_id);
        self.reverse_adjacency.get_mut(&to).unwrap().insert(edge_id);
        self.next_edge_id += 1;

        // Check for cycles
        if let Some(cycle) = self.detect_cycle() {
            // Remove the edge we just added
            self.edges.remove(&edge_id);
            self.adjacency.get_mut(&from).unwrap().remove(&edge_id);
            self.reverse_adjacency.get_mut(&to).unwrap().remove(&edge_id);
            self.next_edge_id -= 1;

            return Err(DagError::CycleDetected(cycle));
        }

        Ok(edge_id)
    }

    /// Remove an edge from the graph
    pub fn remove_edge(&mut self, id: EdgeId) -> DagResult<()> {
        let edge = self.edges.get(&id).ok_or(DagError::EdgeNotFound(id))?;
        let from = edge.from;
        let to = edge.to;

        self.adjacency.get_mut(&from).unwrap().remove(&id);
        self.reverse_adjacency.get_mut(&to).unwrap().remove(&id);
        self.edges.remove(&id);

        Ok(())
    }

    /// Get an edge by ID
    pub fn get_edge(&self, id: EdgeId) -> DagResult<&Edge> {
        self.edges.get(&id).ok_or(DagError::EdgeNotFound(id))
    }

    /// Get all edges
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.values()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get all outgoing edges from a node
    pub fn outgoing_edges(&self, node_id: NodeId) -> DagResult<Vec<&Edge>> {
        let edge_ids = self.adjacency.get(&node_id)
            .ok_or(DagError::NodeNotFound(node_id))?;
        edge_ids.iter()
            .map(|id| self.edges.get(id).ok_or(DagError::EdgeNotFound(*id)))
            .collect()
    }

    /// Get all incoming edges to a node
    pub fn incoming_edges(&self, node_id: NodeId) -> DagResult<Vec<&Edge>> {
        let edge_ids = self.reverse_adjacency.get(&node_id)
            .ok_or(DagError::NodeNotFound(node_id))?;
        edge_ids.iter()
            .map(|id| self.edges.get(id).ok_or(DagError::EdgeNotFound(*id)))
            .collect()
    }

    /// Get all direct successors of a node
    pub fn successors(&self, node_id: NodeId) -> DagResult<Vec<NodeId>> {
        let edges = self.outgoing_edges(node_id)?;
        Ok(edges.iter().map(|e| e.to).collect())
    }

    /// Get all direct predecessors of a node
    pub fn predecessors(&self, node_id: NodeId) -> DagResult<Vec<NodeId>> {
        let edges = self.incoming_edges(node_id)?;
        Ok(edges.iter().map(|e| e.from).collect())
    }

    /// Check if a node has no incoming edges
    pub fn is_root(&self, node_id: NodeId) -> DagResult<bool> {
        let incoming = self.reverse_adjacency.get(&node_id)
            .ok_or(DagError::NodeNotFound(node_id))?;
        Ok(incoming.is_empty())
    }

    /// Check if a node has no outgoing edges
    pub fn is_leaf(&self, node_id: NodeId) -> DagResult<bool> {
        let outgoing = self.adjacency.get(&node_id)
            .ok_or(DagError::NodeNotFound(node_id))?;
        Ok(outgoing.is_empty())
    }

    /// Get all root nodes (nodes with no incoming edges)
    pub fn roots(&self) -> Vec<NodeId> {
        self.node_ids()
            .filter(|id| self.is_root(*id).unwrap_or(false))
            .collect()
    }

    /// Get all leaf nodes (nodes with no outgoing edges)
    pub fn leaves(&self) -> Vec<NodeId> {
        self.node_ids()
            .filter(|id| self.is_leaf(*id).unwrap_or(false))
            .collect()
    }

    /// Perform topological sort of the nodes
    ///
    /// Returns nodes in an order where all dependencies come before dependents
    pub fn topological_sort(&self) -> DagResult<Vec<NodeId>> {
        if self.nodes.is_empty() {
            return Err(DagError::EmptyGraph);
        }

        let mut in_degree: BTreeMap<NodeId, usize> = BTreeMap::new();
        for &node_id in self.nodes.keys() {
            in_degree.insert(node_id, 0);
        }

        for edge in self.edges.values() {
            *in_degree.get_mut(&edge.to).unwrap() += 1;
        }

        let mut queue: Vec<NodeId> = in_degree.iter()
            .filter(|&(_, &degree)| degree == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::with_capacity(self.nodes.len());

        while let Some(node_id) = queue.pop() {
            result.push(node_id);

            if let Some(outgoing) = self.adjacency.get(&node_id) {
                for edge_id in outgoing {
                    if let Some(edge) = self.edges.get(edge_id) {
                        let to = edge.to;
                        if let Some(degree) = in_degree.get_mut(&to) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push(to);
                            }
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(DagError::CycleDetected(result));
        }

        Ok(result)
    }

    /// Detect cycles in the graph using DFS
    ///
    /// Returns the cycle path if found, None otherwise
    pub fn detect_cycle(&self) -> Option<Vec<NodeId>> {
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();
        let mut path = Vec::new();

        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                if let Some(cycle) = self.dfs_cycle(node_id, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS helper for cycle detection
    fn dfs_cycle(
        &self,
        node_id: NodeId,
        visited: &mut BTreeSet<NodeId>,
        rec_stack: &mut BTreeSet<NodeId>,
        path: &mut Vec<NodeId>,
    ) -> Option<Vec<NodeId>> {
        visited.insert(node_id);
        rec_stack.insert(node_id);
        path.push(node_id);

        if let Some(outgoing) = self.adjacency.get(&node_id) {
            for edge_id in outgoing {
                if let Some(edge) = self.edges.get(edge_id) {
                    let to = edge.to;

                    if !visited.contains(&to) {
                        if let Some(cycle) = self.dfs_cycle(to, visited, rec_stack, path) {
                            return Some(cycle);
                        }
                    } else if rec_stack.contains(&to) {
                        // Found a cycle, extract it from the path
                        let cycle_start = path.iter().position(|&id| id == to).unwrap();
                        let mut cycle = path[cycle_start..].to_vec();
                        cycle.push(to);
                        return Some(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(&node_id);
        None
    }

    /// Resolve dependencies for a given node
    ///
    /// Returns all nodes that must be executed before the given node
    pub fn resolve_dependencies(&self, node_id: NodeId) -> DagResult<Vec<NodeId>> {
        if !self.nodes.contains_key(&node_id) {
            return Err(DagError::NodeNotFound(node_id));
        }

        let mut dependencies = Vec::new();
        let mut visited = BTreeSet::new();
        self.collect_dependencies(node_id, &mut dependencies, &mut visited);

        // Remove the node itself from the list
        dependencies.retain(|&id| id != node_id);

        Ok(dependencies)
    }

    /// Collect all dependencies recursively
    fn collect_dependencies(
        &self,
        node_id: NodeId,
        dependencies: &mut Vec<NodeId>,
        visited: &mut BTreeSet<NodeId>,
    ) {
        if visited.contains(&node_id) {
            return;
        }
        visited.insert(node_id);

        if let Ok(incoming) = self.incoming_edges(node_id) {
            for edge in incoming {
                let from = edge.from;
                dependencies.push(from);
                self.collect_dependencies(from, dependencies, visited);
            }
        }
    }

    /// Get execution levels for parallel execution planning
    ///
    /// Nodes in the same level can be executed in parallel
    pub fn execution_levels(&self) -> DagResult<Vec<Vec<NodeId>>> {
        if self.nodes.is_empty() {
            return Err(DagError::EmptyGraph);
        }

        let topo_order = self.topological_sort()?;
        let mut levels: Vec<Vec<NodeId>> = Vec::new();
        let mut node_level: BTreeMap<NodeId, usize> = BTreeMap::new();

        for &node_id in &topo_order {
            let mut max_level = 0;

            if let Ok(predecessors) = self.predecessors(node_id) {
                for pred_id in predecessors {
                    let pred_level = node_level.get(&pred_id).unwrap_or(&0);
                    max_level = max_level.max(*pred_level + 1);
                }
            }

            node_level.insert(node_id, max_level);

            while levels.len() <= max_level {
                levels.push(Vec::new());
            }
            levels[max_level].push(node_id);
        }

        Ok(levels)
    }

    /// Get nodes that are ready to execute given completed nodes
    pub fn ready_nodes(&self, completed: &BTreeSet<NodeId>) -> DagResult<Vec<NodeId>> {
        let mut ready = Vec::new();

        for node_id in self.node_ids() {
            if completed.contains(&node_id) {
                continue;
            }

            let predecessors = self.predecessors(node_id)?;
            let all_deps_done = predecessors.iter()
                .all(|pred_id| completed.contains(pred_id));

            if all_deps_done {
                ready.push(node_id);
            }
        }

        Ok(ready)
    }

    /// Clone the DAG
    pub fn clone_dag(&self) -> Dag {
        Dag {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            adjacency: self.adjacency.clone(),
            reverse_adjacency: self.reverse_adjacency.clone(),
            next_node_id: self.next_node_id,
            next_edge_id: self.next_edge_id,
        }
    }

    /// Clear all nodes and edges
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.adjacency.clear();
        self.reverse_adjacency.clear();
        self.next_node_id = 1;
        self.next_edge_id = 1;
    }

    /// Merge another DAG into this one
    ///
    /// Returns a mapping from old node IDs to new node IDs
    pub fn merge(&mut self, other: &Dag) -> DagResult<BTreeMap<NodeId, NodeId>> {
        let mut id_mapping = BTreeMap::new();

        // Add all nodes
        for node in other.nodes.values() {
            let new_id = self.add_node_with(Node {
                id: node.id,
                name: node.name.clone(),
                task_type: node.task_type.clone(),
                metadata: node.metadata.clone(),
                priority: node.priority,
            })?;
            id_mapping.insert(node.id, new_id);
        }

        // Add all edges with updated IDs
        for edge in other.edges.values() {
            let new_from = *id_mapping.get(&edge.from).unwrap();
            let new_to = *id_mapping.get(&edge.to).unwrap();
            self.add_edge(new_from, new_to, edge.dependency_type)?;
        }

        Ok(id_mapping)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dag() {
        let dag = Dag::new();
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut dag = Dag::new();
        let id = dag.add_node("test").unwrap();
        assert_eq!(id, 1);
        assert_eq!(dag.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut dag = Dag::new();
        let id1 = dag.add_node("node1").unwrap();
        let id2 = dag.add_node("node2").unwrap();
        let edge_id = dag.add_edge(id1, id2, DependencyType::Sequential).unwrap();
        assert_eq!(edge_id, 1);
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn test_topological_sort() {
        let mut dag = Dag::new();
        let id1 = dag.add_node("node1").unwrap();
        let id2 = dag.add_node("node2").unwrap();
        let id3 = dag.add_node("node3").unwrap();
        dag.add_edge(id1, id2, DependencyType::Sequential).unwrap();
        dag.add_edge(id2, id3, DependencyType::Sequential).unwrap();

        let order = dag.topological_sort().unwrap();
        assert_eq!(order, vec![id1, id2, id3]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut dag = Dag::new();
        let id1 = dag.add_node("node1").unwrap();
        let id2 = dag.add_node("node2").unwrap();
        dag.add_edge(id1, id2, DependencyType::Sequential).unwrap();

        // Adding edge back should fail
        let result = dag.add_edge(id2, id1, DependencyType::Sequential);
        assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }

    #[test]
    fn test_execution_levels() {
        let mut dag = Dag::new();
        let id1 = dag.add_node("node1").unwrap();
        let id2 = dag.add_node("node2").unwrap();
        let id3 = dag.add_node("node3").unwrap();
        let id4 = dag.add_node("node4").unwrap();
        dag.add_edge(id1, id3, DependencyType::Sequential).unwrap();
        dag.add_edge(id2, id3, DependencyType::Sequential).unwrap();
        dag.add_edge(id3, id4, DependencyType::Sequential).unwrap();

        let levels = dag.execution_levels().unwrap();
        assert_eq!(levels.len(), 3);
        assert!(levels[0].contains(&id1));
        assert!(levels[0].contains(&id2));
        assert!(levels[1].contains(&id3));
        assert!(levels[2].contains(&id4));
    }

    #[test]
    fn test_ready_nodes() {
        let mut dag = Dag::new();
        let id1 = dag.add_node("node1").unwrap();
        let id2 = dag.add_node("node2").unwrap();
        let id3 = dag.add_node("node3").unwrap();
        dag.add_edge(id1, id2, DependencyType::Sequential).unwrap();
        dag.add_edge(id1, id3, DependencyType::Sequential).unwrap();

        let completed = BTreeSet::new();
        let ready = dag.ready_nodes(&completed).unwrap();
        assert_eq!(ready, vec![id1]);

        let completed2 = BTreeSet::from_iter(vec![id1]);
        let ready2 = dag.ready_nodes(&completed2).unwrap();
        assert!(ready2.contains(&id2));
        assert!(ready2.contains(&id3));
    }
}
