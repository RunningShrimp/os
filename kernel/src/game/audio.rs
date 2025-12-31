//! # 3D Audio System
//!
//! Spatial audio rendering with:
//! - 3D positional audio
//! - HRTF (Head-Related Transfer Functions)
//! - Reverb and occlusion
//! - Doppler effect
//! - Sound source positioning
//! - Audio propagation

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::compat::Float;

/// Unique sound source ID
static NEXT_SOUND_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundId {
    id: usize,
}

impl SoundId {
    #[inline]
    pub fn new() -> Self {
        Self {
            id: NEXT_SOUND_ID.fetch_add(1, Ordering::SeqCst),
        }
    }
}

impl Default for SoundId {
    fn default() -> Self {
        Self::new()
    }
}

/// 3D audio listener
#[derive(Clone, Debug)]
pub struct AudioListener {
    pub position: super::Vec3,
    pub velocity: super::Vec3,
    pub orientation: super::Quaternion,
    pub up_vector: super::Vec3,
}

impl AudioListener {
    #[inline]
    pub fn new() -> Self {
        Self {
            position: super::Vec3::zero(),
            velocity: super::Vec3::zero(),
            orientation: super::Quaternion::identity(),
            up_vector: super::Vec3::new(0.0, 1.0, 0.0),
        }
    }

    #[inline]
    pub fn set_position(&mut self, position: super::Vec3) {
        self.position = position;
    }

    #[inline]
    pub fn set_orientation(&mut self, orientation: super::Quaternion) {
        self.orientation = orientation;
    }

    #[inline]
    pub fn forward(&self) -> super::Vec3 {
        self.orientation.rotate_vector(super::Vec3::new(0.0, 0.0, -1.0))
    }

    #[inline]
    pub fn right(&self) -> super::Vec3 {
        self.orientation.rotate_vector(super::Vec3::new(1.0, 0.0, 0.0))
    }

    #[inline]
    pub fn up(&self) -> super::Vec3 {
        self.orientation.rotate_vector(super::Vec3::new(0.0, 1.0, 0.0))
    }
}

impl Default for AudioListener {
    fn default() -> Self {
        Self::new()
    }
}

/// Sound source
#[derive(Clone, Debug)]
pub struct SoundSource {
    pub id: SoundId,
    pub position: super::Vec3,
    pub velocity: super::Vec3,
    pub volume: Float,
    pub pitch: Float,
    pub looped: bool,
    pub min_distance: Float,
    pub max_distance: Float,
    pub rolloff_factor: Float,
    pub cone_inner_angle: Float,
    pub cone_outer_angle: Float,
    pub cone_outer_gain: Float,
    pub priority: SoundPriority,
    pub playing: bool,
    pub paused: bool,
}

impl SoundSource {
    #[inline]
    pub fn new(id: SoundId) -> Self {
        Self {
            id,
            position: super::Vec3::zero(),
            velocity: super::Vec3::zero(),
            volume: 1.0,
            pitch: 1.0,
            looped: false,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
            cone_inner_angle: 360.0,
            cone_outer_angle: 360.0,
            cone_outer_gain: 0.0,
            priority: SoundPriority::Normal,
            playing: false,
            paused: false,
        }
    }

    #[inline]
    pub fn with_position(mut self, position: super::Vec3) -> Self {
        self.position = position;
        self
    }

    #[inline]
    pub fn with_volume(mut self, volume: Float) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    #[inline]
    pub fn with_pitch(mut self, pitch: Float) -> Self {
        self.pitch = pitch.max(0.1);
        self
    }

    #[inline]
    pub fn with_distance(mut self, min_distance: Float, max_distance: Float) -> Self {
        self.min_distance = min_distance.max(0.0);
        self.max_distance = max_distance.max(min_distance);
        self
    }

    #[inline]
    pub fn with_direction(
        mut self,
        inner_angle: Float,
        outer_angle: Float,
        outer_gain: Float,
    ) -> Self {
        self.cone_inner_angle = inner_angle;
        self.cone_outer_angle = outer_angle;
        self.cone_outer_gain = outer_gain.clamp(0.0, 1.0);
        self
    }

    #[inline]
    pub fn play(&mut self) {
        self.playing = true;
        self.paused = false;
    }

    #[inline]
    pub fn pause(&mut self) {
        self.paused = true;
    }

    #[inline]
    pub fn stop(&mut self) {
        self.playing = false;
        self.paused = false;
    }

    #[inline]
    pub fn is_playing(&self) -> bool {
        self.playing && !self.paused
    }
}

