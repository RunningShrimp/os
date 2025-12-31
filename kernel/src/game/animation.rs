//! # Animation System
//!
//! Advanced character and object animation with:
//! - Skeletal animation (bone hierarchy)
//! - Morph targets (blend shapes)
//! - Animation blend trees
//! - IK (Inverse Kinematics)
//! - Animation state machine
//! - Animation blending and layering

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::time::Duration;

use crate::compat::Float;

pub mod skeletal;
pub mod morph;
pub mod ik;
pub mod state_machine;
pub mod blend_tree;

pub use skeletal::*;
pub use morph::*;
pub use ik::*;
pub use state_machine::*;
pub use blend_tree::*;

/// Animation clip data
#[derive(Clone, Debug)]
pub struct AnimationClip {
    pub name: alloc::string::String,
    pub duration: Float,
    pub tracks: Vec<AnimationTrack>,
    pub looped: bool,
}

impl AnimationClip {
    #[inline]
    pub fn new(name: alloc::string::String, duration: Float) -> Self {
        Self {
            name,
            duration,
            tracks: Vec::new(),
            looped: true,
        }
    }

    #[inline]
    pub fn add_track(&mut self, track: AnimationTrack) {
        self.tracks.push(track);
    }

    #[inline]
    pub fn get_track(&self, bone_name: &str) -> Option<&AnimationTrack> {
        self.tracks.iter().find(|t| t.bone_name == bone_name)
    }

    #[inline]
    pub fn evaluate(&self, time: Float) -> Vec<BoneTransform> {
        let mut transforms = Vec::new();

        let wrapped_time = if self.looped {
            if self.duration > 0.0 {
                time % self.duration
            } else {
                time
            }
        } else {
            time.min(self.duration)
        };

        for track in &self.tracks {
            let transform = track.evaluate(wrapped_time);
            transforms.push(BoneTransform {
                bone_name: track.bone_name.clone(),
                transform,
            });
        }

        transforms
    }
}

/// Animation track for a single bone
#[derive(Clone, Debug)]
pub struct AnimationTrack {
    pub bone_name: alloc::string::String,
    pub position_keys: Vec<KeyFrame<Vec3>>,
    pub rotation_keys: Vec<KeyFrame<Quaternion>>,
    pub scale_keys: Vec<KeyFrame<Vec3>>,
}

impl AnimationTrack {
    #[inline]
    pub fn new(bone_name: alloc::string::String) -> Self {
        Self {
            bone_name,
            position_keys: Vec::new(),
            rotation_keys: Vec::new(),
            scale_keys: Vec::new(),
        }
    }

    #[inline]
    pub fn evaluate(&self, time: Float) -> super::Transform {
        let position = self.evaluate_position(time);
        let rotation = self.evaluate_rotation(time);
        let scale = self.evaluate_scale(time);

        super::Transform {
            position,
            rotation,
            scale,
        }
    }

    #[inline]
    fn evaluate_position(&self, time: Float) -> Vec3 {
        if self.position_keys.is_empty() {
            return Vec3::zero();
        }

        if time <= self.position_keys[0].time {
            return self.position_keys[0].value;
        }

        if time >= self.position_keys.last().unwrap().time {
            return self.position_keys.last().unwrap().value;
        }

        // Find surrounding keys and interpolate
        for i in 0..self.position_keys.len() - 1 {
            if time >= self.position_keys[i].time && time <= self.position_keys[i + 1].time {
                let t = (time - self.position_keys[i].time)
                    / (self.position_keys[i + 1].time - self.position_keys[i].time);
                return self.position_keys[i].value.lerp(self.position_keys[i + 1].value, t);
            }
        }

        Vec3::zero()
    }

    #[inline]
    fn evaluate_rotation(&self, time: Float) -> Quaternion {
        if self.rotation_keys.is_empty() {
            return Quaternion::identity();
        }

        if time <= self.rotation_keys[0].time {
            return self.rotation_keys[0].value;
        }

        if time >= self.rotation_keys.last().unwrap().time {
            return self.rotation_keys.last().unwrap().value;
        }

        // Slerp between rotations
        for i in 0..self.rotation_keys.len() - 1 {
            if time >= self.rotation_keys[i].time && time <= self.rotation_keys[i + 1].time {
                let t = (time - self.rotation_keys[i].time)
                    / (self.rotation_keys[i + 1].time - self.rotation_keys[i].time);
                return self.rotation_keys[i].value.slerp(&self.rotation_keys[i + 1].value, t);
            }
        }

        Quaternion::identity()
    }

