//! # Particle Effects
//!
//! Pre-built particle effects:
//! - Fire
//! - Smoke
//! - Explosion
//! - Rain
//! - Snow
//! - Sparks

use super::{emitter::*, Color, ColorGradient, Particle, ParticleSystem, ParticleSystemConfig, Vec3};

/// Fire effect
pub struct FireEffect;

impl FireEffect {
    #[inline]
    pub fn create(position: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 5000,
            emit_rate: 500.0,
            lifetime: 1.5,
            initial_size: 2.0,
            initial_color: Color::orange(),
            gravity: Vec3::new(0.0, 2.0, 0.0), // Rise upward
            damping: 0.95,
        };

        let emitter = Box::new(CircleEmitter::new(position, 1.0)
            .with_normal(Vec3::new(0.0, 1.0, 0.0))
            .with_speed(3.0));

        let mut system = ParticleSystem::new(config, emitter);

        // Color gradient: orange -> red -> smoke
        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::yellow());
        color_gradient.add_stop(0.2, Color::orange());
        color_gradient.add_stop(0.5, Color::red());
        color_gradient.add_stop(0.8, Color::new(0.3, 0.3, 0.3, 0.5));
        color_gradient.add_stop(1.0, Color::new(0.2, 0.2, 0.2, 0.0));
        system.set_color_gradient(color_gradient);

        // Size gradient: small -> large -> small
        let mut size_gradient = super::SizeGradient::new();
        size_gradient.add_stop(0.0, 0.5);
        size_gradient.add_stop(0.3, 1.5);
        size_gradient.add_stop(1.0, 0.0);
        system.set_size_gradient(size_gradient);

        system
    }

    #[inline]
    pub fn create_torch(position: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 1000,
            emit_rate: 200.0,
            lifetime: 0.8,
            initial_size: 1.0,
            initial_color: Color::orange(),
            gravity: Vec3::new(0.0, 3.0, 0.0),
            damping: 0.95,
        };

        let emitter = Box::new(PointEmitter::new(position)
            .with_velocity(Vec3::new(0.0, 1.0, 0.0))
            .with_variation(Vec3::new(0.3, 0.5, 0.3)));

        let mut system = ParticleSystem::new(config, emitter);

        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::yellow());
        color_gradient.add_stop(0.5, Color::orange());
        color_gradient.add_stop(1.0, Color::new(0.5, 0.2, 0.0, 0.0));
        system.set_color_gradient(color_gradient);

        system
    }
}

/// Smoke effect
pub struct SmokeEffect;

impl SmokeEffect {
    #[inline]
    pub fn create(position: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 3000,
            emit_rate: 100.0,
            lifetime: 4.0,
            initial_size: 3.0,
            initial_color: Color::new(0.5, 0.5, 0.5, 0.3),
            gravity: Vec3::new(0.0, 1.0, 0.0),
            damping: 0.98,
        };

        let emitter = Box::new(CircleEmitter::new(position, 2.0)
            .with_normal(Vec3::new(0.0, 1.0, 0.0))
            .with_speed(1.5));

        let mut system = ParticleSystem::new(config, emitter);

        // Color gradient: gray -> fade out
        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::new(0.6, 0.6, 0.6, 0.4));
        color_gradient.add_stop(0.5, Color::new(0.5, 0.5, 0.5, 0.2));
        color_gradient.add_stop(1.0, Color::new(0.4, 0.4, 0.4, 0.0));
        system.set_color_gradient(color_gradient);

        // Size gradient: expand
        let mut size_gradient = super::SizeGradient::new();
        size_gradient.add_stop(0.0, 0.5);
        size_gradient.add_stop(1.0, 3.0);
        system.set_size_gradient(size_gradient);

        system
    }
}

/// Explosion effect
pub struct ExplosionEffect;

impl ExplosionEffect {
    #[inline]
    pub fn create(position: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 5000,
            emit_rate: 5000.0, // Emit all at once
            lifetime: 1.0,
            initial_size: 2.0,
            initial_color: Color::orange(),
            gravity: Vec3::new(0.0, -2.0, 0.0),
            damping: 0.92,
        };

        let emitter = Box::new(SphereEmitter::new(position, 0.5)
            .from_surface()
            .with_speed(15.0));

        let mut system = ParticleSystem::new(config, emitter);

        // Color gradient: white -> yellow -> orange -> red -> smoke
        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::white());
        color_gradient.add_stop(0.1, Color::yellow());
        color_gradient.add_stop(0.3, Color::orange());
        color_gradient.add_stop(0.6, Color::red());
        color_gradient.add_stop(1.0, Color::new(0.3, 0.3, 0.3, 0.0));
        system.set_color_gradient(color_gradient);

        let mut size_gradient = super::SizeGradient::new();
        size_gradient.add_stop(0.0, 0.5);
        size_gradient.add_stop(0.2, 2.0);
        size_gradient.add_stop(1.0, 0.5);
        system.set_size_gradient(size_gradient);

        // Emit all particles immediately
        system.emit(5000);

        system
    }

    #[inline]
    pub fn create_spark(position: Vec3, direction: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 500,
            emit_rate: 500.0,
            lifetime: 0.5,
            initial_size: 0.3,
            initial_color: Color::yellow(),
            gravity: Vec3::new(0.0, -5.0, 0.0),
            damping: 0.9,
        };

        let emitter = Box::new(ConeEmitter::new(position, direction, 30.0)
            .with_speed(10.0));

        let mut system = ParticleSystem::new(config, emitter);

        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::white());
        color_gradient.add_stop(0.5, Color::orange());
        color_gradient.add_stop(1.0, Color::red());
        system.set_color_gradient(color_gradient);

        system
    }
}

