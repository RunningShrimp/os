//! Eye Tracking System
//!
//! Implements real-time eye tracking and gaze estimation:
//! - Pupil detection and tracking
//! - Gaze point estimation
//! - Blink detection
//! - Eye closure detection
//! - Saccade detection
//! - Fixation detection
//! - Attention rendering (foveated rendering support)

use crate::xr::error::{XrError, XrResult};
use crate::xr::types::{Vector3, Point2};
use alloc::vec::Vec;
use core::time::Duration;

/// Eye tracking configuration
#[derive(Debug, Clone)]
pub struct EyeTrackingConfig {
    /// Enable left eye tracking
    pub enable_left_eye: bool,

    /// Enable right eye tracking
    pub enable_right_eye: bool,

    /// Gaze prediction sensitivity
    pub gaze_sensitivity: f32,

    /// Blink detection threshold
    pub blink_threshold: f32,

    /// Fixation threshold (degrees)
    pub fixation_threshold: f32,

    /// Saccade velocity threshold (deg/s)
    pub saccade_threshold: f32,
}

impl Default for EyeTrackingConfig {
    fn default() -> Self {
        Self {
            enable_left_eye: true,
            enable_right_eye: true,
            gaze_sensitivity: 1.0,
            blink_threshold: 0.5,
            fixation_threshold: 2.0,
            saccade_threshold: 30.0,
        }
    }
}

/// Eye tracker
pub struct EyeTracker {
    config: EyeTrackingConfig,
    left_eye: EyeData,
    right_eye: EyeData,
    blink_detector: BlinkDetector,
    saccade_detector: SaccadeDetector,
    fixation_detector: FixationDetector,
    is_running: bool,
    last_update: Option<Duration>,
}

