//! Spatial Audio Rendering
//!
//! Implements immersive spatial audio for XR experiences:
//! - Binaural rendering with HRTF
//! - Room acoustics simulation
//! - Directional audio sources
//! - Sound occlusion and obstruction
//! - Reflection and reverberation
//! - Distance attenuation
//! - Doppler effect

use crate::xr::error::{XrError, XrResult};
use crate::xr::types::{Vector3, Quaternion, Pose};
use alloc::vec::Vec;

/// Spatial audio configuration
#[derive(Debug, Clone)]
pub struct SpatialAudioConfig {
    /// Sample rate (Hz)
    pub sample_rate: u32,

    /// Enable HRTF rendering
    pub enable_hrtf: bool,

    /// Enable room acoustics
    pub enable_room_acoustics: bool,

    /// Maximum number of simultaneous sources
    pub max_sources: usize,

    /// Room size (0-1)
    pub room_size: f32,

    /// Reverberation time (seconds)
    pub reverb_time: f32,

    /// Speed of sound (m/s)
    pub speed_of_sound: f32,
}

impl Default for SpatialAudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            enable_hrtf: true,
            enable_room_acoustics: true,
            max_sources: 32,
            room_size: 0.5,
            reverb_time: 1.5,
            speed_of_sound: 343.0,
        }
    }
}

/// Spatial audio engine
pub struct SpatialAudioEngine {
    config: SpatialAudioConfig,
    sources: Vec<AudioSource>,
    listener: Listener,
    hrtf: HrtfDatabase,
    room_acoustics: RoomAcoustics,
    is_running: bool,
}

