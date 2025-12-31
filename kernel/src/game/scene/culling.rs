//! # Frustum Culling
//!
//! View frustum culling optimization.

use crate::compat::Float;
use super::{super::Vec3, Frustum, BoundingVolume};

/// Culling result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CullingResult {
    Inside,
    Outside,
    Partial,
}

/// Frustum culling
pub struct FrustumCulling;

impl FrustumCulling {
    #[inline]
    pub fn test_volume(volume: &BoundingVolume, frustum: &Frustum) -> CullingResult {
        match volume {
            BoundingVolume::Sphere { center, radius } => {
                if frustum.intersects_sphere(*center, *radius) {
                    CullingResult::Inside
                } else {
                    CullingResult::Outside
                }
            }
            BoundingVolume::AABB { min, max } => {
                if frustum.intersects_aabb(*min, *max) {
                    CullingResult::Inside
                } else {
                    CullingResult::Outside
                }
            }
            BoundingVolume::OBB { .. } => {
                // Simplified: treat as sphere
                if let BoundingVolume::Sphere { center, radius } = volume.to_sphere().unwrap() {
                    if frustum.intersects_sphere(center, radius) {
                        CullingResult::Inside
                    } else {
                        CullingResult::Outside
                    }
                } else {
                    CullingResult::Inside
                }
            }
        }
    }

    #[inline]
    pub fn test_point(point: Vec3, frustum: &Frustum) -> CullingResult {
        for plane in &frustum.planes {
            let distance = plane.normal.dot(point) + plane.distance;
            if distance < 0.0 {
                return CullingResult::Outside;
            }
        }
        CullingResult::Inside
    }

    #[inline]
    pub fn test_sphere(center: Vec3, radius: Float, frustum: &Frustum) -> CullingResult {
        if frustum.intersects_sphere(center, radius) {
            CullingResult::Inside
        } else {
            CullingResult::Outside
        }
    }
}
