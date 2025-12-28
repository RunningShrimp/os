//! GLib timerfd re-exports
//!
//! This module bridges the core `timerfd` implementation into the
//! `glib` namespace so that `glib::timerfd` can be used without
//! duplicating logic.

pub use crate::subsystems::syscalls::timerfd::*;



