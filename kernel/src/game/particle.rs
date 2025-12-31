//! # Particle System
//!
//! High-performance particle effects with:
//! - Particle emitters
//! - Particle lifetime management
//! - GPU acceleration support
//! - Multiple effect types (fire/smoke/explosion)
//! - Custom shaders and blending

use alloc::vec::Vec;
use core::time::Duration;

use crate::compat::Float;
use crate::subsystems::sync::spinlock::SpinLock;

pub mod emitter;
pub mod effects;

pub use emitter::*;
pub use effects::*;

/// Single particle instance
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: super::Vec3,
    pub velocity: super::Vec3,
    pub acceleration: super::Vec3,
    pub color: Color,
    pub size: Float,
    pub rotation: Float,
    pub lifetime: Float,
    pub age: Float,
    pub active: bool,
}

impl Particle {
    #[inline]
    pub fn new() -> Self {
        Self {
            position: super::Vec3::zero(),
            velocity: super::Vec3::zero(),
            acceleration: super::Vec3::zero(),
            color: Color::white(),
            size: 1.0,
            rotation: 0.0,
            lifetime: 1.0,
            age: 0.0,
            active: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.active && self.age < self.lifetime
    }

    #[inline]
    pub fn life_ratio(&self) -> Float {
        if self.lifetime > 0.0 {
            self.age / self.lifetime
        } else {
            1.0
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self::new()
    }
}

/// RGBA Color
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: Float,
    pub g: Float,
    pub b: Float,
    pub a: Float,
}

impl Color {
    #[inline]
    pub fn new(r: Float, g: Float, b: Float, a: Float) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub fn rgb(r: Float, g: Float, b: Float) -> Self {
        Self::new(r, g, b, 1.0)
    }

    #[inline]
    pub fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }

    #[inline]
    pub fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn red() -> Self {
        Self::rgb(1.0, 0.0, 0.0)
    }

    #[inline]
    pub fn green() -> Self {
        Self::rgb(0.0, 1.0, 0.0)
    }

    #[inline]
    pub fn blue() -> Self {
        Self::rgb(0.0, 0.0, 1.0)
    }

    #[inline]
    pub fn yellow() -> Self {
        Self::rgb(1.0, 1.0, 0.0)
    }

    #[inline]
    pub fn orange() -> Self {
        Self::rgb(1.0, 0.5, 0.0)
    }

    #[inline]
    pub fn lerp(&self, other: Self, t: Float) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    #[inline]
    pub fn with_alpha(&self, alpha: Float) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }

    #[inline]
    pub fn to_u32(&self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        let a = (self.a.clamp(0.0, 1.0) * 255.0) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }

    #[inline]
    pub fn from_u32(color: u32) -> Self {
        Self {
            r: ((color >> 16) & 0xFF) as Float / 255.0,
            g: ((color >> 8) & 0xFF) as Float / 255.0,
            b: (color & 0xFF) as Float / 255.0,
            a: ((color >> 24) & 0xFF) as Float / 255.0,
        }
    }
}

/// Color gradient for particle animation
#[derive(Clone, Debug)]
pub struct ColorGradient {
    pub stops: Vec<(Float, Color)>,
}

impl ColorGradient {
    #[inline]
    pub fn new() -> Self {
        Self {
            stops: Vec::new(),
        }
    }

    #[inline]
    pub fn add_stop(&mut self, time: Float, color: Color) {
        self.stops.push((time, color));
        self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    #[inline]
    pub fn evaluate(&self, t: Float) -> Color {
        if self.stops.is_empty() {
            return Color::white();
        }

        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }

        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }

        // Find the two stops to interpolate between
        for i in 0..self.stops.len() - 1 {
            if t >= self.stops[i].0 && t <= self.stops[i + 1].0 {
                let range = self.stops[i + 1].0 - self.stops[i].0;
                let local_t = if range > 0.0 {
                    (t - self.stops[i].0) / range
                } else {
                    0.0
                };
                return self.stops[i].1.lerp(self.stops[i + 1].1, local_t);
            }
        }

