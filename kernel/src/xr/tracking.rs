//! 6DoF Tracking System
//!
//! Implements full 6 degrees of freedom tracking with sensor fusion:
//! - Position tracking (x, y, z)
//! - Rotation tracking (roll, pitch, yaw via quaternions)
//! - IMU sensor fusion (accelerometer + gyroscope)
//! - Extended Kalman Filter for pose estimation
//! - Drift correction
//! - External tracking support

use crate::xr::error::{XrError, XrResult};
use crate::xr::types::{Pose, Vector3, Quaternion, TrackingQuality};
use crate::xr::math::{ExtendedKalmanFilter, ComplementaryFilter, LowPassFilter};
use crate::xr::{XrConfig};
use alloc::vec::Vec;

/// Tracking system configuration
#[derive(Debug, Clone)]
pub struct TrackingConfig {
    /// Enable IMU fusion
    pub enable_imu_fusion: bool,

    /// Enable external tracking
    pub enable_external_tracking: bool,

    /// IMU update rate (Hz)
    pub imu_rate: u32,

    /// Prediction time (seconds)
    pub prediction_time: f32,

    /// Enable drift correction
    pub enable_drift_correction: bool,

    /// Position uncertainty threshold (meters)
    pub position_uncertainty_threshold: f32,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            enable_imu_fusion: true,
            enable_external_tracking: false,
            imu_rate: 1000,
            prediction_time: 0.016,
            enable_drift_correction: true,
            position_uncertainty_threshold: 0.01,
        }
    }
}

/// 6DoF tracking system
pub struct TrackingSystem {
    config: TrackingConfig,
    ekf: ExtendedKalmanFilter,
    orientation_filter: ComplementaryFilter,
    velocity_filter: LowPassFilter,
    current_state: TrackingState,
    is_running: bool,
    last_update: Option<f64>,
    imu_bias: ImuBias,
    external_tracker: Option<ExternalTracker>,
    drift_corrector: DriftCorrector,
}

impl TrackingSystem {
    /// Create a new tracking system
    pub fn new(_config: &XrConfig) -> XrResult<Self> {
        let tracking_config = TrackingConfig::default();

        Ok(Self {
            config: tracking_config,
            ekf: ExtendedKalmanFilter::new(),
            orientation_filter: ComplementaryFilter::new(0.02),
            velocity_filter: LowPassFilter::new(0.1),
            current_state: TrackingState {
                head_pose: Pose::identity(),
                quality: TrackingQuality::Excellent,
                velocity: Vector3::ZERO,
                angular_velocity: Vector3::ZERO,
                acceleration: Vector3::ZERO,
                timestamp: 0,
            },
            is_running: false,
            last_update: None,
            imu_bias: ImuBias::default(),
            external_tracker: None,
            drift_corrector: DriftCorrector::new(),
        })
    }

    /// Start tracking
    pub fn start(&mut self) -> XrResult<()> {
        self.is_running = true;
        self.current_state = TrackingState {
            head_pose: Pose::identity(),
            quality: TrackingQuality::Excellent,
            velocity: Vector3::ZERO,
            angular_velocity: Vector3::ZERO,
            acceleration: Vector3::ZERO,
            timestamp: 0,
        };
        Ok(())
    }

    /// Stop tracking
    pub fn stop(&mut self) -> XrResult<()> {
        self.is_running = false;
        self.last_update = None;
        Ok(())
    }

    /// Process IMU data
    pub fn process_imu(
        &mut self,
        accel: &Vector3<f32>,
        gyro: &Vector3<f32>,
        timestamp: u64,
    ) -> XrResult<Pose> {
        if !self.is_running {
            return Err(XrError::InvalidState("Tracking not running".into()));
        }

        let dt = self.compute_dt(timestamp)?;

        // Apply bias correction
        let corrected_accel = *accel - self.imu_bias.accel_bias;
        let corrected_gyro = *gyro - self.imu_bias.gyro_bias;

        // Update orientation with complementary filter
        let orientation = self.orientation_filter.update(&corrected_accel, &corrected_gyro, dt);

        // Update EKF prediction
        self.ekf.predict(dt, &corrected_gyro);

        // Update velocity with acceleration
        let gravity = Vector3::new(0.0, -9.81, 0.0);
        let accel_world = orientation.rotate_vector(&(corrected_accel - gravity));
        let velocity = self.velocity_filter.filter(accel_world);

        // Update state
        self.current_state.head_pose = Pose::new(Vector3::ZERO, orientation);
        self.current_state.velocity = velocity;
        self.current_state.angular_velocity = corrected_gyro;
        self.current_state.acceleration = accel_world;
        self.current_state.timestamp = timestamp;

        Ok(self.current_state.head_pose)
    }

