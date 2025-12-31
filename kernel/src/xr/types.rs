//! Core XR types and data structures

use core::ops::{Add, Div, Mul, Sub};

/// 3D Vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3<T> {
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl Vector3<f32> {
    /// Calculate vector length
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalize vector
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }

    /// Dot product
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Calculate distance to another point
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const FORWARD: Self = Self { x: 0.0, y: 0.0, z: -1.0 };
    pub const UP: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
}

impl Add for Vector3<f32> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vector3<f32> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vector3<f32> {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Div<f32> for Vector3<f32> {
    type Output = Self;

    fn div(self, scalar: f32) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

/// Quaternion for rotation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    pub const fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { w, x, y, z }
    }

    /// Identity quaternion
    pub const fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Create from Euler angles (roll, pitch, yaw)
    pub fn from_euler(roll: f32, pitch: f32, yaw: f32) -> Self {
        let cy = (yaw * 0.5).cos();
        let sy = (yaw * 0.5).sin();
        let cp = (pitch * 0.5).cos();
        let sp = (pitch * 0.5).sin();
        let cr = (roll * 0.5).cos();
        let sr = (roll * 0.5).sin();

        Self {
            w: cr * cp * cy + sr * sp * sy,
            x: sr * cp * cy - cr * sp * sy,
            y: cr * sp * cy + sr * cp * sy,
            z: cr * cp * sy - sr * sp * cy,
        }
    }

    /// Normalize quaternion
    pub fn normalize(&self) -> Self {
        let len = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len > 0.0 {
            Self {
                w: self.w / len,
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }

    /// Multiply quaternions
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// Rotate a vector
    pub fn rotate_vector(&self, v: &Vector3<f32>) -> Vector3<f32> {
        let qv = Quaternion {
            w: 0.0,
            x: v.x,
            y: v.y,
            z: v.z,
        };

        let q_conj = Quaternion {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        };

        let result = self.multiply(&qv).multiply(&q_conj);
        Vector3::new(result.x, result.y, result.z)
    }

    /// Compute inverse quaternion
    pub fn inverse(&self) -> Self {
        let len_sq = self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.0 {
            Quaternion {
                w: self.w / len_sq,
                x: -self.x / len_sq,
                y: -self.y / len_sq,
                z: -self.z / len_sq,
            }
        } else {
            *self
        }
    }

    /// Convert to rotation matrix
    pub fn to_matrix(&self) -> [[f32; 3]; 3] {
        let w = self.w;
        let x = self.x;
        let y = self.y;
        let z = self.z;

        [
            [1.0 - 2.0 * y * y - 2.0 * z * z, 2.0 * x * y - 2.0 * z * w, 2.0 * x * z + 2.0 * y * w],
            [2.0 * x * y + 2.0 * z * w, 1.0 - 2.0 * x * x - 2.0 * z * z, 2.0 * y * z - 2.0 * x * w],
            [2.0 * x * z - 2.0 * y * w, 2.0 * y * z + 2.0 * x * w, 1.0 - 2.0 * x * x - 2.0 * y * y],
        ]
    }
}

impl Default for Vector3<f32> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

/// 6DoF pose (position + orientation)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub position: Vector3<f32>,
    pub orientation: Quaternion,
}

impl Pose {
    pub const fn new(position: Vector3<f32>, orientation: Quaternion) -> Self {
        Self {
            position,
            orientation,
        }
    }

    /// Identity pose
    pub const fn identity() -> Self {
        Self {
            position: Vector3::ZERO,
            orientation: Quaternion::identity(),
        }
    }

    /// Invert pose
    pub fn inverse(&self) -> Self {
        let inv_orientation = Quaternion {
            w: self.orientation.w,
            x: -self.orientation.x,
            y: -self.orientation.y,
            z: -self.orientation.z,
        };

        let inv_position = inv_orientation.rotate_vector(&self.position) * -1.0;

        Self {
            position: inv_position,
            orientation: inv_orientation,
        }
    }

    /// Transform a point
    pub fn transform_point(&self, point: &Vector3<f32>) -> Vector3<f32> {
        self.orientation.rotate_vector(point) + self.position
    }