        self.stops.last().unwrap().1
    }
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self::new()
    }
}

/// Size gradient for particle scaling
#[derive(Clone, Debug)]
pub struct SizeGradient {
    pub stops: Vec<(Float, Float)>,
}

impl SizeGradient {
    #[inline]
    pub fn new() -> Self {
        Self {
            stops: Vec::new(),
        }
    }

    #[inline]
    pub fn add_stop(&mut self, time: Float, size: Float) {
        self.stops.push((time, size));
        self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    #[inline]
    pub fn evaluate(&self, t: Float) -> Float {
        if self.stops.is_empty() {
            return 1.0;
        }

        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }

        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }

        for i in 0..self.stops.len() - 1 {
            if t >= self.stops[i].0 && t <= self.stops[i + 1].0 {
                let range = self.stops[i + 1].0 - self.stops[i].0;
                let local_t = if range > 0.0 {
                    (t - self.stops[i].0) / range
                } else {
                    0.0
                };
                return self.stops[i].1 + (self.stops[i + 1].1 - self.stops[i].1) * local_t;
            }
        }

        self.stops.last().unwrap().1
    }
}

impl Default for SizeGradient {
    fn default() -> Self {
        Self::new()
    }
}

/// Particle system configuration
#[derive(Clone, Debug)]
pub struct ParticleSystemConfig {
    pub max_particles: usize,
    pub emit_rate: Float,
    pub lifetime: Float,
    pub initial_size: Float,
    pub initial_color: Color,
    pub gravity: super::Vec3,
    pub damping: Float,
}

impl Default for ParticleSystemConfig {
    fn default() -> Self {
        Self {
            max_particles: 10000,
            emit_rate: 100.0,
            lifetime: 2.0,
            initial_size: 1.0,
            initial_color: Color::white(),
            gravity: super::Vec3::new(0.0, -9.81, 0.0),
            damping: 0.98,
        }
    }
}

/// Particle system
pub struct ParticleSystem {
    config: ParticleSystemConfig,
    particles: SpinLock<Vec<Particle>>,
    emitter: Box<dyn ParticleEmitter>,
    color_gradient: ColorGradient,
    size_gradient: SizeGradient,
    emit_accumulator: Float,
    active_count: usize,
}

impl ParticleSystem {
    #[inline]
    pub fn new(config: ParticleSystemConfig, emitter: Box<dyn ParticleEmitter>) -> Self {
        let particles = SpinLock::new(vec![Particle::new(); config.max_particles]);
        Self {
            config,
            particles,
            emitter,
            color_gradient: ColorGradient::new(),
            size_gradient: SizeGradient::new(),
            emit_accumulator: 0.0,
            active_count: 0,
        }
    }

    #[inline]
    pub fn set_color_gradient(&mut self, gradient: ColorGradient) {
        self.color_gradient = gradient;
    }

    #[inline]
    pub fn set_size_gradient(&mut self, gradient: SizeGradient) {
        self.size_gradient = gradient;
    }

    #[inline]
    pub fn update(&mut self, dt: Float) {
        // Emit new particles
        self.emit_accumulator += dt * self.config.emit_rate;
        let emit_count = self.emit_accumulator as usize;
        self.emit_accumulator -= emit_count as Float;

        for _ in 0..emit_count {
            self.emit_particle();
        }

        // Update existing particles
        let mut particles = self.particles.lock();
        self.active_count = 0;

        for particle in particles.iter_mut() {
            if particle.is_alive() {
                // Apply physics
                particle.acceleration = self.config.gravity;
                particle.velocity += particle.acceleration * dt;
                particle.velocity *= self.config.damping;
                particle.position += particle.velocity * dt;

                // Update age
                particle.age += dt;

                // Update color and size from gradients
                let life_ratio = particle.life_ratio();
                particle.color = self.color_gradient.evaluate(life_ratio);
                particle.size = self.config.initial_size * self.size_gradient.evaluate(life_ratio);

                self.active_count += 1;
            } else {
                particle.active = false;
            }
        }
    }

