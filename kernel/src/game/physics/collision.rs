//! # Collision Detection
//!
//! Broad-phase and narrow-phase collision detection:
//! - AABB (Axis-Aligned Bounding Box)
//! - OBB (Oriented Bounding Box)
//! - Sphere collision
//! - Triangle mesh collision
//! - Continuous collision detection

use super::Vec3;

/// Collision shape types
#[derive(Clone, Debug)]
pub enum ColliderShape {
    Sphere { radius: f32 },
    Box { half_extents: Vec3 },
    Capsule { radius: f32, height: f32 },
    Mesh { vertices: Vec<Vec3>, indices: Vec<usize> },
}

/// Collider component
#[derive(Clone, Debug)]
pub struct Collider {
    pub shape: ColliderShape,
    pub transform: super::Transform,
    pub is_trigger: bool,
    pub friction: f32,
    pub restitution: f32,
}

impl Collider {
    #[inline]
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            transform: super::Transform::identity(),
            is_trigger: false,
            friction: 0.5,
            restitution: 0.3,
        }
    }

    #[inline]
    pub fn sphere(radius: f32) -> Self {
        Self::new(ColliderShape::Sphere { radius })
    }

    #[inline]
    pub fn box_collider(half_extents: Vec3) -> Self {
        Self::new(ColliderShape::Box { half_extents })
    }

    #[inline]
    pub fn capsule(radius: f32, height: f32) -> Self {
        Self::new(ColliderShape::Capsule { radius, height })
    }

    /// Test collision between two colliders
    #[inline]
    pub fn test(a: &Collider, transform_a: super::Transform, b: &Collider, transform_b: super::Transform) -> Option<super::Collision> {
        match (&a.shape, &b.shape) {
            (ColliderShape::Sphere { radius: r1 }, ColliderShape::Sphere { radius: r2 }) => {
                Self::test_sphere_sphere(
                    transform_a.position,
                    *r1,
                    transform_b.position,
                    *r2,
                )
            }
            (ColliderShape::Box { half_extents: h1 }, ColliderShape::Box { half_extents: h2 }) => {
                Self::test_box_box(transform_a, *h1, transform_b, *h2)
            }
            _ => None,
        }
    }

    #[inline]
    fn test_sphere_sphere(
        pos_a: Vec3,
        radius_a: f32,
        pos_b: Vec3,
        radius_b: f32,
    ) -> Option<super::Collision> {
        let diff = pos_b - pos_a;
        let distance = diff.length();
        let radius_sum = radius_a + radius_b;

        if distance < radius_sum {
            let normal = if distance > 0.0001 {
                diff.normalize()
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };
            let penetration = radius_sum - distance;

            Some(super::Collision {
                normal,
                penetration: penetration as super::Float,
                contacts: vec![pos_a + normal * (radius_a as super::Float - penetration as super::Float / 2.0)],
                body_a: super::RigidBody::default(),
                body_b: super::RigidBody::default(),
                restitution: 0.5,
                friction: 0.5,
                inverse_mass_sum: 2.0,
            })
        } else {
            None
        }
    }

    #[inline]
    fn test_box_box(
        transform_a: super::Transform,
        half_extents_a: Vec3,
        transform_b: super::Transform,
        half_extents_b: Vec3,
    ) -> Option<super::Collision> {
        // Simplified OBB-OBB collision test using AABB
        // For production, implement SAT (Separating Axis Theorem)
        let min_a = transform_a.position - half_extents_a;
        let max_a = transform_a.position + half_extents_a;
        let min_b = transform_b.position - half_extents_b;
        let max_b = transform_b.position + half_extents_b;

        if min_a.x <= max_b.x && max_a.x >= min_b.x &&
           min_a.y <= max_b.y && max_a.y >= min_b.y &&
           min_a.z <= max_b.z && max_a.z >= min_b.z {
            // Calculate penetration and normal
            let overlaps = [
                max_a.x - min_b.x,
                max_b.x - min_a.x,
                max_a.y - min_b.y,
                max_b.y - min_a.y,
                max_a.z - min_b.z,
                max_b.z - min_a.z,
            ];

            let min_overlap = overlaps.iter().reduce(|a, b| a.min(b)).unwrap();
            let normal = match overlaps.iter().position(|&x| x == min_overlap) {
                Some(0) => Vec3::new(-1.0, 0.0, 0.0),
                Some(1) => Vec3::new(1.0, 0.0, 0.0),
                Some(2) => Vec3::new(0.0, -1.0, 0.0),
                Some(3) => Vec3::new(0.0, 1.0, 0.0),
                Some(4) => Vec3::new(0.0, 0.0, -1.0),
                Some(5) => Vec3::new(0.0, 0.0, 1.0),
                _ => Vec3::new(0.0, 1.0, 0.0),
            };

            Some(super::Collision {
                normal,
                penetration: *min_overlap as super::Float,
                contacts: vec![],
                body_a: super::RigidBody::default(),
                body_b: super::RigidBody::default(),
                restitution: 0.5,
                friction: 0.5,
                inverse_mass_sum: 2.0,
            })
        } else {
            None
        }
    }
}

/// Axis-Aligned Bounding Box
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self {
            min: center - extents,
            max: center + extents,
        }
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x
            && self.min.y <= other.max.y && self.max.y >= other.min.y
            && self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    #[inline]
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
            && point.z >= self.min.z && point.z <= self.max.z
    }

    #[inline]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min: Vec3 {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
                z: self.min.z.min(other.min.z),
            },
            max: Vec3 {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
                z: self.max.z.max(other.max.z),
            },
        }
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    #[inline]
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

/// Bounding sphere
#[derive(Clone, Copy, Debug)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: super::Float,
}

impl BoundingSphere {
    #[inline]
    pub fn new(center: Vec3, radius: super::Float) -> Self {
        Self { center, radius }
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.center.distance_squared(other.center) <= (self.radius + other.radius).powi(2)
    }

    #[inline]
    pub fn contains(&self, point: Vec3) -> bool {
        self.center.distance_squared(point) <= self.radius.powi(2)
    }
}
