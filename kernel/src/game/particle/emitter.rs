//! # Particle Emitters
//!
//! Various particle emission patterns:
//! - Point emitter
//! - Sphere emitter
//! - Box emitter
//! - Circle emitter
//! - Cone emitter

use super::{Particle, Vec3};

/// Base particle emitter trait
pub trait ParticleEmitter {
    /// Emit a single particle
    fn emit(&self) -> Particle;
    /// Clone the emitter
    fn clone_emitter(&self) -> Box<dyn ParticleEmitter>;
}

/// Point emitter (emits from a single point)
#[derive(Clone, Debug)]
pub struct PointEmitter {
    pub position: Vec3,
    pub velocity: Vec3,
    pub velocity_variation: Vec3,
    pub color: super::Color,
    pub size: super::Float,
}

impl PointEmitter {
    #[inline]
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::zero(),
            velocity_variation: Vec3::new(1.0, 1.0, 1.0),
            color: super::Color::white(),
            size: 1.0,
        }
    }

    #[inline]
    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    #[inline]
    pub fn with_variation(mut self, variation: Vec3) -> Self {
        self.velocity_variation = variation;
        self
    }

    #[inline]
    pub fn with_color(mut self, color: super::Color) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn with_size(mut self, size: super::Float) -> Self {
        self.size = size;
        self
    }

    #[inline]
    fn random_velocity(&self) -> Vec3 {
        Vec3 {
            x: (self.random() - 0.5) * 2.0 * self.velocity_variation.x,
            y: (self.random() - 0.5) * 2.0 * self.velocity_variation.y,
            z: (self.random() - 0.5) * 2.0 * self.velocity_variation.z,
        }
    }

    #[inline]
    fn random(&self) -> super::Float {
        // Simple pseudo-random number generator
        // In production, use a proper RNG
        unsafe {
            static mut SEED: u64 = 12345;
            SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
            ((SEED >> 16) & 0xFFFF) as super::Float / 65536.0
        }
    }
}

impl ParticleEmitter for PointEmitter {
    #[inline]
    fn emit(&self) -> Particle {
        Particle {
            position: self.position,
            velocity: self.velocity + self.random_velocity(),
            acceleration: Vec3::zero(),
            color: self.color,
            size: self.size,
            rotation: 0.0,
            lifetime: 1.0,
            age: 0.0,
            active: true,
        }
    }

    #[inline]
    fn clone_emitter(&self) -> Box<dyn ParticleEmitter> {
        Box::new(self.clone())
    }
}

/// Sphere emitter (emits from sphere surface or volume)
#[derive(Clone, Debug)]
pub struct SphereEmitter {
    pub center: Vec3,
    pub radius: super::Float,
    pub from_surface: bool,
    pub speed: super::Float,
    pub color: super::Color,
    pub size: super::Float,
}

impl SphereEmitter {
    #[inline]
    pub fn new(center: Vec3, radius: super::Float) -> Self {
        Self {
            center,
            radius,
            from_surface: false,
            speed: 1.0,
            color: super::Color::white(),
            size: 1.0,
        }
    }

    #[inline]
    pub fn from_surface(mut self) -> Self {
        self.from_surface = true;
        self
    }

    #[inline]
    pub fn with_speed(mut self, speed: super::Float) -> Self {
        self.speed = speed;
        self
    }

    #[inline]
    pub fn with_color(mut self, color: super::Color) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn with_size(mut self, size: super::Float) -> Self {
        self.size = size;
        self
    }

    #[inline]
    fn random_point(&self) -> Vec3 {
        // Generate random point on unit sphere
        let u = self.random();
        let v = self.random();
        let theta = 2.0 * core::f64::consts::PI as super::Float * u;
        let phi = (2.0 * v - 1.0).acos();

        let x = phi.sin() * theta.cos();
        let y = phi.sin() * theta.sin();
        let z = phi.cos();

        let direction = Vec3 { x, y, z };

        if self.from_surface {
            self.center + direction * self.radius
        } else {
            // Random point in sphere volume
            let r = self.radius * self.random().cbrt();
            self.center + direction * r
        }
    }