    #[inline]
    fn emit_particle(&mut self) {
        let mut particles = self.particles.lock();

        // Find inactive particle
        for particle in particles.iter_mut() {
            if !particle.active {
                *particle = self.emitter.emit();
                particle.lifetime = self.config.lifetime;
                particle.age = 0.0;
                particle.active = true;
                return;
            }
        }

        // No inactive particles, replace oldest
        if let Some(oldest) = particles.iter_mut().min_by(|a, b| {
            a.age
                .partial_cmp(&b.age)
                .unwrap_or(core::cmp::Ordering::Equal)
        }) {
            *oldest = self.emitter.emit();
            oldest.lifetime = self.config.lifetime;
            oldest.age = 0.0;
            oldest.active = true;
        }
    }

    #[inline]
    pub fn get_active_particles(&self) -> usize {
        self.active_count
    }

    #[inline]
    pub fn get_particles(&self) -> Vec<Particle> {
        let particles = self.particles.lock();
        particles
            .iter()
            .filter(|p| p.is_alive())
            .copied()
            .collect()
    }

    #[inline]
    pub fn emit(&mut self, count: usize) {
        for _ in 0..count {
            self.emit_particle();
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        let mut particles = self.particles.lock();
        for particle in particles.iter_mut() {
            particle.reset();
        }
        self.active_count = 0;
    }

    #[inline]
    pub fn set_emitter(&mut self, emitter: Box<dyn ParticleEmitter>) {
        self.emitter = emitter;
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active_count > 0
    }
}

/// Rendering modes for particles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleRenderMode {
    Additive,
    AlphaBlend,
    Subtractive,
}

/// Particle batch for GPU rendering
#[derive(Clone, Debug)]
pub struct ParticleBatch {
    pub particles: Vec<Particle>,
    pub texture: Option<usize>,
    pub render_mode: ParticleRenderMode,
}

impl ParticleBatch {
    #[inline]
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            texture: None,
            render_mode: ParticleRenderMode::Additive,
        }
    }

    #[inline]
    pub fn add(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
}

impl Default for ParticleBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Particle pool for efficient allocation
#[derive(Clone, Debug)]
pub struct ParticlePool {
    pool: Vec<Particle>,
    free_list: Vec<usize>,
}

impl ParticlePool {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        let pool = vec![Particle::new(); capacity];
        let free_list = (0..capacity).collect();
        Self { pool, free_list }
    }

    #[inline]
    pub fn acquire(&mut self) -> Option<&mut Particle> {
        if let Some(index) = self.free_list.pop() {
            Some(&mut self.pool[index])
        } else {
            None
        }
    }

    #[inline]
    pub fn release(&mut self, particle: &Particle) {
        // Find index and return to pool
        if let Some(index) = self
            .pool
            .iter()
            .position(|p| p as *const _ == particle as *const _)
        {
            self.free_list.push(index);
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.pool.len()
    }

    #[inline]
    pub fn available(&self) -> usize {
        self.free_list.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_operations() {
        let color1 = Color::rgb(1.0, 0.0, 0.0);
        let color2 = Color::rgb(0.0, 1.0, 0.0);
        let blended = color1.lerp(color2, 0.5);

        assert!((blended.r - 0.5).abs() < 0.001);
        assert!((blended.g - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_gradient() {
        let mut gradient = ColorGradient::new();
        gradient.add_stop(0.0, Color::red());
        gradient.add_stop(1.0, Color::blue());

        let mid_color = gradient.evaluate(0.5);
        assert!(mid_color.r > 0.0);
        assert!(mid_color.b > 0.0);
    }

    #[test]
    fn test_particle_lifetime() {
        let mut particle = Particle::new();
        particle.lifetime = 1.0;
        particle.age = 0.5;
        particle.active = true;

        assert!(particle.is_alive());
        assert!((particle.life_ratio() - 0.5).abs() < 0.001);
    }
}