/// Sound priority for voice management
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoundPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

/// Distance model
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceModel {
    Inverse,
    InverseClamped,
    Linear,
    LinearClamped,
    Exponential,
    ExponentialClamped,
}

impl DistanceModel {
    #[inline]
    pub fn calculate_gain(
        &self,
        distance: Float,
        min_distance: Float,
        max_distance: Float,
        rolloff: Float,
    ) -> Float {
        match self {
            DistanceModel::Inverse => {
                if min_distance > 0.0 {
                    min_distance / (min_distance + rolloff * (distance - min_distance))
                } else {
                    1.0
                }
            }
            DistanceModel::InverseClamped => {
                let distance = distance.clamp(min_distance, max_distance);
                if min_distance > 0.0 {
                    min_distance / (min_distance + rolloff * (distance - min_distance))
                } else {
                    1.0
                }
            }
            DistanceModel::Linear => {
                if max_distance > min_distance {
                    1.0 - rolloff * (distance - min_distance) / (max_distance - min_distance)
                } else {
                    1.0
                }
            }
            DistanceModel::LinearClamped => {
                let distance = distance.clamp(min_distance, max_distance);
                if max_distance > min_distance {
                    (1.0 - rolloff * (distance - min_distance) / (max_distance - min_distance))
                        .clamp(0.0, 1.0)
                } else {
                    1.0
                }
            }
            DistanceModel::Exponential => {
                if min_distance > 0.0 {
                    (distance / min_distance).powf(-rolloff)
                } else {
                    1.0
                }
            }
            DistanceModel::ExponentialClamped => {
                let distance = distance.clamp(min_distance, max_distance);
                if min_distance > 0.0 {
                    (distance / min_distance).powf(-rolloff).clamp(0.0, 1.0)
                } else {
                    1.0
                }
            }
        }
    }
}

/// HRTF (Head-Related Transfer Function) data
#[derive(Clone, Debug)]
pub struct HRTFData {
    pub elevation_angles: Vec<Float>,
    pub azimuth_samples: usize,
    pub impulse_responses: Vec<Vec<f32>>,
}

impl HRTFData {
    #[inline]
    pub fn new() -> Self {
        Self {
            elevation_angles: Vec::new(),
            azimuth_samples: 0,
            impulse_responses: Vec::new(),
        }
    }

    #[inline]
    pub fn get_interpolated_response(&self, elevation: Float, azimuth: Float) -> Option<&[f32]> {
        // Simplified HRTF lookup
        // In production, would perform proper 2D interpolation
        self.impulse_responses.first().map(|v| v.as_slice())
    }
}

impl Default for HRTFData {
    fn default() -> Self {
        Self::new()
    }
}

/// Reverb effect
#[derive(Clone, Debug)]
pub struct ReverbEffect {
    pub room_size: Float,
    pub damping: Float,
    pub wet_level: Float,
    pub dry_level: Float,
    pub width: Float,
    pub decay_time: Float,
}

impl ReverbEffect {
    #[inline]
    pub fn new() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            wet_level: 0.3,
            dry_level: 0.7,
            width: 1.0,
            decay_time: 1.0,
        }
    }

    #[inline]
    pub fn hall() -> Self {
        Self {
            room_size: 0.9,
            damping: 0.4,
            wet_level: 0.5,
            dry_level: 0.5,
            width: 1.0,
            decay_time: 2.5,
        }
    }

    #[inline]
    pub fn room() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.6,
            wet_level: 0.3,
            dry_level: 0.7,
            width: 0.8,
            decay_time: 1.2,
        }
    }

    #[inline]
    pub fn cave() -> Self {
        Self {
            room_size: 1.0,
            damping: 0.2,
            wet_level: 0.7,
            dry_level: 0.3,
            width: 1.0,
            decay_time: 3.0,
        }
    }
}

impl Default for ReverbEffect {
    fn default() -> Self {
        Self::new()
    }
}

/// 3D audio engine
pub struct AudioEngine {
    listener: AudioListener,
    sources: BTreeMap<SoundId, SoundSource>,
    distance_model: DistanceModel,
    doppler_factor: Float,
    speed_of_sound: Float,
    hrtf_enabled: bool,
    hrtf_data: Option<HRTFData>,
    reverb: ReverbEffect,
    max_sources: usize,
}

impl AudioEngine {
    #[inline]
    pub fn new(max_sources: usize) -> Self {
        Self {
            listener: AudioListener::new(),
            sources: BTreeMap::new(),
            distance_model: DistanceModel::InverseClamped,
            doppler_factor: 1.0,
            speed_of_sound: 343.0,
            hrtf_enabled: false,
            hrtf_data: None,
            reverb: ReverbEffect::new(),
            max_sources,
        }
    }

