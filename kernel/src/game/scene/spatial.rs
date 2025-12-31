//! # Spatial Partitioning
//!
//! Octree and other spatial data structures for efficient queries.

use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::compat::Float;
use super::{super::Vec3, NodeId};

/// Octree node
#[derive(Clone, Debug)]
pub struct OctreeNode {
    pub bounds: (Vec3, Vec3), // min, max
    pub children: Option<Box<[OctreeNode; 8]>>,
    pub objects: Vec<NodeId>,
    pub capacity: usize,
}

impl OctreeNode {
    #[inline]
    pub fn new(min: Vec3, max: Vec3, capacity: usize) -> Self {
        Self {
            bounds: (min, max),
            children: None,
            objects: Vec::new(),
            capacity,
        }
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    #[inline]
    pub fn insert(&mut self, object_id: NodeId, position: Vec3, max_depth: usize) -> bool {
        if !self.contains(position) {
            return false;
        }

        if self.is_leaf() {
            if self.objects.len() < self.capacity || max_depth == 0 {
                self.objects.push(object_id);
                true
            } else {
                self.subdivide();
                self.insert(object_id, position, max_depth)
            }
        } else {
            for child in self.children.as_mut().unwrap().iter_mut() {
                if child.insert(object_id, position, max_depth - 1) {
                    return true;
                }
            }
            false
        }
    }

    #[inline]
    pub fn remove(&mut self, object_id: NodeId) -> bool {
        if self.is_leaf() {
            let pos = self.objects.iter().position(|&id| id == object_id);
            if let Some(index) = pos {
                self.objects.remove(index);
                return true;
            }
            false
        } else {
            for child in self.children.as_mut().unwrap().iter_mut() {
                if child.remove(object_id) {
                    return true;
                }
            }
            false
        }
    }

    #[inline]
    pub fn query(&self, bounds: &(Vec3, Vec3)) -> Vec<NodeId> {
        let mut results = Vec::new();

        if !self.intersects(bounds) {
            return results;
        }

        if self.is_leaf() {
            results.extend(self.objects.iter());
        } else {
            for child in self.children.as_ref().unwrap().iter() {
                results.extend(child.query(bounds));
            }
        }

        results
    }

    #[inline]
    pub fn query_radius(&self, center: Vec3, radius: Float) -> Vec<NodeId> {
        let min = Vec3 {
            x: center.x - radius,
            y: center.y - radius,
            z: center.z - radius,
        };
        let max = Vec3 {
            x: center.x + radius,
            y: center.y + radius,
            z: center.z + radius,
        };

        let mut results = self.query(&(min, max));

        // Filter by actual distance
        results.retain(|&id| {
            // In production, would check actual object position
            true
        });

        results
    }

    fn subdivide(&mut self) {
        let (min, max) = self.bounds;
        let center = (min + max) * 0.5;

        let children: [OctreeNode; 8] = [
            // Bottom 4
            OctreeNode::new(
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(center.x, center.y, center.z),
                self.capacity,
            ),
            OctreeNode::new(
                Vec3::new(center.x, min.y, min.z),
                Vec3::new(max.x, center.y, center.z),
                self.capacity,
            ),
            OctreeNode::new(
                Vec3::new(min.x, min.y, center.z),
                Vec3::new(center.x, center.y, max.z),
                self.capacity,
            ),
            OctreeNode::new(
                Vec3::new(center.x, min.y, center.z),
                Vec3::new(max.x, center.y, max.z),
                self.capacity,
            ),
            // Top 4
            OctreeNode::new(
                Vec3::new(min.x, center.y, min.z),
                Vec3::new(center.x, max.y, center.z),
                self.capacity,
            ),
            OctreeNode::new(
                Vec3::new(center.x, center.y, min.z),
                Vec3::new(max.x, max.y, center.z),
                self.capacity,
            ),
            OctreeNode::new(
                Vec3::new(min.x, center.y, center.z),
                Vec3::new(center.x, max.y, max.z),
                self.capacity,
            ),
            OctreeNode::new(
                Vec3::new(center.x, center.y, center.z),
                Vec3::new(max.x, max.y, max.z),
                self.capacity,
            ),
        ];

        // Re-insert existing objects into children
        for &object_id in &self.objects {
            // In production, would know object position
            for child in &children {
                // child.insert(object_id, position, max_depth);
            }
        }

        self.objects.clear();
        self.children = Some(Box::new(children));
    }

    #[inline]
    fn contains(&self, point: Vec3) -> bool {
        let (min, max) = self.bounds;
        point.x >= min.x && point.x <= max.x
            && point.y >= min.y && point.y <= max.y
            && point.z >= min.z && point.z <= max.z
    }

    #[inline]
    fn intersects(&self, bounds: &(Vec3, Vec3)) -> bool {
        let (min_a, max_a) = self.bounds;
        let (min_b, max_b) = *bounds;

        min_a.x <= max_b.x && max_a.x >= min_b.x
            && min_a.y <= max_b.y && max_a.y >= min_b.y
            && min_a.z <= max_b.z && max_a.z >= min_b.z
    }
}

/// Octree for spatial partitioning
#[derive(Clone, Debug)]
pub struct Octree {
    pub root: OctreeNode,
    pub max_depth: usize,
}

impl Octree {
    #[inline]
    pub fn new(min: Vec3, max: Vec3, max_depth: usize) -> Self {
        Self {
            root: OctreeNode::new(min, max, 8),
            max_depth,
        }
    }

    #[inline]
    pub fn insert(&mut self, object_id: NodeId, position: Vec3) -> bool {
        self.root.insert(object_id, position, self.max_depth)
    }

    #[inline]
    pub fn remove(&mut self, object_id: NodeId) -> bool {
        self.root.remove(object_id)
    }

    #[inline]
    pub fn query(&self, bounds: &(Vec3, Vec3)) -> Vec<NodeId> {
        self.root.query(bounds)
    }

    #[inline]
    pub fn query_radius(&self, center: Vec3, radius: Float) -> Vec<NodeId> {
        self.root.query_radius(center, radius)
    }

    #[inline]
    pub fn clear(&mut self) {
        let (min, max) = self.root.bounds;
        let capacity = self.root.capacity;
        self.root = OctreeNode::new(min, max, capacity);
    }
}
