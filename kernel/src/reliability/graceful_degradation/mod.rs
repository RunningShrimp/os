//! Graceful Degradation Module
//!
//! Provides system graceful degradation, feature reduction, and service quality control

mod types;
mod strategy;
mod actions;
mod quality;
mod managers;
mod session;
mod core;

// Re-export all public types
pub use types::*;
pub use strategy::*;
pub use actions::*;
pub use quality::*;
pub use managers::*;
pub use session::*;
pub use core::{GracefulDegradationManager, create_graceful_degradation_manager};