    #[inline]
    fn evaluate_scale(&self, time: Float) -> Vec3 {
        if self.scale_keys.is_empty() {
            return Vec3::one();
        }

        if time <= self.scale_keys[0].time {
            return self.scale_keys[0].value;
        }

        if time >= self.scale_keys.last().unwrap().time {
            return self.scale_keys.last().unwrap().value;
        }

        for i in 0..self.scale_keys.len() - 1 {
            if time >= self.scale_keys[i].time && time <= self.scale_keys[i + 1].time {
                let t = (time - self.scale_keys[i].time)
                    / (self.scale_keys[i + 1].time - self.scale_keys[i].time);
                return self.scale_keys[i].value.lerp(self.scale_keys[i + 1].value, t);
            }
        }

        Vec3::one()
    }
}

/// Key frame for animation
#[derive(Clone, Copy, Debug)]
pub struct KeyFrame<T> {
    pub time: Float,
    pub value: T,
}

impl<T> KeyFrame<T> {
    #[inline]
    pub fn new(time: Float, value: T) -> Self {
        Self { time, value }
    }
}

/// Bone transform result
#[derive(Clone, Debug)]
pub struct BoneTransform {
    pub bone_name: alloc::string::String,
    pub transform: super::Transform,
}

/// Animation blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Linear,
    Additive,
}

/// Animation state
#[derive(Clone, Debug)]
pub struct AnimationState {
    pub clip: AnimationClip,
    pub weight: Float,
    pub speed: Float,
    pub time: Float,
    pub blend_mode: BlendMode,
}

impl AnimationState {
    #[inline]
    pub fn new(clip: AnimationClip) -> Self {
        Self {
            clip,
            weight: 1.0,
            speed: 1.0,
            time: 0.0,
            blend_mode: BlendMode::Linear,
        }
    }

    #[inline]
    pub fn update(&mut self, dt: Float) {
        self.time += dt * self.speed;
        if self.time > self.clip.duration {
            if self.clip.looped {
                self.time %= self.clip.duration;
            } else {
                self.time = self.clip.duration;
            }
        }
    }

    #[inline]
    pub fn set_weight(&mut self, weight: Float) {
        self.weight = weight.clamp(0.0, 1.0);
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        !self.clip.looped && self.time >= self.clip.duration
    }
}

/// Animation layer for blending multiple animations
#[derive(Clone, Debug)]
pub struct AnimationLayer {
    pub name: alloc::string::String,
    pub states: Vec<AnimationState>,
    pub blending: bool,
    pub blend_duration: Float,
    pub current_blend_time: Float,
}

impl AnimationLayer {
    #[inline]
    pub fn new(name: alloc::string::String) -> Self {
        Self {
            name,
            states: Vec::new(),
            blending: false,
            blend_duration: 0.3,
            current_blend_time: 0.0,
        }
    }

    #[inline]
    pub fn add_state(&mut self, state: AnimationState) {
        self.states.push(state);
    }

    #[inline]
    pub fn play(&mut self, clip_name: &str, fade_duration: Float) {
        // Simplified: Just set the first matching state to full weight
        for state in &mut self.states {
            if state.clip.name == clip_name {
                state.weight = 1.0;
            } else {
                state.weight = 0.0;
            }
        }
        self.blending = fade_duration > 0.0;
        self.blend_duration = fade_duration;
        self.current_blend_time = 0.0;
    }

    #[inline]
    pub fn update(&mut self, dt: Float) {
        for state in &mut self.states {
            if state.weight > 0.0 {
                state.update(dt);
            }
        }

        if self.blending {
            self.current_blend_time += dt;
            if self.current_blend_time >= self.blend_duration {
                self.blending = false;
                // Finalize blend
                let max_weight_state = self.states.iter()
                    .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap())
                    .map(|s| s.clip.name.clone());

                if let Some(name) = max_weight_state {
                    for state in &mut self.states {
                        state.weight = if state.clip.name == name { 1.0 } else { 0.0 };
                    }
                }
            }
        }
    }

    #[inline]
    pub fn evaluate(&self) -> Vec<BoneTransform> {
        if self.states.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let total_weight: Float = self.states.iter().map(|s| s.weight).sum();

        if total_weight < 0.0001 {
            return result;
        }

        // Collect all bone names
        let mut bone_names = alloc::collections::BTreeSet::new();
        for state in &self.states {
            for track in &state.clip.tracks {
                bone_names.insert(track.bone_name.clone());
            }
        }

        // Evaluate and blend per bone
        for bone_name in bone_names {
            let mut position = Vec3::zero();
            let mut rotation = Quaternion::identity();
            let mut scale = Vec3::one();
            let mut total_bone_weight = 0.0;

            for state in &self.states {
                if state.weight > 0.0 {
                    if let Some(track) = state.clip.get_track(&bone_name) {
                        let transform = track.evaluate(state.time);
                        let normalized_weight = state.weight / total_weight;

                        position = position + transform.position * normalized_weight;
                        rotation = rotation.slerp(&transform.rotation, normalized_weight);
                        scale = scale + (transform.scale - Vec3::one()) * normalized_weight;
                        total_bone_weight += normalized_weight;
                    }
                }
            }

            if total_bone_weight > 0.0 {
                result.push(BoneTransform {
                    bone_name,
                    transform: super::Transform {
                        position,
                        rotation,
                        scale,
                    },
                });
            }
        }

        result
    }
}

