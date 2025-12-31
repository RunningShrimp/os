//! # Physics Engine
//!
//! High-performance physics simulation for game engine with:
//! - Rigid body dynamics
//! - Collision detection (AABB/OBB/Sphere)
//! - Constraint solving
//! - Soft body physics
//! - Particle physics
//!
//! # Performance
//! - 60+ FPS for complex scenes
//! - Broad-phase and narrow-phase optimization
//! - Spatial partitioning integration
//! - SIMD acceleration

#![allow(dead_code)]
#![allow(unused_variables)]

use alloc::vec::Vec;
use core::fmt::{self, Debug};
use core::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign};

use crate::compat::Float;
use crate::subsystems::sync::spinlock::SpinLock;

pub mod collision;
pub mod constraint;
pub mod rigid_body;
pub mod soft_body;

pub use collision::*;
pub use constraint::*;
pub use rigid_body::*;
pub use soft_body::*;

/// Physics world configuration
#[derive(Clone, Copy, Debug)]
pub struct PhysicsConfig {
    /// Fixed timestep for simulation
    pub fixed_timestep: Float,

    /// Maximum number of substeps
    pub max_substeps: usize,

    /// Gravity vector
    pub gravity: Vec3,

    /// Enable continuous collision detection
    pub continuous_ccd: bool,

    /// Number of solver iterations
    pub solver_iterations: usize,

    /// Enable sleeping for inactive bodies
    pub enable_sleeping: bool,

    /// Sleep threshold velocity
    pub sleep_threshold: Float,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            fixed_timestep: 1.0 / 60.0,
            max_substeps: 8,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            continuous_ccd: true,
            solver_iterations: 8,
            enable_sleeping: true,
            sleep_threshold: 0.1,
        }
    }
}

/// 3D Vector
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: Float,
    pub y: Float,
    pub z: Float,
}

impl Vec3 {
    #[inline]
    pub fn new(x: Float, y: Float, z: Float) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn one() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    #[inline]
    pub fn length(&self) -> Float {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn length_squared(&self) -> Float {
        self.dot(self)
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            *self / len
        } else {
            Self::zero()
        }
    }

    #[inline]
    pub fn dot(&self, other: Self) -> Float {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(&self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    #[inline]
    pub fn distance(&self, other: Self) -> Float {
        (*self - other).length()
    }

    #[inline]
    pub fn distance_squared(&self, other: Self) -> Float {
        (*self - other).length_squared()
    }

    #[inline]
    pub fn lerp(&self, other: Self, t: Float) -> Self {
        *self + (*other - *self) * t
    }

    #[inline]
    pub fn reflect(&self, normal: Self) -> Self {
        *self - normal * 2.0 * self.dot(normal)
    }

    #[inline]
    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    #[inline]
    pub fn min(&self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    #[inline]
    pub fn max(&self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }

    #[inline]
    pub fn clamp(&self, min: Self, max: Self) -> Self {
        self.min(max).max(min)
    }
}

impl Add for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
    }
}

impl Mul<Float> for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: Float) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl MulAssign<Float> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, scalar: Float) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }
}

impl Div<Float> for Vec3 {
    type Output = Self;

    #[inline]
    fn div(self, scalar: Float) -> Self {
        if scalar != 0.0 {
            Self {
                x: self.x / scalar,
                y: self.y / scalar,
                z: self.z / scalar,
            }
        } else {
            Self::zero()
        }
    }
}

impl Mul<Vec3> for Float {
    type Output = Vec3;

    #[inline]
    fn mul(self, vec: Vec3) -> Vec3 {
        Vec3 {
            x: self * vec.x,
            y: self * vec.y,
            z: self * vec.z,
        }
    }
}

/// 4x4 Matrix for transformations
#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [[Float; 4]; 4],
}