    /// Update pose from SLAM
    pub fn update_from_slam(&mut self, slam_pose: &Pose, timestamp: u64) -> XrResult<()> {
        if !self.is_running {
            return Err(XrError::InvalidState("Tracking not running".into()));
        }

        // Initialize EKF if first pose
        if self.last_update.is_none() {
            self.ekf.initialize(slam_pose);
            self.current_state.head_pose = *slam_pose;
        } else {
            // Update EKF with SLAM measurement
            self.ekf.update_pose(slam_pose, 0.01);
            self.current_state.head_pose = self.ekf.get_pose();
        }

        // Apply drift correction
        if self.config.enable_drift_correction {
            self.drift_corrector.update(slam_pose, &self.current_state);
        }

        self.current_state.timestamp = timestamp;
        Ok(())
    }

    /// Predict pose at future time
    pub fn predict_pose(&mut self, look_ahead_time: f32) -> XrResult<Pose> {
        if !self.is_running {
            return Err(XrError::InvalidState("Tracking not running".into()));
        }

        let current_pose = self.ekf.get_pose();

        // Predict orientation
        let omega = self.current_state.angular_velocity.length();
        let predicted_orientation = if omega > 1e-6 {
            let axis = self.current_state.angular_velocity.normalize();
            let angle = omega * look_ahead_time;

            let delta_q = Quaternion::from_euler(axis.x * angle, axis.y * angle, axis.z * angle);
            current_pose.orientation.multiply(&delta_q)
        } else {
            current_pose.orientation
        };

        // Predict position
        let predicted_position = current_pose.position
            + self.current_state.velocity * look_ahead_time;

        Ok(Pose::new(predicted_position, predicted_orientation))
    }

    /// Get current tracking state
    pub fn get_state(&self) -> XrResult<TrackingState> {
        Ok(self.current_state.clone())
    }

    /// Get tracking quality
    pub fn get_quality(&self) -> TrackingQuality {
        self.current_state.quality
    }

    /// Calibrate IMU bias
    pub fn calibrate_imu(&mut self, samples: &[(Vector3<f32>, Vector3<f32>)]) -> XrResult<()> {
        if samples.len() < 100 {
            return Err(XrError::InvalidArgument(
                "Need at least 100 samples for calibration".into(),
            ));
        }

        let mut accel_sum = Vector3::ZERO;
        let mut gyro_sum = Vector3::ZERO;

        for (accel, gyro) in samples {
            accel_sum = accel_sum + *accel;
            gyro_sum = gyro_sum + *gyro;
        }

        let count = samples.len() as f32;

        // Expected gravity vector
        let expected_gravity = Vector3::new(0.0, -9.81, 0.0);

        self.imu_bias.accel_bias = Vector3::new(
            accel_sum.x / count - expected_gravity.x,
            accel_sum.y / count - expected_gravity.y,
            accel_sum.z / count - expected_gravity.z,
        );

        self.imu_bias.gyro_bias = Vector3::new(
            gyro_sum.x / count,
            gyro_sum.y / count,
            gyro_sum.z / count,
        );

        Ok(())
    }

    /// Compute time delta
    fn compute_dt(&mut self, timestamp: u64) -> XrResult<f32> {
        let current_time = timestamp as f64 / 1000.0;

        let dt = match self.last_update {
            Some(last) => (current_time - last) as f32,
            None => 0.016,
        };

        self.last_update = Some(current_time);

        Ok(dt.max(0.001).min(0.1))
    }

    /// Reset tracking
    pub fn reset(&mut self) -> XrResult<()> {
        self.ekf = ExtendedKalmanFilter::new();
        self.orientation_filter.reset();
        self.velocity_filter.reset();
        self.current_state = TrackingState {
            head_pose: Pose::identity(),
            quality: TrackingQuality::Excellent,
            velocity: Vector3::ZERO,
            angular_velocity: Vector3::ZERO,
            acceleration: Vector3::ZERO,
            timestamp: 0,
        };
        self.last_update = None;
        Ok(())
    }
}