/// Rain effect
pub struct RainEffect;

impl RainEffect {
    #[inline]
    pub fn create(area_size: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 10000,
            emit_rate: 2000.0,
            lifetime: 2.0,
            initial_size: 0.1,
            initial_color: Color::new(0.6, 0.7, 0.9, 0.6),
            gravity: Vec3::new(0.0, -20.0, 0.0),
            damping: 0.99,
        };

        let center = Vec3::zero();
        let emitter = Box::new(BoxEmitter::new(center, area_size)
            .with_velocity(Vec3::new(0.0, -15.0, 0.0))
            .with_variation(Vec3::new(0.5, 1.0, 0.5)));

        let system = ParticleSystem::new(config, emitter);
        system
    }
}

/// Snow effect
pub struct SnowEffect;

impl SnowEffect {
    #[inline]
    pub fn create(area_size: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 8000,
            emit_rate: 500.0,
            lifetime: 5.0,
            initial_size: 0.3,
            initial_color: Color::white(),
            gravity: Vec3::new(0.0, -2.0, 0.0),
            damping: 0.99,
        };

        let center = Vec3::zero();
        let emitter = Box::new(BoxEmitter::new(center, area_size)
            .with_velocity(Vec3::new(0.0, -1.5, 0.0))
            .with_variation(Vec3::new(1.0, 0.5, 1.0)));

        let mut system = ParticleSystem::new(config, emitter);

        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::white());
        color_gradient.add_stop(1.0, Color::new(0.95, 0.95, 1.0, 0.8));
        system.set_color_gradient(color_gradient);

        system
    }
}

/// Magic effect
pub struct MagicEffect;

impl MagicEffect {
    #[inline]
    pub fn create_trail(start: Vec3, end: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 2000,
            emit_rate: 300.0,
            lifetime: 1.0,
            initial_size: 1.0,
            initial_color: Color::new(0.5, 0.3, 1.0, 1.0),
            gravity: Vec3::zero(),
            damping: 0.98,
        };

        let direction = (end - start).normalize();
        let emitter = Box::new(ConeEmitter::new(start, direction, 15.0)
            .with_speed(5.0));

        let mut system = ParticleSystem::new(config, emitter);

        // Purple to blue gradient
        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::new(0.8, 0.5, 1.0, 1.0));
        color_gradient.add_stop(0.5, Color::new(0.4, 0.6, 1.0, 0.8));
        color_gradient.add_stop(1.0, Color::new(0.2, 0.4, 0.8, 0.0));
        system.set_color_gradient(color_gradient);

        let mut size_gradient = super::SizeGradient::new();
        size_gradient.add_stop(0.0, 1.0);
        size_gradient.add_stop(1.0, 0.0);
        system.set_size_gradient(size_gradient);

        system
    }

    #[inline]
    pub fn create_aura(position: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 1000,
            emit_rate: 200.0,
            lifetime: 2.0,
            initial_size: 0.5,
            initial_color: Color::new(0.6, 0.4, 1.0, 0.5),
            gravity: Vec3::zero(),
            damping: 0.97,
        };

        let emitter = Box::new(SphereEmitter::new(position, 1.5)
            .with_speed(2.0));

        let mut system = ParticleSystem::new(config, emitter);

        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::new(0.7, 0.5, 1.0, 0.6));
        color_gradient.add_stop(0.5, Color::new(0.5, 0.3, 0.8, 0.3));
        color_gradient.add_stop(1.0, Color::new(0.3, 0.2, 0.6, 0.0));
        system.set_color_gradient(color_gradient);

        system
    }
}

/// Dust effect
pub struct DustEffect;

impl DustEffect {
    #[inline]
    pub fn create_cloud(position: Vec3) -> ParticleSystem {
        let config = ParticleSystemConfig {
            max_particles: 2000,
            emit_rate: 100.0,
            lifetime: 3.0,
            initial_size: 2.0,
            initial_color: Color::new(0.6, 0.55, 0.45, 0.4),
            gravity: Vec3::new(0.0, -0.5, 0.0),
            damping: 0.98,
        };

        let emitter = Box::new(SphereEmitter::new(position, 2.0)
            .with_speed(0.5));

        let mut system = ParticleSystem::new(config, emitter);

        let mut color_gradient = ColorGradient::new();
        color_gradient.add_stop(0.0, Color::new(0.7, 0.65, 0.55, 0.4));
        color_gradient.add_stop(1.0, Color::new(0.5, 0.45, 0.35, 0.0));
        system.set_color_gradient(color_gradient);

        let mut size_gradient = super::SizeGradient::new();
        size_gradient.add_stop(0.0, 1.0);
        size_gradient.add_stop(1.0, 3.0);
        system.set_size_gradient(size_gradient);

        system
    }
}