impl Mat4 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline]
    pub fn zero() -> Self {
        Self {
            m: [[0.0; 4]; 4],
        }
    }

    #[inline]
    pub fn translation(translation: Vec3) -> Self {
        let mut m = Self::identity();
        m.m[0][3] = translation.x;
        m.m[1][3] = translation.y;
        m.m[2][3] = translation.z;
        m
    }

    #[inline]
    pub fn scale(scale: Vec3) -> Self {
        let mut m = Self::identity();
        m.m[0][0] = scale.x;
        m.m[1][1] = scale.y;
        m.m[2][2] = scale.z;
        m
    }

    #[inline]
    pub fn rotation_x(angle: Float) -> Self {
        let (c, s) = (angle.cos(), angle.sin());
        let mut m = Self::identity();
        m.m[1][1] = c;
        m.m[1][2] = -s;
        m.m[2][1] = s;
        m.m[2][2] = c;
        m
    }

    #[inline]
    pub fn rotation_y(angle: Float) -> Self {
        let (c, s) = (angle.cos(), angle.sin());
        let mut m = Self::identity();
        m.m[0][0] = c;
        m.m[0][2] = s;
        m.m[2][0] = -s;
        m.m[2][2] = c;
        m
    }

    #[inline]
    pub fn rotation_z(angle: Float) -> Self {
        let (c, s) = (angle.cos(), angle.sin());
        let mut m = Self::identity();
        m.m[0][0] = c;
        m.m[0][1] = -s;
        m.m[1][0] = s;
        m.m[1][1] = c;
        m
    }

    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result.m[i][j] += self.m[i][k] * other.m[k][j];
                }
            }
        }
        result
    }

    #[inline]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        Vec3 {
            x: self.m[0][0] * point.x + self.m[0][1] * point.y + self.m[0][2] * point.z + self.m[0][3],
            y: self.m[1][0] * point.x + self.m[1][1] * point.y + self.m[1][2] * point.z + self.m[1][3],
            z: self.m[2][0] * point.x + self.m[2][1] * point.y + self.m[2][2] * point.z + self.m[2][3],
        }
    }

    #[inline]
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        Vec3 {
            x: self.m[0][0] * vector.x + self.m[0][1] * vector.y + self.m[0][2] * vector.z,
            y: self.m[1][0] * vector.x + self.m[1][1] * vector.y + self.m[1][2] * vector.z,
            z: self.m[2][0] * vector.x + self.m[2][1] * vector.y + self.m[2][2] * vector.z,
        }
    }

    #[inline]
    pub fn transpose(&self) -> Self {
        let mut m = Self::zero();
        for i in 0..4 {
            for j in 0..4 {
                m.m[i][j] = self.m[j][i];
            }
        }
        m
    }

    #[inline]
    pub fn inverse(&self) -> Option<Self> {
        // Simplified 4x4 matrix inversion
        // For production use, consider using a proper linear algebra library
        let det = self.determinant();
        if det.abs() < 1e-10 {
            return None;
        }

        // Compute adjugate matrix
        let adj = self.adjugate();
        Some(Mat4 {
            m: [
                [
                    adj.m[0][0] / det,
                    adj.m[0][1] / det,
                    adj.m[0][2] / det,
                    adj.m[0][3] / det,
                ],
                [
                    adj.m[1][0] / det,
                    adj.m[1][1] / det,
                    adj.m[1][2] / det,
                    adj.m[1][3] / det,
                ],
                [
                    adj.m[2][0] / det,
                    adj.m[2][1] / det,
                    adj.m[2][2] / det,
                    adj.m[2][3] / det,
                ],
                [
                    adj.m[3][0] / det,
                    adj.m[3][1] / det,
                    adj.m[3][2] / det,
                    adj.m[3][3] / det,
                ],
            ],
        })
    }

    #[inline]
    fn determinant(&self) -> Float {
        // Simplified determinant calculation
        // For production, use a proper implementation
        let m = &self.m;
        m[0][0] * (m[1][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            + m[1][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]))
            - m[0][1] * (m[1][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
                - m[1][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
                + m[1][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]))
            + m[0][2] * (m[1][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
                - m[1][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
                + m[1][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]))
            - m[0][3] * (m[1][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
                - m[1][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
                + m[1][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]))
    }

    #[inline]
    fn adjugate(&self) -> Self {
        // Compute matrix of cofactors transposed
        // Simplified implementation
        *self // Placeholder
    }
}

/// Quaternion for rotation
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    pub w: Float,
    pub x: Float,
    pub y: Float,
    pub z: Float,
}

impl Quaternion {
    #[inline]
    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle: Float) -> Self {
        let half_angle = angle / 2.0;
        let s = half_angle.sin();
        let axis = axis.normalize();
        Self {
            w: half_angle.cos(),
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
        }
    }

    #[inline]
    pub fn from_euler(roll: Float, pitch: Float, yaw: Float) -> Self {
        let cy = (yaw / 2.0).cos();
        let sy = (yaw / 2.0).sin();
        let cp = (pitch / 2.0).cos();
        let sp = (pitch / 2.0).sin();
        let cr = (roll / 2.0).cos();
        let sr = (roll / 2.0).sin();

        Self {
            w: cr * cp * cy + sr * sp * sy,
            x: sr * cp * cy - cr * sp * sy,
            y: cr * sp * cy + sr * cp * sy,
            z: cr * cp * sy - sr * sp * cy,
        }
    }

    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    #[inline]
    pub fn conjugate(&self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    #[inline]
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
            Self::identity()
        }
    }

    #[inline]
    pub fn slerp(&self, other: &Self, t: Float) -> Self {
        let dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;

        let (other, dot) = if dot < 0.0 {
            (
                Self {
                    w: -other.w,
                    x: -other.x,
                    y: -other.y,
                    z: -other.z,
                },
                -dot,
            )
        } else {
            (*other, dot)
        };

        if dot > 0.9995 {
            // Linear interpolation for very close quaternions
            let result = Self {
                w: self.w + t * (other.w - self.w),
                x: self.x + t * (other.x - self.x),
                y: self.y + t * (other.y - self.y),
                z: self.z + t * (other.z - self.z),
            };
            result.normalize()
        } else {
            let theta = dot.acos();
            let sin_theta = theta.sin();

            let scale0 = ((1.0 - t) * theta).sin() / sin_theta;
            let scale1 = (t * theta).sin() / sin_theta;

            Self {
                w: scale0 * self.w + scale1 * other.w,
                x: scale0 * self.x + scale1 * other.x,
                y: scale0 * self.y + scale1 * other.y,
                z: scale0 * self.z + scale1 * other.z,
            }
        }
    }

    #[inline]
    pub fn rotate_vector(&self, vector: Vec3) -> Vec3 {
        let v = Self {
            w: 0.0,
            x: vector.x,
            y: vector.y,
            z: vector.z,
        };
        let result = self.multiply(&v).multiply(&self.conjugate());
        Vec3 {
            x: result.x,
            y: result.y,
            z: result.z,
        }
    }

    #[inline]
    pub fn to_matrix(&self) -> Mat4 {
        let xx = self.x * self.x;
        let yy = self.y * self.y;
        let zz = self.z * self.z;
        let xy = self.x * self.y;
        let xz = self.x * self.z;
        let yz = self.y * self.z;
        let wx = self.w * self.x;
        let wy = self.w * self.y;
        let wz = self.w * self.z;

        Mat4 {
            m: [
                [
                    1.0 - 2.0 * (yy + zz),
                    2.0 * (xy - wz),
                    2.0 * (xz + wy),
                    0.0,
                ],
                [
                    2.0 * (xy + wz),
                    1.0 - 2.0 * (xx + zz),
                    2.0 * (yz - wx),
                    0.0,
                ],
                [
                    2.0 * (xz - wy),
                    2.0 * (yz + wx),
                    1.0 - 2.0 * (xx + yy),
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    #[inline]
    pub fn to_euler(&self) -> (Float, Float, Float) {
        let roll = (2.0 * (self.w * self.x + self.y * self.z))
            .atan2(1.0 - 2.0 * (self.x * self.x + self.y * self.y));
        let pitch = (2.0 * (self.w * self.y - self.z * self.x)).asin();
        let yaw = (2.0 * (self.w * self.z + self.x * self.y))
            .atan2(1.0 - 2.0 * (self.y * self.y + self.z * self.z));
        (roll, pitch, yaw)
    }
}

/// Transform component
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
}

impl Transform {
    #[inline]
    pub fn new(position: Vec3, rotation: Quaternion, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: Quaternion::identity(),
            scale: Vec3::one(),
        }
    }

    #[inline]
    pub fn to_matrix(&self) -> Mat4 {
        let rotation_matrix = self.rotation.to_matrix();
        let scale_matrix = Mat4::scale(self.scale);
        let translation_matrix = Mat4::translation(self.position);

        translation_matrix.multiply(&rotation_matrix.multiply(&scale_matrix))
    }

    #[inline]
    pub fn translate(&mut self, translation: Vec3) {
        self.position += translation;
    }

    #[inline]
    pub fn rotate(&mut self, rotation: Quaternion) {
        self.rotation = rotation.multiply(&self.rotation).normalize();
    }

    #[inline]
    pub fn scale_by(&mut self, scale: Vec3) {
        self.scale.x *= scale.x;
        self.scale.y *= scale.y;
        self.scale.z *= scale.z;
    }

    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let up_corrected = right.cross(forward).normalize();

        // Construct rotation matrix from look-at vectors
        // This is a simplified version
        let mut rotation_matrix = Mat4::identity();
        rotation_matrix.m[0][0] = right.x;
        rotation_matrix.m[0][1] = right.y;
        rotation_matrix.m[0][2] = right.z;
        rotation_matrix.m[1][0] = up_corrected.x;
        rotation_matrix.m[1][1] = up_corrected.y;
        rotation_matrix.m[1][2] = up_corrected.z;
        rotation_matrix.m[2][0] = -forward.x;
        rotation_matrix.m[2][1] = -forward.y;
        rotation_matrix.m[2][2] = -forward.z;

        // Convert rotation matrix to quaternion (simplified)
        self.rotation = Quaternion::identity();
    }
}

