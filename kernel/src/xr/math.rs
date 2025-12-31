//! Mathematical utilities for XR operations

use alloc::vec::Vec;
use crate::xr::types::{Vector3, Quaternion, Pose};

/// Kalman filter for sensor fusion
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    /// State vector [x, y, z, vx, vy, vz]
    state: [f32; 6],

    /// State covariance matrix
    covariance: [[f32; 6]; 6],

    /// Process noise
    process_noise: [[f32; 6]; 6],

    /// Measurement noise
    measurement_noise: [[f32; 6]; 6],
}

impl KalmanFilter {
    /// Create a new Kalman filter
    pub fn new(process_noise: f32, measurement_noise: f32) -> Self {
        Self {
            state: [0.0; 6],
            covariance: [[0.0; 6]; 6],
            process_noise: Self::diagonal(process_noise),
            measurement_noise: Self::diagonal(measurement_noise),
        }
    }

    fn diagonal(value: f32) -> [[f32; 6]; 6] {
        let mut mat = [[0.0; 6]; 6];
        for i in 0..6 {
            mat[i][i] = value;
        }
        mat
    }

    /// Predict next state
    pub fn predict(&mut self, dt: f32) {
        // State transition model (constant velocity)
        // x(k+1) = x(k) + v(k) * dt
        // v(k+1) = v(k)

        for i in 0..3 {
            self.state[i] += self.state[i + 3] * dt;
        }

        // Update covariance
        // P(k+1|k) = F * P(k|k) * F^T + Q
        for i in 0..6 {
            for j in 0..6 {
                self.covariance[i][j] += self.process_noise[i][j] * dt;
            }
        }
    }

    /// Update with measurement
    pub fn update(&mut self, measurement: &[f32; 6]) {
        // Innovation: y = z - H * x
        let mut innovation = [0.0f32; 6];
        for i in 0..6 {
            innovation[i] = measurement[i] - self.state[i];
        }

        // Innovation covariance: S = H * P * H^T + R
        let mut s = [[0.0f32; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                s[i][j] = self.covariance[i][j] + self.measurement_noise[i][j];
            }
        }

        // Kalman gain: K = P * H^T * S^(-1)
        let k = self.matrix_inverse(&s);

        // Update state: x = x + K * y
        for i in 0..6 {
            for j in 0..6 {
                self.state[i] += k[i][j] * innovation[j];
            }
        }

        // Update covariance: P = (I - K * H) * P
        for i in 0..6 {
            for j in 0..6 {
                for m in 0..6 {
                    self.covariance[i][j] -= (if i == m { 1.0 } else { 0.0 } - k[i][m]) * self.covariance[m][j];
                }
            }
        }
    }

    /// Simple matrix inversion (for small matrices)
    fn matrix_inverse(&self, mat: &[[f32; 6]; 6]) -> [[f32; 6]; 6] {
        // Simplified: assume diagonal matrix
        let mut inv = [[0.0f32; 6]; 6];
        for i in 0..6 {
            if mat[i][i] != 0.0 {
                inv[i][i] = 1.0 / mat[i][i];
            }
        }
        inv
    }

    /// Get current position estimate
    pub fn get_position(&self) -> Vector3<f32> {
        Vector3::new(self.state[0], self.state[1], self.state[2])
    }

    /// Get current velocity estimate
    pub fn get_velocity(&self) -> Vector3<f32> {
        Vector3::new(self.state[3], self.state[4], self.state[5])
    }
}

/// Extended Kalman Filter for pose estimation
#[derive(Debug, Clone)]
pub struct ExtendedKalmanFilter {
    /// State [x, y, z, qw, qx, qy, qz, vx, vy, vz]
    state: [f32; 10],

    /// State covariance
    covariance: [[f32; 10]; 10],
}

impl ExtendedKalmanFilter {
    /// Create a new EKF
    pub fn new() -> Self {
        Self {
            state: [0.0; 10],
            covariance: [[0.0; 10]; 10],
        }
    }

    /// Initialize with pose
    pub fn initialize(&mut self, pose: &Pose) {
        self.state[0] = pose.position.x;
        self.state[1] = pose.position.y;
        self.state[2] = pose.position.z;
        self.state[3] = pose.orientation.w;
        self.state[4] = pose.orientation.x;
        self.state[5] = pose.orientation.y;
        self.state[6] = pose.orientation.z;
    }

