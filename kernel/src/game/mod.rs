//! # Game Engine Module
//!
//! Comprehensive game engine systems for the kernel, including:
//!
//! - **Physics Engine** (`physics`): Rigid body dynamics, collision detection, constraints, soft body physics
//! - **Particle System** (`particle`): High-performance particle effects with GPU acceleration
//! - **Animation System** (`animation`): Skeletal animation, morph targets, blend trees, IK
//! - **Scene Management** (`scene`): Scene graph, spatial partitioning, culling, LOD
//! - **3D Audio** (`audio`): Spatial audio rendering with HRTF, reverb, and Doppler effects
//! - **Input System** (`input`): Keyboard, mouse, gamepad, and touch input handling
//!
//! # Performance Targets
//! - 60+ FPS for complex scenes
//! - Sub-millisecond physics updates
//! - Efficient memory usage
//! - Cross-platform compatibility
//!
//! # Example
//!
//! ```ignore
//! use kernel::game::{PhysicsWorld, ParticleSystem, AnimationMixer};
//!
//! // Create physics world
//! let physics = PhysicsWorld::new(PhysicsConfig::default());
//!
//! // Create particle effect
//! let fire = FireEffect::create(Vec3::new(0.0, 0.0, 0.0));
//!
//! // Create animation mixer
//! let mixer = AnimationMixer::new().with_skeleton(skeleton);
//! ```

pub mod animation;
pub mod audio;
pub mod input;
pub mod particle;
pub mod physics;
pub mod scene;

// Re-export core types for convenience
pub use physics::{
    Collider, Material, PhysicsConfig, PhysicsWorld, Quaternion, RigidBody, Transform, Vec3,
};
pub use particle::{Color, ColorGradient, Particle, ParticleSystem, SizeGradient};
pub use animation::{AnimationClip, AnimationMixer, AnimationState, Skeleton};
pub use scene::{BoundingVolume, Frustum, NodeId, SceneGraph};
pub use audio::{AudioEngine, AudioListener, SoundSource};
pub use input::{InputAction, InputContext, InputManager};

/// Game engine version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Game engine configuration
#[derive(Clone, Debug)]
pub struct GameEngineConfig {
    /// Enable physics simulation
    pub enable_physics: bool,

    /// Enable particle systems
    pub enable_particles: bool,

    /// Enable audio
    pub enable_audio: bool,

    /// Target frame rate
    pub target_fps: u32,

    /// Maximum concurrent sounds
    pub max_sounds: usize,

    /// Maximum particles per system
    pub max_particles: usize,
}

impl Default for GameEngineConfig {
    fn default() -> Self {
        Self {
            enable_physics: true,
            enable_particles: true,
            enable_audio: true,
            target_fps: 60,
            max_sounds: 32,
            max_particles: 10000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_config() {
        let config = GameEngineConfig::default();
        assert_eq!(config.target_fps, 60);
        assert!(config.enable_physics);
    }
}
