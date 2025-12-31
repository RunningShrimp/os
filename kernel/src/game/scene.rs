//! # Scene Management and Spatial Partitioning
//!
//! Advanced scene management with:
//! - Scene graph (hierarchical transforms)
//! - Octree spatial partitioning
//! - BSP tree for static geometry
//! - Frustum culling
//! - LOD (Level of Detail) system
//! - Scene queries and traversal

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::compat::Float;

pub mod graph;
pub mod spatial;
pub mod culling;
pub mod lod;

pub use graph::*;
pub use spatial::*;
pub use culling::*;
pub use lod::*;

/// Unique ID for scene nodes
static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId {
    id: usize,
}

impl NodeId {
    #[inline]
    pub fn new() -> Self {
        Self {
            id: NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline]
    pub fn from_raw(id: usize) -> Self {
        Self { id }
    }

    #[inline]
    pub fn raw(&self) -> usize {
        self.id
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene node types
#[derive(Clone, Debug)]
pub enum NodeType {
    Empty,
    Mesh,
    Light,
    Camera,
    ParticleSystem,
    AudioSource,
    Custom(alloc::string::String),
}

/// Scene bounding volume
#[derive(Clone, Copy, Debug)]
pub enum BoundingVolume {
    Sphere { center: super::Vec3, radius: Float },
    AABB { min: super::Vec3, max: super::Vec3 },
    OBB { center: super::Vec3, rotation: super::Quaternion, half_extents: super::Vec3 },
}

impl BoundingVolume {
    #[inline]
    pub fn sphere(center: super::Vec3, radius: Float) -> Self {
        Self::Sphere { center, radius }
    }

    #[inline]
    pub fn aabb(min: super::Vec3, max: super::Vec3) -> Self {
        Self::AABB { min, max }
    }

    #[inline]
    pub fn contains_point(&self, point: super::Vec3) -> bool {
        match self {
            Self::Sphere { center, radius } => {
                center.distance_squared(point) <= radius * radius
            }
            Self::AABB { min, max } => {
                point.x >= min.x && point.x <= max.x
                    && point.y >= min.y && point.y <= max.y
                    && point.z >= min.z && point.z <= max.z
            }
            Self::OBB { center, rotation, half_extents } => {
                // Transform point to OBB local space
                let inv_rotation = rotation.conjugate();
                let local_point = inv_rotation.rotate_vector(point - *center);
                local_point.x.abs() <= half_extents.x
                    && local_point.y.abs() <= half_extents.y
                    && local_point.z.abs() <= half_extents.z
            }
        }
    }

    #[inline]
    pub fn intersects_frustum(&self, frustum: &Frustum) -> bool {
        match self {
            Self::Sphere { center, radius } => frustum.intersects_sphere(*center, *radius),
            Self::AABB { min, max } => frustum.intersects_aabb(*min, *max),
            Self::OBB { .. } => {
                // Simplified: treat as sphere
                if let Self::Sphere { center, radius } = self.to_sphere() {
                    frustum.intersects_sphere(center, radius)
                } else {
                    true
                }
            }
        }
    }

    #[inline]
    fn to_sphere(&self) -> Option<Self> {
        match self {
            Self::Sphere { .. } => Some(*self),
            Self::AABB { min, max } => {
                let center = (*min + *max) * 0.5;
                let radius = center.distance(*max);
                Some(Self::Sphere { center, radius })
            }
            _ => None,
        }
    }
}

/// View frustum for culling
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: super::Vec3,
    pub distance: Float,
}

impl Frustum {
    #[inline]
    pub fn from_view_projection(view_projection: &super::Mat4) -> Self {
        // Extract frustum planes from view-projection matrix
        let m = &view_projection.m;

        let left = Plane {
            normal: super::Vec3::new(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0]),
            distance: m[3][3] + m[3][0],
        };

        let right = Plane {
            normal: super::Vec3::new(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0]),
            distance: m[3][3] - m[3][0],
        };

        let bottom = Plane {
            normal: super::Vec3::new(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1]),
            distance: m[3][3] + m[3][1],
        };

