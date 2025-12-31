//! Graphics subsystem for NOS
//!
//! Provides comprehensive graphics and display management including:
//! - GPU management and scheduling
//! - Display engine and mode setting
//! - DRM/KMS framework
//! - 3D acceleration
//! - Multi-display output management
//! - GPU security and isolation
//!
//! This module implements a high-performance graphics system designed for
//! low latency and efficient resource usage.

// Legacy graphics modules
pub mod buffer;
pub mod compositor;
pub mod gui;
pub mod ime;
pub mod input;
pub mod surface;
pub mod vsync;

// New graphics subsystem modules
pub mod error;
pub mod gpu;
pub mod display;
pub mod drm;
pub mod acceleration;
pub mod output;
pub mod security;

// Re-export common types from new modules
pub use error::{GraphicsError, GraphicsResult};
pub use gpu::{GpuDevice, GpuContext, GpuScheduler, Priority};
pub use display::{DisplayEngine, DisplayMode, Framebuffer, Edid, Crtc, Plane, Connector};
pub use drm::{DrmDevice, GemHandle, AtomicCommit, DrmFence, SyncFile};
pub use acceleration::{AccelerationEngine, CommandBuffer, Shader, Texture};
pub use output::{OutputManager, Output, OutputConfig, AudioConfig};
pub use security::{GpuSecurity, SecureContext, SecurityContext};

/// Initialize graphics subsystem
pub fn init() {
    // Initialize buffer manager
    if let Err(e) = buffer::init_buffer_manager() {
        crate::println!("[graphics] Failed to initialize buffer manager: {}", e);
    }

    // Initialize surface manager
    if let Err(e) = surface::init_surface_manager() {
        crate::println!("[graphics] Failed to initialize surface manager: {}", e);
    }

    // Initialize input manager
    if let Err(e) = input::init_input_manager() {
        crate::println!("[graphics] Failed to initialize input manager: {}", e);
    }

    // Initialize GUI manager
    if let Err(e) = gui::init_gui_manager() {
        crate::println!("[graphics] Failed to initialize GUI manager: {}", e);
    }

    // Initialize IME manager
    if let Err(e) = ime::init_ime_manager() {
        crate::println!("[graphics] Failed to initialize IME manager: {}", e);
    }

    crate::println!("[graphics] Graphics subsystem initialized");
}
