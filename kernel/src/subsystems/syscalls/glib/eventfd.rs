//! GLib eventfd re-exports
//!
//! This module bridges the core `eventfd` implementation into the
//! `glib` namespace so that `glib::eventfd` can be used without
//! duplicating logic.

pub use crate::subsystems::syscalls::eventfd::*;