/// Animation mixer (manages all animation layers)
#[derive(Clone, Debug)]
pub struct AnimationMixer {
    pub layers: Vec<AnimationLayer>,
    pub skeleton: Option<Skeleton>,
}

impl AnimationMixer {
    #[inline]
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            skeleton: None,
        }
    }

    #[inline]
    pub fn with_skeleton(mut self, skeleton: Skeleton) -> Self {
        self.skeleton = Some(skeleton);
        self
    }

    #[inline]
    pub fn add_layer(&mut self, layer: AnimationLayer) {
        self.layers.push(layer);
    }

    #[inline]
    pub fn update(&mut self, dt: Float) {
        for layer in &mut self.layers {
            layer.update(dt);
        }
    }

    #[inline]
    pub fn evaluate(&self) -> Vec<BoneTransform> {
        if let Some(skeleton) = &self.skeleton {
            // Combine all layers and apply to skeleton
            let mut final_transforms = Vec::new();

            for bone in &skeleton.bones {
                let mut transform = bone.bind_pose;

                for layer in &self.layers {
                    let layer_transforms = layer.evaluate();
                    if let Some(bone_transform) = layer_transforms.iter()
                        .find(|bt| bt.bone_name == bone.name)
                    {
                        transform = bone_transform.transform;
                    }
                }

                final_transforms.push(BoneTransform {
                    bone_name: bone.name.clone(),
                    transform,
                });
            }

            final_transforms
        } else {
            // No skeleton, just return layer transforms
            let mut all_transforms = Vec::new();
            for layer in &self.layers {
                all_transforms.extend(layer.evaluate());
            }
            all_transforms
        }
    }

    #[inline]
    pub fn get_bone_transform(&self, bone_name: &str) -> Option<super::Transform> {
        let transforms = self.evaluate();
        transforms
            .iter()
            .find(|bt| bt.bone_name == bone_name)
            .map(|bt| bt.transform)
    }

    #[inline]
    pub fn play(&mut self, layer_name: &str, clip_name: &str, fade_duration: Float) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.name == layer_name) {
            layer.play(clip_name, fade_duration);
        }
    }
}

impl Default for AnimationMixer {
    fn default() -> Self {
        Self::new()
    }
}

/// Reusable animation types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationCurveType {
    Linear,
    Step,
    Cubic,
}

#[derive(Clone, Debug)]
pub struct AnimationCurve {
    pub keys: Vec<KeyFrame<Float>>,
    pub curve_type: AnimationCurveType,
}

impl AnimationCurve {
    #[inline]
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            curve_type: AnimationCurveType::Linear,
        }
    }

    #[inline]
    pub fn evaluate(&self, time: Float) -> Float {
        if self.keys.is_empty() {
            return 0.0;
        }

        if time <= self.keys[0].time {
            return self.keys[0].value;
        }

        if time >= self.keys.last().unwrap().time {
            return self.keys.last().unwrap().value;
        }

        for i in 0..self.keys.len() - 1 {
            if time >= self.keys[i].time && time <= self.keys[i + 1].time {
                match self.curve_type {
                    AnimationCurveType::Linear => {
                        let t = (time - self.keys[i].time)
                            / (self.keys[i + 1].time - self.keys[i].time);
                        return self.keys[i].value + (self.keys[i + 1].value - self.keys[i].value) * t;
                    }
                    AnimationCurveType::Step => {
                        return self.keys[i].value;
                    }
                    AnimationCurveType::Cubic => {
                        // Simplified cubic interpolation
                        let t = (time - self.keys[i].time)
                            / (self.keys[i + 1].time - self.keys[i].time);
                        let t2 = t * t;
                        let t3 = t2 * t;
                        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
                        let h10 = t3 - 2.0 * t2 + t;
                        let h01 = -2.0 * t3 + 3.0 * t2;
                        let h11 = t3 - t2;
                        return h00 * self.keys[i].value + h01 * self.keys[i + 1].value;
                    }
                }
            }
        }

        0.0
    }

    #[inline]
    pub fn add_key(&mut self, time: Float, value: Float) {
        self.keys.push(KeyFrame::new(time, value));
        self.keys.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }
}

impl Default for AnimationCurve {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export commonly used types
pub use super::physics::{Vec3, Quaternion, Transform};
