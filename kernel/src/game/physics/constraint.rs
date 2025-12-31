//! # Physics Constraints
//!
//! Constraints for restricting body motion:
//! - Distance constraints
//! - Hinge constraints
//! - Fixed constraints
//! - Slider constraints
//! - Cone twist constraints

use super::Vec3;

/// Base constraint trait
pub trait Constraint {
    /// Solve the constraint
    fn solve(&self);
    /// Get constraint type
    fn constraint_type(&self) -> ConstraintType;
}

/// Constraint types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintType {
    Distance,
    Hinge,
    Fixed,
    Slider,
    ConeTwist,
    Spring,
}

/// Distance constraint (keeps two points at fixed distance)
#[derive(Clone, Debug)]
pub struct DistanceConstraint {
    pub body_a: super::BodyHandle,
    pub body_b: super::BodyHandle,
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    pub distance: super::Float,
    pub stiffness: super::Float,
}

impl DistanceConstraint {
    #[inline]
    pub fn new(
        body_a: super::BodyHandle,
        body_b: super::BodyHandle,
        anchor_a: Vec3,
        anchor_b: Vec3,
        distance: super::Float,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            distance,
            stiffness: 0.5,
        }
    }
}

impl Constraint for DistanceConstraint {
    #[inline]
    fn solve(&self) {
        // Simplified distance constraint solving
        // In production, would access bodies and apply corrections
    }

    #[inline]
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Distance
    }
}

/// Hinge constraint (allows rotation around one axis)
#[derive(Clone, Debug)]
pub struct HingeConstraint {
    pub body_a: super::BodyHandle,
    pub body_b: super::BodyHandle,
    pub anchor: Vec3,
    pub axis: Vec3,
    pub limits: Option<(super::Float, super::Float)>,
    pub motor: Option<HingeMotor>,
}

/// Hinge motor for actuated joints
#[derive(Clone, Copy, Debug)]
pub struct HingeMotor {
    pub target_velocity: super::Float,
    pub max_force: super::Float,
}

impl HingeConstraint {
    #[inline]
    pub fn new(
        body_a: super::BodyHandle,
        body_b: super::BodyHandle,
        anchor: Vec3,
        axis: Vec3,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor,
            axis: axis.normalize(),
            limits: None,
            motor: None,
        }
    }

    #[inline]
    pub fn with_limits(mut self, min: super::Float, max: super::Float) -> Self {
        self.limits = Some((min, max));
        self
    }

    #[inline]
    pub fn with_motor(mut self, motor: HingeMotor) -> Self {
        self.motor = Some(motor);
        self
    }
}

impl Constraint for HingeConstraint {
    #[inline]
    fn solve(&self) {
        // Simplified hinge constraint solving
    }

    #[inline]
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Hinge
    }
}

/// Fixed constraint (locks relative position and rotation)
#[derive(Clone, Debug)]
pub struct FixedConstraint {
    pub body_a: super::BodyHandle,
    pub body_b: super::BodyHandle,
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    pub rotation_a: super::Quaternion,
    pub rotation_b: super::Quaternion,
}

impl FixedConstraint {
    #[inline]
    pub fn new(
        body_a: super::BodyHandle,
        body_b: super::BodyHandle,
        anchor_a: Vec3,
        anchor_b: Vec3,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            rotation_a: super::Quaternion::identity(),
            rotation_b: super::Quaternion::identity(),
        }
    }
}

impl Constraint for FixedConstraint {
    #[inline]
    fn solve(&self) {
        // Simplified fixed constraint solving
    }

    #[inline]
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Fixed
    }
}

/// Slider constraint (allows translation along one axis)
#[derive(Clone, Debug)]
pub struct SliderConstraint {
    pub body_a: super::BodyHandle,
    pub body_b: super::BodyHandle,
    pub anchor: Vec3,
    pub axis: Vec3,
    pub limits: Option<(super::Float, super::Float)>,
}

impl SliderConstraint {
    #[inline]
    pub fn new(
        body_a: super::BodyHandle,
        body_b: super::BodyHandle,
        anchor: Vec3,
        axis: Vec3,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor,
            axis: axis.normalize(),
            limits: None,
        }
    }

    #[inline]
    pub fn with_limits(mut self, min: super::Float, max: super::Float) -> Self {
        self.limits = Some((min, max));
        self
    }
}

impl Constraint for SliderConstraint {
    #[inline]
    fn solve(&self) {
        // Simplified slider constraint solving
    }

    #[inline]
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Slider
    }
}

/// Spring constraint (elastic connection)
#[derive(Clone, Debug)]
pub struct SpringConstraint {
    pub body_a: super::BodyHandle,
    pub body_b: super::BodyHandle,
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
    pub rest_length: super::Float,
    pub stiffness: super::Float,
    pub damping: super::Float,
}

impl SpringConstraint {
    #[inline]
    pub fn new(
        body_a: super::BodyHandle,
        body_b: super::BodyHandle,
        anchor_a: Vec3,
        anchor_b: Vec3,
        rest_length: super::Float,
        stiffness: super::Float,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            rest_length,
            stiffness,
            damping: 0.5,
        }
    }

    #[inline]
    pub fn calculate_force(&self, current_length: super::Float) -> super::Float {
        let displacement = current_length - self.rest_length;
        -self.stiffness * displacement
    }

    #[inline]
    pub fn calculate_damping(&self, relative_velocity: Vec3, axis: Vec3) -> Vec3 {
        let velocity_along_axis = relative_velocity.dot(axis);
        axis * (-self.damping * velocity_along_axis)
    }
}

impl Constraint for SpringConstraint {
    #[inline]
    fn solve(&self) {
        // Spring forces applied during integration
    }

    #[inline]
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Spring
    }
}

/// Cone twist constraint (limited angular range)
#[derive(Clone, Debug)]
pub struct ConeTwistConstraint {
    pub body_a: super::BodyHandle,
    pub body_b: super::BodyHandle,
    pub anchor: Vec3,
    pub swing_span: super::Float,
    pub twist_span: super::Float,
}

impl ConeTwistConstraint {
    #[inline]
    pub fn new(
        body_a: super::BodyHandle,
        body_b: super::BodyHandle,
        anchor: Vec3,
        swing_span: super::Float,
        twist_span: super::Float,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor,
            swing_span,
            twist_span,
        }
    }
}

impl Constraint for ConeTwistConstraint {
    #[inline]
    fn solve(&self) {
        // Simplified cone twist constraint solving
    }

    #[inline]
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::ConeTwist
    }
}