    /// Predict step
    pub fn predict(&mut self, dt: f32, angular_velocity: &Vector3<f32>) {
        // Update position with velocity
        self.state[0] += self.state[7] * dt;
        self.state[1] += self.state[8] * dt;
        self.state[2] += self.state[9] * dt;

        // Update orientation with angular velocity
        let omega = angular_velocity.length();
        if omega > 1e-6 {
            let axis = angular_velocity.normalize();
            let angle = omega * dt;

            let delta_q = Quaternion::from_euler(axis.x * angle, axis.y * angle, axis.z * angle);

            let current_q = Quaternion::new(
                self.state[3],
                self.state[4],
                self.state[5],
                self.state[6],
            );

            let new_q = current_q.multiply(&delta_q).normalize();

            self.state[3] = new_q.w;
            self.state[4] = new_q.x;
            self.state[5] = new_q.y;
            self.state[6] = new_q.z;
        }
    }

    /// Update with pose measurement
    pub fn update_pose(&mut self, pose: &Pose, _measurement_noise: f32) {
        // Simple update: blend prediction with measurement
        let alpha = 0.1; // Measurement weight

        self.state[0] = self.state[0] * (1.0 - alpha) + pose.position.x * alpha;
        self.state[1] = self.state[1] * (1.0 - alpha) + pose.position.y * alpha;
        self.state[2] = self.state[2] * (1.0 - alpha) + pose.position.z * alpha;

        // SLERP for quaternion would be better, but using simple lerp
        self.state[3] = self.state[3] * (1.0 - alpha) + pose.orientation.w * alpha;
        self.state[4] = self.state[4] * (1.0 - alpha) + pose.orientation.x * alpha;
        self.state[5] = self.state[5] * (1.0 - alpha) + pose.orientation.y * alpha;
        self.state[6] = self.state[6] * (1.0 - alpha) + pose.orientation.z * alpha;

        // Normalize quaternion
        let len = (self.state[3] * self.state[3]
            + self.state[4] * self.state[4]
            + self.state[5] * self.state[5]
            + self.state[6] * self.state[6])
        .sqrt();
        if len > 0.0 {
            self.state[3] /= len;
            self.state[4] /= len;
            self.state[5] /= len;
            self.state[6] /= len;
        }
    }

    /// Get current pose estimate
    pub fn get_pose(&self) -> Pose {
        Pose::new(
            Vector3::new(self.state[0], self.state[1], self.state[2]),
            Quaternion::new(self.state[3], self.state[4], self.state[5], self.state[6]),
        )
    }

    /// Get velocity estimate
    pub fn get_velocity(&self) -> Vector3<f32> {
        Vector3::new(self.state[7], self.state[8], self.state[9])
    }
}

/// Complementary filter for sensor fusion
#[derive(Debug, Clone)]
pub struct ComplementaryFilter {
    /// Acceleration weighting (0-1)
    accel_weight: f32,

    /// Gyro integration
    gyro_integration: Quaternion,

    /// Previous timestamp
    prev_time: Option<u64>,
}

impl ComplementaryFilter {
    /// Create a new complementary filter
    pub fn new(accel_weight: f32) -> Self {
        Self {
            accel_weight: accel_weight.clamp(0.0, 1.0),
            gyro_integration: Quaternion::identity(),
            prev_time: None,
        }
    }

    /// Update with accelerometer and gyroscope data
    pub fn update(&mut self, accel: &Vector3<f32>, gyro: &Vector3<f32>, dt: f32) -> Quaternion {
        // Integrate gyro
        let gyro_angle = gyro.length() * dt;
        if gyro_angle > 1e-6 {
            let gyro_axis = gyro.normalize();
            let gyro_delta = Quaternion::from_euler(
                gyro_axis.x * gyro_angle,
                gyro_axis.y * gyro_angle,
                gyro_axis.z * gyro_angle,
            );
            self.gyro_integration = self.gyro_integration.multiply(&gyro_delta).normalize();
        }

        // Compute orientation from accelerometer (assuming mostly horizontal)
        let accel_norm = accel.normalize();
        let accel_pitch = accel_norm.y.atan2(accel_norm.z);
        let accel_roll = accel_norm.x.atan2(accel_norm.z);

        let accel_orientation = Quaternion::from_euler(accel_roll, accel_pitch, 0.0);

        // Blend gyro integration with accelerometer orientation
        let w = self.accel_weight;
        let fused = Quaternion::new(
            self.gyro_integration.w * (1.0 - w) + accel_orientation.w * w,
            self.gyro_integration.x * (1.0 - w) + accel_orientation.x * w,
            self.gyro_integration.y * (1.0 - w) + accel_orientation.y * w,
            self.gyro_integration.z * (1.0 - w) + accel_orientation.z * w,
        )
        .normalize();

        self.gyro_integration = fused;
        fused
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.gyro_integration = Quaternion::identity();
        self.prev_time = None;
    }

