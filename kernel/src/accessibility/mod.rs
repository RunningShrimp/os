//! Accessibility
//!
//! This module implements accessibility features for NOS:
//! - Screen reader support
//! - High contrast mode
//! - Keyboard navigation
//!
//! Features:
//! - Text-to-speech announcements
//! - Focus tracking and announcements
//! - High contrast color themes
//! - Full keyboard navigation
//! - Customizable shortcuts
//! - WCAG compliance

pub mod screen_reader;
pub mod high_contrast;
pub mod keyboard_nav;

// Re-export accessibility types
pub use screen_reader::*;
pub use high_contrast::*;
pub use keyboard_nav::*;
