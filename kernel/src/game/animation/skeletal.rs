//! # Skeletal Animation
//!
//! Bone hierarchy and skeletal animation support.

use alloc::vec::Vec;
use alloc::string::String;

use crate::compat::Float;
use super::{super::Vec3, super::Quaternion, super::Transform};

/// Bone in a skeleton
#[derive(Clone, Debug)]
pub struct Bone {
    pub name: String,
    pub parent_index: Option<usize>,
    pub children: Vec<usize>,
    pub bind_pose: Transform,
    pub inverse_bind_pose: Transform,
    pub local_transform: Transform,
}

impl Bone {
    #[inline]
    pub fn new(name: String, parent_index: Option<usize>) -> Self {
        let bind_pose = Transform::identity();
        Self {
            name,
            parent_index,
            children: Vec::new(),
            bind_pose,
            inverse_bind_pose: Transform::identity(),
            local_transform: Transform::identity(),
        }
    }

    #[inline]
    pub fn with_bind_pose(mut self, pose: Transform) -> Self {
        self.bind_pose = pose;
        self
    }
}

/// Skeleton (bone hierarchy)
#[derive(Clone, Debug)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub name: String,
}

impl Skeleton {
    #[inline]
    pub fn new(name: String) -> Self {
        Self {
            bones: Vec::new(),
            name,
        }
    }

    #[inline]
    pub fn add_bone(&mut self, bone: Bone) -> usize {
        let index = self.bones.len();
        // Add to parent's children if parent exists
        if let Some(parent) = bone.parent_index {
            if parent < self.bones.len() {
                self.bones[parent].children.push(index);
            }
        }
        self.bones.push(bone);
        index
    }

    #[inline]
    pub fn get_bone(&self, name: &str) -> Option<&Bone> {
        self.bones.iter().find(|b| b.name == name)
    }

    #[inline]
    pub fn get_bone_mut(&mut self, name: &str) -> Option<&mut Bone> {
        self.bones.iter_mut().find(|b| b.name == name)
    }

    #[inline]
    pub fn get_bone_index(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }

    #[inline]
    pub fn calculate_inverse_bind_poses(&mut self) {
        for i in 0..self.bones.len() {
            let world_transform = self.get_world_transform(i);
            self.bones[i].inverse_bind_pose = world_transform.inverse().unwrap_or(Transform::identity());
        }
    }

    #[inline]
    pub fn get_world_transform(&self, bone_index: usize) -> Transform {
        let bone = &self.bones[bone_index];
        let mut transform = bone.local_transform;

        let mut current_index = bone.parent_index;
        while let Some(parent_idx) = current_index {
            let parent = &self.bones[parent_idx];
            transform = parent.local_transform.to_matrix().multiply(&transform.to_matrix()).into();
            current_index = parent.parent_index;
        }

        transform
    }

    #[inline]
    pub fn set_bone_transform(&mut self, bone_index: usize, transform: Transform) {
        if bone_index < self.bones.len() {
            self.bones[bone_index].local_transform = transform;
        }
    }
}