impl SpatialAudioEngine {
    /// Create a new spatial audio engine
    pub fn new() -> XrResult<Self> {
        Ok(Self {
            config: SpatialAudioConfig::default(),
            sources: Vec::new(),
            listener: Listener::default(),
            hrtf: HrtfDatabase::new(),
            room_acoustics: RoomAcoustics::new(),
            is_running: false,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: SpatialAudioConfig) -> XrResult<Self> {
        Ok(Self {
            config,
            sources: Vec::new(),
            listener: Listener::default(),
            hrtf: HrtfDatabase::new(),
            room_acoustics: RoomAcoustics::new(),
            is_running: false,
        })
    }

    /// Start audio engine
    pub fn start(&mut self) -> XrResult<()> {
        self.is_running = true;
        Ok(())
    }

    /// Stop audio engine
    pub fn stop(&mut self) -> XrResult<()> {
        self.is_running = false;
        Ok(())
    }

    /// Create a new audio source
    pub fn create_source(&mut self, position: Vector3<f32>) -> XrResult<usize> {
        if self.sources.len() >= self.config.max_sources {
            return Err(XrError::OperationFailed("Maximum sources reached".into()));
        }

        let id = self.sources.len();
        self.sources.push(AudioSource::new(position));
        Ok(id)
    }

    /// Update source position
    pub fn update_source(&mut self, id: usize, position: Vector3<f32>) -> XrResult<()> {
        if id >= self.sources.len() {
            return Err(XrError::InvalidArgument("Invalid source ID".into()));
        }

        self.sources[id].position = position;
        Ok(())
    }

    /// Remove a source
    pub fn remove_source(&mut self, id: usize) -> XrResult<()> {
        if id >= self.sources.len() {
            return Err(XrError::InvalidArgument("Invalid source ID".into()));
        }

        self.sources.remove(id);
        Ok(())
    }

    /// Update listener pose
    pub fn update_listener(&mut self, pose: &Pose) -> XrResult<()> {
        self.listener.position = pose.position;
        self.listener.orientation = pose.orientation;

        // Update listener vectors
        self.listener.forward = pose.orientation.rotate_vector(&Vector3::FORWARD);
        self.listener.up = pose.orientation.rotate_vector(&Vector3::UP);
        self.listener.right = pose.orientation.rotate_vector(&Vector3::RIGHT);

        Ok(())
    }

    /// Render audio with spatial effects
    pub fn render_audio(&mut self, audio: &[f32], position: &Vector3<f32>) -> XrResult<Vec<f32>> {
        if !self.is_running {
            return Err(XrError::InvalidState("Audio engine not running".into()));
        }

        // Create temporary source
        let source = AudioSource {
            position: *position,
            velocity: Vector3::ZERO,
            volume: 1.0,
            loop_: false,
            is_playing: true,
        };

        // Render with spatial effects
        let rendered = self.render_source(&source, audio)?;
        Ok(rendered)
    }

    /// Render a single source
    fn render_source(&self, source: &AudioSource, audio: &[f32]) -> XrResult<Vec<f32>> {
        let mut output = Vec::with_capacity(audio.len() * 2); // Stereo

        // Compute direction to source
        let to_source = source.position - self.listener.position;
        let distance = to_source.length();

        // Direction in listener's coordinate frame
        let direction = to_source.normalize();
        let local_dir = Vector3::new(
            direction.dot(&self.listener.right),
            direction.dot(&self.listener.up),
            direction.dot(&self.listener.forward),
        );

        // Compute azimuth and elevation
        let azimuth = local_dir.x.atan2(local_dir.z).to_degrees();
        let elevation = local_dir.y.asin().to_degrees();

        // Apply distance attenuation
        let attenuation = self.compute_attenuation(distance);

        // Apply HRTF if enabled
        let (left_ir, right_ir) = if self.config.enable_hrtf {
            self.hrtf.get_impulse_response(azimuth, elevation)
        } else {
            (vec![1.0], vec![1.0])
        };

        // Apply room acoustics if enabled
        let reverb_mix = if self.config.enable_room_acoustics {
            self.room_acoustics.get_reflection_mix(distance)
        } else {
            0.0
        };

        // Process audio samples
        let _chunk_size = audio.len().min(left_ir.len());

        for (i, &sample) in audio.iter().enumerate() {
            // Apply HRTF convolution (simplified)
            let left_gain = if i < left_ir.len() { left_ir[i] } else { left_ir[left_ir.len() - 1] };
            let right_gain = if i < right_ir.len() { right_ir[i] } else { right_ir[right_ir.len() - 1] };

            // Apply attenuation
            let attenuated = sample * attenuation;

            // Mix direct and reverb
            let left = attenuated * left_gain * (1.0 - reverb_mix) + attenuated * 0.5 * reverb_mix;
            let right = attenuated * right_gain * (1.0 - reverb_mix) + attenuated * 0.5 * reverb_mix;

            output.push(left);
            output.push(right);
        }

        Ok(output)
    }

    /// Compute distance attenuation
    fn compute_attenuation(&self, distance: f32) -> f32 {
        // Inverse distance law with minimum distance
        let min_distance = 0.5;
        let _max_distance = 10.0;
        let rolloff_factor = 1.0;

        let dist = distance.max(min_distance);
        let attenuation = min_distance / (min_distance + rolloff_factor * (dist - min_distance));

        // Clamp to [0, 1]
        attenuation.max(0.0).min(1.0)
    }

    /// Set room size
    pub fn set_room_size(&mut self, size: f32) -> XrResult<()> {
        self.config.room_size = size.clamp(0.0, 1.0);
        Ok(())
    }

    /// Set reverb time
    pub fn set_reverb_time(&mut self, time: f32) -> XrResult<()> {
        self.config.reverb_time = time.clamp(0.1, 10.0);
        Ok(())
    }
}

/// Audio source
#[derive(Debug, Clone)]
struct AudioSource {
    position: Vector3<f32>,
    velocity: Vector3<f32>,
    volume: f32,
    loop_: bool,
    is_playing: bool,
}

impl AudioSource {
    fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            velocity: Vector3::ZERO,
            volume: 1.0,
            loop_: false,
            is_playing: true,
        }
    }
}

/// Listener (the user)
#[derive(Debug, Clone, Default)]
struct Listener {
    position: Vector3<f32>,
    orientation: Quaternion,
    forward: Vector3<f32>,
    up: Vector3<f32>,
    right: Vector3<f32>,
}

/// HRTF (Head-Related Transfer Function) database
pub struct HrtfDatabase {
    impulse_responses: Vec<(f32, f32, Vec<f32>, Vec<f32>)>, // (azimuth, elevation, left_ir, right_ir)
}