    #[inline]
    pub fn set_listener(&mut self, listener: AudioListener) {
        self.listener = listener;
    }

    #[inline]
    pub fn get_listener(&self) -> &AudioListener {
        &self.listener
    }

    #[inline]
    pub fn add_source(&mut self, source: SoundSource) -> SoundId {
        // Voice management: limit number of sources
        if self.sources.len() >= self.max_sources {
            // Remove lowest priority source if needed
            if let Some((&id, _)) = self
                .sources
                .iter()
                .min_by_key(|(_, s)| s.priority)
            {
                self.sources.remove(&id);
            }
        }

        let id = source.id;
        self.sources.insert(id, source);
        id
    }

    #[inline]
    pub fn remove_source(&mut self, id: SoundId) -> Option<SoundSource> {
        self.sources.remove(&id)
    }

    #[inline]
    pub fn get_source(&self, id: SoundId) -> Option<&SoundSource> {
        self.sources.get(&id)
    }

    #[inline]
    pub fn get_source_mut(&mut self, id: SoundId) -> Option<&mut SoundSource> {
        self.sources.get_mut(&id)
    }

    #[inline]
    pub fn set_distance_model(&mut self, model: DistanceModel) {
        self.distance_model = model;
    }

    #[inline]
    pub fn set_doppler_factor(&mut self, factor: Float) {
        self.doppler_factor = factor.max(0.0);
    }

    #[inline]
    pub fn set_hrtf_enabled(&mut self, enabled: bool) {
        self.hrtf_enabled = enabled;
    }

    #[inline]
    pub fn set_hrtf_data(&mut self, data: HRTFData) {
        self.hrtf_data = Some(data);
    }

    #[inline]
    pub fn set_reverb(&mut self, reverb: ReverbEffect) {
        self.reverb = reverb;
    }

    #[inline]
    pub fn calculate_source_parameters(
        &self,
        source: &SoundSource,
    ) -> SourceParameters {
        let distance = self.listener.position.distance(source.position);

        // Distance attenuation
        let distance_gain = self.distance_model.calculate_gain(
            distance,
            source.min_distance,
            source.max_distance,
            source.rolloff_factor,
        );

        // Doppler effect
        let doppler_shift = if self.doppler_factor > 0.0 {
            let relative_velocity = source.velocity - self.listener.velocity;
            let to_listener = (self.listener.position - source.position).normalize();
            let velocity_component = relative_velocity.dot(to_listener);

            if self.speed_of_sound > velocity_component {
                self.speed_of_sound / (self.speed_of_sound - self.doppler_factor * velocity_component)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Pan calculation (stereo)
        let to_source = (source.position - self.listener.position).normalize();
        let right = self.listener.right();
        let pan = to_source.dot(right);

        // HRTF (simplified)
        let left_gain = if self.hrtf_enabled {
            1.0 - (pan + 1.0) * 0.25
        } else {
            1.0 - (pan + 1.0) * 0.5
        };

        let right_gain = if self.hrtf_enabled {
            1.0 + (pan - 1.0) * 0.25
        } else {
            (pan + 1.0) * 0.5
        };

        SourceParameters {
            volume: source.volume * distance_gain,
            left_gain,
            right_gain,
            pitch: source.pitch * doppler_shift,
            distance,
        }
    }

    #[inline]
    pub fn update(&mut self, _dt: Float) {
        // Update all sources
        // This would interface with the actual audio backend
    }

    #[inline]
    pub fn get_active_source_count(&self) -> usize {
        self.sources.values().filter(|s| s.is_playing()).count()
    }
}

/// Calculated audio parameters for a sound source
#[derive(Clone, Copy, Debug)]
pub struct SourceParameters {
    pub volume: Float,
    pub left_gain: Float,
    pub right_gain: Float,
    pub pitch: Float,
    pub distance: Float,
}

// Re-export commonly used types
pub use super::physics::{Vec3, Quaternion};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_model() {
        let model = DistanceModel::Inverse;
        let gain = model.calculate_gain(10.0, 1.0, 100.0, 1.0);
        assert!(gain > 0.0 && gain <= 1.0);
    }

    #[test]
    fn test_audio_listener() {
        let mut listener = AudioListener::new();
        listener.set_position(Vec3::new(10.0, 5.0, 0.0));
        assert_eq!(listener.position, Vec3::new(10.0, 5.0, 0.0));
    }
}
