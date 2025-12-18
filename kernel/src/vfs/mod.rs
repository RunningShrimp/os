//! /sys pseudo-filesystem
//!
//! Provides kernel object information through a virtual filesystem.
//! This module is split into multiple submodules for maintainability.

extern crate alloc;

pub mod fs;
pub mod devices;
pub mod kernel;
pub mod error;

pub use fs::*;