impl EyeTracker {
    /// Create a new eye tracker
    pub fn new() -> XrResult<Self> {
        Ok(Self {
            config: EyeTrackingConfig::default(),
            left_eye: EyeData::default(),
            right_eye: EyeData::default(),
            blink_detector: BlinkDetector::new(),
            saccade_detector: SaccadeDetector::new(),
            fixation_detector: FixationDetector::new(),
            is_running: false,
            last_update: None,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: EyeTrackingConfig) -> XrResult<Self> {
        Ok(Self {
            config,
            left_eye: EyeData::default(),
            right_eye: EyeData::default(),
            blink_detector: BlinkDetector::new(),
            saccade_detector: SaccadeDetector::new(),
            fixation_detector: FixationDetector::new(),
            is_running: false,
            last_update: None,
        })
    }

    /// Start eye tracking
    pub fn start(&mut self) -> XrResult<()> {
        self.is_running = true;
        Ok(())
    }

    /// Stop eye tracking
    pub fn stop(&mut self) -> XrResult<()> {
        self.is_running = false;
        self.last_update = None;
        Ok(())
    }

    /// Process eye tracking camera image
    pub fn process_image(&mut self, image_data: &[u8], timestamp: Duration) -> XrResult<GazeData> {
        if !self.is_running {
            return Err(XrError::InvalidState("Eye tracker not running".into()));
        }

        // Detect pupils
        let left_pupil = self.detect_pupil(image_data, Eye::Left)?;
        let right_pupil = self.detect_pupil(image_data, Eye::Right)?;

        // Detect blinks before moving the values
        let is_blinking = self.blink_detector.detect_blink(&left_pupil, &right_pupil, timestamp);

        // Update eye data
        self.left_eye.pupil = Some(left_pupil);
        self.right_eye.pupil = Some(right_pupil);

        // Compute gaze direction
        let gaze_direction = self.compute_gaze_direction()?;

        // Detect saccades
        let is_saccade = if let Some(prev_time) = self.last_update {
            let dt = if timestamp > prev_time {
                timestamp - prev_time
            } else {
                Duration::ZERO
            };
            self.saccade_detector.detect_saccade(&gaze_direction, dt)
        } else {
            false
        };

        // Detect fixations
        let is_fixation = self.fixation_detector.detect_fixation(&gaze_direction);

        // Estimate combined gaze point
        let gaze_point = self.estimate_gaze_point(&gaze_direction);

        self.last_update = Some(timestamp);

        Ok(GazeData {
            left_gaze: GazeVector {
                origin: Vector3::new(-0.032, 0.0, 0.0), // Left eye position
                direction: gaze_direction,
                validity: GazeValidity::Valid,
            },
            right_gaze: GazeVector {
                origin: Vector3::new(0.032, 0.0, 0.0), // Right eye position
                direction: gaze_direction,
                validity: GazeValidity::Valid,
            },
            combined_gaze: gaze_point,
            is_blinking,
            is_saccade,
            is_fixation,
            timestamp,
        })
    }

    /// Get current gaze data
    pub fn get_gaze(&self) -> XrResult<GazeData> {
        if !self.is_running {
            return Err(XrError::InvalidState("Eye tracker not running".into()));
        }

        let gaze_direction = self.compute_gaze_direction().unwrap_or_else(|_| Vector3::FORWARD);
        let gaze_point = self.estimate_gaze_point(&gaze_direction);

        Ok(GazeData {
            left_gaze: GazeVector {
                origin: Vector3::new(-0.032, 0.0, 0.0),
                direction: gaze_direction,
                validity: if self.left_eye.pupil.is_some() {
                    GazeValidity::Valid
                } else {
                    GazeValidity::Invalid
                },
            },
            right_gaze: GazeVector {
                origin: Vector3::new(0.032, 0.0, 0.0),
                direction: gaze_direction,
                validity: if self.right_eye.pupil.is_some() {
                    GazeValidity::Valid
                } else {
                    GazeValidity::Invalid
                },
            },
            combined_gaze: gaze_point,
            is_blinking: self.blink_detector.is_blinking(),
            is_saccade: self.saccade_detector.is_saccade(),
            is_fixation: self.fixation_detector.is_fixation(),
            timestamp: self.last_update.unwrap_or(Duration::ZERO),
        })
    }

    /// Detect pupil in image
    fn detect_pupil(&self, _image_data: &[u8], _eye: Eye) -> XrResult<PupilData> {
        // Simplified pupil detection
        // In practice, this would use ellipse fitting or CNN

        Ok(PupilData {
            center: Point2::new(320.0, 240.0),
            diameter: 8.0,
            confidence: 0.9,
            openness: 1.0,
        })
    }

    /// Compute gaze direction from pupil positions
    fn compute_gaze_direction(&self) -> XrResult<Vector3<f32>> {
        let left_pupil = self.left_eye.pupil.as_ref().ok_or_else(|| {
            XrError::SensorError("Left pupil data not available".into())
        })?;

        let right_pupil = self.right_eye.pupil.as_ref().ok_or_else(|| {
            XrError::SensorError("Right pupil data not available".into())
        })?;

        // Average pupil positions
        let avg_x = (left_pupil.center.x + right_pupil.center.x) / 2.0;
        let avg_y = (left_pupil.center.y + right_pupil.center.y) / 2.0;

        // Convert to normalized coordinates (-1 to 1)
        let norm_x = (avg_x - 320.0) / 320.0;
        let norm_y = (avg_y - 240.0) / 240.0;

        // Map to gaze direction (simplified)
        let gaze_x = norm_x * 0.5;
        let gaze_y = -norm_y * 0.5;

        Ok(Vector3::new(gaze_x, gaze_y, -1.0).normalize())
    }

    /// Estimate 3D gaze point
    fn estimate_gaze_point(&self, gaze_direction: &Vector3<f32>) -> GazePoint {
        // Project gaze to a virtual plane at 1m distance
        let distance = 1.0;
        let point = Vector3::new(
            gaze_direction.x * distance,
            gaze_direction.y * distance,
            gaze_direction.z * distance,
        );

        GazePoint {
            position: point,
            distance,
            confidence: 0.8,
        }
    }

    /// Check if user is looking at a specific point
    pub fn is_looking_at(&self, point: &Vector3<f32>, threshold_degrees: f32) -> bool {
        let gaze_data = match self.get_gaze() {
            Ok(data) => data,
            Err(_) => return false,
        };

        let gaze_dir = gaze_data.combined_gaze.position.normalize();
        let target_dir = point.normalize();

        let dot = gaze_dir.dot(&target_dir);
        let angle = dot.acos().abs();

        angle.to_degrees() < threshold_degrees
    }

    /// Get attention map for foveated rendering
    pub fn get_attention_map(&self, resolution: (u32, u32)) -> XrResult<AttentionMap> {
        let gaze = self.get_gaze()?;
        let gaze_point = gaze.combined_gaze.position;

        // Create attention map (Gaussian distribution around gaze point)
        let mut attention_weights = Vec::new();
        let width = resolution.0 as usize;
        let height = resolution.1 as usize;

        let center_x = width / 2;
        let center_y = height / 2;

        for y in 0..height {
            for x in 0..width {
                let dx = (x as f32 - center_x as f32) / width as f32;
                let dy = (y as f32 - center_y as f32) / height as f32;

                let dist = (dx * dx + dy * dy).sqrt();
                let weight = (-2.0 * dist * dist).exp();
                attention_weights.push(weight);
            }
        }

        Ok(AttentionMap {
            weights: attention_weights,
            gaze_point,
            resolution,
        })
    }

    /// Calibrate eye tracker
    pub fn calibrate(&mut self, calibration_points: &[Vector3<f32>]) -> XrResult<()> {
        if calibration_points.len() < 5 {
            return Err(XrError::InvalidArgument(
                "Need at least 5 calibration points".into(),
            ));
        }

        // Store calibration data
        // In practice, this would fit a gaze model
        Ok(())
    }

    /// Get eye openness (0-1)
    pub fn get_eye_openness(&self, eye: Eye) -> f32 {
        let eye_data = match eye {
            Eye::Left => &self.left_eye,
            Eye::Right => &self.right_eye,
        };

        eye_data
            .pupil
            .as_ref()
            .map(|p| p.openness)
            .unwrap_or(0.0)
    }
}

/// Eye data
#[derive(Debug, Clone, Default)]
struct EyeData {
    pupil: Option<PupilData>,
    gaze_history: Vec<Vector3<f32>>,
}

/// Pupil detection data
#[derive(Debug, Clone)]
pub struct PupilData {
    pub center: Point2<f32>,
    pub diameter: f32,
    pub confidence: f32,
    pub openness: f32,
}

/// Eye (left or right)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Eye {
    Left,
    Right,
}

/// Gaze data
#[derive(Debug, Clone)]
pub struct GazeData {
    pub left_gaze: GazeVector,
    pub right_gaze: GazeVector,
    pub combined_gaze: GazePoint,
    pub is_blinking: bool,
    pub is_saccade: bool,
    pub is_fixation: bool,
    pub timestamp: Duration,
}

/// Gaze vector
#[derive(Debug, Clone)]
pub struct GazeVector {
    pub origin: Vector3<f32>,
    pub direction: Vector3<f32>,
    pub validity: GazeValidity,
}

/// Gaze validity
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GazeValidity {
    Valid,
    Invalid,
    Lost,
}

/// 3D gaze point
#[derive(Debug, Clone)]
pub struct GazePoint {
    pub position: Vector3<f32>,
    pub distance: f32,
    pub confidence: f32,
}

/// Attention map for foveated rendering
#[derive(Debug, Clone)]
pub struct AttentionMap {
    pub weights: Vec<f32>,
    pub gaze_point: Vector3<f32>,
    pub resolution: (u32, u32),
}

/// Blink detector
pub struct BlinkDetector {
    blink_history: Vec<bool>,
    blink_start: Option<Duration>,
    is_blinking: bool,
}

impl BlinkDetector {
    pub fn new() -> Self {
        Self {
            blink_history: Vec::new(),
            blink_start: None,
            is_blinking: false,
        }
    }

