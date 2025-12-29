//! GLib signalfd re-exports
//!
//! This module bridges the core `signalfd` implementation into the
//! `glib` namespace so that `glib::signalfd` can be used without
//! duplicating logic.

pub use crate::subsystems::syscalls::signalfd::*;