    #[inline]
    fn random(&self) -> super::Float {
        unsafe {
            static mut SEED: u64 = 54321;
            SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
            ((SEED >> 16) & 0xFFFF) as super::Float / 65536.0
        }
    }
}

impl ParticleEmitter for SphereEmitter {
    #[inline]
    fn emit(&self) -> Particle {
        let position = self.random_point();
        let direction = (position - self.center).normalize();
        let velocity = direction * self.speed;

        Particle {
            position,
            velocity,
            acceleration: Vec3::zero(),
            color: self.color,
            size: self.size,
            rotation: 0.0,
            lifetime: 1.0,
            age: 0.0,
            active: true,
        }
    }

    #[inline]
    fn clone_emitter(&self) -> Box<dyn ParticleEmitter> {
        Box::new(self.clone())
    }
}

/// Box emitter (emits from box volume)
#[derive(Clone, Debug)]
pub struct BoxEmitter {
    pub center: Vec3,
    pub size: Vec3,
    pub velocity: Vec3,
    pub velocity_variation: Vec3,
    pub color: super::Color,
    pub particle_size: super::Float,
}

impl BoxEmitter {
    #[inline]
    pub fn new(center: Vec3, size: Vec3) -> Self {
        Self {
            center,
            size,
            velocity: Vec3::zero(),
            velocity_variation: Vec3::new(1.0, 1.0, 1.0),
            color: super::Color::white(),
            particle_size: 1.0,
        }
    }

    #[inline]
    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    #[inline]
    pub fn with_color(mut self, color: super::Color) -> Self {
        self.color = color;
        self
    }

    #[inline]
    fn random_point(&self) -> Vec3 {
        let half_size = self.size * 0.5;
        Vec3 {
            x: self.center.x + (self.random() - 0.5) * 2.0 * half_size.x,
            y: self.center.y + (self.random() - 0.5) * 2.0 * half_size.y,
            z: self.center.z + (self.random() - 0.5) * 2.0 * half_size.z,
        }
    }

    #[inline]
    fn random(&self) -> super::Float {
        unsafe {
            static mut SEED: u64 = 65432;
            SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
            ((SEED >> 16) & 0xFFFF) as super::Float / 65536.0
        }
    }
}

impl ParticleEmitter for BoxEmitter {
    #[inline]
    fn emit(&self) -> Particle {
        let position = self.random_point();
        let variation = Vec3 {
            x: (self.random() - 0.5) * 2.0 * self.velocity_variation.x,
            y: (self.random() - 0.5) * 2.0 * self.velocity_variation.y,
            z: (self.random() - 0.5) * 2.0 * self.velocity_variation.z,
        };

        Particle {
            position,
            velocity: self.velocity + variation,
            acceleration: Vec3::zero(),
            color: self.color,
            size: self.particle_size,
            rotation: 0.0,
            lifetime: 1.0,
            age: 0.0,
            active: true,
        }
    }

    #[inline]
    fn clone_emitter(&self) -> Box<dyn ParticleEmitter> {
        Box::new(self.clone())
    }
}

/// Circle emitter (emits from circle perimeter or area)
#[derive(Clone, Debug)]
pub struct CircleEmitter {
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: super::Float,
    pub from_edge: bool,
    pub speed: super::Float,
    pub spread_angle: super::Float,
    pub color: super::Color,
    pub size: super::Float,
}

impl CircleEmitter {
    #[inline]
    pub fn new(center: Vec3, radius: super::Float) -> Self {
        Self {
            center,
            normal: Vec3::new(0.0, 1.0, 0.0),
            radius,
            from_edge: false,
            speed: 1.0,
            spread_angle: 45.0,
            color: super::Color::white(),
            size: 1.0,
        }
    }

    #[inline]
    pub fn with_normal(mut self, normal: Vec3) -> Self {
        self.normal = normal.normalize();
        self
    }

    #[inline]
    pub fn from_edge(mut self) -> Self {
        self.from_edge = true;
        self
    }