        let top = Plane {
            normal: super::Vec3::new(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1]),
            distance: m[3][3] - m[3][1],
        };

        let near = Plane {
            normal: super::Vec3::new(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2]),
            distance: m[3][3] + m[3][2],
        };

        let far = Plane {
            normal: super::Vec3::new(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2]),
            distance: m[3][3] - m[3][2],
        };

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    #[inline]
    pub fn intersects_sphere(&self, center: super::Vec3, radius: Float) -> bool {
        for plane in &self.planes {
            let distance = plane.normal.dot(center) + plane.distance;
            if distance < -radius {
                return false;
            }
        }
        true
    }

    #[inline]
    pub fn intersects_aabb(&self, min: super::Vec3, max: super::Vec3) -> bool {
        // Test all 8 corners against all 6 planes
        let corners = [
            super::Vec3::new(min.x, min.y, min.z),
            super::Vec3::new(max.x, min.y, min.z),
            super::Vec3::new(min.x, max.y, min.z),
            super::Vec3::new(max.x, max.y, min.z),
            super::Vec3::new(min.x, min.y, max.z),
            super::Vec3::new(max.x, min.y, max.z),
            super::Vec3::new(min.x, max.y, max.z),
            super::Vec3::new(max.x, max.y, max.z),
        ];

        for plane in &self.planes {
            let mut outside = true;
            for corner in &corners {
                if plane.normal.dot(*corner) + plane.distance >= 0.0 {
                    outside = false;
                    break;
                }
            }
            if outside {
                return false;
            }
        }
        true
    }
}

/// LOD (Level of Detail) configuration
#[derive(Clone, Debug)]
pub struct LODConfig {
    pub levels: Vec<LODLevel>,
}

#[derive(Clone, Debug)]
pub struct LODLevel {
    pub distance: Float,
    pub screen_size: Float,
    pub mesh_variant: Option<alloc::string::String>,
    pub material_variant: Option<alloc::string::String>,
}

impl LODConfig {
    #[inline]
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
        }
    }

    #[inline]
    pub fn add_level(&mut self, distance: Float, mesh_variant: alloc::string::String) {
        self.levels.push(LODLevel {
            distance,
            screen_size: 0.0,
            mesh_variant: Some(mesh_variant),
            material_variant: None,
        });
        self.levels.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    }

    #[inline]
    pub fn select_level(&self, distance: Float) -> usize {
        for (i, level) in self.levels.iter().enumerate() {
            if distance <= level.distance {
                return i;
            }
        }
        self.levels.len().saturating_sub(1)
    }

    #[inline]
    pub fn get_level(&self, index: usize) -> Option<&LODLevel> {
        self.levels.get(index)
    }
}

impl Default for LODConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene query types
#[derive(Clone, Debug)]
pub enum SceneQuery {
    Raycast {
        origin: super::Vec3,
        direction: super::Vec3,
        max_distance: Float,
    },
    SphereOverlap {
        center: super::Vec3,
        radius: Float,
    },
    AABBOverlap {
        min: super::Vec3,
        max: super::Vec3,
    },
}

/// Scene query result
#[derive(Clone, Debug)]
pub struct SceneQueryResult {
    pub node_id: NodeId,
    pub distance: Float,
    pub point: super::Vec3,
    pub normal: super::Vec3,
}

/// Scene statistics
#[derive(Clone, Copy, Debug)]
pub struct SceneStats {
    pub total_nodes: usize,
    pub visible_nodes: usize,
    pub culled_nodes: usize,
    pub draw_calls: usize,
    pub triangle_count: usize,
}

impl SceneStats {
    #[inline]
    pub fn new() -> Self {
        Self {
            total_nodes: 0,
            visible_nodes: 0,
            culled_nodes: 0,
            draw_calls: 0,
            triangle_count: 0,
        }
    }

    #[inline]
    pub fn culling_ratio(&self) -> Float {
        if self.total_nodes > 0 {
            self.culled_nodes as Float / self.total_nodes as Float
        } else {
            0.0
        }
    }
}

impl Default for SceneStats {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export commonly used types
pub use super::physics::{Vec3, Mat4, Quaternion, Transform};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_generation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert!(id2.raw() > id1.raw());
    }

    #[test]
    fn test_bounding_volume_sphere() {
        let volume = BoundingVolume::sphere(Vec3::zero(), 5.0);
        assert!(volume.contains_point(Vec3::new(3.0, 0.0, 0.0)));
        assert!(!volume.contains_point(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_lod_level_selection() {
        let mut config = LODConfig::new();
        config.add_level(10.0, alloc::string::String::from("high"));
        config.add_level(20.0, alloc::string::String::from("medium"));
        config.add_level(40.0, alloc::string::String::from("low"));

        assert_eq!(config.select_level(5.0), 0);
        assert_eq!(config.select_level(15.0), 1);
        assert_eq!(config.select_level(30.0), 2);
        assert_eq!(config.select_level(50.0), 2);
    }
}