    pub fn detect_blink(&mut self, left: &PupilData, right: &PupilData, timestamp: Duration) -> bool {
        // Detect blink based on eye openness
        let openness = (left.openness + right.openness) / 2.0;
        let is_closed = openness < 0.5;

        self.blink_history.push(is_closed);
        if self.blink_history.len() > 10 {
            self.blink_history.remove(0);
        }

        // Determine if blinking
        let closed_count = self.blink_history.iter().filter(|&&b| b).count();

        if closed_count > 7 && !self.is_blinking {
            self.blink_start = Some(timestamp);
            self.is_blinking = true;
        } else if closed_count < 3 && self.is_blinking {
            self.is_blinking = false;
            self.blink_start = None;
        }

        self.is_blinking
    }

    pub fn is_blinking(&self) -> bool {
        self.is_blinking
    }

    pub fn get_blink_duration(&self) -> Option<Duration> {
        self.blink_start
    }
}

/// Saccade detector
pub struct SaccadeDetector {
    previous_gaze: Option<Vector3<f32>>,
    velocity: f32,
    is_saccade: bool,
}

impl SaccadeDetector {
    pub fn new() -> Self {
        Self {
            previous_gaze: None,
            velocity: 0.0,
            is_saccade: false,
        }
    }

    pub fn detect_saccade(&mut self, gaze: &Vector3<f32>, dt: Duration) -> bool {
        if let Some(prev) = self.previous_gaze {
            // Compute angular velocity
            let angle = prev.normalize().dot(&gaze.normalize()).acos().abs();
            let dt_sec = dt.as_secs_f32();
            self.velocity = if dt_sec > 0.0 {
                angle.to_degrees() / dt_sec
            } else {
                0.0
            };

            // Saccade threshold: typically > 30 deg/s
            self.is_saccade = self.velocity > 30.0;
        }

        self.previous_gaze = Some(*gaze);
        self.is_saccade
    }