    #[inline]
    fn random_point(&self) -> Vec3 {
        let angle = self.random() * 2.0 * core::f64::consts::PI as super::Float;

        // Create tangent vectors
        let tangent = if self.normal.x.abs() > 0.9 {
            Vec3::new(0.0, 1.0, 0.0).cross(self.normal).normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0).cross(self.normal).normalize()
        };
        let bitangent = self.normal.cross(tangent);

        let r = if self.from_edge {
            self.radius
        } else {
            self.radius * self.random().sqrt()
        };

        let offset = tangent * (r * angle.cos()) + bitangent * (r * angle.sin());
        self.center + offset
    }

    #[inline]
    fn random(&self) -> super::Float {
        unsafe {
            static mut SEED: u64 = 76543;
            SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
            ((SEED >> 16) & 0xFFFF) as super::Float / 65536.0
        }
    }
}

impl ParticleEmitter for CircleEmitter {
    #[inline]
    fn emit(&self) -> Particle {
        let position = self.random_point();

        // Velocity with spread around normal
        let spread_rad = (self.spread_angle / 2.0).to_radians();
        let tangent = if self.normal.x.abs() > 0.9 {
            Vec3::new(0.0, 1.0, 0.0).cross(self.normal).normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0).cross(self.normal).normalize()
        };

        let angle_offset = (self.random() - 0.5) * spread_rad;
        let velocity = self.normal + tangent * angle_offset.sin();
        let velocity = velocity.normalize() * self.speed;

        Particle {
            position,
            velocity,
            acceleration: Vec3::zero(),
            color: self.color,
            size: self.size,
            rotation: 0.0,
            lifetime: 1.0,
            age: 0.0,
            active: true,
        }
    }

    #[inline]
    fn clone_emitter(&self) -> Box<dyn ParticleEmitter> {
        Box::new(self.clone())
    }
}

/// Cone emitter (emits in cone shape)
#[derive(Clone, Debug)]
pub struct ConeEmitter {
    pub origin: Vec3,
    pub direction: Vec3,
    pub angle: super::Float,
    pub speed: super::Float,
    pub color: super::Color,
    pub size: super::Float,
}

impl ConeEmitter {
    #[inline]
    pub fn new(origin: Vec3, direction: Vec3, angle: super::Float) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
            angle,
            speed: 1.0,
            color: super::Color::white(),
            size: 1.0,
        }
    }

    #[inline]
    pub fn with_speed(mut self, speed: super::Float) -> Self {
        self.speed = speed;
        self
    }

    #[inline]
    fn random_direction(&self) -> Vec3 {
        // Generate random direction within cone
        let half_angle = self.angle / 2.0;
        let theta = self.random() * 2.0 * core::f64::consts::PI as super::Float;
        let phi = self.random() * half_angle.to_radians();

        // Create orthogonal basis
        let tangent = if self.direction.x.abs() > 0.9 {
            Vec3::new(0.0, 1.0, 0.0).cross(self.direction).normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0).cross(self.direction).normalize()
        };
        let bitangent = self.direction.cross(tangent);

        let x = phi.sin() * theta.cos();
        let y = phi.cos();
        let z = phi.sin() * theta.sin();

        let dir = tangent * x + self.direction * y + bitangent * z;
        dir.normalize()
    }

    #[inline]
    fn random(&self) -> super::Float {
        unsafe {
            static mut SEED: u64 = 87654;
            SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
            ((SEED >> 16) & 0xFFFF) as super::Float / 65536.0
        }
    }
}

impl ParticleEmitter for ConeEmitter {
    #[inline]
    fn emit(&self) -> Particle {
        let velocity = self.random_direction() * self.speed;

        Particle {
            position: self.origin,
            velocity,
            acceleration: Vec3::zero(),
            color: self.color,
            size: self.size,
            rotation: 0.0,
            lifetime: 1.0,
            age: 0.0,
            active: true,
        }
    }

    #[inline]
    fn clone_emitter(&self) -> Box<dyn ParticleEmitter> {
        Box::new(self.clone())
    }
}
