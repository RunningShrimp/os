//! # Morph Targets (Blend Shapes)
//!
//! Morph target animation for facial expressions and deformations.

use alloc::vec::Vec;
use alloc::string::String;
use crate::compat::Float;
use super::super::Vec3;

/// Single morph target
#[derive(Clone, Debug)]
pub struct MorphTarget {
    pub name: String,
    pub vertex_offsets: Vec<Vec3>,
    pub normal_offsets: Vec<Vec3>,
}

impl MorphTarget {
    #[inline]
    pub fn new(name: String, vertex_count: usize) -> Self {
        Self {
            name,
            vertex_offsets: vec![Vec3::zero(); vertex_count],
            normal_offsets: vec![Vec3::zero(); vertex_count],
        }
    }

    #[inline]
    pub fn set_vertex_offset(&mut self, index: usize, offset: Vec3) {
        if index < self.vertex_offsets.len() {
            self.vertex_offsets[index] = offset;
        }
    }
}

/// Morph target controller
#[derive(Clone, Debug)]
pub struct MorphTargetController {
    pub targets: Vec<MorphTarget>,
    pub weights: Vec<Float>,
}

impl MorphTargetController {
    #[inline]
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            weights: Vec::new(),
        }
    }

    #[inline]
    pub fn add_target(&mut self, target: MorphTarget) -> usize {
        let index = self.targets.len();
        self.targets.push(target);
        self.weights.push(0.0);
        index
    }

    #[inline]
    pub fn set_weight(&mut self, index: usize, weight: Float) {
        if index < self.weights.len() {
            self.weights[index] = weight.clamp(0.0, 1.0);
        }
    }

    #[inline]
    pub fn get_weight(&self, index: usize) -> Float {
        self.weights.get(index).copied().unwrap_or(0.0)
    }

    #[inline]
    pub fn get_target(&self, index: usize) -> Option<&MorphTarget> {
        self.targets.get(index)
    }
}

impl Default for MorphTargetController {
    fn default() -> Self {
        Self::new()
    }
}