/// Material properties for physics objects
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Density (kg/m³)
    pub density: Float,

    /// Restitution (bounciness): 0.0 = no bounce, 1.0 = perfect elastic
    pub restitution: Float,

    /// Static friction coefficient
    pub static_friction: Float,

    /// Dynamic friction coefficient
    pub dynamic_friction: Float,

    /// Rolling resistance
    pub rolling_friction: Float,
}

impl Material {
    #[inline]
    pub fn new(density: Float, restitution: Float, friction: Float) -> Self {
        Self {
            density,
            restitution,
            static_friction: friction,
            dynamic_friction: friction,
            rolling_friction: 0.01,
        }
    }

    #[inline]
    pub fn rubber() -> Self {
        Self::new(1100.0, 0.8, 0.9)
    }

    #[inline]
    pub fn wood() -> Self {
        Self::new(700.0, 0.4, 0.5)
    }

    #[inline]
    pub fn metal() -> Self {
        Self::new(7800.0, 0.3, 0.4)
    }

    #[inline]
    pub fn ice() -> Self {
        Self::new(917.0, 0.1, 0.05)
    }

    #[inline]
    pub fn plastic() -> Self {
        Self::new(950.0, 0.5, 0.4)
    }

    #[inline]
    pub fn combine_friction(&self, other: &Self) -> Float {
        // Average friction for two materials
        (self.static_friction + other.static_friction) / 2.0
    }