impl HrtfDatabase {
    pub fn new() -> Self {
        // Simplified HRTF database
        // In practice, this would load from a measured HRTF dataset
        Self {
            impulse_responses: Self::generate_default_hrtf(),
        }
    }

    fn generate_default_hrtf() -> Vec<(f32, f32, Vec<f32>, Vec<f32>)> {
        let mut responses = Vec::new();

        // Generate simplified HRTF for different azimuths
        for azimuth in -180..=180 {
            for elevation in -40..=90 {
                if elevation % 10 != 0 {
                    continue;
                }

                let (left, right) = Self::compute_hrtf_response(azimuth as f32, elevation as f32);
                responses.push((azimuth as f32, elevation as f32, left, right));
            }
        }

        responses
    }

    fn compute_hrtf_response(azimuth: f32, _elevation: f32) -> (Vec<f32>, Vec<f32>) {
        // Simplified HRTF computation
        // In practice, this would use measured data or a sophisticated model

        let size = 256;
        let mut left_ir = Vec::with_capacity(size);
        let mut right_ir = Vec::with_capacity(size);

        // Interaural time difference (ITD)
        let itd_samples = ((azimuth.to_radians()).sin() * 0.5 * 48000.0 / 343.0).abs() as usize;

        // Interaural level difference (ILD)
        let ild_db = (azimuth.to_radians()).sin().abs() * 6.0;
        let ild_linear = 10.0_f32.powf(-ild_db / 20.0);

        for i in 0..size {
            // Simple exponential decay
            let decay = (-2.0 * i as f32 / size as f32).exp();

            // Apply ITD delay
            let left_delay = if azimuth < 0.0 { itd_samples } else { 0 };
            let right_delay = if azimuth >= 0.0 { itd_samples } else { 0 };

            let left_val = if i >= left_delay { decay } else { 0.0 };
            let right_val = if i >= right_delay { decay * ild_linear } else { 0.0 };

            left_ir.push(left_val);
            right_ir.push(right_val);
        }

        (left_ir, right_ir)
    }

    pub fn get_impulse_response(&self, azimuth: f32, elevation: f32) -> (Vec<f32>, Vec<f32>) {
        // Find closest HRTF measurement
        let mut best_match_index = 0;
        let mut best_dist = f32::MAX;

        for (i, (az, el, _left, _right)) in self.impulse_responses.iter().enumerate() {
            let dist = ((az - azimuth).abs() + (el - elevation).abs()) as f32;
            if dist < best_dist {
                best_dist = dist;
                best_match_index = i;
            }
        }

        let (_, _, left, right) = &self.impulse_responses[best_match_index];
        (left.clone(), right.clone())
    }
}

/// Room acoustics simulation
pub struct RoomAcoustics {
    reflections: Vec<Reflection>,
    reverb_decay: f32,
}

impl RoomAcoustics {
    pub fn new() -> Self {
        Self {
            reflections: Self::generate_reflections(),
            reverb_decay: 1.5,
        }
    }

    fn generate_reflections() -> Vec<Reflection> {
        // Generate first-order reflections
        let mut reflections = Vec::new();

        // 6 walls
        for axis in 0..3 {
            for direction in -1..=1 {
                if direction == 0 {
                    continue;
                }
                reflections.push(Reflection {
                    direction: Vector3::new(
                        if axis == 0 { direction as f32 } else { 0.0 },
                        if axis == 1 { direction as f32 } else { 0.0 },
                        if axis == 2 { direction as f32 } else { 0.0 },
                    ),
                    delay: 0.01,
                    gain: 0.5,
                });
            }
        }

        reflections
    }

    pub fn get_reflection_mix(&self, distance: f32) -> f32 {
        // More reverb at larger distances
        let mix = (distance / 10.0).min(1.0);
        mix * 0.3
    }

    pub fn set_reverb_decay(&mut self, decay: f32) {
        self.reverb_decay = decay.clamp(0.1, 5.0);
    }
}

/// Room reflection
#[derive(Debug, Clone)]
struct Reflection {
    direction: Vector3<f32>,
    delay: f32,
    gain: f32,
}