    /// Get current orientation
    pub fn get_orientation(&self) -> Quaternion {
        self.gyro_integration
    }
}

/// Moving average filter for smoothing
#[derive(Debug, Clone)]
pub struct MovingAverageFilter {
    /// Window size
    window_size: usize,

    /// Sample buffer
    buffer: Vec<Vector3<f32>>,

    /// Current index
    index: usize,
}

impl MovingAverageFilter {
    /// Create a new moving average filter
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            buffer: Vec::with_capacity(window_size),
            index: 0,
        }
    }

    /// Add sample
    pub fn add_sample(&mut self, sample: Vector3<f32>) -> Vector3<f32> {
        if self.buffer.len() < self.window_size {
            self.buffer.push(sample);
        } else {
            self.buffer[self.index] = sample;
            self.index = (self.index + 1) % self.window_size;
        }

        self.get_average()
    }

    /// Get current average
    pub fn get_average(&self) -> Vector3<f32> {
        if self.buffer.is_empty() {
            return Vector3::ZERO;
        }

        let mut sum = Vector3::ZERO;
        for sample in &self.buffer {
            sum = sum + *sample;
        }

        let count = self.buffer.len() as f32;
        Vector3::new(sum.x / count, sum.y / count, sum.z / count)
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.index = 0;
    }
}

/// Low-pass filter for noise reduction
#[derive(Debug, Clone)]
pub struct LowPassFilter {
    /// Filter coefficient (0-1)
    alpha: f32,

    /// Previous value
    previous: Option<Vector3<f32>>,
}

impl LowPassFilter {
    /// Create a new low-pass filter
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            previous: None,
        }
    }

    /// Filter a sample
    pub fn filter(&mut self, sample: Vector3<f32>) -> Vector3<f32> {
        match self.previous {
            Some(prev) => {
                let filtered = Vector3::new(
                    prev.x + self.alpha * (sample.x - prev.x),
                    prev.y + self.alpha * (sample.y - prev.y),
                    prev.z + self.alpha * (sample.z - prev.z),
                );
                self.previous = Some(filtered);
                filtered
            }
            None => {
                self.previous = Some(sample);
                sample
            }
        }
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.previous = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_filter() {
        let mut kf = KalmanFilter::new(0.1, 0.1);

        kf.predict(0.016);
        kf.update(&[1.0, 2.0, 3.0, 0.0, 0.0, 0.0]);

        let pos = kf.get_position();
        assert!(pos.x > 0.0);
    }

    #[test]
    fn test_ekf_pose() {
        let mut ekf = ExtendedKalmanFilter::new();
        let pose = Pose::identity();
        ekf.initialize(&pose);

        let estimated = ekf.get_pose();
        assert_eq!(estimated.position, Vector3::ZERO);
    }

    #[test]
    fn test_complementary_filter() {
        let mut cf = ComplementaryFilter::new(0.02);

        let accel = Vector3::new(0.0, 0.0, 9.81);
        let gyro = Vector3::new(0.0, 0.0, 0.0);

        let orientation = cf.update(&accel, &gyro, 0.016);
        assert_eq!(orientation, Quaternion::identity());
    }

    #[test]
    fn test_moving_average() {
        let mut ma = MovingAverageFilter::new(5);

        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(2.0, 4.0, 6.0);

        ma.add_sample(v1);
        let avg1 = ma.add_sample(v2);

        assert!(avg1.x > 1.0 && avg1.x < 2.0);
    }
}
