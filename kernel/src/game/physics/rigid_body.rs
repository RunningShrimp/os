//! # Rigid Body Dynamics
//!
//! Newtonian physics for rigid bodies:
//! - Force and torque accumulation
//! - Velocity and position integration
//! - Mass properties
//! - Collision response

use super::{Collider, Material, Transform, Vec3};

/// Rigid body state
#[derive(Clone, Copy, Debug)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

/// Rigid body
#[derive(Clone, Debug)]
pub struct RigidBody {
    pub transform: Transform,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub force: Vec3,
    pub torque: Vec3,
    pub mass: super::Float,
    pub inv_mass: super::Float,
    pub inertia: super::Float,
    pub inv_inertia: super::Float,
    pub body_type: BodyType,
    pub collider: Option<Collider>,
    pub material: Material,
    pub is_sleeping: bool,
    pub sleep_time: super::Float,
}

impl RigidBody {
    #[inline]
    pub fn new(transform: Transform, mass: super::Float) -> Self {
        let inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        Self {
            transform,
            velocity: Vec3::zero(),
            angular_velocity: Vec3::zero(),
            force: Vec3::zero(),
            torque: Vec3::zero(),
            mass,
            inv_mass,
            inertia: 1.0,
            inv_inertia: 1.0,
            body_type: if mass > 0.0 { BodyType::Dynamic } else { BodyType::Static },
            collider: None,
            material: Material::metal(),
            is_sleeping: false,
            sleep_time: 0.0,
        }
    }

    #[inline]
    pub fn with_collider(mut self, collider: Collider) -> Self {
        self.collider = Some(collider);
        self
    }

    #[inline]
    pub fn with_material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    #[inline]
    pub fn with_type(mut self, body_type: BodyType) -> Self {
        self.body_type = body_type;
        self
    }

    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self.body_type, BodyType::Static)
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        matches!(self.body_type, BodyType::Dynamic)
    }

    #[inline]
    pub fn is_kinematic(&self) -> bool {
        matches!(self.body_type, BodyType::Kinematic)
    }

    #[inline]
    pub fn apply_force(&mut self, force: Vec3) {
        self.force += force;
    }

    #[inline]
    pub fn apply_force_at_point(&mut self, force: Vec3, point: Vec3) {
        self.force += force;
        let arm = point - self.transform.position;
        self.torque += arm.cross(force);
    }

    #[inline]
    pub fn apply_torque(&mut self, torque: Vec3) {
        self.torque += torque;
    }

    #[inline]
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        self.velocity += impulse * self.inv_mass;
    }

    #[inline]
    pub fn apply_angular_impulse(&mut self, impulse: Vec3) {
        self.angular_velocity += impulse * self.inv_inertia;
    }

    #[inline]
    pub fn integrate_velocity(&mut self, dt: super::Float) {
        if !self.is_static() && !self.is_sleeping {
            let acceleration = self.force * self.inv_mass;
            self.velocity += acceleration * dt;

            let angular_acceleration = self.torque * self.inv_inertia;
            self.angular_velocity += angular_acceleration * dt;

            // Clear forces
            self.force = Vec3::zero();
            self.torque = Vec3::zero();
        }
    }

    #[inline]
    pub fn integrate_position(&mut self, dt: super::Float) {
        if !self.is_static() && !self.is_sleeping {
            self.transform.position += self.velocity * dt;

            // Update rotation (simplified)
            let angle = self.angular_velocity.length() * dt;
            if angle > 0.0001 {
                let axis = self.angular_velocity.normalize();
                let rotation = super::Quaternion::from_axis_angle(axis, angle);
                self.transform.rotate(rotation);
            }
        }
    }

    #[inline]
    pub fn mass(&self) -> super::Float {
        self.mass
    }

    #[inline]
    pub fn inverse_mass(&self) -> super::Float {
        self.inv_mass
    }

    #[inline]
    pub fn update_sleep_state(&mut self, threshold: super::Float) {
        let speed = self.velocity.length();
        if speed < threshold {
            self.sleep_time += 1.0 / 60.0;
            if self.sleep_time > 0.5 {
                self.is_sleeping = true;
            }
        } else {
            self.sleep_time = 0.0;
            self.is_sleeping = false;
        }
    }

    #[inline]
    pub fn wake_up(&mut self) {
        self.is_sleeping = false;
        self.sleep_time = 0.0;
    }

    #[inline]
    pub fn kinetic_energy(&self) -> super::Float {
        0.5 * self.mass * self.velocity.length_squared()
    }

    #[inline]
    pub fn bounding_box(&self) -> Option<super::collision::AABB> {
        self.collider.as_ref().map(|c| match &c.shape {
            super::collision::ColliderShape::Sphere { radius } => {
                super::collision::AABB::from_center_extents(
                    self.transform.position,
                    Vec3::new(*radius as super::Float, *radius as super::Float, *radius as super::Float),
                )
            }
            super::collision::ColliderShape::Box { half_extents } => {
                super::collision::AABB::from_center_extents(
                    self.transform.position,
                    *half_extents,
                )
            }
            _ => super::collision::AABB::from_center_extents(
                self.transform.position,
                Vec3::new(0.5, 0.5, 0.5),
            ),
        })
    }

    #[inline]
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: super::Float) -> Option<super::RaycastHit> {
        self.collider.as_ref()?.raycast(origin, direction, max_distance, self.transform)
    }
}

impl Default for RigidBody {
    fn default() -> Self {
        Self::new(Transform::identity(), 1.0)
    }
}

/// Physics utilities for rigid bodies
pub struct RigidBodyUtils;

impl RigidBodyUtils {
    #[inline]
    pub fn calculate_sphere_mass(radius: super::Float, density: super::Float) -> super::Float {
        let volume = (4.0 / 3.0) * core::f64::consts::PI as super::Float * radius.powi(3);
        volume * density
    }

    #[inline]
    pub fn calculate_box_mass(extents: Vec3, density: super::Float) -> super::Float {
        let volume = extents.x * extents.y * extents.z * 8.0;
        volume * density
    }

    #[inline]
    pub fn calculate_sphere_inertia(mass: super::Float, radius: super::Float) -> super::Float {
        (2.0 / 5.0) * mass * radius * radius
    }

    #[inline]
    pub fn calculate_box_inertia(mass: super::Float, extents: Vec3) -> Vec3 {
        let w = extents.x * 2.0;
        let h = extents.y * 2.0;
        let d = extents.z * 2.0;
        Vec3 {
            x: (1.0 / 12.0) * mass * (h * h + d * d),
            y: (1.0 / 12.0) * mass * (w * w + d * d),
            z: (1.0 / 12.0) * mass * (w * w + h * h),
        }
    }
}
