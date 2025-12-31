//! # Extended Reality (XR) Subsystem
//!
//! This module provides comprehensive support for Augmented Reality (AR) and Virtual Reality (VR).
//! It implements low-latency, high-precision tracking, rendering, and interaction capabilities.
//!
//! ## Key Components
//!
//! - **SLAM**: Simultaneous Localization and Mapping
//! - **Tracking**: 6DoF pose tracking with sensor fusion
//! - **Hand Tracking**: Gesture recognition and hand pose estimation
//! - **Eye Tracking**: Gaze tracking and foveated rendering
//! - **Spatial Audio**: Binaural audio with room acoustics
//! - **Rendering**: Stereo rendering with distortion correction
//!
//! ## Performance Goals
//!
//! - Motion-to-photon latency: < 20ms
//! - Tracking accuracy: < 1mm positional, < 1° rotational
//! - Frame rate: 90Hz or 120Hz
//! - SLAM stability: < 0.1% drift per minute

pub mod slam;
pub mod tracking;
pub mod hand;
pub mod eye;
pub mod spatial_audio;
pub mod render;
pub mod types;
pub mod math;
pub mod error;

pub use error::{XrError, XrResult};
pub use types::*;

use alloc::vec::Vec;
use core::time::Duration;

/// XR subsystem configuration
#[derive(Debug, Clone)]
pub struct XrConfig {
    /// Target frame rate (Hz)
    pub target_frame_rate: u32,

    /// Maximum acceptable latency (ms)
    pub max_latency_ms: u32,

    /// Enable SLAM
    pub enable_slam: bool,

    /// Enable hand tracking
    pub enable_hand_tracking: bool,

    /// Enable eye tracking
    pub enable_eye_tracking: bool,

    /// Enable spatial audio
    pub enable_spatial_audio: bool,

    /// Enable foveated rendering
    pub enable_foveated_rendering: bool,

    /// Display resolution (per eye)
    pub display_resolution: (u32, u32),

    /// Field of view (degrees)
    pub field_of_view: (f32, f32, f32, f32), // (left, right, top, bottom)
}

impl Default for XrConfig {
    fn default() -> Self {
        Self {
            target_frame_rate: 90,
            max_latency_ms: 20,
            enable_slam: true,
            enable_hand_tracking: true,
            enable_eye_tracking: true,
            enable_spatial_audio: true,
            enable_foveated_rendering: true,
            display_resolution: (1832, 1920), // Quest 2 resolution
            field_of_view: (95.0, 95.0, 90.0, 90.0),
        }
    }
}

/// XR runtime state
pub struct XrRuntime {
    config: XrConfig,
    slam: Option<slam::SlamEngine>,
    tracking: tracking::TrackingSystem,
    hand_tracking: Option<hand::HandTracker>,
    eye_tracking: Option<eye::EyeTracker>,
    spatial_audio: Option<spatial_audio::SpatialAudioEngine>,
    renderer: render::XrRenderer,
}

impl XrRuntime {
    /// Create a new XR runtime
    pub fn new(config: XrConfig) -> XrResult<Self> {
        let tracking = tracking::TrackingSystem::new(&config)?;
        let renderer = render::XrRenderer::new(&config)?;

        let mut slam = None;
        if config.enable_slam {
            slam = Some(slam::SlamEngine::new()?);
        }

        let mut hand_tracking = None;
        if config.enable_hand_tracking {
            hand_tracking = Some(hand::HandTracker::new()?);
        }

        let mut eye_tracking = None;
        if config.enable_eye_tracking {
            eye_tracking = Some(eye::EyeTracker::new()?);
        }

        let mut spatial_audio = None;
        if config.enable_spatial_audio {
            spatial_audio = Some(spatial_audio::SpatialAudioEngine::new()?);
        }

        Ok(Self {
            config,
            slam,
            tracking,
            hand_tracking,
            eye_tracking,
            spatial_audio,
            renderer,
        })
    }

