//! # Scene Graph
//!
//! Hierarchical scene management.

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use super::{super::Transform, super::Vec3, NodeId, NodeType};

/// Scene node
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub id: NodeId,
    pub name: String,
    pub node_type: NodeType,
    pub local_transform: Transform,
    pub world_transform: Transform,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub enabled: bool,
}

impl SceneNode {
    #[inline]
    pub fn new(name: String, node_type: NodeType) -> Self {
        Self {
            id: NodeId::new(),
            name,
            node_type,
            local_transform: Transform::identity(),
            world_transform: Transform::identity(),
            parent: None,
            children: Vec::new(),
            enabled: true,
        }
    }

    #[inline]
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.local_transform = transform;
        self
    }

    #[inline]
    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    #[inline]
    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    #[inline]
    pub fn remove_child(&mut self, child: NodeId) {
        self.children.retain(|&c| c != child);
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    #[inline]
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

/// Scene graph
#[derive(Clone, Debug)]
pub struct SceneGraph {
    pub nodes: BTreeMap<NodeId, SceneNode>,
    pub root: NodeId,
}

impl SceneGraph {
    #[inline]
    pub fn new() -> Self {
        let mut graph = Self {
            nodes: BTreeMap::new(),
            root: NodeId::new(),
        };

        let root_node = SceneNode {
            id: graph.root,
            name: String::from("Root"),
            node_type: NodeType::Empty,
            local_transform: Transform::identity(),
            world_transform: Transform::identity(),
            parent: None,
            children: Vec::new(),
            enabled: true,
        };

        graph.nodes.insert(graph.root, root_node);
        graph
    }

    #[inline]
    pub fn add_node(&mut self, mut node: SceneNode) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    #[inline]
    pub fn remove_node(&mut self, id: NodeId) -> Option<SceneNode> {
        // Remove from parent
        if let Some(node) = self.nodes.get(&id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.remove_child(id);
                }
            }
        }

        // Remove children recursively
        let node = self.nodes.remove(&id)?;
        for &child_id in &node.children {
            self.remove_node(child_id);
        }

        Some(node)
    }

    #[inline]
    pub fn get_node(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    #[inline]
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    #[inline]
    pub fn set_parent(&mut self, node_id: NodeId, parent_id: NodeId) -> bool {
        // Remove from current parent
        if let Some(node) = self.nodes.get(&node_id) {
            if let Some(old_parent) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&old_parent) {
                    parent.remove_child(node_id);
                }
            }
        }

        // Set new parent
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.parent = Some(parent_id);
        }

        // Add to new parent's children
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.add_child(node_id);
        }

        // Update world transforms
        self.update_transforms(node_id);

        true
    }

    #[inline]
    pub fn update_transforms(&mut self, node_id: NodeId) {
        let node = if let Some(node) = self.nodes.get(&node_id) {
            node.clone()
        } else {
            return;
        };

        // Calculate world transform
        let world_transform = if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get(&parent_id) {
                parent.world_transform.to_matrix().multiply(&node.local_transform.to_matrix()).into()
            } else {
                node.local_transform
            }
        } else {
            node.local_transform
        };

        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.world_transform = world_transform;
        }

        // Update children
        for &child_id in &node.children {
            self.update_transforms(child_id);
        }
    }

    #[inline]
    pub fn find_node_by_name(&self, name: &str) -> Option<&SceneNode> {
        self.nodes.values().find(|n| n.name == name)
    }

    #[inline]
    pub fn traverse(&self, visitor: &mut dyn FnMut(&SceneNode)) {
        self.traverse_recursive(self.root, visitor);
    }

    fn traverse_recursive(&self, node_id: NodeId, visitor: &mut dyn FnMut(&SceneNode)) {
        if let Some(node) = self.nodes.get(&node_id) {
            visitor(node);

            for &child_id in &node.children {
                self.traverse_recursive(child_id, visitor);
            }
        }
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}