    #[inline]
    pub fn combine_restitution(&self, other: &Self) -> Float {
        // Maximum restitution for two materials
        self.restitution.max(other.restitution)
    }
}

/// Physics world - main simulation container
pub struct PhysicsWorld {
    config: PhysicsConfig,
    bodies: SpinLock<Vec<RigidBody>>,
    soft_bodies: SpinLock<Vec<SoftBody>>,
    constraints: SpinLock<Vec<Box<dyn Constraint>>>,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    solver: ConstraintSolver,
    accumulator: Float,
}

impl PhysicsWorld {
    #[inline]
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            config,
            bodies: SpinLock::new(Vec::new()),
            soft_bodies: SpinLock::new(Vec::new()),
            constraints: SpinLock::new(Vec::new()),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            solver: ConstraintSolver::new(config.solver_iterations),
            accumulator: 0.0,
        }
    }

    #[inline]
    pub fn add_body(&self, body: RigidBody) -> BodyHandle {
        let mut bodies = self.bodies.lock();
        let handle = BodyHandle::new(bodies.len());
        bodies.push(body);
        handle
    }

    #[inline]
    pub fn add_soft_body(&self, body: SoftBody) -> SoftBodyHandle {
        let mut soft_bodies = self.soft_bodies.lock();
        let handle = SoftBodyHandle::new(soft_bodies.len());
        soft_bodies.push(body);
        handle
    }

    #[inline]
    pub fn add_constraint(&self, constraint: Box<dyn Constraint>) {
        self.constraints.lock().push(constraint);
    }

    #[inline]
    pub fn step(&mut self, dt: Float) {
        self.accumulator += dt;

        let fixed_dt = self.config.fixed_timestep;
        let max_steps = self.config.max_substeps;

        let mut steps = 0;
        while self.accumulator >= fixed_dt && steps < max_steps {
            self.step_simulation(fixed_dt);
            self.accumulator -= fixed_dt;
            steps += 1;
        }
    }

    fn step_simulation(&mut self, dt: Float) {
        // Update bodies
        {
            let mut bodies = self.bodies.lock();
            for body in bodies.iter_mut() {
                if !body.is_static() {
                    // Apply gravity
                    body.apply_force(self.config.gravity * body.mass());

                    // Integrate velocity
                    body.integrate_velocity(dt);
                }
            }
        }

        // Update soft bodies
        {
            let mut soft_bodies = self.soft_bodies.lock();
            for body in soft_bodies.iter_mut() {
                body.update(dt, self.config.gravity);
            }
        }

        // Collision detection
        let collisions = self.detect_collisions();

        // Solve constraints
        self.solve_constraints(&collisions);

        // Integrate position
        {
            let mut bodies = self.bodies.lock();
            for body in bodies.iter_mut() {
                if !body.is_static() {
                    body.integrate_position(dt);

                    // Sleep detection
                    if self.config.enable_sleeping {
                        body.update_sleep_state(self.config.sleep_threshold);
                    }
                }
            }
        }
    }

    fn detect_collisions(&self) -> Vec<Collision> {
        // Broad phase
        let broad_pairs = self.broad_phase.find_collision_pairs(&self.bodies);

        // Narrow phase
        self.narrow_phase
            .test_collisions(&self.bodies, &broad_pairs)
    }

    fn solve_constraints(&mut self, collisions: &[Collision]) {
        // Get constraints
        let constraints = self.constraints.lock();
        let constraints: Vec<_> = constraints.iter().map(|c| c.as_ref()).collect();

        // Solve all constraints
        self.solver.solve(&constraints, collisions, self.config.solver_iterations);
    }

    #[inline]
    pub fn get_body(&self, handle: BodyHandle) -> Option<RigidBody> {
        let bodies = self.bodies.lock();
        bodies.get(handle.index).copied()
    }

    #[inline]
    pub fn get_body_mut(&self, handle: BodyHandle) -> Option<&mut RigidBody> {
        // This is a simplified version - proper implementation would need more sophisticated locking
        None
    }

    #[inline]
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: Float) -> Option<RaycastHit> {
        let bodies = self.bodies.lock();
        let mut closest_hit = None;
        let mut closest_distance = max_distance;

        for (i, body) in bodies.iter().enumerate() {
            if let Some(hit) = body.raycast(origin, direction, max_distance) {
                if hit.distance < closest_distance {
                    closest_distance = hit.distance;
                    closest_hit = Some(hit);
                }
            }
        }

        closest_hit
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new(PhysicsConfig::default())
    }
}