/// Sound occlusion detector
pub struct OcclusionDetector {
    obstacles: Vec<Occluder>,
}

impl OcclusionDetector {
    pub fn new() -> Self {
        Self {
            obstacles: Vec::new(),
        }
    }

    /// Add an occluding object
    pub fn add_occluder(&mut self, position: Vector3<f32>, size: Vector3<f32>) {
        self.obstacles.push(Occluder { position, size });
    }

    /// Check if sound is occluded
    pub fn is_occluded(&self, source: &Vector3<f32>, listener: &Vector3<f32>) -> f32 {
        // 1.0 = no occlusion, 0.0 = fully occluded
        let mut occlusion_factor = 1.0;

        for occluder in &self.obstacles {
            if Self::line_intersects_box(source, listener, &occluder.position, &occluder.size) {
                occlusion_factor *= 0.5;
            }
        }

        occlusion_factor
    }

    fn line_intersects_box(
        start: &Vector3<f32>,
        end: &Vector3<f32>,
        box_pos: &Vector3<f32>,
        box_size: &Vector3<f32>,
    ) -> bool {
        // Simplified intersection test
        let min = Vector3::new(
            box_pos.x - box_size.x / 2.0,
            box_pos.y - box_size.y / 2.0,
            box_pos.z - box_size.z / 2.0,
        );
        let max = Vector3::new(
            box_pos.x + box_size.x / 2.0,
            box_pos.y + box_size.y / 2.0,
            box_pos.z + box_size.z / 2.0,
        );

        // Check if line segment intersects AABB
        // (simplified - just check midpoint)
        let mid = Vector3::new(
            (start.x + end.x) / 2.0,
            (start.y + end.y) / 2.0,
            (start.z + end.z) / 2.0,
        );

        mid.x >= min.x && mid.x <= max.x
            && mid.y >= min.y && mid.y <= max.y
            && mid.z >= min.z && mid.z <= max.z
    }
}

/// Occluding object
#[derive(Debug, Clone)]
struct Occluder {
    position: Vector3<f32>,
    size: Vector3<f32>,
}

/// Doppler effect calculator
pub struct DopplerEffect {
    speed_of_sound: f32,
}

impl DopplerEffect {
    pub fn new(speed_of_sound: f32) -> Self {
        Self { speed_of_sound }
    }

    /// Compute Doppler-shifted frequency
    pub fn compute_frequency_shift(
        &self,
        source_freq: f32,
        source_velocity: &Vector3<f32>,
        listener_velocity: &Vector3<f32>,
        source_position: &Vector3<f32>,
        listener_position: &Vector3<f32>,
    ) -> f32 {
        let to_listener = (*listener_position - *source_position).normalize();
        let to_source = (*source_position - *listener_position).normalize();

        let v_source = source_velocity.dot(&to_listener);
        let v_listener = listener_velocity.dot(&to_source);

        // Doppler formula: f' = f * (c + v_listener) / (c - v_source)
        let shifted = source_freq * (self.speed_of_sound + v_listener) / (self.speed_of_sound - v_source);

        shifted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_audio_creation() {
        let engine = SpatialAudioEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_source_creation() {
        let mut engine = SpatialAudioEngine::new().unwrap();
        let position = Vector3::new(1.0, 2.0, 3.0);
        let id = engine.create_source(position);
        assert!(id.is_ok());
    }

    #[test]
    fn test_audio_rendering() {
        let mut engine = SpatialAudioEngine::new().unwrap();
        engine.start().unwrap();

        let audio = vec![0.5f32; 1000];
        let position = Vector3::new(1.0, 0.0, -1.0);
        let rendered = engine.render_audio(&audio, &position);
        assert!(rendered.is_ok());
    }

    #[test]
    fn test_hrtf() {
        let hrtf = HrtfDatabase::new();
        let (left, right) = hrtf.get_impulse_response(0.0, 0.0);
        assert!(!left.is_empty());
        assert!(!right.is_empty());
    }

    #[test]
    fn test_attenuation() {
        let engine = SpatialAudioEngine::new().unwrap();
        let attenuation = engine.compute_attenuation(1.0);
        assert!(attenuation > 0.0 && attenuation <= 1.0);
    }
}