    /// Start the XR runtime
    pub fn start(&mut self) -> XrResult<()> {
        self.tracking.start()?;

        if let Some(slam) = &mut self.slam {
            slam.start()?;
        }

        if let Some(hand_tracking) = &mut self.hand_tracking {
            hand_tracking.start()?;
        }

        if let Some(eye_tracking) = &mut self.eye_tracking {
            eye_tracking.start()?;
        }

        if let Some(spatial_audio) = &mut self.spatial_audio {
            spatial_audio.start()?;
        }

        self.renderer.start()?;

        Ok(())
    }

    /// Stop the XR runtime
    pub fn stop(&mut self) -> XrResult<()> {
        self.tracking.stop()?;

        if let Some(slam) = &mut self.slam {
            slam.stop()?;
        }

        if let Some(hand_tracking) = &mut self.hand_tracking {
            hand_tracking.stop()?;
        }

        if let Some(eye_tracking) = &mut self.eye_tracking {
            eye_tracking.stop()?;
        }

        if let Some(spatial_audio) = &mut self.spatial_audio {
            spatial_audio.stop()?;
        }

        self.renderer.stop()?;

        Ok(())
    }

    /// Get current tracking state
    pub fn get_tracking_state(&self) -> XrResult<TrackingState> {
        let state = self.tracking.get_state()?;
        Ok(TrackingState {
            head_pose: state.head_pose,
            quality: state.quality,
            velocity: state.velocity,
            angular_velocity: state.angular_velocity,
            acceleration: state.acceleration,
            timestamp: state.timestamp,
        })
    }

    /// Get SLAM map (if available)
    pub fn get_slam_map(&self) -> XrResult<Option<slam::SlamMap>> {
        match &self.slam {
            Some(slam) => Ok(Some(slam.get_map()?)),
            None => Ok(None),
        }
    }

    /// Get hand poses (if available)
    pub fn get_hand_poses(&self) -> XrResult<Option<Vec<hand::HandPose>>> {
        match &self.hand_tracking {
            Some(tracker) => Ok(Some(tracker.get_poses()?)),
            None => Ok(None),
        }
    }

    /// Get eye gaze (if available)
    pub fn get_eye_gaze(&self) -> XrResult<Option<eye::GazeData>> {
        match &self.eye_tracking {
            Some(tracker) => Ok(Some(tracker.get_gaze()?)),
            None => Ok(None),
        }
    }

    /// Submit audio for spatial rendering
    pub fn submit_audio(&mut self, audio: &[f32], position: &Vector3<f32>) -> XrResult<()> {
        match &mut self.spatial_audio {
            Some(engine) => {
                engine.render_audio(audio, position)?;
                Ok(())
            }
            None => Err(XrError::NotSupported("Spatial audio not enabled".into())),
        }
    }

    /// Submit frame for rendering
    pub fn submit_frame(&mut self, frame: render::XrFrame) -> XrResult<()> {
        self.renderer.submit(frame)
    }

    /// Wait for frame timing
    pub fn wait_frame(&self) -> XrResult<FrameTiming> {
        Ok(FrameTiming {
            target_frame_rate: self.config.target_frame_rate,
            max_latency: Duration::from_millis(self.config.max_latency_ms as u64),
        })
    }
}

/// Frame timing information
#[derive(Debug, Clone)]
pub struct FrameTiming {
    pub target_frame_rate: u32,
    pub max_latency: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = XrConfig::default();
        assert_eq!(config.target_frame_rate, 90);
        assert_eq!(config.max_latency_ms, 20);
    }

    #[test]
    fn test_runtime_creation() {
        let config = XrConfig {
            enable_slam: false,
            enable_hand_tracking: false,
            enable_eye_tracking: false,
            enable_spatial_audio: false,
            ..Default::default()
        };

        let runtime = XrRuntime::new(config);
        assert!(runtime.is_ok());
    }
}