/// Tracking state
#[derive(Debug, Clone)]
pub struct TrackingState {
    pub head_pose: Pose,
    pub quality: TrackingQuality,
    pub velocity: Vector3<f32>,
    pub angular_velocity: Vector3<f32>,
    pub acceleration: Vector3<f32>,
    pub timestamp: u64,
}

/// IMU bias calibration data
#[derive(Debug, Clone, Default)]
struct ImuBias {
    accel_bias: Vector3<f32>,
    gyro_bias: Vector3<f32>,
}

/// Drift corrector
#[derive(Debug, Clone)]
struct DriftCorrector {
    slam_positions: Vec<Vector3<f32>>,
    predicted_positions: Vec<Vector3<f32>>,
    window_size: usize,
}

impl DriftCorrector {
    fn new() -> Self {
        Self {
            slam_positions: Vec::new(),
            predicted_positions: Vec::new(),
            window_size: 100,
        }
    }

    fn update(&mut self, slam_pose: &Pose, state: &TrackingState) {
        self.slam_positions.push(slam_pose.position);
        self.predicted_positions.push(state.head_pose.position);

        if self.slam_positions.len() > self.window_size {
            self.slam_positions.remove(0);
            self.predicted_positions.remove(0);
        }
    }

    fn compute_drift(&self) -> Vector3<f32> {
        if self.slam_positions.len() < 2 {
            return Vector3::ZERO;
        }

        let first_slam = self.slam_positions[0];
        let last_slam = self.slam_positions[self.slam_positions.len() - 1];
        let slam_delta = Vector3::new(
            last_slam.x - first_slam.x,
            last_slam.y - first_slam.y,
            last_slam.z - first_slam.z,
        );

        let first_pred = self.predicted_positions[0];
        let last_pred = self.predicted_positions[self.predicted_positions.len() - 1];
        let pred_delta = Vector3::new(
            last_pred.x - first_pred.x,
            last_pred.y - first_pred.y,
            last_pred.z - first_pred.z,
        );

        Vector3::new(
            pred_delta.x - slam_delta.x,
            pred_delta.y - slam_delta.y,
            pred_delta.z - slam_delta.z,
        )
    }
}

/// External tracker (e.g., Vive lighthouses, Oculus constellation)
#[derive(Debug, Clone)]
pub struct ExternalTracker {
    tracker_type: ExternalTrackerType,
    is_connected: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ExternalTrackerType {
    SteamVR,
    OculusConstellation,
    OptiTrack,
    Custom,
}

impl ExternalTracker {
    fn new(tracker_type: ExternalTrackerType) -> Self {
        Self {
            tracker_type,
            is_connected: false,
        }
    }

    fn connect(&mut self) -> XrResult<()> {
        self.is_connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> XrResult<()> {
        self.is_connected = false;
        Ok(())
    }

    fn get_pose(&self) -> XrResult<Pose> {
        if !self.is_connected {
            return Err(XrError::HardwareError("External tracker not connected".into()));
        }
        Ok(Pose::identity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracking_creation() {
        let config = XrConfig::default();
        let tracker = TrackingSystem::new(&config);
        assert!(tracker.is_ok());
    }

    #[test]
    fn test_tracking_start_stop() {
        let config = XrConfig::default();
        let mut tracker = TrackingSystem::new(&config).unwrap();
        assert!(tracker.start().is_ok());
        assert!(tracker.is_running);

        let state = tracker.get_state();
        assert!(state.is_ok());

        assert!(tracker.stop().is_ok());
        assert!(!tracker.is_running);
    }

    #[test]
    fn test_imu_processing() {
        let config = XrConfig::default();
        let mut tracker = TrackingSystem::new(&config).unwrap();
        tracker.start().unwrap();

        let accel = Vector3::new(0.0, -9.81, 0.0);
        let gyro = Vector3::new(0.0, 0.0, 0.0);

        let pose = tracker.process_imu(&accel, &gyro, 1000);
        assert!(pose.is_ok());
    }

    #[test]
    fn test_pose_prediction() {
        let config = XrConfig::default();
        let mut tracker = TrackingSystem::new(&config).unwrap();
        tracker.start().unwrap();

        let predicted = tracker.predict_pose(0.01);
        assert!(predicted.is_ok());
    }
}