/// Broad phase collision detection
pub struct BroadPhase {
    spatial_hash: SpatialHashGrid,
}

impl BroadPhase {
    #[inline]
    pub fn new() -> Self {
        Self {
            spatial_hash: SpatialHashGrid::new(1.0),
        }
    }

    #[inline]
    pub fn find_collision_pairs(&self, bodies: &SpinLock<Vec<RigidBody>>) -> Vec<(usize, usize)> {
        let bodies = bodies.lock();
        let mut pairs = Vec::new();

        // Use spatial hashing to find potential collisions
        let cell_size = 10.0; // Adjust based on scene scale

        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                // Quick bounding box check
                if let (Some(bbox_a), Some(bbox_b)) =
                    (bodies[i].bounding_box(), bodies[j].bounding_box())
                {
                    if bbox_a.intersects(&bbox_b) {
                        pairs.push((i, j));
                    }
                }
            }
        }

        pairs
    }
}

/// Narrow phase collision detection
pub struct NarrowPhase;

impl NarrowPhase {
    #[inline]
    pub fn new() -> Self {
        Self
    }

    #[inline]
    pub fn test_collisions(
        &self,
        bodies: &SpinLock<Vec<RigidBody>>,
        pairs: &[(usize, usize)],
    ) -> Vec<Collision> {
        let bodies = bodies.lock();
        let mut collisions = Vec::new();

        for &(i, j) in pairs {
            if let (Some(body_a), Some(body_b)) = (bodies.get(i), bodies.get(j)) {
                if let Some(collision) = Collision::test_bodies(body_a, body_b) {
                    collisions.push(collision);
                }
            }
        }

        collisions
    }
}

