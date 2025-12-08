//! /proc pseudo-filesystem
//!
//! Provides process and system information through a virtual filesystem.
//! This module is split into multiple submodules for maintainability.

extern crate alloc;

pub mod fs;
pub mod proc_info;
pub mod sys_info;

pub use fs::*;