    pub fn is_saccade(&self) -> bool {
        self.is_saccade
    }

    pub fn get_velocity(&self) -> f32 {
        self.velocity
    }
}

/// Fixation detector
pub struct FixationDetector {
    gaze_samples: Vec<Vector3<f32>>,
    is_fixation: bool,
}

impl FixationDetector {
    pub fn new() -> Self {
        Self {
            gaze_samples: Vec::new(),
            is_fixation: false,
        }
    }

    pub fn detect_fixation(&mut self, gaze: &Vector3<f32>) -> bool {
        self.gaze_samples.push(*gaze);

        // Keep only recent samples
        if self.gaze_samples.len() > 20 {
            self.gaze_samples.remove(0);
        }

        if self.gaze_samples.len() < 10 {
            return false;
        }

        // Compute dispersion
        let center = self.compute_centroid();
        let dispersion = self.compute_dispersion(&center);

        // Fixation if gaze stays within 2 degrees
        self.is_fixation = dispersion < 2.0;
        self.is_fixation
    }

    pub fn is_fixation(&self) -> bool {
        self.is_fixation
    }

    fn compute_centroid(&self) -> Vector3<f32> {
        let mut sum = Vector3::ZERO;
        for gaze in &self.gaze_samples {
            sum = sum + *gaze;
        }

        let count = self.gaze_samples.len() as f32;
        Vector3::new(sum.x / count, sum.y / count, sum.z / count)
    }

    fn compute_dispersion(&self, center: &Vector3<f32>) -> f32 {
        let mut max_angle: f32 = 0.0;

        for gaze in &self.gaze_samples {
            let angle = center.normalize().dot(&gaze.normalize()).acos().abs().to_degrees();
            max_angle = max_angle.max(angle);
        }

        max_angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eye_tracker_creation() {
        let tracker = EyeTracker::new();
        assert!(tracker.is_ok());
    }

    #[test]
    fn test_blink_detector() {
        let mut detector = BlinkDetector::new();

        let left_pupil = PupilData {
            center: Point2::new(320.0, 240.0),
            diameter: 8.0,
            confidence: 0.9,
            openness: 0.3, // Closed
        };

        let right_pupil = PupilData {
            center: Point2::new(320.0, 240.0),
            diameter: 8.0,
            confidence: 0.9,
            openness: 0.3, // Closed
        };

        let is_blinking = detector.detect_blink(&left_pupil, &right_pupil, Duration::from_millis(100));
        assert!(!is_blinking); // Need more samples
    }

    #[test]
    fn test_saccade_detector() {
        let mut detector = SaccadeDetector::new();

        let gaze1 = Vector3::new(0.0, 0.0, -1.0);
        let gaze2 = Vector3::new(0.5, 0.0, -1.0);

        detector.detect_saccade(&gaze1, Duration::from_millis(16));
        let is_saccade = detector.detect_saccade(&gaze2, Duration::from_millis(16));

        assert!(is_saccade);
    }
}