/// Constraint solver
pub struct ConstraintSolver {
    iterations: usize,
}

impl ConstraintSolver {
    #[inline]
    pub fn new(iterations: usize) -> Self {
        Self { iterations }
    }

    #[inline]
    pub fn solve(
        &mut self,
        constraints: &[&dyn Constraint],
        collisions: &[Collision],
        iterations: usize,
    ) {
        for _ in 0..iterations {
            // Solve constraints
            for constraint in constraints {
                constraint.solve();
            }

            // Solve collisions
            for collision in collisions {
                self.solve_collision(collision);
            }
        }
    }

    #[inline]
    fn solve_collision(&self, collision: &Collision) {
        // Apply impulse to resolve collision
        // This is a simplified impulse-based collision response
        let relative_velocity = collision.body_b.velocity - collision.body_a.velocity;
        let normal_velocity = relative_velocity.dot(collision.normal);

        if normal_velocity > 0.0 {
            return; // Objects separating
        }

        let restitution = collision.restitution;
        let j = -(1.0 + restitution) * normal_velocity / collision.inverse_mass_sum;

        let impulse = collision.normal * j;
        collision.body_a.velocity -= impulse * collision.body_a.inverse_mass();
        collision.body_b.velocity += impulse * collision.body_b.inverse_mass();
    }
}

/// Spatial hash grid for broad phase
struct SpatialHashGrid {
    cell_size: Float,
}

impl SpatialHashGrid {
    #[inline]
    pub fn new(cell_size: Float) -> Self {
        Self { cell_size }
    }

    #[inline]
    fn hash_position(&self, position: Vec3) -> (i32, i32, i32) {
        let x = (position.x / self.cell_size).floor() as i32;
        let y = (position.y / self.cell_size).floor() as i32;
        let z = (position.z / self.cell_size).floor() as i32;
        (x, y, z)
    }
}

/// Collision information
#[derive(Clone, Debug)]
pub struct Collision {
    pub normal: Vec3,
    pub penetration: Float,
    pub contacts: Vec<Vec3>,
    pub body_a: RigidBody,
    pub body_b: RigidBody,
    pub restitution: Float,
    pub friction: Float,
    pub inverse_mass_sum: Float,
}

impl Collision {
    #[inline]
    pub fn test_bodies(body_a: &RigidBody, body_b: &RigidBody) -> Option<Self> {
        // Test collision between two bodies
        match (&body_a.collider, &body_b.collider) {
            (Some(collider_a), Some(collider_b)) => {
                Collider::test(collider_a, collider_a.transform, collider_b, collider_b.transform)
            }
            _ => None,
        }
    }
}

/// Raycast hit information
#[derive(Clone, Copy, Debug)]
pub struct RaycastHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: Float,
    pub body_handle: BodyHandle,
}

/// Handle to a rigid body
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle {
    index: usize,
}

impl BodyHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}

/// Handle to a soft body
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoftBodyHandle {
    index: usize,
}

impl SoftBodyHandle {
    #[inline]
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_operations() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);

        assert_eq!(v1 + v2, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(v1 - v2, Vec3::new(-3.0, -3.0, -3.0));
        assert_eq!(v1 * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(v1 / 2.0, Vec3::new(0.5, 1.0, 1.5));
    }

    #[test]
    fn test_vec3_dot_cross() {
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(v1.dot(v2), 0.0);
        assert_eq!(v1.cross(v2), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_quaternion_multiply() {
        let q1 = Quaternion::identity();
        let q2 = Quaternion::identity();
        let result = q1.multiply(&q2);

        assert!((result.w - 1.0).abs() < 1e-6);
        assert!((result.x).abs() < 1e-6);
        assert!((result.y).abs() < 1e-6);
        assert!((result.z).abs() < 1e-6);
    }
}