    /// Multiply poses
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            position: self.transform_point(&other.position),
            orientation: self.orientation.multiply(&other.orientation),
        }
    }

    /// Interpolate between poses
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let position = Vector3::new(
            self.position.x + (other.position.x - self.position.x) * t,
            self.position.y + (other.position.y - self.position.y) * t,
            self.position.z + (other.position.z - self.position.z) * t,
        );

        // SLERP for orientation would be better, but lerp is simpler
        let orientation = Quaternion {
            w: self.orientation.w + (other.orientation.w - self.orientation.w) * t,
            x: self.orientation.x + (other.orientation.x - self.orientation.x) * t,
            y: self.orientation.y + (other.orientation.y - self.orientation.y) * t,
            z: self.orientation.z + (other.orientation.z - self.orientation.z) * t,
        }
        .normalize();

        Self {
            position,
            orientation,
        }
    }
}

/// 2D Point
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Point2<f32> {
    /// Distance to another point
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Camera intrinsics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraIntrinsics {
    /// Focal length (fx, fy)
    pub focal_length: (f32, f32),

    /// Principal point (cx, cy)
    pub principal_point: (f32, f32),

    /// Image dimensions
    pub image_size: (u32, u32),

    /// Distortion coefficients (k1, k2, k3, p1, p2)
    pub distortion: [f32; 5],
}

impl CameraIntrinsics {
    /// Project 3D point to 2D
    pub fn project(&self, point: &Vector3<f32>) -> Option<Point2<f32>> {
        if point.z <= 0.0 {
            return None;
        }

        let x = point.x / point.z;
        let y = point.y / point.z;

        let r2 = x * x + y * y;

        // Apply radial distortion
        let radial_distortion =
            1.0 + self.distortion[0] * r2 + self.distortion[1] * r2 * r2 + self.distortion[2] * r2 * r2 * r2;

        let dx = 2.0 * self.distortion[3] * x * y + self.distortion[4] * (r2 + 2.0 * x * x);
        let dy = self.distortion[3] * (r2 + 2.0 * y * y) + 2.0 * self.distortion[4] * x * y;

        let x_distorted = x * radial_distortion + dx;
        let y_distorted = y * radial_distortion + dy;

        Some(Point2::new(
            self.focal_length.0 * x_distorted + self.principal_point.0,
            self.focal_length.1 * y_distorted + self.principal_point.1,
        ))
    }

    /// Unproject 2D point to 3D ray
    pub fn unproject(&self, point: &Point2<f32>) -> Vector3<f32> {
        let x = (point.x - self.principal_point.0) / self.focal_length.0;
        let y = (point.y - self.principal_point.1) / self.focal_length.1;

        Vector3::new(x, y, 1.0).normalize()
    }
}

/// Tracking state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackingQuality {
    /// Tracking is lost
    Lost,

    /// Tracking is poor (high uncertainty)
    Poor,

    /// Tracking is fair (moderate uncertainty)
    Fair,

    /// Tracking is good (low uncertainty)
    Good,

    /// Tracking is excellent (minimal uncertainty)
    Excellent,
}

/// Current tracking state
#[derive(Debug, Clone)]
pub struct TrackingState {
    /// Current head pose
    pub head_pose: Pose,

    /// Tracking quality
    pub quality: TrackingQuality,

    /// Positional velocity (m/s)
    pub velocity: Vector3<f32>,

    /// Angular velocity (rad/s)
    pub angular_velocity: Vector3<f32>,

    /// Acceleration (m/s²)
    pub acceleration: Vector3<f32>,

    /// Timestamp
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector3_operations() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);

        let sum = v1 + v2;
        assert_eq!(sum, Vector3::new(5.0, 7.0, 9.0));

        let dot = v1.dot(&v2);
        assert_eq!(dot, 32.0);

        let cross = v1.cross(&v2);
        assert_eq!(cross, Vector3::new(-3.0, 6.0, -3.0));
    }

    #[test]
    fn test_quaternion_identity() {
        let q = Quaternion::identity();
        let v = Vector3::new(1.0, 2.0, 3.0);
        let rotated = q.rotate_vector(&v);
        assert_eq!(rotated, v);
    }

    #[test]
    fn test_pose_identity() {
        let pose = Pose::identity();
        let point = Vector3::new(1.0, 2.0, 3.0);
        let transformed = pose.transform_point(&point);
        assert_eq!(transformed, point);
    }

    #[test]
    fn test_pose_inverse() {
        let pose = Pose::new(
            Vector3::new(1.0, 2.0, 3.0),
            Quaternion::from_euler(0.1, 0.2, 0.3),
        );

        let inv = pose.inverse();
        let composed = pose.multiply(&inv);

        assert!((composed.position.x).abs() < 0.001);
        assert!((composed.position.y).abs() < 0.001);
        assert!((composed.position.z).abs() < 0.001);
    }
}
